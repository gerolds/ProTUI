// ADR: `Sketch` paints tiny-skia shapes over a cell rectangle in design coordinates that keep their proportions on screen for a given cell aspect ("cover": centred, overflow cropped) — adapted from Runner art.rs.
// ADR: Painting runs at SUPERSAMPLE x the 2 x 4 sample grid in an sRGB pixmap; `finish` box-averages it in linear light to one `StillImage` sample per octant sub-cell, so callers cache a small raster and encode it each frame.
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::color::{linear_to_srgb, srgb_to_linear};
use crate::{ImageError, Rgb, StillImage};

/// Painted pixels per raster sample, each way.
pub const SUPERSAMPLE: u32 = 6;

/// A validated paint: a display colour (linear, each channel 0..=1) and an opacity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ink {
    rgba: [u8; 4],
}

impl Ink {
    /// Refuses channels or alpha outside 0..=1 or nonfinite. The pixmap stores
    /// sRGB bytes, so colours above 1 cannot be painted; light them afterwards.
    pub fn new(color: Rgb, alpha: f32) -> Result<Self, ImageError> {
        let unit = |v: f32| v.is_finite() && (0.0..=1.0).contains(&v);
        if ![color.r, color.g, color.b, alpha].into_iter().all(unit) {
            return Err(ImageError::InvalidInk);
        }
        let byte = |v: f32| (v * 255.0 + 0.5) as u8;
        let [r, g, b] = [color.r, color.g, color.b].map(|c| byte(linear_to_srgb(c)));
        Ok(Self {
            rgba: [r, g, b, byte(alpha)],
        })
    }

    /// An opaque ink.
    pub fn opaque(color: Rgb) -> Result<Self, ImageError> {
        Self::new(color, 1.0)
    }

    fn paint(self) -> Paint<'static> {
        let mut paint = Paint::default();
        let [r, g, b, a] = self.rgba;
        paint.set_color_rgba8(r, g, b, a);
        paint.anti_alias = true;
        paint
    }
}

/// A canvas over `columns` x `rows` cells, each `cell_aspect` times as tall as
/// wide. Design coordinates show at one scale both ways on screen: the unit
/// square until `view` sets another rect. Shapes with degenerate geometry
/// (zero size, nonfinite points, fewer than two points) draw nothing.
/// Anti-aliased edges blend in sRGB bytes, as tiny-skia does.
pub struct Sketch {
    pixmap: Pixmap,
    transform: Transform,
    /// A pixmap row's height over a column's width on screen: half the cell aspect.
    pixel_aspect: f32,
    columns: u16,
    rows: u16,
}

impl Sketch {
    /// A canvas filled with `ground`, showing the unit square centred and cropped.
    pub fn new(columns: u16, rows: u16, cell_aspect: f32, ground: Ink) -> Result<Self, ImageError> {
        if !cell_aspect.is_finite() || cell_aspect <= 0.0 {
            return Err(ImageError::InvalidAspect);
        }
        let width = 2 * u32::from(columns) * SUPERSAMPLE;
        let height = 4 * u32::from(rows) * SUPERSAMPLE;
        let mut pixmap = if columns == 0 || rows == 0 {
            return Err(ImageError::Empty);
        } else {
            Pixmap::new(width, height).ok_or(ImageError::Overflow)?
        };
        let [r, g, b, _] = ground.rgba;
        pixmap.fill(tiny_skia::Color::from_rgba8(r, g, b, 255));
        let mut sketch = Self {
            pixmap,
            transform: Transform::identity(),
            pixel_aspect: cell_aspect / 2.0,
            columns,
            rows,
        };
        sketch.view(0.0, 0.0, 1.0, 1.0)?;
        Ok(sketch)
    }

    /// The canvas width over its height as it shows on screen.
    pub fn shape(&self) -> f32 {
        self.pixmap.width() as f32 / (self.pixmap.height() as f32 * self.pixel_aspect)
    }

    /// Shows design coordinates `left..right` across and `top..bottom` down at
    /// one scale both ways on screen. The rect covers the canvas, centred, and
    /// what overflows is cropped; a rect of the canvas `shape` fills it exactly.
    pub fn view(&mut self, left: f32, top: f32, right: f32, bottom: f32) -> Result<(), ImageError> {
        let (dw, dh) = (right - left, bottom - top);
        if ![left, top, dw, dh].into_iter().all(f32::is_finite) || dw <= 0.0 || dh <= 0.0 {
            return Err(ImageError::InvalidView);
        }
        let (w, h) = (self.pixmap.width() as f32, self.pixmap.height() as f32);
        let r = self.pixel_aspect;
        // Screen units are pixmap column widths; the pixmap is `h * r` of them tall.
        let scale = (w / dw).max(h * r / dh);
        let (ox, oy) = ((w - dw * scale) / 2.0, (h * r - dh * scale) / 2.0);
        self.transform = Transform::from_row(
            scale,
            0.0,
            0.0,
            scale / r,
            ox - left * scale,
            (oy - top * scale) / r,
        );
        Ok(())
    }

    fn fill(&mut self, path: &tiny_skia::Path, ink: Ink) {
        self.pixmap
            .fill_path(path, &ink.paint(), FillRule::Winding, self.transform, None);
    }

    fn stroke(&mut self, path: &tiny_skia::Path, width: f32, ink: Ink) {
        let stroke = Stroke {
            width,
            line_cap: tiny_skia::LineCap::Round,
            line_join: tiny_skia::LineJoin::Round,
            ..Stroke::default()
        };
        self.pixmap
            .stroke_path(path, &ink.paint(), &stroke, self.transform, None);
    }

    fn oval_path(cx: f32, cy: f32, rx: f32, ry: f32) -> Option<tiny_skia::Path> {
        tiny_skia::Rect::from_ltrb(cx - rx, cy - ry, cx + rx, cy + ry)
            .and_then(PathBuilder::from_oval)
    }

    fn polygon_path(points: &[(f32, f32)]) -> Option<tiny_skia::Path> {
        let (first, rest) = points.split_first()?;
        let mut pb = PathBuilder::new();
        pb.move_to(first.0, first.1);
        for p in rest {
            pb.line_to(p.0, p.1);
        }
        pb.close();
        pb.finish()
    }

    /// Fills an ellipse centred on `cx, cy` with radii `rx, ry`.
    pub fn oval(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, ink: Ink) {
        if let Some(path) = Self::oval_path(cx, cy, rx, ry) {
            self.fill(&path, ink);
        }
    }

    /// Strokes an ellipse outline `width` design units wide.
    pub fn stroke_oval(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, width: f32, ink: Ink) {
        if let Some(path) = Self::oval_path(cx, cy, rx, ry) {
            self.stroke(&path, width, ink);
        }
    }

    /// Fills the rect from `x0, y0` to `x1, y1`.
    pub fn rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, ink: Ink) {
        if let Some(rect) = tiny_skia::Rect::from_ltrb(x0, y0, x1, y1) {
            self.fill(&PathBuilder::from_rect(rect), ink);
        }
    }

    /// Fills the closed polygon through `points` (nonzero winding).
    pub fn polygon(&mut self, points: &[(f32, f32)], ink: Ink) {
        if let Some(path) = Self::polygon_path(points) {
            self.fill(&path, ink);
        }
    }

    /// Strokes the closed polygon through `points` with round joins.
    pub fn stroke_polygon(&mut self, points: &[(f32, f32)], width: f32, ink: Ink) {
        if let Some(path) = Self::polygon_path(points) {
            self.stroke(&path, width, ink);
        }
    }

    /// A segment from `a` to `b`, `width` design units wide, with round caps.
    pub fn line(&mut self, a: (f32, f32), b: (f32, f32), width: f32, ink: Ink) {
        let mut pb = PathBuilder::new();
        pb.move_to(a.0, a.1);
        pb.line_to(b.0, b.1);
        if let Some(path) = pb.finish() {
            self.stroke(&path, width, ink);
        }
    }

    /// The picture as linear light at 2 x 4 samples per cell: each sample is the
    /// mean of its SUPERSAMPLE x SUPERSAMPLE painted pixels.
    pub fn finish(&self) -> StillImage {
        let (width, height) = (2 * usize::from(self.columns), 4 * usize::from(self.rows));
        let stride = self.pixmap.width() as usize;
        let ss = SUPERSAMPLE as usize;
        let mut lut = [0.0f32; 256];
        for (i, v) in lut.iter_mut().enumerate() {
            *v = srgb_to_linear(i as f32 / 255.0);
        }
        let pixels = self.pixmap.pixels();
        let mut out = Vec::with_capacity(width * height);
        let weight = 1.0 / (ss * ss) as f32;
        for y in 0..height {
            for x in 0..width {
                let mut sum = Rgb::BLACK;
                for py in y * ss..(y + 1) * ss {
                    for p in &pixels[py * stride + x * ss..py * stride + (x + 1) * ss] {
                        // The pixmap is opaque, so premultiplied bytes are the colour.
                        sum += Rgb::new(
                            lut[usize::from(p.red())],
                            lut[usize::from(p.green())],
                            lut[usize::from(p.blue())],
                        );
                    }
                }
                out.push(sum * weight);
            }
        }
        StillImage::new(out, width, height).expect("a sketch has 2 x 4 finite samples per cell")
    }
}

/// The aspect guide: a circle and a square of one size side by side, as outlines
/// in `ink` on `ground`. They look round and square only when `cell_aspect`
/// matches the terminal font's cell height over width.
pub fn calibration(
    columns: u16,
    rows: u16,
    cell_aspect: f32,
    ground: Ink,
    ink: Ink,
) -> Result<StillImage, ImageError> {
    let mut s = Sketch::new(columns, rows, cell_aspect, ground)?;
    // A design rect of the canvas shape fills it, so nothing is cropped.
    let shape = s.shape();
    s.view(0.0, 0.0, shape, 1.0)?;
    let side = 0.8_f32.min(shape * 0.4);
    let (cy, gap) = (0.5, side * 0.15);
    let circle_x = shape / 2.0 - side / 2.0 - gap;
    let square_x = shape / 2.0 + side / 2.0 + gap;
    let (half, width) = (side / 2.0, side * 0.1);
    s.stroke_oval(
        circle_x,
        cy,
        half - width / 2.0,
        half - width / 2.0,
        width,
        ink,
    );
    let inset = half - width / 2.0;
    let corners = [
        (square_x - inset, cy - inset),
        (square_x + inset, cy - inset),
        (square_x + inset, cy + inset),
        (square_x - inset, cy + inset),
    ];
    s.stroke_polygon(&corners, width, ink);
    // A centre cross in each makes a squashed shape easier to judge.
    for x in [circle_x, square_x] {
        s.line((x - half * 0.3, cy), (x + half * 0.3, cy), width * 0.5, ink);
        s.line((x, cy - half * 0.3), (x, cy + half * 0.3), width * 0.5, ink);
    }
    Ok(s.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The lit extent of `image` in samples across and down.
    fn extent(image: &StillImage, columns: std::ops::Range<usize>) -> (usize, usize) {
        let raster = image.raster();
        let (width, _) = raster.size();
        let lit: Vec<_> = raster
            .pixels()
            .iter()
            .enumerate()
            .map(|(i, p)| (i % width, i / width, p))
            .filter(|&(x, _, p)| columns.contains(&x) && p.luminance() > 0.2)
            .map(|(x, y, _)| (x, y))
            .collect();
        let span = |pick: fn(&(usize, usize)) -> usize| {
            lit.iter().map(pick).max().unwrap() - lit.iter().map(pick).min().unwrap() + 1
        };
        (span(|p| p.0), span(|p| p.1))
    }

    fn black() -> Ink {
        Ink::opaque(Rgb::BLACK).unwrap()
    }
    fn white() -> Ink {
        Ink::opaque(Rgb::WHITE).unwrap()
    }

    #[test]
    fn a_circle_stays_round_on_any_cell_aspect() {
        for aspect in [2.0, 18.0 / 7.0, 1.6] {
            // A box wide on screen and one tall on screen.
            for (columns, rows) in [(40u16, 8u16), (10, 14)] {
                for fitted in [false, true] {
                    let mut s = Sketch::new(columns, rows, aspect, black()).unwrap();
                    let (cx, r) = if fitted {
                        let shape = s.shape();
                        s.view(0.0, 0.0, shape, 1.0).unwrap();
                        (shape / 2.0, 0.3_f32.min(shape * 0.3))
                    } else {
                        // The unit square covers the box, cropped; a small
                        // circle in its middle shows whole.
                        (0.5, 0.1)
                    };
                    s.oval(cx, 0.5, r, r, white());
                    let image = s.finish();
                    let (across, down) = extent(&image, 0..usize::MAX);
                    // On screen a sample row is aspect / 2 sample columns tall.
                    let ratio = across as f32 / (down as f32 * aspect / 2.0);
                    assert!(
                        (ratio - 1.0).abs() < 0.12,
                        "{aspect} {columns}x{rows} fitted {fitted}: {across} x {down}"
                    );
                }
            }
        }
    }

    #[test]
    fn calibration_shapes_are_true_only_at_the_painted_aspect() {
        let (columns, rows) = (40u16, 12u16);
        let painted = 18.0 / 7.0;
        let image = calibration(columns, rows, painted, black(), white()).unwrap();
        assert_eq!(image.raster().size(), (80, 48));
        let half = usize::from(columns);
        for side in [0..half, half..2 * half] {
            let (across, down) = extent(&image, side.clone());
            let at = |aspect: f32| across as f32 / (down as f32 * aspect / 2.0);
            assert!((at(painted) - 1.0).abs() < 0.1, "{side:?}: {}", at(painted));
            // Shown on 2.0 cells, the same raster looks clearly squashed.
            assert!(at(2.0) > 1.2, "{side:?}: {}", at(2.0));
        }
    }

    #[test]
    fn invalid_inputs_are_refused() {
        assert_eq!(
            Ink::new(Rgb::new(1.5, 0.0, 0.0), 1.0),
            Err(ImageError::InvalidInk)
        );
        assert_eq!(Ink::new(Rgb::BLACK, -0.1), Err(ImageError::InvalidInk));
        assert_eq!(Ink::new(Rgb::BLACK, f32::NAN), Err(ImageError::InvalidInk));
        assert_eq!(
            Sketch::new(0, 3, 2.0, black()).err(),
            Some(ImageError::Empty)
        );
        assert_eq!(
            Sketch::new(3, 3, 0.0, black()).err(),
            Some(ImageError::InvalidAspect)
        );
        let mut s = Sketch::new(3, 3, 2.0, black()).unwrap();
        assert_eq!(s.view(1.0, 0.0, 1.0, 1.0), Err(ImageError::InvalidView));
        assert_eq!(
            s.view(0.0, 0.0, f32::INFINITY, 1.0),
            Err(ImageError::InvalidView)
        );
    }

    #[test]
    fn finish_averages_in_linear_light() {
        // Left half white, right half black over one cell: each sample column is
        // pure, so the result is exactly white/black, not a gamma-blended grey.
        let mut s = Sketch::new(1, 1, 2.0, black()).unwrap();
        s.view(0.0, 0.0, 2.0, 2.0).unwrap();
        s.rect(0.0, 0.0, 1.0, 2.0, white());
        let image = s.finish();
        let p = image.raster().pixels();
        assert_eq!((p[0], p[1]), (Rgb::WHITE, Rgb::BLACK));
        // Half-transparent white over black is sRGB 128, linear about 0.216.
        let mut s = Sketch::new(1, 1, 2.0, black()).unwrap();
        s.rect(-1.0, -1.0, 2.0, 2.0, Ink::new(Rgb::WHITE, 0.5).unwrap());
        let r = s.finish().raster().pixels()[0].r;
        assert!((r - Rgb::from_srgb8([128; 3]).r).abs() < 0.01, "{r}");
    }
}
