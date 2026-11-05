# Playable first: how agent-built game prototypes fail

Read before any game prototype work: a new slice, a new system, a scope proposal, a design question to the user. It applies to any game built by agents for a designer. Record this repository's playtests and failures in `design/iteration.md`.

## The failure

The same failure happened three times across two projects. Each time the build followed its spec, and the designer bounced off it in minutes: "obtuse, unfun, slow, pointless, devoid of rewards". Every agent check had passed.

The steps that led to it:

1. The designer approves parts one at a time: a layout full of hand-written sample text, numbers from a headless model, sound clips. Nobody judges the whole.
2. The agent asks mechanical questions, such as a threshold, a rule or what a failure costs. Each answer adds a rule, and each rule becomes text on screen.
3. The questions test clarity ("can the player explain it?") and balance ("does one plan win everything?"). Neither tests whether anyone wants to play. Legibility is a floor, not a goal.
4. Parallel agents build breadth fast: many screens, saves with migrations, themes, escape hatches. The engineering rigour is real, but it is spent on a target nobody has played.
5. The build's text comes from rule tables. It prints arithmetic and category labels, not the people and causes the mockup promised.
6. The economy is tuned against a model that plays well. A first-time player waits minutes and ends up poorer.
7. The agents' engineering writing style (terse, one fact per sentence) leaks into the fiction.
8. Nobody owns fun. Design questions "belong to the user", so the agent proposes no feel, pacing or reward. The user's first real play is the first judgement of the whole.

## Gates

Pass these in order. A later gate never excuses an earlier one.

1. **Hook stated.** Write one sentence on what holds a first-time player for five minutes, in the designer's words. If the design documents conflict on it (for example, "idle appeal" against "no reward systems"), ask before building.
2. **Find the formula, then find it in the fantasy.** Matching is the agent's job, not the designer's; do not ask the designer to choose references. A clone or a noun-swap is not the goal.
   - Write the formula as *jobs*. Each element of a proven game does one job: one number that grows, a manual verb, the purchase that replaces it, geometric prices, ×2 thresholds, a visible next goal, reveal by permission, phase change, mystery, low load. For each job, cite how two or three proven games do it.
   - Then choose, for each job, the element of *this* fantasy that does that job natively, and write the mapping as a table. The best mappings make a treatment's own loop carry a job, for example an expedition that is the tier unlock.
   - Take numbers from the references as a start, model the loop headless, and show the reveal timeline.
   - Check which job each failed attempt broke; that diagnosis tests whether the formula is understood.
   - Mechanics collected from many games and recombined, without naming the job each one does, gave a dull loop, then one that was too much work, then a pointless one. That happened within one week of an earlier project.
   - When the loop will escalate into a web, put its structure under invariants that a validator checks on all content before content grows: a faucet, sinks that lower rates only, a load cap, reachability, an exact pacing check.
3. **Playable throwaway before crates.** The first build of any loop is a throwaway in `scratch/`: hand-authored content, simple rules, one screen. A static mockup only settles layout.
   - **The presentation is part of the formula, and it is about what appears when, not about graphics.** Minimal text incrementals (Universal Paperclips, A Dark Room) hold players for thousands of hours. Start with one or two lines and one verb. Reveal each element only when it becomes relevant, and let its arrival be the reward. Do not explain with labels, legends or how-to text; action names say what they do. A full dashboard on the first screen is the UI form of "declaration instead of discovery". Pictures help sell the place, but they cannot fix a screen that shows everything at once.
   - **The core action is one key or click, always visible.** No modes and no panel switching to reach the main verbs.
4. **The agent plays it first, through the UI.** Self-play bots must drive the game by key presses and read only what the screen shows; a bot that calls game functions directly hides every discoverability failure. A newcomer bot presses only keys the screen names. The lead also watches a real-terminal capture strip of the first 60 s, key by key, as a newcomer, before the designer sees anything. Report these measures:
   - seconds from launch to the first meaningful choice;
   - the longest stretch with nothing to do or see;
   - the time until the next goal the player can see and nearly afford: always under ~30 s, and shown as a bar;
   - reward felt: each payoff is visible, announced and bigger than the last kind; growth in orders of magnitude per 5 minutes;
   - reveals: a new mechanic, panel or tier every 1–3 minutes early on, each shown with its target in advance;
   - load: decisions the game demands per minute. Idle games demand few, and none is mandatory. Count them, and treat a high count as a defect, not a strength;
   - purpose: what the player is building toward, visible from the first minute. If the player cannot name it, the slice fails;
   - a naive player's resources and capabilities at 1, 3 and 5 minutes;
   - whether the first return is worth more than it cost.
5. **Content contract.** If the designer approved a candidate with authored lines, the build must produce lines of that kind from real state. Compare the build's frames with the approved candidate's frames side by side; a gap blocks the work.
6. **Designer plays.** Their report is ground truth. Record it in the iteration log before you interpret it.
7. **Only then expand.** Add a second screen, a save format, a migration, a theme or another app only after a slice is worth playing. Engineering contracts protect work that deserves protecting.

## The agent owns proposing feel

- Propose pacing, rewards, first-minute moments and feel variants as playable options. The designer decides; the agent must still bring proposals.
- Ask experience questions ("which of these two first minutes do you want more of?"), not mechanism questions ("should a failure post a follow-up?"). A mechanism question is allowed only when two played variants differ on exactly that mechanism.
- Tune for the naive player first. Show a naive player's row first in any economy table. A bad first plan should still return something.
- A payoff counts only if it feeds a decision: it unblocks a purchase, moves a bottleneck or opens an option. A number that only rises is not a payoff.

## Red flags in a session

- More than one screen exists, and nobody has played the first one through.
- A smoke test that checks for a string is quoted as evidence that a screen works.
- A headless model marks "interest" or "boredom" as not testable, and the build goes ahead anyway.
- A preview shows certainty (100 %, 0 %) while the outcome can differ.
- The fiction text reads like a changelog.
- The agent has asked the designer five mechanical questions in a row.
- Every purchase is obviously right, and every number only rises. The player is executing, not deciding.
- The player must think constantly, or the screen holds more than one decision at a time. The player is working, not playing.
- No visible long-term goal: "it is all pointless".
- A measure is being pushed up for its own sake (responses, events, unlocks). Measures guard against failure; they do not define fun.

When a red flag appears, stop and go back to the earliest gate that failed.
