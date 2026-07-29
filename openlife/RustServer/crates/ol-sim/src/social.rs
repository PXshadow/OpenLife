//! Lineage, following, exile — Haxe Connection bootstrap / social packets subset.
//!
//! Wire tags (server→client):
//! - LN / lineage lines (minimal family chain; Haxe LINEAGE)
//! - FW FOLLOWING: `follower_id leader_id color`
//! - EX EXILED: `target_id exiler_id` lines
//! - LR is LEARNED_TOOL_REPORT (tools), not lineage
//!
//! **LINEAGE-24H:** `death_sim_time` / `death_reason` / `age_at_death` for last-day
//! starving window. Persisted on OLN2; boot-seeded into `WorldFoodStats` death stamps.

use crate::prestige::{prestige_class_wire_token, PrestigeClass};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Haxe `ServerSettings.TimeConfirmNewFollower` default (seconds).
// Haxe: ServerSettings.TimeConfirmNewFollower = 15
pub const TIME_CONFIRM_NEW_FOLLOWER: f32 = 15.0;

/// Shared lineage list for web (`/api/lineages`, `/lineage`).
pub type LineageView = Arc<RwLock<LineageSnapshot>>;

/// One lineage node for JSON / HTML.
#[derive(Debug, Clone, Serialize)]
pub struct LineageEntryView {
    pub id: i32,
    pub name: String,
    pub mother_id: Option<i32>,
    pub father_id: Option<i32>,
    pub generation: i32,
    pub prestige: f32,
    pub prestige_class: String,
    /// Haxe `Lineage.deathTime` (sim seconds); 0 = never died / alive.
    // Haxe: Lineage.deathTime — WEB-LINEAGE-STATS
    #[serde(default)]
    pub death_sim_time: f32,
    /// Haxe `Lineage.deathReason` wire tag.
    // Haxe: Lineage.deathReason
    #[serde(default)]
    pub death_reason: String,
    /// Age years at death (for stats / kid remap).
    // Haxe: age at GenerateLineageStatistics
    #[serde(default)]
    pub age_at_death: f32,
    /// Haxe `Lineage.alive`.
    #[serde(default)]
    pub alive: bool,
}

/// Lineage book snapshot (no SQL; mirrors OLN1/OLN2 in-memory state).
#[derive(Debug, Clone, Serialize, Default)]
pub struct LineageSnapshot {
    pub lineages: Vec<LineageEntryView>,
    pub count: usize,
    /// On-disk format hint for operators.
    pub format: String,
    /// Sim time when this snapshot was published (for last-day/hour windows).
    // Haxe: TimeHelper.tick → years since death
    #[serde(default)]
    pub sim_time: f32,
}

/// Lightweight lineage node (not full Haxe Lineage.hx).
#[derive(Debug, Clone)]
pub struct LineageNode {
    pub id: i32,
    pub name: String,
    pub mother_id: Option<i32>,
    pub father_id: Option<i32>,
    pub generation: i32,
    pub prestige: f32,
    /// Haxe `lineage.prestigeClass` (kept in sync via [`Self::set_prestige`]).
    pub prestige_class: PrestigeClass,
    /// Haxe `Lineage.alive` — current life is active (birth-class samples require this).
    /// Session field (not OLN1-persisted); false after death, true on spawn/revive.
    // Haxe: Lineage.alive
    pub alive: bool,
    /// Haxe `Lineage.ownsObject` — map object still owned by this lineage (session;
    /// set by `InitObjectHelpersAfterRead`; not OLN1-persisted).
    pub owns_object: bool,
    /// Haxe `Lineage.deathTime` — sim seconds at death; 0 = never died / alive.
    /// LINEAGE-24H: OLN2-persisted; boot-seeded into starving death stamps.
    // Haxe: Lineage.deathTime
    pub death_sim_time: f32,
    /// Haxe `Lineage.deathReason` wire tag (empty if never died / cleared on new life).
    /// OLN2-persisted.
    // Haxe: Lineage.deathReason
    pub death_reason: String,
    /// Haxe age at death (for `reason_hunger_kid` remap); OLN2-persisted.
    // Haxe: Lineage.age at GenerateLineageStatistics
    pub age_at_death: f32,
}

impl LineageNode {
    pub fn eve(id: i32, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            mother_id: None,
            father_id: None,
            generation: 0,
            prestige: 0.0,
            prestige_class: PrestigeClass::from_prestige(0.0),
            alive: true,
            owns_object: false,
            death_sim_time: 0.0,
            death_reason: String::new(),
            age_at_death: 0.0,
        }
    }

    /// Child born to `mother` (generation + 1, mother_id set).
    pub fn with_mother(id: i32, name: impl Into<String>, mother: &LineageNode) -> Self {
        Self {
            id,
            name: name.into(),
            mother_id: Some(mother.id),
            father_id: None,
            generation: mother.generation.saturating_add(1),
            prestige: 0.0,
            prestige_class: PrestigeClass::from_prestige(0.0),
            alive: true,
            owns_object: false,
            death_sim_time: 0.0,
            death_reason: String::new(),
            age_at_death: 0.0,
        }
    }

    /// Update prestige and recompute class (Haxe `calculatePrestigeClass` subset).
    pub fn set_prestige(&mut self, prestige: f32) {
        self.prestige = prestige.max(0.0);
        self.prestige_class = PrestigeClass::from_prestige(self.prestige);
    }

    /// Set living-percentile prestige class without changing prestige float.
    ///
    /// Used by [`crate::SimState::refresh_living_prestige_classes`] (score rank path).
    pub fn set_prestige_class(&mut self, class: PrestigeClass) {
        self.prestige_class = class;
    }

    /// Add delta prestige and recompute class.
    pub fn add_prestige(&mut self, delta: f32) {
        self.set_prestige(self.prestige + delta);
    }

    /// Prestige class for this lineage (cached field).
    pub fn prestige_class(&self) -> PrestigeClass {
        self.prestige_class
    }

    /// Stamp death session fields (LINEAGE-24H).
    // Haxe: Lineage.deathTime / deathReason / age
    pub fn stamp_death(&mut self, death_sim_time: f32, reason: &str, age_years: f32) {
        self.alive = false;
        self.death_sim_time = if death_sim_time.is_finite() {
            death_sim_time.max(0.0)
        } else {
            0.0
        };
        self.death_reason = reason.trim().to_string();
        self.age_at_death = if age_years.is_finite() {
            age_years.max(0.0)
        } else {
            0.0
        };
    }

    /// Clear death session fields on new life (spawn / revive).
    // Haxe: new life clears deathTime until next death
    pub fn clear_death_for_new_life(&mut self) {
        self.alive = true;
        self.death_sim_time = 0.0;
        self.death_reason.clear();
        self.age_at_death = 0.0;
    }

    /// Haxe-style compact lineage summary for bootstrap (includes class + prestige).
    pub fn wire_line(&self) -> String {
        let mother = self.mother_id.unwrap_or(self.id);
        let class_tok = prestige_class_wire_token(self.prestige);
        format!(
            "{} eve={} gen={} name={} {}",
            self.id, mother, self.generation, self.name, class_tok
        )
    }
}

#[derive(Debug, Default, Clone)]
pub struct SocialState {
    pub lineages: HashMap<i32, LineageNode>,
    /// follower_p_id → leader_p_id
    pub following: HashMap<i32, i32>,
    /// leader_p_id → set of exiled p_ids
    pub exiles: HashMap<i32, HashSet<i32>>,
    /// badge color index per leader (0..7)
    pub leader_colors: HashMap<i32, i32>,
    /// Haxe `hiredByPlayer`: worker_p_id → boss_p_id (0 / missing = none).
    /// Session map (DO-COMMANDS / `I HIRE`); not OLN1-persisted.
    // Haxe: GlobalPlayerInstance.hiredByPlayer
    pub hired_by: HashMap<i32, i32>,
}

impl SocialState {
    pub fn ensure_lineage(&mut self, p_id: i32, name: &str) {
        self.lineages
            .entry(p_id)
            .or_insert_with(|| LineageNode::eve(p_id, name.to_string()));
    }

    /// Haxe `lineage.alive` stamp (spawn/revive → true; death → false).
    // Haxe: Lineage.alive
    pub fn set_lineage_alive(&mut self, p_id: i32, alive: bool) {
        if let Some(n) = self.lineages.get_mut(&p_id) {
            n.alive = alive;
        }
    }

    /// Ensure lineage node exists and mark current life alive (spawn / revive).
    // Haxe: lineage.alive = true on new life
    pub fn ensure_lineage_alive(&mut self, p_id: i32, name: &str) {
        self.ensure_lineage(p_id, name);
        if let Some(n) = self.lineages.get_mut(&p_id) {
            n.clear_death_for_new_life();
        }
    }

    /// Stamp lineage death fields (LINEAGE-24H). Ensures a node exists so death
    /// is never dropped when birth forgot to register the lineage.
    // Haxe: Lineage.deathTime / deathReason / alive=false
    pub fn stamp_lineage_death(
        &mut self,
        p_id: i32,
        death_sim_time: f32,
        reason: &str,
        age_years: f32,
    ) {
        // Ensure node: Haxe always has lineage on the player; Rust may miss edge paths.
        self.ensure_lineage(p_id, &format!("p{p_id}"));
        if let Some(n) = self.lineages.get_mut(&p_id) {
            n.stamp_death(death_sim_time, reason, age_years);
        }
    }

    /// Collect death/stat rows for starving window seed + web lineage stats.
    // Haxe: AllLineages scan in GenerateLineageStatistics
    pub fn lineage_stat_rows(&self) -> Vec<crate::world_food_stats::LineageStatRow> {
        use crate::world_food_stats::LineageStatRow;
        let mut rows: Vec<LineageStatRow> = self
            .lineages
            .values()
            .map(|n| {
                LineageStatRow::from_death_fields(
                    n.death_sim_time,
                    n.death_reason.clone(),
                    n.age_at_death,
                    n.generation,
                )
            })
            .collect();
        // Stable order for deterministic tests / dumps.
        rows.sort_by(|a, b| {
            a.death_sim_time
                .partial_cmp(&b.death_sim_time)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.generation.cmp(&b.generation))
                .then_with(|| a.death_reason.cmp(&b.death_reason))
        });
        rows
    }

    /// Set lineage prestige and recompute prestige class.
    pub fn set_lineage_prestige(&mut self, p_id: i32, prestige: f32) {
        if let Some(n) = self.lineages.get_mut(&p_id) {
            n.set_prestige(prestige);
        }
    }

    /// Set lineage prestige class from living score percentiles (no float change).
    pub fn set_lineage_prestige_class(&mut self, p_id: i32, class: PrestigeClass) {
        if let Some(n) = self.lineages.get_mut(&p_id) {
            n.set_prestige_class(class);
        }
    }

    /// Prestige class for a lineage id (default Serf if missing).
    pub fn prestige_class(&self, p_id: i32) -> PrestigeClass {
        self.lineages
            .get(&p_id)
            .map(|n| n.prestige_class())
            .unwrap_or(PrestigeClass::Serf)
    }

    pub fn set_follow(&mut self, follower: i32, leader: i32) -> Result<(), &'static str> {
        if follower == leader {
            self.following.remove(&follower);
            return Ok(());
        }
        // Reject obvious cycles: leader already follows follower chain back.
        let mut walk = leader;
        let mut guard = 0;
        while let Some(&next) = self.following.get(&walk) {
            if next == follower {
                return Err("circular_follow");
            }
            walk = next;
            guard += 1;
            if guard > 64 {
                break;
            }
        }
        self.following.insert(follower, leader);
        self.leader_colors.entry(leader).or_insert(0);
        Ok(())
    }

    pub fn unfollow(&mut self, follower: i32) {
        self.following.remove(&follower);
    }

    pub fn exile(&mut self, leader: i32, target: i32) {
        self.exiles.entry(leader).or_default().insert(target);
        // Being exiled ends follow relationship both ways.
        if self.following.get(&target) == Some(&leader) {
            self.following.remove(&target);
        }
    }

    pub fn is_exiled_by(&self, leader: i32, target: i32) -> bool {
        self.exiles
            .get(&leader)
            .map(|s| s.contains(&target))
            .unwrap_or(false)
    }

    /// Haxe `getLeaderWhoExiled`: first exiler of `target` that is `leader` or
    /// a follower of `leader` (top-of-chain check via follow map).
    // Haxe: GlobalPlayerInstance.getLeaderWhoExiled
    pub fn leader_who_exiled(&self, leader: i32, target: i32) -> Option<i32> {
        for (&exiler, set) in &self.exiles {
            if !set.contains(&target) {
                continue;
            }
            if exiler == leader {
                return Some(exiler);
            }
            // Exiler follows under leader's tree
            if self.is_follower_from(exiler, leader) {
                return Some(exiler);
            }
        }
        None
    }

    /// Haxe `isFollowerFrom(player)`: getTopLeader(from self) == player.
    ///
    /// Walks follow chain; true when `leader` appears as an ancestor leader of
    /// `follower` (including direct follow).
    // Haxe: GlobalPlayerInstance.isFollowerFrom
    pub fn is_follower_from(&self, follower: i32, leader: i32) -> bool {
        if follower == leader {
            return false;
        }
        let mut walk = follower;
        let mut guard = 0;
        while let Some(&next) = self.following.get(&walk) {
            if next == leader {
                return true;
            }
            if next == walk {
                break;
            }
            walk = next;
            guard += 1;
            if guard > 64 {
                break;
            }
        }
        false
    }

    /// Haxe redeem: clear exile edges on `target` authored by `exiler` or their followers.
    ///
    /// Returns number of exile edges removed.
    // Haxe: GlobalPlayerInstance.redeem
    pub fn redeem(&mut self, exiler: i32, target: i32) -> i32 {
        let mut removed = 0_i32;
        let mut leaders_to_clear: Vec<i32> = Vec::new();
        for (&leader, set) in &self.exiles {
            if !set.contains(&target) {
                continue;
            }
            if leader == exiler || self.is_follower_from(leader, exiler) {
                leaders_to_clear.push(leader);
            }
        }
        for leader in leaders_to_clear {
            if let Some(set) = self.exiles.get_mut(&leader) {
                if set.remove(&target) {
                    removed += 1;
                }
                if set.is_empty() {
                    self.exiles.remove(&leader);
                }
            }
        }
        removed
    }

    /// Haxe `hiredByPlayer` boss id (0 = none).
    // Haxe: GlobalPlayerInstance.hiredByPlayer
    pub fn hired_boss(&self, worker: i32) -> i32 {
        self.hired_by.get(&worker).copied().unwrap_or(0)
    }

    /// Set hire link (worker → boss). `boss == 0` clears.
    // Haxe: hiredByPlayer =
    pub fn set_hired(&mut self, worker: i32, boss: i32) {
        if boss == 0 {
            self.hired_by.remove(&worker);
        } else {
            self.hired_by.insert(worker, boss);
        }
    }

    /// Count living hired workers under `boss` (ages filter optional via maps).
    // Haxe: processHireCommand HireCostIncreasePerPerson
    pub fn count_hired(
        &self,
        boss: i32,
        _ages: &HashMap<i32, f32>,
        deleted: &HashSet<i32>,
    ) -> i32 {
        self.hired_by
            .iter()
            .filter(|(&w, &b)| b == boss && !deleted.contains(&w))
            .count() as i32
    }

    /// LN body lines for bootstrap.
    pub fn lineage_packets(&self) -> Vec<String> {
        let mut ids: Vec<i32> = self.lineages.keys().copied().collect();
        ids.sort_unstable();
        ids.into_iter()
            .filter_map(|id| self.lineages.get(&id).map(|n| n.wire_line()))
            .collect()
    }

    /// FW body lines for bootstrap (`follower leader color`).
    pub fn following_packets(&self) -> Vec<String> {
        let mut pairs: Vec<(i32, i32)> = self.following.iter().map(|(&f, &l)| (f, l)).collect();
        pairs.sort_unstable();
        pairs
            .into_iter()
            .map(|(f, l)| {
                let color = following_badge_color(self, l);
                format_following_line(f, l, color)
            })
            .collect()
    }

    /// EX body lines for bootstrap (`target exiler`).
    pub fn exile_packets(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut leaders: Vec<i32> = self.exiles.keys().copied().collect();
        leaders.sort_unstable();
        for leader in leaders {
            if let Some(set) = self.exiles.get(&leader) {
                let mut targets: Vec<i32> = set.iter().copied().collect();
                targets.sort_unstable();
                for t in targets {
                    out.push(format_exile_line(t, leader));
                }
            }
        }
        out
    }

    /// Web/API lineage snapshot.
    pub fn snapshot(&self) -> LineageSnapshot {
        self.snapshot_at(0.0)
    }

    /// Snapshot with sim clock for web last-day/hour death windows.
    // Haxe: GenerateLineageStatistics uses current time for yearsSinceDeath
    pub fn snapshot_at(&self, sim_time: f32) -> LineageSnapshot {
        let mut ids: Vec<i32> = self.lineages.keys().copied().collect();
        ids.sort_unstable();
        let lineages: Vec<LineageEntryView> = ids
            .into_iter()
            .filter_map(|id| {
                let n = self.lineages.get(&id)?;
                Some(LineageEntryView {
                    id: n.id,
                    name: n.name.clone(),
                    mother_id: n.mother_id,
                    father_id: n.father_id,
                    generation: n.generation,
                    prestige: n.prestige,
                    prestige_class: format!("{:?}", n.prestige_class),
                    death_sim_time: n.death_sim_time,
                    death_reason: n.death_reason.clone(),
                    age_at_death: n.age_at_death,
                    alive: n.alive,
                })
            })
            .collect();
        let count = lineages.len();
        LineageSnapshot {
            lineages,
            count,
            format: "OLN2".into(),
            sim_time: if sim_time.is_finite() {
                sim_time.max(0.0)
            } else {
                0.0
            },
        }
    }
}

impl LineageSnapshot {
    /// Rows for [`crate::generate_lineage_statistics`] (Haxe AllLineages scan).
    // Haxe: Lineage.GenerateLineageStatistics
    pub fn stat_rows(&self) -> Vec<crate::world_food_stats::LineageStatRow> {
        use crate::world_food_stats::LineageStatRow;
        self.lineages
            .iter()
            .map(|e| {
                LineageStatRow::from_death_fields(
                    e.death_sim_time,
                    e.death_reason.clone(),
                    e.age_at_death,
                    e.generation,
                )
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Wire format helpers (FW / EX body lines + follow speech)
// ---------------------------------------------------------------------------

/// FW FOLLOWING body: `follower_id leader_id color`.
// Haxe: Connection.SendFollowing
pub fn format_following_line(follower: i32, leader: i32, color: i32) -> String {
    format!("{follower} {leader} {color}")
}

/// FW unfollowed: leader = -1 (Haxe FOLLOW ME / clear).
// Haxe: followPlayer = null → FW with -1
pub fn format_following_line_unfollowed(follower: i32) -> String {
    format!("{follower} -1 0")
}

/// EX EXILED body: `target_id exiler_id`.
// Haxe: Connection.SendExile
pub fn format_exile_line(target: i32, exiler: i32) -> String {
    format!("{target} {exiler}")
}

/// Badge color for a leader (0 default).
pub fn following_badge_color(social: &SocialState, leader: i32) -> i32 {
    social.leader_colors.get(&leader).copied().unwrap_or(0)
}

/// FW line for follower under resolved top leader.
// Haxe: Connection.SendFollowingToAll
pub fn format_following_for_player(
    social: &SocialState,
    follower: i32,
    top_leader: i32,
) -> String {
    let color = following_badge_color(social, top_leader);
    // Unfollowed self-top uses -1
    if top_leader == follower || top_leader <= 0 {
        format_following_line_unfollowed(follower)
    } else {
        format_following_line(follower, top_leader, color)
    }
}

/// Haxe say: `I FOLLOW SOON ${name}`.
// Haxe: processFollowCommand L2300
pub fn format_i_follow_soon(name: &str) -> String {
    format!("I FOLLOW SOON {name}")
}

/// Haxe say: `I follow now ${name} ${family}`.
// Haxe: TimeHelper L428
pub fn format_i_follow_now(name: &str, family: &str) -> String {
    format!("I follow now {name} {family}")
}

/// Haxe GM: `YOU_HAVE_A_NEW_FOLLOWER:_${name}_${family}`.
// Haxe: processFollowCommand L2289
pub fn format_you_have_new_follower(name: &str, family: &str) -> String {
    format!("YOU_HAVE_A_NEW_FOLLOWER:_{name}_{family}")
}

/// Haxe GM: `In ${secs} seconds you follow ${name}_${family}`.
// Haxe: processFollowCommand L2283
pub fn format_follow_pending_global(secs: f32, name: &str, family: &str) -> String {
    let s = if secs.is_finite() && secs > 0.0 {
        secs.round() as i32
    } else {
        TIME_CONFIRM_NEW_FOLLOWER as i32
    };
    format!("In {s} seconds you follow {name}_{family}")
}
