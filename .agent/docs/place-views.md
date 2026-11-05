# Top-down place views: findings from a sibling project

Reference material from another project, not a requirement. This game's design choices live in `design/`.

The sibling project drew a top-down tile map as a lit, low-resolution raster in quadrant or octant cells, with
figures, effects and a camera, and changed it after playtests. Encoder, font, tone-curve and cell-aspect facts are in
[rendering.md](rendering.md) and [tui-ratatui.md](tui-ratatui.md) "Cell aspect". Playtests had one player (that
project's user). [INFERENCE] marks what no source states.

## What players read on the map

- **Where the action is.** On the whole map a moving group read at once, as a moving band of coloured light. Which
  figure was which read only by colour.
- **The visited place.** The player asked for explored ground as "a map in shadow, read as explored but not lit".
  Near-black remembered ground failed; a dim cool grey, in which walls, pillars and corridors read, was kept.
- **The way out.** A near-white glyph on the exit tile was missed. The next playtest asked for "an obvious overlay";
  a boxed text label at the exit, or on the map's edge toward it, followed.
- **Information at the action.** Short damage text rising at the figure was "the clearest element: it explains what
  happens". State changes shown at the screen's edge were missed.
- **Moving text.** Labels that moved with the camera could not be read; labels held still while open were kept.
- **Zoom as a signal.** A close-up at up to 5 cells per tile (about 8 on a 200-column terminal) "tells the player that
  details matter". The zoom was capped at 4 cells per tile.
- **Camera motion.** Automatic switches between the whole map and a room were "jarring". Framing became a player
  setting: follow the group (default), the whole explored map, or automatic close-ups.
- **Slowed time.** Choices with several facts beside them could not be read even at a twentieth of normal speed.
  The change made was to show each fact on the map from the moment its place was seen.
- **Where facts live.** The lit style was accepted with less map detail than a line map. Facts went to the
  surrounding text UI and to crisp glyphs on the map; the raster carried mood and position.

## Techniques and their numbers

### Layers and resolution

The map data stayed a plain tile grid; every outline, texture, shadow and light was derived from it. A bake at 16
raster px per tile edge (albedo, wall bands, occlusion, shapes) was built once per map; light (soft shadows, flicker,
remembered colour) every frame; raster effects before encoding; crisp glyphs (markers, pins, labels, numbers) over the
encoded cells. Octant gives 2 × 4 samples per cell, quadrant 2 × 2. At `c` cells per tile, a tile gets `2c` samples
across: one sample spans `8 / c` raster px, 4 px at 2 cells per tile, 2 px at 4. The whole map showed a tile as
about 3 × 2 cells.

### Camera and framing

- Lower bound 2 cells per tile. Below it shapes stopped reading, so the camera cropped around a focus point inside the
  framed rect; on 80 × 24 that showed part of a 48 × 26 map.
- Upper bound 4 cells per tile after the playtest: a larger terminal showed more of the place, not larger tiles. A
  room close-up framed the room with 3 tiles of surroundings (before: 1 tile, at the largest scale that fit).
- The followed point plus 3 tiles joined every framing and close-up. The explored framing used a 1-tile margin and at
  least 30 × 14 tiles.
- Close-ups eased over 0.35 s (smoothstep), framing growth over 0.2 s. Between two cameras the scale moved
  geometrically and the centre in step, so every view edge moved linearly and a point both showed stayed in view.
- Shake moved only the raster camera, `amplitude × (1 − age / duration)²` cells; glyphs kept the steady camera (mixing
  the two made glyphs jump). Reduced motion cut close-ups instead of easing and turned off shake, flashes and dust.

### Walls, floors and texture

- **Corner cuts.** A tile corner was cut to 45° only on a staircase step: the vertex had 1 or 3 open tiles and a
  neighbouring vertex the opposite count. Square rooms kept square corners; irregular regions read round. An open
  region with at least 6 cut corners counted as a cave: more outline wobble, a rougher floor.
- **Distance field.** Coverage sampled 4 × 4 per pixel, then an exact Euclidean distance transform (Felzenszwalb and
  Huttenlocher) gave a signed distance, positive in walls. Wall bands, occlusion and outline noise read it.
- **Occlusion.** Floor brightness `0.28 + 0.72 × smoothstep(0, 14, −sd)`, `sd` in px: floors darken toward walls.
- **Wall band.** A pale lip in the wall's first 1.6 px, a band of 6 px blocks, plain rock from 7–10 px in.
  [INFERENCE] The lip draws the wall line once blocks fall below one sample.
- **Round occluders.** Pillars were discs of 0.3 tile radius with a contact shadow 9 px past the edge.
- **Detail that survives.** A feature under one sample vanished, or became speckle when the two-colour fit split on
  texture. Floor rows of 8 px, stones 9–18 px long and 0.3% pale dots were in use; which survived at overview zoom
  was not recorded.
- **A browser lab** rendered one scene at full resolution and at 120 × 80 to 288 × 192 samples, with passes for
  albedo, light, solid mask and moving parts alone. It showed which layer caused a look before terminal code existed.

### Light

- A light grid point every 4 px. Each source raymarched once over a box of 1.12 × its reach, 1 px per step. Soft
  shadows: visibility averaged from 5 origins, the centre and 4 points 3 px out. A ray ending in a wall may cross
  9 px of wall, so wall faces toward a light catch it.
- Falloff `(1 − x²)² / (1 + 5x²)`, `x` = distance / reach; the grid read bilinearly.
- Fixed lamps: one field each, per map. Moving lights: fields cached by key, recomputed only when moved more than
  0.25 px or radius changed. A frame then only sums fields.
- Flicker: `n = 0.65 wave(2.3 t) + 0.35 wave(6.1 t)` per seed; gain `1 + depth × n`; green and blue vary by an extra
  0.3 and 0.6 × `depth × n`, so a dimming flame reddens; reach × `(1 + 0.3 depth wave(1.7 t))`. It scales the stored
  field: no shadow recompute, no per-pixel noise.
- Two clocks: an ambient clock (flicker, glints) that always runs; a timeline (figures, effects, camera) that slows.
- **Light as the subject.** A bright profile (ambient about 0.10–0.25 linear per channel) read everywhere. A dark one
  (ambient about 0.012–0.03, fixed lamps as small accents) let carried and effect lights carry the scene, with fire
  and blasts brightest. The dark one was chosen, then needed the remembered-ground fix.

### Sight, fog and memory

- Line of sight per tile from each viewer within 22 tiles, limited to the rooms and corridors the viewer stands in
  (doors as borders). A wall tile counted as seen when a ray reached it and one of its 8 neighbours was seen; this
  stopped a lit strip leaking through a doorway. Per pixel: `smoothstep(0.2, 0.8, bilinear(tiles))`. Recomputed
  only when a viewer changed tile: 3.3 ms.
- Memory raster, built once: albedo under steady lamp light, keeping luminance and 45% of the hue, times a cool tint
  and a gain. Gain 0.7 read near black; gain 4 gave the grey players accepted.
- Light showed only on explored pixels. Tiles were explored one step ahead of the walker, and the explored raster
  redone only around them. Decals marked only seen pixels; memory had none.

### Crisp glyphs over the raster

- Written after encoding, with the steady camera, on a backing from the cell's mean colour: markers bold on
  mean × 0.45, capped at linear luminance 0.03; pins plain on mean × 0.7. They read on any lit ground.
- Lit order: pins, markers, figure glyphs, then glyph effects and numbers. Raw order on a shared cell: number >
  figure > effect > pin > marker > tile.
- A placement query gave a tile point's cell, or the cell on the view's edge toward an off-view point. Screen
  distances used cells per tile across and rows per tile down, not an assumed 1:2 cell.
- Pins were rebuilt when their data changed (a route, per tile entered); choice labels were placed once through the
  camera the choice held.

### Raw view

- One cell per tile at every zoom, tile edges on cell edges; centred where the map fits, else on the lit camera's
  centre, clamped to the map's edge. One row per tile, so not square.
- Same data at the same moment as the lit view; switching never changed what happened. Effects as plain chars: `*`
  or a direction glyph for projectiles, a ring of `*` at the lit radius for a blast's 0.4 s pulse. No light, decals,
  shake or flash.
- Uses found: test snapshots (expected text grid, one line per row), debugging, and the ASCII renderer setting for
  fonts without block glyphs. Cost: a second drawing path kept in step with the data.

### Figures: glyphs or silhouettes

- **Token:** a bold glyph on a darkened backing over a ring in the raster (radius 0.36 tile, stroke 0.09, floor 55%
  inside). It marked the tile at any zoom.
- **Silhouette:** a dark body of about 0.6 tile (10 px) from shapes joined as signed distances; rim light
  `0.7 × light × facing^1.5` in a 1.4 px band; short shadows from every other light (`min(0.55 + 0.28 × distance,
  1.6)` tiles, occlusion weighted by `share^0.3` so bodies read in rooms lit from many sides); light cores of at
  least 1.7 px so each stayed a point when zoomed out.
- Findings: on the whole map only colour told figures apart; in a close-up 3 of 4 distinct outlines read. Features
  under 0.125 tile (one sample at 4 cells per tile) vanished: outlines told figures apart, details did not. An identity
  glyph on the overview covered the light; showing it only from 4 cells per tile worked best. Silhouettes cost about
  2 ms more per moving frame.

### Effects and how long they hold

- Raster effects (fire, smoke, flashes, particles, scorch) went into world pixels before encoding; a cell effect
  library works on finished cells and cannot add light ([rendering.md](rendering.md) "Raster protocols and effects").
  Light-emitting effects used the same light model.
- Each effect was a timed record with a start, a duration and a tile region; writes clipped to region ∩ view.
  Particles came from a generator seeded per particle index, so any frame rendered alone (captures, scrubbing).
- Values in play: blast light pulse 0.4 s, blast flash 0.1 s, sparks 0.12 s, dust puffs 0.5 s, scorch fading in over
  0.12 s and staying; a blast covered about 2.5 tiles at fight zoom. Floating numbers held 0.75 s, up to three words
  1.5 s, a one-row banner 2.5 s. Only the floating numbers were named in playtest as reading well.

### Timing to drawn frames

- Every frame was a pure function of time. Motion keys, pose records and effects were append-only; a key before the
  last was refused, so a shown frame never changed. Finished records were pruned.
- The picture ran one simulation step (0.05 s) behind the simulation, so motion interpolated between known points;
  the source judged that lag invisible, as its own inference.
- Sound was queued with its event's time and released when the drawn timeline reached it ([audio.md](audio.md)
  "Timing"). Pauses a player reported came from rule waits, not drawing; a bench listing every stand over 0.3 s with
  its cause showed that.

## What was reversed, and why

Reversals already described under "What players read on the map" (zoom, exit, framing, moving labels) are not repeated.

| Symptom | Cause found | What changed |
|---|---|---|
| Box-drawing rooms on flat colour did not appeal | No light, texture or depth | A lit raster derived from tile codes |
| An ink-on-paper style was rejected | User preference between two probed styles | Lit style only; ink code removed |
| Half-block walls blurred; sextants added little | 1 × 2 and 2 × 3 samples per cell | Quadrant and octant kept |
| The camera trailed the walker by up to 7 tiles | Framing grew over 0.35 s at a tile per 0.05 s | Followed point + 3 tiles in view; growth 0.2 s |
| A close-up held a room the group had left | Fixed 0.6 s hold plus 0.35 s ease-out | Close-up ends when the group leaves |
| Explored ground nearly black | Memory gain 0.7 under a dark light profile | Gain 4: dim cool grey |
| A raw-only view stalled 0.4–0.55 s on open | It triggered the lit bake it never used | Bake built at the first lit frame: 10–15 ms |
| Map crops as portraits showed floor | Figures stood together on one floor | Separate pictures |

## Performance

Budget 33 ms (30 frames a second); Ratatui writes only changed cells. Release, 160 × 50, real crossterm backend
counting bytes, 90 frames per run:

| Run | Quadrant ms mean (max) | Octant ms mean (max) | Bytes per frame (Q / O) |
|---|---|---|---|
| Whole map, flicker only | 3.5 (3.8) | 4.2 (4.5) | 5 036 / 5 521 |
| Walker moving: light field and sight recomputed | 8.9 (13.2) | 9.7 (16.5) | 3 470 / 3 815 |
| Same, four silhouettes with carried lights | 10.8 (16.0) | 11.6 (16.6) | 2 998 / 3 314 |
| Close-up at rest | 2.3 (2.5) | 3.0 (3.3) | 17 703 / 19 107 |
| Blast at close-up | 3.9 (10.0) | 4.6 (10.7) | 46 496 / 49 837 |

- One full generated run (quadrant, four silhouettes, cap 4): 8.3 ms mean, 20 ms worst, 15.9 kB per frame.
- What dominated: a large moving light's field and the sight update, about 3 ms each, once per tile entered. A 5-tile
  carried light's field cost well under 1 ms; body shadows, bodies and cores about 1 ms. Octant cost about 0.7 ms more.
- Bytes follow changed cells ([INFERENCE] a close-up shows more flickering lit cells). A blast reached about 50 KB per
  frame (1.5 MB a second); Windows Terminal kept up in a short test.
- Built once per map: geometry, bake, fixed lamp fields, memory raster: 172 ms on a 48 × 26 map, 0.4–0.55 s on
  generated maps, about 1 s in debug. Per frame: sum cached fields, scale by flicker, composite only the camera's
  pixel region, encode into reused buffers. A second render of the same moment reused its sight and light grid.

## Relation to this repository

The crate `crates/terminal-tilemap` (in a worktree at the time of writing) ports the map side. Its README measured a
66 ms build for 46 × 15 tiles and 1.5–6.6 ms mean per frame from 80 × 24 to 160 × 50.

| Finding | Covered by |
|---|---|
| Tile grid as the only source; 16 px per tile bake | `Place`, `TILE_PX`; a TOML legend instead of fixed tile kinds |
| Corner cuts and signed distance | `Geometry` |
| Linear light, 4 px grid, soft shadows, rim, falloff, flicker | `Light`, `STEP`, `World::render` with time `t`, `World::light_at` |
| Fixed fields once; moving fields cached by key | `World::new`; `Frame::dynamic` with `DynamicLight::key` |
| Square tiles, 2-cell floor, zoom cap, crop around a focus | `Camera::overview(area, rect, focus, limit, aspect)`, `MIN_CELLS_PER_TILE` |
| Raw view | `draw_raw`, `Camera::raw` |
| Crisp glyphs on a darkened backing | `draw_pins` (backing: mean of the cell's two colours × 0.6) |

Not ported, and what a port would involve:

- **Sight, fog, memory:** per-tile sight with the room rule, an explored mask, a memory raster built with the bake,
  and some border notion for doors in the legend [INFERENCE].
- **Camera motion:** framing tracks, close-ups, easing between cameras, follow margin, shake, reduced motion.
  `Camera` is a static fit today.
- **Figures:** tokens or silhouettes and their shadows. Where a design asks for views whose interest is not the
  characters, pins may already cover markers [INFERENCE].
- **Scene timeline and effects:** append-only timed records, the draw delay, raster effects and decals before
  encoding, glyph effects after, light-emitting effects through `Frame::dynamic`, seeded particles.
- **Placement and occluders:** the cell toward an off-view point; pillar discs; emissive surfaces that light others.

Provenance: Runner (`C:/Users/GeroldSchneider/Projects/Runner/Runner`): `.agent/docs/lowres-rendering.md`,
`plans/discovery/{dungeon-rendering,lantern-figures,tui-chrome,presentation,telling,dive-structure}.md`, decisions
D13, D15, D17, D32, playtests 2026-11-03 and 2026-11-04, `samples/dungeon-surfaces/`, commits beceee7, a5c8a42, 5807c37.
