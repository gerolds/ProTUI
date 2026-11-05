// ADR: Manifest contract tests write real TOML and WAV files to a temporary folder and check loads, refusals and their `file:line: key` places.
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use terminal_audio::{
    InstanceLimit, Manifest, Pick, Playback, Problem, Steal, WavError, cue_files, synth,
};

fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("terminal-audio-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}
fn wav(dir: &Path, file: &str, value: f32) {
    let path = dir.join(file);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    synth::write_wav(path, 44_100, &vec![value; 4_410]).unwrap();
}
fn manifest(dir: &Path, text: &str) -> PathBuf {
    let path = dir.join("sounds.toml");
    fs::write(&path, text).unwrap();
    path
}
fn reasons(path: &Path) -> Vec<String> {
    match Manifest::load(path) {
        Ok(_) => panic!("the manifest must be refused"),
        Err(errors) => errors.0.iter().map(ToString::to_string).collect(),
    }
}

const HEAD: &str = "[mix]\nmax_voices = 8\n[bus.ui]\nvolume_db = 0.0\n[bus.bed]\nvolume_db = -8.0\n\
    [duck]\nbus = \"bed\"\nby_db = -6.0\nattack_ms = 40\nrelease_ms = 500\n";

#[test]
fn a_full_manifest_loads_with_documented_defaults() {
    let dir = folder("manifest-full");
    for file in ["a-1.wav", "a-2.wav", "loops/bed.wav"] {
        wav(&dir, file, 0.25);
    }
    let path = manifest(
        &dir,
        &format!(
            "{HEAD}[cue.\"ui/a\"]\nbus = \"ui\"\nfiles = [\"a-1.wav\", \"a-2.wav\"]\npick = \"shuffle\"\n\
             volume_db = -3.0\nvolume_jitter_db = 2.0\npitch_jitter_semitones = 1.0\nmax_instances = 4\n\
             steal = \"oldest\"\ncooldown_ms = 40\nduck = true\n\
             [cue.\"bed/b\"]\nbus = \"bed\"\nfiles = [\"loops/bed.wav\", \"a-1.wav\"]\nloop = true\nfade_ms = 1500\n"
        ),
    );
    let loaded = Manifest::load(&path).unwrap();
    let bank = &loaded.bank;
    assert_eq!((bank.cue_count(), bank.bus_count()), (2, 2));
    assert_eq!(loaded.config.max_voices, 8);
    let duck = loaded.config.duck.unwrap();
    assert_eq!(duck.bus, bank.bus("bed").unwrap());
    assert_eq!(
        (duck.by_db, duck.attack, duck.release),
        (-6.0, Duration::from_millis(40), Duration::from_millis(500))
    );
    let a = bank.cue("ui/a").unwrap();
    let options = bank.options(a).unwrap();
    assert_eq!(bank.variations(a), Some(2));
    assert_eq!(options.bus, bank.bus("ui").unwrap());
    assert_eq!(options.pick, Pick::Shuffle);
    assert_eq!(
        (
            options.volume_db,
            options.volume_jitter_db,
            options.pitch_jitter_semitones
        ),
        (-3.0, 2.0, 1.0)
    );
    assert_eq!(
        options.instances,
        InstanceLimit::Instances {
            count: 4,
            steal: Steal::Oldest
        }
    );
    assert_eq!(
        (options.cooldown, options.ducks),
        (Duration::from_millis(40), true)
    );
    assert_eq!(options.playback, Playback::Once);
    let b = bank.options(bank.cue("bed/b").unwrap()).unwrap();
    assert_eq!(
        b.playback,
        Playback::Loop {
            fade_in: Duration::from_millis(1500)
        }
    );
    assert_eq!(
        (b.pick, b.instances, b.ducks),
        (Pick::Random, InstanceLimit::Unlimited, false)
    );
    assert_eq!(
        (b.volume_db, b.volume_jitter_db, b.pitch_jitter_semitones),
        (0.0, 0.0, 0.0)
    );
    assert!(loaded.require(&["ui/a", "bed/b"]).is_ok());
    // Without `[duck]` no bus ducks.
    let plain = manifest(
        &dir,
        &HEAD.replace(
            "[duck]\nbus = \"bed\"\nby_db = -6.0\nattack_ms = 40\nrelease_ms = 500\n",
            "[cue]\n",
        ),
    );
    assert_eq!(Manifest::load(plain).unwrap().config.duck, None);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn unknown_and_missing_keys_are_refused_at_their_lines() {
    let dir = folder("manifest-shape");
    let path = manifest(
        &dir,
        &format!("{HEAD}[cue.\"ui/a\"]\nbus = \"ui\"\nfiles = [\"a.wav\"]\nvolume = 1.0\n"),
    );
    let found = reasons(&path);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].starts_with(&format!("{}:15: ", path.display())),
        "{found:?}"
    );
    assert!(found[0].contains("unknown field `volume`"), "{found:?}");
    let path = manifest(&dir, &format!("{HEAD}[cue.\"ui/a\"]\nbus = \"ui\"\n"));
    assert!(reasons(&path)[0].contains("missing field `files`"));
    let path = manifest(&dir, "[bus.ui]\nvolume_db = 0.0\n[cue]\n");
    assert!(reasons(&path)[0].contains("missing field `mix`"));
    let missing = reasons(&dir.join("absent.toml"));
    assert!(missing[0].contains("cannot read"), "{missing:?}");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn every_bad_value_is_reported_as_file_line_key_reason() {
    let dir = folder("manifest-values");
    let path = manifest(
        &dir,
        &format!(
            "{HEAD}[cue.\"Bad Name\"]\nbus = \"music\"\nfiles = [\"../x.wav\", \"y.ogg\"]\n\
             steal = \"oldest\"\nfade_ms = 100\npitch_jitter_semitones = -1.0\n\
             [cue.\"bed/x\"]\nbus = \"ui\"\nfiles = []\nloop = true\nmax_instances = 0\nvolume_db = 30.0\n"
        ),
    );
    let found = reasons(&path);
    let file = path.display();
    let expect = [
        format!("{file}:12: cue.Bad Name: a cue name is `group/name`"),
        format!("{file}:13: cue.Bad Name.bus: no bus \"music\""),
        format!("{file}:14: cue.Bad Name.files[0]: write a path relative"),
        format!("{file}:14: cue.Bad Name.files[1]: write a path relative"),
        format!("{file}:15: cue.Bad Name.steal: `steal` needs `max_instances`"),
        format!("{file}:16: cue.Bad Name.fade_ms: `fade_ms` needs `loop = true`"),
        format!(
            "{file}:17: cue.Bad Name.pitch_jitter_semitones: must be a finite number within 0..=12"
        ),
        format!("{file}:20: cue.bed/x.files: list at least one file"),
        format!("{file}:21: cue.bed/x.loop: a loop needs `fade_ms`"),
        format!("{file}:22: cue.bed/x.max_instances: must be within 1..="),
        format!("{file}:23: cue.bed/x.volume_db: must be a finite number within -120..=24"),
    ];
    assert_eq!(found.len(), expect.len(), "{found:#?}");
    for (found, expected) in found.iter().zip(&expect) {
        assert!(found.starts_with(expected.as_str()), "{found} / {expected}");
    }
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn load_is_all_or_nothing_over_wav_files() {
    let dir = folder("manifest-wav");
    wav(&dir, "good.wav", 0.25);
    fs::write(dir.join("text.wav"), "not audio").unwrap();
    let text = format!(
        "{HEAD}[cue.\"ui/good\"]\nbus = \"ui\"\nfiles = [\"good.wav\"]\n\
         [cue.\"ui/bad\"]\nbus = \"ui\"\nfiles = [\"good.wav\", \"text.wav\", \"none.wav\"]\n"
    );
    let path = manifest(&dir, &text);
    let errors = Manifest::load(&path)
        .err()
        .expect("one bad cue refuses the whole manifest");
    assert_eq!(errors.0.len(), 2, "{errors}");
    assert_eq!(
        (errors.0[0].line, errors.0[0].key.as_str()),
        (Some(17), "cue.ui/bad.files[1]")
    );
    assert!(matches!(
        &errors.0[0].problem,
        Problem::Wav {
            error: WavError::NotWav,
            ..
        }
    ));
    assert_eq!(errors.0[1].key, "cue.ui/bad.files[2]");
    assert!(matches!(
        &errors.0[1].problem,
        Problem::Wav {
            error: WavError::Read(std::io::ErrorKind::NotFound),
            ..
        }
    ));
    // Tools can list the files before they exist.
    let listed = cue_files(&path).unwrap();
    assert_eq!(listed[0].name, "ui/bad");
    assert_eq!(listed[0].files[2], dir.join("none.wav"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn require_names_every_missing_cue() {
    let dir = folder("manifest-require");
    wav(&dir, "a.wav", 0.25);
    let path = manifest(
        &dir,
        &format!("{HEAD}[cue.\"ui/a\"]\nbus = \"ui\"\nfiles = [\"a.wav\"]\n"),
    );
    let loaded = Manifest::load(&path).unwrap();
    let errors = loaded.require(&["ui/a", "ui/b", "bed/c"]).unwrap_err();
    let lines: Vec<String> = errors.0.iter().map(ToString::to_string).collect();
    assert_eq!(
        lines,
        [
            format!(
                "{}: cue.ui/b: missing; the host plays this cue",
                path.display()
            ),
            format!(
                "{}: cue.bed/c: missing; the host plays this cue",
                path.display()
            ),
        ]
    );
    fs::remove_dir_all(&dir).unwrap();
}
