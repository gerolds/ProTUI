use ratatui::buffer::{Buffer, CellDiffOption};
use ratatui::layout::Rect;

use super::*;
use crate::Rgb;

/// Decoded image: size in device pixels, colours in RGB100 per pixel (`None` = not drawn).
struct Decoded {
    p2: u8,
    size: (usize, usize),
    pixels: Vec<Option<[u16; 3]>>,
    defined: usize,
}

/// A minimal sixel reader for the subset the encoder writes: header, raster attributes, colour
/// definitions and selections, repeats, `$` and `-`.
fn decode(bytes: &[u8]) -> Decoded {
    assert!(bytes.starts_with(b"\x1bP0;"), "DCS header");
    assert!(bytes.ends_with(b"\x1b\\"), "string terminator");
    let p2 = bytes[4] - b'0';
    assert_eq!(bytes[5], b'q');
    let body = &bytes[6..bytes.len() - 2];
    let mut i = 0;
    let number = |i: &mut usize| {
        let start = *i;
        while *i < body.len() && body[*i].is_ascii_digit() {
            *i += 1;
        }
        std::str::from_utf8(&body[start..*i])
            .unwrap()
            .parse::<usize>()
            .unwrap()
    };
    assert_eq!(body[i], b'"', "raster attributes first");
    i += 1;
    let mut attrs = [0; 4];
    for (n, a) in attrs.iter_mut().enumerate() {
        if n > 0 {
            assert_eq!(body[i], b';');
            i += 1;
        }
        *a = number(&mut i);
    }
    let (aspect, size) = (attrs[0] / attrs[1], (attrs[2], attrs[3]));
    let mut pixels = vec![None; size.0 * size.1];
    let mut colors = [None; 256];
    let (mut defined, mut current, mut x, mut band) = (0, 0, 0, 0);
    while i < body.len() {
        match body[i] {
            b'#' => {
                i += 1;
                current = number(&mut i);
                if body.get(i) == Some(&b';') {
                    i += 1;
                    assert_eq!(number(&mut i), 2, "RGB colour model");
                    let mut rgb = [0u16; 3];
                    for v in &mut rgb {
                        assert_eq!(body[i], b';');
                        i += 1;
                        *v = number(&mut i) as u16;
                    }
                    colors[current] = Some(rgb);
                    defined += 1;
                }
            }
            b'$' => {
                x = 0;
                i += 1;
            }
            b'-' => {
                x = 0;
                band += 1;
                i += 1;
            }
            b'!' | b'?'..=b'~' => {
                let count = if body[i] == b'!' {
                    i += 1;
                    number(&mut i)
                } else {
                    1
                };
                let bits = body[i] - b'?';
                i += 1;
                for _ in 0..count {
                    for bit in 0..6 {
                        if bits & (1 << bit) == 0 {
                            continue;
                        }
                        for dy in 0..aspect {
                            let y = (band * 6 + bit) * aspect + dy;
                            assert!(x < size.0 && y < size.1, "pixel {x},{y} outside {size:?}");
                            pixels[y * size.0 + x] = Some(colors[current].expect("defined"));
                        }
                    }
                    x += 1;
                }
            }
            other => panic!("unexpected byte {other}"),
        }
    }
    Decoded {
        p2,
        size,
        pixels,
        defined,
    }
}

/// The RGB100 colour the encoder sends for an sRGB colour (after the display round trip).
fn rgb100(srgb: [u8; 3]) -> [u16; 3] {
    crate::color::display(Rgb::from_srgb8(srgb))
        .map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u32)
        .map(|v| ((v * 100 + 127) / 255) as u16)
}

const RED: [u8; 3] = [200, 30, 20];
const BLUE: [u8; 3] = [20, 40, 180];
const GREY: [u8; 3] = [90, 90, 90];

/// A `w` × `h` raster of three vertical stripes, in exact sRGB bytes below the tone knee.
fn stripes(w: usize, h: usize) -> Vec<Rgb> {
    (0..w * h)
        .map(|i| {
            let x = i % w;
            Rgb::from_srgb8(match x * 3 / w {
                0 => RED,
                1 => BLUE,
                _ => GREY,
            })
        })
        .collect()
}

fn options(palette: Palette, scale: (u16, u16)) -> SixelOptions {
    SixelOptions { palette, scale }
}

#[test]
fn adaptive_round_trip_restores_every_pixel() {
    let (w, h) = (13, 7);
    let pixels = stripes(w, h);
    let raster = Raster::new(&pixels, w, h).unwrap();
    let mut encoder = SixelEncoder::new();
    let mut out = Vec::new();
    let adaptive = Palette::Adaptive { colors: 16 };
    encoder
        .encode(raster, &options(adaptive, (1, 1)), &mut out)
        .unwrap();
    let decoded = decode(&out);
    assert_eq!(decoded.p2, 0, "opaque background");
    assert_eq!(decoded.size, (w, h));
    assert_eq!(encoder.colors(), 3);
    assert_eq!(decoded.defined, 3);
    for (i, pixel) in decoded.pixels.iter().enumerate() {
        let want = match (i % w) * 3 / w {
            0 => RED,
            1 => BLUE,
            _ => GREY,
        };
        assert_eq!(*pixel, Some(rgb100(want)), "pixel {i}");
    }
}

#[test]
fn scale_repeats_across_and_sets_the_pixel_aspect() {
    let (w, h) = (6, 8);
    let pixels = stripes(w, h);
    let raster = Raster::new(&pixels, w, h).unwrap();
    let mut out = Vec::new();
    let adaptive = Palette::Adaptive { colors: 4 };
    SixelEncoder::new()
        .encode(raster, &options(adaptive, (5, 5)), &mut out)
        .unwrap();
    assert!(out.starts_with(b"\x1bP0;0q\"5;1;30;40"), "{out:?}");
    let decoded = decode(&out);
    assert_eq!(decoded.size, (30, 40));
    for y in 0..40 {
        for x in 0..30 {
            let want = match (x / 5) * 3 / w {
                0 => RED,
                1 => BLUE,
                _ => GREY,
            };
            assert_eq!(decoded.pixels[y * 30 + x], Some(rgb100(want)), "{x},{y}");
        }
    }
}

#[test]
fn runs_compress_to_repeat_introducers() {
    let pixels = vec![Rgb::from_srgb8(GREY); 10];
    let raster = Raster::new(&pixels, 10, 1).unwrap();
    let mut out = Vec::new();
    let adaptive = Palette::Adaptive { colors: 2 };
    SixelEncoder::new()
        .encode(raster, &options(adaptive, (1, 1)), &mut out)
        .unwrap();
    // One row sets bit 0 ('@'); three or fewer repeats are written out.
    assert_eq!(out, b"\x1bP0;0q\"1;1;10;1#0;2;35;35;35#0!10@\x1b\\");
    out.clear();
    let short = Raster::new(&pixels[..3], 3, 1).unwrap();
    SixelEncoder::new()
        .encode(short, &options(adaptive, (1, 1)), &mut out)
        .unwrap();
    assert!(out.ends_with(b"#0@@@\x1b\\"), "{out:?}");
}

#[test]
fn fixed_palette_dithers_between_neighbouring_cube_levels() {
    let (w, h) = (8, 8);
    let pixels = vec![Rgb::from_srgb8([128, 128, 128]); w * h];
    let raster = Raster::new(&pixels, w, h).unwrap();
    let mut encoder = SixelEncoder::new();
    let mut out = Vec::new();
    encoder
        .encode(raster, &options(Palette::Fixed, (1, 1)), &mut out)
        .unwrap();
    let decoded = decode(&out);
    assert!(decoded.pixels.iter().all(Option::is_some));
    assert!((2..=8).contains(&encoder.colors()), "{}", encoder.colors());
    let mean: f64 = decoded
        .pixels
        .iter()
        .map(|p| f64::from(p.unwrap()[1]))
        .sum::<f64>()
        / (w * h) as f64;
    // Green has 7 levels; the dither keeps the mean near 128/255 = 50 %.
    assert!((mean - 50.2).abs() < 4.0, "mean green {mean}");
}

#[test]
fn masked_pixels_stay_transparent() {
    let (w, h) = (4, 2);
    let pixels = stripes(w, h);
    let raster = Raster::new(&pixels, w, h).unwrap();
    let mask: Vec<bool> = (0..w * h).map(|i| i % 2 == 0).collect();
    let mut out = Vec::new();
    SixelEncoder::new()
        .encode_masked(raster, &mask, &options(Palette::Fixed, (1, 1)), &mut out)
        .unwrap();
    let decoded = decode(&out);
    assert_eq!(decoded.p2, 1, "transparent background");
    for (i, pixel) in decoded.pixels.iter().enumerate() {
        assert_eq!(pixel.is_some(), mask[i], "pixel {i}");
    }
}

#[test]
fn bad_input_is_refused() {
    let pixels = stripes(2, 2);
    let raster = Raster::new(&pixels, 2, 2).unwrap();
    let mut encoder = SixelEncoder::new();
    let mut out = Vec::new();
    let fixed = |scale| options(Palette::Fixed, scale);
    assert_eq!(
        encoder.encode(raster, &fixed((0, 1)), &mut out),
        Err(SixelError::Scale((0, 1)))
    );
    let one = options(Palette::Adaptive { colors: 1 }, (1, 1));
    assert_eq!(
        encoder.encode(raster, &one, &mut out),
        Err(SixelError::Colors(1))
    );
    assert_eq!(
        encoder.encode_masked(raster, &[true], &fixed((1, 1)), &mut out),
        Err(SixelError::Mask { pixels: 4, mask: 1 })
    );
    assert!(out.is_empty());
}

#[test]
fn reused_encoder_repeats_its_output_without_growing() {
    let big = stripes(40, 13);
    let small = stripes(7, 3);
    let big = Raster::new(&big, 40, 13).unwrap();
    let small = Raster::new(&small, 7, 3).unwrap();
    let mut encoder = SixelEncoder::new();
    for palette in [Palette::Fixed, Palette::Adaptive { colors: 256 }] {
        let opts = options(palette, (2, 3));
        let mut first = Vec::new();
        encoder.encode(big, &opts, &mut first).unwrap();
        let mut other = Vec::new();
        encoder.encode(small, &opts, &mut other).unwrap();
        let mut out = Vec::with_capacity(first.len());
        let capacity = out.capacity();
        encoder.encode(big, &opts, &mut out).unwrap();
        assert_eq!(out, first, "{palette:?}");
        assert_eq!(out.capacity(), capacity);
    }
}

fn grey(w: usize, h: usize) -> Vec<Rgb> {
    vec![Rgb::from_srgb8(GREY); w * h]
}

const PALETTE: Palette = Palette::Adaptive { colors: 8 };

#[test]
fn placing_skips_exactly_the_rect_and_queues_one_image() {
    let area = Rect::new(0, 0, 10, 6);
    let mut buf = Buffer::empty(area);
    let mut layer = SixelLayer::new(WT_CELL_PX);
    let rect = Rect::new(2, 1, 3, 2);
    let pixels = grey(6, 8);
    layer.begin(area);
    layer
        .place(
            &mut buf,
            1,
            0,
            rect,
            Raster::new(&pixels, 6, 8).unwrap(),
            PALETTE,
        )
        .unwrap();
    layer.end(&mut buf);
    for y in 0..6 {
        for x in 0..10 {
            let want = if rect.contains((x, y).into()) {
                CellDiffOption::Skip
            } else {
                CellDiffOption::None
            };
            assert_eq!(buf[(x, y)].diff_option, want, "{x},{y}");
        }
    }
    // 3 × 2 cells of 10 × 20 device pixels from a 6 × 8 raster: scale 5 × 5.
    assert!(
        layer
            .pending()
            .starts_with(b"\x1b[2;3H\x1bP0;0q\"5;1;30;40"),
        "{:?}",
        String::from_utf8_lossy(layer.pending())
    );
}

#[test]
fn unchanged_images_are_not_resent_until_invalidated_or_resized() {
    let area = Rect::new(0, 0, 10, 6);
    let rect = Rect::new(0, 0, 2, 1);
    let pixels = grey(2, 1);
    let raster = Raster::new(&pixels, 2, 1).unwrap();
    let mut layer = SixelLayer::new(WT_CELL_PX);
    let frame = |layer: &mut SixelLayer, area: Rect, revision: u64| {
        let mut buf = Buffer::empty(area);
        layer.begin(area);
        layer
            .place(&mut buf, 7, revision, rect, raster, PALETTE)
            .unwrap();
        layer.end(&mut buf);
        assert_eq!(buf[(1, 0)].diff_option, CellDiffOption::Skip);
        layer.pending().len()
    };
    assert!(frame(&mut layer, area, 0) > 0);
    assert_eq!(frame(&mut layer, area, 0), 0, "same id, revision, rect");
    assert!(frame(&mut layer, area, 1) > 0, "new revision");
    layer.invalidate();
    assert!(frame(&mut layer, area, 1) > 0, "after clear");
    assert!(
        frame(&mut layer, Rect::new(0, 0, 12, 6), 1) > 0,
        "after resize"
    );
}

#[test]
fn cells_an_image_leaves_are_redrawn_once() {
    let area = Rect::new(0, 0, 8, 4);
    let pixels = grey(2, 1);
    let raster = Raster::new(&pixels, 2, 1).unwrap();
    let mut layer = SixelLayer::new(WT_CELL_PX);
    let mut buf = Buffer::empty(area);
    layer.begin(area);
    layer
        .place(&mut buf, 1, 0, Rect::new(0, 0, 2, 1), raster, PALETTE)
        .unwrap();
    layer.end(&mut buf);
    // Moved one cell right: column 0 is left, column 1 stays covered.
    let mut buf = Buffer::empty(area);
    layer.begin(area);
    layer
        .place(&mut buf, 1, 0, Rect::new(1, 0, 2, 1), raster, PALETTE)
        .unwrap();
    layer.end(&mut buf);
    assert!(!layer.pending().is_empty(), "moved image is re-sent");
    assert_eq!(buf[(0, 0)].diff_option, CellDiffOption::AlwaysUpdate);
    assert_eq!(buf[(1, 0)].diff_option, CellDiffOption::Skip);
    assert_eq!(buf[(2, 0)].diff_option, CellDiffOption::Skip);
    // Removed: both cells are redrawn, nothing is queued.
    let mut buf = Buffer::empty(area);
    layer.begin(area);
    layer.end(&mut buf);
    assert!(layer.pending().is_empty());
    assert_eq!(buf[(1, 0)].diff_option, CellDiffOption::AlwaysUpdate);
    assert_eq!(buf[(2, 0)].diff_option, CellDiffOption::AlwaysUpdate);
    assert_eq!(buf[(0, 0)].diff_option, CellDiffOption::None);
    // Gone for good: the next frame leaves the cells alone.
    let mut buf = Buffer::empty(area);
    layer.begin(area);
    layer.end(&mut buf);
    assert_eq!(buf[(1, 0)].diff_option, CellDiffOption::None);
}

#[test]
fn an_image_on_the_bottom_row_is_cropped_to_whole_bands() {
    // 2 rows of 20 px = 40 px; at scale 1 bands are 6 px, so 36 px (6 bands) fit.
    let area = Rect::new(0, 0, 4, 5);
    let rect = Rect::new(0, 3, 1, 2);
    let pixels = grey(10, 40);
    let raster = Raster::new(&pixels, 10, 40).unwrap();
    let mut layer = SixelLayer::new(WT_CELL_PX);
    let mut buf = Buffer::empty(area);
    layer.begin(area);
    layer.place(&mut buf, 1, 0, rect, raster, PALETTE).unwrap();
    layer.end(&mut buf);
    let image = &layer.pending()[b"\x1b[4;1H".len()..];
    assert_eq!(decode(image).size, (10, 36));
    // Above the bottom row the whole image goes out; its last band ends in the row below.
    let mut buf = Buffer::empty(Rect::new(0, 0, 4, 6));
    layer.begin(Rect::new(0, 0, 4, 6));
    layer.place(&mut buf, 1, 0, rect, raster, PALETTE).unwrap();
    layer.end(&mut buf);
    assert_eq!(decode(&layer.pending()[6..]).size, (10, 40));
}

#[test]
fn misfit_placements_are_refused_without_marking() {
    let area = Rect::new(0, 0, 4, 4);
    let mut buf = Buffer::empty(area);
    let pixels = grey(3, 4);
    let raster = Raster::new(&pixels, 3, 4).unwrap();
    let mut layer = SixelLayer::new(WT_CELL_PX);
    layer.begin(area);
    assert_eq!(
        layer.place(&mut buf, 1, 0, Rect::new(0, 0, 2, 1), raster, PALETTE),
        Err(PlaceError::Scale {
            raster: (3, 4),
            device: (20, 20)
        })
    );
    assert_eq!(
        layer.place(&mut buf, 1, 0, Rect::new(3, 3, 2, 1), raster, PALETTE),
        Err(PlaceError::Outside(Rect::new(3, 3, 2, 1)))
    );
    let fits = Rect::new(0, 0, 3, 1);
    let pixels = grey(6, 4);
    let raster = Raster::new(&pixels, 6, 4).unwrap();
    layer.place(&mut buf, 1, 0, fits, raster, PALETTE).unwrap();
    assert_eq!(
        layer.place(&mut buf, 1, 0, fits, raster, PALETTE),
        Err(PlaceError::Duplicate(1))
    );
    assert!(
        buf.content
            .iter()
            .skip(3)
            .all(|c| c.diff_option == CellDiffOption::None)
    );
}
