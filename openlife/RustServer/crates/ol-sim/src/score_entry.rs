//! Haxe `openlife.server.ScoreEntry` — prestige boni/mali queue per account.
//!
//! Distinct from the session scoreboard row also named [`crate::score::ScoreEntry`].
//! This module is the grave / ancestor prestige pipeline + **SES1 disk** (Haxe had
//! `// TODO save to disk` — Rust implements it).
//!
//! // Haxe: ScoreEntry.hx

use crate::accounts::{normalize_email, AccountBook, AccountRecord, AccountScoreEntry};
use crate::animal_move::is_bone_grave;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::time::Instant;
use tracing::info;

/// Re-export Haxe `ScoreEntry` type (stored on [`AccountRecord::score_entries`]).
pub type ScoreEntryAccount = AccountScoreEntry;

// ── ServerSettings defaults (Haxe settings/ServerSettings.hx) ───────────────

/// Haxe `ServerSettings.AncestorPrestigeFactor` — dead prestige × factor → ancestor.
pub const ANCESTOR_PRESTIGE_FACTOR: f32 = 0.2;
/// Haxe `ServerSettings.OldGraveDecayMali` — prestige mali when Old Grave (89) decays.
pub const OLD_GRAVE_DECAY_MALI: f32 = 20.0;
/// Haxe `ServerSettings.CursedGraveMali` — prestige mali per sharp-stone curse tick.
pub const CURSED_GRAVE_MALI: f32 = 2.0;
/// Haxe old grave object id checked in `CreateScoreEntryIfGrave`.
pub const OLD_GRAVE_OBJECT_ID: i32 = 89;
/// Minimum prestige to create a dead-relative score entry.
pub const DEAD_RELATIVE_MIN_PRESTIGE: f32 = 10.0;
/// Process gate: Haxe `Std.int(player.trueAge) % 5 == 0`.
pub const PROCESS_AGE_MOD: i32 = 5;
/// When prestige is below this, negative entries are requeued without apply.
pub const PROCESS_NEG_PRESTIGE_FLOOR: f32 = 10.0;
/// Haxe `score < -20` partial-apply threshold.
pub const PROCESS_LARGE_NEG_THRESHOLD: f32 = -20.0;
/// Haxe partial-apply: `scoreEntry.score += 10` when large negative.
pub const PROCESS_LARGE_NEG_STEP: f32 = 10.0;

// ── SES1 disk format ────────────────────────────────────────────────────────

/// Versioned score-entry store (implements Haxe disk TODO).
///
/// ```text
/// magic[4] = b"SES1"
/// version: u32 LE (= SCORE_ENTRY_FORMAT_VERSION)
/// account_count: u32 LE  (only accounts with ≥1 entry)
/// records × account_count:
///   email_len: u32 LE
///   email: [u8; email_len]
///   entry_count: u32 LE
///   entries × entry_count:
///     player_id: i32 LE
///     relative_player_id: i32 LE
///     relative_email_len: u32 LE
///     relative_email: [u8; …]
///     score: f32 LE
///     text_len: u32 LE
///     text: [u8; …]
/// ```
pub const SCORE_ENTRY_FORMAT_VERSION: u32 = 1;
const SES_MAGIC: &[u8; 4] = b"SES1";
/// Default on-disk name under the save directory.
pub const DEFAULT_SCORE_ENTRY_FILE: &str = "score_entries_v1.bin";

// ── Types ───────────────────────────────────────────────────────────────────

/// Result of one [`process_score_entry`] step.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessScoreResult {
    /// Prestige applied to the living player (Haxe `addPrestige`).
    pub prestige_delta: f32,
    /// Global message body (without p_id prefix).
    pub message: String,
    /// True when the entry was requeued (not fully consumed).
    pub requeued: bool,
}

/// Pure inputs for [`create_score_entry_for_dead_relative`].
#[derive(Debug, Clone)]
pub struct DeadRelativePlayer {
    pub p_id: i32,
    pub account_email: String,
    pub prestige: f32,
    pub name: String,
    pub family_name: String,
    /// Mother lineage player id; `None` → no entry (Haxe mother == null).
    pub mother_lineage_id: Option<i32>,
}

/// One node on the mother-line walk for dead-relative selection.
#[derive(Debug, Clone)]
pub struct MotherLineNode {
    pub player_id: i32,
    pub account_email: String,
    /// Haxe `ancestor.grave != null && !ancestor.grave.isBoneGrave()`.
    pub has_non_bone_grave: bool,
    /// Next mother up the chain (`None` at Eve).
    pub mother_id: Option<i32>,
}

// ── Create helpers ──────────────────────────────────────────────────────────

/// Haxe `ScoreEntry.CreateScoreEntryIfGrave` — Old Grave (89) decays unburied.
///
/// // Haxe: ScoreEntry.CreateScoreEntryIfGrave
pub fn create_score_entry_if_grave(
    decayed_obj_id: i32,
    owner_email: Option<&str>,
    creator_p_id: i32,
    creator_name: Option<&str>,
    creator_family: Option<&str>,
) -> Option<AccountScoreEntry> {
    if decayed_obj_id != OLD_GRAVE_OBJECT_ID {
        return None;
    }
    let email = owner_email.filter(|e| !e.is_empty())?;
    let text = match (creator_name, creator_family) {
        (Some(n), Some(f)) if !n.is_empty() => {
            format!("No one burried {n} {f}!")
        }
        (Some(n), _) if !n.is_empty() => {
            format!("No one burried {n}!")
        }
        _ => "No one burried your old bones!".into(),
    };
    Some(AccountScoreEntry {
        account_email: normalize_email(email),
        player_id: creator_p_id,
        relative_account_email: String::new(),
        relative_player_id: 0,
        score: -OLD_GRAVE_DECAY_MALI,
        text,
    })
}

/// Haxe `ScoreEntry.CreateScoreEntryForCursedGrave` — sharp stone extends grave.
///
/// Finds or creates an entry for `creator_id` and subtracts [`CURSED_GRAVE_MALI`].
/// // Haxe: ScoreEntry.CreateScoreEntryForCursedGrave
pub fn create_score_entry_for_cursed_grave(
    entries: &mut Vec<AccountScoreEntry>,
    owner_email: &str,
    creator_p_id: i32,
    creator_id: i32,
    creator_name: Option<&str>,
    creator_family: Option<&str>,
) {
    if owner_email.is_empty() {
        return;
    }
    let email = normalize_email(owner_email);
    let mut found = None;
    if creator_id > 0 {
        for (i, e) in entries.iter().enumerate() {
            if e.player_id == creator_id {
                found = Some(i);
                break;
            }
        }
    }
    if let Some(i) = found {
        entries[i].score -= CURSED_GRAVE_MALI;
        return;
    }
    let text = match (creator_name, creator_family) {
        (Some(n), Some(f)) if !n.is_empty() => {
            format!("{n} {f} bones where cursed!")
        }
        (Some(n), _) if !n.is_empty() => {
            format!("{n} bones where cursed!")
        }
        _ => "Your old bones where cursed!".into(),
    };
    entries.push(AccountScoreEntry {
        account_email: email,
        player_id: if creator_p_id != 0 {
            creator_p_id
        } else {
            creator_id
        },
        relative_account_email: String::new(),
        relative_player_id: 0,
        score: -CURSED_GRAVE_MALI,
        text,
    });
}

/// Haxe `ScoreEntry.CreateNewScoreEntry`.
/// // Haxe: ScoreEntry.CreateNewScoreEntry
pub fn create_new_score_entry(
    player: &DeadRelativePlayer,
    ancestor_account_email: &str,
    ancestor_player_id: i32,
    factor: f32,
) -> AccountScoreEntry {
    AccountScoreEntry {
        account_email: normalize_email(ancestor_account_email),
        player_id: ancestor_player_id,
        relative_account_email: normalize_email(&player.account_email),
        relative_player_id: player.p_id,
        score: player.prestige * factor,
        text: format!("{} {}!", player.name, player.family_name),
    }
}

/// Haxe `ScoreEntry.CreateScoreEntryForDeadRelative` pure selection + entry.
///
/// `lookup(id)` returns the mother-line node for that lineage id.
/// `rand01` is one roll per loop step (Haxe `WorldMap.calculateRandomFloat`).
/// // Haxe: ScoreEntry.CreateScoreEntryForDeadRelative
pub fn create_score_entry_for_dead_relative(
    player: &DeadRelativePlayer,
    lookup: &dyn Fn(i32) -> Option<MotherLineNode>,
    mut rand01: impl FnMut() -> f32,
    factor: f32,
) -> Option<AccountScoreEntry> {
    if player.prestige < DEAD_RELATIVE_MIN_PRESTIGE {
        return None;
    }
    let mut ancestor_id = player.mother_lineage_id?;
    let mut ancestor = lookup(ancestor_id)?;

    for _ in 0..10 {
        let next_id = ancestor.mother_id;
        let next = next_id.and_then(|id| lookup(id));

        if next.is_some()
            && (same_account(&player.account_email, &ancestor.account_email) || rand01() > 0.1)
        {
            ancestor_id = next_id.unwrap();
            ancestor = next.unwrap();
            continue;
        }

        return finalize_dead_relative(player, &ancestor, factor);
    }

    finalize_dead_relative(player, &ancestor, factor)
}

fn same_account(a: &str, b: &str) -> bool {
    normalize_email(a) == normalize_email(b)
}

fn finalize_dead_relative(
    player: &DeadRelativePlayer,
    ancestor: &MotherLineNode,
    factor: f32,
) -> Option<AccountScoreEntry> {
    if same_account(&player.account_email, &ancestor.account_email) {
        return None;
    }
    if !ancestor.has_non_bone_grave {
        return None;
    }
    Some(create_new_score_entry(
        player,
        &ancestor.account_email,
        ancestor.player_id,
        factor,
    ))
}

/// Convenience: `has_non_bone_grave` from optional grave object id.
pub fn grave_is_non_bone(grave_obj_id: Option<i32>) -> bool {
    match grave_obj_id {
        Some(id) if id > 0 => !is_bone_grave(id),
        _ => false,
    }
}

// ── Process ─────────────────────────────────────────────────────────────────

/// Haxe `ScoreEntry.ProcessScoreEntry` age gate.
/// // Haxe: Std.int(player.trueAge) % 5 != 0
#[inline]
pub fn should_process_score_entry(true_age_years: f32) -> bool {
    if !true_age_years.is_finite() || true_age_years < 0.0 {
        return false;
    }
    (true_age_years as i32) % PROCESS_AGE_MOD == 0
}

/// Haxe TimeHelper age-year block: only when `Std.int(tmpAge) != Std.int(player.age)`,
/// then [`should_process_score_entry`] on the new trueAge.
///
/// // Haxe: TimeHelper L736 + ScoreEntry.ProcessScoreEntry L104
#[inline]
pub fn should_process_score_entry_on_year_cross(prev_age_years: f32, new_age_years: f32) -> bool {
    if !prev_age_years.is_finite() || !new_age_years.is_finite() || new_age_years < 0.0 {
        return false;
    }
    let prev_i = prev_age_years.max(0.0) as i32;
    let new_i = new_age_years as i32;
    if new_i == prev_i {
        return false;
    }
    should_process_score_entry(new_age_years)
}

/// Haxe `Lineage.get_grave`: match `getCreatorId() == ancestor.myId`, then `!isBoneGrave`.
///
/// `graves` are `(object_id, creator_player_id)` pairs already resolved from world tiles.
/// // Haxe: Lineage.get_grave + ScoreEntry.CreateScoreEntryForDeadRelative L79
pub fn creator_grave_is_non_bone(
    graves: &[(i32, i32)],
    ancestor_player_id: i32,
) -> bool {
    graves.iter().any(|&(obj_id, creator_id)| {
        creator_id == ancestor_player_id && obj_id > 0 && !is_bone_grave(obj_id)
    })
}

/// Haxe `ScoreEntry.ProcessScoreEntry` queue body (after age gate).
///
/// Mutates `entries` (shift / requeue). Returns `None` when empty or requeued
/// without applying (negative + low prestige).
///
/// Port-as-is Haxe quirk: when `score < -20`, local `score` is set to `-10` but
/// `addPrestige` uses the **mutated** entry (`score += 10`) — same as Haxe.
/// // Haxe: ScoreEntry.ProcessScoreEntry
pub fn process_score_entry(
    entries: &mut Vec<AccountScoreEntry>,
    player_prestige: f32,
) -> Option<ProcessScoreResult> {
    if entries.is_empty() {
        return None;
    }
    let mut score_entry = entries.remove(0);
    let score = score_entry.score;

    if score < 0.0 && player_prestige < PROCESS_NEG_PRESTIGE_FLOOR {
        entries.push(score_entry);
        return None;
    }

    let mut requeued = false;
    if score < PROCESS_LARGE_NEG_THRESHOLD {
        // Haxe: score = -10; (unused for apply) scoreEntry.score += 10; push
        score_entry.score += PROCESS_LARGE_NEG_STEP;
        entries.push(score_entry.clone());
        requeued = true;
    }

    let tmp_score = score_entry.score.round() as i32;
    let message = if score_entry.score > 0.0 {
        format!(
            "You gained {tmp_score} prestige from your offsprings {} life",
            score_entry.text
        )
    } else {
        format!(
            "You lost {} prestige from {}",
            -tmp_score,
            score_entry.text
        )
    };

    Some(ProcessScoreResult {
        prestige_delta: score_entry.score,
        message,
        requeued,
    })
}

/// Haxe `Connection.sendGlobalMessage` text transform: UPPER + spaces → `_`.
pub fn format_global_message_text(message: &str) -> String {
    message.trim().to_uppercase().replace(' ', "_")
}

// ── Account book helpers ────────────────────────────────────────────────────

impl AccountRecord {
    /// Push a score entry onto this account's queue.
    pub fn push_score_entry(&mut self, entry: AccountScoreEntry) {
        self.score_entries.push(entry);
    }
}

impl AccountBook {
    /// Push entry onto the owner account (creates row if needed).
    pub fn push_score_entry(&mut self, entry: AccountScoreEntry) {
        let email = entry.account_email.clone();
        self.ensure(&email).score_entries.push(entry);
    }

    /// Process one entry for a living player on this email; returns apply result.
    pub fn process_score_entry_for(
        &mut self,
        email: &str,
        player_prestige: f32,
    ) -> Option<ProcessScoreResult> {
        let r = self.ensure(email);
        process_score_entry(&mut r.score_entries, player_prestige)
    }

    /// Total queued score entries across all accounts (tests / metrics).
    pub fn score_entry_count(&self) -> usize {
        self.by_email.values().map(|r| r.score_entries.len()).sum()
    }
}

// ── SES1 save / load ────────────────────────────────────────────────────────

/// Write all non-empty score-entry queues to `path` (atomic-ish via temp rename).
/// // Haxe: ScoreEntry TODO save to disk
pub fn save_score_entries(book: &AccountBook, path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    let t0 = Instant::now();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("bin.tmp");
    {
        let f = File::create(&tmp).map_err(|e| e.to_string())?;
        let mut w = BufWriter::with_capacity(64 * 1024, f);
        write_score_entries(book, &mut w)?;
        w.flush().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    info!(
        path = %path.display(),
        entries = book.score_entry_count(),
        ms = t0.elapsed().as_millis() as u64,
        "score entries saved"
    );
    Ok(())
}

/// Load score entries from `path` into `book` (merges onto matching emails;
/// creates account rows when needed). Missing file is OK (no-op Ok).
pub fn load_score_entries(book: &mut AccountBook, path: impl AsRef<Path>) -> Result<(), String> {
    let path: &Path = path.as_ref();
    if !path.exists() {
        return Ok(());
    }
    let t0 = Instant::now();
    let f = File::open(path).map_err(|e| e.to_string())?;
    let mut r = BufReader::with_capacity(64 * 1024, f);
    let n = read_score_entries_into(book, &mut r)?;
    info!(
        path = %path.display(),
        entries = n,
        ms = t0.elapsed().as_millis() as u64,
        "score entries loaded"
    );
    Ok(())
}

fn write_score_entries(book: &AccountBook, w: &mut impl Write) -> Result<(), String> {
    w.write_all(SES_MAGIC).map_err(|e| e.to_string())?;
    w.write_u32::<LittleEndian>(SCORE_ENTRY_FORMAT_VERSION)
        .map_err(|e| e.to_string())?;

    let mut emails: Vec<&String> = book
        .by_email
        .iter()
        .filter(|(_, r)| !r.score_entries.is_empty())
        .map(|(e, _)| e)
        .collect();
    emails.sort();
    w.write_u32::<LittleEndian>(emails.len() as u32)
        .map_err(|e| e.to_string())?;

    for email in emails {
        let r = book.by_email.get(email).expect("email from keys");
        write_email_entries(email, &r.score_entries, w)?;
    }
    Ok(())
}

fn write_email_entries(
    email: &str,
    entries: &[AccountScoreEntry],
    w: &mut impl Write,
) -> Result<(), String> {
    let eb = email.as_bytes();
    w.write_u32::<LittleEndian>(eb.len() as u32)
        .map_err(|e| e.to_string())?;
    w.write_all(eb).map_err(|e| e.to_string())?;
    w.write_u32::<LittleEndian>(entries.len() as u32)
        .map_err(|e| e.to_string())?;
    for e in entries {
        w.write_i32::<LittleEndian>(e.player_id)
            .map_err(|e| e.to_string())?;
        w.write_i32::<LittleEndian>(e.relative_player_id)
            .map_err(|e| e.to_string())?;
        let reb = e.relative_account_email.as_bytes();
        w.write_u32::<LittleEndian>(reb.len() as u32)
            .map_err(|e| e.to_string())?;
        w.write_all(reb).map_err(|e| e.to_string())?;
        w.write_f32::<LittleEndian>(e.score)
            .map_err(|e| e.to_string())?;
        let tb = e.text.as_bytes();
        w.write_u32::<LittleEndian>(tb.len() as u32)
            .map_err(|e| e.to_string())?;
        w.write_all(tb).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn read_score_entries_into(
    book: &mut AccountBook,
    r: &mut impl Read,
) -> Result<usize, String> {
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if &magic != SES_MAGIC {
        return Err(format!("bad score-entry magic {:?}", magic));
    }
    let version = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())?;
    if version != SCORE_ENTRY_FORMAT_VERSION {
        return Err(format!(
            "unsupported score-entry version {version} (want {SCORE_ENTRY_FORMAT_VERSION})"
        ));
    }
    let account_count = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    let mut total = 0usize;
    for _ in 0..account_count {
        let email_len = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
        if email_len > 4096 {
            return Err(format!("score-entry email too long ({email_len})"));
        }
        let mut email_buf = vec![0u8; email_len];
        r.read_exact(&mut email_buf).map_err(|e| e.to_string())?;
        let email = normalize_email(&String::from_utf8(email_buf).map_err(|e| e.to_string())?);
        let n = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
        let mut entries = Vec::with_capacity(n);
        for _ in 0..n {
            entries.push(read_one_entry(r, &email)?);
        }
        total += entries.len();
        let rec = book.ensure(&email);
        rec.score_entries = entries;
    }
    Ok(total)
}

fn read_one_entry(r: &mut impl Read, account_email: &str) -> Result<AccountScoreEntry, String> {
    let player_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let relative_player_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let rel_len = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    if rel_len > 4096 {
        return Err(format!("relative email too long ({rel_len})"));
    }
    let mut rel_buf = vec![0u8; rel_len];
    r.read_exact(&mut rel_buf).map_err(|e| e.to_string())?;
    let relative_account_email =
        normalize_email(&String::from_utf8(rel_buf).map_err(|e| e.to_string())?);
    let score = r.read_f32::<LittleEndian>().map_err(|e| e.to_string())?;
    let text_len = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    if text_len > 16 * 1024 {
        return Err(format!("score-entry text too long ({text_len})"));
    }
    let mut text_buf = vec![0u8; text_len];
    r.read_exact(&mut text_buf).map_err(|e| e.to_string())?;
    let text = String::from_utf8(text_buf).map_err(|e| e.to_string())?;
    Ok(AccountScoreEntry {
        account_email: account_email.to_string(),
        player_id,
        relative_account_email,
        relative_player_id,
        score,
        text,
    })
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
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
        let dir = env::temp_dir().join(format!("ol_score_entry_{prefix}_{t}_{n}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn if_grave_only_old_grave_89() {
        assert!(
            create_score_entry_if_grave(87, Some("a@b.c"), 1, Some("Ada"), Some("SNOW")).is_none()
        );
        let e =
            create_score_entry_if_grave(89, Some("a@b.c"), 7, Some("Ada"), Some("SNOW")).unwrap();
        assert_eq!(e.player_id, 7);
        assert!((e.score + OLD_GRAVE_DECAY_MALI).abs() < 1e-5);
        assert!(e.text.contains("Ada"));
        assert!(e.text.contains("SNOW"));
        let e2 = create_score_entry_if_grave(89, Some("a@b.c"), 0, None, None).unwrap();
        assert!(e2.text.contains("old bones"));
        assert!(create_score_entry_if_grave(89, None, 1, None, None).is_none());
    }

    #[test]
    fn cursed_grave_stacks_or_creates() {
        let mut entries = Vec::new();
        create_score_entry_for_cursed_grave(
            &mut entries,
            "x@y.z",
            3,
            3,
            Some("Bob"),
            Some("FOX"),
        );
        assert_eq!(entries.len(), 1);
        assert!((entries[0].score + CURSED_GRAVE_MALI).abs() < 1e-5);
        assert!(entries[0].text.contains("cursed"));
        create_score_entry_for_cursed_grave(
            &mut entries,
            "x@y.z",
            3,
            3,
            Some("Bob"),
            Some("FOX"),
        );
        assert_eq!(entries.len(), 1);
        assert!((entries[0].score + 2.0 * CURSED_GRAVE_MALI).abs() < 1e-5);
    }

    #[test]
    fn dead_relative_skips_low_prestige_and_same_account() {
        let player = DeadRelativePlayer {
            p_id: 2,
            account_email: "kid@x".into(),
            prestige: 5.0,
            name: "Kid".into(),
            family_name: "SNOW".into(),
            mother_lineage_id: Some(1),
        };
        let nodes: HashMap<i32, MotherLineNode> = HashMap::new();
        assert!(create_score_entry_for_dead_relative(
            &player,
            &|id| nodes.get(&id).cloned(),
            || 0.5,
            ANCESTOR_PRESTIGE_FACTOR
        )
        .is_none());

        let player = DeadRelativePlayer {
            prestige: 50.0,
            mother_lineage_id: Some(1),
            ..player
        };
        let mut nodes = HashMap::new();
        nodes.insert(
            1,
            MotherLineNode {
                player_id: 1,
                account_email: "kid@x".into(),
                has_non_bone_grave: true,
                mother_id: None,
            },
        );
        assert!(create_score_entry_for_dead_relative(
            &player,
            &|id| nodes.get(&id).cloned(),
            || 0.0,
            ANCESTOR_PRESTIGE_FACTOR
        )
        .is_none());
    }

    #[test]
    fn dead_relative_awards_ancestor_with_non_bone_grave() {
        let player = DeadRelativePlayer {
            p_id: 5,
            account_email: "kid@x".into(),
            prestige: 100.0,
            name: "Kid".into(),
            family_name: "SNOW".into(),
            mother_lineage_id: Some(1),
        };
        let mut nodes = HashMap::new();
        nodes.insert(
            1,
            MotherLineNode {
                player_id: 1,
                account_email: "mom@x".into(),
                has_non_bone_grave: true,
                mother_id: None,
            },
        );
        let e = create_score_entry_for_dead_relative(
            &player,
            &|id| nodes.get(&id).cloned(),
            || 0.0,
            ANCESTOR_PRESTIGE_FACTOR,
        )
        .expect("award");
        assert_eq!(e.account_email, "mom@x");
        assert_eq!(e.player_id, 1);
        assert_eq!(e.relative_player_id, 5);
        assert!((e.score - 20.0).abs() < 1e-4);
        assert!(e.text.contains("Kid"));
    }

    #[test]
    fn dead_relative_skips_bone_grave() {
        let player = DeadRelativePlayer {
            p_id: 5,
            account_email: "kid@x".into(),
            prestige: 100.0,
            name: "Kid".into(),
            family_name: "SNOW".into(),
            mother_lineage_id: Some(1),
        };
        let mut nodes = HashMap::new();
        nodes.insert(
            1,
            MotherLineNode {
                player_id: 1,
                account_email: "mom@x".into(),
                has_non_bone_grave: false,
                mother_id: None,
            },
        );
        assert!(create_score_entry_for_dead_relative(
            &player,
            &|id| nodes.get(&id).cloned(),
            || 0.0,
            ANCESTOR_PRESTIGE_FACTOR
        )
        .is_none());
    }

    #[test]
    fn process_age_gate() {
        assert!(should_process_score_entry(0.0));
        assert!(should_process_score_entry(5.2));
        assert!(should_process_score_entry(10.9));
        assert!(!should_process_score_entry(6.0));
        assert!(!should_process_score_entry(1.0));
    }

    #[test]
    fn process_age_year_cross_once_per_boundary() {
        // Same floor band — no process even at age 5.x mid-year.
        assert!(!should_process_score_entry_on_year_cross(5.1, 5.9));
        // Cross into year 5 — process once.
        assert!(should_process_score_entry_on_year_cross(4.99, 5.01));
        // Cross into non-mod-5 year — skip.
        assert!(!should_process_score_entry_on_year_cross(5.9, 6.01));
        // Cross into year 10.
        assert!(should_process_score_entry_on_year_cross(9.9, 10.0));
        // No change.
        assert!(!should_process_score_entry_on_year_cross(10.0, 10.0));
    }

    #[test]
    fn creator_grave_gate_matches_lineage_get_grave() {
        // Non-bone (418 wolf not bone) for creator 7.
        assert!(creator_grave_is_non_bone(&[(418, 7), (87, 7)], 7));
        // Only bone graves for creator 7.
        assert!(!creator_grave_is_non_bone(&[(87, 7), (89, 7)], 7));
        // Non-bone but wrong creator.
        assert!(!creator_grave_is_non_bone(&[(418, 9)], 7));
        // Empty.
        assert!(!creator_grave_is_non_bone(&[], 7));
    }

    #[test]
    fn process_positive_applies_and_consumes() {
        let mut entries = vec![AccountScoreEntry {
            account_email: "a@b".into(),
            player_id: 1,
            relative_account_email: "c@d".into(),
            relative_player_id: 2,
            score: 12.4,
            text: "Kid SNOW!".into(),
        }];
        let r = process_score_entry(&mut entries, 50.0).unwrap();
        assert!((r.prestige_delta - 12.4).abs() < 1e-5);
        assert!(r.message.contains("gained 12"));
        assert!(!r.requeued);
        assert!(entries.is_empty());
    }

    #[test]
    fn process_negative_low_prestige_requeues() {
        let mut entries = vec![AccountScoreEntry {
            score: -5.0,
            text: "bones".into(),
            ..Default::default()
        }];
        assert!(process_score_entry(&mut entries, 5.0).is_none());
        assert_eq!(entries.len(), 1);
        assert!((entries[0].score + 5.0).abs() < 1e-5);
    }

    #[test]
    fn process_large_negative_partial_haxe_quirk() {
        let mut entries = vec![AccountScoreEntry {
            score: -25.0,
            text: "cursed!".into(),
            ..Default::default()
        }];
        let r = process_score_entry(&mut entries, 50.0).unwrap();
        assert!(r.requeued);
        assert!((r.prestige_delta + 15.0).abs() < 1e-5);
        assert!(r.message.contains("lost 15"));
        assert_eq!(entries.len(), 1);
        assert!((entries[0].score + 15.0).abs() < 1e-5);
    }

    #[test]
    fn ses1_roundtrip() {
        let dir = unique_temp_dir("ses1");
        let path = dir.join(DEFAULT_SCORE_ENTRY_FILE);
        let mut book = AccountBook::default();
        book.push_score_entry(AccountScoreEntry {
            account_email: "a@b.c".into(),
            player_id: 9,
            relative_account_email: "r@s.t".into(),
            relative_player_id: 11,
            score: -20.0,
            text: "No one burried Ada SNOW!".into(),
        });
        book.push_score_entry(AccountScoreEntry {
            account_email: "a@b.c".into(),
            player_id: 9,
            relative_account_email: String::new(),
            relative_player_id: 0,
            score: 4.5,
            text: "Kid FOX!".into(),
        });
        book.push_score_entry(AccountScoreEntry {
            account_email: "other@x".into(),
            player_id: 1,
            relative_account_email: String::new(),
            relative_player_id: 0,
            score: -2.0,
            text: "cursed".into(),
        });
        save_score_entries(&book, &path).unwrap();

        let mut loaded = AccountBook::default();
        loaded.ensure("a@b.c");
        load_score_entries(&mut loaded, &path).unwrap();
        assert_eq!(loaded.score_entry_count(), 3);
        let a = loaded.get("a@b.c").unwrap();
        assert_eq!(a.score_entries.len(), 2);
        assert!((a.score_entries[0].score + 20.0).abs() < 1e-5);
        assert!(a.score_entries[0].text.contains("Ada"));
        assert_eq!(a.score_entries[0].relative_player_id, 11);
        assert_eq!(a.score_entries[1].relative_player_id, 0);
        assert!((a.score_entries[1].score - 4.5).abs() < 1e-5);
        let o = loaded.get("other@x").unwrap();
        assert_eq!(o.score_entries.len(), 1);

        let mut empty = AccountBook::default();
        load_score_entries(&mut empty, dir.join("nope.bin")).unwrap();
        assert_eq!(empty.score_entry_count(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn grave_is_non_bone_helper() {
        assert!(!grave_is_non_bone(None));
        assert!(!grave_is_non_bone(Some(0)));
        assert!(!grave_is_non_bone(Some(87)));
        assert!(!grave_is_non_bone(Some(89)));
        assert!(grave_is_non_bone(Some(418)));
    }

    #[test]
    fn account_book_process() {
        let mut book = AccountBook::default();
        book.push_score_entry(AccountScoreEntry {
            account_email: "p@q".into(),
            score: 8.0,
            text: "A B!".into(),
            ..Default::default()
        });
        let r = book.process_score_entry_for("p@q", 20.0).unwrap();
        assert!((r.prestige_delta - 8.0).abs() < 1e-5);
        assert!(book.get("p@q").unwrap().score_entries.is_empty());
    }

    #[test]
    fn global_message_transform() {
        assert_eq!(
            format_global_message_text("You gained 12 prestige from your offsprings Kid SNOW! life"),
            "YOU_GAINED_12_PRESTIGE_FROM_YOUR_OFFSPRINGS_KID_SNOW!_LIFE"
        );
    }
}
