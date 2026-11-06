// ADR: The host owns the loop: it blocks on input while nothing moves, polls until the next 33 ms frame while something animates (a notice, or a lit place preview on screen), draws only when dirty or animating, and acts on key presses only. While audio is open it also wakes every 250 ms to advance the play rules and check the sound folder for edits.
// ADR: ratatui::try_init/try_restore enter and leave the terminal (with ratatui's restoring panic hook); an unreadable settings file, a missing or invalid sample place, or a sound manifest error stops the start before the terminal opens, and files are left as they are. No output device is not an error: the run continues silent.
// ADR: Sound requests wait for the draw that shows their cause, then go to the sound host.
mod app;
mod bench;
#[cfg(test)]
mod capture_test;
mod cli;
mod keys;
mod preview;
mod settings;
mod smoke;
mod sound;
mod store;
mod ui;

use std::env;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEvent};

use app::App;
use cli::{Command, RunOptions};
use preview::View;
use sound::Sound;

/// The frame budget: at most 30 frames a second.
pub const FRAME: Duration = Duration::from_millis(33);

fn main() -> ExitCode {
    let command = match cli::parse(
        env::args_os().skip(1),
        cli::data_home(),
        Path::new(env!("CARGO_MANIFEST_DIR")),
    ) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("prototype: {error}");
            return ExitCode::from(2);
        }
    };
    let result = match command {
        Command::Help => io::stdout().lock().write_all(cli::HELP.as_bytes()),
        Command::Version => writeln!(io::stdout().lock(), "prototype {}", ui::BUILD),
        Command::Smoke { assets } => load_view(assets.as_deref()).and_then(|view| {
            let sound = load_sound(assets.as_deref())?;
            smoke::run(view, sound, &mut io::stdout().lock())
        }),
        Command::Bench { frames, assets } => load_view(assets.as_deref())
            .and_then(|view| bench::run(frames, view, &mut io::stdout().lock())),
        Command::Run(options) => run_interactive(options),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("prototype: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The sample place's view, built once; its file errors read `file:line: key: reason`.
fn load_view(assets: Option<&Path>) -> io::Result<View> {
    View::load(assets, Instant::now())
        .map_err(|error| io::Error::other(format!("the sample place cannot be loaded: {error}")))
}

/// The sound manifest in `assets` with the host's cues; its errors read `file:line: key: reason`,
/// one per line.
fn load_sound(assets: Option<&Path>) -> io::Result<sound::Loaded> {
    let assets = assets.ok_or_else(|| {
        io::Error::other("no assets folder found above the crate; pass --assets <dir>")
    })?;
    sound::Loaded::load(&sound::manifest_path(assets))
        .map_err(|lines| io::Error::other(format!("the sound manifest cannot be loaded:\n{lines}")))
}

fn run_interactive(options: RunOptions) -> io::Result<()> {
    let settings = settings::load(&options.data).map_err(|error| {
        io::Error::other(format!(
            "the settings cannot be read: {error}\nFix or delete the file; the program leaves it as it is."
        ))
    })?;
    let view = load_view(options.assets.as_deref())?;
    let loaded = load_sound(options.assets.as_deref())?;
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::new(
            io::ErrorKind::NotConnected,
            "interactive mode requires terminal stdin and stdout; use --smoke for headless checks",
        ));
    }
    let log = options
        .log_sounds
        .as_deref()
        .map(sound::open_log)
        .transpose()?;
    let audio = settings.unwrap_or_default().audio;
    let sound = Sound::open_device(loaded, audio, log, Instant::now());
    let mut app = App::open(options.data, options.assets, settings);
    app.audio_off = sound.off_reason().map(str::to_string);

    // Ratatui installs its restoration panic hook before changing terminal modes.
    let mut terminal = match ratatui::try_init() {
        Ok(terminal) => terminal,
        Err(error) => {
            // Initialization can fail after enabling raw mode or the alternate screen.
            ratatui::restore();
            return Err(error);
        }
    };
    let result = run(&mut terminal, app, view, sound, &options.keys);
    // Always attempt restoration, including when drawing or reading input failed.
    let restoration = ratatui::try_restore();
    result.and(restoration)
}

/// The loop. `script` holds presses at times after start (from `--keys`), sorted.
fn run(
    terminal: &mut DefaultTerminal,
    mut app: App,
    mut view: View,
    mut sound: Sound<kira::DefaultBackend>,
    script: &[(Duration, KeyEvent)],
) -> io::Result<()> {
    let start = Instant::now();
    let mut script = script.iter().peekable();
    let mut dirty = true;
    loop {
        let now = Instant::now();
        let clock = now.duration_since(start);
        dirty |= app.step(now);
        sound.update(clock);
        sound.poll_device();
        sound.poll_files(now, clock);
        dirty |= show_problem(&mut app, &sound);
        let animating = app.animating() || view.animating();
        if dirty || animating {
            terminal
                .draw(|frame| ui::draw(&app, &mut view, frame.area(), frame.buffer_mut(), now))?;
            dirty = false;
        }
        for request in app.requests.drain(..) {
            sound.handle(request, clock);
        }
        dirty |= show_problem(&mut app, &sound);
        if app.quit {
            return Ok(());
        }
        // Wait for input, but no longer than the next frame while animating, the next sound
        // check while audio is open, or the next scripted press.
        let frame_due = (app.animating() || view.animating()).then(|| now + FRAME);
        let sound_due = sound.is_open().then(|| now + sound::WATCH_INTERVAL);
        let script_due = script.peek().map(|&&(at, _)| start + at);
        let due = [frame_due, sound_due, script_due]
            .into_iter()
            .flatten()
            .min();
        let ready = match due {
            Some(due) => event::poll(due.saturating_duration_since(Instant::now()))?,
            None => true,
        };
        if ready {
            match event::read()? {
                Event::Key(key) => {
                    if let Some(intent) = keys::intent(key) {
                        app.intent(intent, Instant::now());
                        dirty = true;
                    }
                }
                Event::Resize(..) => dirty = true,
                _ => {}
            }
        }
        while let Some(&&(at, key)) = script.peek()
            && start + at <= Instant::now()
        {
            script.next();
            if let Some(intent) = keys::intent(key) {
                app.intent(intent, Instant::now());
                dirty = true;
            }
        }
    }
}

/// Copies the sound problem line into `app` when it changed; `true` then.
fn show_problem<B: kira::backend::Backend>(app: &mut App, sound: &Sound<B>) -> bool {
    if app.sound_problem.as_deref() == sound.problem() {
        return false;
    }
    app.sound_problem = sound.problem().map(str::to_string);
    true
}
