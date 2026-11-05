// ADR: Each light's shadowed reach is a field raymarched on a 4 px grid with five origins for soft shadows; place lamps compute it once, dynamic lights recompute it only when they move, into reused buffers.
// ADR: Flicker varies a light's brightness and reach smoothly over caller-supplied time from its own seed; there is no per-pixel noise per frame.
use std::fmt;

use terminal_graphics::Rgb;
use terminal_graphics::noise::wave;

use crate::geometry::Geometry;
use crate::place::{MAX_LIGHT_RADIUS, TILE_PX};

/// Pixels between light grid points.
pub const STEP: usize = 4;
/// A ray that ends inside a wall may cross this much wall, so wall faces toward a light catch it.
const RIM_PX: f32 = 9.0;
/// Soft shadows: the light is sampled at its centre and four points this far out.
const SOURCE_RADIUS_PX: f32 = 3.0;
/// Fields extend past the nominal radius so flicker can stretch the reach.
const REACH_MARGIN: f32 = 1.12;
/// A dynamic light within this distance of its last computed position reuses its field.
const MOVE_EPSILON_PX: f32 = 0.25;
/// Scales every light's intensity.
const SOURCE_GAIN: f32 = 1.25;

/// A point light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    /// Position in tile units; tile centres sit at .5.
    pub x: f32,
    pub y: f32,
    /// Linear colour.
    pub color: Rgb,
    /// Brightness at the source, 0..=16.
    pub intensity: f32,
    /// Reach in tiles, above 0 and at most `MAX_LIGHT_RADIUS`; the light is zero beyond it.
    pub radius: f32,
    /// Flicker depth, 0..=1: 0 is steady.
    pub flicker: f32,
    /// Strength of the glowing core drawn at the source, 0..=16; 0 draws none.
    pub glow: f32,
    /// Seeds this light's flicker, so lights do not pulse in step.
    pub seed: u32,
}

/// Why a dynamic light was refused; `index` is its position in `Frame::dynamic`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightError {
    pub index: usize,
    pub field: &'static str,
}

impl fmt::Display for LightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "dynamic light {}: {} is not finite or outside its range",
            self.index, self.field
        )
    }
}

impl std::error::Error for LightError {}

impl Light {
    /// The first field that is not finite or lies outside its documented range.
    pub fn check(&self) -> Result<(), &'static str> {
        let colour = [self.color.r, self.color.g, self.color.b];
        let checks = [
            ("x", self.x.is_finite()),
            ("y", self.y.is_finite()),
            ("color", colour.iter().all(|c| c.is_finite() && *c >= 0.0)),
            ("intensity", (0.0..=16.0).contains(&self.intensity)),
            (
                "radius",
                self.radius > 0.0 && self.radius <= MAX_LIGHT_RADIUS,
            ),
            ("flicker", (0.0..=1.0).contains(&self.flicker)),
            ("glow", (0.0..=16.0).contains(&self.glow)),
        ];
        match checks.into_iter().find(|(_, ok)| !ok) {
            Some((field, _)) => Err(field),
            None => Ok(()),
        }
    }
}

/// A light as it shines at one moment, in raster pixels.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Source {
    pub x: f32,
    pub y: f32,
    /// Colour at the source: colour × intensity × flicker gain × `SOURCE_GAIN`, linear.
    pub color: Rgb,
    pub reach_px: f32,
    /// Flicker gain, for the glowing core.
    pub gain: f32,
}

impl Source {
    pub(crate) fn at(light: &Light, t: f32) -> Source {
        let reach_px = light.radius * TILE_PX as f32;
        let (gain, reach_px) = if light.flicker == 0.0 {
            (1.0, reach_px)
        } else {
            let n = 0.65 * wave(t * 2.3, light.seed) + 0.35 * wave(t * 6.1, light.seed + 101);
            let stretch = 1.0 + 0.3 * light.flicker * wave(t * 1.7, light.seed + 202);
            ((1.0 + light.flicker * n).max(0.0), reach_px * stretch)
        };
        Source {
            x: light.x * TILE_PX as f32,
            y: light.y * TILE_PX as f32,
            color: light.color * (light.intensity * gain * SOURCE_GAIN),
            reach_px,
            gain,
        }
    }
}

/// Light reaching `distance / reach` of the way out: bright near the source, zero at the edge.
fn falloff(x: f32) -> f32 {
    if x >= 1.0 {
        return 0.0;
    }
    let window = 1.0 - x * x;
    window * window / (1.0 + 5.0 * x * x)
}

/// 1.0 if a straight line reaches `to` from `from`, else 0.0.
fn visible(geo: &Geometry, from: (f32, f32), to: (f32, f32)) -> f32 {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length = (dx * dx + dy * dy).sqrt();
    if length < 1.0 {
        return 1.0;
    }
    let allowance = if geo.occluded(to.0.floor() as i32, to.1.floor() as i32) {
        RIM_PX
    } else {
        0.7
    };
    let (ux, uy) = (dx / length, dy / length);
    let mut d = 1.0;
    while length - d > allowance {
        let (x, y) = (from.0 + ux * d, from.1 + uy * d);
        if geo.occluded(x.floor() as i32, y.floor() as i32) {
            return 0.0;
        }
        d += 1.0;
    }
    1.0
}

/// A light's shadowed reach on the light grid: visibility and distance per grid point in its box.
#[derive(Default)]
struct Field {
    x0: usize,
    y0: usize,
    width: usize,
    height: usize,
    visibility: Vec<f32>,
    distance: Vec<f32>,
    /// Light position (px) and radius (px) the field was computed for.
    source: (f32, f32, f32),
}

impl Field {
    /// Recomputes the field in place for a light at `(lx, ly)` px reaching `radius_px`.
    fn compute(&mut self, geo: &Geometry, grid: (usize, usize), lx: f32, ly: f32, radius_px: f32) {
        let reach = radius_px * REACH_MARGIN;
        let span = |centre: f32, last: usize| {
            let low = (((centre - reach) / STEP as f32).floor().max(0.0) as usize).min(last);
            let high = (((centre + reach) / STEP as f32).ceil().max(0.0) as usize).min(last);
            (low, high.max(low))
        };
        let (x0, x1) = span(lx, grid.0 - 1);
        let (y0, y1) = span(ly, grid.1 - 1);
        let (width, height) = (x1 - x0 + 1, y1 - y0 + 1);
        let mut origins = [(0.0f32, 0.0f32); 5];
        let mut count = 0;
        for (ox, oy) in [(0.0, 0.0), (1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            let o = (lx + ox * SOURCE_RADIUS_PX, ly + oy * SOURCE_RADIUS_PX);
            if !geo.occluded(o.0.floor() as i32, o.1.floor() as i32) {
                origins[count] = o;
                count += 1;
            }
        }
        let origins = &origins[..count];
        self.visibility.clear();
        self.visibility.resize(width * height, 0.0);
        self.distance.clear();
        self.distance.resize(width * height, f32::MAX);
        for gy in 0..height {
            for gx in 0..width {
                let p = (((x0 + gx) * STEP) as f32, ((y0 + gy) * STEP) as f32);
                let d = ((p.0 - lx).powi(2) + (p.1 - ly).powi(2)).sqrt();
                if d > reach || origins.is_empty() {
                    continue;
                }
                let seen: f32 = origins.iter().map(|&o| visible(geo, o, p)).sum();
                self.visibility[gy * width + gx] = seen / origins.len() as f32;
                self.distance[gy * width + gx] = d;
            }
        }
        (self.x0, self.y0, self.width, self.height) = (x0, y0, width, height);
        self.source = (lx, ly, radius_px);
    }

    fn matches(&self, lx: f32, ly: f32, radius_px: f32) -> bool {
        let (sx, sy, sr) = self.source;
        !self.visibility.is_empty()
            && (sx - lx).abs() < MOVE_EPSILON_PX
            && (sy - ly).abs() < MOVE_EPSILON_PX
            && sr == radius_px
    }

    /// Adds this field's light to the grid.
    fn add_to(&self, color: Rgb, reach_px: f32, grid_width: usize, out: &mut [Rgb]) {
        let inverse_reach = 1.0 / reach_px;
        for gy in 0..self.height {
            let row = (self.y0 + gy) * grid_width + self.x0;
            for gx in 0..self.width {
                let i = gy * self.width + gx;
                let v = self.visibility[i];
                if v <= 0.0 {
                    continue;
                }
                let f = falloff(self.distance[i] * inverse_reach) * v;
                if f > 0.0 {
                    out[row + gx] += color * f;
                }
            }
        }
    }
}

/// A light that exists for one frame: something the host moves or switches on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DynamicLight {
    /// Identifies the light across frames so its field is reused while it stands still.
    pub key: usize,
    pub light: Light,
}

/// Fields of dynamic lights, kept across frames.
#[derive(Default)]
pub(crate) struct FieldCache {
    entries: Vec<(usize, Field)>,
}

/// The light grid of a place: its size and each lamp's fixed field.
pub(crate) struct Lighting {
    grid_width: usize,
    grid_height: usize,
    fields: Vec<Field>,
}

impl Lighting {
    pub(crate) fn build(lamps: &[Light], geo: &Geometry) -> Lighting {
        let (width, height) = geo.size();
        let (grid_width, grid_height) = (width / STEP + 1, height / STEP + 1);
        let fields = lamps
            .iter()
            .map(|lamp| {
                let mut field = Field::default();
                field.compute(
                    geo,
                    (grid_width, grid_height),
                    lamp.x * TILE_PX as f32,
                    lamp.y * TILE_PX as f32,
                    lamp.radius * TILE_PX as f32,
                );
                field
            })
            .collect();
        Lighting {
            grid_width,
            grid_height,
            fields,
        }
    }

    pub(crate) fn grid_size(&self) -> (usize, usize) {
        (self.grid_width, self.grid_height)
    }

    /// Fills `out` with `ambient` plus every lamp and dynamic light at time `t`. Returns how many
    /// dynamic fields were recomputed because their light moved or changed radius.
    pub(crate) fn accumulate(
        &self,
        geo: &Geometry,
        (lamps, ambient): (&[Light], Rgb),
        dynamic: &[DynamicLight],
        cache: &mut FieldCache,
        t: f32,
        out: &mut Vec<Rgb>,
    ) -> usize {
        out.clear();
        out.resize(self.grid_width * self.grid_height, ambient);
        for (lamp, field) in lamps.iter().zip(&self.fields) {
            let source = Source::at(lamp, t);
            field.add_to(source.color, source.reach_px, self.grid_width, out);
        }
        cache
            .entries
            .retain(|(key, _)| dynamic.iter().any(|d| d.key == *key));
        let mut recomputed = 0;
        for dynamic in dynamic {
            let light = &dynamic.light;
            let (lx, ly) = (light.x * TILE_PX as f32, light.y * TILE_PX as f32);
            let radius_px = light.radius * TILE_PX as f32;
            let index = match cache.entries.iter().position(|(k, _)| *k == dynamic.key) {
                Some(index) => index,
                None => {
                    cache.entries.push((dynamic.key, Field::default()));
                    cache.entries.len() - 1
                }
            };
            let field = &mut cache.entries[index].1;
            if !field.matches(lx, ly, radius_px) {
                field.compute(geo, (self.grid_width, self.grid_height), lx, ly, radius_px);
                recomputed += 1;
            }
            let source = Source::at(light, t);
            field.add_to(source.color, source.reach_px, self.grid_width, out);
        }
        recomputed
    }
}

/// Bilinear sample of the light grid at a pixel position.
pub(crate) fn sample(grid: &[Rgb], grid_width: usize, grid_height: usize, px: f32, py: f32) -> Rgb {
    let gx = (px / STEP as f32).clamp(0.0, (grid_width - 1) as f32);
    let gy = (py / STEP as f32).clamp(0.0, (grid_height - 1) as f32);
    let (x0, y0) = (gx.floor() as usize, gy.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(grid_width - 1), (y0 + 1).min(grid_height - 1));
    let (tx, ty) = (gx - x0 as f32, gy - y0 as f32);
    let top = grid[y0 * grid_width + x0] * (1.0 - tx) + grid[y0 * grid_width + x1] * tx;
    let bottom = grid[y1 * grid_width + x0] * (1.0 - tx) + grid[y1 * grid_width + x1] * tx;
    top * (1.0 - ty) + bottom * ty
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falloff_is_bright_at_the_source_and_zero_at_the_edge() {
        assert_eq!(falloff(0.0), 1.0);
        assert!(falloff(0.5) > falloff(0.9));
        assert_eq!(falloff(1.0), 0.0);
    }

    #[test]
    fn flicker_is_steady_at_zero_and_varies_otherwise() {
        let light = Light {
            x: 1.5,
            y: 1.5,
            color: Rgb::new(1.0, 1.0, 1.0),
            intensity: 1.0,
            radius: 4.0,
            flicker: 0.0,
            glow: 0.0,
            seed: 3,
        };
        let a = Source::at(&light, 0.0);
        let b = Source::at(&light, 7.3);
        assert_eq!((a.gain, a.reach_px), (b.gain, b.reach_px));
        let lively = Light {
            flicker: 0.5,
            ..light
        };
        let gains: Vec<f32> = (0..20)
            .map(|i| Source::at(&lively, i as f32 * 0.21).gain)
            .collect();
        assert!(gains.iter().any(|&g| (g - gains[0]).abs() > 0.01));
        assert_eq!(Source::at(&lively, 2.0).gain, Source::at(&lively, 2.0).gain);
        assert_eq!(light.check(), Ok(()));
        assert_eq!(
            Light {
                radius: 0.0,
                ..light
            }
            .check(),
            Err("radius")
        );
        assert_eq!(
            Light {
                x: f32::NAN,
                ..light
            }
            .check(),
            Err("x")
        );
        assert_eq!(
            Light {
                color: Rgb::new(-1.0, 0.0, 0.0),
                ..light
            }
            .check(),
            Err("color")
        );
    }
}
