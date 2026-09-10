// AI-SAY-HELPER live fan-out (included from lib.rs)

/// Clear assigned job flags then set the matching runtime (Haxe one assignedProfession).
// Haxe: assignedProfession = prof; job ladders read the string
fn apply_player_assigned_profession_from_say(
    p: &mut crate::Player,
    assign: Option<String>,
) {
    p.assigned_profession = assign.clone();
    p.farm_profession.assigned_profession = None;
    p.smith_profession.is_assigned_smith = false;
    p.baker_profession.is_assigned_baker = false;
    p.pottery_profession.is_assigned_potter = false;
    p.shepherd_profession.is_assigned_shepherd = false;
    p.fire_food_profession.is_assigned_fire_food = false;
    p.fire_keeper_profession.is_assigned_fire_keeper = false;
    p.grave_keeper_profession.is_assigned_grave_keeper = false;
    p.hunter_profession.is_assigned_hunter = false;
    p.lumberjack_profession.is_assigned_lumberjack = false;
    p.collector_profession.is_assigned_collector = false;
    p.foodserver_profession.is_assigned_foodserver = false;
    let Some(key) = assign.as_deref() else {
        return;
    };
    let bang = format!("{key}!");
    let _ = crate::assign_farm_from_speech(&mut p.farm_profession, &bang);
    let _ = crate::assign_smith_from_speech(&mut p.smith_profession, &bang);
    let _ = crate::assign_baker_from_speech(&mut p.baker_profession, &bang);
    let _ = crate::assign_potter_from_speech(&mut p.pottery_profession, &bang);
    let _ = crate::assign_shepherd_from_speech(&mut p.shepherd_profession, &bang);
    let _ = crate::assign_fire_food_from_speech(&mut p.fire_food_profession, &bang);
    let _ = crate::assign_fire_keeper_from_speech(&mut p.fire_keeper_profession, &bang);
    let _ = crate::assign_grave_keeper_from_speech(&mut p.grave_keeper_profession, &bang);
    let _ = crate::assign_hunter_from_speech(&mut p.hunter_profession, &bang);
    let _ = crate::assign_lumberjack_from_speech(&mut p.lumberjack_profession, &bang);
    let _ = crate::assign_collector_from_speech(&mut p.collector_profession, &bang);
    let _ = crate::assign_foodserver_from_speech(&mut p.foodserver_profession, &bang);
}

// AI-SAY-HELPER live fan-out (included from lib.rs)
// Haxe: AiBase.sayHelper L4755–4970 after sendSayToAllClose
// AI-SAY-HELPER-FAN

/// Scripted sayHelper for every human SAY. Returns AI conn_ids that handled a command.
///
/// Hearer path only — speaker `I AM` / `YOU ARE` is DoNaming, not sayHelper.
// Haxe: Connection.sendSayToAllClose → each AI sayHelper (before LLM)
fn fan_out_ai_say_scripted(
    state: &mut SimState,
    outbound: &OutboundHub,
    speaker_conn: u64,
    text: &str,
) -> std::collections::HashSet<u64> {
    use crate::ai_follow_walk::{ally_goto_speaker_xy, follow_seed};
    use crate::ai_handler::{mark_llm_reacted, set_waiting_time_min};
    use crate::ai_llm_apply::{get_object_by_name_like, resolve_make_item_id};
    use crate::ai_say_helper::{
        apply_scripted_waiting, go_home_debug_say, go_home_goal_xy, go_home_move_target,
        plan_scripted_say_helper, scripted_waiting_forces_write, ScriptedSayCtx,
    };
    use crate::ai_takeover::player_is_ai;
    use crate::animal_damage::is_holding_weapon;
    use crate::handling_fire::{get_close_fire, HandlingFireMapObj, GET_CLOSE_FIRE_MAXDIST};
    use crate::move_live_gates::is_friendly;
    use crate::player_soul::is_angry_or_terrified;
    use crate::relations::{is_close_relative, is_leadership_ally};
    use crate::speech::{search_new_home_ex, HOME_SEARCH_MAX_QUAD};

    let mut handled = std::collections::HashSet::new();
    let hearers = live_collect_ai_speech_hearers(state, speaker_conn, text);
    if hearers.is_empty() {
        return handled;
    }
    let Some(speaker) = state.players.get(&speaker_conn).cloned() else {
        return handled;
    };
    if speaker.deleted || player_is_ai(speaker.connected, speaker.ai_controlled, &speaker.email)
    {
        return handled;
    }

    let now = state.sim_time;
    let speaker_p_id = speaker.p_id;
    let speaker_name = speaker.first_name.clone();
    let speaker_held = speaker.held_id;
    let speaker_held_name = state
        .content
        .get(speaker_held)
        .map(|d| d.name.clone())
        .unwrap_or_default();
    let speaker_weapon = is_holding_weapon(speaker_held, &speaker_held_name);
    let speaker_angry = is_angry_or_terrified(speaker.angry_time);
    let speaker_x = speaker.x;
    let speaker_y = speaker.y;

    let name_hits: Vec<(i32, String)> = state
        .content
        .objects
        .iter()
        .filter(|(id, _)| **id > 0)
        .map(|(id, d)| (*id, d.name.clone()))
        .collect();

    let mut drop_feet: Vec<(u64, i32, i32)> = Vec::new();
    let mut pending_says: Vec<(u64, i32, i32, i32, String)> = Vec::new();
    let mut pending_emotes: Vec<(u64, i32, i32, i32, i32)> = Vec::new();
    let mut path_jobs: Vec<(u64, i32, i32)> = Vec::new();
    let mut jump_cids: Vec<u64> = Vec::new();

    for h in hearers {
        let Some(ai_snap) = state.players.get(&h.conn_id).cloned() else {
            continue;
        };
        let leadership = is_leadership_ally(&state.social.following, h.p_id, speaker_p_id)
            || state.allies.is_mutual_or_either(h.p_id, speaker_p_id);
        let friendly = is_friendly(
            leadership,
            ai_snap.last_attacked_player_id,
            ai_snap.last_player_attacked_me_id,
            speaker_p_id,
        );
        let should_cmd = state.social.is_follower_from(h.p_id, speaker_p_id)
            || is_close_relative(&state.social, h.p_id, speaker_p_id);
        let home_qd = {
            let dx = (ai_snap.home_x - ai_snap.x) as f32;
            let dy = (ai_snap.home_y - ai_snap.y) as f32;
            dx * dx + dy * dy
        };
        let home_name = {
            let id = state
                .world
                .read()
                .map(|w| w.get_object(ai_snap.home_x, ai_snap.home_y))
                .unwrap_or(0);
            state
                .content
                .get(id)
                .map(|d| d.name.clone())
                .unwrap_or_else(|| "HOME".into())
        };
        let rand_ai = ((now as i64)
            .wrapping_mul(31)
            .wrapping_add(h.p_id as i64)
            .wrapping_add(speaker_p_id as i64)
            .rem_euclid(9)) as u8;

        let ctx = ScriptedSayCtx {
            text: h.normalized_text.clone(),
            now_sim: now,
            last_react_sim_time: ai_snap.llm_speech.last_react_sim_time,
            ai_name: ai_snap.first_name.clone(),
            ai_family_name: ai_snap.family_name.clone(),
            speaker_name: speaker_name.clone(),
            speaker_p_id,
            ai_angry: is_angry_or_terrified(ai_snap.angry_time),
            speaker_angry,
            speaker_holding_weapon: speaker_weapon,
            is_friendly: friendly,
            should_do_command: should_cmd,
            is_nice_baby: ai_snap.ai_is_nice_baby,
            assigned_profession: ai_snap.assigned_profession.clone(),
            last_profession: ai_snap.last_profession.clone(),
            home_quad_dist: home_qd,
            home_name,
            rand_ai,
            debug_say: ai_snap.ai_debug_say,
            debug_profession: ai_snap.ai_debug_profession,
        };

        let plan = plan_scripted_say_helper(&ctx);
        if !plan.handled {
            continue;
        }
        handled.insert(h.conn_id);

        let mut extra_say: Option<String> = None;
        let mut go_home_job: Option<(i32, i32, bool)> = None;
        if let Some(ai) = state.players.get_mut(&h.conn_id) {
            if plan.mark_reacted {
                mark_llm_reacted(&mut ai.llm_speech, now);
            }
            let wait = apply_scripted_waiting(ai.llm_speech.waiting_time_min, &plan);
            // Haxe STOP/DROP `waitingTime = N` can lower.
            if scripted_waiting_forces_write(&plan) {
                ai.llm_speech.waiting_time_min = wait;
            } else if wait > ai.llm_speech.waiting_time_min {
                set_waiting_time_min(&mut ai.llm_speech, wait);
            }
            if plan.think_time_bump > 0.0 {
                set_waiting_time_min(&mut ai.llm_speech, plan.think_time_bump);
            }
            if plan.stop_goto_self {
                ai.move_path = None;
                ai.moving = false;
                ai.force_stop_on_next_tile = true;
            }
            if plan.start_follow {
                ai.ai_follow_p_id = plan.follow_p_id;
                ai.ai_auto_stop_follow = plan.set_auto_stop_follow.unwrap_or(false);
                ai.ai_follow_started_sim_time = plan.follow_started_sim_time;
                ai.force_stop_on_next_tile = false;
            }
            if plan.clear_follow {
                ai.ai_follow_p_id = 0;
            }
            if let Some(v) = plan.set_auto_stop_follow {
                if !plan.start_follow {
                    ai.ai_auto_stop_follow = v;
                }
            }
            if plan.ordered_to_drop {
                ai.ai_ordered_to_drop = true;
            }
            if plan.do_drop_now && ai.held_id != 0 {
                drop_feet.push((h.conn_id, ai.x, ai.y));
            }
            if let Some(v) = plan.set_debug_say {
                ai.ai_debug_say = v;
            }
            if let Some(v) = plan.set_debug_profession {
                ai.ai_debug_profession = v;
            }
            if let Some(ref assign) = plan.set_assigned_profession {
                apply_player_assigned_profession_from_say(ai, assign.clone());
            }
            if let Some(ref raw) = plan.make_craft_text {
                if let Some(id) = resolve_make_item_id(raw, |search, from_end| {
                    get_object_by_name_like(
                        name_hits.iter().map(|(i, n)| (*i, n.as_str())),
                        search,
                        from_end,
                    )
                }) {
                    let name = name_hits
                        .iter()
                        .find(|(i, _)| *i == id)
                        .map(|(_, n)| n.clone());
                    extra_say = ai.craft_ai.do_make_craft_command(id, name, false);
                }
            }
            if plan.jump {
                ai.done_moving_seq = ai.done_moving_seq.saturating_add(1).max(1);
                jump_cids.push(h.conn_id);
            }
            if plan.goto_speaker_offset {
                let (gx, gy) = ally_goto_speaker_xy(speaker_x, speaker_y);
                path_jobs.push((h.conn_id, gx, gy));
            }
            if plan.move_to_home {
                let fire = if ai.ai_fire_place_id != 0 {
                    Some((ai.ai_fire_place_x, ai.ai_fire_place_y))
                } else {
                    None
                };
                let (tx, ty) = go_home_move_target(ai.home_x, ai.home_y, fire);
                let seed = follow_seed(now, h.p_id);
                let (gx, gy) = go_home_goal_xy(tx, ty, seed);
                go_home_job = Some((gx, gy, ai.ai_debug_say));
            }
        }
        if plan.search_new_home {
            if let Some(ai) = state.players.get(&h.conn_id) {
                let ax = ai.x;
                let ay = ai.y;
                let old_home = (ai.home_x, ai.home_y);
                let oven_tiles: Vec<(i32, i32)> =
                    state.world_map_time.ovens.values().copied().collect();
                let (ovens, map_w, map_h) = match state.world.read() {
                    Ok(w) => {
                        let ovens = crate::do_commands_wire::collect_home_search_ovens(
                            &w,
                            &oven_tiles,
                            |x, y| state.world_map_time.orig_biome_at(x, y),
                            ax,
                            ay,
                        );
                        let (mw, mh) = if w.wrap {
                            (w.width_tiles, w.height_tiles)
                        } else {
                            (0, 0)
                        };
                        (ovens, mw, mh)
                    }
                    Err(_) => (Vec::new(), 0, 0),
                };
                if let Some((hx, hy)) =
                    search_new_home_ex(ax, ay, &ovens, HOME_SEARCH_MAX_QUAD, map_w, map_h)
                {
                    let oid = state.world.read().map(|w| w.get_object(hx, hy)).unwrap_or(0);
                    let new_name = state
                        .content
                        .get(oid)
                        .map(|d| d.name.clone())
                        .unwrap_or_else(|| "home".into());
                    extra_say = Some(if old_home != (hx, hy) {
                        format!("Have a new home! {new_name}")
                    } else {
                        format!("No mew home! {new_name}")
                    });
                    if let Some(ai) = state.players.get_mut(&h.conn_id) {
                        ai.home_x = hx;
                        ai.home_y = hy;
                    }
                }
            }
            // Haxe: myPlayer.firePlace = AiHelper.GetCloseFire(myPlayer)
            let (hx, hy) = state
                .players
                .get(&h.conn_id)
                .map(|p| (p.home_x, p.home_y))
                .unwrap_or((0, 0));
            let r = GET_CLOSE_FIRE_MAXDIST;
            let tiles: Vec<(i32, i32, i32)> = match state.world.read() {
                Ok(w) => {
                    let mut v = Vec::new();
                    for y in (hy - r)..=(hy + r) {
                        for x in (hx - r)..=(hx + r) {
                            let id = w.get_object(x, y);
                            if id > 0 {
                                v.push((id, x, y));
                            }
                        }
                    }
                    v
                }
                Err(_) => Vec::new(),
            };
            let map: Vec<HandlingFireMapObj> = tiles
                .into_iter()
                .map(|(id, x, y)| HandlingFireMapObj {
                    parent_id: state.content.resolve_base_id(id),
                    x,
                    y,
                })
                .collect();
            if let Some((fid, fx, fy)) = get_close_fire(&map, hx, hy, r) {
                if let Some(ai) = state.players.get_mut(&h.conn_id) {
                    ai.ai_fire_place_id = fid;
                    ai.ai_fire_place_x = fx;
                    ai.ai_fire_place_y = fy;
                }
            }
        }
        if let Some((gx, gy, debug)) = go_home_job {
            let ok = try_ai_follow_path_to(state, outbound, h.conn_id, gx, gy);
            if let Some(line) = go_home_debug_say(debug, ok) {
                extra_say = Some(line.to_string());
            }
        }

        let say_text = extra_say.or(plan.say);
        if let Some(s) = say_text {
            if let Some(ai) = state.players.get(&h.conn_id) {
                pending_says.push((h.conn_id, ai.p_id, ai.x, ai.y, s));
            }
        }
        if let Some(eid) = plan.emote_id {
            if let Some(ai) = state.players.get(&h.conn_id) {
                pending_emotes.push((h.conn_id, h.p_id, ai.x, ai.y, eid));
            }
        }
    }

    for (_cid, p_id, x, y, eid) in pending_emotes {
        let pe = format_player_emot(p_id, eid).into_bytes();
        let near = nearby_conn_ids(state, x, y, nearby_range(state));
        send_nearby(outbound, &near, pe);
        for &nid in &near {
            send_frame(outbound, nid);
        }
    }
    for (cid, p_id, x, y, s) in pending_says {
        let near = nearby_conn_ids(state, x, y, crate::say_close_range(state, 20.0));
        send_chat_ps(state, outbound, cid, p_id, &s, &near);
        info!(p_id, text = %s, "sim: AI-SAY-HELPER scripted SAY");
    }
    for cid in jump_cids {
        // Haxe: myPlayer.jump() — PU + baby BW (JUMP-BW-FULL)
        if let Some(p) = state.players.get(&cid) {
            let spd = player_move_speed(state, p);
            let pu = format_player_update_line(
                p.p_id,
                person_object_id(p),
                p.held_id,
                p.x,
                p.y,
                p.age,
                spd,
                p.done_moving_seq.max(1),
            );
            let near = nearby_conn_ids(state, p.x, p.y, nearby_range(state));
            send_nearby(
                outbound,
                &near,
                format_server_message("PU", &[&pu]).into_bytes(),
            );
            if p.age < BABY_AGE_THRESHOLD {
                send_nearby(outbound, &near, format_baby_wiggle(p.p_id).into_bytes());
            }
            for &nid in &near {
                send_frame(outbound, nid);
            }
        }
    }
    for (cid, x, y) in drop_feet {
        apply_drop(state, outbound, cid, x, y, None);
        if let Some(ai) = state.players.get_mut(&cid) {
            ai.ai_ordered_to_drop = false;
        }
    }
    for (cid, gx, gy) in path_jobs {
        let _ = try_ai_follow_path_to(state, outbound, cid, gx, gy);
    }
    handled
}
