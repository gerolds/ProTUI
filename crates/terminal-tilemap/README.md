# terminal-tilemap

Top-down lit tile maps. A place is authored as ASCII plus a data legend. The crate bakes it into a 16 px per
tile raster, lights it with soft-shadowed point lights, and encodes the camera's view into octant or quadrant
cells of a Ratatui `Buffer` through `terminal_graphics::encode_image`. A raw view draws one legend char per tile.
Coordinates are in tiles; tile centres sit at `.5`. Use one `Frame` per `World`.

## Entry points

- `Place::new(rows, legend, ambient, seed)`: build a place in code. `Legend::insert(char, TileDef)` checks
  each entry. Errors: `PlaceError` (empty map, ragged row, unknown char, side above 256, bad legend field, bad
  marker). `Place::add_marker(name, (x, y))` names an open tile; `marker(name)` and `markers()` read them back.
- `load_place(path)` / `parse_place(source, file)`: the TOML format below. Errors: `LoadError`, shown as
  `file:line: key: reason`.
- `World::new(place)`: geometry, bake and lamp shadow fields, built once (66 ms in release for the sample).
- `World::render(&mut Frame, t, &Camera, Encoder, area, &mut Buffer)`: one lit frame. `t` is seconds and drives
  flicker. Fill `Frame::dynamic` with `DynamicLight`s first; a light keeps its field while its key stays and it
  does not move. `World::light_at(&frame, (x, y))` reads the last frame's light.
- `World::render_raster(&mut Frame, t, &Camera, area, (sx, sy))`: the same lit frame as a linear `Raster` with
  `sx × sy` pixels per cell, borrowed from the `Frame`, for raster outputs such as `terminal_graphics::sixel`.
- `Camera::overview(area, TileRect, focus, limit, aspect)`: fits the rect into `area` with square tiles on cells
  `aspect` times as tall as wide, at most `limit` and at least 2 cells per tile; it crops around `focus` when the
  rect does not fit. `Camera::raw(area, size, centre)`: one cell per tile for `draw_raw`.
- `draw_raw(place, camera, area, buf)`: the ASCII source view. `draw_pins(pins, camera, area, buf)`: crisp chars
  over either view, for markers. `Geometry`: the outline after 45° corner cuts and its signed distance field.

## Place TOML

```toml
seed = 1                  # varies textures and flicker; the same seed bakes the same raster
ambient = "#3a4250"       # sRGB light every pixel receives
map = '''
####
#LL#
####
'''

[legend."#"]              # one printable ASCII char per entry
solid = true              # solid tiles are walls: they stop light and form the outline
color = "#7a7e84"         # sRGB base colour; also the raw view's char colour
pattern = "noise"         # flat | noise | tiles | planks
grain = 0.15              # fine noise depth, 0..=1
grain_scale = 3.0         # fine noise size in bake pixels, 0.5..=64

[legend."L"]              # an open tile with a light at its centre
solid = false
color = "#a8a296"
pattern = "tiles"
grain = 0.12
grain_scale = 2.0
emissive = "#202020"      # optional: colour the surface emits regardless of light
light = { color = "#ffd8a8", radius = 7.0, intensity = 2.2, flicker = 0.15, glow = 1.6 }

[markers]                 # optional: named open tiles the host uses as pins and route points
entry = [1, 1]            # [x, y], tile column and row from 0
```

All fields except `emissive` and `light` are required; unknown fields are refused. `light` sits at the tile
centre and is refused on a solid tile. Ranges: `radius` above 0 and at most 32 tiles; `intensity` and `glow`
0..=16; `flicker` 0..=1. Map errors give the line of the offending row when the map is a `'''` string. A marker
name is a lowercase letter followed by lowercase letters, digits or `_`; its tile must lie inside the map and be
open. The crate gives marker names no meaning.

## Example

`cargo run --release -p terminal-tilemap --example place` opens the live view of `examples/sample.toml`: arrows
pan a 24 × 14 tile window, `o` toggles octant/quadrant, `r` the raw view, `+`/`-` the cell aspect, `q` quits.
A dynamic light and a `+` pin follow the focus. `--place FILE` loads another place. Headless:
`-- --frames 120 --size 160x50` prints the render call's ms per encoder (lamps only, then a moving light);
`-- --export out.png --font CascadiaMono.ttf [--size WxH]` saves one octant frame via `terminal-graphics`
`capture` (the font needs Unicode 16 octants, e.g. Windows Terminal 1.24's Cascadia Mono).

Measured 2026-11-04, release, sample place (46 × 15 tiles), whole-place overview, mean (worst) ms per frame:

| View | Octant, lamps | Octant, moving light | Quadrant, lamps | Quadrant, moving light |
|---|---|---|---|---|
| 80 × 24 | 1.96 (3.08) | 3.46 (5.05) | 1.53 (2.78) | 2.97 (4.47) |
| 160 × 50 | 5.44 (7.81) | 6.57 (9.46) | 3.44 (4.82) | 4.35 (6.65) |

`cargo run --release -p terminal-tilemap --example sixel_probe` shows the sample place, lit with flicker and a
moving light, in a 60 × 20 cell rect as sixel (Windows Terminal 1.22+) or octants, beside text panels with measured
fps and ms. Keys: `d` density (2 × 4, 5 × 10, 10 × 20 px per cell), `p` palette, `o` octant/sixel, `m` move and
shrink the image, `c` clear, `q` quit. `--bench N` measures headless; `--log FILE`, `--seconds N` and
`--script 2:o,4:m` serve captures with `tools/wt-capture.ps1`; `--query FILE` records the terminal's DA1 and
cell-size replies. Numbers: `.agent/docs/rendering.md` "Sixel".

## Limits and provenance

No field of view, fog, actors or effects; emissive surfaces do not light their surroundings. At most 256 tiles
per side. The live view is not covered by tests; captures prove one font's glyphs, not a terminal.
Adapted from Runner `crates/runner-lowres/src/` (`geometry.rs`, `light.rs`, `camera.rs`, `world.rs`, `raw.rs`,
`draw.rs` pins), without its tile kinds, palette, fixtures, explored memory or scene. The bake shaders and the
legend/TOML format are new.
