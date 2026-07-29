//! Build-time wire for **AI-LLM-APPLY** / `llm_actions`.
//!
//! Live `ApplyAiResponsePlan` follow/drop/makeItem/emote after LLM reply.
//! Idempotent. Handles CRLF. Patches player + lib + docs.

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

/// True when AI-LLM-APPLY is fully wired.
pub fn llm_apply_wired(lib: &str, player: &str, apply_mod: &str) -> bool {
    apply_mod.contains("fn apply_sticky_from_plan")
        && apply_mod.contains("fn resolve_make_item_id")
        && player.contains("ai_follow_p_id")
        && player.contains("ai_ordered_to_drop")
        && lib.contains("mod ai_llm_apply;")
        && lib.contains("apply_sticky_from_plan")
        && lib.contains("AI-LLM-APPLY: live follow")
}

/// Patch all surfaces. Returns true when fully ready.
pub fn patch_ai_llm_apply(src_dir: &Path, workspace: &Path) -> bool {
    // Prefer Python apply script when present (full docs + same anchors).
    let py = workspace.join("docs/port/_apply_ai_llm_apply.py");
    if py.exists() {
        let _ = std::process::Command::new("python")
            .arg(&py)
            .status()
            .or_else(|_| std::process::Command::new("python3").arg(&py).status());
    }

    let _ = patch_player(&src_dir.join("player.rs"));
    let _ = patch_lib(&src_dir.join("lib.rs"));
    let _ = patch_ai_handler_comment(&src_dir.join("ai_handler.rs"));
    patch_docs(workspace);

    let lib = std::fs::read_to_string(src_dir.join("lib.rs")).unwrap_or_default();
    let player = std::fs::read_to_string(src_dir.join("player.rs")).unwrap_or_default();
    let apply_mod = std::fs::read_to_string(src_dir.join("ai_llm_apply.rs")).unwrap_or_default();
    llm_apply_wired(&lib, &player, &apply_mod)
}

fn patch_player(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("ai_follow_p_id") {
        let old = "/// Sticky AI food/use/drop/block claims for live CalculateBlockedByAi (**BLOCKED-BY-AI**).\n// Haxe: AiBase.foodTarget / dropTarget / useTarget + GPI.blockTargetForAi\npub ai_block_targets: crate::ai_path_reach::AiStickyBlockTargets,\n}\n";
        let new = "/// Sticky AI food/use/drop/block claims for live CalculateBlockedByAi (**BLOCKED-BY-AI**).\n// Haxe: AiBase.foodTarget / dropTarget / useTarget + GPI.blockTargetForAi\npub ai_block_targets: crate::ai_path_reach::AiStickyBlockTargets,\n    /// Haxe `AiBase.playerToFollow` p_id (0 = none). LLM + scripted FOLLOW.\n    /// Not leadership social.following — AI walk-with target (**AI-LLM-APPLY**).\n    // Haxe: AiBase.playerToFollow\n    pub ai_follow_p_id: i32,\n    /// Haxe `AiBase.autoStopFollow` — true = loose follow / auto clear.\n    // Haxe: AiBase.autoStopFollow\n    pub ai_auto_stop_follow: bool,\n    /// Haxe `AiBase.timeStartedToFolow` as sim_time when ordered follow began.\n    // Haxe: AiBase.timeStartedToFolow\n    pub ai_follow_started_sim_time: f32,\n    /// Haxe `AiBase.orderedToDrop` — next AI tick dropHeldObject(0).\n    // Haxe: AiBase.orderedToDrop\n    pub ai_ordered_to_drop: bool,\n}\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        } else {
            // looser: inject before closing of struct if ai_block_targets present
            let anchor = "pub ai_block_targets: crate::ai_path_reach::AiStickyBlockTargets,\n}\n";
            let insert = "pub ai_block_targets: crate::ai_path_reach::AiStickyBlockTargets,\n    // AI-LLM-APPLY sticky follow/drop\n    pub ai_follow_p_id: i32,\n    pub ai_auto_stop_follow: bool,\n    pub ai_follow_started_sim_time: f32,\n    pub ai_ordered_to_drop: bool,\n}\n";
            if t.contains(anchor) {
                t = t.replacen(anchor, insert, 1);
                changed = true;
            }
        }
    }

    if t.contains("ai_follow_p_id") && !t.contains("ai_follow_p_id: 0,") {
        let old = "// BLOCKED-BY-AI: sticky food/use/drop/block claims\nai_block_targets: crate::ai_path_reach::AiStickyBlockTargets::default(),\n        }\n    }\n";
        let new = "// BLOCKED-BY-AI: sticky food/use/drop/block claims\nai_block_targets: crate::ai_path_reach::AiStickyBlockTargets::default(),\n            // AI-LLM-APPLY: playerToFollow / orderedToDrop sticky\n            ai_follow_p_id: 0,\n            ai_auto_stop_follow: true,\n            ai_follow_started_sim_time: 0.0,\n            ai_ordered_to_drop: false,\n        }\n    }\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        } else {
            let anchor = "ai_block_targets: crate::ai_path_reach::AiStickyBlockTargets::default(),\n        }\n    }\n";
            let insert = "ai_block_targets: crate::ai_path_reach::AiStickyBlockTargets::default(),\n            ai_follow_p_id: 0,\n            ai_auto_stop_follow: true,\n            ai_follow_started_sim_time: 0.0,\n            ai_ordered_to_drop: false,\n        }\n    }\n";
            if t.contains(anchor) && !t.contains("ai_follow_p_id: 0,") {
                t = t.replacen(anchor, insert, 1);
                changed = true;
            }
        }
    }

    if changed {
        write_if_changed(path, crlf, &t)
    } else {
        t.contains("ai_follow_p_id")
    }
}

fn patch_lib(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("mod ai_llm_apply;") {
        if t.contains("mod ai_handler;\n") {
            t = t.replacen("mod ai_handler;\n", "mod ai_handler;\nmod ai_llm_apply;\n", 1);
            changed = true;
        }
    }

    if !t.contains("pub use ai_llm_apply::") {
        let marker = "// Haxe: AIProvider.hx (AI-PROVIDER / S-AIP llm_http) — pure re-exports + config helpers\n";
        let insert = "// Haxe: AiHandler.parseAiResponse live apply (AI-LLM-APPLY / llm_actions)\npub use ai_llm_apply::{\n    apply_drop_waiting_floor, apply_plan_has_work, apply_sticky_from_plan,\n    get_object_by_name_like, make_item_search_token, normalize_make_item_search,\n    plan_do_drop_command, plan_start_following_player, resolve_make_item_alias,\n    resolve_make_item_id, set_drop_waiting_on_runtime, AppliedAiResponseSticky,\n    DropCommandPlan, StartFollowPlan, MAKE_ITEM_ALIASES,\n};\n\n";
        if t.contains(marker) {
            t = t.replacen(marker, &format!("{insert}{marker}"), 1);
            changed = true;
        }
    }

    if !t.contains("AI-LLM-APPLY: live follow") {
        let old = r#"        let emote_id = complete.apply.emote_id;
        // follow/drop/makeItem: pure plan ready; live craft/follow residual
        let _apply = complete.apply;
        if state.players.contains_key(&res.ai_conn_id) {
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
                // Ally Goto speaker residual: clear force-stop so AI can repath later
                if complete.goto_speaker {
                    ai.force_stop_on_next_tile = false;
                }
            }
            if let Some(eid) = emote_id {
                outbound.send_urgent(
                    res.ai_conn_id,
                    format_player_emot(res.ai_p_id, eid).into_bytes(),
                );
                send_frame(outbound, res.ai_conn_id);
            }
        }
"#;
        let new = r#"        let apply_plan = complete.apply.clone();
        let emote_id = apply_plan.emote_id;
        // AI-LLM-APPLY: live follow/drop/makeItem sticky + PE emote
        let mut drop_feet: Option<(u64, i32, i32)> = None;
        if state.players.contains_key(&res.ai_conn_id) {
            // Content name lookup for makeItem (Haxe GetObjectByName)
            let name_hits: Vec<(i32, String)> = {
                let c = state.content.as_ref();
                c.objects
                    .iter()
                    .filter(|(id, _)| **id > 0)
                    .map(|(id, d)| (*id, d.name.clone()))
                    .collect()
            };
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
                // Sticky follow / drop / make from ApplyAiResponsePlan
                // Haxe: parseAiResponse doEmote/startFollowing/doDrop/doMakeCraft
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
                    // Haxe: doDropCommand Goto(self)
                    ai.move_path = None;
                    ai.moving = false;
                    if ai.held_id != 0 {
                        drop_feet = Some((res.ai_conn_id, ai.x, ai.y));
                    }
                }
                if apply_plan.follow_player {
                    // Haxe startFollowingPlayer: clear stop + Goto(speaker) residual path
                    ai.force_stop_on_next_tile = false;
                }
                // Ally Goto speaker residual: clear force-stop so AI can repath later
                if complete.goto_speaker {
                    ai.force_stop_on_next_tile = false;
                }
            }
            if let Some(eid) = emote_id {
                // Haxe: doEmote → SendEmoteToAll (seconds unused)
                outbound.send_urgent(
                    res.ai_conn_id,
                    format_player_emot(res.ai_p_id, eid).into_bytes(),
                );
                send_frame(outbound, res.ai_conn_id);
            }
            // Immediate dropHeldObject(0) when ordered (Haxe next-tick drop; live now)
            if let Some((cid, x, y)) = drop_feet {
                apply_drop(state, outbound, cid, x, y, None);
                if let Some(ai) = state.players.get_mut(&cid) {
                    ai.ai_ordered_to_drop = false;
                }
            }
        }
"#;
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if changed {
        write_if_changed(path, crlf, &t)
    } else {
        t.contains("AI-LLM-APPLY: live follow") && t.contains("mod ai_llm_apply;")
    }
}

fn patch_ai_handler_comment(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let old = "// Apply plan from parseAiResponse (live side-effects described; wire later)\n";
    let new = "// Apply plan from parseAiResponse (pure plan; live → ai_llm_apply / AI-LLM-APPLY)\n";
    if t.contains(old) {
        t = t.replacen(old, new, 1);
        write_if_changed(path, crlf, &t)
    } else {
        true
    }
}

fn patch_docs(workspace: &Path) {
    // FILE_MATRIX
    let fm = workspace.join("docs/port/FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&fm) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if !t.contains("**AI-LLM-APPLY**") || !t.contains("llm_actions") {
            let old = "| **AI-LLM-HTTP-DRAIN** | `AiHandler.respondToPlayerAsync` Thread + `AIProvider.callAi` | — | `ol-server/ai_provider` drain + `main` + `LlmSpeechIoShare` | **DONE** (llm_server_drain) | export/import share + `run_llm_speech_http_drain` take→`call_ai_async`→`logToFile`→push; env secrets + `AI_CONVERSATION_LOG_BASE`; residual apply → **AI-LLM-APPLY** |\n";
            let new = format!("{old}| **AI-LLM-APPLY** | `AiHandler.parseAiResponse` emote/follow/drop/makeItem | — | `ol-sim/ai_llm_apply.rs` + `Player.ai_follow_*` / `ai_ordered_to_drop` + `tick_llm_speech_wire` | **DONE** (llm_actions) | Live sticky follow (speaker)/drop/makeItem resolve + PE emote; immediate DROP feet; residual: ally Goto pathfind, follow walk tick, scripted sayHelper |\n");
            if t.contains(old) {
                t = t.replacen(old, &new, 1);
                let _ = write_if_changed(&fm, crlf, &t);
            }
        }
    }

    // TODO_PORT
    let todo = workspace.join("docs/port/TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if !t.contains("AI-LLM-APPLY llm_actions DONE") {
            let needle = "**AI-LLM-HTTP-DRAIN llm_server_drain DONE**";
            if let Some(pos) = t.find(needle) {
                if let Some(eol) = t[pos..].find('\n') {
                    let insert_at = pos + eol + 1;
                    let line = "- [x] **AI-LLM-APPLY llm_actions DONE** — live ApplyAiResponsePlan follow/drop/make/emote sticky + makeItem resolve + DROP feet; residual ally Goto pathfind + scripted cmds\n";
                    t.insert_str(insert_at, line);
                    let _ = write_if_changed(&todo, crlf, &t);
                }
            }
        }
        // dashboard
        let old_dash = "| LLM AiHandler / AIProvider | ■■ | ■ | | **AI-HANDLER** pure + **AI-PROVIDER** HTTP DONE; **AI-LLM-WIRE** speech core DONE; **AI-LLM-HTTP-DRAIN** DONE (+live logToFile); residual: scripted cmds + action apply |";
        let new_dash = "| LLM AiHandler / AIProvider | ■■■ | ■ | | **AI-HANDLER** pure + **AI-PROVIDER** HTTP DONE; **AI-LLM-WIRE** speech core DONE; **AI-LLM-HTTP-DRAIN** DONE; **AI-LLM-APPLY** live follow/drop/make DONE; residual: scripted sayHelper + ally Goto pathfind |";
        if t.contains(old_dash) {
            t = t.replacen(old_dash, new_dash, 1);
            let _ = write_if_changed(&todo, crlf, &t);
        }
    }

    // QUEUE
    let q = workspace.join("docs/port/QUEUE.md");
    if let Ok(raw) = std::fs::read_to_string(&q) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        let old = "| `AI-LLM-APPLY` | llm_actions | Live ApplyAiResponsePlan follow/drop/make |\n";
        let new = "| ~~`AI-LLM-APPLY`~~ | llm_actions | **DONE** live follow/drop/make/emote |\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            let _ = write_if_changed(&q, crlf, &t);
        }
        if t.contains("**AI-LLM-HTTP-DRAIN** DONE") && !t.contains("**AI-LLM-APPLY** DONE") {
            t = t.replacen(
                "**AI-LLM-HTTP-DRAIN** DONE",
                "**AI-LLM-APPLY** DONE · **AI-LLM-HTTP-DRAIN** DONE",
                1,
            );
            let _ = write_if_changed(&q, crlf, &t);
        }
    }

    // changelog
    let cl = workspace.join("docs/port/changelog/2026-07-29-AI-LLM-APPLY.md");
    if !cl.exists() {
        let body = r#"# AI-LLM-APPLY / llm_actions

**Date:** 2026-07-29
**Mode:** implement
**Status:** DONE (core live apply)

## Scope

Live-wire `ApplyAiResponsePlan` after LLM HTTP result in `tick_llm_speech_wire`:

- emote → PE (`doEmote` seconds unused in Haxe)
- `followPlayer` → sticky `Player.ai_follow_p_id` = **speaker** (Haxe incorrectly passed self)
- `drop` → `ai_ordered_to_drop` + stop + waiting 1s + immediate `apply_drop` at feet
- `makeItem` → resolve id/name/alias → `craft_ai.do_make_craft_command(..., silent=true)`

## Rust

- `ol-sim/src/ai_llm_apply.rs` — pure resolve + sticky apply
- `Player.ai_follow_p_id` / `ai_auto_stop_follow` / `ai_follow_started_sim_time` / `ai_ordered_to_drop`
- `tick_llm_speech_wire` live apply after `plan_speech_llm_complete`

## Intentional deltas

| Topic | Haxe | Rust | Why |
|-------|------|------|-----|
| follow target | `startFollowingPlayer(aiPlayer)` self | speaker p_id | schema "walk with the player"; Haxe bug |
| bare makeItem | `findObjectByCommand` needs ≥2 tokens | bare id/name ok | LLM JSON `makeItem:"knife"` |

## Residual

- Ally Goto(speaker) pathfind (force-stop clear only)
- Continuous follow walk tick (npc uses sticky)
- Scripted sayHelper → **AI-SAY-HELPER**

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- ai_llm_apply -- --test-threads=1
```
"#;
        let _ = std::fs::write(cl, body);
    }
}
