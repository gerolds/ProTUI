# Content files

Read this before you add a content format (TOML, PNG, WAV) or its loader. Root rule: validate external input at
entry; never repair it ([AGENTS.md](../../AGENTS.md) "Code"). The loader rules here are engineering rules; where
problems show to the player is a reference finding from a sibling project (Runner). Which content the game has is
design (`design/`).

## TOML

- Strict: every key is required unless the format lists it as optional; unknown keys are refused. Derive
  `serde::Deserialize` with `#[serde(deny_unknown_fields)]` and no `#[serde(default)]`. An optional key is an
  `Option` only when its absence has a stated meaning, written in the format's documentation.
- Problems print as `file:line: key: reason`, one per line. `toml::de::Error::span()` gives the byte range of a
  syntax or shape problem; convert it to a line number. For a value that parses but breaks a rule (out of range, a
  name that points nowhere), keep its position with `toml::Spanned<T>` and report that line.
- A loader returns the whole checked set or every problem it found, not only the first.
- A file starts with one comment line saying what the thing is. The format itself is documented once, in the
  loader crate's README or a guide this index points at.
- Names are file stems (lowercase letters, digits, `-`). Files refer to each other by name; every name is checked to
  point somewhere. Colours are sRGB `"#rrggbb"` and become linear light at load. Durations are seconds.
- Version formats that are saved by the game (saves, settings): a save of another version is refused or migrated by
  a written step; it never loads into a different shape. A test fails when a change alters the loaded state without
  a version bump.
- Sample content in this repository is visibly neutral test data (a plain test room, a sine `ui/click`), never
  proposed game content.

## PNG pictures

- Decode 8- or 16-bit grey, RGB or palette PNGs with `png`, convert sRGB to linear light at load.
- State the alpha rule in the format: either composite over a stated background colour or refuse alpha. Never
  drop it silently.
- Store pictures as sRGB PNGs no wider than the largest box needs: 2 samples per cell across, so 256 px covers a box
  128 cells wide.
- Refuse a picture whose name matches no content entry, and an entry that requires a picture and has none. Report
  the path where the picture belongs.
- How a picture fits a box and reads at small sizes: [rendering.md](rendering.md). Making pictures:
  [generated-art.md](generated-art.md).

## WAV sounds

Sound manifest and file rules: [audio.md](audio.md) "Manifest".

## Live reload

- A watcher may poll file times a few times a second. A good set replaces the old one by name, so IDs held by the
  running game stay valid; a reload that drops a name in use is refused.
- A bad set keeps the old one running and reports the `file:line: key: reason` lines. The sibling project showed
  them in a problem strip; where they show is a `design/` choice.

## Checking

- Give each content folder one check command that loads everything, prints every problem as
  `file:line: key: reason`, and exits with code 1 if there is any. Run it after every content edit.
- Contract tests load the shipped content and a set of broken samples, and assert the error variant and line, not
  the wording.
