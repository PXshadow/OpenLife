//! Haxe `TimeHelper.DoTimeStuffForPlayer` angryTime recovery / drain.
//!
//! `moreAngry` (killMode or last attacker holding a weapon) drains toward
//! `CombatAngryTimeMinimum`; otherwise recover toward `CombatAngryTimeBeforeAttack`
//! with a biome factor (passable river ×2, desert ×0.5).

use ol_world::{DESERT, PASSABLE_RIVER};

/// Haxe `CombatAngryTimeMinimum`.
pub const COMBAT_ANGRY_TIME_MINIMUM: f32 = -60.0;

/// Biome factor for angry recovery (Haxe TimeHelper ~396–397).
// Haxe: PASSABLERIVER → 2, DESERT → 0.5, else 1
#[inline]
pub fn biome_angry_recover_factor(biome: u8) -> f32 {
    if biome == PASSABLE_RIVER {
        2.0
    } else if biome == DESERT {
        0.5
    } else {
        1.0
    }
}

/// Step `angryTime` for one player.
// Haxe: TimeHelper.DoTimeStuffForPlayer ~387–400
pub fn tick_angry_time(
    angry_time: f32,
    dt: f32,
    more_angry: bool,
    biome: u8,
    min_angry: f32,
    before_attack: f32,
) -> f32 {
    let dt = if dt.is_finite() && dt > 0.0 { dt } else { 0.0 };
    let min_a = if min_angry.is_finite() {
        min_angry
    } else {
        COMBAT_ANGRY_TIME_MINIMUM
    };
    let before = if before_attack.is_finite() {
        before_attack
    } else {
        5.0
    };
    let mut t = if angry_time.is_finite() {
        angry_time
    } else {
        before
    };
    if more_angry {
        if t > min_a {
            t -= dt;
        }
        if t < min_a {
            t = min_a;
        }
    } else if t < before {
        t += dt * biome_angry_recover_factor(biome);
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recover_toward_before_attack() {
        let t = tick_angry_time(0.0, 1.0, false, 0, -60.0, 5.0);
        assert!((t - 1.0).abs() < 1e-5);
    }

    #[test]
    fn river_recovers_faster() {
        let t = tick_angry_time(0.0, 1.0, false, PASSABLE_RIVER, -60.0, 5.0);
        assert!((t - 2.0).abs() < 1e-5);
    }

    #[test]
    fn desert_recovers_slower() {
        let t = tick_angry_time(0.0, 1.0, false, DESERT, -60.0, 5.0);
        assert!((t - 0.5).abs() < 1e-5);
    }

    #[test]
    fn more_angry_drains_to_minimum() {
        let t = tick_angry_time(1.0, 10.0, true, 0, -60.0, 5.0);
        assert!((t - (-9.0)).abs() < 1e-5);
        let clamped = tick_angry_time(-59.0, 10.0, true, 0, -60.0, 5.0);
        assert!((clamped - (-60.0)).abs() < 1e-5);
    }

    #[test]
    fn does_not_recover_past_before_attack() {
        let t = tick_angry_time(5.0, 1.0, false, 0, -60.0, 5.0);
        assert!((t - 5.0).abs() < 1e-5);
    }

}
