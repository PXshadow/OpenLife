//! Build-time wire for **AI-SAY-HELPER** / `scripted_cmds`.
//!
//! Pure module lives at `ai_llm_apply::ai_say_helper` (nested like AI-FOLLOW-WALK).
//! This patches player fields + live `fan_out_ai_say_scripted` + docs.
//! Idempotent. Handles CRLF.

use std::path::Path;

fn normalize_nl(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

fn restore_nl(s: &str, crlf: bool) -> String {
    if crlf {
        s.replace('\n', "\r\n")
    } else {
        s.to_string()
    }
}

fn write_if_changed(path: &Path, crlf: bool, text: &str) -> bool {
    let out = restore_nl(text, crlf);
    let prev = std::fs::read_to_string(path).unwrap_or_default();
    if prev == out {
        return false;
    }
    std::fs::write(path, out).is_ok()
}

/// True when AI-SAY-HELPER is fully wired.
pub fn say_helper_wired(lib: &str, player: &str, apply_mod: &str, pure: &str) -> bool {
    pure.contains("fn plan_scripted_say_helper")
        && apply_mod.contains("mod ai_say_helper")
        && player.contains("ai_debug_say")
        && player.contains("ai_is_nice_baby")
        && lib.contains("fan_out_ai_say_scripted")
        && lib.contains("AI-SAY-HELPER")
}

/// Patch all surfaces. Returns true when fully ready.
pub fn patch_ai_say_helper(src_dir: &Path, workspace: &Path) -> bool {
    let _ = patch_player(&src_dir.join("player.rs"));
    let _ = patch_lib(&src_dir.join("lib.rs"));
    let _ = patch_apply_mod(&src_dir.join("ai_llm_apply.rs"));
    patch_docs(workspace);

    let lib = std::fs::read_to_string(src_dir.join("lib.rs")).unwrap_or_default();
    let player = std::fs::read_to_string(src_dir.join("player.rs")).unwrap_or_default();
    let apply_mod = std::fs::read_to_string(src_dir.join("ai_llm_apply.rs")).unwrap_or_default();
    let pure = std::fs::read_to_string(src_dir.join("ai_say_helper.rs")).unwrap_or_default();
    say_helper_wired(&lib, &player, &apply_mod, &pure)
}

fn patch_apply_mod(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    if t.contains("mod ai_say_helper") || t.contains("ai_say_helper.rs") {
        return true;
    }
    // Insert after ai_follow_walk block if present
    let insert = "\n// Haxe: AiBase.sayHelper scripted cmds (AI-SAY-HELPER / scripted_cmds)\n#[path = \"ai_say_helper.rs\"]\npub mod ai_say_helper;\npub use ai_say_helper::*;\n";
    if t.contains("pub use ai_follow_walk::*;\n") {
        t = t.replacen(
            "pub use ai_follow_walk::*;\n",
            &format!("pub use ai_follow_walk::*;\n{insert}"),
            1,
        );
        write_if_changed(path, crlf, &t)
    } else if t.contains("use crate::craft_ai_sticky::PlayerCraftAi;\n") {
        t = t.replacen(
            "use crate::craft_ai_sticky::PlayerCraftAi;\n",
            &format!("use crate::craft_ai_sticky::PlayerCraftAi;\n{insert}"),
            1,
        );
        write_if_changed(path, crlf, &t)
    } else {
        false
    }
}

fn patch_player(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("ai_debug_say") {
        let anchor = "/// Haxe `AiBase.orderedToDrop` — next AI tick dropHeldObject(0).\n    // Haxe: AiBase.orderedToDrop\n    pub ai_ordered_to_drop: bool,\n}";
        let insert = "/// Haxe `AiBase.orderedToDrop` — next AI tick dropHeldObject(0).\n    // Haxe: AiBase.orderedToDrop\n    pub ai_ordered_to_drop: bool,\n    /// Haxe `AiBase.debugSay` (AI-SAY-HELPER DEBUG ON/OFF).\n    // Haxe: AiBase.debugSay\n    pub ai_debug_say: bool,\n    /// Haxe `AiBase.debugProfession` (PROF ON/OFF).\n    // Haxe: AiBase.debugProfession\n    pub ai_debug_profession: bool,\n    /// Haxe `AiBase.isNiceBaby` (NICE? reply).\n    // Haxe: AiBase.isNiceBaby\n    pub ai_is_nice_baby: bool,\n}";
        if t.contains(anchor) {
            t = t.replacen(anchor, insert, 1);
            changed = true;
        }
    }

    if t.contains("ai_debug_say") && !t.contains("ai_debug_say: false,") {
        let anchor = "ai_ordered_to_drop: false,\n        }\n    }";
        let insert = "ai_ordered_to_drop: false,\n            // AI-SAY-HELPER: debugSay / debugProfession / isNiceBaby\n            ai_debug_say: false,\n            ai_debug_profession: false,\n            ai_is_nice_baby: true,\n        }\n    }";
        if t.contains(anchor) {
            t = t.replacen(anchor, insert, 1);
            changed = true;
        }
    }

    if changed {
        write_if_changed(path, crlf, &t)
    } else {
        t.contains("ai_debug_say") && t.contains("ai_is_nice_baby")
    }
}

fn patch_lib(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    // Expand pub use ai_llm_apply with say-helper symbols (idempotent)
    if t.contains("pub use ai_llm_apply::{") && !t.contains("plan_scripted_say_helper") {
        let old = "pub use ai_llm_apply::{\n    apply_drop_waiting_floor, apply_plan_has_work, apply_sticky_from_plan,\n    get_object_by_name_like, make_item_search_token, normalize_make_item_search,\n    plan_do_drop_command, plan_start_following_player, resolve_make_item_alias,\n    resolve_make_item_id, set_drop_waiting_on_runtime, AppliedAiResponseSticky,\n    DropCommandPlan, StartFollowPlan, MAKE_ITEM_ALIASES,\n};";
        let new = "pub use ai_llm_apply::{\n    apply_drop_waiting_floor, apply_plan_has_work, apply_sticky_from_plan,\n    apply_scripted_waiting, create_profession_text, get_object_by_name_like,\n    make_item_search_token, normalize_make_item_search, normalize_profession_token,\n    plan_ally_gate, plan_do_drop_command, plan_scripted_say_helper, plan_should_do_command,\n    plan_start_following_player, profession_is_known, resolve_make_item_alias,\n    resolve_make_item_id, scripted_cooldown_ok, set_drop_waiting_on_runtime,\n    AppliedAiResponseSticky, DropCommandPlan, ScriptedSayCtx, ScriptedSayPlan,\n    StartFollowPlan, AI_PROFESSIONS, ARE_YOU_AI_REPLIES, GO_HOME_FAR_TIME_BUMP,\n    GO_HOME_NEAR_QUAD, GO_HOME_NEAR_TIME_BUMP, HOLA_WAITING_TIME_ADD, MAKE_ITEM_ALIASES,\n    NOT_FOLLOWER_SAY, SCRIPTED_CMD_COOLDOWN_SECS, STOP_WAITING_TIME,\n};";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if !t.contains("fn fan_out_ai_say_scripted") {
        let marker =
            "/// AI-LLM-WIRE: Haxe `Connection.sendSayToAllClose` AI loop → `AiBase.say` LLM fallback.";
        if t.contains(marker) {
            t = t.replacen(marker, &format!("{FAN_OUT_SCRIPTED_FN}\n\n{marker}"), 1);
            changed = true;
        }
    }

    if t.contains("fan_out_ai_speech_llm(state, outbound, conn_id, chat_body);")
        && !t.contains("fan_out_ai_say_scripted(state, outbound, conn_id, chat_body)")
    {
        let old = "fan_out_ai_speech_llm(state, outbound, conn_id, chat_body);";
        let new = "let scripted_handled = fan_out_ai_say_scripted(state, outbound, conn_id, chat_body);\n        fan_out_ai_speech_llm(state, outbound, conn_id, chat_body, &scripted_handled);";
        t = t.replacen(old, new, 1);
        changed = true;
    }

    if t.contains("fn fan_out_ai_speech_llm(")
        && !t.contains("scripted_handled: &std::collections::HashSet<u64>")
    {
        let old_sig = "fn fan_out_ai_speech_llm(\n    state: &mut SimState,\n    outbound: &OutboundHub,\n    speaker_conn: u64,\n    text: &str,\n) {";
        let new_sig = "fn fan_out_ai_speech_llm(\n    state: &mut SimState,\n    outbound: &OutboundHub,\n    speaker_conn: u64,\n    text: &str,\n    scripted_handled: &std::collections::HashSet<u64>,\n) {";
        if t.contains(old_sig) {
            t = t.replacen(old_sig, new_sig, 1);
            changed = true;
        }
    }

    if t.contains("scripted_handled: &std::collections::HashSet<u64>")
        && !t.contains("scripted_handled.contains(&h.conn_id)")
    {
        let old = "for h in hearers {\n        // Skip if already awaiting LLM\n        if state\n            .players\n            .get(&h.conn_id)\n            .map(|p| p.llm_speech.in_flight)\n            .unwrap_or(true)\n        {\n            continue;\n        }";
        let new = "for h in hearers {\n        // AI-SAY-HELPER: skip AIs that already handled a scripted command\n        if scripted_handled.contains(&h.conn_id) {\n            continue;\n        }\n        // Skip if already awaiting LLM\n        if state\n            .players\n            .get(&h.conn_id)\n            .map(|p| p.llm_speech.in_flight)\n            .unwrap_or(true)\n        {\n            continue;\n        }";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if changed {
        write_if_changed(path, crlf, &t)
    } else {
        t.contains("fan_out_ai_say_scripted")
    }
}

const FAN_OUT_SCRIPTED_FN: &str = r#"/// AI-SAY-HELPER: Haxe `AiBase.sayHelper` scripted commands (HOLA/FOLLOW/MAKE/…).
///
/// Runs for every human SAY (even when LLM is off). Returns conn_ids of AIs that
/// matched a scripted command so the LLM path can skip them.
// Haxe: AiBase.sayHelper L4755–4970
fn fan_out_ai_say_scripted(
    state: &mut SimState,
    outbound: &OutboundHub,
    speaker_conn: u64,
    text: &str,
) -> std::collections::HashSet<u64> {
    let mut handled = std::collections::HashSet::new();
    let Some(speaker) = state.players.get(&speaker_conn).cloned() else {
        return handled;
    };
    if speaker.deleted {
        return handled;
    }
    if player_is_ai(speaker.connected, speaker.ai_controlled, &speaker.email) {
        return handled;
    }

    let speaker_view = AiSpeechPlayerView {
        conn_id: speaker_conn,
        p_id: speaker.p_id,
        x: speaker.x,
        y: speaker.y,
        name: speaker.first_name.clone(),
        is_ai: false,
        age: speaker.age,
    };
    let others: Vec<AiSpeechPlayerView> = state
        .players
        .iter()
        .filter(|(_, p)| !p.deleted)
        .map(|(&cid, p)| AiSpeechPlayerView {
            conn_id: cid,
            p_id: p.p_id,
            x: p.x,
            y: p.y,
            name: p.first_name.clone(),
            is_ai: player_is_ai(p.connected, p.ai_controlled, &p.email),
            age: p.age,
        })
        .collect();
    let hearers = collect_ai_speech_hearers(
        &speaker_view,
        text,
        &others,
        MAX_DISTANCE_SAY_AI,
    );
    if hearers.is_empty() {
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

    let name_hits: Vec<(i32, String)> = {
        let c = state.content.as_ref();
        c.objects
            .iter()
            .filter(|(id, _)| **id > 0)
            .map(|(id, d)| (*id, d.name.clone()))
            .collect()
    };

    let mut drop_feet: Vec<(u64, i32, i32)> = Vec::new();
    let mut pending_says: Vec<(u64, i32, i32, i32, String)> = Vec::new();
    let mut pending_emotes: Vec<(u64, i32, i32)> = Vec::new();

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
        if let Some(ai) = state.players.get_mut(&h.conn_id) {
            if plan.mark_reacted {
                mark_llm_reacted(&mut ai.llm_speech, now);
            }
            let wait = apply_scripted_waiting(ai.llm_speech.waiting_time_min, &plan);
            if wait > ai.llm_speech.waiting_time_min {
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
                ai.assigned_profession = assign.clone();
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
            if plan.search_new_home {
                let mut ovens: Vec<(i32, i32, bool)> = Vec::new();
                let r = 40;
                if let Ok(w) = state.world.read() {
                    for dy in -r..=r {
                        for dx in -r..=r {
                            let tx = ai.x + dx;
                            let ty = ai.y + dy;
                            let id = w.get_object(tx, ty);
                            if is_home_oven_id(id) {
                                let floor = w.get_floor(tx, ty) != 0;
                                ovens.push((tx, ty, floor));
                            }
                        }
                    }
                }
                if let Some((hx, hy)) =
                    pick_nearest_home_oven(ai.x, ai.y, &ovens, HOME_SEARCH_MAX_QUAD)
                {
                    let oid = state
                        .world
                        .read()
                        .map(|w| w.get_object(hx, hy))
                        .unwrap_or(0);
                    let new_name = state
                        .content
                        .get(oid)
                        .map(|d| d.name.clone())
                        .unwrap_or_else(|| "home".into());
                    if ai.home_x != hx || ai.home_y != hy {
                        extra_say = Some(format!("Have a new home! {new_name}"));
                    } else {
                        extra_say = Some(format!("No mew home! {new_name}"));
                    }
                    ai.home_x = hx;
                    ai.home_y = hy;
                }
            }
            if plan.jump {
                ai.done_moving_seq = ai.done_moving_seq.saturating_add(1).max(1);
            }
            if plan.goto_speaker_offset {
                ai.force_stop_on_next_tile = false;
            }
        }

        let say_text = extra_say.or(plan.say);
        if let Some(s) = say_text {
            if let Some(ai) = state.players.get(&h.conn_id) {
                pending_says.push((h.conn_id, ai.p_id, ai.x, ai.y, s));
            }
        }
        if let Some(eid) = plan.emote_id {
            pending_emotes.push((h.conn_id, h.p_id, eid));
        }
    }

    for (cid, p_id, eid) in pending_emotes {
        outbound.send_urgent(cid, format_player_emot(p_id, eid).into_bytes());
        send_frame(outbound, cid);
    }
    for (cid, p_id, x, y, s) in pending_says {
        let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);
        send_chat_ps(state, outbound, cid, p_id, &s, &near);
        info!(p_id, text = %s, "sim: AI-SAY-HELPER scripted SAY");
    }
    for (cid, x, y) in drop_feet {
        apply_drop(state, outbound, cid, x, y, None);
        if let Some(ai) = state.players.get_mut(&cid) {
            ai.ai_ordered_to_drop = false;
        }
    }
    handled
}"#;

fn patch_docs(workspace: &Path) {
    let port = workspace.join("docs/port");

    let fm = port.join("FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&fm) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if !t.contains("| **AI-SAY-HELPER**") {
            if let Some(idx) = t.find("| **AI-LLM-APPLY**") {
                if let Some(end) = t[idx..].find('\n') {
                    let row = "| **AI-SAY-HELPER** | `AiBase.sayHelper` HOLA/NAME?/FOLLOW/STOP/DROP/MAKE/PROF | — | `ol-sim/ai_say_helper.rs` + `fan_out_ai_say_scripted` + Player debug/nice | **DONE** (scripted_cmds) | Pure plan + live fan-out before LLM; ally/follower gates; MAKE→craft_ai; residual: ally Goto pathfind walk, continuous follow tick, full jump BW |\n";
                    t.insert_str(idx + end + 1, row);
                }
            }
        }
        t = t.replace(
            "residual: scripted cmds (**AI-SAY-HELPER**), ally Goto pathfind",
            "scripted cmds → **AI-SAY-HELPER DONE**; residual: ally Goto pathfind",
        );
        t = t.replace(
            "residual: ally Goto pathfind, follow walk tick, scripted sayHelper",
            "residual: ally Goto pathfind, follow walk tick; scripted → **AI-SAY-HELPER**",
        );
        let _ = write_if_changed(&fm, crlf, &t);
    }

    let todo = port.join("TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if !t.contains("AI-SAY-HELPER scripted_cmds DONE") {
            let line = "- [x] **AI-SAY-HELPER scripted_cmds DONE** — pure `plan_scripted_say_helper` HOLA/NAME?/AI?/NICE?/JUMP/MOVE/FOLLOW/STOP/DROP/GO HOME/MAKE/CRAFT/DEBUG/PROF/profession! + live `fan_out_ai_say_scripted` before LLM; ally/follower gates; residual: Goto pathfind walk, continuous follow tick, full jump BW\n";
            if let Some(idx) = t.find("**AI-LLM-APPLY llm_actions DONE**") {
                if let Some(end) = t[idx..].find('\n') {
                    t.insert_str(idx + end + 1, line);
                }
            }
        }
        let old_dash = "residual: scripted sayHelper + ally Goto pathfind |";
        let new_dash =
            "**AI-SAY-HELPER** scripted DONE; residual: ally Goto pathfind + follow walk tick |";
        if t.contains(old_dash) {
            t = t.replace(old_dash, new_dash);
        }
        if !t.contains("**AI-SAY-HELPER scripted_cmds**") {
            if let Some(idx) = t.find("| 2026-07-29 | **AI-LLM-APPLY") {
                let row = "| 2026-07-29 | **AI-SAY-HELPER scripted_cmds**: pure `plan_scripted_say_helper` + live `fan_out_ai_say_scripted` HOLA/NAME/FOLLOW/STOP/DROP/MAKE/PROF before LLM; Player ai_debug_*/is_nice_baby; tests ai_say_helper::*; residual Goto pathfind / follow walk |\n";
                t.insert_str(idx, row);
            }
        }
        let _ = write_if_changed(&todo, crlf, &t);
    }

    let q = port.join("QUEUE.md");
    if let Ok(raw) = std::fs::read_to_string(&q) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        t = t.replace(
            "| `AI-SAY-HELPER` | scripted_cmds | sayHelper scripted cmds (wf-108) |",
            "| ~~`AI-SAY-HELPER`~~ | scripted_cmds | **DONE** HOLA/FOLLOW/MAKE/PROF live |",
        );
        t = t.replace(
            "| `AI-SAY-HELPER` | scripted_cmds | Scripted sayHelper HOLA/NAME?/FOLLOW/… |",
            "| ~~`AI-SAY-HELPER`~~ | scripted_cmds | **DONE** HOLA/FOLLOW/MAKE/PROF live |",
        );
        if t.contains("**AI-LLM-APPLY** DONE") && !t.contains("**AI-SAY-HELPER** DONE") {
            t = t.replacen(
                "**AI-LLM-APPLY** DONE",
                "**AI-SAY-HELPER** DONE · **AI-LLM-APPLY** DONE",
                1,
            );
        }
        let _ = write_if_changed(&q, crlf, &t);
    }

    let ci = port.join("CALL_INDEX.md");
    if let Ok(raw) = std::fs::read_to_string(&ci) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if !t.contains("### AI-SAY-HELPER") {
            let block = r#"
### AI-SAY-HELPER (`AiBase.sayHelper` scripted_cmds)

| Haxe | Rust | Notes |
|------|------|-------|
| `sayHelper` HOLA/HELLO/HI | `plan_scripted_say_helper` + `fan_out_ai_say_scripted` | weapon/angry gates + cooldown 4s |
| NAME? / ARE YOU AI / NICE? / JUMP! | same | |
| FOLLOW/COME / STOP FOLLOW / STOP/WAIT / DROP | sticky `ai_follow_*` / `ai_ordered_to_drop` | follower or ally gates |
| MAKE/CRAFT | `resolve_make_item_id` + `craft_ai.do_make_craft_command` | ally gate; non-silent say |
| PROF?/PROF ON / profession! | `create_profession_text` / `AI_PROFESSIONS` | assigned_profession |
| checkIfYouAreAllied / checkIfShouldDoCommand | `plan_ally_gate` / `plan_should_do_command` | loud reject + angry PE |
| Residual | — | ally Goto pathfind walk; continuous follow tick; full jump BW |

"#;
            if let Some(idx) = t.find("### AI-LLM-APPLY") {
                let rest = &t[idx + 1..];
                if let Some(e2) = rest.find("\n### ") {
                    t.insert_str(idx + 1 + e2 + 1, block);
                } else {
                    t.push_str(block);
                }
            } else {
                t.push_str(block);
            }
        }
        t = t.replace(
            "scripted cmds (**AI-SAY-HELPER**); ally Goto pathfind",
            "**AI-SAY-HELPER DONE**; residual ally Goto pathfind",
        );
        let _ = write_if_changed(&ci, crlf, &t);
    }
}
