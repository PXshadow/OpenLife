//! Build-time wire for **MOVE-MIDPATH** / `path_recon`.
//!
//! Ports Haxe `MoveHelper.calculateNewPos` into live `apply_move_path_start`:
//! mid-path recon before jump gate + `start_sim_time` on path accept.
//! Idempotent pure-Rust string patch (no Python required).

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

/// True when exports + recon wire + start_sim_time + live test are present.
pub fn move_midpath_wired(lib_text: &str) -> bool {
    lib_text.contains("calculate_new_pos")
        && lib_text.contains("reconcile_mid_path_tile")
        && lib_text.contains("LET_THE_CLIENT_CHEAT_LITTLE_BIT_FACTOR")
        && lib_text.contains("MOVE-MIDPATH: before jump gate")
        && lib_text.contains("path.start_sim_time = state.sim_time")
        && lib_text.contains("mid_path_recon_commits_half_step_before_replace")
}

/// Apply MOVE-MIDPATH patches. Returns true when ready.
pub fn patch_move_midpath(lib_path: &Path, workspace: &Path) -> bool {
    // Prefer Python apply when present (docs + lib in one shot).
    let py_script = workspace.join("docs/port/_apply_move_midpath_lib_only.py");
    if py_script.exists() {
        let py = Command::new("python")
            .arg(&py_script)
            .status()
            .or_else(|_| Command::new("python3").arg(&py_script).status())
            .or_else(|_| Command::new("py").arg("-3").arg(&py_script).status());
        if let Ok(s) = py {
            if s.success() {
                let t = std::fs::read_to_string(lib_path).unwrap_or_default();
                if move_midpath_wired(&t) {
                    return true;
                }
            }
        }
    }

    let _ = patch_lib_rs(lib_path);
    let t = std::fs::read_to_string(lib_path).unwrap_or_default();
    move_midpath_wired(&t)
}

fn patch_lib_rs(lib_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let orig = t.clone();

    // ── exports ──────────────────────────────────────────────────────────
    let old_export = r#"pub use move_path::{
    advance_path, apply_jump_cost, biome_blocks_move_at, build_move_path, calculate_length,
    check_if_not_moving_and_close_enough, chebyshev as move_chebyshev, client_path_deltas_to_steps,
    decay_jumped_tiles, effective_use_distance, format_pm_body, in_use_range, in_use_range_ex,
    is_moving, jump_quad_with_floor, jump_rate_limited, movement_age_allowed, springy_door_open_id,
    steps_to_client_path_deltas, still_waiting_for_force, quad_dist as move_quad_dist,
    resolve_move_seq, round2, truncate_walkable, MovePath, MoveReject, DEFAULT_MOVE_SPEED,
    EXHAUSTION_ON_JUMP, MAX_CLIENT_PATH_STEPS, MAX_JUMPS_PER_TEN_SEC,
    MAX_MOVE_QUAD_JUMP_BEFORE_FORCE, MIN_MOVEMENT_AGE_IN_SEC, SPRINGY_DOOR_CLOSED_H,
    SPRINGY_DOOR_CLOSED_V, SPRINGY_DOOR_OPEN_H, SPRINGY_DOOR_OPEN_V, WAIT_FOR_FORCE_SECS,
};"#;

    let new_export = r#"pub use move_path::{
    advance_path, apply_jump_cost, biome_blocks_move_at, build_move_path, calculate_length,
    calculate_new_pos, calculate_segment_length_haxe, check_if_not_moving_and_close_enough,
    chebyshev as move_chebyshev, client_path_deltas_to_steps, decay_jumped_tiles,
    effective_use_distance, format_pm_body, in_use_range, in_use_range_ex, is_moving,
    jump_quad_with_floor, jump_rate_limited, movement_age_allowed, reconcile_mid_path_tile,
    springy_door_open_id, steps_to_client_path_deltas, still_waiting_for_force,
    quad_dist as move_quad_dist, resolve_move_seq, round2, truncate_walkable, MovePath,
    MoveReject, DEFAULT_MOVE_SPEED, EXHAUSTION_ON_JUMP, LET_THE_CLIENT_CHEAT_LITTLE_BIT_FACTOR,
    MAX_CLIENT_PATH_STEPS, MAX_JUMPS_PER_TEN_SEC, MAX_MOVE_QUAD_JUMP_BEFORE_FORCE,
    MIN_MOVEMENT_AGE_IN_SEC, SPRINGY_DOOR_CLOSED_H, SPRINGY_DOOR_CLOSED_V, SPRINGY_DOOR_OPEN_H,
    SPRINGY_DOOR_OPEN_V, WAIT_FOR_FORCE_SECS,
};"#;

    if !t.contains("calculate_new_pos") || !t.contains("LET_THE_CLIENT_CHEAT_LITTLE_BIT_FACTOR") {
        if t.contains(old_export) {
            t = t.replacen(old_export, new_export, 1);
        } else if t.contains("pub use move_path::{") && !t.contains("reconcile_mid_path_tile") {
            // Soft inject: expand export list if symbols missing.
            if let Some(start) = t.find("pub use move_path::{") {
                if let Some(rel) = t[start..].find("\n};") {
                    let end = start + rel + "\n};".len();
                    t = format!("{}{}{}", &t[..start], new_export, &t[end..]);
                }
            }
        }
    }

    // ── recon before jump gate ───────────────────────────────────────────
    let old_jump = r#"    // Haxe MoveHelper.moveHelper:
    //   if isBlocked(clientStart) || quadDist > MaxMovementQuadJumpDistanceBeforeForce(5)
    //     → CancleMovement (no snap).
    // Else if jump: snap to client start (positionChanged), then accept path.
    // Mid-path new MOVE replaces the path (Haxe always overwrites newMoves).
    //
    // Timed path jump gate is Haxe quadDist ≤ 5, softened /10 when floorId>0.
    // `move_jump_max_chebyshev` applies to instant MOVE only.
    let raw_jump_quad = move_quad_dist(xs, ys, px, py) as f64;"#;

    let new_jump = r#"    // MOVE-MIDPATH: before jump gate, reconcile integer tile from path time.
    // Haxe calculateNewPos is dead in legacy; wire residual so mid-path MOVE
    // uses half-step progress (not only last committed tile).
    // Haxe: MoveHelper.calculateNewPos L853-877
    let (px, py) = {
        let mut px = px;
        let mut py = py;
        let wrap_world = {
            let w = state.world.read().unwrap();
            (w.width_tiles, w.height_tiles, w.wrap)
        };
        if let Some(p) = state.players.get_mut(&conn_id) {
            if let Some(path) = p.move_path.as_ref() {
                let (ww, hh, wrap_on) = wrap_world;
                let (rx, ry) = reconcile_mid_path_tile(path, sim_time, &|x, y| {
                    if wrap_on && ww > 0 && hh > 0 {
                        (x.rem_euclid(ww), y.rem_euclid(hh))
                    } else {
                        (x, y)
                    }
                });
                if (rx, ry) != (p.x, p.y) {
                    p.x = rx;
                    p.y = ry;
                    // Keep exact in sync with recon tile (half-step commit).
                    if let Some(path_m) = p.move_path.as_mut() {
                        path_m.exact_x = rx as f32;
                        path_m.exact_y = ry as f32;
                    }
                    debug!(
                        conn_id,
                        from_x = px,
                        from_y = py,
                        recon_x = rx,
                        recon_y = ry,
                        "sim: MOVE mid-path recon calculateNewPos"
                    );
                    px = rx;
                    py = ry;
                }
            }
        }
        (px, py)
    };
    // Haxe MoveHelper.moveHelper:
    //   if isBlocked(clientStart) || quadDist > MaxMovementQuadJumpDistanceBeforeForce(5)
    //     → CancleMovement (no snap).
    // Else if jump: snap to client start (positionChanged), then accept path.
    // Mid-path new MOVE replaces the path (Haxe always overwrites newMoves).
    //
    // Timed path jump gate is Haxe quadDist ≤ 5, softened /10 when floorId>0.
    // `move_jump_max_chebyshev` applies to instant MOVE only.
    let raw_jump_quad = move_quad_dist(xs, ys, px, py) as f64;"#;

    if !t.contains("MOVE-MIDPATH: before jump gate") {
        if t.contains(old_jump) {
            t = t.replacen(old_jump, new_jump, 1);
        }
    }

    // ── start_sim_time on path install ───────────────────────────────────
    let old_install = r#"    let path = build_move_path(
        start_x,
        start_y,
        accepted.clone(),
        speed,
        seq,
        trunc,
        state.tick,
    );
    let total = path.total_sec;"#;

    let new_install = r#"    let mut path = build_move_path(
        start_x,
        start_y,
        accepted.clone(),
        speed,
        seq,
        trunc,
        state.tick,
    );
    // Haxe startingMoveTicks → sim_time for calculateNewPos recon on next MOVE.
    // Haxe: MoveHelper.moveHelper L653
    path.start_sim_time = state.sim_time;
    let total = path.total_sec;"#;

    if !t.contains("path.start_sim_time = state.sim_time") {
        if t.contains(old_install) {
            t = t.replacen(old_install, new_install, 1);
        }
    }

    // ── live test ────────────────────────────────────────────────────────
    let old_test = r#"    fn path_replace_mid_move_clears_residual() {
        let mut state = SimState::with_default_empty(test_content());
        state.timed_movement = true;
        spawn_player(&mut state, 1, "rep@t");
        set_player_position(&mut state, 1, 0, 0);
        let hub = OutboundHub::new();
        apply_move_path_start(
            &mut state,
            &hub,
            1,
            0,
            0,
            &[(1, 0), (1, 0), (1, 0)],
            Some(1),
        )
        .unwrap();
        tick_move_paths(&mut state, 0.01, &hub);
        apply_move_path_start(&mut state, &hub, 1, 0, 0, &[(0, 1)], Some(2)).unwrap();
        let path = state.players.get(&1).unwrap().move_path.as_ref().unwrap();
        assert_eq!(path.remaining.len(), 1);
        assert_eq!(path.remaining[0], (0, 1));
        assert_eq!(path.seq, 2);
    }"#;

    let new_test = r#"    fn path_replace_mid_move_clears_residual() {
        let mut state = SimState::with_default_empty(test_content());
        state.timed_movement = true;
        spawn_player(&mut state, 1, "rep@t");
        set_player_position(&mut state, 1, 0, 0);
        let hub = OutboundHub::new();
        apply_move_path_start(
            &mut state,
            &hub,
            1,
            0,
            0,
            &[(1, 0), (1, 0), (1, 0)],
            Some(1),
        )
        .unwrap();
        tick_move_paths(&mut state, 0.01, &hub);
        apply_move_path_start(&mut state, &hub, 1, 0, 0, &[(0, 1)], Some(2)).unwrap();
        let path = state.players.get(&1).unwrap().move_path.as_ref().unwrap();
        assert_eq!(path.remaining.len(), 1);
        assert_eq!(path.remaining[0], (0, 1));
        assert_eq!(path.seq, 2);
    }

    /// MOVE-MIDPATH: half-step recon advances server tile before path replace.
    // Haxe: MoveHelper.calculateNewPos
    #[test]
    fn mid_path_recon_commits_half_step_before_replace() {
        let mut state = SimState::with_default_empty(test_content());
        state.timed_movement = true;
        spawn_player(&mut state, 1, "recon@t");
        set_player_position(&mut state, 1, 0, 0);
        let hub = OutboundHub::new();
        apply_move_path_start(
            &mut state,
            &hub,
            1,
            0,
            0,
            &[(1, 0), (1, 0)],
            Some(1),
        )
        .unwrap();
        // Overwrite path with speed=1 and start_sim_time so recon is deterministic.
        {
            let p = state.players.get_mut(&1).unwrap();
            let mut path = build_move_path(0, 0, vec![(1, 0), (1, 0)], 1.0, 1, 0, 0);
            path.start_sim_time = state.sim_time;
            p.move_path = Some(path);
            p.moving = true;
            p.x = 0;
            p.y = 0;
        }
        // Advance sim_time by 0.6s without tick_move_paths (recon uses calculateNewPos only).
        state.sim_time += 0.6;
        // Client claims (1,0); recon also snaps server to (1,0) (past half of first step).
        apply_move_path_start(&mut state, &hub, 1, 1, 0, &[(1, 0)], Some(2)).unwrap();
        let p = state.players.get(&1).unwrap();
        assert_eq!((p.x, p.y), (1, 0));
        assert_eq!(p.move_path.as_ref().unwrap().seq, 2);
        assert_eq!(p.move_path.as_ref().unwrap().start_sim_time, state.sim_time);
    }"#;

    if !t.contains("mid_path_recon_commits_half_step_before_replace") {
        if t.contains(old_test) {
            t = t.replacen(old_test, new_test, 1);
        }
    }

    if t == orig {
        return move_midpath_wired(&t);
    }
    std::fs::write(lib_path, restore_nl(&t, crlf)).is_ok()
}
