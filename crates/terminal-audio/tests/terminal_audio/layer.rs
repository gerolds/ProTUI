// ADR: Background-layer tests check keep/crossfade/fade-out/restart on the offline mixer: a left-only and a right-only loop make each voice visible in the PCM.
use std::time::Duration;
use terminal_audio::{
    AudioError, Bank, Channels, Clip, CueOptions, Dropped, Engine, EngineConfig, Layer,
    PlayOutcome, Playback,
};

const ZERO: Duration = Duration::ZERO;
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn side(left: f32, right: f32) -> [Clip; 1] {
    [Clip::from_samples(48_000, Channels::Stereo, &[left, right].repeat(4_800)).unwrap()]
}

struct Setup {
    engine: Engine<terminal_audio::OfflineBackend>,
    left: terminal_audio::CueId,
    right: terminal_audio::CueId,
    once: terminal_audio::CueId,
}
fn setup(max_voices: usize) -> Setup {
    let mut bank = Bank::new();
    let bus = bank.add_bus("bed", 0.0).unwrap();
    let looping = CueOptions {
        playback: Playback::Loop { fade_in: ms(500) },
        ..CueOptions::new(bus)
    };
    let left = bank
        .add_cue("bed/left", side(0.2, 0.0), looping.clone())
        .unwrap();
    let right = bank.add_cue("bed/right", side(0.0, 0.2), looping).unwrap();
    let once = bank
        .add_cue("bed/once", side(0.1, 0.1), CueOptions::new(bus))
        .unwrap();
    let config = EngineConfig {
        max_voices,
        duck: None,
    };
    Setup {
        engine: Engine::offline(bank, config, 48_000, 0).unwrap(),
        left,
        right,
        once,
    }
}
/// Renders 100 ms and returns the last frame (left, right).
fn tail(engine: &mut Engine<terminal_audio::OfflineBackend>) -> (f32, f32) {
    let mut pcm = [0.0; 9_600];
    engine.render(&mut pcm).unwrap();
    (pcm[9_598], pcm[9_599])
}

#[test]
fn same_cue_keeps_another_crossfades_and_none_fades_out() {
    let Setup {
        mut engine,
        left,
        right,
        ..
    } = setup(4);
    let mut layer = Layer::new();
    let started = layer
        .set(&mut engine, Some(left), ms(20), ZERO)
        .unwrap()
        .unwrap();
    let first = started.voice().unwrap();
    // The layer's fade overrides the cue's 500 ms fade-in: full level after 100 ms.
    let (l, r) = tail(&mut engine);
    assert!((l - 0.2).abs() < 1e-3 && r.abs() < 1e-6, "{l} {r}");
    assert_eq!(
        layer.set(&mut engine, Some(left), ms(20), ms(100)).unwrap(),
        None
    );
    assert_eq!(layer.current(), Some((left, first)));
    assert!(engine.is_active(first) && engine.active_voices() == 1);

    let next = layer
        .set(&mut engine, Some(right), ms(50), ms(100))
        .unwrap()
        .unwrap();
    let second = next.voice().unwrap();
    assert!(!engine.is_active(first) && engine.is_active(second));
    let mut pcm = [0.0; 9_600];
    engine.render(&mut pcm).unwrap();
    // Overlap at 25 ms (Kira fades linearly in decibels, so both sides are quiet there),
    // then only the new side.
    assert!(
        pcm[2_400] > 1e-4 && pcm[2_401] > 1e-4,
        "{} {}",
        pcm[2_400],
        pcm[2_401]
    );
    assert!(pcm[9_598].abs() < 1e-6 && (pcm[9_599] - 0.2).abs() < 1e-3);

    assert_eq!(layer.set(&mut engine, None, ms(30), ms(200)).unwrap(), None);
    assert_eq!((layer.current(), engine.active_voices()), (None, 0));
    let (l, r) = tail(&mut engine);
    assert!(l.abs() < 1e-6 && r.abs() < 1e-6, "{l} {r}");
    // Fading out an empty slot is a no-op.
    assert_eq!(layer.set(&mut engine, None, ms(30), ms(300)).unwrap(), None);
}

#[test]
fn a_lost_voice_restarts_and_a_dropped_crossfade_keeps_the_old_one() {
    let Setup {
        mut engine,
        left,
        right,
        once,
    } = setup(1);
    let mut layer = Layer::new();
    let first = layer
        .set(&mut engine, Some(left), ZERO, ZERO)
        .unwrap()
        .unwrap()
        .voice()
        .unwrap();
    // With a cap of one voice the replacement cannot overlap, so the old voice stays.
    assert_eq!(
        layer.set(&mut engine, Some(right), ms(50), ms(10)).unwrap(),
        Some(PlayOutcome::Dropped(Dropped::Voices))
    );
    assert_eq!(layer.current(), Some((left, first)));
    assert!(engine.is_active(first));
    engine.stop_all(ZERO, ms(20)).unwrap();
    let again = layer
        .set(&mut engine, Some(left), ZERO, ms(30))
        .unwrap()
        .unwrap()
        .voice()
        .unwrap();
    assert_ne!(again, first, "the same cue restarts once its voice is gone");
    assert!(matches!(
        layer.set(&mut engine, Some(once), ZERO, ms(40)),
        Err(AudioError::InvalidConfig(_))
    ));
    assert_eq!(
        layer.current(),
        Some((left, again)),
        "a refused request changes nothing"
    );
}
