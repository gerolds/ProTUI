// ADR: One cache entry per panel holds the centre crop of one image at one cell size and aspect, box-averaged in linear light to 2 x 4 samples per cell — adapted from Runner painting.rs without its light states.
// ADR: A tint multiplies the cached samples into a second reused buffer, so a changing tint (flicker, fades) never resamples the source.
use std::sync::Arc;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::encode::box_average;
use crate::image::pixel_count;
use crate::{Encoder, ImageError, Raster, Rgb, StillImage, encode_image};

/// Centre-cropped still-image sampler. A cell is `cell_aspect` times as tall as
/// wide (default 2.0). Keeps one image/size/aspect result, sampled at 2 x 4 per
/// cell, shared by both encoders. Repeated draws at a settled size allocate no
/// raster storage and do not resample the source; a new tint only multiplies the
/// cached samples again. Use one cache per image panel.
pub struct ImageCache {
    source: Option<StillImage>,
    size: (u16, u16),
    cell_aspect: f32,
    pixels: Vec<Rgb>,
    /// `pixels` times `tint`; stale when `tint` is `None`.
    tinted: Vec<Rgb>,
    tint: Option<Rgb>,
}
impl Default for ImageCache {
    fn default() -> Self {
        Self {
            source: None,
            size: (0, 0),
            cell_aspect: 2.0,
            pixels: Vec::new(),
            tinted: Vec::new(),
            tint: None,
        }
    }
}
impl ImageCache {
    pub fn cell_aspect(&self) -> f32 {
        self.cell_aspect
    }

    /// Refuses invalid input without changing the cache. Changing the aspect
    /// invalidates the sample but retains its allocation for reuse.
    pub fn set_cell_aspect(&mut self, aspect: f32) -> Result<(), ImageError> {
        if !aspect.is_finite() || aspect <= 0.0 {
            return Err(ImageError::InvalidAspect);
        }
        if self.cell_aspect != aspect {
            self.cell_aspect = aspect;
            self.source = None;
        }
        Ok(())
    }

    /// Encodes the centre crop of `image` over `area`, each linear sample
    /// multiplied by `tint` (`Rgb::WHITE` leaves it unchanged; channels above
    /// one brighten and are tone mapped at output). A negative or nonfinite tint
    /// is refused. An area outside `buffer` draws nothing.
    pub fn draw(
        &mut self,
        image: &StillImage,
        tint: Rgb,
        encoder: Encoder,
        area: Rect,
        buffer: &mut Buffer,
    ) -> Result<(), ImageError> {
        if !tint.valid() {
            return Err(ImageError::InvalidTint);
        }
        if area.intersection(buffer.area).is_empty() {
            return Ok(());
        }
        let (width, height) = self.sample(image, area.width, area.height)?.size();
        if tint == Rgb::WHITE {
            encode_image(self.raster(width, height), encoder, area, buffer);
            return Ok(());
        }
        if self.tint != Some(tint) {
            self.tinted.clear();
            self.tinted
                .try_reserve_exact(self.pixels.len())
                .map_err(|_| ImageError::Allocation)?;
            self.tinted.extend(self.pixels.iter().map(|&p| p * tint));
            self.tint = Some(tint);
        }
        let tinted = Raster {
            pixels: &self.tinted,
            width,
            height,
        };
        encode_image(tinted, encoder, area, buffer);
        Ok(())
    }

    fn raster(&self, width: usize, height: usize) -> Raster<'_> {
        Raster {
            pixels: &self.pixels,
            width,
            height,
        }
    }

    /// Returns the cached center crop. Empty output dimensions are errors.
    /// Position is deliberately not part of the cache key.
    pub fn sample(
        &mut self,
        image: &StillImage,
        columns: u16,
        rows: u16,
    ) -> Result<Raster<'_>, ImageError> {
        let (width, height) = (2 * usize::from(columns), 4 * usize::from(rows));
        let count = pixel_count(width, height)?;
        let matches = self.size == (columns, rows)
            && self
                .source
                .as_ref()
                .is_some_and(|source| Arc::ptr_eq(&source.pixels, &image.pixels));
        if !matches {
            if count > self.pixels.capacity() {
                self.pixels
                    .try_reserve_exact(count.saturating_sub(self.pixels.len()))
                    .map_err(|_| ImageError::Allocation)?;
            }
            self.source = None;
            self.tint = None;
            self.pixels.clear();
            let raster = image.raster();
            let shape = f64::from(columns) / (f64::from(rows) * f64::from(self.cell_aspect));
            let (mut crop_w, mut crop_h) = (raster.width as f64, raster.height as f64);
            if crop_w / crop_h > shape {
                crop_w = crop_h * shape;
            } else {
                crop_h = crop_w / shape;
            }
            let origin_x = (raster.width as f64 - crop_w) / 2.0;
            let origin_y = (raster.height as f64 - crop_h) / 2.0;
            let step_x = crop_w / width as f64;
            let step_y = crop_h / height as f64;
            for y in 0..height {
                for x in 0..width {
                    self.pixels.push(box_average(
                        raster,
                        origin_x + x as f64 * step_x,
                        origin_y + y as f64 * step_y,
                        step_x,
                        step_y,
                    ));
                }
            }
            self.size = (columns, rows);
            self.source = Some(image.clone());
        }
        Ok(self.raster(width, height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disc(width: usize, height: usize) -> StillImage {
        let pixels = (0..width * height)
            .map(|i| {
                let dx = (i % width) as f64 + 0.5 - width as f64 / 2.0;
                let dy = (i / width) as f64 + 0.5 - height as f64 / 2.0;
                if dx * dx + dy * dy < 40.0 * 40.0 {
                    Rgb::new(0.8, 0.7, 0.6)
                } else {
                    Rgb::BLACK
                }
            })
            .collect();
        StillImage::new(pixels, width, height).unwrap()
    }

    #[test]
    fn crop_preserves_disc_proportions_at_both_cell_shapes() {
        for (w, h) in [(256, 256), (256, 171)] {
            let image = disc(w, h);
            for aspect in [2.0, 18.0 / 7.0] {
                for (columns, rows) in [(40, 8), (12, 10)] {
                    let mut cache = ImageCache::default();
                    cache.set_cell_aspect(aspect).unwrap();
                    let raster = cache.sample(&image, columns, rows).unwrap();
                    let lit: Vec<_> = raster
                        .pixels
                        .iter()
                        .enumerate()
                        .filter(|(_, p)| p.luminance() > 0.2)
                        .map(|(i, _)| (i % raster.width, i / raster.width))
                        .collect();
                    let across = lit.iter().map(|p| p.0).max().unwrap()
                        - lit.iter().map(|p| p.0).min().unwrap()
                        + 1;
                    let down = lit.iter().map(|p| p.1).max().unwrap()
                        - lit.iter().map(|p| p.1).min().unwrap()
                        + 1;
                    let ratio = (across as f32 / 2.0) / (down as f32 * aspect / 4.0);
                    assert!(
                        (ratio - 1.0).abs() < 0.15,
                        "{w}x{h}, {aspect}, {columns}x{rows}: {ratio}"
                    );
                }
            }
        }
    }

    #[test]
    fn cache_reuses_pixels_and_invalidates_for_source_size_and_aspect() {
        let image = disc(128, 128);
        let mut cache = ImageCache::default();
        let first = cache.sample(&image, 12, 6).unwrap().pixels.to_vec();
        let pointer = cache.pixels.as_ptr();
        let capacity = cache.pixels.capacity();
        assert_eq!(cache.sample(&image.clone(), 12, 6).unwrap().pixels, first);
        assert_eq!(cache.pixels.as_ptr(), pointer);
        assert_eq!(cache.pixels.capacity(), capacity);
        cache.set_cell_aspect(18.0 / 7.0).unwrap();
        assert_ne!(cache.sample(&image, 12, 6).unwrap().pixels, first);
        assert_eq!(cache.pixels.as_ptr(), pointer);
        let black = StillImage::new(vec![Rgb::BLACK; 4], 2, 2).unwrap();
        assert!(
            cache
                .sample(&black, 12, 6)
                .unwrap()
                .pixels
                .iter()
                .all(|p| *p == Rgb::BLACK)
        );
        assert_eq!(cache.sample(&black, 8, 3).unwrap().size(), (16, 12));
        for invalid in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(
                cache.set_cell_aspect(invalid),
                Err(ImageError::InvalidAspect)
            );
        }
        assert_eq!(cache.cell_aspect(), 18.0 / 7.0);
        assert_eq!(cache.sample(&image, 0, 3).unwrap_err(), ImageError::Empty);
    }

    #[test]
    fn tint_multiplies_cached_samples_without_resampling() {
        let image = disc(64, 64);
        let area = Rect::new(0, 0, 6, 3);
        let mut buffer = Buffer::empty(area);
        let mut cache = ImageCache::default();
        cache
            .draw(&image, Rgb::WHITE, Encoder::Octant, area, &mut buffer)
            .unwrap();
        // A marker only survives if the source is not sampled again.
        let marker = Rgb::new(0.2, 0.4, 0.8);
        cache.pixels[0] = marker;
        let tint = Rgb::new(1.5, 1.0, 0.5);
        cache
            .draw(&image, tint, Encoder::Quadrant, area, &mut buffer)
            .unwrap();
        assert_eq!(cache.tinted[0], marker * tint);
        let expected: Vec<_> = cache.pixels.iter().map(|&p| p * tint).collect();
        assert_eq!(cache.tinted, expected);
        let mut direct = Buffer::empty(area);
        encode_image(
            Raster::new(&expected, 12, 12).unwrap(),
            Encoder::Quadrant,
            area,
            &mut direct,
        );
        assert_eq!(buffer, direct);
        // The same tint again reuses the tinted buffer; a new size resamples.
        let pointer = cache.tinted.as_ptr();
        cache
            .draw(&image, tint, Encoder::Octant, area, &mut buffer)
            .unwrap();
        assert_eq!(cache.tinted.as_ptr(), pointer);
        cache
            .draw(
                &image,
                tint,
                Encoder::Octant,
                Rect::new(0, 0, 4, 2),
                &mut buffer,
            )
            .unwrap();
        assert_ne!(cache.pixels[0], marker);
        for invalid in [Rgb::new(-0.1, 1.0, 1.0), Rgb::new(f32::NAN, 1.0, 1.0)] {
            assert_eq!(
                cache.draw(&image, invalid, Encoder::Octant, area, &mut buffer),
                Err(ImageError::InvalidTint)
            );
        }
    }
}
