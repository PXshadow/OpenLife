//! Full-server login policy (Haxe `Connection.loginHelper` TODOs L135–138).
//!
//! Live Haxe already rejects when `countPlayers > MaxPlayers` and when
//! `GetOrCreatePlayerAccount` hits the daily IP/new-account cap. The TODOs add:
//! limit AIs when full, score / last-life priority, IP login spam.
//!
//! Cap is `living + incoming > max_players` (MaxPlayers is a real cap). Empty IP
//! skips IP gates (Haxe GetOrCreate with `clientIp == ''`).

use std::collections::HashMap;

/// Haxe `ServerSettings.MaxPlayers` default is 100; Rust `server.toml` default 200.
pub const DEFAULT_MAX_PLAYERS: u32 = 200;
/// Haxe `MinNumberOfAis` floor while culling for a human login.
pub const DEFAULT_NPC_MIN: u32 = 20;
/// Haxe `NewAccountsPerIpPerDay`.
// Haxe: ServerSettings.NewAccountsPerIpPerDay = 3
pub const NEW_ACCOUNTS_PER_IP_PER_DAY: u32 = 3;
/// Haxe `TotalNewAccountsPerDay`.
// Haxe: ServerSettings.TotalNewAccountsPerDay = 20
pub const TOTAL_NEW_ACCOUNTS_PER_DAY: u32 = 20;
/// Login attempts per IP inside [`IP_LOGIN_SPAM_WINDOW_SECS`].
pub const IP_LOGIN_SPAM_LIMIT: u32 = 8;
pub const IP_LOGIN_SPAM_WINDOW_SECS: f32 = 60.0;
/// Human priority that may cull AIs below `npc_min` when the server is full.
pub const PRIORITY_CULL_BELOW_MIN: f32 = 5.0;
/// Self-play / NPC reserved conn band (`ol-server` selfplay + `NPC_CONN_BASE`).
pub const SYNTHETIC_CONN_MIN: u64 = 9_000_000;

/// Sensors for one LOGIN / RLOGIN.
#[derive(Debug, Clone, PartialEq)]
pub struct SpawnQueueInput {
    pub max_players: u32,
    pub living_humans: u32,
    pub living_ais: u32,
    pub npc_min: u32,
    pub incoming_is_human: bool,
    /// Existing non-deleted body for this conn (reconnect / already spawned).
    pub already_living: bool,
    pub account_score: f32,
    pub last_seen_ago_secs: f32,
    pub last_life_age: f32,
    pub client_ip: String,
    pub is_new_account: bool,
    pub new_accounts_today_from_ip: u32,
    pub new_accounts_today_total: u32,
    pub recent_logins_from_ip: u32,
}

impl SpawnQueueInput {
    pub fn basic_human(living_humans: u32, living_ais: u32, max_players: u32) -> Self {
        Self {
            max_players,
            living_humans,
            living_ais,
            npc_min: DEFAULT_NPC_MIN,
            incoming_is_human: true,
            already_living: false,
            account_score: 0.0,
            last_seen_ago_secs: 0.0,
            last_life_age: 0.0,
            client_ip: String::new(),
            is_new_account: false,
            new_accounts_today_from_ip: 0,
            new_accounts_today_total: 0,
            recent_logins_from_ip: 0,
        }
    }
}

/// LOGIN admit / reject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnQueueDecision {
    Allow,
    /// Human login: delete this many living AIs then spawn.
    AllowCullAi { cull: u32 },
    RejectFull,
    RejectLowPriority,
    RejectIpAccountLimit,
    RejectIpSpam,
}

impl SpawnQueueDecision {
    pub fn is_allow(self) -> bool {
        matches!(self, Self::Allow | Self::AllowCullAi { .. })
    }
}

/// Haxe loginHelper TODO: score + short last life + time away.
// Haxe: Connection.loginHelper L136–137
pub fn spawn_priority(score: f32, last_seen_ago_secs: f32, last_life_age: f32) -> f32 {
    let score_part = score.max(0.0);
    let life_bonus = if last_life_age <= 0.0 {
        0.0
    } else {
        (1.0 - (last_life_age / 14.0).clamp(0.0, 1.0)) * 10.0
    };
    let away_bonus = (last_seen_ago_secs / 3600.0).clamp(0.0, 10.0);
    score_part + life_bonus + away_bonus
}

pub fn incoming_is_human_conn(conn_id: u64) -> bool {
    conn_id < SYNTHETIC_CONN_MIN
}

/// Pure admit policy.
// Haxe: Connection.loginHelper L132–167
pub fn spawn_queue_decide(inp: &SpawnQueueInput) -> SpawnQueueDecision {
    let ip = inp.client_ip.trim();
    if !ip.is_empty() {
        if inp.recent_logins_from_ip >= IP_LOGIN_SPAM_LIMIT {
            return SpawnQueueDecision::RejectIpSpam;
        }
        if inp.is_new_account {
            if inp.new_accounts_today_from_ip >= NEW_ACCOUNTS_PER_IP_PER_DAY
                || inp.new_accounts_today_total >= TOTAL_NEW_ACCOUNTS_PER_DAY
            {
                return SpawnQueueDecision::RejectIpAccountLimit;
            }
        }
    }

    if inp.already_living {
        return SpawnQueueDecision::Allow;
    }

    let max = inp.max_players;
    if max == 0 {
        return SpawnQueueDecision::Allow;
    }

    let living = inp.living_humans.saturating_add(inp.living_ais);
    if living < max {
        return SpawnQueueDecision::Allow;
    }

    // Server full (this login would exceed MaxPlayers).
    if !inp.incoming_is_human {
        return SpawnQueueDecision::RejectFull;
    }

    let room_needed = living.saturating_sub(max).saturating_add(1);
    let cullable_to_min = inp.living_ais.saturating_sub(inp.npc_min);
    if cullable_to_min >= room_needed {
        return SpawnQueueDecision::AllowCullAi {
            cull: room_needed,
        };
    }
    let pri = spawn_priority(
        inp.account_score,
        inp.last_seen_ago_secs,
        inp.last_life_age,
    );
    if pri >= PRIORITY_CULL_BELOW_MIN && inp.living_ais > 0 {
        let cull = room_needed.min(inp.living_ais);
        return SpawnQueueDecision::AllowCullAi { cull };
    }
    if cullable_to_min == 0 {
        SpawnQueueDecision::RejectLowPriority
    } else {
        SpawnQueueDecision::RejectFull
    }
}

/// Session book for IP caps, last-seen, last-life.
#[derive(Debug, Clone)]
pub struct SpawnQueueBook {
    pub max_players: u32,
    pub npc_min: u32,
    last_seen_sim: HashMap<String, f32>,
    last_life_age: HashMap<String, f32>,
    new_accounts_today_by_ip: HashMap<String, u32>,
    total_new_accounts_today: u32,
    account_day: u64,
    recent_logins_by_ip: HashMap<String, Vec<f32>>,
}

impl Default for SpawnQueueBook {
    fn default() -> Self {
        Self {
            max_players: DEFAULT_MAX_PLAYERS,
            npc_min: DEFAULT_NPC_MIN,
            last_seen_sim: HashMap::new(),
            last_life_age: HashMap::new(),
            new_accounts_today_by_ip: HashMap::new(),
            total_new_accounts_today: 0,
            account_day: 0,
            recent_logins_by_ip: HashMap::new(),
        }
    }
}

impl SpawnQueueBook {
    pub fn unix_day(unix_secs: u64) -> u64 {
        unix_secs / 86_400
    }

    fn roll_account_day(&mut self, day: u64) {
        if day != self.account_day {
            self.account_day = day;
            self.new_accounts_today_by_ip.clear();
            self.total_new_accounts_today = 0;
        }
    }

    pub fn note_login_attempt(&mut self, ip: &str, now_sim: f32) {
        let ip = ip.trim();
        if ip.is_empty() {
            return;
        }
        let times = self.recent_logins_by_ip.entry(ip.to_string()).or_default();
        times.retain(|t| now_sim - *t <= IP_LOGIN_SPAM_WINDOW_SECS);
        times.push(now_sim);
    }

    pub fn recent_logins_from_ip(&self, ip: &str, now_sim: f32) -> u32 {
        let ip = ip.trim();
        if ip.is_empty() {
            return 0;
        }
        self.recent_logins_by_ip
            .get(ip)
            .map(|v| {
                v.iter()
                    .filter(|t| now_sim - **t <= IP_LOGIN_SPAM_WINDOW_SECS)
                    .count() as u32
            })
            .unwrap_or(0)
    }

    pub fn new_accounts_today_from_ip(&self, ip: &str) -> u32 {
        let ip = ip.trim();
        if ip.is_empty() {
            return 0;
        }
        self.new_accounts_today_by_ip.get(ip).copied().unwrap_or(0)
    }

    pub fn total_new_accounts_today(&self) -> u32 {
        self.total_new_accounts_today
    }

    pub fn note_new_account(&mut self, ip: &str, day: u64) {
        let ip = ip.trim();
        if ip.is_empty() {
            return;
        }
        self.roll_account_day(day);
        self.total_new_accounts_today = self.total_new_accounts_today.saturating_add(1);
        *self
            .new_accounts_today_by_ip
            .entry(ip.to_string())
            .or_insert(0) += 1;
    }

    pub fn note_seen(&mut self, email: &str, now_sim: f32) {
        self.last_seen_sim.insert(email.to_ascii_lowercase(), now_sim);
    }

    pub fn last_seen_ago(&self, email: &str, now_sim: f32) -> f32 {
        self.last_seen_sim
            .get(&email.to_ascii_lowercase())
            .map(|t| (now_sim - *t).max(0.0))
            .unwrap_or(0.0)
    }

    pub fn note_life_end(&mut self, email: &str, age: f32) {
        self.last_life_age
            .insert(email.to_ascii_lowercase(), age.max(0.0));
    }

    pub fn last_life_age(&self, email: &str) -> f32 {
        self.last_life_age
            .get(&email.to_ascii_lowercase())
            .copied()
            .unwrap_or(0.0)
    }
}

/// Living humans vs synthetic AIs (`conn_id >= SYNTHETIC_CONN_MIN`).
pub fn count_living_humans_ais<'a, I>(players: I) -> (u32, u32)
where
    I: IntoIterator<Item = (&'a u64, &'a crate::Player)>,
{
    let mut humans = 0u32;
    let mut ais = 0u32;
    for (cid, p) in players {
        if p.deleted {
            continue;
        }
        if *cid >= SYNTHETIC_CONN_MIN {
            ais += 1;
        } else {
            humans += 1;
        }
    }
    (humans, ais)
}

/// Mark newest synthetic AIs deleted (no death inherit). Returns culled count.
pub fn cull_ai_slots(state: &mut crate::SimState, n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let mut ids: Vec<u64> = state
        .players
        .iter()
        .filter(|(cid, p)| **cid >= SYNTHETIC_CONN_MIN && !p.deleted)
        .map(|(cid, _)| *cid)
        .collect();
    ids.sort_unstable_by(|a, b| b.cmp(a));
    let mut culled = 0u32;
    for id in ids {
        if culled >= n {
            break;
        }
        if let Some(p) = state.players.get_mut(&id) {
            p.deleted = true;
            p.connected = false;
            culled += 1;
        }
    }
    culled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_cap_allows() {
        let inp = SpawnQueueInput::basic_human(1, 1, 10);
        assert_eq!(spawn_queue_decide(&inp), SpawnQueueDecision::Allow);
    }

    #[test]
    fn full_ai_login_rejected() {
        let mut inp = SpawnQueueInput::basic_human(2, 8, 10);
        inp.incoming_is_human = false;
        assert_eq!(spawn_queue_decide(&inp), SpawnQueueDecision::RejectFull);
    }

    #[test]
    fn full_human_culls_ais_down_to_min() {
        let inp = SpawnQueueInput::basic_human(2, 8, 10);
        assert_eq!(
            spawn_queue_decide(&inp),
            SpawnQueueDecision::AllowCullAi { cull: 1 }
        );
    }

    #[test]
    fn full_no_cullable_ai_low_priority_rejected() {
        let mut inp = SpawnQueueInput::basic_human(10, 0, 10);
        inp.npc_min = 0;
        inp.account_score = 0.0;
        assert_eq!(
            spawn_queue_decide(&inp),
            SpawnQueueDecision::RejectLowPriority
        );
    }

    #[test]
    fn high_score_culls_below_min() {
        let mut inp = SpawnQueueInput::basic_human(9, 1, 10);
        inp.npc_min = 3;
        inp.account_score = 20.0;
        assert_eq!(
            spawn_queue_decide(&inp),
            SpawnQueueDecision::AllowCullAi { cull: 1 }
        );
    }

    #[test]
    fn short_last_life_and_away_raise_priority() {
        let low = spawn_priority(0.0, 0.0, 0.0);
        let short = spawn_priority(0.0, 0.0, 1.0);
        let away = spawn_priority(0.0, 3600.0 * 5.0, 1.0);
        assert!(short > low);
        assert!(away > short);
        assert!(spawn_priority(20.0, 0.0, 0.0) >= PRIORITY_CULL_BELOW_MIN);
    }

    #[test]
    fn ip_new_account_daily_cap() {
        let mut inp = SpawnQueueInput::basic_human(0, 0, 10);
        inp.client_ip = "1.2.3.4".into();
        inp.is_new_account = true;
        inp.new_accounts_today_from_ip = NEW_ACCOUNTS_PER_IP_PER_DAY;
        assert_eq!(
            spawn_queue_decide(&inp),
            SpawnQueueDecision::RejectIpAccountLimit
        );
        inp.new_accounts_today_from_ip = 0;
        inp.new_accounts_today_total = TOTAL_NEW_ACCOUNTS_PER_DAY;
        assert_eq!(
            spawn_queue_decide(&inp),
            SpawnQueueDecision::RejectIpAccountLimit
        );
    }

    #[test]
    fn ip_login_spam_rejected() {
        let mut inp = SpawnQueueInput::basic_human(0, 0, 10);
        inp.client_ip = "8.8.8.8".into();
        inp.recent_logins_from_ip = IP_LOGIN_SPAM_LIMIT;
        assert_eq!(spawn_queue_decide(&inp), SpawnQueueDecision::RejectIpSpam);
    }

    #[test]
    fn empty_ip_skips_ip_gates() {
        let mut inp = SpawnQueueInput::basic_human(0, 0, 10);
        inp.is_new_account = true;
        inp.new_accounts_today_from_ip = 99;
        inp.recent_logins_from_ip = 99;
        assert_eq!(spawn_queue_decide(&inp), SpawnQueueDecision::Allow);
    }

    #[test]
    fn already_living_reconnect_allowed_when_full() {
        let mut inp = SpawnQueueInput::basic_human(10, 0, 10);
        inp.already_living = true;
        assert_eq!(spawn_queue_decide(&inp), SpawnQueueDecision::Allow);
    }

    #[test]
    fn book_rolls_daily_ip_counts() {
        let mut b = SpawnQueueBook::default();
        b.note_new_account("10.0.0.1", 100);
        assert_eq!(b.new_accounts_today_from_ip("10.0.0.1"), 1);
        b.note_new_account("10.0.0.1", 101);
        assert_eq!(b.new_accounts_today_from_ip("10.0.0.1"), 1);
        assert_eq!(b.total_new_accounts_today(), 1);
    }
}
