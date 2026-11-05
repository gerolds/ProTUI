# Checkpoint

## Goal
Template instance: no game yet. Waiting for the user's design in `design/treatment.md`.

## State
- Host `crates/prototype`: event loop (idle blocks; ~33 ms frames only while animating), one key table, strict versioned settings (`<data>/settings.toml` v1: cell_aspect, renderer octant|quadrant|ascii, volumes, mute), first-start setup screen with live circle/square aspect check, sample lit place preview, sound checks; audio via manifest with hot reload; `--keys`, `--bench`, `--smoke`, `--log-sounds`, `--version` (commit stamp).
- Media crates: `terminal-graphics` (encode, stills, Sketch, calibration, tint cache, PNG/APNG capture, optional `sixel`), `terminal-tilemap` (ASCII + legend → lit place or raw view, `[markers]`), `terminal-audio` (manifest, variations, layers, reload, synth kit, offline render). Samples: `assets/samples/place.toml`, `assets/audio/` (LFS WAVs).
- Guidance: `AGENTS.md`, `.agent/docs/` (index `README.md`; rows marked *reference* are findings from other projects, not design), `.claude/` skills and review roles, `tools/` (wt-capture, art generator, token-report). `.omp/config.yml` hides Unity plugin skills.
- Dev profile: deps opt 3, pixel crates opt 2, line tables only.

## Constraints
- Bones and guidance come from the Company prototype; no Company or Runner design is imported.

## Open
- Setup screen volume bars render oddly beside the selection backing; cosmetic.
- Place preview crops at ≤120 columns (tilemap floor of 2 cells per tile).

## Next
Get `design/treatment.md` from the user. Then read `.agent/docs/playable-first.md` and pass its gates: hook, formula mapping, a playable throwaway in `scratch/`, agent self-play with measures, then the user's playtest. Create `design/iteration.md` at the first experiment.

## Last verification
2026-11-05: `verify.cmd` pass (fmt, clippy -D warnings, all tests, smoke), ~22 s warm after the first build. Real Windows Terminal capture of first-start setup at 120x40: octants, round circle at 2.00, sample place lit, corner `v0.1.0 · 21 fps`. Not verified: sound output, keyboard input in the real terminal, other terminals and fonts.

HEAD: see `git log -1 main`.
