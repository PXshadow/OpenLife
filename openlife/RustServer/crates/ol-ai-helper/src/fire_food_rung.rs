//! AI-FIREFOOD-RUNG: assigned/last FIREFOODMAKER â†’ makeFireFood(100) ladder bridge.
//!
//! Haxe: `assignedProfession == 'FIREFOODMAKER' || lastProfession == 'FIREFOODMAKER'`
//! â†’ `makeFireFood(100)` in `AiBase.doTimeStuffHelper` ~754â€“756.

use crate::fire_food_profession::{
    fire_food_max_people_for_dispatch, make_fire_food, FireFoodAction, FireFoodCounts,
    FireFoodProfessionRuntime,
};

/// Ladder rung labels that may run makeFireFood (assigned/last job band).
// Haxe: doTimeStuffHelper FIREFOODMAKER assigned/last ~754â€“756
pub fn fire_food_job_rung_label(rung_label: &str) -> bool {
    matches!(
        rung_label,
        "ASSIGNED_JOB"
            | "AGE_ROTATED_JOB"
            | "LOW_PRIORITY_WORK"
            | "MID_PRIORITY_TASKS"
            | "CRITICAL_MISC"
            | "CRAFT_QUEUE"
            | "CRITICAL_CRAFT"
    )
}

/// Thin ladder bridge: assigned/last FIREFOODMAKER â†’ pure `make_fire_food`.
///
/// `is_assigned_job` or rung `ASSIGNED_JOB` selects maxPeople **100** (Haxe makeFireFood(100)).
// Haxe: assignedProfession == 'FIREFOODMAKER' || lastProfession == 'FIREFOODMAKER' â†’ makeFireFood(100)
pub fn try_decide_fire_food_from_rung(
    profession_is_sticky: bool,
    rung_label: &str,
    is_assigned_job: bool,
    counts: &FireFoodCounts,
    runtime: &mut FireFoodProfessionRuntime,
    peer_count_with_last: f32,
    was_idle: f32,
) -> Option<FireFoodAction> {
    if !fire_food_job_rung_label(rung_label) {
        return None;
    }
    let _ = profession_is_sticky;
    let assigned = is_assigned_job || rung_label == "ASSIGNED_JOB";
    let max_people = fire_food_max_people_for_dispatch(assigned, false);
    Some(make_fire_food(
        counts,
        runtime,
        max_people,
        peer_count_with_last,
        was_idle,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baker_profession::RAW_MUTTON;
    use crate::fire_food_profession::{
        fire_food_counts_from_nearby, resolve_fire_food_assigned_job, HOT_COALS,
    };

    #[test]
    fn try_decide_fire_food_assigned_max_people() {
        assert!(fire_food_job_rung_label("ASSIGNED_JOB"));
        assert!(!fire_food_job_rung_label("ESCAPE"));
        assert!(resolve_fire_food_assigned_job(&FireFoodProfessionRuntime {
            is_assigned_fire_food: true,
            is_last_fire_food: false,
            weight: 1.0,
        }));
        assert!(resolve_fire_food_assigned_job(&FireFoodProfessionRuntime {
            is_assigned_fire_food: false,
            is_last_fire_food: true,
            weight: 1.0,
        }));
        let mut r = FireFoodProfessionRuntime::default();
        let mut c = fire_food_counts_from_nearby(
            &[(HOT_COALS, 1), (RAW_MUTTON, 1)],
            0,
            false,
            false,
            false,
            true,
            true,
            false,
            false,
        );
        c.has_hot_coals = true;
        c.has_fire_place = true;
        let a = try_decide_fire_food_from_rung(true, "ASSIGNED_JOB", true, &c, &mut r, 50.0, 0.0);
        assert!(a.is_some());
        assert!(a.unwrap().is_some(), "expected cook action under assigned max");
        assert!(r.is_last_fire_food);
    }
}
