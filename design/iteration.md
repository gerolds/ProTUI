# Iteration log

Agent proposals here are candidates until the user's playtest decides. Rules, numbers and fiction in the throwaway are agent drafts, not approved design. User direction: `design/treatment.md`, `design/steering.md`. World: `../Support/CLAIM7.wiki/CLAIM7/Alternative-Reality*`.

## Hook

Each walnut-shelled asteroid holds a named thing at its kernel; you work the claim over several descents, decide how deep to push before the air runs out, and what you deliver earns the gear that opens the next, harder shell. The Drupe, a class-5 shell with a signal inside, is in sight from the first screen. (The user: "generally right"; shells open through gear, and an underequipped worker survives inside only briefly.)

## Formula mapping (candidate, after the wiki)

| Job | Proven games | This fantasy |
|---|---|---|
| Push-your-luck core | Incan Gold, Deep Rock Galactic, Dredge | Air, seals and carried load against the way back |
| Mission base between sorties | MGS5 Mother Base, Dredge's port | The tender: supplies, refinery, ledger, contracts |
| Pot per mission | MGS5 deployment cost, the wiki's Demand Engine | The claim purse: advance plus sales, spent on supplies, taken by the house at close |
| Extraction mid-mission | MGS5 Fulton | Flares lift a find to the tender; it is safe from a blackout |
| Prepare or commit | the wiki's "bring it now or prepare more" | Foam, lines and cleared mats persist in the shaft between descents |
| Progression = access | Subnautica depth gates; the wiki's fixed rule | One gear item from the house per delivered thing: pod refit (class 2), cutter, scanner, heat liner |
| A new kind of job, not a harder one | the wiki's badge tiers | Husk 9 is a recovery (a dead crew's index); Gall is hot inside |
| Reading | the wiki's oracle organ | Signs before each risky feature are true 3 times in 4; the scanner's reading is always true |
| Visible long goal | Dredge, Outer Wilds | The Drupe on the contract board, out of reach |

## Experiment 1: push-your-luck dives with pausing cards (closed)

- Built: one dive per trip, cards paused the clock, upgrades to air and drill speed, all text written inline with the code.
- User feedback (2026-11-05, from bot frames and captures, not play): the pausing cards felt like a text adventure. The writing announced what the player should infer. The UI used key shortcuts instead of focus and select. The setting read as campy sci-fi instead of the wiki's world. The full words are in `design/steering.md`.
- Decision: replace it with experiment 2. Speed and air upgrades are dropped because the wiki rules out stat growth ("progression = access").

## Experiment 2: a claim worked over self-running descents

- Question: when the descent runs by itself and the player steers through short timed windows, are the steering choices and the preparation between descents interesting enough to carry three claims?
- Hypothesis: the interesting choices are (1) reading a sign and choosing between the quick option and the sure one, (2) when to climb, and (3) what to buy or refine before the next descent, given what is still in the shaft.
- Playable: `scratch\meta-loop\play.cmd --voice a` or `--voice b`, in Windows Terminal at least 110×34. Keys: ↑↓ and Enter. Two competing text drafts with the same mechanics: A names the house Ardane and the neighbour Ida Krall; B names them Brannock and Wenna Coyle.
- What the slice holds:
  - **Tender:** a focus list with a context pane for the focused entry.
  - **Claims:** Husk 7 (a lineage plate), then Husk 9 (a dead crew's index; their old shaft and lines are already there) or Gall (class 2, hot inside). The Drupe is shown but cannot be breached.
  - **Descent:** digs by itself. Each feature opens a window a few seconds before the worker reaches it. The focused option is carried out on arrival, and rolls resolve on their own. On the way down, Climb is in every window. Mats grow back; loose rubble can shift on the way up; unlined draughts come back.
  - **Delivery:** the house takes the purse and strikes a badge, and the player picks one gear item from the stores. A blackout or a cracked kernel goes into the ledger as an error mark.
- Agent self-play: 18 runs of 8 minutes (naive, careful and bold bots × 6 seeds). The bots read only the screen and press only ↑↓ and Enter.

  | Measure | Naive (defaults) | Careful | Bold |
  |---|---|---|---|
  | First window | 5.7 s | 5.7 s | 5.7 s |
  | Longest stretch with nothing new | 15–21 s | 14–22 s | 14–30 s |
  | Husk 7 delivered | 85–99 s | 88–97 s | 55–94 s |
  | Three claims delivered | 360–413 s | 338–427 s | 248–327 s |
  | Blackouts in 8 min | 0–2 | 0–1 | 1–2 |
  | Windows per minute | 4.4–5.0 | 3.5–4.8 | 4.6–5.9 |

- What the agent sees:
  - The three play styles finish within a minute of each other, so the choices may not matter enough. This is a red flag to check in play.
  - The purse matters little: air flasks are the only purchase that changes a descent much.
  - Husk 7 opens the same way on every seed (pump housing, ice, a sealed void), so the first window always comes at 5.7 s.
- User playtest: (pending)
- Decision: (pending)
