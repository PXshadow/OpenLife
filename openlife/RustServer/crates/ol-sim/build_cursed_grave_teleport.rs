//! Build-time wire for **CURSED-GRAVE-TELEPORT** / `tcg_tv_teleport`.
//!
//! Haxe `GlobalPlayerInstance` `!TCG`/`!CURSEDGRAVE` + `!TV`/`!VILLAGE` teleport
//! consumers of `WorldMapTimeState.cursed_graves` / `.ovens`.
//! Idempotent pure-Rust file patches (Python optional).

use std::path::Path;
use std::process::Command;

fn nl(s: &str) -> (String, bool) {
    let crlf = s.contains("\r\n");
    (s.replace("\r\n", "\n").replace('\r', "\n"), crlf)
}

fn out(s: String, crlf: bool) -> String {
    if crlf {
        s.replace('\n', "\r\n")
    } else {
        s
    }
}

fn once(hay: &mut String, old: &str, new: &str) -> bool {
    if hay.contains(new) {
        return false;
    }
    if let Some(i) = hay.find(old) {
        hay.replace_range(i..i + old.len(), new);
        true
    } else {
        false
    }
}

/// True when pure module + live wire markers present.
pub fn cursed_grave_teleport_wired(lib_text: &str, player_text: &str, module_exists: bool) -> bool {
    module_exists
        && lib_text.contains("mod teleport_cmd;")
        && lib_text.contains("try_apply_teleport_bang")
        && lib_text.contains("apply_do_teleport")
        && lib_text.contains("CURSED-GRAVE-TELEPORT")
        && player_text.contains("blocked_teleport_locations")
}

fn try_run_python(script: &Path) -> bool {
    if !script.exists() {
        return false;
    }
    let py = Command::new("python")
        .arg(script)
        .status()
        .or_else(|_| Command::new("python3").arg(script).status())
        .or_else(|_| Command::new("py").arg("-3").arg(script).status());
    matches!(py, Ok(s) if s.success())
}

/// Apply CURSED-GRAVE-TELEPORT. Returns true when ready.
pub fn patch_cursed_grave_teleport(src: &Path, workspace: &Path) -> bool {
    let lib_path = src.join("lib.rs");
    let player_path = src.join("player.rs");
    let mod_path = src.join("teleport_cmd.rs");
    let module_exists = mod_path.exists();

    // Prefer Python one-shot when present.
    let _ = try_run_python(&src.join("_apply_cursed_grave_teleport.py"));
    let _ = try_run_python(&workspace.join("docs/port/_apply_cursed_grave_teleport.py"));

    let lib_t = std::fs::read_to_string(&lib_path).unwrap_or_default();
    let player_t = std::fs::read_to_string(&player_path).unwrap_or_default();
    if cursed_grave_teleport_wired(&lib_t, &player_t, module_exists) {
        return true;
    }

    let _ = patch_player_rs(&player_path);
    let _ = patch_lib_rs(&lib_path);

    let lib_f = std::fs::read_to_string(&lib_path).unwrap_or_default();
    let player_f = std::fs::read_to_string(&player_path).unwrap_or_default();
    cursed_grave_teleport_wired(&lib_f, &player_f, mod_path.exists())
}

fn patch_player_rs(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let (mut t, crlf) = nl(&raw);
    let mut changed = false;

    changed |= once(
        &mut t,
        "    /// Haxe `AiBase.didNotReachFood` — gates considerAnimals + escape.\n    // Haxe: AiBase.didNotReachFood\n    pub ai_did_not_reach_food: f32,\n}\n",
        "    /// Haxe `AiBase.didNotReachFood` — gates considerAnimals + escape.\n    // Haxe: AiBase.didNotReachFood\n    pub ai_did_not_reach_food: f32,\n    /// Haxe `blockedTeleportLocations` — linear map indexes already tried by\n    /// `!TCG`/`!TV`/`teleport` closest-pick cycle (session-only).\n    // Haxe: GlobalPlayerInstance.blockedTeleportLocations\n    // CURSED-GRAVE-TELEPORT\n    pub blocked_teleport_locations: Vec<i32>,\n}\n",
    );

    changed |= once(
        &mut t,
        "            ai_last_goto_obj_distance: -1.0,\n            ai_did_not_reach_food: 0.0,\n        }\n    }\n",
        "            ai_last_goto_obj_distance: -1.0,\n            ai_did_not_reach_food: 0.0,\n            // CURSED-GRAVE-TELEPORT: blockedTeleportLocations session list\n            blocked_teleport_locations: Vec::new(),\n        }\n    }\n",
    );

    if changed {
        let _ = std::fs::write(path, out(t, crlf));
    }
    std::fs::read_to_string(path)
        .map(|s| s.contains("blocked_teleport_locations"))
        .unwrap_or(false)
}

fn patch_lib_rs(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let (mut t, crlf) = nl(&raw);
    let mut changed = false;

    // mod
    changed |= once(
        &mut t,
        "// Haxe: TimeHelper.DoWorldMapTimeStuff family (TIME-WORLD)\nmod world_time;\n",
        "// Haxe: TimeHelper.DoWorldMapTimeStuff family (TIME-WORLD)\nmod world_time;\n// Haxe: GlobalPlayerInstance !TCG/!TV teleport + blockedTeleportLocations (CURSED-GRAVE-TELEPORT)\nmod teleport_cmd;\n",
    );

    // pub use after world_time block
    if !t.contains("pub use teleport_cmd::") {
        let anchor = "    WorldMapTimeState, CURSED_GRAVES_CLEAR_TICK_MOD, CURSED_GRAVE_SHARP_STONE_EXTRA_SECS,\n    CURSED_GRAVE_TIME_HOURS, NESTED_TIMER_MAX_DEPTH, WORLD_TIME_PARTS,\n};\n";
        let insert = "    WorldMapTimeState, CURSED_GRAVES_CLEAR_TICK_MOD, CURSED_GRAVE_SHARP_STONE_EXTRA_SECS,\n    CURSED_GRAVE_TIME_HOURS, NESTED_TIMER_MAX_DEPTH, WORLD_TIME_PARTS,\n};\n// Haxe: GlobalPlayerInstance !TCG/!TV teleport (CURSED-GRAVE-TELEPORT / tcg_tv_teleport)\npub use teleport_cmd::{\n    clear_blocked_teleport, not_found_text, parse_teleport_bang, pick_closest_from_index_map,\n    pick_closest_teleport, push_blocked_teleport, teleport_location_index, teleport_quad_distance,\n    TeleportBang, TeleportPick, TCG_NOT_FOUND, TELEPORT_ALL_TRIED, TELEPORT_NOT_ALLOWED,\n    TV_NOT_FOUND,\n};\n";
        changed |= once(&mut t, anchor, insert);
    }

    // live fns before apply_player_jump
    if !t.contains("fn try_apply_teleport_bang") {
        let marker = "/// Haxe `GlobalPlayerInstance.jump` — baby out of arms or BW wiggle + PU.\n///\n/// Returns `true` when dropped from arms (held path).\n// Haxe: GlobalPlayerInstance.jump L5098-5120\n// JUMP-BW-FULL\npub fn apply_player_jump(\n";
        let fns = r#"/// Haxe `GlobalPlayerInstance.doTeleport` — absolute world snap + JumpToNonBlocked + VOG/MC/force PU.
///
/// Returns `false` when the landing tile is still blocked after jump (Haxe early return).
// Haxe: GlobalPlayerInstance.doTeleport L5828-5843
// CURSED-GRAVE-TELEPORT
pub fn apply_do_teleport(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    tx: i32,
    ty: i32,
) -> bool {
    let Some(p) = state.players.get_mut(&conn_id) else {
        return false;
    };
    if p.deleted {
        return false;
    }
    // Haxe: player.x/y = transformX/Y(player, tx/ty) — Rust stores absolute world.
    p.x = tx;
    p.y = ty;
    p.move_path = None;
    p.moving = false;
    state.world.write().unwrap().touch_radius(tx, ty, 1);

    // Haxe: if (player.isBlocked(player.tx, player.ty)) MoveHelper.JumpToNonBlocked(player);
    let seq = state
        .players
        .get(&conn_id)
        .map(|pl| pl.done_moving_seq.max(1))
        .unwrap_or(1);
    let _ = try_jump_to_non_blocked(state, outbound, conn_id, seq);

    // Haxe: if (player.isBlocked(player.tx, player.ty)) return;
    let still_blocked = {
        let Some(p) = state.players.get(&conn_id) else {
            return false;
        };
        let world = state.world.read().unwrap();
        let content = &state.content;
        let (sx, sy) = world.wrap_tile(p.x, p.y);
        biome_blocks_move_at(&world, sx, sy) || !is_walkable(&world, content, sx, sy)
    };
    if still_blocked {
        return false;
    }

    // Haxe: VOG_UPDATE + sendMapChunk + forced PU + FRAME
    // (JumpToNonBlocked may already have sent when it stepped; Haxe double-sends.)
    cancel_movement(state, outbound, conn_id, 0, true);
    true
}

/// Haxe `!TCG`/`!CURSEDGRAVE` / `!TV`/`!VILLAGE` — admin teleport via global indexes.
///
/// Gate: `Player.godmode` stands in for Haxe `account.canUseServerCommands`.
/// coinCost is 0 for both bangs (no PayTeleportCost).
// Haxe: GlobalPlayerInstance.doServerCommand L5515-5596 + teleport L5792
// CURSED-GRAVE-TELEPORT
fn try_apply_teleport_bang(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    upper: &str,
) -> bool {
    let Some(cmd) = parse_teleport_bang(upper) else {
        return false;
    };
    let Some(p) = state.players.get(&conn_id).cloned() else {
        return true;
    };
    if p.deleted {
        return true;
    }
    // Haxe: checkIfNotAllowed → canUseServerCommands
    if !p.godmode {
        let line = format!("{} {}", p.p_id, TELEPORT_NOT_ALLOWED.to_ascii_uppercase());
        send_ps_reply(outbound, conn_id, &line);
        return true;
    }

    let index_map = match cmd {
        TeleportBang::CursedGrave => state.world_map_time.cursed_graves.clone(),
        TeleportBang::Village => state.world_map_time.ovens.clone(),
    };
    let blocked = p.blocked_teleport_locations.clone();
    let pick = pick_closest_from_index_map(p.x, p.y, &index_map, &blocked);

    match pick {
        TeleportPick::Empty => {
            let msg = not_found_text(cmd);
            let line = format!("{} {}", p.p_id, msg.to_ascii_uppercase());
            send_ps_reply(outbound, conn_id, &line);
        }
        TeleportPick::AllBlocked => {
            if let Some(pl) = state.players.get_mut(&conn_id) {
                clear_blocked_teleport(&mut pl.blocked_teleport_locations);
            }
            let line = format!("{} {}", p.p_id, TELEPORT_ALL_TRIED.to_ascii_uppercase());
            send_ps_reply(outbound, conn_id, &line);
        }
        TeleportPick::Found { index, tx, ty } => {
            if let Some(pl) = state.players.get_mut(&conn_id) {
                push_blocked_teleport(&mut pl.blocked_teleport_locations, index);
            }
            let _ = apply_do_teleport(state, outbound, conn_id, tx, ty);
            info!(
                conn_id,
                ?cmd,
                tx,
                ty,
                index,
                "sim: teleport bang"
            );
        }
    }
    true
}

"#;
        changed |= once(&mut t, marker, &format!("{fns}{marker}"));
    }

    // SAY handler
    changed |= once(
        &mut t,
        "        // Haxe `!YUM` — toggle PlayerAccount.displayYum\n        if upper == \"!YUM\" || upper == \"YUMDISPLAY\" || upper == \"!YUMDISPLAY\" {\n",
        "        // CURSED-GRAVE-TELEPORT: !TCG/!CURSEDGRAVE + !TV/!VILLAGE via global indexes\n        // Haxe: GlobalPlayerInstance.doServerCommand L5515 / L5577\n        if try_apply_teleport_bang(state, outbound, conn_id, &upper) {\n            return;\n        }\n        // Haxe `!YUM` — toggle PlayerAccount.displayYum\n        if upper == \"!YUM\" || upper == \"YUMDISPLAY\" || upper == \"!YUMDISPLAY\" {\n",
    );

    // Live tests — insert before say_vogset or heart yum test
    if !t.contains("say_tcg_teleports_to_closest_cursed_grave") {
        let tests = r#"
    // --- CURSED-GRAVE-TELEPORT / tcg_tv_teleport ---

    /// !TCG / !TV require godmode (Haxe canUseServerCommands).
    #[test]
    fn say_tcg_denied_without_godmode() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "tcg@x");
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "!TCG".into(),
            },
        );
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id}/0 NOT ALLOWED!")) {
                saw = true;
            }
        }
        assert!(saw, "expected NOT ALLOWED! without godmode");
    }

    /// !TCG snaps to closest cursed grave, blocks index, cycles on second try.
    #[test]
    fn say_tcg_teleports_to_closest_cursed_grave() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "tcg2@x");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.godmode = true;
            p.x = 0;
            p.y = 0;
            p.birth_x = 0;
            p.birth_y = 0;
        }
        let w = state.world.read().unwrap().width;
        let near = map_linear_index(3, 0, w);
        let far = map_linear_index(40, 0, w);
        state.world_map_time.cursed_graves.insert(near, (3, 0));
        state.world_map_time.cursed_graves.insert(far, (40, 0));
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "!TCG".into(),
            },
        );
        let p = state.players.get(&1).unwrap();
        assert_eq!((p.x, p.y), (3, 0), "should teleport to closest cursed grave");
        assert!(
            p.blocked_teleport_locations.contains(&near),
            "blocked list should record linear index"
        );
        let mut saw_vu = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("VU\n") {
                saw_vu = true;
            }
        }
        assert!(saw_vu, "expected VOG_UPDATE after teleport");

        while rx.try_recv().is_ok() {}
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "!CURSEDGRAVE".into(),
            },
        );
        let p = state.players.get(&1).unwrap();
        assert_eq!((p.x, p.y), (40, 0), "second pick should be far grave");
        let _ = p_id;
    }

    /// Empty cursed_graves → private "No graves found!".
    #[test]
    fn say_tcg_empty_index_says_not_found() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "tcg3@x");
        state.players.get_mut(&1).unwrap().godmode = true;
        state.world_map_time.cursed_graves.clear();
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "!TCG".into(),
            },
        );
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n") && s.contains(&format!("{p_id}/0 NO GRAVES FOUND!")) {
                saw = true;
            }
        }
        assert!(saw, "expected NO GRAVES FOUND!");
    }

    /// !TV uses global ovens index.
    #[test]
    fn say_tv_teleports_to_closest_oven() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "tv@x");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.godmode = true;
            p.x = 10;
            p.y = 10;
            p.birth_x = 0;
            p.birth_y = 0;
        }
        let w = state.world.read().unwrap().width;
        let idx = map_linear_index(12, 10, w);
        state.world_map_time.ovens.insert(idx, (12, 10));
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "!TV".into(),
            },
        );
        let p = state.players.get(&1).unwrap();
        assert_eq!((p.x, p.y), (12, 10));
        assert!(p.blocked_teleport_locations.contains(&idx));
        let _ = (rx, p_id);
    }

    /// Exhaust all locations → clear blocked + "Tried all locations. Start again!".
    #[test]
    fn say_tcg_all_blocked_clears_and_says() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "tcg4@x");
        state.players.get_mut(&1).unwrap().godmode = true;
        let w = state.world.read().unwrap().width;
        let idx = map_linear_index(5, 5, w);
        state.world_map_time.cursed_graves.insert(idx, (5, 5));
        state
            .players
            .get_mut(&1)
            .unwrap()
            .blocked_teleport_locations
            .push(idx);
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "!TCG".into(),
            },
        );
        assert!(
            state
                .players
                .get(&1)
                .unwrap()
                .blocked_teleport_locations
                .is_empty(),
            "blocked list cleared"
        );
        let mut saw = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PS\n")
                && s.contains(&format!("{p_id}/0 TRIED ALL LOCATIONS. START AGAIN!"))
            {
                saw = true;
            }
        }
        assert!(saw, "expected TRIED ALL LOCATIONS message");
    }

"#;
        if t.contains("    fn say_vogset_godmode_sets_tile() {") {
            changed |= once(
                &mut t,
                "    fn say_vogset_godmode_sets_tile() {",
                &format!("{tests}    fn say_vogset_godmode_sets_tile() {{"),
            );
        } else if t.contains("    fn say_heart_yum_clear_boost_godmode_flags() {") {
            changed |= once(
                &mut t,
                "    fn say_heart_yum_clear_boost_godmode_flags() {",
                &format!("{tests}    fn say_heart_yum_clear_boost_godmode_flags() {{"),
            );
        }
    }

    if changed {
        let _ = std::fs::write(path, out(t, crlf));
    }
    std::fs::read_to_string(path)
        .map(|s| s.contains("try_apply_teleport_bang") && s.contains("mod teleport_cmd;"))
        .unwrap_or(false)
}
