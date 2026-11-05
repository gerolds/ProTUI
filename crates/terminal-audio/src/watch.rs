// ADR: The watcher polls modification time and size instead of OS notifications — no dependency, the same on every platform, cheap for a few hundred files.
// ADR: A change is a watched file added, removed, or with a new modification time or size; it reports the newest modification time so a caller can measure save-to-sound time.
//! Adapted from Runner `crates/runner-data/src/watch.rs`.
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

/// What changed since the last check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Change {
    /// The latest modification time among the files now present; `None` when no file is left.
    pub newest: Option<SystemTime>,
}

/// One file's state: path, modification time and size.
type Stamp = (PathBuf, SystemTime, u64);

/// Polls the files with the given extensions under one folder (recursively) for changes.
pub struct Watcher {
    dir: PathBuf,
    extensions: &'static [&'static str],
    interval: Duration,
    next: Instant,
    stamps: Vec<Stamp>,
}

impl Watcher {
    /// Starts watching `dir` and every folder below it for files ending in one of
    /// `extensions` (without the dot, such as `["toml", "wav"]`), checking at most once per
    /// `interval`. The files as they are now count as seen.
    pub fn new(
        dir: impl AsRef<Path>,
        extensions: &'static [&'static str],
        interval: Duration,
        now: Instant,
    ) -> Self {
        let dir = dir.as_ref().to_path_buf();
        let stamps = stamps(&dir, extensions);
        Self {
            dir,
            extensions,
            interval,
            next: now + interval,
            stamps,
        }
    }

    /// When `interval` has passed since the last check, compares the files with the last check
    /// and returns the change, if any. Before that, returns `None` without touching the disk.
    pub fn poll(&mut self, now: Instant) -> Option<Change> {
        if now < self.next {
            return None;
        }
        self.next = now + self.interval;
        let current = stamps(&self.dir, self.extensions);
        if current == self.stamps {
            return None;
        }
        self.stamps = current;
        Some(Change {
            newest: self.stamps.iter().map(|(_, time, _)| *time).max(),
        })
    }
}

/// The state of every watched file under `dir`, sorted by path. A folder or file that cannot be
/// read is left out; the next load reports it.
fn stamps(dir: &Path, extensions: &[&str]) -> Vec<Stamp> {
    let mut out = Vec::new();
    let mut folders = vec![dir.to_path_buf()];
    while let Some(folder) = folders.pop() {
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                folders.push(path);
            } else if path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| extensions.contains(&ext))
                && let Ok(modified) = meta.modified()
            {
                out.push((path, modified, meta.len()));
            }
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_added_resized_and_removed_files_only_after_the_interval() {
        let dir = std::env::temp_dir().join(format!("terminal-audio-watch-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("a.toml"), "x").unwrap();
        let start = Instant::now();
        let step = Duration::from_millis(250);
        let mut watcher = Watcher::new(&dir, &["toml", "wav"], step, start);
        assert_eq!(watcher.poll(start + step), None, "nothing changed");
        fs::write(dir.join("sub/b.wav"), "y").unwrap();
        fs::write(dir.join("ignored.txt"), "z").unwrap();
        assert_eq!(watcher.poll(start + step), None, "interval not yet passed");
        assert!(watcher.poll(start + step * 2).is_some(), "added file");
        assert_eq!(watcher.poll(start + step * 3), None);
        fs::write(dir.join("ignored.txt"), "zz").unwrap();
        assert_eq!(watcher.poll(start + step * 4), None, "unwatched extension");
        fs::write(dir.join("a.toml"), "longer").unwrap();
        assert!(watcher.poll(start + step * 5).is_some(), "new size");
        fs::remove_file(dir.join("sub/b.wav")).unwrap();
        fs::remove_file(dir.join("a.toml")).unwrap();
        assert_eq!(
            watcher.poll(start + step * 6),
            Some(Change { newest: None }),
            "every watched file removed"
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
