---
name: scope-probe
description: >
  On-demand fresh-eyes GROUND-CHECK of a scope, plan, classification, taxonomy, or
  milestone structure before you commit it — the antidote to overfitting a plan to
  NARRATIVE coherence instead of to the actual code + contracts. Spawns a
  context-separated reviewer (the grounding-adversary subagent) that traces each
  load-bearing claim to the real artifact and flags the ones that are invented,
  already-settled by the existing system, mechanism-mismatched, or contradicted by a
  locked contract. Use BEFORE committing any plan/scope/taxonomy — ESPECIALLY when
  matching or replacing an existing system (where most "decisions" are already
  settled), when you've produced a clean same/loosen/new or tier/primitive
  classification, or whenever the plan feels elegant and coherent (coherence is the
  warning sign, not the green light). Invoke proactively — you do not need to be asked.
---

Read [the shared execution contract](../../ROUTING.md) before applying this skill.


# Scope probe — externalized "is this grounded, or just a good story?"

A plan can be perfectly coherent and entirely wrong. **Coherence is not correctness.** When you both write a plan and check it, you check whether the *story* hangs together — clean categories, a sensible arc, decisions that follow from premises — and a story you built to cohere will always pass that check. That is **narrative overfit**: fitting the scope to a satisfying narrative instead of to the ground truth. It is the planning-stage twin of the hallucinated-coherence failure, and it wastes whole plans (and the builds that follow them).

Unlike a design seam — where soundness has no external oracle and you must walk the failure — a *plan* almost always has an oracle: the actual code and the **locked contract docs**. So the fix is two moves: **context separation** (hand the plan to someone who doesn't share your narrative) *and* **appeal to the artifact** (the agent's job is to GROUND each claim against the real thing, and the artifact — not the agent, not you — is the verdict).

Reach for this strategically; invoke it yourself when you hit the triggers — don't wait to be prompted.

## When to run it

- About to **commit a plan, scope, classification, taxonomy, or milestone structure** — the more elegant it feels, the more it pays.
- The task is to **match / replace / extend an EXISTING system.** This is the highest-risk case: the existing system *already settled* most "decisions," so any "new decision" you list is a prime overfit suspect.
- You produced a **clean taxonomy** — `same/loosen/new`, cost tiers, a "primitive set," a milestone arc — and the boundaries feel satisfying. Story-convenient boundaries are the tell.
- You're **naming a technique or option** (algorithm, pattern, "X vs Y") — verify the mechanism actually applies before offering it (e.g. a method that only exists for a *different* algorithm than the one in use).
- A plan **leans on a speculative / redesign / discovery doc** — those can be wrong; they must be checked against the locked contract.
- You catch yourself feeling the plan is **elegant**. Elegance is a warning sign at planning time, not a green light.

## The procedure (all four steps — skip one and it is theater)

1. **Pre-register the load-bearing claims.** Before spawning, list the plan's decisions, options, classifications, and scope boundaries — and tag each: *grounded* (you can point to the artifact), *believed* (you think so but haven't checked), or *narrative* (it's there because the story needs it). The "believed" and the ones you're **most confident** in are where overfit hides. This list is the value signal: if the probe flags something **outside** it, separation worked; if it only **echoes** your "believed" tags, your prompt leaked your framing.
2. **Spawn `grounding-adversary` with the plan + pointers to the real artifacts.** State the plan **truthfully and self-contained**, and point it at the code + the contract docs it should check against (don't pre-digest them — let it read them). Open mandate: "for each load-bearing claim, decision, option, and category, GROUND it in the cited artifact; flag every one that is invented, already-settled by the existing system, mechanism-mismatched, or contradicted by a locked contract; cite the file:line / contract §." Do **not** defend the plan, reveal which claims you want kept, or pre-digest the artifacts into your framing — the agent reading the real thing cold is the point.
3. **Verify each flag against the ARTIFACT — not the agent, not your memory.** Re-open the cited code/contract yourself. The agent's flag is a pointer; the artifact is the verdict.
4. **Reconcile by appeal to the artifact.** For each flag, the locked contract / code decides it:
   - **Artifact confirms the flag** (the claim is invented / already-settled / mismatched / contradicted) → the plan was overfit there. Fix it: reclassify the "decision" as parity, delete the invented option, drop the story-driven boundary. A reclassification that collapses a whole section is the high-value find — say so.
   - **Artifact supports your claim** → false flag. Keep the claim, but now **with the citation you were missing** — the probe made you ground it.
   - **No artifact exists either way** → it's a *genuine* open item (a real new limitation, or a true decision the existing system never faced). These are the few things worth discussing. Everything the artifact settles is NOT.
   - **Agent echoes your "believed" tags, finds nothing the artifact contradicts** → contaminated separation (step 1) OR a genuinely well-grounded plan. Distinguish by checking whether the agent actually READ the artifacts (did it cite file:line?) — an agent that didn't read is a broken probe, not a clearance.

## The honesty guardrails (name them; a missing one means it is theater)

**Separate context · pre-registered claim-tags · neutral prompt that points at the real artifacts · the artifact is the verdict.**

Separation gets fresh eyes on the plan; the artifact is what those eyes must check against. The distinctive trap here, versus a design seam: you do **not** get to "walk it and decide" — for a scope/plan there is a ground truth, and "I reasoned it through" is exactly the move that produced the overfit. Ground every load-bearing claim in a real artifact, or mark it an explicit assumption. A plan whose every "decision" turns out to be settled in a contract you hadn't read was never a plan — it was a story.
