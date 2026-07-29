//! Haxe `GlobalPlayerInstance.DoDamage` **weapon+0** / **animal+0** wound path.
//!
//! Chunk **WEAPON-WOUND-TRANS** / `weapon_zero`:
//! - `GetTransition(weapon, 0, lastUseActor=true)` then non-LA
//! - `woundFactor` gate vs `food_store_max` / not-reduced max
//! - equip `newTargetID` wound on held, or ground-place when arrow wound / !doWound
//! - attacker held → content `newActorID` (bloody) + cool-down TTC
//!
//! Chunk **WEAPON-ANIMAL-ZERO** / `animal_wound_zero`:
//! - Same `GetTransition(animal, 0)` + doWound equip/ground as weapon path
//! - `attacker == null` → `fromObj.id = trans.newActorID` (attacking form residual)
//! - cool-down TTC on animal via `-1` auto-decay × WeaponCoolDownFactor*
//! - `takeCoins` skipped (no human attacker)
//! - Animal retaliate bloody (Haxe commented out) — skipped
//!
//! Chunk **WALLET-COINS** / `take_coins`:
//! - pure [`coins_stolen_on_wound`] + [`take_coins_say_text`]
//! - live wallet gift path on lethal + first wound equip (human attacker)

use crate::death_polish::{
    is_arrow_wound_description, is_arrow_wound_object, is_wound_description, is_wound_object,
};
use crate::weapons::{
    bloody_weapon_after_strike, bloody_weapon_after_strike_ex, bloody_weapon_auto_decay_base_ttc,
    weapon_bloody_time_to_change, weapon_bloody_time_to_change_ex,
    BloodyWeaponTransform, BLOODY_WEAPON_STRIKE_BASE_TTC, WEAPON_COOLDOWN_FACTOR,
    WEAPON_COOLDOWN_FACTOR_IF_WOUNDING,
};
use ol_content::{ContentDb, Transition};

/// Haxe default `ObjectData.woundFactor`.
pub const DEFAULT_WOUND_FACTOR: f32 = 0.5;
/// Haxe Rattle Snake parent id (shoes protect).
pub const RATTLE_SNAKE_ID: i32 = 764;
/// Haxe patched snake woundFactor.
pub const RATTLE_SNAKE_WOUND_FACTOR: f32 = 0.98;
/// Haxe Mosquito Swarm — non-real damage; always try wound/fever path.
pub const MOSQUITO_SWARM_ID: i32 = 2156;
/// Haxe Yellow Fever wound object (fever NestedHelper id).
pub const YELLOW_FEVER_WOUND_ID: i32 = 2155;
/// Haxe non-real hit resistance bump: `yellowfeverCount += 0.02`.
// Haxe: DoDamage doesRealDamage==false yellowfeverCount L4654
pub const NON_REAL_YELLOWFEVER_COUNT_DELTA: f32 = 0.02;
/// Haxe fever infect base: `0.2 * pow(moskitoDamageFactor, 2)`.
// Haxe: DoDamage moskito fever roll L4743
pub const FEVER_INFECT_BASE_CHANCE: f32 = 0.2;
/// Haxe `CalculateHealthFactor(2, 0.5)` maxBoni/maxMali for mosquito path.
// Haxe: DoDamage healthFactor = CalculateHealthFactor(2, 0.5)
pub const MOSKITO_HEALTH_MAX_BONI: f32 = 2.0;
pub const MOSKITO_HEALTH_MAX_MALI: f32 = 0.5;
/// Haxe ground-place wound `timeToChange = 2` when already arrow wound or !doWound.
pub const GROUND_WOUND_TTC: f32 = 2.0;
/// Haxe `ServerSettings.CoinsOnWoundingFactor` default (live via GameplayKnobs).
// Haxe: ServerSettings.CoinsOnWoundingFactor = 0.5
// WALLET-COINS — prefer `state.gameplay.coins_on_wounding_factor` on live paths
pub const COINS_ON_WOUNDING_FACTOR: f32 = 0.5;
/// Common knife wound / arrow wound object ids (content + patches).
pub const STABLE_KNIFE_WOUND_ID: i32 = 797;
pub const ARROW_WOUND_ID: i32 = 798;
pub const SNAKE_BITE_ID: i32 = 1377;
/// Haxe Hog Cut from Wild Boar + 0.
pub const HOG_CUT_ID: i32 = 1364;
/// Haxe Bite Wound (grizzly / shot wolf + 0).
pub const BITE_WOUND_ID: i32 = 1363;
/// Wild Boar content id (AnimalKind::Boar.object_id).
pub const WILD_BOAR_ID: i32 = 1323;
/// Attacking Wild Boar (1323+0 newActor).
pub const ATTACKING_WILD_BOAR_ID: i32 = 1333;
/// Attacking Rattle Snake (764+0 newActor).
pub const ATTACKING_RATTLE_SNAKE_ID: i32 = 1385;
/// Default base TTC when animal newActor has no `-1` auto-decay.
pub const ANIMAL_ZERO_DEFAULT_BASE_TTC: f32 = 1.0;

// ── Description / content wound flags ───────────────────────────────────────

/// Re-export style: wound description (any Wound / Snake Bite / Hog Cut).
#[inline]
pub fn is_any_wound_description(description: &str) -> bool {
    is_wound_description(description)
}

// ── woundFactor / doWound gate ──────────────────────────────────────────────

/// Resolve base woundFactor from content, with hard-coded snake fallback.
// Haxe: fromObj.objectData.woundFactor (+ ServerSettings patch 764 → 0.98)
pub fn object_wound_factor(content: &ContentDb, weapon_id: i32) -> f32 {
    if weapon_id == 0 {
        return DEFAULT_WOUND_FACTOR;
    }
    if let Some(d) = content.objects.get(&weapon_id) {
        // Content may still be default 0.5 when unpatched; snake parent needs 0.98.
        if weapon_id == RATTLE_SNAKE_ID || content.resolve_base_id(weapon_id) == RATTLE_SNAKE_ID {
            if (d.wound_factor - DEFAULT_WOUND_FACTOR).abs() < 1e-6 {
                return RATTLE_SNAKE_WOUND_FACTOR;
            }
        }
        return d.wound_factor;
    }
    if weapon_id == RATTLE_SNAKE_ID {
        return RATTLE_SNAKE_WOUND_FACTOR;
    }
    DEFAULT_WOUND_FACTOR
}

/// Haxe: snake + both shoes → `woundFactor /= 1.5`.
// Haxe: DoDamage fromObj.parentId == 764 && hasBothShoes
#[inline]
pub fn effective_wound_factor(
    base_wound_factor: f32,
    weapon_parent_id: i32,
    has_both_shoes: bool,
) -> f32 {
    let mut wf = base_wound_factor;
    if weapon_parent_id == RATTLE_SNAKE_ID && has_both_shoes {
        wf /= 1.5;
    }
    wf
}

/// Haxe: `doWound = food_store_max < maxFoodStore * woundFactor`.
/// Non-real damage (mosquito) forces `doWound = true` (TODO random chance — port-as-is).
// Haxe: DoDamage doWound gate L4717–4719
#[inline]
pub fn should_do_wound(
    food_store_max: f32,
    not_reduced_max: f32,
    wound_factor: f32,
    real_damage: bool,
) -> bool {
    if !real_damage {
        return true;
    }
    let threshold = not_reduced_max * wound_factor;
    food_store_max < threshold
}

/// Haxe `doesRealDamage = fromObj.parentId != 2156`.
// Haxe: DoDamage doesRealDamage mosquito
#[inline]
pub fn does_real_damage(weapon_parent_id: i32) -> bool {
    weapon_parent_id != MOSQUITO_SWARM_ID
}

// ── Mosquito / yellow fever pure ────────────────────────────────────────────

/// Clamp biome love for damage factors (Haxe `if (loves < -0.5) loves = -0.5`).
// Haxe: DoDamage lovesJungle clamp L4648
#[inline]
pub fn clamp_biome_love_for_damage(love: f32) -> f32 {
    let l = if love.is_finite() { love } else { 0.0 };
    if l < -0.5 {
        -0.5
    } else {
        l
    }
}

/// Haxe `moskitoDamageFactor = 1/(1+lovesJungle+yellowfeverCount) / healthFactor`.
///
/// Without yellowfeverCount, range is roughly 0.33..2 for love in [-0.5, 2] and hf=1.
// Haxe: GlobalPlayerInstance.DoDamage moskitoDamageFactor L4647–4652
pub fn moskito_damage_factor(
    loves_jungle: f32,
    yellowfever_count: f32,
    health_factor: f32,
) -> f32 {
    let loves = clamp_biome_love_for_damage(loves_jungle);
    let yf = if yellowfever_count.is_finite() {
        yellowfever_count.max(0.0)
    } else {
        0.0
    };
    let hf = if health_factor.is_finite() && health_factor.abs() > 1e-6 {
        health_factor
    } else {
        1.0
    };
    let base = 1.0 / (1.0 + loves + yf);
    base / hf
}

/// Haxe fever infect: `0.2 * pow(moskitoDamageFactor, 2) > rng`.
// Haxe: DoDamage mosquito fever roll L4743
#[inline]
pub fn roll_yellow_fever_infect(moskito_factor: f32, rng01: f32) -> bool {
    let f = if moskito_factor.is_finite() {
        moskito_factor.max(0.0)
    } else {
        0.0
    };
    let chance = FEVER_INFECT_BASE_CHANCE * f * f;
    let r = if rng01.is_finite() {
        rng01.clamp(0.0, 1.0)
    } else {
        1.0
    };
    chance > r
}

/// Fever NestedHelper TTC: `CalculateTimeToChangeForObj(wound) * moskitoDamageFactor`.
// Haxe: DoDamage newWound.timeToChange L4747
#[inline]
pub fn fever_time_to_change(base_ttc: f32, moskito_factor: f32) -> f32 {
    let base = if base_ttc.is_finite() {
        base_ttc.max(0.0)
    } else {
        0.0
    };
    let f = if moskito_factor.is_finite() {
        moskito_factor.max(0.0)
    } else {
        1.0
    };
    base * f
}

/// Outcome of pure mosquito fever-infect branch (after non-real +0.02 already applied).
// Haxe: DoDamage mosquito fever L4741–4752
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeverInfectPlan {
    /// Whether fever NestedHelper is set this hit.
    pub infected: bool,
    /// yellowfeverCount after infect (+1) or unchanged (still has non-real +0.02 from caller).
    pub yellowfever_delta_on_infect: f32,
    pub fever_wound_id: i32,
    pub fever_ttc: f32,
    /// Always true for mosquito candidate path (Haxe doEmote sad).
    pub emote_sad: bool,
}

/// Pure plan: roll fever infect + TTC for MosquitoFeverCandidate.
// Haxe: DoDamage mosquito fever branch L4741–4752
pub fn plan_mosquito_fever_infect(
    wound_id: i32,
    moskito_factor: f32,
    base_ttc: f32,
    rng01: f32,
) -> FeverInfectPlan {
    let infected = roll_yellow_fever_infect(moskito_factor, rng01);
    if infected {
        FeverInfectPlan {
            infected: true,
            yellowfever_delta_on_infect: 1.0,
            fever_wound_id: if wound_id != 0 {
                wound_id
            } else {
                YELLOW_FEVER_WOUND_ID
            },
            fever_ttc: fever_time_to_change(base_ttc, moskito_factor),
            emote_sad: true,
        }
    } else {
        FeverInfectPlan {
            infected: false,
            yellowfever_delta_on_infect: 0.0,
            fever_wound_id: 0,
            fever_ttc: 0.0,
            emote_sad: true,
        }
    }
}

/// Resolve which NestedHelper id bleeds (held wound if wounded, else hiddenWound).
// Haxe: TimeHelper.updateFoodAndDoHealing isWounded ? heldObject : hiddenWound L875–876
#[inline]
pub fn wound_object_id_for_bleed(
    held_is_wound: bool,
    held_id: i32,
    hidden_wound_id: Option<i32>,
) -> Option<i32> {
    if held_is_wound && held_id != 0 {
        Some(held_id)
    } else {
        hidden_wound_id.filter(|&id| id != 0)
    }
}

/// Bleed DPS rate from `ObjectDef.damage` (before `WoundDamageFactor` × dt).
// Haxe: wound.objectData.damage * WoundDamageFactor L877
#[inline]
pub fn object_damage_bleed_rate(object_damage: f32) -> f32 {
    if object_damage.is_finite() {
        object_damage.max(0.0)
    } else {
        0.0
    }
}

// ── (weapon, 0) transition resolve ──────────────────────────────────────────

/// Outcome of Haxe `GetTransition(weapon, 0, lastUseActor)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeaponZeroTransition {
    pub new_actor_id: i32,
    pub new_target_id: i32,
    pub prefer_last_use: bool,
}

/// Haxe `TransitionImporter.GetTransition(fromObj.id, 0, true, false)` then non-LA.
// Haxe: DoDamage GetTransition(weapon, 0, lastUseActor)
pub fn resolve_weapon_zero_transition(
    content: &ContentDb,
    weapon_id: i32,
) -> Option<WeaponZeroTransition> {
    if weapon_id == 0 {
        return None;
    }
    // Prefer last-use actor table (Haxe lastUseActor=true first).
    if let Some(tr) = content.find_transition_prefer(weapon_id, 0, true) {
        return Some(WeaponZeroTransition {
            new_actor_id: tr.new_actor_id,
            new_target_id: tr.new_target_id,
            prefer_last_use: tr.last_use_actor || content.find_transition_last_use(weapon_id, 0).is_some(),
        });
    }
    None
}

/// Same as [`resolve_weapon_zero_transition`] returning raw [`Transition`] ref fields.
pub fn resolve_weapon_zero_transition_ref(
    content: &ContentDb,
    weapon_id: i32,
) -> Option<&Transition> {
    if weapon_id == 0 {
        return None;
    }
    content.find_transition_prefer(weapon_id, 0, true)
}

// ── Pure wound application plan ─────────────────────────────────────────────

/// How the victim receives the wound object from `newTargetID`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WoundVictimAction {
    /// Equip wound as held (Haxe real-damage first wound / non-arrow).
    EquipHeld {
        wound_id: i32,
        /// Prior held id to PlaceObject (0 = none).
        drop_prior_held: i32,
        /// Drop carried baby when true (Haxe `heldPlayer != null` → dropPlayer).
        drop_held_baby: bool,
        /// Haxe takeCoins on first real wound equip.
        take_coins: bool,
    },
    /// Place wound on ground with short ttc (arrow already held or !doWound).
    GroundPlace {
        wound_id: i32,
        time_to_change: f32,
        allow_replace: bool,
    },
    /// Mosquito non-real path: fever candidate only (caller may roll yellow fever).
    MosquitoFeverCandidate { wound_id: i32 },
    /// No victim-side object action (no transition or new_target 0).
    None,
}

/// Full pure plan for one DoDamage wound-object branch (after damage pipe).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponZeroWoundPlan {
    pub has_transition: bool,
    pub new_actor_id: i32,
    pub new_target_id: i32,
    pub do_wound: bool,
    /// Haxe `longWeaponCoolDown` (lethal, or first object-wound).
    pub long_weapon_cooldown: bool,
    pub victim_action: WoundVictimAction,
}

/// Inputs for [`plan_weapon_zero_wound`].
#[derive(Debug, Clone, Copy)]
pub struct WeaponZeroWoundInput {
    pub weapon_id: i32,
    pub weapon_parent_id: i32,
    pub new_actor_id: i32,
    pub new_target_id: i32,
    pub has_transition: bool,
    pub food_store_max: f32,
    pub not_reduced_max: f32,
    pub wound_factor: f32,
    pub real_damage: bool,
    /// Haxe `isWounded()` before equipping this wound (held is wound, not light-hidden).
    pub already_wounded: bool,
    /// Haxe `heldObject.isArrowWound()`.
    pub held_is_arrow_wound: bool,
    pub prior_held_id: i32,
    pub holding_player_id: i32,
    /// Haxe `food_store_max < 0` after damage pipe.
    pub combat_lethal: bool,
}

impl Default for WeaponZeroWoundInput {
    fn default() -> Self {
        Self {
            weapon_id: 0,
            weapon_parent_id: 0,
            new_actor_id: 0,
            new_target_id: 0,
            has_transition: false,
            food_store_max: 20.0,
            not_reduced_max: 20.0,
            wound_factor: DEFAULT_WOUND_FACTOR,
            real_damage: true,
            already_wounded: false,
            held_is_arrow_wound: false,
            prior_held_id: 0,
            holding_player_id: 0,
            combat_lethal: false,
        }
    }
}

/// Pure DoDamage wound-object branch (no world mutation).
// Haxe: GlobalPlayerInstance.DoDamage L4702–4788
pub fn plan_weapon_zero_wound(inp: WeaponZeroWoundInput) -> WeaponZeroWoundPlan {
    if !inp.has_transition {
        return WeaponZeroWoundPlan {
            has_transition: false,
            new_actor_id: 0,
            new_target_id: 0,
            do_wound: false,
            long_weapon_cooldown: inp.combat_lethal,
            victim_action: WoundVictimAction::None,
        };
    }

    let do_wound = should_do_wound(
        inp.food_store_max,
        inp.not_reduced_max,
        inp.wound_factor,
        inp.real_damage,
    );

    let mut long_cd = inp.combat_lethal;
    if do_wound && !inp.already_wounded {
        long_cd = true;
    }

    let victim_action = if do_wound && !inp.held_is_arrow_wound {
        if inp.real_damage {
            WoundVictimAction::EquipHeld {
                wound_id: inp.new_target_id,
                drop_prior_held: inp.prior_held_id,
                drop_held_baby: inp.holding_player_id != 0,
                take_coins: true,
            }
        } else {
            // Mosquito: fever path (no held equip of wound as cargo).
            WoundVictimAction::MosquitoFeverCandidate {
                wound_id: inp.new_target_id,
            }
        }
    } else {
        // !doWound or already arrow wound → ground place with ttc=2, allowReplace.
        if inp.new_target_id != 0 {
            WoundVictimAction::GroundPlace {
                wound_id: inp.new_target_id,
                time_to_change: GROUND_WOUND_TTC,
                allow_replace: true,
            }
        } else {
            WoundVictimAction::None
        }
    };

    WeaponZeroWoundPlan {
        has_transition: true,
        new_actor_id: inp.new_actor_id,
        new_target_id: inp.new_target_id,
        do_wound,
        long_weapon_cooldown: long_cd,
        victim_action,
    }
}

/// Build plan from content lookup + live player flags.
// Haxe: DoDamage GetTransition + doWound + equip/ground
pub fn plan_weapon_zero_wound_from_content(
    content: &ContentDb,
    weapon_id: i32,
    weapon_parent_id: i32,
    food_store_max: f32,
    not_reduced_max: f32,
    has_both_shoes: bool,
    already_wounded: bool,
    held_is_arrow_wound: bool,
    prior_held_id: i32,
    holding_player_id: i32,
    combat_lethal: bool,
) -> WeaponZeroWoundPlan {
    let real = does_real_damage(weapon_parent_id);
    let base_wf = object_wound_factor(content, weapon_id);
    let wf = effective_wound_factor(base_wf, weapon_parent_id, has_both_shoes);
    let tr = resolve_weapon_zero_transition(content, weapon_id);
    let (has_tr, na, nt) = match tr {
        Some(t) => (true, t.new_actor_id, t.new_target_id),
        None => (false, 0, 0),
    };
    plan_weapon_zero_wound(WeaponZeroWoundInput {
        weapon_id,
        weapon_parent_id,
        new_actor_id: na,
        new_target_id: nt,
        has_transition: has_tr,
        food_store_max,
        not_reduced_max,
        wound_factor: wf,
        real_damage: real,
        already_wounded,
        held_is_arrow_wound,
        prior_held_id,
        holding_player_id,
        combat_lethal,
    })
}

// ── Animal+0 residual (attacker == null) ────────────────────────────────────

/// Haxe animal residual after DoDamage wound branch: `fromObj.id = newActor` + TTC.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimalZeroResidual {
    /// Animal object id before transform.
    pub from_object_id: i32,
    /// `trans.newActorID` (attacking form / same id).
    pub new_object_id: i32,
    /// Haxe `fromObj.timeToChange` after `-1` auto-decay × cool-down factor.
    pub time_to_change: f32,
    /// True when map object id should change.
    pub transforms: bool,
    /// Whether a (animal,0) transition existed.
    pub has_transition: bool,
}

impl Default for AnimalZeroResidual {
    fn default() -> Self {
        Self {
            from_object_id: 0,
            new_object_id: 0,
            time_to_change: 0.0,
            transforms: false,
            has_transition: false,
        }
    }
}

/// Full pure outcome for animal DoDamage wound path (victim + animal residual).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimalZeroWoundPlan {
    /// Shared victim equip/ground plan (`take_coins` forced false).
    pub victim: WeaponZeroWoundPlan,
    /// Animal object transform + cool-down TTC.
    pub residual: AnimalZeroResidual,
}

/// Resolve `(animal_id, 0)` the same way as weapon+0 (LA prefer then non-LA).
// Haxe: DoDamage GetTransition(fromObj.id, 0, true) then false — "animal" comment
#[inline]
pub fn resolve_animal_zero_transition(
    content: &ContentDb,
    animal_object_id: i32,
) -> Option<WeaponZeroTransition> {
    resolve_weapon_zero_transition(content, animal_object_id)
}

/// Cool-down factor for animal residual TTC (Haxe uses weapon factors; AnimalCoolDown commented).
// Haxe: WeaponCoolDownFactor / WeaponCoolDownFactorIfWounding (AnimalCoolDownFactorIfWounding dead)
#[inline]
pub fn animal_zero_cooldown_factor(long_wounding: bool) -> f32 {
    animal_zero_cooldown_factor_ex(
        long_wounding,
        WEAPON_COOLDOWN_FACTOR,
        WEAPON_COOLDOWN_FACTOR_IF_WOUNDING,
    )
}

/// Live-knob variant of [`animal_zero_cooldown_factor`].
// C-SS-MORE-BATCH5
#[inline]
pub fn animal_zero_cooldown_factor_ex(
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
    if long_wounding {
        wf
    } else {
        nf
    }
}

/// Pure animal residual from transition newActor + optional content base TTC.
// Haxe: DoDamage L4765–4788 attacker==null branch
pub fn plan_animal_zero_residual(
    from_object_id: i32,
    new_actor_id: i32,
    has_transition: bool,
    base_ttc: Option<f32>,
    long_wounding: bool,
) -> AnimalZeroResidual {
    plan_animal_zero_residual_ex(
        from_object_id,
        new_actor_id,
        has_transition,
        base_ttc,
        long_wounding,
        WEAPON_COOLDOWN_FACTOR,
        WEAPON_COOLDOWN_FACTOR_IF_WOUNDING,
    )
}

/// Live-knob variant of [`plan_animal_zero_residual`].
// Haxe: DoDamage L4765–4788 × ServerSettings.WeaponCoolDownFactor*
// C-SS-MORE-BATCH5
pub fn plan_animal_zero_residual_ex(
    from_object_id: i32,
    new_actor_id: i32,
    has_transition: bool,
    base_ttc: Option<f32>,
    long_wounding: bool,
    normal_factor: f32,
    wounding_factor: f32,
) -> AnimalZeroResidual {
    if !has_transition {
        return AnimalZeroResidual {
            from_object_id,
            new_object_id: from_object_id,
            time_to_change: 0.0,
            transforms: false,
            has_transition: false,
        };
    }
    // Haxe always assigns fromObj.id = trans.newActorID when transition exists.
    let new_id = if new_actor_id != 0 {
        new_actor_id
    } else {
        from_object_id
    };
    let base = base_ttc
        .filter(|s| *s > 0.0)
        .unwrap_or(ANIMAL_ZERO_DEFAULT_BASE_TTC);
    let ttc = base
        * animal_zero_cooldown_factor_ex(long_wounding, normal_factor, wounding_factor);
    AnimalZeroResidual {
        from_object_id,
        new_object_id: new_id,
        time_to_change: ttc,
        transforms: new_id != from_object_id,
        has_transition: true,
    }
}

/// Strip coin theft from equip action (Haxe `takeCoins` returns when attacker null).
// Haxe: takeCoins if (attacker == null) return
pub fn force_no_coins_on_equip(action: WoundVictimAction) -> WoundVictimAction {
    match action {
        WoundVictimAction::EquipHeld {
            wound_id,
            drop_prior_held,
            drop_held_baby,
            take_coins: _,
        } => WoundVictimAction::EquipHeld {
            wound_id,
            drop_prior_held,
            drop_held_baby,
            take_coins: false,
        },
        other => other,
    }
}

/// Pure DoDamage animal+0 wound plan (victim + residual). No world mutation.
// Haxe: GlobalPlayerInstance.DoDamage when attacker == null L4702–4788
pub fn plan_animal_zero_wound(inp: WeaponZeroWoundInput) -> AnimalZeroWoundPlan {
    let mut victim = plan_weapon_zero_wound(inp);
    victim.victim_action = force_no_coins_on_equip(victim.victim_action);
    let residual = plan_animal_zero_residual(
        inp.weapon_id,
        victim.new_actor_id,
        victim.has_transition,
        None, // caller fills via from_content when base TTC known
        victim.long_weapon_cooldown,
    );
    AnimalZeroWoundPlan { victim, residual }
}

/// Content-aware animal+0 plan: resolve transition, doWound gate, residual TTC.
// Haxe: DoDamage GetTransition(animal,0) + animal residual
pub fn plan_animal_zero_wound_from_content(
    content: &ContentDb,
    animal_object_id: i32,
    animal_parent_id: i32,
    food_store_max: f32,
    not_reduced_max: f32,
    has_both_shoes: bool,
    already_wounded: bool,
    held_is_arrow_wound: bool,
    prior_held_id: i32,
    holding_player_id: i32,
    combat_lethal: bool,
) -> AnimalZeroWoundPlan {
    plan_animal_zero_wound_from_content_ex(
        content,
        animal_object_id,
        animal_parent_id,
        food_store_max,
        not_reduced_max,
        has_both_shoes,
        already_wounded,
        held_is_arrow_wound,
        prior_held_id,
        holding_player_id,
        combat_lethal,
        WEAPON_COOLDOWN_FACTOR,
        WEAPON_COOLDOWN_FACTOR_IF_WOUNDING,
    )
}

/// Live-knob residual TTC for animal+0 content plan.
// Haxe: DoDamage animal residual × WeaponCoolDownFactor*
// C-SS-MORE-BATCH5
pub fn plan_animal_zero_wound_from_content_ex(
    content: &ContentDb,
    animal_object_id: i32,
    animal_parent_id: i32,
    food_store_max: f32,
    not_reduced_max: f32,
    has_both_shoes: bool,
    already_wounded: bool,
    held_is_arrow_wound: bool,
    prior_held_id: i32,
    holding_player_id: i32,
    combat_lethal: bool,
    normal_factor: f32,
    wounding_factor: f32,
) -> AnimalZeroWoundPlan {
    let real = does_real_damage(animal_parent_id);
    let base_wf = object_wound_factor(content, animal_object_id);
    let wf = effective_wound_factor(base_wf, animal_parent_id, has_both_shoes);
    let tr = resolve_animal_zero_transition(content, animal_object_id);
    let (has_tr, na, nt) = match tr {
        Some(t) => (true, t.new_actor_id, t.new_target_id),
        None => (false, 0, 0),
    };
    let mut victim = plan_weapon_zero_wound(WeaponZeroWoundInput {
        weapon_id: animal_object_id,
        weapon_parent_id: animal_parent_id,
        new_actor_id: na,
        new_target_id: nt,
        has_transition: has_tr,
        food_store_max,
        not_reduced_max,
        wound_factor: wf,
        real_damage: real,
        already_wounded,
        held_is_arrow_wound,
        prior_held_id,
        holding_player_id,
        combat_lethal,
    });
    victim.victim_action = force_no_coins_on_equip(victim.victim_action);
    let base_ttc = if has_tr {
        content_auto_decay_base_ttc(content, na)
    } else {
        None
    };
    let residual = plan_animal_zero_residual_ex(
        animal_object_id,
        na,
        has_tr,
        base_ttc,
        victim.long_weapon_cooldown,
        normal_factor,
        wounding_factor,
    );
    AnimalZeroWoundPlan { victim, residual }
}

// ── Bloody attacker held from content newActor ──────────────────────────────

/// Prefer content `trans.newActorID` for bloody held; fall back to hard-coded table.
///
/// Base TTC: content `-1` auto-decay when provided, else patched bloody table / 2.
// Haxe: DoDamage fromObj.id = trans.newActorID + GetTransition(-1, newActor) * factor
pub fn bloody_weapon_from_zero_transition(
    held_id: i32,
    content_new_actor: i32,
    content_base_ttc: Option<f32>,
    long_wounding: bool,
) -> Option<BloodyWeaponTransform> {
    bloody_weapon_from_zero_transition_ex(
        held_id,
        content_new_actor,
        content_base_ttc,
        long_wounding,
        WEAPON_COOLDOWN_FACTOR,
        WEAPON_COOLDOWN_FACTOR_IF_WOUNDING,
    )
}

/// Live-knob variant of [`bloody_weapon_from_zero_transition`].
// C-SS-MORE-BATCH5
pub fn bloody_weapon_from_zero_transition_ex(
    held_id: i32,
    content_new_actor: i32,
    content_base_ttc: Option<f32>,
    long_wounding: bool,
    normal_factor: f32,
    wounding_factor: f32,
) -> Option<BloodyWeaponTransform> {
    // Haxe always sets fromObj.id = trans.newActorID when transition exists.
    if content_new_actor != 0 && content_new_actor != held_id {
        let base = content_base_ttc
            .or_else(|| bloody_weapon_auto_decay_base_ttc(content_new_actor))
            .unwrap_or(BLOODY_WEAPON_STRIKE_BASE_TTC);
        let ttc = weapon_bloody_time_to_change_ex(
            base,
            long_wounding,
            normal_factor,
            wounding_factor,
        );
        return Some(BloodyWeaponTransform {
            from_held_id: held_id,
            new_held_id: content_new_actor,
            time_to_change: ttc,
        });
    }
    if content_new_actor != 0 && content_new_actor == held_id {
        // Already new actor (e.g. already bloody) — re-arm cool-down.
        let base = content_base_ttc
            .or_else(|| bloody_weapon_auto_decay_base_ttc(content_new_actor))
            .unwrap_or(BLOODY_WEAPON_STRIKE_BASE_TTC);
        let ttc = weapon_bloody_time_to_change_ex(
            base,
            long_wounding,
            normal_factor,
            wounding_factor,
        );
        return Some(BloodyWeaponTransform {
            from_held_id: held_id,
            new_held_id: content_new_actor,
            time_to_change: ttc,
        });
    }
    // No useful content actor — hard-coded knife/sword/bow table.
    bloody_weapon_after_strike_ex(held_id, long_wounding, normal_factor, wounding_factor)
}

/// Look up `-1` auto-decay base seconds for new actor (content).
// Haxe: TransitionImporter.GetTransition(-1, trans.newActorID).calculateTimeToChange
pub fn content_auto_decay_base_ttc(content: &ContentDb, object_id: i32) -> Option<f32> {
    if object_id == 0 {
        return None;
    }
    content
        .auto_decays
        .get(&object_id)
        .map(|tr| tr.auto_decay_seconds)
        .filter(|&s| s > 0.0)
        .or_else(|| bloody_weapon_auto_decay_base_ttc(object_id))
}

// ── takeCoins pure ──────────────────────────────────────────────────────────

/// Haxe `takeCoins` amount stolen on wound/kill (integer coins).
// Haxe: GlobalPlayerInstance.takeCoins
pub fn coins_stolen_on_wound(target_coins: f32, factor: f32, dark_nosaj: bool) -> i32 {
    let mut f = factor;
    if dark_nosaj {
        f *= 2.0;
    }
    if f > 1.0 {
        f = 1.0;
    }
    let target_i = target_coins.floor() as i32;
    if target_i < 1 {
        return 0;
    }
    let mut coins = (target_coins * f).floor() as i32 + 1;
    if coins > target_i {
        coins = target_i;
    }
    if coins < 1 {
        0
    } else {
        coins
    }
}

/// Haxe attacker.say after successful takeCoins (`Got N coin!` / `Got N coins!`).
// Haxe: GlobalPlayerInstance.takeCoins L4838–4844
// WALLET-COINS
pub fn take_coins_say_text(amount: i32) -> Option<String> {
    if amount < 1 {
        None
    } else if amount == 1 {
        Some(format!("Got {amount} coin!"))
    } else {
        Some(format!("Got {amount} coins!"))
    }
}

// ── setHeld light-wound ctx helper ──────────────────────────────────────────

/// Build [`crate::nested_body::SetHeldWoundCtx`] for a wound object id from content.
// Haxe: setHeldObject + GetTransition(-1, parent).newTargetID light-wound
pub fn set_held_wound_ctx_for(
    content: &ContentDb,
    wound_id: i32,
    health_factor: f32,
) -> crate::nested_body::SetHeldWoundCtx {
    let is_wound = is_wound_object(content, wound_id);
    let auto_decay_new_target = content
        .auto_decays
        .get(&wound_id)
        .map(|tr| tr.new_target_id)
        .or_else(|| {
            // Some light wounds use alternativeTimeOutcome=0 via patches; check -1 table.
            content
                .find_transition(-1, wound_id)
                .map(|tr| tr.new_target_id)
        });
    let base_ttc = content
        .auto_decays
        .get(&wound_id)
        .map(|tr| tr.auto_decay_seconds.max(0.0))
        .unwrap_or(0.0);
    crate::nested_body::SetHeldWoundCtx {
        is_wound,
        auto_decay_new_target,
        base_time_to_change: base_ttc,
        health_factor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::{ObjectDef, Transition};

    fn bare_tr(a: i32, t: i32, na: i32, nt: i32) -> Transition {
        Transition {
            actor_id: a,
            target_id: t,
            new_actor_id: na,
            new_target_id: nt,
            last_use_actor: false,
            last_use_target: false,
            auto_decay_seconds: 0.0,
            reverse_use_actor: false,
            reverse_use_target: false,
            no_use_actor: false,
            no_use_target: false,
            move_dist: 0,
            desired_move_dist: 0,
            actor_min_use_fraction: 0.0,
            target_min_use_fraction: 0.0,
            switch_number_of_uses: false,
            target_number_of_uses: -1,
            is_pickup_or_drop: false,
        }
    }

    #[test]
    fn is_arrow_wound_vs_is_wound() {
        assert!(is_arrow_wound_description("Arrow Wound"));
        assert!(is_arrow_wound_description("Deep Arrow Wound"));
        assert!(!is_arrow_wound_description("Knife Wound"));
        assert!(!is_arrow_wound_description("Snake Bite"));
        assert!(is_any_wound_description("Arrow Wound"));
        assert!(is_any_wound_description("Snake Bite"));
        assert!(is_any_wound_description("Hog Cut"));
        assert!(!is_any_wound_description("Gooseberry"));
    }

    #[test]
    fn do_wound_threshold_default_half() {
        // food_max 9 < 20*0.5 → wound
        assert!(should_do_wound(9.0, 20.0, 0.5, true));
        // food_max 10 == 10 → not wound (strict <)
        assert!(!should_do_wound(10.0, 20.0, 0.5, true));
        // food_max 11 > 10 → not wound
        assert!(!should_do_wound(11.0, 20.0, 0.5, true));
        // mosquito always
        assert!(should_do_wound(20.0, 20.0, 0.5, false));
    }

    #[test]
    fn snake_shoes_shift_threshold() {
        let base = RATTLE_SNAKE_WOUND_FACTOR;
        let with_shoes = effective_wound_factor(base, RATTLE_SNAKE_ID, true);
        assert!((with_shoes - base / 1.5).abs() < 1e-5);
        // Without shoes threshold ~19.6; with shoes ~13.07
        assert!(should_do_wound(15.0, 20.0, base, true)); // 15 < 19.6
        assert!(!should_do_wound(15.0, 20.0, with_shoes, true)); // 15 > 13.07
        assert!(should_do_wound(12.0, 20.0, with_shoes, true));
        // Non-snake shoes no-op
        assert!(
            (effective_wound_factor(0.5, 560, true) - 0.5).abs() < 1e-6
        );
    }

    #[test]
    fn resolve_weapon_zero_prefer_last_use() {
        let mut db = ContentDb::default();
        db.objects.insert(560, ObjectDef::empty(560));
        db.objects.insert(750, ObjectDef::empty(750));
        db.objects.insert(797, {
            let mut o = ObjectDef::empty(797);
            o.description = "Stable Knife Wound".into();
            o
        });
        // Normal: 560+0 → 750, 797
        db.transitions
            .insert((560, 0), bare_tr(560, 0, 750, 797));
        let t = resolve_weapon_zero_transition(&db, 560).unwrap();
        assert_eq!(t.new_actor_id, 750);
        assert_eq!(t.new_target_id, 797);

        // Last-use wins when present
        let mut la = bare_tr(560, 0, 750, 3816);
        la.last_use_actor = true;
        db.transitions_last_use.insert((560, 0), la);
        let t2 = resolve_weapon_zero_transition(&db, 560).unwrap();
        assert_eq!(t2.new_target_id, 3816);

        assert!(resolve_weapon_zero_transition(&db, 999).is_none());
        assert!(resolve_weapon_zero_transition(&db, 0).is_none());
    }

    #[test]
    fn plan_first_wound_equips_held() {
        let plan = plan_weapon_zero_wound(WeaponZeroWoundInput {
            has_transition: true,
            new_actor_id: 750,
            new_target_id: 797,
            food_store_max: 5.0,
            not_reduced_max: 20.0,
            wound_factor: 0.5,
            real_damage: true,
            already_wounded: false,
            held_is_arrow_wound: false,
            prior_held_id: 33,
            holding_player_id: 0,
            combat_lethal: false,
            ..Default::default()
        });
        assert!(plan.do_wound);
        assert!(plan.long_weapon_cooldown);
        assert_eq!(
            plan.victim_action,
            WoundVictimAction::EquipHeld {
                wound_id: 797,
                drop_prior_held: 33,
                drop_held_baby: false,
                take_coins: true,
            }
        );
        assert_eq!(plan.new_actor_id, 750);
    }

    #[test]
    fn plan_already_arrow_wound_grounds() {
        let plan = plan_weapon_zero_wound(WeaponZeroWoundInput {
            has_transition: true,
            new_actor_id: 151,
            new_target_id: 798,
            food_store_max: 5.0,
            not_reduced_max: 20.0,
            wound_factor: 0.5,
            real_damage: true,
            already_wounded: true,
            held_is_arrow_wound: true,
            prior_held_id: 798,
            holding_player_id: 0,
            combat_lethal: false,
            ..Default::default()
        });
        assert!(plan.do_wound);
        // already wounded → not long cool-down from first-wound
        assert!(!plan.long_weapon_cooldown);
        assert_eq!(
            plan.victim_action,
            WoundVictimAction::GroundPlace {
                wound_id: 798,
                time_to_change: GROUND_WOUND_TTC,
                allow_replace: true,
            }
        );
    }

    #[test]
    fn plan_no_wound_above_threshold_grounds() {
        let plan = plan_weapon_zero_wound(WeaponZeroWoundInput {
            has_transition: true,
            new_actor_id: 750,
            new_target_id: 797,
            food_store_max: 15.0,
            not_reduced_max: 20.0,
            wound_factor: 0.5,
            real_damage: true,
            already_wounded: false,
            held_is_arrow_wound: false,
            prior_held_id: 0,
            holding_player_id: 0,
            combat_lethal: false,
            ..Default::default()
        });
        assert!(!plan.do_wound);
        assert!(!plan.long_weapon_cooldown);
        assert_eq!(
            plan.victim_action,
            WoundVictimAction::GroundPlace {
                wound_id: 797,
                time_to_change: GROUND_WOUND_TTC,
                allow_replace: true,
            }
        );
    }

    #[test]
    fn plan_no_transition_damage_only() {
        let plan = plan_weapon_zero_wound(WeaponZeroWoundInput {
            has_transition: false,
            food_store_max: 5.0,
            combat_lethal: true,
            ..Default::default()
        });
        assert!(!plan.has_transition);
        assert!(plan.long_weapon_cooldown); // lethal still
        assert_eq!(plan.victim_action, WoundVictimAction::None);
    }

    #[test]
    fn plan_drop_baby_on_equip() {
        let plan = plan_weapon_zero_wound(WeaponZeroWoundInput {
            has_transition: true,
            new_actor_id: 750,
            new_target_id: 797,
            food_store_max: 5.0,
            not_reduced_max: 20.0,
            wound_factor: 0.5,
            real_damage: true,
            prior_held_id: 0,
            holding_player_id: 42,
            ..Default::default()
        });
        match plan.victim_action {
            WoundVictimAction::EquipHeld {
                drop_held_baby: true,
                ..
            } => {}
            other => panic!("expected drop baby, got {other:?}"),
        }
    }

    #[test]
    fn bloody_from_content_new_actor() {
        // Bow 152 → 151 (yew bow) content newActor (Haxe 152→151 patch family).
        let xf = bloody_weapon_from_zero_transition(152, 151, Some(6.0), false).unwrap();
        assert_eq!(xf.new_held_id, 151);
        assert!((xf.time_to_change - 3.0).abs() < 1e-5); // 6 * 0.5
        // Knife content 750
        let k = bloody_weapon_from_zero_transition(560, 750, Some(3.0), true).unwrap();
        assert_eq!(k.new_held_id, 750);
        assert!((k.time_to_change - 15.0).abs() < 1e-5); // 3 * 5
        // No content actor → hard-coded table
        let fb = bloody_weapon_from_zero_transition(560, 0, None, false).unwrap();
        assert_eq!(fb.new_held_id, 750);
    }

    #[test]
    fn coins_on_wound_factor() {
        assert_eq!(coins_stolen_on_wound(10.0, 0.5, false), 6); // floor(5)+1
        assert_eq!(coins_stolen_on_wound(1.0, 0.5, false), 1);
        assert_eq!(coins_stolen_on_wound(0.0, 0.5, false), 0);
        assert_eq!(coins_stolen_on_wound(10.0, 0.5, true), 10); // factor*2 capped, floor(10)+1→cap
        // Fractional residual: floor(coins*factor)+1 capped to floor(target).
        assert_eq!(coins_stolen_on_wound(2.7, 0.5, false), 2);
        assert_eq!(coins_stolen_on_wound(0.9, 0.5, false), 0); // floor < 1
        assert_eq!(coins_stolen_on_wound(10.0, 1.5, false), 10); // factor clamp
        assert_eq!(take_coins_say_text(0), None);
        assert_eq!(take_coins_say_text(1).as_deref(), Some("Got 1 coin!"));
        assert_eq!(take_coins_say_text(6).as_deref(), Some("Got 6 coins!"));
    }

    #[test]
    fn plan_from_content_integration() {
        let mut db = ContentDb::default();
        db.objects.insert(560, {
            let mut o = ObjectDef::empty(560);
            o.damage = 5.0;
            o.wound_factor = 0.5;
            o
        });
        db.objects.insert(750, ObjectDef::empty(750));
        db.objects.insert(797, {
            let mut o = ObjectDef::empty(797);
            o.description = "Stable Knife Wound".into();
            o.damage = 0.05;
            o
        });
        db.transitions
            .insert((560, 0), bare_tr(560, 0, 750, 797));
        let plan = plan_weapon_zero_wound_from_content(
            &db, 560, 560, 5.0, 20.0, false, false, false, 33, 0, false,
        );
        assert!(plan.do_wound);
        assert!(plan.long_weapon_cooldown);
        assert_eq!(plan.new_actor_id, 750);
        assert_eq!(plan.new_target_id, 797);
        match plan.victim_action {
            WoundVictimAction::EquipHeld {
                wound_id: 797,
                drop_prior_held: 33,
                ..
            } => {}
            other => panic!("{other:?}"),
        }
        assert!(is_arrow_wound_object(&db, 0) == false);
        assert!(!is_arrow_wound_object(&db, 797));
        db.objects.insert(798, {
            let mut o = ObjectDef::empty(798);
            o.description = "Arrow Wound".into();
            o
        });
        assert!(is_arrow_wound_object(&db, 798));
    }

    // ── WEAPON-ANIMAL-ZERO / animal_wound_zero ───────────────────────────────

    #[test]
    fn animal_zero_boar_equips_hog_cut_and_transforms() {
        let mut db = ContentDb::default();
        db.objects.insert(WILD_BOAR_ID, {
            let mut o = ObjectDef::empty(WILD_BOAR_ID);
            o.damage = 3.0;
            o.wound_factor = 0.5;
            o
        });
        db.objects.insert(ATTACKING_WILD_BOAR_ID, ObjectDef::empty(ATTACKING_WILD_BOAR_ID));
        db.objects.insert(HOG_CUT_ID, {
            let mut o = ObjectDef::empty(HOG_CUT_ID);
            o.description = "Hog Cut".into();
            o.damage = 0.05;
            o
        });
        // 1323+0 → 1333 Attacking Wild Boar, 1364 Hog Cut
        db.transitions.insert(
            (WILD_BOAR_ID, 0),
            bare_tr(WILD_BOAR_ID, 0, ATTACKING_WILD_BOAR_ID, HOG_CUT_ID),
        );
        // -1 + 1333 → back to 1323 in 1s (content)
        db.auto_decays.insert(
            ATTACKING_WILD_BOAR_ID,
            bare_tr(-1, ATTACKING_WILD_BOAR_ID, 0, WILD_BOAR_ID),
        );
        // Fix auto_decay_seconds on that transition
        if let Some(tr) = db.auto_decays.get_mut(&ATTACKING_WILD_BOAR_ID) {
            tr.auto_decay_seconds = 1.0;
        }

        let plan = plan_animal_zero_wound_from_content(
            &db,
            WILD_BOAR_ID,
            WILD_BOAR_ID,
            5.0,
            20.0,
            false,
            false,
            false,
            33,
            0,
            false,
        );
        assert!(plan.victim.has_transition);
        assert!(plan.victim.do_wound);
        assert!(plan.victim.long_weapon_cooldown);
        assert_eq!(plan.victim.new_actor_id, ATTACKING_WILD_BOAR_ID);
        assert_eq!(plan.victim.new_target_id, HOG_CUT_ID);
        match plan.victim.victim_action {
            WoundVictimAction::EquipHeld {
                wound_id: HOG_CUT_ID,
                drop_prior_held: 33,
                take_coins: false, // animal: no takeCoins
                ..
            } => {}
            other => panic!("expected hog cut equip no coins, got {other:?}"),
        }
        assert!(plan.residual.transforms);
        assert_eq!(plan.residual.new_object_id, ATTACKING_WILD_BOAR_ID);
        // base 1.0 * wounding factor 5.0
        assert!((plan.residual.time_to_change - 5.0).abs() < 1e-5);
    }

    #[test]
    fn animal_zero_snake_shoes_and_no_transition_wolf() {
        let mut db = ContentDb::default();
        db.objects.insert(RATTLE_SNAKE_ID, {
            let mut o = ObjectDef::empty(RATTLE_SNAKE_ID);
            o.damage = 2.0;
            o.wound_factor = RATTLE_SNAKE_WOUND_FACTOR;
            o
        });
        db.objects
            .insert(ATTACKING_RATTLE_SNAKE_ID, ObjectDef::empty(ATTACKING_RATTLE_SNAKE_ID));
        db.objects.insert(SNAKE_BITE_ID, {
            let mut o = ObjectDef::empty(SNAKE_BITE_ID);
            o.description = "Snake Bite".into();
            o
        });
        db.transitions.insert(
            (RATTLE_SNAKE_ID, 0),
            bare_tr(RATTLE_SNAKE_ID, 0, ATTACKING_RATTLE_SNAKE_ID, SNAKE_BITE_ID),
        );
        db.auto_decays.insert(
            ATTACKING_RATTLE_SNAKE_ID,
            {
                let mut t = bare_tr(-1, ATTACKING_RATTLE_SNAKE_ID, 0, RATTLE_SNAKE_ID);
                t.auto_decay_seconds = 1.0;
                t
            },
        );

        // food_max 15: without shoes 15 < 19.6 → wound; with shoes 15 > 13.07 → ground
        let no_shoes = plan_animal_zero_wound_from_content(
            &db, RATTLE_SNAKE_ID, RATTLE_SNAKE_ID, 15.0, 20.0, false, false, false, 0, 0, false,
        );
        assert!(no_shoes.victim.do_wound);
        let with_shoes = plan_animal_zero_wound_from_content(
            &db, RATTLE_SNAKE_ID, RATTLE_SNAKE_ID, 15.0, 20.0, true, false, false, 0, 0, false,
        );
        assert!(!with_shoes.victim.do_wound);
        match with_shoes.victim.victim_action {
            WoundVictimAction::GroundPlace {
                wound_id: SNAKE_BITE_ID,
                time_to_change: GROUND_WOUND_TTC,
                ..
            } => {}
            other => panic!("expected ground snake bite, got {other:?}"),
        }
        // Residual still transforms animal even when !doWound
        assert!(with_shoes.residual.transforms);
        assert_eq!(with_shoes.residual.new_object_id, ATTACKING_RATTLE_SNAKE_ID);

        // Wolf 418 has no content +0 — damage only
        db.objects.insert(418, {
            let mut o = ObjectDef::empty(418);
            o.damage = 3.0;
            o
        });
        let wolf = plan_animal_zero_wound_from_content(
            &db, 418, 418, 5.0, 20.0, false, false, false, 0, 0, false,
        );
        assert!(!wolf.victim.has_transition);
        assert!(!wolf.residual.has_transition);
        assert!(!wolf.residual.transforms);
        assert_eq!(wolf.victim.victim_action, WoundVictimAction::None);
    }

    #[test]
    fn animal_zero_residual_cooldown_factors() {
        let short = plan_animal_zero_residual(1323, 1333, true, Some(1.0), false);
        assert!((short.time_to_change - 0.5).abs() < 1e-5); // 1 * 0.5
        assert!(short.transforms);
        let long = plan_animal_zero_residual(1323, 1333, true, Some(1.0), true);
        assert!((long.time_to_change - 5.0).abs() < 1e-5); // 1 * 5
        let none = plan_animal_zero_residual(418, 0, false, None, true);
        assert!(!none.has_transition);
        assert!(!none.transforms);
        assert_eq!(none.new_object_id, 418);
    }

    #[test]
    fn animal_zero_residual_live_cooldown_factors() {
        // C-SS-MORE-BATCH5: live WeaponCoolDownFactor* on residual TTC
        let short = plan_animal_zero_residual_ex(1323, 1333, true, Some(1.0), false, 0.25, 4.0);
        assert!((short.time_to_change - 0.25).abs() < 1e-5);
        let long = plan_animal_zero_residual_ex(1323, 1333, true, Some(1.0), true, 0.25, 4.0);
        assert!((long.time_to_change - 4.0).abs() < 1e-5);
        // Non-finite / non-positive factors fall back to module defaults
        let fb = plan_animal_zero_residual_ex(1323, 1333, true, Some(2.0), true, f32::NAN, -1.0);
        assert!((fb.time_to_change - 10.0).abs() < 1e-5); // 2 * 5 default wounding
    }

    // ── COMBAT-FEVER-BLEED / fever pure ─────────────────────────────────────

    #[test]
    fn does_real_damage_mosquito_only() {
        assert!(!does_real_damage(MOSQUITO_SWARM_ID));
        assert!(does_real_damage(560));
        assert!(does_real_damage(WILD_BOAR_ID));
        assert!(does_real_damage(0));
    }

    #[test]
    fn moskito_factor_jungle_and_resistance() {
        // Neutral love, no resistance, hf=1 → 1.0
        assert!((moskito_damage_factor(0.0, 0.0, 1.0) - 1.0).abs() < 1e-5);
        // lovesJungle clamped -0.5 → 1/(1-0.5)=2
        assert!((moskito_damage_factor(-1.0, 0.0, 1.0) - 2.0).abs() < 1e-5);
        // yellowfeverCount 1 → 1/2
        assert!((moskito_damage_factor(0.0, 1.0, 1.0) - 0.5).abs() < 1e-5);
        // health_factor 2 halves factor
        assert!((moskito_damage_factor(0.0, 0.0, 2.0) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn fever_infect_roll_vs_factor_squared() {
        // factor 1 → chance 0.2
        assert!(roll_yellow_fever_infect(1.0, 0.19));
        assert!(!roll_yellow_fever_infect(1.0, 0.20));
        assert!(!roll_yellow_fever_infect(1.0, 0.5));
        // factor 0.5 → chance 0.05
        assert!(roll_yellow_fever_infect(0.5, 0.049));
        assert!(!roll_yellow_fever_infect(0.5, 0.05));
        // factor 0 → never
        assert!(!roll_yellow_fever_infect(0.0, 0.0));
    }

    #[test]
    fn fever_ttc_scales_by_factor() {
        assert!((fever_time_to_change(100.0, 0.5) - 50.0).abs() < 1e-5);
        assert!((fever_time_to_change(10.0, 1.0) - 10.0).abs() < 1e-5);
        assert!((fever_time_to_change(10.0, 0.0) - 0.0).abs() < 1e-5);
    }

    #[test]
    fn plan_mosquito_fever_infect_hit_and_miss() {
        let hit = plan_mosquito_fever_infect(YELLOW_FEVER_WOUND_ID, 1.0, 40.0, 0.1);
        assert!(hit.infected);
        assert!((hit.yellowfever_delta_on_infect - 1.0).abs() < 1e-6);
        assert_eq!(hit.fever_wound_id, YELLOW_FEVER_WOUND_ID);
        assert!((hit.fever_ttc - 40.0).abs() < 1e-5);
        assert!(hit.emote_sad);

        let miss = plan_mosquito_fever_infect(YELLOW_FEVER_WOUND_ID, 1.0, 40.0, 0.5);
        assert!(!miss.infected);
        assert_eq!(miss.yellowfever_delta_on_infect, 0.0);
        assert_eq!(miss.fever_wound_id, 0);
        assert!(miss.emote_sad);
    }

    #[test]
    fn plan_mosquito_fever_zero_wound_defaults_2155() {
        let hit = plan_mosquito_fever_infect(0, 1.0, 20.0, 0.0);
        assert!(hit.infected);
        assert_eq!(hit.fever_wound_id, YELLOW_FEVER_WOUND_ID);
    }

    #[test]
    fn wound_bleed_id_prefers_held_wound() {
        assert_eq!(
            wound_object_id_for_bleed(true, 797, Some(798)),
            Some(797)
        );
        assert_eq!(
            wound_object_id_for_bleed(false, 33, Some(797)),
            Some(797)
        );
        assert_eq!(wound_object_id_for_bleed(false, 0, None), None);
        assert_eq!(wound_object_id_for_bleed(true, 0, Some(797)), Some(797));
        assert!((object_damage_bleed_rate(0.05) - 0.05).abs() < 1e-6);
        assert_eq!(object_damage_bleed_rate(-1.0), 0.0);
    }

    #[test]
    fn plan_mosquito_candidate_action() {
        let plan = plan_weapon_zero_wound(WeaponZeroWoundInput {
            has_transition: true,
            new_actor_id: 0,
            new_target_id: YELLOW_FEVER_WOUND_ID,
            food_store_max: 20.0,
            not_reduced_max: 20.0,
            wound_factor: 0.5,
            real_damage: false,
            already_wounded: false,
            held_is_arrow_wound: false,
            prior_held_id: 0,
            holding_player_id: 0,
            combat_lethal: false,
            ..Default::default()
        });
        assert!(plan.do_wound);
        assert_eq!(
            plan.victim_action,
            WoundVictimAction::MosquitoFeverCandidate {
                wound_id: YELLOW_FEVER_WOUND_ID,
            }
        );
    }

    #[test]
    fn force_no_coins_on_animal_equip() {
        let a = force_no_coins_on_equip(WoundVictimAction::EquipHeld {
            wound_id: 1364,
            drop_prior_held: 1,
            drop_held_baby: false,
            take_coins: true,
        });
        match a {
            WoundVictimAction::EquipHeld {
                take_coins: false, ..
            } => {}
            other => panic!("{other:?}"),
        }
    }
}
