// ADR: `playback` plays the sample manifest on the default output device in real time and hot-reloads it when a file in its folder changes; without a device it prints the reason and exits cleanly, as a host would continue without sound.
//! `cargo run -p terminal-audio --example playback -- [manifest] [seconds]`
use std::{
    error::Error,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use terminal_audio::{Engine, Layer, Manifest, Watcher};

const REQUIRED: [&str; 6] = [
    "ui/move",
    "ui/confirm",
    "ui/back",
    "ui/error",
    "ambience/room-tone",
    "music/drone",
];

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("crate is not inside a workspace")?;
    let path = args
        .next()
        .map_or_else(|| root.join("assets/audio/sounds.toml"), PathBuf::from);
    let seconds: u64 = args.next().map_or(Ok(12), |s| s.parse())?;
    let manifest = Manifest::load(&path)?;
    manifest.require(&REQUIRED)?;
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    let mut engine = match Engine::open_device(manifest.bank, manifest.config, seed) {
        Ok(engine) => engine,
        Err(e) => {
            // A host keeps running without sound and shows this reason.
            println!("no sound: {e}");
            return Ok(());
        }
    };
    println!(
        "device opened; playing {seconds} s; edit {} to hot-reload",
        path.display()
    );
    let start = Instant::now();
    let folder = path.parent().ok_or("manifest has no folder")?.to_path_buf();
    let mut watcher = Watcher::new(&folder, &["toml", "wav"], Duration::from_millis(250), start);
    let mut layer = Layer::new();
    let mut next_move = Duration::from_millis(500);
    let mut step = 0u32;
    while start.elapsed() < Duration::from_secs(seconds) {
        let now = start.elapsed();
        engine.update(now)?;
        engine.poll_device_error()?;
        if watcher.poll(Instant::now()).is_some() {
            match engine.reload(&path, &REQUIRED, Duration::from_millis(200), now) {
                Ok(()) => println!("{:>6.2} s reloaded {}", now.as_secs_f64(), path.display()),
                Err(e) => println!(
                    "{:>6.2} s reload refused, old sounds kept:\n{e}",
                    now.as_secs_f64()
                ),
            }
        }
        // Background: room tone for the first third, then the drone, then silence near the end.
        let third = seconds * 1_000 / 3;
        let wanted = match now.as_millis() as u64 {
            t if t < third => Some("ambience/room-tone"),
            t if t < third * 2 + third / 2 => Some("music/drone"),
            _ => None,
        };
        let cue = wanted.and_then(|n| engine.bank().cue(n));
        if let Some(outcome) = layer.set(&mut engine, cue, Duration::from_millis(2_000), now)? {
            println!(
                "{:>6.2} s background {}: {outcome:?}",
                now.as_secs_f64(),
                wanted.unwrap_or("none")
            );
        }
        if now >= next_move {
            let name = match step % 8 {
                3 => "ui/confirm",
                5 => "ui/back",
                7 => "ui/error",
                _ => "ui/move",
            };
            if let Some(cue) = engine.bank().cue(name) {
                println!(
                    "{:>6.2} s {name}: {:?}",
                    now.as_secs_f64(),
                    engine.play(cue, now)?
                );
            }
            step += 1;
            next_move += Duration::from_millis(700);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    engine.stop_all(Duration::from_millis(300), start.elapsed())?;
    std::thread::sleep(Duration::from_millis(350));
    engine.poll_device_error()?;
    println!("done; device errors polled throughout");
    Ok(())
}
