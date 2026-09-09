//! Weapon/animal wound **plans** — canonical in **`ol-combat-rules`**.
//! Live HIT apply and [`set_held_wound_ctx_for`] stay here.

pub use ol_combat_rules::{
    animal_zero_cooldown_factor, animal_zero_cooldown_factor_ex, bloody_weapon_from_zero_transition,
    bloody_weapon_from_zero_transition_ex, clamp_biome_love_for_damage, coins_stolen_on_wound,
    content_auto_decay_base_ttc, does_real_damage, effective_wound_factor, fever_time_to_change,
    force_no_coins_on_equip, is_any_wound_description, is_arrow_wound_object, moskito_damage_factor,
    object_damage_bleed_rate, object_wound_factor, plan_animal_zero_residual,
    plan_animal_zero_residual_ex, plan_animal_zero_wound, plan_animal_zero_wound_from_content,
    plan_animal_zero_wound_from_content_ex, plan_mosquito_fever_infect, plan_weapon_zero_wound,
    plan_weapon_zero_wound_from_content, resolve_animal_zero_transition,
    resolve_weapon_zero_transition, resolve_weapon_zero_transition_ref, roll_yellow_fever_infect,
    should_do_wound, take_coins_say_text, wound_object_id_for_bleed, AnimalZeroResidual,
    AnimalZeroWoundPlan, FeverInfectPlan, WeaponZeroTransition, WeaponZeroWoundInput,
    WeaponZeroWoundPlan, WoundVictimAction, ANIMAL_ZERO_DEFAULT_BASE_TTC, ARROW_WOUND_ID,
    ATTACKING_RATTLE_SNAKE_ID, ATTACKING_WILD_BOAR_ID, BITE_WOUND_ID, COINS_ON_WOUNDING_FACTOR,
    DEFAULT_WOUND_FACTOR, FEVER_INFECT_BASE_CHANCE, GROUND_WOUND_TTC, HOG_CUT_ID, MOSKITO_HEALTH_MAX_BONI,
    MOSKITO_HEALTH_MAX_MALI, MOSQUITO_SWARM_ID, NON_REAL_YELLOWFEVER_COUNT_DELTA, RATTLE_SNAKE_ID,
    RATTLE_SNAKE_WOUND_FACTOR, SNAKE_BITE_ID, STABLE_KNIFE_WOUND_ID, WILD_BOAR_ID,
    YELLOW_FEVER_WOUND_ID,
};

use crate::nested_body::SetHeldWoundCtx;
use ol_combat_rules::is_wound_object;
use ol_content::ContentDb;

/// Build [`SetHeldWoundCtx`] for a wound object id from content.
// Haxe: setHeldObject + GetTransition(-1, parent).newTargetID light-wound
pub fn set_held_wound_ctx_for(
    content: &ContentDb,
    wound_id: i32,
    health_factor: f32,
) -> SetHeldWoundCtx {
    let is_wound = is_wound_object(content, wound_id);
    let auto_decay_new_target = content
        .auto_decays
        .get(&wound_id)
        .map(|tr| tr.new_target_id)
        .or_else(|| {
            content
                .find_transition(-1, wound_id)
                .map(|tr| tr.new_target_id)
        });
    let base_ttc = content
        .auto_decays
        .get(&wound_id)
        .map(|tr| tr.auto_decay_seconds.max(0.0))
        .unwrap_or(0.0);
    SetHeldWoundCtx {
        is_wound,
        auto_decay_new_target,
        base_time_to_change: base_ttc,
        health_factor,
    }
}
