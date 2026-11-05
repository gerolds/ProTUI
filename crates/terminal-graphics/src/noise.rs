// ADR: Presentation-only randomness: integer hashing and a splitmix64 stream, seeded per use, never shared with game state.
//! Deterministic hash noise for textures, scatter and flicker.

/// A well-mixed 32-bit hash of an integer lattice point and a seed.
pub fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = seed.wrapping_mul(0x9E37_79B9)
        ^ (x as u32).wrapping_mul(0x85EB_CA6B)
        ^ (y as u32).wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846C_A68B);
    h ^= h >> 16;
    h
}

/// A hashed value in 0..1 for an integer lattice point.
pub fn unit(x: i32, y: i32, seed: u32) -> f32 {
    (hash(x, y, seed) >> 8) as f32 / (1u32 << 24) as f32
}

/// Hermite interpolation of `x` between `edge0` and `edge1`, clamped to 0..=1.
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Smooth value noise in 0..1 with one lattice cell per unit.
pub fn value(x: f32, y: f32, seed: u32) -> f32 {
    let (fx, fy) = (x.floor(), y.floor());
    let (ix, iy) = (fx as i32, fy as i32);
    let (tx, ty) = (x - fx, y - fy);
    let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
    let a = unit(ix, iy, seed);
    let b = unit(ix + 1, iy, seed);
    let c = unit(ix, iy + 1, seed);
    let d = unit(ix + 1, iy + 1, seed);
    let top = a + (b - a) * sx;
    let bottom = c + (d - c) * sx;
    top + (bottom - top) * sy
}

/// Fractal value noise in 0..1; each octave doubles frequency and halves weight.
pub fn fbm(x: f32, y: f32, seed: u32, octaves: u32) -> f32 {
    let (mut sum, mut weight, mut total, mut frequency) = (0.0, 1.0, 0.0, 1.0);
    for octave in 0..octaves {
        sum += value(
            x * frequency,
            y * frequency,
            seed.wrapping_add(octave * 7919),
        ) * weight;
        total += weight;
        weight *= 0.5;
        frequency *= 2.0;
    }
    sum / total
}

/// Smooth 1D noise in -1..1 over time, for light flicker.
pub fn wave(t: f32, seed: u32) -> f32 {
    value(t, 0.5, seed) * 2.0 - 1.0
}

/// A splitmix64 stream for scattering details.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in 0..1.
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// A value in `low..high`.
    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.next_f32()
    }
}
