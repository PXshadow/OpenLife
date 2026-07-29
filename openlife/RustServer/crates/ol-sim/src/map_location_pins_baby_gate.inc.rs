/// Birth: human parent gets BABY pin when child age &lt; MinAgeToEat.
// Haxe: GlobalPlayerInstance init L1013–1027
// C-SS-MIN-AGE-AI: live MinAgeToEat
pub fn send_baby_map_pin_to_parent(
    state: &SimState,
    outbound: &OutboundHub,
    parent_conn: u64,
    baby_p_id: i32,
) {
    let Some(parent) = state.players.get(&parent_conn) else {
        return;
    };
    if !player_is_human(parent.connected, parent.ai_controlled, &parent.email) {
        return;
    }
    let Some(baby) = state
        .players
        .values()
        .find(|p| p.p_id == baby_p_id && !p.deleted)
    else {
        return;
    };
    // C-SS-MIN-AGE-AI: live MinAgeToEat (Haxe ServerSettings.MinAgeToEat)
    let min_age = if state.gameplay.min_age_to_eat.is_finite() && state.gameplay.min_age_to_eat >= 0.0 {
        state.gameplay.min_age_to_eat
    } else {
        MIN_AGE_TO_EAT_YEARS
    };
    if baby.age >= min_age {
        return;
    }
    let (rel_x, rel_y) = parent.world_to_client(baby.x, baby.y);
    send_map_location_pin(
        outbound,
        parent_conn,
        parent.p_id,
        BABY_TEXT1,
        BABY_TEXT2,
        baby.p_id,
        rel_x,
        rel_y,
        true,
    );
}
