// ADR: One seeded xorshift64* generator serves synthesis noise and per-play variation — the same seed repeats every offline render and test exactly.
/// A small seeded random generator (xorshift64*). Not for cryptography.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// Uniform in `0..1`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Uniform in `low..high`.
    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    /// White noise in `-1..1`.
    pub fn noise(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }

    /// Uniform in `0..n`; `n` must be within `1..=u32::MAX`, otherwise the result is 0.
    pub fn below(&mut self, n: usize) -> usize {
        let n = u64::try_from(n).unwrap_or(0).min(u64::from(u32::MAX));
        (((self.next_u64() >> 32) * n) >> 32) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::Rng;

    #[test]
    fn same_seed_repeats_and_ranges_hold() {
        let (mut a, mut b) = (Rng::new(7), Rng::new(7));
        let mut other = Rng::new(8);
        let mut differs = false;
        for _ in 0..1_000 {
            let x = a.unit();
            assert_eq!(x, b.unit());
            assert!((0.0..1.0).contains(&x));
            assert!(a.below(5) < 5 && b.below(5) < 5);
            differs |= other.unit() != a.unit();
            b.unit();
        }
        assert!(differs);
        assert_eq!(Rng::new(1).below(1), 0);
    }
}
