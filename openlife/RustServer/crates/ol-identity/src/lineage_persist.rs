//! Versioned binary lineage save/load (no SQL).
//!
//! Format family magic `OLN1` (version field selects record layout):
//! ```text
//! magic[4] = b"OLN1"
//! version: u32 LE   (1 = core; … 13 = + killedByPlayerId; 14 = + trueAge)
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
//!   # version >= 3 (LINEAGE-BIRTH-TIME / Haxe birthTime):
//!   birth_sim_time: f32 LE  (−1 = unknown)
//!   # version >= 4 (LINEAGE-LAST-SAID / Haxe lastSaid):
//!   last_said_len: u32 LE
//!   last_said: [u8; last_said_len]  (UTF-8)
//!   # version >= 5 (LINEAGE-EVE-ID / Haxe myEveId):
//!   my_eve_id: i32 LE  (0 = unset)
//!   # version >= 6 (LINEAGE-REP-DISK / Haxe reputation):
//!   reputation: f32 LE  (lostCombatPrestige * −1 at death)
//!   # version >= 7 (LINEAGE-COINS-DISK / Haxe coins):
//!   coins: f32 LE  (death snapshot; wallet persist is separate)
//!   # version >= 8 (LINEAGE-FAMILY-NAME / Haxe myFamilyName):
//!   family_name_len: u32 LE
//!   family_name: [u8; family_name_len]  (UTF-8)
//!   # version >= 9 (LINEAGE-PO-ID / Haxe po_id):
//!   po_id: i32 LE  (−1 = unset)
//!   # version >= 10 (LINEAGE-ACCOUNT-ID / Haxe accountId):
//!   account_id: i32 LE  (0 = unset)
//!   # version >= 11 (LINEAGE-DYNASTY / Haxe myDynastyId):
//!   my_dynasty_id: i32 LE  (−1 = unset)
//!   # version >= 12 (LINEAGE-FOLLOW-ID / Haxe followPlayerId):
//!   follow_player_id: i32 LE  (0 = none)
//!   # version >= 13 (LINEAGE-KILLED-BY / Haxe killedByPlayerId):
//!   killed_by_player_id: i32 LE  (0 = none)
//!   # version >= 14 (LINEAGE-TRUE-AGE / Haxe trueAge):
//!   true_age_at_death: f32 LE
//!   # version >= 15 (PRESTIGE-FAMILY-PERSIST / Haxe prestigeFrom*):
//!   prestige_from_children: f32 LE
//!   prestige_from_grandkids: f32 LE
//!   prestige_from_eating: f32 LE
//!   prestige_from_followers: f32 LE
//!   prestige_from_wealth: f32 LE
//!   prestige_from_parents: f32 LE
//!   prestige_from_siblings: f32 LE
//! ```
//!
//! Session-only: `alive` (derived on load from death fields), `owns_object`.

use crate::{
    plan_lineage_deletes, LineageNode, PrestigeClass, PrestigeFromBreakdown,
    LINEAGE_BIRTH_UNKNOWN, LINEAGE_STARTING_FAMILY_NAME,
};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::info;

/// Current on-disk lineage record version (writes always use this).
///
/// v1: core identity/parents/gen/prestige/name.  
/// v2: + death_sim_time / age_at_death / death_reason (LINEAGE-24H starving window).  
/// v3: + birth_sim_time (Haxe `Lineage.birthTime`).  
/// v4: + last_said (Haxe `Lineage.lastSaid`).  
/// v5: + my_eve_id (Haxe `Lineage.myEveId`).  
/// v6: + reputation (Haxe `Lineage.reputation` = −lostCombatPrestige).  
/// v7: + coins (Haxe `Lineage.coins` death snapshot).  
/// v8: + family_name (Haxe `Lineage.myFamilyName` / `familyName`).  
/// v9: + po_id (Haxe `Lineage.po_id` person object).  
/// v10: + account_id (Haxe `Lineage.accountId`).  
/// v11: + my_dynasty_id (Haxe `Lineage.myDynastyId`).  
/// v12: + follow_player_id (Haxe `Lineage.followPlayerId`).  
/// v13: + killed_by_player_id (Haxe `Lineage.killedByPlayerId`).  
/// v14: + true_age_at_death (Haxe `Lineage.trueAge`; v2 age_at_death is aging `age`).  
/// v15: + prestigeFrom* breakdown (Haxe GPI; grandkids/parents/siblings were unsaved).
// Haxe: Lineage WriteLineages trueAge L183 / GPI prestigeFrom* L242–248
pub const LINEAGE_FORMAT_VERSION: u32 = 15;
/// Oldest readable version (v1 core without death fields).
pub const LINEAGE_FORMAT_VERSION_MIN: u32 = 1;
const MAGIC: &[u8; 4] = b"OLN1";
/// Sentinel for missing mother/father parent id.
const NONE_PARENT: i32 = -1;

/// Default on-disk name under the save directory.
pub const DEFAULT_LINEAGE_FILE: &str = "lineages_v1.bin";

/// Sidecar for Haxe `WriteLineages` skipped rows (`path` + `.archive`).
// Haxe: Lineage.WriteLineages TODO archive important deleted lineages
pub fn lineage_archive_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    let mut os = path.as_os_str().to_os_string();
    os.push(".archive");
    PathBuf::from(os)
}

/// Write all lineages from `social` to `path` (atomic-ish via temp rename).
pub fn save_lineage_map(lineages: &std::collections::HashMap<i32, LineageNode>, path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    let t0 = Instant::now();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("bin.tmp");
    {
        let f = File::create(&tmp).map_err(|e| e.to_string())?;
        let mut w = BufWriter::with_capacity(64 * 1024, f);
        write_lineages(lineages, &mut w)?;
        w.flush().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    info!(
        path = %path.display(),
        count = lineages.len(),
        ms = t0.elapsed().as_millis() as u64,
        "lineages saved"
    );
    Ok(())
}

/// Load lineages from `path` into a fresh [`SocialState`] (following/exiles empty).
pub fn load_lineage_map(path: impl AsRef<Path>) -> Result<std::collections::HashMap<i32, LineageNode>, String> {
    let path = path.as_ref();
    let t0 = Instant::now();
    let f = File::open(path).map_err(|e| e.to_string())?;
    let mut r = BufReader::with_capacity(64 * 1024, f);
    let lineages = read_lineages(&mut r)?;
    info!(
        path = %path.display(),
        count = lineages.len(),
        ms = t0.elapsed().as_millis() as u64,
        "lineages loaded"
    );
    Ok(lineages)
}

/// Haxe `WriteAllLineages` skip-on-save + sidecar archive.
///
/// In-memory `lineages` is not mutated. Rows [`plan_lineage_deletes`] marks are
/// merged into `{path}.archive` (same OLN2 layout) and omitted from `path`.
// Haxe: Lineage.WriteAllLineages / WriteLineages deleteOld=true
pub fn save_lineage_map_pruned(
    lineages: &HashMap<i32, LineageNode>,
    path: impl AsRef<Path>,
    now_sim: f32,
) -> Result<(), String> {
    let path = path.as_ref();
    let to_delete = plan_lineage_deletes(lineages, now_sim);
    if to_delete.is_empty() {
        return save_lineage_map(lineages, path);
    }
    let mut keep = HashMap::with_capacity(lineages.len().saturating_sub(to_delete.len()));
    let mut archived = HashMap::with_capacity(to_delete.len());
    for (id, n) in lineages {
        if to_delete.contains(id) {
            archived.insert(*id, n.clone());
        } else {
            keep.insert(*id, n.clone());
        }
    }
    merge_lineage_archive(path, &archived)?;
    info!(
        path = %path.display(),
        kept = keep.len(),
        archived = archived.len(),
        "lineages prune-on-save"
    );
    save_lineage_map(&keep, path)
}

fn merge_lineage_archive(
    main: &Path,
    newly: &HashMap<i32, LineageNode>,
) -> Result<(), String> {
    if newly.is_empty() {
        return Ok(());
    }
    let ap = lineage_archive_path(main);
    let mut merged = if ap.exists() {
        load_lineage_map(&ap)?
    } else {
        HashMap::new()
    };
    for (id, n) in newly {
        merged.insert(*id, n.clone());
    }
    save_lineage_map(&merged, &ap)
}

fn write_lineages(lineages: &std::collections::HashMap<i32, LineageNode>, w: &mut impl Write) -> Result<(), String> {
    w.write_all(MAGIC).map_err(|e| e.to_string())?;
    w.write_u32::<LittleEndian>(LINEAGE_FORMAT_VERSION)
        .map_err(|e| e.to_string())?;
    w.write_u32::<LittleEndian>(lineages.len() as u32)
        .map_err(|e| e.to_string())?;

    // Stable order for deterministic files / easier diffs in tests.
    let mut ids: Vec<i32> = lineages.keys().copied().collect();
    ids.sort_unstable();
    for id in ids {
        let n = lineages.get(&id).expect("id from keys");
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
    // OLN3 / LINEAGE-BIRTH-TIME: Haxe WriteLineages birthTime
    // Haxe: Lineage.WriteLineages L181
    w.write_f32::<LittleEndian>(n.birth_sim_time)
        .map_err(|e| e.to_string())?;
    // OLN4 / LINEAGE-LAST-SAID: Haxe WriteLineages lastSaid
    // Haxe: Lineage.WriteLineages L187
    let said_bytes = n.last_said.as_bytes();
    if said_bytes.len() > 4096 {
        return Err(format!("lineage last_said too long ({})", said_bytes.len()));
    }
    w.write_u32::<LittleEndian>(said_bytes.len() as u32)
        .map_err(|e| e.to_string())?;
    w.write_all(said_bytes).map_err(|e| e.to_string())?;
    // OLN5 / LINEAGE-EVE-ID: Haxe WriteLineages myEveId
    // Haxe: Lineage.WriteLineages L191
    w.write_i32::<LittleEndian>(n.my_eve_id)
        .map_err(|e| e.to_string())?;
    // OLN6 / LINEAGE-REP-DISK: Haxe WriteLineages reputation
    // Haxe: Lineage.WriteLineages L199
    let rep = if n.reputation.is_finite() {
        n.reputation
    } else {
        0.0
    };
    w.write_f32::<LittleEndian>(rep)
        .map_err(|e| e.to_string())?;
    // OLN7 / LINEAGE-COINS-DISK: Haxe WriteLineages coins
    // Haxe: Lineage.WriteLineages L189
    let coins = if n.coins.is_finite() { n.coins } else { 0.0 };
    w.write_f32::<LittleEndian>(coins)
        .map_err(|e| e.to_string())?;
    // OLN8 / LINEAGE-FAMILY-NAME: Haxe WriteLineages familyName (eve getter)
    // Haxe: Lineage.WriteLineages L178
    let fam_bytes = n.family_name.as_bytes();
    if fam_bytes.len() > 4096 {
        return Err(format!("lineage family_name too long ({})", fam_bytes.len()));
    }
    w.write_u32::<LittleEndian>(fam_bytes.len() as u32)
        .map_err(|e| e.to_string())?;
    w.write_all(fam_bytes).map_err(|e| e.to_string())?;
    // OLN9 / LINEAGE-PO-ID: Haxe WriteLineages po_id
    // Haxe: Lineage.WriteLineages L180
    w.write_i32::<LittleEndian>(n.po_id)
        .map_err(|e| e.to_string())?;
    // OLN10 / LINEAGE-ACCOUNT-ID: Haxe WriteLineages accountId
    // Haxe: Lineage.WriteLineages L174
    w.write_i32::<LittleEndian>(n.account_id)
        .map_err(|e| e.to_string())?;
    // OLN11 / LINEAGE-DYNASTY: Haxe WriteLineages myDynastyId
    // Haxe: Lineage.WriteLineages L197
    w.write_i32::<LittleEndian>(n.my_dynasty_id)
        .map_err(|e| e.to_string())?;
    // OLN12 / LINEAGE-FOLLOW-ID: Haxe WriteLineages followPlayerId
    // Haxe: Lineage.WriteLineages L205
    w.write_i32::<LittleEndian>(n.follow_player_id)
        .map_err(|e| e.to_string())?;
    // OLN13 / LINEAGE-KILLED-BY: Haxe WriteLineages killedByPlayerId
    // Haxe: Lineage.WriteLineages L204
    w.write_i32::<LittleEndian>(n.killed_by_player_id)
        .map_err(|e| e.to_string())?;
    // OLN14 / LINEAGE-TRUE-AGE: Haxe WriteLineages trueAge
    // Haxe: Lineage.WriteLineages L183
    let true_age = if n.true_age_at_death.is_finite() {
        n.true_age_at_death
    } else {
        0.0
    };
    w.write_f32::<LittleEndian>(true_age)
        .map_err(|e| e.to_string())?;
    // OLN15 / PRESTIGE-FAMILY-PERSIST: Haxe GPI prestigeFrom* (incl. unsaved TODOs)
    // Haxe: GPI L242–248
    let pf = n.prestige_from;
    for v in [
        pf.children,
        pf.grandkids,
        pf.eating,
        pf.followers,
        pf.wealth,
        pf.parents,
        pf.siblings,
    ] {
        let v = if v.is_finite() { v } else { 0.0 };
        w.write_f32::<LittleEndian>(v)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn read_lineages(r: &mut impl Read) -> Result<HashMap<i32, LineageNode>, String> {
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
    Ok(lineages)
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

    // OLN3 birthTime; v1/v2 unknown (not epoch 0).
    // Haxe: Lineage.ReadLineages birthTime
    let birth_sim_time = if version >= 3 {
        let t = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
        if t.is_finite() {
            t
        } else {
            LINEAGE_BIRTH_UNKNOWN
        }
    } else {
        LINEAGE_BIRTH_UNKNOWN
    };

    // OLN4 lastSaid; v1–v3 empty.
    // Haxe: Lineage.ReadLineages lastSaid L273
    let last_said = if version >= 4 {
        let said_len = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
        if said_len > 4096 {
            return Err(format!("lineage last_said too long ({said_len})"));
        }
        let mut said_buf = vec![0u8; said_len];
        r.read_exact(&mut said_buf).map_err(|e| e.to_string())?;
        String::from_utf8(said_buf).map_err(|e| e.to_string())?
    } else {
        String::new()
    };

    // OLN5 myEveId; v1–v4 unset (0). Haxe default is −1; Rust 0 = walk mother / self.
    // Haxe: Lineage.ReadLineages myEveId L277
    let my_eve_id = if version >= 5 {
        r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?
    } else {
        0
    };

    // OLN6 reputation; v1–v5 default 0.
    // Haxe: Lineage.ReadLineages reputation L286
    let reputation = if version >= 6 {
        let t = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
        if t.is_finite() {
            t
        } else {
            0.0
        }
    } else {
        0.0
    };

    // OLN7 coins; v1–v6 default 0.
    // Haxe: Lineage.ReadLineages coins L274
    let coins = if version >= 7 {
        let t = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
        if t.is_finite() {
            t
        } else {
            0.0
        }
    } else {
        0.0
    };

    // OLN8 familyName; v1–v7 StartingFamilyName.
    // Haxe: Lineage.ReadLineages myFamilyName L264
    let family_name = if version >= 8 {
        let fam_len = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
        if fam_len > 4096 {
            return Err(format!("lineage family_name too long ({fam_len})"));
        }
        let mut fam_buf = vec![0u8; fam_len];
        r.read_exact(&mut fam_buf).map_err(|e| e.to_string())?;
        let s = String::from_utf8(fam_buf).map_err(|e| e.to_string())?;
        if s.trim().is_empty() {
            LINEAGE_STARTING_FAMILY_NAME.to_string()
        } else {
            s
        }
    } else {
        LINEAGE_STARTING_FAMILY_NAME.to_string()
    };

    // OLN9 po_id; v1–v8 unset (−1).
    // Haxe: Lineage.ReadLineages po_id L266
    let po_id = if version >= 9 {
        r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?
    } else {
        -1
    };

    // OLN10 accountId; v1–v9 unset (0).
    // Haxe: Lineage.ReadLineages accountId L260
    let account_id = if version >= 10 {
        r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?
    } else {
        0
    };

    // OLN11 myDynastyId; v1–v10 unset (−1).
    // Haxe: Lineage.ReadLineages myDynastyId L284
    let my_dynasty_id = if version >= 11 {
        r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?
    } else {
        -1
    };

    // OLN12 followPlayerId; v1–v11 none (0).
    // Haxe: Lineage.ReadLineages followPlayerId L293
    let follow_player_id = if version >= 12 {
        r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?
    } else {
        0
    };

    // OLN13 killedByPlayerId; v1–v12 none (0).
    // Haxe: Lineage.ReadLineages killedByPlayerId L292
    let killed_by_player_id = if version >= 13 {
        r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?
    } else {
        0
    };

    // OLN14 trueAge; v1–v13 copy age_at_death (legacy stored true_age there).
    // Haxe: Lineage.ReadLineages trueAge L269
    let true_age_at_death = if version >= 14 {
        let t = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
        if t.is_finite() {
            t.max(0.0)
        } else {
            0.0
        }
    } else {
        age_at_death
    };

    // OLN15 prestigeFrom*; v1–v14 default 0.
    // Haxe: GPI.prestigeFromChildren/Grandkids/Eating/Followers/Wealth/Parents/Siblings
    let prestige_from = if version >= 15 {
        let mut read_f = || -> Result<f32, String> {
            let t = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
            Ok(if t.is_finite() { t } else { 0.0 })
        };
        PrestigeFromBreakdown {
            children: read_f()?,
            grandkids: read_f()?,
            eating: read_f()?,
            followers: read_f()?,
            wealth: read_f()?,
            parents: read_f()?,
            siblings: read_f()?,
        }
    } else {
        PrestigeFromBreakdown::default()
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
        true_age_at_death,
        birth_sim_time,
        my_eve_id,
        last_said,
        reputation,
        coins,
        family_name,
        po_id,
        account_id,
        my_dynasty_id,
        follow_player_id,
        killed_by_player_id,
        prestige_from,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lineage_can_be_deleted, LINEAGE_DELETE_AGE_FACTOR};
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

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

    fn dead(id: i32, name: &str, age: f32, death_sim: f32) -> LineageNode {
        let mut n = LineageNode::eve(id, name);
        n.stamp_death(death_sim, "reason_hunger", age);
        n
    }

    #[test]
    fn prune_save_skips_old_dead_and_merges_archive_sidecar() {
        let dir = unique_temp_dir("ol_lineage_archive");
        let path = dir.join("lineages_v1.bin");
        let death = 100.0;
        let keep_secs = 60.0 * 60.0 * 24.0 * LINEAGE_DELETE_AGE_FACTOR * 60.0;
        let now = death + keep_secs + 60.0;

        let mut map = HashMap::new();
        map.insert(1, LineageNode::eve(1, "EVE"));
        let mut kid = LineageNode::with_mother(2, "KID", map.get(&1).unwrap());
        kid.stamp_death(now - 60.0, "reason_hunger", 10.0);
        map.insert(2, kid);
        map.insert(9, dead(9, "STRANGER", 60.0, death));
        assert!(lineage_can_be_deleted(map.get(&9).unwrap(), now));

        save_lineage_map_pruned(&map, &path, now).unwrap();
        let loaded = load_lineage_map(&path).unwrap();
        assert!(loaded.contains_key(&1));
        assert!(loaded.contains_key(&2));
        assert!(!loaded.contains_key(&9));
        assert_eq!(loaded.len(), 2);

        let archived = load_lineage_map(lineage_archive_path(&path)).unwrap();
        assert_eq!(archived.len(), 1);
        assert_eq!(archived.get(&9).unwrap().name, "STRANGER");
        assert!(!archived.get(&9).unwrap().alive);

        // Second prune merges, does not drop prior archive rows.
        save_lineage_map_pruned(&map, &path, now).unwrap();
        let archived2 = load_lineage_map(lineage_archive_path(&path)).unwrap();
        assert!(archived2.contains_key(&9));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prune_save_writes_full_map_when_nothing_expired() {
        let dir = unique_temp_dir("ol_lineage_prune_keep");
        let path = dir.join("lineages_v1.bin");
        let mut map = HashMap::new();
        map.insert(1, LineageNode::eve(1, "EVE"));
        save_lineage_map_pruned(&map, &path, 1_000.0).unwrap();
        let loaded = load_lineage_map(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(!lineage_archive_path(&path).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN3 / LINEAGE-BIRTH-TIME: Haxe `birthTime` survives disk roundtrip.
    // Haxe: Lineage.WriteLineages / ReadLineages birthTime
    #[test]
    fn roundtrip_preserves_birth_time_oln3() {
        let dir = unique_temp_dir("ol_lineage_birth_oln3");
        let path = dir.join("lineages_v1.bin");
        let mut n = LineageNode::eve(1, "EVE");
        n.stamp_birth(1_234.5);
        let mut map = HashMap::new();
        map.insert(1, n);
        save_lineage_map(&map, &path).unwrap();
        let loaded = load_lineage_map(&path).unwrap();
        let got = loaded.get(&1).unwrap();
        assert!(got.has_birth_time());
        assert!((got.birth_sim_time - 1_234.5).abs() < 1e-3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN4 / LINEAGE-LAST-SAID: Haxe `lastSaid` survives disk roundtrip.
    // Haxe: Lineage.WriteLineages / ReadLineages lastSaid
    #[test]
    fn roundtrip_preserves_last_said_oln4() {
        let dir = unique_temp_dir("ol_lineage_last_said_oln4");
        let path = dir.join("lineages_v1.bin");
        let mut n = LineageNode::eve(1, "EVE");
        n.last_said = "HELLO WORLD".into();
        let mut map = HashMap::new();
        map.insert(1, n);
        save_lineage_map(&map, &path).unwrap();
        let loaded = load_lineage_map(&path).unwrap();
        assert_eq!(loaded.get(&1).unwrap().last_said, "HELLO WORLD");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN5 / LINEAGE-EVE-ID: Haxe `myEveId` survives disk roundtrip (Eve + child).
    // Haxe: Lineage.WriteLineages / ReadLineages myEveId
    #[test]
    fn roundtrip_preserves_my_eve_id_oln5() {
        let dir = unique_temp_dir("ol_lineage_eve_id_oln5");
        let path = dir.join("lineages_v1.bin");
        let eve = LineageNode::eve(1, "EVE");
        let kid = LineageNode::with_mother(2, "KID", &eve);
        let mut map = HashMap::new();
        map.insert(1, eve);
        map.insert(2, kid);
        save_lineage_map(&map, &path).unwrap();
        let loaded = load_lineage_map(&path).unwrap();
        assert_eq!(loaded.get(&1).unwrap().my_eve_id, 1);
        assert_eq!(loaded.get(&2).unwrap().my_eve_id, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN6 / LINEAGE-REP-DISK: Haxe `reputation` survives disk roundtrip.
    // Haxe: Lineage.WriteLineages / ReadLineages reputation
    #[test]
    fn roundtrip_preserves_reputation_oln6() {
        let dir = unique_temp_dir("ol_lineage_rep_oln6");
        let path = dir.join("lineages_v1.bin");
        let mut n = LineageNode::eve(1, "EVE");
        n.stamp_reputation_from_lost_combat(12.5);
        let mut map = HashMap::new();
        map.insert(1, n);
        save_lineage_map(&map, &path).unwrap();
        let loaded = load_lineage_map(&path).unwrap();
        let got = loaded.get(&1).unwrap();
        assert!((got.reputation + 12.5).abs() < 1e-4);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN7 / LINEAGE-COINS-DISK: Haxe `coins` death snapshot survives disk roundtrip.
    // Haxe: Lineage.WriteLineages / ReadLineages coins
    #[test]
    fn roundtrip_preserves_coins_oln7() {
        let dir = unique_temp_dir("ol_lineage_coins_oln7");
        let path = dir.join("lineages_v1.bin");
        let mut n = LineageNode::eve(1, "EVE");
        n.stamp_coins(42.0);
        let mut map = HashMap::new();
        map.insert(1, n);
        save_lineage_map(&map, &path).unwrap();
        let loaded = load_lineage_map(&path).unwrap();
        assert!((loaded.get(&1).unwrap().coins - 42.0).abs() < 1e-4);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN8 / LINEAGE-FAMILY-NAME: Haxe `myFamilyName` survives disk roundtrip.
    // Haxe: Lineage.WriteLineages / ReadLineages familyName
    #[test]
    fn roundtrip_preserves_family_name_oln8() {
        let dir = unique_temp_dir("ol_lineage_family_oln8");
        let path = dir.join("lineages_v1.bin");
        let mut n = LineageNode::eve(1, "EVE");
        n.stamp_family_name("JONES");
        let mut map = HashMap::new();
        map.insert(1, n);
        save_lineage_map(&map, &path).unwrap();
        let loaded = load_lineage_map(&path).unwrap();
        assert_eq!(loaded.get(&1).unwrap().family_name, "JONES");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Child copies mother's myFamilyName (Haxe familyName getter / Eve name).
    #[test]
    fn with_mother_copies_family_name() {
        let mut eve = LineageNode::eve(1, "EVE");
        eve.stamp_family_name("BAKER");
        let kid = LineageNode::with_mother(2, "KID", &eve);
        assert_eq!(kid.family_name, "BAKER");
        assert_eq!(kid.po_id, -1);
    }

    /// OLN9 / LINEAGE-PO-ID: Haxe `po_id` survives disk roundtrip.
    // Haxe: Lineage.WriteLineages / ReadLineages po_id
    #[test]
    fn roundtrip_preserves_po_id_oln9() {
        let dir = unique_temp_dir("ol_lineage_po_id_oln9");
        let path = dir.join("lineages_v1.bin");
        let mut n = LineageNode::eve(1, "EVE");
        n.stamp_po_id(19);
        let mut map = HashMap::new();
        map.insert(1, n);
        save_lineage_map(&map, &path).unwrap();
        let loaded = load_lineage_map(&path).unwrap();
        assert_eq!(loaded.get(&1).unwrap().po_id, 19);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Child does not copy mother's accountId (Haxe Lineage.new uses player.account.id).
    #[test]
    fn with_mother_does_not_copy_account_id() {
        let mut eve = LineageNode::eve(1, "EVE");
        eve.stamp_account_id(7);
        let kid = LineageNode::with_mother(2, "KID", &eve);
        assert_eq!(kid.account_id, 0);
        assert_eq!(eve.account_id, 7);
    }

    /// OLN10 / LINEAGE-ACCOUNT-ID: Haxe `accountId` survives disk roundtrip.
    // Haxe: Lineage.WriteLineages / ReadLineages accountId
    #[test]
    fn roundtrip_preserves_account_id_oln10() {
        let dir = unique_temp_dir("ol_lineage_account_id_oln10");
        let path = dir.join("lineages_v1.bin");
        let mut n = LineageNode::eve(1, "EVE");
        n.stamp_account_id(42);
        let mut map = HashMap::new();
        map.insert(1, n);
        save_lineage_map(&map, &path).unwrap();
        let loaded = load_lineage_map(&path).unwrap();
        assert_eq!(loaded.get(&1).unwrap().account_id, 42);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Child does not copy mother's myDynastyId (Haxe Lineage.new leaves default −1).
    #[test]
    fn with_mother_does_not_copy_dynasty_id() {
        let mut eve = LineageNode::eve(1, "EVE");
        eve.stamp_dynasty_id(1);
        let kid = LineageNode::with_mother(2, "KID", &eve);
        assert_eq!(kid.my_dynasty_id, -1);
        assert_eq!(eve.my_dynasty_id, 1);
    }

    /// OLN11 / LINEAGE-DYNASTY: Haxe `myDynastyId` survives disk roundtrip.
    // Haxe: Lineage.WriteLineages / ReadLineages myDynastyId
    #[test]
    fn roundtrip_preserves_my_dynasty_id_oln11() {
        let dir = unique_temp_dir("ol_lineage_dynasty_oln11");
        let path = dir.join("lineages_v1.bin");
        let mut n = LineageNode::eve(1, "EVE");
        n.stamp_dynasty_id(9);
        let mut map = HashMap::new();
        map.insert(1, n);
        save_lineage_map(&map, &path).unwrap();
        let loaded = load_lineage_map(&path).unwrap();
        assert_eq!(loaded.get(&1).unwrap().my_dynasty_id, 9);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Child does not copy mother's followPlayerId (Haxe Lineage.new leaves default 0).
    #[test]
    fn with_mother_does_not_copy_follow_player_id() {
        let mut eve = LineageNode::eve(1, "EVE");
        eve.stamp_follow_player_id(9);
        let kid = LineageNode::with_mother(2, "KID", &eve);
        assert_eq!(kid.follow_player_id, 0);
        assert_eq!(eve.follow_player_id, 9);
    }

    /// OLN12 / LINEAGE-FOLLOW-ID: Haxe `followPlayerId` survives disk roundtrip.
    // Haxe: Lineage.WriteLineages / ReadLineages followPlayerId
    #[test]
    fn roundtrip_preserves_follow_player_id_oln12() {
        let dir = unique_temp_dir("ol_lineage_follow_oln12");
        let path = dir.join("lineages_v1.bin");
        let mut n = LineageNode::eve(1, "EVE");
        n.stamp_follow_player_id(7);
        let mut map = HashMap::new();
        map.insert(1, n);
        save_lineage_map(&map, &path).unwrap();
        let loaded = load_lineage_map(&path).unwrap();
        assert_eq!(loaded.get(&1).unwrap().follow_player_id, 7);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Child does not copy mother's killedByPlayerId.
    #[test]
    fn with_mother_does_not_copy_killed_by_player_id() {
        let mut eve = LineageNode::eve(1, "EVE");
        eve.stamp_killed_by_player_id(9);
        let kid = LineageNode::with_mother(2, "KID", &eve);
        assert_eq!(kid.killed_by_player_id, 0);
        assert_eq!(eve.killed_by_player_id, 9);
    }

    /// OLN13 / LINEAGE-KILLED-BY: Haxe `killedByPlayerId` survives disk roundtrip.
    // Haxe: Lineage.WriteLineages / ReadLineages killedByPlayerId
    #[test]
    fn roundtrip_preserves_killed_by_player_id_oln13() {
        let dir = unique_temp_dir("ol_lineage_killed_oln13");
        let path = dir.join("lineages_v1.bin");
        let mut n = LineageNode::eve(1, "EVE");
        n.stamp_killed_by_player_id(4);
        let mut map = HashMap::new();
        map.insert(1, n);
        save_lineage_map(&map, &path).unwrap();
        let loaded = load_lineage_map(&path).unwrap();
        assert_eq!(loaded.get(&1).unwrap().killed_by_player_id, 4);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN14 / LINEAGE-TRUE-AGE: Haxe `trueAge` survives disk; aging `age` stays separate.
    // Haxe: Lineage.WriteLineages / ReadLineages age + trueAge
    #[test]
    fn roundtrip_preserves_true_age_at_death_oln14() {
        let dir = unique_temp_dir("ol_lineage_true_age_oln14");
        let path = dir.join("lineages_v1.bin");
        let mut n = LineageNode::eve(1, "EVE");
        n.stamp_death_ages(50.0, "reason_hunger", 10.0, 14.0);
        let mut map = HashMap::new();
        map.insert(1, n);
        save_lineage_map(&map, &path).unwrap();
        let loaded = load_lineage_map(&path).unwrap();
        let g = loaded.get(&1).unwrap();
        assert!((g.age_at_death - 10.0).abs() < 1e-4);
        assert!((g.true_age_at_death - 14.0).abs() < 1e-4);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN15 / PRESTIGE-FAMILY-PERSIST: Haxe prestigeFrom* including grandkids/parents/siblings.
    // Haxe: GPI.prestigeFrom* L242–248
    #[test]
    fn roundtrip_preserves_prestige_from_breakdown_oln15() {
        let dir = unique_temp_dir("ol_lineage_prestige_from_oln15");
        let path = dir.join("lineages_v1.bin");
        let mut n = LineageNode::eve(1, "EVE");
        n.prestige_from.children = 1.5;
        n.prestige_from.grandkids = 2.5;
        n.prestige_from.eating = 3.5;
        n.prestige_from.followers = 4.5;
        n.prestige_from.wealth = 5.5;
        n.prestige_from.parents = 6.5;
        n.prestige_from.siblings = 7.5;
        let mut map = HashMap::new();
        map.insert(1, n);
        save_lineage_map(&map, &path).unwrap();
        let loaded = load_lineage_map(&path).unwrap();
        let g = loaded.get(&1).unwrap().prestige_from;
        assert!((g.children - 1.5).abs() < 1e-4);
        assert!((g.grandkids - 2.5).abs() < 1e-4);
        assert!((g.eating - 3.5).abs() < 1e-4);
        assert!((g.followers - 4.5).abs() < 1e-4);
        assert!((g.wealth - 5.5).abs() < 1e-4);
        assert!((g.parents - 6.5).abs() < 1e-4);
        assert!((g.siblings - 7.5).abs() < 1e-4);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

