# Checkpoint

## Goal
Prototype the part of an excavation game that happens between expeditions (progress, narrative, upgrades, economy, motivation) and an abstraction of one expedition (bars, event slots, dice, steering choices) to find what makes expedition decisions interesting. Source: `design/treatment.md`.

## State
- Experiment 1 (`design/iteration.md`): throwaway `scratch/meta-loop` (git-ignored, own `[workspace]`, shares `target/`). Tender + dive + report screens, headless bots that read only the screen. Hook, formula mapping and all numbers are agent candidates.
- Host `crates/prototype` and media crates unchanged from the template.

## Constraints
- Not a management game; solo; several small zero-g asteroid levels with a hard shell the drop pod pierces.
- The real game stores all progression as token counts in inventories; any contract we promote later must map to that.
- QuestSystem (PactSystem) and menu source are available from the user on request, for vocabulary.

## Open
- User playtest of experiment 1.
- Self-play gate failures: next purchase 60–120 s away; 20–29 s quiet returns; Lantern unreached in 5 min.
- Dive layout needs about 110×34; it crops at 80×24.

## Next
User plays `scratch\meta-loop\play.cmd`; record the report in `design/iteration.md` before interpreting it. Then tune or branch per the user's answer.

## Last verification
2026-11-05: `scratch/meta-loop` builds; 12 bot runs (naive/bold × 6 seeds) as tabled in `design/iteration.md`; real Windows Terminal captures at 110×32 of a dive and a hollow card (colours and dim text read on the default profile). Not verified: keyboard input in a real terminal, sound (none), other fonts. `verify.cmd` not run: no workspace code changed.

HEAD: see `git log -1 main`.
