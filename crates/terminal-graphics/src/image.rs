use std::fmt;
use std::io::{BufRead, Seek};
use std::sync::Arc;

use crate::Rgb;

/// Invalid shape or pixel data. Validation happens once, at the input boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageError {
    Empty,
    Overflow,
    Size {
        width: usize,
        height: usize,
        pixels: usize,
    },
    InvalidPixel {
        index: usize,
    },
    NonAscii {
        row: usize,
    },
    UnequalRow {
        row: usize,
        expected: usize,
        actual: usize,
    },
    UnknownSymbol {
        row: usize,
        column: usize,
        symbol: u8,
    },
    DuplicateSymbol {
        symbol: u8,
    },
    InvalidAspect,
    InvalidTint,
    InvalidInk,
    InvalidView,
    Allocation,
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "an image must have nonzero width and height"),
            Self::Overflow => write!(f, "image dimensions overflow addressable storage"),
            Self::Size {
                width,
                height,
                pixels,
            } => write!(f, "{pixels} pixels do not fill {width} x {height}"),
            Self::InvalidPixel { index } => {
                write!(f, "pixel {index} has a negative or nonfinite channel")
            }
            Self::NonAscii { row } => write!(f, "row {row} is not ASCII"),
            Self::UnequalRow {
                row,
                expected,
                actual,
            } => write!(f, "row {row} has width {actual}, expected {expected}"),
            Self::UnknownSymbol {
                row,
                column,
                symbol,
            } => write!(f, "no palette entry for byte {symbol} at {column},{row}"),
            Self::DuplicateSymbol { symbol } => write!(f, "duplicate palette byte {symbol}"),
            Self::InvalidAspect => write!(f, "cell height/width must be positive and finite"),
            Self::InvalidTint => write!(f, "a tint channel is negative or nonfinite"),
            Self::InvalidInk => write!(
                f,
                "an ink colour channel or alpha is outside 0..=1 or nonfinite"
            ),
            Self::InvalidView => write!(
                f,
                "a sketch view needs finite bounds with right > left and bottom > top"
            ),
            Self::Allocation => write!(f, "image storage could not be allocated"),
        }
    }
}
impl std::error::Error for ImageError {}

/// A validated, borrowed row-major linear-light raster.
#[derive(Clone, Copy, Debug)]
pub struct Raster<'a> {
    pub(crate) pixels: &'a [Rgb],
    pub(crate) width: usize,
    pub(crate) height: usize,
}

impl<'a> Raster<'a> {
    pub fn new(pixels: &'a [Rgb], width: usize, height: usize) -> Result<Self, ImageError> {
        let count = pixel_count(width, height)?;
        if pixels.len() != count {
            return Err(ImageError::Size {
                width,
                height,
                pixels: pixels.len(),
            });
        }
        if let Some(index) = pixels.iter().position(|p| !p.valid()) {
            return Err(ImageError::InvalidPixel { index });
        }
        Ok(Self {
            pixels,
            width,
            height,
        })
    }

    pub fn size(self) -> (usize, usize) {
        (self.width, self.height)
    }
    pub fn pixels(self) -> &'a [Rgb] {
        self.pixels
    }

    /// The first `rows` rows (at most the height, at least one).
    #[cfg(feature = "sixel")]
    pub(crate) fn top_rows(self, rows: usize) -> Self {
        let height = rows.clamp(1, self.height);
        Self {
            pixels: &self.pixels[..self.width * height],
            width: self.width,
            height,
        }
    }
}

pub(crate) fn pixel_count(width: usize, height: usize) -> Result<usize, ImageError> {
    if width == 0 || height == 0 {
        return Err(ImageError::Empty);
    }
    let count = width.checked_mul(height).ok_or(ImageError::Overflow)?;
    if count > isize::MAX as usize / std::mem::size_of::<Rgb>() {
        return Err(ImageError::Overflow);
    }
    Ok(count)
}

/// Immutable still image. Clones share pixels; cache identity is stable across clones.
#[derive(Clone, Debug)]
pub struct StillImage {
    pub(crate) pixels: Arc<[Rgb]>,
    width: usize,
    height: usize,
}

impl StillImage {
    pub fn new(pixels: Vec<Rgb>, width: usize, height: usize) -> Result<Self, ImageError> {
        Raster::new(&pixels, width, height)?;
        Ok(Self {
            pixels: pixels.into(),
            width,
            height,
        })
    }

    pub fn raster(&self) -> Raster<'_> {
        Raster {
            pixels: &self.pixels,
            width: self.width,
            height: self.height,
        }
    }

    /// Each ASCII byte becomes one pixel. Palette semantics belong to the caller;
    /// rows must be rectangular, and unknown/duplicate symbols are errors.
    /// A final newline and CRLF line endings are accepted; empty rows are not trimmed.
    pub fn from_ascii(source: &str, palette: &[(u8, Rgb)]) -> Result<Self, ImageError> {
        let width = source.lines().next().ok_or(ImageError::Empty)?.len();
        let height = source.lines().count();
        let count = pixel_count(width, height)?;
        let mut lookup = [None; 128];
        for &(symbol, color) in palette {
            if !symbol.is_ascii() {
                return Err(ImageError::NonAscii { row: 0 });
            }
            if !color.valid() {
                return Err(ImageError::InvalidPixel {
                    index: usize::from(symbol),
                });
            }
            if lookup[usize::from(symbol)].replace(color).is_some() {
                return Err(ImageError::DuplicateSymbol { symbol });
            }
        }
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(count)
            .map_err(|_| ImageError::Allocation)?;
        for (row, line) in source.lines().enumerate() {
            if !line.is_ascii() {
                return Err(ImageError::NonAscii { row });
            }
            if line.len() != width {
                return Err(ImageError::UnequalRow {
                    row,
                    expected: width,
                    actual: line.len(),
                });
            }
            for (column, symbol) in line.bytes().enumerate() {
                pixels.push(
                    lookup[usize::from(symbol)].ok_or(ImageError::UnknownSymbol {
                        row,
                        column,
                        symbol,
                    })?,
                );
            }
        }
        Self::new(pixels, width, height)
    }

    /// Decodes the first PNG frame into linear light, compositing straight alpha
    /// over the caller's explicit linear background. Palette/low-bit grey expand;
    /// 16-bit channels are reduced to their high eight bits, as in Runner.
    /// PNG samples are interpreted as sRGB; embedded color profiles are not applied.
    /// Images above 16,777,216 pixels are refused before output allocation.
    pub fn from_png<R: BufRead + Seek>(input: R, background: Rgb) -> Result<Self, PngError> {
        if !background.valid() {
            return Err(PngError::Image(ImageError::InvalidPixel { index: 0 }));
        }
        let mut decoder = png::Decoder::new(input);
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().map_err(PngError::Decode)?;
        let (width, height) = (reader.info().width as usize, reader.info().height as usize);
        let count = pixel_count(width, height).map_err(PngError::Image)?;
        if count > 16_777_216 {
            return Err(PngError::TooLarge);
        }
        let size = reader.output_buffer_size().ok_or(PngError::TooLarge)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| PngError::Image(ImageError::Allocation))?;
        bytes.resize(size, 0);
        let info = reader.next_frame(&mut bytes).map_err(PngError::Decode)?;
        if info.bit_depth != png::BitDepth::Eight || info.color_type == png::ColorType::Indexed {
            return Err(PngError::Unsupported);
        }
        let channels = info.color_type.samples();
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(count)
            .map_err(|_| PngError::Image(ImageError::Allocation))?;
        for p in bytes[..info.buffer_size()].chunks_exact(channels) {
            let (color, alpha) = match info.color_type {
                png::ColorType::Grayscale => (Rgb::from_srgb8([p[0]; 3]), 1.0),
                png::ColorType::GrayscaleAlpha => {
                    (Rgb::from_srgb8([p[0]; 3]), f32::from(p[1]) / 255.0)
                }
                png::ColorType::Rgb => (Rgb::from_srgb8([p[0], p[1], p[2]]), 1.0),
                png::ColorType::Rgba => {
                    (Rgb::from_srgb8([p[0], p[1], p[2]]), f32::from(p[3]) / 255.0)
                }
                png::ColorType::Indexed => return Err(PngError::Unsupported),
            };
            pixels.push(color * alpha + background * (1.0 - alpha));
        }
        Self::new(pixels, width, height).map_err(PngError::Image)
    }
}

#[derive(Debug)]
pub enum PngError {
    Decode(png::DecodingError),
    Image(ImageError),
    TooLarge,
    Unsupported,
}
impl fmt::Display for PngError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(e) => write!(f, "PNG decoding failed: {e}"),
            Self::Image(e) => e.fmt(f),
            Self::TooLarge => write!(f, "PNG exceeds the decoder's storage limit"),
            Self::Unsupported => write!(f, "PNG did not expand to an eight-bit gray/RGB format"),
        }
    }
}
impl std::error::Error for PngError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Decode(e) => Some(e),
            Self::Image(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_validation_covers_shape_overflow_and_channels() {
        assert_eq!(Raster::new(&[], 0, 1).unwrap_err(), ImageError::Empty);
        assert_eq!(
            Raster::new(&[], usize::MAX, 2).unwrap_err(),
            ImageError::Overflow
        );
        assert!(matches!(
            Raster::new(&[Rgb::BLACK], 2, 1),
            Err(ImageError::Size { .. })
        ));
        for invalid in [f32::NAN, f32::INFINITY, -0.1] {
            assert_eq!(
                Raster::new(&[Rgb::new(invalid, 0.0, 0.0)], 1, 1).unwrap_err(),
                ImageError::InvalidPixel { index: 0 }
            );
        }
    }

    #[test]
    fn ascii_is_rectangular_and_palette_driven() {
        let palette = [(b'.', Rgb::BLACK), (b'#', Rgb::hex(0xffffff))];
        let image = StillImage::from_ascii(".#\r\n#.\r\n", &palette).unwrap();
        assert_eq!(image.raster().size(), (2, 2));
        assert_eq!(image.raster().pixels()[1], palette[1].1);
        assert!(matches!(
            StillImage::from_ascii("..\n.", &palette),
            Err(ImageError::UnequalRow { row: 1, .. })
        ));
        assert!(matches!(
            StillImage::from_ascii("?", &palette),
            Err(ImageError::UnknownSymbol { .. })
        ));
        assert!(matches!(
            StillImage::from_ascii("é", &palette),
            Err(ImageError::NonAscii { .. })
        ));
        assert!(matches!(
            StillImage::from_ascii(".", &[(b'.', Rgb::BLACK); 2]),
            Err(ImageError::DuplicateSymbol { .. })
        ));
    }

    #[test]
    fn png_decode_composites_alpha_in_linear_light() {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer
                .write_image_data(&[255, 0, 0, 128, 0, 255, 0, 0])
                .unwrap();
        }
        let image = StillImage::from_png(std::io::Cursor::new(bytes), Rgb::hex(0x0000ff)).unwrap();
        assert_eq!(image.raster().size(), (2, 1));
        let p = image.raster().pixels()[0];
        assert!((p.r - 128.0 / 255.0).abs() < 1e-6);
        assert!((p.b - 127.0 / 255.0).abs() < 1e-6);
        assert_eq!(image.raster().pixels()[1], Rgb::hex(0x0000ff));
        assert!(StillImage::from_png(std::io::Cursor::new(b"not a PNG"), Rgb::BLACK).is_err());
    }

    fn encode_png(
        width: u32,
        color: png::ColorType,
        depth: png::BitDepth,
        palette: Option<(&[u8], &[u8])>,
        data: &[u8],
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, width, 1);
            encoder.set_color(color);
            encoder.set_depth(depth);
            if let Some((rgb, alpha)) = palette {
                encoder.set_palette(rgb.to_vec());
                encoder.set_trns(alpha.to_vec());
            }
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(data).unwrap();
        }
        bytes
    }

    fn decode(bytes: Vec<u8>, background: Rgb) -> Vec<Rgb> {
        StillImage::from_png(std::io::Cursor::new(bytes), background)
            .unwrap()
            .raster()
            .pixels()
            .to_vec()
    }

    #[test]
    fn png_sixteen_bit_keeps_the_high_byte() {
        // 0x80FF and 0x8000 both reduce to 0x80; 0xFFFF to 0xFF.
        let bytes = encode_png(
            1,
            png::ColorType::Rgb,
            png::BitDepth::Sixteen,
            None,
            &[0x80, 0xFF, 0x80, 0x00, 0xFF, 0xFF],
        );
        let grey = Rgb::from_srgb8([0x80; 3]).r;
        assert_eq!(decode(bytes, Rgb::BLACK), [Rgb::new(grey, grey, 1.0)]);
        let bytes = encode_png(
            2,
            png::ColorType::GrayscaleAlpha,
            png::BitDepth::Sixteen,
            None,
            &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00],
        );
        assert_eq!(
            decode(bytes, Rgb::hex(0x00ff00)),
            [Rgb::WHITE, Rgb::hex(0x00ff00)]
        );
    }

    #[test]
    fn png_palette_and_low_bit_grey_expand() {
        // Two-bit indices 0, 1, 2 into a palette whose entry 2 is transparent.
        let bytes = encode_png(
            3,
            png::ColorType::Indexed,
            png::BitDepth::Two,
            Some((&[255, 0, 0, 0, 0, 255, 9, 9, 9], &[255, 255, 0])),
            &[0b0001_1000],
        );
        let background = Rgb::new(0.25, 0.5, 0.75);
        assert_eq!(
            decode(bytes, background),
            [Rgb::hex(0xff0000), Rgb::hex(0x0000ff), background]
        );
        // One-bit grey: 1 expands to 255, 0 to 0.
        let bytes = encode_png(
            2,
            png::ColorType::Grayscale,
            png::BitDepth::One,
            None,
            &[0b1000_0000],
        );
        assert_eq!(decode(bytes, Rgb::WHITE), [Rgb::WHITE, Rgb::BLACK]);
    }

    #[test]
    fn png_refuses_an_invalid_background() {
        let bytes = encode_png(
            1,
            png::ColorType::Grayscale,
            png::BitDepth::Eight,
            None,
            &[0],
        );
        assert!(matches!(
            StillImage::from_png(std::io::Cursor::new(bytes), Rgb::new(f32::NAN, 0.0, 0.0)),
            Err(PngError::Image(ImageError::InvalidPixel { .. }))
        ));
    }
}
