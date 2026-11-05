// ADR: Ratatui keeps every cell, layout and focus; `SixelLayer` only marks an image's cells `CellDiffOption::Skip` during render and writes the queued sixel images after the text diff, all inside one synchronized update (DEC mode 2026).
// ADR: An image is re-sent only when its id, revision or rect changed or the screen was cleared (area change, `invalidate`); cells an image left get `CellDiffOption::AlwaysUpdate` once, because Windows Terminal erases image pixels in a cell when text is written to it.
// ADR: Windows Terminal scrolls the whole screen when a sixel band crosses the bottom row, so an image is cropped to whole bands that fit above the bottom edge.
use std::fmt;
use std::io::{self, Write};

use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::buffer::{Buffer, CellDiffOption};
use ratatui::layout::Rect;

use super::{Palette, SixelEncoder, SixelError, SixelOptions};
use crate::Raster;

const BEGIN_SYNC: &[u8] = b"\x1b[?2026h";
const END_SYNC: &[u8] = b"\x1b[?2026l";

/// Why an image was not placed. Nothing was marked or queued.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaceError {
    /// The rect is empty or not inside the frame area.
    Outside(Rect),
    /// The raster does not divide the rect's device pixels into whole pixels.
    Scale {
        raster: (usize, usize),
        device: (usize, usize),
    },
    /// The id was already placed in this frame.
    Duplicate(u64),
    Encode(SixelError),
}

impl fmt::Display for PlaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Outside(rect) => write!(f, "sixel rect {rect:?} is empty or outside the frame"),
            Self::Scale { raster, device } => write!(
                f,
                "raster {}x{} does not divide the rect's {}x{} device pixels",
                raster.0, raster.1, device.0, device.1
            ),
            Self::Duplicate(id) => write!(f, "sixel id {id} placed twice in one frame"),
            Self::Encode(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for PlaceError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Placed {
    id: u64,
    revision: u64,
    rect: Rect,
}

/// Sixel images in chosen cell rects of a Ratatui frame. Use one per terminal.
pub struct SixelLayer {
    cell_px: (u16, u16),
    encoder: SixelEncoder,
    pending: Vec<u8>,
    shown: Vec<Placed>,
    placed: Vec<Placed>,
    area: Rect,
    stale: bool,
}

impl SixelLayer {
    /// `cell_px` is the terminal's sixel cell size in device pixels: `WT_CELL_PX` in Windows
    /// Terminal; elsewhere the answer to `CSI 16 t`.
    pub fn new(cell_px: (u16, u16)) -> Self {
        Self {
            cell_px,
            encoder: SixelEncoder::new(),
            pending: Vec::new(),
            shown: Vec::new(),
            placed: Vec::new(),
            area: Rect::default(),
            stale: true,
        }
    }

    /// Re-send every image next frame. Call after `Terminal::clear` or anything else that
    /// erased the screen; a resize is detected by itself.
    pub fn invalidate(&mut self) {
        self.stale = true;
    }

    /// Bytes queued by the last frame: cursor moves and sixel images.
    pub fn pending(&self) -> &[u8] {
        &self.pending
    }

    /// Colours defined by the last image encoded.
    pub fn colors(&self) -> usize {
        self.encoder.colors()
    }

    /// Renders one frame: `render` draws the cells and calls `place`; then the text diff and
    /// the images go out in one synchronized update.
    pub fn draw<B>(
        &mut self,
        terminal: &mut Terminal<B>,
        render: impl FnOnce(&mut Frame<'_>, &mut Self),
    ) -> io::Result<()>
    where
        B: Backend<Error = io::Error> + Write,
    {
        terminal.backend_mut().write_all(BEGIN_SYNC)?;
        let drawn = terminal
            .draw(|frame| {
                self.begin(frame.area());
                render(frame, self);
                self.end(frame.buffer_mut());
            })
            .map(|_| ());
        let backend = terminal.backend_mut();
        let written = drawn.and_then(|()| backend.write_all(&self.pending));
        if written.is_err() {
            // The screen no longer matches what the layer believes is on it.
            self.stale = true;
        }
        let ended = backend
            .write_all(END_SYNC)
            .and_then(|()| Write::flush(backend));
        written.and(ended)
    }

    /// Starts a frame on the terminal area `area`. A changed area means the terminal was resized
    /// (and cleared), so every image is re-sent.
    pub fn begin(&mut self, area: Rect) {
        if area != self.area {
            self.area = area;
            self.stale = true;
        }
        self.pending.clear();
        self.placed.clear();
    }

    /// Shows `raster` in `rect` this frame: marks the cells skipped in `buf` and queues the
    /// image unless the same `id` and `revision` already show in that rect. The raster must
    /// divide the rect's device pixels (cells × `cell_px`) into whole pixels per axis; each
    /// raster pixel then covers that many device pixels.
    pub fn place(
        &mut self,
        buf: &mut Buffer,
        id: u64,
        revision: u64,
        rect: Rect,
        raster: Raster<'_>,
        palette: Palette,
    ) -> Result<(), PlaceError> {
        if rect.is_empty() || rect.intersection(self.area) != rect {
            return Err(PlaceError::Outside(rect));
        }
        if self.placed.iter().any(|p| p.id == id) {
            return Err(PlaceError::Duplicate(id));
        }
        let (w, h) = raster.size();
        let device = (
            usize::from(rect.width) * usize::from(self.cell_px.0),
            usize::from(rect.height) * usize::from(self.cell_px.1),
        );
        if device.0 % w != 0 || device.1 % h != 0 {
            return Err(PlaceError::Scale {
                raster: (w, h),
                device,
            });
        }
        let scale = (device.0 / w, device.1 / h);
        let options = SixelOptions {
            palette,
            scale: (
                u16::try_from(scale.0).unwrap_or(u16::MAX),
                u16::try_from(scale.1).unwrap_or(u16::MAX),
            ),
        };
        let placed = Placed { id, revision, rect };
        if self.stale || !self.shown.contains(&placed) {
            // Whole sixel bands (6 rows) that end above the screen's bottom edge.
            let room = usize::from(self.area.bottom() - rect.y) * usize::from(self.cell_px.1);
            let rows = h.min(room / (6 * scale.1) * 6);
            let mark = self.pending.len();
            self.pending.extend_from_slice(b"\x1b[");
            super::push_num(&mut self.pending, usize::from(rect.y) + 1);
            self.pending.push(b';');
            super::push_num(&mut self.pending, usize::from(rect.x) + 1);
            self.pending.push(b'H');
            if rows > 0
                && let Err(e) =
                    self.encoder
                        .encode(raster.top_rows(rows), &options, &mut self.pending)
            {
                self.pending.truncate(mark);
                return Err(PlaceError::Encode(e));
            }
        }
        for y in rect.top()..rect.bottom() {
            for x in rect.left()..rect.right() {
                buf[(x, y)].set_diff_option(CellDiffOption::Skip);
            }
        }
        self.placed.push(placed);
        Ok(())
    }

    /// Ends a frame: cells an image no longer covers are redrawn by Ratatui, which erases
    /// the image there.
    pub fn end(&mut self, buf: &mut Buffer) {
        if !self.stale {
            for old in &self.shown {
                if self
                    .placed
                    .iter()
                    .any(|p| p.id == old.id && p.rect == old.rect)
                {
                    continue;
                }
                let rect = old.rect.intersection(buf.area);
                for y in rect.top()..rect.bottom() {
                    for x in rect.left()..rect.right() {
                        let covered = self.placed.iter().any(|p| p.rect.contains((x, y).into()));
                        if !covered {
                            buf[(x, y)].set_diff_option(CellDiffOption::AlwaysUpdate);
                        }
                    }
                }
            }
        }
        std::mem::swap(&mut self.shown, &mut self.placed);
        self.stale = false;
    }
}
