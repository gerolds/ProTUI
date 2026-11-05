# terminal-audio

Named-cue sound for terminal games: a Kira mixer with play rules on a caller-owned clock, a strict
TOML sound manifest with hot reload, background layers, an offline renderer for inspection, and a
small synthesis kit for placeholder sounds.

## Entry points

- `Manifest::load(path)` reads `sounds.toml`, checks every key and decodes every WAV. Any problem
  refuses the whole manifest; `ManifestErrors` lists each as `file:line: key: reason`.
  `manifest.require(&[names])` refuses a manifest that lacks a cue the host plays.
- `Engine::open_device(bank, config, seed)` opens the default output. Without a device it returns
  `AudioError::Device`; nothing is substituted. The host continues without sound and shows the
  error text. Call `poll_device_error` regularly.
- `Engine::offline(bank, config, sample_rate, seed)` runs the same mixer without a device.
- `engine.play(cue, now)`, `stop`, `stop_all`, `crossfade`, `update(now)` (every control step),
  bus/master volume and mute. `now` is a nondecreasing `Duration` the host owns.
- `Layer::set(&mut engine, Some(cue) | None, fade, now)`: one background slot. The same cue keeps
  playing; another cue crossfades over `fade`; `None` fades out; a voice lost to a reload restarts.
- `engine.reload(path, required, fade, now)`: loads fully, then swaps the bank. On any error the
  old bank keeps playing and the error is returned. On success every voice stops over `fade`, cue
  IDs must be resolved again (`engine.bank().cue(name)`), bus mutes carry over by bus name.
- `Watcher::new(dir, &["toml", "wav"], interval, now)` and `poll(now)`: dependency-free polling of
  modification time and size.
- `Recording`, `Level`, `engine.run(from, duration, &mut recording)`: offline PCM with peak/RMS per
  window and a 16-bit stereo WAV.
- `synth`: `Rng`, `Sound` (mix, noise, sine, saw, pluck, bell, clink, thump, breath, creak,
  crackle, drip), `env`, `hump`, `lowpass`, `highpass`, `biquad`, `reverb`, `normalize`, `finish`,
  `seam`, `write_wav`, `seed_for`.

Play rules: per-cue cooldown, instance limit with steal-oldest or drop, a global voice cap, and
ducking of one bus while a ducking cue plays. Variations: several files per cue, `pick` random
or shuffle (every file once before a repeat), ± volume and pitch jitter from the seeded RNG, so
equal seeds and requests give equal output.

## Manifest (`sounds.toml`)

- `[mix] max_voices` (1..=65535), required.
- `[bus.<name>] volume_db` (-120..=24), at least the buses cues name; at most 32.
- `[duck]` optional (absent: nothing ducks): `bus`, `by_db` (-120..=0), `attack_ms`, `release_ms`.
- `[cue."group/name"]`: `bus` and `files` (paths relative to the manifest, ending `.wav`, no `..`)
  are required. Optional, with their absence: `pick` (`"random"`), `volume_db` (0),
  `volume_jitter_db` (0, up to 24), `pitch_jitter_semitones` (0, up to 12), `max_instances` (no
  limit), `steal` (`"oldest"` or `"none"`; needs `max_instances`; absent: none), `cooldown_ms` (0),
  `duck` (false), `loop` (false), `fade_ms` (required with `loop = true`, refused without it).
- WAV: 16/24-bit integer or 32-bit float, mono or stereo, 22.05/44.1/48 kHz. A missing or
  unreadable file refuses the manifest. Unlisted files in the folder are not checked.

## Examples

- `cargo run --release -p terminal-audio --example make_sounds -- assets/audio` writes one WAV per
  listed file from its recipe, for the folder's `sounds.toml` (the built-in sample manifest when
  absent). A listed file without a recipe stops the tool before anything is written. The sample
  set: `ui/move` (3 variations), `ui/confirm`, `ui/back`, `ui/error`, `ambience/room-tone` (20 s
  loop), `music/drone` (30 s loop); 8 files, 2.2 MB, byte-identical on every run.
- `cargo run -p terminal-audio --example render -- [manifest] [out.wav]` plays the sample manifest
  offline on a fixed script and seed, prints peak/RMS dBFS per second, and writes
  `target/terminal-audio-render.wav`.
- `cargo run -p terminal-audio --example playback -- [manifest] [seconds]` plays on the real
  device and hot-reloads the manifest folder; without a device it prints `no sound: <reason>`.

## Limits and unverified

- Fades follow Kira's tweens, which move linearly in decibels.
- Reload stops all voices; it does not keep one-shots or loops running across the swap.
- Listening quality of the sample set and device playback is not judged by tests; tests measure
  mixed PCM offline.
- Device behavior was checked only by opening a device and playing (Windows); not on other systems.

## Provenance

Adapted from Runner `crates/runner/src/audio.rs` (play rules, variations, jitter),
`crates/runner-data/src/audio.rs` (manifest, WAV decoding), `crates/runner-data/src/watch.rs`
(watcher) and `crates/runner-data/examples/make_sounds.rs` (synthesis kit, WAV writer). Recipes
and sample manifest are new and neutral.
