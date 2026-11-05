// ADR: Options are parsed strictly at entry: an unknown, repeated or misplaced option is an error; --help and --version stand alone, --smoke and --bench take only --assets, and --data, --assets, --keys and --log-sounds belong to the interactive run.
// ADR: --keys presses keys at set milliseconds after start through the key table (for captures); --data defaults to %LOCALAPPDATA%\Protui; --assets defaults to the nearest ancestor of the crate with `assets/`.
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub const HELP: &str = "Protui prototype host: settings and a placeholder home screen; no game yet.

Usage:
  prototype [--data <dir>] [--assets <dir>] [--keys <ms>:<key>,...] [--log-sounds <file>]
  prototype --smoke [--assets <dir>] | --bench <frames> [--assets <dir>]
  prototype --help | --version

  (no mode)          Open the full-screen program; stdin and stdout must be terminals.
  --data <dir>       Folder for settings.toml (default %LOCALAPPDATA%\\Protui).
  --assets <dir>     Assets folder holding samples/place.toml (default: nearest
                     ancestor of the crate holding assets/).
  --keys <list>      Press keys at milliseconds after start, e.g. 500:down,900:enter.
                     Keys: up down left right shift+left shift+right tab shift+tab
                     enter esc ctrl+c, or one character.
  --log-sounds <file>  Write one line per sound request: ms since start, cue, outcome,
                     whether a device is open. Replaces the file.
  --smoke            Check input, settings, screens and sound (offline mixer, no device)
                     headlessly in a temporary folder.
  --bench <frames>   Draw <frames> frames at 160x50 headless; print build time and bytes.
  --help             Print this help.
  --version          Print the version and commit.

Keys: arrows move and change, Tab/Shift+Tab move focus, Enter activates, Esc goes back,
Ctrl+C quits. Only key presses act. Unknown or repeated options are errors.
";

/// What the program was asked to do.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Help,
    Version,
    /// `assets` is the given or default folder, `None` when neither exists.
    Smoke {
        assets: Option<PathBuf>,
    },
    Bench {
        frames: u32,
        assets: Option<PathBuf>,
    },
    Run(RunOptions),
}

#[derive(Debug, PartialEq, Eq)]
pub struct RunOptions {
    pub data: PathBuf,
    /// The assets folder, or `None` when no default was found and none was given.
    pub assets: Option<PathBuf>,
    /// Scripted presses, sorted by time after start.
    pub keys: Vec<(Duration, KeyEvent)>,
    /// Where `--log-sounds` writes the sound log.
    pub log_sounds: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CliError(String);

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}; use --help for usage", self.0)
    }
}

impl std::error::Error for CliError {}

fn error(message: impl Into<String>) -> CliError {
    CliError(message.into())
}

/// Parses `args` (without the program name). `local_app_data` is `%LOCALAPPDATA%`;
/// `manifest_dir` is where the default assets search starts.
pub fn parse(
    args: impl IntoIterator<Item = OsString>,
    local_app_data: Option<OsString>,
    manifest_dir: &Path,
) -> Result<Command, CliError> {
    let mut mode: Option<(&'static str, Command)> = None;
    let mut data: Option<PathBuf> = None;
    let mut assets: Option<PathBuf> = None;
    let mut keys: Option<Vec<(Duration, KeyEvent)>> = None;
    let mut log_sounds: Option<PathBuf> = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let name = arg
            .to_str()
            .ok_or_else(|| error(format!("unknown argument {arg:?}")))?;
        let mut value = |what: &str| {
            args.next()
                .ok_or_else(|| error(format!("{name} needs {what}")))
        };
        let new_mode = match name {
            "--help" => Some(Command::Help),
            "--version" => Some(Command::Version),
            "--smoke" => Some(Command::Smoke { assets: None }),
            "--bench" => {
                let frames = value("a frame count")?;
                let frames = frames
                    .to_str()
                    .and_then(|f| f.parse::<u32>().ok())
                    .filter(|&f| f > 0)
                    .ok_or_else(|| error(format!("--bench {frames:?}: expected frames > 0")))?;
                Some(Command::Bench {
                    frames,
                    assets: None,
                })
            }
            "--data" => {
                once(&data, name)?;
                data = Some(PathBuf::from(value("a folder")?));
                None
            }
            "--assets" => {
                once(&assets, name)?;
                let dir = PathBuf::from(value("a folder")?);
                if !dir.is_dir() {
                    return Err(error(format!("--assets {}: not a folder", dir.display())));
                }
                assets = Some(dir);
                None
            }
            "--keys" => {
                once(&keys, name)?;
                let list = value("a key list")?;
                let list = list
                    .to_str()
                    .ok_or_else(|| error(format!("--keys {list:?}: not text")))?;
                keys = Some(parse_keys(list)?);
                None
            }
            "--log-sounds" => {
                once(&log_sounds, name)?;
                log_sounds = Some(PathBuf::from(value("a file")?));
                None
            }
            _ => return Err(error(format!("unknown argument {name:?}"))),
        };
        if let Some(command) = new_mode {
            if let Some((earlier, _)) = &mode {
                return Err(error(format!("{name} cannot be combined with {earlier}")));
            }
            mode = Some((name_of(&command), command));
        }
    }
    let given_assets = assets.is_some();
    let assets = assets.or_else(|| default_assets(manifest_dir));
    match mode {
        Some((name, command)) => {
            let stand_alone = matches!(command, Command::Help | Command::Version);
            if data.is_some()
                || keys.is_some()
                || log_sounds.is_some()
                || (stand_alone && given_assets)
            {
                return Err(error(format!(
                    "{name} cannot be combined with these options"
                )));
            }
            Ok(match command {
                Command::Smoke { .. } => Command::Smoke { assets },
                Command::Bench { frames, .. } => Command::Bench { frames, assets },
                other => other,
            })
        }
        None => {
            let data = match data {
                Some(dir) => dir,
                None => PathBuf::from(
                    local_app_data
                        .ok_or_else(|| error("LOCALAPPDATA is not set; pass --data <dir>"))?,
                )
                .join("Protui"),
            };
            Ok(Command::Run(RunOptions {
                data,
                assets,
                keys: keys.unwrap_or_default(),
                log_sounds,
            }))
        }
    }
}

fn name_of(command: &Command) -> &'static str {
    match command {
        Command::Help => "--help",
        Command::Version => "--version",
        Command::Smoke { .. } => "--smoke",
        Command::Bench { .. } => "--bench",
        Command::Run(_) => "",
    }
}

fn once<T>(slot: &Option<T>, name: &str) -> Result<(), CliError> {
    match slot {
        Some(_) => Err(error(format!("{name} given twice"))),
        None => Ok(()),
    }
}

/// The nearest ancestor of `start` (itself included) that holds an `assets` folder.
pub fn default_assets(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|dir| dir.join("assets"))
        .find(|dir| dir.is_dir())
}

/// `<ms>:<key>,...`, sorted by time.
fn parse_keys(list: &str) -> Result<Vec<(Duration, KeyEvent)>, CliError> {
    let mut keys = Vec::new();
    for press in list.split(',') {
        let bad = || error(format!("--keys {press:?}: expected <ms>:<key>"));
        let (at, key) = press.split_once(':').ok_or_else(bad)?;
        let at: u64 = at.trim().parse().map_err(|_| bad())?;
        let key = key_event(key.trim())
            .ok_or_else(|| error(format!("--keys {press:?}: unknown key {key:?}")))?;
        keys.push((Duration::from_millis(at), key));
    }
    keys.sort_by_key(|&(at, _)| at);
    Ok(keys)
}

/// The key press a `--keys` name stands for.
fn key_event(name: &str) -> Option<KeyEvent> {
    let (code, modifiers) = match name.to_ascii_lowercase().as_str() {
        "up" => (KeyCode::Up, KeyModifiers::NONE),
        "down" => (KeyCode::Down, KeyModifiers::NONE),
        "left" => (KeyCode::Left, KeyModifiers::NONE),
        "right" => (KeyCode::Right, KeyModifiers::NONE),
        "shift+left" => (KeyCode::Left, KeyModifiers::SHIFT),
        "shift+right" => (KeyCode::Right, KeyModifiers::SHIFT),
        "tab" => (KeyCode::Tab, KeyModifiers::NONE),
        "shift+tab" => (KeyCode::BackTab, KeyModifiers::SHIFT),
        "enter" => (KeyCode::Enter, KeyModifiers::NONE),
        "esc" => (KeyCode::Esc, KeyModifiers::NONE),
        "ctrl+c" => (KeyCode::Char('c'), KeyModifiers::CONTROL),
        _ => {
            let mut chars = name.chars();
            let c = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            (KeyCode::Char(c), KeyModifiers::NONE)
        }
    };
    Some(KeyEvent::new_with_kind(
        code,
        modifiers,
        KeyEventKind::Press,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::{self, Intent};

    fn run(args: &[&str]) -> Result<Command, CliError> {
        parse(
            args.iter().map(OsString::from),
            Some(OsString::from("C:/local")),
            Path::new("/nonexistent/prototype"),
        )
    }

    #[test]
    fn defaults_and_modes() {
        let Ok(Command::Run(options)) = run(&[]) else {
            panic!("a bare run parses");
        };
        assert_eq!(options.data, PathBuf::from("C:/local").join("Protui"));
        assert_eq!(options.assets, None);
        assert_eq!(run(&["--smoke"]), Ok(Command::Smoke { assets: None }));
        assert_eq!(
            run(&["--bench", "200", "--assets", "."]),
            Ok(Command::Bench {
                frames: 200,
                assets: Some(PathBuf::from("."))
            })
        );
        assert_eq!(run(&["--version"]), Ok(Command::Version));
    }

    #[test]
    fn unknown_repeated_and_combined_arguments_are_errors() {
        for args in [
            &["--nope"][..],
            &["--data", "a", "--data", "b"],
            &["--smoke", "--help"],
            &["--smoke", "--smoke"],
            &["--bench"],
            &["--bench", "0"],
            &["--bench", "x"],
            &["--smoke", "--data", "a"],
            &["--version", "--assets", "."],
            &["--smoke", "--keys", "1:up"],
            &["--smoke", "--log-sounds", "x.log"],
            &["--log-sounds"],
            &["--log-sounds", "a", "--log-sounds", "b"],
            &["--keys", "100:nokey"],
            &["--keys", "abc"],
            &["--keys", "1:up", "--keys", "2:up"],
            &["--assets", "/nonexistent/assets"],
        ] {
            assert!(run(args).is_err(), "{args:?} must be refused");
        }
        assert!(
            parse(Vec::<OsString>::new(), None, Path::new(".")).is_err(),
            "no LOCALAPPDATA and no --data"
        );
    }

    #[test]
    fn scripted_keys_are_sorted_and_go_through_the_key_table() {
        let Ok(Command::Run(options)) = run(&["--keys", "900:enter,100:shift+right,500:Q"]) else {
            panic!("keys parse");
        };
        let intents: Vec<_> = options
            .keys
            .iter()
            .map(|&(at, key)| (at.as_millis(), keys::intent(key)))
            .collect();
        assert_eq!(
            intents,
            [
                (100, Some(Intent::FineRight)),
                (500, Some(Intent::Other)),
                (900, Some(Intent::Activate)),
            ]
        );
    }
}
