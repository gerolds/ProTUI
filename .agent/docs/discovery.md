# Discovery and planning

Run the full discovery below for high-cost work: a contract that later work builds on (a rules state model, a
command contract, a save format, a content format). For medium-cost work, run "Premise" and "Counter" without
writing them down, and name the primitives the work uses in the packet. Low-cost work needs neither. The user decides
game design; the agent proposes feel, pacing and rewards as playable variants ([playable-first.md](playable-first.md)).

## Discovery (before planning, before code)

Use each cue to look at the problems next to the one asked. The output is usually a few sentences or bullets.

- **Premise.** What is wrong with the question? Look for false choices, hidden premises, missing variables, the
  wrong time horizon, the wrong goal.
- **Levels.** What is the surface problem, the deeper problem, and the problem nobody names?
- **Models.** What model would you use to think about this, and what would it make visible?
- **Variations.** Which 3–5 variations of this problem matter? Vary along axes that change the design: lifetime,
  ownership, persistence, streaming, concurrency, error mode, cancellation. Take at least one variation from an
  adjacent system in this repo.
- **Collapse.** What stays the same across the variations? Those are the primitives. Sketch their shape:
  signatures, ownership, and failure as `Result` with a typed error or `Option` for "not there"
  ([rust-conventions.md](rust-conventions.md)).
- **Counter.** What is the strongest case that this is the wrong approach?
- **Escalation.** What will the next two weeks demand: one feature, one load case, one refactor? If the primitives
  do not support two of the three cheaply, revise them and check again.

Stop when you can name the primitives and they pass the escalation check. If too much is unknown, write a discovery
plan and stop.

**Model the loop before building its features.** Before the first rules packet of a new system, a small headless
model (an example binary printing numbers) checks that the system changes what the player's loop depends on, for
example the rate of progress, the wall that stops it, or the reset. On the sibling project a $5 model found what four
playtests and about $360 of feature builds had not: the game had no rate, no wall and no reset. A model settles
balance only. It cannot settle desire, pace or boredom; a played throwaway slice settles those, and it comes before
any crate ([playable-first.md](playable-first.md)).

## Planning

- Keep plans compact and operational. Leave out persuasion and narrative.
- Grounding pass: mark anything not grounded in context, inspected evidence or explicit constraints as assumption or
  guess, or turn it into a check (`scope-probe`).
- Seam pass: assume the draft hides problems because it reads well. Find each place a reviewer would object and
  quote the smallest phrase that causes it (`seam-probe`).
- If the plan collects too many assumptions or weak seams, it is a discovery plan. Say so.
- Open the plan with what changes for the player or the author and the decisions the user must make, each with a
  recommendation. Put the detail a packet needs after that.
- Working plans live in `scratch/` (git-ignored). Promote durable results into a guide, a crate README or `design/`.
