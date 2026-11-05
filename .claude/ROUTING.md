# Shared capability routing

All agents use these canonical files. `.claude` is a storage convention, not a
requirement to use a particular model, vendor, editor, or agent API. Root guidance
and the current assignment take precedence over every skill and companion role.

## Execution contract

- Read only the triggered skill and its named references. Reuse mechanisms, not
  imported game design, assets, historical decisions, approval claims or status.
- Use the host's file/search/agent tools. A role name below denotes a prompt file,
  not a guaranteed installed agent type. When delegation is allowed, read the role
  and give it to an available read-only reviewer with the stated isolation. Do not
  invent unsupported tool calls. Never nest delegation when the assignment forbids it.
- Scale process to the open question. A concrete reuse task with one plausible
  implementation does not require forge artifacts, law extraction or new ledgers.
  Do not turn skill triggers into permission for scope growth or repeated reviews.
- Probe findings are pointers, not verdicts. Inspect the named oracle before acting.
  Do not manufacture a failure to sound rigorous. Mark missing evidence explicitly.
  If no isolated reviewer is available, report that limit rather than claim a probe.
- Check/build/test/formatter commands are recipes, not permission to run them.
  Follow the assignment's verification owner and timing. A leaf in a shared tree
  hands checks to the integration owner. Never call an unexercised command passed.
- Do not create permanent paperwork or placeholder deliverables for process alone.
  Use a host scratch location for working artifacts unless the assignment requests
  repository documents. Root ownership also applies to checkpoints and commits.
- Preserve user changes, source provenance and licensing. Game rules and style are
  not implied by a rendering, audio, design or documentation primitive.

## Skills

| Trigger | Canonical skill | Companion / payload |
|---|---|---|
| Write, edit, find or review documentation | [docs](skills/docs/SKILL.md) | Four [worked examples](skills/docs/examples/forms.md) |
| Several plausible high-stakes designs or a recommendation with real tradeoffs | [design-forge](skills/design-forge/SKILL.md) | [Theater patterns](skills/design-forge/references/theater_patterns.md), [functional checker](skills/design-forge/scripts/check_process.py) |
| Architectural misfit, special-case glue, copied parallel path or invariant exception | [natural-extension](skills/natural-extension/SKILL.md) | Optional laws only for proven architecture |
| Load-bearing inference, diagnosis, completeness or safety claim | [reason-probe](skills/reason-probe/SKILL.md) | [reasoning-adversary](agents/reasoning-adversary.md) |
| Design boundary, lifecycle, ownership, persistence or concurrency seam | [seam-probe](skills/seam-probe/SKILL.md) | [seam-adversary](agents/seam-adversary.md) |
| Scope, parity, category or plan claim against existing code/contracts | [scope-probe](skills/scope-probe/SKILL.md) | [grounding-adversary](agents/grounding-adversary.md) |
| A later reader must recover a plan's intent without the conversation | [readback-probe](skills/readback-probe/SKILL.md) | [plan-reader](agents/plan-reader.md); plan text alone |
| User explicitly requests an interactive design interview | [grill-me](skills/grill-me/SKILL.md) | Inspect source-answerable questions first |
| Game prototype slice, new system, scope proposal or design question to the user | [playable-first](../.agent/docs/playable-first.md) (guide, not a skill) | Gates in order; agent plays first with timed measures |
| Player-facing fiction: intro, work order, report, dialogue, description | [player-text](../.agent/docs/player-text.md) (guide, not a skill) | Two competing drafts for voice questions |
| Continue work in another session | [handoff](skills/handoff/SKILL.md) | Link existing artifacts; respect checkpoint ownership |
| Simplify technical prose or write for non-native readers | [simple-english](skills/simple-english/SKILL.md) | Full compliance still needs the official STE dictionary |

## Companion isolation

`seam-adversary` walks one concrete structural failure. `grounding-adversary`
checks all load-bearing claims against real artifacts. `reasoning-adversary`
names and checks the oracle for one reasoning move. `plan-reader` receives only
the plan text, never code, links, prior discussion or the author's intended answer.
Each role is read-only; its frontmatter limits its tools.

Topical engineering guidance lives in [the local guidance index](../.agent/docs/README.md).
