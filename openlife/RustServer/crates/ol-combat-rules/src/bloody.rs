//! Bloody-weapon id table + DoDamage cool-down (pure). Live equip stays in ol-sim.

/// Flint Knife.
pub const KNIFE_ID: i32 = 560;
/// Bloody Knife.
pub const BLOODY_KNIFE_ID: i32 = 750;
/// War Sword.
pub const WAR_SWORD_ID: i32 = 3047;
/// Bloody War Sword.
pub const BLOODY_WAR_SWORD_ID: i32 = 3048;
/// Bow and Arrow.
pub const BOW_AND_ARROW_ID: i32 = 152;
/// Bow and Arrow with Note.
pub const BOW_AND_ARROW_WITH_NOTE_ID: i32 = 1624;
/// Bloody Yew Bow.
pub const BLOODY_YEW_BOW_ID: i32 = 749;
/// Clean Yew Bow after bloody bow auto-clean.
pub const YEW_BOW_ID: i32 = 151;

/// Haxe `ServerSettings.WeaponCoolDownFactor`.
pub const WEAPON_COOLDOWN_FACTOR: f32 = 0.5;
/// Haxe `ServerSettings.WeaponCoolDownFactorIfWounding`.
pub const WEAPON_COOLDOWN_FACTOR_IF_WOUNDING: f32 = 5.0;
/// Default base seconds for DoDamage time-transition cool-down when content missing.
pub const BLOODY_WEAPON_STRIKE_BASE_TTC: f32 = 2.0;
/// Bloody Knife **750** `-1` auto-decay base.
pub const BLOODY_KNIFE_AUTO_DECAY_TTC: f32 = 3.0;
/// Bloody War Sword **3048** `-1` auto-decay base.
pub const BLOODY_WAR_SWORD_AUTO_DECAY_TTC: f32 = 2.0;
/// Bloody Yew Bow **749** `-1` auto-decay base.
pub const BLOODY_YEW_BOW_AUTO_DECAY_TTC: f32 = 6.0;

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

/// Map clean (or already bloody) weapon → bloody id.
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

/// Base `-1` auto-decay seconds for a bloody weapon id (patched content table).
pub fn bloody_weapon_auto_decay_base_ttc(bloody_id: i32) -> Option<f32> {
    match bloody_id {
        BLOODY_KNIFE_ID => Some(BLOODY_KNIFE_AUTO_DECAY_TTC),
        BLOODY_WAR_SWORD_ID => Some(BLOODY_WAR_SWORD_AUTO_DECAY_TTC),
        id if id == BLOODY_YEW_BOW_ID => Some(BLOODY_YEW_BOW_AUTO_DECAY_TTC),
        _ => None,
    }
}

/// Haxe `DoDamage` cool-down: `timeTransition.calculateTimeToChange() * factor`.
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

/// DoDamage-style bloody equip + cool-down using patched `-1` auto-decay bases.
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
pub fn bloody_weapon_after_strike_ex(
    held_id: i32,
    long_wounding: bool,
    normal_factor: f32,
    wounding_factor: f32,
) -> Option<BloodyWeaponTransform> {
    let bloody_id = bloody_weapon_id_for(held_id)?;
    let base =
        bloody_weapon_auto_decay_base_ttc(bloody_id).unwrap_or(BLOODY_WEAPON_STRIKE_BASE_TTC);
    let ttc = weapon_bloody_time_to_change_ex(base, long_wounding, normal_factor, wounding_factor);
    Some(BloodyWeaponTransform {
        from_held_id: held_id,
        new_held_id: bloody_id,
        time_to_change: ttc,
    })
}
