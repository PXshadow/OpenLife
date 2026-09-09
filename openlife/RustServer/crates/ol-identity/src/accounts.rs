//! Soft account / soul identity (Haxe PlayerAccount / PlayerSoul subset).
//!
//! No SQL — in-memory map keyed by normalized email. Tracks life count,
//! total score across lives, and last display name for web / bootstrap.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Shared soft-account list for web (`/api/accounts`).
pub type AccountView = Arc<RwLock<AccountBookSnapshot>>;

/// One account row for JSON APIs / web score table.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AccountSummary {
    pub email: String,
    pub lives: u32,
    /// Haxe `PlayerAccount.totalScore` display value (Prestige column).
    pub total_score: i32,
    pub total_kills: u32,
    pub total_deaths: u32,
    pub last_name: String,
    pub last_p_id: i32,
    pub lifetime_coins: i32,
    /// Haxe `PlayerAccount.coinsInherited` — priority score for death inheritance.
    pub coins_inherited: f32,
    /// Haxe `PlayerAccount.femaleScore` (session; not OLA1).
    pub female_score: f32,
    /// Haxe `PlayerAccount.maleScore` (session; not OLA1).
    pub male_score: f32,
    /// Haxe `PlayerAccount.isAi` (email heuristic when not stored).
    pub is_ai: bool,
}

/// Book snapshot for `/api/accounts`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AccountBookSnapshot {
    pub accounts: Vec<AccountSummary>,
    pub count: usize,
}

/// Haxe `ScoreEntry` — prestige boni/mali queued on [`AccountRecord::score_entries`].
///
/// Distinct from the session scoreboard row the session scoreboard ScoreEntry.
/// Disk: SES1 via score_entry (SES1, still in ol-sim) (Haxe had TODO save-to-disk).
/// `account_email` is the Rust key for Haxe numeric `accountId`.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountScoreEntry {
    /// Owner account email (Haxe `accountId` / queue owner).
    pub account_email: String,
    /// Haxe `playerId` — creator / ancestor lineage id.
    pub player_id: i32,
    /// Haxe `relativeAccountId` → email (offspring account for positive awards).
    pub relative_account_email: String,
    /// Haxe `relativePlayerId`.
    pub relative_player_id: i32,
    /// Prestige delta (negative = mali).
    pub score: f32,
    /// Display fragment used in process messages.
    pub text: String,
}

impl Default for AccountScoreEntry {
    fn default() -> Self {
        Self {
            account_email: String::new(),
            player_id: 0,
            relative_account_email: String::new(),
            relative_player_id: 0,
            score: 0.0,
            text: String::new(),
        }
    }
}

/// One soft account row.
#[derive(Debug, Clone)]
pub struct AccountRecord {
    /// Haxe `PlayerAccount.id` (numeric; `AccountIdIndex`, starts at 1).
    // Haxe: PlayerAccount.id / AccountIdIndex
    pub id: i32,
    pub email: String,
    pub lives: u32,
    pub total_score: i32,
    pub total_kills: u32,
    pub total_deaths: u32,
    pub last_name: String,
    pub last_p_id: i32,
    /// Cumulative coins earned (not current wallet).
    pub lifetime_coins: i32,
    /// Haxe `PlayerAccount.displayYum` — LS food hints / DisplayBestFood (default true).
    pub display_yum: bool,
    /// Haxe `PlayerAccount.coinsInherited` — past-life credit used as inheritance weight.
    ///
    /// Not yet in OLA1 on-disk format (defaults 0 on load, like `display_yum`).
    pub coins_inherited: f32,
    /// Haxe `PlayerAccount.score` — EMA this-life prestige (session / not OLA1).
    ///
    /// Used by SAY/client DIE prestige gate (`PrestigeCostForDie`); not debited on /DIE.
    pub score: f32,
    /// Haxe `PlayerAccount.femaleScore` — EMA prestige from female lives (session / not OLA1).
    pub female_score: f32,
    /// Haxe `PlayerAccount.maleScore` — EMA prestige from male lives (session / not OLA1).
    pub male_score: f32,
    /// Haxe `PlayerAccount.isAi` — permanent AI account (not OLA1; email heuristic on load).
    pub is_ai: bool,
    /// Haxe `PlayerAccount.familyPrestige` — founder_id → smoothed family score
    /// used by `countLeadershipPower` (session / not OLA1).
    pub family_prestige: HashMap<i32, f32>,
    /// Haxe `PlayerAccount.graves` — session grave tiles from
    /// `ObjectHelper.InitObjectHelpersAfterRead` / death stamp (not OLA1).
    /// Tiles are absolute world `(tx, ty)`.
    pub graves: Vec<(i32, i32)>,
    /// Haxe `PlayerAccount.scoreEntries` — prestige queue (SES1 disk; not OLA1).
    pub score_entries: Vec<AccountScoreEntry>,
    /// Haxe `PlayerAccount.canUseServerCommands` — granted by SAY `!S <Secret>`.
    pub can_use_server_commands: bool,
    /// Haxe `PlayerAccount.role` — 10 when admin secret matches, else 0.
    pub role: i32,
}

impl Default for AccountRecord {
    fn default() -> Self {
        Self {
            id: 0,
            email: String::new(),
            lives: 0,
            total_score: 0,
            total_kills: 0,
            total_deaths: 0,
            last_name: String::new(),
            last_p_id: 0,
            lifetime_coins: 0,
            display_yum: true,
            coins_inherited: 0.0,
            score: 0.0,
            female_score: 0.0,
            male_score: 0.0,
            is_ai: false,
            family_prestige: HashMap::new(),
            graves: Vec::new(),
            score_entries: Vec::new(),
            can_use_server_commands: false,
            role: 0,
        }
    }
}

impl AccountRecord {
    /// Push a score entry onto this account's queue.
    pub fn push_score_entry(&mut self, entry: AccountScoreEntry) {
        self.score_entries.push(entry);
    }
}

impl AccountRecord {
    pub fn new(email: impl Into<String>) -> Self {
        let email = email.into();
        let is_ai = account_email_looks_ai(&email);
        Self {
            email,
            display_yum: true,
            is_ai,
            ..Default::default()
        }
    }

    /// Toggle Haxe `displayYum`; returns new value.
    pub fn toggle_display_yum(&mut self) -> bool {
        self.display_yum = !self.display_yum;
        self.display_yum
    }

    /// Haxe `PlayerAccount.totalScore` getter: `floor((male+female)/2)`, then AI scale.
    ///
    /// When sex-split scores are still zero (legacy OLA1 / additive `total_score` path),
    /// falls back to stored [`Self::total_score`].
    pub fn haxe_total_score(&self) -> i32 {
        self.haxe_total_score_ex(ol_config::gameplay_defaults::AI_TOTAL_SCORE_FACTOR)
    }

    /// Like [`Self::haxe_total_score`] with live `AiTotalScoreFactor`.
    // Haxe: PlayerAccount.get_totalScore `if (isAi) total *= AiTotalScoreFactor`
    pub fn haxe_total_score_ex(&self, ai_factor: f32) -> i32 {
        let mut total = if self.female_score > 0.0 || self.male_score > 0.0 {
            (self.male_score + self.female_score) / 2.0
        } else {
            self.total_score as f32
        };
        if self.is_ai {
            let f = if ai_factor.is_finite() && ai_factor >= 0.0 {
                ai_factor
            } else {
                ol_config::gameplay_defaults::AI_TOTAL_SCORE_FACTOR
            };
            total *= f;
        }
        total.floor() as i32
    }

    /// Chat / web summary line.
    pub fn summary_line(&self) -> String {
        format!(
            "ACCOUNT {} lives={} score={} kills={} deaths={} name=\"{}\"",
            self.email,
            self.lives,
            self.total_score,
            self.total_kills,
            self.total_deaths,
            self.last_name
        )
    }
}

/// Haxe `ServerSettings.ScoreFactor` default (live: `GameplayKnobs.score_factor`).
// Haxe: ServerSettings.ScoreFactor = 0.2
pub const SCORE_FACTOR: f32 = 0.2;

/// Haxe `PlayerAccount.ChangeScore` EMA: `old * (1 - factor) + life * factor`, 2-decimal round.
// Haxe: PlayerAccount.ChangeScore L245–254
pub fn blend_score_ema(old: f32, life_score: f32, factor: f32) -> f32 {
    let f = if factor.is_finite() {
        factor.clamp(0.0, 1.0)
    } else {
        SCORE_FACTOR
    };
    let old = if old.is_finite() { old } else { 0.0 };
    let life = if life_score.is_finite() {
        life_score
    } else {
        0.0
    };
    let v = old * (1.0 - f) + life * f;
    (v * 100.0).round() / 100.0
}

/// EMA one `familyPrestige[founderId]` key (2× when this account is that founder).
// Haxe: PlayerAccount.ChangeScore family / dynasty L187–199
pub fn fold_family_prestige_ema(
    rec: &mut AccountRecord,
    life_score: f32,
    factor: f32,
    founder_id: i32,
    player_is_founder: bool,
) {
    if founder_id <= 0 {
        return;
    }
    let fam = if player_is_founder {
        life_score * 2.0
    } else {
        life_score
    };
    let old = rec.family_prestige.get(&founder_id).copied().unwrap_or(0.0);
    rec.family_prestige
        .insert(founder_id, blend_score_ema(old, fam, factor));
}

/// Apply Haxe `ChangeScore` onto an account record (sex-split + family prestige).
pub fn apply_change_score(
    rec: &mut AccountRecord,
    life_score: f32,
    is_female: bool,
    factor: f32,
    founder_id: i32,
    player_is_founder: bool,
) {
    apply_change_score_ex(
        rec,
        life_score,
        is_female,
        factor,
        founder_id,
        player_is_founder,
        0,
        false,
    );
}

/// Like [`apply_change_score`] plus Haxe dynasty `familyPrestige[myDynastyId]`.
// Haxe: PlayerAccount.ChangeScore L193–200
pub fn apply_change_score_ex(
    rec: &mut AccountRecord,
    life_score: f32,
    is_female: bool,
    factor: f32,
    founder_id: i32,
    player_is_founder: bool,
    dynasty_id: i32,
    player_is_dynasty_founder: bool,
) {
    rec.score = blend_score_ema(rec.score, life_score, factor);
    if is_female {
        rec.female_score = blend_score_ema(rec.female_score, life_score, factor);
    } else {
        rec.male_score = blend_score_ema(rec.male_score, life_score, factor);
    }
    fold_family_prestige_ema(rec, life_score, factor, founder_id, player_is_founder);
    if dynasty_id > 0 {
        fold_family_prestige_ema(
            rec,
            life_score,
            factor,
            dynasty_id,
            player_is_dynasty_founder,
        );
    }
}

/// Normalize email for map keys (trim + lowercase).
pub fn normalize_email(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

/// Permanent AI / NPC email heuristic (Haxe `PlayerAccount.isAi` when not on disk).
///
/// Mirrors `player_soul::email_looks_ai` without a module cycle.
// Haxe: PlayerAccount.isAi / ServerAi permanent AI emails
pub fn account_email_looks_ai(email: &str) -> bool {
    let email_l = email.to_ascii_lowercase();
    email_l.contains("ai@")
        || email_l.starts_with("ai_")
        || email_l.contains("npc")
        || email_l.contains("selfplay")
}

/// Haxe `scoreName` stand-in for web ID column when NamingHelper name tables are absent.
///
/// Prefers last display name; else email local-part (never full email on public table).
// Haxe: PlayerAccount.scoreName → NamingHelper.GenerateAccountName(id)
pub fn account_score_display_id(s: &AccountSummary) -> String {
    let name = s.last_name.trim();
    if !name.is_empty() {
        return name.to_string();
    }
    let email = s.email.trim();
    if let Some((local, _)) = email.split_once('@') {
        if !local.is_empty() {
            return local.to_string();
        }
    }
    if !email.is_empty() {
        return email.to_string();
    }
    "—".into()
}

/// Minimum Haxe `totalScore` to appear on `/stats/accounts` leaderboard.
// Haxe: WebServer.generateAccountStatistics `if (account.totalScore < 5) continue`
pub const ACCOUNT_STATS_MIN_TOTAL_SCORE: i32 = 5;

/// Escape text for HTML table cells (accounts score table).
fn account_html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Haxe `WebServer.generateAccountStatistics` HTML fragment (pure).
///
/// Header: `Score: count: N human: M` then table
/// `ID | Prestige | Female Prestige | Male Prestige | Coins`.
/// Filters AI + `totalScore < 5`, sorts prestige descending.
// Haxe: WebServer.generateAccountStatistics L301–339
pub fn format_account_statistics_html(snap: &AccountBookSnapshot) -> String {
    let count = snap.count;
    let count_human = snap.accounts.iter().filter(|a| !a.is_ai).count();

    let mut list: Vec<&AccountSummary> = snap
        .accounts
        .iter()
        .filter(|a| !a.is_ai && a.total_score >= ACCOUNT_STATS_MIN_TOTAL_SCORE)
        .collect();
    list.sort_by(|a, b| b.total_score.cmp(&a.total_score));

    let mut rows = String::new();
    for a in list {
        let id = account_html_escape(&account_score_display_id(a));
        let prestige = a.total_score;
        let female = a.female_score.floor() as i32;
        let male = a.male_score.floor() as i32;
        let coins = a.coins_inherited.floor() as i32;
        rows.push_str(&format!(
            "<tr><td>{id}</td><td>{prestige}</td><td>{female}</td><td>{male}</td><td>{coins}</td></tr>\n"
        ));
    }

    // Haxe builds header after rows; structure matches generateAccountStatistics.
    format!(
        "<br><br><center>Score: count: {count} human: {count_human}\n\n<table>\n\
         <tr><td><b>ID</b></td><td><b>Prestige</b></td><td><b>Female Prestige</b></td>\
         <td><b>Male Prestige</b></td><td><b>Coins</b></td></tr>\n\
         {rows}</table></center>"
    )
}

/// Session account book.
#[derive(Debug, Clone)]
pub struct AccountBook {
    pub by_email: HashMap<String, AccountRecord>,
    /// Haxe `PlayerAccount.AccountIdIndex` — next numeric id (starts at 1).
    // Haxe: PlayerAccount.AccountIdIndex = 1
    pub next_id: i32,
}

impl Default for AccountBook {
    fn default() -> Self {
        Self {
            by_email: HashMap::new(),
            next_id: 1,
        }
    }
}

impl AccountBook {
    /// Allocate Haxe `AccountIdIndex` (never reuse; 1-based).
    fn alloc_id(&mut self) -> i32 {
        if self.next_id < 1 {
            self.next_id = 1;
        }
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    /// After OLA load: fill missing ids (v1) and set [`Self::next_id`] to max+1.
    pub fn assign_missing_ids(&mut self) {
        let mut max_id = self
            .by_email
            .values()
            .map(|r| r.id)
            .max()
            .unwrap_or(0)
            .max(0);
        let mut need: Vec<String> = self
            .by_email
            .iter()
            .filter(|(_, r)| r.id <= 0)
            .map(|(k, _)| k.clone())
            .collect();
        need.sort();
        for k in need {
            max_id = max_id.saturating_add(1);
            if let Some(r) = self.by_email.get_mut(&k) {
                r.id = max_id;
            }
        }
        self.next_id = max_id.saturating_add(1).max(1);
    }

    pub fn ensure(&mut self, email: &str) -> &mut AccountRecord {
        let key = normalize_email(email);
        let needs_id = self
            .by_email
            .get(&key)
            .map(|r| r.id <= 0)
            .unwrap_or(true);
        if needs_id {
            let id = self.alloc_id();
            self.by_email
                .entry(key.clone())
                .and_modify(|r| {
                    if r.id <= 0 {
                        r.id = id;
                    }
                })
                .or_insert_with(|| {
                    let mut rec = AccountRecord::new(key.clone());
                    rec.id = id;
                    rec
                });
        }
        self.by_email.get_mut(&key).expect("ensure inserted")
    }

    pub fn get(&self, email: &str) -> Option<&AccountRecord> {
        self.by_email.get(&normalize_email(email))
    }

    /// Haxe `PlayerAccount.AllPlayerAccountsById` lookup.
    // Haxe: PlayerAccount.GetPlayerAccountById
    pub fn get_by_id(&self, id: i32) -> Option<&AccountRecord> {
        if id <= 0 {
            return None;
        }
        self.by_email.values().find(|r| r.id == id)
    }

    /// Push a prestige score-entry onto the owner account (creates row if needed).
    pub fn push_score_entry(&mut self, entry: AccountScoreEntry) {
        let email = entry.account_email.clone();
        self.ensure(&email).score_entries.push(entry);
    }

    /// Total queued score entries across all accounts (tests / metrics).
    pub fn score_entry_count(&self) -> usize {
        self.by_email.values().map(|r| r.score_entries.len()).sum()
    }

    /// Record a new life spawn / login.
    pub fn on_spawn(&mut self, email: &str, p_id: i32, display_name: &str) {
        let r = self.ensure(email);
        r.lives = r.lives.saturating_add(1);
        r.last_p_id = p_id;
        if !display_name.is_empty() {
            r.last_name = display_name.to_string();
        }
    }

    /// Haxe `PlayerAccount.ChangeScore` — EMA this life prestige into sex-split + family maps.
    pub fn change_score(
        &mut self,
        email: &str,
        life_score: f32,
        is_female: bool,
        factor: f32,
        founder_id: i32,
        player_is_founder: bool,
    ) {
        self.change_score_ex(
            email,
            life_score,
            is_female,
            factor,
            founder_id,
            player_is_founder,
            0,
            false,
        );
    }

    /// Like [`Self::change_score`] plus dynasty `familyPrestige[myDynastyId]`.
    // Haxe: PlayerAccount.ChangeScore L193–200
    pub fn change_score_ex(
        &mut self,
        email: &str,
        life_score: f32,
        is_female: bool,
        factor: f32,
        founder_id: i32,
        player_is_founder: bool,
        dynasty_id: i32,
        player_is_dynasty_founder: bool,
    ) {
        let r = self.ensure(email);
        apply_change_score_ex(
            r,
            life_score,
            is_female,
            factor,
            founder_id,
            player_is_founder,
            dynasty_id,
            player_is_dynasty_founder,
        );
    }

    /// Fold end-of-life stats into the account.
    pub fn on_death(
        &mut self,
        email: &str,
        score: i32,
        kills: u32,
        deaths: u32,
        coins_earned: i32,
    ) {
        let r = self.ensure(email);
        r.total_score = r.total_score.saturating_add(score);
        r.total_kills = r.total_kills.saturating_add(kills);
        r.total_deaths = r.total_deaths.saturating_add(deaths.max(1));
        r.lifetime_coins = r.lifetime_coins.saturating_add(coins_earned);
    }

    /// Credit Haxe `coinsInherited` on death (factor of wallet, typically 0.8).
    pub fn credit_coins_inherited(&mut self, email: &str, wallet_coins: i32, factor: f32) {
        if wallet_coins <= 0 || factor <= 0.0 {
            return;
        }
        let r = self.ensure(email);
        r.coins_inherited += wallet_coins as f32 * factor;
    }

    /// Haxe `account.familyPrestige[founderId]` lookup (0 when missing).
    pub fn family_prestige_for(&self, email: &str, founder_id: i32) -> f32 {
        self.get(email)
            .and_then(|r| r.family_prestige.get(&founder_id).copied())
            .unwrap_or(0.0)
    }

    /// Set Haxe `account.familyPrestige[founderId]` (tests / score fold).
    pub fn set_family_prestige(&mut self, email: &str, founder_id: i32, value: f32) {
        self.ensure(email)
            .family_prestige
            .insert(founder_id, value.max(0.0));
    }

    /// Haxe found-new: `familyPrestige[newFounder] = familyPrestige[oldFounder]`.
    // Haxe: NamingHelper.DoNaming L141 / L164
    pub fn copy_family_prestige(&mut self, email: &str, from_id: i32, to_id: i32) {
        if to_id <= 0 {
            return;
        }
        let v = self.family_prestige_for(email, from_id);
        self.set_family_prestige(email, to_id, v);
    }

    /// Push a session grave tile (Haxe `account.graves`); dedupes.
    pub fn record_grave(&mut self, email: &str, x: i32, y: i32) {
        let r = self.ensure(email);
        let tile = (x, y);
        if !r.graves.contains(&tile) {
            r.graves.push(tile);
        }
    }

    /// `SAY ?ACCOUNT` body without leading p_id.
    pub fn format_query(&self, email: &str) -> String {
        match self.get(email) {
            Some(r) => r.summary_line(),
            None => format!("ACCOUNT {} lives=0 score=0", normalize_email(email)),
        }
    }

    pub fn len(&self) -> usize {
        self.by_email.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_email.is_empty()
    }

    /// Soft-account summary list for web (sorted by email).
    pub fn snapshot(&self) -> AccountBookSnapshot {
        self.snapshot_ex(ol_config::gameplay_defaults::AI_TOTAL_SCORE_FACTOR)
    }

    /// Like [`Self::snapshot`] with live `AiTotalScoreFactor`.
    pub fn snapshot_ex(&self, ai_total_score_factor: f32) -> AccountBookSnapshot {
        let mut accounts: Vec<AccountSummary> = self
            .by_email
            .values()
            .map(|r| {
                let is_ai = r.is_ai || account_email_looks_ai(&r.email);
                AccountSummary {
                    email: r.email.clone(),
                    lives: r.lives,
                    // Prestige column: prefer Haxe sex-average when female/male populated.
                    total_score: r.haxe_total_score_ex(ai_total_score_factor),
                    total_kills: r.total_kills,
                    total_deaths: r.total_deaths,
                    last_name: r.last_name.clone(),
                    last_p_id: r.last_p_id,
                    lifetime_coins: r.lifetime_coins,
                    coins_inherited: r.coins_inherited,
                    female_score: r.female_score,
                    male_score: r.male_score,
                    is_ai,
                }
            })
            .collect();
        accounts.sort_by(|a, b| a.email.cmp(&b.email));
        let count = accounts.len();
        AccountBookSnapshot { accounts, count }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_and_spawn() {
        let mut b = AccountBook::default();
        b.on_spawn("  Ada@X.COM ", 7, "Ada Snow");
        let r = b.get("ada@x.com").unwrap();
        assert_eq!(r.lives, 1);
        assert_eq!(r.last_p_id, 7);
        assert_eq!(r.last_name, "Ada Snow");
        assert_eq!(r.id, 1);
        b.on_spawn("ada@x.com", 8, "Ada Snow");
        assert_eq!(b.get("ada@x.com").unwrap().lives, 2);
        assert_eq!(b.get("ada@x.com").unwrap().id, 1);
    }

    /// Haxe `AccountIdIndex` — stable numeric ids, 1-based, not reused.
    // Haxe: PlayerAccount.GetOrCreatePlayerAccount AccountIdIndex
    #[test]
    fn ensure_assigns_stable_numeric_ids() {
        let mut b = AccountBook::default();
        assert_eq!(b.ensure("a@x").id, 1);
        assert_eq!(b.ensure("b@x").id, 2);
        assert_eq!(b.ensure("a@x").id, 1);
        assert_eq!(b.get_by_id(2).unwrap().email, "b@x");
        assert!(b.get_by_id(0).is_none());
        assert_eq!(b.next_id, 3);
    }

    #[test]
    fn death_folds_stats() {
        let mut b = AccountBook::default();
        b.on_spawn("a@b.c", 1, "A");
        b.on_death("a@b.c", 15, 2, 1, 3);
        let r = b.get("a@b.c").unwrap();
        assert_eq!(r.total_score, 15);
        assert_eq!(r.total_kills, 2);
        assert_eq!(r.total_deaths, 1);
        assert_eq!(r.lifetime_coins, 3);
        assert!(b.format_query("a@b.c").contains("lives=1"));
    }

    #[test]
    fn change_score_ema_matches_haxe() {
        // factor 0.2: 0*(0.8) + 10*0.2 = 2.00
        assert!((blend_score_ema(0.0, 10.0, 0.2) - 2.0).abs() < 1e-6);
        assert!((blend_score_ema(2.0, 10.0, 0.2) - 3.6).abs() < 1e-6);
        let mut b = AccountBook::default();
        b.change_score("f@x", 10.0, true, 0.2, 7, true);
        let r = b.get("f@x").unwrap();
        assert!((r.score - 2.0).abs() < 1e-6);
        assert!((r.female_score - 2.0).abs() < 1e-6);
        assert!((r.male_score - 0.0).abs() < 1e-6);
        // founder → family prestige uses 2× life score: 0*0.8 + 20*0.2 = 4
        assert!((b.family_prestige_for("f@x", 7) - 4.0).abs() < 1e-6);
        b.change_score("m@x", 10.0, false, 1.0, 0, false);
        assert!((b.get("m@x").unwrap().male_score - 10.0).abs() < 1e-6);
    }

    /// Haxe dynasty ChangeScore: `familyPrestige[myDynastyId]` EMA, 2× if same account.
    // Haxe: PlayerAccount.ChangeScore L193–200
    #[test]
    fn change_score_dynasty_fold_matches_haxe() {
        let mut b = AccountBook::default();
        b.change_score_ex("f@x", 10.0, true, 0.2, 7, true, 3, false);
        // family founder 2×: 0*0.8 + 20*0.2 = 4 at key 7
        assert!((b.family_prestige_for("f@x", 7) - 4.0).abs() < 1e-6);
        // dynasty not founder: 0*0.8 + 10*0.2 = 2 at key 3
        assert!((b.family_prestige_for("f@x", 3) - 2.0).abs() < 1e-6);
        b.change_score_ex("d@x", 10.0, false, 0.2, 7, false, 3, true);
        assert!((b.family_prestige_for("d@x", 7) - 2.0).abs() < 1e-6);
        assert!((b.family_prestige_for("d@x", 3) - 4.0).abs() < 1e-6);
        b.copy_family_prestige("d@x", 7, 11);
        assert!((b.family_prestige_for("d@x", 11) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn missing_account_query() {
        let b = AccountBook::default();
        assert!(b.format_query("nobody@x").contains("lives=0"));
    }

    #[test]
    fn display_yum_defaults_true_and_toggles() {
        let mut r = AccountRecord::new("x@y.z");
        assert!(r.display_yum);
        assert!(!r.toggle_display_yum());
        assert!(!r.display_yum);
        assert!(r.toggle_display_yum());
    }

    #[test]
    fn credit_coins_inherited() {
        let mut b = AccountBook::default();
        b.credit_coins_inherited("a@b.c", 10, 0.8);
        assert!((b.get("a@b.c").unwrap().coins_inherited - 8.0).abs() < 1e-5);
        b.credit_coins_inherited("a@b.c", 5, 0.8);
        assert!((b.get("a@b.c").unwrap().coins_inherited - 12.0).abs() < 1e-5);
    }

    #[test]
    fn graves_session_default_empty() {
        let r = AccountRecord::new("g@h.i");
        assert!(r.graves.is_empty());
        assert!(r.score_entries.is_empty());
    }

    #[test]
    fn family_prestige_and_record_grave() {
        let mut b = AccountBook::default();
        assert_eq!(b.family_prestige_for("x@y.z", 10), 0.0);
        b.set_family_prestige("x@y.z", 10, 42.5);
        assert!((b.family_prestige_for("x@y.z", 10) - 42.5).abs() < 1e-4);
        assert_eq!(b.family_prestige_for("x@y.z", 99), 0.0);
        b.record_grave("x@y.z", 3, 4);
        b.record_grave("x@y.z", 3, 4); // dedupe
        assert_eq!(b.get("x@y.z").unwrap().graves, vec![(3, 4)]);
    }

    #[test]
    fn account_email_looks_ai_heuristic() {
        assert!(account_email_looks_ai("npc-forager@local"));
        assert!(account_email_looks_ai("selfplay@local"));
        assert!(account_email_looks_ai("ai_bot@x"));
        assert!(!account_email_looks_ai("alice@example.com"));
    }

    #[test]
    fn format_account_statistics_html_filters_sorts_columns() {
        let mut b = AccountBook::default();
        // Below threshold — omitted from rows.
        b.on_spawn("low@h.com", 1, "Low");
        b.on_death("low@h.com", 3, 0, 1, 0);
        // Human with sex-split scores — listed; Prestige = floor((30.9+20.1)/2) = 25.
        b.on_spawn("high@h.com", 2, "High Hero");
        b.on_death("high@h.com", 20, 0, 1, 0);
        {
            let r = b.ensure("high@h.com");
            r.female_score = 30.9;
            r.male_score = 20.1;
            r.coins_inherited = 4.9;
        }
        // Mid score 10 — listed after high.
        b.on_spawn("mid@h.com", 3, "Mid");
        b.on_death("mid@h.com", 10, 0, 1, 0);
        // AI — excluded from rows; counted in total, not human.
        b.on_spawn("npc-bot@local", 4, "Bot");
        b.on_death("npc-bot@local", 99, 0, 1, 0);

        let snap = b.snapshot();
        let html = format_account_statistics_html(&snap);
        assert!(html.contains("Score: count: 4 human: 3"));
        assert!(html.contains("<b>ID</b>"));
        assert!(html.contains("<b>Prestige</b>"));
        assert!(html.contains("<b>Female Prestige</b>"));
        assert!(html.contains("<b>Male Prestige</b>"));
        assert!(html.contains("<b>Coins</b>"));
        assert!(html.contains("High Hero"));
        // Prestige = haxe_total_score floor((30.9+20.1)/2) = 25.
        assert!(html.contains("<td>25</td>"));
        assert!(html.contains("<td>30</td>")); // floor female
        assert!(html.contains("<td>20</td>")); // floor male
        assert!(html.contains("<td>4</td>")); // floor coins
        assert!(html.contains("Mid"));
        assert!(!html.contains("Low")); // score 3 < 5
        assert!(!html.contains("Bot"));
        assert!(!html.contains("npc-bot"));
        // Sort desc: High (25) before Mid (10).
        let high_pos = html.find("High Hero").unwrap();
        let mid_pos = html.find("Mid").unwrap();
        assert!(high_pos < mid_pos);
    }

    #[test]
    fn haxe_total_score_prefers_sex_average() {
        let mut r = AccountRecord::new("a@b.c");
        r.total_score = 99;
        r.female_score = 10.0;
        r.male_score = 20.0;
        assert_eq!(r.haxe_total_score(), 15); // floor((10+20)/2)
        r.female_score = 0.0;
        r.male_score = 0.0;
        assert_eq!(r.haxe_total_score(), 99);
    }

    #[test]
    fn haxe_total_score_scales_ai_accounts() {
        let mut r = AccountRecord::new("ai@x");
        r.is_ai = true;
        r.female_score = 10.0;
        r.male_score = 20.0;
        assert_eq!(r.haxe_total_score(), 12); // floor(15 * 0.8)
        assert_eq!(r.haxe_total_score_ex(0.5), 7); // floor(15 * 0.5)
        r.is_ai = false;
        assert_eq!(r.haxe_total_score(), 15);
    }
}
