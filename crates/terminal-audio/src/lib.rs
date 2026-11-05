// ADR: Named cues play through Kira with play rules (cooldown, instance caps with steal, voice cap, ducking) on a caller-owned clock — the same choices with a device, offline, and in tests.
// ADR: No output device is a typed `AudioError::Device`, never a silent substitute inside the crate; the host decides to continue without sound and shows the reason.
// ADR: `replace`/`reload` swap the whole bank only after the new one is fully built; any failure leaves the playing bank untouched.
//! Reusable named-cue audio with explicit-clock policy and Kira output.
//! Decoding and bank construction happen before playback. No game events or assets are required.
mod bank;
mod layer;
mod manifest;
mod offline;
mod rng;
pub mod synth;
mod watch;
mod wav;

pub use bank::{
    Bank, BusId, CueId, CueOptions, Duck, InstanceLimit, MAX_BUSES, MAX_VARIATIONS, Pick, Playback,
    Steal, VoiceId,
};
pub use layer::Layer;
pub use manifest::{CueFiles, Manifest, ManifestError, ManifestErrors, Problem, cue_files};
pub use offline::{CONTROL_STEP, Level, OfflineBackend, Recording};
pub use rng::Rng;
pub use watch::{Change, Watcher};
pub use wav::{Channels, Clip, SAMPLE_RATES, WavError};

use kira::{
    AudioManager, AudioManagerSettings, Capacities, Decibels, DefaultBackend, PlaySoundError,
    Tween,
    backend::Backend,
    sound::static_sound::StaticSoundHandle,
    track::{TrackBuilder, TrackHandle},
};
use std::{fmt, path::Path, time::Duration};

#[derive(Debug)]
pub enum AudioError {
    InvalidConfig(&'static str),
    DuplicateName(String),
    UnknownBus(BusId),
    UnknownCue(CueId),
    UnknownVoice(VoiceId),
    ClockWentBackwards {
        previous: Duration,
        now: Duration,
    },
    ClockOverflow,
    VoiceIdExhausted,
    /// No usable output device, or the device failed.
    Device(kira::backend::cpal::Error),
    /// Kira reports that runtime error notifications were lost; failure is still observable.
    DeviceErrorsDiscarded(u64),
    TrackCapacity,
    Playback(PlaySoundError<()>),
    /// A manifest did not load, or lacks a cue the host requires.
    Manifest(ManifestErrors),
    WavWrite(hound::Error),
}
impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(s) => f.write_str(s),
            Self::DuplicateName(s) => write!(f, "duplicate audio name: {s}"),
            Self::UnknownBus(id) => write!(f, "unknown bus {id:?}"),
            Self::UnknownCue(id) => write!(f, "unknown cue {id:?}"),
            Self::UnknownVoice(id) => write!(f, "unknown or expired voice {id:?}"),
            Self::ClockWentBackwards { previous, now } => {
                write!(f, "audio clock moved backwards: {previous:?} to {now:?}")
            }
            Self::ClockOverflow => f.write_str("audio clock duration overflow"),
            Self::VoiceIdExhausted => f.write_str("voice identifiers exhausted"),
            Self::Device(e) => write!(f, "audio device: {e}"),
            Self::DeviceErrorsDiscarded(n) => {
                write!(f, "audio device lost {n} error notifications")
            }
            Self::TrackCapacity => f.write_str("Kira mixer track capacity exceeded"),
            Self::Playback(e) => write!(f, "Kira playback: {e}"),
            Self::Manifest(e) => write!(f, "{e}"),
            Self::WavWrite(e) => write!(f, "cannot write WAV: {e}"),
        }
    }
}
impl std::error::Error for AudioError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Device(e) => Some(e),
            Self::Playback(e) => Some(e),
            Self::Manifest(e) => Some(e),
            Self::WavWrite(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dropped {
    Cooldown,
    Instances,
    Voices,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayOutcome {
    Played {
        voice: VoiceId,
        stolen: Option<VoiceId>,
        /// Index of the clip variation that plays.
        variant: usize,
        /// Cue volume plus this play's jitter.
        volume_db: f32,
        /// Cue playback rate times this play's pitch jitter.
        playback_rate: f64,
    },
    Dropped(Dropped),
}
impl PlayOutcome {
    pub fn voice(self) -> Option<VoiceId> {
        match self {
            Self::Played { voice, .. } => Some(voice),
            Self::Dropped(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EngineConfig {
    pub max_voices: usize,
    pub duck: Option<Duck>,
}
impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_voices: 32,
            duck: None,
        }
    }
}

struct Voice {
    id: VoiceId,
    cue: CueId,
    start: Duration,
    end: Option<Duration>,
    handle: StaticSoundHandle,
}
struct BusState {
    track: TrackHandle,
    volume_db: f32,
    muted: bool,
}

/// IDs belong to the bank that produced them. `now` is a nondecreasing caller-owned clock.
/// Call `update` every control step, including when no cue is played, to release ducking.
/// Policy counts active voices, not fading tails. Variation picks and jitter come from an RNG
/// seeded at construction, so equal seeds and requests give equal plays.
pub struct Engine<B: Backend = DefaultBackend> {
    manager: AudioManager<B>,
    bank: Bank,
    config: EngineConfig,
    buses: Vec<BusState>,
    voices: Vec<Voice>,
    last_play: Vec<Option<Duration>>,
    /// Per cue: the shuffle order still to play (popped from the end).
    bags: Vec<Vec<u16>>,
    last_variant: Vec<Option<u16>>,
    /// Refill buffer for a shuffle order, so a failed play leaves the bag untouched.
    scratch: Vec<u16>,
    rng: Rng,
    clock: Duration,
    next: u64,
    ducked: bool,
    master_db: f32,
    muted: bool,
    discarded_errors: u64,
}

impl Engine<DefaultBackend> {
    /// Opens the real default device. Without a usable device this returns
    /// `AudioError::Device`; nothing is substituted. A host that can run without sound keeps
    /// going and shows the error's text to the player.
    pub fn open_device(bank: Bank, config: EngineConfig, seed: u64) -> Result<Self, AudioError> {
        validate(&bank, config)?;
        let settings = manager_settings(Default::default());
        let manager = AudioManager::<DefaultBackend>::new(settings).map_err(AudioError::Device)?;
        Self::build(bank, config, manager, seed)
    }

    /// Poll this regularly on desktop, including while idle. Kira may recover its stream;
    /// the original failure is nevertheless returned to the caller.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn poll_device_error(&mut self) -> Result<(), AudioError> {
        if let Some(e) = self.manager.backend_mut().pop_error() {
            return Err(AudioError::Device(e.into()));
        }
        let discarded = self
            .manager
            .backend_mut()
            .num_stream_errors_discarded()
            .unwrap_or(0);
        if discarded > self.discarded_errors {
            let delta = discarded - self.discarded_errors;
            self.discarded_errors = discarded;
            return Err(AudioError::DeviceErrorsDiscarded(delta));
        }
        Ok(())
    }
}

impl Engine<OfflineBackend> {
    pub fn offline(
        bank: Bank,
        config: EngineConfig,
        sample_rate: u32,
        seed: u64,
    ) -> Result<Self, AudioError> {
        validate(&bank, config)?;
        let settings = manager_settings(sample_rate);
        let manager = AudioManager::<OfflineBackend>::new(settings)?;
        Self::build(bank, config, manager, seed)
    }

    /// Processes real Kira mixing into interleaved stereo samples. Does not advance the
    /// policy clock: pass the buffer's start time to `update` just as in a device control loop.
    pub fn render(&mut self, stereo: &mut [f32]) -> Result<(), AudioError> {
        self.manager.backend_mut().render(stereo)
    }

    pub(crate) fn sample_rate(&mut self) -> u32 {
        self.manager.backend_mut().sample_rate()
    }
}

impl<B: Backend> Engine<B> {
    fn build(
        bank: Bank,
        config: EngineConfig,
        mut manager: AudioManager<B>,
        seed: u64,
    ) -> Result<Self, AudioError> {
        let buses = make_buses(&mut manager, &bank, config, |_| false)?;
        let cues = bank.cues.len();
        let mut engine = Self {
            manager,
            bank,
            config,
            buses,
            voices: Vec::with_capacity(config.max_voices),
            last_play: Vec::new(),
            bags: Vec::new(),
            last_variant: Vec::new(),
            scratch: Vec::with_capacity(MAX_VARIATIONS),
            rng: Rng::new(seed),
            clock: Duration::ZERO,
            next: 0,
            ducked: false,
            master_db: 0.0,
            muted: false,
            discarded_errors: 0,
        };
        engine.reset_cue_state(cues);
        Ok(engine)
    }

    fn reset_cue_state(&mut self, cues: usize) {
        self.last_play = vec![None; cues];
        self.last_variant = vec![None; cues];
        self.bags = self
            .bank
            .cues
            .iter()
            .map(|cue| match cue.options.pick {
                Pick::Shuffle => Vec::with_capacity(cue.clips.len()),
                Pick::Random => Vec::new(),
            })
            .collect();
    }

    pub fn bank(&self) -> &Bank {
        &self.bank
    }
    pub fn config(&self) -> EngineConfig {
        self.config
    }
    pub fn active_voices(&self) -> usize {
        self.voices.len()
    }
    pub fn is_active(&self, id: VoiceId) -> bool {
        self.voices.iter().any(|v| v.id == id)
    }
    pub fn is_ducking(&self) -> bool {
        self.ducked
    }

    pub fn update(&mut self, now: Duration) -> Result<(), AudioError> {
        if now < self.clock {
            return Err(AudioError::ClockWentBackwards {
                previous: self.clock,
                now,
            });
        }
        self.clock = now;
        self.voices.retain(|v| v.end.is_none_or(|end| end > now));
        self.refresh_duck();
        Ok(())
    }

    pub fn play(&mut self, cue: CueId, now: Duration) -> Result<PlayOutcome, AudioError> {
        self.play_with_fade(cue, now, None)
    }

    pub(crate) fn play_with_fade(
        &mut self,
        cue: CueId,
        now: Duration,
        fade_override: Option<Duration>,
    ) -> Result<PlayOutcome, AudioError> {
        if cue.0 >= self.bank.cues.len() {
            return Err(AudioError::UnknownCue(cue));
        }
        self.update(now)?;
        let entry = &self.bank.cues[cue.0];
        let options = &entry.options;
        if let Some(last) = self.last_play[cue.0] {
            // Subtraction is safe because the clock is monotonic; avoids Duration addition overflow.
            if now - last < options.cooldown {
                return Ok(PlayOutcome::Dropped(Dropped::Cooldown));
            }
        }
        let mut stolen_index = None;
        if let InstanceLimit::Instances { count, steal } = options.instances {
            let copies = self.voices.iter().enumerate().filter(|(_, v)| v.cue == cue);
            if copies.clone().count() >= count {
                match steal {
                    Steal::Never => return Ok(PlayOutcome::Dropped(Dropped::Instances)),
                    Steal::Oldest => {
                        stolen_index = copies
                            .min_by_key(|(_, v)| (v.start, v.id.0))
                            .map(|(i, _)| i)
                    }
                }
            }
        }
        if stolen_index.is_none() && self.voices.len() >= self.config.max_voices {
            return Ok(PlayOutcome::Dropped(Dropped::Voices));
        }
        let id = VoiceId(
            self.next
                .checked_add(1)
                .ok_or(AudioError::VoiceIdExhausted)?,
        );
        // Variation and jitter draw from a copy of the RNG, committed only with the play.
        // Draw order: variant (when several), volume (when jittered), pitch (when jittered).
        let mut rng = self.rng.clone();
        let count = entry.clips.len();
        let refill = options.pick == Pick::Shuffle && self.bags[cue.0].is_empty();
        let variant = match options.pick {
            Pick::Random if count > 1 => rng.below(count),
            Pick::Random => 0,
            Pick::Shuffle => {
                let order = if refill {
                    shuffle(&mut self.scratch, count, self.last_variant[cue.0], &mut rng);
                    &self.scratch
                } else {
                    &self.bags[cue.0]
                };
                usize::from(
                    *order
                        .last()
                        .ok_or(AudioError::InvalidConfig("shuffle order is empty"))?,
                )
            }
        };
        let volume_db = options.volume_db + jitter(&mut rng, options.volume_jitter_db);
        let semitones = jitter(&mut rng, options.pitch_jitter_semitones);
        let playback_rate = options.playback_rate * 2f64.powf(f64::from(semitones) / 12.0);
        let clip = &entry.clips[variant];
        let end = match options.playback {
            Playback::Once => Some(
                now.checked_add(clip.duration().div_f64(playback_rate))
                    .ok_or(AudioError::ClockOverflow)?,
            ),
            Playback::Loop { .. } => None,
        };
        let mut data = clip.data.volume(volume_db).playback_rate(playback_rate);
        let fade = match options.playback {
            Playback::Once => fade_override,
            Playback::Loop { fade_in } => {
                data = data.loop_region(..);
                Some(fade_override.unwrap_or(fade_in))
            }
        };
        if let Some(duration) = fade {
            data = data.fade_in_tween(Some(tween(duration)));
        }
        // Admission is committed only after the mixer accepts the sound. A failed request
        // neither steals a voice nor consumes its cooldown, RNG draws or shuffle position.
        let handle = self.buses[options.bus.0]
            .track
            .play(data)
            .map_err(AudioError::Playback)?;
        let stolen = stolen_index.map(|i| {
            let mut voice = self.voices.swap_remove(i);
            voice
                .handle
                .stop(tween(fade_override.unwrap_or(Duration::from_millis(10))));
            voice.id
        });
        self.rng = rng;
        if options.pick == Pick::Shuffle {
            let bag = &mut self.bags[cue.0];
            if refill {
                bag.clear();
                bag.extend_from_slice(&self.scratch);
            }
            bag.pop();
        }
        self.last_variant[cue.0] = u16::try_from(variant).ok();
        self.next = id.0;
        self.last_play[cue.0] = Some(now);
        self.voices.push(Voice {
            id,
            cue,
            start: now,
            end,
            handle,
        });
        self.refresh_duck();
        Ok(PlayOutcome::Played {
            voice: id,
            stolen,
            variant,
            volume_db,
            playback_rate,
        })
    }

    pub fn stop(
        &mut self,
        voice: VoiceId,
        fade: Duration,
        now: Duration,
    ) -> Result<(), AudioError> {
        self.update(now)?;
        let i = self
            .voices
            .iter()
            .position(|v| v.id == voice)
            .ok_or(AudioError::UnknownVoice(voice))?;
        self.voices.swap_remove(i).handle.stop(tween(fade));
        self.refresh_duck();
        Ok(())
    }

    pub fn stop_all(&mut self, fade: Duration, now: Duration) -> Result<(), AudioError> {
        self.update(now)?;
        for mut voice in self.voices.drain(..) {
            voice.handle.stop(tween(fade));
        }
        self.refresh_duck();
        Ok(())
    }

    /// Admits/fades in the replacement before stopping the old voice. If rejected or
    /// failed, the old voice remains. Both voices count toward admission during overlap.
    pub fn crossfade(
        &mut self,
        old: VoiceId,
        next: CueId,
        fade: Duration,
        now: Duration,
    ) -> Result<PlayOutcome, AudioError> {
        self.update(now)?;
        if !self.is_active(old) {
            return Err(AudioError::UnknownVoice(old));
        }
        let result = self.play_with_fade(next, now, Some(fade))?;
        if matches!(result, PlayOutcome::Played { .. }) && self.is_active(old) {
            self.stop(old, fade, now)?;
        }
        Ok(result)
    }

    /// Swaps in a new bank and mix. The new buses are built first; if that fails, nothing
    /// changes. On success every voice stops over `fade`, cue IDs of the old bank become
    /// invalid (resolve names again), play-rule state restarts, bus volumes come from the new
    /// bank and a bus keeps its mute when the new bank has a bus of the same name. Master
    /// volume, master mute and the RNG carry over.
    pub fn replace(
        &mut self,
        bank: Bank,
        config: EngineConfig,
        fade: Duration,
        now: Duration,
    ) -> Result<(), AudioError> {
        validate(&bank, config)?;
        self.update(now)?;
        let old = &self.bank;
        let old_buses = &self.buses;
        let buses = make_buses(&mut self.manager, &bank, config, |name| {
            old.bus(name).is_some_and(|id| old_buses[id.0].muted)
        })?;
        for mut voice in self.voices.drain(..) {
            voice.handle.stop(tween(fade));
        }
        // Dropping the old track handles removes each track once its stopped voices finish.
        self.buses = buses;
        let cues = bank.cues.len();
        self.bank = bank;
        self.config = config;
        self.voices.reserve(config.max_voices);
        self.ducked = false;
        self.reset_cue_state(cues);
        Ok(())
    }

    /// Loads `path` fully (every WAV decoded), refuses it when it lacks any `required` cue, and
    /// only then `replace`s the bank. On any error the playing bank stays and the error is
    /// returned for the host to show.
    pub fn reload(
        &mut self,
        path: impl AsRef<Path>,
        required: &[&str],
        fade: Duration,
        now: Duration,
    ) -> Result<(), AudioError> {
        let manifest = Manifest::load(path).map_err(AudioError::Manifest)?;
        manifest.require(required).map_err(AudioError::Manifest)?;
        self.replace(manifest.bank, manifest.config, fade, now)
    }

    pub fn set_master_volume(&mut self, volume_db: f32, fade: Duration) -> Result<(), AudioError> {
        bank::valid_db(volume_db)?;
        self.master_db = volume_db;
        self.apply_master(fade);
        Ok(())
    }
    pub fn set_muted(&mut self, muted: bool, fade: Duration) {
        self.muted = muted;
        self.apply_master(fade);
    }
    pub fn set_bus_volume(
        &mut self,
        bus: BusId,
        volume_db: f32,
        fade: Duration,
    ) -> Result<(), AudioError> {
        bank::valid_db(volume_db)?;
        self.buses
            .get_mut(bus.0)
            .ok_or(AudioError::UnknownBus(bus))?
            .volume_db = volume_db;
        self.apply_bus(bus, fade);
        Ok(())
    }
    pub fn set_bus_muted(
        &mut self,
        bus: BusId,
        muted: bool,
        fade: Duration,
    ) -> Result<(), AudioError> {
        self.buses
            .get_mut(bus.0)
            .ok_or(AudioError::UnknownBus(bus))?
            .muted = muted;
        self.apply_bus(bus, fade);
        Ok(())
    }
    fn apply_master(&mut self, fade: Duration) {
        let db = if self.muted {
            Decibels::SILENCE.0
        } else {
            self.master_db
        };
        self.manager.main_track().set_volume(db, tween(fade));
    }
    fn apply_bus(&mut self, id: BusId, fade: Duration) {
        let bus = &mut self.buses[id.0];
        let duck = self
            .config
            .duck
            .filter(|duck| self.ducked && duck.bus == id)
            .map_or(0.0, |duck| duck.by_db);
        bus.track.set_volume(bus_db(bus, duck), tween(fade));
    }
    fn refresh_duck(&mut self) {
        let ducked = self
            .voices
            .iter()
            .any(|v| self.bank.cues[v.cue.0].options.ducks);
        if ducked != self.ducked {
            self.ducked = ducked;
            if let Some(duck) = self.config.duck {
                self.apply_bus(duck.bus, if ducked { duck.attack } else { duck.release });
            }
        }
    }
}

/// Fills `order` with a random permutation of `0..count`, played from the end; the next play
/// is not `avoid` when there are two or more variations.
fn shuffle(order: &mut Vec<u16>, count: usize, avoid: Option<u16>, rng: &mut Rng) {
    order.clear();
    // `count` is at most MAX_VARIATIONS (256), checked by `Bank::add_cue`.
    order.extend((0..count).map(|i| i as u16));
    for i in (1..order.len()).rev() {
        order.swap(i, rng.below(i + 1));
    }
    if order.len() > 1 && order.last().copied() == avoid {
        let last = order.len() - 1;
        order.swap(0, last);
    }
}

/// A uniform value within ±`range`; draws nothing when the range is zero.
fn jitter(rng: &mut Rng, range: f32) -> f32 {
    if range == 0.0 {
        0.0
    } else {
        rng.noise() * range
    }
}

fn bus_db(bus: &BusState, duck_db: f32) -> f32 {
    if bus.muted {
        Decibels::SILENCE.0
    } else {
        bus.volume_db + duck_db
    }
}

fn make_buses<B: Backend>(
    manager: &mut AudioManager<B>,
    bank: &Bank,
    config: EngineConfig,
    muted: impl Fn(&str) -> bool,
) -> Result<Vec<BusState>, AudioError> {
    // Leave room for stopped/stealing fade tails. Exhaustion remains an explicit playback error.
    let capacity = config
        .max_voices
        .checked_mul(2)
        .and_then(|n| n.checked_add(1))
        .ok_or(AudioError::InvalidConfig("voice capacity overflows"))?;
    let mut buses = Vec::with_capacity(bank.buses.len());
    for bus in &bank.buses {
        let state_muted = muted(&bus.name);
        let volume = if state_muted {
            Decibels::SILENCE.0
        } else {
            bus.volume_db
        };
        let track = manager
            .add_sub_track(
                TrackBuilder::new()
                    .volume(volume)
                    .sound_capacity(capacity)
                    .persist_until_sounds_finish(true),
            )
            .map_err(|_| AudioError::TrackCapacity)?;
        buses.push(BusState {
            track,
            volume_db: bus.volume_db,
            muted: state_muted,
        });
    }
    Ok(buses)
}

fn validate(bank: &Bank, config: EngineConfig) -> Result<(), AudioError> {
    if config.max_voices == 0 || config.max_voices > 65_535 {
        return Err(AudioError::InvalidConfig(
            "voice cap must be within 1..=65535",
        ));
    }
    if let Some(duck) = config.duck {
        if duck.bus.0 >= bank.buses.len() {
            return Err(AudioError::UnknownBus(duck.bus));
        }
        bank::valid_db(duck.by_db)?;
        if duck.by_db > 0.0 {
            return Err(AudioError::InvalidConfig(
                "duck attenuation must be nonpositive",
            ));
        }
    }
    Ok(())
}
fn tween(duration: Duration) -> Tween {
    Tween {
        duration,
        ..Tween::default()
    }
}
fn manager_settings<B: Backend>(backend_settings: B::Settings) -> AudioManagerSettings<B> {
    AudioManagerSettings {
        capacities: Capacities {
            // Old buses persist while their voices fade after `replace`.
            sub_track_capacity: MAX_BUSES * 2,
            ..Capacities::default()
        },
        main_track_builder: Default::default(),
        internal_buffer_size: 128,
        backend_settings,
    }
}
