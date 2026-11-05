---
name: seam-probe
description: >
  On-demand fresh-eyes adversarial test of a design decision, seam, or implementation
  approach during coding. Spawns a context-separated reviewer (the seam-adversary
  subagent) that has no stake in the design surviving, so it surfaces confounds you
  would otherwise rationalize away. Use BEFORE committing to a load-bearing decision,
  when a choice might foreclose future work, when crossing a boundary another feature
  will need, or whenever you catch yourself wanting a design to be true ("is this safe
  to weld shut?"). Invoke proactively — you do not need to be asked.
---

Read [the shared execution contract](../../ROUTING.md) before applying this skill.


# Seam probe — externalized "finger along the seam"

A human building something runs a finger along each seam they touch and decides *safe / not-safe* from memorized experience and first principles. You **cannot do that reliably solo**: when you both construct the experiment and judge it, you construct it to land on the conclusion you already want. That is theater — the hallucinated-coherence failure mode that wastes whole rebuilds. The fix is not cleverness, it is **context separation**: hand the probe to someone with no stake in the design surviving.

This skill is that, on demand. Reach for it strategically; decide to run it yourself when you hit the triggers — do not wait to be prompted.

## When to run it

- About to **lock a load-bearing decision** — the ones you are *most confident* in are where it pays most.
- A choice might **foreclose future work** ("weld this shut now?").
- **Crossing a boundary** another feature will need: batched write/read-before-continuation, lifetime/ownership, persistence, concurrency, error/cancellation.
- You notice **motivated reasoning** — you *want* the design to be true, or your own check keeps returning "safe."
- High stakes: re-run it on the proposed **fix**, or run several probes from different angles.

## The procedure (all four steps — skip one and it is theater)

1. **Pre-register.** Before spawning, write down what you *expect* the probe to find. These priors are **not noise to be overridden** — some are guardrails: real constraints and known failure modes the agent *cannot* reach, because reasoning-from-scratch is its strength and its blind spot. The prediction is also the value signal: if the probe returns something **outside** it, context-separation worked; if it merely **echoes** it and finds nothing new, separation *failed* — your prompt leaked your framing and induced your own blindness in the agent. Treat an echo as a broken probe, not a confirmation.
2. **Spawn `seam-adversary` with a neutral, faithful, inspectable prompt.** State the design / decision / seam **truthfully and self-contained** — the agent has none of your context, which is the point. Give an **open mandate**: "find the single strongest structural problem, pick your own target, walk a concrete scenario, name internal contradictions, rate how fundamental the fix is, don't be agreeable." Do **not** reveal the verdict you want; do **not** strawman the design; do **not** steer toward a pre-chosen gotcha. The prompt is the rig-point — keep it honest, and keep it visible to anyone watching.
3. **Verify the walk yourself.** When it returns, re-derive the failure. The **problem must be real** before you act on it — do not trade your theater for the agent's. The agent's proposed **fix is a candidate, not a conclusion**: grill it separately, often with another probe.
4. **Reconcile — the agent is a second opinion, not a verdict.** You now have two diagnoses: the agent's and your priors. **Neither is authoritative.** Triage by re-deriving *both* against the concrete design/code; the one that survives a concrete walk wins. The four outcomes:
   - **Agent finds a real confound outside your priors** → integrate it (a substantial rethink — say so). Separation earned its keep.
   - **Agent's finding doesn't survive your walk** → false positive. **Push back and discard it, with a reason.** Do not defer because it came from "fresh eyes."
   - **Agent is silent on a prior of yours that is actually real** → its silence is **not a clearance**. Either your spec to it was incomplete (fix the spec, re-probe) or the concern only surfaces with context it lacks (keep it live; *you* must clear it). A real example: a cost/perf prior an adversary reasoning about soundness never reaches.
   - **Agent echoes your priors, nothing new** → contaminated separation (step 1).
   - Signal about the **design, not just the probe**: if the agent misses problems you already know are real, the design *as you specified it* is under-specified — a well-stated design lets fresh eyes reach its real faults.
   - **Guard against deference-as-performance.** "The fresh eyes humbled me" is a flattering narrative and a trap — over-crediting the agent is the same disposition-failure as blind self-trust, wearing the opposite mask. Credit the priors that held; name exactly what the agent *added*. A probe that only ever returns "safe" wasn't a real probe; an agent you only ever agree with wasn't a real second opinion.

## The four honesty guardrails (name them; a missing one means it is theater)

**Separate context · pre-registered prediction · inspectable neutral prompt · post-hoc verification.**

Separation is the engine. The other three are what stop you from quietly rigging it.
