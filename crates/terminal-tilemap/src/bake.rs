// ADR: The bake is a static world-space albedo raster (16 px per tile) from per-pixel shaders driven by the legend: base colour × pattern × grain, then wall lip/depth or floor occlusion from the signed distance.
// ADR: Patterns are scalar tone modulations in world pixels, seeded by the place seed and the legend char, so equal tiles join seamlessly and a seed reproduces the bake exactly.
use terminal_graphics::Rgb;
use terminal_graphics::noise::{fbm, hash, smoothstep, unit};

use crate::geometry::Geometry;
use crate::place::{Pattern, Place, Surface, TILE_PX};

/// The static rasters of a place, row-major, one value per bake pixel.
pub(crate) struct Bake {
    pub albedo: Vec<Rgb>,
    /// Self-emitted light; empty when no legend entry emits.
    pub emissive: Vec<Rgb>,
}

pub(crate) fn bake(place: &Place, geo: &Geometry) -> Bake {
    let (w, h) = geo.size();
    let legend = place.legend();
    let looks: Vec<(Surface, u32, Rgb, Option<Rgb>)> = legend
        .iter()
        .map(|(glyph, def)| {
            let seed = hash(glyph as i32, 0, place.seed());
            let emissive = def.emissive.map(Rgb::from_srgb8);
            (
                def.surface,
                seed,
                Rgb::from_srgb8(def.surface.color),
                emissive,
            )
        })
        .collect();
    let emits = looks.iter().any(|look| look.3.is_some());
    let mut albedo = Vec::with_capacity(w * h);
    let mut emissive = Vec::with_capacity(if emits { w * h } else { 0 });
    let (sd, owner) = (geo.sd(), geo.owner());
    for i in 0..w * h {
        let (px, py) = ((i % w) as f32 + 0.5, (i / w) as f32 + 0.5);
        let (surface, seed, base, glow) = &looks[usize::from(owner[i])];
        let tone = shade(surface, *seed, px, py);
        let d = sd[i];
        let form = if d > 0.0 {
            // Wall top: a bright lip along the outline, darker deeper in.
            let lip = 1.0 - smoothstep(0.5, 2.0, d);
            (1.0 + 0.35 * lip) * (1.0 - 0.5 * smoothstep(2.0, 10.0, d))
        } else {
            // Floor: ambient occlusion near walls.
            0.35 + 0.65 * smoothstep(0.0, 12.0, -d)
        };
        albedo.push(*base * (tone * form));
        if emits {
            emissive.push(glow.map_or(Rgb::BLACK, |c| c * tone));
        }
    }
    Bake { albedo, emissive }
}

/// The surface's tone at world pixel `(px, py)`: pattern × grain, around 1.
fn shade(surface: &Surface, seed: u32, px: f32, py: f32) -> f32 {
    let pattern = match surface.pattern {
        Pattern::Flat => 1.0,
        Pattern::Noise => 0.7 + 0.6 * fbm(px / 14.0, py / 14.0, seed, 3),
        Pattern::Tiles => {
            let size = TILE_PX as f32 / 2.0;
            let (bx, by) = ((px / size).floor(), (py / size).floor());
            let (fx, fy) = (px / size - bx, py / size - by);
            let edge = fx.min(1.0 - fx).min(fy).min(1.0 - fy) * size;
            let tone = 0.88 + 0.24 * unit(bx as i32, by as i32, seed);
            tone * (0.62 + 0.38 * smoothstep(0.3, 1.1, edge))
        }
        Pattern::Planks => {
            let row_h = TILE_PX as f32 / 4.0;
            let row = (py / row_h).floor() as i32;
            let v = py / row_h - row as f32;
            let length = 14.0 + 14.0 * unit(row, 0, seed);
            let shifted = px + unit(row, 1, seed) * length;
            let plank = (shifted / length).floor() as i32;
            let u = shifted / length - plank as f32;
            let edge = (v.min(1.0 - v) * row_h).min(u.min(1.0 - u) * length);
            let tone = 0.85 + 0.3 * unit(plank, row, seed.wrapping_add(1));
            tone * (0.6 + 0.4 * smoothstep(0.2, 0.9, edge))
        }
    };
    let s = surface.grain_scale;
    let grain = 2.0 * fbm(px / s, py / s, seed.wrapping_add(7), 2) - 1.0;
    pattern * (1.0 + surface.grain * grain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::place::tests::{def, surface};
    use crate::place::{Legend, TileDef};

    fn place(seed: u32) -> Place {
        let mut legend = Legend::new();
        legend.insert('#', def(true)).unwrap();
        for (glyph, pattern) in [
            ('.', Pattern::Flat),
            (',', Pattern::Noise),
            ('t', Pattern::Tiles),
            ('=', Pattern::Planks),
        ] {
            let look = TileDef {
                surface: Surface {
                    pattern,
                    grain: 0.3,
                    ..surface([150, 140, 120])
                },
                ..def(false)
            };
            legend.insert(glyph, look).unwrap();
        }
        Place::new("######\n#.,t=#\n#=t,.#\n######", legend, [20; 3], seed).unwrap()
    }

    #[test]
    fn a_seed_reproduces_the_bake_and_another_seed_changes_it() {
        let a = place(5);
        let first = bake(&a, &Geometry::build(&a)).albedo;
        let again = bake(&a, &Geometry::build(&a)).albedo;
        assert_eq!(first, again);
        assert!(first.iter().all(|p| p.r.is_finite() && p.r >= 0.0));
        let b = place(6);
        let other = bake(&b, &Geometry::build(&b)).albedo;
        assert_eq!(first.len(), other.len());
        assert_ne!(first, other);
    }

    #[test]
    fn floors_darken_toward_walls_and_emission_is_only_built_when_used() {
        let a = place(1);
        let geo = Geometry::build(&a);
        let baked = bake(&a, &geo);
        assert!(baked.emissive.is_empty());
        let (w, _) = geo.size();
        let at = |x: usize, y: usize| baked.albedo[y * w + x].luminance();
        // Flat floor tile (1, 1): its centre is lighter than the pixel beside the wall.
        let tile = TILE_PX;
        assert!(at(tile + tile / 2, tile + tile / 2) > at(tile + 1, tile + tile / 2));
    }
}
