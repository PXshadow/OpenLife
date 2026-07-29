//! Versioned binary lineage save/load (no SQL).
//!
//! Format family magic `OLN1` (version field selects record layout):
//! ```text
//! magic[4] = b"OLN1"
//! version: u32 LE   (1 = core; 2 = + death fields for LINEAGE-24H)
//! count: u32 LE
//! records × count:
//!   id: i32 LE
//!   mother: i32 LE   (−1 = none)
//!   father: i32 LE   (−1 = none)
//!   gen: i32 LE
//!   prestige: f32 LE
//!   name_len: u32 LE
//!   name: [u8; name_len]  (UTF-8)
//!   # version >= 2 (LINEAGE-24H / Haxe deathTime+deathReason):
//!   death_sim_time: f32 LE
//!   age_at_death: f32 LE
//!   death_reason_len: u32 LE
//!   death_reason: [u8; death_reason_len]  (UTF-8)
//! ```
//!
//! Session-only: `alive` (derived on load from death fields), `owns_object`.

use crate::prestige::PrestigeClass;
use crate::social::{LineageNode, SocialState};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::time::Instant;
use tracing::info;

/// Current on-disk lineage record version (writes always use this).
///
/// v1: core identity/parents/gen/prestige/name.  
/// v2: + death_sim_time / age_at_death / death_reason (LINEAGE-24H starving window).
// Haxe: Lineage WriteLineages deathTime + deathReason
pub const LINEAGE_FORMAT_VERSION: u32 = 2;
/// Oldest readable version (v1 core without death fields).
pub const LINEAGE_FORMAT_VERSION_MIN: u32 = 1;
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
    // OLN2 / LINEAGE-24H: Haxe WriteLineages deathTime + age + deathReason
    // Haxe: Lineage.WriteLineages L182–186
    w.write_f32::<LittleEndian>(n.death_sim_time)
        .map_err(|e| e.to_string())?;
    w.write_f32::<LittleEndian>(n.age_at_death)
        .map_err(|e| e.to_string())?;
    let reason_bytes = n.death_reason.as_bytes();
    if reason_bytes.len() > 4096 {
        return Err(format!(
            "lineage death_reason too long ({})",
            reason_bytes.len()
        ));
    }
    w.write_u32::<LittleEndian>(reason_bytes.len() as u32)
        .map_err(|e| e.to_string())?;
    w.write_all(reason_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

fn read_lineages(r: &mut impl Read) -> Result<SocialState, String> {
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if &magic != MAGIC {
        return Err(format!("bad lineage magic {:?}", magic));
    }
    let version = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())?;
    if version < LINEAGE_FORMAT_VERSION_MIN || version > LINEAGE_FORMAT_VERSION {
        return Err(format!(
            "unsupported lineage version {version} (want {LINEAGE_FORMAT_VERSION_MIN}..{LINEAGE_FORMAT_VERSION})"
        ));
    }
    let count = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    let mut lineages = HashMap::with_capacity(count);
    for _ in 0..count {
        let node = read_record(r, version)?;
        lineages.insert(node.id, node);
    }
    Ok(SocialState {
        lineages,
        ..SocialState::default()
    })
}

fn read_record(r: &mut impl Read, version: u32) -> Result<LineageNode, String> {
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

    // OLN2 death fields; v1 defaults empty/alive.
    // Haxe: Lineage.ReadLineages deathTime / deathReason / age
    let (death_sim_time, age_at_death, death_reason) = if version >= 2 {
        let death_sim_time = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
        let age_at_death = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
        let reason_len = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
        if reason_len > 4096 {
            return Err(format!("lineage death_reason too long ({reason_len})"));
        }
        let mut reason_buf = vec![0u8; reason_len];
        r.read_exact(&mut reason_buf).map_err(|e| e.to_string())?;
        let death_reason = String::from_utf8(reason_buf).map_err(|e| e.to_string())?;
        let death_sim_time = if death_sim_time.is_finite() {
            death_sim_time.max(0.0)
        } else {
            0.0
        };
        let age_at_death = if age_at_death.is_finite() {
            age_at_death.max(0.0)
        } else {
            0.0
        };
        (death_sim_time, age_at_death, death_reason)
    } else {
        (0.0, 0.0, String::new())
    };

    // Derive alive: any death record means not currently living this life.
    let alive = death_sim_time <= 0.0 && death_reason.is_empty();

    Ok(LineageNode {
        id,
        name,
        mother_id,
        father_id,
        generation,
        prestige,
        prestige_class: PrestigeClass::from_prestige(prestige),
        alive,
        // Haxe ownsObject is session-only (InitObjectHelpersAfterRead); not on disk.
        owns_object: false,
        death_sim_time,
        death_reason,
        age_at_death,
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
            alive: true,
            owns_object: false,
            death_sim_time: 0.0,
            death_reason: String::new(),
            age_at_death: 0.0,
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
                alive: true,
                owns_object: false,
                death_sim_time: 0.0,
                death_reason: String::new(),
                age_at_death: 0.0,
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
        assert!(eve.alive);
        assert_eq!(eve.death_sim_time, 0.0);
        assert!(eve.death_reason.is_empty());

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

    /// OLN2 / LINEAGE-24H: deathTime + deathReason + age survive disk roundtrip.
    // Haxe: Lineage WriteLineages / ReadLineages death fields
    #[test]
    fn roundtrip_preserves_death_fields_oln2() {
        let dir = unique_temp_dir("ol_lineage_death_oln2");
        let path = dir.join("lineages_v1.bin");

        let mut original = sample_social();
        if let Some(n) = original.lineages.get_mut(&10) {
            n.stamp_death(12_345.0, "reason_hunger", 42.5);
        }
        if let Some(n) = original.lineages.get_mut(&3) {
            n.stamp_death(99.0, "reason_killed_33", 18.0);
        }
        save_lineages(&original, &path).unwrap();
        let loaded = load_lineages(&path).unwrap();

        let alice = loaded.lineages.get(&10).unwrap();
        assert!(!alice.alive);
        assert!((alice.death_sim_time - 12_345.0).abs() < 1e-3);
        assert_eq!(alice.death_reason, "reason_hunger");
        assert!((alice.age_at_death - 42.5).abs() < 1e-3);

        let bob = loaded.lineages.get(&3).unwrap();
        assert!(!bob.alive);
        assert!((bob.death_sim_time - 99.0).abs() < 1e-3);
        assert_eq!(bob.death_reason, "reason_killed_33");
        assert!((bob.age_at_death - 18.0).abs() < 1e-3);

        // Eve still living defaults
        let eve = loaded.lineages.get(&2).unwrap();
        assert!(eve.alive);
        assert_eq!(eve.death_sim_time, 0.0);
        assert!(eve.death_reason.is_empty());

        // Boot-seed starving stamps from loaded death fields
        use crate::world_food_stats::WorldFoodStats;
        let mut food = WorldFoodStats::new();
        let n = food.seed_death_stamps_from_lineage_rows(loaded.lineage_stat_rows(), 12_345.0);
        assert_eq!(n, 2);
        assert_eq!(food.reason_hunger_deaths, 1);
        assert_eq!(food.reason_killed_last_day_count("reason_killed_33"), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// v1 files (no death trailer) still load with empty death fields.
    #[test]
    fn loads_legacy_v1_without_death_fields() {
        let dir = unique_temp_dir("ol_lineage_v1_legacy");
        let path = dir.join("legacy.bin");
        // Manually write a v1 file (core fields only).
        {
            use byteorder::WriteBytesExt;
            use std::io::Write;
            let f = std::fs::File::create(&path).unwrap();
            let mut w = std::io::BufWriter::new(f);
            w.write_all(MAGIC).unwrap();
            w.write_u32::<LittleEndian>(1).unwrap(); // version 1
            w.write_u32::<LittleEndian>(1).unwrap(); // count
            w.write_i32::<LittleEndian>(7).unwrap(); // id
            w.write_i32::<LittleEndian>(NONE_PARENT).unwrap();
            w.write_i32::<LittleEndian>(NONE_PARENT).unwrap();
            w.write_i32::<LittleEndian>(0).unwrap(); // gen
            w.write_f32::<LittleEndian>(1.5).unwrap(); // prestige
            let name = b"LEGACY";
            w.write_u32::<LittleEndian>(name.len() as u32).unwrap();
            w.write_all(name).unwrap();
            w.flush().unwrap();
        }
        let loaded = load_lineages(&path).unwrap();
        let n = loaded.lineages.get(&7).unwrap();
        assert_eq!(n.name, "LEGACY");
        assert!(n.alive);
        assert_eq!(n.death_sim_time, 0.0);
        assert!(n.death_reason.is_empty());
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
