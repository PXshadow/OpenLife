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
    pub fn write_object_counts<F>(
        &self,
        path: impl AsRef<Path>,
        desc_of: F,
    ) -> Result<(), String>
    where
        F: FnMut(i32) -> String,
    {
        write_object_counts(
            &self.current_counts,
            &self.original_counts,
            path,
            desc_of,
        )
    }

    pub fn len_current(&self) -> usize {
        self.current_counts.len()
    }
}

/// Outer autosave / shutdown share of object census (OBJECTCOUNTS-LIVE).
// Haxe: WorldMap.write → ObjectCounts{N}.txt when TraceCountObjectsToDisk
pub type ObjectCountsShare = Arc<RwLock<ObjectCountsSnapshot>>;

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
        let text = format_object_counts_text(
            &snap.current_counts,
            &snap.original_counts,
            |id| {
                if id == 33 {
                    "Gooseberry".into()
                } else {
                    String::new()
                }
            },
        );
        assert!(
            text.contains("Count object: [33] Gooseberry: 4 original: 10"),
            "text={text}"
        );
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
        let text = format_object_counts_text(
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
}
