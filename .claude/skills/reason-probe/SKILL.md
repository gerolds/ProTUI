---
name: reason-probe
description: >
  On-demand fresh-eyes externalized check on ANY load-bearing reasoning move you are
  about to lock in during a turn — a diagnosis, an inference from evidence, a chosen
  option over rejected ones, a claimed invariant, a completeness/coverage claim, a
  "this is safe / done / correct" judgment. It is the general form of the probe family:
  you cannot reliably grade reasoning you produced, because you grade it to pass. The
  fix is context separation — hand the move to a reviewer who doesn't share your
  construction. This skill is the router + shared doctrine: it makes you NAME THE
  ORACLE (what, outside your head, settles this), then dispatch to the specialization
  that fits — seam-probe (soundness), scope-probe (groundedness), readback-probe
  (comprehensibility) — or run the general procedure with the reasoning-adversary
  subagent when none fits. Use whenever you catch yourself about to trust a conclusion
  you both produced and checked. Invoke proactively — you do not need to be asked.
---

Read [the shared execution contract](../../ROUTING.md) before applying this skill.


# Reason probe — externalized "can I trust a conclusion I both produced and graded?"

You cannot reliably grade your own reasoning. When you generate a move *and* judge it, you judge whether the move coheres with the story you just told — and a move you built to cohere will always pass that check. This is one disposition failure wearing many masks: **hallucinated coherence** on a design, **narrative overfit** on a plan, **author's-context blindness** on prose, and — most often, least noticed — the quiet self-clearance on a live inference mid-turn ("the bug is in X," "these are the only cases," "this option is best," "this is safe to ship"). The fix is never introspecting harder. It is **context separation**: hand the move to a reviewer with no stake in your conclusion holding.

The three existing probes are three faces of this one mechanism, and they differ in exactly **one variable — the oracle**, the thing outside your head that settles the move:

| The move you're locking | The oracle that settles it | Polarity | Tool |
| --- | --- | --- | --- |
| A design decision / seam is **sound** | a walked failure — no external oracle; you must *construct* the counterexample | a **find** is signal | `seam-probe` |
| A plan / scope / taxonomy is **true** | the code + locked contract docs (the artifact) | a **flag** is signal | `scope-probe` |
| A plan **communicates** its intent | a competent reader who lacks your context | a **match** is signal | `readback-probe` |
| Anything else — an inference, diagnosis, root-cause, option-choice, completeness claim, claimed invariant | *name it* (§ below): usually the evidence, an enumeration, or plain logical validity | depends on what you test for | this skill + `reasoning-adversary` |

**Naming the oracle is the whole design of a probe run** — get it right and the tool, the prompt, and the polarity fall out; get it wrong and you built theater with extra steps. So the router is the point: identify the move, name its oracle, dispatch. Don't reinvent a specialization — if the move is a seam, a scope, or a plan's prose, invoke that probe and stop reading here.

Reach for this strategically; invoke it yourself when you hit the triggers — don't wait to be prompted.

## When to run it

- You're about to **act on a conclusion you both produced and checked** — the ones you're *most confident* in are where the self-clearance hides.
- A **diagnosis / root-cause** claim you're about to fix against ("the failure is X") — before you spend the fix.
- A **choice among options** where you've already picked — the rejected branch is where motivated reasoning buries the cost.
- A **completeness / coverage** claim ("these are all the cases," "nothing else calls this") — enumeration claims are cheap to assert and expensive to be wrong about.
- A **claimed invariant** or a "this is safe / done / correct" judgment you're about to lean on downstream.
- You notice **motivated reasoning** — you *want* the conclusion true, or your own check keeps returning "fine."
- You can't tell which specialization fits — run this to **name the oracle first**, then dispatch.

## Pick the oracle (the one design act that routes the run)

Ask: *what, outside my own head, would settle this move — and am I testing for a defect I expect absent, or a property I expect present?*

- **Testing for a defect that should be absent** (a confound, an ungrounded claim, a missed case) → a **find is the signal**, silence is only weak reassurance. Oracle is a walked counterexample (→ seam-adversary's mode), the artifact (→ grounding-adversary's mode), or an enumeration you can check against.
- **Testing for a property that should be present** (intent transmits, an invariant holds) → a **match is the signal**, divergence is the defect (readback-probe's inverted polarity).

If you cannot name an oracle at all, you don't have a checkable claim — you have a preference. Say so and stop; a probe can't rescue a move with no ground truth.

## The procedure (all four steps — skip one and it is theater)

1. **Pre-register.** Before spawning, write what you *expect* the reviewer to find (or, for a match-polarity check, the answer key — what a faithful reconstruction should say). This is the value signal: a return **outside** your prediction means separation worked; a return that merely **echoes** it means your prompt leaked your framing and induced your own blindness in the agent — a broken probe, not a confirmation. Some priors are **guardrails**, not noise: real constraints (cost, perf, a locked contract) the agent reasoning from scratch *cannot* reach; its silence on those is not a clearance.
2. **Spawn a context-separated reviewer with a neutral, faithful, self-contained prompt that points at the oracle.** State the move **truthfully** — the agent has none of your context, which is the point. Point it at the oracle it must check against (the code, the evidence, the enumeration) but **don't pre-digest that into your framing** — let it read the real thing cold. Give an open mandate; do **not** reveal the verdict you want, strawman the move, or steer toward a pre-chosen gotcha. For a match-polarity (comprehensibility) check, leak **zero** intent — the prompt is the plan text and nothing else. The prompt is the rig-point — keep it honest and inspectable. Use `reasoning-adversary` when no specialization fits.
3. **Verify against the oracle yourself — not against the agent.** Re-derive the finding: walk the failure, re-open the cited file, re-run the enumeration. The agent's return is a *pointer*; the oracle is the verdict. Do not trade your theater for the agent's — an unverified "fresh-eyes" finding is just a different unexamined conclusion.
4. **Reconcile — second opinion, not verdict.** You hold two diagnoses (yours and the agent's) and neither is authoritative; the one that survives a concrete walk against the oracle wins. Outcomes: a **real finding outside your priors** → integrate it, name the rethink; a **finding that doesn't survive the oracle** → push back and discard it *with a reason*, don't defer to "fresh eyes"; **silence on a real prior of yours** → not a clearance (fix the spec and re-probe, or clear it yourself); an **echo of your priors** → contaminated separation (step 1). **Guard against deference-as-performance:** "the fresh eyes humbled me" is a flattering trap — over-crediting the agent is the same disposition failure as blind self-trust in the opposite mask. Credit the priors that held; name exactly what the agent *added*.

## The honesty guardrails (name them; a missing one means it is theater)

**Separate context · pre-registered prediction · neutral inspectable prompt pointing at the named oracle · post-hoc verification against that oracle — the agent is never the verdict.**

Separation is the engine; the oracle is what the fresh eyes must check against; the other two guardrails stop you from quietly rigging the run. The one move that is always available and always self-deluding — "I reasoned it through again and it holds" — is the exact move this skill exists to replace. If you can't name an oracle and can't separate the context, you haven't probed the reasoning; you've re-narrated it.
