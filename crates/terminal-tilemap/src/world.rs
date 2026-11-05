// ADR: `World` holds what is computed once per place (geometry, bake, lamp fields); a frame lights the grid, composites albedo × light + emissive + light cores over the camera's region into reused buffers and samples the view at sub-cell resolution; `render` hands that raster to `encode_image`, `render_raster` returns it for other outputs (sixel).
// ADR: Dynamic lights arrive per frame in `Frame::dynamic` and are checked at entry; a frame allocates nothing once its buffers have grown to the place and view size.
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use terminal_graphics::{Encoder, Raster, Rgb, encode_image};

use crate::bake::bake;
use crate::camera::Camera;
use crate::geometry::Geometry;
use crate::light::{DynamicLight, FieldCache, Light, LightError, Lighting, STEP, Source, sample};
use crate::place::{Place, TILE_PX};
use crate::raw::OUTSIDE;

/// Radius in px of a light's glowing core.
const CORE_PX: f32 = 2.5;

/// A place with everything derived from it once: geometry, albedo, emission and lamp fields.
pub struct World {
    place: Place,
    geo: Geometry,
    albedo: Vec<Rgb>,
    emissive: Vec<Rgb>,
    lamps: Vec<Light>,
    lighting: Lighting,
}

/// Per-frame buffers, reused across frames. Use one `Frame` per `World`.
#[derive(Default)]
pub struct Frame {
    /// Lights for the next `World::render` only; the host refills it each frame. A light keeps
    /// its shadow field while its key stays and it does not move.
    pub dynamic: Vec<DynamicLight>,
    light: Vec<Rgb>,
    /// The light grid interpolated to one raster row.
    row: Vec<Rgb>,
    cores: Vec<(f32, f32, Rgb)>,
    composite: Vec<Rgb>,
    view: Vec<Rgb>,
    fields: FieldCache,
    recomputed: usize,
}

impl Frame {
    /// Dynamic light fields recomputed by the last render because their light moved or is new.
    pub fn recomputed(&self) -> usize {
        self.recomputed
    }
}

impl World {
    /// Builds geometry, bake and lamp fields; this is the slow part (tens of ms in release for a
    /// screen-sized place).
    pub fn new(place: Place) -> World {
        let geo = Geometry::build(&place);
        let baked = bake(&place, &geo);
        let lamps = place.lamps();
        let lighting = Lighting::build(&lamps, &geo);
        World {
            place,
            geo,
            albedo: baked.albedo,
            emissive: baked.emissive,
            lamps,
            lighting,
        }
    }

    pub fn place(&self) -> &Place {
        &self.place
    }

    pub fn geometry(&self) -> &Geometry {
        &self.geo
    }

    /// The unlit albedo raster, row-major, `size_px` pixels.
    pub fn albedo(&self) -> &[Rgb] {
        &self.albedo
    }

    /// Raster size in pixels: tiles × `TILE_PX`.
    pub fn size_px(&self) -> (usize, usize) {
        self.geo.size()
    }

    /// The place's lamps, one per lit tile.
    pub fn lamps(&self) -> &[Light] {
        &self.lamps
    }

    /// The light (linear) on a point in tile units in the last frame rendered with `frame`;
    /// `None` before the first render or outside the place.
    pub fn light_at(&self, frame: &Frame, at: (f32, f32)) -> Option<Rgb> {
        let (gw, gh) = self.lighting.grid_size();
        let inside = at.0 >= 0.0
            && at.1 >= 0.0
            && at.0 < self.place.width() as f32
            && at.1 < self.place.height() as f32;
        (inside && frame.light.len() == gw * gh).then(|| {
            let tile = TILE_PX as f32;
            sample(&frame.light, gw, gh, at.0 * tile, at.1 * tile)
        })
    }

    /// Lights the place at time `t` (seconds; drives flicker) with its lamps and `frame.dynamic`,
    /// then encodes what `camera` shows into `area` of `buf`. Refuses a dynamic light that fails
    /// `Light::check` and then draws nothing.
    pub fn render(
        &self,
        frame: &mut Frame,
        t: f32,
        camera: &Camera,
        encoder: Encoder,
        area: Rect,
        buf: &mut Buffer,
    ) -> Result<(), LightError> {
        if area.is_empty() {
            return self.light(frame, t);
        }
        let samples = match encoder {
            Encoder::Octant => (2, 4),
            Encoder::Quadrant => (2, 2),
        };
        let raster = self.render_raster(frame, t, camera, area, samples)?;
        encode_image(raster, encoder, area, buf);
        Ok(())
    }

    /// Lights the place as `render` does and returns the linear view of `area` with
    /// `samples` (across, down) pixels per cell, borrowed from `frame`. For other raster
    /// outputs, such as sixel. `area` and `samples` must be non-zero.
    pub fn render_raster<'f>(
        &self,
        frame: &'f mut Frame,
        t: f32,
        camera: &Camera,
        area: Rect,
        (sx, sy): (usize, usize),
    ) -> Result<Raster<'f>, LightError> {
        assert!(
            !area.is_empty() && sx > 0 && sy > 0,
            "render_raster needs a non-empty area and samples"
        );
        self.light(frame, t)?;
        let (w, h) = self.size_px();
        let region = camera.region(area, w, h);
        self.composite(frame, region);
        let (vw, vh) = (usize::from(area.width) * sx, usize::from(area.height) * sy);
        frame.view.resize(vw * vh, Rgb::BLACK);
        let (cell_w, cell_h) = camera.cell_size();
        let (sample_w, sample_h) = (cell_w / sx as f32, cell_h / sy as f32);
        let (left, top) = camera.cell_origin(0, 0);
        let outside = Rgb::from_srgb8(OUTSIDE);
        for vy in 0..vh {
            let y = top + vy as f32 * sample_h;
            for vx in 0..vw {
                let x = left + vx as f32 * sample_w;
                frame.view[vy * vw + vx] = box_average(
                    &frame.composite,
                    (w, h),
                    outside,
                    (x, y),
                    (sample_w, sample_h),
                );
            }
        }
        Ok(Raster::new(&frame.view[..vw * vh], vw, vh)
            .expect("composite pixels are finite and nonnegative"))
    }

    /// Fills the light grid and the list of glowing cores.
    fn light(&self, frame: &mut Frame, t: f32) -> Result<(), LightError> {
        for (index, dynamic) in frame.dynamic.iter().enumerate() {
            dynamic
                .light
                .check()
                .map_err(|field| LightError { index, field })?;
        }
        frame.recomputed = self.lighting.accumulate(
            &self.geo,
            (&self.lamps, self.place.ambient()),
            &frame.dynamic,
            &mut frame.fields,
            t,
            &mut frame.light,
        );
        frame.cores.clear();
        let lights = self
            .lamps
            .iter()
            .chain(frame.dynamic.iter().map(|d| &d.light));
        for light in lights.filter(|l| l.glow > 0.0) {
            let source = Source::at(light, t);
            frame
                .cores
                .push((source.x, source.y, light.color * (light.glow * source.gain)));
        }
        Ok(())
    }

    /// Lit colour over `region` (x0, y0, x1, y1) of `frame.composite`. The light grid is
    /// interpolated once per raster row, then per pixel along it (bilinear, as `sample`).
    fn composite(&self, frame: &mut Frame, (x0, y0, x1, y1): (usize, usize, usize, usize)) {
        let (w, h) = self.size_px();
        frame.composite.resize(w * h, Rgb::BLACK);
        let (gw, gh) = self.lighting.grid_size();
        frame.row.resize(gw, Rgb::BLACK);
        let (gxa, gxb) = ((x0 / STEP).min(gw - 1), (x1 / STEP + 1).min(gw - 1));
        let step = STEP as f32;
        for y in y0..y1 {
            let gy = ((y as f32 + 0.5) / step).min((gh - 1) as f32);
            let ga = gy as usize;
            let gb = (ga + 1).min(gh - 1);
            let ty = gy - ga as f32;
            for gx in gxa..=gxb {
                frame.row[gx] =
                    frame.light[ga * gw + gx] * (1.0 - ty) + frame.light[gb * gw + gx] * ty;
            }
            let row = y * w;
            for x in x0..x1 {
                let gx = ((x as f32 + 0.5) / step).min((gw - 1) as f32);
                let a = gx as usize;
                let b = (a + 1).min(gw - 1);
                let tx = gx - a as f32;
                let light = frame.row[a] * (1.0 - tx) + frame.row[b] * tx;
                frame.composite[row + x] = self.albedo[row + x] * light;
            }
            if !self.emissive.is_empty() {
                for x in x0..x1 {
                    frame.composite[row + x] += self.emissive[row + x];
                }
            }
        }
        let reach = CORE_PX * 3.0;
        for &(lx, ly, color) in &frame.cores {
            let span = |c: f32, lo: usize, hi: usize| {
                let a = ((c - reach).max(0.0) as usize).clamp(lo, hi);
                let b = ((c + reach).max(0.0).ceil() as usize).clamp(lo, hi);
                (a, b)
            };
            let ((cx0, cx1), (cy0, cy1)) = (span(lx, x0, x1), span(ly, y0, y1));
            for y in cy0..cy1 {
                for x in cx0..cx1 {
                    let d2 = ((x as f32 + 0.5 - lx).powi(2) + (y as f32 + 0.5 - ly).powi(2))
                        / (CORE_PX * CORE_PX);
                    frame.composite[y * w + x] += color * (-d2).exp();
                }
            }
        }
    }
}

/// Mean colour over a raster-pixel box of a `size` raster; pixels outside it count as `outside`.
fn box_average(
    pixels: &[Rgb],
    (width, height): (usize, usize),
    outside: Rgb,
    (x, y): (f32, f32),
    (w, h): (f32, f32),
) -> Rgb {
    let span = |start: f32, size: f32| {
        let low = (start - 0.5).ceil() as i64;
        let high = (start + size - 0.5).floor() as i64;
        if low > high {
            let mid = (start + size / 2.0).floor() as i64;
            (mid, mid)
        } else {
            (low, high)
        }
    };
    let ((xa, xb), (ya, yb)) = (span(x, w), span(y, h));
    let (width, height) = (width as i64, height as i64);
    let mut sum = Rgb::BLACK;
    for py in ya..=yb {
        for px in xa..=xb {
            sum += if px < 0 || py < 0 || px >= width || py >= height {
                outside
            } else {
                pixels[(py * width + px) as usize]
            };
        }
    }
    sum * (1.0 / ((xb - xa + 1) * (yb - ya + 1)) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::TileRect;
    use crate::place::tests::def;
    use crate::place::{Legend, LightDef, TileDef};

    fn world(rows: &str) -> World {
        let mut legend = Legend::new();
        legend.insert('#', def(true)).unwrap();
        legend.insert('.', def(false)).unwrap();
        let lamp = TileDef {
            light: Some(LightDef {
                color: [255, 240, 220],
                radius: 8.0,
                intensity: 2.0,
                flicker: 0.0,
                glow: 1.0,
            }),
            ..def(false)
        };
        legend.insert('L', lamp).unwrap();
        World::new(Place::new(rows, legend, [10; 3], 1).unwrap())
    }

    fn lit(world: &World, frame: &mut Frame) {
        let area = Rect::new(0, 0, 40, 12);
        let camera = Camera::overview(
            area,
            TileRect::whole(world.place().width(), world.place().height()),
            (0.0, 0.0),
            8.0,
            2.0,
        )
        .unwrap();
        let mut buf = Buffer::empty(area);
        world
            .render(frame, 0.0, &camera, Encoder::Octant, area, &mut buf)
            .unwrap();
    }

    const WALLED: &str =
        "###########\n#....#....#\n#....#....#\n#.L..#....#\n#....#....#\n###########";
    const OPEN: &str =
        "###########\n#.........#\n#.........#\n#.L.......#\n#.........#\n###########";

    #[test]
    fn walls_block_light() {
        let ambient = Rgb::from_srgb8([10; 3]).g;
        let (walled, open) = (world(WALLED), world(OPEN));
        let (mut a, mut b) = (Frame::default(), Frame::default());
        assert_eq!(
            walled.light_at(&a, (2.5, 3.5)),
            None,
            "nothing before a render"
        );
        lit(&walled, &mut a);
        lit(&open, &mut b);
        let beyond = (7.5, 3.5);
        let shadowed = walled.light_at(&a, beyond).unwrap().g;
        let reached = open.light_at(&b, beyond).unwrap().g;
        assert!(
            (shadowed - ambient).abs() < 1e-6,
            "{shadowed} vs ambient {ambient}"
        );
        assert!(reached > ambient + 0.05, "{reached}");
        // The same side of the wall gets the same light in both.
        let near = (3.5, 2.5);
        assert_eq!(walled.light_at(&a, near), open.light_at(&b, near));
        assert!(walled.light_at(&a, (-0.1, 1.0)).is_none());
    }

    #[test]
    fn dynamic_lights_are_checked_and_reuse_their_field_until_they_move() {
        let world = world(OPEN);
        let mut frame = Frame::default();
        let light = Light {
            x: 7.5,
            y: 2.5,
            color: Rgb::new(0.2, 0.4, 1.0),
            intensity: 1.0,
            radius: 3.0,
            flicker: 0.0,
            glow: 0.0,
            seed: 9,
        };
        frame.dynamic.push(DynamicLight { key: 1, light });
        lit(&world, &mut frame);
        assert_eq!(frame.recomputed(), 1);
        lit(&world, &mut frame);
        assert_eq!(frame.recomputed(), 0);
        frame.dynamic[0].light.x += 1.0;
        lit(&world, &mut frame);
        assert_eq!(frame.recomputed(), 1);
        frame.dynamic[0].light.radius = -1.0;
        let area = Rect::new(0, 0, 4, 4);
        let camera = Camera::raw(area, (11, 6), (0.0, 0.0));
        let mut buf = Buffer::empty(area);
        assert_eq!(
            world.render(&mut frame, 0.0, &camera, Encoder::Quadrant, area, &mut buf),
            Err(LightError {
                index: 0,
                field: "radius"
            })
        );
    }

    #[test]
    fn box_average_counts_outside_pixels_as_outside() {
        let pixels = [Rgb::new(1.0, 1.0, 1.0); 4];
        let inside = box_average(&pixels, (2, 2), Rgb::BLACK, (0.0, 0.0), (2.0, 2.0));
        assert_eq!(inside, Rgb::new(1.0, 1.0, 1.0));
        let half = box_average(&pixels, (2, 2), Rgb::BLACK, (-1.0, 0.0), (2.0, 1.0));
        assert_eq!(half, Rgb::new(0.5, 0.5, 0.5));
    }
}
