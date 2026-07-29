//! S-MOVE road_floor_speed build-time patch (included from build.rs).

use std::path::PathBuf;

/// Expand move_notes exports + player_move_speed + path start wire.
///
/// Pure helpers live in `move_notes` → nested `move_speed.rs` (no top-level mod).
pub fn patch_lib_s_move_road_floor(lib_path: &PathBuf) -> bool {
    let Ok(mut text) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    let orig = text.clone();

    // Expand move_notes re-exports (floor/road symbols).
    let old_mn = "pub use move_notes::{\n    ballast_speed_mult, compose_move_speed, format_speed_query, format_weight_query,\n    weight_item_count, BALLAST_PER_ITEM,\n};";
    let new_mn = "pub use move_notes::{\n    ballast_speed_mult, compose_move_speed, compose_move_speed_with_floor,\n    floor_counts_as_road, floor_road_biome_factor, floor_road_factor_at, floor_speed_mult,\n    format_speed_query, format_weight_query, scan_path_road_and_biome,\n    soften_contained_speed_on_floor, soften_held_speed_on_floor, tile_biome_speed,\n    weight_item_count, PathRoadScan, BALLAST_PER_ITEM, INITIAL_PLAYER_MOVE_SPEED,\n    MIN_BIOME_SPEED_FACTOR, ROAD_SPEED_THRESHOLD,\n};";
    if text.contains(old_mn) {
        text = text.replacen(old_mn, new_mn, 1);
    }

    // player_move_speed: multiply floor/road/biome (standing → full_path_has_road = true)
    let old_pms = "/// Reported move speed for PU / FX from ride + weather + snow + fire + ballast.\nfn player_move_speed(state: &SimState, p: &Player) -> f32 {\n    let ballast = weight_item_count(p.held_id, p.backpack.len());\n    compose_move_speed(\n        p.riding,\n        &state.weather,\n        &state.snow,\n        &state.fire,\n        p.x,\n        p.y,\n        ballast,\n    )\n}";
    let new_pms = r#"/// Reported move speed for PU / FX from ride + weather + snow + fire + ballast + floor/road/biome.
///
/// Haxe `MoveHelper.calculateSpeed` floor/road portion (standing: `fullPathHasRoad=true`).
fn player_move_speed(state: &SimState, p: &Player) -> f32 {
    let ballast = weight_item_count(p.held_id, p.backpack.len());
    let base = compose_move_speed(
        p.riding,
        &state.weather,
        &state.snow,
        &state.fire,
        p.x,
        p.y,
        ballast,
    );
    // Standing / report path: Haxe default fullPathHasRoad = true.
    let world = state.world.read().unwrap();
    let floor_factor = floor_road_factor_at(&world, &state.content, p.x, p.y, true, false);
    base * floor_factor
}"#;
    if text.contains(old_pms) {
        text = text.replacen(old_pms, new_pms, 1);
    } else if !text.contains("floor_road_factor_at(&world, &state.content, p.x, p.y, true, false)") {
        if let Some(start) = text.find(
            "/// Reported move speed for PU / FX from ride + weather + snow + fire + ballast.\nfn player_move_speed",
        ) {
            if let Some(rel) = text[start..].find("\n}\n\n/// FX food-change line") {
                let end = start + rel + 1;
                text = format!("{}{}{}", &text[..start], new_pms, &text[end..]);
            }
        }
    }

    // apply_move_path_start: walk truncate → road/biome scan → speed with full_path_has_road
    let old_path = "    let (accepted, trunc) = {\n        let t0 = ol_metrics::ScopeTimer::start();\n        let world = state.world.read().unwrap();\n        let out = truncate_walkable(&world, &state.content, start_x, start_y, deltas);\n        state.last_lock_wait_us = t0.elapsed().as_micros().min(u128::from(u32::MAX)) as u32;\n        out\n    };\n    if accepted.is_empty() {\n        return Err(MoveReject::EmptyPath);\n    }\n    let (speed, seq) = {\n        let p = state.players.get(&conn_id).ok_or(MoveReject::NoPlayer)?;\n        let seq = resolve_move_seq(p, client_seq);\n        let ballast = weight_item_count(p.held_id, p.backpack.len());\n        let speed = compose_move_speed(\n            p.riding,\n            &state.weather,\n            &state.snow,\n            &state.fire,\n            p.x,\n            p.y,\n            ballast,\n        );\n        (speed, seq)\n    };";
    let new_path = r#"    // Haxe calculateNewMovements: walkability + fullPathHasRoad + off-road biome trunc.
    let (accepted, trunc, full_path_has_road) = {
        let t0 = ol_metrics::ScopeTimer::start();
        let world = state.world.read().unwrap();
        let (walk_acc, walk_trunc) =
            truncate_walkable(&world, &state.content, start_x, start_y, deltas);
        let scan = scan_path_road_and_biome(
            &world,
            &state.content,
            start_x,
            start_y,
            &walk_acc,
            walk_trunc,
        );
        state.last_lock_wait_us = t0.elapsed().as_micros().min(u128::from(u32::MAX)) as u32;
        (scan.steps, scan.trunc, scan.full_path_has_road)
    };
    if accepted.is_empty() {
        return Err(MoveReject::EmptyPath);
    }
    let (speed, seq) = {
        let p = state.players.get(&conn_id).ok_or(MoveReject::NoPlayer)?;
        let seq = resolve_move_seq(p, client_seq);
        let ballast = weight_item_count(p.held_id, p.backpack.len());
        let base = compose_move_speed(
            p.riding,
            &state.weather,
            &state.snow,
            &state.fire,
            p.x,
            p.y,
            ballast,
        );
        // Haxe calculateSpeed(p, p.tx, p.ty, fullPathHasRoad)
        let world = state.world.read().unwrap();
        let floor_factor = floor_road_factor_at(
            &world,
            &state.content,
            start_x,
            start_y,
            full_path_has_road,
            false,
        );
        let speed = base * floor_factor;
        (speed, seq)
    };"#;
    if text.contains(old_path) {
        text = text.replacen(old_path, new_path, 1);
    } else if !text.contains("scan_path_road_and_biome(") {
        if let Some(start) = text.find(
            "    let (accepted, trunc) = {\n        let t0 = ol_metrics::ScopeTimer::start();\n        let world = state.world.read().unwrap();\n        let out = truncate_walkable(&world, &state.content, start_x, start_y, deltas);",
        ) {
            if let Some(rel) = text[start..].find("        (speed, seq)\n    };") {
                let end = start + rel + "        (speed, seq)\n    };".len();
                text = format!("{}{}{}", &text[..start], new_path, &text[end..]);
            }
        }
    }

    // Integration tests
    if !text.contains("fn player_move_speed_road_floor_boosts_on_stone_road()") {
        let needle = "    /// Ballast from held + backpack slightly reduces reported move speed.\n    #[test]\n    fn player_move_speed_ballast_from_held_and_backpack()";
        if text.contains(needle) {
            let test = r#"    /// Haxe calculateSpeed: swamp biome 0.9 slows without floor.
    #[test]
    fn player_move_speed_swamp_slows_without_floor() {
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "swamp@x");
        set_player_position(&mut state, 1, 2, 2);
        {
            let mut w = state.world.write().unwrap();
            w.set_floor(2, 2, 0);
            w.set_biome(2, 2, 1); // SWAMP
        }
        let p = state.players.get(&1).unwrap().clone();
        let spd = player_move_speed(&state, &p);
        let expected = WALK_MOVE_SPEED * 0.9;
        assert!(
            (spd - expected).abs() < 0.01,
            "swamp speed spd={spd} expected={expected}"
        );
    }

    /// Haxe calculateSpeed: Stone Road 1596 (speedMult 1.5) boosts reported speed.
    #[test]
    fn player_move_speed_road_floor_boosts_on_stone_road() {
        let mut db = (*test_content()).clone();
        let mut road = ObjectDef::empty(1596);
        road.floor = true;
        road.speed_mult = 1.5;
        road.name = "Stone Road".into();
        db.objects.insert(1596, road);
        let mut state = SimState::with_default_empty(Arc::new(db));
        spawn_player(&mut state, 1, "road@x");
        set_player_position(&mut state, 1, 5, 5);
        {
            let mut w = state.world.write().unwrap();
            w.set_floor(5, 5, 1596);
            w.set_biome(5, 5, 0); // GREEN
        }
        let p = state.players.get(&1).unwrap().clone();
        let spd = player_move_speed(&state, &p);
        let expected = WALK_MOVE_SPEED * 1.5;
        assert!(
            (spd - expected).abs() < 0.01,
            "road speed spd={spd} expected={expected}"
        );
    }

    /// Ballast from held + backpack slightly reduces reported move speed.
    #[test]
    fn player_move_speed_ballast_from_held_and_backpack()"#;
            text = text.replacen(needle, test, 1);
        }
    }

    if text == orig {
        return text.contains("floor_road_factor_at")
            && text.contains("scan_path_road_and_biome")
            && text.contains("floor_road_factor_at(&world, &state.content, p.x, p.y, true, false)");
    }
    std::fs::write(lib_path, text).is_ok()
}
