// ADR: The player's settings are `<data>/settings.toml`, version 1, read and written through `store`; no file means a first start.
// ADR: Values are validated where they are read (cell aspect in whole hundredths 1.50–3.00, volumes 0–100), so a bad value is refused with its line, never clamped.
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::store::{self, Schema, StoreError};

/// The settings file: version 1, no older versions.
pub const SCHEMA: Schema = Schema {
    file: "settings.toml",
    version: 1,
    steps: &[],
};

const HEADER: &str = "# Prototype settings; the setup screen rewrites this file.\n\
# cell_aspect: a terminal cell's height over its width, 1.5 to 3.0 in steps of 0.01\n\
# renderer: octant, quadrant or ascii (ascii shows places as their ASCII source; pictures use quadrant)\n\
# audio volumes: 0 to 100; muted: true or false\n";

/// A terminal cell's height over its width, in whole hundredths within `CellAspect::RANGE`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct CellAspect(u16);

impl CellAspect {
    pub const RANGE: std::ops::RangeInclusive<u16> = 150..=300;
    /// A coarse step (←/→), in hundredths.
    pub const STEP: u16 = 5;
    /// A fine step (Shift+←/→), in hundredths.
    pub const FINE_STEP: u16 = 1;

    /// `None` outside `RANGE`.
    pub fn new(hundredths: u16) -> Option<CellAspect> {
        CellAspect::RANGE
            .contains(&hundredths)
            .then_some(CellAspect(hundredths))
    }

    pub fn hundredths(self) -> u16 {
        self.0
    }

    /// Height over width.
    pub fn get(self) -> f32 {
        f32::from(self.0) / 100.0
    }

    /// `by` hundredths up (`up`) or down, kept within `RANGE`.
    pub fn step(self, up: bool, by: u16) -> CellAspect {
        let next = if up {
            self.0.saturating_add(by)
        } else {
            self.0.saturating_sub(by)
        };
        CellAspect(next.clamp(*CellAspect::RANGE.start(), *CellAspect::RANGE.end()))
    }
}

impl TryFrom<f64> for CellAspect {
    type Error = String;

    fn try_from(value: f64) -> Result<CellAspect, String> {
        let hundredths = (value * 100.0).round();
        let whole = value.is_finite() && (value * 100.0 - hundredths).abs() < 1e-6;
        whole
            .then(|| u16::try_from(hundredths as i64).ok())
            .flatten()
            .and_then(CellAspect::new)
            .ok_or_else(|| format!("{value} is not 1.5 to 3.0 in steps of 0.01"))
    }
}

impl From<CellAspect> for f64 {
    fn from(aspect: CellAspect) -> f64 {
        f64::from(aspect.0) / 100.0
    }
}

impl fmt::Display for CellAspect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:02}", self.0 / 100, self.0 % 100)
    }
}

/// How places and pictures are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Renderer {
    /// 2×4 block cells per character (Unicode 16); in fewer fonts.
    Octant,
    /// 2×2 block cells per character; in most fonts.
    Quadrant,
    /// Places show their ASCII source view; pictures fall back to quadrant.
    Ascii,
}

impl Renderer {
    /// The order ←/→ step through.
    pub const ALL: [Renderer; 3] = [Renderer::Octant, Renderer::Quadrant, Renderer::Ascii];

    /// The next (`forward`) or previous renderer in `ALL`, wrapping.
    pub fn cycle(self, forward: bool) -> Renderer {
        let at = Renderer::ALL.iter().position(|&r| r == self).unwrap_or(0);
        let n = Renderer::ALL.len();
        Renderer::ALL[if forward {
            (at + 1) % n
        } else {
            (at + n - 1) % n
        }]
    }

    pub fn name(self) -> &'static str {
        match self {
            Renderer::Octant => "octant",
            Renderer::Quadrant => "quadrant",
            Renderer::Ascii => "ascii",
        }
    }
}

/// A volume in percent, 0 to 100.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct Volume(u8);

impl Volume {
    pub const MAX: u8 = 100;
    pub const STEP: u8 = 5;
    pub const FINE_STEP: u8 = 1;

    /// `None` above 100.
    pub fn new(percent: u8) -> Option<Volume> {
        (percent <= Volume::MAX).then_some(Volume(percent))
    }

    pub fn percent(self) -> u8 {
        self.0
    }

    /// `by` percent up (`up`) or down, kept within 0 to 100.
    pub fn step(self, up: bool, by: u8) -> Volume {
        let next = if up {
            self.0.saturating_add(by)
        } else {
            self.0.saturating_sub(by)
        };
        Volume(next.min(Volume::MAX))
    }
}

impl TryFrom<u8> for Volume {
    type Error = String;

    fn try_from(percent: u8) -> Result<Volume, String> {
        Volume::new(percent).ok_or_else(|| format!("{percent} is not 0 to 100"))
    }
}

impl From<Volume> for u8 {
    fn from(volume: Volume) -> u8 {
        volume.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Audio {
    pub master: Volume,
    pub music: Volume,
    pub effects: Volume,
    pub muted: bool,
}

/// The player's settings, as the file holds them (without `version`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub cell_aspect: CellAspect,
    pub renderer: Renderer,
    pub audio: Audio,
}

impl Default for Settings {
    /// Cheap to change: the classic 1:2 cell, octant drawing, moderate volumes, sound on.
    fn default() -> Settings {
        Settings {
            cell_aspect: CellAspect(200),
            renderer: Renderer::Octant,
            audio: Audio {
                master: Volume(80),
                music: Volume(60),
                effects: Volume(80),
                muted: false,
            },
        }
    }
}

/// Where the settings live in `dir`.
pub fn path(dir: &Path) -> PathBuf {
    SCHEMA.path(dir)
}

/// The settings in `dir`; `None` without a file: a first start.
pub fn load(dir: &Path) -> Result<Option<Settings>, StoreError> {
    store::load(dir, &SCHEMA)
}

/// Writes `settings` to `dir`; refuses to replace a file it cannot read.
pub fn save(dir: &Path, settings: &Settings) -> Result<(), StoreError> {
    store::save(dir, &SCHEMA, HEADER, settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact text of the default settings at version 1. A change to the file's keys or how
    /// they are written changes it: raise `SCHEMA.version`, add a step, then pin the new text.
    const PINNED_1: &str = "# Prototype settings; the setup screen rewrites this file.
# cell_aspect: a terminal cell's height over its width, 1.5 to 3.0 in steps of 0.01
# renderer: octant, quadrant or ascii (ascii shows places as their ASCII source; pictures use quadrant)
# audio volumes: 0 to 100; muted: true or false
version = 1
cell_aspect = 2.0
renderer = \"octant\"

[audio]
master = 80
music = 60
effects = 80
muted = false
";

    fn read(text: &str) -> Result<Settings, StoreError> {
        store::parse(Path::new("settings.toml"), text, &SCHEMA)
    }

    fn refused_key(text: &str) -> (usize, String) {
        match read(text) {
            Err(StoreError::Format { line, key, .. }) => (line, key),
            other => panic!("expected a format error, got {other:?}"),
        }
    }

    #[test]
    fn pinned_text_is_what_the_defaults_write_and_read() {
        let text = store::write(&SCHEMA, HEADER, &Settings::default()).expect("writes");
        assert_eq!(text, PINNED_1, "settings text changed: raise the version");
        assert_eq!(read(PINNED_1).expect("reads"), Settings::default());
    }

    #[test]
    fn out_of_range_values_are_refused_with_line_and_key() {
        let bad_aspect = PINNED_1.replace("cell_aspect = 2.0", "cell_aspect = 3.05");
        assert_eq!(refused_key(&bad_aspect), (6, "cell_aspect".to_string()));
        let off_step = PINNED_1.replace("cell_aspect = 2.0", "cell_aspect = 2.005");
        assert_eq!(refused_key(&off_step), (6, "cell_aspect".to_string()));
        let loud = PINNED_1.replace("music = 60", "music = 101");
        assert_eq!(refused_key(&loud), (11, "audio.music".to_string()));
        let renderer = PINNED_1.replace("\"octant\"", "\"sextant\"");
        assert_eq!(refused_key(&renderer), (7, "renderer".to_string()));
        let extra = PINNED_1.replace("muted = false", "muted = false\nbass = 3");
        assert_eq!(refused_key(&extra), (14, "audio.bass".to_string()));
        let missing = PINNED_1.replace("muted = false\n", "");
        assert_eq!(refused_key(&missing).1, "audio.muted");
        let future = PINNED_1.replace("version = 1", "version = 2");
        assert_eq!(refused_key(&future), (5, "version".to_string()));
    }

    #[test]
    fn cell_aspect_steps_stay_in_range() {
        let top = CellAspect::new(298).expect("in range");
        assert_eq!(top.step(true, CellAspect::STEP).hundredths(), 300);
        assert_eq!(top.step(true, CellAspect::FINE_STEP).hundredths(), 299);
        let bottom = CellAspect::new(152).expect("in range");
        assert_eq!(bottom.step(false, CellAspect::STEP).hundredths(), 150);
        assert_eq!(CellAspect::new(149), None);
        assert_eq!(CellAspect::new(301), None);
        assert_eq!(
            CellAspect::try_from(1.55).map(CellAspect::hundredths),
            Ok(155)
        );
        assert_eq!(CellAspect::new(205).expect("in range").to_string(), "2.05");
    }

    #[test]
    fn volume_steps_stay_in_range() {
        assert_eq!(Volume(98).step(true, Volume::STEP).percent(), 100);
        assert_eq!(Volume(3).step(false, Volume::STEP).percent(), 0);
        assert_eq!(Volume::new(101), None);
    }

    #[test]
    fn every_value_round_trips_through_a_file() {
        let dir = std::env::temp_dir().join(format!("prototype-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(load(&dir).expect("no file is fine"), None);
        let settings = Settings {
            cell_aspect: CellAspect(157),
            renderer: Renderer::Ascii,
            audio: Audio {
                master: Volume(0),
                music: Volume(100),
                effects: Volume(33),
                muted: true,
            },
        };
        save(&dir, &settings).expect("saves");
        assert_eq!(load(&dir).expect("reads"), Some(settings));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
