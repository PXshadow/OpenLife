//! Minimal combat / prestige (Haxe GlobalPlayerInstance kill / doDamage subset).
//!
//! Wound stack + Chebyshev kill range for hit resolution; `resolve_kill` remains
//! the one-shot prestige path used by SAY KILL when range checks pass.
//!
//! Haxe `DoDamage`: clothing insulation + floor → `protectionFactor = 1/(p+1)`,
//! held weapon `damageProtectionFactor`, hits accumulate and cut food_store_max.

use crate::prestige::PrestigeClass;
use std::collections::HashMap;

/// Max Chebyshev distance for a successful hit / kill (default / bare hands).
pub const KILL_RANGE: i32 = 2;
/// Food drain per second per wound stack while wounded.
pub const WOUND_BLEED_DRAIN: f32 = 0.05;
/// Cap on stacked wounds.
pub const MAX_WOUND: u8 = 5;
/// Wound stacks at or above this trigger a kill via [`CombatState::resolve_hit`].
pub const WOUND_KILL_THRESHOLD: u8 = 3;
/// Cumulative hits at/above this kill when food_max path is not used.
pub const HITS_KILL_THRESHOLD: f32 = 12.0;
/// Death when reduced food_store_max falls below this (Haxe DeathWithFoodStoreMax).
pub const FOOD_MAX_DEATH: f32 = 1.0;

#[derive(Debug, Clone, Default)]
pub struct CombatStats {
    pub prestige: f32,
    /// Negative = good fighter in Haxe lostCombatPrestige convention.
    pub lost_combat_prestige: f32,
    pub kills: u32,
    pub deaths: u32,
    /// Wound stacks (0 = healthy). Caps at [`MAX_WOUND`].
    pub wound: u8,
    /// Accumulated Haxe-style hit damage (reduces effective HP / food_max).
    pub hits: f32,
    /// Object id that last wounded this player (for death reason).
    pub wounded_by: i32,
}

impl CombatStats {
    /// Prestige class derived from combat prestige float.
    pub fn prestige_class(&self) -> PrestigeClass {
        PrestigeClass::from_prestige(self.prestige)
    }
}

/// Outcome of a ranged hit attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitResult {
    /// Target beyond the weapon's max range.
    Miss,
    /// Wound applied; value is the new wound stack.
    Wound(u8),
    /// Wound stack reached kill threshold (or equivalent) and target died.
    Kill,
}

#[derive(Debug, Default, Clone)]
pub struct CombatState {
    pub stats: HashMap<i32, CombatStats>,
}

impl CombatState {
    pub fn stats_mut(&mut self, p_id: i32) -> &mut CombatStats {
        self.stats.entry(p_id).or_default()
    }

    /// Current wound stacks for a player (0 if unknown).
    pub fn wound_of(&self, p_id: i32) -> u8 {
        self.stats.get(&p_id).map(|s| s.wound).unwrap_or(0)
    }

    /// Add wound stacks, capped at [`MAX_WOUND`]. Returns new wound value.
    pub fn apply_wound(&mut self, p_id: i32, amount: u8) -> u8 {
        let s = self.stats_mut(p_id);
        let next = (s.wound as u16 + amount as u16).min(MAX_WOUND as u16) as u8;
        s.wound = next;
        next
    }

    /// Clear all wound stacks for a player.
    pub fn clear_wound(&mut self, p_id: i32) {
        if let Some(s) = self.stats.get_mut(&p_id) {
            s.wound = 0;
        }
    }

    /// Extra food drain/sec while wounded: `wound * WOUND_BLEED_DRAIN` (0 if healthy).
    pub fn bleed_drain(&self, p_id: i32) -> f32 {
        let w = self.wound_of(p_id);
        if w == 0 {
            0.0
        } else {
            w as f32 * WOUND_BLEED_DRAIN
        }
    }

    /// Cumulative hit damage for a player.
    pub fn hits_of(&self, p_id: i32) -> f32 {
        self.stats.get(&p_id).map(|s| s.hits).unwrap_or(0.0)
    }

    /// Chebyshev distance between two tile positions.
    pub fn chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
        (ax - bx).abs().max((ay - by).abs())
    }

    /// Clothing + floor insulation → Haxe `protectionFactor = 1 / (protection + 1)`.
    ///
    /// Insulation typically 0..2 (each clothing slot ~0.5; floor ~0..1).
    pub fn protection_factor(clothing_insulation: f32, floor_insulation: f32) -> f32 {
        let protection = (clothing_insulation + floor_insulation).max(0.0);
        1.0 / (protection + 1.0)
    }

    /// Haxe-style damage roll before protection: `(org/2) + org * r` with `r` in \[0,1\].
    pub fn roll_base_damage(org_damage: f32, rng01: f32) -> f32 {
        let org = org_damage.max(0.0);
        let r = rng01.clamp(0.0, 1.0);
        (org / 2.0) + org * r
    }

    /// Apply clothing + weapon protection factors (Haxe DoDamage core multiply).
    pub fn apply_protection(
        base_damage: f32,
        clothing_insulation: f32,
        floor_insulation: f32,
        weapon_damage_protection: f32,
    ) -> f32 {
        let pf = Self::protection_factor(clothing_insulation, floor_insulation);
        let wpf = weapon_damage_protection.clamp(0.05, 1.5);
        (base_damage * pf * wpf).max(0.0)
    }

    /// Cap damage to half max food + 1 (Haxe DoDamage limit).
    pub fn cap_damage(damage: f32, food_store_max: f32) -> f32 {
        let cap = food_store_max / 2.0 + 1.0;
        damage.min(cap.max(0.0))
    }

    /// Record damage hits; returns new cumulative hits.
    pub fn apply_hits(&mut self, p_id: i32, damage: f32, wounded_by: i32) -> f32 {
        let s = self.stats_mut(p_id);
        s.hits = (s.hits + damage.max(0.0)).max(0.0);
        if wounded_by != 0 {
            s.wounded_by = wounded_by;
        }
        s.hits
    }

    /// Clear hits + wound (heal / post-kill).
    pub fn clear_hits(&mut self, p_id: i32) {
        if let Some(s) = self.stats.get_mut(&p_id) {
            s.hits = 0.0;
            s.wound = 0;
            s.wounded_by = 0;
        }
    }

    /// Attempt a hit with range + wound rules.
    ///
    /// - Distance &gt; `max_range` → [`HitResult::Miss`]
    /// - Else apply wound +1; if wound ≥ [`WOUND_KILL_THRESHOLD`] → resolve kill
    /// - Otherwise [`HitResult::Wound`]
    ///
    /// Pass weapon max range of the held object (or [`KILL_RANGE`] for bare hands).
    pub fn resolve_hit(
        &mut self,
        killer: i32,
        target: i32,
        killer_x: i32,
        killer_y: i32,
        target_x: i32,
        target_y: i32,
        legal: bool,
        max_range: i32,
    ) -> HitResult {
        self.resolve_hit_damaged(
            killer,
            target,
            killer_x,
            killer_y,
            target_x,
            target_y,
            legal,
            max_range,
            1.0, // default bare-hand damage path uses wound stacks primarily
            0.0,
            0.0,
            1.0,
            20.0,
            0,
            0.5, // fixed mid roll for deterministic tests / simple path
        )
        .0
    }

    /// Full Haxe-style hit: range check, damage roll + clothing protection, wound, optional kill.
    ///
    /// Returns `(HitResult, applied_damage)`. Kill when wound ≥ threshold **or**
    /// cumulative hits ≥ [`HITS_KILL_THRESHOLD`] **or** reduced food_max would die.
    #[allow(clippy::too_many_arguments)]
    pub fn resolve_hit_damaged(
        &mut self,
        killer: i32,
        target: i32,
        killer_x: i32,
        killer_y: i32,
        target_x: i32,
        target_y: i32,
        legal: bool,
        max_range: i32,
        org_damage: f32,
        clothing_insulation: f32,
        floor_insulation: f32,
        weapon_protection: f32,
        target_food_max: f32,
        weapon_id: i32,
        rng01: f32,
    ) -> (HitResult, f32) {
        if killer == target {
            return (HitResult::Miss, 0.0);
        }
        let dist = Self::chebyshev(killer_x, killer_y, target_x, target_y);
        if dist > max_range {
            return (HitResult::Miss, 0.0);
        }
        let base = Self::roll_base_damage(org_damage, rng01);
        let raw = Self::apply_protection(
            base,
            clothing_insulation,
            floor_insulation,
            weapon_protection,
        );
        let damage = Self::cap_damage(raw, target_food_max);
        let total_hits = self.apply_hits(target, damage, weapon_id);
        let wound = self.apply_wound(target, 1);
        let food_max_after = (target_food_max - damage).max(0.0);
        let lethal = wound >= WOUND_KILL_THRESHOLD
            || total_hits >= HITS_KILL_THRESHOLD
            || food_max_after < FOOD_MAX_DEATH;
        if lethal {
            if self.resolve_kill(killer, target, legal) {
                self.clear_hits(target);
                (HitResult::Kill, damage)
            } else {
                (HitResult::Wound(wound), damage)
            }
        } else {
            (HitResult::Wound(wound), damage)
        }
    }

    /// Attempt a kill. Returns true if target dies.
    /// Exiled targets can be killed legally (no prestige penalty on killer).
    pub fn resolve_kill(
        &mut self,
        killer: i32,
        target: i32,
        target_exiled_by_killer_side: bool,
    ) -> bool {
        if killer == target {
            return false;
        }
        {
            let t = self.stats_mut(target);
            t.deaths += 1;
            t.prestige = (t.prestige - 1.0).max(0.0);
        }
        {
            let k = self.stats_mut(killer);
            k.kills += 1;
            if target_exiled_by_killer_side {
                k.prestige += 0.5;
                k.lost_combat_prestige -= 0.1;
            } else {
                // Illegal kill — prestige hit.
                k.prestige = (k.prestige - 2.0).max(0.0);
                k.lost_combat_prestige += 1.0;
            }
        }
        true
    }

    /// Prestige class for a player (default Serf at 0 prestige).
    pub fn prestige_class(&self, p_id: i32) -> PrestigeClass {
        self.stats
            .get(&p_id)
            .map(|s| s.prestige_class())
            .unwrap_or(PrestigeClass::Serf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_exile_kill_gains_prestige() {
        let mut c = CombatState::default();
        assert!(c.resolve_kill(1, 2, true));
        assert_eq!(c.stats.get(&1).unwrap().kills, 1);
        assert!(c.stats.get(&1).unwrap().prestige > 0.0);
        assert_eq!(c.stats.get(&2).unwrap().deaths, 1);
    }

    #[test]
    fn illegal_kill_penalizes() {
        let mut c = CombatState::default();
        c.stats_mut(1).prestige = 5.0;
        assert!(c.resolve_kill(1, 2, false));
        assert!(c.stats.get(&1).unwrap().prestige < 5.0);
    }

    #[test]
    fn prestige_class_from_combat_stats() {
        let mut c = CombatState::default();
        assert_eq!(c.prestige_class(1), PrestigeClass::Serf);
        c.stats_mut(1).prestige = 60.0;
        assert_eq!(c.stats.get(&1).unwrap().prestige_class(), PrestigeClass::Noble);
    }

    #[test]
    fn hit_out_of_range_is_miss() {
        let mut c = CombatState::default();
        // Chebyshev 3 > KILL_RANGE 2
        let r = c.resolve_hit(1, 2, 0, 0, 3, 0, true, KILL_RANGE);
        assert_eq!(r, HitResult::Miss);
        assert_eq!(c.wound_of(2), 0);
        assert_eq!(c.stats.get(&1).map(|s| s.kills).unwrap_or(0), 0);
    }

    #[test]
    fn hit_in_range_applies_wound() {
        let mut c = CombatState::default();
        // Distance 2 is in range
        let r = c.resolve_hit(1, 2, 0, 0, 2, 0, false, KILL_RANGE);
        assert_eq!(r, HitResult::Wound(1));
        assert_eq!(c.wound_of(2), 1);
        assert!((c.bleed_drain(2) - WOUND_BLEED_DRAIN).abs() < 1e-6);

        let r2 = c.resolve_hit(1, 2, 0, 0, 1, 1, false, KILL_RANGE);
        assert_eq!(r2, HitResult::Wound(2));
        assert_eq!(c.wound_of(2), 2);
        assert!((c.bleed_drain(2) - 2.0 * WOUND_BLEED_DRAIN).abs() < 1e-6);
    }

    #[test]
    fn third_wound_kills() {
        let mut c = CombatState::default();
        assert_eq!(
            c.resolve_hit(1, 2, 0, 0, 0, 0, true, KILL_RANGE),
            HitResult::Wound(1)
        );
        assert_eq!(
            c.resolve_hit(1, 2, 0, 0, 0, 1, true, KILL_RANGE),
            HitResult::Wound(2)
        );
        let r = c.resolve_hit(1, 2, 0, 0, 1, 0, true, KILL_RANGE);
        assert_eq!(r, HitResult::Kill);
        assert_eq!(c.stats.get(&1).unwrap().kills, 1);
        assert_eq!(c.stats.get(&2).unwrap().deaths, 1);
        assert_eq!(c.wound_of(2), 0, "wound cleared on kill");
    }

    #[test]
    fn apply_wound_caps_at_max() {
        let mut c = CombatState::default();
        c.apply_wound(5, MAX_WOUND);
        assert_eq!(c.wound_of(5), MAX_WOUND);
        c.apply_wound(5, 3);
        assert_eq!(c.wound_of(5), MAX_WOUND);
        c.clear_wound(5);
        assert_eq!(c.wound_of(5), 0);
        assert_eq!(c.bleed_drain(5), 0.0);
    }

    #[test]
    fn resolve_kill_still_one_shot() {
        let mut c = CombatState::default();
        // Direct kill without wounds (one-shot path).
        assert!(c.resolve_kill(10, 20, false));
        assert_eq!(c.stats.get(&10).unwrap().kills, 1);
        assert_eq!(c.wound_of(20), 0);
    }

    #[test]
    fn hit_same_tile_in_range() {
        let mut c = CombatState::default();
        assert_eq!(
            c.resolve_hit(1, 2, 5, 5, 5, 5, true, KILL_RANGE),
            HitResult::Wound(1)
        );
    }

    #[test]
    fn bow_hit_reaches_distance_8() {
        let mut c = CombatState::default();
        let range = 8; // bow
        // Distance 5 misses with default, hits with bow.
        assert_eq!(
            c.resolve_hit(1, 2, 0, 0, 5, 0, true, KILL_RANGE),
            HitResult::Miss
        );
        assert_eq!(
            c.resolve_hit(1, 2, 0, 0, 5, 0, true, range),
            HitResult::Wound(1)
        );
        // Distance 9 still misses with bow.
        assert_eq!(
            c.resolve_hit(1, 2, 0, 0, 9, 0, true, range),
            HitResult::Miss
        );
    }

    #[test]
    fn spear_hit_reaches_distance_3() {
        let mut c = CombatState::default();
        let range = 3; // spear
        assert_eq!(
            c.resolve_hit(1, 2, 0, 0, 3, 0, true, range),
            HitResult::Wound(1)
        );
        assert_eq!(
            c.resolve_hit(1, 2, 0, 0, 4, 0, true, range),
            HitResult::Miss
        );
    }

    #[test]
    fn protection_factor_haxe_style() {
        // No clothing → factor 1.0
        assert!((CombatState::protection_factor(0.0, 0.0) - 1.0).abs() < 1e-6);
        // Full clothing 1.5 + no floor → 1/2.5 = 0.4
        assert!((CombatState::protection_factor(1.5, 0.0) - 0.4).abs() < 1e-6);
        // 1.0 insulation → 0.5
        assert!((CombatState::protection_factor(1.0, 0.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn damaged_hit_applies_hits_and_clothing() {
        let mut c = CombatState::default();
        // High clothing + shield → less damage than bare.
        let (r_bare, d_bare) = c.resolve_hit_damaged(
            1, 2, 0, 0, 0, 0, true, 2, 4.0, 0.0, 0.0, 1.0, 20.0, 99, 1.0,
        );
        assert!(matches!(r_bare, HitResult::Wound(1)));
        assert!(d_bare > 0.0);
        let hits_bare = c.hits_of(2);

        let mut c2 = CombatState::default();
        let (_r, d_arm) = c2.resolve_hit_damaged(
            1, 2, 0, 0, 0, 0, true, 2, 4.0, 1.5, 0.5, 0.5, 20.0, 99, 1.0,
        );
        assert!(d_arm < d_bare, "clothing+shield must reduce damage");
        assert!(c2.hits_of(2) < hits_bare);
    }

    #[test]
    fn high_hits_can_kill() {
        let mut c = CombatState::default();
        // Massive org damage, no protection, low food_max → lethal via food_max path.
        let (r, d) = c.resolve_hit_damaged(
            1, 2, 0, 0, 0, 0, true, 2, 30.0, 0.0, 0.0, 1.0, 2.0, 5, 1.0,
        );
        assert_eq!(r, HitResult::Kill);
        assert!(d > 0.0);
        assert_eq!(c.stats.get(&1).unwrap().kills, 1);
    }
}
