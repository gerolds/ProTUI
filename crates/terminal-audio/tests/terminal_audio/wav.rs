use std::io::Cursor;
use terminal_audio::{Channels, Clip, SAMPLE_RATES, WavError};

fn wav(channels: u16, sample_rate: u32, bits: u16, float: bool, values: &[f32]) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: bits,
            sample_format: if float {
                hound::SampleFormat::Float
            } else {
                hound::SampleFormat::Int
            },
        };
        let mut writer = hound::WavWriter::new(&mut bytes, spec).unwrap();
        for &value in values {
            if float {
                writer.write_sample(value).unwrap();
            } else {
                writer
                    .write_sample((value * (1_i64 << (bits - 1)) as f32) as i32)
                    .unwrap();
            }
        }
        writer.finalize().unwrap();
    }
    bytes.into_inner()
}

#[test]
fn decodes_every_supported_real_wav_format_rate_and_channel_shape() {
    for rate in SAMPLE_RATES {
        for (bits, float) in [(16, false), (24, false), (32, true)] {
            for channels in [1, 2] {
                let values = [-1.0, -0.5, 0.0, 0.5];
                let clip = Clip::decode_wav(Cursor::new(wav(channels, rate, bits, float, &values)))
                    .unwrap();
                assert_eq!(clip.sample_rate(), rate);
                assert_eq!(clip.frames().len(), values.len() / usize::from(channels));
                assert_eq!(clip.channels().count(), usize::from(channels));
                assert!((clip.frames()[0].left + 1.0).abs() < 1e-6);
                if channels == 2 {
                    assert!((clip.frames()[0].right + 0.5).abs() < 1e-6);
                } else {
                    assert!((clip.frames()[0].right + 1.0).abs() < 1e-6);
                }
                assert!(
                    (clip.duration().as_secs_f64() - clip.frames().len() as f64 / f64::from(rate))
                        .abs()
                        < 1e-9
                );
            }
        }
    }
}

#[test]
fn rejects_unsupported_wav_metadata_and_bad_payloads() {
    assert!(matches!(
        Clip::decode_wav(Cursor::new(b"not a wave")),
        Err(WavError::NotWav)
    ));
    assert!(matches!(
        Clip::decode_wav(Cursor::new(wav(1, 8_000, 16, false, &[0.0]))),
        Err(WavError::SampleRate(8_000))
    ));
    assert!(matches!(
        Clip::decode_wav(Cursor::new(wav(3, 48_000, 16, false, &[0.0; 3]))),
        Err(WavError::Channels(3))
    ));
    assert!(matches!(
        Clip::decode_wav(Cursor::new(wav(1, 48_000, 8, false, &[0.0]))),
        Err(WavError::SampleFormat {
            bits: 8,
            float: false
        })
    ));
    assert!(matches!(
        Clip::decode_wav(Cursor::new(wav(1, 48_000, 16, false, &[]))),
        Err(WavError::Empty)
    ));
    for value in [f32::NAN, f32::INFINITY, -1.01, 1.01] {
        assert!(matches!(
            Clip::decode_wav(Cursor::new(wav(1, 48_000, 32, true, &[value]))),
            Err(WavError::InvalidSample(0))
        ));
    }
    let mut truncated = wav(1, 48_000, 16, false, &[0.5; 10]);
    truncated.truncate(truncated.len() - 3);
    assert!(matches!(
        Clip::decode_wav(Cursor::new(truncated)),
        Err(WavError::NotWav)
    ));
}

#[test]
fn raw_pcm_cannot_bypass_validation() {
    assert!(matches!(
        Clip::from_samples(0, Channels::Mono, &[0.0]),
        Err(WavError::SampleRate(0))
    ));
    assert!(matches!(
        Clip::from_samples(48_000, Channels::Stereo, &[0.0]),
        Err(WavError::IncompleteFrame)
    ));
    assert!(matches!(
        Clip::from_samples(48_000, Channels::Mono, &[]),
        Err(WavError::Empty)
    ));
    assert!(matches!(
        Clip::from_samples(48_000, Channels::Mono, &[f32::NAN]),
        Err(WavError::InvalidSample(0))
    ));
}
