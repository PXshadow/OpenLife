//! Weapon range / damage / held protection + **bloody weapon** transforms.
//!
//! Name table (case-insensitive):
//! - `"bow"` → range 8, damage 2.0
//! - `"sword"` / `"knife"` → range 2, damage 2.5 / 1.5
//! - `"spear"` → range 3, damage 2.5
//! - else → [`KILL_RANGE`], damage 1.0
//!
//! Held `damageProtectionFactor` (Haxe ObjectData): shield/armor reduce incoming.
//!
//! ## Bloody weapons (COMBAT-BLOODY / `makeWeaponBloodyIfNeeded`)
//!
//! Haxe `GlobalPlayerInstance.makeWeaponBloodyIfNeeded` + `ServerSettings.PatchObjectData`:
//! - Knife **560** / Bloody Knife **750** → **750**
//! - War Sword **3047** / Bloody War Sword **3048** → **3048**
//! - Bow escape → Bloody Yew Bow **749** (see `animal_damage`)
//! - `isBloody` / `neverDrop` on 750, 3048, 749
//! - DoDamage weapon cool-down factors when wounding

use crate::animal_damage::{BLOODY_YEW_BOW_ID, BOW_AND_ARROW_ID};
use crate::combat::KILL_RANGE;

/// Bare-hand / default weapon damage (Haxe ~1).
pub const DEFAULT_WEAPON_DAMAGE: f32 = 1.0;

// --- Known weapon object ids (Haxe ServerSettings.PatchObjectData) ---

/// Flint Knife.
pub const KNIFE_ID: i32 = 560;
/// Bloody Knife.
pub const BLOODY_KNIFE_ID: i32 = 750;
/// War Sword.
pub const WAR_SWORD_ID: i32 = 3047;
/// Bloody War Sword.
pub const BLOODY_WAR_SWORD_ID: i32 = 3048;
/// Bow and Arrow with Note (Haxe deadlyDistance patch).
pub const BOW_AND_ARROW_WITH_NOTE_ID: i32 = 1624;

/// Haxe `ServerSettings.WeaponCoolDownFactor` (normal bloody cool-down mult).
pub const WEAPON_COOLDOWN_FACTOR: f32 = 0.5;
/// Haxe `ServerSettings.WeaponCoolDownFactorIfWounding`.
pub const WEAPON_COOLDOWN_FACTOR_IF_WOUNDING: f32 = 5.0;
/// Haxe `makeWeaponBloodyIfNeeded` fixed `heldObject.timeToChange = 3`.
pub const BLOODY_WEAPON_MAKE_TTC: f32 = 3.0;
/// Default base seconds for DoDamage time-transition cool-down when content missing.
pub const BLOODY_WEAPON_STRIKE_BASE_TTC: f32 = 2.0;

// Haxe: ServerSettings.PatchTransitions autoDecaySeconds on (-1, bloody)
/// Bloody Knife **750** `-1` auto-decay base (`PatchTransitions` = 3).
pub const BLOODY_KNIFE_AUTO_DECAY_TTC: f32 = 3.0;
/// Bloody War Sword **3048** `-1` auto-decay base (`PatchTransitions` = 2).
pub const BLOODY_WAR_SWORD_AUTO_DECAY_TTC: f32 = 2.0;
/// Bloody Yew Bow **749** `-1` auto-decay base (`PatchTransitions` = 6).
pub const BLOODY_YEW_BOW_AUTO_DECAY_TTC: f32 = 6.0;
/// Clean Yew Bow after bloody bow auto-clean (`-1+749 → 151`).
pub const YEW_BOW_ID: i32 = 151;
/// Haxe `TransitionHelper` neverDrop re-arm when timer already expired + isBloody.
pub const NEVER_DROP_REARM_TTC: f32 = 3.0;
/// Haxe patched `damage` for Knife (ServerSettings).
pub const KNIFE_DAMAGE: f32 = 5.0;
/// Haxe patched `damage` for War Sword.
pub const WAR_SWORD_DAMAGE: f32 = 6.0;
/// Haxe patched `damage` for Bow and Arrow **152**.
pub const BOW_AND_ARROW_DAMAGE: f32 = 9.0;
/// Haxe patched `damage` for Bow and Arrow with Note **1624**.
pub const BOW_AND_ARROW_WITH_NOTE_DAMAGE: f32 = 12.0;

/// Outcome of transforming a held weapon into its bloody form.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BloodyWeaponTransform {
    /// Clean or already-bloody held id before transform.
    pub from_held_id: i32,
    /// Bloody weapon object id to equip.
    pub new_held_id: i32,
    /// Haxe `heldObject.timeToChange` after transform.
    pub time_to_change: f32,
}

/// True when object id is a patched bloody weapon (`isBloody = true`).
// Haxe: ObjectData.isBloody / ObjectHelper.isBloody (ServerSettings patch)
#[inline]
pub fn is_bloody_weapon(object_id: i32) -> bool {
    matches!(
        object_id,
        BLOODY_KNIFE_ID | BLOODY_WAR_SWORD_ID | BLOODY_YEW_BOW_ID
    )
}

/// Haxe `neverDrop` on bloody weapons — refuse DROP while stuck with blood.
// Haxe: ObjectData.neverDrop (ServerSettings patch 750/3048/749)
#[inline]
pub fn is_never_drop_weapon(object_id: i32) -> bool {
    is_bloody_weapon(object_id)
}

/// Haxe `speedMult` patches for bloody weapons (move mali).
// Haxe: ServerSettings.PatchObjectData speedMult bloody
pub fn bloody_weapon_speed_mult(object_id: i32) -> Option<f32> {
    match object_id {
        BLOODY_KNIFE_ID => Some(0.75),
        BLOODY_WAR_SWORD_ID => Some(0.85),
        BLOODY_YEW_BOW_ID => Some(0.6),
        _ => None,
    }
}

/// Map clean (or already bloody) weapon → bloody id.
///
/// - Knife 560 / Bloody Knife 750 → 750  
/// - War Sword 3047 / Bloody War Sword 3048 → 3048  
/// - Bow 152 / 1624 / 749 → Bloody Yew Bow 749 (strike / escape family)  
///
/// Returns `None` for non-mapped weapons.
// Haxe: makeWeaponBloodyIfNeeded bloodyWeaponId table (+ bow family for DoDamage/escape)
pub fn bloody_weapon_id_for(held_id: i32) -> Option<i32> {
    match held_id {
        KNIFE_ID | BLOODY_KNIFE_ID => Some(BLOODY_KNIFE_ID),
        WAR_SWORD_ID | BLOODY_WAR_SWORD_ID => Some(BLOODY_WAR_SWORD_ID),
        id if id == BOW_AND_ARROW_ID
            || id == BOW_AND_ARROW_WITH_NOTE_ID
            || id == BLOODY_YEW_BOW_ID =>
        {
            Some(BLOODY_YEW_BOW_ID)
        }
        _ => None,
    }
}

/// Haxe `DoDamage` cool-down: `timeTransition.calculateTimeToChange() * factor`.
// Haxe: DoDamage WeaponCoolDownFactor / WeaponCoolDownFactorIfWounding
#[inline]
pub fn weapon_bloody_time_to_change(base_ttc: f32, long_wounding: bool) -> f32 {
    weapon_bloody_time_to_change_ex(
        base_ttc,
        long_wounding,
        WEAPON_COOLDOWN_FACTOR,
        WEAPON_COOLDOWN_FACTOR_IF_WOUNDING,
    )
}

/// Live-knob variant of [`weapon_bloody_time_to_change`].
// Haxe: ServerSettings.WeaponCoolDownFactor / WeaponCoolDownFactorIfWounding
// C-SS-MORE-BATCH5
#[inline]
pub fn weapon_bloody_time_to_change_ex(
    base_ttc: f32,
    long_wounding: bool,
    normal_factor: f32,
    wounding_factor: f32,
) -> f32 {
    let nf = if normal_factor.is_finite() && normal_factor > 0.0 {
        normal_factor
    } else {
        WEAPON_COOLDOWN_FACTOR
    };
    let wf = if wounding_factor.is_finite() && wounding_factor > 0.0 {
        wounding_factor
    } else {
        WEAPON_COOLDOWN_FACTOR_IF_WOUNDING
    };
    let factor = if long_wounding { wf } else { nf };
    (base_ttc.max(0.0) * factor).max(0.0)
}

/// Base `-1` auto-decay seconds for a bloody weapon id (patched content table).
///
/// Haxe `TransitionImporter.GetTransition(-1, bloodyId).calculateTimeToChange()` after
/// `ServerSettings.PatchTransitions` sets `autoDecaySeconds` 3/2/6 for 750/3048/749.
// Haxe: ServerSettings.PatchTransitions + DoDamage timeTransition base
pub fn bloody_weapon_auto_decay_base_ttc(bloody_id: i32) -> Option<f32> {
    match bloody_id {
        BLOODY_KNIFE_ID => Some(BLOODY_KNIFE_AUTO_DECAY_TTC),
        BLOODY_WAR_SWORD_ID => Some(BLOODY_WAR_SWORD_AUTO_DECAY_TTC),
        id if id == BLOODY_YEW_BOW_ID => Some(BLOODY_YEW_BOW_AUTO_DECAY_TTC),
        _ => None,
    }
}

/// Clean weapon id after held bloody auto-decay (`DoTimeOnPlayerObjects` `-1` trans).
///
/// Content outcomes (vanilla + OpenLife): 750→560, 3048→3047, 749→151.
// Haxe: TimeHelper.DoTimeOnPlayerObjects GetTransition(-1, held) → newTarget / alt
pub fn bloody_weapon_clean_id_for(bloody_id: i32) -> Option<i32> {
    match bloody_id {
        BLOODY_KNIFE_ID => Some(KNIFE_ID),
        BLOODY_WAR_SWORD_ID => Some(WAR_SWORD_ID),
        id if id == BLOODY_YEW_BOW_ID => Some(YEW_BOW_ID),
        _ => None,
    }
}

/// When bloody held `timeToChange` elapses, return clean weapon id to equip.
// Haxe: TimeHelper.DoTimeOnPlayerObjects held -1 transition
pub fn try_bloody_weapon_auto_clean(
    held_id: i32,
    creation_time: f32,
    time_to_change: f32,
    sim_time: f32,
) -> Option<i32> {
    if !is_bloody_weapon(held_id) || time_to_change <= 0.0 {
        return None;
    }
    if (sim_time - creation_time) < time_to_change {
        return None;
    }
    bloody_weapon_clean_id_for(held_id)
}

/// Seconds remaining on neverDrop cool-down (`ttc - (sim - creation)`), floored at 0.
// Haxe: TransitionHelper isNeverDrop time = timeToChange - CalculateTimeSinceTicksInSec
#[inline]
pub fn never_drop_remaining_secs(creation_time: f32, time_to_change: f32, sim_time: f32) -> f32 {
    if time_to_change <= 0.0 {
        return 0.0;
    }
    (time_to_change - (sim_time - creation_time)).max(0.0)
}

/// Haxe: `time <= 0 && isBloody` → re-arm `timeToChange = 3` (unstick forever-stuck bloody).
// Haxe: TransitionHelper isNeverDrop isBloody re-arm
#[inline]
pub fn never_drop_should_rearm(remaining_secs: f32, is_bloody: bool) -> bool {
    is_bloody && remaining_secs <= 0.0
}

/// Haxe: `if (time > 4 && time <= 60) player.say('${Math.ceil(time)} seconds...', true)`.
// Haxe: TransitionHelper isNeverDrop countdown say
pub fn never_drop_countdown_say(remaining_secs: f32) -> Option<String> {
    let ceil = remaining_secs.ceil() as i32;
    if ceil > 4 && ceil <= 60 {
        Some(format!("{ceil} seconds..."))
    } else {
        None
    }
}

/// Haxe `GlobalPlayerInstance.makeWeaponBloodyIfNeeded(target)`.
///
/// Only when `target_is_deadly_animal` (Haxe `target.isDeadlyAnimal()`).
/// Sets bloody id + fixed `timeToChange = 3`. Returns `None` if no transform.
// Haxe: GlobalPlayerInstance.makeWeaponBloodyIfNeeded
pub fn make_weapon_bloody_if_needed(
    held_id: i32,
    target_is_deadly_animal: bool,
) -> Option<BloodyWeaponTransform> {
    if !target_is_deadly_animal {
        return None;
    }
    // Haxe only maps knife/sword (not bow) in makeWeaponBloodyIfNeeded.
    let bloody_id = match held_id {
        KNIFE_ID | BLOODY_KNIFE_ID => BLOODY_KNIFE_ID,
        WAR_SWORD_ID | BLOODY_WAR_SWORD_ID => BLOODY_WAR_SWORD_ID,
        _ => return None,
    };
    Some(BloodyWeaponTransform {
        from_held_id: held_id,
        new_held_id: bloody_id,
        time_to_change: BLOODY_WEAPON_MAKE_TTC,
    })
}

/// DoDamage-style bloody equip + cool-down using patched `-1` auto-decay bases.
///
/// Base TTC = [`bloody_weapon_auto_decay_base_ttc`] (3/2/6 for knife/sword/bow) else
/// [`BLOODY_WEAPON_STRIKE_BASE_TTC`]. Multiplied by cool-down factors 0.5 / 5.
/// `long_wounding` = Haxe `longWeaponCoolDown` (first wound / kill).
// Haxe: DoDamage fromObj.id = trans.newActorID + GetTransition(-1,newActor) * factor
pub fn bloody_weapon_after_strike(
    held_id: i32,
    long_wounding: bool,
) -> Option<BloodyWeaponTransform> {
    bloody_weapon_after_strike_ex(
        held_id,
        long_wounding,
        WEAPON_COOLDOWN_FACTOR,
        WEAPON_COOLDOWN_FACTOR_IF_WOUNDING,
    )
}

/// Live-knob variant of [`bloody_weapon_after_strike`].
// Haxe: DoDamage cool-down × WeaponCoolDownFactor*
// C-SS-MORE-BATCH5
pub fn bloody_weapon_after_strike_ex(
    held_id: i32,
    long_wounding: bool,
    normal_factor: f32,
    wounding_factor: f32,
) -> Option<BloodyWeaponTransform> {
    let bloody_id = bloody_weapon_id_for(held_id)?;
    // Already bloody with same id still re-arms cool-down (Haxe re-sets held + ttc).
    let base = bloody_weapon_auto_decay_base_ttc(bloody_id)
        .unwrap_or(BLOODY_WEAPON_STRIKE_BASE_TTC);
    let ttc = weapon_bloody_time_to_change_ex(base, long_wounding, normal_factor, wounding_factor);
    Some(BloodyWeaponTransform {
        from_held_id: held_id,
        new_held_id: bloody_id,
        time_to_change: ttc,
    })
}

/// Resolve Chebyshev combat range from held object name / known ids.
pub fn weapon_range(held_id: i32, name: &str) -> i32 {
    if held_id == 0 {
        return KILL_RANGE;
    }
    // Known patched ids (bloody keep same melee / bow range family).
    if held_id == KNIFE_ID
        || held_id == BLOODY_KNIFE_ID
        || held_id == WAR_SWORD_ID
        || held_id == BLOODY_WAR_SWORD_ID
    {
        return 2;
    }
    if held_id == BOW_AND_ARROW_ID
        || held_id == BOW_AND_ARROW_WITH_NOTE_ID
        || held_id == BLOODY_YEW_BOW_ID
    {
        return 8;
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
    // Haxe ServerSettings.PatchObjectData damage patches.
    if held_id == KNIFE_ID || held_id == BLOODY_KNIFE_ID {
        return KNIFE_DAMAGE;
    }
    if held_id == WAR_SWORD_ID || held_id == BLOODY_WAR_SWORD_ID {
        return WAR_SWORD_DAMAGE;
    }
    // Haxe: 152 damage=9, 1624 damage=12 (bloody yew bow not damage-patched).
    if held_id == BOW_AND_ARROW_ID {
        return BOW_AND_ARROW_DAMAGE;
    }
    if held_id == BOW_AND_ARROW_WITH_NOTE_ID {
        return BOW_AND_ARROW_WITH_NOTE_DAMAGE;
    }
    if held_id == BLOODY_YEW_BOW_ID {
        return 2.0;
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
    // Knife / War Sword patched 0.8 (bloody same family).
    if held_id == KNIFE_ID
        || held_id == BLOODY_KNIFE_ID
        || held_id == WAR_SWORD_ID
        || held_id == BLOODY_WAR_SWORD_ID
    {
        return 0.8;
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
        // Known bloody / clean ids.
        assert_eq!(weapon_range(KNIFE_ID, ""), 2);
        assert_eq!(weapon_range(BLOODY_KNIFE_ID, ""), 2);
        assert_eq!(weapon_range(WAR_SWORD_ID, ""), 2);
        assert_eq!(weapon_range(BLOODY_WAR_SWORD_ID, ""), 2);
        assert_eq!(weapon_range(BLOODY_YEW_BOW_ID, ""), 8);
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
        // Patched knife/sword/bow damage + protection.
        assert!((weapon_damage(KNIFE_ID, "") - KNIFE_DAMAGE).abs() < 1e-6);
        assert!((weapon_damage(BLOODY_KNIFE_ID, "") - KNIFE_DAMAGE).abs() < 1e-6);
        assert!((weapon_damage(WAR_SWORD_ID, "") - WAR_SWORD_DAMAGE).abs() < 1e-6);
        assert!((weapon_damage(BOW_AND_ARROW_ID, "") - BOW_AND_ARROW_DAMAGE).abs() < 1e-6);
        assert!(
            (weapon_damage(BOW_AND_ARROW_WITH_NOTE_ID, "") - BOW_AND_ARROW_WITH_NOTE_DAMAGE).abs()
                < 1e-6
        );
        assert!((held_damage_protection_factor(KNIFE_ID, "") - 0.8).abs() < 1e-6);
    }

    #[test]
    fn is_bloody_and_never_drop() {
        assert!(is_bloody_weapon(BLOODY_KNIFE_ID));
        assert!(is_bloody_weapon(BLOODY_WAR_SWORD_ID));
        assert!(is_bloody_weapon(BLOODY_YEW_BOW_ID));
        assert!(!is_bloody_weapon(KNIFE_ID));
        assert!(!is_bloody_weapon(0));
        assert!(is_never_drop_weapon(BLOODY_KNIFE_ID));
        assert!(!is_never_drop_weapon(KNIFE_ID));
        assert_eq!(bloody_weapon_speed_mult(BLOODY_KNIFE_ID), Some(0.75));
        assert_eq!(bloody_weapon_speed_mult(BLOODY_WAR_SWORD_ID), Some(0.85));
        assert_eq!(bloody_weapon_speed_mult(BLOODY_YEW_BOW_ID), Some(0.6));
        assert_eq!(bloody_weapon_speed_mult(KNIFE_ID), None);
    }

    #[test]
    fn make_weapon_bloody_if_needed_deadly_only() {
        // Non-deadly → no transform.
        assert!(make_weapon_bloody_if_needed(KNIFE_ID, false).is_none());
        // Deadly + knife → bloody knife, ttc=3.
        let t = make_weapon_bloody_if_needed(KNIFE_ID, true).expect("knife");
        assert_eq!(t.from_held_id, KNIFE_ID);
        assert_eq!(t.new_held_id, BLOODY_KNIFE_ID);
        assert!((t.time_to_change - BLOODY_WEAPON_MAKE_TTC).abs() < 1e-6);
        // Already bloody knife still ok (re-equip).
        let t2 = make_weapon_bloody_if_needed(BLOODY_KNIFE_ID, true).unwrap();
        assert_eq!(t2.new_held_id, BLOODY_KNIFE_ID);
        // War sword.
        let s = make_weapon_bloody_if_needed(WAR_SWORD_ID, true).unwrap();
        assert_eq!(s.new_held_id, BLOODY_WAR_SWORD_ID);
        // Bow not in makeWeaponBloodyIfNeeded table.
        assert!(make_weapon_bloody_if_needed(BOW_AND_ARROW_ID, true).is_none());
        // Stick / empty.
        assert!(make_weapon_bloody_if_needed(0, true).is_none());
        assert!(make_weapon_bloody_if_needed(99, true).is_none());
    }

    #[test]
    fn bloody_weapon_after_strike_cooldown() {
        // Knife 750 base autoDecay=3 → normal 1.5, long 15.
        let normal = bloody_weapon_after_strike(KNIFE_ID, false).unwrap();
        assert_eq!(normal.new_held_id, BLOODY_KNIFE_ID);
        assert!(
            (normal.time_to_change - BLOODY_KNIFE_AUTO_DECAY_TTC * WEAPON_COOLDOWN_FACTOR).abs()
                < 1e-5
        );
        let wound = bloody_weapon_after_strike(WAR_SWORD_ID, true).unwrap();
        assert_eq!(wound.new_held_id, BLOODY_WAR_SWORD_ID);
        assert!(
            (wound.time_to_change
                - BLOODY_WAR_SWORD_AUTO_DECAY_TTC * WEAPON_COOLDOWN_FACTOR_IF_WOUNDING)
                .abs()
                < 1e-5
        );
        // HIT subsequent wound uses normal factor (not IfWounding).
        let sub = bloody_weapon_after_strike(KNIFE_ID, false).unwrap();
        assert!((sub.time_to_change - 1.5).abs() < 1e-5);
        // Bow family → bloody yew bow, base 6 * 0.5 = 3.
        let bow = bloody_weapon_after_strike(BOW_AND_ARROW_ID, false).unwrap();
        assert_eq!(bow.new_held_id, BLOODY_YEW_BOW_ID);
        assert!(
            (bow.time_to_change - BLOODY_YEW_BOW_AUTO_DECAY_TTC * WEAPON_COOLDOWN_FACTOR).abs()
                < 1e-5
        );
        assert!(bloody_weapon_after_strike(12345, false).is_none());
    }

    #[test]
    fn weapon_bloody_time_to_change_factors() {
        assert!((weapon_bloody_time_to_change(2.0, false) - 1.0).abs() < 1e-6);
        assert!((weapon_bloody_time_to_change(2.0, true) - 10.0).abs() < 1e-6);
        assert!((weapon_bloody_time_to_change(0.0, true) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn weapon_bloody_time_to_change_ex_live() {
        // C-SS-MORE-BATCH5: live 0.25 / 4.0 → 2*0.25=0.5, 2*4=8
        assert!(
            (weapon_bloody_time_to_change_ex(2.0, false, 0.25, 4.0) - 0.5).abs() < 1e-6
        );
        assert!((weapon_bloody_time_to_change_ex(2.0, true, 0.25, 4.0) - 8.0).abs() < 1e-6);
        let xf = bloody_weapon_after_strike_ex(KNIFE_ID, false, 0.25, 4.0).unwrap();
        assert!(
            (xf.time_to_change - BLOODY_KNIFE_AUTO_DECAY_TTC * 0.25).abs() < 1e-5
        );
    }

    #[test]
    fn bloody_auto_decay_table_and_clean_map() {
        assert_eq!(
            bloody_weapon_auto_decay_base_ttc(BLOODY_KNIFE_ID),
            Some(3.0)
        );
        assert_eq!(
            bloody_weapon_auto_decay_base_ttc(BLOODY_WAR_SWORD_ID),
            Some(2.0)
        );
        assert_eq!(
            bloody_weapon_auto_decay_base_ttc(BLOODY_YEW_BOW_ID),
            Some(6.0)
        );
        assert_eq!(bloody_weapon_auto_decay_base_ttc(KNIFE_ID), None);
        assert_eq!(bloody_weapon_clean_id_for(BLOODY_KNIFE_ID), Some(KNIFE_ID));
        assert_eq!(
            bloody_weapon_clean_id_for(BLOODY_WAR_SWORD_ID),
            Some(WAR_SWORD_ID)
        );
        assert_eq!(
            bloody_weapon_clean_id_for(BLOODY_YEW_BOW_ID),
            Some(YEW_BOW_ID)
        );
        // Not yet reached.
        assert!(try_bloody_weapon_auto_clean(BLOODY_KNIFE_ID, 10.0, 3.0, 12.9).is_none());
        // Elapsed → clean knife.
        assert_eq!(
            try_bloody_weapon_auto_clean(BLOODY_KNIFE_ID, 10.0, 3.0, 13.0),
            Some(KNIFE_ID)
        );
        assert_eq!(
            try_bloody_weapon_auto_clean(BLOODY_YEW_BOW_ID, 0.0, 6.0, 6.0),
            Some(YEW_BOW_ID)
        );
        // Clean knife never auto-cleans.
        assert!(try_bloody_weapon_auto_clean(KNIFE_ID, 0.0, 3.0, 99.0).is_none());
    }

    #[test]
    fn never_drop_remaining_and_rearm_countdown() {
        assert!((never_drop_remaining_secs(0.0, 10.0, 3.0) - 7.0).abs() < 1e-5);
        assert!((never_drop_remaining_secs(0.0, 2.0, 5.0) - 0.0).abs() < 1e-5);
        assert!(never_drop_should_rearm(0.0, true));
        assert!(!never_drop_should_rearm(0.1, true));
        assert!(!never_drop_should_rearm(0.0, false));
        assert_eq!(
            never_drop_countdown_say(7.2).as_deref(),
            Some("8 seconds...")
        );
        assert!(never_drop_countdown_say(4.0).is_none());
        assert!(never_drop_countdown_say(61.0).is_none());
    }
}
