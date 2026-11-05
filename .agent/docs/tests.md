# Tests

Cover load-bearing flows, contracts and boundaries. Test public APIs. Keep tests deterministic; test behaviour,
not wording, wiring or mock echoes ([AGENTS.md](../../AGENTS.md) "Verify and hand off").

## Where tests live

- Unit tests in a `#[cfg(test)] mod tests` beside the code. They reach that module's private items.
- Contract tests in each crate's `tests/` folder. They see only the public API.

## Fast loop first

- While working, run the tests the change touches: `cargo test -p <crate> <name-filter>`. Run `verify.cmd` (the
  full suite) once, at the end, as the integration owner.
- Headless renders use `TestBackend` and a fake clock ([tui-ratatui.md](tui-ratatui.md) "Tests and headless
  renders"). Audio tests use an offline render, never a device ([audio.md](audio.md)).

## Keeping test runs in seconds

On the sibling project a full test run grew to about 30 minutes and one packet spent most of 1.5 hours waiting on
it. Here, measured on 2026-11-04: a cold `cargo test --workspace --no-run` takes about 80 s, a warm run of all tests
under 2 s, and an edit to one crate rebuilds in 1–2 s. Budgets that keep it there:

| What | Budget | When exceeded |
|---|---|---|
| One unit test | under 100 ms in the dev profile | shrink its input (see below) |
| One crate's tests, warm | a few seconds | find the slow test: libtest prints `finished in` per binary |
| Whole workspace, warm (`verify.cmd`) | under 1 minute | a defect: open a packet to fix it before more features |

Where the time goes, and what keeps it out:

- **Waiting for another build.** Two Cargo commands on one target directory serialise ("Blocking waiting for file
  lock"). One Cargo owner per target directory; parallel agents work in separate worktrees, each with its own
  `target/`. Reuse existing worktrees instead of making new ones: a new target directory pays the cold build again.
- **Rebuilding for changed flags.** Each combination of `-p`/`--exclude`/features/profile builds and caches its own
  artifacts; switching between them rebuilds shared dependencies the first time (a measured 4 s extra here). Use the
  same flags in the loop and in `verify.cmd`. `clippy` and `test` keep separate artifacts; that is expected.
- **Unoptimised dependencies.** The root `Cargo.toml` builds dependencies at `opt-level = 3` in dev and test, so PNG
  decoding, tiny-skia drawing and Kira mixing run at release speed inside tests. Workspace crates stay at
  `opt-level = 0` for fast rebuilds, except the pixel crates `terminal-graphics` and `terminal-tilemap` at
  `opt-level = 2`: a debug frame with a lit place went from 31 to 4.5 ms and their tests from 1.1 to 0.1 s, for a
  rebuild of about 5 s instead of 2 s. Treat another crate with a hot loop a test cannot shrink (a synth, an offline
  mix) the same way, and measure.
- **Big inputs.** Test a renderer on a 4×3 map and an 8×4 cell area, not the sample place at 160×50. Render
  0.1–0.5 s of audio, not 30 s. Decode a 4×4 PNG built in the test. Keep a full-size check as one contract test.
- **Real time.** No `sleep`, no wall clock, no timeouts that wait. Pass the clock in; step it in the test.
- **Devices, terminals, network, fonts.** Never in a default test. A check that needs a font file or an audio device
  is `#[ignore]` with the reason in its name or comment, and runs on request (`cargo test -- --ignored <name>`).
- **Link time on Windows.** Every file in `tests/` is its own binary and link. Keep one contract-test binary per crate
  (`tests/<crate>.rs` with `mod` files) once a crate has more than two. Runnable doc examples compile one binary each;
  write doc examples as `no_run` or as plain text. `cargo test` also builds every example; keep examples small and
  set `test = true` on an example only when it holds a cheap test.
- **Debug info.** The dev profile keeps line tables only, which keeps file and line numbers in panics and halves the
  target directory.

Measure before and after a speed change: `cargo test --workspace --no-run` for build time, then the run alone, and
each binary's `finished in` line. Record the numbers in the ADR banner of the file you changed.

## Reaching private items

A test that needs a private item from another module shows that the seam the test needs does not exist. Try these
in order:

1. Fix the production reason. Make production robust (refuse with a typed reason) and the test no longer needs the
   private item.
2. Use an existing seam that exists because a real collaborator varies at runtime. Never add production API that
   exists only for tests.
3. Widen access to `pub(crate)`, never to `pub`, and never behind a test-only feature.

Prefer fakes of traits that already exist. If a test needs a new trait only to fake something, the design has no
seam: add one, or leave the case to a contract test and say so.

## Trusting a green run

- Check that the test count changed when you added tests. A test that does not show in `cargo test` output was not
  compiled in (a missing `mod` line, a file outside `tests/`).
- A test that fails in the full run and passes alone shows that an earlier test left state behind. Find the leak.
  Do not call it flaky.
- A sweep test (scan all files for a rule) collects every violation and reports them together.
- A hang with green tests is in the integration the tests skip: the loop driver, a blocking wait, an unbounded
  gather.
- A test of something integrated over time (smoothing, anything multiplied by delta time) depends on frame rate.
  Pin the clock.
- Mutation-check new tests where cheap: break the code, see the test fail, restore.

## Shared state

- A test does not write to files, saves or settings that a person also uses. Use a scratch folder per test.
- Destroy everything a test creates.

## Running tests from scripts

- One script (`verify.cmd`) runs the checks every session uses. Never edit it while another session may run it.
- Never put cleanup after `;` where it runs when the step before it failed. In a `&&` chain, a pipe takes the exit
  status of its last command ([common-issues.md](../common-issues.md)).
- On Windows, strip CR from lines a Python step prints to bash (`line=${line%$'\r'}`).
