//! Generic linear-light rasters, PNG/ASCII still images, aspect-correct
//! sketches and Ratatui block encoding; sixel images behind feature `sixel`.
//! See the crate README for font/capture limits and provenance.
mod color;
mod encode;
mod image;
pub mod noise;
mod sampling;
pub mod sketch;

#[cfg(feature = "capture")]
pub mod capture;

#[cfg(feature = "sixel")]
pub mod sixel;

pub use color::Rgb;
pub use encode::{Encoder, encode_image};
pub use image::{ImageError, PngError, Raster, StillImage};
pub use sampling::ImageCache;
pub use sketch::{Ink, Sketch, calibration};
