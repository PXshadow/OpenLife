//! FEVER-EMOTE — `TimeHelper.UpdateEmotes` fever / priority PE ladder.
//!
//! Haxe emits PE faces every `tick % 30` via priority gates (wound → hard combat →
//! yellow fever → starving), then soft combat + ambient heatStroke/pneumonia with a
//! 9s `lastTimeEmoteSend` rate limit.
//!
//! Chunk: **FEVER-EMOTE** / `fever_pe`

// Haxe: GlobalPlayerInstance.Emote
pub const EMOTE_ANGRY: i32 = 2;
pub const EMOTE_YELLOW_FEVER: i32 = 7;
pub const EMOTE_SHOCK: i32 = 15;
pub const EMOTE_MURDER_FACE: i32 = 16;
pub const EMOTE_PNEUMONIA: i32 = 18;
pub const EMOTE_HEAT_STROKE: i32 = 21;
pub const EMOTE_TERRIFIED: i32 = 27;
pub const EMOTE_STARVING: i32 = 31;

/// Haxe `TimeHelper.UpdateEmotes` hard-combat gate: `angryTime < 2`.
// Haxe: TimeHelper.UpdateEmotes L638
pub const UPDATE_EMOTES_ANGRY_HARD: f32 = 2.0;

/// Haxe ambient PE rate limit (`passedTime < 9`).
// Haxe: TimeHelper.UpdateEmotes L676–677
pub const UPDATE_EMOTES_AMBIENT_MIN_SECS: f32 = 9.0;

/// Haxe `DoTimeStuffForPlayer` cadence: `tick % 30 == 0`.
// Haxe: TimeHelper.DoTimeStuffForPlayer L251
pub const UPDATE_EMOTES_TICK_MOD: u64 = 30;

/// Haxe default `ServerSettings.MinAgeToEat` for starving PE.
// Haxe: TimeHelper.UpdateEmotes L657
pub const UPDATE_EMOTES_MIN_AGE_TO_EAT: f32 = 3.0;

/// Inputs for one `UpdateEmotes` evaluation (pure).
// Haxe: TimeHelper.UpdateEmotes L631–689
#[derive(Debug, Clone, Copy)]
pub struct UpdateEmotesInput {
    /// Haxe `isWounded()` — held wound and not hiddenWound alias.
    pub is_wounded: bool,
    /// Haxe `angryTime`.
    pub angry_time: f32,
    /// Haxe `isHoldingWeapon()`.
    pub holding_weapon: bool,
    /// Haxe: lastPlayerAttackedMe holds weapon and lastAttackedPlayer == me.
    pub attacker_mutual_weapon: bool,
    /// Haxe `hasYellowFever()` (fever.id == 2155).
    pub has_yellow_fever: bool,
    /// Haxe `isSuperHot()` with person-color thresholds.
    pub is_super_hot: bool,
    /// Haxe `isSuperCold()` with person-color thresholds.
    pub is_super_cold: bool,
    /// Haxe `food_store`.
    pub food_store: f32,
    /// Haxe `age`.
    pub age: f32,
    /// Haxe `ServerSettings.MinAgeToEat`.
    pub min_age_to_eat: f32,
    /// Haxe `ServerSettings.CombatAngryTimeBeforeAttack` (soft combat faces).
    pub combat_angry_before_attack: f32,
    /// Seconds since `lastTimeEmoteSend` (ambient rate limit).
    pub secs_since_ambient_emote: f32,
}

/// PE plan from one `UpdateEmotes` call.
// Haxe: TimeHelper.UpdateEmotes
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateEmotesPlan {
    /// Emotes to send in order (priority first, then soft combat, then ambient).
    pub emotes: Vec<i32>,
    /// When true, advance `lastTimeEmoteSend` (ambient gate passed).
    pub stamp_ambient_timer: bool,
}

/// Pure yellow-fever PE index: heatStroke(21) when super-hot, else yellowFever(7).
///
/// Returns `None` when not feverish.
// Haxe: TimeHelper.UpdateEmotes L651–654
// FEVER-EMOTE
pub fn resolve_fever_pe_emote(has_yellow_fever: bool, is_super_hot: bool) -> Option<i32> {
    if !has_yellow_fever {
        return None;
    }
    if is_super_hot {
        Some(EMOTE_HEAT_STROKE)
    } else {
        Some(EMOTE_YELLOW_FEVER)
    }
}

/// Soft / hard combat face when angry (not wounded path).
// Haxe: TimeHelper.UpdateEmotes L638–646 / L662–667
fn resolve_combat_emote(
    holding_weapon: bool,
    attacker_mutual_weapon: bool,
    hard: bool,
) -> i32 {
    if holding_weapon {
        if hard {
            EMOTE_MURDER_FACE
        } else {
            EMOTE_ANGRY
        }
    } else if attacker_mutual_weapon {
        if hard {
            EMOTE_TERRIFIED
        } else {
            EMOTE_SHOCK
        }
    } else {
        EMOTE_ANGRY
    }
}

/// Full Haxe `UpdateEmotes` priority ladder (pure).
///
/// Order: wounded → hard angry (`<2`) → yellow fever → food&lt;0 starving → soft angry
/// (`< CombatAngryTime`) → ambient heatStroke/pneumonia (9s rate limit).
// Haxe: TimeHelper.UpdateEmotes L631–689
// FEVER-EMOTE
pub fn resolve_update_emotes(input: &UpdateEmotesInput) -> UpdateEmotesPlan {
    // 1. Wound shock — early return
    // Haxe: L632–635
    if input.is_wounded {
        return UpdateEmotesPlan {
            emotes: vec![EMOTE_SHOCK],
            stamp_ambient_timer: false,
        };
    }

    // 2. Hard combat faces — early return
    // Haxe: L638–648
    if input.angry_time < UPDATE_EMOTES_ANGRY_HARD {
        let e = resolve_combat_emote(
            input.holding_weapon,
            input.attacker_mutual_weapon,
            true,
        );
        return UpdateEmotesPlan {
            emotes: vec![e],
            stamp_ambient_timer: false,
        };
    }

    // 3. Yellow fever — early return (no 9s rate limit)
    // Haxe: L651–654
    if let Some(e) = resolve_fever_pe_emote(input.has_yellow_fever, input.is_super_hot) {
        return UpdateEmotesPlan {
            emotes: vec![e],
            stamp_ambient_timer: false,
        };
    }

    // 4. Starving PE — early return
    // Haxe: L657–659
    let min_age = if input.min_age_to_eat.is_finite() {
        input.min_age_to_eat
    } else {
        UPDATE_EMOTES_MIN_AGE_TO_EAT
    };
    if input.food_store < 0.0 && input.age.is_finite() && input.age >= min_age {
        return UpdateEmotesPlan {
            emotes: vec![EMOTE_STARVING],
            stamp_ambient_timer: false,
        };
    }

    // 5. Soft combat (no early return — ambient may still fire)
    // Haxe: L662–668
    let mut emotes = Vec::new();
    let soft_thresh = if input.combat_angry_before_attack.is_finite() {
        input.combat_angry_before_attack
    } else {
        crate::move_live_gates::COMBAT_ANGRY_TIME_BEFORE_ATTACK
    };
    if input.angry_time < soft_thresh {
        emotes.push(resolve_combat_emote(
            input.holding_weapon,
            input.attacker_mutual_weapon,
            false,
        ));
    }

    // 6. Ambient heatStroke / pneumonia — 9s rate limit
    // Haxe: L676–681
    let secs = if input.secs_since_ambient_emote.is_finite() {
        input.secs_since_ambient_emote
    } else {
        UPDATE_EMOTES_AMBIENT_MIN_SECS
    };
    if secs < UPDATE_EMOTES_AMBIENT_MIN_SECS {
        return UpdateEmotesPlan {
            emotes,
            stamp_ambient_timer: false,
        };
    }
    let mut stamp = true;
    if input.is_super_hot {
        emotes.push(EMOTE_HEAT_STROKE);
    }
    if input.is_super_cold {
        emotes.push(EMOTE_PNEUMONIA);
    }
    // Stamp even when neither extreme (Haxe always updates lastTimeEmoteSend past gate).
    if emotes.is_empty() {
        stamp = true;
    }
    UpdateEmotesPlan {
        emotes,
        stamp_ambient_timer: stamp,
    }
}

/// Seconds since last ambient emote stamp (`lastTimeEmoteSend`).
///
/// When never stamped (`last <= 0`), treat as large enough to pass the 9s gate.
// Haxe: TimeHelper.CalculateTimeSinceTicksInSec(lastTimeEmoteSend)
#[inline]
pub fn secs_since_ambient_emote(sim_time: f32, last_time_emote_send: f32) -> f32 {
    if !sim_time.is_finite() {
        return UPDATE_EMOTES_AMBIENT_MIN_SECS;
    }
    if !last_time_emote_send.is_finite() || last_time_emote_send <= 0.0 {
        return UPDATE_EMOTES_AMBIENT_MIN_SECS;
    }
    (sim_time - last_time_emote_send).max(0.0)
}

/// True when this sim tick should run `UpdateEmotes` (`tick % 30 == 0`).
// Haxe: TimeHelper.DoTimeStuffForPlayer L251
#[inline]
pub fn should_update_emotes_this_tick(tick: u64) -> bool {
    tick % UPDATE_EMOTES_TICK_MOD == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::move_live_gates::COMBAT_ANGRY_TIME_BEFORE_ATTACK;
    use crate::player_soul::{
        is_super_hot_for_person, PERSON_COLOR_BLACK, PERSON_COLOR_BROWN,
    };

    fn calm_input() -> UpdateEmotesInput {
        UpdateEmotesInput {
            is_wounded: false,
            angry_time: COMBAT_ANGRY_TIME_BEFORE_ATTACK + 1.0,
            holding_weapon: false,
            attacker_mutual_weapon: false,
            has_yellow_fever: false,
            is_super_hot: false,
            is_super_cold: false,
            food_store: 10.0,
            age: 20.0,
            min_age_to_eat: UPDATE_EMOTES_MIN_AGE_TO_EAT,
            combat_angry_before_attack: COMBAT_ANGRY_TIME_BEFORE_ATTACK,
            secs_since_ambient_emote: 0.0,
        }
    }

    #[test]
    fn resolve_fever_pe_yellow_and_heatstroke() {
        // yf + !super_hot → 7
        assert_eq!(
            resolve_fever_pe_emote(true, false),
            Some(EMOTE_YELLOW_FEVER)
        );
        // yf + super_hot → 21
        assert_eq!(
            resolve_fever_pe_emote(true, true),
            Some(EMOTE_HEAT_STROKE)
        );
        assert_eq!(resolve_fever_pe_emote(false, true), None);
        assert_eq!(resolve_fever_pe_emote(false, false), None);
    }

    #[test]
    fn black_person_color_needs_higher_heat_for_heatstroke() {
        // Neutral: super hot above 0.8
        assert!(is_super_hot_for_person(0.81, 0));
        assert!(!is_super_hot_for_person(0.8, 0));
        // Black: tooHot = 0.9
        assert!(!is_super_hot_for_person(0.89, PERSON_COLOR_BLACK));
        assert!(is_super_hot_for_person(0.91, PERSON_COLOR_BLACK));
        // Brown: 0.85
        assert!(!is_super_hot_for_person(0.85, PERSON_COLOR_BROWN));
        assert!(is_super_hot_for_person(0.86, PERSON_COLOR_BROWN));
        // Ladder uses person-color flag
        let mut inp = calm_input();
        inp.has_yellow_fever = true;
        inp.is_super_hot = is_super_hot_for_person(0.85, PERSON_COLOR_BLACK);
        assert_eq!(
            resolve_update_emotes(&inp).emotes,
            vec![EMOTE_YELLOW_FEVER]
        );
        inp.is_super_hot = is_super_hot_for_person(0.91, PERSON_COLOR_BLACK);
        assert_eq!(
            resolve_update_emotes(&inp).emotes,
            vec![EMOTE_HEAT_STROKE]
        );
    }

    #[test]
    fn wounded_and_hard_angry_outrank_fever() {
        let mut inp = calm_input();
        inp.has_yellow_fever = true;
        inp.is_wounded = true;
        assert_eq!(resolve_update_emotes(&inp).emotes, vec![EMOTE_SHOCK]);

        inp.is_wounded = false;
        inp.angry_time = 1.5;
        inp.holding_weapon = true;
        assert_eq!(
            resolve_update_emotes(&inp).emotes,
            vec![EMOTE_MURDER_FACE]
        );

        inp.holding_weapon = false;
        inp.attacker_mutual_weapon = true;
        assert_eq!(
            resolve_update_emotes(&inp).emotes,
            vec![EMOTE_TERRIFIED]
        );

        inp.attacker_mutual_weapon = false;
        assert_eq!(resolve_update_emotes(&inp).emotes, vec![EMOTE_ANGRY]);
    }

    #[test]
    fn fever_outranks_starving_and_soft_combat() {
        let mut inp = calm_input();
        inp.has_yellow_fever = true;
        inp.food_store = -1.0;
        inp.angry_time = 3.0; // soft combat band
        assert_eq!(
            resolve_update_emotes(&inp).emotes,
            vec![EMOTE_YELLOW_FEVER]
        );
        assert!(!resolve_update_emotes(&inp).stamp_ambient_timer);
    }

    #[test]
    fn starving_when_food_negative() {
        let mut inp = calm_input();
        inp.food_store = -0.1;
        assert_eq!(
            resolve_update_emotes(&inp).emotes,
            vec![EMOTE_STARVING]
        );
        // Under min age: no starving PE
        inp.age = 2.0;
        assert!(resolve_update_emotes(&inp).emotes.is_empty());
    }

    #[test]
    fn soft_combat_and_ambient_can_both_fire() {
        let mut inp = calm_input();
        inp.angry_time = 3.0;
        inp.holding_weapon = true;
        inp.is_super_hot = true;
        inp.secs_since_ambient_emote = 10.0;
        let plan = resolve_update_emotes(&inp);
        assert_eq!(plan.emotes, vec![EMOTE_ANGRY, EMOTE_HEAT_STROKE]);
        assert!(plan.stamp_ambient_timer);
    }

    #[test]
    fn ambient_rate_limit_blocks_heat_faces() {
        let mut inp = calm_input();
        inp.is_super_hot = true;
        inp.is_super_cold = true;
        inp.secs_since_ambient_emote = 8.9;
        let plan = resolve_update_emotes(&inp);
        assert!(plan.emotes.is_empty());
        assert!(!plan.stamp_ambient_timer);

        inp.secs_since_ambient_emote = 9.0;
        let plan2 = resolve_update_emotes(&inp);
        assert_eq!(
            plan2.emotes,
            vec![EMOTE_HEAT_STROKE, EMOTE_PNEUMONIA]
        );
        assert!(plan2.stamp_ambient_timer);
    }

    #[test]
    fn ambient_stamps_even_without_extreme_heat() {
        let mut inp = calm_input();
        inp.secs_since_ambient_emote = 9.0;
        let plan = resolve_update_emotes(&inp);
        assert!(plan.emotes.is_empty());
        assert!(plan.stamp_ambient_timer);
    }

    #[test]
    fn tick_mod_and_secs_since() {
        assert!(should_update_emotes_this_tick(0));
        assert!(should_update_emotes_this_tick(30));
        assert!(should_update_emotes_this_tick(60));
        assert!(!should_update_emotes_this_tick(1));
        assert!(!should_update_emotes_this_tick(29));

        assert!((secs_since_ambient_emote(10.0, 0.0) - 9.0).abs() < 1e-5);
        assert!((secs_since_ambient_emote(20.0, 10.0) - 10.0).abs() < 1e-5);
        assert!((secs_since_ambient_emote(12.0, 10.0) - 2.0).abs() < 1e-5);
    }
}
