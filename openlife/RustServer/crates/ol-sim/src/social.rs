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
//! **LINEAGE-BIRTH-TIME:** `birth_sim_time` (Haxe `birthTime`) OLN3; living ages from
//! `round(yearsSinceBirth - yearsSinceDeath)`.

use crate::prestige::PrestigeClass;
pub use ol_identity::LineageNode;
use ol_identity::LINEAGE_STARTING_FAMILY_NAME;
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
    /// Haxe `Lineage.trueAge` at death (OLN14).
    #[serde(default)]
    pub true_age_at_death: f32,
    /// Haxe `Lineage.birthTime` (sim seconds); `< 0` = unknown.
    // Haxe: Lineage.birthTime — LINEAGE-BIRTH-TIME
    #[serde(default = "lineage_birth_unknown_serde")]
    pub birth_sim_time: f32,
    /// Haxe `Lineage.alive`.
    #[serde(default)]
    pub alive: bool,
    /// Haxe `Lineage.myEveId` (0 = unset; OLN5).
    #[serde(default)]
    pub my_eve_id: i32,
    /// Haxe `Lineage.lastSaid` (OLN4).
    #[serde(default)]
    pub last_said: String,
    /// Haxe `Lineage.reputation` (OLN6; −lostCombatPrestige at death).
    #[serde(default)]
    pub reputation: f32,
    /// Haxe `Lineage.coins` death snapshot (OLN7; wallet persist is separate).
    #[serde(default)]
    pub coins: f32,
    /// Haxe `Lineage.myFamilyName` (OLN8).
    #[serde(default)]
    pub family_name: String,
    /// Haxe `Lineage.po_id` (OLN9; −1 = unset).
    #[serde(default = "lineage_po_id_unknown_serde")]
    pub po_id: i32,
    /// Haxe `Lineage.accountId` (OLN10; 0 = unset).
    #[serde(default)]
    pub account_id: i32,
    /// Haxe `Lineage.myDynastyId` (OLN11; −1 = unset).
    #[serde(default = "lineage_dynasty_unknown_serde")]
    pub my_dynasty_id: i32,
    /// Haxe `Lineage.followPlayerId` (OLN12; 0 = none).
    #[serde(default)]
    pub follow_player_id: i32,
    /// Haxe `Lineage.killedByPlayerId` (OLN13; 0 = none).
    #[serde(default)]
    pub killed_by_player_id: i32,
    /// Haxe `prestigeFromChildren` (OLN15).
    #[serde(default)]
    pub prestige_from_children: f32,
    /// Haxe `prestigeFromGrandkids` (OLN15; Haxe TODO unsaved).
    #[serde(default)]
    pub prestige_from_grandkids: f32,
    /// Haxe `prestigeFromEating` (OLN15).
    #[serde(default)]
    pub prestige_from_eating: f32,
    /// Haxe `prestigeFromFollowers` (OLN15).
    #[serde(default)]
    pub prestige_from_followers: f32,
    /// Haxe `prestigeFromWealth` (OLN15).
    #[serde(default)]
    pub prestige_from_wealth: f32,
    /// Haxe `prestigeFromParents` (OLN15; Haxe TODO unsaved).
    #[serde(default)]
    pub prestige_from_parents: f32,
    /// Haxe `prestigeFromSiblings` (OLN15; Haxe TODO unsaved).
    #[serde(default)]
    pub prestige_from_siblings: f32,
}

fn lineage_birth_unknown_serde() -> f32 {
    ol_identity::LINEAGE_BIRTH_UNKNOWN
}

fn lineage_po_id_unknown_serde() -> i32 {
    -1
}

fn lineage_dynasty_unknown_serde() -> i32 {
    -1
}

/// Lineage book snapshot (no SQL; mirrors OLN1/OLN2/OLN3 in-memory state).
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

/// Walk `mother_id` to the Eve root (Haxe `lineage.myEveId`).
pub fn lineage_eve_id(nodes: &HashMap<i32, LineageNode>, mut p_id: i32) -> i32 {
    for _ in 0..64 {
        match nodes.get(&p_id).and_then(|n| n.mother_id) {
            Some(m) if m > 0 && m != p_id => p_id = m,
            _ => break,
        }
    }
    p_id
}

/// Haxe `ServerSettings.AiNameEnding` (NAME `getFullName` suffix).
// Haxe: ServerSettings.AiNameEnding = 'X'
pub const AI_NAME_ENDING: &str = "X";

/// Haxe `Lineage.familyName` getter: Eve `myFamilyName`, else this node / fallback.
// Haxe: Lineage.get_familyName L623–627
pub fn resolved_family_name(
    nodes: &HashMap<i32, LineageNode>,
    p_id: i32,
    fallback: &str,
) -> String {
    let Some(node) = nodes.get(&p_id) else {
        let t = fallback.trim();
        return if t.is_empty() {
            LINEAGE_STARTING_FAMILY_NAME.to_string()
        } else {
            t.to_string()
        };
    };
    let eve = if node.my_eve_id > 0 {
        node.my_eve_id
    } else {
        lineage_eve_id(nodes, p_id)
    };
    let from_eve = nodes
        .get(&eve)
        .map(|n| n.family_name.trim())
        .filter(|s| !s.is_empty());
    if let Some(s) = from_eve {
        return s.to_string();
    }
    let mine = node.family_name.trim();
    if !mine.is_empty() {
        return mine.to_string();
    }
    let t = fallback.trim();
    if t.is_empty() {
        LINEAGE_STARTING_FAMILY_NAME.to_string()
    } else {
        t.to_string()
    }
}

/// Haxe `Lineage.getFullName(withUnderscore, ignoreFirstName)`.
// Haxe: Lineage.getFullName L526–533
pub fn lineage_get_full_name(
    family_name: &str,
    class_name: &str,
    is_ai: bool,
    with_underscore: bool,
    ignore_first_name: bool,
    first_name: &str,
) -> String {
    let ai = if is_ai { AI_NAME_ENDING } else { "" };
    let full = if ignore_first_name {
        format!("{family_name}{ai} {class_name}")
    } else {
        format!("{first_name} {family_name}{ai} {class_name}")
    };
    if with_underscore {
        full.replace(' ', "_")
    } else {
        full
    }
}

/// Haxe NAME body: `p_id first getFullName(true, true)`.
// Haxe: Connection.sendToMePlayerInfo L446–448
pub fn format_name_lineage_body(
    p_id: i32,
    first_name: &str,
    family_name: &str,
    class_name: &str,
    is_ai: bool,
) -> String {
    let last = lineage_get_full_name(family_name, class_name, is_ai, true, true, first_name);
    format!("{p_id} {first_name} {last}")
}

/// NAME line from live lineage (Eve family + prestige class).
pub fn format_player_nm_line_ex(
    lineages: &HashMap<i32, LineageNode>,
    p_id: i32,
    first_name: &str,
    family_fallback: &str,
    is_ai: bool,
) -> String {
    let class = lineages
        .get(&p_id)
        .map(|n| n.prestige_class.class_name())
        .unwrap_or("Commoner");
    let family = resolved_family_name(lineages, p_id, family_fallback);
    format_name_lineage_body(p_id, first_name, &family, class, is_ai)
}

/// Haxe `Lineage.createLineageString` — ancestor ids for LN / GRAVE_OLD.
///
/// `with_me=false` omits `p_id` (GO query). Eve-not-reached appends ` eve_id=`.
// Haxe: Lineage.createLineageString L641–669
pub fn create_lineage_string(
    nodes: &HashMap<i32, LineageNode>,
    p_id: i32,
    with_me: bool,
) -> String {
    let Some(node) = nodes.get(&p_id) else {
        return String::new();
    };
    let eve = if node.my_eve_id > 0 {
        node.my_eve_id
    } else {
        lineage_eve_id(nodes, p_id)
    };
    let mut s = if with_me {
        p_id.to_string()
    } else {
        String::new()
    };
    if p_id == eve {
        return s;
    }
    let mut cur = node.mother_id.filter(|&m| m > 0);
    let mut added_eve = false;
    for _ in 0..10 {
        let Some(mid) = cur else {
            break;
        };
        if !s.is_empty() {
            s.push(' ');
        }
        s.push_str(&mid.to_string());
        if mid == eve {
            added_eve = true;
            break;
        }
        cur = nodes.get(&mid).and_then(|n| n.mother_id).filter(|&m| m > 0);
    }
    if !added_eve && eve > 0 {
        // Haxe always prefixes a space: `+= ' eve_id=$myEveId'`
        s.push_str(&format!(" eve_id={eve}"));
    }
    s
}

#[derive(Debug, Default, Clone)]
pub struct SocialState {
    pub lineages: HashMap<i32, LineageNode>,
    /// follower_p_id → leader_p_id
    pub following: HashMap<i32, i32>,
    /// leader_p_id → set of exiled p_ids
    pub exiles: HashMap<i32, HashSet<i32>>,
    /// `(exiler, target)` → `sim_time` when the exile was stamped (session; not OLN).
    /// Missing → not recent (persist restore / tests that insert `exiles` directly).
    // Haxe: GPI L4525 TODO count as ally if exile happened not long ago (both sides)
    pub exile_times: HashMap<(i32, i32), f32>,
    /// badge color index per leader (0..7)
    pub leader_colors: HashMap<i32, i32>,
    /// Haxe `hiredByPlayer`: worker_p_id → boss_p_id (0 / missing = none).
    /// Session map (DO-COMMANDS / `I HIRE`); not OLN1-persisted.
    // Haxe: GlobalPlayerInstance.hiredByPlayer
    pub hired_by: HashMap<i32, i32>,
    /// Sim seconds at last vitals tick — drives Haxe `WriteAllLineages` prune.
    /// Session-only (not OLN); 0 skips prune so persist tests write the full map.
    // Haxe: TimeHelper.tick * tickTime at WriteAllLineages
    pub sim_time: f32,
}

impl SocialState {
    pub fn ensure_lineage(&mut self, p_id: i32, name: &str) {
        let t = self.sim_time;
        self.ensure_lineage_born_at(p_id, name, t);
    }

    /// Ensure a lineage node and stamp Haxe `birthTime` when inserting.
    // Haxe: Lineage.new `birthTime = TimeHelper.tick`
    pub fn ensure_lineage_born_at(&mut self, p_id: i32, name: &str, birth_sim_time: f32) {
        self.lineages.entry(p_id).or_insert_with(|| {
            let mut n = LineageNode::eve(p_id, name.to_string());
            n.stamp_birth(birth_sim_time);
            n
        });
    }

    /// Haxe `myFamilyName` / `setFamilyName`.
    // Haxe: Lineage.setFamilyName
    pub fn stamp_lineage_family_name(&mut self, p_id: i32, family_name: &str) {
        if let Some(n) = self.lineages.get_mut(&p_id) {
            n.stamp_family_name(family_name);
        }
    }

    /// Haxe `lineage.po_id = player.po_id`.
    // Haxe: Lineage.new L514 / GPI.setObjectId
    pub fn stamp_lineage_po_id(&mut self, p_id: i32, po_id: i32) {
        if let Some(n) = self.lineages.get_mut(&p_id) {
            n.stamp_po_id(po_id);
        }
    }

    /// Haxe `lineage.accountId = player.account.id`.
    // Haxe: Lineage.new L537
    pub fn stamp_lineage_account_id(&mut self, p_id: i32, account_id: i32) {
        if let Some(n) = self.lineages.get_mut(&p_id) {
            n.stamp_account_id(account_id);
        }
    }

    /// Haxe `lineage.myDynastyId` after found-new family.
    // Haxe: NamingHelper.DoNaming L137–162
    pub fn stamp_lineage_dynasty_id(&mut self, p_id: i32, dynasty_id: i32) {
        if let Some(n) = self.lineages.get_mut(&p_id) {
            n.stamp_dynasty_id(dynasty_id);
        }
    }

    /// Haxe `lineage.followPlayerId` (live follow).
    // Haxe: Lineage.followPlayerId WriteLineages L205
    pub fn stamp_lineage_follow_player_id(&mut self, p_id: i32, follow_player_id: i32) {
        if let Some(n) = self.lineages.get_mut(&p_id) {
            n.stamp_follow_player_id(follow_player_id);
        }
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
        self.ensure_lineage_born_at(p_id, name, self.sim_time);
        if let Some(n) = self.lineages.get_mut(&p_id) {
            n.begin_new_life(self.sim_time);
        }
    }

    /// Stamp lineage death fields (LINEAGE-24H), combat reputation, and coins snapshot.
    /// Ensures a node exists so death is never dropped when birth forgot the lineage.
    // Haxe: Lineage.deathTime / deathReason / alive=false
    // Haxe: GPI.doDeathHelper reputation + coins before InheritCoins
    pub fn stamp_lineage_death(
        &mut self,
        p_id: i32,
        death_sim_time: f32,
        reason: &str,
        age_years: f32,
        true_age_years: f32,
        lost_combat_prestige: f32,
        coins: f32,
        killed_by_player_id: i32,
    ) {
        // Ensure node: Haxe always has lineage on the player; Rust may miss edge paths.
        self.ensure_lineage(p_id, &format!("p{p_id}"));
        if let Some(n) = self.lineages.get_mut(&p_id) {
            n.stamp_death_ages(death_sim_time, reason, age_years, true_age_years);
            n.stamp_reputation_from_lost_combat(lost_combat_prestige);
            n.stamp_coins(coins);
            n.stamp_killed_by_player_id(killed_by_player_id);
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
                LineageStatRow::from_life_fields(
                    n.birth_sim_time,
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
            self.stamp_lineage_follow_player_id(follower, 0);
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
        self.stamp_lineage_follow_player_id(follower, leader);
        Ok(())
    }

    pub fn unfollow(&mut self, follower: i32) {
        self.following.remove(&follower);
        self.stamp_lineage_follow_player_id(follower, 0);
    }

    pub fn exile(&mut self, leader: i32, target: i32) {
        self.exiles.entry(leader).or_default().insert(target);
        // Being exiled ends follow relationship both ways.
        if self.following.get(&target) == Some(&leader) {
            self.following.remove(&target);
            self.stamp_lineage_follow_player_id(target, 0);
        }
        // Recency stamp for HIT/kill is_ally (Haxe L4525). Persist restore uses
        // sim_time 0 and is treated as not-recent in [`Self::recent_exile_edge`].
        if leader != 0 && target != 0 {
            self.exile_times.insert((leader, target), self.sim_time);
        }
    }

    /// True when `exiler` stamped an exile of `target` within `window` sim seconds.
    // Haxe: GPI L4525 TODO
    pub fn recent_exile_edge(&self, exiler: i32, target: i32, window: f32) -> bool {
        let Some(&t) = self.exile_times.get(&(exiler, target)) else {
            return false;
        };
        if t <= 0.0 {
            return false;
        }
        let w = if window.is_finite() && window >= 0.0 {
            window
        } else {
            0.0
        };
        let dt = self.sim_time - t;
        dt >= 0.0 && dt <= w
    }

    /// Recent exile in **either** direction (Haxe "both sides").
    pub fn recent_exile_between(&self, a: i32, b: i32, window: f32) -> bool {
        self.recent_exile_edge(a, b, window) || self.recent_exile_edge(b, a, window)
    }

    /// Rebuild session `following` from OLN `followPlayerId` (Haxe field on lineage).
    // Haxe: Lineage.followPlayerId ReadLineages L293
    pub fn restore_following_from_lineages(&mut self) {
        let mut pairs: Vec<(i32, i32)> = self
            .lineages
            .iter()
            .filter_map(|(&id, n)| {
                let lead = n.follow_player_id;
                if lead > 0 && lead != id {
                    Some((id, lead))
                } else {
                    None
                }
            })
            .collect();
        pairs.sort_unstable();
        for (follower, leader) in pairs {
            let _ = self.set_follow(follower, leader);
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

    /// Count living hired workers under `boss` (skip deleted and age > 55).
    // Haxe: countHiredPeople — deleted skip + `age > 55` skip
    pub fn count_hired(&self, boss: i32, ages: &HashMap<i32, f32>, deleted: &HashSet<i32>) -> i32 {
        self.hired_by
            .iter()
            .filter(|(&w, &b)| {
                if b != boss || deleted.contains(&w) {
                    return false;
                }
                // Haxe: if (p.age > 55) continue
                match ages.get(&w) {
                    Some(&age) if age > 55.0 => false,
                    _ => true,
                }
            })
            .count() as i32
    }

    /// LN body lines for bootstrap.
    pub fn lineage_packets(&self) -> Vec<String> {
        let mut ids: Vec<i32> = self.lineages.keys().copied().collect();
        ids.sort_unstable();
        ids.into_iter()
            .map(|id| create_lineage_string(&self.lineages, id, true))
            .filter(|s| !s.is_empty())
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
                    true_age_at_death: n.true_age_at_death,
                    birth_sim_time: n.birth_sim_time,
                    alive: n.alive,
                    my_eve_id: n.my_eve_id,
                    last_said: n.last_said.clone(),
                    reputation: n.reputation,
                    coins: n.coins,
                    family_name: n.family_name.clone(),
                    po_id: n.po_id,
                    account_id: n.account_id,
                    my_dynasty_id: n.my_dynasty_id,
                    follow_player_id: n.follow_player_id,
                    killed_by_player_id: n.killed_by_player_id,
                    prestige_from_children: n.prestige_from.children,
                    prestige_from_grandkids: n.prestige_from.grandkids,
                    prestige_from_eating: n.prestige_from.eating,
                    prestige_from_followers: n.prestige_from.followers,
                    prestige_from_wealth: n.prestige_from.wealth,
                    prestige_from_parents: n.prestige_from.parents,
                    prestige_from_siblings: n.prestige_from.siblings,
                })
            })
            .collect();
        let count = lineages.len();
        LineageSnapshot {
            lineages,
            count,
            format: "OLN15".into(),
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
                LineageStatRow::from_life_fields(
                    e.birth_sim_time,
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
pub fn format_following_for_player(social: &SocialState, follower: i32, top_leader: i32) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_lineage_string_eve_and_child() {
        let mut nodes = HashMap::new();
        nodes.insert(1, LineageNode::eve(1, "Eve"));
        let child = LineageNode::with_mother(2, "Kid", nodes.get(&1).unwrap());
        nodes.insert(2, child);
        assert_eq!(create_lineage_string(&nodes, 1, false), "");
        assert_eq!(create_lineage_string(&nodes, 2, false), "1");
        assert_eq!(create_lineage_string(&nodes, 2, true), "2 1");
    }

    #[test]
    fn create_lineage_string_appends_eve_id_when_truncated() {
        let mut nodes = HashMap::new();
        nodes.insert(1, LineageNode::eve(1, "Eve"));
        for id in 2..=12 {
            let mom = nodes.get(&(id - 1)).unwrap().clone();
            nodes.insert(id, LineageNode::with_mother(id, "N", &mom));
        }
        let s = create_lineage_string(&nodes, 12, true);
        assert!(
            s.starts_with("12 11 10 9 8 7 6 5 4 3 2"),
            "ten mother hops: {s}"
        );
        assert!(
            s.contains("eve_id=1"),
            "truncated chain must append eve_id=: {s}"
        );
        assert!(!s.contains(" 1\n") && !s.ends_with(" 1"));
    }

    #[test]
    fn lineage_get_full_name_name_packet_shape() {
        assert_eq!(
            lineage_get_full_name("SNOW", "Commoner", false, true, true, "ADA"),
            "SNOW_Commoner"
        );
        assert_eq!(
            lineage_get_full_name("SNOW", "Commoner", true, true, true, "ADA"),
            "SNOWX_Commoner"
        );
        assert_eq!(
            format_name_lineage_body(7, "ADA", "SNOW", "Commoner", false),
            "7 ADA SNOW_Commoner"
        );
    }
}
