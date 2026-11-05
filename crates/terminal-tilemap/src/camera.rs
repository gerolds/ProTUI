// ADR: The lit camera takes the cell's aspect (height ÷ width, a caller setting) so tiles stay square on screen; the overview fits a tile rect, never below `MIN_CELLS_PER_TILE`, cropping around a focus inside the rect when it cannot.
// ADR: `Camera::raw` is the raw view's camera: one cell per tile on tile edges, ignoring aspect, the place centred when it fits, else clamped to its edges.
use std::fmt;

use ratatui::layout::Rect;

use crate::place::TILE_PX;

/// Smallest lit zoom in cells per tile edge; below it walls and patterns blur, so the view crops instead.
pub const MIN_CELLS_PER_TILE: f32 = 2.0;

/// A rectangle in tile units, `[x0, x1) × [y0, y1)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TileRect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl TileRect {
    /// The whole of a `width` × `height` tile place.
    pub fn whole(width: usize, height: usize) -> TileRect {
        TileRect {
            x0: 0.0,
            y0: 0.0,
            x1: width as f32,
            y1: height as f32,
        }
    }

    /// A `width` × `height` rect centred on `centre`, moved inside `within` where it fits.
    pub fn around_within(
        centre: (f32, f32),
        width: f32,
        height: f32,
        within: TileRect,
    ) -> TileRect {
        let place = |c: f32, size: f32, lo: f32, hi: f32| {
            let start = if size >= hi - lo {
                lo + (hi - lo - size) / 2.0
            } else {
                (c - size / 2.0).clamp(lo, hi - size)
            };
            (start, start + size)
        };
        let (x0, x1) = place(centre.0, width, within.x0, within.x1);
        let (y0, y1) = place(centre.1, height, within.y0, within.y1);
        TileRect { x0, y0, x1, y1 }
    }
}

/// Why a camera was refused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CameraError {
    /// The cell aspect is not finite and positive.
    Aspect(f32),
    /// The zoom limit is not finite or below `MIN_CELLS_PER_TILE`.
    Limit(f32),
    /// The framed rect is not finite or has no area.
    Frame(TileRect),
}

impl fmt::Display for CameraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CameraError::Aspect(a) => write!(f, "cell aspect {a} is not finite and positive"),
            CameraError::Limit(l) => write!(
                f,
                "zoom limit {l} is not finite or below {MIN_CELLS_PER_TILE} cells per tile"
            ),
            CameraError::Frame(r) => write!(f, "framed rect {r:?} is not finite or has no area"),
        }
    }
}

impl std::error::Error for CameraError {}

/// Maps view cells to world raster pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    left: f32,
    top: f32,
    cell_width: f32,
    cell_height: f32,
}

impl Camera {
    /// A camera that fits `frame` (tile units) into `area` at most at `limit` cells per tile, or
    /// keeps `focus` (tile units) in view, inside `frame`, when it cannot fit at
    /// `MIN_CELLS_PER_TILE`. `aspect` is a cell's height over its width.
    pub fn overview(
        area: Rect,
        frame: TileRect,
        focus: (f32, f32),
        limit: f32,
        aspect: f32,
    ) -> Result<Camera, CameraError> {
        if !(aspect.is_finite() && aspect > 0.0) {
            return Err(CameraError::Aspect(aspect));
        }
        if !(limit.is_finite() && limit >= MIN_CELLS_PER_TILE) {
            return Err(CameraError::Limit(limit));
        }
        let (frame_w, frame_h) = (frame.x1 - frame.x0, frame.y1 - frame.y0);
        if !(frame_w.is_finite() && frame_h.is_finite() && frame_w > 0.0 && frame_h > 0.0)
            || !(frame.x0.is_finite() && frame.y0.is_finite())
        {
            return Err(CameraError::Frame(frame));
        }
        let tile = TILE_PX as f32;
        let fit = (f32::from(area.width) / frame_w).min(aspect * f32::from(area.height) / frame_h);
        let k = fit.min(limit).max(MIN_CELLS_PER_TILE);
        let (cell_width, cell_height) = (tile / k, aspect * tile / k);
        let (view_w, view_h) = (
            f32::from(area.width) * cell_width,
            f32::from(area.height) * cell_height,
        );
        let centre = |view: f32, lo: f32, hi: f32, focus: f32| {
            if view >= hi - lo {
                (lo + hi) / 2.0
            } else {
                focus.clamp(lo + view / 2.0, hi - view / 2.0)
            }
        };
        let cx = centre(view_w, frame.x0 * tile, frame.x1 * tile, focus.0 * tile);
        let cy = centre(view_h, frame.y0 * tile, frame.y1 * tile, focus.1 * tile);
        Ok(Camera {
            left: cx - view_w / 2.0,
            top: cy - view_h / 2.0,
            cell_width,
            cell_height,
        })
    }

    /// One view cell per tile, cells on tile edges, for the raw view: a place of `tiles`
    /// (width, height) centred along each axis it fits, otherwise `centre` (tile units) as near the
    /// middle as the place's edges allow.
    pub fn raw(area: Rect, tiles: (usize, usize), centre: (f32, f32)) -> Camera {
        let tile = TILE_PX as f32;
        let first = |view: u16, size: usize, centre: f32| {
            let (view, size) = (i64::from(view), size as i64);
            let first = if view >= size {
                -((view - size) / 2)
            } else {
                ((centre - view as f32 / 2.0).round() as i64).clamp(0, size - view)
            };
            first as f32 * tile
        };
        Camera {
            left: first(area.width, tiles.0, centre.0),
            top: first(area.height, tiles.1, centre.1),
            cell_width: tile,
            cell_height: tile,
        }
    }

    /// Width and height of one view cell in raster pixels.
    pub fn cell_size(&self) -> (f32, f32) {
        (self.cell_width, self.cell_height)
    }

    /// View cells per tile edge across.
    pub fn cells_per_tile(&self) -> f32 {
        TILE_PX as f32 / self.cell_width
    }

    /// View rows per tile edge down.
    pub fn rows_per_tile(&self) -> f32 {
        TILE_PX as f32 / self.cell_height
    }

    /// The raster pixel at the middle of `area`.
    pub fn centre_px(&self, area: Rect) -> (f32, f32) {
        (
            self.left + f32::from(area.width) * self.cell_width / 2.0,
            self.top + f32::from(area.height) * self.cell_height / 2.0,
        )
    }

    /// The raster pixel at the top-left corner of a view cell.
    pub fn cell_origin(&self, col: u16, row: u16) -> (f32, f32) {
        (
            self.left + f32::from(col) * self.cell_width,
            self.top + f32::from(row) * self.cell_height,
        )
    }

    /// The raster pixel at the centre of a view cell.
    pub fn cell_centre(&self, col: u16, row: u16) -> (f32, f32) {
        let (x, y) = self.cell_origin(col, row);
        (x + self.cell_width / 2.0, y + self.cell_height / 2.0)
    }

    /// The view cell (relative to the area) holding a raster pixel, if the view shows it.
    pub fn cell_of(&self, area: Rect, px: f32, py: f32) -> Option<(u16, u16)> {
        let col = ((px - self.left) / self.cell_width).floor();
        let row = ((py - self.top) / self.cell_height).floor();
        (col >= 0.0 && row >= 0.0 && col < f32::from(area.width) && row < f32::from(area.height))
            .then_some((col as u16, row as u16))
    }

    /// The raster pixels the view covers, with a one-pixel margin, clamped to `width` × `height`,
    /// as `(x0, y0, x1, y1)`.
    pub(crate) fn region(
        &self,
        area: Rect,
        width: usize,
        height: usize,
    ) -> (usize, usize, usize, usize) {
        let clamp = |v: f32, max: usize| (v.max(0.0) as usize).min(max);
        (
            clamp(self.left.floor() - 1.0, width),
            clamp(self.top.floor() - 1.0, height),
            clamp(
                (self.left + f32::from(area.width) * self.cell_width).ceil() + 1.0,
                width,
            ),
            clamp(
                (self.top + f32::from(area.height) * self.cell_height).ceil() + 1.0,
                height,
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: TileRect = TileRect {
        x0: 0.0,
        y0: 0.0,
        x1: 10.0,
        y1: 5.0,
    };

    fn overview(area: Rect, frame: TileRect, aspect: f32) -> Camera {
        Camera::overview(area, frame, (5.0, 2.5), 8.0, aspect).unwrap()
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn the_limiting_axis_switches_at_the_boundary_aspect() {
        // 10 × 5 tiles into 40 × 10 cells: across allows 4 cells per tile, down allows 2 × aspect.
        let area = Rect::new(0, 0, 40, 10);
        let at = overview(area, FRAME, 2.0);
        assert_eq!(
            at.cells_per_tile(),
            4.0,
            "both axes fit exactly at aspect 2"
        );
        let below = overview(area, FRAME, 1.9);
        assert!(close(below.cells_per_tile(), 3.8), "height limits below 2");
        let above = overview(area, FRAME, 2.1);
        assert_eq!(above.cells_per_tile(), 4.0, "width limits above 2");
        for camera in [at, below, above] {
            let aspect = camera.cell_size().1 / camera.cell_size().0;
            assert!(close(
                camera.rows_per_tile() * aspect,
                camera.cells_per_tile()
            ));
        }
    }

    #[test]
    fn a_rect_too_large_crops_around_the_focus_inside_the_rect() {
        let area = Rect::new(0, 0, 40, 10);
        let wide = TileRect { x1: 60.0, ..FRAME };
        let left = Camera::overview(area, wide, (0.0, 2.5), 8.0, 2.0).unwrap();
        assert_eq!(left.cells_per_tile(), MIN_CELLS_PER_TILE);
        assert_eq!(
            left.cell_origin(0, 0).0,
            0.0,
            "focus at the edge clamps to the rect"
        );
        let mid = Camera::overview(area, wide, (30.0, 2.5), 8.0, 2.0).unwrap();
        assert_eq!(mid.centre_px(area).0, 30.0 * TILE_PX as f32);
        // The limit caps the zoom of a small rect, which then sits centred.
        let small = TileRect {
            x1: 2.0,
            y1: 1.0,
            ..FRAME
        };
        let capped = Camera::overview(area, small, (0.0, 0.0), 3.0, 2.0).unwrap();
        assert_eq!(capped.cells_per_tile(), 3.0);
        assert_eq!(
            capped.centre_px(area),
            (TILE_PX as f32, TILE_PX as f32 / 2.0)
        );
    }

    #[test]
    fn invalid_aspect_limit_and_frame_are_refused() {
        let area = Rect::new(0, 0, 40, 10);
        for aspect in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(matches!(
                Camera::overview(area, FRAME, (0.0, 0.0), 8.0, aspect),
                Err(CameraError::Aspect(_))
            ));
        }
        assert!(Camera::overview(area, FRAME, (0.0, 0.0), 1.9, 2.0).is_err());
        assert!(Camera::overview(area, FRAME, (0.0, 0.0), MIN_CELLS_PER_TILE, 2.0).is_ok());
        let flat = TileRect { y1: 0.0, ..FRAME };
        assert_eq!(
            Camera::overview(area, flat, (0.0, 0.0), 8.0, 2.0),
            Err(CameraError::Frame(flat))
        );
        // Extreme but valid aspects keep tiles square.
        for aspect in [0.05, 20.0] {
            let camera = overview(area, FRAME, aspect);
            assert!(close(
                camera.rows_per_tile() * aspect,
                camera.cells_per_tile()
            ));
        }
    }

    #[test]
    fn the_raw_camera_centres_a_small_place_and_clamps_a_large_one() {
        let area = Rect::new(0, 0, 20, 10);
        let small = Camera::raw(area, (10, 4), (0.0, 0.0));
        assert_eq!(small.cell_of(area, 0.5, 0.5), Some((5, 3)));
        let large = Camera::raw(area, (100, 50), (99.0, 0.0));
        assert_eq!(large.cell_origin(0, 0), (80.0 * TILE_PX as f32, 0.0));
        let rect = TileRect::around_within((99.0, 0.0), 20.0, 10.0, TileRect::whole(100, 50));
        assert_eq!(
            (rect.x0, rect.y0, rect.x1, rect.y1),
            (80.0, 0.0, 100.0, 10.0)
        );
    }
}
