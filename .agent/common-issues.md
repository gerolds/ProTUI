# Common issues

Match the report to an entry and apply its fix. Check here before debugging anything new. Keep each entry to
symptom, cause and fix. Name the generic shape of the problem, so the entry still matches when type names change.

## Principles

- A red build that you did not cause is probably another agent's unfinished edit. Check file times first.
- A pipe in a `&&` chain hides failures: `cmd | tail -1 && next` takes the exit status of `tail`.

## Build and run

- **`cargo build` fails with "Access is denied. (os error 5)" on an `.exe`.** Cause: the program still runs from
  that target folder, and Windows locks a running executable. Fix: close it. `play.cmd` builds into its own target
  folder under `%LOCALAPPDATA%\<project>\cargo-play\`, so a running play session does not block ordinary Cargo
  builds; close it before rebuilding through `play.cmd` again.
- **`cargo: command not found` in an agent shell.** Cause: the shell does not inherit the user's PATH entry for
  `%USERPROFILE%\.cargo\bin`. Fix: call `~/.cargo/bin/cargo.exe`, or in cmd run
  `set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"` before `verify.cmd`.
- **A test run takes minutes instead of seconds.** Causes, most common first: another Cargo command holds the same
  target directory ("Blocking waiting for file lock"); a fresh worktree or changed `-p`/`--exclude`/feature flags
  rebuild dependencies; a test renders, mixes or decodes full-size input in a debug build; a test sleeps or waits on
  real time; many `tests/*.rs` files or runnable doc examples each link a binary. Fix: [tests.md](docs/tests.md)
  "Keeping test runs in seconds". Check first: each binary's `finished in` line versus the total wall time; a large
  gap is build or link time, not test time.

## TUI

- **Every key acts twice on Windows.** Cause: crossterm on Windows reports a press and a release for each key, and
  the handler ignores `KeyEventKind`. Fix: act only on `KeyEventKind::Press`, in the one input source
  ([tui-ratatui.md](docs/tui-ratatui.md) "Architecture").
- **The UI is slow to respond, or uses CPU while nothing changes.** Cause: the loop draws on every pass instead of
  on a change. Fix: [tui-ratatui.md](docs/tui-ratatui.md) "Frames". Check first: the `--bench` numbers.
- **A program started from an agent shell, or a capture of it, shows no colours.** Cause: agent shells export
  `NO_COLOR=1` and `TERM=dumb`; crossterm honours `NO_COLOR`, and Windows Terminal hands the caller's environment to
  a tab opened with `wt.exe`. Fix: unset `NO_COLOR` and `TERM` before launching (`tools/wt-capture.ps1` does).
  Check first: `env | grep -E 'NO_COLOR|TERM'`.
- **Octant glyphs show as `�` or boxes.** Cause: the terminal's font lacks Unicode 16 octants (for example JetBrains
  Mono, or the older Cascadia Mono in `C:\Windows\Fonts`). Fix: use quadrant, or a font with octants
  ([rendering.md](docs/rendering.md) "Fonts and glyphs").

## Git

- **A commit silently reverts files another commit just changed, or deletes files it just added.** Cause: an
  earlier commit was made through a temporary index (`GIT_INDEX_FILE`), and the real index still held the old
  content of those paths. Fix: after a temporary-index commit, run `git reset -q -- <paths>` on the real index
  ([delegation.md](docs/delegation.md) "Parallel agents and git"). Check first: `git status --short` shows staged
  `M ` or `D ` on paths you did not change.

## Shell (Windows PowerShell 5.1)

- **Box-drawing and other non-ASCII characters in a file read as "Â·" or "â…" after a PowerShell edit.** Cause:
  `Get-Content -Raw` reads a UTF-8 file without a BOM in the ANSI code page, and `Set-Content -Encoding utf8` writes
  the mangled text back with a BOM. Fix: edit with the edit tool, never a PowerShell read-replace-write; to repair,
  encode the text as Windows-1252, decode the bytes as UTF-8, and strip the BOM. Check first: `grep -P "Ã|â€" <file>`.
- **`cat` of a rendered frame shows garbage.** Cause: the console code page. Fix:
  `[Console]::OutputEncoding = [Text.Encoding]::UTF8; Get-Content -Raw -Encoding UTF8 "<path>"`.
- **A script gets `-ExeArgs a,b` as one string `"a,b"`.** Cause: `powershell -File` passes arrays as text. Fix: use
  `-Command "& script.ps1 -ExeArgs 'a','b'"`.

## Shell (Git Bash on Windows)

- **"unexpected EOF while looking for matching `''" from a heredoc.** Cause: nested quotes in a long inline script.
  Fix: write the script to a scratch file and run the file.
- **A scratch script runs another agent's code.** Cause: parallel agents share one scratch folder. Fix: give scratch
  files a per-agent prefix.
- **`rmdir: Device or resource busy` on an empty folder.** Cause: the shell's working directory is inside it. Fix:
  `cd` out of it in the same call, then retry.
