// ADR: WAVs decode with hound into validated shared PCM (16/24-bit integer or 32-bit float, mono or stereo, 22.05/44.1/48 kHz); anything else is a typed `WavError`.
//! WAV decoding adapted from Runner's `runner-data/src/audio.rs`.
use std::{
    fmt,
    io::{self, Read, Seek},
    path::Path,
    sync::Arc,
    time::Duration,
};

use kira::{
    Frame,
    sound::static_sound::{StaticSoundData, StaticSoundSettings},
};

pub const SAMPLE_RATES: [u32; 3] = [22_050, 44_100, 48_000];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channels {
    Mono,
    Stereo,
}

impl Channels {
    pub fn count(self) -> usize {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WavError {
    Read(io::ErrorKind),
    NotWav,
    SampleFormat { bits: u16, float: bool },
    Channels(u16),
    SampleRate(u32),
    Empty,
    IncompleteFrame,
    InvalidSample(usize),
}

impl fmt::Display for WavError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(kind) => write!(f, "cannot read WAV: {kind}"),
            Self::NotWav => f.write_str("malformed or truncated PCM WAV"),
            Self::SampleFormat { bits, float } => write!(
                f,
                "unsupported {bits}-bit {} WAV; expected 16/24-bit integer or 32-bit float",
                if *float { "float" } else { "integer" }
            ),
            Self::Channels(n) => {
                write!(f, "unsupported channel count {n}; expected mono or stereo")
            }
            Self::SampleRate(rate) => write!(
                f,
                "unsupported sample rate {rate}; expected 22050, 44100 or 48000"
            ),
            Self::Empty => f.write_str("WAV contains no samples"),
            Self::IncompleteFrame => f.write_str("incomplete interleaved PCM frame"),
            Self::InvalidSample(i) => write!(f, "sample {i} is nonfinite or outside -1..=1"),
        }
    }
}
impl std::error::Error for WavError {}

/// Immutable, predecoded, shared PCM. Construction validates every sample.
#[derive(Clone, Debug)]
pub struct Clip {
    pub(crate) data: StaticSoundData,
    duration: Duration,
    channels: Channels,
}

impl Clip {
    pub fn from_samples(
        sample_rate: u32,
        channels: Channels,
        samples: &[f32],
    ) -> Result<Self, WavError> {
        if !SAMPLE_RATES.contains(&sample_rate) {
            return Err(WavError::SampleRate(sample_rate));
        }
        if samples.is_empty() {
            return Err(WavError::Empty);
        }
        if !samples.len().is_multiple_of(channels.count()) {
            return Err(WavError::IncompleteFrame);
        }
        if let Some(i) = samples
            .iter()
            .position(|s| !s.is_finite() || !(-1.0..=1.0).contains(s))
        {
            return Err(WavError::InvalidSample(i));
        }
        let frames: Arc<[Frame]> = match channels {
            Channels::Mono => samples.iter().map(|&s| Frame::from_mono(s)).collect(),
            Channels::Stereo => samples
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&[l, r]| Frame::new(l, r))
                .collect(),
        };
        let duration = Duration::from_secs_f64(frames.len() as f64 / f64::from(sample_rate));
        Ok(Self {
            data: StaticSoundData {
                sample_rate,
                frames,
                settings: StaticSoundSettings::default(),
                slice: None,
            },
            duration,
            channels,
        })
    }

    pub fn read_wav(path: impl AsRef<Path>) -> Result<Self, WavError> {
        let file = std::fs::File::open(path).map_err(|e| WavError::Read(e.kind()))?;
        Self::decode_wav(io::BufReader::new(file))
    }

    pub fn decode_wav(reader: impl Read + Seek) -> Result<Self, WavError> {
        let reader = hound::WavReader::new(reader).map_err(wav_error)?;
        let spec = reader.spec();
        let channels = match spec.channels {
            1 => Channels::Mono,
            2 => Channels::Stereo,
            n => return Err(WavError::Channels(n)),
        };
        if !SAMPLE_RATES.contains(&spec.sample_rate) {
            return Err(WavError::SampleRate(spec.sample_rate));
        }
        let samples: Result<Vec<f32>, hound::Error> =
            match (spec.sample_format, spec.bits_per_sample) {
                (hound::SampleFormat::Int, 16 | 24) => {
                    let scale = 1.0 / (1_i64 << (spec.bits_per_sample - 1)) as f32;
                    reader
                        .into_samples::<i32>()
                        .map(|s| s.map(|s| s as f32 * scale))
                        .collect()
                }
                (hound::SampleFormat::Float, 32) => reader.into_samples::<f32>().collect(),
                (format, bits) => {
                    return Err(WavError::SampleFormat {
                        bits,
                        float: format == hound::SampleFormat::Float,
                    });
                }
            };
        Self::from_samples(spec.sample_rate, channels, &samples.map_err(wav_error)?)
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }
    pub fn sample_rate(&self) -> u32 {
        self.data.sample_rate
    }
    pub fn channels(&self) -> Channels {
        self.channels
    }
    pub fn frames(&self) -> &[Frame] {
        &self.data.frames
    }
}

/// hound reports a short read as `ErrorKind::Other` ("Failed to read enough bytes") or
/// `UnexpectedEof`; both mean a truncated file, not an unreadable one.
fn wav_error(error: hound::Error) -> WavError {
    match error {
        hound::Error::IoError(e)
            if !matches!(
                e.kind(),
                io::ErrorKind::UnexpectedEof | io::ErrorKind::Other
            ) =>
        {
            WavError::Read(e.kind())
        }
        _ => WavError::NotWav,
    }
}
