// ADR: The offline backend drives Kira's real renderer in device-sized blocks, starting each block like a device callback — so finished sounds are reclaimed and commands land at the same cadence as on a device.
// ADR: `Recording` keeps rendered stereo PCM for peak/RMS figures and a 16-bit WAV, so output can be inspected without listening.
use crate::{AudioError, Engine, SAMPLE_RATES};
use kira::backend::{Backend, Renderer};
use std::{path::Path, time::Duration};

/// Runs Kira's production renderer into caller-owned stereo PCM, without a device.
/// Unlike Kira's test backend, this exposes the actual mixed output.
pub struct OfflineBackend {
    renderer: Option<Renderer>,
    block_frames: usize,
    sample_rate: u32,
}
impl Backend for OfflineBackend {
    type Settings = u32;
    type Error = AudioError;

    fn setup(sample_rate: u32, internal_buffer_size: usize) -> Result<(Self, u32), AudioError> {
        if !SAMPLE_RATES.contains(&sample_rate) {
            return Err(AudioError::InvalidConfig(
                "offline sample rate must be 22050, 44100 or 48000",
            ));
        }
        Ok((
            Self {
                renderer: None,
                block_frames: internal_buffer_size.max(1),
                sample_rate,
            },
            sample_rate,
        ))
    }

    fn start(&mut self, renderer: Renderer) -> Result<(), AudioError> {
        self.renderer = Some(renderer);
        Ok(())
    }
}
impl OfflineBackend {
    pub(crate) fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    pub(crate) fn render(&mut self, stereo: &mut [f32]) -> Result<(), AudioError> {
        if !stereo.len().is_multiple_of(2) {
            return Err(AudioError::InvalidConfig(
                "offline output must contain whole stereo frames",
            ));
        }
        let renderer = self.renderer.as_mut().ok_or(AudioError::InvalidConfig(
            "offline renderer was not started",
        ))?;
        for block in stereo.chunks_mut(self.block_frames * 2) {
            // A device callback starts every block this way: queued sounds, commands and
            // track removals enter the mixer, and finished sounds free their slots.
            renderer.on_start_processing();
            renderer.process(block, 2);
        }
        Ok(())
    }
}

/// Peak and RMS of a stretch of samples, both linear (1.0 = full scale).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Level {
    pub peak: f32,
    pub rms: f32,
}
impl Level {
    pub fn of(samples: &[f32]) -> Self {
        if samples.is_empty() {
            return Self {
                peak: 0.0,
                rms: 0.0,
            };
        }
        let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        let power =
            samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / samples.len() as f64;
        Self {
            peak,
            rms: power.sqrt() as f32,
        }
    }
    /// Decibels relative to full scale; silence is `-inf`.
    pub fn peak_dbfs(self) -> f32 {
        20.0 * self.peak.log10()
    }
    pub fn rms_dbfs(self) -> f32 {
        20.0 * self.rms.log10()
    }
}

/// Interleaved stereo PCM rendered offline.
#[derive(Clone, Debug)]
pub struct Recording {
    sample_rate: u32,
    stereo: Vec<f32>,
}
impl Recording {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            stereo: Vec::new(),
        }
    }
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    pub fn samples(&self) -> &[f32] {
        &self.stereo
    }
    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.stereo.len() as f64 / 2.0 / f64::from(self.sample_rate))
    }
    /// The level of each consecutive `window` (the last may be shorter).
    pub fn levels(&self, window: Duration) -> Vec<Level> {
        let frames = ((window.as_secs_f64() * f64::from(self.sample_rate)).round() as usize).max(1);
        self.stereo.chunks(frames * 2).map(Level::of).collect()
    }
    /// Writes 16-bit stereo PCM at the recording's sample rate.
    pub fn write_wav(&self, path: impl AsRef<Path>) -> Result<(), AudioError> {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).map_err(AudioError::WavWrite)?;
        for &s in &self.stereo {
            writer
                .write_sample((s.clamp(-1.0, 1.0) * 32_767.0).round() as i16)
                .map_err(AudioError::WavWrite)?;
        }
        writer.finalize().map_err(AudioError::WavWrite)
    }
}

/// Control step of `Engine::<OfflineBackend>::run`: policy updates happen this often.
pub const CONTROL_STEP: Duration = Duration::from_millis(10);

impl Engine<OfflineBackend> {
    /// Renders `duration` from `from` into `recording`, calling `update` every `CONTROL_STEP`
    /// as a device control loop would. Returns the clock after the last step.
    pub fn run(
        &mut self,
        from: Duration,
        duration: Duration,
        recording: &mut Recording,
    ) -> Result<Duration, AudioError> {
        if recording.sample_rate != self.sample_rate() {
            return Err(AudioError::InvalidConfig(
                "recording sample rate differs from the engine's",
            ));
        }
        let end = from
            .checked_add(duration)
            .ok_or(AudioError::ClockOverflow)?;
        let rate = u64::from(recording.sample_rate);
        let mut now = from;
        while now < end {
            self.update(now)?;
            let next = (now + CONTROL_STEP).min(end);
            let frames = (next.as_nanos() as u64 * rate / 1_000_000_000)
                - (now.as_nanos() as u64 * rate / 1_000_000_000);
            let start = recording.stereo.len();
            recording.stereo.resize(start + frames as usize * 2, 0.0);
            self.render(&mut recording.stereo[start..])?;
            now = next;
        }
        self.update(end)?;
        Ok(end)
    }
}
