// ADR: Reload tests swap real manifest files under a running offline engine: a broken or incomplete manifest must leave the playing bank and its sound untouched.
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use terminal_audio::{AudioError, Engine, Layer, Manifest, OfflineBackend, synth};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("terminal-audio-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}
fn write_manifest(dir: &Path, cues: &str) -> PathBuf {
    let path = dir.join("sounds.toml");
    fs::write(
        &path,
        format!(
            "[mix]\nmax_voices = 4\n[bus.bed]\nvolume_db = 0.0\n[bus.ui]\nvolume_db = 0.0\n{cues}"
        ),
    )
    .unwrap();
    path
}
const BED: &str =
    "[cue.\"bed/tone\"]\nbus = \"bed\"\nfiles = [\"tone.wav\"]\nloop = true\nfade_ms = 0\n";
fn rms(engine: &mut Engine<OfflineBackend>) -> f32 {
    let mut pcm = vec![0.0; 8_820];
    engine.render(&mut pcm).unwrap();
    // Skip the block's leading edge, where volume changes ramp.
    terminal_audio::Level::of(&pcm[4_410..]).rms
}

#[test]
fn reload_keeps_the_old_bank_on_error_and_swaps_on_success() {
    let dir = folder("reload");
    synth::write_wav(dir.join("tone.wav"), 44_100, &vec![0.25; 4_410]).unwrap();
    synth::write_wav(dir.join("tick.wav"), 44_100, &vec![0.1; 4_410]).unwrap();
    let path = write_manifest(&dir, BED);
    let manifest = Manifest::load(&path).unwrap();
    let mut engine = Engine::offline(manifest.bank, manifest.config, 44_100, 3).unwrap();
    let mut layer = Layer::new();
    let bed = engine.bank().cue("bed/tone").unwrap();
    let voice = layer
        .set(&mut engine, Some(bed), ms(0), ms(0))
        .unwrap()
        .unwrap()
        .voice()
        .unwrap();
    let before = rms(&mut engine);
    assert!((before - 0.25).abs() < 1e-3, "{before}");

    // Bad TOML, a missing WAV, and a lacking required cue: each refused, nothing changes.
    fs::write(&path, "[mix]\nmax_voices = \"many\"\n").unwrap();
    let error = engine
        .reload(&path, &["bed/tone"], ms(50), ms(100))
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("sounds.toml:2: mix.max_voices: "),
        "{error}"
    );
    write_manifest(&dir, &BED.replace("tone.wav", "gone.wav"));
    assert!(matches!(
        engine.reload(&path, &["bed/tone"], ms(50), ms(100)),
        Err(AudioError::Manifest(_))
    ));
    write_manifest(
        &dir,
        "[cue.\"ui/tick\"]\nbus = \"ui\"\nfiles = [\"tick.wav\"]\n",
    );
    let error = engine
        .reload(&path, &["bed/tone"], ms(50), ms(100))
        .unwrap_err();
    assert!(
        error.to_string().contains("cue.bed/tone: missing"),
        "{error}"
    );
    assert_eq!(engine.bank().cue("bed/tone"), Some(bed));
    assert!(engine.is_active(voice));
    assert!(
        (rms(&mut engine) - before).abs() < 1e-4,
        "the old bank keeps playing"
    );

    // A muted bus stays muted across a reload that keeps its name.
    let ui = engine.bank().bus("ui").unwrap();
    engine.set_bus_muted(ui, true, ms(0)).unwrap();
    write_manifest(
        &dir,
        &format!(
            "{BED}[cue.\"ui/tick\"]\nbus = \"ui\"\nfiles = [\"tick.wav\"]\nmax_instances = 1\n"
        ),
    );
    engine
        .reload(&path, &["bed/tone", "ui/tick"], ms(20), ms(100))
        .unwrap();
    assert!(!engine.is_active(voice), "reload stops every voice");
    assert_eq!(engine.active_voices(), 0);
    assert!(rms(&mut engine) < 1e-6);
    // Hosts resolve names again; the layer restarts its cue on the next request.
    let bed = engine.bank().cue("bed/tone").unwrap();
    let tick = engine.bank().cue("ui/tick").unwrap();
    assert!(
        layer
            .set(&mut engine, Some(bed), ms(0), ms(300))
            .unwrap()
            .is_some()
    );
    engine.play(tick, ms(300)).unwrap();
    assert!(
        (rms(&mut engine) - before).abs() < 1e-3,
        "the muted ui bus adds nothing"
    );
    fs::remove_dir_all(&dir).unwrap();
}
