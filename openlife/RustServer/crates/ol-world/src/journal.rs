//! Append-only world change journal (no SQL).
//!
//! Text lines, one entry per line:
//! ```text
//! x y object_id tick
//! ```
//!
//! Default path: [`DEFAULT_JOURNAL_PATH`] (`SaveFiles/world.journal`).
//! Used by sim to record DROP/USE ground places without hooking `set_object`.
//!
//! When the live journal exceeds [`DEFAULT_JOURNAL_MAX_BYTES`], it is rotated
//! to `{path}.1.bak` before the next append (simple single-slot backup).

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Default on-disk journal under the save directory.
pub const DEFAULT_JOURNAL_PATH: &str = "SaveFiles/world.journal";

/// Rotate when the journal file grows past this size (8 MiB).
pub const DEFAULT_JOURNAL_MAX_BYTES: u64 = 8 * 1024 * 1024;

/// One recorded world tile change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalEntry {
    pub x: i32,
    pub y: i32,
    pub object_id: i32,
    pub tick: u64,
}

impl JournalEntry {
    pub fn new(x: i32, y: i32, object_id: i32, tick: u64) -> Self {
        Self {
            x,
            y,
            object_id,
            tick,
        }
    }

    /// Serialize to a single journal line (no trailing newline).
    pub fn to_line(&self) -> String {
        format!("{} {} {} {}", self.x, self.y, self.object_id, self.tick)
    }

    /// Parse a journal line. Returns `None` for blank or malformed lines.
    pub fn parse_line(line: &str) -> Option<Self> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let mut parts = line.split_whitespace();
        let x: i32 = parts.next()?.parse().ok()?;
        let y: i32 = parts.next()?.parse().ok()?;
        let object_id: i32 = parts.next()?.parse().ok()?;
        let tick: u64 = parts.next()?.parse().ok()?;
        // Reject trailing garbage (extra tokens).
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            x,
            y,
            object_id,
            tick,
        })
    }
}

/// Append-only journal of tile object changes.
///
/// Sim holds this behind `Arc<Mutex<WorldJournal>>` and records DROP/USE places.
#[derive(Debug)]
pub struct WorldJournal {
    path: PathBuf,
    /// Max live file size before rotate-to-`.1.bak` (see [`DEFAULT_JOURNAL_MAX_BYTES`]).
    max_bytes: u64,
}

impl WorldJournal {
    /// Create a journal bound to `path` (parent dirs created on first append).
    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            max_bytes: DEFAULT_JOURNAL_MAX_BYTES,
        }
    }

    /// Journal at [`DEFAULT_JOURNAL_PATH`].
    pub fn open_default() -> Self {
        Self::open(DEFAULT_JOURNAL_PATH)
    }

    /// Override rotation threshold (`0` disables rotation).
    pub fn with_max_bytes(mut self, max_bytes: u64) -> Self {
        self.max_bytes = max_bytes;
        self
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Backup path used on rotate: `{path}.1.bak`.
    pub fn backup_path(&self) -> PathBuf {
        let mut s = self.path.as_os_str().to_os_string();
        s.push(".1.bak");
        PathBuf::from(s)
    }

    /// If the live journal exceeds `max_bytes`, move it to `.1.bak` (replace previous).
    pub fn rotate_if_needed(&self) -> Result<(), String> {
        if self.max_bytes == 0 {
            return Ok(());
        }
        if !self.path.exists() {
            return Ok(());
        }
        let meta = fs::metadata(&self.path).map_err(|e| e.to_string())?;
        if meta.len() <= self.max_bytes {
            return Ok(());
        }
        let bak = self.backup_path();
        if bak.exists() {
            fs::remove_file(&bak).map_err(|e| e.to_string())?;
        }
        fs::rename(&self.path, &bak).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Append one entry and flush (crash-safe enough for a change log).
    ///
    /// Rotates the live file to `.1.bak` when over the size threshold first.
    pub fn append(&mut self, entry: JournalEntry) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }
        self.rotate_if_needed()?;
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| e.to_string())?;
        writeln!(f, "{}", entry.to_line()).map_err(|e| e.to_string())?;
        f.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Load all parseable entries in file order.
    pub fn load_all(&self) -> Result<Vec<JournalEntry>, String> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let f = File::open(&self.path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(f);
        let mut out = Vec::new();
        for line in reader.lines() {
            let line = line.map_err(|e| e.to_string())?;
            if let Some(e) = JournalEntry::parse_line(&line) {
                out.push(e);
            }
        }
        Ok(out)
    }

    /// Load the last `n` entries (for future replay / catch-up). `n == 0` → empty.
    pub fn load_last_n(&self, n: usize) -> Result<Vec<JournalEntry>, String> {
        if n == 0 {
            return Ok(Vec::new());
        }
        let all = self.load_all()?;
        if all.len() <= n {
            return Ok(all);
        }
        Ok(all[all.len() - n..].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_journal_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("ol_world_journal_{label}_{nanos}.journal"))
    }

    #[test]
    fn parse_line_roundtrip() {
        let e = JournalEntry::new(10, -3, 391, 42);
        let line = e.to_line();
        assert_eq!(line, "10 -3 391 42");
        assert_eq!(JournalEntry::parse_line(&line), Some(e));
        assert_eq!(JournalEntry::parse_line("  1  2  3  4  "), Some(JournalEntry::new(1, 2, 3, 4)));
        assert_eq!(JournalEntry::parse_line(""), None);
        assert_eq!(JournalEntry::parse_line("# comment"), None);
        assert_eq!(JournalEntry::parse_line("1 2 3"), None);
        assert_eq!(JournalEntry::parse_line("a b c d"), None);
    }

    #[test]
    fn append_and_read_last_n() {
        let path = temp_journal_path("append");
        let _ = fs::remove_file(&path);

        let mut j = WorldJournal::open(&path);
        j.append(JournalEntry::new(0, 0, 33, 1)).unwrap();
        j.append(JournalEntry::new(1, 2, 0, 2)).unwrap();
        j.append(JournalEntry::new(5, 5, 100, 3)).unwrap();

        let all = j.load_all().unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0], JournalEntry::new(0, 0, 33, 1));
        assert_eq!(all[2].object_id, 100);

        let last2 = j.load_last_n(2).unwrap();
        assert_eq!(last2.len(), 2);
        assert_eq!(last2[0], JournalEntry::new(1, 2, 0, 2));
        assert_eq!(last2[1], JournalEntry::new(5, 5, 100, 3));

        assert!(j.load_last_n(0).unwrap().is_empty());
        assert_eq!(j.load_last_n(99).unwrap().len(), 3);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn missing_file_loads_empty() {
        let path = temp_journal_path("missing");
        let _ = fs::remove_file(&path);
        let j = WorldJournal::open(&path);
        assert!(j.load_all().unwrap().is_empty());
        assert!(j.load_last_n(5).unwrap().is_empty());
    }

    #[test]
    fn rotates_when_over_max_bytes() {
        let path = temp_journal_path("rotate");
        let bak = {
            let mut s = path.as_os_str().to_os_string();
            s.push(".1.bak");
            PathBuf::from(s)
        };
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&bak);

        // Tiny threshold so a few lines trigger rotate.
        let mut j = WorldJournal::open(&path).with_max_bytes(40);
        j.append(JournalEntry::new(0, 0, 1, 1)).unwrap();
        j.append(JournalEntry::new(1, 1, 2, 2)).unwrap();
        j.append(JournalEntry::new(2, 2, 3, 3)).unwrap();
        // Force: inflate file past threshold then append once more.
        {
            let mut f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .unwrap();
            // Pad so len > 40 without needing many journal lines.
            writeln!(f, "0 0 0 999999").unwrap();
            writeln!(f, "0 0 0 999998").unwrap();
            writeln!(f, "0 0 0 999997").unwrap();
            f.flush().unwrap();
        }
        assert!(fs::metadata(&path).unwrap().len() > 40);
        j.append(JournalEntry::new(9, 9, 9, 9)).unwrap();
        assert!(bak.exists(), "expected .1.bak after rotate");
        // Live journal should only contain the new post-rotate line.
        let live = j.load_all().unwrap();
        assert_eq!(live.len(), 1);
        assert_eq!(live[0], JournalEntry::new(9, 9, 9, 9));

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&bak);
    }

    /// Stress: append 1000 entries, reload all, then force rotate and keep writing.
    #[test]
    fn stress_append_1000_and_rotate() {
        let path = temp_journal_path("stress_1000");
        let bak = {
            let mut s = path.as_os_str().to_os_string();
            s.push(".1.bak");
            PathBuf::from(s)
        };
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&bak);

        // High threshold so first 1000 stay in the live file.
        let mut j = WorldJournal::open(&path).with_max_bytes(10 * 1024 * 1024);
        for i in 0..1000u64 {
            j.append(JournalEntry::new(
                (i % 64) as i32,
                ((i / 64) % 64) as i32,
                (i % 500) as i32,
                i,
            ))
            .unwrap();
        }

        let all = j.load_all().unwrap();
        assert_eq!(all.len(), 1000);
        assert_eq!(all[0], JournalEntry::new(0, 0, 0, 0));
        assert_eq!(all[999].tick, 999);
        assert_eq!(all[999].object_id, 499);
        let last10 = j.load_last_n(10).unwrap();
        assert_eq!(last10.len(), 10);
        assert_eq!(last10[0].tick, 990);
        assert_eq!(last10[9].tick, 999);
        assert!(!bak.exists(), "should not rotate under large max_bytes");
        let bulk_len = fs::metadata(&path).unwrap().len();
        assert!(bulk_len > 1000, "1000 lines should be >1KB");

        // Threshold between one line (~20B) and the 1000-line bulk → first append rotates.
        // Stay large enough that a few post-rotate lines do not re-rotate.
        let mut j = WorldJournal::open(&path).with_max_bytes(200);
        j.append(JournalEntry::new(7, 7, 7, 1000)).unwrap();
        assert!(bak.exists(), "expected rotate when bulk exceeds max_bytes");
        let live = j.load_all().unwrap();
        assert_eq!(live.len(), 1);
        assert_eq!(live[0], JournalEntry::new(7, 7, 7, 1000));
        // Backup holds the previous bulk (parseable entries).
        let bak_j = WorldJournal::open(&bak);
        let bak_entries = bak_j.load_all().unwrap();
        assert_eq!(bak_entries.len(), 1000);
        assert_eq!(bak_entries[0].tick, 0);
        assert_eq!(bak_entries[999].tick, 999);

        // Keep writing after rotate without another rotation.
        j.append(JournalEntry::new(8, 8, 8, 1001)).unwrap();
        j.append(JournalEntry::new(9, 9, 9, 1002)).unwrap();
        let live2 = j.load_all().unwrap();
        assert_eq!(live2.len(), 3);
        assert_eq!(live2[0], JournalEntry::new(7, 7, 7, 1000));
        assert_eq!(live2[1], JournalEntry::new(8, 8, 8, 1001));
        assert_eq!(live2[2], JournalEntry::new(9, 9, 9, 1002));
        // bak still the 1000-entry bulk (not overwritten by tiny post-rotate file).
        assert_eq!(WorldJournal::open(&bak).load_all().unwrap().len(), 1000);

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&bak);
    }
}
