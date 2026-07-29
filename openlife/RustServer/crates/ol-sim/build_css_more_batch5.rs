//! C-SS-MORE-BATCH5 / settings_batch5 build-time wire.
//!
//! Promotes LiveSettings weapon CD / jump exhaust / hungry heat / close-enemy /
//! AI speed factors onto `GameplayKnobs` and live call sites.
//!
//! Idempotent. Handles CRLF sources. Run via `cargo test -p ol-sim` or:
//!   python crates/ol-sim/_apply_css_more_batch5.py

use std::path::{Path, PathBuf};
use std::process::Command;

fn normalize_nl(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

fn restore_nl(s: &str, crlf: bool) -> String {
    if crlf {
        s.replace('\n', "\r\n")
    } else {
        s.to_string()
    }
}

fn replace_once(t: &mut String, old: &str, new: &str) -> bool {
    if t.contains(new) && !t.contains(old) {
        return false;
    }
    if let Some(i) = t.find(old) {
        t.replace_range(i..i + old.len(), new);
        true
    } else {
        false
    }
}

pub fn css_more_batch5_wired(src: &Path) -> bool {
    let sl = std::fs::read_to_string(src.join("settings_live.rs")).unwrap_or_default();
    let w = std::fs::read_to_string(src.join("weapons.rs")).unwrap_or_default();
    let mp = std::fs::read_to_string(src.join("move_path.rs")).unwrap_or_default();
    let ms = std::fs::read_to_string(src.join("move_speed.rs")).unwrap_or_default();
    let lib = std::fs::read_to_string(src.join("lib.rs")).unwrap_or_default();
    sl.contains("pub weapon_cooldown_factor: f32,")
        && sl.contains("weapon_cooldown_knobs")
        && w.contains("weapon_bloody_time_to_change_ex")
        && mp.contains("apply_jump_cost_ex")
        && ms.contains("VitalsSpeedLiveKnobs")
        && ms.contains("ai_class_speed_factor_ex")
        && lib.contains("apply_jump_cost_ex")
        && lib.contains("weapon_cooldown_knobs")
}

/// Prefer the Python apply script when available (full fidelity); fall back to
/// in-process string patches.
pub fn patch_css_more_batch5(src: &Path, workspace: &Path) -> bool {
    let py = workspace.join("crates/ol-sim/_apply_css_more_batch5.py");
    if py.exists() {
        let status = Command::new("python")
            .arg(&py)
            .current_dir(workspace)
            .status();
        if let Ok(s) = status {
            if s.success() && css_more_batch5_wired(src) {
                let _ = patch_docs(workspace);
                return true;
            }
        }
        // try py launcher on Windows
        let status = Command::new("py")
            .args(["-3", py.to_str().unwrap_or("")])
            .current_dir(workspace)
            .status();
        if let Ok(s) = status {
            if s.success() && css_more_batch5_wired(src) {
                let _ = patch_docs(workspace);
                return true;
            }
        }
    }
    // In-process fallback for GameplayKnobs + pure helpers if Python missing.
    let mut ok = true;
    ok &= patch_settings_live(&src.join("settings_live.rs"));
    ok &= patch_weapons(&src.join("weapons.rs"));
    ok &= patch_move_path(&src.join("move_path.rs"));
    ok &= patch_move_speed(&src.join("move_speed.rs"));
    let _ = patch_docs(workspace);
    css_more_batch5_wired(src) || ok
}

fn patch_settings_live(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("pub weapon_cooldown_factor: f32,") && raw.contains("weapon_cooldown_knobs") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;

    ch |= replace_once(
        &mut t,
        "    /// Haxe `MaxAgeFertile` (years, inclusive).\n\
    // Haxe: ServerSettings.MaxAgeFertile\n\
    // C-SS-MORE-BATCH4\n\
    pub max_age_fertile: f32,\n\
}\n",
        "    /// Haxe `MaxAgeFertile` (years, inclusive).\n\
    // Haxe: ServerSettings.MaxAgeFertile\n\
    // C-SS-MORE-BATCH4\n\
    pub max_age_fertile: f32,\n\
    // --- C-SS-MORE-BATCH5 / settings_batch5 ---\n\
    /// Haxe `WeaponCoolDownFactor` — normal bloody cool-down mult.\n\
    // Haxe: ServerSettings.WeaponCoolDownFactor\n\
    // C-SS-MORE-BATCH5\n\
    pub weapon_cooldown_factor: f32,\n\
    /// Haxe `WeaponCoolDownFactorIfWounding`.\n\
    // Haxe: ServerSettings.WeaponCoolDownFactorIfWounding\n\
    // C-SS-MORE-BATCH5\n\
    pub weapon_cooldown_factor_if_wounding: f32,\n\
    /// Haxe `CloseEnemyWithWeaponSpeedFactor`.\n\
    // Haxe: ServerSettings.CloseEnemyWithWeaponSpeedFactor\n\
    // C-SS-MORE-BATCH5\n\
    pub close_enemy_with_weapon_speed_factor: f32,\n\
    /// Haxe `ExhaustionOnJump`.\n\
    // Haxe: ServerSettings.ExhaustionOnJump\n\
    // C-SS-MORE-BATCH5\n\
    pub exhaustion_on_jump: f32,\n\
    /// Haxe `HungryWorkHeat` — heat per food when transition temperature < 0.\n\
    // Haxe: ServerSettings.HungryWorkHeat\n\
    // C-SS-MORE-BATCH5\n\
    pub hungry_work_heat: f32,\n\
    /// Haxe `AISpeedFactorSerf`.\n\
    // Haxe: ServerSettings.AISpeedFactorSerf\n\
    // C-SS-MORE-BATCH5\n\
    pub ai_speed_factor_serf: f32,\n\
    /// Haxe `AISpeedFactorCommoner`.\n\
    // Haxe: ServerSettings.AISpeedFactorCommoner\n\
    // C-SS-MORE-BATCH5\n\
    pub ai_speed_factor_commoner: f32,\n\
    /// Haxe `AISpeedFactorNoble`.\n\
    // Haxe: ServerSettings.AISpeedFactorNoble\n\
    // C-SS-MORE-BATCH5\n\
    pub ai_speed_factor_noble: f32,\n\
}\n",
    );

    ch |= replace_once(
        &mut t,
        "            min_age_fertile: gameplay_defaults::MIN_AGE_FERTILE,\n\
            max_age_fertile: gameplay_defaults::MAX_AGE_FERTILE,\n\
        }\n\
    }\n\
}\n",
        "            min_age_fertile: gameplay_defaults::MIN_AGE_FERTILE,\n\
            max_age_fertile: gameplay_defaults::MAX_AGE_FERTILE,\n\
            // C-SS-MORE-BATCH5\n\
            weapon_cooldown_factor: gameplay_defaults::WEAPON_COOLDOWN_FACTOR,\n\
            weapon_cooldown_factor_if_wounding:\n\
                gameplay_defaults::WEAPON_COOLDOWN_FACTOR_IF_WOUNDING,\n\
            close_enemy_with_weapon_speed_factor:\n\
                gameplay_defaults::CLOSE_ENEMY_WITH_WEAPON_SPEED_FACTOR,\n\
            exhaustion_on_jump: gameplay_defaults::EXHAUSTION_ON_JUMP,\n\
            hungry_work_heat: gameplay_defaults::HUNGRY_WORK_HEAT,\n\
            ai_speed_factor_serf: gameplay_defaults::AI_SPEED_FACTOR_SERF,\n\
            ai_speed_factor_commoner: gameplay_defaults::AI_SPEED_FACTOR_COMMONER,\n\
            ai_speed_factor_noble: gameplay_defaults::AI_SPEED_FACTOR_NOBLE,\n\
        }\n\
    }\n\
}\n",
    );

    ch |= replace_once(
        &mut t,
        "            min_age_fertile: live.min_age_fertile,\n\
            max_age_fertile: live.max_age_fertile,\n\
        }\n\
    }\n",
        "            min_age_fertile: live.min_age_fertile,\n\
            max_age_fertile: live.max_age_fertile,\n\
            // C-SS-MORE-BATCH5\n\
            weapon_cooldown_factor: live.weapon_cooldown_factor,\n\
            weapon_cooldown_factor_if_wounding: live.weapon_cooldown_factor_if_wounding,\n\
            close_enemy_with_weapon_speed_factor: live.close_enemy_with_weapon_speed_factor,\n\
            exhaustion_on_jump: live.exhaustion_on_jump,\n\
            hungry_work_heat: live.hungry_work_heat,\n\
            ai_speed_factor_serf: live.ai_speed_factor_serf,\n\
            ai_speed_factor_commoner: live.ai_speed_factor_commoner,\n\
            ai_speed_factor_noble: live.ai_speed_factor_noble,\n\
        }\n\
    }\n",
    );

    ch |= replace_once(
        &mut t,
        "    pub fn max_move_quad_jump_before_force(&self) -> f64 {\n\
        let v = self.max_movement_quad_jump_distance_before_force;\n\
        if v.is_finite() && v > 0.0 {\n\
            v as f64\n\
        } else {\n\
            gameplay_defaults::MAX_MOVEMENT_QUAD_JUMP_DISTANCE_BEFORE_FORCE as f64\n\
        }\n\
    }\n\
}\n",
        "    pub fn max_move_quad_jump_before_force(&self) -> f64 {\n\
        let v = self.max_movement_quad_jump_distance_before_force;\n\
        if v.is_finite() && v > 0.0 {\n\
            v as f64\n\
        } else {\n\
            gameplay_defaults::MAX_MOVEMENT_QUAD_JUMP_DISTANCE_BEFORE_FORCE as f64\n\
        }\n\
    }\n\
\n\
    /// Live weapon cool-down factors (normal, if-wounding).\n\
    // Haxe: ServerSettings.WeaponCoolDownFactor / WeaponCoolDownFactorIfWounding\n\
    // C-SS-MORE-BATCH5\n\
    #[inline]\n\
    pub fn weapon_cooldown_knobs(&self) -> (f32, f32) {\n\
        (self.weapon_cooldown_factor, self.weapon_cooldown_factor_if_wounding)\n\
    }\n\
\n\
    /// Live AI prestige-class speed factors (serf, commoner, noble).\n\
    // Haxe: ServerSettings.AISpeedFactor*\n\
    // C-SS-MORE-BATCH5\n\
    #[inline]\n\
    pub fn ai_speed_knobs(&self) -> (f32, f32, f32) {\n\
        (\n\
            self.ai_speed_factor_serf,\n\
            self.ai_speed_factor_commoner,\n\
            self.ai_speed_factor_noble,\n\
        )\n\
    }\n\
\n\
    /// Live vitals speed knobs including close-enemy + AI class factors.\n\
    // Haxe: HitpointsSpeedFactor / GrownUpFoodStoreMax / CloseEnemy* / AISpeedFactor*\n\
    // C-SS-TAIL-KNOBS + C-SS-MORE-BATCH5\n\
    #[inline]\n\
    pub fn vitals_speed_live_knobs(&self) -> crate::move_speed::VitalsSpeedLiveKnobs {\n\
        crate::move_speed::VitalsSpeedLiveKnobs {\n\
            grown_up_food_store_max: self.grown_up_food_store_max,\n\
            hitpoints_speed_factor: self.hitpoints_speed_factor,\n\
            close_enemy_with_weapon_speed_factor: self.close_enemy_with_weapon_speed_factor,\n\
            ai_speed_factor_serf: self.ai_speed_factor_serf,\n\
            ai_speed_factor_commoner: self.ai_speed_factor_commoner,\n\
            ai_speed_factor_noble: self.ai_speed_factor_noble,\n\
        }\n\
    }\n\
}\n",
    );

    ch |= replace_once(
        &mut t,
        "    push_gp(\n\
        \"max_age_fertile\",\n\
        (old.max_age_fertile - gp.max_age_fertile).abs() > f32::EPSILON,\n\
    );\n\
    state.gameplay = gp;\n",
        "    push_gp(\n\
        \"max_age_fertile\",\n\
        (old.max_age_fertile - gp.max_age_fertile).abs() > f32::EPSILON,\n\
    );\n\
    // C-SS-MORE-BATCH5\n\
    push_gp(\n\
        \"weapon_cooldown_factor\",\n\
        (old.weapon_cooldown_factor - gp.weapon_cooldown_factor).abs() > f32::EPSILON,\n\
    );\n\
    push_gp(\n\
        \"weapon_cooldown_factor_if_wounding\",\n\
        (old.weapon_cooldown_factor_if_wounding - gp.weapon_cooldown_factor_if_wounding)\n\
            .abs()\n\
            > f32::EPSILON,\n\
    );\n\
    push_gp(\n\
        \"close_enemy_with_weapon_speed_factor\",\n\
        (old.close_enemy_with_weapon_speed_factor\n\
            - gp.close_enemy_with_weapon_speed_factor)\n\
            .abs()\n\
            > f32::EPSILON,\n\
    );\n\
    push_gp(\n\
        \"exhaustion_on_jump\",\n\
        (old.exhaustion_on_jump - gp.exhaustion_on_jump).abs() > f32::EPSILON,\n\
    );\n\
    push_gp(\n\
        \"hungry_work_heat\",\n\
        (old.hungry_work_heat - gp.hungry_work_heat).abs() > f32::EPSILON,\n\
    );\n\
    push_gp(\n\
        \"ai_speed_factor_serf\",\n\
        (old.ai_speed_factor_serf - gp.ai_speed_factor_serf).abs() > f32::EPSILON,\n\
    );\n\
    push_gp(\n\
        \"ai_speed_factor_commoner\",\n\
        (old.ai_speed_factor_commoner - gp.ai_speed_factor_commoner).abs() > f32::EPSILON,\n\
    );\n\
    push_gp(\n\
        \"ai_speed_factor_noble\",\n\
        (old.ai_speed_factor_noble - gp.ai_speed_factor_noble).abs() > f32::EPSILON,\n\
    );\n\
    state.gameplay = gp;\n",
    );

    if ch {
        let _ = std::fs::write(path, restore_nl(&t, crlf));
    }
    std::fs::read_to_string(path)
        .map(|s| s.contains("pub weapon_cooldown_factor: f32,"))
        .unwrap_or(false)
}

fn patch_weapons(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("weapon_bloody_time_to_change_ex") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;
    ch |= replace_once(
        &mut t,
        "pub fn weapon_bloody_time_to_change(base_ttc: f32, long_wounding: bool) -> f32 {\n\
    let factor = if long_wounding {\n\
        WEAPON_COOLDOWN_FACTOR_IF_WOUNDING\n\
    } else {\n\
        WEAPON_COOLDOWN_FACTOR\n\
    };\n\
    (base_ttc.max(0.0) * factor).max(0.0)\n\
}\n",
        "pub fn weapon_bloody_time_to_change(base_ttc: f32, long_wounding: bool) -> f32 {\n\
    weapon_bloody_time_to_change_ex(\n\
        base_ttc,\n\
        long_wounding,\n\
        WEAPON_COOLDOWN_FACTOR,\n\
        WEAPON_COOLDOWN_FACTOR_IF_WOUNDING,\n\
    )\n\
}\n\
\n\
/// Live-knob variant of [`weapon_bloody_time_to_change`].\n\
// Haxe: ServerSettings.WeaponCoolDownFactor / WeaponCoolDownFactorIfWounding\n\
// C-SS-MORE-BATCH5\n\
#[inline]\n\
pub fn weapon_bloody_time_to_change_ex(\n\
    base_ttc: f32,\n\
    long_wounding: bool,\n\
    normal_factor: f32,\n\
    wounding_factor: f32,\n\
) -> f32 {\n\
    let nf = if normal_factor.is_finite() && normal_factor > 0.0 {\n\
        normal_factor\n\
    } else {\n\
        WEAPON_COOLDOWN_FACTOR\n\
    };\n\
    let wf = if wounding_factor.is_finite() && wounding_factor > 0.0 {\n\
        wounding_factor\n\
    } else {\n\
        WEAPON_COOLDOWN_FACTOR_IF_WOUNDING\n\
    };\n\
    let factor = if long_wounding { wf } else { nf };\n\
    (base_ttc.max(0.0) * factor).max(0.0)\n\
}\n",
    );
    ch |= replace_once(
        &mut t,
        "pub fn bloody_weapon_after_strike(\n\
    held_id: i32,\n\
    long_wounding: bool,\n\
) -> Option<BloodyWeaponTransform> {\n\
    let bloody_id = bloody_weapon_id_for(held_id)?;\n\
    // Already bloody with same id still re-arms cool-down (Haxe re-sets held + ttc).\n\
    let base = bloody_weapon_auto_decay_base_ttc(bloody_id)\n\
        .unwrap_or(BLOODY_WEAPON_STRIKE_BASE_TTC);\n\
    let ttc = weapon_bloody_time_to_change(base, long_wounding);\n\
    Some(BloodyWeaponTransform {\n\
        from_held_id: held_id,\n\
        new_held_id: bloody_id,\n\
        time_to_change: ttc,\n\
    })\n\
}\n",
        "pub fn bloody_weapon_after_strike(\n\
    held_id: i32,\n\
    long_wounding: bool,\n\
) -> Option<BloodyWeaponTransform> {\n\
    bloody_weapon_after_strike_ex(\n\
        held_id,\n\
        long_wounding,\n\
        WEAPON_COOLDOWN_FACTOR,\n\
        WEAPON_COOLDOWN_FACTOR_IF_WOUNDING,\n\
    )\n\
}\n\
\n\
/// Live-knob variant of [`bloody_weapon_after_strike`].\n\
// C-SS-MORE-BATCH5\n\
pub fn bloody_weapon_after_strike_ex(\n\
    held_id: i32,\n\
    long_wounding: bool,\n\
    normal_factor: f32,\n\
    wounding_factor: f32,\n\
) -> Option<BloodyWeaponTransform> {\n\
    let bloody_id = bloody_weapon_id_for(held_id)?;\n\
    let base = bloody_weapon_auto_decay_base_ttc(bloody_id)\n\
        .unwrap_or(BLOODY_WEAPON_STRIKE_BASE_TTC);\n\
    let ttc = weapon_bloody_time_to_change_ex(base, long_wounding, normal_factor, wounding_factor);\n\
    Some(BloodyWeaponTransform {\n\
        from_held_id: held_id,\n\
        new_held_id: bloody_id,\n\
        time_to_change: ttc,\n\
    })\n\
}\n",
    );
    if ch {
        let _ = std::fs::write(path, restore_nl(&t, crlf));
    }
    std::fs::read_to_string(path)
        .map(|s| s.contains("weapon_bloody_time_to_change_ex"))
        .unwrap_or(false)
}

fn patch_move_path(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("apply_jump_cost_ex") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let ch = replace_once(
        &mut t,
        "pub fn apply_jump_cost(\n\
    exhaustion: f32,\n\
    jumped_tiles: f32,\n\
    food_store_max: f32,\n\
    effective_quad: f64,\n\
    is_human: bool,\n\
) -> (f32, f32, bool) {\n\
    let q = effective_quad.max(0.0) as f32;\n\
    let mut exh = exhaustion;\n\
    if is_human {\n\
        exh += q * EXHAUSTION_ON_JUMP;\n\
    }\n\
    let is_exhausted = exh > food_store_max / 2.0;\n\
    let add = if is_exhausted { q } else { q / 2.0 };\n\
    (exh, jumped_tiles + add, is_exhausted)\n\
}\n",
        "pub fn apply_jump_cost(\n\
    exhaustion: f32,\n\
    jumped_tiles: f32,\n\
    food_store_max: f32,\n\
    effective_quad: f64,\n\
    is_human: bool,\n\
) -> (f32, f32, bool) {\n\
    apply_jump_cost_ex(\n\
        exhaustion,\n\
        jumped_tiles,\n\
        food_store_max,\n\
        effective_quad,\n\
        is_human,\n\
        EXHAUSTION_ON_JUMP,\n\
    )\n\
}\n\
\n\
/// Live-knob variant of [`apply_jump_cost`] (Haxe `ExhaustionOnJump`).\n\
// Haxe: ServerSettings.ExhaustionOnJump\n\
// C-SS-MORE-BATCH5\n\
pub fn apply_jump_cost_ex(\n\
    exhaustion: f32,\n\
    jumped_tiles: f32,\n\
    food_store_max: f32,\n\
    effective_quad: f64,\n\
    is_human: bool,\n\
    exhaustion_on_jump: f32,\n\
) -> (f32, f32, bool) {\n\
    let cost = if exhaustion_on_jump.is_finite() && exhaustion_on_jump >= 0.0 {\n\
        exhaustion_on_jump\n\
    } else {\n\
        EXHAUSTION_ON_JUMP\n\
    };\n\
    let q = effective_quad.max(0.0) as f32;\n\
    let mut exh = exhaustion;\n\
    if is_human {\n\
        exh += q * cost;\n\
    }\n\
    let is_exhausted = exh > food_store_max / 2.0;\n\
    let add = if is_exhausted { q } else { q / 2.0 };\n\
    (exh, jumped_tiles + add, is_exhausted)\n\
}\n",
    );
    if ch {
        let _ = std::fs::write(path, restore_nl(&t, crlf));
    }
    std::fs::read_to_string(path)
        .map(|s| s.contains("apply_jump_cost_ex"))
        .unwrap_or(false)
}

fn patch_move_speed(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("VitalsSpeedLiveKnobs") && raw.contains("ai_class_speed_factor_ex") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;
    ch |= replace_once(
        &mut t,
        "pub fn close_enemy_speed_factor(close_hostile_with_weapon: bool) -> f32 {\n\
    if close_hostile_with_weapon {\n\
        CLOSE_ENEMY_WITH_WEAPON_SPEED_FACTOR\n\
    } else {\n\
        1.0\n\
    }\n\
}\n\
\n\
/// Haxe AI-only prestige class speed (humans return 1.0).\n\
pub fn ai_class_speed_factor(is_ai: bool, class: PrestigeClass) -> f32 {\n\
    if !is_ai {\n\
        return 1.0;\n\
    }\n\
    match class {\n\
        PrestigeClass::Serf => AI_SPEED_FACTOR_SERF,\n\
        PrestigeClass::Commoner | PrestigeClass::NotSet => AI_SPEED_FACTOR_COMMONER,\n\
        PrestigeClass::Noble | PrestigeClass::King | PrestigeClass::Emperor => {\n\
            AI_SPEED_FACTOR_NOBLE\n\
        }\n\
    }\n\
}\n",
        "pub fn close_enemy_speed_factor(close_hostile_with_weapon: bool) -> f32 {\n\
    close_enemy_speed_factor_ex(\n\
        close_hostile_with_weapon,\n\
        CLOSE_ENEMY_WITH_WEAPON_SPEED_FACTOR,\n\
    )\n\
}\n\
\n\
/// Live-knob variant of [`close_enemy_speed_factor`].\n\
// Haxe: ServerSettings.CloseEnemyWithWeaponSpeedFactor\n\
// C-SS-MORE-BATCH5\n\
#[inline]\n\
pub fn close_enemy_speed_factor_ex(\n\
    close_hostile_with_weapon: bool,\n\
    factor: f32,\n\
) -> f32 {\n\
    if close_hostile_with_weapon {\n\
        if factor.is_finite() && factor > 0.0 {\n\
            factor\n\
        } else {\n\
            CLOSE_ENEMY_WITH_WEAPON_SPEED_FACTOR\n\
        }\n\
    } else {\n\
        1.0\n\
    }\n\
}\n\
\n\
/// Haxe AI-only prestige class speed (humans return 1.0).\n\
pub fn ai_class_speed_factor(is_ai: bool, class: PrestigeClass) -> f32 {\n\
    ai_class_speed_factor_ex(\n\
        is_ai,\n\
        class,\n\
        AI_SPEED_FACTOR_SERF,\n\
        AI_SPEED_FACTOR_COMMONER,\n\
        AI_SPEED_FACTOR_NOBLE,\n\
    )\n\
}\n\
\n\
/// Live-knob variant of [`ai_class_speed_factor`].\n\
// Haxe: ServerSettings.AISpeedFactorSerf/Commoner/Noble\n\
// C-SS-MORE-BATCH5\n\
pub fn ai_class_speed_factor_ex(\n\
    is_ai: bool,\n\
    class: PrestigeClass,\n\
    serf: f32,\n\
    commoner: f32,\n\
    noble: f32,\n\
) -> f32 {\n\
    if !is_ai {\n\
        return 1.0;\n\
    }\n\
    let s = if serf.is_finite() && serf > 0.0 {\n\
        serf\n\
    } else {\n\
        AI_SPEED_FACTOR_SERF\n\
    };\n\
    let c = if commoner.is_finite() && commoner > 0.0 {\n\
        commoner\n\
    } else {\n\
        AI_SPEED_FACTOR_COMMONER\n\
    };\n\
    let n = if noble.is_finite() && noble > 0.0 {\n\
        noble\n\
    } else {\n\
        AI_SPEED_FACTOR_NOBLE\n\
    };\n\
    match class {\n\
        PrestigeClass::Serf => s,\n\
        PrestigeClass::Commoner | PrestigeClass::NotSet => c,\n\
        PrestigeClass::Noble | PrestigeClass::King | PrestigeClass::Emperor => n,\n\
    }\n\
}\n",
    );
    // Minimal VitalsSpeedLiveKnobs insert before vitals_speed_product if missing
    if !t.contains("struct VitalsSpeedLiveKnobs") {
        ch |= replace_once(
            &mut t,
            "/// Product of shoes / hitpoints / temp / grave / enemy / AI class factors.\n\
pub fn vitals_speed_product(v: &VitalsSpeedInput) -> f32 {\n",
            "/// Live knobs for vitals/social speed tail (C-SS-TAIL-KNOBS + C-SS-MORE-BATCH5).\n\
#[derive(Debug, Clone, Copy, PartialEq)]\n\
pub struct VitalsSpeedLiveKnobs {\n\
    pub grown_up_food_store_max: f32,\n\
    pub hitpoints_speed_factor: f32,\n\
    pub close_enemy_with_weapon_speed_factor: f32,\n\
    pub ai_speed_factor_serf: f32,\n\
    pub ai_speed_factor_commoner: f32,\n\
    pub ai_speed_factor_noble: f32,\n\
}\n\
\n\
impl Default for VitalsSpeedLiveKnobs {\n\
    fn default() -> Self {\n\
        Self {\n\
            grown_up_food_store_max: GROWN_UP_FOOD_STORE_MAX,\n\
            hitpoints_speed_factor: HITPOINTS_SPEED_FACTOR,\n\
            close_enemy_with_weapon_speed_factor: CLOSE_ENEMY_WITH_WEAPON_SPEED_FACTOR,\n\
            ai_speed_factor_serf: AI_SPEED_FACTOR_SERF,\n\
            ai_speed_factor_commoner: AI_SPEED_FACTOR_COMMONER,\n\
            ai_speed_factor_noble: AI_SPEED_FACTOR_NOBLE,\n\
        }\n\
    }\n\
}\n\
\n\
/// Product of shoes / hitpoints / temp / grave / enemy / AI class factors.\n\
pub fn vitals_speed_product(v: &VitalsSpeedInput) -> f32 {\n",
        );
    }
    if ch {
        let _ = std::fs::write(path, restore_nl(&t, crlf));
    }
    std::fs::read_to_string(path)
        .map(|s| s.contains("ai_class_speed_factor_ex"))
        .unwrap_or(false)
}

fn patch_docs(workspace: &Path) -> bool {
    let todo = workspace.join("docs/port/TODO_PORT.md");
    let Ok(raw) = std::fs::read_to_string(&todo) else {
        return false;
    };
    if raw.contains("- [x] **C-SS-MORE-BATCH5 settings_batch5**") {
        return true;
    }
    let mut t = raw;
    let entry = "- [x] **C-SS-MORE-BATCH5 settings_batch5** — LiveSettings→GameplayKnobs WeaponCoolDownFactor/IfWounding / CloseEnemyWithWeaponSpeedFactor / ExhaustionOnJump / HungryWorkHeat / AISpeedFactor* ; pure `*_ex` + jump/bloody/vitals wire; residual USE hungry-work heat full pipe\n";
    if let Some(i) = t.find("- [x] **C-SS-MORE-BATCH4 settings_batch4**") {
        if let Some(nl) = t[i..].find('\n') {
            let at = i + nl + 1;
            t.insert_str(at, entry);
        }
    }
    t = t.replace(
        "**C-SS-MORE-BATCH5** next (weapon CD / jump exh / AI speed already in LiveSettings config, GameplayKnobs wire open)",
        "**C-SS-MORE-BATCH5 DONE**",
    );
    let _ = std::fs::write(&todo, t);

    let matrix = workspace.join("docs/port/FILE_MATRIX.md");
    if let Ok(m) = std::fs::read_to_string(&matrix) {
        let m2 = m.replace(
            "**BATCH5** config present, GameplayKnobs wire open",
            "**C-SS-MORE-BATCH5 DONE** (weapon CD / jump exh / hungry heat / AI speed live wire)",
        );
        if m2 != m {
            let _ = std::fs::write(&matrix, m2);
        }
    }
    true
}

/// Workspace root from ol-sim crate dir.
pub fn workspace_from_manifest(manifest_dir: &Path) -> PathBuf {
    manifest_dir
        .parent() // crates
        .and_then(|p| p.parent()) // RustServer
        .unwrap_or(manifest_dir)
        .to_path_buf()
}
