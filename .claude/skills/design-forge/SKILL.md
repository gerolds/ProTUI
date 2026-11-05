---
name: design-forge
description: A stateful, pre-registered design-and-review loop for open-ended, high-stakes tasks with no single right answer. Use whenever the user asks for architecture or system design, technical or strategic planning, design review, a build-vs-buy or approach decision, a migration or refactoring plan, an RFC/ADR, or any recommendation where multiple plausible approaches exist and tradeoffs matter. Also trigger when the user says "think this through properly", "don't just give me the first answer", "compare approaches", or asks for a decision they will act on. Do NOT use for factual lookups, single-constraint tasks, or quick code fixes.
---

Read [the shared execution contract](../../ROUTING.md) before applying this skill.


# Design Forge

A commitment-ordered workflow for design, architecture, planning, and review
tasks. It converts inference budget into decision quality by forcing the work
that produces good judgment — constraint inventory, genuine alternatives,
adversarial critique — to happen **in an order that cannot be faked
retroactively**, with artifacts on disk as the enforcement mechanism.

## Why the structure is the way it is (read this, it changes how you should behave)

The failure mode this skill exists to prevent is **process theater**: a model
(you) settles on an answer within the first few tokens of thinking, then
generates artifacts — "options", "critiques", "revisions" — that are
back-formed justifications for that answer. The output *looks* like a design
review. It is actually a press release.

Three mechanisms make theater cheap, and this skill breaks each one:

1. **Retro-fitting**: writing evaluation criteria after seeing the options, so
   the criteria favor the pre-chosen winner. → Broken by **pre-registration**:
   the rubric is written and committed to disk *before* any option exists, and
   amendments after that point are allowed but logged and visible.
2. **Strawmanning**: generating alternatives designed to lose. → Broken by the
   **axis-distinctness requirement**: options must occupy genuinely different
   points on named design axes, and each option must be written as if a
   competent advocate proposed it.
3. **Generic critique**: objections that apply equally to everything
   ("complexity risk", "may not scale") and therefore change nothing. →
   Broken by **ID-grounded critique**: every objection must cite a specific
   constraint or rubric item by ID, and every option must receive at least one
   *potentially fatal* objection. If you cannot find a fatal objection to an
   option, the option spec is too vague to falsify — sharpen it and try again.

The most important device is the **sealed lean** (Phase 0): you write your
gut answer first, seal it, and never look at it again until the audit at the
end. If the process rarely changes your gut answer across many uses, the
process is decoration — and the audit makes that measurable. Do not skip
Phase 0 and do not peek. Its entire value is that it exists *before* the work.

This costs tokens and time. That is the point: the extra budget goes into
alternatives you will reject and objections you must answer. Do not compress
phases together, and do not summarize a phase in chat instead of writing its
artifact. The files are the process.

## Setup

Use a unique run directory `scratch/forge/<timestamp>-<topic>/`. `/scratch/` is git-ignored. Do not stage working artifacts unless requested.

Create files with the host's file-writing tools. In `ledger.log`, record each creation at the moment it happens:

```text
2026-01-01T12:00:00Z CREATE 02_rubric.md
2026-01-01T12:01:00Z AMEND 02_rubric.md Reason: new measured constraint C4
```

Use real UTC timestamps, not these example dates. An amendment is legal; a silent amendment is not. Here **committed** means saved with its ledger entry before the next phase, not a required git commit. Never invent a creation order after the fact.

Scale to the task: for a medium-stakes question, phases can be short (a
constraint inventory can be ten lines). What is not negotiable is the *order*
and the *separation*. Never merge Phase 4 into Phase 3's file.

## The phases

Run these strictly in order. Each phase = one or more files committed before
the next phase begins.

### Phase 0 — Sealed lean → `00_sealed_lean.md`

Before any analysis, write 3–6 sentences: what your immediate instinct says
the answer is, and why. Commit it. **Do not open or reference this file again
until Phase 6.** If you notice yourself steering later phases toward this
lean, that is the exact bias the skill exists to catch — let the process run.

### Phase 1 — Problem frame & constraint inventory → `01_frame.md`

- Restate the problem operationally: what decision will be made, who acts on
  it, what "failure" concretely looks like.
- Inventory with stable IDs, one item per line:
    - `C#` constraints — split **stated** (traceable to the user's words; quote
      or cite the phrase) vs **inferred** (your assumption; say why you infer it).
    - `A#` assumptions you are making to proceed.
    - `U#` unknowns that materially affect the answer. For each: could you ask
      the user or look it up cheaply? If yes, do so now, before Phase 2.
    - `F#` failure conditions — outcomes that would make any answer wrong.
- Mark which items are **load-bearing** (the decision plausibly flips if they
  change).

### Phase 2 — Pre-registered rubric → `02_rubric.md`

Written before any option exists. Contains:

- `R#` evaluation criteria, each with a weight (H/M/L) and the constraint IDs
  it derives from. A criterion that traces to no constraint is decoration —
  delete it.
- **Kill conditions**: rubric items that are pass/fail, not scored. An option
  that violates one is eliminated regardless of other merits.
- The scoring procedure you will use in Phase 5, stated now.

After this file is committed, new criteria may be added only via an `AMEND`
ledger entry naming what was learned that justifies the addition. A criterion
added after options exist that happens to favor one option deserves your own
suspicion — note that suspicion in the amendment.

### Phase 3 — Options → `03_option_A.md`, `03_option_B.md`, `03_option_C.md`, …

First, in `03_axes.md`, name the design axes this problem actually has
(e.g., sync/async, centralized/federated, buy/build/extend, big-bang/
incremental, generalize/specialize). Then generate **at least three** options
such that every pair differs on at least one axis — cosmetic variants of one
idea do not count.

Each option file must be **self-contained** (the Phase 4 critic reads it in
isolation) and written **as its best advocate would write it**: mechanism,
how it satisfies the load-bearing constraints, resource/effort sketch, what
it deliberately gives up. If you cannot write a genuine advocate's case for
an option, replace it with one you can — a strawman in the set corrupts the
comparison.

Do not rank, compare, or express preference anywhere in Phase 3 files.

### Phase 4 — Blind adversarial critique → `04_critique_A.md`, `04_critique_B.md`, …

Role switch. For each option, produce a critique using **only** three inputs:
that option's file, `01_frame.md`, and `02_rubric.md`. Do not consult your
chat reasoning, the other options, or anything else from earlier phases.
Behave as a reviewer with an incentive to find real problems.

Rules for a valid critique:

- Every objection cites at least one ID (`C#`, `A#`, `U#`, `F#`, or `R#`).
  An objection that cites nothing is invalid — delete or ground it.
- At least one objection per option must be marked `FATAL-IF:` — a condition
  under which this option should be killed outright. If no fatal objection
  exists, the option spec is unfalsifiable; go back and sharpen `03_option_X.md`
  (via an `AMEND` entry), then critique again.
- Attack the assumptions (`A#`) the option silently relies on, not just its
  mechanics. Check it against every kill condition explicitly.
- Forbidden: objections true of all options equally, unless you then say how
  severity *differs* per option.

### Phase 5 — Scoring, decision → `05_decision.md`

- Score each option against the pre-registered rubric — the committed `R#`
  items and weights, plus any logged amendments. Nothing else.
- Apply kill conditions first. Then resolve every `FATAL-IF:` from Phase 4:
  for each, state whether the triggering condition holds, and your evidence
  or explicit uncertainty. "This is unlikely" without a reason is not a
  resolution.
- Recommend one option (or a justified hybrid — but a hybrid must inherit and
  resolve the fatal objections of *each* parent).
- **Mandatory concession section**: name at least one real thing a losing
  option did better and what accepting that loss costs. If you cannot write a
  genuine concession, your options were not genuinely distinct — that is a
  Phase 3 failure; go back.
- State the conditions under which this decision should be revisited (tie to
  `U#` unknowns).

### Phase 6 — Audit → `06_audit.md`

Now, and only now, open `00_sealed_lean.md`.

- **If the decision matches the sealed lean**: list every `FATAL-IF:` that was
  raised against it in Phase 4 and, for each, quote the specific resolution
  from Phase 5. A gut answer surviving genuine adversarial review is a fine
  outcome — but the survival must be demonstrated line by line, not asserted.
- **If the decision differs**: name the specific critique or constraint that
  moved it.
- Run the checker and paste its output into the audit file:

```bash
python .claude/skills/design-forge/scripts/check_process.py <run-directory>
```

Run this only when the assignment permits checks; otherwise hand the exact command to the integration owner and mark the audit unverified. Fix any observed FAIL before presenting a verified audit. The checker verifies structure, not quality —
passing it is necessary, not sufficient.

### Presenting to the user

The user gets the decision, not the archaeology: recommendation, the 2–3
decisive tradeoffs, the concession, revisit conditions, and the execution
sketch if they asked for one. Offer the forge directory ("full working
artifacts available") rather than dumping it. If the audit revealed that the
process changed your initial lean, say so — users find it informative.

## Anti-patterns (recognize these in your own output)

Read `references/theater_patterns.md` before your first run with this skill,
and re-read it if the user challenges a decision. It catalogs the specific
ways this process gets faked — retro-fitted rubrics, decoy options, severity
laundering, resolution hand-waving — with examples of each, so you can detect
them in your own artifacts during the Phase 6 audit.

## When to bail out

If during Phase 1 you discover the problem has one plausible answer and no
real tradeoffs, say so, answer directly, and note in one line why the loop
was unnecessary. Running theater in reverse — pretending a closed question is
open — is also theater.