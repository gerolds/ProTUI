// ADR: Sixel is an opt-in escape hatch (feature `sixel`): a linear `Raster` is tone mapped through the same display path as the glyph encoders, quantised to at most 256 colours (fixed 6×7×6 cube with a 4×4 ordered dither, or a per-image median cut), and written as one DCS sixel image with run-length compression.
// ADR: Integer scale only: across, pixels repeat through run lengths; down, the pixel aspect ratio (`"Pan;Pad`) makes one pixel `scale.1` device pixels tall, so a coarse image costs few sixel bands. Windows Terminal honours both (SixelParser.cpp, v1.24).
// ADR: The encoder keeps every work buffer between calls and appends to a caller-owned `Vec<u8>`, so a steady frame size allocates nothing.
use std::fmt;
use std::sync::LazyLock;

use crate::Raster;
use crate::color::display;

mod layer;

pub use layer::{PlaceError, SixelLayer};

/// Windows Terminal's sixel cell in virtual pixels (VT340 level). It answers `CSI 16 t` with
/// this size whatever the font, and scales each row of image pixels to the real cell.
pub const WT_CELL_PX: (u16, u16) = (10, 20);

/// How colours are chosen. Sixel allows at most 256 colours per image in Windows Terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Palette {
    /// 252 fixed colours: 6 red × 7 green × 6 blue sRGB levels, with a 4 × 4 ordered dither in
    /// source pixels. Only the colours a frame uses are sent.
    Fixed,
    /// Up to `colors` (2..=256) colours by median cut over a 5-bit-per-channel histogram of this
    /// image; no dither. A masked image keeps at most 255.
    Adaptive { colors: u16 },
}

/// Encoder settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SixelOptions {
    pub palette: Palette,
    /// Device pixels per source pixel (across, down), each 1..=255.
    pub scale: (u16, u16),
}

/// Refused encoder input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SixelError {
    /// A scale factor outside 1..=255.
    Scale((u16, u16)),
    /// An adaptive palette size outside 2..=256.
    Colors(u16),
    /// A mask whose length differs from the raster's pixel count.
    Mask { pixels: usize, mask: usize },
}

impl fmt::Display for SixelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scale((x, y)) => write!(f, "sixel scale {x}x{y} is outside 1..=255"),
            Self::Colors(n) => write!(f, "adaptive palette of {n} colours is outside 2..=256"),
            Self::Mask { pixels, mask } => {
                write!(f, "mask has {mask} entries for {pixels} pixels")
            }
        }
    }
}

impl std::error::Error for SixelError {}

const BINS: usize = 1 << 15;
const NONE: usize = usize::MAX;
/// Index of a transparent pixel; masked images use at most 255 colours.
const CLEAR: u8 = 255;
const LEVELS: [usize; 3] = [6, 7, 6];
const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// Per threshold (16) and channel (3): sRGB byte → fixed-cube level.
static DITHER: LazyLock<Box<[[[u8; 256]; 3]; 16]>> = LazyLock::new(|| {
    let mut table = Box::new([[[0u8; 256]; 3]; 16]);
    for (t, per_channel) in table.iter_mut().enumerate() {
        let offset = (t as f32 + 0.5) / 16.0 - 0.5;
        for (channel, levels) in per_channel.iter_mut().zip(LEVELS) {
            let top = (levels - 1) as f32;
            for (v, level) in channel.iter_mut().enumerate() {
                *level = (v as f32 * top / 255.0 + offset).round().clamp(0.0, top) as u8;
            }
        }
    }
    table
});

/// The fixed cube's colour `index` in sRGB bytes.
fn fixed_color(index: usize) -> [u8; 3] {
    let level = |l: usize, levels: usize| (l as f32 * 255.0 / (levels - 1) as f32).round() as u8;
    [
        level(index / 42, 6),
        level(index / 6 % 7, 7),
        level(index % 6, 6),
    ]
}

/// One sixel encoder; reuse it across frames.
#[derive(Default)]
pub struct SixelEncoder {
    srgb: Vec<[u8; 3]>,
    index: Vec<u8>,
    palette: Vec<[u8; 3]>,
    used: Vec<bool>,
    hist: Vec<u32>,
    sums: Vec<[u32; 3]>,
    occupied: Vec<u16>,
    bin_color: Vec<u8>,
    boxes: Vec<(usize, usize)>,
    bits: Vec<u8>,
    span: Vec<(usize, usize)>,
    band: Vec<u8>,
    colors: usize,
}

impl SixelEncoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Colours defined by the last encode.
    pub fn colors(&self) -> usize {
        self.colors
    }

    /// Appends one opaque sixel image of `raster` to `out`.
    pub fn encode(
        &mut self,
        raster: Raster<'_>,
        options: &SixelOptions,
        out: &mut Vec<u8>,
    ) -> Result<(), SixelError> {
        self.run(raster, None, options, out)
    }

    /// Appends one sixel image with a transparent background: pixels whose `mask` entry is
    /// false are not drawn, so the terminal's earlier pixels or cell background show through.
    pub fn encode_masked(
        &mut self,
        raster: Raster<'_>,
        mask: &[bool],
        options: &SixelOptions,
        out: &mut Vec<u8>,
    ) -> Result<(), SixelError> {
        self.run(raster, Some(mask), options, out)
    }

    fn run(
        &mut self,
        raster: Raster<'_>,
        mask: Option<&[bool]>,
        options: &SixelOptions,
        out: &mut Vec<u8>,
    ) -> Result<(), SixelError> {
        let (sx, sy) = options.scale;
        if !(1..=255).contains(&sx) || !(1..=255).contains(&sy) {
            return Err(SixelError::Scale(options.scale));
        }
        if let Palette::Adaptive { colors } = options.palette
            && !(2..=256).contains(&colors)
        {
            return Err(SixelError::Colors(colors));
        }
        let (w, h) = raster.size();
        if let Some(mask) = mask
            && mask.len() != w * h
        {
            return Err(SixelError::Mask {
                pixels: w * h,
                mask: mask.len(),
            });
        }
        self.srgb.clear();
        self.srgb.extend(
            raster
                .pixels()
                .iter()
                .map(|&p| display(p).map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8)),
        );
        self.index.resize(w * h, 0);
        self.used.clear();
        self.used.resize(256, false);
        match options.palette {
            Palette::Fixed => self.quantise_fixed(w),
            Palette::Adaptive { colors } => {
                let limit = if mask.is_some() {
                    colors.min(255)
                } else {
                    colors
                };
                self.quantise_adaptive(usize::from(limit));
            }
        }
        if let Some(mask) = mask {
            for (index, &shown) in self.index.iter_mut().zip(mask) {
                if !shown {
                    *index = CLEAR;
                }
            }
            // A colour may now be unused; recount so only drawn colours are defined.
            self.used.iter_mut().for_each(|u| *u = false);
            for &i in &self.index {
                if i != CLEAR {
                    self.used[usize::from(i)] = true;
                }
            }
        }
        self.emit(w, h, mask.is_some(), (sx, sy), out);
        Ok(())
    }

    fn quantise_fixed(&mut self, w: usize) {
        let table = &**DITHER;
        self.palette.clear();
        self.palette.extend((0..252).map(fixed_color));
        for (i, (rgb, index)) in self.srgb.iter().zip(&mut self.index).enumerate() {
            let t = &table[usize::from(BAYER[(i / w) % 4][(i % w) % 4])];
            let [r, g, b] = *rgb;
            let c = usize::from(t[0][usize::from(r)]) * 42
                + usize::from(t[1][usize::from(g)]) * 6
                + usize::from(t[2][usize::from(b)]);
            *index = c as u8;
            self.used[c] = true;
        }
    }

    fn quantise_adaptive(&mut self, limit: usize) {
        self.hist.resize(BINS, 0);
        self.sums.resize(BINS, [0; 3]);
        self.bin_color.resize(BINS, 0);
        self.occupied.clear();
        for &[r, g, b] in &self.srgb {
            let bin = bin_of([r, g, b]);
            if self.hist[bin] == 0 {
                self.occupied.push(bin as u16);
            }
            self.hist[bin] += 1;
            let sum = &mut self.sums[bin];
            sum[0] += u32::from(r);
            sum[1] += u32::from(g);
            sum[2] += u32::from(b);
        }
        self.boxes.clear();
        self.boxes.push((0, self.occupied.len()));
        while self.boxes.len() < limit {
            let Some((which, channel)) = self.widest_box() else {
                break;
            };
            let (start, end) = self.boxes[which];
            let hist = &self.hist;
            let bins = &mut self.occupied[start..end];
            bins.sort_unstable_by_key(|&bin| channel_of(bin, channel));
            let total: u32 = bins.iter().map(|&b| hist[usize::from(b)]).sum();
            let mut seen = 0;
            let mut split = 1;
            for (i, &bin) in bins.iter().enumerate() {
                seen += hist[usize::from(bin)];
                if seen * 2 >= total {
                    split = (i + 1).clamp(1, bins.len() - 1);
                    break;
                }
            }
            self.boxes[which] = (start, start + split);
            self.boxes.push((start + split, end));
        }
        self.palette.clear();
        for (color, &(start, end)) in self.boxes.iter().enumerate() {
            let (mut count, mut sum) = (0u64, [0u64; 3]);
            for &bin in &self.occupied[start..end] {
                let bin = usize::from(bin);
                count += u64::from(self.hist[bin]);
                for (s, v) in sum.iter_mut().zip(self.sums[bin]) {
                    *s += u64::from(v);
                }
                self.bin_color[bin] = color as u8;
            }
            self.palette
                .push(sum.map(|s| ((s + count / 2) / count.max(1)) as u8));
        }
        for (rgb, index) in self.srgb.iter().zip(&mut self.index) {
            let c = self.bin_color[bin_of(*rgb)];
            *index = c;
            self.used[usize::from(c)] = true;
        }
        for &bin in &self.occupied {
            self.hist[usize::from(bin)] = 0;
            self.sums[usize::from(bin)] = [0; 3];
        }
    }

    /// The box with the largest pixel count × channel range, and that channel; `None` when no
    /// box holds two bins.
    fn widest_box(&self) -> Option<(usize, usize)> {
        let mut best = None;
        let mut best_score = 0u64;
        for (i, &(start, end)) in self.boxes.iter().enumerate() {
            if end - start < 2 {
                continue;
            }
            let (mut lo, mut hi, mut count) = ([u16::MAX; 3], [0u16; 3], 0u64);
            for &bin in &self.occupied[start..end] {
                count += u64::from(self.hist[usize::from(bin)]);
                for c in 0..3 {
                    let v = channel_of(bin, c);
                    lo[c] = lo[c].min(v);
                    hi[c] = hi[c].max(v);
                }
            }
            let (channel, range) = (0..3)
                .map(|c| (c, hi[c] - lo[c]))
                .max_by_key(|&(_, r)| r)
                .expect("three channels");
            let score = count * u64::from(range);
            if range > 0 && score > best_score {
                best_score = score;
                best = Some((i, channel));
            }
        }
        best
    }

    fn emit(&mut self, w: usize, h: usize, masked: bool, (sx, sy): (u16, u16), out: &mut Vec<u8>) {
        out.extend_from_slice(if masked { b"\x1bP0;1q" } else { b"\x1bP0;0q" });
        out.push(b'"');
        push_num(out, usize::from(sy));
        out.extend_from_slice(b";1;");
        push_num(out, w * usize::from(sx));
        out.push(b';');
        push_num(out, h * usize::from(sy));
        self.colors = 0;
        for (i, rgb) in self.palette.iter().enumerate() {
            if !self.used[i] {
                continue;
            }
            self.colors += 1;
            out.push(b'#');
            push_num(out, i);
            out.extend_from_slice(b";2");
            for v in rgb {
                out.push(b';');
                push_num(out, (usize::from(*v) * 100 + 127) / 255);
            }
        }
        self.bits.resize(256 * w, 0);
        self.span.clear();
        self.span.resize(256, (NONE, 0));
        let repeat = usize::from(sx);
        for top in (0..h).step_by(6) {
            self.band.clear();
            for r in 0..(h - top).min(6) {
                let line = &self.index[(top + r) * w..(top + r + 1) * w];
                for (x, &c) in line.iter().enumerate() {
                    if masked && c == CLEAR {
                        continue;
                    }
                    let span = &mut self.span[usize::from(c)];
                    if span.0 == NONE {
                        *span = (x, x);
                        self.band.push(c);
                    } else {
                        span.0 = span.0.min(x);
                        span.1 = span.1.max(x);
                    }
                    self.bits[usize::from(c) * w + x] |= 1 << r;
                }
            }
            if top > 0 {
                out.push(b'-');
            }
            for (n, &c) in self.band.iter().enumerate() {
                if n > 0 {
                    out.push(b'$');
                }
                out.push(b'#');
                push_num(out, usize::from(c));
                let (lo, hi) = self.span[usize::from(c)];
                let row = &mut self.bits[usize::from(c) * w..][..=hi];
                push_run(out, b'?', lo * repeat);
                let mut x = lo;
                while x <= hi {
                    let value = row[x];
                    let start = x;
                    while x <= hi && row[x] == value {
                        x += 1;
                    }
                    push_run(out, b'?' + value, (x - start) * repeat);
                }
                // Leave the scratch row zeroed for the next band and the next image.
                row[lo..].fill(0);
                self.span[usize::from(c)] = (NONE, 0);
            }
        }
        out.extend_from_slice(b"\x1b\\");
    }
}

fn bin_of([r, g, b]: [u8; 3]) -> usize {
    (usize::from(r >> 3) << 10) | (usize::from(g >> 3) << 5) | usize::from(b >> 3)
}

fn channel_of(bin: u16, channel: usize) -> u16 {
    (bin >> (10 - 5 * channel)) & 31
}

/// `n` copies of sixel `ch`: plain up to three, else a `!n` repeat.
fn push_run(out: &mut Vec<u8>, ch: u8, n: usize) {
    if n >= 4 {
        out.push(b'!');
        push_num(out, n);
        out.push(ch);
    } else {
        out.extend(std::iter::repeat_n(ch, n));
    }
}

fn push_num(out: &mut Vec<u8>, mut n: usize) {
    let mut digits = [0u8; 20];
    let mut i = digits.len();
    loop {
        i -= 1;
        digits[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    out.extend_from_slice(&digits[i..]);
}

#[cfg(test)]
mod tests;
