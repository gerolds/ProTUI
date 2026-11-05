---
name: natural-extension
description: >
  Check architectural fit before adding glue. Use whenever a feature, change, or integration in an
  existing codebase does not fall out of the current design — when the implementation
  you are about to write needs an adapter, wrapper, shim, special case, feature flag,
  parallel path, copy-with-divergence, or an exception to an invariant. Also use when:
  the user asks how something "should be shaped" or how to make an ask "fit"; when
  starting work in a grown, unevenly specified codebase (consider descriptive law extraction); when a
  LAWS.md or ANOMALIES.md exists in the repo (they are this skill's optional artifacts — read them under root guidance); when reviewing a diff for architectural fit; when deciding refactor-first vs
  feature-first; during integration-phase or last-20% work where the final shape is
  not known. Invoke proactively the moment you notice yourself gluing — you do not
  need to be asked, and the halt is the point: do NOT write the shim first and ask
  forgiveness after.
---

Read [the shared execution contract](../../ROUTING.md) before applying this skill.


# Natural extension — what set of laws solves this problem as an expression of itself?

> Seek the change in everything that makes the ask no change at all.

The ask should land as a **theorem** of the codebase's laws, not as an axiom bolted on
with glue. When it doesn't, the misfit is not a construction problem — it is
**evidence about the model**. You hold three levers at every misfit: the
implementation, the model, and the ask itself. Spending all your degrees of freedom
on the first lever is how correct-but-complicated shims happen. This skill forbids
that spend without a hearing.

## Doctrine — settled vocabulary, cite it, never litigate it

These names are load-bearing. Each one imports its whole apparatus; respond to the
meaning, do not re-derive or debate it in-session.

| Name | What it settles here |
| --- | --- |
| **Lakatos** (hard core / protective belt; progressive vs degenerating programmes) | The laws are the hard core; code is the belt. A shim is an **epicycle**: belt grown to spare a wrong core. A programme that only ever grows belt is degenerating. Misfit = falsification data. |
| **Naur** (*Programming as Theory Building*) | The theory is the product; code is the byproduct. "This is confused, rewrite everything" is what a theory-less codebase feels like from inside. Rewrite cost is low **iff** the theory exists outside the code. |
| **Beck** ("make the change easy — warning: this may be hard — then make the easy change") | Core revision ships as two changes: first a behavior-preserving amendment that makes the ask natural, then the now-trivial ask. Never interleave them. |
| **Deutsch** (good explanations are hard to vary) | In a derived structure every part has a reason. "Could this be otherwise without consequence?" — if yes, it is glue, not derivation. |
| **Ousterhout** (deep modules, narrow interfaces; tactical vs strategic programming) | The default agent failure mode is hyper-tactical. Depth behind narrowness is what the laws exist to protect. |

## Artifacts

Use existing guidance first. A closed reuse task or a small change with one
plausible implementation does not require new law files, ledgers, human
ratification or an architectural review packet. The model still matters:
identify the invariant, use the existing seam and implement the direct change.

For proven architecture that needs explicit laws, these optional artifacts
live at the repo root and are versioned with the code. Create them only when
the assignment authorizes them; do not legislate speculative future systems.

**`LAWS.md`** — the hard core. Single-digit count, brutally small. Each law:

```
## L3 — Every resource is exposed through the lifecycle pattern
Rationale: one sentence — why this, what it buys under future change.
Exemplar: path/to/derived_thing.py
Anti-exemplar: path/to/thing_that_glues.py (or "none yet")
Jurisdiction: everything except modules marked EXPLORATORY
```

**`ANOMALIES.md`** — the ledger. When this ledger is adopted, record each accepted shim **against a specific
law** at integration:

```
## A17 — 2026-08-01 — violates L3
Shim: retry-adapter wrapping the ingest client
Why accepted: deadline; revision touches 4 modules
Cost it adds: every new ingest source must know about the adapter
Status: open
```

The ledger is the repaired pain signal: an agent absorbs friction silently, but it
cannot absorb a ledger entry. Clustering on one law is a quantitative fact replacing
the vague feeling "this whole thing is confused" — it says exactly *where* the
theory is wrong.

## Procedure

**1. Load the actual constraints.** Read existing root guidance, relevant contracts
and any adopted `LAWS.md` and open anomalies. If no law file exists, do not create
one as a prerequisite to an ordinary implementation. For mature, repeated
architectural misfit, extract what the code *currently obeys*, including
contradictions, and propose the smallest useful statements. Their acceptance
follows the root's decision authority. Never invent laws ahead of working code
(see Guards: premature legislation).

**2. Attempt derivation.** Design the ask as a theorem of the laws. If the design
falls out — implement it, and cite existing governing constraints when useful
(for an adopted law file: `Derives from: L2, L5`). Done; the rest of this skill never fires.

**3. Halt on epicycle.** If the design requires any of: adapter, wrapper, special
case, flag guarding a single caller, parallel code path, copied-then-diverged code,
an exception to a stated invariant, or anything you would annotate "for now" —
**STOP. Do not write it.** Not even as a draft, not even to "show the option": a
working diff has gravity, and the shim you wrote to illustrate becomes the shim
that ships. Produce a Misfit Report instead.

**4. The Misfit Report.** Four sections, all mandatory:

```
# Misfit: <ask, one line>

## Conflict
Which laws, versus which part of the ask. Quote the law numbers.

## The epicycle
Shape and size of the shim; which specific future changes it taxes
(name the next two features that would have to know about it).

## The revision
The MINIMAL amendment to the core under which the ask becomes a theorem.
Blast radius: modules touched, migration shape, what breaks.
Beck two-step: what change-1 (behavior-preserving) and change-2 (the ask,
now trivial) each look like.

## The re-ask
A reshaped requirement that dissolves the conflict entirely.
NEVER blank, never "N/A". If the honest answer is "the ask is fine",
say why the ask is already law-shaped and this section found nothing —
but look first. This section exists because you will otherwise never
volunteer requirement critique; sometimes the cheapest law-compatible
move is that the human wanted slightly the wrong thing.
```

**5. Resolve within the assigned authority.** Decide reversible engineering changes; ask the user about changed experience or costly contracts. Three possible verdicts:
- **Shim** → implement it *and* write the `ANOMALIES.md` entry in the same change.
  If no ledger is adopted, state the explicit exception and cost in the existing change record.
- **Revise** → execute the Beck two-step as two separate changes. Update adopted laws or existing guidance
  in change-1. Re-derive or close any open anomalies the amendment dissolves.
- **Re-ask** → return the reshaped requirement; implement nothing yet.

**6. Revision sessions.** When ≥3 open anomalies cluster on one law, propose a
revision session whose only permitted goal is amending that law and dissolving the
cluster. Every clustered anomaly must end the session either re-derivable from the
amended core (close it, note the resolution) or consciously re-accepted with an
updated cost line. Cadence-driven if clustering never quite triggers — the friction
signal is muted by design, so shape-work cannot wait for pain.

## Law quality tests (apply when writing or amending laws)

- **Generative:** can the next feature's design be derived from the law alone,
  before anyone designs it? A law that only forbids is half a law.
- **Hard to vary** (Deutsch): walk the derived structure asking "could this be
  otherwise without consequence?" — every "yes" is glue the law failed to reach.
- **Earns its place:** a law that has neither blocked a shim nor generated a design
  in recent memory is belt wearing a core costume. Propose pruning it.
- **Pairwise compatibility:** check laws against each other; a law-law conflict is
  a shim factory, and the code will find it before you do unless you look.

## The critic split

For a load-bearing architectural review, use a context-separated reviewer when
the host and assignment permit it. The rubric is the root contract plus adopted
laws, not a new authority above them. Ask per hunk: **derived or glued?**
Glued without a ledger entry → reject. The generator cannot self-grade against
the core it was tempted to spare; do not merge on the generator's own verdict.

## Promotion discipline — the test that the model exists

Spikes may explore outside settled laws. Preserve a compact **harvest**:
constraints discovered, decisions made, evidence, and laws challenged. Before
landing, remove experimental paths and shape production code around those
findings. A clean implementation from the harvest by a fresh agent can test
whether the model is explicit, but is not required for a sound reuse task.
Do not reimplement proven code merely to satisfy a process ritual. A working
spike is evidence, not automatic approval to promote all of its structure.

## Guards

- **Proportionality.** Not every conditional is an epicycle. The halt triggers on
  structure that exists *only to reconcile the ask with the model* — branching that
  is domain logic is just domain logic. Do not weaponize this skill into scope
  explosion: revisions are **minimal** amendments with stated blast radius, and
  "rewrite the module" is almost never the minimal amendment.
- **Premature legislation.** Modules marked `EXPLORATORY` are outside jurisdiction
  (Lakatos: young programmes get leniency). Laws are extracted from code that has
  proven itself, then promoted — not decreed ahead of evidence. Legislating a module
  still finding its shape freezes exploration exactly where it is needed.
- **Meta-cruft.** `LAWS.md` degrades under the same dynamics it polices: laws
  accumulate patch-shaped. Apply the earns-its-place test on every revision session.
  Single digits, always.
- **The always-available self-deluding move** is: *"the shim is correct, I'll just
  add it."* Correctness of the shim was never the question. The question is what it
  does to the derivability of everything that comes after it — and that question is
  answered in a Misfit Report or not at all.