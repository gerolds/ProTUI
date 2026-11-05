---
name: readback-probe
description: >
  Produce the plain-language GROUNDING section for a plan — the held-in-head
  orientation a human (or a future agent) can carry without reconstructing intent
  from the obtuse details our planning regime tends to produce. It grounds the
  READER in the plan's intent (distinct from scope-probe, which grounds the plan's
  CLAIMS in artifacts). Spawns a context-separated cold reader (the plan-reader
  subagent) that reconstructs the plan's intent from the plan text ALONE; you
  pre-register the intent first and compare — a faithful match earns the section, a
  muddy or divergent readback means the PLAN is obtuse and must be revised. Use when
  finalizing or committing a plan — especially a long, dense, failure-mode-hardened
  one. Invoke proactively — you do not need to be asked.
---

Read [the shared execution contract](../../ROUTING.md) before applying this skill.


# Readback probe — externalized "can a stranger hold this in their head?"

Our planning regime is tuned to fix agent failure modes, so plans come out dense, defensive, and jargon-heavy — often correct but **obtuse**. The trap: **you can always summarize a plan you already understand.** Because you hold the intent in your head, your summary will read fine to you — and prove nothing about whether the *plan* carries that intent to someone who lacks your context. Comprehensibility has exactly one oracle: a competent reader who **doesn't share your context**. Hand the plan to that reader and have them **read its intent back** to you; the gap between what they recover and what you meant is the measure of how obtuse the plan is.

This is the comprehension twin of the other probes: **scope-probe** checks the plan is TRUE (grounded in artifacts), **seam-probe** checks a decision is SOUND, **readback-probe** checks the plan is COMPREHENSIBLE — that its intent survives without you there to narrate it. And the deliverable doubles as the test: the cold reader's faithful reconstruction, once it matches your intent, **is** the plain-language grounding section — the only summary *proven* recoverable without your context.

Reach for it strategically; invoke it yourself when you hit the triggers — don't wait to be prompted.

## When to run it

- **Finalizing / committing a plan** — especially a long, dense, or failure-mode-hardened one (the kind this repo's regime produces).
- A plan a human **or a future agent will act on later**, without the conversation that produced it.
- The plan is all **mechanism and guards** and you'd struggle to state its point in two sentences.
- **After scope-probe** — get it true first; there's no point making a wrong plan readable.
- You catch yourself about to **write the summary yourself.** That impulse is the tell that you're about to prove nothing.

## The procedure (all four steps — skip one and it is theater)

1. **Pre-register the intent — the answer key.** Before spawning, write the plan's true intent as *you* hold it (you have all the context): 2–4 plain sentences — the problem (*why*), the end state (*what*), the core approach (*how*). This is your prediction of the readback. Note the **polarity**, which is inverted from seam-probe: there you *want* the probe to surprise you; here a faithful **match is success** — it means the plan carries the intent — and **divergence is the defect.** If you can't write this answer key crisply yourself, stop: the plan has no clear intent to communicate, and that is a plan problem before it is a probe one.
2. **Spawn `plan-reader` with the plan and NOTHING else.** Hand it the plan text — *preferably pasted*, so the reader provably sees only the plan — or a path to the plan file alone. Neutral mandate: "reconstruct in plain language what this plan is trying to achieve and why; list what you could not determine; name the one thing most likely to be misread." Hygiene, in its **strictest form**: leak **zero** intent — no "this is about X," no conversation context, no answer hints, no pointing it at code it could read to back-fill meaning. The whole test is whether intent survives in the plan **alone**; any leak buys you a parrot and a false pass. (This is seam-probe's neutral-prompt rule taken to its limit: there, leaking the verdict makes the adversary agreeable; here, leaking *anything* makes the readback worthless.)
3. **Compare the readback to your answer key — the GAP is the signal.**
   - **Faithful + confident** (matches your intent; no load-bearing ambiguity) → the plan communicates. Go to step 4.
   - **Divergent** (the reader reconstructs a different goal or approach than you intended) → the plan **miscommunicates**. The divergence pins the exact passages that misled — fix *those passages*, then re-probe.
   - **Muddy** (the reader hedges, stays generic, recovers the *what* but not the *why*, or offers several incompatible readings) → the plan is **obtuse**. The reader's "where the intent went dark" list is your fix-list — revise the plan for clarity, then re-probe.
   - In every divergent/muddy case the defect is the **PLAN, not the section.** Never patch a clean summary over an obtuse plan: that makes the plan only *look* holdable while its body still demands reconstruction — the exact theater this skill exists to prevent.
4. **Adopt the validated readback as the plan's grounding section.** Only a readback that passed — faithful, confident, holdable — earns a place at the top of the plan (a `## Grounding` / `## Orientation` section). Use the **reader's** reconstruction, lightly edited — **not your own gloss** — because only the cold reconstruction is *proven* recoverable without context. Add no intent the readback didn't surface: anything you feel compelled to add is, by the test you just ran, **not** recoverable from the plan and belongs in the plan **body**, not smuggled into the summary. Re-running the probe on the revised plan should pass on the first read; if it stays muddy after targeted fixes, the plan's **structure** (not its wording) is the problem — a planning defect, not a phrasing one.

## The honesty guardrails (name them; a missing one means it is theater)

**Separate context · pre-registered intent (the answer key) · zero-leak prompt (the plan text, nothing else) · the gap is the verdict — muddy ⇒ fix the plan, never the section.**

The distinctive trap here, versus the other probes: **a summary you write yourself proves only that *you* hold the intent — never that the plan carries it.** So the grounding section is not something you *author*; it is something you *earn* — the reconstruction a stranger produced from the plan alone that turned out to match what you meant. If no such reconstruction exists yet, you don't have a grounding section — you have an obtuse plan wearing one.
