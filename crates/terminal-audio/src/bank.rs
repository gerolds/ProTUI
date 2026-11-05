// ADR: A `Bank` holds every bus and cue, validated and decoded before playback; names resolve to index handles so the play path never searches strings.
// ADR: A cue owns one or more clip variations, a `Pick` rule and ± volume/pitch jitter; the engine draws them from its seeded RNG.
use crate::{AudioError, Clip};
use std::{collections::HashMap, time::Duration};

/// Most buses a bank may hold. The mixer reserves twice this, so a reload can build new buses
/// while the old ones finish their fades.
pub const MAX_BUSES: usize = 32;
/// Most clip variations one cue may hold.
pub const MAX_VARIATIONS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BusId(pub(crate) usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CueId(pub(crate) usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VoiceId(pub(crate) u64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Steal {
    #[default]
    Never,
    Oldest,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InstanceLimit {
    #[default]
    Unlimited,
    Instances {
        count: usize,
        steal: Steal,
    },
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Playback {
    #[default]
    Once,
    Loop {
        fade_in: Duration,
    },
}
/// How a play chooses one of a cue's variations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pick {
    /// Any variation, each time.
    #[default]
    Random,
    /// Every variation once in a random order, then a new order; the first of a new order is
    /// never the last of the previous one when the cue has two or more variations.
    Shuffle,
}

#[derive(Clone, Debug)]
pub struct CueOptions {
    pub bus: BusId,
    pub volume_db: f32,
    pub playback_rate: f64,
    pub playback: Playback,
    pub cooldown: Duration,
    pub instances: InstanceLimit,
    pub ducks: bool,
    pub pick: Pick,
    /// Each play shifts the volume by a uniform amount within ± this many dB (0..=24).
    pub volume_jitter_db: f32,
    /// Each play shifts the pitch (and speed) within ± this many semitones (0..=12).
    pub pitch_jitter_semitones: f32,
}
impl CueOptions {
    pub fn new(bus: BusId) -> Self {
        Self {
            bus,
            volume_db: 0.0,
            playback_rate: 1.0,
            playback: Playback::Once,
            cooldown: Duration::ZERO,
            instances: InstanceLimit::Unlimited,
            ducks: false,
            pick: Pick::Random,
            volume_jitter_db: 0.0,
            pitch_jitter_semitones: 0.0,
        }
    }
}

/// Dips one bus while any admitted ducking voice remains active.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Duck {
    pub bus: BusId,
    /// Nonpositive decibels added to the bus's current volume.
    pub by_db: f32,
    pub attack: Duration,
    pub release: Duration,
}

pub(crate) struct Cue {
    pub clips: Vec<Clip>,
    pub options: CueOptions,
}
pub(crate) struct Bus {
    pub name: String,
    pub volume_db: f32,
}

/// Build once outside playback; names resolve to compact handles before the hot path.
#[derive(Default)]
pub struct Bank {
    pub(crate) buses: Vec<Bus>,
    pub(crate) cues: Vec<Cue>,
    bus_names: HashMap<String, BusId>,
    cue_names: HashMap<String, CueId>,
}
impl Bank {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_bus(
        &mut self,
        name: impl Into<String>,
        volume_db: f32,
    ) -> Result<BusId, AudioError> {
        valid_db(volume_db)?;
        let name = name.into();
        if name.is_empty() {
            return Err(AudioError::InvalidConfig("bus name is empty"));
        }
        if self.bus_names.contains_key(&name) {
            return Err(AudioError::DuplicateName(name));
        }
        if self.buses.len() >= MAX_BUSES {
            return Err(AudioError::InvalidConfig("a bank holds at most 32 buses"));
        }
        let id = BusId(self.buses.len());
        self.buses.push(Bus {
            name: name.clone(),
            volume_db,
        });
        self.bus_names.insert(name, id);
        Ok(id)
    }

    /// Adds a cue with one or more variations (`[clip]` for a single clip).
    pub fn add_cue(
        &mut self,
        name: impl Into<String>,
        clips: impl IntoIterator<Item = Clip>,
        options: CueOptions,
    ) -> Result<CueId, AudioError> {
        valid_db(options.volume_db)?;
        if options.bus.0 >= self.buses.len() {
            return Err(AudioError::UnknownBus(options.bus));
        }
        if !options.playback_rate.is_finite() || !(0.01..=100.0).contains(&options.playback_rate) {
            return Err(AudioError::InvalidConfig(
                "playback rate must be finite and within 0.01..=100",
            ));
        }
        if let InstanceLimit::Instances { count: 0, .. } = options.instances {
            return Err(AudioError::InvalidConfig("instance limit must be positive"));
        }
        if !(0.0..=24.0).contains(&options.volume_jitter_db) {
            return Err(AudioError::InvalidConfig(
                "volume jitter must be finite and within 0..=24 dB",
            ));
        }
        if !(0.0..=12.0).contains(&options.pitch_jitter_semitones) {
            return Err(AudioError::InvalidConfig(
                "pitch jitter must be finite and within 0..=12 semitones",
            ));
        }
        let clips: Vec<Clip> = clips.into_iter().collect();
        if clips.is_empty() || clips.len() > MAX_VARIATIONS {
            return Err(AudioError::InvalidConfig(
                "a cue holds 1..=256 clip variations",
            ));
        }
        let name = name.into();
        if name.is_empty() {
            return Err(AudioError::InvalidConfig("cue name is empty"));
        }
        if self.cue_names.contains_key(&name) {
            return Err(AudioError::DuplicateName(name));
        }
        let id = CueId(self.cues.len());
        self.cues.push(Cue { clips, options });
        self.cue_names.insert(name, id);
        Ok(id)
    }

    pub fn bus(&self, name: &str) -> Option<BusId> {
        self.bus_names.get(name).copied()
    }
    pub fn cue(&self, name: &str) -> Option<CueId> {
        self.cue_names.get(name).copied()
    }
    pub fn options(&self, cue: CueId) -> Option<&CueOptions> {
        self.cues.get(cue.0).map(|c| &c.options)
    }
    /// The number of clip variations of `cue`.
    pub fn variations(&self, cue: CueId) -> Option<usize> {
        self.cues.get(cue.0).map(|c| c.clips.len())
    }
    pub fn cue_count(&self) -> usize {
        self.cues.len()
    }
    pub fn bus_count(&self) -> usize {
        self.buses.len()
    }
}

pub(crate) fn valid_db(db: f32) -> Result<(), AudioError> {
    if !db.is_finite() || !(-120.0..=24.0).contains(&db) {
        return Err(AudioError::InvalidConfig(
            "volume must be finite and within -120..=24 dB",
        ));
    }
    Ok(())
}
