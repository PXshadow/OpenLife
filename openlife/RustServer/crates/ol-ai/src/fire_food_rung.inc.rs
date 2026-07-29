// AI-FIREFOOD-RUNG: assigned/last makeFireFood(100) ladder bridge
// Included from fire_food_profession.rs

/// Ladder rung labels that may run makeFireFood (assigned/last job band).
// Haxe: doTimeStuffHelper FIREFOODMAKER assigned/last ~754–756
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

/// Thin ladder bridge: assigned/last FIREFOODMAKER → pure `make_fire_food`.
///
/// `is_assigned_job` or rung `ASSIGNED_JOB` selects maxPeople **100** (Haxe makeFireFood(100)).
// Haxe: assignedProfession == 'FIREFOODMAKER' || lastProfession == 'FIREFOODMAKER' → makeFireFood(100)
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
    // Assigned/last plans set profession_is_sticky; still allow when sticky on runtime.
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
