//! Sparse special-object index + simple mutation notes (Haxe special object subset).
//!
//! Tracks tiles with "special" objects for fast scans (animals, graves, gates)
//! without full world iteration each tick.

use std::collections::{HashMap, HashSet};

/// Kind of special tile for indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialKind {
    Gate,
    Grave,
    Container,
    AnimalNest,
    HomeMarker,
    Other,
}

impl SpecialKind {
    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Gate => "gate",
            Self::Grave => "grave",
            Self::Container => "container",
            Self::AnimalNest => "nest",
            Self::HomeMarker => "home",
            Self::Other => "other",
        }
    }

    /// Infer from object name (case-insensitive).
    pub fn from_object_name(name: &str) -> Option<Self> {
        let n = name.to_ascii_lowercase();
        if n.contains("gate") || n.contains("door") {
            Some(Self::Gate)
        } else if n.contains("grave") || n.contains("tomb") {
            Some(Self::Grave)
        } else if n.contains("basket") || n.contains("chest") || n.contains("box") {
            Some(Self::Container)
        } else if n.contains("nest") || n.contains("den") {
            Some(Self::AnimalNest)
        } else {
            None
        }
    }
}

/// Sparse index: kind → set of (x,y).
#[derive(Debug, Default, Clone)]
pub struct SpecialIndex {
    pub by_kind: HashMap<SpecialKind, HashSet<(i32, i32)>>,
    pub by_tile: HashMap<(i32, i32), SpecialKind>,
}

impl SpecialIndex {
    pub fn insert(&mut self, x: i32, y: i32, kind: SpecialKind) {
        if let Some(old) = self.by_tile.insert((x, y), kind) {
            if let Some(set) = self.by_kind.get_mut(&old) {
                set.remove(&(x, y));
            }
        }
        self.by_kind.entry(kind).or_default().insert((x, y));
    }

    pub fn remove(&mut self, x: i32, y: i32) {
        if let Some(kind) = self.by_tile.remove(&(x, y)) {
            if let Some(set) = self.by_kind.get_mut(&kind) {
                set.remove(&(x, y));
            }
        }
    }

    pub fn kind_at(&self, x: i32, y: i32) -> Option<SpecialKind> {
        self.by_tile.get(&(x, y)).copied()
    }

    pub fn count(&self, kind: SpecialKind) -> usize {
        self.by_kind.get(&kind).map(|s| s.len()).unwrap_or(0)
    }

    pub fn total(&self) -> usize {
        self.by_tile.len()
    }

    /// `SPECIAL gates=N graves=N containers=N total=N`
    pub fn format_query(&self) -> String {
        format!(
            "SPECIAL gates={} graves={} containers={} nests={} total={}",
            self.count(SpecialKind::Gate),
            self.count(SpecialKind::Grave),
            self.count(SpecialKind::Container),
            self.count(SpecialKind::AnimalNest),
            self.total()
        )
    }
}

/// Sparse map of tiles with non-zero object `owner_id` for fast `SAY ?OWN`.
///
/// Maintained on CLAIM / DROP (insert) and take / clear / UNLOCK (remove) so
/// listing owned property never scans the full world map.
#[derive(Debug, Default, Clone)]
pub struct OwnedTiles {
    /// `(x, y)` → `owner_id`
    pub by_tile: HashMap<(i32, i32), i32>,
    /// `owner_id` → set of tiles
    pub by_owner: HashMap<i32, HashSet<(i32, i32)>>,
}

impl OwnedTiles {
    /// Record that `(x, y)` is owned by `owner_id`. `owner_id == 0` removes.
    pub fn insert(&mut self, x: i32, y: i32, owner_id: i32) {
        if owner_id == 0 {
            self.remove(x, y);
            return;
        }
        if let Some(old) = self.by_tile.insert((x, y), owner_id) {
            if old != owner_id {
                if let Some(set) = self.by_owner.get_mut(&old) {
                    set.remove(&(x, y));
                    if set.is_empty() {
                        self.by_owner.remove(&old);
                    }
                }
            }
        }
        self.by_owner.entry(owner_id).or_default().insert((x, y));
    }

    /// Drop tile from the sparse ownership index (object taken / cleared / unowned).
    pub fn remove(&mut self, x: i32, y: i32) {
        if let Some(owner) = self.by_tile.remove(&(x, y)) {
            if let Some(set) = self.by_owner.get_mut(&owner) {
                set.remove(&(x, y));
                if set.is_empty() {
                    self.by_owner.remove(&owner);
                }
            }
        }
    }

    pub fn owner_at(&self, x: i32, y: i32) -> Option<i32> {
        self.by_tile.get(&(x, y)).copied()
    }

    pub fn count_of(&self, owner_id: i32) -> usize {
        self.by_owner.get(&owner_id).map(|s| s.len()).unwrap_or(0)
    }

    /// Sorted list of tiles owned by `owner_id`.
    pub fn tiles_of(&self, owner_id: i32) -> Vec<(i32, i32)> {
        let mut v: Vec<(i32, i32)> = self
            .by_owner
            .get(&owner_id)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default();
        v.sort_unstable();
        v
    }

    /// `OWN none` or `OWN x,y x,y …` (sorted tile list for speaker).
    pub fn format_query(&self, owner_id: i32) -> String {
        let tiles = self.tiles_of(owner_id);
        if tiles.is_empty() {
            "OWN none".into()
        } else {
            let body = tiles
                .iter()
                .map(|(x, y)| format!("{x},{y}"))
                .collect::<Vec<_>>()
                .join(" ");
            format!("OWN {body}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_remove_query() {
        let mut idx = SpecialIndex::default();
        idx.insert(1, 2, SpecialKind::Gate);
        idx.insert(3, 4, SpecialKind::Grave);
        idx.insert(1, 2, SpecialKind::Container); // overwrite
        assert_eq!(idx.kind_at(1, 2), Some(SpecialKind::Container));
        assert_eq!(idx.count(SpecialKind::Gate), 0);
        assert_eq!(idx.count(SpecialKind::Container), 1);
        assert!(idx.format_query().contains("total=2"));
        idx.remove(3, 4);
        assert_eq!(idx.total(), 1);
    }

    #[test]
    fn from_name() {
        assert_eq!(
            SpecialKind::from_object_name("Wooden Gate"),
            Some(SpecialKind::Gate)
        );
        assert_eq!(SpecialKind::from_object_name("Stone"), None);
    }

    #[test]
    fn owned_tiles_insert_remove_format() {
        let mut o = OwnedTiles::default();
        assert_eq!(o.format_query(7), "OWN none");
        o.insert(4, 5, 7);
        o.insert(1, 2, 7);
        o.insert(9, 9, 3); // other owner
        assert_eq!(o.count_of(7), 2);
        assert_eq!(o.owner_at(4, 5), Some(7));
        // Sorted: 1,2 then 4,5
        assert_eq!(o.format_query(7), "OWN 1,2 4,5");
        assert_eq!(o.format_query(3), "OWN 9,9");
        // Reassign tile 4,5 to player 3
        o.insert(4, 5, 3);
        assert_eq!(o.count_of(7), 1);
        assert_eq!(o.format_query(7), "OWN 1,2");
        assert_eq!(o.format_query(3), "OWN 4,5 9,9");
        o.remove(1, 2);
        assert_eq!(o.format_query(7), "OWN none");
        // owner_id 0 acts as remove
        o.insert(9, 9, 0);
        assert_eq!(o.owner_at(9, 9), None);
        assert_eq!(o.format_query(3), "OWN 4,5");
    }
}
