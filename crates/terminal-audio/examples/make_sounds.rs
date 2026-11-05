// ADR: `make_sounds <out-dir>` writes neutral sample WAVs from one synthesis recipe per cue file, for the manifest in that folder (the built-in sample manifest when there is none).
// ADR: Every listed file needs a recipe; a missing one stops the tool with the cue's name before any file is written. Effects are mono 16-bit 44.1 kHz; loops mono 16-bit 22.05 kHz with a cross-faded seam.
//! `cargo run -p terminal-audio --example make_sounds -- assets/audio`
use std::{f32::consts::PI, path::PathBuf, process::ExitCode};
use terminal_audio::{
    Manifest, cue_files,
    synth::{
        Band, Sound, biquad, env, finish, highpass, hump, lowpass, normalize, reverb, seam,
        seed_for, write_wav,
    },
};

const EFFECT_RATE: u32 = 44_100;
const LOOP_RATE: u32 = 22_050;

const SAMPLE_MANIFEST: &str = r#"# Sample sounds written by `cargo run -p terminal-audio --example make_sounds -- assets/audio`.
# Each entry's comment says what the sound is like and how long it is.

[mix]
max_voices = 16

[bus.ui]
volume_db = -6.0

[bus.ambience]
volume_db = -12.0

[bus.music]
volume_db = -10.0

# While ui/confirm or ui/error plays, the music dips.
[duck]
bus = "music"
by_db = -5.0
attack_ms = 30
release_ms = 400

# A soft short tick, 60 ms; three slightly different variations.
[cue."ui/move"]
bus = "ui"
files = ["ui-move-1.wav", "ui-move-2.wav", "ui-move-3.wav"]
pick = "shuffle"
volume_jitter_db = 1.5
pitch_jitter_semitones = 0.5
cooldown_ms = 25
max_instances = 3
steal = "oldest"

# Two rising bell tones, 0.4 s.
[cue."ui/confirm"]
bus = "ui"
files = ["ui-confirm.wav"]
max_instances = 1
steal = "oldest"
duck = true

# Two falling plucked tones, 0.25 s.
[cue."ui/back"]
bus = "ui"
files = ["ui-back.wav"]
volume_db = -2.0
max_instances = 1
steal = "oldest"

# Two low muted buzzes, 0.26 s.
[cue."ui/error"]
bus = "ui"
files = ["ui-error.wav"]
cooldown_ms = 150
max_instances = 1
duck = true

# Quiet filtered air with a faint hiss, a 20 s seamless loop.
[cue."ambience/room-tone"]
bus = "ambience"
files = ["room-tone.wav"]
loop = true
fade_ms = 1500

# A calm open-fifth pad that swells slowly, a 30 s seamless loop.
[cue."music/drone"]
bus = "music"
files = ["drone.wav"]
loop = true
fade_ms = 2500
"#;

/// The recipe for `name`'s `variant`-th file, or `None` when there is none.
fn recipe(name: &str, variant: usize) -> Option<(u32, Vec<f32>)> {
    let seed = seed_for(name, variant);
    let v = variant as f32;
    let fx = |seconds: f32| Sound::new(EFFECT_RATE, seconds, seed);
    let rate = EFFECT_RATE as f32;
    let mut s;
    match (name, variant) {
        ("ui/move", 0..=2) => {
            s = fx(0.06);
            let click = s.breath(
                0.012,
                |_| 3_000.0 + 400.0 * v,
                1.5,
                |t| env(t, 0.0005, 0.002),
            );
            s.mix(0.0, &click, 0.6);
            let blip: Vec<f32> = s
                .sine(0.05, |_| 1_500.0 + 180.0 * v)
                .iter()
                .enumerate()
                .map(|(i, x)| x * env(i as f32 / rate, 0.001, 0.008))
                .collect();
            s.mix(0.002, &blip, 0.5);
            finish(&mut s.out, rate, 0.5);
        }
        ("ui/confirm", 0) => {
            s = fx(0.4);
            let low = s.bell(0.35, 660.0, 0.12);
            let high = s.bell(0.33, 990.0, 0.15);
            s.mix(0.0, &low, 0.6);
            s.mix(0.07, &high, 0.7);
            lowpass(&mut s.out, rate, |_| 6_000.0);
            finish(&mut s.out, rate, 0.6);
        }
        ("ui/back", 0) => {
            s = fx(0.25);
            let first = s.pluck(0.2, 784.0, 0.5);
            let second = s.pluck(0.19, 523.25, 0.6);
            s.mix(0.0, &first, 0.6);
            s.mix(0.06, &second, 0.6);
            for (i, x) in s.out.iter_mut().enumerate() {
                *x *= env(i as f32 / rate, 0.002, 0.08);
            }
            lowpass(&mut s.out, rate, |_| 4_000.0);
            finish(&mut s.out, rate, 0.5);
        }
        ("ui/error", 0) => {
            s = fx(0.26);
            let mut buzz = s.saw(0.26, |_| 150.0);
            for (i, x) in buzz.iter_mut().enumerate() {
                let t = i as f32 / rate;
                *x *= hump(t, 0.0, 0.02, 0.1) + hump(t, 0.12, 0.14, 0.24);
            }
            biquad(&mut buzz, rate, Band::Low, |_| 900.0, 0.9);
            s.mix(0.0, &buzz, 1.0);
            finish(&mut s.out, rate, 0.55);
        }
        ("ambience/room-tone", 0) => return Some(room_tone(seed)),
        ("music/drone", 0) => return Some(drone(seed)),
        _ => return None,
    }
    Some((EFFECT_RATE, s.out))
}

/// 20 s of quiet filtered air: brownish noise with slow movement and a faint hiss.
fn room_tone(seed: u64) -> (u32, Vec<f32>) {
    let (length, fade) = (20.0, 1.5);
    let total = length + fade;
    let rate = LOOP_RATE as f32;
    let mut s = Sound::new(LOOP_RATE, total, seed);
    let mut air = s.noise(total);
    lowpass(&mut air, rate, |t| 350.0 + 120.0 * (t * 0.21).sin());
    highpass(&mut air, rate, 40.0);
    for (i, x) in air.iter_mut().enumerate() {
        *x *= 0.8 + 0.2 * (i as f32 / rate * 0.31).sin();
    }
    normalize(&mut air, 1.0);
    s.mix(0.0, &air, 1.0);
    let hiss = s.breath(
        total,
        |t| 2_500.0 + 500.0 * (t * 0.13).sin(),
        0.7,
        |t| 0.15 + 0.05 * (t * 0.17).sin(),
    );
    s.mix(0.0, &hiss, 0.3);
    seam(&mut s.out, rate, fade);
    // A loop's ends meet each other, so they are not faded.
    normalize(&mut s.out, 0.35);
    (LOOP_RATE, s.out)
}

/// 30 s of an open-fifth pad (A2 E3 A3 B3): detuned saws, a slowly moving lowpass, reverb.
fn drone(seed: u64) -> (u32, Vec<f32>) {
    let (length, fade) = (30.0, 2.0);
    let total = length + fade;
    let rate = LOOP_RATE as f32;
    let mut s = Sound::new(LOOP_RATE, total, seed);
    for (k, freq) in [110.0f32, 164.81, 220.0, 246.94].into_iter().enumerate() {
        let phase = k as f32 * 1.7;
        let swell = 0.07 + 0.013 * k as f32;
        let mut voice = s.saw(total, |_| freq * 1.0015);
        let other = s.saw(total, |_| freq * 0.9985);
        for (i, (x, o)) in voice.iter_mut().zip(other).enumerate() {
            let t = i as f32 / rate;
            *x = (*x + o) * 0.5 * (0.65 + 0.35 * (t * swell * 2.0 * PI + phase).sin());
        }
        biquad(
            &mut voice,
            rate,
            Band::Low,
            |t| 700.0 + 350.0 * (t * 0.05 * 2.0 * PI + phase).sin(),
            0.8,
        );
        s.mix(0.0, &voice, 1.0 / (1.0 + k as f32 * 0.4));
    }
    reverb(&mut s.out, rate, 1.2, 0.72, 0.3);
    highpass(&mut s.out, rate, 50.0);
    seam(&mut s.out, rate, fade);
    normalize(&mut s.out, 0.4);
    (LOOP_RATE, s.out)
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let (Some(out), None) = (args.next(), args.next()) else {
        eprintln!("usage: make_sounds <out-dir>");
        return ExitCode::from(2);
    };
    let out = PathBuf::from(out);
    let manifest = out.join("sounds.toml");
    if manifest.exists() {
        println!("using {}", manifest.display());
    } else {
        if let Err(e) =
            std::fs::create_dir_all(&out).and_then(|()| std::fs::write(&manifest, SAMPLE_MANIFEST))
        {
            eprintln!("{}: {e}", manifest.display());
            return ExitCode::FAILURE;
        }
        println!("wrote the sample manifest {}", manifest.display());
    }
    let cues = match cue_files(&manifest) {
        Ok(cues) => cues,
        Err(errors) => {
            eprintln!("{errors}");
            return ExitCode::FAILURE;
        }
    };
    let mut made = Vec::new();
    let mut missing = Vec::new();
    for cue in &cues {
        for (variant, path) in cue.files.iter().enumerate() {
            match recipe(&cue.name, variant) {
                Some(sound) => made.push((cue.name.as_str(), path, sound)),
                None => missing.push(format!("{} (file {})", cue.name, variant + 1)),
            }
        }
    }
    if !missing.is_empty() {
        eprintln!("no recipe for: {}; nothing written", missing.join(", "));
        return ExitCode::FAILURE;
    }
    let mut bytes = 0u64;
    for (name, path, (rate, samples)) in &made {
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .map_err(hound::Error::IoError)
            .and_then(|()| write_wav(path, *rate, samples));
        if let Err(e) = written {
            eprintln!("{}: {e}", path.display());
            return ExitCode::FAILURE;
        }
        let size = std::fs::metadata(path).map_or(0, |m| m.len());
        bytes += size;
        println!(
            "{name:<20} {:<16} {:>6.2} s {:>6} Hz {size:>8} bytes",
            path.file_name()
                .map(|f| f.to_string_lossy())
                .unwrap_or_default(),
            samples.len() as f32 / *rate as f32,
            rate,
        );
    }
    println!(
        "{} files, {bytes} bytes ({:.2} MB)",
        made.len(),
        bytes as f64 / 1_048_576.0
    );
    match Manifest::load(&manifest) {
        Ok(loaded) => {
            println!(
                "{} loads: {} cues",
                manifest.display(),
                loaded.bank.cue_count()
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("{errors}");
            ExitCode::FAILURE
        }
    }
}
