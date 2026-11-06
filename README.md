# ProTUI
A template for game prototypes played as a full-screen terminal UI, in Rust with Ratatui. Each copy prototypes one game. The game's design goes in [design/](design/README.md) and is owned by the user. This repository holds generic bones for building it fast: a terminal host, octant image encoding, ASCII-authored lit place views, sound, capture tools and agent guidance. No game rules exist yet.

Agents start at [AGENTS.md](AGENTS.md) and [.agent/checkpoint.md](.agent/checkpoint.md).

## New instance

1. Copy the folder without `.git/` and `target/`. Then run `git init -b main` and `git lfs install --local`.
2. Choose a project name. Replace every `Protui` in the repository: `play.cmd` (`APP_NAME`), `crates/prototype/src/cli.rs` (help text, default data folder and its test), `crates/prototype/src/ui.rs` (`TITLE`), `crates/prototype/README.md` and this file.
3. Put the user's design in `design/treatment.md`. Optional: `design/steering.md` for later requirements.
4. Run `verify.cmd` (macOS and Linux: `./verify.sh`), then commit.
5. Write the goal in `.agent/checkpoint.md`.

## Run

Requirements: stable Rust with MSVC build tools, Git and Git LFS. Windows Terminal is recommended. Its bundled Cascadia Mono draws octant glyphs; the copy in `C:\Windows\Fonts` may not. The first start asks for the renderer and cell shape.

```bat
play.cmd           rem build release and play
verify.cmd         rem fmt check, clippy -D warnings, tests, headless smoke
```

`play.cmd` builds into `%LOCALAPPDATA%\Protui\cargo-play\<repo-hash>\`, outside the repository, so a running game does not block other Cargo builds. It passes `--assets` and forwards other arguments (`play.cmd --help`).

On macOS and Linux, run the POSIX sh scripts from `sh`, `bash` or `zsh`. Requirements: stable Rust, Git and Git LFS. A Linux build also needs the ALSA headers and `pkg-config` for sound (`libasound2-dev pkg-config` on Debian and Ubuntu).

```sh
./play.sh          # build release and play; forwards arguments
./verify.sh        # fmt check, clippy -D warnings, tests, headless smoke
```

`play.sh` builds into the repository's `target/`, because only Windows locks a running executable.

Host options (`cargo run --locked -p prototype -- <option>`):

| Option | Effect |
|---|---|
| `--data <dir>` | settings and saves folder; default `Protui` in `%LOCALAPPDATA%` (Windows), `~/Library/Application Support` (macOS), `$XDG_DATA_HOME` or `~/.local/share` (Linux) |
| `--assets <dir>` | content folder; default: the repository's `assets/` |
| `--keys <ms>:<key>,...` | scripted key presses through the key table, for captures |
| `--log-sounds <file>` | one line per sound request: time, cue, outcome, whether a device is open |
| `--bench <frames>` | headless frames; prints build ms and bytes written per frame |
| `--smoke` | headless checks of input, settings and frames at 80x24, 160x50, 1x1 |
| `--version`, `--help` | |

Screenshots of the real terminal: `tools/wt-capture.ps1`. Headless PNG frames: the `capture` feature of `terminal-graphics`.

## Repository

| Path | Holds |
|---|---|
| `crates/terminal-graphics/` | linear-light rasters, PNG/ASCII stills, octant/quadrant encoding, shape sketching, cell-aspect calibration, PNG/APNG frame capture, optional sixel encoder ([README](crates/terminal-graphics/README.md)) |
| `crates/prototype/` | the game binary: loop, key table, versioned settings and store, setup screen (renderer, cell aspect with circle/square check, sample place preview, volumes, sound checks), sound host with hot reload, version/commit/fps corner ([README](crates/prototype/README.md)) |
| `crates/terminal-tilemap/` | ASCII map + legend → lit top-down place view or raw ASCII view ([README](crates/terminal-tilemap/README.md)) |
| `crates/terminal-audio/` | named sound cues on Kira: manifest, buses, ducking, variations, crossfading layers, hot reload, synthesis kit, offline render ([README](crates/terminal-audio/README.md)) |
| `assets/` | content the host loads: sample place, sample sounds |
| `design/` | the user's design: treatment, steering, iterations |
| `.agent/` | checkpoint, topical guides (`docs/`), common issues, templates |
| `.claude/` | skills and review roles ([routing](.claude/ROUTING.md)) |
| `.omp/` | omp project settings (hides unrelated plugin skills) |
| `tools/` | Windows Terminal capture, picture generation, token report |
| `scratch/` | ignored; disposable experiments |

Locked versions: Ratatui 0.30.2 (crossterm 0.29), Kira 0.12.6 (cpal only), hound 3.5.1, png 0.18.1, tiny-skia 0.12.0, ab_glyph 0.2.32, serde 1.0.229, toml 1.1.8. `Cargo.lock` is the authority.

Git is local on `main` with LFS for binary assets (`.gitattributes`); no remote. The bones and guidance come from the Company prototype, which adapted its technology from the Runner project; neither project's game design is included.
