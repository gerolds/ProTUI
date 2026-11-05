//! Top-down lit tile maps from ASCII and a data legend, encoded into octant or quadrant cells of
//! a Ratatui `Buffer`, plus the raw one-char-per-tile view. See the crate README for the place
//! TOML format, frame cost and provenance.
mod bake;
mod camera;
mod format;
mod geometry;
mod light;
mod pin;
mod place;
mod raw;
mod world;

pub use camera::{Camera, CameraError, MIN_CELLS_PER_TILE, TileRect};
pub use format::{LoadError, load_place, parse_place};
pub use geometry::Geometry;
pub use light::{DynamicLight, Light, LightError, STEP};
pub use pin::{Pin, draw_pins};
pub use place::{
    Invalid, Legend, LightDef, MAX_LIGHT_RADIUS, MAX_SIDE, Pattern, Place, PlaceError, Surface,
    TILE_PX, TileDef,
};
pub use raw::{OUTSIDE, draw_raw};
pub use world::{Frame, World};
