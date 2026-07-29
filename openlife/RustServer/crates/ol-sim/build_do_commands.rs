//! DO-COMMANDS / say_commands build-time wire (included from build.rs).
//!
//! Source is already wired for DO-COMMANDS + LEADER-RANGE; this module keeps
//! the build.rs hook and piggybacks **FOODSTATS-DISK** (FoodStats.txt dump),
//! **FOLLOW-HIRE-DELAY** (live TimeConfirm + HireCost; hire immediate),
//! **MAP-LOCATION-PINS** (social map pins), **PO-FAR-PLAYERS** (PO fan),
//! **PATH-REACH** (live notReachableObjects / hostile / blockedByAI),
//! **PATH-REACH-MERGE** (Player ↔ NPC dual_map_merge each tick),
//! **BLOCKED-BY-AI** (live CalculateBlockedByAi sticky rebuild),
//! **AI-HANDLER** (llm_prompt pure rate-limit/prompt/parse/log),
//! **AI-PROVIDER** (llm_http MiniMax/Anthropic pure body+parse + ol-server HTTP),
//! **AI-LLM-HTTP-DRAIN** (ol-server take→call_ai_async→push speech jobs),
//! **AI-LLM-APPLY** (live ApplyAiResponsePlan follow/drop/make/emote),
//! **AI-SAY-HELPER** (scripted sayHelper HOLA/FOLLOW/MAKE/PROF before LLM),
//! **TWIN-PARTY-RESID** (same-server twin heart-link / male / wait timeout),
//! and **PO-MAX-DISTANCE** (CloseForSay 20 vs NEARBY 24).
//!
//! Idempotent.

// Piggyback FOODSTATS-DISK without editing build.rs mod list.
#[path = "build_foodstats_disk.rs"]
mod foodstats_disk;

// Piggyback FOLLOW-HIRE-DELAY / hire_confirm.
#[path = "build_follow_hire_delay.rs"]
mod follow_hire_delay;

// Piggyback MAP-LOCATION-PINS / social_pins.
#[path = "build_map_location_pins.rs"]
mod map_location_pins;

// Piggyback PO-FAR-PLAYERS / player_out_of_range.
#[path = "build_po_far_players.rs"]
mod po_far_players;

// Piggyback PATH-REACH / not_reachable_maps.
#[path = "build_path_reach.rs"]
mod path_reach;

// Piggyback PATH-REACH-MERGE / dual_map_merge (Player ↔ NPC path maps).
#[path = "build_path_reach_merge.rs"]
mod path_reach_merge;

// Piggyback BLOCKED-BY-AI / blocked_rebuild sticky live rebuild.
#[path = "build_blocked_by_ai.rs"]
mod blocked_by_ai;

// Piggyback AI-HANDLER / llm_prompt pure module wire.
#[path = "build_ai_handler.rs"]
mod ai_handler_wire;

// Piggyback AI-PROVIDER / llm_http pure + ol-server HTTP wire.
#[path = "build_ai_provider.rs"]
mod ai_provider_wire;

// Piggyback AI-LLM-HTTP-DRAIN / llm_server_drain (take jobs → call_ai_async → push).
#[path = "build_ai_llm_http_drain.rs"]
mod ai_llm_http_drain_wire;

// Piggyback AI-LLM-APPLY / llm_actions (live ApplyAiResponsePlan follow/drop/make).
#[path = "build_ai_llm_apply.rs"]
mod ai_llm_apply_wire;

// Piggyback AI-SAY-HELPER / scripted_cmds (AiBase.sayHelper HOLA/FOLLOW/MAKE…).
#[path = "build_ai_say_helper.rs"]
mod ai_say_helper_wire;

// Piggyback TWIN-PARTY-RESID / twin_wait_edges (same-server residual; no multi-server).
#[path = "build_twin_party_resid.rs"]
mod twin_party_resid_wire;

// Piggyback PO-MAX-DISTANCE / close_say_range (CloseForSay 20).
#[path = "build_po_max_distance.rs"]
mod po_max_distance;

use std::path::Path;
use std::process::Command;

pub fn do_commands_wired(lib: &str) -> bool {
    lib.contains("mod do_commands_wire;")
        && lib.contains("DO-COMMANDS")
        && lib.contains("apply_do_commands_live")
        && lib.contains("parse_do_command")
}

fn leader_range_wired(lib: &str) -> bool {
    lib.contains("mod leader_range;")
        && lib.contains("nearby_conn_ids_for_player_update")
        && lib.contains("parse_leader_personal_command")
}

/// Build hook: verify DO-COMMANDS source wire + always run FOODSTATS-DISK + FOLLOW-HIRE-DELAY + MAP-LOCATION-PINS + PO-FAR-PLAYERS + PATH-REACH + PATH-REACH-MERGE + BLOCKED-BY-AI + AI-HANDLER + AI-PROVIDER + AI-LLM-HTTP-DRAIN + AI-LLM-APPLY + AI-SAY-HELPER + TWIN-PARTY-RESID + PO-MAX-DISTANCE.
pub fn patch_do_commands(_manifest: &Path, src: &Path, workspace: &Path) -> bool {
    // Optional python docs/finish for FOODSTATS-DISK
    let py_docs = workspace.join("docs/port/_patch_foodstats_docs.py");
    if py_docs.exists() {
        let _ = Command::new("python")
            .arg(&py_docs)
            .status()
            .or_else(|_| Command::new("python3").arg(&py_docs).status());
    }
    let py_wire = src
        .parent()
        .map(|p| p.join("_patch_foodstats_disk.py"))
        .unwrap_or_else(|| Path::new("_patch_foodstats_disk.py").to_path_buf());
    if py_wire.exists() {
        let _ = Command::new("python")
            .arg(&py_wire)
            .status()
            .or_else(|_| Command::new("python3").arg(&py_wire).status());
    }

    // FOLLOW-HIRE-DELAY / hire_confirm — live TimeConfirm + HireCost*
    // Haxe: processFollowCommand delayed; processHireCommand immediate
    let fhd_ok = follow_hire_delay::patch_follow_hire_delay(src, workspace);
    if !fhd_ok {
        println!(
            "cargo:warning=FOLLOW-HIRE-DELAY: could not fully wire TimeConfirm/HireCost live knobs"
        );
    }

    // MAP-LOCATION-PINS / social_pins — MOTHER/BABY/FOLLOWER/HUMAN/ALLY/FAM
    // Haxe: Connection.sendMapLocation + CountAndDisplay*
    let mlp_ok = map_location_pins::patch_map_location_pins(src, workspace);
    if !mlp_ok {
        println!(
            "cargo:warning=MAP-LOCATION-PINS: could not fully wire social map pins into lib/player"
        );
    } else {
        let stamp = src.join(".map_location_pins_patched");
        let _ = std::fs::write(&stamp, b"map-location-pins-1-rs-patched\n");
    }

    // PO-FAR-PLAYERS / player_out_of_range — SendToMeAllClosePlayers PO fan
    // Haxe: Connection.SendToMeAllClosePlayers + sendToMePlayerInfo L429
    let po_ok = po_far_players::patch_po_far_players(src, workspace);
    if !po_ok {
        println!(
            "cargo:warning=PO-FAR-PLAYERS: could not wire send_to_me_all_close_players into LOGIN"
        );
    }

    // PATH-REACH / not_reachable_maps — AiBase notReachableObjects / hostile / blockedByAI
    // Haxe: AiBase L85–86 + cleanupBlockedObjects + isObjectNotReachable
    let pr_ok = path_reach::patch_path_reach(src, workspace);
    if !pr_ok {
        println!(
            "cargo:warning=PATH-REACH: could not fully wire live notReachable/hostile maps"
        );
    }

    // PATH-REACH-MERGE / dual_map_merge — Player.ai_path_reach ↔ NpcProfessionState.path_reach
    // Haxe: single AiBase maps L85–86; Rust dual ownership max-merge each tick
    // Also runs docs/port/_apply_path_reach_merge.py via pure RS + python fallback.
    let prm_ok = path_reach_merge::patch_path_reach_merge(src, workspace);
    if !prm_ok {
        println!(
            "cargo:warning=PATH-REACH-MERGE: could not fully wire dual_map_merge (Player↔Npc)"
        );
    } else {
        let stamp = src.join(".path_reach_merge_patched");
        let _ = std::fs::write(&stamp, b"path-reach-merge-1-rs-patched\n");
    }
    // Always try python apply as well (idempotent; catches any RS anchor miss).
    let py_merge = workspace.join("docs/port/_apply_path_reach_merge.py");
    if py_merge.exists() {
        let _ = Command::new("python")
            .arg(&py_merge)
            .status()
            .or_else(|_| Command::new("python3").arg(&py_merge).status());
    }

    // BLOCKED-BY-AI / blocked_rebuild — sticky food/use/drop → live CalculateBlockedByAi
    // Haxe: AiBase.CalculateBlockedByAi ~222–239
    let bba_ok = blocked_by_ai::patch_blocked_by_ai_live(src, workspace);
    if !bba_ok {
        println!(
            "cargo:warning=BLOCKED-BY-AI: could not fully wire sticky CalculateBlockedByAi rebuild"
        );
    }

    // AI-HANDLER / llm_prompt — rate limit, buildPrompt, parse, log, respond async pure
    // Haxe: openlife.server.AiHandler (S-AIH); secrets env-only; no multi-server twins
    let aih_ok = ai_handler_wire::patch_ai_handler(src, workspace);
    if !aih_ok {
        println!(
            "cargo:warning=AI-HANDLER: could not fully wire ai_handler pure module into lib.rs"
        );
    }

    // AI-PROVIDER / llm_http — MiniMax/Anthropic request body + parse + ol-server HTTP
    // Haxe: openlife.server.AIProvider; secrets env only
    let aip_ok = ai_provider_wire::patch_ai_provider(src, workspace);
    if !aip_ok {
        println!(
            "cargo:warning=AI-PROVIDER: could not fully wire ai_provider pure module into lib.rs"
        );
    }

    // AI-LLM-HTTP-DRAIN / llm_server_drain — take jobs → call_ai_async → push results
    // Haxe: AiHandler.respondToPlayerAsync Thread.create + AIProvider.callAi
    let drain_ok = ai_llm_http_drain_wire::patch_ai_llm_http_drain(src, workspace);
    if !drain_ok {
        println!(
            "cargo:warning=AI-LLM-HTTP-DRAIN: could not fully wire speech HTTP drain (share + call_ai_async)"
        );
    } else {
        let stamp = src.join(".ai_llm_http_drain_patched");
        let _ = std::fs::write(&stamp, b"ai-llm-http-drain-1-rs-patched\n");
    }

    // AI-LLM-APPLY / llm_actions — live ApplyAiResponsePlan follow/drop/make/emote
    // Haxe: AiHandler.parseAiResponse doEmote/startFollowing/doDrop/doMakeCraft
    let apply_ok = ai_llm_apply_wire::patch_ai_llm_apply(src, workspace);
    if !apply_ok {
        println!(
            "cargo:warning=AI-LLM-APPLY: could not fully wire live ApplyAiResponsePlan (follow/drop/make)"
        );
    } else {
        let stamp = src.join(".ai_llm_apply_patched");
        let _ = std::fs::write(&stamp, b"ai-llm-apply-1-rs-patched\n");
    }

    // AI-SAY-HELPER / scripted_cmds — Haxe AiBase.sayHelper before LLM
    // Haxe: AiBase.sayHelper HOLA/NAME?/FOLLOW/STOP/DROP/MAKE/PROF
    let say_ok = ai_say_helper_wire::patch_ai_say_helper(src, workspace);
    if !say_ok {
        println!(
            "cargo:warning=AI-SAY-HELPER: could not fully wire scripted sayHelper fan-out"
        );
    } else {
        let stamp = src.join(".ai_say_helper_patched");
        let _ = std::fs::write(&stamp, b"ai-say-helper-1-rs-patched\n");
    }

    // TWIN-PARTY-RESID / twin_wait_edges — same-server heart-link + male flag + wait timeout
    // Haxe: Connection.loginHelper TODO twins; OHOL plan #10 murder→broken heart
    // Multi-server twin peers stay parked.
    let twin_ok = twin_party_resid_wire::patch_twin_party_resid(src, workspace);
    if !twin_ok {
        println!(
            "cargo:warning=TWIN-PARTY-RESID: could not fully wire twin_wait_edges into lib.rs"
        );
    } else {
        let stamp = src.join(".twin_party_resid_patched");
        let _ = std::fs::write(&stamp, b"twin-party-resid-1-rs-patched\n");
    }

    // PO-MAX-DISTANCE / close_say_range — Haxe MaxDistanceToBeConsideredAsCloseForSay = 20
    // Keep NEARBY_RANGE=24 for PU/MX; adult chat ADULT_CHAT_RANGE=20; ModuleConst residual.
    let pom_ok = po_max_distance::patch_po_max_distance(src, workspace);
    if !pom_ok {
        println!(
            "cargo:warning=PO-MAX-DISTANCE: could not fully wire CloseForSay=20 chat range"
        );
    } else {
        let stamp = src.join(".po_max_distance_patched");
        let _ = std::fs::write(&stamp, b"po-max-distance-1-rs-patched\n");
    }

    let lib = std::fs::read_to_string(src.join("lib.rs")).unwrap_or_default();
    let ok = do_commands_wired(&lib);
    if !ok {
        println!("cargo:warning=DO-COMMANDS: wire incomplete (apply_do_commands_live / mod)");
    }
    if !leader_range_wired(&lib) {
        println!("cargo:warning=LEADER-RANGE: source missing leader_break markers");
    }

    // FOODSTATS-DISK / foodstats_txt — Haxe writeFoodStatistics → FoodStats.txt
    let fs_ok = foodstats_disk::patch_foodstats_disk(src, workspace);
    if !fs_ok {
        println!(
            "cargo:warning=FOODSTATS-DISK: could not wire FoodStats.txt dump into lib/config/main"
        );
    } else {
        let stamp = src.join(".foodstats_disk_patched");
        let _ = std::fs::write(&stamp, b"foodstats-disk-1-rs-patched\n");
    }

    ok
}
