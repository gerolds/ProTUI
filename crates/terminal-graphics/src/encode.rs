// ADR: Box-average linear samples per sub-cell, then fit two colours in display (sRGB) space: luminance split refined by up to four 2-means rounds; glyphs come from ratatui's QUADRANTS/OCTANTS tables.
// ADR: `Encoder` has no default — the caller picks it from a player setting, because Windows Terminal draws quadrants itself but octants need a font that has them.
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::symbols::pixel::{OCTANTS, QUADRANTS};

use crate::color::{display, to_color};
use crate::{Raster, Rgb};

/// Two display-space colours and one block glyph per cell. Quadrant (2 x 2)
/// works in any Windows Terminal font; octant (2 x 4) needs a font with the
/// Unicode 16 octant glyphs. There is no default and no automatic fallback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoder {
    Octant,
    Quadrant,
}
impl Encoder {
    fn grid(self) -> (usize, usize) {
        match self {
            Self::Octant => (2, 4),
            Self::Quadrant => (2, 2),
        }
    }
    fn table(self) -> &'static [char] {
        match self {
            Self::Octant => &OCTANTS,
            Self::Quadrant => &QUADRANTS,
        }
    }
}

/// Stretch the raster over `area`. Sampling is linear-light box averaging, then
/// luminance split plus four two-means iterations in display/sRGB space.
/// Only the intersection with the buffer is written; clipping does not reframe
/// the image. Empty/disjoint rectangles do nothing. Cell styles are replaced.
pub fn encode_image(image: Raster<'_>, encoder: Encoder, area: Rect, buffer: &mut Buffer) {
    let visible = area.intersection(buffer.area);
    if visible.is_empty() {
        return;
    }
    let (sx, sy) = encoder.grid();
    let sample_w = image.width as f64 / (f64::from(area.width) * sx as f64);
    let sample_h = image.height as f64 / (f64::from(area.height) * sy as f64);
    let mut samples = [[0.0; 3]; 8];
    for y in visible.top()..visible.bottom() {
        for x in visible.left()..visible.right() {
            let col = usize::from(x - area.x);
            let row = usize::from(y - area.y);
            for j in 0..sy {
                for i in 0..sx {
                    samples[j * sx + i] = display(box_average(
                        image,
                        (col * sx + i) as f64 * sample_w,
                        (row * sy + j) as f64 * sample_h,
                        sample_w,
                        sample_h,
                    ));
                }
            }
            let cell = &mut buffer[(x, y)];
            cell.reset();
            fit_cell(&samples[..sx * sy], encoder.table(), cell);
        }
    }
}

/// Exact area-weighted average of an in-bounds source box. f64 accumulation
/// prevents finite HDR inputs overflowing while summed; no temporary allocation.
pub(crate) fn box_average(image: Raster<'_>, x: f64, y: f64, w: f64, h: f64) -> Rgb {
    let x1 = (x + w).min(image.width as f64);
    let y1 = (y + h).min(image.height as f64);
    let first_x = (x.floor() as usize).min(image.width - 1);
    let first_y = (y.floor() as usize).min(image.height - 1);
    let last_x = (x1.ceil() as usize).clamp(first_x + 1, image.width);
    let last_y = (y1.ceil() as usize).clamp(first_y + 1, image.height);
    let mut sum = [0.0f64; 3];
    let mut total = 0.0;
    for py in first_y..last_y {
        let wy = (y1.min(py as f64 + 1.0) - y.max(py as f64)).max(0.0);
        for px in first_x..last_x {
            let wx = (x1.min(px as f64 + 1.0) - x.max(px as f64)).max(0.0);
            let weight = wx * wy;
            let p = image.pixels[py * image.width + px];
            for (sum, channel) in sum.iter_mut().zip([p.r, p.g, p.b]) {
                *sum += f64::from(channel) * weight;
            }
            total += weight;
        }
    }
    if total == 0.0 {
        // At an extreme aspect, rounding can collapse a subpixel box. Its center
        // still names a source pixel; no outside color or fabricated padding.
        return image.pixels[first_y * image.width + first_x];
    }
    let [r, g, b] = sum.map(|v| (v / total).min(f64::from(f32::MAX)) as f32);
    Rgb::new(r, g, b)
}

fn distance2(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

fn fit(samples: &[[f32; 3]]) -> Option<(usize, [f32; 3], [f32; 3])> {
    let luma = |c: [f32; 3]| 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
    let (mut low, mut high) = (f32::MAX, f32::MIN);
    for &s in samples {
        low = low.min(luma(s));
        high = high.max(luma(s));
    }
    let threshold = (low + high) / 2.0;
    let mut pattern = samples
        .iter()
        .enumerate()
        .filter(|(_, s)| luma(**s) > threshold)
        .fold(0usize, |bits, (i, _)| bits | 1 << i);
    for _ in 0..4 {
        let (mut fg, mut bg) = ([0.0f32; 3], [0.0f32; 3]);
        let (mut nf, mut nb) = (0.0f32, 0.0f32);
        for (i, s) in samples.iter().enumerate() {
            let (sum, count) = if pattern & (1 << i) != 0 {
                (&mut fg, &mut nf)
            } else {
                (&mut bg, &mut nb)
            };
            for k in 0..3 {
                sum[k] += s[k];
            }
            *count += 1.0;
        }
        if nf == 0.0 || nb == 0.0 {
            return None;
        }
        let fg = fg.map(|v| v / nf);
        let bg = bg.map(|v| v / nb);
        let next = samples
            .iter()
            .enumerate()
            .filter(|(_, s)| distance2(**s, fg) < distance2(**s, bg))
            .fold(0usize, |bits, (i, _)| bits | 1 << i);
        if next == pattern || next == 0 || next == (1 << samples.len()) - 1 {
            return Some((pattern, fg, bg));
        }
        pattern = next;
    }
    let mean = |bit: bool| {
        let (mut sum, mut count) = ([0.0f32; 3], 0.0f32);
        for (i, s) in samples.iter().enumerate() {
            if (pattern & (1 << i) != 0) == bit {
                for k in 0..3 {
                    sum[k] += s[k];
                }
                count += 1.0;
            }
        }
        sum.map(|v| v / count)
    };
    Some((pattern, mean(true), mean(false)))
}

fn fit_cell(samples: &[[f32; 3]], table: &[char], cell: &mut Cell) {
    match fit(samples) {
        Some((pattern, fg, bg)) if to_color(fg) != to_color(bg) => {
            cell.set_char(table[pattern])
                .set_fg(to_color(fg))
                .set_bg(to_color(bg));
        }
        _ => {
            let n = samples.len() as f32;
            let mean = samples.iter().fold([0.0; 3], |acc, c| {
                [acc[0] + c[0] / n, acc[1] + c[1] / n, acc[2] + c[2] / n]
            });
            cell.set_char(' ')
                .set_fg(Color::Reset)
                .set_bg(to_color(mean));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pattern_matches_ratatui_row_major_glyph_shape() {
        for encoder in [Encoder::Octant, Encoder::Quadrant] {
            let (sx, sy) = encoder.grid();
            let count = sx * sy;
            let table = encoder.table();
            let all = (1 << count) - 1;
            for pattern in 1..all {
                let samples: Vec<_> = (0..count)
                    .map(|i| {
                        if pattern & (1 << i) != 0 {
                            [1.0; 3]
                        } else {
                            [0.0; 3]
                        }
                    })
                    .collect();
                let mut cell = Cell::default();
                fit_cell(&samples, table, &mut cell);
                let expected = if cell.fg == Color::Rgb(255, 255, 255) {
                    pattern
                } else {
                    !pattern & all
                };
                assert_eq!(cell.symbol(), table[expected].to_string());
            }
        }
        // Samples are numbered row-major: 0 1 / 2 3 / 4 5 / 6 7 (octant), 0 1 / 2 3 (quadrant).
        let block = |bits: &[usize], encoder: Encoder| {
            let (sx, sy) = encoder.grid();
            let samples: Vec<_> = (0..sx * sy)
                .map(|i| {
                    if bits.contains(&i) {
                        [1.0; 3]
                    } else {
                        [0.0; 3]
                    }
                })
                .collect();
            let mut cell = Cell::default();
            fit_cell(&samples, encoder.table(), &mut cell);
            cell.symbol().to_owned()
        };
        assert_eq!(block(&[0, 1, 2, 3], Encoder::Octant), "▀");
        assert_eq!(block(&[0, 2, 4, 6], Encoder::Octant), "▌");
        assert_eq!(block(&[0, 2], Encoder::Octant), "▘");
        assert_eq!(block(&[5, 7], Encoder::Octant), "▗");
        assert_eq!(block(&[0, 1], Encoder::Quadrant), "▀");
        assert_eq!(block(&[0, 2], Encoder::Quadrant), "▌");
        assert_eq!(block(&[3], Encoder::Quadrant), "▗");
        assert_eq!(block(&[1, 2], Encoder::Quadrant), "▞");
    }

    #[test]
    fn one_colour_cells_become_a_space_on_the_mean() {
        let mut cell = Cell::default();
        fit_cell(&[[0.5; 3]; 8], &OCTANTS, &mut cell);
        assert_eq!(
            (cell.symbol(), cell.fg, cell.bg),
            (" ", Color::Reset, Color::Rgb(128, 128, 128))
        );
        // Two groups that round to the same 8-bit colour are one colour.
        let mut samples = [[0.5; 3]; 4];
        samples[0] = [0.501; 3];
        fit_cell(&samples, &QUADRANTS, &mut cell);
        assert_eq!((cell.symbol(), cell.fg), (" ", Color::Reset));
        // One step apart at 8 bits is two colours.
        samples[0] = [0.5 + 1.0 / 255.0; 3];
        fit_cell(&samples, &QUADRANTS, &mut cell);
        assert_eq!(cell.symbol(), "▘");
        assert_eq!(cell.fg, Color::Rgb(129, 129, 129));
        assert_eq!(cell.bg, Color::Rgb(128, 128, 128));
    }

    #[test]
    fn two_means_refines_the_luma_split() {
        // The luma midpoint (0.5) puts 0.52 with 1.0 (mean 0.76). The dark group
        // (five 0.4 and one 0.0) has mean 0.333, which is nearer, so 2-means moves
        // 0.52 to the dark group and stops when the split no longer changes.
        let samples = [0.4, 0.4, 0.4, 0.0, 1.0, 0.52, 0.4, 0.4].map(|v| [v; 3]);
        let (pattern, fg, bg) = fit(&samples).unwrap();
        assert_eq!(pattern, 0b0001_0000);
        assert_eq!(fg[0], 1.0);
        assert!((bg[0] - 2.52 / 7.0).abs() < 1e-6);
        // Equal samples leave one group empty: one colour, no glyph.
        assert!(fit(&[[0.3; 3]; 4]).is_none());
    }

    #[test]
    fn clipping_keeps_original_sampling_and_outside_cells() {
        let pixels: Vec<_> = (0..32)
            .map(|i| {
                if i % 8 < 4 {
                    Rgb::new(1.0, 1.0, 1.0)
                } else {
                    Rgb::BLACK
                }
            })
            .collect();
        let raster = Raster::new(&pixels, 8, 4).unwrap();
        let area = Rect::new(2, 3, 4, 2);
        let mut full = Buffer::empty(Rect::new(0, 0, 8, 8));
        encode_image(raster, Encoder::Octant, area, &mut full);
        let mut clipped = Buffer::empty(Rect::new(4, 4, 3, 2));
        encode_image(raster, Encoder::Octant, area, &mut clipped);
        for x in 4..6 {
            assert_eq!(clipped[(x, 4)], full[(x, 4)]);
        }
        assert_eq!(clipped[(6, 4)], Cell::default());
        let before = clipped.clone();
        encode_image(raster, Encoder::Octant, Rect::new(0, 0, 0, 1), &mut clipped);
        encode_image(raster, Encoder::Octant, Rect::new(0, 0, 1, 1), &mut clipped);
        assert_eq!(clipped, before);
    }

    #[test]
    fn fractional_box_sampling_is_linear_and_handles_hdr() {
        let p = [Rgb::BLACK, Rgb::new(1.0, 1.0, 1.0)];
        let raster = Raster::new(&p, 2, 1).unwrap();
        assert!((box_average(raster, 0.5, 0.0, 1.0, 1.0).r - 0.5).abs() < 1e-6);
        let p = [Rgb::new(f32::MAX, f32::MAX, f32::MAX); 4];
        assert_eq!(
            box_average(Raster::new(&p, 2, 2).unwrap(), 0.0, 0.0, 2.0, 2.0).r,
            f32::MAX
        );
    }
}
