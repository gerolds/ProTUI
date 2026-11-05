// ADR: Engine contract tests measure Kira's real mixed PCM through the offline backend, not mock volume values.
use std::time::Duration;
use terminal_audio::{
    AudioError, Bank, Channels, Clip, CueOptions, Dropped, Duck, Engine, EngineConfig,
    InstanceLimit, OfflineBackend, PlayOutcome, Playback, Steal,
};

const ZERO: Duration = Duration::ZERO;
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
/// One 100 ms constant-valued variation.
fn clip(value: f32) -> [Clip; 1] {
    [Clip::from_samples(48_000, Channels::Mono, &vec![value; 4_800]).unwrap()]
}
fn loop_options(bus: terminal_audio::BusId) -> CueOptions {
    CueOptions {
        playback: Playback::Loop { fade_in: ZERO },
        ..CueOptions::new(bus)
    }
}
/// Renders 50 ms and measures the last 25 ms.
fn energy(engine: &mut Engine<OfflineBackend>) -> f32 {
    let mut pcm = [0.0; 4_800];
    engine.render(&mut pcm).unwrap();
    // Discard interpolation/parameter transition at the block's leading edge.
    pcm[2_400..].iter().map(|s| s * s).sum::<f32>() / 2_400.0
}
fn near_ratio(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.025,
        "actual {actual}, expected {expected}"
    );
}

#[test]
fn actual_mixer_obeys_loop_bus_master_and_mutes() {
    let mut bank = Bank::new();
    let bus = bank.add_bus("bed", 0.0).unwrap();
    let cue = bank
        .add_cue("continuous", clip(0.2), loop_options(bus))
        .unwrap();
    let mut engine = Engine::offline(bank, EngineConfig::default(), 48_000, 0).unwrap();
    let id = engine.play(cue, ZERO).unwrap().voice().unwrap();
    let base = energy(&mut engine);
    assert!(base > 0.005);
    assert!(energy(&mut engine) > base * 0.99); // Beyond the original 100ms clip: it really loops.
    engine.set_bus_volume(bus, -6.020_6, ZERO).unwrap();
    near_ratio(energy(&mut engine) / base, 0.25);
    engine.set_master_volume(-6.020_6, ZERO).unwrap();
    near_ratio(energy(&mut engine) / base, 0.0625);
    engine.set_bus_muted(bus, true, ZERO).unwrap();
    assert!(energy(&mut engine) < 1e-12);
    engine.set_bus_volume(bus, 0.0, ZERO).unwrap(); // Changing volume must not unmute.
    assert!(energy(&mut engine) < 1e-12);
    engine.set_bus_muted(bus, false, ZERO).unwrap();
    near_ratio(energy(&mut engine) / base, 0.25);
    engine.set_muted(true, ZERO);
    engine.set_master_volume(0.0, ZERO).unwrap();
    assert!(energy(&mut engine) < 1e-12);
    engine.set_muted(false, ZERO);
    near_ratio(energy(&mut engine) / base, 1.0);
    engine.stop(id, ZERO, ZERO).unwrap();
    assert_eq!(engine.active_voices(), 0);
    assert!(energy(&mut engine) < 1e-12);
}

#[test]
fn actual_duck_attack_release_and_bus_updates_compose() {
    let mut bank = Bank::new();
    let bed = bank.add_bus("bed", 0.0).unwrap();
    let foreground = bank.add_bus("foreground", 0.0).unwrap();
    let looping = bank.add_cue("bed", clip(0.2), loop_options(bed)).unwrap();
    // Silent trigger isolates the bus attenuation in measured PCM, not a mock volume value.
    let trigger = bank
        .add_cue(
            "trigger",
            clip(0.0),
            CueOptions {
                ducks: true,
                ..CueOptions::new(foreground)
            },
        )
        .unwrap();
    let config = EngineConfig {
        max_voices: 4,
        duck: Some(Duck {
            bus: bed,
            by_db: -6.020_6,
            attack: ms(10),
            release: ms(10),
        }),
    };
    let mut engine = Engine::offline(bank, config, 48_000, 0).unwrap();
    engine.play(looping, ZERO).unwrap();
    let base = energy(&mut engine);
    let trigger_id = engine.play(trigger, ZERO).unwrap().voice().unwrap();
    assert!(engine.is_ducking());
    near_ratio(energy(&mut engine) / base, 0.25);
    engine.set_bus_volume(bed, -6.020_6, ZERO).unwrap();
    near_ratio(energy(&mut engine) / base, 0.0625);
    engine.update(ms(100)).unwrap(); // Natural completion releases at the exact boundary.
    assert!(!engine.is_active(trigger_id));
    assert!(!engine.is_ducking());
    near_ratio(energy(&mut engine) / base, 0.25);
}

#[test]
fn actual_fade_and_crossfade_replace_audio() {
    let mut bank = Bank::new();
    let bus = bank.add_bus("music", 0.0).unwrap();
    let left = Clip::from_samples(48_000, Channels::Stereo, &[0.2, 0.0].repeat(4_800)).unwrap();
    let right = Clip::from_samples(48_000, Channels::Stereo, &[0.0, 0.2].repeat(4_800)).unwrap();
    let a = bank.add_cue("left", [left], loop_options(bus)).unwrap();
    let b = bank.add_cue("right", [right], loop_options(bus)).unwrap();
    let mut engine = Engine::offline(bank, EngineConfig::default(), 48_000, 0).unwrap();
    let old = engine.play(a, ZERO).unwrap().voice().unwrap();
    let mut pcm = [0.0; 9_600];
    engine.render(&mut pcm).unwrap();
    assert!(pcm[9_598] > 0.1 && pcm[9_599].abs() < 1e-6);
    let new = engine
        .crossfade(old, b, ms(50), ZERO)
        .unwrap()
        .voice()
        .unwrap();
    assert!(!engine.is_active(old) && engine.is_active(new));
    engine.render(&mut pcm).unwrap();
    // Both channels during overlap, then only the new channel after the actual fade.
    assert!(pcm[960] > 0.0 && pcm[961] > 0.0);
    assert!(pcm[9_598].abs() < 1e-6 && pcm[9_599] > 0.1);
    engine.stop(new, ms(50), ZERO).unwrap();
    engine.render(&mut pcm).unwrap();
    assert!(pcm[961] > 0.0 && pcm[9_599].abs() < 1e-6);
}

#[test]
fn cooldown_instances_and_cap_have_exact_clock_boundaries() {
    let mut bank = Bank::new();
    let bus = bank.add_bus("effects", 0.0).unwrap();
    let cue = bank
        .add_cue(
            "tick",
            clip(0.1),
            CueOptions {
                cooldown: ms(50),
                instances: InstanceLimit::Instances {
                    count: 1,
                    steal: Steal::Never,
                },
                ..CueOptions::new(bus)
            },
        )
        .unwrap();
    let other = bank
        .add_cue("other", clip(0.1), CueOptions::new(bus))
        .unwrap();
    let mut engine = Engine::offline(
        bank,
        EngineConfig {
            max_voices: 1,
            duck: None,
        },
        48_000,
        0,
    )
    .unwrap();
    engine.play(cue, ZERO).unwrap();
    assert_eq!(
        engine.play(cue, ms(49)).unwrap(),
        PlayOutcome::Dropped(Dropped::Cooldown)
    );
    assert_eq!(
        engine.play(cue, ms(50)).unwrap(),
        PlayOutcome::Dropped(Dropped::Instances)
    );
    assert_eq!(
        engine.play(other, ms(50)).unwrap(),
        PlayOutcome::Dropped(Dropped::Voices)
    );
    assert!(engine.play(cue, ms(100)).unwrap().voice().is_some());
    assert!(matches!(
        engine.update(ms(99)),
        Err(AudioError::ClockWentBackwards { .. })
    ));
}

#[test]
fn oldest_steal_is_deterministic_even_at_equal_timestamps() {
    let mut bank = Bank::new();
    let bus = bank.add_bus("effects", 0.0).unwrap();
    let cue = bank
        .add_cue(
            "loop",
            clip(0.1),
            CueOptions {
                instances: InstanceLimit::Instances {
                    count: 2,
                    steal: Steal::Oldest,
                },
                ..loop_options(bus)
            },
        )
        .unwrap();
    let mut engine = Engine::offline(
        bank,
        EngineConfig {
            max_voices: 2,
            duck: None,
        },
        48_000,
        0,
    )
    .unwrap();
    let first = engine.play(cue, ZERO).unwrap().voice().unwrap();
    let second = engine.play(cue, ZERO).unwrap().voice().unwrap();
    assert!(
        matches!(engine.play(cue, ZERO).unwrap(), PlayOutcome::Played { stolen: Some(id), .. } if id == first)
    );
    assert!(
        matches!(engine.play(cue, ZERO).unwrap(), PlayOutcome::Played { stolen: Some(id), .. } if id == second)
    );
    assert_eq!(engine.active_voices(), 2);
}

#[test]
fn mixer_failure_does_not_commit_policy_or_steal() {
    let mut bank = Bank::new();
    let bus = bank.add_bus("effects", 0.0).unwrap();
    let cue = bank
        .add_cue(
            "loop",
            clip(0.1),
            CueOptions {
                instances: InstanceLimit::Instances {
                    count: 1,
                    steal: Steal::Oldest,
                },
                ..loop_options(bus)
            },
        )
        .unwrap();
    let mut engine = Engine::offline(
        bank,
        EngineConfig {
            max_voices: 1,
            duck: None,
        },
        48_000,
        0,
    )
    .unwrap();
    // No processing between requests: stopped tails still occupy real Kira slots.
    engine.play(cue, ZERO).unwrap();
    engine.play(cue, ZERO).unwrap();
    let current = engine.play(cue, ZERO).unwrap().voice().unwrap();
    assert!(matches!(
        engine.play(cue, ZERO),
        Err(AudioError::Playback(_))
    ));
    assert_eq!(engine.active_voices(), 1);
    assert!(engine.is_active(current));
    energy(&mut engine); // Mixer reclaims stopped tails.
    assert!(engine.play(cue, ZERO).unwrap().voice().is_some());
}

#[test]
fn pitch_changes_policy_duration_and_crossfade_rejection_keeps_old() {
    let mut bank = Bank::new();
    let bus = bank.add_bus("effects", 0.0).unwrap();
    let cue = bank
        .add_cue(
            "fast",
            clip(0.1),
            CueOptions {
                playback_rate: 2.0,
                ..CueOptions::new(bus)
            },
        )
        .unwrap();
    let mut engine = Engine::offline(
        bank,
        EngineConfig {
            max_voices: 1,
            duck: None,
        },
        48_000,
        0,
    )
    .unwrap();
    let first = engine.play(cue, ZERO).unwrap().voice().unwrap();
    assert_eq!(
        engine.crossfade(first, cue, ms(10), ms(49)).unwrap(),
        PlayOutcome::Dropped(Dropped::Voices)
    );
    assert!(engine.is_active(first));
    engine.update(ms(50)).unwrap();
    assert_eq!(engine.active_voices(), 0);
}
