//! Live score tracking subset (kills / deaths / coins / season bonus → ranked scoreboard).
//!
//! Not the full Haxe grave/ancestor [`ScoreEntry`] pipeline — a session-local
//! ranking surface for `?SCORE` / `?LEADERBOARD` / `?HIGHSCORE` queries.
//!
//! Seasonal leaderboard: kills, deaths, and [`ScoreEntry::season_bonus`] reset when
//! the environment season changes; coins are kept (economy).

use crate::prestige::{prestige_classes_from_living_scores, PrestigeClass};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Shared prestige / scoreboard snapshot for web (`/api/prestige`).
pub type PrestigeView = Arc<RwLock<PrestigeSnapshot>>;

/// One living player's percentile prestige class row.
#[derive(Debug, Clone, Serialize)]
pub struct PrestigePlayerRow {
    pub p_id: i32,
    pub name: String,
    pub score: i32,
    pub prestige_class: String,
    pub class_id: i32,
}

/// Living percentile prestige board for `/api/prestige`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct PrestigeSnapshot {
    pub players: Vec<PrestigePlayerRow>,
    pub count: usize,
}

/// Points awarded per kill.
pub const SCORE_PER_KILL: i32 = 10;
/// Points deducted per death.
pub const SCORE_PER_DEATH: i32 = 5;

/// One player's scoreboard row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreEntry {
    pub p_id: i32,
    pub name: String,
    pub score: i32,
    pub kills: u32,
    pub deaths: u32,
    pub coins: i32,
    /// Extra points for the current season window (resets on season change).
    pub season_bonus: i32,
}

impl Default for ScoreEntry {
    fn default() -> Self {
        Self {
            p_id: 0,
            name: String::new(),
            score: 0,
            kills: 0,
            deaths: 0,
            coins: 0,
            season_bonus: 0,
        }
    }
}

impl ScoreEntry {
    pub fn new(p_id: i32, name: impl Into<String>) -> Self {
        Self {
            p_id,
            name: name.into(),
            ..Default::default()
        }
    }

    /// Recompute `score` from kills, deaths, coins, and season bonus.
    ///
    /// `score = kills * SCORE_PER_KILL - deaths * SCORE_PER_DEATH + coins + season_bonus`
    pub fn recompute(&mut self) {
        self.score = compute_score(self.kills, self.deaths, self.coins, self.season_bonus);
    }
}

/// `kills * 10 - deaths * 5 + coins + season_bonus` (saturating).
pub fn compute_score(kills: u32, deaths: u32, coins: i32, season_bonus: i32) -> i32 {
    (kills as i32)
        .saturating_mul(SCORE_PER_KILL)
        .saturating_sub((deaths as i32).saturating_mul(SCORE_PER_DEATH))
        .saturating_add(coins)
        .saturating_add(season_bonus)
}

/// Ranks players by score for chat queries and live views.
#[derive(Debug, Default, Clone)]
pub struct Scoreboard {
    pub entries: HashMap<i32, ScoreEntry>,
    /// Season tag last applied for leaderboard reset (e.g. `"SPRING"`).
    pub season_tag: String,
}

impl Scoreboard {
    pub fn ensure_player(&mut self, p_id: i32, name: impl Into<String>) -> &mut ScoreEntry {
        let name = name.into();
        self.entries
            .entry(p_id)
            .and_modify(|e| {
                if e.name.is_empty() && !name.is_empty() {
                    e.name = name.clone();
                }
            })
            .or_insert_with(|| ScoreEntry::new(p_id, name))
    }

    pub fn entry(&self, p_id: i32) -> Option<&ScoreEntry> {
        self.entries.get(&p_id)
    }

    pub fn set_name(&mut self, p_id: i32, name: impl Into<String>) {
        self.ensure_player(p_id, name);
    }

    /// Record a successful kill (killer +1 kill, victim +1 death).
    pub fn record_kill(&mut self, killer: i32, victim: i32) {
        if killer == victim {
            return;
        }
        {
            let e = self.ensure_player(killer, format!("P{killer}"));
            e.kills = e.kills.saturating_add(1);
            e.recompute();
        }
        {
            let e = self.ensure_player(victim, format!("P{victim}"));
            e.deaths = e.deaths.saturating_add(1);
            e.recompute();
        }
    }

    /// Record a death without a killer (suicide, hunger, age, etc.).
    pub fn record_death(&mut self, p_id: i32) {
        let e = self.ensure_player(p_id, format!("P{p_id}"));
        e.deaths = e.deaths.saturating_add(1);
        e.recompute();
    }

    /// Sync coin balance after pay / grant; recomputes score.
    pub fn set_coins(&mut self, p_id: i32, coins: i32) {
        let e = self.ensure_player(p_id, format!("P{p_id}"));
        e.coins = coins;
        e.recompute();
    }

    /// Add (or subtract) seasonal bonus points; recomputes score.
    pub fn add_season_bonus(&mut self, p_id: i32, delta: i32) {
        let e = self.ensure_player(p_id, format!("P{p_id}"));
        e.season_bonus = e.season_bonus.saturating_add(delta);
        e.recompute();
    }

    /// Set absolute season bonus; recomputes score.
    pub fn set_season_bonus(&mut self, p_id: i32, season_bonus: i32) {
        let e = self.ensure_player(p_id, format!("P{p_id}"));
        e.season_bonus = season_bonus;
        e.recompute();
    }

    /// Full sync from combat + economy sources (season_bonus left unchanged).
    pub fn sync_stats(&mut self, p_id: i32, name: &str, kills: u32, deaths: u32, coins: i32) {
        let e = self.ensure_player(p_id, name);
        if !name.is_empty() {
            e.name = name.to_string();
        }
        e.kills = kills;
        e.deaths = deaths;
        e.coins = coins;
        e.recompute();
    }

    /// Reset seasonal leaderboard when the environment season changes.
    ///
    /// Zeros kills, deaths, and `season_bonus` for every entry; keeps names and
    /// coins. First call (empty tag) only binds the tag without wiping. No-op
    /// when `new_season` matches the stored [`Self::season_tag`]. Returns
    /// `true` when a reset ran.
    pub fn on_season_change(&mut self, new_season: &str) -> bool {
        if self.season_tag.is_empty() {
            self.season_tag = new_season.to_string();
            return false;
        }
        if self.season_tag == new_season {
            return false;
        }
        self.season_tag = new_season.to_string();
        for e in self.entries.values_mut() {
            e.kills = 0;
            e.deaths = 0;
            e.season_bonus = 0;
            e.recompute();
        }
        true
    }

    /// Alias used by sim tick / SETSEASON wiring.
    pub fn reset_season_leaderboard(&mut self, new_season: &str) -> bool {
        self.on_season_change(new_season)
    }

    /// Players ranked by score descending, then `p_id` ascending for ties.
    pub fn ranked(&self) -> Vec<&ScoreEntry> {
        let mut v: Vec<&ScoreEntry> = self.entries.values().collect();
        v.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.p_id.cmp(&b.p_id)));
        v
    }

    /// Top `n` entries (score desc).
    pub fn top(&self, n: usize) -> Vec<&ScoreEntry> {
        self.ranked().into_iter().take(n).collect()
    }

    /// Self score line for `?SCORE` (without leading p_id prefix used by PS).
    pub fn format_score_text(&self, p_id: i32) -> String {
        match self.entries.get(&p_id) {
            Some(e) => format!(
                "SCORE {} K{} D{} C{} B{} ({})",
                e.score, e.kills, e.deaths, e.coins, e.season_bonus, e.name
            ),
            None => "SCORE 0 K0 D0 C0 B0".into(),
        }
    }

    /// Living percentile prestige classes from current scoreboard scores.
    pub fn prestige_snapshot(&self) -> PrestigeSnapshot {
        let scores: Vec<(i32, i32)> = self.entries.values().map(|e| (e.p_id, e.score)).collect();
        let classes = prestige_classes_from_living_scores(&scores);
        let mut players: Vec<PrestigePlayerRow> = self
            .entries
            .values()
            .map(|e| {
                let class = classes
                    .get(&e.p_id)
                    .copied()
                    .unwrap_or(PrestigeClass::Commoner);
                PrestigePlayerRow {
                    p_id: e.p_id,
                    name: if e.name.is_empty() {
                        format!("P{}", e.p_id)
                    } else {
                        e.name.clone()
                    },
                    score: e.score,
                    prestige_class: class.wire_name().to_string(),
                    class_id: class.as_i32(),
                }
            })
            .collect();
        // Rank by score desc for the API consumer.
        players.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.p_id.cmp(&b.p_id)));
        let count = players.len();
        PrestigeSnapshot { players, count }
    }

    /// Leaderboard line for `?LEADERBOARD` (top `limit`, default use 10).
    pub fn format_leaderboard_text(&self, limit: usize) -> String {
        let top = self.top(limit);
        if top.is_empty() {
            return "LEADERBOARD empty".into();
        }
        let parts: Vec<String> = top
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let name = if e.name.is_empty() {
                    format!("P{}", e.p_id)
                } else {
                    e.name.clone()
                };
                format!("{}:{}={}", i + 1, name, e.score)
            })
            .collect();
        format!("LEADERBOARD {}", parts.join(" "))
    }

    /// Highscore line for `?HIGHSCORE` — top by prestige (not score).
    ///
    /// `prestiges` is `(p_id, prestige)` pairs. Names resolve from scoreboard
    /// rows when present. Ties break by lower `p_id`.
    pub fn format_highscore_text(&self, prestiges: &[(i32, f32)], limit: usize) -> String {
        if prestiges.is_empty() || limit == 0 {
            return "HIGHSCORE empty".into();
        }
        let mut rows: Vec<(i32, String, f32)> = prestiges
            .iter()
            .map(|(p_id, prest)| {
                let name = self
                    .entries
                    .get(p_id)
                    .map(|e| {
                        if e.name.is_empty() {
                            format!("P{p_id}")
                        } else {
                            e.name.clone()
                        }
                    })
                    .unwrap_or_else(|| format!("P{p_id}"));
                (*p_id, name, *prest)
            })
            .collect();
        rows.sort_by(|a, b| {
            b.2.partial_cmp(&a.2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        rows.truncate(limit);
        if rows.is_empty() {
            return "HIGHSCORE empty".into();
        }
        let parts: Vec<String> = rows
            .iter()
            .enumerate()
            .map(|(i, (p_id, name, prest))| {
                let _ = p_id;
                format!("{}:{}={:.1}", i + 1, name, prest)
            })
            .collect();
        format!("HIGHSCORE {}", parts.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_score_formula() {
        assert_eq!(compute_score(0, 0, 0, 0), 0);
        assert_eq!(compute_score(1, 0, 0, 0), SCORE_PER_KILL);
        assert_eq!(compute_score(0, 1, 0, 0), -SCORE_PER_DEATH);
        assert_eq!(
            compute_score(2, 1, 5, 0),
            2 * SCORE_PER_KILL - SCORE_PER_DEATH + 5
        );
        assert_eq!(compute_score(0, 0, 0, 7), 7);
        assert_eq!(
            compute_score(1, 0, 3, 2),
            SCORE_PER_KILL + 3 + 2
        );
    }

    #[test]
    fn kill_and_coins_update_rank() {
        let mut sb = Scoreboard::default();
        sb.ensure_player(1, "Alice");
        sb.ensure_player(2, "Bob");
        sb.set_coins(1, 5);
        sb.set_coins(2, 5);
        sb.record_kill(1, 2);

        let a = sb.entry(1).unwrap();
        assert_eq!(a.kills, 1);
        assert_eq!(a.deaths, 0);
        assert_eq!(a.score, SCORE_PER_KILL + 5);

        let b = sb.entry(2).unwrap();
        assert_eq!(b.kills, 0);
        assert_eq!(b.deaths, 1);
        assert_eq!(b.score, -SCORE_PER_DEATH + 5);

        let ranked = sb.ranked();
        assert_eq!(ranked[0].p_id, 1);
        assert_eq!(ranked[1].p_id, 2);
    }

    #[test]
    fn pay_style_coin_sync_recomputes() {
        let mut sb = Scoreboard::default();
        sb.ensure_player(1, "A");
        sb.ensure_player(2, "B");
        sb.set_coins(1, 10);
        sb.set_coins(2, 0);
        // A pays B 3
        sb.set_coins(1, 7);
        sb.set_coins(2, 3);
        assert_eq!(sb.entry(1).unwrap().score, 7);
        assert_eq!(sb.entry(2).unwrap().score, 3);
        assert_eq!(sb.top(1)[0].p_id, 1);
    }

    #[test]
    fn format_score_and_leaderboard() {
        let mut sb = Scoreboard::default();
        sb.ensure_player(1, "Alice");
        sb.ensure_player(2, "Bob");
        sb.set_coins(1, 3);
        sb.record_kill(1, 2);
        let s = sb.format_score_text(1);
        assert!(s.contains("SCORE"));
        assert!(s.contains("K1"));
        assert!(s.contains("Alice"));
        assert!(s.contains("B0"));
        let lb = sb.format_leaderboard_text(10);
        assert!(lb.starts_with("LEADERBOARD"));
        assert!(lb.contains("Alice"));
        assert!(lb.contains("1:"));
    }

    #[test]
    fn self_kill_ignored() {
        let mut sb = Scoreboard::default();
        sb.ensure_player(1, "X");
        sb.record_kill(1, 1);
        assert_eq!(sb.entry(1).unwrap().kills, 0);
        assert_eq!(sb.entry(1).unwrap().deaths, 0);
    }

    #[test]
    fn record_death_increments() {
        let mut sb = Scoreboard::default();
        sb.ensure_player(4, "Suzy");
        sb.set_coins(4, 10);
        sb.record_death(4);
        let e = sb.entry(4).unwrap();
        assert_eq!(e.deaths, 1);
        assert_eq!(e.kills, 0);
        assert_eq!(e.score, 10 - SCORE_PER_DEATH);
        sb.record_death(4);
        assert_eq!(sb.entry(4).unwrap().deaths, 2);
    }

    #[test]
    fn tie_break_by_p_id() {
        let mut sb = Scoreboard::default();
        sb.ensure_player(5, "E");
        sb.ensure_player(3, "C");
        sb.set_coins(5, 1);
        sb.set_coins(3, 1);
        let r = sb.ranked();
        assert_eq!(r[0].p_id, 3);
        assert_eq!(r[1].p_id, 5);
    }

    #[test]
    fn season_bonus_adds_to_score() {
        let mut sb = Scoreboard::default();
        sb.ensure_player(1, "Alice");
        sb.set_coins(1, 5);
        sb.add_season_bonus(1, 20);
        let e = sb.entry(1).unwrap();
        assert_eq!(e.season_bonus, 20);
        assert_eq!(e.score, 25);
        sb.set_season_bonus(1, 3);
        assert_eq!(sb.entry(1).unwrap().score, 8);
    }

    #[test]
    fn on_season_change_resets_kills_deaths_and_bonus_keeps_coins() {
        let mut sb = Scoreboard::default();
        sb.ensure_player(1, "Alice");
        sb.ensure_player(2, "Bob");
        sb.set_coins(1, 12);
        sb.set_coins(2, 4);
        sb.record_kill(1, 2);
        sb.add_season_bonus(1, 50);
        assert_eq!(sb.entry(1).unwrap().kills, 1);
        assert_eq!(sb.entry(2).unwrap().deaths, 1);
        assert_eq!(sb.entry(1).unwrap().season_bonus, 50);

        // First bind only sets tag — does not wipe in-progress season stats.
        assert!(!sb.on_season_change("SPRING"));
        assert_eq!(sb.season_tag, "SPRING");
        assert_eq!(sb.entry(1).unwrap().kills, 1);
        assert_eq!(sb.entry(1).unwrap().season_bonus, 50);

        // Real season change wipes seasonal fields, keeps coins/names.
        assert!(sb.on_season_change("SUMMER"));
        assert_eq!(sb.season_tag, "SUMMER");

        let a = sb.entry(1).unwrap();
        assert_eq!(a.kills, 0);
        assert_eq!(a.deaths, 0);
        assert_eq!(a.season_bonus, 0);
        assert_eq!(a.coins, 12);
        assert_eq!(a.score, 12);
        assert_eq!(a.name, "Alice");

        let b = sb.entry(2).unwrap();
        assert_eq!(b.kills, 0);
        assert_eq!(b.deaths, 0);
        assert_eq!(b.coins, 4);
        assert_eq!(b.score, 4);

        // Same season tag is a no-op (does not wipe fresh season stats).
        sb.record_kill(1, 2);
        assert!(!sb.on_season_change("SUMMER"));
        assert_eq!(sb.entry(1).unwrap().kills, 1);

        // Next season resets again.
        assert!(sb.on_season_change("AUTUMN"));
        assert_eq!(sb.entry(1).unwrap().kills, 0);
    }

    #[test]
    fn format_highscore_ranks_by_prestige() {
        let mut sb = Scoreboard::default();
        sb.ensure_player(1, "Alice");
        sb.ensure_player(2, "Bob");
        sb.ensure_player(3, "Cara");
        let prestiges = [(1, 5.0_f32), (2, 40.0), (3, 40.0)];
        let hs = sb.format_highscore_text(&prestiges, 10);
        assert!(hs.starts_with("HIGHSCORE"));
        // Bob and Cara tie at 40; lower p_id first → Bob then Cara; Alice last.
        assert!(hs.contains("1:Bob=40.0"));
        assert!(hs.contains("2:Cara=40.0"));
        assert!(hs.contains("3:Alice=5.0"));

        let empty = Scoreboard::default().format_highscore_text(&[], 10);
        assert_eq!(empty, "HIGHSCORE empty");

        let limited = sb.format_highscore_text(&prestiges, 1);
        assert!(limited.contains("1:Bob=40.0"));
        assert!(!limited.contains("Alice"));
    }
}
