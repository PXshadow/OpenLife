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
//!   birth_sim_time: f32 LE
//!   # version >= 4 (LINEAGE-LAST-SAID / Haxe lastSaid):
//!   last_said_len: u32 LE
//!   last_said: [u8; last_said_len]
//!   # version >= 5 (LINEAGE-EVE-ID / Haxe myEveId):
//!   my_eve_id: i32 LE
//!   # version >= 6 (LINEAGE-REP-DISK / Haxe reputation):
//!   reputation: f32 LE
//!   # version >= 7 (LINEAGE-COINS-DISK / Haxe coins):
//!   coins: f32 LE
//!   # version >= 8 (LINEAGE-FAMILY-NAME / Haxe familyName):
//!   family_name_len: u32 LE
//!   family_name: [u8; family_name_len]
//!   # version >= 9 (LINEAGE-PO-ID / Haxe po_id):
//!   po_id: i32 LE
//!   # version >= 10 (LINEAGE-ACCOUNT-ID / Haxe accountId):
//!   account_id: i32 LE
//!   # version >= 11 (LINEAGE-DYNASTY / Haxe myDynastyId):
//!   my_dynasty_id: i32 LE
//!   # version >= 12 (LINEAGE-FOLLOW-ID / Haxe followPlayerId):
//!   follow_player_id: i32 LE
//!   # version >= 13 (LINEAGE-KILLED-BY / Haxe killedByPlayerId):
//!   killed_by_player_id: i32 LE
//!   # version >= 14 (LINEAGE-TRUE-AGE / Haxe trueAge):
//!   true_age_at_death: f32 LE
//!   # version >= 15 (PRESTIGE-FAMILY-PERSIST): seven prestigeFrom* f32
//! ```
//!
//! Session-only: `alive` (derived on load from death fields), `owns_object`.

use crate::prestige::PrestigeClass;
use crate::social::{LineageNode, SocialState};
use ol_identity::{
    load_lineage_map, save_lineage_map, save_lineage_map_pruned, LINEAGE_BIRTH_UNKNOWN,
    LINEAGE_STARTING_FAMILY_NAME,
};
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
/// v3: + birth_sim_time (Haxe `Lineage.birthTime`).  
/// v4: + last_said (Haxe `Lineage.lastSaid`).  
/// v5: + my_eve_id (Haxe `Lineage.myEveId`).  
/// v6: + reputation (Haxe `Lineage.reputation`).  
/// v7: + coins (Haxe `Lineage.coins` death snapshot).  
/// v8: + family_name (Haxe `Lineage.myFamilyName`).  
/// v9: + po_id (Haxe `Lineage.po_id`).  
/// v10: + account_id (Haxe `Lineage.accountId`).  
/// v11: + my_dynasty_id (Haxe `Lineage.myDynastyId`).  
/// v12: + follow_player_id (Haxe `Lineage.followPlayerId`).  
/// v13: + killed_by_player_id (Haxe `Lineage.killedByPlayerId`).  
/// v14: + true_age_at_death (Haxe `Lineage.trueAge`).  
/// v15: + prestigeFrom* breakdown.
// Haxe: Lineage WriteLineages trueAge L183
pub const LINEAGE_FORMAT_VERSION: u32 = 15;
/// Oldest readable version (v1 core without death fields).
pub const LINEAGE_FORMAT_VERSION_MIN: u32 = 1;
const MAGIC: &[u8; 4] = b"OLN1";
/// Sentinel for missing mother/father parent id.
const NONE_PARENT: i32 = -1;

/// Default on-disk name under the save directory.
pub const DEFAULT_LINEAGE_FILE: &str = "lineages_v1.bin";

/// Write lineages from `social` to `path` (atomic-ish via temp rename).
///
/// When [`SocialState::sim_time`] > 0, skip Haxe `canBeDeleted` rows and merge
/// them into `{path}.archive` (in-memory map stays full). `sim_time == 0` writes
/// the full map so persist tests / pre-tick boot saves stay unchanged.
// Haxe: Lineage.WriteAllLineages deleteOld=true
pub fn save_lineages(social: &SocialState, path: impl AsRef<Path>) -> Result<(), String> {
    if social.sim_time > 0.0 {
        save_lineage_map_pruned(&social.lineages, path, social.sim_time)
    } else {
        save_lineage_map(&social.lineages, path)
    }
}

/// Load lineages from `path` into a fresh [`SocialState`].
///
/// Restores session `following` from OLN `followPlayerId` (Haxe WriteLineages field).
pub fn load_lineages(path: impl AsRef<Path>) -> Result<SocialState, String> {
    let lineages = load_lineage_map(path)?;
    let mut social = SocialState {
        lineages,
        ..SocialState::default()
    };
    social.restore_following_from_lineages();
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
    // OLN3 / LINEAGE-BIRTH-TIME
    // Haxe: Lineage.WriteLineages L181
    w.write_f32::<LittleEndian>(n.birth_sim_time)
        .map_err(|e| e.to_string())?;
    // OLN4 / LINEAGE-LAST-SAID
    // Haxe: Lineage.WriteLineages L187
    let said_bytes = n.last_said.as_bytes();
    if said_bytes.len() > 4096 {
        return Err(format!("lineage last_said too long ({})", said_bytes.len()));
    }
    w.write_u32::<LittleEndian>(said_bytes.len() as u32)
        .map_err(|e| e.to_string())?;
    w.write_all(said_bytes).map_err(|e| e.to_string())?;
    // OLN5 / LINEAGE-EVE-ID
    // Haxe: Lineage.WriteLineages L191
    w.write_i32::<LittleEndian>(n.my_eve_id)
        .map_err(|e| e.to_string())?;
    // OLN6 / LINEAGE-REP-DISK
    // Haxe: Lineage.WriteLineages L199
    let rep = if n.reputation.is_finite() {
        n.reputation
    } else {
        0.0
    };
    w.write_f32::<LittleEndian>(rep)
        .map_err(|e| e.to_string())?;
    // OLN7 / LINEAGE-COINS-DISK
    // Haxe: Lineage.WriteLineages L189
    let coins = if n.coins.is_finite() { n.coins } else { 0.0 };
    w.write_f32::<LittleEndian>(coins)
        .map_err(|e| e.to_string())?;
    // OLN8 / LINEAGE-FAMILY-NAME
    // Haxe: Lineage.WriteLineages L178
    let fam_bytes = n.family_name.as_bytes();
    if fam_bytes.len() > 4096 {
        return Err(format!("lineage family_name too long ({})", fam_bytes.len()));
    }
    w.write_u32::<LittleEndian>(fam_bytes.len() as u32)
        .map_err(|e| e.to_string())?;
    w.write_all(fam_bytes).map_err(|e| e.to_string())?;
    // OLN9 / LINEAGE-PO-ID
    // Haxe: Lineage.WriteLineages L180
    w.write_i32::<LittleEndian>(n.po_id)
        .map_err(|e| e.to_string())?;
    // OLN10 / LINEAGE-ACCOUNT-ID
    // Haxe: Lineage.WriteLineages L174
    w.write_i32::<LittleEndian>(n.account_id)
        .map_err(|e| e.to_string())?;
    // OLN11 / LINEAGE-DYNASTY
    // Haxe: Lineage.WriteLineages L197
    w.write_i32::<LittleEndian>(n.my_dynasty_id)
        .map_err(|e| e.to_string())?;
    // OLN12 / LINEAGE-FOLLOW-ID
    // Haxe: Lineage.WriteLineages L205
    w.write_i32::<LittleEndian>(n.follow_player_id)
        .map_err(|e| e.to_string())?;
    // OLN13 / LINEAGE-KILLED-BY
    // Haxe: Lineage.WriteLineages L204
    w.write_i32::<LittleEndian>(n.killed_by_player_id)
        .map_err(|e| e.to_string())?;
    // OLN14 / LINEAGE-TRUE-AGE
    // Haxe: Lineage.WriteLineages L183
    let true_age = if n.true_age_at_death.is_finite() {
        n.true_age_at_death
    } else {
        0.0
    };
    w.write_f32::<LittleEndian>(true_age)
        .map_err(|e| e.to_string())?;
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

    let my_eve_id = if version >= 5 {
        r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?
    } else {
        0
    };

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

    let po_id = if version >= 9 {
        r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?
    } else {
        -1
    };

    let account_id = if version >= 10 {
        r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?
    } else {
        0
    };

    let my_dynasty_id = if version >= 11 {
        r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?
    } else {
        -1
    };

    let follow_player_id = if version >= 12 {
        r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?
    } else {
        0
    };

    let killed_by_player_id = if version >= 13 {
        r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?
    } else {
        0
    };

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

    let prestige_from = if version >= 15 {
        let mut read_f = || -> Result<f32, String> {
            let t = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
            Ok(if t.is_finite() { t } else { 0.0 })
        };
        ol_identity::PrestigeFromBreakdown {
            children: read_f()?,
            grandkids: read_f()?,
            eating: read_f()?,
            followers: read_f()?,
            wealth: read_f()?,
            parents: read_f()?,
            siblings: read_f()?,
        }
    } else {
        ol_identity::PrestigeFromBreakdown::default()
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

impl SocialState {
    /// Persist lineages (OLN includes `followPlayerId`; session exile map is not).
    pub fn save_lineages_file(&self, path: impl AsRef<Path>) -> Result<(), String> {
        save_lineages(self, path)
    }

    /// Replace `self.lineages` from file; keeps following/exiles/colors.
    pub fn load_lineages_file(&mut self, path: impl AsRef<Path>) -> Result<(), String> {
        let loaded = load_lineages(path)?;
        self.lineages = loaded.lineages;
        Ok(())
    }

    /// Build a SocialState from a lineage save (restores following from OLN).
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
            true_age_at_death: 0.0,
            birth_sim_time: LINEAGE_BIRTH_UNKNOWN,
            my_eve_id: 2,
            last_said: String::new(),
            reputation: 0.0,
            coins: 0.0,
            family_name: LINEAGE_STARTING_FAMILY_NAME.to_string(),
            po_id: -1,
            account_id: 0,
            my_dynasty_id: -1,
            follow_player_id: 0,
            killed_by_player_id: 0,
            prestige_from: ol_identity::PrestigeFromBreakdown::default(),
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
                true_age_at_death: 0.0,
                birth_sim_time: LINEAGE_BIRTH_UNKNOWN,
                my_eve_id: 3,
                last_said: String::new(),
                reputation: 0.0,
                coins: 0.0,
                family_name: LINEAGE_STARTING_FAMILY_NAME.to_string(),
                po_id: -1,
                account_id: 0,
                my_dynasty_id: -1,
                follow_player_id: 0,
                killed_by_player_id: 0,
                prestige_from: ol_identity::PrestigeFromBreakdown::default(),
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
        assert!(!eve.has_birth_time());

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
        assert!(!n.has_birth_time());
        assert_eq!(n.my_eve_id, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN3 / LINEAGE-BIRTH-TIME: birthTime roundtrip via SocialState save/load.
    // Haxe: Lineage.WriteLineages / ReadLineages birthTime
    #[test]
    fn roundtrip_preserves_birth_time_oln3() {
        let dir = unique_temp_dir("ol_lineage_birth_oln3_sim");
        let path = dir.join("lineages_v1.bin");
        let mut original = sample_social();
        if let Some(n) = original.lineages.get_mut(&10) {
            n.stamp_birth(880.0);
        }
        save_lineages(&original, &path).unwrap();
        let loaded = load_lineages(&path).unwrap();
        let alice = loaded.lineages.get(&10).unwrap();
        assert!(alice.has_birth_time());
        assert!((alice.birth_sim_time - 880.0).abs() < 1e-3);
        let eve = loaded.lineages.get(&2).unwrap();
        assert!(!eve.has_birth_time());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN4 / LINEAGE-LAST-SAID: lastSaid roundtrip via SocialState save/load.
    // Haxe: Lineage.WriteLineages / ReadLineages lastSaid
    #[test]
    fn roundtrip_preserves_last_said_oln4() {
        let dir = unique_temp_dir("ol_lineage_last_said_oln4_sim");
        let path = dir.join("lineages_v1.bin");
        let mut original = sample_social();
        if let Some(n) = original.lineages.get_mut(&10) {
            n.last_said = "I AM HUNGRY".into();
        }
        save_lineages(&original, &path).unwrap();
        let loaded = load_lineages(&path).unwrap();
        assert_eq!(loaded.lineages.get(&10).unwrap().last_said, "I AM HUNGRY");
        assert!(loaded.lineages.get(&2).unwrap().last_said.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN5 / LINEAGE-EVE-ID: myEveId roundtrip via SocialState save/load.
    // Haxe: Lineage.WriteLineages / ReadLineages myEveId
    #[test]
    fn roundtrip_preserves_my_eve_id_oln5() {
        let dir = unique_temp_dir("ol_lineage_eve_id_oln5_sim");
        let path = dir.join("lineages_v1.bin");
        let original = sample_social();
        save_lineages(&original, &path).unwrap();
        let loaded = load_lineages(&path).unwrap();
        assert_eq!(loaded.lineages.get(&2).unwrap().my_eve_id, 2);
        assert_eq!(loaded.lineages.get(&10).unwrap().my_eve_id, 2);
        assert_eq!(loaded.lineages.get(&3).unwrap().my_eve_id, 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN6 / LINEAGE-REP-DISK: reputation roundtrip via SocialState save/load.
    // Haxe: Lineage.WriteLineages / ReadLineages reputation
    #[test]
    fn roundtrip_preserves_reputation_oln6() {
        let dir = unique_temp_dir("ol_lineage_rep_oln6_sim");
        let path = dir.join("lineages_v1.bin");
        let mut original = sample_social();
        if let Some(n) = original.lineages.get_mut(&10) {
            n.stamp_reputation_from_lost_combat(8.0);
        }
        save_lineages(&original, &path).unwrap();
        let loaded = load_lineages(&path).unwrap();
        assert!((loaded.lineages.get(&10).unwrap().reputation + 8.0).abs() < 1e-4);
        assert_eq!(loaded.lineages.get(&2).unwrap().reputation, 0.0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN7 / LINEAGE-COINS-DISK: coins death snapshot roundtrip via SocialState.
    // Haxe: Lineage.WriteLineages / ReadLineages coins
    #[test]
    fn roundtrip_preserves_coins_oln7() {
        let dir = unique_temp_dir("ol_lineage_coins_oln7_sim");
        let path = dir.join("lineages_v1.bin");
        let mut original = sample_social();
        if let Some(n) = original.lineages.get_mut(&10) {
            n.stamp_coins(19.0);
        }
        save_lineages(&original, &path).unwrap();
        let loaded = load_lineages(&path).unwrap();
        assert!((loaded.lineages.get(&10).unwrap().coins - 19.0).abs() < 1e-4);
        assert_eq!(loaded.lineages.get(&2).unwrap().coins, 0.0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN8 / LINEAGE-FAMILY-NAME: familyName roundtrip via SocialState.
    // Haxe: Lineage.WriteLineages / ReadLineages familyName
    #[test]
    fn roundtrip_preserves_family_name_oln8() {
        let dir = unique_temp_dir("ol_lineage_family_oln8_sim");
        let path = dir.join("lineages_v1.bin");
        let mut original = sample_social();
        if let Some(n) = original.lineages.get_mut(&10) {
            n.stamp_family_name("JONES");
        }
        save_lineages(&original, &path).unwrap();
        let loaded = load_lineages(&path).unwrap();
        assert_eq!(loaded.lineages.get(&10).unwrap().family_name, "JONES");
        assert_eq!(
            loaded.lineages.get(&2).unwrap().family_name,
            LINEAGE_STARTING_FAMILY_NAME
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN9 / LINEAGE-PO-ID: po_id roundtrip via SocialState.
    // Haxe: Lineage.WriteLineages / ReadLineages po_id
    #[test]
    fn roundtrip_preserves_po_id_oln9() {
        let dir = unique_temp_dir("ol_lineage_po_id_oln9_sim");
        let path = dir.join("lineages_v1.bin");
        let mut original = sample_social();
        if let Some(n) = original.lineages.get_mut(&10) {
            n.stamp_po_id(19);
        }
        save_lineages(&original, &path).unwrap();
        let loaded = load_lineages(&path).unwrap();
        assert_eq!(loaded.lineages.get(&10).unwrap().po_id, 19);
        assert_eq!(loaded.lineages.get(&2).unwrap().po_id, -1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN10 / LINEAGE-ACCOUNT-ID: accountId roundtrip via SocialState.
    // Haxe: Lineage.WriteLineages / ReadLineages accountId
    #[test]
    fn roundtrip_preserves_account_id_oln10() {
        let dir = unique_temp_dir("ol_lineage_account_id_oln10_sim");
        let path = dir.join("lineages_v1.bin");
        let mut original = sample_social();
        if let Some(n) = original.lineages.get_mut(&10) {
            n.stamp_account_id(42);
        }
        save_lineages(&original, &path).unwrap();
        let loaded = load_lineages(&path).unwrap();
        assert_eq!(loaded.lineages.get(&10).unwrap().account_id, 42);
        assert_eq!(loaded.lineages.get(&2).unwrap().account_id, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN11 / LINEAGE-DYNASTY: myDynastyId roundtrip via SocialState.
    // Haxe: Lineage.WriteLineages / ReadLineages myDynastyId
    #[test]
    fn roundtrip_preserves_my_dynasty_id_oln11() {
        let dir = unique_temp_dir("ol_lineage_dynasty_oln11_sim");
        let path = dir.join("lineages_v1.bin");
        let mut original = sample_social();
        if let Some(n) = original.lineages.get_mut(&10) {
            n.stamp_dynasty_id(2);
        }
        save_lineages(&original, &path).unwrap();
        let loaded = load_lineages(&path).unwrap();
        assert_eq!(loaded.lineages.get(&10).unwrap().my_dynasty_id, 2);
        assert_eq!(loaded.lineages.get(&2).unwrap().my_dynasty_id, -1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN12 / LINEAGE-FOLLOW-ID: set_follow stamps followPlayerId; load restores following.
    // Haxe: Lineage.WriteLineages / ReadLineages followPlayerId
    #[test]
    fn roundtrip_preserves_follow_player_id_oln12() {
        let dir = unique_temp_dir("ol_lineage_follow_oln12_sim");
        let path = dir.join("lineages_v1.bin");
        let mut original = sample_social();
        original.set_follow(10, 2).unwrap();
        assert_eq!(original.lineages.get(&10).unwrap().follow_player_id, 2);
        save_lineages(&original, &path).unwrap();
        let loaded = load_lineages(&path).unwrap();
        assert_eq!(loaded.lineages.get(&10).unwrap().follow_player_id, 2);
        assert_eq!(loaded.following.get(&10), Some(&2));
        assert_eq!(loaded.lineages.get(&2).unwrap().follow_player_id, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN13 / LINEAGE-KILLED-BY: killedByPlayerId roundtrip via SocialState.
    // Haxe: Lineage.WriteLineages / ReadLineages killedByPlayerId
    #[test]
    fn roundtrip_preserves_killed_by_player_id_oln13() {
        let dir = unique_temp_dir("ol_lineage_killed_oln13_sim");
        let path = dir.join("lineages_v1.bin");
        let mut original = sample_social();
        if let Some(n) = original.lineages.get_mut(&10) {
            n.stamp_killed_by_player_id(3);
        }
        save_lineages(&original, &path).unwrap();
        let loaded = load_lineages(&path).unwrap();
        assert_eq!(loaded.lineages.get(&10).unwrap().killed_by_player_id, 3);
        assert_eq!(loaded.lineages.get(&2).unwrap().killed_by_player_id, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// OLN14 / LINEAGE-TRUE-AGE: trueAge vs aging age roundtrip via SocialState.
    // Haxe: Lineage.WriteLineages / ReadLineages age + trueAge
    #[test]
    fn roundtrip_preserves_true_age_at_death_oln14() {
        let dir = unique_temp_dir("ol_lineage_true_age_oln14_sim");
        let path = dir.join("lineages_v1.bin");
        let mut original = sample_social();
        if let Some(n) = original.lineages.get_mut(&10) {
            n.stamp_death_ages(50.0, "reason_hunger", 10.0, 14.0);
        }
        save_lineages(&original, &path).unwrap();
        let loaded = load_lineages(&path).unwrap();
        let n = loaded.lineages.get(&10).unwrap();
        assert!((n.age_at_death - 10.0).abs() < 1e-4);
        assert!((n.true_age_at_death - 14.0).abs() < 1e-4);
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
            let mother = original
                .lineages
                .get(&mother_id)
                .cloned()
                .unwrap_or_else(|| LineageNode::eve(mother_id, format!("M{mother_id}")));
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

    /// LINEAGE-ARCHIVE: sim_time>0 skip-on-save + `{path}.archive` sidecar.
    // Haxe: Lineage.WriteAllLineages deleteOld + TODO archive
    #[test]
    fn save_lineages_prunes_old_dead_when_sim_time_set() {
        let dir = unique_temp_dir("ol_lineage_prune_sim");
        let path = dir.join("lineages_v1.bin");
        let death = 100.0;
        let keep_secs = 60.0 * 60.0 * 24.0 * 1.5 * 60.0;
        let now = death + keep_secs + 60.0;

        let mut s = SocialState::default();
        s.sim_time = now;
        s.lineages.insert(1, LineageNode::eve(1, "EVE"));
        let mut stranger = LineageNode::eve(9, "STRANGER");
        stranger.stamp_death(death, "reason_hunger", 60.0);
        s.lineages.insert(9, stranger);

        save_lineages(&s, &path).unwrap();
        // In-memory map stays full (Haxe AllLineages is not dropped).
        assert_eq!(s.lineages.len(), 2);

        let loaded = load_lineages(&path).unwrap();
        assert_eq!(loaded.lineages.len(), 1);
        assert!(loaded.lineages.contains_key(&1));
        assert!(!loaded.lineages.contains_key(&9));

        let archived = ol_identity::load_lineage_map(ol_identity::lineage_archive_path(&path))
            .unwrap();
        assert_eq!(archived.get(&9).unwrap().name, "STRANGER");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_lineages_sim_time_zero_writes_full_map() {
        let dir = unique_temp_dir("ol_lineage_no_prune");
        let path = dir.join("lineages_v1.bin");
        let mut s = SocialState::default();
        assert_eq!(s.sim_time, 0.0);
        s.lineages.insert(1, LineageNode::eve(1, "EVE"));
        let mut old = LineageNode::eve(9, "OLD");
        old.stamp_death(1.0, "reason_hunger", 60.0);
        s.lineages.insert(9, old);
        save_lineages(&s, &path).unwrap();
        let loaded = load_lineages(&path).unwrap();
        assert_eq!(loaded.lineages.len(), 2);
        assert!(!ol_identity::lineage_archive_path(&path).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
