// ADR: `sounds.toml` loads all-or-nothing: strict keys, every value checked at its line, every WAV decoded; any problem refuses the whole manifest and is reported as `file:line: key: reason`.
// ADR: A missing or unreadable WAV refuses the manifest (Runner played nothing instead), so a loaded bank always holds every listed sound; files nobody lists are not checked.
// ADR: A cue name is `group/name` (lowercase letters, digits, `-`); `Manifest::require` lets a host refuse a manifest that lacks a name its code plays.
//! Adapted from Runner `crates/runner-data/src/audio.rs` and its `error.rs` line/key reporting.
use crate::{
    Bank, BusId, Clip, CueOptions, Duck, EngineConfig, InstanceLimit, MAX_BUSES, MAX_VARIATIONS,
    Pick, Playback, Steal, WavError,
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap},
    fmt, io,
    ops::Range,
    path::{Path, PathBuf},
    time::Duration,
};
use toml::{
    Spanned,
    de::{DeTable, DeValue},
};

/// A loaded manifest: the bank with every clip decoded, and the mix it names.
pub struct Manifest {
    pub bank: Bank,
    pub config: EngineConfig,
    path: PathBuf,
}

/// A cue's name and the WAV paths it lists (resolved beside the manifest), for tools that write
/// the files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CueFiles {
    pub name: String,
    pub files: Vec<PathBuf>,
}

impl Manifest {
    /// Reads, checks and decodes the manifest at `path` and every WAV it lists. Returns every
    /// problem found, or a complete bank.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ManifestErrors> {
        let path = path.as_ref();
        let text = read(path)?;
        let source = Source { path, text: &text };
        let checked = check(&source)?;
        let bank = build(&source, checked.cues, &checked.buses)?;
        Ok(Self {
            bank,
            config: EngineConfig {
                max_voices: checked.max_voices,
                duck: checked.duck,
            },
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Refuses the manifest when it lacks any of `names`; lists every missing one.
    pub fn require(&self, names: &[&str]) -> Result<(), ManifestErrors> {
        let missing: Vec<ManifestError> = names
            .iter()
            .filter(|name| self.bank.cue(name).is_none())
            .map(|name| ManifestError {
                file: self.path.clone(),
                line: None,
                key: format!("cue.{name}"),
                problem: Problem::Required,
            })
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(ManifestErrors(missing))
        }
    }
}

/// Checks the manifest at `path` like `Manifest::load` but decodes no WAV: lists each cue's
/// files, which need not exist yet.
pub fn cue_files(path: impl AsRef<Path>) -> Result<Vec<CueFiles>, ManifestErrors> {
    let path = path.as_ref();
    let text = read(path)?;
    let checked = check(&Source { path, text: &text })?;
    Ok(checked
        .cues
        .into_iter()
        .map(|cue| CueFiles {
            name: cue.name,
            files: cue.files.into_iter().map(|(path, _)| path).collect(),
        })
        .collect())
}

/// One problem at a place in a manifest.
#[derive(Clone, Debug, PartialEq)]
pub struct ManifestError {
    pub file: PathBuf,
    /// 1-based; `None` for problems of the whole file or of a key that is absent.
    pub line: Option<usize>,
    /// Dotted TOML key, such as `cue.ui/move.files[0]`; empty when none applies.
    pub key: String,
    pub problem: Problem,
}
impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = self.file.display();
        match (self.line, self.key.is_empty()) {
            (Some(line), _) => write!(f, "{file}:{line}: {}: {}", self.key, self.problem),
            (None, false) => write!(f, "{file}: {}: {}", self.key, self.problem),
            (None, true) => write!(f, "{file}: {}", self.problem),
        }
    }
}
impl std::error::Error for ManifestError {}

/// Every problem of one load, in file order.
#[derive(Clone, Debug, PartialEq)]
pub struct ManifestErrors(pub Vec<ManifestError>);
impl fmt::Display for ManifestErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, error) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str("\n")?;
            }
            write!(f, "{error}")?;
        }
        Ok(())
    }
}
impl std::error::Error for ManifestErrors {}

#[derive(Clone, Debug, PartialEq)]
pub enum Problem {
    /// The manifest file cannot be read.
    Read(io::ErrorKind),
    /// TOML syntax, an unknown or missing key, or a value of the wrong type.
    Shape(String),
    OutOfRange {
        min: f32,
        max: f32,
    },
    IntOutOfRange {
        min: u32,
        max: u32,
    },
    TooMany {
        max: usize,
    },
    CueName,
    BusName,
    UnknownBus(String),
    NoFiles,
    WavPath,
    Wav {
        file: String,
        error: WavError,
    },
    StealNeedsLimit,
    LoopNeedsFade,
    FadeNeedsLoop,
    /// The host requires this cue and the manifest lacks it.
    Required,
    /// The bank refused a checked value; names the bank's reason.
    Rejected(String),
}
impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(kind) => write!(f, "cannot read: {kind}"),
            Self::Shape(message) => f.write_str(message),
            Self::OutOfRange { min, max } => {
                write!(f, "must be a finite number within {min}..={max}")
            }
            Self::IntOutOfRange { min, max } => write!(f, "must be within {min}..={max}"),
            Self::TooMany { max } => write!(f, "at most {max} entries"),
            Self::CueName => f.write_str(
                "a cue name is `group/name`: lowercase letters, digits and `-` in each part",
            ),
            Self::BusName => f.write_str("a bus name is lowercase letters, digits and `-`"),
            Self::UnknownBus(name) => write!(f, "no bus \"{name}\""),
            Self::NoFiles => f.write_str("list at least one file"),
            Self::WavPath => f.write_str(
                "write a path relative to the manifest ending in .wav, `/` between folders, no `..`",
            ),
            Self::Wav { file, error } => write!(f, "{file}: {error}"),
            Self::StealNeedsLimit => f.write_str("`steal` needs `max_instances`"),
            Self::LoopNeedsFade => f.write_str("a loop needs `fade_ms`"),
            Self::FadeNeedsLoop => f.write_str("`fade_ms` needs `loop = true`"),
            Self::Required => f.write_str("missing; the host plays this cue"),
            Self::Rejected(reason) => f.write_str(reason),
        }
    }
}

/// Whether `name` is a cue name: `group/name`, both parts lowercase letters, digits and `-`.
pub(crate) fn is_cue_name(name: &str) -> bool {
    matches!(name.split_once('/'), Some((group, rest)) if is_part(group) && is_part(rest))
}
fn is_part(p: &str) -> bool {
    !p.is_empty()
        && p.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

// ---- file shape ----

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileSpec {
    mix: MixSpec,
    bus: BTreeMap<Spanned<String>, BusSpec>,
    duck: Option<DuckSpec>,
    cue: BTreeMap<Spanned<String>, CueSpec>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MixSpec {
    max_voices: Spanned<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BusSpec {
    volume_db: Spanned<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DuckSpec {
    bus: Spanned<String>,
    by_db: Spanned<f32>,
    attack_ms: u32,
    release_ms: u32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum PickSpec {
    Random,
    Shuffle,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum StealSpec {
    Oldest,
    None,
}

/// Only `bus` and `files` are required; the README names each optional key's absence.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CueSpec {
    bus: Spanned<String>,
    files: Spanned<Vec<Spanned<String>>>,
    pick: Option<PickSpec>,
    volume_db: Option<Spanned<f32>>,
    volume_jitter_db: Option<Spanned<f32>>,
    pitch_jitter_semitones: Option<Spanned<f32>>,
    max_instances: Option<Spanned<u32>>,
    steal: Option<Spanned<StealSpec>>,
    cooldown_ms: Option<u32>,
    duck: Option<bool>,
    #[serde(rename = "loop")]
    looping: Option<Spanned<bool>>,
    fade_ms: Option<Spanned<u32>>,
}

// ---- checking ----

struct CheckedCue {
    name: String,
    span: Range<usize>,
    files: Vec<(PathBuf, Range<usize>)>,
    options: CueOptions,
}

struct Checked {
    max_voices: usize,
    buses: Vec<(String, f32)>,
    duck: Option<Duck>,
    cues: Vec<CheckedCue>,
}

fn read(path: &Path) -> Result<String, ManifestErrors> {
    std::fs::read_to_string(path).map_err(|e| {
        ManifestErrors(vec![ManifestError {
            file: path.to_path_buf(),
            line: None,
            key: String::new(),
            problem: Problem::Read(e.kind()),
        }])
    })
}

fn check(source: &Source<'_>) -> Result<Checked, ManifestErrors> {
    let spec: FileSpec =
        toml::from_str(source.text).map_err(|e| ManifestErrors(vec![source.shape(&e)]))?;
    let mut errors = Vec::new();
    let folder = source.path.parent().unwrap_or(Path::new("."));
    let max_voices = *spec.mix.max_voices.get_ref();
    if !(1..=65_535).contains(&max_voices) {
        errors.push(source.at(
            spec.mix.max_voices.span(),
            Problem::IntOutOfRange {
                min: 1,
                max: 65_535,
            },
        ));
    }
    if spec.bus.len() > MAX_BUSES {
        errors.push(source.keyed("bus", Problem::TooMany { max: MAX_BUSES }));
    }
    let buses: Vec<(String, f32)> = spec
        .bus
        .iter()
        .map(|(name, bus)| {
            if !is_part(name.get_ref()) {
                errors.push(source.at(name.span(), Problem::BusName));
            }
            let volume = ranged(&bus.volume_db, -120.0, 24.0, source, &mut errors);
            (name.get_ref().clone(), volume)
        })
        .collect();
    let bus_id = |name: &Spanned<String>, errors: &mut Vec<ManifestError>| {
        let found = buses.iter().position(|(b, _)| b == name.get_ref());
        if found.is_none() {
            errors.push(source.at(name.span(), Problem::UnknownBus(name.get_ref().clone())));
        }
        BusId(found.unwrap_or(0))
    };
    let duck = spec.duck.as_ref().map(|duck| Duck {
        bus: bus_id(&duck.bus, &mut errors),
        by_db: ranged(&duck.by_db, -120.0, 0.0, source, &mut errors),
        attack: Duration::from_millis(u64::from(duck.attack_ms)),
        release: Duration::from_millis(u64::from(duck.release_ms)),
    });
    let mut cues = Vec::with_capacity(spec.cue.len());
    for (name, cue) in &spec.cue {
        if !is_cue_name(name.get_ref()) {
            errors.push(source.at(name.span(), Problem::CueName));
        }
        let listed = cue.files.get_ref();
        if listed.is_empty() {
            errors.push(source.at(cue.files.span(), Problem::NoFiles));
        }
        if listed.len() > MAX_VARIATIONS {
            errors.push(source.at(
                cue.files.span(),
                Problem::TooMany {
                    max: MAX_VARIATIONS,
                },
            ));
        }
        let mut files = Vec::with_capacity(listed.len());
        for file in listed {
            match wav_path(file.get_ref()) {
                Some(relative) => files.push((folder.join(relative), file.span())),
                None => errors.push(source.at(file.span(), Problem::WavPath)),
            }
        }
        let instances = match (&cue.max_instances, &cue.steal) {
            (None, None) => InstanceLimit::Unlimited,
            (None, Some(steal)) => {
                errors.push(source.at(steal.span(), Problem::StealNeedsLimit));
                InstanceLimit::Unlimited
            }
            (Some(count), steal) => {
                if *count.get_ref() == 0 {
                    errors.push(source.at(
                        count.span(),
                        Problem::IntOutOfRange {
                            min: 1,
                            max: u32::MAX,
                        },
                    ));
                }
                InstanceLimit::Instances {
                    count: *count.get_ref() as usize,
                    steal: match steal.as_ref().map(|s| *s.get_ref()) {
                        Some(StealSpec::Oldest) => Steal::Oldest,
                        Some(StealSpec::None) | None => Steal::Never,
                    },
                }
            }
        };
        let playback = match (&cue.looping, &cue.fade_ms) {
            (Some(looping), Some(fade)) if *looping.get_ref() => Playback::Loop {
                fade_in: Duration::from_millis(u64::from(*fade.get_ref())),
            },
            (Some(looping), None) if *looping.get_ref() => {
                errors.push(source.at(looping.span(), Problem::LoopNeedsFade));
                Playback::Once
            }
            (_, Some(fade)) => {
                errors.push(source.at(fade.span(), Problem::FadeNeedsLoop));
                Playback::Once
            }
            (_, None) => Playback::Once,
        };
        let optional = |value: &Option<Spanned<f32>>, min, max, errors: &mut Vec<_>| {
            value
                .as_ref()
                .map_or(0.0, |v| ranged(v, min, max, source, errors))
        };
        let options = CueOptions {
            bus: bus_id(&cue.bus, &mut errors),
            volume_db: optional(&cue.volume_db, -120.0, 24.0, &mut errors),
            playback_rate: 1.0,
            playback,
            cooldown: Duration::from_millis(u64::from(cue.cooldown_ms.unwrap_or(0))),
            instances,
            ducks: cue.duck.unwrap_or(false),
            pick: match cue.pick {
                Some(PickSpec::Shuffle) => Pick::Shuffle,
                Some(PickSpec::Random) | None => Pick::Random,
            },
            volume_jitter_db: optional(&cue.volume_jitter_db, 0.0, 24.0, &mut errors),
            pitch_jitter_semitones: optional(&cue.pitch_jitter_semitones, 0.0, 12.0, &mut errors),
        };
        cues.push(CheckedCue {
            name: name.get_ref().clone(),
            span: name.span(),
            files,
            options,
        });
    }
    if !errors.is_empty() {
        errors.sort_by_key(|e| e.line);
        return Err(ManifestErrors(errors));
    }
    Ok(Checked {
        max_voices: max_voices as usize,
        buses,
        duck,
        cues,
    })
}

/// Decodes every listed WAV once and builds the bank; nothing is returned unless all succeed.
fn build(
    source: &Source<'_>,
    cues: Vec<CheckedCue>,
    buses: &[(String, f32)],
) -> Result<Bank, ManifestErrors> {
    let mut errors = Vec::new();
    let mut decoded: HashMap<PathBuf, Option<Clip>> = HashMap::new();
    for cue in &cues {
        for (path, span) in &cue.files {
            if decoded.contains_key(path) {
                continue;
            }
            let clip = match Clip::read_wav(path) {
                Ok(clip) => Some(clip),
                Err(error) => {
                    errors.push(source.at(
                        span.clone(),
                        Problem::Wav {
                            file: path.display().to_string(),
                            error,
                        },
                    ));
                    None
                }
            };
            decoded.insert(path.clone(), clip);
        }
    }
    if !errors.is_empty() {
        return Err(ManifestErrors(errors));
    }
    let mut bank = Bank::new();
    for (name, volume) in buses {
        bank.add_bus(name.clone(), *volume).map_err(|e| {
            ManifestErrors(vec![
                source.keyed(&format!("bus.{name}"), Problem::Rejected(e.to_string())),
            ])
        })?;
    }
    for cue in cues {
        let clips = cue
            .files
            .iter()
            .filter_map(|(path, _)| decoded.get(path).cloned().flatten());
        bank.add_cue(cue.name, clips, cue.options).map_err(|e| {
            ManifestErrors(vec![source.at(cue.span, Problem::Rejected(e.to_string()))])
        })?;
    }
    Ok(bank)
}

fn ranged(
    value: &Spanned<f32>,
    min: f32,
    max: f32,
    source: &Source<'_>,
    errors: &mut Vec<ManifestError>,
) -> f32 {
    let v = *value.get_ref();
    if !(min..=max).contains(&v) {
        errors.push(source.at(value.span(), Problem::OutOfRange { min, max }));
    }
    v
}

/// A relative `/`-separated path ending in `.wav`, without empty, `.` or `..` parts.
fn wav_path(file: &str) -> Option<PathBuf> {
    let valid = file.ends_with(".wav")
        && !file.contains(['\\', ':'])
        && file
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..");
    valid.then(|| file.split('/').collect())
}

// ---- line and key reporting ----

struct Source<'a> {
    path: &'a Path,
    text: &'a str,
}

impl Source<'_> {
    fn line(&self, offset: usize) -> usize {
        self.text.as_bytes()[..offset.min(self.text.len())]
            .iter()
            .filter(|&&b| b == b'\n')
            .count()
            + 1
    }

    fn shape(&self, error: &toml::de::Error) -> ManifestError {
        let offset = error.span().map_or(0, |span| span.start);
        ManifestError {
            file: self.path.to_path_buf(),
            line: Some(self.line(offset)),
            key: self.key_at(offset),
            problem: Problem::Shape(error.message().trim_end().to_string()),
        }
    }

    fn keyed(&self, key: &str, problem: Problem) -> ManifestError {
        ManifestError {
            file: self.path.to_path_buf(),
            line: None,
            key: key.to_string(),
            problem,
        }
    }

    fn at(&self, span: Range<usize>, problem: Problem) -> ManifestError {
        ManifestError {
            file: self.path.to_path_buf(),
            line: Some(self.line(span.start)),
            key: self.key_at(span.start),
            problem,
        }
    }

    /// The dotted path of the deepest key or value that holds byte `offset`, such as
    /// `cue.ui/move.files[2]`. In text that does not parse, the key written on that line; `-`
    /// when there is none.
    fn key_at(&self, offset: usize) -> String {
        let Ok(document) = DeTable::parse(self.text) else {
            let start = self.text[..offset.min(self.text.len())]
                .rfind('\n')
                .map_or(0, |i| i + 1);
            let line = self.text[start..].lines().next().unwrap_or_default();
            return match line.split_once('=') {
                Some((key, _)) if !key.trim().is_empty() => key.trim().to_string(),
                _ => "-".to_string(),
            };
        };
        let mut path = String::new();
        if find_in_table(document.get_ref(), offset, &mut path) {
            path
        } else {
            "-".to_string()
        }
    }
}

fn find_in_table(table: &DeTable<'_>, offset: usize, path: &mut String) -> bool {
    for (key, value) in table.iter() {
        let kept = path.len();
        if !path.is_empty() {
            path.push('.');
        }
        path.push_str(key.get_ref());
        if key.span().contains(&offset)
            || find_in_value(value, offset, path)
            || value.span().contains(&offset)
        {
            return true;
        }
        path.truncate(kept);
    }
    false
}

fn find_in_value(value: &Spanned<DeValue<'_>>, offset: usize, path: &mut String) -> bool {
    match value.get_ref() {
        DeValue::Table(table) => find_in_table(table, offset, path),
        DeValue::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let kept = path.len();
                path.push_str(&format!("[{index}]"));
                if find_in_value(item, offset, path) || item.span().contains(&offset) {
                    return true;
                }
                path.truncate(kept);
            }
            false
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_cue_name, wav_path};

    #[test]
    fn cue_names_have_a_group_and_a_name() {
        assert!(is_cue_name("ui/move"));
        assert!(is_cue_name("ambience/room-2"));
        for bad in [
            "move",
            "ui/",
            "/move",
            "Ui/move",
            "a/b/c",
            "ui/move.wav",
            "",
        ] {
            assert!(!is_cue_name(bad), "{bad}");
        }
    }

    #[test]
    fn wav_paths_stay_below_the_manifest() {
        assert!(wav_path("a.wav").is_some());
        assert!(wav_path("ui/a.wav").is_some());
        for bad in [
            "../a.wav",
            "/a.wav",
            "a.ogg",
            "ui//a.wav",
            "C:/a.wav",
            "ui\\a.wav",
            "./a.wav",
        ] {
            assert!(wav_path(bad).is_none(), "{bad}");
        }
    }
}
