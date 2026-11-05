// ADR: The host plays only neutral check cues (a ui cue, an ambience loop, a music loop); which sounds the game plays is a `design/` choice. `App` emits `Request`s; the loop hands them to `Sound` after the frame that shows their cause is drawn.
// ADR: Volume settings map to the manifest's buses: buses of `music/…` cues follow the music volume, every other bus the effects volume, each on top of the bus's manifest level; percent maps to decibels as 40·log10(p/100), 0 % is silence.
// ADR: Without a device the host runs silent and keeps the reason for the screens; a bad manifest edit keeps the playing bank and its error lines show until a good edit loads. Nothing here writes to stderr.
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use kira::backend::Backend;
use serde::Deserialize;
use terminal_audio::{
    AudioError, CueId, Dropped, Engine, Layer, Manifest, OfflineBackend, PlayOutcome, Watcher,
};

use crate::settings::{self, Volume};

/// The manifest, relative to the assets folder.
pub const MANIFEST: &str = "audio/sounds.toml";
/// Played by setup's "Test sound" row.
pub const TEST_CUE: &str = "ui/confirm";
pub const AMBIENCE_CUE: &str = "ambience/room-tone";
pub const MUSIC_CUE: &str = "music/drone";
/// Every cue the host plays; start-up refuses a manifest without one of them.
pub const REQUIRED: [&str; 3] = [TEST_CUE, AMBIENCE_CUE, MUSIC_CUE];
/// Crossfade of the background test layer.
pub const BACKGROUND_FADE: Duration = Duration::from_millis(1500);
/// How often the manifest folder is checked for edits while audio is open.
pub const WATCH_INTERVAL: Duration = Duration::from_millis(250);
/// Fade of a live volume change, short enough to follow the cursor without clicks.
const MIX_FADE: Duration = Duration::from_millis(50);
/// Fade-out of the old bank's voices on a reload.
const RELOAD_FADE: Duration = Duration::from_millis(300);
/// The quietest level the mixer takes; 0 % maps here.
const SILENT_DB: f32 = -120.0;

pub fn manifest_path(assets: &Path) -> PathBuf {
    assets.join(MANIFEST)
}

/// The background test layer's states, in Enter order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Background {
    #[default]
    Off,
    Ambience,
    Music,
}

impl Background {
    pub fn next(self) -> Background {
        match self {
            Background::Off => Background::Ambience,
            Background::Ambience => Background::Music,
            Background::Music => Background::Off,
        }
    }

    /// The cue this state plays; `None` when off.
    pub fn cue(self) -> Option<&'static str> {
        match self {
            Background::Off => None,
            Background::Ambience => Some(AMBIENCE_CUE),
            Background::Music => Some(MUSIC_CUE),
        }
    }
}

/// What the app asks of the sound host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    /// Apply these volumes and mute now (a live setup change, or the saved values restored).
    Mix(settings::Audio),
    /// Play the test cue.
    Test,
    /// Set the background test layer.
    Background(Background),
}

/// Which volume setting a bus follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Music,
    Effects,
}

#[derive(Clone, Debug, PartialEq)]
struct MixBus {
    name: String,
    /// The bus level the manifest sets.
    base_db: f32,
    part: Part,
}

#[derive(Clone, Copy, Debug)]
struct Cues {
    test: CueId,
    ambience: CueId,
    music: CueId,
}

/// A manifest loaded, checked for the host's cues and mapped to the volume settings.
pub struct Loaded {
    manifest: Manifest,
    buses: Vec<MixBus>,
    cues: Cues,
}

/// The part of `sounds.toml` the volume mapping reads; `Manifest::load` has checked the rest.
#[derive(Deserialize)]
struct BusTable {
    #[serde(default)]
    bus: BTreeMap<String, RawBus>,
    #[serde(default)]
    cue: BTreeMap<String, RawCue>,
}

#[derive(Deserialize)]
struct RawBus {
    volume_db: f32,
}

#[derive(Deserialize)]
struct RawCue {
    bus: String,
}

impl Loaded {
    /// Loads `path`; an error is one `file:line: key: reason` line per problem.
    pub fn load(path: &Path) -> Result<Loaded, String> {
        let manifest = Manifest::load(path).map_err(|e| e.to_string())?;
        manifest.require(&REQUIRED).map_err(|e| e.to_string())?;
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let table: BusTable =
            toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let buses = table
            .bus
            .iter()
            .map(|(name, bus)| {
                let music = table
                    .cue
                    .iter()
                    .any(|(cue, raw)| raw.bus == *name && cue.starts_with("music/"));
                MixBus {
                    name: name.clone(),
                    base_db: bus.volume_db,
                    part: if music { Part::Music } else { Part::Effects },
                }
            })
            .collect();
        let bank = &manifest.bank;
        let cue = |name: &str| {
            bank.cue(name)
                .ok_or_else(|| format!("{}: cue {name:?} is missing", path.display()))
        };
        let cues = Cues {
            test: cue(TEST_CUE)?,
            ambience: cue(AMBIENCE_CUE)?,
            music: cue(MUSIC_CUE)?,
        };
        Ok(Loaded {
            manifest,
            buses,
            cues,
        })
    }
}

/// A volume setting in decibels: 40·log10(percent / 100), 0 % silent.
pub fn percent_db(volume: Volume) -> f32 {
    match volume.percent() {
        0 => SILENT_DB,
        p => (40.0 * (f32::from(p) / 100.0).log10()).max(SILENT_DB),
    }
}

/// The host's sound: the engine when a device (or the offline mixer) is open, the check
/// layer, the live mix and the sound log.
pub struct Sound<B: Backend> {
    engine: Option<Engine<B>>,
    /// Why there is no engine.
    off: Option<String>,
    manifest: PathBuf,
    buses: Vec<MixBus>,
    cues: Cues,
    layer: Layer,
    background: Background,
    audio: settings::Audio,
    watcher: Option<Watcher>,
    /// A failed reload's lines joined into one, or a runtime audio or log error.
    problem: Option<String>,
    log: Option<Box<dyn Write>>,
    /// The log's device column: `open`, `none`, or `offline` for the offline mixer.
    device: &'static str,
}

impl Sound<kira::DefaultBackend> {
    /// Opens the default output device; without one the host runs silent and keeps the reason.
    /// While the device is open, edits to the manifest folder are watched from `at`.
    pub fn open_device(
        loaded: Loaded,
        audio: settings::Audio,
        log: Option<Box<dyn Write>>,
        at: Instant,
    ) -> Self {
        let seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        let Loaded {
            manifest,
            buses,
            cues,
        } = loaded;
        let path = manifest.path().to_path_buf();
        let engine = Engine::open_device(manifest.bank, manifest.config, seed);
        let mut sound = Sound::new(engine, path, buses, cues, audio, log);
        if sound.engine.is_some() {
            sound.watch(at);
        }
        sound
    }

    /// Reports a failure the device signalled since the last call in the problem strip.
    pub fn poll_device(&mut self) {
        if let Some(engine) = &mut self.engine
            && let Err(error) = engine.poll_device_error()
        {
            self.problem = Some(error.to_string());
        }
    }
}

impl Sound<OfflineBackend> {
    /// The same mixer without a device, for `--smoke` and tests.
    pub fn offline(
        loaded: Loaded,
        audio: settings::Audio,
        sample_rate: u32,
        seed: u64,
    ) -> Result<Self, AudioError> {
        let Loaded {
            manifest,
            buses,
            cues,
        } = loaded;
        let path = manifest.path().to_path_buf();
        let engine = Engine::offline(manifest.bank, manifest.config, sample_rate, seed)?;
        let mut sound = Sound::new(Ok(engine), path, buses, cues, audio, None);
        sound.device = "offline";
        Ok(sound)
    }

    pub fn engine_mut(&mut self) -> Option<&mut Engine<OfflineBackend>> {
        self.engine.as_mut()
    }
}

impl<B: Backend> Sound<B> {
    fn new(
        engine: Result<Engine<B>, AudioError>,
        manifest: PathBuf,
        buses: Vec<MixBus>,
        cues: Cues,
        audio: settings::Audio,
        log: Option<Box<dyn Write>>,
    ) -> Self {
        let (engine, off, device) = match engine {
            Ok(engine) => (Some(engine), None, "open"),
            Err(error) => (None, Some(error.to_string()), "none"),
        };
        let mut sound = Sound {
            engine,
            off,
            manifest,
            buses,
            cues,
            layer: Layer::new(),
            background: Background::Off,
            audio,
            watcher: None,
            problem: None,
            log,
            device,
        };
        sound.apply_mix(Duration::ZERO);
        sound
    }

    /// Starts watching the manifest's folder for `.toml` and `.wav` edits.
    fn watch(&mut self, at: Instant) {
        let dir = self.manifest.parent().unwrap_or(Path::new("."));
        self.watcher = Some(Watcher::new(dir, &["toml", "wav"], WATCH_INTERVAL, at));
    }

    /// Whether an engine plays (a device, or the offline mixer).
    pub fn is_open(&self) -> bool {
        self.engine.is_some()
    }

    /// Why audio is off, when it is.
    pub fn off_reason(&self) -> Option<&str> {
        self.off.as_deref()
    }

    /// The problem strip's line, when there is a problem.
    pub fn problem(&self) -> Option<&str> {
        self.problem.as_deref()
    }

    /// Advances the play rules to `now` (time since start).
    pub fn update(&mut self, now: Duration) {
        if let Some(engine) = &mut self.engine
            && let Err(error) = engine.update(now)
        {
            self.problem = Some(error.to_string());
        }
    }

    /// Reloads when the watched files changed since the last check at `at` (checks at most
    /// every `WATCH_INTERVAL`); `true` when it reloaded.
    pub fn poll_files(&mut self, at: Instant, now: Duration) -> bool {
        let changed = self
            .watcher
            .as_mut()
            .is_some_and(|watcher| watcher.poll(at).is_some());
        if changed {
            self.reload(now);
        }
        changed
    }

    /// Loads the manifest again. A good one replaces the bank, keeps the mix and restarts the
    /// background layer; a bad one keeps the playing bank and shows its lines.
    pub fn reload(&mut self, now: Duration) {
        let Some(engine) = &mut self.engine else {
            return;
        };
        let loaded = match Loaded::load(&self.manifest) {
            Ok(loaded) => loaded,
            Err(lines) => {
                self.problem = Some(lines.replace('\n', "; "));
                return;
            }
        };
        let manifest = loaded.manifest;
        if let Err(error) = engine.replace(manifest.bank, manifest.config, RELOAD_FADE, now) {
            self.problem = Some(error.to_string());
            return;
        }
        self.buses = loaded.buses;
        self.cues = loaded.cues;
        self.problem = None;
        self.apply_mix(Duration::ZERO);
        if self.background != Background::Off {
            self.handle(Request::Background(self.background), now);
        }
    }

    /// Carries out `request` at `now`; a sound request returns its log line, also written to
    /// the sound log.
    pub fn handle(&mut self, request: Request, now: Duration) -> Option<String> {
        let (cue, outcome) = match request {
            Request::Mix(audio) => {
                self.audio = audio;
                self.apply_mix(MIX_FADE);
                return None;
            }
            Request::Test => (TEST_CUE, self.play_test(now)),
            Request::Background(background) => {
                self.background = background;
                (
                    background.cue().unwrap_or("background-off"),
                    self.set_background(now),
                )
            }
        };
        let mut line = String::new();
        let _ = write!(
            line,
            "{} ms | {cue} | {outcome} | device {}",
            now.as_millis(),
            self.device
        );
        if let Some(log) = &mut self.log
            && let Err(error) = writeln!(log, "{line}").and_then(|()| log.flush())
        {
            self.problem = Some(format!("sound log: {error}"));
            self.log = None;
        }
        Some(line)
    }

    fn play_test(&mut self, now: Duration) -> String {
        let Some(engine) = &mut self.engine else {
            return "not played: audio off".to_string();
        };
        let cue = self.cues.test;
        match engine.play(cue, now) {
            Ok(outcome) => describe(outcome, engine.bank().variations(cue)),
            Err(error) => self.fail(error),
        }
    }

    fn set_background(&mut self, now: Duration) -> String {
        let Some(engine) = &mut self.engine else {
            return "not played: audio off".to_string();
        };
        let cue = match self.background {
            Background::Off => None,
            Background::Ambience => Some(self.cues.ambience),
            Background::Music => Some(self.cues.music),
        };
        match self.layer.set(engine, cue, BACKGROUND_FADE, now) {
            Ok(Some(outcome)) => describe(outcome, cue.and_then(|c| engine.bank().variations(c))),
            Ok(None) if cue.is_some() => "already playing".to_string(),
            Ok(None) => "faded out".to_string(),
            Err(error) => self.fail(error),
        }
    }

    fn fail(&mut self, error: AudioError) -> String {
        let text = format!("error: {error}");
        self.problem = Some(error.to_string());
        text
    }

    /// Sets master, bus volumes and mute from the settings over `fade`.
    fn apply_mix(&mut self, fade: Duration) {
        let Some(engine) = &mut self.engine else {
            return;
        };
        let audio = self.audio;
        let mut result = engine.set_master_volume(percent_db(audio.master), fade);
        engine.set_muted(audio.muted, fade);
        for bus in &self.buses {
            let Some(id) = engine.bank().bus(&bus.name) else {
                continue;
            };
            let setting = match bus.part {
                Part::Music => audio.music,
                Part::Effects => audio.effects,
            };
            let db = (bus.base_db + percent_db(setting)).clamp(SILENT_DB, 24.0);
            result = result.and(engine.set_bus_volume(id, db, fade));
        }
        if let Err(error) = result {
            self.problem = Some(error.to_string());
        }
    }
}

/// A play's outcome for the log: the variation played (1-based, of how many), or why it was
/// dropped.
fn describe(outcome: PlayOutcome, variations: Option<usize>) -> String {
    match outcome {
        PlayOutcome::Played { variant, .. } => {
            format!(
                "played variation {}/{}",
                variant + 1,
                variations.unwrap_or(1)
            )
        }
        PlayOutcome::Dropped(reason) => format!(
            "dropped: {}",
            match reason {
                Dropped::Cooldown => "cooldown",
                Dropped::Instances => "instance limit",
                Dropped::Voices => "voice cap",
            }
        ),
    }
}

/// Opens the sound log for `--log-sounds`, replacing an earlier one.
pub fn open_log(path: &Path) -> io::Result<Box<dyn Write>> {
    let file = fs::File::create(path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "the sound log {} cannot be written: {error}",
                path.display()
            ),
        )
    })?;
    Ok(Box::new(io::LineWriter::new(file)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;
    use terminal_audio::{Level, Recording, synth};

    const RATE: u32 = 44_100;

    fn manifest(extra: &str) -> String {
        format!(
            "[mix]\nmax_voices = 8\n\
             [bus.ui]\nvolume_db = -6.0\n[bus.bed]\nvolume_db = -12.0\n[bus.tunes]\nvolume_db = -10.0\n\
             [cue.\"ui/confirm\"]\nbus = \"ui\"\nfiles = [\"tick.wav\"]\n\
             [cue.\"ambience/room-tone\"]\nbus = \"bed\"\nfiles = [\"tone.wav\"]\nloop = true\nfade_ms = 100\n\
             [cue.\"music/drone\"]\nbus = \"tunes\"\nfiles = [\"tone.wav\"]\nloop = true\nfade_ms = 100\n{extra}"
        )
    }

    /// A folder with a small manifest and two 0.1 s clips.
    fn folder(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("prototype-sound-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp folder");
        synth::write_wav(dir.join("tone.wav"), RATE, &vec![0.25; 4_410]).expect("wav");
        synth::write_wav(dir.join("tick.wav"), RATE, &vec![0.5; 4_410]).expect("wav");
        fs::write(dir.join("sounds.toml"), manifest("")).expect("manifest");
        dir
    }

    fn offline(dir: &Path) -> Sound<OfflineBackend> {
        let loaded = Loaded::load(&dir.join("sounds.toml")).expect("loads");
        Sound::offline(loaded, Settings::default().audio, RATE, 7).expect("offline")
    }

    fn peak(sound: &mut Sound<OfflineBackend>, from: Duration, length: Duration) -> f32 {
        let mut recording = Recording::new(RATE);
        let engine = sound.engine_mut().expect("open");
        engine.run(from, length, &mut recording).expect("renders");
        Level::of(recording.samples()).peak_dbfs()
    }

    #[test]
    fn volumes_follow_the_manifest_buses_by_cue_group() {
        let dir = folder("buses");
        let loaded = Loaded::load(&dir.join("sounds.toml")).expect("loads");
        let part = |name: &str| loaded.buses.iter().find(|b| b.name == name).map(|b| b.part);
        assert_eq!(part("tunes"), Some(Part::Music));
        assert_eq!(part("ui"), Some(Part::Effects));
        assert_eq!(part("bed"), Some(Part::Effects));
        let db = |p| percent_db(Volume::new(p).expect("percent"));
        assert_eq!(db(100), 0.0);
        assert_eq!(db(0), SILENT_DB);
        assert!(db(1) < db(50) && db(50) < db(99));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_manifest_without_a_host_cue_is_refused_with_its_line() {
        let dir = folder("missing");
        fs::write(
            dir.join("sounds.toml"),
            manifest("").replace("ui/confirm", "ui/other"),
        )
        .expect("manifest");
        let error = Loaded::load(&dir.join("sounds.toml"))
            .err()
            .expect("refused");
        assert!(error.contains("sounds.toml"), "{error}");
        assert!(error.contains("ui/confirm"), "{error}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_cue_and_background_cycle_play_offline_and_log() {
        let dir = folder("cycle");
        let mut sound = offline(&dir);
        let ms = Duration::from_millis;
        let line = sound.handle(Request::Test, ms(0)).expect("logged");
        assert_eq!(
            line,
            "0 ms | ui/confirm | played variation 1/1 | device offline"
        );
        assert!(peak(&mut sound, ms(0), ms(100)) > -30.0);
        let line = sound.handle(Request::Background(Background::Ambience), ms(100));
        assert!(
            line.expect("logged")
                .contains("ambience/room-tone | played")
        );
        assert!(peak(&mut sound, ms(100), ms(1600)) > -60.0);
        let line = sound.handle(Request::Background(Background::Off), ms(1700));
        assert!(line.expect("logged").contains("background-off | faded out"));
        // After the 1.5 s fade the layer is silent.
        peak(&mut sound, ms(1700), ms(1550));
        assert!(peak(&mut sound, ms(3250), ms(50)) < -90.0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn mute_and_zero_volume_silence_the_test_cue() {
        let dir = folder("mute");
        let mut sound = offline(&dir);
        let ms = Duration::from_millis;
        let saved = Settings::default().audio;
        let mut zero = saved;
        zero.effects = Volume::new(0).expect("percent");
        let mut muted = saved;
        muted.muted = true;
        // Each mix fades in over 50 ms; the test cue plays after it.
        for (audio, at, silent) in [(zero, 0, true), (muted, 200, true), (saved, 400, false)] {
            assert_eq!(sound.handle(Request::Mix(audio), ms(at)), None);
            peak(&mut sound, ms(at), ms(60));
            sound.handle(Request::Test, ms(at + 60));
            let peak = peak(&mut sound, ms(at + 60), ms(100));
            assert_eq!(peak < -90.0, silent, "{audio:?}: {peak} dBFS");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_bad_edit_keeps_the_bank_until_fixed() {
        let dir = folder("reload");
        let mut sound = offline(&dir);
        fs::write(dir.join("sounds.toml"), manifest("bogus = 1\n")).expect("edit");
        sound.reload(Duration::ZERO);
        let problem = sound.problem().expect("shown").to_string();
        assert!(problem.contains("sounds.toml:"), "{problem}");
        assert!(!problem.contains('\n'), "one line: {problem}");
        let line = sound.handle(Request::Test, Duration::ZERO).expect("logged");
        assert!(line.contains("played"), "the old bank plays: {line}");
        fs::write(dir.join("sounds.toml"), manifest("")).expect("fix");
        sound.reload(Duration::from_millis(10));
        assert_eq!(sound.problem(), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn without_a_device_requests_are_logged_as_not_played() {
        let dir = folder("closed");
        let loaded = Loaded::load(&dir.join("sounds.toml")).expect("loads");
        let log = dir.join("log.txt");
        let sound: Sound<OfflineBackend> = Sound::new(
            Err(AudioError::InvalidConfig("no device here")),
            dir.join("sounds.toml"),
            loaded.buses,
            loaded.cues,
            Settings::default().audio,
            Some(open_log(&log).expect("log")),
        );
        let mut sound = sound;
        assert!(!sound.is_open());
        assert_eq!(sound.off_reason(), Some("no device here"));
        sound.handle(Request::Test, Duration::from_millis(42));
        sound.handle(
            Request::Background(Background::Music),
            Duration::from_millis(50),
        );
        drop(sound);
        let text = fs::read_to_string(&log).expect("log");
        assert_eq!(
            text,
            "42 ms | ui/confirm | not played: audio off | device none\n\
             50 ms | music/drone | not played: audio off | device none\n"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
