// ADR: A place file is strict TOML (`seed`, `ambient`, `map`, `[legend."c"]` tables, optional `[markers]`); unknown or missing fields are refused and every error reads `file:line: key: reason`.
// ADR: Values that need range checks keep their spans (`toml::Spanned`), so a semantic error points at the offending line, and map errors point at the map row.
use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;
use std::path::Path;

use serde::Deserialize;
use toml::Spanned;

use crate::place::{Legend, LightDef, Pattern, Place, PlaceError, Surface, TileDef};

/// Why a place file was refused.
#[derive(Debug)]
pub enum LoadError {
    /// The file could not be read.
    Io { file: String, error: std::io::Error },
    /// The file was read but its content is invalid. `line` counts from 1.
    Invalid {
        file: String,
        line: usize,
        key: String,
        reason: String,
    },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io { file, error } => write!(f, "{file}: {error}"),
            LoadError::Invalid {
                file,
                line,
                key,
                reason,
            } => write!(f, "{file}:{line}: {key}: {reason}"),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoadError::Io { error, .. } => Some(error),
            LoadError::Invalid { .. } => None,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPlace {
    seed: u32,
    ambient: Spanned<String>,
    map: Spanned<String>,
    legend: BTreeMap<Spanned<String>, RawTile>,
    #[serde(default)]
    markers: BTreeMap<Spanned<String>, Spanned<Vec<usize>>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTile {
    solid: bool,
    color: Spanned<String>,
    pattern: Pattern,
    grain: Spanned<f32>,
    grain_scale: Spanned<f32>,
    emissive: Option<Spanned<String>>,
    light: Option<Spanned<RawLight>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLight {
    color: Spanned<String>,
    radius: Spanned<f32>,
    intensity: Spanned<f32>,
    flicker: Spanned<f32>,
    glow: Spanned<f32>,
}

/// Reads and parses a place file; see `parse_place`.
pub fn load_place(path: &Path) -> Result<Place, LoadError> {
    let file = path.display().to_string();
    let source = std::fs::read_to_string(path).map_err(|error| LoadError::Io {
        file: file.clone(),
        error,
    })?;
    parse_place(&source, &file)
}

/// Parses place TOML; `file` names the source in errors.
pub fn parse_place(source: &str, file: &str) -> Result<Place, LoadError> {
    let at = |offset: usize, key: String, reason: String| LoadError::Invalid {
        file: file.to_string(),
        line: line_of(source, offset),
        key,
        reason,
    };
    let raw: RawPlace = toml::from_str(source).map_err(|e| {
        // An empty span at the start or one over the whole document (a missing top-level field)
        // names no line's key.
        let whole =
            |s: &Range<usize>| s.start == 0 && (s.end == 0 || s.end >= source.trim_end().len());
        match e.span().filter(|s| !whole(s)) {
            Some(span) => at(span.start, key_at(source, span.start), e.message().into()),
            None => at(0, "document".into(), e.message().into()),
        }
    })?;
    let ambient =
        color(&raw.ambient).map_err(|r| at(raw.ambient.span().start, "ambient".into(), r))?;
    let mut legend = Legend::new();
    for (glyph, tile) in &raw.legend {
        let key = format!("legend.{:?}", glyph.get_ref());
        let mut chars = glyph.get_ref().chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            return Err(at(
                glyph.span().start,
                key,
                "a legend key must be exactly one character".into(),
            ));
        };
        let def = tile_def(tile)
            .map_err(|(span, field, reason)| at(span.start, format!("{key}.{field}"), reason))?;
        legend.insert(c, def).map_err(|e| match e {
            PlaceError::Tile { invalid, .. } => {
                let span = tile_span(tile, invalid.field);
                at(
                    span.start,
                    format!("{key}.{}", invalid.field),
                    invalid.reason,
                )
            }
            other => at(glyph.span().start, key.clone(), other.to_string()),
        })?;
    }
    let map_line = map_first_row_line(source, raw.map.span());
    let mut place = Place::new(raw.map.get_ref(), legend, ambient, raw.seed).map_err(|e| {
        let (line, reason) = match e {
            PlaceError::RaggedRow { row, .. } | PlaceError::UnknownChar { row, .. } => (
                map_line.map_or(line_of(source, raw.map.span().start), |l| l + row),
                e.to_string(),
            ),
            other => (line_of(source, raw.map.span().start), other.to_string()),
        };
        LoadError::Invalid {
            file: file.to_string(),
            line,
            key: "map".into(),
            reason,
        }
    })?;
    for (name, tile) in &raw.markers {
        let key = || format!("markers.{}", name.get_ref());
        let &[x, y] = tile.get_ref().as_slice() else {
            return Err(at(
                tile.span().start,
                key(),
                "a marker is [x, y], two tile coordinates".into(),
            ));
        };
        place.add_marker(name.get_ref(), (x, y)).map_err(|e| {
            let span = match e {
                PlaceError::MarkerName { .. } => name.span(),
                _ => tile.span(),
            };
            at(span.start, key(), e.to_string())
        })?;
    }
    Ok(place)
}

type FieldError = (Range<usize>, &'static str, String);

fn tile_def(tile: &RawTile) -> Result<TileDef, FieldError> {
    let field = |s: &Spanned<String>, name: &'static str| {
        color(s).map_err(|reason| (s.span(), name, reason))
    };
    let light = match &tile.light {
        Some(light) => {
            let l = light.get_ref();
            Some(LightDef {
                color: field(&l.color, "light.color")?,
                radius: *l.radius.get_ref(),
                intensity: *l.intensity.get_ref(),
                flicker: *l.flicker.get_ref(),
                glow: *l.glow.get_ref(),
            })
        }
        None => None,
    };
    Ok(TileDef {
        solid: tile.solid,
        surface: Surface {
            color: field(&tile.color, "color")?,
            pattern: tile.pattern,
            grain: *tile.grain.get_ref(),
            grain_scale: *tile.grain_scale.get_ref(),
        },
        emissive: tile
            .emissive
            .as_ref()
            .map(|e| field(e, "emissive"))
            .transpose()?,
        light,
    })
}

/// The span of the field `TileDef::check` named.
fn tile_span(tile: &RawTile, field: &str) -> Range<usize> {
    let light = tile.light.as_ref();
    let of_light = |pick: fn(&RawLight) -> Range<usize>| {
        light.map_or(tile.color.span(), |l| pick(l.get_ref()))
    };
    match field {
        "grain" => tile.grain.span(),
        "grain_scale" => tile.grain_scale.span(),
        "light" => light.map_or(tile.color.span(), |l| l.span()),
        "light.radius" => of_light(|l| l.radius.span()),
        "light.intensity" => of_light(|l| l.intensity.span()),
        "light.flicker" => of_light(|l| l.flicker.span()),
        "light.glow" => of_light(|l| l.glow.span()),
        _ => tile.color.span(),
    }
}

/// Parses `#rrggbb` into sRGB bytes.
fn color(s: &Spanned<String>) -> Result<[u8; 3], String> {
    let text = s.get_ref();
    let digits = text
        .strip_prefix('#')
        .filter(|d| d.len() == 6 && d.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| format!("{text:?} is not a colour of the form \"#rrggbb\""))?;
    let byte = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).expect("checked hex digits");
    Ok([byte(0), byte(2), byte(4)])
}

/// The 1-based line holding byte `offset`.
fn line_of(source: &str, offset: usize) -> usize {
    source.as_bytes()[..offset.min(source.len())]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        + 1
}

/// The line of the map's first row when the map is a multi-line string, whose rows sit one per
/// source line; `None` for a single-line string, whose rows share the value's line.
fn map_first_row_line(source: &str, span: Range<usize>) -> Option<usize> {
    let raw = source.get(span.clone())?;
    if !(raw.starts_with("'''") || raw.starts_with("\"\"\"")) {
        return None;
    }
    let mut start = span.start + 3;
    let rest = &source[start..];
    if rest.starts_with("\r\n") {
        start += 2;
    } else if rest.starts_with('\n') {
        start += 1;
    }
    Some(line_of(source, start))
}

/// The dotted key of the line holding `offset`: the nearest table header above it joined with
/// the line's own key, as written in the source.
fn key_at(source: &str, offset: usize) -> String {
    let line = line_of(source, offset);
    let mut table = String::new();
    let mut own = String::new();
    for (index, text) in source.lines().take(line).enumerate() {
        let text = text.trim();
        if let Some(header) = text.strip_prefix('[') {
            table = header.trim_end_matches(']').trim().to_string();
            own.clear();
        } else if index + 1 == line {
            own = text
                .split_once('=')
                .map(|(k, _)| k.trim().to_string())
                .unwrap_or_default();
        }
    }
    match (table.is_empty(), own.is_empty()) {
        (true, true) => "document".into(),
        (true, false) => own,
        (false, true) => table,
        (false, false) => format!("{table}.{own}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r##"seed = 3
ambient = "#202428"
map = '''
#####
#.L.#
#####
'''

[legend."#"]
solid = true
color = "#707478"
pattern = "flat"
grain = 0.1
grain_scale = 3.0

[legend."."]
solid = false
color = "#a09888"
pattern = "tiles"
grain = 0.2
grain_scale = 2.0

[legend."L"]
solid = false
color = "#a09888"
pattern = "tiles"
grain = 0.2
grain_scale = 2.0
emissive = "#302010"
light = { color = "#ffe8c0", radius = 5, intensity = 2.0, flicker = 0.2, glow = 1.5 }
"##;

    fn error(source: &str) -> String {
        parse_place(source, "test.toml").unwrap_err().to_string()
    }

    #[test]
    fn a_good_file_loads() {
        let place = parse_place(GOOD, "test.toml").unwrap();
        assert_eq!((place.width(), place.height(), place.seed()), (5, 3, 3));
        let (glyph, def) = place.tile(2, 1).unwrap();
        assert_eq!(glyph, 'L');
        assert_eq!(def.light.unwrap().radius, 5.0);
        assert_eq!(def.emissive, Some([0x30, 0x20, 0x10]));
        assert_eq!(place.lamps().len(), 1);
    }

    #[test]
    fn map_errors_point_at_the_map_row() {
        let unknown = GOOD.replace("#.L.#", "#.Lx#");
        assert_eq!(
            error(&unknown),
            "test.toml:5: map: map row 1 column 3: char 'x' is not in the legend"
        );
        let ragged = GOOD.replace("#####\n'''", "####\n'''");
        assert_eq!(
            error(&ragged),
            "test.toml:6: map: map row 2 has width 4, expected 5"
        );
        let empty = GOOD.replace("#####\n#.L.#\n#####\n", "");
        assert_eq!(error(&empty), "test.toml:3: map: the map has no tiles");
        let inline = GOOD.replace("'''\n#####\n#.L.#\n#####\n'''", "\"###\\n#x#\"");
        assert_eq!(
            error(&inline),
            "test.toml:3: map: map row 1 column 1: char 'x' is not in the legend"
        );
    }

    #[test]
    fn legend_errors_point_at_the_field() {
        assert_eq!(
            error(&GOOD.replace("radius = 5,", "radius = 0,")),
            "test.toml:30: legend.\"L\".light.radius: 0 is outside 0 (exclusive)..=32"
        );
        assert_eq!(
            error(&GOOD.replace("grain = 0.1", "grain = 2.0")),
            "test.toml:13: legend.\"#\".grain: 2 is outside 0..=1"
        );
        assert_eq!(
            error(&GOOD.replace("color = \"#707478\"", "color = \"grey\"")),
            "test.toml:11: legend.\"#\".color: \"grey\" is not a colour of the form \"#rrggbb\""
        );
        assert_eq!(
            error(&GOOD.replace("[legend.\".\"]", "[legend.\"..\"]")),
            "test.toml:16: legend.\"..\": a legend key must be exactly one character"
        );
        let solid_light = GOOD.replace(
            "solid = false\ncolor = \"#a09888\"\npattern = \"tiles\"\ngrain = 0.2\ngrain_scale = 2.0\nemissive",
            "solid = true\ncolor = \"#a09888\"\npattern = \"tiles\"\ngrain = 0.2\ngrain_scale = 2.0\nemissive",
        );
        assert!(
            error(&solid_light).starts_with("test.toml:30: legend.\"L\".light: a solid tile"),
            "{}",
            error(&solid_light)
        );
    }

    #[test]
    fn unknown_and_missing_fields_are_refused_with_their_line() {
        let unknown = GOOD.replace("grain_scale = 3.0", "grain_scale = 3.0\nshine = 1");
        let text = error(&unknown);
        assert!(
            text.starts_with("test.toml:15: legend.\"#\".shine: unknown field `shine`"),
            "{text}"
        );
        let missing = GOOD.replace("grain_scale = 3.0\n", "");
        let text = error(&missing);
        assert!(
            text.starts_with("test.toml:9: legend.\"#\": missing field `grain_scale`"),
            "{text}"
        );
        let pattern = GOOD.replace("pattern = \"flat\"", "pattern = \"stripes\"");
        let text = error(&pattern);
        assert!(
            text.starts_with("test.toml:12: legend.\"#\".pattern: unknown variant `stripes`"),
            "{text}"
        );
        let root = GOOD.replace("seed = 3\n", "");
        assert_eq!(error(&root), "test.toml:1: document: missing field `seed`");
    }

    /// `GOOD` with a `[markers]` table holding `body`; returns the source and `body`'s first line.
    fn with_markers(body: &str) -> (String, usize) {
        (
            format!("{GOOD}\n[markers]\n{body}\n"),
            GOOD.lines().count() + 3,
        )
    }

    #[test]
    fn markers_load_by_name() {
        let (source, _) = with_markers("exit = [3, 1]\nentry = [1, 1]");
        let place = parse_place(&source, "test.toml").unwrap();
        assert_eq!(place.marker("entry"), Some((1, 1)));
        assert_eq!(place.marker("exit"), Some((3, 1)));
        assert_eq!(place.markers().count(), 2);
        let plain = parse_place(GOOD, "test.toml").unwrap();
        assert_eq!(plain.markers().count(), 0);
    }

    #[test]
    fn marker_errors_point_at_the_marker() {
        let (source, line) = with_markers("entry = [1, 1]\nexit = [5, 1]");
        assert_eq!(
            error(&source),
            format!(
                "test.toml:{}: markers.exit: marker \"exit\" at (5, 1) is outside the 5 × 3 map",
                line + 1
            )
        );
        let (source, line) = with_markers("work = [0, 2]");
        assert_eq!(
            error(&source),
            format!(
                "test.toml:{line}: markers.work: marker \"work\" at (0, 2) is on solid tile '#'"
            )
        );
        let (source, line) = with_markers("Work = [1, 1]");
        assert!(
            error(&source).starts_with(&format!(
                "test.toml:{line}: markers.Work: marker name \"Work\" must be"
            )),
            "{}",
            error(&source)
        );
        for bad in [
            "work = [1]",
            "work = [1, 1, 1]",
            "work = [-1, 1]",
            "work = \"a\"",
        ] {
            let (source, line) = with_markers(bad);
            let text = error(&source);
            assert!(
                text.starts_with(&format!("test.toml:{line}: markers.work: ")),
                "{bad}: {text}"
            );
        }
    }
}
