//! Weapon range / damage / held protection from object names (Haxe weapon subset).
//!
//! Table (name contains, case-insensitive):
//! - `"bow"` → range 8, damage 2.0
//! - `"sword"` / `"knife"` → range 2, damage 2.5 / 1.5
//! - `"spear"` → range 3, damage 2.5
//! - else → [`KILL_RANGE`], damage 1.0
//!
//! Held `damageProtectionFactor` (Haxe ObjectData): shield/armor reduce incoming.

use crate::combat::KILL_RANGE;

/// Bare-hand / default weapon damage (Haxe ~1).
pub const DEFAULT_WEAPON_DAMAGE: f32 = 1.0;

/// Resolve Chebyshev combat range from held object name.
pub fn weapon_range(held_id: i32, name: &str) -> i32 {
    if held_id == 0 {
        return KILL_RANGE;
    }
    let n = name.to_ascii_lowercase();
    if n.contains("bow") || n.contains("arrow") {
        8
    } else if n.contains("spear") || n.contains("lance") {
        3
    } else if n.contains("sword") || n.contains("knife") || n.contains("axe") {
        2
    } else if n.contains("club") || n.contains("hammer") {
        1
    } else {
        KILL_RANGE
    }
}

/// Base weapon damage before clothing / distance / RNG (Haxe `objectData.damage`).
pub fn weapon_damage(held_id: i32, name: &str) -> f32 {
    if held_id == 0 {
        return DEFAULT_WEAPON_DAMAGE;
    }
    let n = name.to_ascii_lowercase();
    if n.contains("bow") || n.contains("arrow") {
        2.0
    } else if n.contains("sword") {
        2.5
    } else if n.contains("spear") || n.contains("lance") {
        2.5
    } else if n.contains("axe") {
        2.2
    } else if n.contains("knife") {
        1.5
    } else if n.contains("club") || n.contains("hammer") {
        1.8
    } else {
        DEFAULT_WEAPON_DAMAGE
    }
}

/// Haxe `damageProtectionFactor` for the **target's held** object (1.0 = none).
/// Lower = more protection. Shield ~0.5, light armor ~0.8.
pub fn held_damage_protection_factor(held_id: i32, name: &str) -> f32 {
    if held_id == 0 {
        return 1.0;
    }
    let n = name.to_ascii_lowercase();
    if n.contains("shield") {
        0.5
    } else if n.contains("armor") || n.contains("mail") || n.contains("plate") {
        0.65
    } else if n.contains("sword") || n.contains("spear") || n.contains("knife") {
        // Parrying weapon — mild protection (Haxe class-for-weapon boost applied elsewhere).
        0.85
    } else {
        1.0
    }
}

/// Format `RANGE held=id range=N` chat body.
pub fn format_range_query(held_id: i32, range: i32) -> String {
    format!("RANGE held={held_id} range={range}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weapon_range_table() {
        // Required table.
        assert_eq!(weapon_range(1, "Long Bow"), 8);
        assert_eq!(weapon_range(1, "Composite Bow"), 8);
        assert_eq!(weapon_range(2, "Iron Sword"), 2);
        assert_eq!(weapon_range(2, "Flint Knife"), 2);
        assert_eq!(weapon_range(3, "Wooden Spear"), 3);
        assert_eq!(weapon_range(0, ""), KILL_RANGE);
        assert_eq!(weapon_range(9, "Stone"), KILL_RANGE);
        assert_eq!(weapon_range(9, "Berry"), KILL_RANGE);
        // Case-insensitive + priority (bow before knife).
        assert_eq!(weapon_range(1, "BOW"), 8);
        assert_eq!(weapon_range(1, "bow knife"), 8);
        // Extra aliases.
        assert_eq!(weapon_range(4, "Arrow"), 8);
        assert_eq!(weapon_range(5, "Lance"), 3);
        assert_eq!(weapon_range(6, "Club"), 1);
    }

    #[test]
    fn query_shape() {
        assert_eq!(format_range_query(12, 8), "RANGE held=12 range=8");
    }

    #[test]
    fn weapon_damage_and_protection_table() {
        assert!((weapon_damage(0, "") - DEFAULT_WEAPON_DAMAGE).abs() < 1e-6);
        assert!((weapon_damage(1, "Long Bow") - 2.0).abs() < 1e-6);
        assert!((weapon_damage(2, "Iron Sword") - 2.5).abs() < 1e-6);
        assert!((weapon_damage(3, "Flint Knife") - 1.5).abs() < 1e-6);
        assert!((held_damage_protection_factor(0, "") - 1.0).abs() < 1e-6);
        assert!((held_damage_protection_factor(1, "Wooden Shield") - 0.5).abs() < 1e-6);
        assert!(held_damage_protection_factor(2, "Plate Armor") < 0.7);
    }
}
