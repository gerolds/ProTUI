// ADR: Captures rasterize a Buffer as Windows Terminal 1.24 draws it with one font: U+2500–U+259F (box drawing, blocks, quadrants, rounded ╭╮╰╯ as quarter ellipses) procedurally, every other glyph from the font — adapted from Runner capture.rs.
// ADR: Style modifiers other than REVERSED are ignored (drawn regular), as in Runner; a missing glyph, a multi-codepoint cell or a font without octants is an error, never a tofu box.
// ADR: APNG frames store only the bounding box of pixels changed since the previous frame (blend Source, dispose None): small files, same playback.
//! Actual-font cell captures for headless checks. Not a terminal emulator:
//! shaping, ligatures, wide-cell text, bold/italic faces and underlines are not
//! drawn. Cell size: height = ascent − descent + line gap at 18 px (gap split
//! above and below), width = the advance of 'M'.
use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use ab_glyph::{Font, FontArc, PxScale, ScaleFont, point};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::pixel::{OCTANTS, QUADRANTS};

/// Font size in px; Cascadia Mono gives a 9 x 18 px cell at this size.
const FONT_PX: f32 = 18.0;
const DEFAULT_FG: [u8; 3] = [204, 204, 204];
const DEFAULT_BG: [u8; 3] = [12, 12, 12];
const SHEET_BG: [u8; 3] = [24, 24, 28];

#[derive(Debug)]
pub enum CaptureError {
    Io(std::io::Error),
    FontParse,
    /// The font lacks this glyph (for octant output: a Unicode 16 octant).
    MissingGlyph(char),
    InvalidMetrics,
    /// Empty, overflowing, over 256 MiB, or not the size the operation needs.
    Size,
    /// The cell holds more than one codepoint.
    Grapheme {
        x: u16,
        y: u16,
    },
    /// An animation ended this many frames short of what it announced.
    MissingFrames(u32),
    Encode(png::EncodingError),
}
impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "capture I/O failed: {e}"),
            Self::FontParse => write!(f, "font data is not a supported TrueType/OpenType font"),
            Self::MissingGlyph(c) => write!(
                f,
                "font lacks U+{:04X} ({c}); for octants use a font that has them, such as the CascadiaMono.ttf in the Windows Terminal 1.24 install folder",
                *c as u32
            ),
            Self::InvalidMetrics => write!(f, "font has invalid or unsupported cell metrics"),
            Self::Size => write!(
                f,
                "capture is empty, overflows, exceeds 256 MiB, or has the wrong size"
            ),
            Self::Grapheme { x, y } => write!(f, "cell {x},{y} holds more than one codepoint"),
            Self::MissingFrames(n) => write!(f, "animation ended {n} frames short"),
            Self::Encode(e) => write!(f, "capture PNG encoding failed: {e}"),
        }
    }
}
impl std::error::Error for CaptureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Encode(e) => Some(e),
            _ => None,
        }
    }
}

/// An opaque 8-bit RGB picture of rasterized cells.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}
impl Picture {
    fn filled(width: u32, height: u32, rgb: [u8; 3]) -> Result<Self, CaptureError> {
        let count = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(3))
            .ok_or(CaptureError::Size)?;
        if count == 0 || count > 256 * 1024 * 1024 {
            return Err(CaptureError::Size);
        }
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(count)
            .map_err(|_| CaptureError::Size)?;
        for _ in 0..count / 3 {
            pixels.extend_from_slice(&rgb);
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    pub fn rgb(&self) -> &[u8] {
        &self.pixels
    }

    pub fn write_png(&self, output: impl Write) -> Result<(), CaptureError> {
        let mut encoder = png::Encoder::new(output, self.width, self.height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(CaptureError::Encode)?;
        writer
            .write_image_data(&self.pixels)
            .map_err(CaptureError::Encode)?;
        writer.finish().map_err(CaptureError::Encode)
    }

    pub fn save_png(&self, path: &Path) -> Result<(), CaptureError> {
        let mut output = BufWriter::new(File::create(path).map_err(CaptureError::Io)?);
        self.write_png(&mut output)?;
        output.flush().map_err(CaptureError::Io)
    }

    /// Copies `source` with its top left at `x, y`, clipped to this picture and
    /// to `max_width` columns of the source.
    fn blit(&mut self, source: &Picture, x: u32, y: u32, max_width: u32) {
        let width = source
            .width
            .min(max_width)
            .min(self.width.saturating_sub(x)) as usize;
        let rows = source.height.min(self.height.saturating_sub(y));
        for row in 0..rows {
            let from = (row * source.width) as usize * 3;
            let to = (((y + row) * self.width) + x) as usize * 3;
            self.pixels[to..to + width * 3].copy_from_slice(&source.pixels[from..from + width * 3]);
        }
    }

    /// RGB bytes inside `[x0, x1) x [y0, y1)`.
    fn crop_rgb(&self, (x0, y0, x1, y1): (u32, u32, u32, u32)) -> Vec<u8> {
        let mut out = Vec::with_capacity(((x1 - x0) * (y1 - y0) * 3) as usize);
        for y in y0..y1 {
            let start = (y * self.width + x0) as usize * 3;
            out.extend_from_slice(&self.pixels[start..start + (x1 - x0) as usize * 3]);
        }
        out
    }

    /// The smallest box holding every pixel that differs from `previous`, a
    /// picture of the same size; `None` when nothing changed.
    fn changed_since(&self, previous: &Picture) -> Option<(u32, u32, u32, u32)> {
        let width = self.width as usize;
        let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
        for (i, (a, b)) in previous
            .pixels
            .as_chunks::<3>()
            .0
            .iter()
            .zip(self.pixels.as_chunks::<3>().0)
            .enumerate()
        {
            if a != b {
                let (x, y) = (i % width, i / width);
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
            }
        }
        (x0 != usize::MAX).then_some((x0 as u32, y0 as u32, x1 as u32, y1 as u32))
    }
}

/// A looping animated PNG written one frame at a time.
pub struct Animation<W: Write> {
    writer: png::Writer<W>,
    previous: Picture,
    /// Frames announced but not yet written.
    missing: u32,
}

impl Animation<BufWriter<File>> {
    /// Creates the file at `path`; see [`Animation::new`].
    pub fn create(
        path: &Path,
        first: Picture,
        frames: u32,
        fps: u16,
    ) -> Result<Self, CaptureError> {
        let file = File::create(path).map_err(CaptureError::Io)?;
        Self::new(BufWriter::new(file), first, frames, fps)
    }
}

impl<W: Write> Animation<W> {
    /// Writes `first` as frame one of `frames`, each shown for 1/`fps` s.
    pub fn new(output: W, first: Picture, frames: u32, fps: u16) -> Result<Self, CaptureError> {
        if frames == 0 || fps == 0 {
            return Err(CaptureError::Size);
        }
        let (width, height) = first.size();
        let mut encoder = png::Encoder::new(output, width, height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::High);
        let encode = CaptureError::Encode;
        encoder.set_animated(frames, 0).map_err(encode)?;
        encoder.set_frame_delay(1, fps).map_err(encode)?;
        encoder.set_blend_op(png::BlendOp::Source).map_err(encode)?;
        encoder
            .set_dispose_op(png::DisposeOp::None)
            .map_err(encode)?;
        let mut writer = encoder.write_header().map_err(encode)?;
        writer.write_image_data(&first.pixels).map_err(encode)?;
        Ok(Self {
            writer,
            previous: first,
            missing: frames - 1,
        })
    }

    /// Appends the next frame, a picture of the first frame's size. An
    /// unchanged frame is stored as a 1 x 1 copy so its time still counts.
    pub fn push(&mut self, next: Picture) -> Result<(), CaptureError> {
        if next.size() != self.previous.size() {
            return Err(CaptureError::Size);
        }
        let area = next.changed_since(&self.previous).unwrap_or((0, 0, 1, 1));
        let encode = CaptureError::Encode;
        self.writer.reset_frame_position().map_err(encode)?;
        self.writer
            .set_frame_dimension(area.2 - area.0, area.3 - area.1)
            .map_err(encode)?;
        self.writer
            .set_frame_position(area.0, area.1)
            .map_err(encode)?;
        self.writer
            .write_image_data(&next.crop_rgb(area))
            .map_err(encode)?;
        self.previous = next;
        self.missing = self.missing.saturating_sub(1);
        Ok(())
    }

    /// Ends the file; fails when fewer frames were pushed than announced.
    pub fn finish(self) -> Result<(), CaptureError> {
        if self.missing > 0 {
            return Err(CaptureError::MissingFrames(self.missing));
        }
        self.writer.finish().map_err(CaptureError::Encode)
    }
}

/// One font at 18 px drawing cells as Windows Terminal 1.24 does.
pub struct CellFont {
    font: FontArc,
    scale: PxScale,
    cell_width: u32,
    cell_height: u32,
    baseline: f32,
    glyphs: HashMap<char, Vec<f32>>,
    scratch: Vec<f32>,
}

impl CellFont {
    pub fn load(path: &Path) -> Result<Self, CaptureError> {
        Self::from_bytes(std::fs::read(path).map_err(CaptureError::Io)?)
    }

    /// Refuses a font without every octant glyph that Windows Terminal does not
    /// draw itself, so octant captures never show missing-glyph boxes.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, CaptureError> {
        let font = FontArc::try_from_vec(bytes).map_err(|_| CaptureError::FontParse)?;
        if let Some(&c) = OCTANTS
            .iter()
            .find(|&&c| c != ' ' && !is_procedural(c) && font.glyph_id(c).0 == 0)
        {
            return Err(CaptureError::MissingGlyph(c));
        }
        let scale = PxScale::from(FONT_PX);
        let scaled = font.as_scaled(scale);
        let width = scaled.h_advance(font.glyph_id('M')).round();
        let height = (scaled.ascent() - scaled.descent() + scaled.line_gap()).round();
        let baseline = scaled.ascent() + scaled.line_gap() / 2.0;
        if !baseline.is_finite()
            || !(1.0..=512.0).contains(&width)
            || !(1.0..=512.0).contains(&height)
        {
            return Err(CaptureError::InvalidMetrics);
        }
        let (cell_width, cell_height) = (width as u32, height as u32);
        Ok(Self {
            font,
            scale,
            cell_width,
            cell_height,
            baseline,
            glyphs: HashMap::new(),
            scratch: vec![0.0; (cell_width * cell_height) as usize],
        })
    }

    /// Width and height of one cell in pixels.
    pub fn cell_size(&self) -> (u32, u32) {
        (self.cell_width, self.cell_height)
    }

    /// The cell height over width, the aspect a sketch needs for this font.
    pub fn cell_aspect(&self) -> f32 {
        self.cell_height as f32 / self.cell_width as f32
    }

    /// Font coverage (0..1) of `c`, clipped to one cell and cached.
    fn coverage(&mut self, c: char) -> Result<&[f32], CaptureError> {
        let id = self.font.glyph_id(c);
        if id.0 == 0 {
            return Err(CaptureError::MissingGlyph(c));
        }
        let (w, h, baseline, scale) =
            (self.cell_width, self.cell_height, self.baseline, self.scale);
        let font = &self.font;
        Ok(self.glyphs.entry(c).or_insert_with(|| {
            let mut out = vec![0.0f32; (w * h) as usize];
            let glyph = id.with_scale_and_position(scale, point(0.0, baseline));
            if let Some(outlined) = font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                outlined.draw(|gx, gy, v| {
                    let x = bounds.min.x as i32 + gx as i32;
                    let y = bounds.min.y as i32 + gy as i32;
                    if x >= 0 && y >= 0 && (x as u32) < w && (y as u32) < h {
                        let cell = &mut out[(y as u32 * w + x as u32) as usize];
                        *cell = (*cell + v).min(1.0);
                    }
                });
            }
            out
        }))
    }

    /// Rasterizes every cell of `buffer`; colours follow the Campbell scheme,
    /// `Color::Reset` is Windows Terminal's default foreground/background.
    /// REVERSED swaps them; other modifiers are ignored.
    pub fn rasterize(&mut self, buffer: &Buffer) -> Result<Picture, CaptureError> {
        let (cw, ch) = self.cell_size();
        let area = buffer.area;
        let width = u32::from(area.width)
            .checked_mul(cw)
            .ok_or(CaptureError::Size)?;
        let height = u32::from(area.height)
            .checked_mul(ch)
            .ok_or(CaptureError::Size)?;
        let mut picture = Picture::filled(width, height, DEFAULT_BG)?;
        let mut scratch = std::mem::take(&mut self.scratch);
        for row in 0..area.height {
            for col in 0..area.width {
                let (x, y) = (area.x + col, area.y + row);
                let cell = &buffer[(x, y)];
                let mut chars = cell.symbol().chars();
                let c = chars.next().unwrap_or(' ');
                if chars.next().is_some() {
                    self.scratch = scratch;
                    return Err(CaptureError::Grapheme { x, y });
                }
                let mut fg = rgb(cell.fg, DEFAULT_FG);
                let mut bg = rgb(cell.bg, DEFAULT_BG);
                if cell.modifier.contains(Modifier::REVERSED) {
                    std::mem::swap(&mut fg, &mut bg);
                }
                let coverage: &[f32] = if c == ' ' {
                    scratch.fill(0.0);
                    &scratch
                } else if draw_procedural(c, cw, ch, &mut scratch) {
                    &scratch
                } else {
                    match self.coverage(c) {
                        Ok(coverage) => coverage,
                        Err(e) => {
                            self.scratch = scratch;
                            return Err(e);
                        }
                    }
                };
                for gy in 0..ch {
                    let line = ((u32::from(row) * ch + gy) * width + u32::from(col) * cw) as usize;
                    for gx in 0..cw {
                        let a = coverage[(gy * cw + gx) as usize];
                        let i = (line + gx as usize) * 3;
                        for k in 0..3 {
                            let (f, b) = (f32::from(fg[k]), f32::from(bg[k]));
                            picture.pixels[i + k] = (b + (f - b) * a + 0.5) as u8;
                        }
                    }
                }
            }
        }
        self.scratch = scratch;
        Ok(picture)
    }

    /// Lays captioned pictures out in rows of `columns`, captions drawn with
    /// this font and clipped to the panel width. Every picture must have the
    /// first one's size; an empty list is refused.
    pub fn contact_sheet(
        &mut self,
        panels: &[(&str, &Picture)],
        columns: usize,
    ) -> Result<Picture, CaptureError> {
        let (_, first) = panels.first().ok_or(CaptureError::Size)?;
        let (panel_w, panel_h) = first.size();
        if panels.iter().any(|(_, p)| p.size() != (panel_w, panel_h)) {
            return Err(CaptureError::Size);
        }
        let gap = 12u32;
        let caption_h = self.cell_height + 4;
        let columns = columns.clamp(1, panels.len()) as u32;
        let rows = panels.len().div_ceil(columns as usize) as u32;
        let width = columns * panel_w + (columns + 1) * gap;
        let height = rows * (panel_h + caption_h) + (rows + 1) * gap;
        let mut sheet = Picture::filled(width, height, SHEET_BG)?;
        let fit = u16::try_from(panel_w / self.cell_width).unwrap_or(u16::MAX);
        for (index, (text, picture)) in panels.iter().enumerate() {
            let (column, row) = (index as u32 % columns, index as u32 / columns);
            let x = gap + column * (panel_w + gap);
            let y = gap + row * (panel_h + caption_h + gap);
            let chars = u16::try_from(text.chars().count())
                .unwrap_or(u16::MAX)
                .min(fit);
            if chars > 0 {
                let mut caption = Buffer::empty(Rect::new(0, 0, chars, 1));
                let [r, g, b] = SHEET_BG;
                caption.set_stringn(
                    0,
                    0,
                    text,
                    usize::from(chars),
                    Style::default()
                        .fg(Color::Rgb(230, 226, 214))
                        .bg(Color::Rgb(r, g, b)),
                );
                sheet.blit(&self.rasterize(&caption)?, x, y, panel_w);
            }
            sheet.blit(picture, x, y + caption_h, panel_w);
        }
        Ok(sheet)
    }
}

/// Line arms up, right, down, left: 0 none, 1 light, 2 heavy, 3 double.
type Arms = [u8; 4];

enum Shape {
    /// Filled boxes in cell fractions: x0, y0, x1, y1.
    Boxes(&'static [[f32; 4]]),
    Quadrants(usize),
    /// The whole cell in the foreground colour at this opacity.
    Shade(f32),
    Lines(Arms),
    /// A straight light/heavy line broken into `dashes` dashes.
    Dashed {
        horizontal: bool,
        weight: u8,
        dashes: u32,
    },
    /// A light quarter ellipse centred on the cell corner `(right, down)` points
    /// to: `╭` is `(true, true)`.
    Arc {
        right: bool,
        down: bool,
    },
    /// Light diagonals: `rising` is ╱, `falling` is ╲.
    Diagonal {
        rising: bool,
        falling: bool,
    },
}

/// Arms of U+2500–U+257F; dashed, arc and diagonal entries are handled first.
#[rustfmt::skip]
const BOX_ARMS: [Arms; 128] = [
    // 2500 ─ ━ │ ┃ ┄ ┅ ┆ ┇ ┈ ┉ ┊ ┋ ┌ ┍ ┎ ┏
    [0,1,0,1], [0,2,0,2], [1,0,1,0], [2,0,2,0], [0;4], [0;4], [0;4], [0;4],
    [0;4], [0;4], [0;4], [0;4], [0,1,1,0], [0,2,1,0], [0,1,2,0], [0,2,2,0],
    // 2510 ┐ ┑ ┒ ┓ └ ┕ ┖ ┗ ┘ ┙ ┚ ┛ ├ ┝ ┞ ┟
    [0,0,1,1], [0,0,1,2], [0,0,2,1], [0,0,2,2], [1,1,0,0], [1,2,0,0], [2,1,0,0], [2,2,0,0],
    [1,0,0,1], [1,0,0,2], [2,0,0,1], [2,0,0,2], [1,1,1,0], [1,2,1,0], [2,1,1,0], [1,1,2,0],
    // 2520 ┠ ┡ ┢ ┣ ┤ ┥ ┦ ┧ ┨ ┩ ┪ ┫ ┬ ┭ ┮ ┯
    [2,1,2,0], [2,2,1,0], [1,2,2,0], [2,2,2,0], [1,0,1,1], [1,0,1,2], [2,0,1,1], [1,0,2,1],
    [2,0,2,1], [2,0,1,2], [1,0,2,2], [2,0,2,2], [0,1,1,1], [0,1,1,2], [0,2,1,1], [0,2,1,2],
    // 2530 ┰ ┱ ┲ ┳ ┴ ┵ ┶ ┷ ┸ ┹ ┺ ┻ ┼ ┽ ┾ ┿
    [0,1,2,1], [0,1,2,2], [0,2,2,1], [0,2,2,2], [1,1,0,1], [1,1,0,2], [1,2,0,1], [1,2,0,2],
    [2,1,0,1], [2,1,0,2], [2,2,0,1], [2,2,0,2], [1,1,1,1], [1,1,1,2], [1,2,1,1], [1,2,1,2],
    // 2540 ╀ ╁ ╂ ╃ ╄ ╅ ╆ ╇ ╈ ╉ ╊ ╋ ╌ ╍ ╎ ╏
    [2,1,1,1], [1,1,2,1], [2,1,2,1], [2,1,1,2], [2,2,1,1], [1,1,2,2], [1,2,2,1], [2,2,1,2],
    [1,2,2,2], [2,1,2,2], [2,2,2,1], [2,2,2,2], [0;4], [0;4], [0;4], [0;4],
    // 2550 ═ ║ ╒ ╓ ╔ ╕ ╖ ╗ ╘ ╙ ╚ ╛ ╜ ╝ ╞ ╟
    [0,3,0,3], [3,0,3,0], [0,3,1,0], [0,1,3,0], [0,3,3,0], [0,0,1,3], [0,0,3,1], [0,0,3,3],
    [1,3,0,0], [3,1,0,0], [3,3,0,0], [1,0,0,3], [3,0,0,1], [3,0,0,3], [1,3,1,0], [3,1,3,0],
    // 2560 ╠ ╡ ╢ ╣ ╤ ╥ ╦ ╧ ╨ ╩ ╪ ╫ ╬ ╭ ╮ ╯
    [3,3,3,0], [1,0,1,3], [3,0,3,1], [3,0,3,3], [0,3,1,3], [0,1,3,1], [0,3,3,3], [1,3,0,3],
    [3,1,0,1], [3,3,0,3], [1,3,1,3], [3,1,3,1], [3,3,3,3], [0;4], [0;4], [0;4],
    // 2570 ╰ ╱ ╲ ╳ ╴ ╵ ╶ ╷ ╸ ╹ ╺ ╻ ╼ ╽ ╾ ╿
    [0;4], [0;4], [0;4], [0;4], [0,0,0,1], [1,0,0,0], [0,1,0,0], [0,0,1,0],
    [0,0,0,2], [2,0,0,0], [0,2,0,0], [0,0,2,0], [0,2,0,1], [1,0,2,0], [0,1,0,2], [2,0,1,0],
];

fn is_procedural(c: char) -> bool {
    (0x2500..=0x259F).contains(&(c as u32))
}

fn procedural(c: char) -> Option<Shape> {
    let code = c as u32;
    if !is_procedural(c) {
        return None;
    }
    if let Some(index) = QUADRANTS.iter().position(|&q| q == c).filter(|&i| i != 0) {
        return Some(Shape::Quadrants(index));
    }
    let dashed = |horizontal, weight, dashes| Shape::Dashed {
        horizontal,
        weight,
        dashes,
    };
    Some(match code {
        0x2504 => dashed(true, 1, 3),
        0x2505 => dashed(true, 2, 3),
        0x2506 => dashed(false, 1, 3),
        0x2507 => dashed(false, 2, 3),
        0x2508 => dashed(true, 1, 4),
        0x2509 => dashed(true, 2, 4),
        0x250A => dashed(false, 1, 4),
        0x250B => dashed(false, 2, 4),
        0x254C => dashed(true, 1, 2),
        0x254D => dashed(true, 2, 2),
        0x254E => dashed(false, 1, 2),
        0x254F => dashed(false, 2, 2),
        0x256D => Shape::Arc {
            right: true,
            down: true,
        },
        0x256E => Shape::Arc {
            right: false,
            down: true,
        },
        0x256F => Shape::Arc {
            right: false,
            down: false,
        },
        0x2570 => Shape::Arc {
            right: true,
            down: false,
        },
        0x2571 => Shape::Diagonal {
            rising: true,
            falling: false,
        },
        0x2572 => Shape::Diagonal {
            rising: false,
            falling: true,
        },
        0x2573 => Shape::Diagonal {
            rising: true,
            falling: true,
        },
        0x2500..=0x257F => Shape::Lines(BOX_ARMS[(code - 0x2500) as usize]),
        0x2581..=0x2587 => Shape::Boxes(&LOWER[(code - 0x2581) as usize]),
        0x2589..=0x258F => Shape::Boxes(&LEFT[(code - 0x2589) as usize]),
        0x2591 => Shape::Shade(0.25),
        0x2592 => Shape::Shade(0.5),
        0x2593 => Shape::Shade(0.75),
        0x2594 => Shape::Boxes(&[[0.0, 0.0, 1.0, 0.125]]),
        0x2595 => Shape::Boxes(&[[0.875, 0.0, 1.0, 1.0]]),
        // ▀ █ ▐ and the quadrants are in QUADRANTS above.
        _ => unreachable!("U+{code:04X} is a quadrant"),
    })
}

/// ▁ … ▇: the lower 1/8 … 7/8.
static LOWER: [[[f32; 4]; 1]; 7] = {
    let mut out = [[[0.0; 4]]; 7];
    let mut i = 0;
    while i < 7 {
        out[i] = [[0.0, 1.0 - (i + 1) as f32 / 8.0, 1.0, 1.0]];
        i += 1;
    }
    out
};
/// ▉ … ▏: the left 7/8 … 1/8.
static LEFT: [[[f32; 4]; 1]; 7] = {
    let mut out = [[[0.0; 4]]; 7];
    let mut i = 0;
    while i < 7 {
        out[i] = [[0.0, 0.0, (7 - i) as f32 / 8.0, 1.0]];
        i += 1;
    }
    out
};

/// Writes the coverage of a U+2500–U+259F glyph into `coverage` (cw x ch);
/// returns false, leaving `coverage` unspecified, for any other char.
fn draw_procedural(c: char, cw: u32, ch: u32, coverage: &mut [f32]) -> bool {
    let Some(shape) = procedural(c) else {
        return false;
    };
    coverage.fill(0.0);
    let (w, h) = (cw as i32, ch as i32);
    let mut fill = |x0: i32, y0: i32, x1: i32, y1: i32, a: f32| {
        for y in y0.clamp(0, h)..y1.clamp(0, h) {
            for x in x0.clamp(0, w)..x1.clamp(0, w) {
                coverage[(y * w + x) as usize] = a;
            }
        }
    };
    let snap = |f: f32, size: i32| (f * size as f32).round() as i32;
    let light = (w / 8).max(1);
    match shape {
        Shape::Boxes(boxes) => {
            for b in boxes {
                fill(
                    snap(b[0], w),
                    snap(b[1], h),
                    snap(b[2], w),
                    snap(b[3], h),
                    1.0,
                );
            }
        }
        Shape::Quadrants(index) => {
            let (mx, my) = (snap(0.5, w), snap(0.5, h));
            let quarters = [
                (0, 0, mx, my),
                (mx, 0, w, my),
                (0, my, mx, h),
                (mx, my, w, h),
            ];
            for (bit, q) in quarters.iter().enumerate() {
                if index & (1 << bit) != 0 {
                    fill(q.0, q.1, q.2, q.3, 1.0);
                }
            }
        }
        Shape::Shade(a) => fill(0, 0, w, h, a),
        Shape::Lines(arms) => draw_lines(arms, w, h, light, &mut fill),
        Shape::Dashed {
            horizontal,
            weight,
            dashes,
        } => {
            let arms = if horizontal {
                [0, weight, 0, weight]
            } else {
                [weight, 0, weight, 0]
            };
            draw_lines(arms, w, h, light, &mut fill);
            // Equal dashes and gaps, the cell edge in the middle of a gap.
            let (len, n) = (if horizontal { w } else { h }, dashes as i32);
            for p in 0..len {
                // Half-unit q of 2n units, shifted by half a unit: even q is a gap.
                if ((2 * p + 1) * 2 * n + len) / (2 * len) % 2 == 0 {
                    if horizontal {
                        fill(p, 0, p + 1, h, 0.0);
                    } else {
                        fill(0, p, w, p + 1, 0.0);
                    }
                }
            }
        }
        Shape::Arc { right, down } => draw_arc(right, down, cw, ch, light as f32, coverage),
        Shape::Diagonal { rising, falling } => {
            draw_diagonals(rising, falling, cw, ch, light as f32, coverage)
        }
    }
    true
}

/// Straight box-drawing arms. Double lines are two light strokes `light + 1`
/// from the centre line; joins mitre the way Windows Terminal's glyphs do.
fn draw_lines(
    arms: Arms,
    w: i32,
    h: i32,
    light: i32,
    fill: &mut impl FnMut(i32, i32, i32, i32, f32),
) {
    let gap = light + 1;
    let thickness = |weight: u8| if weight == 2 { 2 * light } else { light };
    // A stroke of thickness `t` centred on pixel `c`: [c - t/2, c - t/2 + t).
    let range = |c: i32, t: i32| (c - t / 2, c - t / 2 + t);
    for dir in 0..4 {
        let weight = arms[dir];
        if weight == 0 {
            continue;
        }
        let horizontal = dir % 2 == 1;
        let positive = dir == 1 || dir == 2;
        let (len, lateral_len) = if horizontal { (w, h) } else { (h, w) };
        let (m, lateral_mid) = (len / 2, lateral_len / 2);
        // Perpendicular arms: on the negative lateral side, then the positive.
        let (p_neg, p_pos) = if horizontal {
            (arms[0], arms[2])
        } else {
            (arms[3], arms[1])
        };
        let opposite = arms[(dir + 2) % 4];
        // Where the perpendicular strokes lie along this arm's axis.
        let double_neg = range(m - gap, light);
        let double_pos = range(m + gap, light);
        let single = range(m, thickness(p_neg.max(p_pos)));
        let (far_double, near_double, far_single) = if positive {
            (double_neg.0, double_pos.0, single.0)
        } else {
            (double_pos.1, double_neg.1, single.1)
        };
        let center = if positive { m } else { m + 1 };
        let strokes: &[(i32, i32)] = if weight == 3 {
            &[(-gap, -1), (gap, 1)]
        } else {
            &[(0, 0)]
        };
        for &(offset, side) in strokes {
            let edge = if weight == 3 {
                let (same, other) = if side < 0 {
                    (p_neg, p_pos)
                } else {
                    (p_pos, p_neg)
                };
                match (same, other) {
                    (3, _) => near_double,
                    (1 | 2, _) => center,
                    (0, 3) => far_double,
                    _ => center,
                }
            } else if opposite != 0 {
                center
            } else {
                match (p_neg == 3, p_pos == 3) {
                    (true, true) => near_double,
                    (true, false) | (false, true) => far_double,
                    _ if p_neg != 0 || p_pos != 0 => far_single,
                    _ => center,
                }
            };
            let (a0, a1) = if positive { (edge, len) } else { (0, edge) };
            let t = if weight == 3 {
                light
            } else {
                thickness(weight)
            };
            let (l0, l1) = range(lateral_mid + offset, t);
            if horizontal {
                fill(a0, l0, a1, l1, 1.0);
            } else {
                fill(l0, a0, l1, a1, 1.0);
            }
        }
    }
}

/// Coverage at 4 x 4 subsamples per pixel of points within `inside`.
fn supersample(cw: u32, ch: u32, coverage: &mut [f32], inside: impl Fn(f32, f32) -> bool) {
    const SUB: u32 = 4;
    for y in 0..ch {
        for x in 0..cw {
            let mut hits = 0;
            for sy in 0..SUB {
                for sx in 0..SUB {
                    let px = x as f32 + (sx as f32 + 0.5) / SUB as f32;
                    let py = y as f32 + (sy as f32 + 0.5) / SUB as f32;
                    hits += u32::from(inside(px, py));
                }
            }
            if hits > 0 {
                coverage[(y * cw + x) as usize] = hits as f32 / (SUB * SUB) as f32;
            }
        }
    }
}

fn draw_arc(right: bool, down: bool, cw: u32, ch: u32, thick: f32, coverage: &mut [f32]) {
    // The arms' stroke centre, as `draw_lines` places a light stroke.
    let centre = |size: u32| (size / 2) as f32 - (thick / 2.0).floor() + thick / 2.0;
    let (mx, my) = (centre(cw), centre(ch));
    let ox = if right { cw as f32 } else { 0.0 };
    let oy = if down { ch as f32 } else { 0.0 };
    let (rx, ry) = ((ox - mx).abs(), (oy - my).abs());
    supersample(cw, ch, coverage, |px, py| {
        let (dx, dy) = (px - ox, py - oy);
        let f = (dx / rx).powi(2) + (dy / ry).powi(2) - 1.0;
        let g = 2.0 * ((dx / (rx * rx)).powi(2) + (dy / (ry * ry)).powi(2)).sqrt();
        f.abs() / g.max(1e-6) <= thick / 2.0
    });
}

fn draw_diagonals(rising: bool, falling: bool, cw: u32, ch: u32, thick: f32, coverage: &mut [f32]) {
    let (w, h) = (cw as f32, ch as f32);
    let length = (w * w + h * h).sqrt();
    supersample(cw, ch, coverage, |px, py| {
        // Distances to the lines (0,h)-(w,0) and (0,0)-(w,h).
        let up = (h * px + w * py - w * h).abs() / length;
        let down = (h * px - w * py).abs() / length;
        (rising && up <= thick / 2.0) || (falling && down <= thick / 2.0)
    });
}

const CAMPBELL: [[u8; 3]; 16] = [
    [12, 12, 12],
    [197, 15, 31],
    [19, 161, 14],
    [193, 156, 0],
    [0, 55, 218],
    [136, 23, 152],
    [58, 150, 221],
    [204, 204, 204],
    [118, 118, 118],
    [231, 72, 86],
    [22, 198, 12],
    [249, 241, 165],
    [59, 120, 255],
    [180, 0, 158],
    [97, 214, 214],
    [242, 242, 242],
];

/// The colour Windows Terminal's default Campbell scheme shows for a ratatui colour.
fn rgb(color: Color, default: [u8; 3]) -> [u8; 3] {
    let indexed = |n: u8| match n {
        0..=15 => CAMPBELL[n as usize],
        16..=231 => {
            let level = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            let n = n - 16;
            [level(n / 36), level(n / 6 % 6), level(n % 6)]
        }
        _ => [8 + (n - 232) * 10; 3],
    };
    match color {
        Color::Reset => default,
        Color::Rgb(r, g, b) => [r, g, b],
        Color::Indexed(n) => indexed(n),
        Color::Black => CAMPBELL[0],
        Color::Red => CAMPBELL[1],
        Color::Green => CAMPBELL[2],
        Color::Yellow => CAMPBELL[3],
        Color::Blue => CAMPBELL[4],
        Color::Magenta => CAMPBELL[5],
        Color::Cyan => CAMPBELL[6],
        Color::Gray => CAMPBELL[7],
        Color::DarkGray => CAMPBELL[8],
        Color::LightRed => CAMPBELL[9],
        Color::LightGreen => CAMPBELL[10],
        Color::LightYellow => CAMPBELL[11],
        Color::LightBlue => CAMPBELL[12],
        Color::LightMagenta => CAMPBELL[13],
        Color::LightCyan => CAMPBELL[14],
        Color::White => CAMPBELL[15],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CW: u32 = 9;
    const CH: u32 = 18;

    fn shape(c: char) -> Vec<f32> {
        let mut coverage = vec![0.0; (CW * CH) as usize];
        assert!(draw_procedural(c, CW, CH, &mut coverage), "{c}");
        coverage
    }
    fn at(coverage: &[f32], x: u32, y: u32) -> f32 {
        coverage[(y * CW + x) as usize]
    }

    #[test]
    fn every_box_and_block_glyph_is_drawn_procedurally() {
        for code in 0x2500..=0x259F {
            let c = char::from_u32(code).unwrap();
            let coverage = shape(c);
            let ink: f32 = coverage.iter().sum();
            assert!(ink > 0.0, "U+{code:04X} {c} draws nothing");
            assert!(
                ink < (CW * CH) as f32 || c == '█',
                "U+{code:04X} {c} fills the cell"
            );
        }
        let mut coverage = vec![0.0; (CW * CH) as usize];
        assert!(!draw_procedural('A', CW, CH, &mut coverage));
        assert!(!draw_procedural('\u{1CD00}', CW, CH, &mut coverage));
        assert!(!draw_procedural('\u{25A0}', CW, CH, &mut coverage));
    }

    #[test]
    fn lines_reach_the_edges_they_name() {
        let (mx, my) = (CW / 2, CH / 2);
        let h = shape('─');
        assert!((0..CW).all(|x| at(&h, x, my) == 1.0));
        assert_eq!(at(&h, mx, 0), 0.0);
        let corner = shape('┌');
        assert_eq!(at(&corner, CW - 1, my), 1.0);
        assert_eq!(at(&corner, mx, CH - 1), 1.0);
        assert_eq!(at(&corner, 0, my), 0.0);
        assert_eq!(at(&corner, mx, 0), 0.0);
        // A rounded corner joins the same two arm ends but leaves the centre.
        let round = shape('╭');
        assert!(at(&round, CW - 1, my) > 0.0 && at(&round, mx, CH - 1) > 0.0);
        assert_eq!(at(&round, mx, my), 0.0);
        let half = shape('╴');
        assert_eq!(at(&half, 0, my), 1.0);
        assert_eq!(at(&half, CW - 1, my), 0.0);
        // Double lines are two strokes with a gap on the centre line.
        let double = shape('═');
        assert_eq!(at(&double, 0, my), 0.0);
        assert_eq!(at(&double, 0, my - 2), 1.0);
        assert_eq!(at(&double, 0, my + 2), 1.0);
        // ╔ has an outer and an inner corner and nothing at the centre.
        let corner = shape('╔');
        assert_eq!(at(&corner, mx - 2, my - 2), 1.0);
        assert_eq!(at(&corner, mx + 2, my + 2), 1.0);
        assert_eq!(at(&corner, mx, my), 0.0);
        assert_eq!(at(&corner, mx - 2, my + 4), 1.0);
        // Dashes leave gaps along the line.
        let dashed = shape('┄');
        let lit = (0..CW).filter(|&x| at(&dashed, x, my) == 1.0).count();
        assert!(lit > 0 && lit < CW as usize, "{lit}");
        assert_eq!(shape('▄').iter().sum::<f32>(), (CW * CH / 2) as f32);
        assert_eq!(at(&shape('▏'), 0, 0), 1.0);
        assert_eq!(at(&shape('▏'), 1, 0), 0.0);
    }

    fn picture(width: u32, height: u32, fill: u8) -> Picture {
        Picture {
            width,
            height,
            pixels: vec![fill; (width * height * 3) as usize],
        }
    }

    #[test]
    fn animation_writes_every_frame_as_a_changed_box() {
        let mut bytes = Vec::new();
        let mut animation = Animation::new(&mut bytes, picture(8, 6, 0), 3, 10).unwrap();
        let mut second = picture(8, 6, 0);
        second.pixels[(2 * 8 + 3) * 3] = 255;
        second.pixels[(4 * 8 + 5) * 3] = 255;
        animation.push(second.clone()).unwrap();
        assert!(matches!(
            animation.push(picture(4, 4, 0)),
            Err(CaptureError::Size)
        ));
        animation.push(second).unwrap();
        animation.finish().unwrap();
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().unwrap();
        let control = reader.info().animation_control.unwrap();
        assert_eq!((control.num_frames, control.num_plays), (3, 0));
        let mut frames = Vec::new();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        while frames.len() < 3 {
            reader.next_frame(&mut buf).unwrap();
            let fc = reader.info().frame_control.unwrap();
            frames.push((fc.x_offset, fc.y_offset, fc.width, fc.height));
        }
        assert_eq!(frames, [(0, 0, 8, 6), (3, 2, 3, 3), (0, 0, 1, 1)]);
    }

    #[test]
    fn too_few_frames_fail_to_finish() {
        let animation = Animation::new(Vec::new(), picture(2, 2, 0), 2, 10).unwrap();
        assert!(animation.finish().is_err());
        assert!(Animation::new(Vec::new(), picture(2, 2, 0), 0, 10).is_err());
    }

    #[test]
    fn blit_clips_to_both_pictures() {
        let mut sheet = picture(4, 3, 0);
        sheet.blit(&picture(3, 3, 9), 2, 1, 2);
        let lit: Vec<_> = (0..12).filter(|i| sheet.pixels[i * 3] == 9).collect();
        assert_eq!(lit, [6, 7, 10, 11]);
        assert!(Picture::filled(0, 4, [0; 3]).is_err());
        assert!(Picture::filled(u32::MAX, u32::MAX, [0; 3]).is_err());
    }

    #[test]
    fn malformed_font_is_reported() {
        assert!(matches!(
            CellFont::from_bytes(vec![]),
            Err(CaptureError::FontParse)
        ));
    }

    #[test]
    fn indexed_colors_cover_cube_and_greys() {
        assert_eq!(rgb(Color::Indexed(16), [1; 3]), [0; 3]);
        assert_eq!(rgb(Color::Indexed(231), [1; 3]), [255; 3]);
        assert_eq!(rgb(Color::Indexed(255), [1; 3]), [238; 3]);
        assert_eq!(rgb(Color::Reset, [1; 3]), [1; 3]);
    }
}
