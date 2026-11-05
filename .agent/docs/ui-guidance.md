# UI guidance: findings from the Runner project

Reference material from another project, not a requirement. This game's design decisions live in `design/`; where a
finding here conflicts with `design/`, `design/` is the authority.

The sibling project Runner (`Projects/Runner/Runner`) is a real-time Ratatui game in which the player mostly watches
and sometimes chooses. It went through four playtests by the user on 2026-11-03 and 2026-11-04. This page reports
what it tried for drawing affordances and guiding the player, what the playtests and measurements showed, and what
was kept or reversed. Its game content is left out. The focus and cursor rule it settled on is already in
[tui-ratatui.md](tui-ratatui.md) "Focus and cursor"; this page does not repeat it.

## 1. What the playtests showed

- **Context on every screen.** Playtest 1: "the first screen cannot be read; each view needs a header that says its
  mode and purpose." Runner added a header to every view: a mode chip, the view's name, one purpose line and a
  breadcrumb of where the view sits in the loop. Playtest 2: "headers are good", and asked for a flavour banner so
  the player feels in a place. Playtest 4 still found the main screen "a dashboard, not a place": headers fixed
  orientation but not identity.
- **What matters now, at the place it concerns.** Playtest 1: choices were not announced, news shown at the screen's
  edge was missed, and the automation's next step was never shown ahead. The clearest element was the damage number
  drawn at the figure it hit. Runner moved every prompt to the place it concerns and kept the log for reading later.
- **The next decision and its reason.** Playtest 4: "in the first ten minutes the player makes no decision whose
  reason they understand." Layout and telling could not fix that; it went back to game design.
- **Show what automation does next, in the quantity's own units.** Runner showed the next step and each automatic
  threshold as "acts if X drops below 12 Health · has 20", not as a percentage, and a marker on the bar at the line.
  Sources it cites: Slay the Spire players forgot an intent shown after a click; an always-visible icon fixed it.
  Melvor Idle players built their own tools to check what the automation would do before leaving it running.
- **Hierarchy.** The first-screen design lit only the main action's key cap and kept the other keys quiet. One
  standing decision stayed on screen at every size. Cited sources: Riot's rule that attention matches importance;
  Into the Breach cut features it could not show clearly after the screen became "an icon mess".
- **Cursor versus selection.** Playtest 3: the picker's cursor sat on a part of an option while the player was
  choosing whole options, and that confused. The fix moved the cursor to the thing being chosen and marked the
  looked-at detail with a small caret. Runner's first-run page wrote nothing until its last Enter.
- **Visible reasons for unavailable actions.** A locked option showed what opens it, read from the rule that opens
  it. A refused action dropped its key cap and wrote what is missing on the item. Playtest 2 added a pacing finding:
  the player should meet a limitation before being offered its fix, and an upgrade should visibly fill a slot.

## 2. Techniques, with specifics

**Key hints from one table.** Every on-screen key hint read its cap from the one key table (`keys::cap`), and
screens received intents, never raw key events. A renamed key changed every hint at once. When a key bar ran short,
hints were dropped in a set order. One helper gave each option its key, and the handler, the option boxes and the
prompt text all read it, so an option's key was the same everywhere.

**Hint placement.** Runner tried a decision band across the top of the main view, then moved each prompt to the
thing it concerns: the option's key in a bordered box at the option, the question and its keys at the actor. The
user added a rule after playtest 1: every chance to decide is announced so nobody can miss it, with the key in a
bordered box and a pulse. The key bar at the bottom kept a quiet copy. Playtest 2 found the same call to action
shown twice and asked for one place; the boxes were then placed once per prompt and held still, because moving
callouts could not be read. A target outside the view got its box at the view's edge, toward it.

**Keys by place versus numbered options.** Options were first numbered `1`-`9`. Playtest 3: "numbers need real
thought and cannot be used by feel." Runner switched to arrow keys by side (left option, right option, a route by its
compass side) and stopped numbering options on screen; digits kept one meaning (a per-character action).

**Many shortcuts versus one menu with focus.** Runner's hub screen grew letter and digit shortcuts. Playtest 4: it
should be "a menu with a focus, not a buffet of shortcuts"; one action was "almost impossible to discover", another
"cannot be undone and is not explained". The settings decision moved three shortcuts off the key bars onto the
settings page and kept three as optional accelerators that also change the saved setting and show a notice.

**Focus that survives colour loss.** A pointer glyph plus a backing, each enough alone; see
[tui-ratatui.md](tui-ratatui.md). Runner first gave the panel holding the cursor a heavy frame (playtest 2 asked for
a more obvious selection), then limited heavy frames to screens where a key moves the cursor between panels
(playtest 3: "frame highlights make no sense where no cursor moves between frames"). Where no cursor is, a highlight
only draws the eye to something new or urgent. A pulse (a cell group brightening twice) pointed at a fact and never
carried one alone.

**Telling elements and the placement rule.** Runner's telling design (approved after playtest 1) used six elements:

| Element | Where | How long | Carries |
|---|---|---|---|
| Floating text | At the figure, rising, on a solid backing | numbers 0.75 s; up to three words 1.5 s | what just happened to one figure |
| Callout | Beside the thing it names, raised backing | while its fact is true | lasting facts about a place |
| Prompt | At the actor, key boxes, a draining bar | the decision window (3 s real time, game slowed to 25 %) | the question, its keys, what happens if the player does nothing |
| Banner | One row across the main view | 2.5 s of game time (situation banners 1.5 s), then the log | a change to the whole run |
| Beat | A framed box over the view | 2.5 s | one character's fate |
| Notice (toast) | The bottom bar's notice slot | 3 s | host messages: bought, saved, content reloaded, a refused key |

The rule asks two questions of each piece of news: does it need the player now, and does it have a place on screen?
Needs the player and has a place: a prompt there. Over at once: floating text. Stays true: a callout. Changes the
whole run: a banner, then the mode chip holds it. Not a game event: a notice. Nothing the player must act on appeared
only in the log or a notice. Priority when two compete: prompt, beat, banner, callout, floating text; the lower one
waits, shrinks to its key box or goes to the log, and never covers the higher one. A banner named only situations the
rules report: "a banner must not name something the game does not do". Cited: Cogmind's log is ignored "for 99% of
play time" and the map announces; Dwarf Fortress places announcements by weight.

**Pacing.** Banners waited while a prompt was open. The frame ran one simulation tick behind, so every event was
keyed before a frame showed it, and a sound waited for the frame that showed its event. Playtest 1 found pauses
jarring; a headless run that lists every stand over 0.3 s found a one-second stop with nothing on screen, which then
got floating text for what happened in it. Playtest 2 asked for a short "get ready, set, go" start; start and
summary beats were skippable with any key. The settings page paused the running game.

**Decision windows.** Playtest 2: the facts beside a prompt could not be read in 3 s. Runner showed those facts from
the moment they were first visible and let the player set the window (short, 2.5×, 5×). Playtest 3: still too
stressful "even at a twentieth of the speed", so the decisions needed a structural change: choices answered ahead
in a queue, built as a switchable experiment beside the timed prompts. The sources do not record the outcome.

**Progress bars.** A draining bar with words on the prompt ("acts in 2.5 s"); a bar with a tick at the next
purchase and whether the current haul pays for it; a marker on a meter at an automatic threshold with its value in
words. The design never showed a chance or odds, and labels stayed beside bars ("Health", not "HP"; playtest 2).

**Panels and density.** Runner drew three layouts with the same content and measured them before choosing; the one
that gave the main view most room and put the facts for the standing decision in one bar won. The user approved two
layouts behind a key for one playtest, with the loser to be deleted after. Playtest 2 asked for people as cards
with pictures rather than rows in a list, and for purchasable things as two-row items or cards rather than
resources. A key cap never covered a value. Frame cost of the chrome at 160×50: 4.3–5.2 ms mean, 13.4 ms worst,
against a 33 ms budget.

**Small terminals.** Sizes laid out: 80×24 (smallest), 120×40 (the target) and 160×50. At 80×24 every layout kept
the main view first, folded each person to one line and kept the standing action on screen. Below 110×34 the prompt
took two rows across the top of the main view. Richer panels with pictures appeared only above a size step
(186×34), and the tests covered both sides of the step. A list with no room said how many rows it hid. Text was
cropped with an ellipsis, never wrapped by accident.

**First-run checks.** Playtest 4 asked the first run to say this is a terminal game, which font each renderer needs,
and what to expect of aspect, window size, fidelity and frame rate. Runner built one page with a list of three
checks: (1) the window, measured each frame, with a table of sizes and the player's own marked, red below 80×24; the
terminal named from `WT_SESSION` or `TERM_PROGRAM=vscode`, used only for the advice text; (2) the renderer: the same
picture in octant, quadrant and ASCII side by side, octant preselected, picked by eye because no terminal reports
which glyphs its font has; (3) the cell shape: a circle and a square adjusted with `←` `→` (Shift for 0.01). With the
defaults three Enters start the game; nothing is written until the last Enter; it opens again after an older settings
file. The accepted cost: each page before the first play delays the hook.

**Settings page.** One page, one key from every screen. `↑` `↓` choose a row, `←` `→` change it, Enter opens a
check, Esc goes back. Every change applied at once, was written and was shown as a notice. At 120×40 a right column
explained the focused row with a preview; at 80×24 the text sat under the list without a preview. Opened over a
running game, the header said "settings, paused" and Esc's hint named where it returns.

**Version and frame-rate corner.** At the right end of the key bar: the version and short commit hash from 100
columns up, and the frames drawn in the last whole second (`–` when none, amber under 20). A setting hides the rate.
Each line of the run log starts with the version, so a log line names its build. A change of the shown value marks the
frame dirty, so the corner never shows an old rate.

**Terminal details.** A key that closes an overlay is often pressed twice; Runner dropped keys for about 250 ms
after an overlay closed. A page that took 0.4–0.55 s to appear was "slow" and found "only by accident"
(playtest 4); it was made to open in the next frame (10–15 ms) and named in the key bar and its heading.

## 3. What failed or was reversed

| Symptom | Cause | What changed |
|---|---|---|
| First screen unreadable | No statement of mode or purpose | Header on every view; a "start here" primer until the first run ends |
| Choices missed, news missed | Prompts in a band and news at the screen's edge | Prompts at the place; placement rule; log only records |
| Same call to action shown twice | Prompt in the bar and at the place | One place per prompt; the bar keeps a quiet hint |
| Callouts unreadable | They moved with the view | Placed once per prompt, held still |
| Numbered options used with effort | Numbers carry no meaning by feel | Arrow keys by side; no numbers on screen |
| Features hard to discover | Many letter shortcuts on one screen | Menu with focus; three shortcuts moved to the settings page |
| Heavy frame meant nothing | Frame drawn where no cursor moves between panels | Heavy frame only where a key moves between panels |
| "HP" and "SAN" flagged in playtest 2 | Abbreviated labels | Full words ("Health") on every screen |
| Close zoom implied details matter | Camera framing at up to 5–8 cells per tile | At most 4 cells per tile; 3 tiles of surroundings |
| Explored area unreadable, exit hidden | Lighting decoration drew already-seen ground nearly black | Dim grey for explored ground; an exit overlay (asked again in playtest 2) |
| A named thing could not be found | The word named a place the UI did not show | The word left the game's text |
| Eye-catching strip explained nothing | It could not be inspected | Inspectable slots |
| Main screen read as a "soulless dashboard" | No place or people to go to (playtest 4, notes 7 and 9) | Redesign proposed: a walked place with a focus menu and portraits |
| Primer and threshold words too long | Seen in real-terminal captures | Shorter primer; the purpose line kept (one row of 40) |
| Player text read flat | Documentation plainness rules applied to game lines | Lines given a voice and a speaker (playtest 3: "captions break immersion") |
| Pauses felt jarring | A timed stop with nothing on screen | Floating text shows what the stop is |

The first-start guide (one cell-shape screen) was replaced by the three-check first-run page within a day, once the
renderer became a setting. The reversals in this table came from playtest notes and real-terminal captures.

## 4. Testing guidance cheaply

- **Headless frames.** `TestBackend` at 80×24 and 160×50, compared with `Buffer::with_lines`; a render command
  writes frames as text and PNG. This proves layout, cropping and input handling only.
- **Mockups in the game's own drawing.** Runner's telling and first-run mockups drew the real screens with the game's
  kit over real state, added the proposed elements with a scratch program, and rasterized them in the font of a real
  terminal (Cascadia Mono, Windows Terminal 1.24) at 120×40 and 80×24. The user approved from those pictures.
- **Real-terminal captures.** `tools/wt-capture.ps1` with `--keys <time>:<key>` reaches a set moment; real-time
  jitter is about a second, so several captures around the moment. Saved sample states reach a deep moment quickly.
  Captures found key caps over values, crowded labels at 80 columns and a label pile-up.
- **Measure the stand-stills.** A headless run that lists every stretch where nothing moved over 0.3 s, with the
  events in it, showed that no pause came from drawing; each had a rule-timed cause.
- **Cold readers.** A reader without context found nine real gaps in one how-to page in one pass. [INFERENCE] The
  same check on a frame's text ("what is this screen for, what can I do next, why is this unavailable?") tests the
  first principle cheaply; Runner did not record doing this.
- **Competing variants.** Two drafts of one brief, or two layouts behind one key, let the user choose by reading or
  playing.
- **What only a playtest shows.** Whether the reason for a decision is understood; whether text can be read in the
  time given; whether the screen feels like a place; whether the loop pulls. Runner's records list each note
  verbatim in substance with where it is handled, which made reversals traceable. Tests, headless frames and
  scripted smoke do not rebut a playtest.

## 5. Relation to this repository

The host prototype (`crates/prototype`) already covers several findings:

| Finding | In the host |
|---|---|
| One key table, hints from it | `keys.rs`: `KEYS`, `keys::intent` (press only), `keys::cap`; `ui.rs` hint lists per focus group read their caps from it |
| Generic intents | `Up` `Down` `Left` `Right` `FineLeft` `FineRight` (Shift+arrows) `NextFocus` `PrevFocus` (Tab, Shift+Tab) `Activate` (Enter) `Back` (Esc, never quits) `Quit` (Ctrl+C); unnamed keys send `Other` ("any key") |
| Short key bar | Hints draw in list order while they fit, then `…`; a first start shows no Esc hint |
| Focus without colour | `▶` and a reversed backing |
| Cursor versus commitment | The setup screen edits a draft; only Save writes; a changed value shows the saved one beside it |
| First-run checks | No settings file opens setup with no Cancel; it shows a box that is square at the right cell aspect and samples of each renderer |
| Notice | `NOTICE` 3 s with a draining bar, info or error, on the row above the hints |
| Small terminals | Text cropped with `…`; `--smoke` draws at 80×24, 160×50 and 1×1 |
| Captures | `--keys <ms>:<key>`, `--bench <frames>` at 160×50, `tools/wt-capture.ps1` |

Not in the host: a header with a purpose line, any telling element beyond the notice, progress bars, a window-size
check, and a settings key reachable from every screen.

## Sources

Runner, read 2026-11-04: `.agent/docs/tui-ratatui.md`, `.agent/docs/lessons.md`, `.agent/experience.md`;
`plans/discovery/` `tui-chrome.md`, `telling.md`, `first-run.md`, `presentation.md`,
`prototype-presentation-kit.md`, `terminal-game-presentation.md`; the four records in `plans/playtest/`; the UI
decision records in `plans/design/decisions/`; commit messages on its master branch.
