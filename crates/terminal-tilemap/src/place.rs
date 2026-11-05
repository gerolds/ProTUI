// ADR: A place is an ASCII tile grid plus a legend that maps each char to a tile definition (solid or open, surface look, optional emission and light); no tile kind is hard-coded.
// ADR: Input is checked where it enters (`Legend::insert`, `Place::new`, `Place::add_marker`): a bad legend entry, map or marker is a typed error, never repaired.
// ADR: Markers are named open tiles (entry, work points, exit) the host reads to place pins and route points; the crate gives names no meaning.
use std::fmt;

use serde::Deserialize;
use terminal_graphics::Rgb;
use terminal_graphics::noise::hash;

use crate::light::Light;

/// Bake raster pixels per tile edge.
pub const TILE_PX: usize = 16;
/// Largest place side in tiles; bounds the baked rasters (4096 × 4096 px).
pub const MAX_SIDE: usize = 256;
/// Largest light radius in tiles; bounds the shadow field a light computes.
pub const MAX_LIGHT_RADIUS: f32 = 32.0;

/// A neutral procedural pattern that modulates a surface's base colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Pattern {
    /// The base colour only.
    Flat,
    /// Large soft blotches.
    Noise,
    /// Square tiles, two per tile edge, with dark joints and varied tone.
    Tiles,
    /// Horizontal boards, four per tile edge, varied length and tone.
    Planks,
}

/// How a tile's surface looks before light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    /// sRGB base colour; the raw view draws the tile's char in it.
    pub color: [u8; 3],
    pub pattern: Pattern,
    /// Fine noise depth, 0..=1.
    pub grain: f32,
    /// Fine noise feature size in bake pixels, 0.5..=64.
    pub grain_scale: f32,
}

/// A point light at the centre of every tile of a legend entry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightDef {
    /// sRGB colour.
    pub color: [u8; 3],
    /// Reach in tiles, above 0 and at most `MAX_LIGHT_RADIUS`.
    pub radius: f32,
    /// Brightness at the source, 0..=16.
    pub intensity: f32,
    /// Flicker depth, 0..=1: 0 is steady.
    pub flicker: f32,
    /// Strength of the small glowing core drawn at the source, 0..=16; 0 draws none.
    pub glow: f32,
}

/// What one map char stands for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TileDef {
    /// Solid tiles are walls: they stop light and form the outline.
    pub solid: bool,
    pub surface: Surface,
    /// sRGB colour the surface emits regardless of light, modulated by its pattern.
    pub emissive: Option<[u8; 3]>,
    /// A light at the tile's centre; refused on a solid tile, whose walls would block it.
    pub light: Option<LightDef>,
}

/// A tile definition field outside its range: the field's dotted name and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invalid {
    pub field: &'static str,
    pub reason: String,
}

fn within(field: &'static str, value: f32, low: f32, high: f32) -> Result<(), Invalid> {
    if value.is_finite() && (low..=high).contains(&value) {
        Ok(())
    } else {
        Err(Invalid {
            field,
            reason: format!("{value} is outside {low}..={high}"),
        })
    }
}

impl TileDef {
    /// Checks every number against its range and refuses a light on a solid tile.
    pub fn check(&self) -> Result<(), Invalid> {
        within("grain", self.surface.grain, 0.0, 1.0)?;
        within("grain_scale", self.surface.grain_scale, 0.5, 64.0)?;
        let Some(light) = self.light else {
            return Ok(());
        };
        if self.solid {
            return Err(Invalid {
                field: "light",
                reason: "a solid tile cannot hold a light; its walls would block it".into(),
            });
        }
        if !(light.radius.is_finite() && light.radius > 0.0 && light.radius <= MAX_LIGHT_RADIUS) {
            return Err(Invalid {
                field: "light.radius",
                reason: format!(
                    "{} is outside 0 (exclusive)..={MAX_LIGHT_RADIUS}",
                    light.radius
                ),
            });
        }
        within("light.intensity", light.intensity, 0.0, 16.0)?;
        within("light.flicker", light.flicker, 0.0, 1.0)?;
        within("light.glow", light.glow, 0.0, 16.0)
    }
}

/// Why a legend or a place was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaceError {
    /// A legend char is not printable ASCII (space to `~`).
    Glyph { glyph: char },
    /// A legend char is defined twice.
    DuplicateGlyph { glyph: char },
    /// A legend entry has a field outside its range.
    Tile { glyph: char, invalid: Invalid },
    /// The map has no rows, or its first row is empty.
    Empty,
    /// The map is wider or taller than `MAX_SIDE` tiles.
    TooLarge { width: usize, height: usize },
    /// A row's width differs from the first row's. Rows count from 0.
    RaggedRow {
        row: usize,
        width: usize,
        expected: usize,
    },
    /// A map char has no legend entry. Rows and columns count from 0.
    UnknownChar {
        row: usize,
        column: usize,
        found: char,
    },
    /// A marker name is not a lowercase ASCII letter followed by lowercase letters, digits or `_`.
    MarkerName { name: String },
    /// A marker name is used twice.
    DuplicateMarker { name: String },
    /// A marker lies outside the map.
    MarkerOutside {
        name: String,
        at: (usize, usize),
        width: usize,
        height: usize,
    },
    /// A marker lies on a solid tile.
    MarkerOnSolid {
        name: String,
        at: (usize, usize),
        glyph: char,
    },
}

impl fmt::Display for PlaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlaceError::Glyph { glyph } => {
                write!(f, "legend char {glyph:?} is not printable ASCII")
            }
            PlaceError::DuplicateGlyph { glyph } => {
                write!(f, "legend char {glyph:?} is defined twice")
            }
            PlaceError::Tile { glyph, invalid } => {
                write!(f, "legend {glyph:?}: {}: {}", invalid.field, invalid.reason)
            }
            PlaceError::Empty => write!(f, "the map has no tiles"),
            PlaceError::TooLarge { width, height } => write!(
                f,
                "the map is {width} × {height} tiles; at most {MAX_SIDE} per side"
            ),
            PlaceError::RaggedRow {
                row,
                width,
                expected,
            } => write!(f, "map row {row} has width {width}, expected {expected}"),
            PlaceError::UnknownChar { row, column, found } => write!(
                f,
                "map row {row} column {column}: char {found:?} is not in the legend"
            ),
            PlaceError::MarkerName { name } => write!(
                f,
                "marker name {name:?} must be a lowercase letter followed by lowercase letters, digits or '_'"
            ),
            PlaceError::DuplicateMarker { name } => {
                write!(f, "marker {name:?} is defined twice")
            }
            PlaceError::MarkerOutside {
                name,
                at: (x, y),
                width,
                height,
            } => write!(
                f,
                "marker {name:?} at ({x}, {y}) is outside the {width} × {height} map"
            ),
            PlaceError::MarkerOnSolid {
                name,
                at: (x, y),
                glyph,
            } => write!(
                f,
                "marker {name:?} at ({x}, {y}) is on solid tile {glyph:?}"
            ),
        }
    }
}

impl std::error::Error for PlaceError {}

/// Tile definitions by map char, in insertion order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Legend {
    entries: Vec<(char, TileDef)>,
}

impl Legend {
    pub fn new() -> Legend {
        Legend::default()
    }

    /// Adds `def` for `glyph`. Refuses a char that is not printable ASCII, a char already defined,
    /// and a definition that fails `TileDef::check`.
    pub fn insert(&mut self, glyph: char, def: TileDef) -> Result<(), PlaceError> {
        if !(' '..='~').contains(&glyph) {
            return Err(PlaceError::Glyph { glyph });
        }
        if self.get(glyph).is_some() {
            return Err(PlaceError::DuplicateGlyph { glyph });
        }
        def.check()
            .map_err(|invalid| PlaceError::Tile { glyph, invalid })?;
        self.entries.push((glyph, def));
        Ok(())
    }

    pub fn get(&self, glyph: char) -> Option<&TileDef> {
        self.entries
            .iter()
            .find(|(g, _)| *g == glyph)
            .map(|(_, d)| d)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Entries in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (char, &TileDef)> {
        self.entries.iter().map(|(g, d)| (*g, d))
    }

    fn index(&self, glyph: char) -> Option<u8> {
        // At most 95 printable ASCII entries, so the index fits a u8.
        self.entries
            .iter()
            .position(|(g, _)| *g == glyph)
            .map(|i| i as u8)
    }

    pub(crate) fn entry(&self, index: u8) -> (char, &TileDef) {
        let (glyph, def) = &self.entries[usize::from(index)];
        (*glyph, def)
    }
}

/// A tile grid with its legend, ambient light and texture seed.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    width: usize,
    height: usize,
    tiles: Vec<u8>,
    legend: Legend,
    ambient: [u8; 3],
    seed: u32,
    markers: Vec<(String, (usize, usize))>,
}

impl Place {
    /// Builds a place from map rows separated by `\n` (or `\r\n`), one char per tile. Refuses an
    /// empty map, a side above `MAX_SIDE`, ragged rows and chars without a legend entry.
    /// `ambient` is the sRGB light every pixel receives; `seed` varies the textures and flicker.
    pub fn new(
        rows: &str,
        legend: Legend,
        ambient: [u8; 3],
        seed: u32,
    ) -> Result<Place, PlaceError> {
        let mut lines = rows.lines();
        let width = lines.next().map_or(0, |row| row.chars().count());
        if width == 0 {
            return Err(PlaceError::Empty);
        }
        let mut tiles = Vec::new();
        let mut height = 0;
        for (row, line) in rows.lines().enumerate() {
            let found = line.chars().count();
            if found != width {
                return Err(PlaceError::RaggedRow {
                    row,
                    width: found,
                    expected: width,
                });
            }
            for (column, c) in line.chars().enumerate() {
                let index = legend.index(c).ok_or(PlaceError::UnknownChar {
                    row,
                    column,
                    found: c,
                })?;
                tiles.push(index);
            }
            height += 1;
        }
        if width > MAX_SIDE || height > MAX_SIDE {
            return Err(PlaceError::TooLarge { width, height });
        }
        Ok(Place {
            width,
            height,
            tiles,
            legend,
            ambient,
            seed,
            markers: Vec::new(),
        })
    }

    /// Width in tiles.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Height in tiles.
    pub fn height(&self) -> usize {
        self.height
    }

    pub fn legend(&self) -> &Legend {
        &self.legend
    }

    pub fn seed(&self) -> u32 {
        self.seed
    }

    /// The ambient light in linear colour.
    pub fn ambient(&self) -> Rgb {
        Rgb::from_srgb8(self.ambient)
    }

    /// The legend index of the tile at `(x, y)`; `None` outside the grid.
    pub(crate) fn index(&self, x: i32, y: i32) -> Option<u8> {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return None;
        }
        Some(self.tiles[y as usize * self.width + x as usize])
    }

    /// The char and definition of the tile at `(x, y)`; `None` outside the grid.
    pub fn tile(&self, x: i32, y: i32) -> Option<(char, &TileDef)> {
        self.index(x, y).map(|i| self.legend.entry(i))
    }

    /// Whether the tile at `(x, y)` is solid; outside the grid is solid.
    pub fn solid(&self, x: i32, y: i32) -> bool {
        self.tile(x, y).is_none_or(|(_, def)| def.solid)
    }

    /// One light per tile whose legend entry has one, at the tile's centre, each with its own
    /// flicker seed; row-major order.
    pub fn lamps(&self) -> Vec<Light> {
        let mut lamps = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let (_, def) = self.legend.entry(self.tiles[y * self.width + x]);
                if let Some(light) = def.light {
                    lamps.push(Light {
                        x: x as f32 + 0.5,
                        y: y as f32 + 0.5,
                        color: Rgb::from_srgb8(light.color),
                        intensity: light.intensity,
                        radius: light.radius,
                        flicker: light.flicker,
                        glow: light.glow,
                        seed: hash(x as i32, y as i32, self.seed),
                    });
                }
            }
        }
        lamps
    }

    /// Names the open tile `at` (column, row). Refuses a bad or repeated name, a tile outside the
    /// map and a solid tile.
    pub fn add_marker(&mut self, name: &str, at: (usize, usize)) -> Result<(), PlaceError> {
        let mut chars = name.chars();
        let valid = chars.next().is_some_and(|c| c.is_ascii_lowercase())
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if !valid {
            return Err(PlaceError::MarkerName { name: name.into() });
        }
        if self.marker(name).is_some() {
            return Err(PlaceError::DuplicateMarker { name: name.into() });
        }
        if at.0 >= self.width || at.1 >= self.height {
            return Err(PlaceError::MarkerOutside {
                name: name.into(),
                at,
                width: self.width,
                height: self.height,
            });
        }
        let (glyph, def) = self.legend.entry(self.tiles[at.1 * self.width + at.0]);
        if def.solid {
            return Err(PlaceError::MarkerOnSolid {
                name: name.into(),
                at,
                glyph,
            });
        }
        self.markers.push((name.into(), at));
        Ok(())
    }

    /// The tile (column, row) a marker names.
    pub fn marker(&self, name: &str) -> Option<(usize, usize)> {
        self.markers
            .iter()
            .find(|(n, _)| n == name)
            .map(|&(_, at)| at)
    }

    /// Markers in the order they were added.
    pub fn markers(&self) -> impl Iterator<Item = (&str, (usize, usize))> {
        self.markers.iter().map(|(n, at)| (n.as_str(), *at))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn surface(color: [u8; 3]) -> Surface {
        Surface {
            color,
            pattern: Pattern::Flat,
            grain: 0.0,
            grain_scale: 2.0,
        }
    }

    pub(crate) fn def(solid: bool) -> TileDef {
        TileDef {
            solid,
            surface: surface(if solid { [90; 3] } else { [160; 3] }),
            emissive: None,
            light: None,
        }
    }

    pub(crate) fn legend() -> Legend {
        let mut legend = Legend::new();
        legend.insert('#', def(true)).unwrap();
        legend.insert('.', def(false)).unwrap();
        legend
    }

    #[test]
    fn map_errors_name_row_and_column() {
        assert_eq!(Place::new("", legend(), [0; 3], 1), Err(PlaceError::Empty));
        assert_eq!(
            Place::new("\n##", legend(), [0; 3], 1),
            Err(PlaceError::Empty)
        );
        assert_eq!(
            Place::new("###\n#.\n###", legend(), [0; 3], 1),
            Err(PlaceError::RaggedRow {
                row: 1,
                width: 2,
                expected: 3
            })
        );
        assert_eq!(
            Place::new("###\n#.x\n", legend(), [0; 3], 1),
            Err(PlaceError::UnknownChar {
                row: 1,
                column: 2,
                found: 'x'
            })
        );
        let wide = "#".repeat(MAX_SIDE + 1);
        assert_eq!(
            Place::new(&wide, legend(), [0; 3], 1),
            Err(PlaceError::TooLarge {
                width: MAX_SIDE + 1,
                height: 1
            })
        );
        let place = Place::new("###\r\n#.#\r\n###\r\n", legend(), [0; 3], 1).unwrap();
        assert_eq!((place.width(), place.height()), (3, 3));
        assert!(!place.solid(1, 1) && place.solid(0, 1) && place.solid(-1, 1));
    }

    #[test]
    fn legend_refuses_bad_glyphs_duplicates_and_ranges() {
        let mut legend = legend();
        assert_eq!(
            legend.insert('#', def(true)),
            Err(PlaceError::DuplicateGlyph { glyph: '#' })
        );
        assert_eq!(
            legend.insert('\t', def(true)),
            Err(PlaceError::Glyph { glyph: '\t' })
        );
        assert_eq!(
            legend.insert('é', def(true)),
            Err(PlaceError::Glyph { glyph: 'é' })
        );
        let light = LightDef {
            color: [255; 3],
            radius: 4.0,
            intensity: 1.0,
            flicker: 0.0,
            glow: 1.0,
        };
        let lit = |solid: bool, light: LightDef| TileDef {
            light: Some(light),
            ..def(solid)
        };
        assert!(legend.insert('L', lit(false, light)).is_ok());
        let field = |r: Result<(), PlaceError>| match r {
            Err(PlaceError::Tile { invalid, .. }) => invalid.field,
            other => panic!("{other:?}"),
        };
        assert_eq!(field(legend.insert('a', lit(true, light))), "light");
        let edge = LightDef {
            radius: MAX_LIGHT_RADIUS,
            flicker: 1.0,
            ..light
        };
        assert!(legend.insert('b', lit(false, edge)).is_ok());
        for (glyph, bad, name) in [
            (
                'c',
                LightDef {
                    radius: 0.0,
                    ..light
                },
                "light.radius",
            ),
            (
                'd',
                LightDef {
                    radius: MAX_LIGHT_RADIUS + 0.01,
                    ..light
                },
                "light.radius",
            ),
            (
                'e',
                LightDef {
                    flicker: 1.01,
                    ..light
                },
                "light.flicker",
            ),
            (
                'f',
                LightDef {
                    intensity: f32::NAN,
                    ..light
                },
                "light.intensity",
            ),
            (
                'g',
                LightDef {
                    glow: -0.1,
                    ..light
                },
                "light.glow",
            ),
        ] {
            assert_eq!(field(legend.insert(glyph, lit(false, bad))), name);
        }
        let grainy = TileDef {
            surface: Surface {
                grain: 1.5,
                ..surface([1; 3])
            },
            ..def(false)
        };
        assert_eq!(field(legend.insert('h', grainy)), "grain");
    }

    #[test]
    fn markers_must_name_open_tiles_inside_the_map() {
        let mut place = Place::new("####\n#..#\n####", legend(), [0; 3], 1).unwrap();
        place.add_marker("entry", (1, 1)).unwrap();
        place.add_marker("work_2", (2, 1)).unwrap();
        assert_eq!(place.marker("entry"), Some((1, 1)));
        assert_eq!(place.marker("exit"), None);
        let all: Vec<_> = place.markers().collect();
        assert_eq!(all, [("entry", (1, 1)), ("work_2", (2, 1))]);
        let name = |n: &str| PlaceError::MarkerName { name: n.into() };
        for bad in ["", "Entry", "2nd", "work point", "_a"] {
            assert_eq!(place.add_marker(bad, (1, 1)), Err(name(bad)));
        }
        assert_eq!(
            place.add_marker("entry", (2, 1)),
            Err(PlaceError::DuplicateMarker {
                name: "entry".into()
            })
        );
        assert_eq!(
            place.add_marker("exit", (4, 1)),
            Err(PlaceError::MarkerOutside {
                name: "exit".into(),
                at: (4, 1),
                width: 4,
                height: 3
            })
        );
        assert_eq!(
            place.add_marker("exit", (0, 1)),
            Err(PlaceError::MarkerOnSolid {
                name: "exit".into(),
                at: (0, 1),
                glyph: '#'
            })
        );
    }
}
