//! Lineage node (Haxe Lineage.hx subset).

use crate::{prestige_class_wire_token, PrestigeClass};
use std::collections::{HashMap, HashSet};

/// Haxe `ServerSettings.LineageDeleteAgeFactor` — keep dead lineage for `trueAge * factor` days
/// (died at 60 → delete after 90 days).
// Haxe: ServerSettings.LineageDeleteAgeFactor = 1.5
pub const LINEAGE_DELETE_AGE_FACTOR: f32 = 1.5;

/// Sentinel: Haxe `birthTime` missing (legacy OLN1/OLN2). `0` is a valid epoch birth.
// Haxe: Lineage.birthTime
pub const LINEAGE_BIRTH_UNKNOWN: f32 = -1.0;

/// Haxe `ServerSettings.StartingFamilyName` default (`myFamilyName` field init).
// Haxe: ServerSettings.StartingFamilyName = "SNOW"
pub const LINEAGE_STARTING_FAMILY_NAME: &str = "SNOW";

/// Haxe `GlobalPlayerInstance.prestigeFrom*` buckets (eat/family/leader fan).
///
/// Haxe saved children/eating/followers/wealth on Players.bin and left
/// grandkids/parents/siblings as `TODO not saved yet`. Open Life keeps all
/// seven on the lineage record (OLN15) so web/character pages survive restart.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PrestigeFromBreakdown {
    pub children: f32,
    pub grandkids: f32,
    pub eating: f32,
    pub followers: f32,
    pub wealth: f32,
    pub parents: f32,
    pub siblings: f32,
}

impl PrestigeFromBreakdown {
    pub fn add_eating(&mut self, amount: f32) {
        Self::accum(&mut self.eating, amount);
    }
    pub fn add_children(&mut self, amount: f32) {
        Self::accum(&mut self.children, amount);
    }
    pub fn add_grandkids(&mut self, amount: f32) {
        Self::accum(&mut self.grandkids, amount);
    }
    pub fn add_parents(&mut self, amount: f32) {
        Self::accum(&mut self.parents, amount);
    }
    pub fn add_siblings(&mut self, amount: f32) {
        Self::accum(&mut self.siblings, amount);
    }
    pub fn add_followers(&mut self, amount: f32) {
        Self::accum(&mut self.followers, amount);
    }
    pub fn add_wealth(&mut self, amount: f32) {
        Self::accum(&mut self.wealth, amount);
    }
    fn accum(slot: &mut f32, amount: f32) {
        if amount.is_finite() && amount != 0.0 {
            let next = *slot + amount;
            if next.is_finite() {
                *slot = next;
            }
        }
    }
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
    /// Haxe `Lineage.age` at death (aging-factor years); OLN2-persisted.
    // Haxe: Lineage.age WriteLineages L182 / GPI.doDeathHelper L3975
    pub age_at_death: f32,
    /// Haxe `Lineage.trueAge` at death (wall-clock years); OLN14-persisted.
    /// Used by `canBeDeleted`. Legacy OLN2–13 copied `age_at_death` (then true_age).
    // Haxe: Lineage.trueAge WriteLineages L183 / GPI.doDeathHelper L3976
    pub true_age_at_death: f32,
    /// Haxe `Lineage.birthTime` — sim seconds at birth. `< 0` = unknown (legacy OLN).
    /// `0` = born at server epoch (valid). OLN3-persisted.
    // Haxe: Lineage.birthTime
    pub birth_sim_time: f32,
    /// Haxe `lineage.myEveId` — family founder. 0 = unset (walk mother / self if Eve).
    /// OLN5-persisted; set on Eve/child spawn and `foundFamily`.
    // Haxe: Lineage.myEveId WriteLineages L191 / ReadLineages L277
    pub my_eve_id: i32,
    /// Haxe `Lineage.reputation` = `lostCombatPrestige * (-1)` at death (higher = better).
    /// OLN6-persisted. Live combat float stays on `CombatStats.lost_combat_prestige`.
    // Haxe: GPI.doDeathHelper L3982 / WriteLineages L199
    pub reputation: f32,
    /// Haxe `Lineage.coins` death snapshot (wallet persist is separate). OLN7-persisted.
    // Haxe: GPI.doDeathHelper L3983 / WriteLineages L189
    pub coins: f32,
    /// Haxe `Lineage.myFamilyName` (getter `familyName` walks Eve). OLN8-persisted.
    /// Player `family_name` is separate; this is the lineage record.
    // Haxe: Lineage.myFamilyName / familyName WriteLineages L178
    pub family_name: String,
    /// Haxe `Lineage.po_id` — person object / race id. `-1` = unset. OLN9-persisted.
    // Haxe: Lineage.po_id WriteLineages L180 / ReadLineages L266 / Lineage.new L514
    pub po_id: i32,
    /// Haxe `Lineage.accountId` — numeric `PlayerAccount.id`. `0` = unset. OLN10-persisted.
    // Haxe: Lineage.accountId WriteLineages L174 / ReadLineages L260 / Lineage.new L537
    pub account_id: i32,
    /// Haxe `Lineage.myDynastyId` — old Eve after a family split. `-1` = unset. OLN11-persisted.
    // Haxe: Lineage.myDynastyId WriteLineages L197 / ReadLineages L284 / NamingHelper L137–162
    pub my_dynasty_id: i32,
    /// Haxe `Lineage.followPlayerId` — live leader. `0` = none. OLN12-persisted.
    // Haxe: Lineage.followPlayerId WriteLineages L205 / ReadLineages L293
    pub follow_player_id: i32,
    /// Haxe `Lineage.killedByPlayerId` — last attacker at death. `0` = none. OLN13-persisted.
    // Haxe: Lineage.killedByPlayerId WriteLineages L204 / ReadLineages L292
    pub killed_by_player_id: i32,
    /// Haxe `Lineage.lastSaid` — last free-form SAY. OLN4-persisted.
    // Haxe: Lineage.lastSaid WriteLineages L187 / ReadLineages L273
    pub last_said: String,
    /// Family/eat/leader prestige sources. OLN15-persisted (Haxe Players.bin subset + TODOs).
    // Haxe: GPI.prestigeFromChildren/Grandkids/Eating/Followers/Wealth/Parents/Siblings
    pub prestige_from: PrestigeFromBreakdown,
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
            true_age_at_death: 0.0,
            birth_sim_time: LINEAGE_BIRTH_UNKNOWN,
            my_eve_id: id,
            last_said: String::new(),
            reputation: 0.0,
            coins: 0.0,
            family_name: LINEAGE_STARTING_FAMILY_NAME.to_string(),
            po_id: -1,
            account_id: 0,
            my_dynasty_id: -1,
            follow_player_id: 0,
            killed_by_player_id: 0,
            prestige_from: PrestigeFromBreakdown::default(),
        }
    }

    /// Child born to `mother` (generation + 1, mother_id set).
    pub fn with_mother(id: i32, name: impl Into<String>, mother: &LineageNode) -> Self {
        let founder = if mother.my_eve_id > 0 {
            mother.my_eve_id
        } else {
            mother.id
        };
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
            true_age_at_death: 0.0,
            birth_sim_time: LINEAGE_BIRTH_UNKNOWN,
            my_eve_id: founder,
            last_said: String::new(),
            reputation: 0.0,
            coins: 0.0,
            family_name: mother.family_name.clone(),
            po_id: -1,
            account_id: 0,
            my_dynasty_id: -1,
            follow_player_id: 0,
            killed_by_player_id: 0,
            prestige_from: PrestigeFromBreakdown::default(),
        }
    }

    /// Update prestige and recompute class (Haxe `calculatePrestigeClass` subset).
    pub fn set_prestige(&mut self, prestige: f32) {
        self.prestige = prestige.max(0.0);
        self.prestige_class = PrestigeClass::from_prestige(self.prestige);
    }

    /// Set living-percentile prestige class without changing prestige float.
    ///
    /// Used by living percentile refresh in ol-sim (score rank path).
    pub fn set_prestige_class(&mut self, class: PrestigeClass) {
        self.prestige_class = class;
    }

    /// Haxe `lineage.myEveId` (0 + no mother → self).
    // Haxe: Lineage.myEveId
    pub fn family_eve_id(&self) -> i32 {
        if self.my_eve_id > 0 {
            self.my_eve_id
        } else if self.mother_id.is_none() {
            self.id
        } else {
            0
        }
    }

    /// True when this life is the family Eve (`myEveId == id`).
    // Haxe: AiBase.foundFamily `lineage.myEveId == player.id`
    pub fn is_family_founder(&self) -> bool {
        let eve = self.family_eve_id();
        eve > 0 && eve == self.id
    }

    /// Add delta prestige and recompute class.
    pub fn add_prestige(&mut self, delta: f32) {
        self.set_prestige(self.prestige + delta);
    }

    /// Prestige class for this lineage (cached field).
    pub fn prestige_class(&self) -> PrestigeClass {
        self.prestige_class
    }

    /// Stamp death session fields (LINEAGE-24H). Sets both ages equal (tests / legacy).
    // Haxe: Lineage.deathTime / deathReason / age
    pub fn stamp_death(&mut self, death_sim_time: f32, reason: &str, age_years: f32) {
        self.stamp_death_ages(death_sim_time, reason, age_years, age_years);
    }

    /// Haxe `lineage.age` + `lineage.trueAge` at death.
    // Haxe: GPI.doDeathHelper L3975–3976
    pub fn stamp_death_ages(
        &mut self,
        death_sim_time: f32,
        reason: &str,
        age_years: f32,
        true_age_years: f32,
    ) {
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
        self.true_age_at_death = if true_age_years.is_finite() {
            true_age_years.max(0.0)
        } else {
            0.0
        };
    }

    /// Haxe `canBeDeleted` uses `trueAge` (legacy OLN: fall back to `age_at_death`).
    // Haxe: Lineage.canBeDeleted L102
    pub fn true_age_for_delete(&self) -> f32 {
        if self.true_age_at_death > 0.0 {
            self.true_age_at_death
        } else {
            self.age_at_death.max(0.0)
        }
    }

    /// Haxe `lineage.reputation = lostCombatPrestige * (-1)` (higher = better).
    // Haxe: GlobalPlayerInstance.doDeathHelper L3982
    pub fn stamp_reputation_from_lost_combat(&mut self, lost_combat_prestige: f32) {
        self.reputation = if lost_combat_prestige.is_finite() {
            -lost_combat_prestige
        } else {
            0.0
        };
    }

    /// Haxe `lineage.coins = this.coins` death snapshot.
    // Haxe: GlobalPlayerInstance.doDeathHelper L3983
    pub fn stamp_coins(&mut self, coins: f32) {
        self.coins = if coins.is_finite() { coins } else { 0.0 };
    }

    /// Haxe `myFamilyName` / `setFamilyName` (Eve record). Empty → StartingFamilyName.
    // Haxe: Lineage.myFamilyName / setFamilyName
    pub fn stamp_family_name(&mut self, family_name: &str) {
        let t = family_name.trim();
        self.family_name = if t.is_empty() {
            LINEAGE_STARTING_FAMILY_NAME.to_string()
        } else {
            t.to_string()
        };
    }

    /// Haxe `lineage.po_id = player.po_id` / `setObjectId`.
    // Haxe: Lineage.new L514 / GPI.setObjectId L943–945
    pub fn stamp_po_id(&mut self, po_id: i32) {
        self.po_id = if po_id > 0 { po_id } else { -1 };
    }

    /// Haxe `lineage.accountId = player.account.id`.
    // Haxe: Lineage.new L537
    pub fn stamp_account_id(&mut self, account_id: i32) {
        self.account_id = if account_id > 0 { account_id } else { 0 };
    }

    /// Haxe `myDynastyId` after found-new family (`< 1` → old Eve).
    // Haxe: NamingHelper.DoNaming L137–139
    pub fn stamp_dynasty_id(&mut self, dynasty_id: i32) {
        self.my_dynasty_id = if dynasty_id > 0 { dynasty_id } else { -1 };
    }

    /// Dynasty id for ChangeScore (`myDynastyId > 0`).
    // Haxe: PlayerAccount.ChangeScore L194
    pub fn dynasty_id(&self) -> i32 {
        if self.my_dynasty_id > 0 {
            self.my_dynasty_id
        } else {
            0
        }
    }

    /// Haxe `lineage.followPlayerId` (live `followPlayer.p_id`; 0 = none).
    // Haxe: Lineage.followPlayerId WriteLineages L205
    pub fn stamp_follow_player_id(&mut self, follow_player_id: i32) {
        self.follow_player_id = if follow_player_id > 0 {
            follow_player_id
        } else {
            0
        };
    }

    /// Haxe `lineage.killedByPlayerId` (live last attacker; 0 = none).
    // Haxe: Lineage.killedByPlayerId WriteLineages L204
    pub fn stamp_killed_by_player_id(&mut self, killed_by_player_id: i32) {
        self.killed_by_player_id = if killed_by_player_id > 0 {
            killed_by_player_id
        } else {
            0
        };
    }

    /// Stamp Haxe `lineage.birthTime` (sim seconds). Non-finite → unknown.
    // Haxe: Lineage.new `this.birthTime = TimeHelper.tick`
    pub fn stamp_birth(&mut self, birth_sim_time: f32) {
        self.birth_sim_time = if birth_sim_time.is_finite() {
            birth_sim_time.max(0.0)
        } else {
            LINEAGE_BIRTH_UNKNOWN
        };
    }

    /// True when [`Self::birth_sim_time`] is a real Haxe `birthTime` (`>= 0`).
    #[inline]
    pub fn has_birth_time(&self) -> bool {
        self.birth_sim_time.is_finite() && self.birth_sim_time >= 0.0
    }

    /// Clear death session fields on new life (spawn / revive).
    // Haxe: new life clears deathTime until next death
    pub fn clear_death_for_new_life(&mut self) {
        self.alive = true;
        self.death_sim_time = 0.0;
        self.death_reason.clear();
        self.age_at_death = 0.0;
        self.true_age_at_death = 0.0;
        self.killed_by_player_id = 0;
    }

    /// New life: clear death and stamp `birthTime`.
    // Haxe: Lineage.new birthTime = TimeHelper.tick
    pub fn begin_new_life(&mut self, birth_sim_time: f32) {
        self.clear_death_for_new_life();
        self.stamp_birth(birth_sim_time);
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

/// Minutes to keep a dead lineage on disk (Haxe `trueAge * 60 * 24 * LineageDeleteAgeFactor`).
// Haxe: Lineage.canBeDeleted L103
pub fn lineage_keep_minutes(true_age: f32) -> f32 {
    let age = if true_age.is_finite() {
        true_age.max(0.0)
    } else {
        0.0
    };
    age * 60.0 * 24.0 * LINEAGE_DELETE_AGE_FACTOR
}

/// Haxe `getDeadSince` — `floor((now_sim - death_sim_time) / 60)` minutes.
///
/// `death_sim_time <= 0` is the Rust never-died sentinel → 0 (do not treat as ancient).
// Haxe: Lineage.getDeadSince L536–541
pub fn dead_since_minutes(now_sim: f32, death_sim_time: f32) -> i32 {
    if !death_sim_time.is_finite() || death_sim_time <= 0.0 {
        return 0;
    }
    let now = if now_sim.is_finite() { now_sim } else { 0.0 };
    let secs = (now - death_sim_time).max(0.0);
    let minutes = (secs / 60.0).floor();
    if minutes >= i32::MAX as f32 {
        i32::MAX
    } else {
        minutes as i32
    }
}

/// Haxe `Lineage.canBeDeleted` (Eve / parent keep is [`plan_lineage_deletes`]).
// Haxe: Lineage.canBeDeleted L97–107
pub fn lineage_can_be_deleted(n: &LineageNode, now_sim: f32) -> bool {
    if n.alive {
        return false;
    }
    if n.owns_object {
        return false;
    }
    let death_since = dead_since_minutes(now_sim, n.death_sim_time) as f32;
    death_since > lineage_keep_minutes(n.true_age_for_delete())
}

/// Eve id for delete-keep (OLN5 `my_eve_id`, else walk `mother_id`).
// Haxe: Lineage.eveLineage / myEveId — load may have eveLineage == null
fn founder_id(n: &LineageNode, lineages: &HashMap<i32, LineageNode>) -> i32 {
    if n.my_eve_id > 0 {
        return n.my_eve_id;
    }
    let mut p = n.id;
    for _ in 0..64 {
        match lineages.get(&p).and_then(|x| x.mother_id) {
            Some(m) if m > 0 && m != p => p = m,
            _ => break,
        }
    }
    p
}

/// Haxe `setDelete(false)`: unmark this id then Eve / mother / father.
// Haxe: Lineage.setDelete L110–123
fn keep_chain(id: i32, lineages: &HashMap<i32, LineageNode>, delete: &mut HashSet<i32>) {
    if id <= 0 || !delete.remove(&id) {
        return;
    }
    let Some(n) = lineages.get(&id) else {
        return;
    };
    let eve = founder_id(n, lineages);
    if eve > 0 && eve != id {
        keep_chain(eve, lineages, delete);
    }
    if let Some(m) = n.mother_id {
        keep_chain(m, lineages, delete);
    }
    if let Some(f) = n.father_id {
        keep_chain(f, lineages, delete);
    }
}

/// Haxe `WriteLineages(..., deleteOld=true)` skip-on-save set.
///
/// In-memory map is unchanged (Haxe only skips the disk write). Kept rows protect
/// Eve / mother / father (Haxe `setDelete(false)`).
// Haxe: Lineage.WriteLineages L131–172
pub fn plan_lineage_deletes(
    lineages: &HashMap<i32, LineageNode>,
    now_sim: f32,
) -> HashSet<i32> {
    let mut delete: HashSet<i32> = lineages
        .iter()
        .filter(|(_, n)| lineage_can_be_deleted(n, now_sim))
        .map(|(&id, _)| id)
        .collect();
    let kept: Vec<i32> = lineages
        .keys()
        .copied()
        .filter(|id| !delete.contains(id))
        .collect();
    for id in kept {
        let Some(n) = lineages.get(&id) else {
            continue;
        };
        let eve = founder_id(n, lineages);
        if eve > 0 {
            keep_chain(eve, lineages, &mut delete);
        }
        if let Some(m) = n.mother_id {
            keep_chain(m, lineages, &mut delete);
        }
        if let Some(f) = n.father_id {
            keep_chain(f, lineages, &mut delete);
        }
    }
    delete
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Age-60 keep window in sim seconds (90 days).
    const AGE60_KEEP_SECS: f32 = 60.0 * 60.0 * 24.0 * LINEAGE_DELETE_AGE_FACTOR * 60.0;

    fn dead(id: i32, name: &str, age: f32, death_sim: f32) -> LineageNode {
        let mut n = LineageNode::eve(id, name);
        n.stamp_death(death_sim, "reason_hunger", age);
        n
    }

    #[test]
    fn dead_since_minutes_floors_like_haxe() {
        assert_eq!(dead_since_minutes(120.0, 0.0), 0);
        assert_eq!(dead_since_minutes(100.0 + 59.0, 100.0), 0);
        assert_eq!(dead_since_minutes(100.0 + 60.0, 100.0), 1);
        assert_eq!(dead_since_minutes(100.0 + 119.0, 100.0), 1);
        assert_eq!(dead_since_minutes(100.0 + 120.0, 100.0), 2);
        assert_eq!(dead_since_minutes(50.0, 100.0), 0);
    }

    #[test]
    fn can_be_deleted_age_60_after_90_days() {
        let death = 100.0;
        let n = dead(1, "X", 60.0, death);
        assert!(!lineage_can_be_deleted(&n, death + AGE60_KEEP_SECS));
        assert!(lineage_can_be_deleted(&n, death + AGE60_KEEP_SECS + 60.0));
    }

    /// Haxe `canBeDeleted` uses trueAge, not aging `age`.
    // Haxe: Lineage.canBeDeleted L102
    #[test]
    fn can_be_deleted_uses_true_age_not_aging_age() {
        let death = 100.0;
        let mut n = dead(1, "X", 1.0, death);
        n.true_age_at_death = 60.0;
        assert!(!lineage_can_be_deleted(&n, death + AGE60_KEEP_SECS));
        assert!(lineage_can_be_deleted(&n, death + AGE60_KEEP_SECS + 60.0));
    }

    #[test]
    fn can_be_deleted_skips_alive_and_owns_object() {
        let death = 100.0;
        let now = death + AGE60_KEEP_SECS + 60.0;
        let mut alive = dead(1, "A", 60.0, death);
        alive.alive = true;
        assert!(!lineage_can_be_deleted(&alive, now));
        let mut grave = dead(2, "G", 60.0, death);
        grave.owns_object = true;
        assert!(!lineage_can_be_deleted(&grave, now));
    }

    #[test]
    fn plan_keeps_eve_mother_father_of_survivor() {
        let death = 100.0;
        let now = death + AGE60_KEEP_SECS + 60.0;
        let mut map = HashMap::new();
        let mut eve = dead(1, "EVE", 60.0, death);
        eve.my_eve_id = 1;
        map.insert(1, eve);
        let mut father = dead(3, "DAD", 60.0, death);
        father.my_eve_id = 1;
        map.insert(3, father);
        let mut child = LineageNode::with_mother(2, "KID", map.get(&1).unwrap());
        child.father_id = Some(3);
        child.my_eve_id = 1;
        map.insert(2, child);
        let drop = plan_lineage_deletes(&map, now);
        assert!(drop.is_empty(), "survivor keeps Eve + father: {drop:?}");
    }

    #[test]
    fn plan_deletes_unrelated_old_dead() {
        let death = 100.0;
        let now = death + AGE60_KEEP_SECS + 60.0;
        let mut map = HashMap::new();
        map.insert(1, LineageNode::eve(1, "EVE"));
        map.insert(9, dead(9, "STRANGER", 60.0, death));
        let drop = plan_lineage_deletes(&map, now);
        assert_eq!(drop, HashSet::from([9]));
    }
}

