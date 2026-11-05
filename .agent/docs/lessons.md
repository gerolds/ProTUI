# Lessons from agent work

Rules for working with the user, design work and verification. The working agreement is [AGENTS.md](../../AGENTS.md);
packet rules are in [delegation.md](delegation.md). Lines about how UI or tools should look or feel are reference
findings from a sibling project (Runner); design choices belong to `design/`.

## Working with the user

- **Ask only what no source settles.** Before asking, check `design/`, the checkpoint, the code and earlier
  discussion. Write the derived draft, then ask only the open intent. Each question says what was searched.
- **Do not re-ask a settled decision.** Check `design/` before any design question.
- **Show a picture before building UI.** A rendered candidate (headless PNG or a real terminal capture), and a yes
  from the user, before building past a skeleton. ASCII wireframes do not work for most people. An existing UI the
  user likes is the reference; only its named defects change. Ask what the view is for first.
- **Reference: UI inside the platform's idioms read best.** In the sibling project, invented visual languages
  (glyphs, letter badges, dense metadata) confused the user; deviations held up only where a default fought what the
  thing is.
- **For a creative tool, ask what the user does with their hands.** A tool built for agents to write files was
  wrong when the user wanted to work inside the app by play.
- **Two competing drafts let the user choose.** For a question of voice, look or feel, give two agents the same
  brief and show both. The user judges by reading or looking faster than by answering questions.
- **Ask experience questions, not mechanism questions.** Each answer to a mechanism question became a rule, and each
  rule became text on screen; the build passed every check, and the user bounced off it in minutes. Offer two playable
  variants and ask which one to have more of ([playable-first.md](playable-first.md)).
- **The user works in the same tree.** Check `git status` before any step that saves, regenerates or runs a long
  suite. A long tool-blocking run while the user works looks like a crash.

## Design

- **Build toward the final shape; do not cut to "what consumers use today".** Resolve open shape by asking, not by
  shrinking scope.
- **Nothing is fixed before load proves it.** While a part's shape is still being found, its tooling stays plain.
- **When one primitive turns out to be inferred, audit all of them.** Re-plan before the next packet.
- **Before adding an abstraction seam, check whether an existing public function already is the seam.**
- **Check a rule against its first diff before fanning it out.** Start rules from the convention, not from the
  current code.
- **Check the direction of a formula and the order of filters** before you write a ruling.
- **Probe the reasoning you are about to lock in.** You cannot grade reasoning you produced. Hand load-bearing
  decisions to a reviewer without your context (`seam-probe`, `scope-probe`, `reason-probe`, `readback-probe`).
- **Keep visual and audio exploration in a separate scratch folder** (`scratch/<topic>/`): source, captures and
  output together, uncommitted. Do not integrate sample code into the game before the user selects a direction.

## Verification

- **Done means load-bearing behaviour under a real producer.** Compiling and passing unit tests make a part wired,
  not done.
- **Read the tool's output first** before theorising about a stuck tool.
- **Never report a tool's failure only in your own words.** Show the command as a person would type it, the last
  lines it printed, and where the full output lives.
- **Check that a success is fresh.** Compare a build output's time with its sources before accepting "compiled".
- **Reports drift from artifacts.** Check commit file lists against the brief. Review the code, not the report.
- **Attribute every spike.** Set a budget per frame and treat a breach as a defect.
- **Run a cold reader on every how-to page.** A reader without your context (`readback-probe`, `plan-reader`) found
  nine real gaps in one pass on one page.

## Packets

- **Split packets that change a shared contract.** A packet that changes a contract and every screen that uses it
  runs out of budget before the second half compiles. Contract first, users second.
- **Settled design in the packet makes fast agents.** A scout inventory first, then a packet with the design
  decided, gave complete results in under 20 minutes.

## Tooling hazards

- **Post-edit hooks can revert edits.** A formatter hook can write a stale buffer over the file just changed while
  the edit tool reports success. Check `git diff --stat` after each step.
- **An editor can save stale buffers over newer files** when it regains focus.
- **Shared scratch files collide.** Name scratch files per packet.

## Recording what you learn

- `.agent/common-issues.md`: symptom, cause, fix. Checked before any new debugging. Consolidated, never a history.
- When the same correction appears twice, promote it into [AGENTS.md](../../AGENTS.md) (lead) or the guide the
  index points at.
