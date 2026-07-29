// COMBAT-MOSQUITO-KIND — pure helpers included from animals.rs
// Haxe: GlobalPlayerInstance.biomeLoveFactor(BiomeTag.JUNGLE) for moskitoDamageFactor

use crate::food_store_max::{biome_love_factor, BIOME_TAG_JUNGLE};

/// Haxe `biomeLoveFactor(BiomeTag.JUNGLE)` for mosquito damage / fever infect.
// Haxe: DoDamage lovesJungle L4647
// COMBAT-MOSQUITO-KIND
#[inline]
pub fn jungle_biome_love_for_mosquito(
    floor_id: i32,
    self_color: i32,
    mother_color: Option<i32>,
    father_color: Option<i32>,
) -> f32 {
    biome_love_factor(
        BIOME_TAG_JUNGLE,
        floor_id,
        self_color,
        mother_color,
        father_color,
    )
}

/// Apply Haxe mosquito damage scale onto rolled path damage.
// Haxe: else damage *= moskitoDamageFactor L4660
#[inline]
pub fn scale_damage_by_moskito_factor(applied_damage: f32, moskito_factor: f32) -> f32 {
    let d = if applied_damage.is_finite() {
        applied_damage.max(0.0)
    } else {
        0.0
    };
    let f = if moskito_factor.is_finite() {
        moskito_factor.max(0.0)
    } else {
        1.0
    };
    d * f
}

#[cfg(test)]
mod mosquito_kind_tests {
    use super::*;
    use crate::weapon_wound::moskito_damage_factor;

    #[test]
    fn jungle_love_brown_person() {
        // PersonColor.Brown = 3 loves JUNGLE → 1.0
        let love = jungle_biome_love_for_mosquito(0, 3, None, None);
        assert!((love - 1.0).abs() < 1e-6);
        // black (1) in jungle alone → −0.5
        let hate = jungle_biome_love_for_mosquito(0, 1, None, None);
        assert!((hate - (-0.5)).abs() < 1e-6);
    }

    #[test]
    fn moskito_factor_uses_jungle_love() {
        let love = jungle_biome_love_for_mosquito(0, 3, None, None);
        // loves=1, yf=0, hf=1 → 1/(1+1)=0.5
        assert!((moskito_damage_factor(love, 0.0, 1.0) - 0.5).abs() < 1e-5);
        let scaled = scale_damage_by_moskito_factor(2.0, 0.5);
        assert!((scaled - 1.0).abs() < 1e-5);
    }
}
