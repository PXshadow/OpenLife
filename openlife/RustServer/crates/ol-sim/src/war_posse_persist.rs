//! Session war / posse disk persist (**SOCIAL-WAR-PERSIST** / `war_posse_disk`).
//!
//! Haxe never persisted WAR/POSSE session maps (they are protocol-driven session
//! state on the Rust server: [`crate::WarState`] + [`crate::PosseState`]).
//! This module implements versioned **WPS1** so restart / autosave keeps them.
//!
//! Keys are **session player ids** (same as live `SAY WAR` / `POSSE`). Useful for
//! crash recovery and soft restart while the same `p_id`s remain meaningful;
//! full sticky identity across fresh spawns still needs Players.bin residual.
//!
//! On **death**, live maps prune the deceased via [`prune_war_posse_for_player`]
//! so orphans do not accumulate; disconnect without death keeps edges (same life).
//!
//! ```text
//! magic[4] = b"WPS1"
//! version: u32 LE (= WAR_POSSE_FORMAT_VERSION)
//! war_pair_count: u32 LE
//! pairs × war_pair_count:
//!   a: i32 LE
//!   b: i32 LE
//!   status_len: u32 LE
//!   status: [u8; status_len]   // "War" / "Alliance" / …
//! posse_killer_count: u32 LE
//! killers × posse_killer_count:
//!   killer: i32 LE
//!   target_count: u32 LE
//!   targets × target_count: i32 LE
//! ```

use crate::posse::PosseState;
use crate::war::{pair_key, WarState, STATUS_PEACE};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tracing::info;

/// Versioned war+posse store format id.
pub const WAR_POSSE_FORMAT_VERSION: u32 = 1;
const WPS_MAGIC: &[u8; 4] = b"WPS1";
/// Default on-disk name under the save directory.
pub const DEFAULT_WAR_POSSE_FILE: &str = "war_posse_v1.bin";

/// Combined session snapshot for disk + outer autosave mirror.
#[derive(Debug, Default, Clone)]
pub struct WarPosseSnapshot {
    pub war: WarState,
    pub posse: PosseState,
}

impl WarPosseSnapshot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_parts(war: WarState, posse: PosseState) -> Self {
        Self { war, posse }
    }

    /// Non-Peace war pairs + total posse edges (for logging).
    pub fn counts(&self) -> (usize, usize) {
        let wars = self.war.list_relations(None).len();
        let posse_edges: usize = self.posse.by_killer.values().map(|s| s.len()).sum();
        (wars, posse_edges)
    }

    /// Drop war pairs + posse edges involving `p_id`. Returns `(war_pairs, posse_edges)` removed.
    pub fn prune_player(&mut self, p_id: i32) -> (usize, usize) {
        prune_war_posse_for_player(&mut self.war, &mut self.posse, p_id)
    }

    /// Keep only relations where both ends are in `alive`. Returns `(war_pairs, posse_edges)` removed.
    pub fn prune_absent(&mut self, alive: &HashSet<i32>) -> (usize, usize) {
        prune_war_posse_absent(&mut self.war, &mut self.posse, alive)
    }
}

/// Drop session war + posse edges for one player (death cleanup).
///
/// Returns `(war_pairs_removed, posse_edges_removed)`.
/// // Haxe: none — Rust product session maps (no server WAR/POSSE disk).
pub fn prune_war_posse_for_player(
    war: &mut WarState,
    posse: &mut PosseState,
    p_id: i32,
) -> (usize, usize) {
    let w = war.prune_player(p_id);
    let p = posse.prune_player(p_id);
    (w, p)
}

/// Drop edges where either end is missing from `alive` (bulk orphan sweep).
pub fn prune_war_posse_absent(
    war: &mut WarState,
    posse: &mut PosseState,
    alive: &HashSet<i32>,
) -> (usize, usize) {
    let w = war.prune_absent(alive);
    let p = posse.prune_absent(alive);
    (w, p)
}

/// Shared mirror for sim ↔ ol-server autosave (same pattern as accounts/social).
pub type WarPosseShare = Arc<RwLock<WarPosseSnapshot>>;

/// Atomic save of war+posse to `path` (write tmp then rename).
pub fn save_war_posse(snap: &WarPosseSnapshot, path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    let t0 = Instant::now();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("bin.tmp");
    {
        let f = File::create(&tmp).map_err(|e| e.to_string())?;
        let mut w = BufWriter::with_capacity(32 * 1024, f);
        write_war_posse(snap, &mut w)?;
        w.flush().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    let (wars, posse_edges) = snap.counts();
    info!(
        path = %path.display(),
        wars,
        posse_edges,
        ms = t0.elapsed().as_millis() as u64,
        "war/posse saved (WPS1)"
    );
    Ok(())
}

/// Load war+posse from `path`. Missing file → empty snapshot (Ok).
pub fn load_war_posse(path: impl AsRef<Path>) -> Result<WarPosseSnapshot, String> {
    let path: &Path = path.as_ref();
    if !path.exists() {
        return Ok(WarPosseSnapshot::default());
    }
    let t0 = Instant::now();
    let f = File::open(path).map_err(|e| e.to_string())?;
    let mut r = BufReader::with_capacity(32 * 1024, f);
    let snap = read_war_posse(&mut r)?;
    let (wars, posse_edges) = snap.counts();
    info!(
        path = %path.display(),
        wars,
        posse_edges,
        ms = t0.elapsed().as_millis() as u64,
        "war/posse loaded (WPS1)"
    );
    Ok(snap)
}

/// Apply snapshot into live sim fields (replaces current maps).
pub fn apply_war_posse_snapshot(war: &mut WarState, posse: &mut PosseState, snap: &WarPosseSnapshot) {
    *war = snap.war.clone();
    *posse = snap.posse.clone();
}

/// Build snapshot from live sim fields.
pub fn capture_war_posse_snapshot(war: &WarState, posse: &PosseState) -> WarPosseSnapshot {
    WarPosseSnapshot {
        war: war.clone(),
        posse: posse.clone(),
    }
}

fn write_war_posse(snap: &WarPosseSnapshot, w: &mut impl Write) -> Result<(), String> {
    w.write_all(WPS_MAGIC).map_err(|e| e.to_string())?;
    w.write_u32::<LittleEndian>(WAR_POSSE_FORMAT_VERSION)
        .map_err(|e| e.to_string())?;

    // War pairs: non-Peace only (Peace is absence).
    let mut pairs: Vec<(i32, i32, &str)> = snap
        .war
        .pairs
        .iter()
        .filter(|(_, s)| s.as_str() != STATUS_PEACE)
        .map(|((a, b), s)| (*a, *b, s.as_str()))
        .collect();
    pairs.sort_by(|x, y| (x.0, x.1).cmp(&(y.0, y.1)));
    w.write_u32::<LittleEndian>(pairs.len() as u32)
        .map_err(|e| e.to_string())?;
    for (a, b, status) in pairs {
        w.write_i32::<LittleEndian>(a).map_err(|e| e.to_string())?;
        w.write_i32::<LittleEndian>(b).map_err(|e| e.to_string())?;
        let sb = status.as_bytes();
        w.write_u32::<LittleEndian>(sb.len() as u32)
            .map_err(|e| e.to_string())?;
        w.write_all(sb).map_err(|e| e.to_string())?;
    }

    // Posse: killer → targets
    let mut killers: Vec<i32> = snap.posse.by_killer.keys().copied().collect();
    killers.sort_unstable();
    w.write_u32::<LittleEndian>(killers.len() as u32)
        .map_err(|e| e.to_string())?;
    for killer in killers {
        w.write_i32::<LittleEndian>(killer)
            .map_err(|e| e.to_string())?;
        let mut targets: Vec<i32> = snap
            .posse
            .by_killer
            .get(&killer)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default();
        targets.sort_unstable();
        w.write_u32::<LittleEndian>(targets.len() as u32)
            .map_err(|e| e.to_string())?;
        for t in targets {
            w.write_i32::<LittleEndian>(t).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn read_war_posse(r: &mut impl Read) -> Result<WarPosseSnapshot, String> {
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if &magic != WPS_MAGIC {
        return Err(format!(
            "bad war/posse magic: {:?} (want WPS1)",
            String::from_utf8_lossy(&magic)
        ));
    }
    let ver = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())?;
    if ver != WAR_POSSE_FORMAT_VERSION {
        return Err(format!(
            "unsupported war/posse version {ver} (want {WAR_POSSE_FORMAT_VERSION})"
        ));
    }

    let mut war = WarState::new();
    let pair_count = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    for _ in 0..pair_count {
        let a = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
        let b = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
        let slen = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
        if slen > 1024 {
            return Err(format!("war status string too long: {slen}"));
        }
        let mut buf = vec![0u8; slen];
        r.read_exact(&mut buf).map_err(|e| e.to_string())?;
        let status = String::from_utf8(buf).map_err(|e| e.to_string())?;
        if a != b && status != STATUS_PEACE {
            // Normalize undirected key on insert.
            let (lo, hi) = pair_key(a, b);
            war.pairs.insert((lo, hi), status);
        }
    }

    let mut posse = PosseState::new();
    let killer_count = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    for _ in 0..killer_count {
        let killer = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
        let tcount = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
        if tcount > 100_000 {
            return Err(format!("posse target count absurd: {tcount}"));
        }
        let mut set = HashSet::new();
        for _ in 0..tcount {
            let t = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
            if t > 0 && t != killer {
                set.insert(t);
            }
        }
        if !set.is_empty() {
            posse.by_killer.insert(killer, set);
        }
    }

    Ok(WarPosseSnapshot { war, posse })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::war::{STATUS_ALLIANCE, STATUS_WAR};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("ol_war_posse_{prefix}_{t}_{n}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn wps1_roundtrip_war_and_posse() {
        let dir = unique_temp_dir("rt");
        let path = dir.join(DEFAULT_WAR_POSSE_FILE);

        let mut war = WarState::new();
        war.declare_war(2, 5);
        war.make_alliance(1, 3);
        // Peace self / unset not stored
        war.make_peace(9, 10);

        let mut posse = PosseState::new();
        assert!(posse.add_posse(7, 12));
        assert!(posse.add_posse(7, 4));
        assert!(posse.add_posse(8, 12));

        let snap = WarPosseSnapshot::from_parts(war, posse);
        save_war_posse(&snap, &path).unwrap();

        let loaded = load_war_posse(&path).unwrap();
        assert!(loaded.war.is_at_war(2, 5));
        assert!(loaded.war.is_at_war(5, 2));
        assert!(loaded.war.is_allied(1, 3));
        assert!(!loaded.war.is_at_war(9, 10));
        assert_eq!(loaded.war.status(1, 3), STATUS_ALLIANCE);
        assert_eq!(loaded.war.status(2, 5), STATUS_WAR);
        assert!(loaded.posse.has_target(7, 12));
        assert!(loaded.posse.has_target(7, 4));
        assert!(loaded.posse.has_target(8, 12));
        assert_eq!(loaded.posse.targets_sorted(7), vec![4, 12]);
        assert_eq!(loaded.counts(), (2, 3));

        let empty = load_war_posse(dir.join("nope.bin")).unwrap();
        assert_eq!(empty.counts(), (0, 0));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn capture_apply_roundtrip() {
        let mut war = WarState::new();
        war.declare_war(1, 2);
        let mut posse = PosseState::new();
        posse.add_posse(1, 9);
        let snap = capture_war_posse_snapshot(&war, &posse);

        let mut w2 = WarState::new();
        let mut p2 = PosseState::new();
        apply_war_posse_snapshot(&mut w2, &mut p2, &snap);
        assert!(w2.is_at_war(1, 2));
        assert!(p2.has_target(1, 9));
    }

    #[test]
    fn bad_magic_errors() {
        let dir = unique_temp_dir("bad");
        let path = dir.join("bad.bin");
        std::fs::write(&path, b"XXXX\x01\x00\x00\x00").unwrap();
        let err = load_war_posse(&path).unwrap_err();
        assert!(err.contains("magic") || err.contains("WPS1"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_snapshot_writes_header_only() {
        let dir = unique_temp_dir("empty");
        let path = dir.join(DEFAULT_WAR_POSSE_FILE);
        save_war_posse(&WarPosseSnapshot::default(), &path).unwrap();
        let loaded = load_war_posse(&path).unwrap();
        assert_eq!(loaded.counts(), (0, 0));
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[0..4], b"WPS1");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prune_player_and_roundtrip() {
        let mut war = WarState::new();
        war.declare_war(1, 2);
        war.make_alliance(1, 3);
        war.declare_war(2, 3);
        let mut posse = PosseState::new();
        posse.add_posse(1, 2);
        posse.add_posse(4, 1);
        posse.add_posse(4, 5);

        let (w, p) = prune_war_posse_for_player(&mut war, &mut posse, 1);
        assert_eq!(w, 2);
        assert_eq!(p, 2); // killer set {2} + as target of 4
        assert!(!war.is_at_war(1, 2));
        assert!(war.is_at_war(2, 3));
        assert!(!posse.has_target(1, 2));
        assert!(!posse.has_target(4, 1));
        assert!(posse.has_target(4, 5));

        let mut snap = WarPosseSnapshot::from_parts(war, posse);
        let dir = unique_temp_dir("prune");
        let path = dir.join(DEFAULT_WAR_POSSE_FILE);
        save_war_posse(&snap, &path).unwrap();
        let loaded = load_war_posse(&path).unwrap();
        assert!(loaded.war.is_at_war(2, 3));
        assert!(loaded.posse.has_target(4, 5));
        assert_eq!(loaded.counts(), (1, 1));

        // Bulk absent: only 2,3,4,5 living → keep remaining.
        let alive: HashSet<i32> = [2, 3, 4, 5].into_iter().collect();
        let (w2, p2) = snap.prune_absent(&alive);
        assert_eq!((w2, p2), (0, 0));
        let only_2: HashSet<i32> = [2].into_iter().collect();
        let (w3, p3) = snap.prune_absent(&only_2);
        assert!(w3 + p3 >= 1);
        assert_eq!(snap.counts(), (0, 0));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
