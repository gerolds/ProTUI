# Pictures and maps in block glyphs

Read this before you draw a picture, a map or an icon with quadrant or octant glyphs, or change how a picture fits a
box. The encoder and its API are in [crates/terminal-graphics/README.md](../../crates/terminal-graphics/README.md);
lit tile-map place views are in [crates/terminal-tilemap/README.md](../../crates/terminal-tilemap/README.md). This
page holds the facts behind them. Numbers below were measured on earlier code on this machine; recheck them before
they decide anything. Readability, renderer defaults, first-run checks and colour use are reference findings from a
sibling project (Runner); design choices belong to `design/`.

## Encoding

- A picture is a raster of linear-light RGB pixels. An encoder samples it in a small grid per cell, splits the
  samples into two colours, and picks the block glyph whose filled pattern matches. The cell's foreground and
  background carry the two colours. Text and markers are written over the cells afterwards.
- Encoders compared:

  | Encoder | Samples per cell | Who draws the glyph | Finding |
  |---|---|---|---|
  | one glyph per cell | 1 | every terminal | no sub-cell detail |
  | half block (`▀`) | 1 × 2 | Windows Terminal itself; elsewhere the font | soft; edges look blurred |
  | quadrant (`▖▞▟`…) | 2 × 2 | Windows Terminal itself; elsewhere the font | close to octant; safe default |
  | sextant | 2 × 3 | font only | little gain over quadrant |
  | octant | 2 × 4 | font only (Unicode 16) | sharpest |

- Ratatui ships the glyph tables: `ratatui::symbols::pixel::{QUADRANTS, OCTANTS}`, indexed by the filled pattern
  with samples numbered row-major, bit 0 top left (`0 1 / 2 3 / …`). Guard that bit order with a test that encodes
  each single filled sample and checks the glyph.

## Two-colour fit

- Average each sub-cell sample area in linear light (box average), so mixed light keeps its energy.
- Convert to display values (tone curve, then sRGB), then split the samples in display space: first at the mid
  luma, then up to 4 rounds of 2-means.
- A cell whose samples form one group, or whose two colours round to the same 8-bit colour, becomes a space with the
  mean as background.

## Linear light and the tone curve

- Pixels are linear light until the encoder converts them. Feeding sRGB values as linear washes the picture out;
  convert sRGB hex or 8-bit values to linear at the entry.
- The tone curve passes values up to 0.6 unchanged and rolls off above with a soft shoulder toward 1, so bright light
  turns white instead of clipping per channel. Linear 1.0 then displays as about 237 of 255.

## Size and aspect

- Image size for a box: 2 × width by 4 × height pixels. Octant shows each pixel; quadrant averages vertical pairs.
- A cell's aspect is its height over its width. A sample pixel is `aspect / 2` times as tall as wide on screen:
  square only at 2.0. Never assume 1:2; the player sets the aspect ([tui-ratatui.md](tui-ratatui.md) "Cell aspect").
- Keep true proportions by "cover": scale the picture to fill the box's shape on screen (cells × aspect), centred,
  and crop what overflows. A box much wider than tall leaves empty columns when the picture is square; size the box
  to the picture.
- A map tile spans `cells_per_tile` cells across and `cells_per_tile / aspect` rows down, so it stays square on
  screen. In the sibling project, below about 2 cells per tile a map no longer showed its shapes, and cropping
  around the point of interest was the remedy used.
- Cache the sampled raster per picture, box and aspect. Every cache keys on the aspect; changing it drops the rest.

## Readability by size (reference)

Measured in the sibling project on painted portraits in octants at aspect 2.57:

- At 9 × 4 cells only a subject's dominant colour and silhouette read. Two subjects with the same colour look alike.
- From 12 × 5 cells outlines such as a hood or a hat read.
- From 16 × 8 cells a face reads.
- A flash over the whole subject read at every size. A desaturation to grey at full strength looked like an empty or
  disabled state; below about 0.6 it did not.
- A feature smaller than one sample vanishes, or shows as speckle when the two-colour fit splits on texture instead.
  There, outline and colour mass told things apart; detail did not.

## Fonts and glyphs

- Windows Terminal 1.24 draws U+2500–U+259F itself (`font.builtinGlyphs`, on by default): box drawing, block
  elements, half blocks and quadrants. It does not draw sextants, octants or braille; those come from the font or
  from DirectWrite's fallback, and a glyph no font supplies draws as `�`.
  ([BuiltinGlyphs.h](https://github.com/microsoft/terminal/blob/v1.24.12741.0/src/renderer/atlas/BuiltinGlyphs.h),
  [fallback](https://github.com/microsoft/terminal/blob/v1.24.12741.0/src/renderer/atlas/AtlasEngine.cpp#L867-L882))
- Cascadia Mono 2407.24 has sextants and octants. The copy in `C:\Windows\Fonts` on this machine (May 2023) lacks
  them; Windows Terminal 1.24 ships its own newer copy in its install folder.
  ([Cascadia 2404 notes](https://devblogs.microsoft.com/commandline/cascadia-code-2404-23/))
- JetBrains Mono 2.304 has no sextants, octants or braille
  ([source](https://github.com/JetBrains/JetBrainsMono/blob/v2.304/sources/JetBrainsMono.glyphs)).
- Windows Terminal answers the cell-size query `CSI 16 t` with a fixed 10 × 20. A program cannot detect the font,
  its octant coverage or its aspect. The sibling project made the renderer (octant, quadrant, ASCII) and the cell
  aspect saved player settings, picked on a first-run check showing the same picture in each renderer and a circle
  and square at the aspect. It defaulted to octant, because the Cascadia Mono that Windows Terminal 1.24 ships draws
  octants; quadrant was the fallback for any font, and ASCII showed the source map. Defaults are a `design/` choice.
- A first-run check can name the terminal from the environment for its advice line: `WT_SESSION` is set in Windows
  Terminal, `TERM_PROGRAM=vscode` in the VS Code terminal.
- A TUI does not select its font. Shipping a font file does not activate it. Monospaced does not mean square cells.
- Emoji and Nerd Font private-use icons are unreliable for information: their width and fallback vary
  ([Unicode UAX #11](https://www.unicode.org/reports/tr11/)).
- Explicit RGB gives an intended palette. Named ANSI colours follow the terminal's scheme. Ratatui's crossterm RGB
  path has no reliable automatic fallback. In the sibling project colour reinforced labels and shapes rather than
  carrying meaning alone, which kept screens readable without colour.

## Raster protocols and effects

- Sixel is a real raster protocol. Windows Terminal draws it since 1.22
  ([release](https://github.com/microsoft/terminal/releases/tag/v1.22.2362.0)); this repository has an encoder and a
  Ratatui seam behind the `terminal-graphics` feature `sixel` (see "Sixel" below). Windows Terminal erases image
  pixels under any text written to those cells, so glyphs cannot be drawn over a sixel image
  ([erasure](https://github.com/microsoft/terminal/blob/v1.24.12741.0/src/buffer/out/textBuffer.cpp#L526-L534)).
- The Kitty graphics protocol (retained placements, animation) is not supported by Windows Terminal; Kitty and
  Ghostty do not run natively on Windows. WezTerm's Kitty support is partial.
- The `ratatui-image` crate is not a dependency; `terminal_graphics::sixel` covers sixel only.
- `tachyonfx` 0.25.2 works with Ratatui 0.30 and is driven by elapsed time. Its effects work on finished cells, so
  light and particles that belong in the picture must be drawn into the raster before encoding. It fits screen
  transitions and panels.
- Do not confuse three things: glyph approximation of pixels (this page), raster protocols (Kitty, Sixel), and
  graphical cell hosts that draw textures in their own window.

## Sixel

Facts from the Windows Terminal 1.24.12741.0 source and from this machine (Windows Terminal 1.24.12741.0, default
profile, real cell about 9 × 19 px). The encoder and seam are in
[crates/terminal-graphics/README.md](../../crates/terminal-graphics/README.md). Sixel is an option, not a decision.

- Support: DA1 (`CSI c`) answers `?61;4;6;7;14;21;22;23;24;28;32;42;52c`; `4` is sixel
  ([source](https://github.com/microsoft/terminal/blob/v1.24.12741.0/src/terminal/adapter/adaptDispatch.cpp#L1464-L1491)).
  DECRQM for mode 2026 (synchronized output) answers `?2026;2$y`: known, off.
- Cell mapping: sixel pixels are virtual. A cell is 10 × 20 of them whatever the font; `CSI 16 t` answers `6;20;10t`
  and `CSI 14 t` the text area times 10 × 20
  ([cell size](https://github.com/microsoft/terminal/blob/v1.24.12741.0/src/terminal/adapter/SixelParser.cpp#L20-L29),
  [reports](https://github.com/microsoft/terminal/blob/v1.24.12741.0/src/terminal/adapter/adaptDispatch.cpp#L3483-L3493)).
  Each row's 10·n × 20 slice is stretched to the real cell with linear filtering
  ([draw](https://github.com/microsoft/terminal/blob/v1.24.12741.0/src/renderer/atlas/BackendD3D.cpp#L1873-L1923)).
  So an image fills exactly its cells, and shows the same aspect distortion as glyphs: square only when the real
  cell aspect is 2.0. Pixel density per cell must divide 10 × 20 to stay whole: 2 × 4, 5 × 10 and 10 × 20 do;
  4 × 8 does not (2.5 virtual pixels each).
- Raster attributes `"Pan;Pad;Ph;Pv` are honoured: the pixel aspect is `ceil(Pan / Pad)`, an integer, so one sixel
  pixel can be several virtual pixels tall; `Ph;Pv` limit the background fill
  ([source](https://github.com/microsoft/terminal/blob/v1.24.12741.0/src/terminal/adapter/SixelParser.cpp#L369-L386)).
  Seen in the window: a 2 × 4-per-cell image with aspect 5 filled its 60 × 20 cells. Width has no such scale; the
  encoder repeats pixels with `!n`.
- Colours: at most 256 per image, given as RGB in 0–100 per channel (101 levels, coarser than 8 bit)
  ([source](https://github.com/microsoft/terminal/blob/v1.24.12741.0/src/terminal/adapter/SixelParser.cpp#L529-L591)).
  Pixels are converted to RGB when the image is stored, so a later palette does not change an earlier image.
- Text over an image: writing any character to a cell erases the image in that cell (seen: cells an image left were
  clean after Ratatui wrote spaces there). Ratatui must therefore skip the image's cells.
- Images belong to buffer rows: they scroll with text and are copied on resize reflow
  ([reflow](https://github.com/microsoft/terminal/blob/v1.24.12741.0/src/buffer/out/textBuffer.cpp#L3121-L3125)). The
  alternate screen has its own buffer: after the probe left it, the main screen showed no image. A window resize
  (100 × 34 → 79 × 40 cells) cleared the screen through Ratatui; the redrawn image was intact; that frame took 189 ms.
- Bottom edge: without DECSDM, a sixel band (6 × aspect virtual rows) that crosses the bottom row scrolls the whole
  screen ([source](https://github.com/microsoft/terminal/blob/v1.24.12741.0/src/terminal/adapter/SixelParser.cpp#L800-L818)).
  DECSDM instead puts every image at the top-left of the page. `SixelLayer` crops an image to whole bands that fit.
- `PrintWindow` (`tools/wt-capture.ps1`) captures sixel content.

Cost, release build, sample place lit with flicker and a moving light, 60 × 20 cell view at 2 cells per tile,
`sixel_probe --bench 120`, mean per frame. Octant bytes are the Ratatui diff of the view alone.

| Output | Palette | Build ms | Encode ms | Bytes per frame | Colours |
|---|---|---|---|---|---|
| octant | — | 3.2 (with encode) | — | 2 900 (worst 7 600) | — |
| sixel 2 × 4 | fixed 6×7×6, dither | 2.8 | 0.18 | 16 500 | 41 |
| sixel 2 × 4 | adaptive 256 | 2.9 | 0.39 | 34 200 | 251 |
| sixel 2 × 4 | adaptive 64 | 2.8 | 0.25 | 20 200 | 64 |
| sixel 5 × 10 | fixed | 3.4 | 0.97 | 60 200 | 47 |
| sixel 5 × 10 | adaptive 256 | 3.4 | 1.45 | 104 900 | 256 |
| sixel 5 × 10 | adaptive 64 | 3.4 | 1.16 | 61 700 | 64 |
| sixel 10 × 20 | fixed | 5.6 | 3.80 | 129 900 | 47 |
| sixel 10 × 20 | adaptive 256 | 5.6 | 4.73 | 191 400 | 256 |
| sixel 10 × 20 | adaptive 64 | 5.8 | 4.34 | 113 500 | 64 |

In Windows Terminal (100 × 34 window, `--log`), measured at the program: frames it completed per second, and the
time to diff and write a frame until the write returned. The terminal may display fewer frames than the program
writes; motion was not watched.

| Output, adaptive 256 | fps at 30 target | Frame ms mean (worst) | Write ms | fps uncapped |
|---|---|---|---|---|
| octant | 30.1 | 4.6 (5.3) | 1.0 | about 250 |
| sixel 2 × 4 | 30.1 | 6.2 (7.8) | 2.5 | about 165 |
| sixel 5 × 10 | 30.0 | 9.8 (10.7) | 4.6 | about 100 |
| sixel 10 × 20 | 30.0 | 17.8 (19.0) | 7.1 | about 55 |

- The fixed palette's dither shows as 5 × 5 virtual-pixel checks at 2 × 4 per cell; at 10 × 20 it is fine-grained
  and costs fewer bytes than adaptive 256. The adaptive palette had no visible banding in the captures.
- At 2 × 4 per cell sixel shows the same samples as octant with no two-colour limit per cell, for 6–12 times the
  bytes of an octant diff of a lit view. Native 10 × 20 is about 190 KB a frame, 5.7 MB a second at 30 fps.
- Use sixel for a picture or view that must show more than two colours per cell or finer detail than octants, in
  Windows Terminal. Use octant or quadrant where text, pins or cursors must sit on the picture, where the terminal
  may lack sixel (Windows Terminal before 1.22; other terminals: check DA1), and as the fallback.
- Pitfalls: a sixel-only screen breaks in a terminal without sixel (check DA1 for `4`); text over the image erases
  it; transparent pixels keep whatever the cell showed before, so an animated transparent image leaves trails;
  an image that crosses the bottom row scrolls the screen.

## Capture tooling and its limits

- A headless capture rasterizes a Ratatui `Buffer` with a TTF font into a PNG: U+2500–U+259F drawn procedurally, as
  Windows Terminal does, every other glyph from the font. Use a font with octants (the Cascadia Mono inside the
  Windows Terminal install folder). A loader should refuse a font without octants rather than draw blanks.
- A headless capture is not proof of the real terminal. Real fonts, hinting, fallback and colour handling differ.
  Check the real window with [tools/wt-capture.ps1](../../tools/wt-capture.ps1) and record terminal version, font,
  size and cell size beside the capture.
- Animated PNGs and large capture sets are generated output: keep them out of git (`/captures/`, `/scratch/`).

## Frame cost reference

Release build, 160 × 50, through a real `CrosstermBackend` that counts bytes, budget 33 ms per frame:

| Frame | Mean ms (max) | Bytes per frame |
|---|---|---|
| Lit map, light flicker only, quadrant | 3.5 (3.9) | about 5 000 |
| Same, octant | 4.2 (4.5) | about 5 500 |
| Close view at rest, quadrant | 2.4 (2.6) | about 17 700 |
| Full-screen flash and particles, octant | 5.0 (12.6) | about 50 000 |
| TUI frame with panels, small pictures and a map | 4.3–5.2 (13.4) | 11 000–19 000 |

- Octant costs about 0.7 ms more than quadrant per frame here.
- About 50 KB per frame is 1.5 MB a second at 30 frames a second. Windows Terminal kept up in a short test. Sixel
  costs are in "Sixel" above.
- A one-time world build (geometry, baked texture, static light) took 0.17–0.55 s in release and over 1 s in debug.
  Build it once per map, behind a screen that covers the cost, never per frame.

## Pitfalls

- Feeding sRGB as linear (washed out), or linear as sRGB (too dark).
- Glyphs drawn with a shaken or eased camera while the raster uses another: glyphs jump.
- A per-frame allocation of the raster or sample buffers: reuse them.
- Rebuilding a cached raster every frame because the cache key misses the aspect or the box.
- Running a live full-screen example from an agent shell: it takes the terminal. Use export or bench modes.
- Captures from an agent shell without colour: unset `NO_COLOR` and `TERM` ([common-issues.md](../common-issues.md)).
