// ADR: Variation tests check picks and jitter through `PlayOutcome` and the real mixed PCM, on a fixed seed.
use std::time::Duration;
use terminal_audio::{
    Bank, Channels, Clip, CueId, CueOptions, Engine, EngineConfig, OfflineBackend, Pick,
    PlayOutcome,
};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
/// Variations of 100 ms at constant values 0.1, 0.2, ...
fn variations(count: usize) -> Vec<Clip> {
    (1..=count)
        .map(|i| Clip::from_samples(48_000, Channels::Mono, &vec![i as f32 * 0.1; 4_800]).unwrap())
        .collect()
}
fn engine(
    pick: Pick,
    count: usize,
    jitter: (f32, f32),
    seed: u64,
) -> (Engine<OfflineBackend>, CueId) {
    let mut bank = Bank::new();
    let bus = bank.add_bus("effects", 0.0).unwrap();
    let cue = bank
        .add_cue(
            "varied",
            variations(count),
            CueOptions {
                pick,
                volume_jitter_db: jitter.0,
                pitch_jitter_semitones: jitter.1,
                ..CueOptions::new(bus)
            },
        )
        .unwrap();
    (
        Engine::offline(bank, EngineConfig::default(), 48_000, seed).unwrap(),
        cue,
    )
}
/// Plays one request every 200 ms (after the previous one ended) and returns the outcomes.
fn plays(engine: &mut Engine<OfflineBackend>, cue: CueId, n: u64) -> Vec<(usize, f32, f64)> {
    (0..n)
        .map(|i| match engine.play(cue, ms(i * 200)).unwrap() {
            PlayOutcome::Played {
                variant,
                volume_db,
                playback_rate,
                ..
            } => (variant, volume_db, playback_rate),
            PlayOutcome::Dropped(d) => panic!("dropped: {d:?}"),
        })
        .collect()
}

#[test]
fn jitter_is_bounded_and_repeats_for_the_same_seed() {
    let (mut a, cue_a) = engine(Pick::Random, 3, (3.0, 2.0), 42);
    let (mut b, cue_b) = engine(Pick::Random, 3, (3.0, 2.0), 42);
    let (mut c, cue_c) = engine(Pick::Random, 3, (3.0, 2.0), 43);
    let first = plays(&mut a, cue_a, 30);
    assert_eq!(first, plays(&mut b, cue_b, 30));
    assert_ne!(first, plays(&mut c, cue_c, 30));
    let (low, high) = (2f64.powf(-2.0 / 12.0), 2f64.powf(2.0 / 12.0));
    for &(variant, volume, rate) in &first {
        assert!(variant < 3);
        assert!((-3.0..=3.0).contains(&volume), "{volume}");
        assert!((low..=high).contains(&rate), "{rate}");
    }
    assert!(first.iter().any(|p| p.1 != first[0].1), "volume varies");
    // The same requests mix to the same PCM.
    let (mut a, cue_a) = engine(Pick::Random, 3, (3.0, 2.0), 7);
    let (mut b, cue_b) = engine(Pick::Random, 3, (3.0, 2.0), 7);
    let (mut pa, mut pb) = (vec![0.0; 4_800], vec![0.0; 4_800]);
    for i in 0..3 {
        a.play(cue_a, ms(i * 200)).unwrap();
        b.play(cue_b, ms(i * 200)).unwrap();
        a.render(&mut pa).unwrap();
        b.render(&mut pb).unwrap();
        assert_eq!(pa, pb);
    }
}

#[test]
fn zero_jitter_plays_the_cue_as_written() {
    let (mut engine, cue) = engine(Pick::Random, 1, (0.0, 0.0), 1);
    for (variant, volume, rate) in plays(&mut engine, cue, 5) {
        assert_eq!((variant, volume, rate), (0, 0.0, 1.0));
    }
}

#[test]
fn shuffle_plays_every_variation_before_any_repeats() {
    let (mut engine, cue) = engine(Pick::Shuffle, 4, (0.0, 0.0), 9);
    let order: Vec<usize> = plays(&mut engine, cue, 40)
        .into_iter()
        .map(|p| p.0)
        .collect();
    for round in order.chunks(4) {
        let mut seen = round.to_vec();
        seen.sort_unstable();
        assert_eq!(seen, [0, 1, 2, 3], "{order:?}");
    }
    assert!(
        order.windows(2).all(|w| w[0] != w[1]),
        "no repeat across rounds: {order:?}"
    );
    assert!(
        order.chunks(4).any(|r| r != &order[..4]),
        "rounds are reshuffled"
    );
}

#[test]
fn the_reported_variation_is_the_one_that_sounds() {
    let (mut engine, cue) = engine(Pick::Random, 3, (0.0, 0.0), 5);
    let mut pcm = vec![0.0; 11_520];
    for i in 0..4 {
        let PlayOutcome::Played { variant, .. } = engine.play(cue, ms(i * 200)).unwrap() else {
            panic!("dropped");
        };
        // 100 ms of sound, then 20 ms of silence before the next play.
        engine.render(&mut pcm).unwrap();
        let expected = (variant + 1) as f32 * 0.1;
        assert!(
            (pcm[4_000] - expected).abs() < 1e-3,
            "{} vs {expected}",
            pcm[4_000]
        );
        assert!(pcm[11_000].abs() < 1e-6);
    }
}
