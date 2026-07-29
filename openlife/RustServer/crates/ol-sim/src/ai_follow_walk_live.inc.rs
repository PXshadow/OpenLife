// AI-FOLLOW-WALK live pathfind + continuous tick (included from lib.rs)
// + AI-FOLLOW-ACQUIRE empty-sticky child-mother / AutoFollowPlayer closest
// Haxe: AiBase.isMovingToPlayer / startFollowingPlayer Goto / ally Goto(speaker)

/// Process deferred `orderedToDrop` (Haxe AI frame start before escape/jobs).
// Haxe: AiBase.doTimeStuffHelper orderedToDrop L481–484
// AI-SAY-HELPER / AI-LLM-APPLY
fn tick_ordered_ai_drop(state: &mut SimState, outbound: &OutboundHub) {
    // Snapshot then clear flag first (Haxe: orderedToDrop = false; dropHeldObject(0))
    let jobs: Vec<(u64, i32, i32)> = state
        .players
        .iter_mut()
        .filter(|(_, p)| !p.deleted && p.ai_ordered_to_drop)
        .map(|(&cid, p)| {
            p.ai_ordered_to_drop = false;
            (cid, p.x, p.y, p.held_id)
        })
        .filter(|(_, _, _, held)| *held != 0)
        .map(|(cid, x, y, _)| (cid, x, y))
        .collect();
    for (cid, x, y) in jobs {
        apply_drop(state, outbound, cid, x, y, None);
    }
}

/// Pathfind AI toward absolute goal and start timed MOVE path (follow / ally Goto).
// Haxe: myPlayer.gotoAdv / Goto → MoveHelper path
// AI-FOLLOW-WALK
fn try_ai_follow_path_to(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    goal_x: i32,
    goal_y: i32,
) -> bool {
    let (sx, sy, p_id, moving) = match state.players.get(&conn_id) {
        Some(p) if !p.deleted => (p.x, p.y, p.p_id, p.moving || p.move_path.is_some()),
        _ => return false,
    };
    if sx == goal_x && sy == goal_y {
        return false;
    }
    // Continuous tick: do not thrash mid-path; repath when stationary.
    if moving {
        return true;
    }
    let content = state.content.clone();
    let allies = state.allies.clone();
    let steps = {
        let world = state.world.read().unwrap();
        find_path(
            &world,
            sx,
            sy,
            goal_x,
            goal_y,
            &|x, y| {
                is_walkable_for_player(&world, &content, x, y, p_id, &|a, b| {
                    allies.is_mutual_or_either(a, b)
                })
            },
            2000,
        )
    };
    let Some(steps) = steps else {
        return false;
    };
    if steps.is_empty() {
        return false;
    }
    let capped = truncate_follow_path_steps(&steps, FOLLOW_PATH_STEP_CAP);
    let deltas = steps_to_client_path_deltas(&capped);
    match apply_move_path_start(state, outbound, conn_id, sx, sy, &deltas, None) {
        Ok(()) => true,
        Err(_) => {
            let dx = (goal_x - sx).signum();
            let dy = (goal_y - sy).signum();
            let step = if dx != 0 { (dx, 0) } else { (0, dy) };
            if step.0 == 0 && step.1 == 0 {
                return false;
            }
            apply_move_path_start(state, outbound, conn_id, sx, sy, &[step], None).is_ok()
        }
    }
}

/// True when this body is driven by AI (permanent NPC, takeover, selfplay).
// Haxe: myPlayer.isAi() / ServerAi body
fn player_is_ai_follow_body(p: &Player) -> bool {
    p.ai_controlled
        || email_looks_ai(&p.email)
        || p.email.to_ascii_lowercase().contains("npc")
        || p.email.to_ascii_lowercase().contains("selfplay")
}

/// Empty-sticky acquire: child-mother `getFollowPlayer` or AutoFollowPlayer closest.
// Haxe: AiBase.isMovingToPlayer playerToFollow == null branch L8287–8296
// AI-FOLLOW-ACQUIRE / auto_follow
fn tick_ai_follow_acquire(state: &mut SimState, auto_follow_player_enabled: bool) {
    use crate::ai_llm_apply::ai_follow_walk::{
        resolve_auto_follow_acquire_ex, AutoFollowCandidate, AUTO_FOLLOW_PLAYER_DEFAULT,
    };
    let _ = AUTO_FOLLOW_PLAYER_DEFAULT; // documented default; live passes explicit flag

    let ai_jobs: Vec<(u64, i32, i32, i32, f32)> = state
        .players
        .iter()
        .filter(|(_, p)| !p.deleted && p.ai_follow_p_id <= 0 && player_is_ai_follow_body(p))
        .map(|(c, p)| (*c, p.p_id, p.x, p.y, p.age))
        .collect();
    if ai_jobs.is_empty() {
        return;
    }

    // Leadership followPlayer map + deleted lookup for mother gate.
    let following = state.social.following.clone();
    let deleted_by_pid: std::collections::HashMap<i32, bool> = state
        .players
        .values()
        .map(|p| (p.p_id, p.deleted))
        .collect();

    // Closest-player candidates only needed when AutoFollowPlayer is on.
    let candidates: Vec<AutoFollowCandidate> = if auto_follow_player_enabled {
        state
            .players
            .values()
            .filter(|p| !p.deleted && p.p_id > 0)
            .map(|p| AutoFollowCandidate {
                p_id: p.p_id,
                x: p.x,
                y: p.y,
                // Haxe: Connection.getConnections = humans; getAis = AI bodies
                is_human: p.is_human_body(),
                deleted: false,
            })
            .collect()
    } else {
        Vec::new()
    };

    for (conn_id, ai_p_id, ax, ay, age) in ai_jobs {
        let leader = direct_follow_leader(&following, ai_p_id);
        let leader_deleted = leader
            .map(|id| deleted_by_pid.get(&id).copied().unwrap_or(true))
            .unwrap_or(true);
        // Haxe isMovingToPlayer(…, followHuman: true) default
        let only_human = true;
        let plan = resolve_auto_follow_acquire_ex(
            0,
            age,
            ax,
            ay,
            ai_p_id,
            leader,
            leader_deleted,
            auto_follow_player_enabled,
            only_human,
            &candidates,
            state.gameplay.min_age_to_eat,
        );
        if let Some(acq) = plan {
            if let Some(p) = state.players.get_mut(&conn_id) {
                // Loose follow (autoStopFollow stays default true) — Haxe just assigns
                p.ai_follow_p_id = acq.follow_p_id;
                let _ = acq.source; // ChildMother | ClosestPlayer (audit / future say)
            }
        }
    }
}

/// Continuous follow walk for AI sticky `ai_follow_p_id` (Haxe isMovingToPlayer).
// Haxe: AiBase.doTimeStuffHelper sticky clear → isMovingToPlayer(acquire+walk)
// AI-FOLLOW-WALK + AI-FOLLOW-ACQUIRE
fn tick_ai_follow_walk(state: &mut SimState, outbound: &OutboundHub) {
    use crate::ai_goals::priority_ladder::is_hungry_simple;
    use crate::ai_llm_apply::ai_follow_walk::{
        follow_max_tiles_for_context_ex, should_say_follow_target_name_ex,
    };

    let now = state.sim_time;

    // Haxe: doTimeStuffHelper L560–568 sticky auto-clear BEFORE isMovingToPlayer
    // (ordered 5min timeout / age>MinAgeToEat*2 clear). Must run before acquire so
    // adult AutoFollow can re-latch same tick after age clear (Haxe order).
    {
        let clear_jobs: Vec<(u64, i32, bool, f32, f32)> = state
            .players
            .iter()
            .filter(|(_, p)| !p.deleted && p.ai_follow_p_id > 0)
            .filter(|(_, p)| player_is_ai_follow_body(p))
            .map(|(c, p)| {
                (
                    *c,
                    p.ai_follow_p_id,
                    p.ai_auto_stop_follow,
                    p.ai_follow_started_sim_time,
                    p.age,
                )
            })
            .collect();
        for (conn_id, follow_p_id, auto_stop, started, age) in clear_jobs {
            let mut sticky = AiFollowSticky {
                follow_p_id,
                auto_stop_follow: auto_stop,
                follow_started_sim_time: started,
            };
            // C-SS-MIN-AGE-AI: live MinAgeToEat * 2 clear gate
            let clear = plan_follow_sticky_clear_ex(
                &sticky,
                age,
                now,
                state.gameplay.min_age_to_eat,
            );
            apply_follow_sticky_clear(&mut sticky, clear);
            if let Some(p) = state.players.get_mut(&conn_id) {
                p.ai_auto_stop_follow = sticky.auto_stop_follow;
                p.ai_follow_p_id = sticky.follow_p_id;
            }
        }
    }

    // AI-FOLLOW-ACQUIRE: fill empty sticky before walk (child-mother / AutoFollowPlayer)
    // Haxe: isMovingToPlayer when playerToFollow == null
    // LiveSettings / GameplayKnobs.auto_follow_player (Haxe ServerSettings.AutoFollowPlayer)
    let auto_follow = state.gameplay.auto_follow_player;
    tick_ai_follow_acquire(state, auto_follow);

    let following = state.social.following.clone();
    let ai_jobs: Vec<(
        u64,
        i32,
        i32,
        i32,
        f32,
        bool,
        bool,
        bool,
        bool,
        bool,
        bool,
    )> = {
        let content = &state.content;
        state
            .players
            .iter()
            .filter(|(_, p)| !p.deleted && p.ai_follow_p_id > 0)
            .filter(|(_, p)| player_is_ai_follow_body(p))
            .map(|(c, p)| {
                // Haxe: isWounded() || hasYellowFever()
                let held_wound =
                    p.is_wounded_held(is_wound_object(content, p.held_id));
                let wounded_or_fever =
                    held_wound || p.hidden_wound.is_some() || p.fever.is_some();
                (
                    *c,
                    p.p_id,
                    p.x,
                    p.y,
                    p.age,
                    p.ai_auto_stop_follow,
                    p.moving || p.move_path.is_some(),
                    is_hungry_simple(p.food),
                    p.ai_is_nice_baby,
                    wounded_or_fever,
                    p.ai_debug_say,
                )
            })
            .collect()
    };
    if ai_jobs.is_empty() {
        return;
    }
    let by_pid: std::collections::HashMap<i32, (i32, i32, bool, String)> = state
        .players
        .values()
        .map(|p| (p.p_id, (p.x, p.y, p.deleted, p.display_name())))
        .collect();

    for (
        conn_id,
        ai_p_id,
        ax,
        ay,
        age,
        auto_stop,
        moving,
        hungry,
        nice_baby,
        wounded_or_fever,
        debug_say,
    ) in ai_jobs
    {
        let follow_p_id = state
            .players
            .get(&conn_id)
            .map(|p| p.ai_follow_p_id)
            .unwrap_or(0);
        if follow_p_id <= 0 {
            continue;
        }
        let target_row = by_pid.get(&follow_p_id);
        let target = target_row.map(|&(x, y, deleted, _)| FollowTargetSnap {
            p_id: follow_p_id,
            x,
            y,
            deleted,
        });
        // Living leadership mother for child band (same social.following as acquire).
        let leader = direct_follow_leader(&following, ai_p_id);
        let has_living_mother = leader
            .map(|id| {
                by_pid
                    .get(&id)
                    .map(|(_, _, deleted, _)| !*deleted)
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        // Haxe doTimeStuffHelper specialized distance bands before general 5/10
        // C-SS-MIN-AGE-AI: live MinAgeToEat for baby/child follow bands
        let max_tiles = follow_max_tiles_for_context_ex(
            age,
            hungry,
            has_living_mother,
            nice_baby,
            wounded_or_fever,
            auto_stop,
            state.gameplay.min_age_to_eat,
        );
        let seed = follow_seed(now, ai_p_id);
        let decision = decide_follow_walk(follow_p_id, target, ax, ay, max_tiles, seed);
        match decision {
            FollowWalkDecision::TargetDeleted => {
                if let Some(p) = state.players.get_mut(&conn_id) {
                    p.ai_follow_p_id = 0;
                    p.ai_auto_stop_follow = true;
                }
            }
            FollowWalkDecision::WalkTo { goal_x, goal_y } => {
                if !moving {
                    let _ = try_ai_follow_path_to(state, outbound, conn_id, goal_x, goal_y);
                }
                // Haxe: isMovingToPlayer L8318 say target name while walking
                if should_say_follow_target_name_ex(age, debug_say, state.gameplay.min_age_to_eat) {
                    if let Some((_, _, _, name)) = target_row {
                        if !name.is_empty() {
                            // PO-MAX-DISTANCE: CloseForSay 20
                            let near = nearby_conn_ids(state, ax, ay, ADULT_CHAT_RANGE);
                            send_chat_ps(state, outbound, conn_id, ai_p_id, name, &near);
                        }
                    }
                }
            }
            FollowWalkDecision::CloseEnough | FollowWalkDecision::NoTarget => {}
        }
    }
}
