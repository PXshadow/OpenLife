//! Minimal combat / prestige (Haxe GlobalPlayerInstance kill / doDamage subset).
//!
//! Wound stack + Chebyshev kill range for hit resolution; `resolve_kill` remains
//! the one-shot kill/score-prestige path used by SAY KILL when range checks pass
//! (lostCombatPrestige is REPUTATION-HIT only — not mutated here).
//!
//! Haxe `DoDamage`: clothing insulation + floor → `protectionFactor = 1/(p+1)`,
//! held weapon `damageProtectionFactor`, hits + exhaustion cut food_store_max via
//! recompute ([`crate::food_store_max`]).
//!
//! Chunk **ALLY-STRENGTH** / `ally_combat`:
//! - `calculateEnemyVsAllyStrengthFactor`
//! - `makeAllCloseAllyAngryAt`
//! - DoDamage `allyFactor` (0.5 vs ally, else strength factor capped at 1.2)
//! - kill first-hit unarmed ally warn / second-hit exile (`resolve_unarmed_ally_hit_gate`)
//!
//! Chunk **EXHAUSTION-WOUND** / `wound_food_pipes`:
//! - `cap_damage` uses not-reduced base (~20), not live reduced food_max
//! - `resolve_hit_full` applies hits + exhaustion recompute death (`food_max < 0`)

use crate::food_store_max::{
    apply_damage_food_pipe, calculate_not_reduced_food_store_max, DEATH_WITH_FOOD_STORE_MAX,
};
use crate::move_live_gates::calculate_distance_sq;
use crate::prestige::PrestigeClass;
use std::collections::HashMap;

/// Snapshot of hits/exhaustion/food_max after a resolved damage roll.
// Haxe: DoDamage targetPlayer.hits / exhaustion / food_store_max after apply
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DamagePipeSnapshot {
    pub damage: f32,
    pub hits_after: f32,
    pub exhaustion_after: f32,
    pub food_store_max: f32,
    pub combat_lethal: bool,
}

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
/// Alias of Haxe `DeathWithFoodStoreMax` (−0.1). Prefer
/// [`crate::food_store_max::DEATH_WITH_FOOD_STORE_MAX`].
pub const FOOD_MAX_DEATH: f32 = DEATH_WITH_FOOD_STORE_MAX;

// --- ALLY-STRENGTH (Haxe GlobalPlayerInstance DoDamage / calculateEnemyVsAllyStrengthFactor) ---

/// Haxe `ServerSettings.AllyConsideredClose` (quad-distance radius in tiles).
// Haxe: ServerSettings.AllyConsideredClose
pub const ALLY_CONSIDERED_CLOSE: i32 = 5;
/// Base bias both ally and enemy strength start at.
// Haxe: calculateEnemyVsAllyStrengthFactor allyStrength = enemyStrength = 10.0
pub const ALLY_STRENGTH_BASE: f32 = 10.0;
/// Haxe DoDamage: `allyFactor > 1.2 ? 1.2 : allyFactor` when not ally-on-ally.
// Haxe: DoDamage allyFactor cap
pub const ALLY_STRENGTH_FACTOR_CAP: f32 = 1.2;
/// Haxe DoDamage: `if (targetPlayer.isAlly(attacker)) allyFactor = 0.5`.
// Haxe: DoDamage ally half damage
pub const ALLY_ON_ALLY_DAMAGE_FACTOR: f32 = 0.5;
/// Haxe `ServerSettings.AllyStrenghTooLowForPickup` default (0 = gate disabled).
// Haxe: ServerSettings.AllyStrenghTooLowForPickup
pub const ALLY_STRENGTH_TOO_LOW_FOR_PICKUP_DEFAULT: f32 = 0.0;

// --- C-SS-MORE-BATCH4 cursed damage multipliers (Haxe DoDamage) ---

/// Haxe `ServerSettings.CursedReceiveDamageFactor` — cursed target takes more damage.
// Haxe: ServerSettings.CursedReceiveDamageFactor = 1.2
// C-SS-MORE-BATCH4
pub const CURSED_RECEIVE_DAMAGE_FACTOR: f32 = 1.2;
/// Haxe `ServerSettings.CursedMakeDamageFactor` — cursed attacker deals less damage.
// Haxe: ServerSettings.CursedMakeDamageFactor = 0.5
// C-SS-MORE-BATCH4
pub const CURSED_MAKE_DAMAGE_FACTOR: f32 = 0.5;

/// Damage mult when target is cursed (else 1.0). Live factor override.
// Haxe: GlobalPlayerInstance.DoDamage L4628 damage *= targetPlayer.isCursed ? CursedReceiveDamageFactor : 1
// C-SS-MORE-BATCH4
#[inline]
pub fn cursed_receive_damage_mul(target_is_cursed: bool, factor: f32) -> f32 {
    if !target_is_cursed {
        return 1.0;
    }
    if factor.is_finite() && factor > 0.0 {
        factor
    } else {
        CURSED_RECEIVE_DAMAGE_FACTOR
    }
}

/// Damage mult when attacker is cursed (else 1.0). Live factor override.
// Haxe: GlobalPlayerInstance.DoDamage L4629 damage *= attacker.isCursed ? CursedMakeDamageFactor : 1
// C-SS-MORE-BATCH4
#[inline]
pub fn cursed_make_damage_mul(attacker_is_cursed: bool, factor: f32) -> f32 {
    if !attacker_is_cursed {
        return 1.0;
    }
    if factor.is_finite() && factor > 0.0 {
        factor
    } else {
        CURSED_MAKE_DAMAGE_FACTOR
    }
}

/// Snapshot of one living player for ally-strength / anger scans.
// Haxe: AllPlayers loop body in calculateEnemyVsAllyStrengthFactor / makeAllCloseAllyAngryAt
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AllyStrengthPlayer {
    pub p_id: i32,
    pub x: i32,
    pub y: i32,
    pub deleted: bool,
    /// Haxe `food_store_max` used as combat strength base.
    pub food_store_max: f32,
    /// Haxe `isHoldingWeapon()`.
    pub holding_weapon: bool,
    /// Haxe `p.isFriendly(observer)` for strength scan (`observer` = attacker).
    pub friendly_to_observer: bool,
    /// Haxe `p.isFriendly(targetPlayer)` when target is present.
    pub friendly_to_target: bool,
    /// Haxe `p.isAlly(observer)` for anger scan (`observer` = victim).
    pub ally_to_observer: bool,
}

/// Haxe strength per nearby player: weapon holders count double food_store_max.
// Haxe: p.isHoldingWeapon() ? 2 * p.food_store_max : p.food_store_max
#[inline]
pub fn combat_strength(food_store_max: f32, holding_weapon: bool) -> f32 {
    let s = food_store_max.max(0.0);
    if holding_weapon {
        2.0 * s
    } else {
        s
    }
}

/// Haxe `isCloseToPlayer(this, AllyConsideredClose)` — squared Euclidean ≤ distance².
// Haxe: GlobalPlayerInstance.isCloseToPlayer + ServerSettings.AllyConsideredClose
#[inline]
pub fn is_close_for_ally_strength(
    observer_x: i32,
    observer_y: i32,
    px: i32,
    py: i32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> bool {
    is_close_for_ally_strength_ex(
        observer_x,
        observer_y,
        px,
        py,
        map_w,
        map_h,
        wrap,
        ALLY_CONSIDERED_CLOSE as f32,
    )
}

/// Live radius variant of [`is_close_for_ally_strength`].
// Haxe: ServerSettings.AllyConsideredClose
// C-SS-MORE-BATCH3
#[inline]
pub fn is_close_for_ally_strength_ex(
    observer_x: i32,
    observer_y: i32,
    px: i32,
    py: i32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
    ally_considered_close: f32,
) -> bool {
    let max_d = if ally_considered_close.is_finite() && ally_considered_close > 0.0 {
        ally_considered_close as f64
    } else {
        ALLY_CONSIDERED_CLOSE as f64
    };
    calculate_distance_sq(observer_x, observer_y, px, py, map_w, map_h, wrap) <= max_d * max_d
}

/// Haxe `GlobalPlayerInstance.calculateEnemyVsAllyStrengthFactor(targetPlayer)`.
///
/// Scan players close to **observer** (attacker). Friendly → ally strength; else if
/// no target **or** friendly to target → enemy strength. Returns
/// `2 * ally / (enemy + ally)` (always in (0, 2] with base bias 10).
// Haxe: GlobalPlayerInstance.calculateEnemyVsAllyStrengthFactor
pub fn calculate_enemy_vs_ally_strength_factor(
    observer_x: i32,
    observer_y: i32,
    players: &[AllyStrengthPlayer],
    has_target: bool,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> f32 {
    calculate_enemy_vs_ally_strength_factor_ex(
        observer_x,
        observer_y,
        players,
        has_target,
        map_w,
        map_h,
        wrap,
        ALLY_CONSIDERED_CLOSE as f32,
    )
}

/// Live-radius variant of [`calculate_enemy_vs_ally_strength_factor`].
// Haxe: ServerSettings.AllyConsideredClose
// C-SS-MORE-BATCH3
pub fn calculate_enemy_vs_ally_strength_factor_ex(
    observer_x: i32,
    observer_y: i32,
    players: &[AllyStrengthPlayer],
    has_target: bool,
    map_w: i32,
    map_h: i32,
    wrap: bool,
    ally_considered_close: f32,
) -> f32 {
    let mut ally_strength = ALLY_STRENGTH_BASE;
    let mut enemy_strength = ALLY_STRENGTH_BASE;

    for p in players {
        if p.deleted {
            continue;
        }
        if !is_close_for_ally_strength_ex(
            observer_x,
            observer_y,
            p.x,
            p.y,
            map_w,
            map_h,
            wrap,
            ally_considered_close,
        ) {
            continue;
        }
        let strength = combat_strength(p.food_store_max, p.holding_weapon);
        if p.friendly_to_observer {
            ally_strength += strength;
        } else if !has_target || p.friendly_to_target {
            // Haxe: if (targetPlayer == null || p.isFriendly(targetPlayer)) enemyStrength += strength
            enemy_strength += strength;
        }
        // else: hostile to both attacker and target — ignored when target is set
    }

    let denom = enemy_strength + ally_strength;
    if denom <= f32::EPSILON {
        1.0
    } else {
        // Haxe: (allyStrength + allyStrength) / (enemyStrength + allyStrength)
        (ally_strength + ally_strength) / denom
    }
}

/// Haxe DoDamage `allyFactor` selection.
///
/// - Target is leadership-ally of attacker → always `0.5` (duel still halves).
/// - Else → `strength_factor` capped at [`ALLY_STRENGTH_FACTOR_CAP`] (no lower clamp).
// Haxe: DoDamage allyFactor = 0.5 | calculateEnemyVsAllyStrengthFactor + min 1.2
#[inline]
pub fn resolve_ally_damage_factor(target_is_ally_of_attacker: bool, strength_factor: f32) -> f32 {
    if target_is_ally_of_attacker {
        ALLY_ON_ALLY_DAMAGE_FACTOR
    } else {
        if strength_factor > ALLY_STRENGTH_FACTOR_CAP {
            ALLY_STRENGTH_FACTOR_CAP
        } else {
            strength_factor
        }
    }
}

/// Haxe `makeAllCloseAllyAngryAt`: p_ids of non-deleted players close to observer
/// with `p.isAlly(observer)` (leadership ally). Caller sets `lastPlayerAttackedMe`.
// Haxe: GlobalPlayerInstance.makeAllCloseAllyAngryAt
pub fn close_ally_ids_for_anger(
    observer_x: i32,
    observer_y: i32,
    players: &[AllyStrengthPlayer],
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> Vec<i32> {
    close_ally_ids_for_anger_ex(
        observer_x,
        observer_y,
        players,
        map_w,
        map_h,
        wrap,
        ALLY_CONSIDERED_CLOSE as f32,
    )
}

/// Live-radius variant of [`close_ally_ids_for_anger`].
// Haxe: ServerSettings.AllyConsideredClose
// C-SS-MORE-BATCH3
pub fn close_ally_ids_for_anger_ex(
    observer_x: i32,
    observer_y: i32,
    players: &[AllyStrengthPlayer],
    map_w: i32,
    map_h: i32,
    wrap: bool,
    ally_considered_close: f32,
) -> Vec<i32> {
    let mut out = Vec::new();
    for p in players {
        if p.deleted {
            continue;
        }
        if !is_close_for_ally_strength_ex(
            observer_x,
            observer_y,
            p.x,
            p.y,
            map_w,
            map_h,
            wrap,
            ally_considered_close,
        ) {
            continue;
        }
        if p.ally_to_observer {
            out.push(p.p_id);
        }
    }
    out
}

/// Haxe TransitionHelper pickup gate: refuse when factor &lt; threshold (threshold default 0 = off).
// Haxe: TransitionHelper doCommandHelper AllyStrenghTooLowForPickup
#[inline]
pub fn ally_strength_blocks_pickup(
    strength_factor: f32,
    threshold: f32,
    target_object_id: i32,
) -> bool {
    threshold > 0.0 && strength_factor < threshold && target_object_id != 0
}

/// Haxe `kill()` first-hit unarmed ally warn / second-hit exile before DoDamage.
// Haxe: GlobalPlayerInstance.kill ally unarmed gate L4454–4482
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnarmedAllyHitGate {
    /// Not ally, or ally holding a weapon (duel path) — proceed with damage.
    Proceed,
    /// First strike on unarmed ally — refuse damage; set `lastAttackedPlayer`.
    WarnAndRefuse {
        /// Haxe `targetPlayer.isFollowerFrom(this)`.
        is_follower: bool,
    },
    /// Second consecutive strike on the same unarmed ally — exile then damage.
    ExileThenProceed,
}

/// Decide kill/HIT gate for ally without weapon.
///
/// Haxe allows free duel when target holds a weapon; otherwise first hit only
/// warns and second hit on the same target exiles then continues into damage.
// Haxe: GlobalPlayerInstance.kill targetPlayer.isAlly && !isHoldingWeapon
#[inline]
pub fn resolve_unarmed_ally_hit_gate(
    target_is_ally_of_attacker: bool,
    target_holding_weapon: bool,
    attacker_last_attacked_player_id: i32,
    target_id: i32,
    target_is_follower_of_attacker: bool,
) -> UnarmedAllyHitGate {
    if !target_is_ally_of_attacker || target_holding_weapon {
        return UnarmedAllyHitGate::Proceed;
    }
    if attacker_last_attacked_player_id != target_id {
        UnarmedAllyHitGate::WarnAndRefuse {
            is_follower: target_is_follower_of_attacker,
        }
    } else {
        UnarmedAllyHitGate::ExileThenProceed
    }
}

/// Haxe first-hit ally warn strings (private PS say + global message).
///
/// Note: Haxe literally says `Its my allyr!` (typo kept for parity).
// Haxe: GlobalPlayerInstance.kill first-hit ally say / sendGlobalMessage
pub fn unarmed_ally_first_hit_messages(
    target_display_name: &str,
    is_follower: bool,
) -> (&'static str, String) {
    if is_follower {
        (
            "Its my follower!",
            format!("{target_display_name} is your follower! Attack again to exile!"),
        )
    } else {
        (
            "Its my allyr!",
            format!("{target_display_name} is your ally be careful!"),
        )
    }
}

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

    /// Cap damage to half **not-reduced** max food + 1 (Haxe DoDamage limit).
    ///
    /// Pass [`calculate_not_reduced_food_store_max`] (~20), **not** the live reduced
    /// `Player.food_max` (hits/exhaustion already lower that value).
    // Haxe: DoDamage maxFoodStore = calculateNotReducedFoodStoreMax(); damage > max/2+1
    pub fn cap_damage(damage: f32, not_reduced_food_store_max: f32) -> f32 {
        let base = if not_reduced_food_store_max.is_finite() && not_reduced_food_store_max > 0.0 {
            not_reduced_food_store_max
        } else {
            calculate_not_reduced_food_store_max()
        };
        let cap = base / 2.0 + 1.0;
        damage.min(cap.max(0.0))
    }

    /// Cap using Haxe not-reduced grown-up base.
    #[inline]
    pub fn cap_damage_default(damage: f32) -> f32 {
        Self::cap_damage(damage, calculate_not_reduced_food_store_max())
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
    /// cumulative hits ≥ [`HITS_KILL_THRESHOLD`].
    ///
    /// **Cap** always uses not-reduced food max (~20). Live reduced `target_food_max`
    /// is ignored for the cap (kept as a parameter for API stability / legacy callers).
    /// For hits+exhaustion recompute death, use [`Self::resolve_hit_full`].
    ///
    /// Callers should multiply `org_damage` by [`resolve_ally_damage_factor`] (and other
    /// DoDamage factors) before invoking.
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
        _target_food_max: f32,
        weapon_id: i32,
        rng01: f32,
    ) -> (HitResult, f32) {
        let (r, d, _) = self.resolve_hit_full(
            killer,
            target,
            killer_x,
            killer_y,
            target_x,
            target_y,
            legal,
            max_range,
            org_damage,
            clothing_insulation,
            floor_insulation,
            weapon_protection,
            weapon_id,
            rng01,
            30.0, // age unused when health_factor path not lethal via food
            10.0,
            0.0,
            1.0,
            true,
        );
        (r, d)
    }

    /// DoDamage-style hit with hits + exhaustion recompute for combat death.
    ///
    /// - Caps damage with [`calculate_not_reduced_food_store_max`]
    /// - Accumulates hits (if `real_damage`) and returns exhaustion/food_max after pipe
    /// - Kill when wound ≥ threshold, hits ≥ [`HITS_KILL_THRESHOLD`], or
    ///   recomputed `food_store_max < 0` (Haxe DoDamage)
    ///
    /// Caller must apply `exhaustion_after` / `food_store_max` onto the target
    /// [`crate::player::Player`] (and attacker combat exhaustion cost separately).
    // Haxe: GlobalPlayerInstance.DoDamage + calculateFoodStoreMax
    #[allow(clippy::too_many_arguments)]
    pub fn resolve_hit_full(
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
        weapon_id: i32,
        rng01: f32,
        target_age: f32,
        target_food: f32,
        target_exhaustion: f32,
        health_factor: f32,
        real_damage: bool,
    ) -> (HitResult, f32, DamagePipeSnapshot) {
        let empty = DamagePipeSnapshot::default();
        if killer == target {
            return (HitResult::Miss, 0.0, empty);
        }
        let dist = Self::chebyshev(killer_x, killer_y, target_x, target_y);
        if dist > max_range {
            return (HitResult::Miss, 0.0, empty);
        }
        let base = Self::roll_base_damage(org_damage, rng01);
        let raw = Self::apply_protection(
            base,
            clothing_insulation,
            floor_insulation,
            weapon_protection,
        );
        // Haxe: cap vs calculateNotReducedFoodStoreMax, not live reduced max
        let damage = Self::cap_damage_default(raw);
        let hits_before = self.hits_of(target);
        let pipe = apply_damage_food_pipe(
            target_age,
            target_food,
            hits_before,
            target_exhaustion,
            damage,
            health_factor,
            real_damage,
        );
        if real_damage {
            self.apply_hits(target, damage, weapon_id);
        } else if weapon_id != 0 {
            // Still record wounded_by for mosquito-style non-real damage
            let s = self.stats_mut(target);
            if s.wounded_by == 0 {
                s.wounded_by = weapon_id;
            }
        }
        let total_hits = self.hits_of(target);
        let wound = if real_damage {
            self.apply_wound(target, 1)
        } else {
            self.wound_of(target)
        };
        let snap = DamagePipeSnapshot {
            damage,
            hits_after: pipe.hits_after,
            exhaustion_after: pipe.exhaustion_after,
            food_store_max: pipe.food_store_max,
            combat_lethal: pipe.combat_lethal,
        };
        let lethal = wound >= WOUND_KILL_THRESHOLD
            || total_hits >= HITS_KILL_THRESHOLD
            || pipe.combat_lethal;
        if lethal {
            if self.resolve_kill(killer, target, legal) {
                self.clear_hits(target);
                (HitResult::Kill, damage, snap)
            } else {
                (HitResult::Wound(wound), damage, snap)
            }
        } else {
            (HitResult::Wound(wound), damage, snap)
        }
    }

    /// Attempt a kill. Returns true if target dies.
    /// Exiled targets can be killed legally (no **score** prestige penalty on killer).
    ///
    /// **Does not** mutate [`CombatStats::lost_combat_prestige`] — that float is only
    /// updated by REPUTATION-HIT [`crate::reputation::compute_hit_reputation`] /
    /// `apply_connecting_hit_reputation` after DoDamage (Haxe kill L4504–4561).
    // Haxe: resolve_kill scoreboard prestige only; lostCombatPrestige is separate
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
            } else {
                // Illegal kill — score prestige hit (not lostCombatPrestige).
                k.prestige = (k.prestige - 2.0).max(0.0);
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

    fn player(
        p_id: i32,
        x: i32,
        y: i32,
        food: f32,
        weapon: bool,
        friendly_obs: bool,
        friendly_tgt: bool,
        ally_obs: bool,
    ) -> AllyStrengthPlayer {
        AllyStrengthPlayer {
            p_id,
            x,
            y,
            deleted: false,
            food_store_max: food,
            holding_weapon: weapon,
            friendly_to_observer: friendly_obs,
            friendly_to_target: friendly_tgt,
            ally_to_observer: ally_obs,
        }
    }

    #[test]
    fn combat_strength_weapon_doubles() {
        assert!((combat_strength(10.0, false) - 10.0).abs() < 1e-6);
        assert!((combat_strength(10.0, true) - 20.0).abs() < 1e-6);
        assert!((combat_strength(-5.0, true) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn ally_strength_base_only_is_one() {
        // No nearby players → 2*10 / (10+10) = 1.0
        let f = calculate_enemy_vs_ally_strength_factor(0, 0, &[], true, 100, 100, false);
        assert!((f - 1.0).abs() < 1e-5);
    }

    #[test]
    fn ally_strength_friendly_nearby_boosts_factor() {
        // Observer at 0,0; friend at 1,0 with food 20 unarmed → ally 10+20=30, enemy 10
        // factor = 60/40 = 1.5 → will be capped to 1.2 at DoDamage layer
        let players = [player(2, 1, 0, 20.0, false, true, false, true)];
        let f = calculate_enemy_vs_ally_strength_factor(0, 0, &players, true, 100, 100, false);
        assert!((f - 1.5).abs() < 1e-5);
        let capped = resolve_ally_damage_factor(false, f);
        assert!((capped - ALLY_STRENGTH_FACTOR_CAP).abs() < 1e-5);
    }

    #[test]
    fn ally_strength_enemy_nearby_reduces_factor() {
        // Hostile-to-observer but friendly-to-target → enemy strength
        let players = [player(3, 1, 0, 30.0, false, false, true, false)];
        let f = calculate_enemy_vs_ally_strength_factor(0, 0, &players, true, 100, 100, false);
        // ally 10, enemy 10+30=40 → 20/50 = 0.4
        assert!((f - 0.4).abs() < 1e-5);
        assert!((resolve_ally_damage_factor(false, f) - 0.4).abs() < 1e-5);
    }

    #[test]
    fn ally_strength_hostile_to_both_ignored_when_target_set() {
        // Not friendly to observer, not friendly to target → skip when has_target
        let players = [player(3, 1, 0, 50.0, true, false, false, false)];
        let with_tgt =
            calculate_enemy_vs_ally_strength_factor(0, 0, &players, true, 100, 100, false);
        assert!((with_tgt - 1.0).abs() < 1e-5);
        // Without target: counts as enemy (Haxe targetPlayer == null)
        let no_tgt =
            calculate_enemy_vs_ally_strength_factor(0, 0, &players, false, 100, 100, false);
        // ally 10, enemy 10+100 = 110 → 20/120 ≈ 0.1667
        assert!((no_tgt - (20.0 / 120.0)).abs() < 1e-5);
    }

    #[test]
    fn ally_strength_far_players_ignored() {
        // AllyConsideredClose = 5 → dist² > 25 ignored (e.g. dx=6)
        let players = [player(2, 6, 0, 100.0, true, true, false, true)];
        let f = calculate_enemy_vs_ally_strength_factor(0, 0, &players, true, 100, 100, false);
        assert!((f - 1.0).abs() < 1e-5);
    }

    /// C-SS-MORE-BATCH3: live AllyConsideredClose radius 1 excludes peer at dist 3.
    // Haxe: ServerSettings.AllyConsideredClose
    #[test]
    fn ally_strength_live_radius_excludes_mid_range() {
        let players = [player(2, 3, 0, 100.0, true, true, false, true)];
        // Default radius 5 includes dx=3
        let f_def =
            calculate_enemy_vs_ally_strength_factor(0, 0, &players, true, 100, 100, false);
        assert!(f_def > 1.0);
        // Radius 1 excludes dx=3
        let f_tight = calculate_enemy_vs_ally_strength_factor_ex(
            0, 0, &players, true, 100, 100, false, 1.0,
        );
        assert!((f_tight - 1.0).abs() < 1e-5);
        assert!(!is_close_for_ally_strength_ex(0, 0, 3, 0, 100, 100, false, 1.0));
        assert!(is_close_for_ally_strength_ex(0, 0, 3, 0, 100, 100, false, 5.0));
    }

    #[test]
    fn ally_on_ally_half_damage() {
        assert!((resolve_ally_damage_factor(true, 1.5) - 0.5).abs() < 1e-6);
        assert!((resolve_ally_damage_factor(true, 0.1) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn make_angry_close_allies_only() {
        let players = [
            player(1, 0, 0, 20.0, false, true, true, true),  // self ally
            player(2, 1, 0, 20.0, false, true, true, true),  // close ally
            player(3, 2, 0, 20.0, false, false, false, false), // close non-ally
            player(4, 10, 0, 20.0, false, true, true, true), // far ally
        ];
        let ids = close_ally_ids_for_anger(0, 0, &players, 100, 100, false);
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
        assert!(!ids.contains(&3));
        assert!(!ids.contains(&4));
    }

    #[test]
    fn pickup_gate_default_off() {
        assert!(!ally_strength_blocks_pickup(
            0.5,
            ALLY_STRENGTH_TOO_LOW_FOR_PICKUP_DEFAULT,
            99
        ));
        assert!(ally_strength_blocks_pickup(0.5, 0.8, 99));
        assert!(!ally_strength_blocks_pickup(0.5, 0.8, 0)); // empty target allowed
        assert!(!ally_strength_blocks_pickup(0.9, 0.8, 99));
    }

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

    /// REPUTATION-HIT: resolve_kill must not touch lost_combat_prestige (avoids double-count
    /// with apply_connecting_hit_reputation on HIT Kill / SAY KILL).
    // Haxe: lostCombatPrestige only after DoDamage in kill(), not kill counters
    #[test]
    fn resolve_kill_leaves_lost_combat_prestige_unchanged() {
        let mut c = CombatState::default();
        c.stats_mut(1).lost_combat_prestige = 3.5;
        c.stats_mut(2).lost_combat_prestige = 1.0;
        assert!(c.resolve_kill(1, 2, false));
        assert!((c.stats.get(&1).unwrap().lost_combat_prestige - 3.5).abs() < 1e-6);
        assert!((c.stats.get(&2).unwrap().lost_combat_prestige - 1.0).abs() < 1e-6);
        // Exile-legal path also leaves float alone.
        let mut c2 = CombatState::default();
        c2.stats_mut(3).lost_combat_prestige = 2.0;
        assert!(c2.resolve_kill(3, 4, true));
        assert!((c2.stats.get(&3).unwrap().lost_combat_prestige - 2.0).abs() < 1e-6);
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
        // Pre-load hits so one capped strike (11) crosses HITS_KILL_THRESHOLD (12).
        c.apply_hits(2, 2.0, 0);
        let (r, d) = c.resolve_hit_damaged(
            1, 2, 0, 0, 0, 0, true, 2, 30.0, 0.0, 0.0, 1.0, 20.0, 5, 1.0,
        );
        assert_eq!(r, HitResult::Kill);
        assert!(d > 0.0);
        assert_eq!(c.stats.get(&1).unwrap().kills, 1);
    }

    #[test]
    fn resolve_hit_full_exhaustion_and_recompute() {
        let mut c = CombatState::default();
        let (r, d, snap) = c.resolve_hit_full(
            1, 2, 0, 0, 0, 0, true, 2, 4.0, 0.0, 0.0, 1.0, 99, 1.0, 30.0, 10.0, 0.0, 1.0, true,
        );
        assert!(matches!(r, HitResult::Wound(_)));
        // rng01=1 → roll = org/2 + org*1 = 2+4 = 6, cap 11 → 6
        assert!((d - 6.0).abs() < 1e-4);
        assert!((snap.hits_after - 6.0).abs() < 1e-4);
        assert!((snap.exhaustion_after - 6.0).abs() < 1e-4);
        // food_max = 20 - 6 hits - 6 exh = 8
        assert!((snap.food_store_max - 8.0).abs() < 1e-4);
        assert!(!snap.combat_lethal);
    }

    #[test]
    fn cap_damage_uses_not_reduced_base() {
        // Live reduced max must not shrink the cap below grown-up half+1
        let cap_live = CombatState::cap_damage(100.0, 4.0); // wrong call pattern
        let cap_ok = CombatState::cap_damage_default(100.0);
        assert!((cap_ok - 11.0).abs() < 1e-5);
        // Document: callers should use not-reduced; wrong live value yields small cap
        assert!((cap_live - 3.0).abs() < 1e-5);
    }

    #[test]
    fn resolve_hit_full_combat_lethal_on_stacked_hits() {
        let mut c = CombatState::default();
        c.apply_hits(2, 15.0, 0);
        let (r, _d, snap) = c.resolve_hit_full(
            1, 2, 0, 0, 0, 0, true, 2, 20.0, 0.0, 0.0, 1.0, 5, 1.0, 30.0, 10.0, 0.0, 1.0, true,
        );
        assert!(snap.combat_lethal || matches!(r, HitResult::Kill));
        assert_eq!(r, HitResult::Kill);
    }

    #[test]
    fn ally_factor_halves_org_damage_path() {
        // Same roll: org 4 * 0.5 vs org 4 * 1.0 → half applied before protection.
        let mut c_half = CombatState::default();
        let org = 4.0 * resolve_ally_damage_factor(true, 1.0);
        let (_, d_half) = c_half.resolve_hit_damaged(
            1, 2, 0, 0, 0, 0, true, 2, org, 0.0, 0.0, 1.0, 40.0, 1, 1.0,
        );
        let mut c_full = CombatState::default();
        let (_, d_full) = c_full.resolve_hit_damaged(
            1, 2, 0, 0, 0, 0, true, 2, 4.0, 0.0, 0.0, 1.0, 40.0, 1, 1.0,
        );
        assert!((d_half * 2.0 - d_full).abs() < 1e-4);
    }

    /// C-SS-MORE-BATCH4: cursed receive/make damage muls.
    // Haxe: GlobalPlayerInstance.DoDamage L4628-4629
    #[test]
    fn cursed_damage_mul_defaults_and_live() {
        assert!((cursed_receive_damage_mul(false, 1.2) - 1.0).abs() < 1e-6);
        assert!((cursed_receive_damage_mul(true, 1.2) - 1.2).abs() < 1e-6);
        assert!((cursed_receive_damage_mul(true, 2.0) - 2.0).abs() < 1e-6);
        assert!((cursed_receive_damage_mul(true, f32::NAN) - 1.2).abs() < 1e-6);
        assert!((cursed_make_damage_mul(false, 0.5) - 1.0).abs() < 1e-6);
        assert!((cursed_make_damage_mul(true, 0.5) - 0.5).abs() < 1e-6);
        assert!((cursed_make_damage_mul(true, 0.25) - 0.25).abs() < 1e-6);
        assert!((cursed_make_damage_mul(true, 0.0) - 0.5).abs() < 1e-6);
        // Combined with ally half + defaults: 4 * 0.5 * 1.2 * 0.5 = 1.2
        let org = 4.0
            * resolve_ally_damage_factor(true, 1.0)
            * cursed_receive_damage_mul(true, CURSED_RECEIVE_DAMAGE_FACTOR)
            * cursed_make_damage_mul(true, CURSED_MAKE_DAMAGE_FACTOR);
        assert!((org - 1.2).abs() < 1e-4);
    }

    #[test]
    fn unarmed_ally_first_hit_warns_second_exiles() {
        // Armed ally or non-ally → proceed.
        assert_eq!(
            resolve_unarmed_ally_hit_gate(true, true, 0, 2, false),
            UnarmedAllyHitGate::Proceed
        );
        assert_eq!(
            resolve_unarmed_ally_hit_gate(false, false, 0, 2, false),
            UnarmedAllyHitGate::Proceed
        );
        // First hit unarmed ally.
        assert_eq!(
            resolve_unarmed_ally_hit_gate(true, false, 0, 2, true),
            UnarmedAllyHitGate::WarnAndRefuse { is_follower: true }
        );
        assert_eq!(
            resolve_unarmed_ally_hit_gate(true, false, 9, 2, false),
            UnarmedAllyHitGate::WarnAndRefuse { is_follower: false }
        );
        // Second hit same target → exile then proceed.
        assert_eq!(
            resolve_unarmed_ally_hit_gate(true, false, 2, 2, true),
            UnarmedAllyHitGate::ExileThenProceed
        );
        let (say, gm) = unarmed_ally_first_hit_messages("Bob_Smith", false);
        assert_eq!(say, "Its my allyr!");
        assert!(gm.contains("ally be careful"));
        let (say_f, gm_f) = unarmed_ally_first_hit_messages("Bob_Smith", true);
        assert_eq!(say_f, "Its my follower!");
        assert!(gm_f.contains("Attack again to exile"));
    }
}
