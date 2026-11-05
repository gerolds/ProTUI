# Iteration log

Agent proposals here are candidates until the user's playtest decides. Rules, numbers and fiction in the throwaway are agent drafts, not approved design.

## Hook (candidate)

Each walnut-shelled asteroid holds something at its kernel; you dive in, decide how deep to push before the air runs out, and what you drag back opens the next, harder shell. The Drupe, a class-5 shell that pings from inside, is visible from the first screen and is the long goal.

## Formula mapping (candidate)

| Job | Proven games | This fantasy |
|---|---|---|
| Push-your-luck core | Incan Gold (stay or leave per card), Deep Rock Galactic (call the pod), Dredge (night falls) | Air: dig deeper or turn back; the way back costs more with more haul |
| One number that grows | Motherload (cash per trip), Dredge (money) | Credits per dive; each shell class ×3 value |
| Manual verb | Motherload drill, Dredge fishing | Drop; answer the card at each find; turn back |
| Purchase that eases the verb | Motherload tank and drill, Dome Keeper drill and carry | Air tank, drill heads, grapple coils |
| Purchase that adds an option | Subnautica blueprints, FTL systems unlock event options | Cutter, tether, substrate scanner add answers to find cards and steering cards |
| Reveal by permission | A Dark Room, Subnautica fragments | Tools come from finds (a broken cutter in a wreck); tender panels appear when first relevant |
| Geometric prices, ×N thresholds | Motherload, Cookie Clicker | Upgrade tiers ×2–2.5; pod shell class gates asteroids |
| Visible next goal | Dredge hull upgrades | Workbench bar to the next item; chart shows the shell class each rock needs |
| Phase change | Dredge relic quest, Paperclips stages | Salvage for credits → relics from kernels point to new rocks → crack the Drupe |
| Mystery | Dredge, Outer Wilds | Why shells grow around wrecks; what pings inside the Drupe |
| Low load | idle games | One find card per notable layer; steering forks are optional and default to "hold course" |

## Experiment 1: the first five minutes

- Question: does a push-your-luck descent, with tools that add answers to find cards, make a first-time player want "one more drop" within five minutes?
- Hypothesis: the interesting decision is when to turn back, given what is carried and what the scanner shows below; tools are worth buying because they change that decision, not because a number rises.
- Playable: `scratch\meta-loop\play.cmd` (Windows Terminal, at least 110×34). Headless self-play: `--bot naive|bold --seed N` writes `out/<bot>-<seed>/report.txt` and `frames.txt`.
- What the slice holds (agent drafts): tender with chart, workbench and relic shelf, each shown only once relevant. Dive: a shaft of layers above a kernel; air drains while digging; a yellow reserve on the air bar shows the way back, which grows with depth and carried mass. Find cards pause the air (wreck: pry on a die, cut, cut the core, leave; hollow: ride the draught or seal; fronds: push on a die, burn, turn back). Steering forks run beside the dig for 6 s and default to holding course. Fronds you push past can regrow and block the way back. Tools come from finds (a cracked cutter in the first wreck) or from relics (scanner, pod shell-breaker). Gall appears after dive 1; the Husk 7 kernel relic reveals the Lantern (class 2).
- Agent self-play, 12 runs of 5 minutes (naive and bold bots, 6 seeds, screen-reading only):

  | Measure | Naive | Bold |
  |---|---|---|
  | First choice | 7–15 s | 7–15 s |
  | Longest stretch with nothing new | 13–28 s | 16–29 s |
  | Dives in 5 min | 3–4 | 3 |
  | Dive 1 earnings | 14–60 cr | 8–79 cr |
  | Best dive by minute 5 | 223–373 cr | 175–426 cr |
  | Husk 7 relic on dive 1 | 3 of 6 | 4 of 6 |
  | Blackouts | 1 of 6 | 1 of 6 |
  | Pod class 2 bought | 0 of 6 | 0 of 6 |

- Gate failures the agent sees: the next purchase is one dive away (60–120 s), not under 30 s; the haul panel's "toward" bar is the stand-in. The long quiet stretches are returns with heavy hauls. Nobody reaches the Lantern inside 5 minutes; growth is one order of magnitude (about 20 → 300 cr per dive).
- User playtest: (pending)
- Decision: (pending)
- Next test candidates: shorter dives so the tender loop turns faster; a visible two-way branch in the shaft instead of the timed fork; events on the way back beyond fronds.
