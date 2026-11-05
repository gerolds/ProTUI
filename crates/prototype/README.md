# prototype

The game host binary. It owns the terminal, the loop, input, settings and the setup screen.
It has no game yet: after setup it shows a home screen that says so.

## What it provides

- `main.rs`: start-up and the loop. The loop blocks on input while nothing moves. While a
  notice or a lit place preview is on screen it polls until the next 33 ms frame; while audio
  is open it also wakes every 250 ms (play rules, device errors, sound folder edits). It draws
  only when the state changed, on resize, or while animating. Only key presses act. Sound
  requests go to the sound host after the draw that shows their cause.
- `keys.rs`: the one key table. `keys::intent(KeyEvent) -> Option<Intent>`, `keys::cap(Intent)`.
  Up/Down/Left/Right, FineLeft/FineRight (Shift+←/→), NextFocus (Tab), PrevFocus (Shift+Tab),
  Activate (Enter), Back (Esc, never quits), Quit (Ctrl+C), Other. Letters match either case.
- `store.rs`: versioned strict TOML files over `Schema { file, version, steps }`: `version`
  first, `Step`s migrate older tables, unknown and missing keys refused, errors as
  `file:line: key: reason`, temp file then rename, an unreadable file is never overwritten.
- `settings.rs`: `<data>/settings.toml`, version 1: `cell_aspect` (1.50–3.00 in hundredths),
  `renderer` (`octant` | `quadrant` | `ascii`, default `octant`), `[audio]` `master`, `music`,
  `effects` (0–100), `muted`. No file means a first start.
- `app.rs`: screen state and intents (`App::open`, `intent`, `step`, `animating`). Setup's
  volume and mute changes request the mix live (`Request::Mix`); Esc/Cancel requests the saved
  mix again, Save keeps it. "Test sound" (Enter) requests `ui/confirm`; "Background test"
  (Enter) cycles off → `ambience/room-tone` → `music/drone` → off. Leaving setup turns it off.
- `sound.rs`: the sound host over `terminal-audio`. `Loaded::load` reads
  `<assets>/audio/sounds.toml` and requires the three check cues. `Sound::open_device` runs
  silent without a device and keeps the reason (`audio off: <reason>` on home and on the check
  rows). Volumes: master on the main track; buses of `music/…` cues follow music, every other
  bus effects, on top of the bus's manifest level; percent p is 40·log10(p/100) dB, 0 % silent.
  The background layer crossfades over 1.5 s. A `Watcher` reloads on edits; a bad edit keeps
  the playing bank and shows its lines in the one-line problem strip above the key hints.
- `preview.rs`: `View`, the render-side state: the sample place's `World` (built once at
  start-up from `<assets>/samples/place.toml`), its light `Frame`, the calibration picture and
  the frame counter. Pictures use the renderer's encoder; ascii falls back to quadrant.
- `ui.rs`: `ui::draw(&App, &mut View, Rect, &mut Buffer, Instant)`. Setup shows the calibration
  circle and square and the sample place in the draft's renderer and aspect: beside the list
  from 100 columns, below it otherwise, a one-line note when too few rows are left. From 100
  columns the hint row ends with `v<version> <commit> · <n> fps` (frames in the last whole
  second). `build.rs` stamps the commit (`git rev-parse --short=7 HEAD`, "no commit" without git).

## Run

```text
cargo run -p prototype                       # full screen; needs a terminal
cargo run -p prototype -- --data <dir>       # settings elsewhere (default %LOCALAPPDATA%\Protui)
cargo run -p prototype -- --assets <dir>     # default: nearest ancestor with assets/
cargo run -p prototype -- --keys 500:down,900:enter   # scripted presses, ms after start
cargo run -p prototype -- --log-sounds sounds.log     # one line per sound request
cargo run -p prototype -- --smoke            # headless checks in a temporary folder
cargo run --release -p prototype -- --bench 200        # 160x50 headless: ms and bytes per frame
cargo run -p prototype -- --help | --version
cargo test -p prototype capture -- --ignored # captures/setup-120x40-octant.png
```

`--smoke` and `--bench` take `--assets` only; `--help` and `--version` take nothing. A missing or
invalid sample place, a sound manifest error (one `file:line: key: reason` line each) or an
unreadable settings file stops the start with its error. No output device is not an error.
`--smoke` plays the check rows through the offline mixer (never a device) and prints each log
line with the mix's peak dBFS. A sound log line reads
`<ms since start> ms | <cue> | <played variation i/n | dropped: reason | already playing | faded
out | not played: audio off | error: …> | device <open|none|offline>`.
The capture test uses `PROTOTYPE_CAPTURE_FONT`, else Windows Terminal's bundled
`CascadiaMono.ttf` (the system copy lacks octants).

Measured 2026-11-04, `--bench 200`, 160x50, setup with the lit place preview: release 2.97 ms
mean, 5.61 ms worst; debug 39 ms mean. About 8.4 kB written per frame.

## Limits

- The interactive run in a real terminal is not covered by tests; smoke, bench and the capture
  are headless. A capture shows one font's glyphs, not a terminal.
- Device playback is checked only by the sound log of a scripted run; loudness is not judged.
  The 250 ms idle wake delays releasing a duck by up to 250 ms.
- At 80x24 the place preview is cropped around the place's centre (2 cells per tile minimum).
- Text width counts one cell per character. Paths with wide characters may crop early.

## Provenance

Adapted from Runner `crates/runner/src/main.rs` (loop, `--keys`, `--bench` counting writer),
`keys.rs` (key table), `settings.rs` (strict versioned TOML, temp-and-rename write, cell aspect
in hundredths) and `save.rs` (migration steps on the TOML table, pinned text test). Runner's
game content is not used. `assets/samples/place.toml` is the neutral terminal-tilemap sample.
