// ADR: Colours are linear-light f32 RGB; tone mapping (pass-through below 0.6, soft shoulder above) and sRGB encoding happen once, at encoder output.
use std::ops::{Add, AddAssign, Mul};
use std::sync::LazyLock;

use ratatui::style::Color;

/// Linear-light RGB. Raster constructors require finite, nonnegative channels;
/// light values above one are allowed and tone mapped at display time.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Rgb {
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0);

    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    pub fn hex(rgb: u32) -> Self {
        Self::from_srgb8([(rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8])
    }

    pub fn from_srgb8(rgb: [u8; 3]) -> Self {
        let [r, g, b] = rgb.map(|v| srgb_to_linear(f32::from(v) / 255.0));
        Self::new(r, g, b)
    }

    pub fn luminance(self) -> f32 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }

    /// The colour `t` of the way from `self` to `other`.
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self + (other + self * -1.0) * t
    }

    pub(crate) fn valid(self) -> bool {
        [self.r, self.g, self.b]
            .into_iter()
            .all(|v| v.is_finite() && v >= 0.0)
    }
}

impl Add for Rgb {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self::new(self.r + other.r, self.g + other.g, self.b + other.b)
    }
}

impl AddAssign for Rgb {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl Mul<f32> for Rgb {
    type Output = Self;
    fn mul(self, scale: f32) -> Self {
        Self::new(self.r * scale, self.g * scale, self.b * scale)
    }
}

impl Mul for Rgb {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        Self::new(self.r * other.r, self.g * other.g, self.b * other.b)
    }
}

pub(crate) fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub(crate) fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

const KNEE: f32 = 0.6;
const LUT_SIZE: usize = 4096;
static DISPLAY_LUT: LazyLock<[f32; LUT_SIZE]> =
    LazyLock::new(|| std::array::from_fn(|i| linear_to_srgb(i as f32 / (LUT_SIZE - 1) as f32)));

fn display_channel(c: f32) -> f32 {
    let mapped = if c <= KNEE {
        c
    } else {
        KNEE + (1.0 - KNEE) * (1.0 - (-(c - KNEE) / (1.0 - KNEE)).exp())
    };
    let index = (mapped * (LUT_SIZE - 1) as f32 + 0.5) as usize;
    DISPLAY_LUT[index.min(LUT_SIZE - 1)]
}

pub(crate) fn display(c: Rgb) -> [f32; 3] {
    [
        display_channel(c.r),
        display_channel(c.g),
        display_channel(c.b),
    ]
}

pub(crate) fn to_color(srgb: [f32; 3]) -> Color {
    let [r, g, b] = srgb.map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
    Color::Rgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_and_tone_curve_preserve_order() {
        let grey = Rgb::from_srgb8([128; 3]);
        assert!((grey.r - 0.21586).abs() < 0.0001);
        assert_eq!(Rgb::hex(0xff0000), Rgb::new(1.0, 0.0, 0.0));
        assert!(display_channel(0.3) < display_channel(0.6));
        assert!(display_channel(0.6) < display_channel(1.0));
        assert!(display_channel(1.0) < display_channel(4.0));
        assert!(display_channel(f32::MAX) <= 1.0);
    }
}
