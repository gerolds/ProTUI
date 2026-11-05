// ADR: `render` plays the sample manifest through the offline mixer on a fixed script and seed, prints peak/RMS per second and writes a stereo WAV — output an agent can inspect without listening.
//! `cargo run -p terminal-audio --example render -- [manifest] [out.wav]`
use std::{
    error::Error,
    path::{Path, PathBuf},
    time::Duration,
};
use terminal_audio::{Engine, Layer, Manifest, Recording};

const RATE: u32 = 44_100;
const SEED: u64 = 1;
const END: Duration = Duration::from_secs(10);

enum Action {
    Play(&'static str),
    Background(Option<&'static str>, u64),
}

/// (milliseconds, action)
const SCRIPT: [(u64, Action); 12] = [
    (0, Action::Background(Some("ambience/room-tone"), 1_000)),
    (500, Action::Play("ui/move")),
    (700, Action::Play("ui/move")),
    (900, Action::Play("ui/move")),
    (1_100, Action::Play("ui/move")),
    (1_500, Action::Play("ui/confirm")),
    (2_500, Action::Play("ui/back")),
    (3_000, Action::Background(Some("music/drone"), 2_000)),
    (5_500, Action::Play("ui/error")),
    (6_000, Action::Play("ui/error")),
    (6_050, Action::Play("ui/error")),
    (8_000, Action::Background(None, 1_500)),
];

fn main() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("crate is not inside a workspace")?;
    let mut args = std::env::args().skip(1);
    let manifest_path = args
        .next()
        .map_or_else(|| root.join("assets/audio/sounds.toml"), PathBuf::from);
    let out = args.next().map_or_else(
        || root.join("target/terminal-audio-render.wav"),
        PathBuf::from,
    );
    let manifest = Manifest::load(&manifest_path)?;
    let names: Vec<&str> = SCRIPT
        .iter()
        .filter_map(|(_, action)| match action {
            Action::Play(name) | Action::Background(Some(name), _) => Some(*name),
            Action::Background(None, _) => None,
        })
        .collect();
    manifest.require(&names)?;
    let mut engine = Engine::offline(manifest.bank, manifest.config, RATE, SEED)?;
    let mut layer = Layer::new();
    let mut recording = Recording::new(RATE);
    let mut now = Duration::ZERO;
    for (at, action) in &SCRIPT {
        let at = Duration::from_millis(*at);
        now = engine.run(now, at - now, &mut recording)?;
        let (label, outcome) = match action {
            Action::Play(name) => {
                let cue = engine.bank().cue(name).ok_or("required above")?;
                (format!("play {name}"), Some(engine.play(cue, now)?))
            }
            Action::Background(name, fade) => {
                let cue = name.and_then(|n| engine.bank().cue(n));
                let fade = Duration::from_millis(*fade);
                let outcome = layer.set(&mut engine, cue, fade, now)?;
                (
                    format!("background {} over {fade:?}", name.unwrap_or("none")),
                    outcome,
                )
            }
        };
        match outcome {
            Some(outcome) => println!("{:>6.2} s  {label}: {outcome:?}", now.as_secs_f64()),
            None => println!("{:>6.2} s  {label}: no new voice", now.as_secs_f64()),
        }
    }
    engine.run(now, END - now, &mut recording)?;
    println!("second   peak dBFS   RMS dBFS");
    for (second, level) in recording.levels(Duration::from_secs(1)).iter().enumerate() {
        println!(
            "{second:>6} {:>11.1} {:>10.1}",
            level.peak_dbfs(),
            level.rms_dbfs()
        );
    }
    recording.write_wav(&out)?;
    println!(
        "wrote {} ({:.2} s stereo {RATE} Hz, {} bytes)",
        out.display(),
        recording.duration().as_secs_f64(),
        std::fs::metadata(&out)?.len()
    );
    Ok(())
}
