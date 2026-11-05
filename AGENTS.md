# Agent guidance

Shared by Claude, Codex/ChatGPT and subagents. `CLAUDE.md` points here.

## Purpose

- This repository is a rapid prototype of one game, played as a full-screen terminal UI. Rust + Ratatui on Windows first. It starts as a copy of a template; `README.md` "New instance" lists the setup steps.
- The game's design is in `design/` and nowhere else. The user owns it: `design/treatment.md` is the source design; `design/steering.md`, if present, holds the user's later requirements. Without a treatment, ask the user for one before any game work. Code and guidance never present a game rule as approved unless `design/` says so; agents propose rules, pacing and rewards as labelled candidates in playable throwaways (`.agent/docs/playable-first.md`).
- The repository supplies generic bones, ready for the first playable experiment: terminal host (`crates/prototype`), image encoding (`crates/terminal-graphics`), ASCII-authored lit place views (`crates/terminal-tilemap`), sound (`crates/terminal-audio`), capture tools (`tools/`), topical guides (`.agent/docs/`) and skills (`.claude/`). See `README.md` for the map and commands.
- The user is the designer and playtester. The agent's job: turn the user's intent into playable, robust increments and good game design.

## Read first

- `.agent/checkpoint.md`: current state and next action.
- `.agent/docs/README.md`: topical guides by trigger (terminal UI, rendering, audio, Rust, tests, delegation, content, generated art). Read the guide before the work it covers.
- `.claude/ROUTING.md`: skills and review roles by trigger.
- `.agent/common-issues.md`: before debugging anything new.

## Priorities

Scarcest first: (1) the user's attention, (2) whether the game is worth playing: the felt first minutes, pace and payoffs, (3) the soundness of contracts later work builds on (game state model, saves, content formats), (4) tokens. Spend tokens to save attention or protect a contract. Search before asking; check your work before handing it over. Infrastructure (save migrations, renderer escape hatches, more apps) waits until a slice is worth playing.

Fit process to the cost of undoing, not diff size:

| Cost to undo | Examples | Process |
|---|---|---|
| Low | wording, docs, a layout fix, a bug with a repro | Do it, verify, commit. |
| Medium | a new screen or content of an existing kind | Name the primitives used; one packet; verify before yielding. |
| High | game state model, command contract, save format | Discovery (`.agent/docs/discovery.md`), a no-stake probe, user approval before code. |

"Minimal" means enough to carry the next two weeks of known work, not a throwaway sketch.

Tokens: re-reading context is most of the cost, so what enters context early is paid on every later request.

- Search before reading. Read ranges or symbols; read a file whole only if it is under about 300 lines. Bound tool output. Do not reread what is already in context.
- Keep source files under about 1,200 lines; split larger ones by part, with tests in their own module file.
- Point to doc sections instead of copying them. Write a document only if someone will read it again. Status goes in the checkpoint, history in git.
- Build and run the full suite once at the end of a change, not after each edit. Test runs stay in seconds: a warm `verify.cmd` over a minute is a defect to fix before more features (budgets and causes: `.agent/docs/tests.md` "Keeping test runs in seconds").
- Before building features for a new system, model its loop headless with numbers (`.agent/docs/discovery.md`).
- Leverage the prompt cache: the lead's cache lives 1 hour, a subagent's 5 minutes, and an expired cache is rewritten in full. Only the lead keeps context; subagents are one fresh agent per packet, follow-ups only while warm, finished well under 300k tokens, with no tool call or wait longer than their cache. Fan out siblings in one batch with an identical shared prefix. Edit root guidance only between sessions or at milestones. Details: `.agent/docs/delegation.md` "Cache, context and quota".
- At natural break points (milestone committed and checkpoint updated, topic change, a pause over an hour), the lead tells the user that compaction is a good idea now.

## Design work

- Test one experience hypothesis per experiment with an end-to-end playable change. At the first experiment create `design/iteration.md`: question, hypothesis, playable command, observation, keep/change/remove decision, next test.
- Show a rendered candidate (headless frame or capture) before committing to layout or interaction. Compare alternatives only to resolve an open question.
- An approved candidate is a contract for content as well as layout: the build must produce text and pictures of the same kind from real state, or the gap is a blocker reported to the user.
- Judge the whole first five minutes, not parts. Before adding a system, an agent plays the integrated slice from launch to first payoff with a timed screen log and reports the measures in `.agent/docs/playable-first.md` "Gates". Then the user plays it. Read that guide before any slice, system, scope proposal or design question.
- Tune for a naive first-time player first; economy tables show the naive row first.
- Separate player actions and observations from interpretation. Playtest reports are ground truth; tests and smoke runs do not prove a game is understandable or worth playing.
- Isolate experiments (`scratch/` is ignored); integrate the chosen direction; remove superseded paths.
- Add simulation, formats, dependencies and abstractions only for approved behavior. Local detail (a client, a place's history, a person's habit) may be invented as a marked draft; voice questions get two competing drafts.

## Work and delegation

- The lead owns scope, contracts, integration, verification, guidance and the checkpoint.
- Delegate bounded work when it keeps more out of the lead's context than the brief costs; otherwise work inline. Read `.agent/docs/delegation.md` before writing a packet. No nested delegation.
- Packet: goal, scope, non-goals, contract, document pointers, acceptance, evidence. Report: result, paths, evidence, deviations, blockers; 15 lines maximum.
- Concurrent implementation uses separate git worktrees. In a shared tree, owners have disjoint files and leaf agents do not stage, commit or run Cargo. One owner per stateful tool (builds, formatters, terminal sessions).
- The integration owner changes shared manifests, `Cargo.lock` and guidance, then runs checks.

## Code

- Stable Rust, edition 2024, workspace crates in `crates/`, `unsafe` forbidden. Dependency versions are pinned once in the root `Cargo.toml`.
- Game rules live apart from terminal I/O and media. Rendering reads state; the app owns the loop and key mapping. Ratatui renders complete frames through its crossterm re-export; act on key presses only; restore the terminal on success, error and panic.
- Typed interfaces and errors. Validate external input (content files, saves, settings) strictly where it enters: unknown or missing fields are refused, errors say `file:line: key: reason`. Never repair bad data with a default, hide a failure, or branch on an error message.
- A save or settings file carries a schema version; another version is migrated by a written step or refused. An unreadable file stops start-up and is never overwritten.
- One mechanism per job. If a primitive does not fit, fix the primitive. A primitive is done when its load-bearing behavior has run under a real producer, not when it compiles.
- No avoidable allocation, copying or recomputation in per-frame paths.
- ADR banner: each source or script file starts with one to three `// ADR: <decision> [— <why>]` lines (the file's comment syntax) that record its current decisions. Rewrite, do not append; no history. Markdown, TOML content and agent guidance carry none.
- Cargo owns `Cargo.lock`, caches and `target/`. Commit `Cargo.lock`. LFS tracks binary assets (`.gitattributes`); add a pattern before adding a new binary format.
- Git is local, with no remote. Stage only owned paths; no `git add -A`, reset or clean. Do not publish or add services unless asked.

## Verify and hand off

- After integration run `verify.cmd`: fmt check, Clippy with denied warnings, tests, headless smoke. For terminal changes also run the real terminal path (`play.cmd`, or `tools/wt-capture.ps1` for a screenshot) and report what was not verified: font, colour, keyboard, sound, cleanup.
- Tests are deterministic and cover behavior and boundaries, not wording, wiring or mock echoes. Fix contracts; do not weaken assertions.
- Update affected docs, remove probes, commit verified increments with terse messages.
- The lead keeps `.agent/checkpoint.md` under 50 lines: goal, constraints, open work, next action, last verification, HEAD. Update it at milestones and handoff.
- Retire outdated information in scope on every change. Delete completed tasks and superseded guidance; fix references. Git holds history.
- When something stalls you, record symptom, cause and fix in `.agent/common-issues.md`.

## Reporting to the user

- First what needs the user: each decision with a recommendation, each playtest request with the exact command. Then what changed as a player sees it, at most five bullets. Then the commit range. No process narrative, file lists or test counts unless they change a conclusion.
- Batch questions. Decide cheap, reversible engineering choices yourself and record the assumption.
- Hand over working things: run, render or test before asking the user to look.

## Writing

Applies to answers, docs, comments, ADR banners and commit messages; player-facing fiction follows `.agent/docs/player-text.md` instead. State facts, decisions, constraints and evidence for a reader who did not see the work. Short words, active voice, one fact per sentence. Name types, values and units. No narration, persuasion, figures of speech or invented vocabulary.
