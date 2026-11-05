# Terminal graphics (`crates/terminal-graphics`)

Fits linear-light RGB pictures into Ratatui cells: one block glyph and two
colours per cell. It does not own the application loop, terminal setup or game
state.

- Draw a picture: build a `Raster` (borrowed) or `StillImage` (owned) and call
  `encode_image(raster, encoder, area, buffer)`. It stretches the raster over
  `area`. Give it `2 × width` by `4 × height` samples for one sample per octant.
- Load a picture: `StillImage::from_png(reader, background)` (alpha is
  composited over `background`) or `StillImage::from_ascii(text, palette)`.
- Draw a PNG into a panel every frame: keep one `ImageCache` per panel and call
  `draw(image, tint, encoder, area, buffer)`. It centre-crops to the panel's
  shape on screen and samples once per size and aspect. A new `tint` (an `Rgb`
  multiplier) does not resample.
- Paint shapes: `Sketch::new(columns, rows, cell_aspect, ground)`, then `oval`,
  `rect`, `polygon`, `line`, `stroke_oval`, `stroke_polygon` with an `Ink`, then
  `finish()` → `StillImage`. Design units keep their proportions on screen.
  `view` sets the design rect ("cover": centred, overflow cropped).
- First-run aspect guide: `calibration(columns, rows, cell_aspect, ground, ink)`
  draws a circle and a square. They look true only when `cell_aspect` matches
  the font's cell height over width.
- Textures and flicker: `noise` (`hash`, `unit`, `value`, `fbm`, `wave`, `Rng`).
- Headless captures (feature `capture`): `capture::CellFont::load(font)`,
  `rasterize(&buffer)` → `Picture` → `save_png`; `contact_sheet`;
  `Animation::create(path, first, frames, fps)` / `push` / `finish` for APNG.
- Sixel images (feature `sixel`), an escape hatch beside the glyph encoders:
  `sixel::SixelEncoder::encode(raster, &SixelOptions { palette, scale }, &mut out)`
  appends one image to a reused `Vec<u8>`; `encode_masked` leaves pixels with a
  false mask entry transparent. `Palette::Fixed` (6×7×6 cube, ordered dither)
  or `Palette::Adaptive { colors }` (median cut, 2..=256). `scale` repeats
  pixels across and sets the pixel aspect down. In a Ratatui app keep one
  `SixelLayer::new(WT_CELL_PX)` and draw with `layer.draw(&mut terminal,
  |frame, layer| …)`; inside, `layer.place(frame.buffer_mut(), id, revision,
  rect, raster, palette)` marks the rect's cells skipped and queues the image.
  The raster must divide the rect's device pixels (cells × 10 × 20 in Windows
  Terminal) into whole pixels: 2 × 4, 5 × 10 or 10 × 20 per cell. The image is
  re-sent only when id, revision or rect change, after a resize, or after
  `layer.invalidate()` (call it after `Terminal::clear`). Cells an image leaves
  are redrawn by Ratatui. `begin` / `place` / `end` / `pending` serve other
  draw loops. Facts and costs: `.agent/docs/rendering.md` "Sixel".

## Surprises

- Pixels are linear light. Convert sRGB with `Rgb::hex` or `Rgb::from_srgb8`.
  Values above 0.6 roll off softly; linear 1.0 shows as about 237 of 255.
- `Encoder` has no default. Let the player choose: Windows Terminal draws
  quadrants itself in any font; octants need a font that has them. The program
  cannot detect the font.
- Never assume a 1:2 cell. Cascadia Mono at 18 px is 9 × 18 px (2.0); Iosevka
  Term is 7 × 18 px (2.57). Get the value from the player (the calibration
  drawing) or, in captures, from `CellFont::cell_aspect`.
- `C:\Windows\Fonts\CascadiaMono.ttf` lacks octants (`CellFont::load` refuses
  it: U+1CEA8). The `CascadiaMono.ttf` in the Windows Terminal 1.24 install
  folder has them.
- Captures draw U+2500–U+259F procedurally, as Windows Terminal does, and every
  other glyph from the font. Modifiers other than `REVERSED` are ignored: bold,
  italic and underlined cells draw as regular text. A glyph the font lacks or a
  cell with more than one codepoint is an error.
- `Sketch` ink is 0..=1 (the pixmap stores sRGB bytes); anti-aliased edges blend
  in sRGB, as tiny-skia does. Light a sketch afterwards with a tint.
- Sixel: Windows Terminal erases image pixels in any cell text is written to,
  stretches each 10 × 20 virtual cell to the real cell (so images keep the
  cell aspect distortion), and scrolls the screen when an image crosses the
  bottom row; `SixelLayer` crops such an image to whole 6-row bands.
  Transparent pixels keep the cell's earlier content, so a moving transparent
  image leaves trails.

## Run

```
cargo run -p terminal-graphics --example preview                # live viewer
cargo run -p terminal-graphics --example preview -- --smoke     # no terminal, no font
cargo run -p terminal-graphics --example preview -- --export target/preview/preview.png --font "<WT install dir>/CascadiaMono.ttf"
```

Live keys: Tab source (PNG / ASCII), Space quadrant/octant, Left/Right aspect.
`--export` writes `OUT.png` (100 × 30 cells, octants, font aspect),
`OUT-sheet.png` (both encoders at the font aspect and at 2.0) and `OUT.apng`
(aspect swept 1.6–3.0). `--png PATH`, `--quadrant`, `--aspect X` change the
input. Glyph sheet of U+2500–U+259F: `TERMINAL_GRAPHICS_FONT=<font> cargo test
-p terminal-graphics --test capture -- --ignored` → `target/glyphs.png`.

## Limits

- A capture shows one font's glyphs, not a live terminal. The live viewer was
  not run from an agent shell; its key handling is unverified in a terminal.
- Double-line joins and dashes approximate Windows Terminal's builtin glyphs;
  they were compared by eye only, not pixel for pixel.
- PNG colour profiles are not applied; 16-bit channels keep the high byte.

## Provenance

Adapted from Runner `crates/runner-lowres/src/{encode.rs,color.rs,capture.rs,
painting.rs}`, `crates/runner/src/art.rs` (`Sketch`, calibration) and
`crates/runner-data/src/pictures.rs` (PNG decode), without Runner's game
content, palettes or light states.
