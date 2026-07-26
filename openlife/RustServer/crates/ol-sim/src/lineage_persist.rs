//! Versioned binary lineage save/load (no SQL).
//!
//! Format `OLN1` (u32 version = LINEAGE_FORMAT_VERSION):
//! ```text
//! magic[4] = b"OLN1"
//! version: u32 LE
//! count: u32 LE
//! records × count:
//!   id: i32 LE
//!   mother: i32 LE   (−1 = none)
//!   father: i32 LE   (−1 = none)
//!   gen: i32 LE
//!   prestige: f32 LE
//!   name_len: u32 LE
//!   name: [u8; name_len]  (UTF-8)
//! ```

use crate::prestige::PrestigeClass;
use crate::social::{LineageNode, SocialState};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::time::Instant;
use tracing::info;

pub const LINEAGE_FORMAT_VERSION: u32 = 1;
const MAGIC: &[u8; 4] = b"OLN1";
/// Sentinel for missing mother/father parent id.
const NONE_PARENT: i32 = -1;

/// Default on-disk name under the save directory.
pub const DEFAULT_LINEAGE_FILE: &str = "lineages_v1.bin";

/// Write all lineages from `social` to `path` (atomic-ish via temp rename).
pub fn save_lineages(social: &SocialState, path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    let t0 = Instant::now();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("bin.tmp");
    {
        let f = File::create(&tmp).map_err(|e| e.to_string())?;
        let mut w = BufWriter::with_capacity(64 * 1024, f);
        write_lineages(social, &mut w)?;
        w.flush().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    info!(
        path = %path.display(),
        count = social.lineages.len(),
        ms = t0.elapsed().as_millis() as u64,
        "lineages saved"
    );
    Ok(())
}

/// Load lineages from `path` into a fresh [`SocialState`] (following/exiles empty).
pub fn load_lineages(path: impl AsRef<Path>) -> Result<SocialState, String> {
    let path = path.as_ref();
    let t0 = Instant::now();
    let f = File::open(path).map_err(|e| e.to_string())?;
    let mut r = BufReader::with_capacity(64 * 1024, f);
    let social = read_lineages(&mut r)?;
    info!(
        path = %path.display(),
        count = social.lineages.len(),
        ms = t0.elapsed().as_millis() as u64,
        "lineages loaded"
    );
    Ok(social)
}

fn write_lineages(social: &SocialState, w: &mut impl Write) -> Result<(), String> {
    w.write_all(MAGIC).map_err(|e| e.to_string())?;
    w.write_u32::<LittleEndian>(LINEAGE_FORMAT_VERSION)
        .map_err(|e| e.to_string())?;
    w.write_u32::<LittleEndian>(social.lineages.len() as u32)
        .map_err(|e| e.to_string())?;

    // Stable order for deterministic files / easier diffs in tests.
    let mut ids: Vec<i32> = social.lineages.keys().copied().collect();
    ids.sort_unstable();
    for id in ids {
        let n = social.lineages.get(&id).expect("id from keys");
        write_record(n, w)?;
    }
    Ok(())
}

fn write_record(n: &LineageNode, w: &mut impl Write) -> Result<(), String> {
    w.write_i32::<LittleEndian>(n.id)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(n.mother_id.unwrap_or(NONE_PARENT))
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(n.father_id.unwrap_or(NONE_PARENT))
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(n.generation)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(n.prestige)
        .map_err(|e| e.to_string())?;
    let name_bytes = n.name.as_bytes();
    w.write_u32::<LittleEndian>(name_bytes.len() as u32)
        .map_err(|e| e.to_string())?;
    w.write_all(name_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

fn read_lineages(r: &mut impl Read) -> Result<SocialState, String> {
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if &magic != MAGIC {
        return Err(format!("bad lineage magic {:?}", magic));
    }
    let version = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())?;
    if version != LINEAGE_FORMAT_VERSION {
        return Err(format!(
            "unsupported lineage version {version} (want {LINEAGE_FORMAT_VERSION})"
        ));
    }
    let count = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    let mut lineages = HashMap::with_capacity(count);
    for _ in 0..count {
        let node = read_record(r)?;
        lineages.insert(node.id, node);
    }
    Ok(SocialState {
        lineages,
        ..SocialState::default()
    })
}

fn read_record(r: &mut impl Read) -> Result<LineageNode, String> {
    let id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let mother_raw = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let father_raw = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let generation = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let prestige = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    let name_len = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    if name_len > 4096 {
        return Err(format!("lineage name too long ({name_len})"));
    }
    let mut name_buf = vec![0u8; name_len];
    r.read_exact(&mut name_buf).map_err(|e| e.to_string())?;
    let name = String::from_utf8(name_buf).map_err(|e| e.to_string())?;
    let mother_id = if mother_raw == NONE_PARENT {
        None
    } else {
        Some(mother_raw)
    };
    let father_id = if father_raw == NONE_PARENT {
        None
    } else {
        Some(father_raw)
    };
    let prestige = prestige.max(0.0);
    Ok(LineageNode {
        id,
        name,
        mother_id,
        father_id,
        generation,
        prestige,
        prestige_class: PrestigeClass::from_prestige(prestige),
    })
}

impl SocialState {
    /// Persist lineages only (following / exile not included in OLN1).
    pub fn save_lineages_file(&self, path: impl AsRef<Path>) -> Result<(), String> {
        save_lineages(self, path)
    }

    /// Replace `self.lineages` from file; keeps following/exiles/colors.
    pub fn load_lineages_file(&mut self, path: impl AsRef<Path>) -> Result<(), String> {
        let loaded = load_lineages(path)?;
        self.lineages = loaded.lineages;
        Ok(())
    }

    /// Build a SocialState from a lineage save (following empty).
    pub fn from_lineages_file(path: impl AsRef<Path>) -> Result<Self, String> {
        load_lineages(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Unique temp dir per call (parallel tests must not share fixed names).
    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = env::temp_dir().join(format!("{prefix}_{t}_{n}"));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    fn sample_social() -> SocialState {
        let mut s = SocialState::default();
        s.lineages.insert(2, LineageNode::eve(2, "EVE"));
        let mut child = LineageNode {
            id: 10,
            name: "ALICE".into(),
            mother_id: Some(2),
            father_id: Some(3),
            generation: 1,
            prestige: 55.0,
            prestige_class: PrestigeClass::from_prestige(55.0),
        };
        child.set_prestige(55.0);
        s.lineages.insert(10, child);
        s.lineages.insert(
            3,
            LineageNode {
                id: 3,
                name: "BOB".into(),
                mother_id: None,
                father_id: None,
                generation: 0,
                prestige: 12.5,
                prestige_class: PrestigeClass::from_prestige(12.5),
            },
        );
        s
    }

    #[test]
    fn roundtrip_preserves_records() {
        let dir = unique_temp_dir("ol_lineage_persist_test");
        let path = dir.join("lineages_v1.bin");

        let original = sample_social();
        save_lineages(&original, &path).unwrap();
        let loaded = load_lineages(&path).unwrap();

        assert_eq!(loaded.lineages.len(), 3);
        let eve = loaded.lineages.get(&2).unwrap();
        assert_eq!(eve.name, "EVE");
        assert_eq!(eve.mother_id, None);
        assert_eq!(eve.father_id, None);
        assert_eq!(eve.generation, 0);
        assert_eq!(eve.prestige, 0.0);

        let alice = loaded.lineages.get(&10).unwrap();
        assert_eq!(alice.name, "ALICE");
        assert_eq!(alice.mother_id, Some(2));
        assert_eq!(alice.father_id, Some(3));
        assert_eq!(alice.generation, 1);
        assert!((alice.prestige - 55.0).abs() < 1e-5);
        assert_eq!(alice.prestige_class, PrestigeClass::from_prestige(55.0));

        let bob = loaded.lineages.get(&3).unwrap();
        assert_eq!(bob.name, "BOB");
        assert!((bob.prestige - 12.5).abs() < 1e-5);

        // Following not in file.
        assert!(loaded.following.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn social_helpers_roundtrip() {
        let dir = unique_temp_dir("ol_lineage_helpers_test");
        let path = dir.join("lineages_v1.bin");

        let mut s = sample_social();
        s.set_follow(10, 2).unwrap();
        s.save_lineages_file(&path).unwrap();

        let mut other = SocialState::default();
        other.load_lineages_file(&path).unwrap();
        assert_eq!(other.lineages.len(), 3);
        assert!(other.following.is_empty());

        let from = SocialState::from_lineages_file(&path).unwrap();
        assert_eq!(from.lineages.get(&10).unwrap().name, "ALICE");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_bad_magic() {
        let dir = unique_temp_dir("ol_lineage_bad_magic");
        let path = dir.join("bad.bin");
        std::fs::write(&path, b"XXXX").unwrap();
        assert!(load_lineages(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_roundtrip() {
        let dir = unique_temp_dir("ol_lineage_empty");
        let path = dir.join("empty.bin");
        let s = SocialState::default();
        save_lineages(&s, &path).unwrap();
        let loaded = load_lineages(&path).unwrap();
        assert!(loaded.lineages.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Stress: 50 lineage nodes save → load preserves ids, parents, gen, prestige, names.
    #[test]
    fn stress_roundtrip_50_nodes() {
        let dir = unique_temp_dir("ol_lineage_stress_50");
        let path = dir.join("lineages_v1.bin");

        let mut original = SocialState::default();
        // Eve root + chain of descendants + a few sibling branches.
        original.lineages.insert(0, LineageNode::eve(0, "EVE0"));
        for i in 1..=49 {
            let mother_id = if i <= 10 { 0 } else { (i - 1) / 2 };
            let mother = original.lineages.get(&mother_id).cloned().unwrap_or_else(|| {
                LineageNode::eve(mother_id, format!("M{mother_id}"))
            });
            let mut n = LineageNode::with_mother(i, format!("N{i}"), &mother);
            if i % 3 == 0 {
                n.father_id = Some(i.saturating_sub(2).max(0));
            }
            n.set_prestige((i as f32) * 1.5);
            original.lineages.insert(i, n);
        }
        assert_eq!(original.lineages.len(), 50);

        save_lineages(&original, &path).unwrap();
        // Second save overwrites cleanly (idempotent path).
        save_lineages(&original, &path).unwrap();
        let loaded = load_lineages(&path).unwrap();

        assert_eq!(loaded.lineages.len(), 50);
        for id in 0..50 {
            let a = original.lineages.get(&id).unwrap();
            let b = loaded.lineages.get(&id).expect("missing id after load");
            assert_eq!(b.id, a.id);
            assert_eq!(b.name, a.name);
            assert_eq!(b.mother_id, a.mother_id);
            assert_eq!(b.father_id, a.father_id);
            assert_eq!(b.generation, a.generation);
            assert!(
                (b.prestige - a.prestige).abs() < 1e-4,
                "prestige id={id}: {} vs {}",
                b.prestige,
                a.prestige
            );
            assert_eq!(b.prestige_class, a.prestige_class);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
