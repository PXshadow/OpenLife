// AI-LLM-FAN live fan-out + tick (included from lib.rs)
// Haxe: AiBase.sayHelper LLM fallback L4971–5001 after scripted cmds
// Haxe: Connection.sendSayToAllClose → respondToPlayerAsync / sendResponseInChunks

/// Drain pending LLM speech jobs (for ol-server HTTP worker / tests).
pub fn take_llm_speech_jobs(state: &mut SimState) -> Vec<crate::ai_handler::LlmSpeechJob> {
    std::mem::take(&mut state.llm_speech_jobs)
}

/// Push a completed LLM result for next `tick_llm_speech_wire`.
pub fn push_llm_speech_result(state: &mut SimState, result: crate::ai_handler::LlmSpeechResult) {
    state.llm_speech_results.push(result);
}

/// Import completed LLM results from the outer HTTP share into sim queues.
// Haxe: respondToPlayerAsync Thread onSuccess → main apply
pub fn import_llm_speech_results_from_share(state: &mut SimState) {
    let Some(share) = state.llm_speech_io.clone() else {
        return;
    };
    for r in crate::ai_handler::take_completed_llm_results_from_share(&share) {
        push_llm_speech_result(state, r);
    }
}

/// Export pending LLM speech jobs onto the outer HTTP share.
// Haxe: respondToPlayerAsync Thread.create payload handoff
pub fn export_llm_speech_jobs_to_share(state: &mut SimState) {
    let Some(share) = state.llm_speech_io.clone() else {
        return;
    };
    let jobs = take_llm_speech_jobs(state);
    if jobs.is_empty() {
        return;
    }
    let n = jobs.len();
    crate::ai_handler::push_pending_llm_jobs_to_share(&share, jobs);
    info!(n, "sim: AI-LLM-HTTP-DRAIN exported speech jobs");
}

/// Live LLM fallback after scripted sayHelper. Skip `scripted_handled` conn_ids.
// Haxe: AiBase.sayHelper L4971 after scripted return
fn fan_out_ai_speech_llm(
    state: &mut SimState,
    outbound: &OutboundHub,
    speaker_conn: u64,
    text: &str,
    scripted_handled: &std::collections::HashSet<u64>,
) {
    let activated = crate::ai_handler::is_llm_activated(
        crate::ai_handler::api_key_from_env().as_deref(),
    );
    fan_out_ai_speech_llm_ex(
        state,
        outbound,
        speaker_conn,
        text,
        scripted_handled,
        activated,
    );
}

/// Testable inner: `llm_activated` is Haxe `AIProvider.IsLLMActivated`.
// Haxe: AIProvider.IsLLMActivated
fn fan_out_ai_speech_llm_ex(
    state: &mut SimState,
    outbound: &OutboundHub,
    speaker_conn: u64,
    text: &str,
    scripted_handled: &std::collections::HashSet<u64>,
    llm_activated: bool,
) {
    use crate::ai_handler::{
        check_if_should_do_command, get_relationship_info, mark_llm_inflight, plan_respond_to_player,
        plan_speech_llm_start, set_waiting_time_min, speech_llm_gate_from_runtime, LlmSpeechJob,
        PromptParts, RelationshipView,
    };
    use crate::ai_takeover::player_is_ai;
    use crate::move_live_gates::is_friendly;
    use crate::relations::{is_close_relative, is_leadership_ally};

    if !llm_activated {
        return;
    }
    let hearers = live_collect_ai_speech_hearers(state, speaker_conn, text);
    if hearers.is_empty() {
        return;
    }
    let Some(speaker) = state.players.get(&speaker_conn).cloned() else {
        return;
    };
    if speaker.deleted || player_is_ai(speaker.connected, speaker.ai_controlled, &speaker.email)
    {
        return;
    }

    let now = state.sim_time;
    let speaker_p_id = speaker.p_id;
    let speaker_name = speaker.first_name.clone();
    let speaker_family = speaker.family_name.clone();

    let mut pending_says: Vec<(u64, i32, i32, i32, String)> = Vec::new();
    let mut pending_emotes: Vec<(u64, i32, i32, i32, i32)> = Vec::new();
    let mut new_jobs: Vec<LlmSpeechJob> = Vec::new();

    for h in hearers {
        if scripted_handled.contains(&h.conn_id) {
            continue;
        }
        let Some(ai_snap) = state.players.get(&h.conn_id).cloned() else {
            continue;
        };
        if ai_snap.deleted || ai_snap.llm_speech.in_flight {
            continue;
        }
        let leadership = is_leadership_ally(&state.social.following, h.p_id, speaker_p_id)
            || state.allies.is_mutual_or_either(h.p_id, speaker_p_id);
        let friendly = is_friendly(
            leadership,
            ai_snap.last_attacked_player_id,
            ai_snap.last_player_attacked_me_id,
            speaker_p_id,
        );
        let gate = speech_llm_gate_from_runtime(
            &ai_snap.llm_speech,
            true,
            llm_activated,
            ai_snap.age,
            &h.normalized_text,
            now,
        );
        let Some(start) = plan_speech_llm_start(&gate, friendly) else {
            continue;
        };

        let own_context = state.player_soul_text(h.p_id).unwrap_or_default();
        let other_context = state
            .player_external_intro(speaker_p_id)
            .unwrap_or_default();
        let memory_context = state.player_soul_memory_text(h.p_id).unwrap_or_default();
        let chat_memory_context = state.player_soul_chat_text(h.p_id).unwrap_or_default();
        let follower = state.social.is_follower_from(h.p_id, speaker_p_id);
        let close_rel = is_close_relative(&state.social, h.p_id, speaker_p_id);
        let do_command_text = check_if_should_do_command(follower, close_rel);
        let relationship_context = get_relationship_info(&RelationshipView {
            is_ally: leadership,
            is_friendly: friendly,
            ..Default::default()
        });
        let parts = PromptParts {
            own_context,
            other_context,
            relationship_context,
            do_command_text,
            memory_context,
            chat_memory_context,
            message: h.normalized_text.clone(),
        };
        let plan = plan_respond_to_player(
            &parts,
            h.p_id,
            speaker_p_id,
            &speaker_name,
            &speaker_family,
        );

        if let Some(ai) = state.players.get_mut(&h.conn_id) {
            mark_llm_inflight(&mut ai.llm_speech, true);
            if start.stop_goto_self {
                // Haxe ally: Goto(self) stop + setWaitingTimeMin(6)
                ai.move_path = None;
                ai.moving = false;
                ai.force_stop_on_next_tile = true;
                set_waiting_time_min(&mut ai.llm_speech, start.ally_wait_secs);
            }
        }
        pending_emotes.push((h.conn_id, h.p_id, ai_snap.x, ai_snap.y, start.emote_id));
        pending_says.push((
            h.conn_id,
            h.p_id,
            ai_snap.x,
            ai_snap.y,
            start.thinking_say.to_string(),
        ));
        new_jobs.push(LlmSpeechJob {
            ai_conn_id: h.conn_id,
            ai_p_id: h.p_id,
            speaker_p_id,
            speaker_name: speaker_name.clone(),
            speaker_family: speaker_family.clone(),
            human_message: h.normalized_text.clone(),
            full_prompt: plan.full_prompt,
            is_ally: friendly,
            enqueued_at: now,
        });
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
        info!(p_id, text = %s, "sim: AI-LLM-FAN thinking SAY");
    }
    state.llm_speech_jobs.extend(new_jobs);
}

/// Apply completed LLM replies + drain delayed SAY chunks + export jobs.
// Haxe: respondToPlayerAsync onSuccess + sendResponseInChunks
fn tick_llm_speech_wire(state: &mut SimState, outbound: &OutboundHub) {
    use crate::ai_follow_walk::ally_goto_speaker_xy;
    use crate::ai_handler::{
        enqueue_llm_say_chunks, mark_llm_inflight, mark_llm_reacted, plan_speech_llm_complete,
        poll_ready_llm_say, set_waiting_time_min, AI_WAIT_TIME_PER_100_CHARS_DEFAULT,
        MAX_AI_RESPONSE_PER_SAY_DEFAULT,
    };
    use crate::ai_llm_apply::{
        apply_sticky_from_plan, get_object_by_name_like, set_drop_waiting_on_runtime,
    };

    import_llm_speech_results_from_share(state);
    let results = std::mem::take(&mut state.llm_speech_results);
    let now = state.sim_time;

    let mut drop_feet: Vec<(u64, i32, i32)> = Vec::new();
    let mut pending_emotes: Vec<(i32, i32, i32, i32)> = Vec::new();
    let mut path_jobs: Vec<(u64, i32, i32)> = Vec::new();

    for res in results {
        if !state.players.contains_key(&res.ai_conn_id) {
            continue;
        }
        let complete = plan_speech_llm_complete(
            res.raw_response.as_deref(),
            res.is_ally,
            MAX_AI_RESPONSE_PER_SAY_DEFAULT,
            AI_WAIT_TIME_PER_100_CHARS_DEFAULT,
        );
        let apply_plan = complete.apply.clone();
        let emote_id = apply_plan.emote_id;
        let name_hits: Vec<(i32, String)> = state
            .content
            .objects
            .iter()
            .filter(|(id, _)| **id > 0)
            .map(|(id, d)| (*id, d.name.clone()))
            .collect();
        let lookup = |search: &str, from_end: bool| -> Option<(i32, String)> {
            let id = get_object_by_name_like(
                name_hits.iter().map(|(i, n)| (*i, n.as_str())),
                search,
                from_end,
            )?;
            let name = name_hits
                .iter()
                .find(|(i, _)| *i == id)
                .map(|(_, n)| n.clone())
                .unwrap_or_default();
            Some((id, name))
        };
        let speaker_xy = state
            .players
            .values()
            .find(|p| !p.deleted && p.p_id == res.speaker_p_id)
            .map(|p| (p.x, p.y));
        if let Some(ai) = state.players.get_mut(&res.ai_conn_id) {
            mark_llm_inflight(&mut ai.llm_speech, false);
            if complete.record_chat_memory {
                enqueue_llm_say_chunks(
                    &mut ai.llm_speech,
                    &complete.say_chunks,
                    &complete.wait_secs_per_chunk,
                    now,
                );
                mark_llm_reacted(&mut ai.llm_speech, now);
            }
            let mut wait_floor = ai.llm_speech.waiting_time_min;
            let _applied = apply_sticky_from_plan(
                &apply_plan,
                res.speaker_p_id,
                now,
                &mut ai.craft_ai,
                &mut ai.ai_follow_p_id,
                &mut ai.ai_auto_stop_follow,
                &mut ai.ai_follow_started_sim_time,
                &mut ai.ai_ordered_to_drop,
                &mut wait_floor,
                lookup,
            );
            if wait_floor > ai.llm_speech.waiting_time_min {
                set_waiting_time_min(&mut ai.llm_speech, wait_floor);
            }
            if apply_plan.drop {
                set_drop_waiting_on_runtime(&mut ai.llm_speech);
                ai.move_path = None;
                ai.moving = false;
                if ai.held_id != 0 {
                    drop_feet.push((res.ai_conn_id, ai.x, ai.y));
                }
            }
            if apply_plan.follow_player || complete.goto_speaker {
                ai.force_stop_on_next_tile = false;
                if let Some((sx, sy)) = speaker_xy {
                    let (gx, gy) = ally_goto_speaker_xy(sx, sy);
                    path_jobs.push((res.ai_conn_id, gx, gy));
                }
            }
        }
        if complete.record_chat_memory && !complete.reply_text.is_empty() {
            let _ = state.add_player_soul_chat_entry(
                res.ai_p_id,
                res.speaker_p_id,
                &res.speaker_name,
                &res.speaker_family,
                &res.human_message,
                &complete.reply_text,
            );
        }
        if let Some(eid) = emote_id {
            if let Some(ai) = state.players.get(&res.ai_conn_id) {
                pending_emotes.push((ai.p_id, ai.x, ai.y, eid));
            }
        }
    }

    for (p_id, x, y, eid) in pending_emotes {
        let pe = format_player_emot(p_id, eid).into_bytes();
        let near = nearby_conn_ids(state, x, y, nearby_range(state));
        send_nearby(outbound, &near, pe);
        for &nid in &near {
            send_frame(outbound, nid);
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

    let mut ready: Vec<(u64, i32, i32, i32, String)> = Vec::new();
    let now = state.sim_time;
    for (&cid, ai) in state.players.iter_mut() {
        if ai.deleted {
            continue;
        }
        if let Some(text) = poll_ready_llm_say(&mut ai.llm_speech, now) {
            ready.push((cid, ai.p_id, ai.x, ai.y, text));
        }
    }
    for (cid, p_id, x, y, text) in ready {
        let near = nearby_conn_ids(state, x, y, crate::say_close_range(state, 20.0));
        send_chat_ps(state, outbound, cid, p_id, &text, &near);
        info!(p_id, text = %text, "sim: AI-LLM-WIRE chunk SAY");
    }
    export_llm_speech_jobs_to_share(state);
}
