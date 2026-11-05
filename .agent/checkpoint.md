# Checkpoint

## Goal
Prototype the part of an excavation game that happens between expeditions (progress, narrative, upgrades, economy, motivation), plus an abstraction of one expedition, to find what makes expedition decisions interesting. Sources: `design/treatment.md`, `design/steering.md`, and the wiki world at `../Support/CLAIM7.wiki/CLAIM7/Alternative-Reality*`.

## State
- Experiment 2 (`design/iteration.md`): throwaway `scratch/meta-loop` (git-ignored, own `[workspace]`, shares `target/`). Claims are worked over several self-running descents with timed focus windows. Tender, report and descent are focus lists with a context pane. Bots read only the screen.
- Player text lives in `scratch/meta-loop/text/{a,b}.toml`: two competing drafts with the same keys, checked against `skeleton.toml` at start-up. A reviewer pass removed lines that announced. The rules emit text keys only.
- Guidance: `player-text.md` gained "Do not announce; name plainly, explain in context". AGENTS.md "Writing" now says that player text is written in its own pass.
- Host `crates/prototype` and the media crates are unchanged from the template.

## Constraints
- Not a management game; solo; small zero-g asteroids; the pod breaches the shell.
- From the wiki: progression = access + equipment + knowledge (no stat growth); the purse does not carry between claims; loss must trace to the player's own wager; the return must host the wager.
- The real game stores progression as token counts in inventories; any contract we promote must map to that. PactSystem and menu sources are available on request.

## Open
- User playtest of experiment 2, and the user's choice of voice A or B.
- Red flags in self-play: the three bot styles finish within a minute of each other; the purse matters little apart from air flasks.
- The dive layout needs about 110×34.

## Next
User plays `scratch\meta-loop\play.cmd --voice a` and `--voice b`. Record the report in `design/iteration.md` before interpreting it.

## Last verification
2026-11-05: `scratch/meta-loop` builds clean under clippy; 18 bot runs (3 policies × 6 seeds, 8 min) as tabled in `design/iteration.md`; both voices load and run. Real Windows Terminal captures at 110×34 of the tender intro, a descent and an open window. Die glyphs were missing from the font, so rolls now print as "roll N". Not verified: real keyboard input, other fonts. `verify.cmd` not run: no workspace code changed.

HEAD: see `git log -1 main`.
