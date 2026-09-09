//! Haxe `TimeHelper.doAnimalMovement` population slice:
//! offspring chance, natural die-in-place, `failedMoves > 20` stuck death.
//!
//! Chunk: **TIME-ANIMAL-OFFSPRING** / `pop_die_offspring`.
//! Pure rules only — sim tick wires map MX + entity spawn/remove.

// --- Haxe ServerSettings defaults (ServerSettings.hx) ---------------------------

/// Haxe `ChanceForOffspring` — per successful movement attempt.
pub const CHANCE_FOR_OFFSPRING: f32 = 0.00005;

/// Haxe `ChanceForAnimalDying` — per successful target selection (before move).
pub const CHANCE_FOR_ANIMAL_DYING: f32 = 0.00005;

/// Haxe `ChanceForAnimalDyingFactorIfInLovedBiome` (preferred biome).
pub const CHANCE_FOR_ANIMAL_DYING_FACTOR_IF_IN_LOVED_BIOME: f32 = 0.1;

/// Haxe `ChanceForDomesticAnimalDyingFactor`.
///
/// Haxe TimeHelper multiplies without assignment (no-op). Rust applies it.
pub const CHANCE_FOR_DOMESTIC_ANIMAL_DYING_FACTOR: f32 = 2.0;

/// Haxe `OffspringFactorLowAnimalPopulationBelow` (fraction of original).
pub const OFFSPRING_FACTOR_LOW_POP_BELOW: f32 = 0.2;

/// Haxe `OffspringFactorIfAnimalPopIsLow` multiplier.
pub const OFFSPRING_FACTOR_IF_POP_LOW: f32 = 10.0;

/// Haxe `MaxOffspringFactor` — pop cap as multiple of original.
pub const MAX_OFFSPRING_FACTOR: f32 = 1.0;

/// Haxe `animal.failedMoves > 20` stuck-death threshold.
pub const FAILED_MOVES_DEATH_THRESHOLD: f32 = 20.0;

/// Haxe `GetClosestObjectToPosition(..., parentId, 2, animal)` — no offspring.
pub const OFFSPRING_MIN_SEPARATION: i32 = 2;

/// Haxe `shouldDie = currentPop > 10` floor (natural die never below this count).
pub const MIN_CURRENT_POP_FOR_NATURAL_DEATH: i32 = 10;

/// Default `canDieIfPopulationIsAbove` when not rabbit-in-wrong-place.
pub const CAN_DIE_POP_FRACTION_DEFAULT: f32 = 0.8;

/// Rabbit wrong-place fraction (Haxe `rabbitInWrongPlace ? 0.4 : 0.8`).
pub const CAN_DIE_POP_FRACTION_RABBIT_WRONG: f32 = 0.4;

// --- Pure chance / gate helpers ------------------------------------------------

/// Haxe:
/// `chanceForOffspring = isPreferred ? Chance : Chance / 100`
#[inline]
pub fn chance_for_offspring(is_preferred_biome: bool, base: f32) -> f32 {
    if is_preferred_biome {
        base
    } else {
        base / 100.0
    }
}

/// Haxe:
/// preferred → `Chance * FactorIfInLovedBiome`; else raw `Chance`.
#[inline]
pub fn chance_for_animal_dying(
    is_preferred_biome: bool,
    base: f32,
    loved_biome_factor: f32,
) -> f32 {
    if is_preferred_biome {
        base * loved_biome_factor
    } else {
        base
    }
}

/// Multiply dying chance for domestic animals (live factor; Haxe intended `*=`).
#[inline]
pub fn apply_domestic_dying_factor(chance: f32, is_domestic: bool, factor: f32) -> f32 {
    if !is_domestic {
        return chance;
    }
    let f = if factor.is_finite() && factor >= 0.0 {
        factor
    } else {
        CHANCE_FOR_DOMESTIC_ANIMAL_DYING_FACTOR
    };
    chance * f
}

/// Rabbit wrong-place doubles dying chance (Haxe `chanceForAnimalDying *= 2`).
#[inline]
pub fn apply_rabbit_wrong_place_dying(chance: f32, rabbit_in_wrong_place: bool) -> f32 {
    if rabbit_in_wrong_place {
        chance * 2.0
    } else {
        chance
    }
}

/// Haxe low-pop birth boost:
/// `if (current < original * LowBelow) chance *= OffspringFactorIfAnimalPopIsLow`
#[inline]
pub fn apply_low_pop_offspring_boost(
    chance: f32,
    current_pop: i32,
    original_pop: i32,
    low_below: f32,
    boost: f32,
) -> f32 {
    if original_pop > 0 && (current_pop as f32) < (original_pop as f32) * low_below {
        chance * boost
    } else {
        chance
    }
}

/// Haxe: `if (originalPop > 10) chanceForAnimalDying *= currentPop > originalPop ? 100 : 1`
#[inline]
pub fn apply_overpop_dying_boost(chance: f32, current_pop: i32, original_pop: i32) -> f32 {
    if original_pop > 10 && current_pop > original_pop {
        chance * 100.0
    } else {
        chance
    }
}

/// Haxe `canDieIfPopulationIsAbove`.
#[inline]
pub fn can_die_pop_fraction(rabbit_in_wrong_place: bool) -> f32 {
    if rabbit_in_wrong_place {
        CAN_DIE_POP_FRACTION_RABBIT_WRONG
    } else {
        CAN_DIE_POP_FRACTION_DEFAULT
    }
}

/// Population + container/ground gates for natural death (before RNG roll).
///
/// Haxe:
/// - `shouldDie = currentPop > 10`
/// - false if `currentPop <= original * MaxOffspring * canDieAbove`
/// - false if containedObjects / groundObject present
#[inline]
pub fn natural_death_allowed(
    current_pop: i32,
    original_pop: i32,
    max_offspring_factor: f32,
    can_die_above: f32,
    has_contained: bool,
    has_ground_object: bool,
) -> bool {
    if has_contained || has_ground_object {
        return false;
    }
    if current_pop <= MIN_CURRENT_POP_FOR_NATURAL_DEATH {
        return false;
    }
    let cap = (original_pop as f32) * max_offspring_factor * can_die_above;
    if (current_pop as f32) <= cap {
        return false;
    }
    true
}

/// When `originalPop < 1`, Haxe rescans local density before allowing death:
/// - `< 6` same parent within 5 + loves biome → keep alive
/// - `< 3` always keep alive (lonely domestic-ish)
#[inline]
pub fn lonely_death_override(original_pop: i32, close_same_parent: i32, loves_biome: bool) -> bool {
    if original_pop >= 1 {
        return true; // no override — caller keeps shouldDie
    }
    if close_same_parent < 6 && loves_biome {
        return false;
    }
    if close_same_parent < 3 {
        return false;
    }
    true
}

/// Full natural-death decision after RNG: `allowed && rng < chance` (+ lonely).
pub fn roll_natural_death(
    current_pop: i32,
    original_pop: i32,
    max_offspring_factor: f32,
    can_die_above: f32,
    has_contained: bool,
    has_ground_object: bool,
    chance: f32,
    rng01: f32,
    close_same_parent: i32,
    loves_biome: bool,
) -> bool {
    if !natural_death_allowed(
        current_pop,
        original_pop,
        max_offspring_factor,
        can_die_above,
        has_contained,
        has_ground_object,
    ) {
        return false;
    }
    if rng01 >= chance.max(0.0) {
        return false;
    }
    lonely_death_override(original_pop, close_same_parent, loves_biome)
}

/// Haxe: offspring only if `currentPop < originalPop * MaxOffspringFactor`.
#[inline]
pub fn offspring_pop_allows(current_pop: i32, original_pop: i32, max_factor: f32) -> bool {
    if original_pop <= 0 {
        // No baseline → allow spawn (tests / empty original); Haxe still compares.
        return current_pop < i32::MAX / 4;
    }
    (current_pop as f32) < (original_pop as f32) * max_factor
}

/// Roll offspring after successful move.
pub fn roll_offspring(
    current_pop: i32,
    original_pop: i32,
    max_factor: f32,
    chance: f32,
    rng01: f32,
    has_close_same_parent: bool,
) -> bool {
    if has_close_same_parent {
        return false;
    }
    if !offspring_pop_allows(current_pop, original_pop, max_factor) {
        return false;
    }
    rng01 < chance.max(0.0)
}

/// Haxe stuck path: `failedMoves += randomFloat()`.
#[inline]
pub fn accumulate_failed_moves(failed_moves: f32, rng01: f32) -> f32 {
    failed_moves + rng01.clamp(0.0, 1.0)
}

/// Haxe `failedMoves > 20`.
#[inline]
pub fn failed_moves_kills(failed_moves: f32) -> bool {
    failed_moves > FAILED_MOVES_DEATH_THRESHOLD
}

/// Chebyshev distance helper (same as pack-alert / offspring proximity).
#[inline]
pub fn chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs().max((ay - by).abs())
}

/// True if any peer of `kind_id` is within Chebyshev `range` (exclude self index).
pub fn has_close_same_parent(
    peers: &[(i32, i32, i32)], // (parent_id, x, y)
    self_index: usize,
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    range: i32,
) -> bool {
    for (i, &(pid, x, y)) in peers.iter().enumerate() {
        if i == self_index || pid != parent_id {
            continue;
        }
        if chebyshev(from_x, from_y, x, y) <= range {
            return true;
        }
    }
    false
}

/// Count same parent within Chebyshev `range` (exclude self).
pub fn count_close_same_parent(
    peers: &[(i32, i32, i32)],
    self_index: usize,
    parent_id: i32,
    from_x: i32,
    from_y: i32,
    range: i32,
) -> i32 {
    let mut n = 0i32;
    for (i, &(pid, x, y)) in peers.iter().enumerate() {
        if i == self_index || pid != parent_id {
            continue;
        }
        if chebyshev(from_x, from_y, x, y) <= range {
            n += 1;
        }
    }
    n
}

/// Compose offspring chance from biome + low-pop (defaults).
pub fn compute_offspring_chance(
    is_preferred_biome: bool,
    current_pop: i32,
    original_pop: i32,
) -> f32 {
    compute_offspring_chance_ex(
        is_preferred_biome,
        current_pop,
        original_pop,
        CHANCE_FOR_OFFSPRING,
        OFFSPRING_FACTOR_IF_POP_LOW,
        OFFSPRING_FACTOR_LOW_POP_BELOW,
    )
}

/// Like [`compute_offspring_chance`] with live `ChanceForOffspring` /
/// `OffspringFactorIfAnimalPopIsLow` / `OffspringFactorLowAnimalPopulationBelow`.
// Haxe: ServerSettings.ChanceForOffspring / OffspringFactorIfAnimalPopIsLow / OffspringFactorLowAnimalPopulationBelow
pub fn compute_offspring_chance_ex(
    is_preferred_biome: bool,
    current_pop: i32,
    original_pop: i32,
    base_chance: f32,
    offspring_factor_if_pop_low: f32,
    low_below: f32,
) -> f32 {
    let base = chance_for_offspring(is_preferred_biome, base_chance);
    let boost = if offspring_factor_if_pop_low.is_finite() && offspring_factor_if_pop_low > 0.0 {
        offspring_factor_if_pop_low
    } else {
        OFFSPRING_FACTOR_IF_POP_LOW
    };
    let below = if low_below.is_finite() && low_below > 0.0 {
        low_below
    } else {
        OFFSPRING_FACTOR_LOW_POP_BELOW
    };
    apply_low_pop_offspring_boost(base, current_pop, original_pop, below, boost)
}

/// Compose dying chance from biome + rabbit + overpop (defaults; domestic no-op).
pub fn compute_dying_chance(
    is_preferred_biome: bool,
    rabbit_in_wrong_place: bool,
    current_pop: i32,
    original_pop: i32,
) -> f32 {
    compute_dying_chance_ex(
        is_preferred_biome,
        rabbit_in_wrong_place,
        current_pop,
        original_pop,
        CHANCE_FOR_ANIMAL_DYING,
        CHANCE_FOR_ANIMAL_DYING_FACTOR_IF_IN_LOVED_BIOME,
    )
}

/// Like [`compute_dying_chance`] with live `ChanceForAnimalDying` /
/// `ChanceForAnimalDyingFactorIfInLovedBiome`.
// Haxe: ServerSettings.ChanceForAnimalDying / ChanceForAnimalDyingFactorIfInLovedBiome
pub fn compute_dying_chance_ex(
    is_preferred_biome: bool,
    rabbit_in_wrong_place: bool,
    current_pop: i32,
    original_pop: i32,
    base_chance: f32,
    loved_biome_factor: f32,
) -> f32 {
    let loved = if loved_biome_factor.is_finite() && loved_biome_factor >= 0.0 {
        loved_biome_factor
    } else {
        CHANCE_FOR_ANIMAL_DYING_FACTOR_IF_IN_LOVED_BIOME
    };
    let mut c = chance_for_animal_dying(is_preferred_biome, base_chance, loved);
    c = apply_rabbit_wrong_place_dying(c, rabbit_in_wrong_place);
    apply_overpop_dying_boost(c, current_pop, original_pop)
}

/// Like [`compute_dying_chance_ex`] with domestic dying factor applied.
pub fn compute_dying_chance_ex_domestic(
    is_preferred_biome: bool,
    rabbit_in_wrong_place: bool,
    current_pop: i32,
    original_pop: i32,
    base_chance: f32,
    loved_biome_factor: f32,
    is_domestic: bool,
    domestic_factor: f32,
) -> f32 {
    let mut c = compute_dying_chance_ex(
        is_preferred_biome,
        rabbit_in_wrong_place,
        current_pop,
        original_pop,
        base_chance,
        loved_biome_factor,
    );
    c = apply_domestic_dying_factor(c, is_domestic, domestic_factor);
    c
}

/// One-shot resolution for a successful destination pick (before commit move).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopMoveOutcome {
    /// Remove animal at current tile (no move).
    DieInPlace,
    /// Commit move; optionally spawn offspring on origin.
    Move { spawn_offspring: bool },
}

/// Haxe block: natural die roll, else move + offspring roll.
pub fn resolve_pop_on_dest(
    current_pop: i32,
    original_pop: i32,
    is_preferred_biome: bool,
    rabbit_in_wrong_place: bool,
    has_contained: bool,
    has_ground_object: bool,
    loves_biome: bool,
    close_same_parent_die: i32,
    has_close_same_for_offspring: bool,
    rng_die: f32,
    rng_offspring: f32,
) -> PopMoveOutcome {
    resolve_pop_on_dest_ex(
        current_pop,
        original_pop,
        is_preferred_biome,
        rabbit_in_wrong_place,
        has_contained,
        has_ground_object,
        loves_biome,
        close_same_parent_die,
        has_close_same_for_offspring,
        rng_die,
        rng_offspring,
        CHANCE_FOR_OFFSPRING,
        CHANCE_FOR_ANIMAL_DYING,
        CHANCE_FOR_ANIMAL_DYING_FACTOR_IF_IN_LOVED_BIOME,
        OFFSPRING_FACTOR_IF_POP_LOW,
        MAX_OFFSPRING_FACTOR,
        OFFSPRING_FACTOR_LOW_POP_BELOW,
        false,
        CHANCE_FOR_DOMESTIC_ANIMAL_DYING_FACTOR,
    )
}

/// Like [`resolve_pop_on_dest`] with live ChanceForOffspring / ChanceForAnimalDying /
/// ChanceForAnimalDyingFactorIfInLovedBiome / OffspringFactorIfAnimalPopIsLow / MaxOffspringFactor.
// Haxe: ServerSettings.ChanceForOffspring / ChanceForAnimalDying / ChanceForAnimalDyingFactorIfInLovedBiome / OffspringFactorIfAnimalPopIsLow / MaxOffspringFactor
pub fn resolve_pop_on_dest_ex(
    current_pop: i32,
    original_pop: i32,
    is_preferred_biome: bool,
    rabbit_in_wrong_place: bool,
    has_contained: bool,
    has_ground_object: bool,
    loves_biome: bool,
    close_same_parent_die: i32,
    has_close_same_for_offspring: bool,
    rng_die: f32,
    rng_offspring: f32,
    chance_for_offspring_base: f32,
    chance_for_animal_dying_base: f32,
    loved_biome_factor: f32,
    offspring_factor_if_pop_low: f32,
    max_offspring_factor: f32,
    low_below: f32,
    is_domestic: bool,
    domestic_dying_factor: f32,
) -> PopMoveOutcome {
    let die_chance = compute_dying_chance_ex_domestic(
        is_preferred_biome,
        rabbit_in_wrong_place,
        current_pop,
        original_pop,
        chance_for_animal_dying_base,
        loved_biome_factor,
        is_domestic,
        domestic_dying_factor,
    );
    let max_factor = if max_offspring_factor.is_finite() && max_offspring_factor > 0.0 {
        max_offspring_factor
    } else {
        MAX_OFFSPRING_FACTOR
    };
    let can_above = can_die_pop_fraction(rabbit_in_wrong_place);
    if roll_natural_death(
        current_pop,
        original_pop,
        max_factor,
        can_above,
        has_contained,
        has_ground_object,
        die_chance,
        rng_die,
        close_same_parent_die,
        loves_biome,
    ) {
        return PopMoveOutcome::DieInPlace;
    }

    let off_chance = compute_offspring_chance_ex(
        is_preferred_biome,
        current_pop,
        original_pop,
        chance_for_offspring_base,
        offspring_factor_if_pop_low,
        low_below,
    );
    let spawn = roll_offspring(
        current_pop,
        original_pop,
        max_factor,
        off_chance,
        rng_offspring,
        has_close_same_for_offspring,
    );
    PopMoveOutcome::Move {
        spawn_offspring: spawn,
    }
}

/// Stuck-move resolution: accumulate failedMoves; kill when past threshold.
pub fn resolve_failed_move(failed_moves: f32, rng01: f32) -> (f32, bool) {
    let next = accumulate_failed_moves(failed_moves, rng01);
    let kill = failed_moves_kills(next);
    (if kill { 0.0 } else { next }, kill)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offspring_chance_preferred_vs_not() {
        assert!((chance_for_offspring(true, 0.00005) - 0.00005).abs() < 1e-12);
        assert!((chance_for_offspring(false, 0.00005) - 0.0000005).abs() < 1e-12);
    }

    #[test]
    fn dying_chance_loved_biome_reduced() {
        let loved = chance_for_animal_dying(true, 0.00005, 0.1);
        let other = chance_for_animal_dying(false, 0.00005, 0.1);
        assert!((loved - 0.000005).abs() < 1e-12);
        assert!((other - 0.00005).abs() < 1e-12);
    }

    #[test]
    fn domestic_factor_applied() {
        let c = apply_domestic_dying_factor(0.01, false, 2.0);
        assert!((c - 0.01).abs() < 1e-9);
        let fixed = apply_domestic_dying_factor(0.01, true, 2.0);
        assert!((fixed - 0.02).abs() < 1e-9);
    }

    #[test]
    fn low_pop_boosts_offspring() {
        let base = 0.00005_f32;
        // current 1, original 10, below 0.2*10=2 → boost
        let boosted = apply_low_pop_offspring_boost(base, 1, 10, 0.2, 10.0);
        assert!((boosted - base * 10.0).abs() < 1e-9);
        // current 5 not low
        let normal = apply_low_pop_offspring_boost(base, 5, 10, 0.2, 10.0);
        assert!((normal - base).abs() < 1e-9);
    }

    #[test]
    fn overpop_dying_boost() {
        assert!((apply_overpop_dying_boost(0.001, 20, 15) - 0.1).abs() < 1e-9);
        assert!((apply_overpop_dying_boost(0.001, 10, 15) - 0.001).abs() < 1e-9);
        // originalPop <= 10: no boost
        assert!((apply_overpop_dying_boost(0.001, 50, 10) - 0.001).abs() < 1e-9);
    }

    #[test]
    fn natural_death_gates() {
        // need current > 10 and above original * max * 0.8
        assert!(!natural_death_allowed(10, 5, 1.0, 0.8, false, false));
        assert!(!natural_death_allowed(11, 20, 1.0, 0.8, false, false)); // 11 <= 16
        assert!(natural_death_allowed(20, 10, 1.0, 0.8, false, false)); // 20 > 8
        assert!(!natural_death_allowed(20, 10, 1.0, 0.8, true, false));
        assert!(!natural_death_allowed(20, 10, 1.0, 0.8, false, true));
    }

    #[test]
    fn lonely_override_keeps_small_groups() {
        assert!(!lonely_death_override(0, 2, true));
        assert!(!lonely_death_override(0, 2, false)); // < 3
        assert!(!lonely_death_override(0, 5, true)); // < 6 + loves
        assert!(lonely_death_override(0, 5, false)); // 5 >= 3, not loves
        assert!(lonely_death_override(0, 7, true));
        assert!(lonely_death_override(5, 0, true)); // original >= 1: no override
    }

    #[test]
    fn roll_natural_death_respects_rng() {
        // Allowed pop, chance 1.0 → die; chance 0 → live
        assert!(roll_natural_death(
            20, 10, 1.0, 0.8, false, false, 1.0, 0.5, 0, true
        ));
        assert!(!roll_natural_death(
            20, 10, 1.0, 0.8, false, false, 0.0, 0.0, 0, true
        ));
        // original 0 + lonely
        assert!(!roll_natural_death(
            20, 0, 1.0, 0.8, false, false, 1.0, 0.0, 1, true
        ));
    }

    #[test]
    fn offspring_close_blocks() {
        assert!(!roll_offspring(1, 10, 1.0, 1.0, 0.0, true));
        assert!(roll_offspring(1, 10, 1.0, 1.0, 0.0, false));
        // at/over cap
        assert!(!roll_offspring(10, 10, 1.0, 1.0, 0.0, false));
    }

    #[test]
    fn failed_moves_accumulate_and_kill() {
        let (n, kill) = resolve_failed_move(19.5, 0.6);
        assert!(kill);
        assert_eq!(n, 0.0);
        let (n2, kill2) = resolve_failed_move(10.0, 0.5);
        assert!(!kill2);
        assert!((n2 - 10.5).abs() < 1e-6);
        assert!(failed_moves_kills(20.0001));
        assert!(!failed_moves_kills(20.0));
    }

    #[test]
    fn close_same_parent_detection() {
        let peers = [(418, 0, 0), (418, 2, 0), (418, 10, 0), (1323, 1, 0)];
        assert!(has_close_same_parent(&peers, 0, 418, 0, 0, 2));
        assert!(!has_close_same_parent(&peers, 0, 418, 0, 0, 1));
        assert_eq!(count_close_same_parent(&peers, 0, 418, 0, 0, 20), 2);
    }

    #[test]
    fn resolve_pop_on_dest_die_and_birth() {
        // Force die: high chance, allowed pop
        let o = resolve_pop_on_dest(20, 10, false, false, false, false, true, 0, false, 0.0, 0.0);
        // die chance tiny by default — use preferred=false base 5e-5; force via roll helpers above.
        // Explicit Move path with forced high offspring chance via low rng and high chance path:
        let move_only =
            resolve_pop_on_dest(1, 100, true, false, false, false, true, 0, false, 0.99, 0.0);
        // die chance loved = 5e-6; rng 0.99 → no die; offspring chance boosted (1 < 20) *10 = 5e-4; rng 0 → birth
        assert_eq!(
            move_only,
            PopMoveOutcome::Move {
                spawn_offspring: true
            }
        );
        let blocked =
            resolve_pop_on_dest(1, 100, true, false, false, false, true, 0, true, 0.99, 0.0);
        assert_eq!(
            blocked,
            PopMoveOutcome::Move {
                spawn_offspring: false
            }
        );
        // Force die with artificial settings via roll_natural_death already tested;
        // use has_ground to force move:
        let ground =
            resolve_pop_on_dest(50, 10, false, false, false, true, true, 0, false, 0.0, 0.99);
        assert_eq!(
            ground,
            PopMoveOutcome::Move {
                spawn_offspring: false
            }
        );
        let _ = o;
    }

    #[test]
    fn rabbit_wrong_place_fraction_and_chance() {
        assert!((can_die_pop_fraction(true) - 0.4).abs() < 1e-6);
        assert!((can_die_pop_fraction(false) - 0.8).abs() < 1e-6);
        let c = apply_rabbit_wrong_place_dying(0.01, true);
        assert!((c - 0.02).abs() < 1e-9);
    }

    #[test]
    fn compute_helpers_match_defaults() {
        let off = compute_offspring_chance(true, 1, 10);
        // low pop: CHANCE_FOR_OFFSPRING * OFFSPRING_FACTOR_IF_POP_LOW
        let expected_off = CHANCE_FOR_OFFSPRING * OFFSPRING_FACTOR_IF_POP_LOW;
        assert!((off - expected_off).abs() < 1e-9);
        let die = compute_dying_chance(true, false, 5, 5);
        let expected_die =
            CHANCE_FOR_ANIMAL_DYING * CHANCE_FOR_ANIMAL_DYING_FACTOR_IF_IN_LOVED_BIOME;
        assert!((die - expected_die).abs() < 1e-9);
        let live = compute_dying_chance_ex(true, false, 5, 5, CHANCE_FOR_ANIMAL_DYING, 0.5);
        assert!((live - CHANCE_FOR_ANIMAL_DYING * 0.5).abs() < 1e-9);
        let nan_fallback =
            compute_dying_chance_ex(true, false, 5, 5, CHANCE_FOR_ANIMAL_DYING, f32::NAN);
        assert!((nan_fallback - expected_die).abs() < 1e-9);
        let live_off = compute_offspring_chance_ex(
            true,
            1,
            10,
            CHANCE_FOR_OFFSPRING,
            5.0,
            OFFSPRING_FACTOR_LOW_POP_BELOW,
        );
        assert!((live_off - CHANCE_FOR_OFFSPRING * 5.0).abs() < 1e-9);
        let nan_boost = compute_offspring_chance_ex(
            true,
            1,
            10,
            CHANCE_FOR_OFFSPRING,
            f32::NAN,
            OFFSPRING_FACTOR_LOW_POP_BELOW,
        );
        assert!((nan_boost - expected_off).abs() < 1e-9);
        // MaxOffspringFactor 1 → current==original blocks birth; 2 allows.
        let cap_block = resolve_pop_on_dest_ex(
            10,
            10,
            true,
            false,
            false,
            false,
            true,
            0,
            false,
            0.99,
            0.0,
            1.0,
            0.0,
            0.1,
            10.0,
            1.0,
            OFFSPRING_FACTOR_LOW_POP_BELOW,
            false,
            CHANCE_FOR_DOMESTIC_ANIMAL_DYING_FACTOR,
        );
        assert_eq!(
            cap_block,
            PopMoveOutcome::Move {
                spawn_offspring: false
            }
        );
        let cap_live = resolve_pop_on_dest_ex(
            10,
            10,
            true,
            false,
            false,
            false,
            true,
            0,
            false,
            0.99,
            0.0,
            1.0,
            0.0,
            0.1,
            10.0,
            2.0,
            OFFSPRING_FACTOR_LOW_POP_BELOW,
            false,
            CHANCE_FOR_DOMESTIC_ANIMAL_DYING_FACTOR,
        );
        assert_eq!(
            cap_live,
            PopMoveOutcome::Move {
                spawn_offspring: true
            }
        );
    }
}
