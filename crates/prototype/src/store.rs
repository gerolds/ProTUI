// ADR: One store for every versioned TOML file (settings now, saves later): `version` is read first, older versions pass written migration steps on the TOML table, then the strict read refuses unknown and missing fields; nothing is repaired.
// ADR: Writes go to `<file>.tmp` and are renamed over the file, so a failed write leaves the old file; an existing file the store cannot read is never overwritten.
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use toml::de::{DeTable, DeValue, Deserializer};

/// One written step from version `from` to `from + 1`, applied to the file's TOML table.
pub struct Step {
    pub from: u32,
    pub apply: fn(&mut toml::Table) -> Result<(), Invalid>,
}

/// A key a migration step could not convert, and why.
#[derive(Debug, PartialEq, Eq)]
pub struct Invalid {
    pub key: String,
    pub reason: String,
}

/// A versioned file kind: its file name in the data folder, the version it writes and the steps
/// that bring older versions up to it.
pub struct Schema {
    pub file: &'static str,
    pub version: u32,
    /// Steps by `from`, ending at `version - 1`; the oldest readable version is the first `from`.
    pub steps: &'static [Step],
}

impl Schema {
    /// The oldest version a file may have and still be read.
    pub fn oldest(&self) -> u32 {
        self.steps.first().map_or(self.version, |step| step.from)
    }

    /// Where this file lives in `dir`.
    pub fn path(&self, dir: &Path) -> PathBuf {
        dir.join(self.file)
    }

    fn temporary(&self, dir: &Path) -> PathBuf {
        dir.join(format!("{}.tmp", self.file))
    }
}

/// Why a file could not be read or written.
#[derive(Debug)]
pub enum StoreError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    /// Bad TOML, an unknown or missing key, a value out of range, or a version this build cannot
    /// read. `line` is 1-based.
    Format {
        path: PathBuf,
        line: usize,
        key: String,
        reason: String,
    },
    /// A write refused because the file there is not one this build can read.
    Unreadable {
        path: PathBuf,
        cause: Box<StoreError>,
    },
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Io { path, source } => write!(f, "{}: {source}", path.display()),
            StoreError::Format {
                path,
                line,
                key,
                reason,
            } => write!(f, "{}:{line}: {key}: {reason}", path.display()),
            StoreError::Unreadable { path, cause } => write!(
                f,
                "{} was not overwritten because it cannot be read: {cause}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for StoreError {}

/// Reads the file of `schema` in `dir`; `Ok(None)` when there is no file.
pub fn load<T: DeserializeOwned>(dir: &Path, schema: &Schema) -> Result<Option<T>, StoreError> {
    let path = schema.path(dir);
    match fs::read_to_string(&path) {
        Ok(text) => parse(&path, &text, schema).map(Some),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(StoreError::Io { path, source }),
    }
}

/// Writes `value` as the file of `schema` in `dir` (creating `dir`), with `header` (comment
/// lines, each starting with `#`) above it. Refuses when the file there cannot be read.
pub fn save<T: Serialize + DeserializeOwned>(
    dir: &Path,
    schema: &Schema,
    header: &str,
    value: &T,
) -> Result<(), StoreError> {
    let path = schema.path(dir);
    if let Err(cause) = load::<T>(dir, schema) {
        return Err(StoreError::Unreadable {
            path,
            cause: Box::new(cause),
        });
    }
    let text = write(schema, header, value).map_err(|reason| StoreError::Format {
        path: path.clone(),
        line: 0,
        key: "-".to_string(),
        reason,
    })?;
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |source| StoreError::Io { path, source }
    };
    fs::create_dir_all(dir).map_err(io(dir))?;
    let temporary = schema.temporary(dir);
    let written =
        write_file(&temporary, text.as_bytes()).and_then(|()| fs::rename(&temporary, &path));
    if let Err(source) = written {
        // The old file is untouched; leave no partial temporary file behind.
        let _ = fs::remove_file(&temporary);
        return Err(io(&path)(source));
    }
    Ok(())
}

fn write_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// The file text of `value`: `header`, then `version`, then the value's keys.
pub fn write<T: Serialize>(schema: &Schema, header: &str, value: &T) -> Result<String, String> {
    let body = toml::to_string(value).map_err(|e| e.to_string())?;
    Ok(format!("{header}version = {}\n{body}", schema.version))
}

/// Reads `text` (the contents of `path`, which only labels errors) of any readable version.
pub fn parse<T: DeserializeOwned>(
    path: &Path,
    text: &str,
    schema: &Schema,
) -> Result<T, StoreError> {
    let at = |span: Option<Range<usize>>, key: Option<&str>, reason: &str| {
        located(path, text, span, key, reason)
    };
    let mut root = DeTable::parse(text).map_err(|e| at(e.span(), None, e.message()))?;
    let Some(version) = root.get_mut().remove("version") else {
        return Err(at(
            None,
            Some("version"),
            "missing; the file's format version comes first",
        ));
    };
    let version_span = version.span();
    let found = match version.get_ref() {
        DeValue::Integer(integer) if integer.radix() == 10 => {
            integer.as_str().replace('_', "").parse::<i64>().ok()
        }
        _ => None,
    }
    .ok_or_else(|| {
        at(
            Some(version_span.clone()),
            Some("version"),
            "not a whole number",
        )
    })?;
    let current = i64::from(schema.version);
    if found == current {
        return T::deserialize(Deserializer::from(root))
            .map_err(|e| at(e.span(), None, e.message()));
    }
    if !(i64::from(schema.oldest())..current).contains(&found) {
        let reason = format!(
            "version {found} cannot be read; this build reads versions {} to {current}",
            schema.oldest()
        );
        return Err(at(Some(version_span), Some("version"), &reason));
    }
    // An older file: the steps work on a plain table, which has no spans, so an error after
    // migration points at the version line.
    let migrated = |key: &str, reason: &str| {
        at(
            Some(version_span.clone()),
            Some(key),
            &format!("after migration from version {found}: {reason}"),
        )
    };
    let mut table: toml::Table =
        toml::from_str(text).map_err(|e| at(e.span(), None, e.message()))?;
    table.remove("version");
    for from in found..current {
        let step = schema
            .steps
            .iter()
            .find(|step| i64::from(step.from) == from)
            .ok_or_else(|| migrated("version", &format!("no step from version {from}")))?;
        (step.apply)(&mut table).map_err(|invalid| migrated(&invalid.key, &invalid.reason))?;
    }
    T::deserialize(toml::Value::Table(table)).map_err(|e| {
        let key = quoted_key(e.message()).unwrap_or("-");
        migrated(key, e.message().trim())
    })
}

/// A format error at `span` of `text`: its 1-based line and the key there, qualified by the
/// `[table]` it sits in. A key named in `reason` (unknown or missing field) wins over the line's.
fn located(
    path: &Path,
    text: &str,
    span: Option<Range<usize>>,
    key: Option<&str>,
    reason: &str,
) -> StoreError {
    let start = span.map_or(0, |span| span.start.min(text.len()));
    let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let line_number = text[..start].matches('\n').count() + 1;
    let line_text = text[line_start..].lines().next().unwrap_or("").trim();
    let section = text[..line_start]
        .lines()
        .chain(std::iter::once(line_text))
        .filter_map(table_name)
        .next_back()
        .unwrap_or("");
    let name = key
        .or_else(|| quoted_key(reason))
        .or_else(|| line_text.split_once('=').map(|(left, _)| left.trim()))
        .unwrap_or("-");
    let key = if section.is_empty() || name == "version" || name == "-" {
        name.to_string()
    } else {
        format!("{section}.{name}")
    };
    StoreError::Format {
        path: path.to_path_buf(),
        line: line_number,
        key,
        reason: reason.trim().to_string(),
    }
}

/// `name` of a `[name]` header line.
fn table_name(line: &str) -> Option<&str> {
    let line = line.trim();
    let inner = line.strip_prefix('[')?.split(']').next()?;
    Some(inner.trim_matches(['[', ' ']))
}

/// The field a serde message names: "unknown field `x`", "missing field `x`".
fn quoted_key(message: &str) -> Option<&str> {
    let rest = message
        .strip_prefix("unknown field `")
        .or_else(|| message.strip_prefix("missing field `"))?;
    rest.split('`').next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    /// A sample format at version 2; version 1 named `width` `size`.
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Sample {
        width: u32,
        label: String,
    }

    fn rename_size(table: &mut toml::Table) -> Result<(), Invalid> {
        let size = table.remove("size").ok_or_else(|| Invalid {
            key: "size".to_string(),
            reason: "missing".to_string(),
        })?;
        table.insert("width".to_string(), size);
        Ok(())
    }

    const SAMPLE: Schema = Schema {
        file: "sample.toml",
        version: 2,
        steps: &[Step {
            from: 1,
            apply: rename_size,
        }],
    };

    /// A fresh, empty folder under the system temporary folder.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("prototype-store-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch folder");
        dir
    }

    fn read(text: &str) -> Result<Sample, StoreError> {
        parse(Path::new("sample.toml"), text, &SAMPLE)
    }

    fn format_error(text: &str) -> (usize, String, String) {
        match read(text) {
            Err(StoreError::Format {
                line, key, reason, ..
            }) => (line, key, reason),
            other => panic!("expected a format error, got {other:?}"),
        }
    }

    #[test]
    fn current_version_reads() {
        let sample = read("version = 2\nwidth = 4\nlabel = \"a\"\n").expect("reads");
        assert_eq!(
            sample,
            Sample {
                width: 4,
                label: "a".to_string()
            }
        );
    }

    #[test]
    fn missing_or_unknown_version_is_refused() {
        let (line, key, _) = format_error("width = 4\nlabel = \"a\"\n");
        assert_eq!((line, key.as_str()), (1, "version"));
        for version in ["0", "3", "-1"] {
            let (line, key, reason) = format_error(&format!(
                "# note\nversion = {version}\nwidth = 4\nlabel = \"a\"\n"
            ));
            assert_eq!((line, key.as_str()), (2, "version"));
            assert!(reason.contains("versions 1 to 2"), "{reason}");
        }
        let (_, key, _) = format_error("version = \"2\"\nwidth = 4\nlabel = \"a\"\n");
        assert_eq!(key, "version");
    }

    #[test]
    fn unknown_and_missing_fields_are_refused_with_their_line() {
        let (line, key, _) = format_error("version = 2\nwidth = 4\nlabel = \"a\"\nextra = 1\n");
        assert_eq!((line, key.as_str()), (4, "extra"));
        let (_, key, _) = format_error("version = 2\nwidth = 4\n");
        assert_eq!(key, "label");
        let (line, key, _) = format_error("version = 2\nwidth = -4\nlabel = \"a\"\n");
        assert_eq!((line, key.as_str()), (2, "width"));
        let (line, _, _) = format_error("version = 2\nwidth = \n");
        assert_eq!(line, 2);
    }

    #[test]
    fn older_version_migrates_through_its_steps() {
        let sample = read("version = 1\nsize = 7\nlabel = \"b\"\n").expect("version 1 migrates");
        assert_eq!(sample.width, 7);
        // A step's own refusal and a strict read after migration both report the version line.
        let (line, key, reason) = format_error("\nversion = 1\nlabel = \"b\"\n");
        assert_eq!((line, key.as_str()), (2, "size"));
        assert!(
            reason.starts_with("after migration from version 1"),
            "{reason}"
        );
        let (_, key, _) = format_error("version = 1\nsize = 7\nlabel = \"b\"\nextra = 1\n");
        assert_eq!(key, "extra");
    }

    #[test]
    fn save_then_load_round_trips_and_leaves_no_temporary_file() {
        let dir = scratch("round");
        let sample = Sample {
            width: 3,
            label: "c".to_string(),
        };
        assert!(matches!(load::<Sample>(&dir, &SAMPLE), Ok(None)));
        save(&dir, &SAMPLE, "# sample\n", &sample).expect("saves");
        assert_eq!(load::<Sample>(&dir, &SAMPLE).expect("loads"), Some(sample));
        let names: Vec<_> = fs::read_dir(&dir)
            .expect("lists")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(names, ["sample.toml"]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn failed_write_keeps_the_old_file_whole() {
        let dir = scratch("fail");
        let old = Sample {
            width: 1,
            label: "old".to_string(),
        };
        save(&dir, &SAMPLE, "", &old).expect("saves the old file");
        let before = fs::read(SAMPLE.path(&dir)).expect("old bytes");
        // A folder at the temporary name makes the write fail before the rename.
        fs::create_dir(SAMPLE.temporary(&dir)).expect("blocking folder");
        let new = Sample {
            width: 2,
            label: "new".to_string(),
        };
        assert!(matches!(
            save(&dir, &SAMPLE, "", &new),
            Err(StoreError::Io { .. })
        ));
        assert_eq!(fs::read(SAMPLE.path(&dir)).expect("old bytes"), before);
        assert_eq!(load::<Sample>(&dir, &SAMPLE).expect("loads"), Some(old));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn failed_first_write_makes_no_file() {
        let dir = scratch("first");
        fs::create_dir(SAMPLE.temporary(&dir)).expect("blocking folder");
        let sample = Sample {
            width: 1,
            label: "e".to_string(),
        };
        assert!(save(&dir, &SAMPLE, "", &sample).is_err());
        assert!(!SAMPLE.path(&dir).exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unreadable_file_is_refused_and_never_overwritten() {
        let dir = scratch("unreadable");
        let path = SAMPLE.path(&dir);
        let sample = Sample {
            width: 2,
            label: "f".to_string(),
        };
        for bad in [
            &b"version = 9\nwidth = 1\nlabel = \"x\"\n"[..],
            b"not toml at all",
            b"version = 2\nwidth = 1\nlabel = \"\xff\"\n",
        ] {
            fs::write(&path, bad).expect("writes the bad file");
            assert!(load::<Sample>(&dir, &SAMPLE).is_err());
            assert!(matches!(
                save(&dir, &SAMPLE, "", &sample),
                Err(StoreError::Unreadable { .. })
            ));
            assert_eq!(fs::read(&path).expect("still there"), bad);
            assert!(!SAMPLE.temporary(&dir).exists());
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
