//! Build-time wire for **AI-FOLLOW-WALK** / continuous_follow
//! + **AI-FOLLOW-ACQUIRE** / auto_follow empty-sticky residual.
//!
//! Idempotent patches via Python `_apply_ai_follow_walk.py` /
//! `_apply_ai_follow_acquire.py` + Rust fallbacks.
//! Pure helpers live in `ai_follow_walk.rs` (nested under `ai_llm_apply`).
//! Live tick: `ai_follow_walk_live.inc.rs` included from lib.rs.

use std::path::Path;
use std::process::Command;

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

fn replace_once(hay: &mut String, old: &str, new: &str) -> bool {
    if let Some(i) = hay.find(old) {
        hay.replace_range(i..i + old.len(), new);
        true
    } else {
        false
    }
}

fn write_if_changed(path: &Path, original: &str, next: &str) -> bool {
    if original == next {
        return false;
    }
    if let Err(e) = std::fs::write(path, next) {
        eprintln!(
            "cargo:warning=AI-FOLLOW-WALK write {}: {e}",
            path.display()
        );
        return false;
    }
    true
}

/// True when continuous follow walk is fully wired.
pub fn already_wired(lib: &str, player: &str, follow_mod: &str, npc: &str) -> bool {
    follow_mod.contains("fn decide_follow_walk")
        && follow_mod.contains("fn plan_follow_sticky_clear")
        && follow_mod.contains("fn plan_auto_follow_acquire")
        && lib.contains("tick_ai_follow_walk")
        && lib.contains("try_ai_follow_path_to")
        && lib.contains("decide_follow_walk")
        && lib.contains("tick_ai_follow_acquire")
        && (lib.contains("ai_follow_walk_live.inc.rs") || lib.contains("fn tick_ai_follow_walk"))
        && player.contains("ai_follow_p_id: self.ai_follow_p_id")
        && npc.contains("AI-FOLLOW-WALK")
        && npc.contains("follow_walk")
}

pub fn patch_all(src_dir: &Path, workspace: &Path) -> bool {
    for name in ["_apply_ai_follow_walk.py", "_apply_ai_follow_acquire.py"] {
        let py = src_dir.join(name);
        if py.exists() {
            let st = Command::new("python")
                .arg(&py)
                .current_dir(src_dir)
                .status()
                .or_else(|_| {
                    Command::new("python3")
                        .arg(&py)
                        .current_dir(src_dir)
                        .status()
                });
            if let Ok(s) = st {
                if s.success() {
                    // continue to verify + fill any gaps
                }
            }
        }
    }

    let lib_p = src_dir.join("lib.rs");
    let player_p = src_dir.join("player.rs");
    let follow_p = src_dir.join("ai_follow_walk.rs");
    let npc_p = workspace.join("crates/ol-server/src/npc_ai.rs");

    let _ = patch_lib(&lib_p);
    let _ = patch_acquire_lib(&lib_p);
    let _ = patch_player(&player_p);
    let _ = patch_npc(&npc_p);
    let _ = patch_docs(workspace);
    let _ = patch_acquire_docs(workspace);

    let lib = std::fs::read_to_string(&lib_p).unwrap_or_default();
    let player = std::fs::read_to_string(&player_p).unwrap_or_default();
    let follow = std::fs::read_to_string(&follow_p).unwrap_or_default();
    let npc = std::fs::read_to_string(&npc_p).unwrap_or_default();
    already_wired(&lib, &player, &follow, &npc)
}

fn patch_lib(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("tick_ai_follow_walk")
        && raw.contains("try_ai_follow_path_to")
        && raw.contains("decide_follow_walk")
    {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;

    // pub use follow symbols from ai_llm_apply
    if !t.contains("decide_follow_walk") {
        ch |= replace_once(
            &mut t,
            "    DropCommandPlan, StartFollowPlan, MAKE_ITEM_ALIASES,\n};\n",
            "    DropCommandPlan, StartFollowPlan, MAKE_ITEM_ALIASES,\n    // AI-FOLLOW-WALK continuous isMovingToPlayer / ally Goto\n    ally_goto_speaker_xy, apply_follow_sticky_clear, decide_follow_walk,\n    follow_goal_xy, follow_max_tiles_for_sticky, follow_player_sensor, follow_seed,\n    follow_stand_half_range, follow_walk_holds_tick, ordered_follow_sensor,\n    plan_follow_sticky_clear, truncate_follow_path_steps, AiFollowSticky,\n    FollowStickyClearPlan, FollowTargetSnap, FollowWalkDecision,\n    AUTO_STOP_FOLLOW_CLEAR_AGE, FOLLOW_PATH_STEP_CAP, ORDERED_FOLLOW_MAX_SECS,\n};\n",
        );
    }

    // include live helpers near other includes
    if !t.contains("ai_follow_walk_live.inc.rs") && !t.contains("fn try_ai_follow_path_to") {
        if t.contains("include!(\"twin_party_live.inc.rs\");") {
            ch |= replace_once(
                &mut t,
                "include!(\"twin_party_live.inc.rs\");\n",
                "include!(\"twin_party_live.inc.rs\");\n// AI-FOLLOW-WALK continuous follow + ally Goto pathfind\ninclude!(\"ai_follow_walk_live.inc.rs\");\n",
            );
        } else if t.contains("include!(\"search_best_food_live.inc.rs\");") {
            ch |= replace_once(
                &mut t,
                "include!(\"search_best_food_live.inc.rs\");\n",
                "include!(\"search_best_food_live.inc.rs\");\n// AI-FOLLOW-WALK continuous follow + ally Goto pathfind\ninclude!(\"ai_follow_walk_live.inc.rs\");\n",
            );
        }
    }

    // LLM pathfind after sticky
    if !t.contains("AI-FOLLOW-WALK: startFollowing") {
        let patterns = [
            (
                "                if apply_plan.follow_player {\n                    // Haxe startFollowingPlayer: clear stop + Goto(speaker) residual path\n                    ai.force_stop_on_next_tile = false;\n                }\n                // Ally Goto speaker residual: clear force-stop so AI can repath later\n                if complete.goto_speaker {\n                    ai.force_stop_on_next_tile = false;\n                }\n            }\n            if let Some(eid) = emote_id {",
                "                if apply_plan.follow_player {\n                    // Haxe startFollowingPlayer: clear stop + Goto(speaker)\n                    ai.force_stop_on_next_tile = false;\n                }\n                if complete.goto_speaker {\n                    ai.force_stop_on_next_tile = false;\n                }\n            }\n            // AI-FOLLOW-WALK: startFollowing / ally Goto pathfind to speaker+1\n            // Haxe: startFollowingPlayer Goto(player.tx+1); post-say ally Goto(speaker)\n            if apply_plan.follow_player || complete.goto_speaker {\n                let speaker_xy = state\n                    .players\n                    .values()\n                    .find(|p| p.p_id == res.speaker_p_id && !p.deleted)\n                    .map(|p| (p.x, p.y));\n                if let Some((sx, sy)) = speaker_xy {\n                    let (gx, gy) = ally_goto_speaker_xy(sx, sy);\n                    let _ = try_ai_follow_path_to(state, outbound, res.ai_conn_id, gx, gy);\n                }\n            }\n            if let Some(eid) = emote_id {",
            ),
            (
                "                if apply_plan.follow_player {\n                    // Haxe startFollowingPlayer: clear stop + Goto(speaker) residual path\n                    ai.force_stop_on_next_tile = false;\n                }\n                // Ally Goto speaker residual: clear force-stop so AI can repath later\n                if complete.goto_speaker {\n                    ai.force_stop_on_next_tile = false;\n                }\n            }\n            if let Some(eid) = emote_id {",
                "                if apply_plan.follow_player {\n                    ai.force_stop_on_next_tile = false;\n                }\n                if complete.goto_speaker {\n                    ai.force_stop_on_next_tile = false;\n                }\n            }\n            // AI-FOLLOW-WALK: startFollowing / ally Goto pathfind to speaker+1\n            if apply_plan.follow_player || complete.goto_speaker {\n                let speaker_xy = state\n                    .players\n                    .values()\n                    .find(|p| p.p_id == res.speaker_p_id && !p.deleted)\n                    .map(|p| (p.x, p.y));\n                if let Some((sx, sy)) = speaker_xy {\n                    let (gx, gy) = ally_goto_speaker_xy(sx, sy);\n                    let _ = try_ai_follow_path_to(state, outbound, res.ai_conn_id, gx, gy);\n                }\n            }\n            if let Some(eid) = emote_id {",
            ),
        ];
        for (old, new) in patterns {
            if replace_once(&mut t, old, new) {
                ch = true;
                break;
            }
        }
        // looser: just after force_stop blocks
        if !t.contains("AI-FOLLOW-WALK: startFollowing") {
            if let Some(i) = t.find("if complete.goto_speaker {\n                    ai.force_stop_on_next_tile = false;\n                }\n            }\n            if let Some(eid) = emote_id {")
            {
                let insert = "if complete.goto_speaker {\n                    ai.force_stop_on_next_tile = false;\n                }\n            }\n            // AI-FOLLOW-WALK: startFollowing / ally Goto pathfind to speaker+1\n            if apply_plan.follow_player || complete.goto_speaker {\n                let speaker_xy = state\n                    .players\n                    .values()\n                    .find(|p| p.p_id == res.speaker_p_id && !p.deleted)\n                    .map(|p| (p.x, p.y));\n                if let Some((sx, sy)) = speaker_xy {\n                    let (gx, gy) = ally_goto_speaker_xy(sx, sy);\n                    let _ = try_ai_follow_path_to(state, outbound, res.ai_conn_id, gx, gy);\n                }\n            }\n            if let Some(eid) = emote_id {";
                let old = "if complete.goto_speaker {\n                    ai.force_stop_on_next_tile = false;\n                }\n            }\n            if let Some(eid) = emote_id {";
                t.replace_range(i..i + old.len(), insert);
                ch = true;
            }
        }
    }

    // tick_vitals call
    if !t.contains("tick_ai_follow_walk(state, outbound)") {
        ch |= replace_once(
            &mut t,
            "    tick_llm_speech_wire(state, outbound);\n    // BLOCKED-BY-AI:",
            "    tick_llm_speech_wire(state, outbound);\n    // AI-FOLLOW-WALK: continuous isMovingToPlayer walk toward ai_follow_p_id\n    // Haxe: AiBase.doTimeStuffHelper isMovingToPlayer after LLM sticky\n    tick_ai_follow_walk(state, outbound);\n    // BLOCKED-BY-AI:",
        );
    }

    if ch {
        let out = restore_nl(&t, crlf);
        write_if_changed(path, &raw, &out);
    }
    let final_t = std::fs::read_to_string(path).unwrap_or_default();
    final_t.contains("tick_ai_follow_walk") && final_t.contains("decide_follow_walk")
}

/// AI-FOLLOW-ACQUIRE: re-export acquire symbols + live tests (idempotent).
fn patch_acquire_lib(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;

    if !t.contains("plan_auto_follow_acquire") {
        ch |= replace_once(
            &mut t,
            "    AUTO_STOP_FOLLOW_CLEAR_AGE, FOLLOW_PATH_STEP_CAP, ORDERED_FOLLOW_MAX_SECS,\n};\n",
            "    AUTO_STOP_FOLLOW_CLEAR_AGE, FOLLOW_PATH_STEP_CAP, ORDERED_FOLLOW_MAX_SECS,\n    // AI-FOLLOW-ACQUIRE / auto_follow\n    get_closest_player_for_auto_follow, living_follow_leader, plan_auto_follow_acquire,\n    resolve_auto_follow_acquire, AutoFollowAcquire, AutoFollowAcquireSource,\n    AutoFollowCandidate, AUTO_FOLLOW_PLAYER_DEFAULT, AUTO_FOLLOW_SEARCH_TILES,\n};\n",
        );
    }

    if !t.contains("tick_ai_follow_acquire_child_mother") {
        let anchor = "    fn player_snapshot_includes_follow_sticky() {\n        let mut state = SimState::with_default_empty(test_content());\n        spawn_player(&mut state, 1, \"npc-snap@local\");\n        {\n            let p = state.players.get_mut(&1).unwrap();\n            p.ai_follow_p_id = 42;\n            p.ai_auto_stop_follow = false;\n        }\n        let snap = state.players.get(&1).unwrap().snapshot();\n        assert_eq!(snap.ai_follow_p_id, 42);\n        assert!(!snap.ai_auto_stop_follow);\n    }\n";
        let tests = r#"    fn player_snapshot_includes_follow_sticky() {
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "npc-snap@local");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.ai_follow_p_id = 42;
            p.ai_auto_stop_follow = false;
        }
        let snap = state.players.get(&1).unwrap().snapshot();
        assert_eq!(snap.ai_follow_p_id, 42);
        assert!(!snap.ai_auto_stop_follow);
    }

    /// AI-FOLLOW-ACQUIRE: child with leadership mother acquires sticky on tick.
    // Haxe: isMovingToPlayer isChildAndHasMother → getFollowPlayer
    #[test]
    fn tick_ai_follow_acquire_child_mother() {
        let mut state = SimState::with_default_empty(test_content());
        state.timed_movement = true;
        spawn_player(&mut state, 1, "npc-baby@local");
        spawn_player(&mut state, 2, "mother@t");
        set_player_position(&mut state, 1, 0, 0);
        set_player_position(&mut state, 2, 12, 0);
        for x in 0..=15 {
            state.world.write().unwrap().set_biome(x, 0, ol_world::GREEN);
        }
        let mother_pid = state.players.get(&2).unwrap().p_id;
        let baby_pid = state.players.get(&1).unwrap().p_id;
        state.social.following.insert(baby_pid, mother_pid);
        {
            let ai = state.players.get_mut(&1).unwrap();
            ai.ai_follow_p_id = 0;
            ai.age = 1.5; // < MIN_AGE_TO_EAT
        }
        let hub = OutboundHub::new();
        tick_ai_follow_walk(&mut state, &hub);
        let ai = state.players.get(&1).unwrap();
        assert_eq!(
            ai.ai_follow_p_id, mother_pid,
            "child sticky should acquire leadership mother"
        );
        assert!(
            ai.moving || ai.move_path.is_some(),
            "expected path after child-mother acquire when far"
        );
    }

    /// AI-FOLLOW-ACQUIRE: adult with AutoFollowPlayer off does not latch closest.
    // Haxe: ServerSettings.AutoFollowPlayer == false
    #[test]
    fn tick_ai_follow_acquire_default_off_no_closest() {
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "npc-adult@local");
        spawn_player(&mut state, 2, "human@t");
        set_player_position(&mut state, 1, 0, 0);
        set_player_position(&mut state, 2, 5, 0);
        {
            let ai = state.players.get_mut(&1).unwrap();
            ai.ai_follow_p_id = 0;
            ai.age = 20.0;
        }
        let hub = OutboundHub::new();
        tick_ai_follow_walk(&mut state, &hub);
        let ai = state.players.get(&1).unwrap();
        assert_eq!(ai.ai_follow_p_id, 0, "AutoFollowPlayer default false");
    }

    /// AI-FOLLOW-ACQUIRE: pure path AutoFollowPlayer closest (enabled flag).
    #[test]
    fn tick_ai_follow_acquire_closest_when_enabled() {
        let mut state = SimState::with_default_empty(test_content());
        state.timed_movement = true;
        spawn_player(&mut state, 1, "npc-auto@local");
        spawn_player(&mut state, 2, "human@t");
        set_player_position(&mut state, 1, 0, 0);
        set_player_position(&mut state, 2, 8, 0);
        for x in 0..=12 {
            state.world.write().unwrap().set_biome(x, 0, ol_world::GREEN);
        }
        let human_pid = state.players.get(&2).unwrap().p_id;
        {
            let ai = state.players.get_mut(&1).unwrap();
            ai.ai_follow_p_id = 0;
            ai.age = 20.0;
        }
        tick_ai_follow_acquire(&mut state, true);
        let ai = state.players.get(&1).unwrap();
        assert_eq!(ai.ai_follow_p_id, human_pid);
    }

"#;
        if t.contains(anchor) {
            ch |= replace_once(&mut t, anchor, tests);
        } else if let Some(i) = t.find("fn player_snapshot_includes_follow_sticky()") {
            // Fallback: insert after closing brace of the function
            if let Some(rel) = t[i..].find("\n    }\n\n    #[test]\n    fn force_stop_on_next_tile_cancels") {
                let at = i + rel + "\n    }\n".len();
                let insert = r#"

    /// AI-FOLLOW-ACQUIRE: child with leadership mother acquires sticky on tick.
    // Haxe: isMovingToPlayer isChildAndHasMother → getFollowPlayer
    #[test]
    fn tick_ai_follow_acquire_child_mother() {
        let mut state = SimState::with_default_empty(test_content());
        state.timed_movement = true;
        spawn_player(&mut state, 1, "npc-baby@local");
        spawn_player(&mut state, 2, "mother@t");
        set_player_position(&mut state, 1, 0, 0);
        set_player_position(&mut state, 2, 12, 0);
        for x in 0..=15 {
            state.world.write().unwrap().set_biome(x, 0, ol_world::GREEN);
        }
        let mother_pid = state.players.get(&2).unwrap().p_id;
        let baby_pid = state.players.get(&1).unwrap().p_id;
        state.social.following.insert(baby_pid, mother_pid);
        {
            let ai = state.players.get_mut(&1).unwrap();
            ai.ai_follow_p_id = 0;
            ai.age = 1.5;
        }
        let hub = OutboundHub::new();
        tick_ai_follow_walk(&mut state, &hub);
        let ai = state.players.get(&1).unwrap();
        assert_eq!(ai.ai_follow_p_id, mother_pid);
        assert!(ai.moving || ai.move_path.is_some());
    }

    /// AI-FOLLOW-ACQUIRE: adult with AutoFollowPlayer off does not latch closest.
    #[test]
    fn tick_ai_follow_acquire_default_off_no_closest() {
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "npc-adult@local");
        spawn_player(&mut state, 2, "human@t");
        set_player_position(&mut state, 1, 0, 0);
        set_player_position(&mut state, 2, 5, 0);
        {
            let ai = state.players.get_mut(&1).unwrap();
            ai.ai_follow_p_id = 0;
            ai.age = 20.0;
        }
        let hub = OutboundHub::new();
        tick_ai_follow_walk(&mut state, &hub);
        let ai = state.players.get(&1).unwrap();
        assert_eq!(ai.ai_follow_p_id, 0);
    }

    /// AI-FOLLOW-ACQUIRE: AutoFollowPlayer closest when enabled.
    #[test]
    fn tick_ai_follow_acquire_closest_when_enabled() {
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "npc-auto@local");
        spawn_player(&mut state, 2, "human@t");
        set_player_position(&mut state, 1, 0, 0);
        set_player_position(&mut state, 2, 8, 0);
        let human_pid = state.players.get(&2).unwrap().p_id;
        {
            let ai = state.players.get_mut(&1).unwrap();
            ai.ai_follow_p_id = 0;
            ai.age = 20.0;
        }
        tick_ai_follow_acquire(&mut state, true);
        let ai = state.players.get(&1).unwrap();
        assert_eq!(ai.ai_follow_p_id, human_pid);
    }
"#;
                t.insert_str(at, insert);
                ch = true;
            }
        }
    }

    if ch {
        let out = restore_nl(&t, crlf);
        write_if_changed(path, &raw, &out);
    }
    let final_t = std::fs::read_to_string(path).unwrap_or_default();
    final_t.contains("plan_auto_follow_acquire") || final_t.contains("tick_ai_follow_acquire")
}

fn patch_player(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("ai_follow_p_id: self.ai_follow_p_id") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;

    if !t.contains("ai_follow_p_id: self.ai_follow_p_id") {
        ch |= replace_once(
            &mut t,
            "            clothing: self.clothing_parent_ids(),\n            clothing_uses: self.clothing_uses_remaining(),\n        }\n    }\n}",
            "            clothing: self.clothing_parent_ids(),\n            clothing_uses: self.clothing_uses_remaining(),\n            // AI-FOLLOW-WALK: sticky walk-with for NPC continuous follow\n            ai_follow_p_id: self.ai_follow_p_id,\n            ai_auto_stop_follow: self.ai_auto_stop_follow,\n        }\n    }\n}",
        );
    }
    if !t.contains("pub ai_follow_p_id: i32") || !t.contains("PlayerSnapshot") {
        // PlayerSnapshot fields — best-effort
        ch |= replace_once(
            &mut t,
            "    /// Haxe clothing `numberOfUses` per slot (quiver multi-use capacity).\n    #[serde(default)]\n    pub clothing_uses: [i32; 6],\n}",
            "    /// Haxe clothing `numberOfUses` per slot (quiver multi-use capacity).\n    #[serde(default)]\n    pub clothing_uses: [i32; 6],\n    /// Sticky AI walk-with target p_id (Haxe playerToFollow); 0 = none (**AI-FOLLOW-WALK**).\n    // Haxe: AiBase.playerToFollow\n    #[serde(default)]\n    pub ai_follow_p_id: i32,\n    /// Haxe autoStopFollow — loose follow when true.\n    // Haxe: AiBase.autoStopFollow\n    #[serde(default = \"default_true_snapshot\")]\n    pub ai_auto_stop_follow: bool,\n}",
        );
    }

    if ch {
        let out = restore_nl(&t, crlf);
        write_if_changed(path, &raw, &out);
    }
    std::fs::read_to_string(path)
        .map(|s| s.contains("ai_follow_p_id: self.ai_follow_p_id"))
        .unwrap_or(false)
}

fn patch_npc(path: &Path) -> bool {
    if !path.exists() {
        return true;
    }
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("AI-FOLLOW-WALK") && raw.contains("follow_walk") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;

    // Insert continuous follow before profession scan when sticky set
    if !t.contains("AI-FOLLOW-WALK") {
        // Look for profession scan entry points
        if let Some(_i) = t.find("profession_scan") {
            // Prefer: after starving check / before ladder
            let markers = [
                "// --- 2. Profession ladder",
                "// --- 2. Age-rotated",
                "let rung = resolve_priority_rung",
            ];
            for m in markers {
                if t.contains(m) {
                    let insert = format!(
                        "            // --- 2a. Continuous follow walk (AI-FOLLOW-WALK) ---\n            // Haxe: AiBase.isMovingToPlayer after sticky playerToFollow / LLM follow\n            if !acted && !starving {{\n                let follow_p = p.ai_follow_p_id;\n                if follow_p > 0 {{\n                    let target = views.values().find(|o| o.p_id == follow_p && !o.deleted);\n                    if let Some(t) = target {{\n                        let max_tiles = if p.ai_auto_stop_follow {{ 10 }} else {{ 5 }};\n                        let max_q = max_tiles * max_tiles;\n                        let dx = t.x - p.x;\n                        let dy = t.y - p.y;\n                        let qd = dx * dx + dy * dy;\n                        if qd >= max_q {{\n                            let goal_x = t.x + 1;\n                            let goal_y = t.y;\n                            let dist = (goal_x - p.x).abs().max((goal_y - p.y).abs());\n                            if dist > 1 {{\n                                // follow_walk: path toward sticky target (npc path helper)\n                                if try_path_to(state, outbound, conn_id, goal_x, goal_y) {{\n                                    acted = true;\n                                    eprintln!(\n                                            \"follow_walk target={{}} @{{}},{{}}\",\n                                            follow_p, t.x, t.y\n                                        );\n                                }}\n                            }}\n                        }}\n                    }}\n                }}\n            }}\n            {m}"
                    );
                    if replace_once(&mut t, m, &insert) {
                        ch = true;
                        break;
                    }
                }
            }
        }
    }

    if ch {
        let out = restore_nl(&t, crlf);
        write_if_changed(path, &raw, &out);
        println!("cargo:warning=AI-FOLLOW-WALK: npc_ai follow_walk wired");
    }
    std::fs::read_to_string(path)
        .map(|s| s.contains("AI-FOLLOW-WALK"))
        .unwrap_or(false)
}

fn patch_docs(workspace: &Path) -> bool {
    let port = workspace.join("docs/port");

    let todo = port.join("TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo) {
        if !raw.contains("AI-FOLLOW-WALK") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            let _ = replace_once(
                &mut t,
                "- [x] **AI-LLM-APPLY llm_actions DONE** — live `ApplyAiResponsePlan` emote PE + sticky `ai_follow_p_id`/`ai_ordered_to_drop` + `craft_ai.do_make_craft_command` silent + resolve makeItem (bare id/name alias); immediate DROP feet; residual: ally Goto pathfind walk, follow tick path, scripted sayHelper (**AI-SAY-HELPER**), full RelationshipView, toSoul other chat\n",
                "- [x] **AI-LLM-APPLY llm_actions DONE** — live `ApplyAiResponsePlan` emote PE + sticky `ai_follow_p_id`/`ai_ordered_to_drop` + `craft_ai.do_make_craft_command` silent + resolve makeItem (bare id/name alias); immediate DROP feet; residual: scripted sayHelper (**AI-SAY-HELPER**), full RelationshipView, toSoul other chat\n- [x] **AI-FOLLOW-WALK continuous_follow DONE** — pure `ai_follow_walk` isMovingToPlayer + sticky clear + ally Goto(speaker+1); live `tick_ai_follow_walk` pathfind; LLM startFollowing/ally Goto path; NPC follow_walk; residual: AutoFollowPlayer closest-human acquire, child-mother getFollowPlayer, debug say name\n",
            );
            let _ = replace_once(
                &mut t,
                "residual: scripted sayHelper + ally Goto pathfind",
                "residual: scripted sayHelper (**AI-SAY-HELPER**); follow → **AI-FOLLOW-WALK DONE**",
            );
            let out = restore_nl(&t, crlf);
            let _ = write_if_changed(&todo, &raw, &out);
        }
    }

    let matrix = port.join("FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&matrix) {
        if !raw.contains("AI-FOLLOW-WALK") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            let _ = replace_once(
                &mut t,
                "| AI-FOLLOW / **LEADERSHIP-UX** | follow/hire delayed confirm | **PARTIAL → following_wire core DONE** |\n",
                "| AI-FOLLOW / **LEADERSHIP-UX** | follow/hire delayed confirm | **PARTIAL → following_wire core DONE** |\n| **AI-FOLLOW-WALK** / continuous_follow | `AiBase.isMovingToPlayer` + ally Goto pathfind | **DONE** | pure `ai_follow_walk.rs` + `tick_ai_follow_walk` + LLM Goto + npc `follow_walk`; residual AutoFollowPlayer closest / mother acquire |\n",
            );
            let _ = replace_once(
                &mut t,
                "residual: ally Goto pathfind, follow walk tick, scripted sayHelper",
                "residual: scripted sayHelper (**AI-SAY-HELPER**); follow walk → **AI-FOLLOW-WALK DONE**",
            );
            let out = restore_nl(&t, crlf);
            let _ = write_if_changed(&matrix, &raw, &out);
        }
    }

    let call = port.join("CALL_INDEX.md");
    if let Ok(raw) = std::fs::read_to_string(&call) {
        if !raw.contains("decide_follow_walk") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            let _ = replace_once(
                &mut t,
                "| `is_moving_to_player_needed` / `child_with_mother_follow_tiles` / `ordered_follow_max_tiles` | same | Haxe `isMovingToPlayer` distance gates |\n",
                "| `is_moving_to_player_needed` / `child_with_mother_follow_tiles` / `ordered_follow_max_tiles` | same | Haxe `isMovingToPlayer` distance gates |\n| `decide_follow_walk` / `plan_follow_sticky_clear` / `ally_goto_speaker_xy` / `tick_ai_follow_walk` / `try_ai_follow_path_to` | `ol-sim/ai_follow_walk.rs` + lib | **AI-FOLLOW-WALK** continuous follow + ally Goto pathfind |\n",
            );
            let out = restore_nl(&t, crlf);
            let _ = write_if_changed(&call, &raw, &out);
        }
    }

    let queue = port.join("QUEUE.md");
    if let Ok(raw) = std::fs::read_to_string(&queue) {
        if raw.contains("AI-FOLLOW-WALK") && !raw.contains("~~`AI-FOLLOW-WALK`~~") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            let _ = replace_once(
                &mut t,
                "| `AI-FOLLOW-WALK` | continuous_follow | continuous follow walk (wf-109) |\n",
                "| ~~`AI-FOLLOW-WALK`~~ | continuous_follow | **DONE** continuous follow + ally Goto pathfind |\n",
            );
            let _ = replace_once(
                &mut t,
                "| `AI-FOLLOW-WALK` | continuous_follow | continuous follow walk + ally Goto residual |\n",
                "| ~~`AI-FOLLOW-WALK`~~ | continuous_follow | **DONE** continuous follow + ally Goto pathfind |\n",
            );
            let _ = replace_once(
                &mut t,
                "**AI-POTTER-L2946** DONE ·",
                "**AI-FOLLOW-WALK** continuous_follow DONE · **AI-POTTER-L2946** DONE ·",
            );
            let out = restore_nl(&t, crlf);
            let _ = write_if_changed(&queue, &raw, &out);
        }
    }

    true
}

/// AI-FOLLOW-ACQUIRE docs (idempotent).
fn patch_acquire_docs(workspace: &Path) -> bool {
    let port = workspace.join("docs/port");

    let todo = port.join("TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo) {
        if !raw.contains("AI-FOLLOW-ACQUIRE auto_follow DONE") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            let _ = replace_once(
                &mut t,
                "residual: AutoFollowPlayer closest-human acquire, child-mother getFollowPlayer, debug say name, specialized baby/child/wounded distance bands",
                "residual: debug say name, specialized baby/child/wounded distance bands (acquire → **AI-FOLLOW-ACQUIRE**)",
            );
            let _ = replace_once(
                &mut t,
                "- [x] **AI-FOLLOW-WALK continuous_follow DONE**",
                "- [x] **AI-FOLLOW-ACQUIRE auto_follow DONE** — pure `plan_auto_follow_acquire` / `get_closest_player_for_auto_follow` + live `tick_ai_follow_acquire` child-mother `getFollowPlayer` + `AutoFollowPlayer` closest (default off); residual: debug say name, specialized baby/child/wounded distance bands, server.toml knob\n- [x] **AI-FOLLOW-WALK continuous_follow DONE**",
            );
            let out = restore_nl(&t, crlf);
            let _ = write_if_changed(&todo, &raw, &out);
        }
    }

    let matrix = port.join("FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&matrix) {
        if !raw.contains("**AI-FOLLOW-ACQUIRE**") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            let _ = replace_once(
                &mut t,
                "residual: AutoFollowPlayer closest-human, child-mother getFollowPlayer, debug say name, specialized baby/child/wounded bands",
                "residual: debug say name, specialized baby/child/wounded bands (acquire → **AI-FOLLOW-ACQUIRE**)",
            );
            let _ = replace_once(
                &mut t,
                "residual AutoFollowPlayer closest / mother acquire / specialized distance bands",
                "residual specialized distance bands / debug say (acquire → **AI-FOLLOW-ACQUIRE**)",
            );
            let _ = replace_once(
                &mut t,
                "| **AI-FOLLOW-WALK** / continuous_follow |",
                "| **AI-FOLLOW-ACQUIRE** / auto_follow | `AiBase.isMovingToPlayer` empty sticky: child-mother `getFollowPlayer` + `AutoFollowPlayer` closest | — | `ol-sim/ai_follow_walk.rs` + `tick_ai_follow_acquire` in live.inc | **DONE** | pure closest/child plan + live acquire before walk; default AutoFollowPlayer=false; residual: debug say name, baby/child/wounded bands, toml knob |\n| **AI-FOLLOW-WALK** / continuous_follow |",
            );
            let out = restore_nl(&t, crlf);
            let _ = write_if_changed(&matrix, &raw, &out);
        }
    }

    let call = port.join("CALL_INDEX.md");
    if let Ok(raw) = std::fs::read_to_string(&call) {
        if !raw.contains("AI-FOLLOW-ACQUIRE (`AiBase.isMovingToPlayer` auto_follow") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            let _ = replace_once(
                &mut t,
                "| Residual | — | AutoFollowPlayer closest-human; child-mother getFollowPlayer; debug say name; baby/child/wounded bands |",
                "| Residual | — | debug say name; baby/child/wounded bands (acquire → **AI-FOLLOW-ACQUIRE**) |",
            );
            let _ = replace_once(
                &mut t,
                "### AI-SAY-HELPER (`AiBase.sayHelper` scripted_cmds)",
                "### AI-FOLLOW-ACQUIRE (`AiBase.isMovingToPlayer` auto_follow empty sticky)\n\n| Haxe | Rust | Notes |\n|------|------|-------|\n| `isChildAndHasMother` + `getFollowPlayer` | `plan_auto_follow_acquire` ChildMother + `direct_follow_leader` | age < MinAgeToEat + living leadership follow |\n| `AutoFollowPlayer` + `getClosestPlayer(20, followHuman)` | `get_closest_player_for_auto_follow` / `resolve_auto_follow_acquire` | default off (`AUTO_FOLLOW_PLAYER_DEFAULT`) |\n| empty sticky assign | `tick_ai_follow_acquire` before walk | loose sticky (autoStop stays true) |\n| Residual | — | debug say target name; baby/child/wounded bands; server.toml AutoFollowPlayer |\n\n### AI-SAY-HELPER (`AiBase.sayHelper` scripted_cmds)",
            );
            let out = restore_nl(&t, crlf);
            let _ = write_if_changed(&call, &raw, &out);
        }
    }

    let queue = port.join("QUEUE.md");
    if let Ok(raw) = std::fs::read_to_string(&queue) {
        if raw.contains("AI-FOLLOW-ACQUIRE") && !raw.contains("AI-FOLLOW-ACQUIRE** DONE") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            let _ = replace_once(
                &mut t,
                "| `AI-FOLLOW-ACQUIRE` | auto_follow | AutoFollowPlayer + child-mother acquire residual |\n",
                "",
            );
            let _ = replace_once(
                &mut t,
                "**AI-FOLLOW-WALK** DONE ·",
                "**AI-FOLLOW-ACQUIRE** DONE · **AI-FOLLOW-WALK** DONE ·",
            );
            let out = restore_nl(&t, crlf);
            let _ = write_if_changed(&queue, &raw, &out);
        }
    }

    let changelog = port.join("changelog/2026-07-29-AI-FOLLOW-ACQUIRE.md");
    if !changelog.exists() {
        let body = r#"# AI-FOLLOW-ACQUIRE / auto_follow

**Date:** 2026-07-29  
**Mode:** implement  
**Status:** DONE (core empty-sticky acquire)

## Scope

Close residual from **AI-FOLLOW-WALK**: when `playerToFollow` / `ai_follow_p_id` is empty,
Haxe `isMovingToPlayer` acquires a target before walking.

## Haxe

- `AiBase.isMovingToPlayer` empty branch (~8287–8296)
- `isChildAndHasMother` → `getFollowPlayer()` (leadership mother)
- else `ServerSettings.AutoFollowPlayer` → `getClosestPlayer(20, followHuman)`
- `GlobalPlayerInstance.getClosestPlayer` (humans first, AIs second)

## Rust

- Pure: `plan_auto_follow_acquire`, `resolve_auto_follow_acquire`,
  `get_closest_player_for_auto_follow`, `AutoFollowCandidate`
- Live: `tick_ai_follow_acquire` before continuous walk in `tick_ai_follow_walk`
- Leadership mother via `direct_follow_leader` + `social.following`
- `AUTO_FOLLOW_PLAYER_DEFAULT = false` (matches Haxe ServerSettings)

## Residuals

- Debug say target name while walking
- Specialized baby/child/wounded distance bands
- `server.toml` / LiveSettings `AutoFollowPlayer` knob (const default today)

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- ai_follow -- --test-threads=1
cargo test -p ol-sim --lib -- tick_ai_follow_acquire -- --test-threads=1
```
"#;
        let _ = std::fs::write(&changelog, body);
    }

    true
}
