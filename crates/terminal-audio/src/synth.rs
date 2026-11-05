// ADR: A dependency-free DSP kit (oscillators, noise sources, envelopes, filters, comb reverb, declick, loop seam) makes placeholder sounds from code; a fixed seed per file repeats the output byte for byte.
// ADR: Output is mono 16-bit PCM WAV via hound, the format the bank decodes; loops are cross-faded tail-into-head so they repeat without a click.
//! Adapted from Runner `crates/runner-data/examples/make_sounds.rs` (the generic kit only).
use crate::Rng;
use std::{
    f32::consts::{PI, TAU},
    path::Path,
};

/// A stable seed for the `variant`-th file of `name` (FNV-1a of the name, mixed with the variant).
pub fn seed_for(name: &str, variant: usize) -> u64 {
    name.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    }) ^ ((variant as u64 + 1).wrapping_mul(0x51_7cc1))
}

/// One sound being made: its sample rate, its random source and its mono samples.
pub struct Sound {
    rate: f32,
    rng: Rng,
    pub out: Vec<f32>,
}

impl Sound {
    /// `seconds` of silence at `rate` Hz, with noise from `seed`.
    pub fn new(rate: u32, seconds: f32, seed: u64) -> Self {
        Self {
            rate: rate as f32,
            rng: Rng::new(seed),
            out: vec![0.0; (seconds * rate as f32).round() as usize],
        }
    }

    pub fn rate(&self) -> f32 {
        self.rate
    }

    pub fn rng(&mut self) -> &mut Rng {
        &mut self.rng
    }

    /// Samples in `seconds`.
    pub fn len(&self, seconds: f32) -> usize {
        (seconds * self.rate).round() as usize
    }

    /// Mixes `part` in from `at` seconds with `gain`; what runs past the end is cut.
    pub fn mix(&mut self, at: f32, part: &[f32], gain: f32) {
        let start = self.len(at);
        for (o, &s) in self.out.iter_mut().skip(start).zip(part) {
            *o += s * gain;
        }
    }

    /// White noise in -1..1.
    pub fn noise(&mut self, seconds: f32) -> Vec<f32> {
        let n = self.len(seconds);
        (0..n).map(|_| self.rng.noise()).collect()
    }

    /// A sine whose frequency follows `freq(t)`.
    pub fn sine(&self, seconds: f32, freq: impl Fn(f32) -> f32) -> Vec<f32> {
        let mut phase = 0.0f32;
        (0..self.len(seconds))
            .map(|i| {
                let s = phase.sin();
                phase = (phase + TAU * freq(i as f32 / self.rate) / self.rate) % TAU;
                s
            })
            .collect()
    }

    /// A naive sawtooth whose frequency follows `freq(t)`; filter it afterwards.
    pub fn saw(&self, seconds: f32, freq: impl Fn(f32) -> f32) -> Vec<f32> {
        let mut phase = 0.0f32;
        (0..self.len(seconds))
            .map(|i| {
                let s = 2.0 * phase - 1.0;
                phase = (phase + freq(i as f32 / self.rate) / self.rate).fract();
                s
            })
            .collect()
    }

    /// Karplus-Strong string at `freq`, with `damp` 0..1 (higher: duller, shorter).
    pub fn pluck(&mut self, seconds: f32, freq: f32, damp: f32) -> Vec<f32> {
        let period = (self.rate / freq).round().max(2.0) as usize;
        let mut line: Vec<f32> = (0..period).map(|_| self.rng.noise()).collect();
        let mut out = Vec::with_capacity(self.len(seconds));
        for i in 0..self.len(seconds) {
            let a = line[i % period];
            let b = line[(i + 1) % period];
            line[i % period] = (a * (1.0 - damp * 0.5) + b * damp * 0.5) * 0.996;
            out.push(a);
        }
        out
    }

    /// An inharmonic bell at `freq`, ringing for about `decay` seconds.
    pub fn bell(&self, seconds: f32, freq: f32, decay: f32) -> Vec<f32> {
        let partials = [
            (1.0, 1.0, 1.0),
            (2.76, 0.5, 0.6),
            (5.4, 0.25, 0.4),
            (8.93, 0.12, 0.25),
        ];
        let mut out = vec![0.0; self.len(seconds)];
        for (ratio, gain, life) in partials {
            let tone = self.sine(seconds, |_| freq * ratio);
            for (i, s) in tone.iter().enumerate() {
                out[i] += s * gain * env(i as f32 / self.rate, 0.002, decay * life);
            }
        }
        out
    }

    /// A metal tick: a few high inharmonic partials with a short ring.
    pub fn clink(&self, freq: f32, ring: f32) -> Vec<f32> {
        let seconds = ring * 5.0;
        let mut out = vec![0.0; self.len(seconds)];
        for (ratio, gain) in [(1.0, 1.0), (1.47, 0.6), (2.09, 0.4), (2.56, 0.25)] {
            let tone = self.sine(seconds, |_| freq * ratio);
            for (i, s) in tone.iter().enumerate() {
                out[i] += s * gain * env(i as f32 / self.rate, 0.0005, ring / ratio);
            }
        }
        out
    }

    /// A low thump: a sine falling from `from` to `to` Hz, with a lowpassed noise knock.
    pub fn thump(&mut self, from: f32, to: f32, decay: f32) -> Vec<f32> {
        let seconds = decay * 5.0;
        let tone = self.sine(seconds, |t| to + (from - to) * (-t / (decay * 0.5)).exp());
        let mut knock = self.noise(seconds);
        lowpass(&mut knock, self.rate, |_| 400.0);
        tone.iter()
            .zip(&knock)
            .enumerate()
            .map(|(i, (s, k))| (s * 0.9 + k * 1.2) * env(i as f32 / self.rate, 0.002, decay))
            .collect()
    }

    /// Breath or air: noise through a moving band at `freq(t)` with resonance `q`, shaped by
    /// `amp(t)`.
    pub fn breath(
        &mut self,
        seconds: f32,
        freq: impl Fn(f32) -> f32,
        q: f32,
        amp: impl Fn(f32) -> f32,
    ) -> Vec<f32> {
        let mut n = self.noise(seconds);
        let rate = self.rate;
        biquad(&mut n, rate, Band::Pass, freq, q);
        for (i, s) in n.iter_mut().enumerate() {
            *s *= amp(i as f32 / rate);
        }
        n
    }

    /// A creak: an impulse train whose rate follows `rate_hz(t)`, through a resonance at `body` Hz.
    pub fn creak(
        &mut self,
        seconds: f32,
        rate_hz: impl Fn(f32) -> f32,
        body: f32,
        amp: impl Fn(f32) -> f32,
    ) -> Vec<f32> {
        let mut out = vec![0.0; self.len(seconds)];
        let mut phase = 0.0f32;
        for (i, o) in out.iter_mut().enumerate() {
            let t = i as f32 / self.rate;
            phase += rate_hz(t) * (1.0 + 0.3 * self.rng.noise()) / self.rate;
            if phase >= 1.0 {
                phase -= 1.0;
                *o = amp(t) * (0.6 + 0.4 * self.rng.unit());
            }
        }
        let rate = self.rate;
        biquad(
            &mut out,
            rate,
            Band::Pass,
            |t| body * (1.0 + 0.15 * (t * 3.0).sin()),
            8.0,
        );
        biquad(&mut out, rate, Band::Pass, |_| body * 2.3, 6.0);
        normalize(&mut out, 1.0);
        out
    }

    /// Sparse crackle: clicks at `density(t)` per second, each a few samples of bright noise.
    pub fn crackle(&mut self, seconds: f32, density: impl Fn(f32) -> f32) -> Vec<f32> {
        let mut out = vec![0.0; self.len(seconds)];
        let n = out.len();
        for i in 0..n {
            if self.rng.unit() < density(i as f32 / self.rate) / self.rate {
                let size = self.rng.range(0.2, 1.0);
                let len = ((self.rng.range(0.0005, 0.003) * self.rate) as usize).max(1);
                for k in 0..len.min(n - i) {
                    out[i + k] += self.rng.noise() * size * (1.0 - k as f32 / len as f32);
                }
            }
        }
        out
    }

    /// A water-drop blip: a short rising sine.
    pub fn drip(&mut self) -> Vec<f32> {
        let base = self.rng.range(900.0, 1700.0);
        self.sine(0.08, |t| base * (1.0 + t * 12.0))
            .iter()
            .enumerate()
            .map(|(i, s)| s * env(i as f32 / self.rate, 0.001, 0.018))
            .collect()
    }
}

/// An attack-decay envelope: linear rise over `attack`, then exponential fall with time
/// constant `decay`.
pub fn env(t: f32, attack: f32, decay: f32) -> f32 {
    if t < attack {
        t / attack
    } else {
        (-(t - attack) / decay).exp()
    }
}

/// A smooth hump from 0 at `start` to 1 at `peak` and back to 0 at `end`.
pub fn hump(t: f32, start: f32, peak: f32, end: f32) -> f32 {
    if t <= start || t >= end {
        0.0
    } else if t < peak {
        (0.5 - 0.5 * (PI * (t - start) / (peak - start)).cos()).max(0.0)
    } else {
        (0.5 + 0.5 * (PI * (t - peak) / (end - peak)).cos()).max(0.0)
    }
}

/// One-pole lowpass with cutoff `fc(t)`.
pub fn lowpass(x: &mut [f32], rate: f32, fc: impl Fn(f32) -> f32) {
    let mut y = 0.0;
    for (i, s) in x.iter_mut().enumerate() {
        let a = 1.0 - (-TAU * fc(i as f32 / rate) / rate).exp();
        y += a * (*s - y);
        *s = y;
    }
}

/// One-pole highpass at `fc`.
pub fn highpass(x: &mut [f32], rate: f32, fc: f32) {
    let a = 1.0 - (-TAU * fc / rate).exp();
    let mut low = 0.0;
    for s in x.iter_mut() {
        low += a * (*s - low);
        *s -= low;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    Low,
    Pass,
}

/// A resonant filter (RBJ biquad) whose centre or cutoff follows `freq(t)`.
pub fn biquad(x: &mut [f32], rate: f32, band: Band, freq: impl Fn(f32) -> f32, q: f32) {
    let (mut x1, mut x2, mut y1, mut y2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for (i, s) in x.iter_mut().enumerate() {
        let f = freq(i as f32 / rate).clamp(20.0, rate * 0.45);
        let w = TAU * f / rate;
        let (sin, cos) = w.sin_cos();
        let alpha = sin / (2.0 * q);
        let (b0, b1, b2) = match band {
            Band::Low => ((1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0),
            Band::Pass => (alpha, 0.0, -alpha),
        };
        let (a0, a1, a2) = (1.0 + alpha, -2.0 * cos, 1.0 - alpha);
        let y = (b0 * *s + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2) / a0;
        x2 = x1;
        x1 = *s;
        y2 = y1;
        y1 = y;
        *s = y;
    }
}

/// A plain Schroeder reverb: four damped combs and two allpasses; `size` scales the delays,
/// `feedback` the tail, `wet` the reverb's share. The tail past the input's end is cut.
pub fn reverb(x: &mut Vec<f32>, rate: f32, size: f32, feedback: f32, wet: f32) {
    let scale = rate / 44_100.0 * size;
    let combs = [1557, 1617, 1491, 1422].map(|d| ((d as f32 * scale) as usize).max(1));
    let tail = (rate * 1.5) as usize;
    let dry_len = x.len();
    x.resize(dry_len + tail, 0.0);
    let mut sum = vec![0.0f32; x.len()];
    for delay in combs {
        let mut line = vec![0.0f32; delay];
        let mut damped = 0.0f32;
        for (i, s) in x.iter().enumerate() {
            let out = line[i % delay];
            damped = out * 0.7 + damped * 0.3;
            line[i % delay] = s + damped * feedback;
            sum[i] += out * 0.25;
        }
    }
    for delay in [225, 556].map(|d| ((d as f32 * scale) as usize).max(1)) {
        let mut line = vec![0.0f32; delay];
        for (i, s) in sum.iter_mut().enumerate() {
            let buffered = line[i % delay];
            let input = *s;
            line[i % delay] = input + buffered * 0.5;
            *s = buffered - input * 0.5;
        }
    }
    for (s, r) in x.iter_mut().zip(sum) {
        *s = *s * (1.0 - wet) + r * wet;
    }
    x.truncate(dry_len);
}

/// Scales `x` so its peak is `peak`; silence stays silent.
pub fn normalize(x: &mut [f32], peak: f32) {
    let max = x.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if max > 0.0 {
        for s in x.iter_mut() {
            *s *= peak / max;
        }
    }
}

/// Fades a one-shot's ends over 3 ms against clicks and scales its peak to `peak`.
pub fn finish(x: &mut [f32], rate: f32, peak: f32) {
    let edge = ((rate * 0.003) as usize).min(x.len() / 2);
    let n = x.len();
    for i in 0..edge {
        let g = i as f32 / edge as f32;
        x[i] *= g;
        x[n - 1 - i] *= g;
    }
    normalize(x, peak);
}

/// Folds a loop's last `seconds` into its head with an equal-power cross-fade and drops them,
/// so the end runs into the start without a click. Make the sound `seconds` longer than the
/// wanted loop. Does nothing when `x` is not longer than the fade.
pub fn seam(x: &mut Vec<f32>, rate: f32, seconds: f32) {
    let fade = (seconds * rate) as usize;
    if fade == 0 || x.len() <= fade {
        return;
    }
    let body = x.len() - fade;
    for i in 0..fade {
        let g = i as f32 / fade as f32;
        let head = (g * PI / 2.0).sin();
        let tail = (g * PI / 2.0).cos();
        x[i] = x[i] * head + x[body + i] * tail;
    }
    x.truncate(body);
}

/// Writes mono 16-bit PCM; samples are clamped to -1..=1.
pub fn write_wav(path: impl AsRef<Path>, rate: u32, samples: &[f32]) -> Result<(), hound::Error> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for &s in samples {
        writer.write_sample((s.clamp(-1.0, 1.0) * 32_767.0).round() as i16)?;
    }
    writer.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn max_step(x: &[f32]) -> f32 {
        x.windows(2).fold(0.0, |m, w| m.max((w[1] - w[0]).abs()))
    }

    #[test]
    fn seam_joins_the_end_to_the_start_without_a_jump() {
        let rate = 1_000.0;
        // A 3 Hz sine over 2.25 s ends a quarter cycle early, at -1: a raw loop jumps.
        let s = Sound::new(1_000, 0.0, 1);
        let raw = s.sine(2.25, |_| 3.0);
        let wrap = |x: &[f32]| (x[0] - x[x.len() - 1]).abs();
        let natural = max_step(&raw);
        assert!(wrap(&raw) > natural * 10.0, "the raw loop must click");
        let mut looped = raw.clone();
        seam(&mut looped, rate, 0.5);
        assert_eq!(looped.len(), raw.len() - 500);
        // The wrap equals one step of the source; inside the fade no step exceeds two source steps.
        assert!((wrap(&looped) - (raw[looped.len()] - raw[looped.len() - 1]).abs()).abs() < 1e-6);
        assert!(max_step(&looped) <= natural * 2.0, "{}", max_step(&looped));
        assert_eq!(
            &looped[500..],
            &raw[500..looped.len()],
            "the body stays untouched"
        );
    }

    #[test]
    fn finish_declicks_and_normalizes() {
        let mut x = vec![0.5; 1_000];
        finish(&mut x, 44_100.0, 0.8);
        assert_eq!((x[0], x[999]), (0.0, 0.0));
        assert!((x.iter().fold(0.0f32, |m, s| m.max(*s)) - 0.8).abs() < 1e-6);
        let mut silent = vec![0.0; 10];
        normalize(&mut silent, 1.0);
        assert!(silent.iter().all(|s| *s == 0.0));
    }

    #[test]
    fn the_same_seed_makes_the_same_sound() {
        let make = |seed| {
            let mut s = Sound::new(22_050, 0.2, seed);
            let n = s.breath(0.2, |_| 800.0, 2.0, |t| env(t, 0.01, 0.05));
            let p = s.pluck(0.2, 220.0, 0.4);
            s.mix(0.0, &n, 1.0);
            s.mix(0.05, &p, 0.5);
            s.out
        };
        assert_eq!(make(seed_for("ui/move", 0)), make(seed_for("ui/move", 0)));
        assert_ne!(make(seed_for("ui/move", 0)), make(seed_for("ui/move", 1)));
    }

    #[test]
    fn written_wavs_decode_as_clips() {
        let path =
            std::env::temp_dir().join(format!("terminal-audio-synth-{}.wav", std::process::id()));
        let s = Sound::new(22_050, 0.0, 1);
        let mut tone = s.sine(0.1, |_| 440.0);
        finish(&mut tone, 22_050.0, 0.5);
        write_wav(&path, 22_050, &tone).unwrap();
        let clip = crate::Clip::read_wav(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(clip.sample_rate(), 22_050);
        assert_eq!(clip.frames().len(), tone.len());
        let peak = clip
            .frames()
            .iter()
            .fold(0.0f32, |m, f| m.max(f.left.abs()));
        assert!((peak - 0.5).abs() < 1e-3);
    }
}
