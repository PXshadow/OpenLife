//! ObjectCounts autosave share (OBJECTCOUNTS-LIVE / object_counts_share).
//!
//! Haxe: `WorldMap.write` when `TraceCountObjectsToDisk` → `ObjectCounts{N}.txt`
//! (sibling of `writeFoodStatistics` / FoodStats).
//!
//! Pure line format lives in [`crate::long_term`]; this module is the outer
//! Arc mirror used by ol-server autosave/shutdown (same pattern as WorldFoodShare).

use crate::long_term::{write_object_counts, LongTermState};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

/// Cloneable world object-census maps for outer autosave (OBJECTCOUNTS-LIVE).
///
/// Haxe: `WorldMap.currentObjectsCount` / `originalObjectsCount`.
// Haxe: WorldMap.write L806–809
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectCountsSnapshot {
    pub current_counts: HashMap<i32, i32>,
    pub original_counts: HashMap<i32, i32>,
    /// True after first census seed / load.
    pub counts_ready: bool,
}

impl ObjectCountsSnapshot {
    pub fn new() -> Self {
        Self::default()
    }

    /// Capture census maps from live long-term state.
    // Haxe: WorldMap.write TraceCountObjectsToDisk reads currentObjectsCount
    pub fn from_long_term(lt: &LongTermState) -> Self {
        Self {
            current_counts: lt.current_counts.clone(),
            original_counts: lt.original_counts.clone(),
            counts_ready: lt.counts_ready,
        }
    }

    /// Write `ObjectCounts.txt` (or path) from this snapshot.
    // Haxe: WorldMap.writeToDiskHelper TraceCountObjectsToDisk L797–812
    pub fn write_object_counts<F>(&self, path: impl AsRef<Path>, desc_of: F) -> Result<(), String>
    where
        F: FnMut(i32) -> String,
    {
        write_object_counts(&self.current_counts, &self.original_counts, path, desc_of)
    }

    pub fn len_current(&self) -> usize {
        self.current_counts.len()
    }
}

/// Outer autosave / shutdown share of object census (OBJECTCOUNTS-LIVE).
// Haxe: WorldMap.write → ObjectCounts{N}.txt when TraceCountObjectsToDisk
pub type ObjectCountsShare = Arc<RwLock<ObjectCountsSnapshot>>;

/// One minute-sample of world object census for `/object-counts`.
#[derive(Debug, Clone)]
pub struct ObjectCountSample {
    pub wall_unix_ms: u64,
    pub total: i64,
    pub unique: u32,
    pub top: Vec<ObjectCountTop>,
}

/// One of the most common object ids in a sample.
#[derive(Debug, Clone)]
pub struct ObjectCountTop {
    pub id: i32,
    pub current: i32,
    pub original: i32,
    pub name: String,
}

pub const OBJECT_COUNT_SERIES_MAX: usize = 4096;
pub const OBJECT_COUNT_TOP_N: usize = 10;
pub const OBJECT_COUNT_LIST_DEFAULT: usize = 100;
/// Minute census for `/object-counts` (first sample is immediate once counts are ready).
pub const OBJECT_COUNT_SAMPLE_INTERVAL_MS: u64 = 60_000;
pub const MS_DAY: u64 = 86_400_000;
pub const MS_WEEK: u64 = 7 * MS_DAY;
pub const MS_MONTH: u64 = 30 * MS_DAY;

/// Record a dashboard sample once the world census exists, then once a minute.
pub fn should_record_object_count_sample(
    counts_ready: bool,
    last_recorded_ms: u64,
    now_ms: u64,
    interval_ms: u64,
) -> bool {
    if !counts_ready {
        return false;
    }
    if last_recorded_ms == 0 {
        return true;
    }
    now_ms.saturating_sub(last_recorded_ms) >= interval_ms.max(1)
}

/// Copy live long-term census into the autosave / dashboard share.
pub fn mirror_object_counts_share(lt: &LongTermState, share: &Option<ObjectCountsShare>) {
    let Some(share) = share else {
        return;
    };
    if let Ok(mut g) = share.write() {
        *g = ObjectCountsSnapshot::from_long_term(lt);
    }
}

impl ObjectCountsSnapshot {
    /// Minute sample: total, unique types, every counted id (for search / % change).
    pub fn minute_sample<F>(&self, wall_unix_ms: u64, mut name_of: F) -> ObjectCountSample
    where
        F: FnMut(i32) -> String,
    {
        let mut total: i64 = 0;
        let mut rows: Vec<(i32, i32, i32)> = Vec::with_capacity(self.current_counts.len());
        for (&id, &cur) in &self.current_counts {
            total += i64::from(cur.max(0));
            let orig = self.original_counts.get(&id).copied().unwrap_or(0);
            rows.push((id, cur, orig));
        }
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let top = rows
            .into_iter()
            .map(|(id, current, original)| ObjectCountTop {
                id,
                current,
                original,
                name: name_of(id),
            })
            .collect();
        ObjectCountSample {
            wall_unix_ms,
            total,
            unique: self.current_counts.len() as u32,
            top,
        }
    }
}

/// True when this row stored every type (`unique` matches listed ids). Old top-N journal lines fail this.
pub fn is_complete_object_count_sample(s: &ObjectCountSample) -> bool {
    s.total > 0 && s.unique > 0 && s.top.len() as u32 == s.unique
}

/// Last **complete** sample at or before `target_ms`. If none, the oldest complete sample.
pub fn nearest_sample_at_or_before(
    samples: &[ObjectCountSample],
    target_ms: u64,
) -> Option<&ObjectCountSample> {
    if samples.is_empty() {
        return None;
    }
    let mut best: Option<&ObjectCountSample> = None;
    let mut oldest_complete: Option<&ObjectCountSample> = None;
    for s in samples {
        if !is_complete_object_count_sample(s) {
            continue;
        }
        if oldest_complete.is_none() {
            oldest_complete = Some(s);
        }
        if s.wall_unix_ms <= target_ms {
            best = Some(s);
        } else if best.is_some() {
            break;
        }
    }
    best.or(oldest_complete)
}

/// Percent change from `then` to `now`.
///
/// A missing baseline is **not** a 100% bump — that was a journal artifact
/// (top-12 rows treated absent ids as zero). Callers pass `None` when the
/// lookback sample does not list the id.
pub fn pct_change(now: i64, then: i64) -> f64 {
    if then == 0 {
        if now == 0 {
            0.0
        } else {
            100.0
        }
    } else {
        (now - then) as f64 / (then.abs() as f64) * 100.0
    }
}

pub fn pct_change_opt(now: i64, then: Option<i64>) -> Option<f64> {
    then.map(|t| pct_change(now, t))
}

/// Count of `id` when the sample listed it. Incomplete top-N rows return None for other ids.
pub fn sample_count_opt(sample: &ObjectCountSample, id: i32) -> Option<i32> {
    sample.top.iter().find(|t| t.id == id).map(|t| t.current)
}

/// Current count of `id` in a sample (0 if that id was not stored).
pub fn sample_count_of(sample: &ObjectCountSample, id: i32) -> i32 {
    sample_count_opt(sample, id).unwrap_or(0)
}

/// Keep minute resolution for a day, 15-minute after that for a week, hourly for a month.
pub fn compact_object_count_series(samples: &mut Vec<ObjectCountSample>, now_ms: u64) {
    if samples.len() < 3 {
        return;
    }
    let mut keep: Vec<ObjectCountSample> = Vec::with_capacity(samples.len());
    for s in samples.drain(..) {
        let age = now_ms.saturating_sub(s.wall_unix_ms);
        let minute = (s.wall_unix_ms / 60_000) % 60;
        let keep_it = if age <= MS_DAY {
            true
        } else if age <= MS_WEEK {
            minute % 15 == 0
        } else if age <= MS_MONTH {
            minute == 0
        } else {
            false
        };
        if keep_it {
            keep.push(s);
        }
    }
    if keep.len() > OBJECT_COUNT_SERIES_MAX {
        let n = keep.len() - OBJECT_COUNT_SERIES_MAX;
        keep.drain(0..n);
    }
    *samples = keep;
}

/// Ring of minute samples (about 24 hours at one per minute).
#[derive(Debug, Clone, Default)]
pub struct ObjectCountSeries {
    pub samples: Vec<ObjectCountSample>,
}

impl ObjectCountSeries {
    pub fn push(&mut self, sample: ObjectCountSample) {
        let now = sample.wall_unix_ms;
        self.samples.push(sample);
        compact_object_count_series(&mut self.samples, now);
    }
}

pub fn format_object_count_journal_line(s: &ObjectCountSample) -> String {
    let mut line = format!("{} {} {}", s.wall_unix_ms, s.total, s.unique);
    for t in &s.top {
        line.push_str(&format!(" {} {}", t.id, t.current));
    }
    line
}

/// Parse `{wall} {total} {unique} [id current]*`. Names are filled on live samples only.
pub fn parse_object_count_journal_line(line: &str) -> Option<ObjectCountSample> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let mut it = line.split_whitespace();
    let wall_unix_ms = it.next()?.parse().ok()?;
    let total = it.next()?.parse().ok()?;
    let unique = it.next()?.parse().ok()?;
    let mut top = Vec::new();
    loop {
        let Some(id_s) = it.next() else {
            break;
        };
        let cur_s = it.next()?;
        top.push(ObjectCountTop {
            id: id_s.parse().ok()?,
            current: cur_s.parse().ok()?,
            original: 0,
            name: String::new(),
        });
    }
    Some(ObjectCountSample {
        wall_unix_ms,
        total,
        unique,
        top,
    })
}

/// Load newest non-empty journal samples (skip all-zero rows from before the census existed).
pub fn load_object_count_journal(path: &std::path::Path, max_samples: usize) -> Vec<ObjectCountSample> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut samples = Vec::new();
    for line in text.lines() {
        let Some(s) = parse_object_count_journal_line(line) else {
            continue;
        };
        if !is_complete_object_count_sample(&s) {
            continue;
        }
        samples.push(s);
    }
    let max = max_samples.max(1);
    if samples.len() > max {
        samples.drain(0..samples.len() - max);
    }
    samples
}

/// Drop zero and top-N-only journal rows (those made every type look like a 100% bump).
pub fn rewrite_object_count_journal_complete(path: &Path) -> std::io::Result<usize> {
    let samples = load_object_count_journal(path, OBJECT_COUNT_SERIES_MAX);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut body = String::new();
    for s in &samples {
        body.push_str(&format_object_count_journal_line(s));
        body.push('\n');
    }
    std::fs::write(path, body)?;
    Ok(samples.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::long_term::{format_object_counts_text, LongTermState};

    #[test]
    fn object_counts_snapshot_from_long_term() {
        let mut state = LongTermState::default();
        state.current_counts.insert(33, 4);
        state.original_counts.insert(33, 10);
        state.counts_ready = true;
        let snap = ObjectCountsSnapshot::from_long_term(&state);
        assert_eq!(snap.current_counts.get(&33), Some(&4));
        assert_eq!(snap.original_counts.get(&33), Some(&10));
        assert!(snap.counts_ready);
        assert_eq!(snap.len_current(), 1);
        let text = format_object_counts_text(&snap.current_counts, &snap.original_counts, |id| {
            if id == 33 {
                "Gooseberry".into()
            } else {
                String::new()
            }
        });
        assert!(
            text.contains("Count object: [33] Gooseberry: 4 original: 10"),
            "text={text}"
        );
        let sample = snap.minute_sample(1_000, |id| {
            if id == 33 {
                "Gooseberry".into()
            } else {
                String::new()
            }
        });
        assert_eq!(sample.total, 4);
        assert_eq!(sample.unique, 1);
        assert_eq!(sample.top[0].id, 33);
        assert_eq!(sample.top[0].name, "Gooseberry");
        let mut series = ObjectCountSeries::default();
        series.push(sample);
        assert_eq!(series.samples.len(), 1);
    }

    #[test]
    fn object_counts_share_roundtrip_lock() {
        let share: ObjectCountsShare = Arc::new(RwLock::new(ObjectCountsSnapshot::new()));
        {
            let mut g = share.write().unwrap();
            g.current_counts.insert(1, 2);
            g.original_counts.insert(1, 3);
            g.counts_ready = true;
        }
        let snap = share.read().unwrap().clone();
        assert_eq!(snap.current_counts.get(&1), Some(&2));
        assert_eq!(snap.original_counts.get(&1), Some(&3));
    }

    #[test]
    fn from_long_term_before_seed_empty_after_ensure_non_empty() {
        use ol_content::{ContentDb, ObjectDef};
        use ol_world::{ComplexObject, World};

        let mut db = ContentDb::default();
        db.objects.insert(33, ObjectDef::empty(33));
        db.objects.insert(391, ObjectDef::empty(391));
        let mut world = World::new(3, 3, false);
        let mut h = ComplexObject::new_simple(391);
        h.contained = vec![33];
        world.set_object_complex(0, 0, h);

        let mut lt = LongTermState::default();
        let before = ObjectCountsSnapshot::from_long_term(&lt);
        assert!(!before.counts_ready);
        assert!(before.current_counts.is_empty());

        lt.ensure_counts_for_dump(&world, &db);
        let after = ObjectCountsSnapshot::from_long_term(&lt);
        assert!(after.counts_ready);
        assert_eq!(after.current_counts.get(&391), Some(&1));
        assert_eq!(after.current_counts.get(&33), Some(&1), "nest in dump");
        let text =
            format_object_counts_text(
                &after.current_counts,
                &after.original_counts,
                |id| match id {
                    33 => "Berry".into(),
                    391 => "Basket".into(),
                    _ => String::new(),
                },
            );
        assert!(text.contains("[391]"), "text={text}");
        assert!(text.contains("[33]"), "text={text}");
    }

    #[test]
    fn should_record_skips_until_census_then_first_and_interval() {
        assert!(!should_record_object_count_sample(false, 0, 1_000, 60_000));
        assert!(should_record_object_count_sample(true, 0, 1_000, 60_000));
        assert!(!should_record_object_count_sample(
            true, 1_000, 30_000, 60_000
        ));
        assert!(should_record_object_count_sample(
            true, 1_000, 61_000, 60_000
        ));
    }

    #[test]
    fn mirror_object_counts_share_copies_ready_census() {
        let mut lt = LongTermState::default();
        lt.current_counts.insert(33, 4);
        lt.original_counts.insert(33, 10);
        lt.counts_ready = true;
        let share: ObjectCountsShare = Arc::new(RwLock::new(ObjectCountsSnapshot::new()));
        mirror_object_counts_share(&lt, &None);
        assert!(share.read().unwrap().current_counts.is_empty());
        mirror_object_counts_share(&lt, &Some(Arc::clone(&share)));
        let snap = share.read().unwrap().clone();
        assert!(snap.counts_ready);
        assert_eq!(snap.current_counts.get(&33), Some(&4));
        assert_eq!(snap.original_counts.get(&33), Some(&10));
        let sample = snap.minute_sample(5_000, |_| "Gooseberry".into());
        assert_eq!(sample.total, 4);
        assert_eq!(sample.unique, 1);
        assert_eq!(sample.top[0].id, 33);
    }

    #[test]
    fn parse_and_load_object_count_journal_skips_zeros() {
        let line = format_object_count_journal_line(&ObjectCountSample {
            wall_unix_ms: 9,
            total: 12,
            unique: 2,
            top: vec![ObjectCountTop {
                id: 33,
                current: 10,
                original: 8,
                name: "Gooseberry".into(),
            }],
        });
        let parsed = parse_object_count_journal_line(&line).expect("parse");
        assert_eq!(parsed.total, 12);
        assert_eq!(parsed.unique, 2);
        assert_eq!(parsed.top[0].id, 33);
        assert_eq!(parsed.top[0].current, 10);
        let dir = std::env::temp_dir().join(format!(
            "ol_oc_j_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("object_counts.journal");
        std::fs::write(
            &path,
            "1 0 0\n2 12 1 33 10\n3 0 0\n4 20 1 33 11\n5 50 103 161 1 32 2\n",
        )
        .unwrap();
        let loaded = load_object_count_journal(&path, 10);
        assert_eq!(loaded.len(), 2, "zero and top-N-only rows must not load");
        assert_eq!(loaded[0].total, 12);
        assert_eq!(loaded[1].total, 20);
        let n = rewrite_object_count_journal_complete(&path).unwrap();
        assert_eq!(n, 2);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains(" 0 0\n"));
        assert!(!text.contains("50 103"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pct_change_and_nearest_sample() {
        assert!((pct_change(10, 5) - 100.0).abs() < 1e-9);
        assert!((pct_change(5, 10) + 50.0).abs() < 1e-9);
        assert_eq!(pct_change(0, 0), 0.0);
        assert_eq!(pct_change(8, 0), 100.0);
        let samples = vec![
            ObjectCountSample {
                wall_unix_ms: 1_000,
                total: 10,
                unique: 1,
                top: vec![ObjectCountTop {
                    id: 33,
                    current: 10,
                    original: 10,
                    name: "Gooseberry".into(),
                }],
            },
            ObjectCountSample {
                wall_unix_ms: 2_000,
                total: 12,
                unique: 1,
                top: vec![ObjectCountTop {
                    id: 33,
                    current: 12,
                    original: 10,
                    name: "Gooseberry".into(),
                }],
            },
            ObjectCountSample {
                wall_unix_ms: 3_000,
                total: 8,
                unique: 1,
                top: vec![ObjectCountTop {
                    id: 33,
                    current: 8,
                    original: 10,
                    name: "Gooseberry".into(),
                }],
            },
        ];
        assert_eq!(
            nearest_sample_at_or_before(&samples, 2_500)
                .unwrap()
                .wall_unix_ms,
            2_000
        );
        assert_eq!(
            nearest_sample_at_or_before(&samples, 0)
                .unwrap()
                .wall_unix_ms,
            1_000,
            "no sample that old → oldest available"
        );
        assert_eq!(
            nearest_sample_at_or_before(&samples, 9_000)
                .unwrap()
                .wall_unix_ms,
            3_000
        );
        assert_eq!(sample_count_of(&samples[2], 33), 8);
        assert_eq!(sample_count_of(&samples[2], 99), 0);
        let now = samples[2].total;
        let then = nearest_sample_at_or_before(&samples, 3_000u64.saturating_sub(MS_DAY))
            .unwrap()
            .total;
        assert_eq!(then, 10);
        assert!((pct_change(now, then) + 20.0).abs() < 1e-9);
        // Top-12 row (unique=103, only 1 id listed) must not become a 100% baseline.
        let mixed = vec![
            ObjectCountSample {
                wall_unix_ms: 10,
                total: 50_000,
                unique: 103,
                top: vec![ObjectCountTop {
                    id: 161,
                    current: 5000,
                    original: 5000,
                    name: "Rabbit Hole".into(),
                }],
            },
            samples[2].clone(),
        ];
        assert!(
            nearest_sample_at_or_before(&mixed, 10).is_some()
                && is_complete_object_count_sample(nearest_sample_at_or_before(&mixed, 10).unwrap())
        );
        assert_eq!(sample_count_opt(&mixed[0], 33), None);
        assert_eq!(
            pct_change_opt(8, sample_count_opt(&mixed[0], 33).map(i64::from)),
            None
        );
    }

    #[test]
    fn compact_keeps_day_minutes_and_drops_old() {
        let mut samples = Vec::new();
        let now = MS_MONTH + MS_DAY;
        for i in 0..4 {
            samples.push(ObjectCountSample {
                wall_unix_ms: now - MS_MONTH - 1_000 - i,
                total: 1,
                unique: 1,
                top: vec![],
            });
        }
        for m in 0..4u64 {
            samples.push(ObjectCountSample {
                wall_unix_ms: now - 3 * MS_DAY + m * 60_000,
                total: 2,
                unique: 1,
                top: vec![],
            });
        }
        samples.push(ObjectCountSample {
            wall_unix_ms: now,
            total: 3,
            unique: 1,
            top: vec![],
        });
        compact_object_count_series(&mut samples, now);
        assert!(
            samples.iter().all(|s| now.saturating_sub(s.wall_unix_ms) <= MS_MONTH),
            "older than a month dropped"
        );
        assert!(samples.iter().any(|s| s.total == 3));
    }
}
