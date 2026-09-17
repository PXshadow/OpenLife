// AI-JOB-FOODSERVER: live scan helpers included from profession_scan.rs

/// FOODSERVER / isFeedingPlayerInNeed: starving cands + held food → goto/feed/seek.
// Haxe: assigned/last FOODSERVER → isFeedingPlayerInNeed(100) ~739
// Haxe: mid doStuff && SMITH<1 → isFeedingPlayerInNeed() max=1 ~594 (**AI-FEED-MID**)
pub fn feeding_player_profession_scan_tick(
    inp: &ProfessionScanInput,
    rung_label: &str,
    foodserver: &mut crate::FoodServerProfessionRuntime,
) -> ProfessionScanTickResult {
    use crate::feeding_player::{
        try_decide_feeding_player_from_rung, FeedingPlayerAction, FeedingPlayerSensors,
    };
    use crate::MIN_AGE_TO_EAT;
    let sensors = FeedingPlayerSensors {
        player_x: inp.player_x,
        player_y: inp.player_y,
        age: inp.age,
        food: inp.food_store,
        min_age_to_eat: MIN_AGE_TO_EAT,
        is_moving: inp.is_moving,
        is_fertile: inp.feeder_is_fertile,
        is_smith: inp.feeder_is_smith,
        held_id: inp.held_id,
        held_food_value: inp.held_food_value,
        self_p_id: inp.feeder_p_id,
    };
    let was_idle = if inp.profession_is_sticky {
        0.0
    } else {
        inp.was_idle
    };
    let Some(action) = try_decide_feeding_player_from_rung(
        rung_label,
        inp.is_assigned_job,
        &sensors,
        foodserver,
        inp.peer_count,
        was_idle,
        &inp.feeding_cands,
    ) else {
        return ProfessionScanTickResult::none();
    };
    match action {
        FeedingPlayerAction::None => ProfessionScanTickResult::none(),
        FeedingPlayerAction::Wait => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::Wait,
            had_action: true,
        },
        FeedingPlayerAction::ForceStopWait => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::ForceStopWait,
            had_action: true,
        },
        FeedingPlayerAction::Goto { x, y } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::Goto { x, y },
            had_action: true,
        },
        FeedingPlayerAction::SeekFood {
            target_p_id,
            target_conn,
        } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::SeekFeedFood {
                target_p_id,
                target_conn,
            },
            had_action: true,
        },
        FeedingPlayerAction::Feed {
            target_p_id,
            target_conn,
        } => ProfessionScanTickResult {
            intent: ShortCraftLiveIntent::FeedOther {
                target_p_id,
                target_conn,
            },
            had_action: true,
        },
    }
}

/// Haxe `isStayingCloseToChild` live: fertile mother Goto most-distant own infant.
// Haxe: AiBase.isStayingCloseToChild L6399–6409
pub fn apply_stay_close_child_tick(
    state: &mut crate::SimState,
    conn_id: u64,
) -> ShortCraftLiveApplyResult {
    let Some(p) = state.players.get(&conn_id) else {
        return ShortCraftLiveApplyResult::Failed;
    };
    if !crate::player_is_fertile(state, p) {
        return ShortCraftLiveApplyResult::Failed;
    }
    let mother_id = p.p_id;
    let mx = p.x;
    let my = p.y;
    let min_age = crate::MIN_AGE_TO_EAT;
    let cands: Vec<crate::HungryChildCand> = state
        .players
        .values()
        .filter(|o| !o.deleted && o.p_id != mother_id)
        .map(|o| crate::HungryChildCand {
            conn_id: o.conn_id,
            p_id: o.p_id,
            x: o.x,
            y: o.y,
            age: o.age,
            food: o.food,
            held_by: o.held_by,
            is_own: o.ai_follow_p_id == mother_id,
        })
        .collect();
    let child = crate::pick_most_distant_own_child(
        mx,
        my,
        &cands,
        crate::DISTANT_OWN_CHILD_MIN_DIST,
        crate::DISTANT_OWN_CHILD_SEARCH_DIST,
        min_age,
    );
    let xy = crate::is_staying_close_to_child(true, child.map(|c| (c.x, c.y)));
    match xy {
        Some((x, y)) => ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { x, y }),
        None => ShortCraftLiveApplyResult::Failed,
    }
}

/// Fill `GetCloseStarvingPlayer` cands from live players.
// Haxe: AiHelper.GetCloseStarvingPlayerHelper AllPlayers
pub fn starving_cands_from_state(
    state: &crate::SimState,
    self_conn: u64,
    held_id: i32,
    held_food_value: i32,
) -> Vec<crate::StarvingCand> {
    use crate::ai_takeover::player_is_ai;
    use crate::nested_body::is_yellow_fever;
    use crate::relations::{is_ally, is_close_relative};
    use ol_player_helper::food_eat_gates::can_feed_to_me_obj_ex_yum;
    let Some(self_p) = state.players.get(&self_conn) else {
        return Vec::new();
    };
    let self_p_id = self_p.p_id;
    let follow_id = self_p.ai_follow_p_id;
    let deleted: std::collections::HashSet<i32> = state
        .players
        .values()
        .filter(|p| p.deleted)
        .map(|p| p.p_id)
        .collect();
    let mut out = Vec::new();
    for p in state.players.values() {
        if p.conn_id == self_conn || p.p_id == 0 {
            continue;
        }
        let count_eaten = p.yum.get_count_eaten(held_id);
        let fever = is_yellow_fever(p.fever.as_ref());
        let can_feed = if held_id <= 0 || held_food_value < 1 {
            false
        } else {
            can_feed_to_me_obj_ex_yum(
                held_id,
                held_food_value,
                count_eaten,
                p.food,
                p.food_max,
                fever,
                5.0,
            )
        };
        let is_smith = p.smith_profession.is_last_smith
            || p.last_profession.as_deref() == Some("SMITH");
        let lost = state.combat.stats.get(&p.p_id).map(|s| s.lost_combat_prestige).unwrap_or(0.0);
        out.push(crate::StarvingCand {
            conn_id: p.conn_id,
            p_id: p.p_id,
            x: p.x,
            y: p.y,
            age: p.age,
            food: p.food,
            food_max: p.food_max,
            deleted: p.deleted,
            held_by: p.held_by != 0,
            is_ai: player_is_ai(p.connected, p.ai_controlled, &p.email),
            is_wounded: state.combat.wound_of(p.p_id) > 0 && !p.is_holding_hidden_wound(),
            has_yellow_fever: fever,
            is_smith,
            prestige_class: state.social.prestige_class(p.p_id).as_i32(),
            is_ally: is_ally(
                &state.social.following,
                &state.social,
                &deleted,
                self_p_id,
                p.p_id,
            ),
            is_close_relative: is_close_relative(&state.social, self_p_id, p.p_id),
            is_follow_target: follow_id != 0 && follow_id == p.p_id,
            angry_time: p.angry_time,
            lost_combat_prestige: lost,
            can_feed_held: can_feed,
            is_starting_name: {
                let t = state.gameplay.starting_name.trim();
                let start = if t.is_empty() {
                    crate::naming::STARTING_NAME
                } else {
                    t
                };
                p.first_name.trim().eq_ignore_ascii_case(start)
            },
            is_female: crate::player_is_female(state, p),
        });
    }
    out
}
