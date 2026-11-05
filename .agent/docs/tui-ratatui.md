# The Ratatui TUI

Read this before you write or review code that draws to the terminal or reads keys. Locked versions and commands are
in [README.md](../../README.md). Pictures in block glyphs: [rendering.md](rendering.md). Engineering rules here (loop,
input, lifecycle, time, tests) are binding. "Focus and cursor", the cell-aspect guide screen and small-terminal
fallbacks are reference findings from a sibling project (Runner); design choices belong to `design/`.

## Requirements

- A full-screen interactive TUI. A scrolling menu or a numbered prompt loop is not a game screen.
- Ratatui is the renderer only, on its crossterm backend (`ratatui::crossterm`). Our code owns the loop, input,
  timing and focus. Do not add a crate that runs the application (an event loop, a component tree, a focus system).

## Architecture

```text
crossterm events → key table → intent → current screen → command → game rules
game rules → view data → screen render(area, buf) → Terminal::draw → Ratatui diff → terminal
```

- The loop owns input, timing and the screen stack. It waits with `event::poll(timeout)` and reads with
  `event::read()`, so it never blocks past the next animation step or timer.
- Keys map to intents in one table. Screens receive intents and never see a crossterm `KeyEvent`. Any screen that
  lists keys reads that table; it does not repeat it.
- Act on key presses only. crossterm on Windows reports a press and a release for every key. A handler that ignores
  `KeyEventKind` acts twice.
- Game rules do not depend on terminal I/O. The screen reads view data and sends commands. It computes no rule; a
  number on screen comes from the rules' own table or query.
- Focus lives in the screen state, keyed by a stable ID, not by list index.

## Frames

- Enter and leave the terminal with `ratatui::init` and `ratatui::restore` (or `ratatui::run`). They switch to the
  alternate screen and raw mode, and install a panic hook that restores the terminal. Restore on success, on error
  and on panic. Print errors after leaving the alternate screen.
- Each `Terminal::draw` call describes the complete frame. Ratatui writes only the changed cells, so an unchanged
  frame writes nothing, but building it still costs time.
  See the [rendering contract](https://ratatui.rs/concepts/rendering/under-the-hood/).
- Draw only when something changed: input, a screen change, a resize, or an animation, at most 30 times a second.
  When nothing changes, the loop draws nothing.
- A view is a widget: `impl Widget for &View` with `render(self, area, buf)`. Built-in widgets (`Paragraph`, `List`,
  `Table`, `Gauge`, `Canvas`) and `Layout` only draw into the buffer.
- Set a frame budget (33 ms at 30 frames a second) and measure it with `--bench` (see "Captures").
- Text never wraps by accident. `Paragraph` wraps only with `.wrap(...)`, and a cell area crops without a sign.
  Crop a line to its width. In the sibling project, ending a cut line with an ellipsis and giving each view a
  small-terminal fallback kept text readable; what a small terminal shows is a `design/` choice. Test at 80×24 and
  160×50.

## Time

- Animation and progress follow clock time, never frame counts. Timers and frames do not arrive at the rate you ask.
- Inject the clock. Tests and headless renders use a fake clock.
- Randomness used only for presentation has its own random source, separate from the rules' random source.
- Not every terminal reports key releases. crossterm reports them in the Windows console, and elsewhere only with
  the Kitty keyboard protocol after `PushKeyboardEnhancementFlags(REPORT_EVENT_TYPES)`. Other terminals report
  presses and auto-repeat only. A rule must not depend on holding a key. Sources:
  [crossterm Windows input](https://github.com/crossterm-rs/crossterm/blob/master/src/event/sys/windows/parse.rs),
  [`PushKeyboardEnhancementFlags`](https://docs.rs/crossterm/latest/crossterm/event/struct.PushKeyboardEnhancementFlags.html).
- Sound waits for the frame that shows its event ([audio.md](audio.md) "Timing").

## UI design

- Show a rendered candidate before you build past a skeleton ([lessons.md](lessons.md)).
- Layout, controls and styles are game design. They live in `design/`, approved by the user, not here.

## Focus and cursor (reference)

Findings from the sibling project's playtests, offered as options for `design/`. There, one shared focus look
across screens read better than per-screen looks.

1. A heavy frame (`┏━┓`) and a bold title marked the panel holding the cursor, used only where a key moves the
   cursor between panels; other panels had thin lines.
2. The cursor's item had one backing colour over all its rows and a pointer glyph (for example `▶`) in its first
   column; one such item per screen avoided ambiguity.
3. A detail of the cursor's item got a small caret rather than a second backing or frame, which competed with the
   selection.
4. When a key acted on the selected item, its cap was drawn on the item (`Enter open`); when refused, the cap was
   left out and the item said what was missing. The key bar repeated the key as a quiet hint; a second lit cap
   confused players.
5. Highlights away from the cursor were used only for new or urgent things, without pointer or selection backing,
   so they were not mistaken for the selection.
6. Pointer and backing both showed focus, so focus still read with a short palette or no colour; colour alone
   failed there.

## Captures

A host gives agents ways to reach a set moment without a person at the keyboard:

- `--keys <seconds>:<key>,...` presses keys through the key table at set real times.
- `--data <dir>` reads and writes saves and settings in a scratch folder instead of the player's folder. Copy a
  sample save into the scratch folder before each run, because a run writes it.
- `--bench` runs headless at a set size and prints frame build time (mean and worst) and bytes written per frame,
  through a real `CrosstermBackend` that counts bytes.
- [tools/wt-capture.ps1](../../tools/wt-capture.ps1) opens a Windows Terminal window at a set size, runs the
  program and saves a PNG of the window. Its header lists the parameters and quirks. Real-time jitter is about a
  second; take a few captures around the moment.
- Never run a live full-screen program from an agent shell: it takes the terminal. Use headless modes or
  `wt-capture.ps1`.

## Cell aspect

- A cell's height over its width differs by font and size: Iosevka Term 11 in Windows Terminal is 7 × 18 px
  (2.57), Cascadia Mono 12 is 9 × 19 (2.11). Windows Terminal answers the cell-size query `CSI 16 t` with a fixed
  10 × 20, so a program cannot measure it. Never assume 1:2.
- Make the aspect a player setting, stored with the settings, range about 1.50 to 3.00 in steps of 0.05 (0.01 with
  Shift). Each step applies at once to every picture and map.
- Option seen in the sibling project: a guide screen where the player fits it — a circle and a square drawn at the
  current aspect, adjusted with `←` `→` until both look right, opened at first start and from a key. Whether and
  when to show it is a `design/` choice. How to draw at an aspect: [rendering.md](rendering.md).

## Terminal facts

- A key that closes an overlay is often pressed twice. Drop keys for about 250 ms after an overlay closes.
- Half blocks (`▀`, `▄`) and quadrants draw in every terminal tested. Octants depend on the font
  ([rendering.md](rendering.md) "Fonts and glyphs").
- A terminal without colour still gets a frame that renders; how it looks is a `design/` choice.
- Agent shells export `NO_COLOR=1` and `TERM=dumb`; crossterm then drops every colour. Unset both before launching a
  program you want to see in colour ([common-issues.md](../common-issues.md)).

## Windows terminals

- Windows PowerShell 5.1 `cat` breaks UTF-8 text such as rendered frames. Use
  `[Console]::OutputEncoding = [Text.Encoding]::UTF8; Get-Content -Raw -Encoding UTF8 "<path>"`.
- Never edit a source file with a PowerShell read-replace-write: it mangles box-drawing characters.
- A real terminal may delay Esc while it waits for an escape sequence. Check Esc latency in Windows Terminal.
- Windows locks a running executable; `play.cmd` builds into its own target folder ([README.md](../../README.md)).

## Tests and headless renders

- Render with `ratatui::backend::TestBackend` at a set width and height, then compare
  `terminal.backend().buffer()` with `Buffer::with_lines(...)`. A single widget renders into `Buffer::empty(area)`.
- In Ratatui 0.30.2, `TestBackend` returns `Infallible` errors, unlike the real backend's I/O errors. Remove that
  impossible error explicitly; `?` does not convert it into `io::Error`.
- The loop's single pass is the headless seam: scripted intents and a fake clock drive it without a terminal.
- Give the game a render command that writes frames to files (text and PNG). Use it to show the user a view and to
  review your own work. A headless buffer proves layout and input logic only; fonts, colours, key delivery and
  cleanup need a real terminal.
