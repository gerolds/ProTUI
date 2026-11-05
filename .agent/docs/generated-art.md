# Generated pictures for block-glyph drawing

Reference findings from another project, not a requirement: the look of this game is a choice in [design/](../../design/README.md). The workflow at the end is tooling fact.

## What survives the encoding

An encoder gives each cell one glyph and two colours. Octant reads 2 × 4 samples per cell, quadrant 2 × 2 (it averages
vertical pairs). Each sample is the box average of its source pixels in linear light ([rendering.md](rendering.md)
"Encoding", "Two-colour fit"). So a picture shown in a 9 × 4 box is 18 × 16 samples in octant and 18 × 8 in quadrant,
with only two colours per 2 × 4 group.

**Survives:**

- Big value masses: a light shape against a dark ground, a light half and a dark half on each big shape.
- The silhouette, when dark space surrounds it.
- One to three colour masses, each covering a large area.
- A whole-picture change of light: a flash over the whole subject read at every size tried.

**Dies:**

- Anything smaller than one sample. At 9 × 4 one sample is about 1/18 of the picture's width, at 16 × 8 about 1/32.
  It averages away.
- High-contrast fine detail (texture, hair, rivets, sparks, scattered highlights, bright edge outlines). The cell's
  two-colour split follows the texture instead of the shape, and the result is speckle.
- Thin lines: below a sample wide they fade into the average; with high contrast they break into dots
  [INFERENCE from the encoder].
- Low-contrast gradients and soft light from everywhere: soft light gave dark, dull pictures at every size. Each cell
  flattens to one or two near-equal colours (equal colours become a space), so a smooth gradient steps into bands
  [INFERENCE from the encoder].

**Readability by size** (painted pictures, octant, cell aspect 2.57; details in [rendering.md](rendering.md)
"Readability by size"):

| Box (cells) | What read | Prompt consequence |
|---|---|---|
| below 9 × 4 | noise | do not use a painted picture; draw a shape instead |
| 9 × 4 | dominant colour and silhouette; two subjects of one colour look alike | one colour per subject across the set |
| 12 × 5 | outlines such as a hood or a hat | headgear and outline shapes carry identity |
| 16 × 8 | a face; a raised arm, a sling | variants must change a big shape |
| wide scenes | read at every size tried | one focal object is enough |

At 9 × 4, quadrant has 8 sample rows and octant 16. Octant made heads recognisable there (ears, a hat brim, a hood);
a font with a narrower cell added less than the switch to octant.

## Prompt structure

The generator sends the subject line first, then the style text ([Make a picture](#make-a-picture)). Each rule below
is structure, not theme.

### Shared style block

- **One style text per picture type, kept in one place** (`[styles]` in the prompts file), never retyped. Why: retyped
  style text drifted; one block keeps light, ground and finish the same across a set.
- **One key light from one side, no rim light.** Name the side and keep it the same for the whole set. Ask that the
  lit side reaches light values and the far side falls into deep shadow, so each big shape has a light half and a
  dark half. Effect: the strongest gain in readability at 9 × 4. A rim light drew a bright outline that broke into
  speckle; soft light alone gave dark, dull pictures.
- **Contrast only between big shapes.** Ask for three masses: the silhouette, one large mid-tone colour area that
  identifies the subject, and exactly one small bright accent (eyes, a gem, a flame). All fine detail stays "soft,
  sparse and low in contrast inside its mass, with smooth painted surfaces rather than busy texture". Effect: detail
  blurs away at small sizes instead of turning into speckle.
- **Colour: deep and saturated over a large area.** "Each subject keeps the colour its description names … as a
  deep, clearly saturated natural colour over a large area, so it is recognisable from that colour alone; everything
  else stays in quiet tones." Never "muted": it washed out the identifying colour.
- **A known flat ground.** Name an exact ground colour (the source used one near-black hex) "around the subject" or
  "in the darkest areas and at the edges". Why: the silhouette needs dark space, and a known ground lets code find the
  subject by its luminance above the ground (a flash tinted only the subject, not the ground) and composite cleanly.
- **Exclusions:** "No text, no frame, no border, no signature." "Not a cartoon, not childish, no thick outlines."
  "Not glossy, not poster art."

### Portraits

Subject line: framing first, then identity, then the colour.

```text
A square head-and-shoulders portrait bust, centred, filling most of the frame. <who, in a few words>:
<two or three big identifying shapes>. Signature colour: <one colour>.
```

- Framing: a bust, head large in the frame, clear dark space around head and shoulders. Every subject tried came out
  usable with this opening. Full figures and three-quarter views were not tried; they make the head smaller, so
  the 12 × 5 and 16 × 8 thresholds move up [INFERENCE].
- Identity through big shapes: a hat brim wider than the shoulders, ears sticking out sideways, a very wide hood.
  Outlines like these read from 12 × 5; facial features need 16 × 8.
- At most one prop (held or carried object). Clothing and headgear are not props, but keep them simple. Subjects with
  a weapon, armour plates and gear stayed busy at 9 × 4 in every prompt set.
- No story or mood paragraph, no text in the picture: the style text already sets the mood.

### Places and scenes

```text
A wide landscape (3:2) scene, seen at eye level, centred composition with one clear focal object. <the object and
its setting>. Signature colour: <one colour>.
```

- One focal object, large in the frame, as one solid shape against darker surroundings; the picture's edges fall off
  into the ground colour. Scenes made this way read at every size tried.
- Match the frame to the box. A 3:2 painting cropped to a strip about 4:1 on screen kept only a dark band. A prompt
  asking for "everything in one band across the middle third" filled the strip and its items read at 152 × 11 cells.
- Several objects in one place: ask for each on its own patch of ground, and give the count; check the count, the
  model may paint a different number on another run.
- Top-down room views were not generated in the source project. Carrying the rules over [INFERENCE]: a straight-down
  view, the floor as one flat dark ground, walls and large furniture as the big masses, light from the same side as
  the set, few objects each on its own floor patch. For a lit tile map see [place-views.md](place-views.md); there,
  crops of a shared map used as portraits showed mostly floor, and separate pictures were the fix.

### Items and icons

- Below 9 × 4 a painted picture turned to noise. At icon size draw shapes (`Sketch` in
  [crates/terminal-graphics](../../crates/terminal-graphics/README.md)) or use a glyph.
- For an item picture at 9 × 4 or larger use the portrait rules: one object, centred, filling the frame, on the
  known ground, one colour mass and one accent.
- A set with the head filling the frame in 3 to 5 flat colours read at 9 × 4 but looked childish. Flat colour is
  the reliable route to tiny sizes; its cost is a cartoon look.

### Variants of a picture

- Make a variant as an edit of the base picture, not as a fresh prompt. Edits kept the picture pixel for pixel
  (median difference about 1 in 255; only 0.1–0.9 % of pixels differed by more than 30). A fresh prompt for the same
  subject gave another composition (median 7, 8 % of pixels).
- Change a big shape or a colour mass: a raised arm, a sling, a bandage over half the face, a sickly colour over the
  whole subject. A cut or a glint in the eye did not read at 16 × 8.
- Make every variant from the same base. Edits chained on edits repainted earlier changes slightly, so cut-out masks
  bled.
- For parts that combine (a scene that gains objects): paint the full scene first, then remove objects in edits
  (median difference 2 in 255), and cut each object out by its colour difference to the picture without it. Adding
  each object as its own edit of an empty scene failed: the model put two objects on the same ground. Regions that
  move (fire, smoke) changed in every edit, so they cannot be cut out this way.

## What failed

| Symptom | Cause | What changed |
|---|---|---|
| Speckle at 9 × 4, every subject orange, read as cliché | warm rim light, flat poster shapes, fine bright detail | no rim light; detail low in contrast inside its mass |
| Calm but dark and dull at every size | soft light from everywhere, no value split | one key light, a light half and a dark half per shape |
| Distinct but noisy | bright light plus some high-contrast detail left | "smooth painted surfaces rather than busy texture" |
| Identifying colours washed out | "slightly muted colours" in the style | "deep, clearly saturated … over a large area" |
| Three subjects came out red; in another set three green | subject texts named red or green parts; the model filled in its default hue | a colour plan for the whole set, one named colour each, checked against existing entries |
| A purple hood came out mauve | the model ignored the named colour | retry, or keep that subject's picture from an earlier set |
| Busy at 9 × 4 | weapon, armour and gear together | one prop at most |
| Read at 9 × 4 but childish | 3–5 flat colours, head filling the frame | dropped; "not a cartoon, no thick outlines" |
| Sharpening plus saturation, or 8-colour clustering, after shrinking | post-processing | dropped; plain linear-light box average looked better |
| Wide strip showed a dark band | 3:2 composition cropped to about 4:1 | "everything in one band across the middle third" |
| Pictures 1.29 times too tall in a narrow font | code assumed 1:2 cells | crop at the player's cell aspect |
| Variant change invisible | change in fine detail | change a big shape |
| Desaturation looked like a disabled state | full-strength grey | keep it below about 0.6 |
| A warm light shifted a cyan identifying colour | tint by a strongly coloured light | mix 60 % white into the light colour |

## Production pipeline

- **Size.** The model's output was shrunk for storage: 512 px wide in the experiments, 256 px in production. The
  widest box drawn in the experiments needed 108 px (54 cells in octant). 256 px covers a box 128 cells wide
  ([content.md](content.md) "PNG pictures"). Shrink once with a good filter; do not post-process.
- **Into the game.** Load with `StillImage::from_png(reader, background)`: sRGB converts to linear light, and alpha
  composites over `background`, so use the prompted ground colour there. Draw with one `ImageCache` per panel:
  `draw(image, tint, encoder, area, buffer)` centre-crops to the panel's shape on screen (cells × cell aspect),
  box-averages in linear light once per size and aspect, and applies `tint` (an `Rgb` multiplier) without
  resampling ([crates/terminal-graphics/README.md](../../crates/terminal-graphics/README.md)).
- **Aspect.** Prompt the frame the box will crop to. A square picture in a box much wider than tall leaves empty
  columns; size the box to the picture ([rendering.md](rendering.md) "Size and aspect").
- **Light states as tints.** A flash over the whole subject read at every size; masking it to pixels above the ground
  luminance kept the ground dark. Grey for "uneasy" stayed below 0.6. A coloured light keeps identifying colours only
  with a large white share.
- **Judge on contact sheets.** Render each picture at every box size the game uses, in octant and quadrant, at two
  cell aspects (2.0 and 2.57 in the source), and save them side by side (`capture` feature: `contact_sheet`). Put the
  whole set on one sheet to catch colour collisions.
- **Consistency.** One style block, one light side, one ground colour and one framing line per picture type for the
  whole set; variants as edits of their base.

## Accept check

Before keeping a picture:

1. Look at it encoded at the smallest box it will use, at actual cell size in a terminal or a capture, not the
   source PNG.
2. Squint (or view from a distance): one distinct shape in its own colour, no speckle, ground dark around it.
3. Compare it with the rest of the set at that size: no other subject in the same colour; same light side.
4. Check octant and quadrant, and a second cell aspect.
5. At the largest box: the look still holds, and the framing fills the box without empty columns.
6. For a variant: the change reads at its box size, and the rest matches the base.
7. Counts and colours as asked; the model sometimes ignores them. Delete and regenerate if not.

## Make a picture

**Before you start:** Python 3.11 or newer with Pillow. The Codex CLI with its image tool: set `CODEX_EXE` to its
path, or put `codex` on `PATH`. The model comes from Codex's own configuration.

The prompts file (default `assets/pictures/prompts.toml`; pass another path as the first argument):

```toml
[styles]
plain = "One key light from the left; the far side falls into deep shadow. Plain near-black ground. No rim light. No text, no frame, no border."

[samples]
test-cube = { style = "plain", subject = "A wide landscape (3:2) scene, seen at eye level, centred composition with one clear focal object. A single matte cube on a table. Signature colour: teal." }
test-cube-lit = { style = "plain", from = "test-cube", change = "Make the cube orange." }
```

Each table except `[styles]` is a kind; each key becomes `<out>/<kind>/<key>.png`. Kinds and keys are lowercase
letters, digits and `-`. An entry is `{ style, subject }`, or a variant `{ style, from, change }` that edits the
picture of `from`, a plain entry in the same kind. The whole file is checked before any run, and every problem is
named. The prompt sent is the subject line, then the style text; a variant asks to keep everything and change only
`change`.

1. Add the entry to the prompts file.
2. From the repository root run `python tools/art/generate.py [prompts.toml] [--out <dir>] [--width 256]`. It makes
   each missing picture, six at a time, and stores it at most `--width` px wide. Variants wait for their base. `--out`
   defaults to the prompts file's folder. Run `--help` for the options.
3. Run the accept check above.

**When it breaks:** a missing picture prints the end of Codex's error output (or a timeout after 1200 s). Delete a
bad picture and run again; the generator never overwrites an existing picture. An older npm `codex` that fails on
configuration it does not know is the wrong executable; point `CODEX_EXE` at the desktop CLI.

Provenance: Runner `samples/portrait-images/README.md`, `.agent/docs/generated-art.md`, `assets/pictures/prompts.toml`
and its picture design records, 2026-11-04.
