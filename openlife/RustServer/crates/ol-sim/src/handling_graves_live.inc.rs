// AI-JOB-GRAVE: live scan helpers included from profession_scan.rs

/// Grave-keeper / isHandlingGraves: sensors → pure body → live intent.
// Haxe: isHandlingGraves assigned|last ~742; mid ~657; hungry ~8538
pub fn handling_graves_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    rung_label: &str,
    grave_keeper: &mut crate::GraveKeeperProfessionRuntime,
) -> ProfessionScanTickResult {
    let map: Vec<crate::handling_graves::HandlingGravesMapObj> = tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| crate::handling_graves::HandlingGravesMapObj {
            parent_id: t.parent_id,
            x: t.x,
            y: t.y,
            floor_id: t.floor_id,
            contained_count: t.contained_count,
        })
        .collect();
    let sticky = if grave_keeper.last_grave_id != 0 {
        Some((
            grave_keeper.last_grave_id,
            grave_keeper.last_grave_x,
            grave_keeper.last_grave_y,
        ))
    } else {
        None
    };
    let sensors = crate::handling_graves_sensors_from_map(
        &map,
        inp.held_id,
        inp.held_contained,
        inp.player_x,
        inp.player_y,
        inp.home_x,
        inp.home_y,
        inp.age,
        inp.is_hungry,
        inp.target_reachable,
        false,
        inp.is_best_grave_keeper,
        false,
        sticky,
    );
    let Some(action) = crate::try_decide_handling_graves_from_rung(
        rung_label,
        inp.is_assigned_job,
        &sensors,
        grave_keeper,
    ) else {
        return ProfessionScanTickResult::none();
    };
    if !action.is_some() {
        return ProfessionScanTickResult::none();
    }
    handling_graves_action_to_live_intent(inp, action)
}

fn handling_graves_action_to_live_intent(
    inp: &ProfessionScanInput,
    action: crate::HandlingGravesAction,
) -> ProfessionScanTickResult {
    use crate::HandlingGravesAction;
    match action {
        HandlingGravesAction::None => ProfessionScanTickResult::none(),
        HandlingGravesAction::DropHeld => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::DropAt {
                x: inp.player_x,
                y: inp.player_y,
            },
            had_action: true,
        },
        HandlingGravesAction::RemoveFromGrave {
            x,
            y,
            object_id,
        } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::StageRemoveFromContainer {
                x,
                y,
                expected_parent: object_id,
            },
            had_action: true,
        },
        HandlingGravesAction::UseHeldOnGrave { x, y } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::UseAt {
                x,
                y,
                target_id: 0,
                actor_id: inp.held_id,
            },
            had_action: true,
        },
        HandlingGravesAction::PickupItem { object_id }
        | HandlingGravesAction::GetItem { object_id } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::SeekOrCraft {
                actor: object_id,
                craft_if_needed: false,
            },
            had_action: true,
        },
        HandlingGravesAction::GetOrCraft { object_id } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::SeekOrCraft {
                actor: object_id,
                craft_if_needed: true,
            },
            had_action: true,
        },
    }
}

/// Roster for Haxe `getBestAiForObjByProfession('GRAVEKEEPER', grave)`.
pub fn grave_keeper_peers_from_state(
    state: &SimState,
    obj_x: i32,
    obj_y: i32,
    self_home_x: i32,
    self_home_y: i32,
) -> Vec<crate::GraveKeeperPeer> {
    let following = &state.social.following;
    state
        .players
        .values()
        .map(|p| {
            let combat = state.combat.wound_of(p.p_id) > 0;
            let wounded =
                peer_roster_flags_for_player(p, &state.content, combat, Some(following)).is_wounded;
            let (ohx, ohy) = if p.home_x != 0 || p.home_y != 0 {
                (p.home_x, p.home_y)
            } else {
                (p.x, p.y)
            };
            let dx = (p.x - obj_x) as f32;
            let dy = (p.y - obj_y) as f32;
            crate::GraveKeeperPeer {
                p_id: p.p_id,
                quad_dist_to_obj: dx * dx + dy * dy,
                deleted: p.deleted,
                age: p.age,
                is_wounded: wounded,
                food_store: p.food,
                same_home: ohx == self_home_x && ohy == self_home_y,
                has_grave_keeper: p.grave_keeper_profession.weight > 0.0,
            }
        })
        .collect()
}

/// True when `self_p_id` wins Haxe GRAVEKEEPER distance pick vs `obj`.
pub fn is_self_best_grave_keeper_from_state(
    state: &SimState,
    self_p_id: i32,
    obj_x: i32,
    obj_y: i32,
    self_home_x: i32,
    self_home_y: i32,
) -> bool {
    let min_age =
        if state.gameplay.min_age_to_eat.is_finite() && state.gameplay.min_age_to_eat >= 0.0 {
            state.gameplay.min_age_to_eat
        } else {
            MIN_AGE_TO_EAT
        };
    let peers = grave_keeper_peers_from_state(state, obj_x, obj_y, self_home_x, self_home_y);
    crate::is_self_best_grave_keeper_for_obj(self_p_id, &peers, min_age)
}

/// Closest grave-dig tile in the scan (sticky lastGrave if still valid).
pub fn pick_grave_xy_from_scan(
    tiles: &[ScanTile],
    player_x: i32,
    player_y: i32,
    last_grave: Option<(i32, i32, i32)>,
) -> Option<(i32, i32)> {
    let map: Vec<crate::handling_graves::HandlingGravesMapObj> = tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| crate::handling_graves::HandlingGravesMapObj {
            parent_id: t.parent_id,
            x: t.x,
            y: t.y,
            floor_id: t.floor_id,
            contained_count: t.contained_count,
        })
        .collect();
    crate::handling_graves::pick_grave(
        &map,
        player_x,
        player_y,
        crate::GRAVE_SEARCH_RADIUS,
        last_grave,
    )
    .map(|(_, x, y, _, _)| (x, y))
}

/// Haxe `getBestAiForObjByProfession('GRAVEKEEPER', grave)` for [`ProfessionScanInput`].
// Haxe: AiBase.isHandlingGraves ~1527
pub fn best_grave_keeper_flag_from_state(
    state: &SimState,
    self_p_id: i32,
    home_x: i32,
    home_y: i32,
    player_x: i32,
    player_y: i32,
    tiles: &[ScanTile],
    last_grave: Option<(i32, i32, i32)>,
) -> bool {
    let Some((gx, gy)) = pick_grave_xy_from_scan(tiles, player_x, player_y, last_grave) else {
        return true;
    };
    is_self_best_grave_keeper_from_state(state, self_p_id, gx, gy, home_x, home_y)
}
