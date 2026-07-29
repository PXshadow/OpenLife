//! PATH-REACH / not_reachable_maps — build-time pure-Rust wire (idempotent).
//!
//! Live `AiBase.notReachableObjects` + `objectsWithHostilePath` + `blockedByAI`:
//! - `mod ai_path_reach` + exports on lib.rs
//! - `Player.ai_path_reach` + `SimState.blocked_by_ai` + tick_vitals cleanup
//! - profession_scan filter tiles + failed USE mark
//! - npc_ai path_reach state + filter + walk-fail mark
//! - docs + tests
//!
//! // Haxe: AiBase L85–86, cleanupBlockedObjects ~6258, addNotReachable ~9265

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
        eprintln!("cargo:warning=PATH-REACH write {}: {e}", path.display());
        return false;
    }
    true
}

/// True when live maps + lib mod + profession filter helpers are present.
pub fn path_reach_wired(lib: &str, player: &str, scan: &str) -> bool {
    lib.contains("mod ai_path_reach;")
        && lib.contains("AiPathReachMaps")
        && lib.contains("blocked_by_ai")
        && lib.contains("PATH-REACH: cleanup AI path maps")
        && player.contains("ai_path_reach")
        && scan.contains("path_filters_from_player")
        && scan.contains("PATH-REACH: filter notReachableObjects")
}

fn run_python(workspace: &Path) {
    let py = workspace.join("docs/port/_apply_path_reach.py");
    if !py.exists() {
        return;
    }
    let _ = Command::new("python")
        .arg(&py)
        .status()
        .or_else(|_| Command::new("python3").arg(&py).status());
}

/// Build hook: apply PATH-REACH wire (python first, then pure RS fallback).
pub fn patch_path_reach(src: &Path, workspace: &Path) -> bool {
    println!("cargo:rerun-if-changed=build_path_reach.rs");
    println!("cargo:rerun-if-changed=src/ai_path_reach.rs");
    println!("cargo:rerun-if-changed=docs/port/_apply_path_reach.py");

    run_python(workspace);

    let lib_path = src.join("lib.rs");
    let player_path = src.join("player.rs");
    let scan_path = src.join("profession_scan.rs");
    let tests_path = src.join("profession_scan_tests.inc.rs");
    let npc_path = workspace.join("crates/ol-server/src/npc_ai.rs");

    let lib = std::fs::read_to_string(&lib_path).unwrap_or_default();
    let player = std::fs::read_to_string(&player_path).unwrap_or_default();
    let scan = std::fs::read_to_string(&scan_path).unwrap_or_default();
    if path_reach_wired(&lib, &player, &scan) {
        let stamp = src.join(".path_reach_patched");
        let _ = std::fs::write(&stamp, b"path-reach-1-source-wired\n");
        return true;
    }

    let mut any = false;
    any |= patch_lib(&lib_path);
    any |= patch_player(&player_path);
    any |= patch_profession_scan(&scan_path);
    any |= patch_tests(&tests_path);
    if npc_path.exists() {
        any |= patch_npc_ai(&npc_path);
    }
    any |= patch_docs(workspace);

    let lib2 = std::fs::read_to_string(&lib_path).unwrap_or_default();
    let player2 = std::fs::read_to_string(&player_path).unwrap_or_default();
    let scan2 = std::fs::read_to_string(&scan_path).unwrap_or_default();
    let ok = path_reach_wired(&lib2, &player2, &scan2);
    if ok {
        let stamp = src.join(".path_reach_patched");
        let _ = std::fs::write(
            &stamp,
            if any {
                b"path-reach-1-rs-patched\n".as_slice()
            } else {
                b"path-reach-1-source-wired\n"
            },
        );
    } else {
        println!("cargo:warning=PATH-REACH: could not fully wire notReachable maps");
    }
    ok || any
}

fn patch_lib(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;

    if !t.contains("mod ai_path_reach;") {
        ch |= replace_once(
            &mut t,
            "mod ai_takeover;\n",
            "mod ai_takeover;\n// Haxe: AiBase notReachableObjects / objectsWithHostilePath / blockedByAI (PATH-REACH)\nmod ai_path_reach;\n",
        );
    }

    if !t.contains("pub use ai_path_reach::") {
        ch |= replace_once(
            &mut t,
            "// Haxe: profession world-scan → shortCraft USE/DROP (CRAFT-LIVE-TICK / NPC-CRAFT-LADDER)\n",
            "// Haxe: AiBase notReachable / hostile path maps (PATH-REACH)\npub use ai_path_reach::{\n\
    add_blocked_by_ai, blocked_coords_from_live, cleanup_blocked_by_ai,\n\
    mark_not_reachable_on_player, AiPathReachMaps, BLOCKED_BY_AI_DEFAULT_SECS,\n\
    HOSTILE_PATH_DEFAULT_SECS, NOT_REACHABLE_DEFAULT_SECS, NOT_REACHABLE_FOOD_SECS,\n\
};\n// Haxe: profession world-scan → shortCraft USE/DROP (CRAFT-LIVE-TICK / NPC-CRAFT-LADDER)\n",
        );
    }

    if !t.contains("pub blocked_by_ai:") {
        ch |= replace_once(
            &mut t,
            "    /// Haxe `ServerSettings.StartingEveAge`.\n\
    // Haxe: ServerSettings.StartingEveAge\n\
    pub starting_eve_age: f32,\n\
}\n",
            "    /// Haxe `ServerSettings.StartingEveAge`.\n\
    // Haxe: ServerSettings.StartingEveAge\n\
    pub starting_eve_age: f32,\n\
    /// Haxe `AiBase.blockedByAI` — static tiles claimed by AI targets (PATH-REACH).\n\
    /// Key = absolute world (tx, ty); value = remaining block seconds.\n\
    // Haxe: AiBase.blockedByAI + AddObjBlockedByAi ~9253\n\
    pub blocked_by_ai: std::collections::HashMap<(i32, i32), f32>,\n\
}\n",
        );
    }

    if !t.contains("blocked_by_ai: HashMap::new(),") {
        ch |= replace_once(
            &mut t,
            "            spawn_at_last_dead: false,\n\
            starting_eve_age: STARTING_EVE_AGE,\n\
        }\n\
    }\n",
            "            spawn_at_last_dead: false,\n\
            starting_eve_age: STARTING_EVE_AGE,\n\
            // PATH-REACH: AiBase.blockedByAI static map\n\
            blocked_by_ai: HashMap::new(),\n\
        }\n\
    }\n",
        );
    }

    if !t.contains("PATH-REACH: cleanup AI path maps") {
        ch |= replace_once(
            &mut t,
            "    let dt = dt * speed;\n\
    state.sim_time += dt;\n\
    // REPUTATION-HIT: TimeHelper calm restore of lostCombatPrestige.\n",
            "    let dt = dt * speed;\n\
    state.sim_time += dt;\n\
    // PATH-REACH: cleanup AI path maps (Haxe cleanupBlockedObjects each reaction).\n\
    // Haxe: AiBase.cleanupBlockedObjectsHelper ~6264\n\
    {\n\
        use crate::ai_path_reach::cleanup_blocked_by_ai;\n\
        cleanup_blocked_by_ai(&mut state.blocked_by_ai, dt);\n\
        for p in state.players.values_mut() {\n\
            p.ai_path_reach.cleanup(dt);\n\
        }\n\
    }\n\
    // REPUTATION-HIT: TimeHelper calm restore of lostCombatPrestige.\n",
        );
    }

    if !t.contains("path_filters_from_player") {
        ch |= replace_once(
            &mut t,
            "    farm_profession_scan_tick, filter_scan_tiles_path, filter_scan_tiles_path_owned, floor_at_scan,\n",
            "    farm_profession_scan_tick, filter_scan_tiles_path, filter_scan_tiles_path_owned,\n\
    apply_path_filters_to_tiles, path_filters_from_player, floor_at_scan,\n",
        );
    }

    if !ch {
        return false;
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf))
}

fn patch_player(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("ai_path_reach") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;

    ch |= replace_once(
        &mut t,
        "    // MAP-LOCATION-PINS\n\
    pub allow_show_human: bool,\n\
}\n",
        "    // MAP-LOCATION-PINS\n\
    pub allow_show_human: bool,\n\
    /// Haxe `AiBase.notReachableObjects` + `objectsWithHostilePath` (PATH-REACH).\n\
    /// Session AI path-block timers; cleaned in `tick_vitals`.\n\
    // Haxe: AiBase L85–86 notReachableObjects / objectsWithHostilePath\n\
    pub ai_path_reach: crate::ai_path_reach::AiPathReachMaps,\n\
}\n",
    );

    ch |= replace_once(
        &mut t,
        "            allow_show_human: true,\n\
        }\n\
    }\n",
        "            allow_show_human: true,\n\
            // PATH-REACH: empty notReachable / hostile path maps\n\
            ai_path_reach: crate::ai_path_reach::AiPathReachMaps::default(),\n\
        }\n\
    }\n",
    );

    if !t.contains("ai_path_reach_sticky_defaults_and_survives") {
        if let Some(idx) = t.find("    fn farm_profession_sticky_defaults_and_survives()") {
            if let Some(rel) = t[idx + 10..].find("\n    fn ") {
                let next = idx + 10 + rel;
                let insert = "\n    #[test]\n\
    fn ai_path_reach_sticky_defaults_and_survives() {\n\
        // PATH-REACH: Player.ai_path_reach sticky across ticks\n\
        let mut p = Player::new(1, 1, \"path@test\");\n\
        assert!(p.ai_path_reach.is_empty());\n\
        p.ai_path_reach.add_not_reachable(10, 20, 90.0);\n\
        p.ai_path_reach.add_hostile_path(11, 21, 20.0);\n\
        assert!(p.ai_path_reach.is_personal_not_reachable(10, 20));\n\
        assert!(p.ai_path_reach.is_object_with_hostile_path(11, 21));\n\
        p.ai_path_reach.cleanup(100.0);\n\
        assert!(p.ai_path_reach.is_empty());\n\
    }\n";
                t.insert_str(next, insert);
                ch = true;
            }
        }
    }

    if !ch {
        return false;
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf))
}

fn patch_profession_scan(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;

    ch |= replace_once(
        &mut t,
        "//! Residual (deferred): full dropHeldObject AI body, useHeldObjOnTarget staging\n\
//! fields, GetOrCraftItem multi-step, Defer* baker farm tails, hungry-cost pair,\n\
//! live AI notReachableObjects map on Player (pure [`ProfessionPathFilters`] exists).\n",
        "//! Residual (deferred): full dropHeldObject AI body, useHeldObjOnTarget staging\n\
//! fields, GetOrCraftItem multi-step, Defer* baker farm tails, hungry-cost pair.\n\
//! **PATH-REACH**: live `Player.ai_path_reach` + `SimState.blocked_by_ai` filter scan tiles.\n",
    );

    if !t.contains("pub fn path_filters_from_player") {
        ch |= replace_once(
            &mut t,
            "    pub fn target_reachable(&self, x: i32, y: i32) -> bool {\n\
        !self.is_object_not_reachable(x, y)\n\
    }\n\
}\n\
\n\
/// Drop tiles marked not-reachable / hostile from a scan slice (pure).\n",
            "    pub fn target_reachable(&self, x: i32, y: i32) -> bool {\n\
        !self.is_object_not_reachable(x, y)\n\
    }\n\
\n\
    #[inline]\n\
    pub fn is_empty(&self) -> bool {\n\
        self.blocked.is_empty()\n\
    }\n\
}\n\
\n\
/// Build live filters from Player maps + SimState.blocked_by_ai (PATH-REACH).\n\
// Haxe: isObjectNotReachable || isObjectWithHostilePath (+ blockedByAI)\n\
#[inline]\n\
pub fn path_filters_from_player(\n\
    maps: &crate::ai_path_reach::AiPathReachMaps,\n\
    blocked_by_ai: &std::collections::HashMap<(i32, i32), f32>,\n\
) -> ProfessionPathFilters {\n\
    ProfessionPathFilters::with_blocked(maps.blocked_coords(Some(blocked_by_ai)))\n\
}\n\
\n\
/// Filter scan tiles when filters non-empty; otherwise return owned copy.\n\
// Haxe: GetClosestObjectToPositionHelper skip not reachable\n\
pub fn apply_path_filters_to_tiles(\n\
    tiles: &[ScanTile],\n\
    filters: &ProfessionPathFilters,\n\
) -> Vec<ScanTile> {\n\
    if filters.is_empty() {\n\
        tiles.to_vec()\n\
    } else {\n\
        filter_scan_tiles_path_owned(tiles, filters)\n\
    }\n\
}\n\
\n\
/// Drop tiles marked not-reachable / hostile from a scan slice (pure).\n",
        );
    }

    t = t.replace(
        "/// Live Player maps are residual; pure scans take an explicit filter.\n",
        "/// Live maps: [`crate::ai_path_reach::AiPathReachMaps`] on Player (PATH-REACH).\n",
    );

    if !t.contains("PATH-REACH: filter notReachableObjects") {
        ch |= replace_once(
            &mut t,
            "    let tiles = {\n\
        let world = state.world.read().unwrap();\n\
        // Haxe: pottery dual home craft + player clay r=80 when ladder includes Pottery\n\
        if steps.iter().any(|s| s.kind == ProfessionScanKind::Pottery) {\n\
            pottery_scan_tiles_from_world(\n\
                &world,\n\
                Some(&state.content),\n\
                home_x,\n\
                home_y,\n\
                px,\n\
                py,\n\
            )\n\
        } else {\n\
            scan_world_radius(&world, Some(&state.content), home_x, home_y, scan_r)\n\
        }\n\
    };\n\
\n\
    let has_carrot_seeds = has_carrot_seeds_from_scan(&tiles);\n",
            "    let mut tiles = {\n\
        let world = state.world.read().unwrap();\n\
        // Haxe: pottery dual home craft + player clay r=80 when ladder includes Pottery\n\
        if steps.iter().any(|s| s.kind == ProfessionScanKind::Pottery) {\n\
            pottery_scan_tiles_from_world(\n\
                &world,\n\
                Some(&state.content),\n\
                home_x,\n\
                home_y,\n\
                px,\n\
                py,\n\
            )\n\
        } else {\n\
            scan_world_radius(&world, Some(&state.content), home_x, home_y, scan_r)\n\
        }\n\
    };\n\
\n\
    // PATH-REACH: filter notReachableObjects / hostile / blockedByAI before profession picks.\n\
    // Haxe: GetClosestObjectToPositionHelper isObjectNotReachable skip\n\
    {\n\
        let p = state.players.get(&conn_id).expect(\"player checked above\");\n\
        let filters = path_filters_from_player(&p.ai_path_reach, &state.blocked_by_ai);\n\
        tiles = apply_path_filters_to_tiles(&tiles, &filters);\n\
    }\n\
\n\
    let has_carrot_seeds = has_carrot_seeds_from_scan(&tiles);\n",
        );

        ch |= replace_once(
            &mut t,
            "    let tiles = {\n\
        let world = state.world.read().unwrap();\n\
        match kind {\n\
            // Haxe: gatherClay deposit/pit r=80 player + doPottery home craft r=30\n\
            ProfessionScanKind::Pottery => pottery_scan_tiles_from_world(\n\
                &world,\n\
                Some(&state.content),\n\
                home_x,\n\
                home_y,\n\
                px,\n\
                py,\n\
            ),\n\
            other => {\n\
                let scan_r = match other {\n\
                    ProfessionScanKind::Farm => DEFAULT_PROFESSION_SCAN_RADIUS,\n\
                    ProfessionScanKind::Smith => SMITH_SCAN_RADIUS,\n\
                    ProfessionScanKind::Baker => BAKER_SCAN_RADIUS,\n\
                    ProfessionScanKind::Pottery => POTTERY_SCAN_RADIUS,\n\
                    ProfessionScanKind::Shepherd => SHEPHERD_SHORTCRAFT_RADIUS,\n\
                };\n\
                scan_world_radius(&world, Some(&state.content), home_x, home_y, scan_r)\n\
            }\n\
        }\n\
    };\n\
\n\
    // Haxe: countSeeds / hasBeanSeeds from current objects near home scan.\n",
            "    let mut tiles = {\n\
        let world = state.world.read().unwrap();\n\
        match kind {\n\
            // Haxe: gatherClay deposit/pit r=80 player + doPottery home craft r=30\n\
            ProfessionScanKind::Pottery => pottery_scan_tiles_from_world(\n\
                &world,\n\
                Some(&state.content),\n\
                home_x,\n\
                home_y,\n\
                px,\n\
                py,\n\
            ),\n\
            other => {\n\
                let scan_r = match other {\n\
                    ProfessionScanKind::Farm => DEFAULT_PROFESSION_SCAN_RADIUS,\n\
                    ProfessionScanKind::Smith => SMITH_SCAN_RADIUS,\n\
                    ProfessionScanKind::Baker => BAKER_SCAN_RADIUS,\n\
                    ProfessionScanKind::Pottery => POTTERY_SCAN_RADIUS,\n\
                    ProfessionScanKind::Shepherd => SHEPHERD_SHORTCRAFT_RADIUS,\n\
                };\n\
                scan_world_radius(&world, Some(&state.content), home_x, home_y, scan_r)\n\
            }\n\
        }\n\
    };\n\
\n\
    // PATH-REACH: filter notReachableObjects / hostile / blockedByAI before profession picks.\n\
    // Haxe: GetClosestObjectToPositionHelper isObjectNotReachable skip\n\
    {\n\
        let p = state.players.get(&conn_id).expect(\"player checked above\");\n\
        let filters = path_filters_from_player(&p.ai_path_reach, &state.blocked_by_ai);\n\
        tiles = apply_path_filters_to_tiles(&tiles, &filters);\n\
    }\n\
\n\
    // Haxe: countSeeds / hasBeanSeeds from current objects near home scan.\n",
        );
    }

    let before = t.clone();
    t = t.replace(
        "        // Residual: live AI notReachableObjects map not on Player yet — pure filters exist.\n\
        target_reachable: true,\n",
        "        // PATH-REACH: tiles already filtered via Player.ai_path_reach + blocked_by_ai.\n\
        target_reachable: true,\n",
    );
    t = t.replace(
        "        // Residual: live AI notReachableObjects map not on Player — pure ProfessionPathFilters.\n\
        target_reachable: true,\n",
        "        // PATH-REACH: tiles already filtered via Player.ai_path_reach + blocked_by_ai.\n\
        target_reachable: true,\n",
    );
    if t != before {
        ch = true;
    }

    if !t.contains("PATH-REACH: failed USE → addNotReachableObject") {
        let fail_block = "    // PATH-REACH: failed USE → addNotReachableObject (Haxe ~8916 / ~9133).\n\
    let intent = result.intent;\n\
    let apply_r = apply_short_craft_live_intent(state, outbound, conn_id, intent);\n\
    if matches!(apply_r, ShortCraftLiveApplyResult::Failed) {\n\
        if let ShortCraftLiveIntent::UseAt { x, y, .. }\n\
        | ShortCraftLiveIntent::UseOnEmptyGround { x, y, .. } = intent\n\
        {\n\
            if let Some(p) = state.players.get_mut(&conn_id) {\n\
                crate::mark_not_reachable_on_player(\n\
                    &mut p.ai_path_reach,\n\
                    x,\n\
                    y,\n\
                    crate::NOT_REACHABLE_DEFAULT_SECS,\n\
                );\n\
            }\n\
        }\n\
    }\n\
    apply_r\n";

        ch |= replace_once(
            &mut t,
            "    apply_short_craft_live_intent(state, outbound, conn_id, result.intent)\n\
}\n\
\n\
/// Convenience: resolve rung from sticky job flags + vitals, then apply ladder scan.\n",
            &format!(
                "{fail_block}}}\n\n/// Convenience: resolve rung from sticky job flags + vitals, then apply ladder scan.\n"
            ),
        );

        ch |= replace_once(
            &mut t,
            "    apply_short_craft_live_intent(state, outbound, conn_id, result.intent)\n\
}\n\
\n\
// ── Unit tests ──────────────────────────────────────────────────────────────\n",
            &format!("{fail_block}}}\n\n// ── Unit tests ──────────────────────────────────────────────────────────────\n"),
        );
    }

    if !ch && t == normalize_nl(&raw) {
        return false;
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf))
}

fn patch_tests(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("path_reach_maps_filter_and_cleanup") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    t.push_str(
        r#"
#[test]
fn path_reach_maps_filter_and_cleanup() {
    // PATH-REACH: timed maps → ProfessionPathFilters → scan filter
    let mut maps = crate::AiPathReachMaps::new();
    maps.add_not_reachable(1, 0, 90.0);
    maps.add_hostile_path(3, 0, 20.0);
    let mut global = std::collections::HashMap::new();
    crate::add_blocked_by_ai(&mut global, 2, 0, 5.0);
    let filters = path_filters_from_player(&maps, &global);
    let tiles = vec![
        ScanTile::simple(DYING_BUSH, 1, 0),
        ScanTile::simple(DYING_BUSH, 2, 0),
        ScanTile::simple(DYING_BUSH, 3, 0),
        ScanTile::simple(DYING_BUSH, 5, 0),
    ];
    let kept = apply_path_filters_to_tiles(&tiles, &filters);
    assert_eq!(kept.len(), 1);
    assert_eq!((kept[0].x, kept[0].y), (5, 0));

    maps.cleanup(100.0);
    assert!(maps.is_empty());
    crate::cleanup_blocked_by_ai(&mut global, 10.0);
    assert!(global.is_empty());
}

#[test]
fn mark_not_reachable_blocks_subsequent_filter() {
    let mut maps = crate::AiPathReachMaps::new();
    crate::mark_not_reachable_on_player(&mut maps, 7, 8, crate::NOT_REACHABLE_DEFAULT_SECS);
    assert!(maps.is_personal_not_reachable(7, 8));
    let f = path_filters_from_player(&maps, &std::collections::HashMap::new());
    assert!(!f.target_reachable(7, 8));
}
"#,
    );
    write_if_changed(path, &raw, &restore_nl(&t, crlf))
}

fn patch_npc_ai(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("path_reach:") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;

    ch |= replace_once(
        &mut t,
        "use ol_sim::{\n\
    evaluate_nearby_crafts, force_drop_at_feet, has_bean_seeds_from_scan, has_carrot_seeds_from_scan,\n\
    is_walkable, ladder_profession_scan_tick, next_step, plan_profession_ladder_steps,\n\
    scan_world_radius, self_clothing_raw_payload, smart_drop_held_from_sensors,\n\
    BakerProfessionRuntime, BakerTaskState, CraftProfession, DropHeldSensorExtras, FarmProfession,\n\
    FarmTaskState, NearbyObj, PlayerSnapshot, PotterProfessionRuntime, PriorityRung,\n\
    ProfessionScanInput, ProfessionScanKind, ProfessionStickySnapshot, ShepherdProfessionRuntime,\n\
    ShortCraftLiveIntent, SmithProfessionRuntime, BAKER_SCAN_RADIUS, DEFAULT_CRAFT_RADIUS,\n\
    DEFAULT_PROFESSION_SCAN_RADIUS, DEFAULT_WALK_SPEED, INTERACTION_SEC, POTTERY_SCAN_RADIUS,\n\
    SHEPHERD_SHORTCRAFT_RADIUS, SMITH_SCAN_RADIUS,\n\
};\n",
        "use ol_sim::{\n\
    apply_path_filters_to_tiles, blocked_coords_from_live, evaluate_nearby_crafts,\n\
    force_drop_at_feet, has_bean_seeds_from_scan, has_carrot_seeds_from_scan, is_walkable,\n\
    ladder_profession_scan_tick, next_step, plan_profession_ladder_steps,\n\
    scan_world_radius, self_clothing_raw_payload, smart_drop_held_from_sensors,\n\
    AiPathReachMaps, BakerProfessionRuntime, BakerTaskState, CraftProfession,\n\
    DropHeldSensorExtras, FarmProfession, FarmTaskState, NearbyObj, PlayerSnapshot,\n\
    PotterProfessionRuntime, PriorityRung, ProfessionPathFilters, ProfessionScanInput,\n\
    ProfessionScanKind, ProfessionStickySnapshot, ShepherdProfessionRuntime,\n\
    ShortCraftLiveIntent, SmithProfessionRuntime, BAKER_SCAN_RADIUS, DEFAULT_CRAFT_RADIUS,\n\
    DEFAULT_PROFESSION_SCAN_RADIUS, DEFAULT_WALK_SPEED, INTERACTION_SEC,\n\
    NOT_REACHABLE_DEFAULT_SECS, POTTERY_SCAN_RADIUS, SHEPHERD_SHORTCRAFT_RADIUS,\n\
    SMITH_SCAN_RADIUS,\n\
};\n",
    );

    ch |= replace_once(
        &mut t,
        "struct NpcProfessionState {\n\
    farm_task: FarmTaskState,\n\
    smith_rt: SmithProfessionRuntime,\n\
    baker_rt: BakerProfessionRuntime,\n\
    baker_task: BakerTaskState,\n\
    shepherd_rt: ShepherdProfessionRuntime,\n\
    pottery_rt: PotterProfessionRuntime,\n\
}\n",
        "struct NpcProfessionState {\n\
    farm_task: FarmTaskState,\n\
    smith_rt: SmithProfessionRuntime,\n\
    baker_rt: BakerProfessionRuntime,\n\
    baker_task: BakerTaskState,\n\
    shepherd_rt: ShepherdProfessionRuntime,\n\
    pottery_rt: PotterProfessionRuntime,\n\
    /// Haxe AiBase notReachableObjects / objectsWithHostilePath (PATH-REACH).\n\
    path_reach: AiPathReachMaps,\n\
}\n",
    );

    ch |= replace_once(
        &mut t,
        "                    let tiles = {\n\
                        let w = world.read().unwrap();\n\
                        scan_world_radius(&w, Some(content.as_ref()), home_x, home_y, scan_r)\n\
                    };\n\
                    let inp = ProfessionScanInput {\n\
                        player_x: p.x,\n\
                        player_y: p.y,\n\
                        home_x,\n\
                        home_y,\n\
                        held_id: p.held_id,\n\
                        held_uses: p.held_uses.max(1),\n\
                        food_store: p.food,\n\
                        transition_hungry_cost: 0.0,\n\
                        has_carrot_seeds: has_carrot_seeds_from_scan(&tiles),\n\
                        has_bean_seeds: has_bean_seeds_from_scan(&tiles),\n\
                        is_hungry: false,\n\
                        basic_farmer_weight: 1.0,\n\
                        hardened_row_biome: None,\n\
                        target_reachable: true,\n\
                        peer_count: 0.0,\n\
                        was_idle: if sticky.has_sticky_profession() { 0.0 } else { 1.0 },\n\
                        age: p.age,\n\
                        profession_is_sticky: sticky.has_sticky_profession(),\n\
                        is_assigned_job: sticky.has_assigned_job(),\n\
                    };\n\
                    let st = profession_state.entry(conn_id).or_default();\n",
        "                    let st = profession_state.entry(conn_id).or_default();\n\
                    // PATH-REACH: decay maps ~think period (Haxe cleanupBlockedObjects reactionTime).\n\
                    let think_secs = (think_period as f32) * 0.2;\n\
                    st.path_reach.cleanup(think_secs);\n\
                    let path_filters = ProfessionPathFilters::with_blocked(\n\
                        blocked_coords_from_live(&st.path_reach, &std::collections::HashMap::new()),\n\
                    );\n\
                    let tiles = {\n\
                        let w = world.read().unwrap();\n\
                        let raw = scan_world_radius(&w, Some(content.as_ref()), home_x, home_y, scan_r);\n\
                        apply_path_filters_to_tiles(&raw, &path_filters)\n\
                    };\n\
                    let inp = ProfessionScanInput {\n\
                        player_x: p.x,\n\
                        player_y: p.y,\n\
                        home_x,\n\
                        home_y,\n\
                        held_id: p.held_id,\n\
                        held_uses: p.held_uses.max(1),\n\
                        food_store: p.food,\n\
                        transition_hungry_cost: 0.0,\n\
                        has_carrot_seeds: has_carrot_seeds_from_scan(&tiles),\n\
                        has_bean_seeds: has_bean_seeds_from_scan(&tiles),\n\
                        is_hungry: false,\n\
                        basic_farmer_weight: 1.0,\n\
                        hardened_row_biome: None,\n\
                        // PATH-REACH: tiles pre-filtered via path_reach maps\n\
                        target_reachable: true,\n\
                        peer_count: 0.0,\n\
                        was_idle: if sticky.has_sticky_profession() { 0.0 } else { 1.0 },\n\
                        age: p.age,\n\
                        profession_is_sticky: sticky.has_sticky_profession(),\n\
                        is_assigned_job: sticky.has_assigned_job(),\n\
                    };\n",
    );

    ch |= replace_once(
        &mut t,
        "                                            game_ms = 250;\n\
                                            acted = true;\n\
                                        }\n\
                                    }\n\
                                }\n\
                            }\n\
                            ShortCraftLiveIntent::UseOnEmptyGround { x, y, held } => {\n",
        "                                            game_ms = 250;\n\
                                            acted = true;\n\
                                        }\n\
                                    } else {\n\
                                        // PATH-REACH: path blocked → Haxe addNotReachable (AiHelper goto fail)\n\
                                        st.path_reach.add_not_reachable(\n\
                                            x,\n\
                                            y,\n\
                                            NOT_REACHABLE_DEFAULT_SECS,\n\
                                        );\n\
                                    }\n\
                                }\n\
                            }\n\
                            ShortCraftLiveIntent::UseOnEmptyGround { x, y, held } => {\n",
    );

    if !ch {
        return false;
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf))
}

fn patch_docs(workspace: &Path) -> bool {
    let docs = workspace.join("docs/port");
    let mut any = false;

    // FILE_MATRIX
    let fm_path = docs.join("FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&fm_path) {
        if !raw.contains("**PATH-REACH**") {
            let next = raw.replace(
                "| **NPC-SCAN-RESID** / peers_path_nest | peer wound/follow + clay basket nest + path filters | **PARTIAL → solid chunk** | peers wound/follow; clay-in-basket; path filters pure. Residual: live Player notReachableObjects map |",
                "| **NPC-SCAN-RESID** / peers_path_nest | peer wound/follow + clay basket nest + path filters | **PARTIAL → solid chunk** | peers wound/follow; clay-in-basket; path filters pure. Live path maps → **PATH-REACH** |\n\
| **PATH-REACH** / not_reachable_maps | AiBase notReachableObjects / hostile / blockedByAI | **PARTIAL** | `Player.ai_path_reach` + `SimState.blocked_by_ai`; tick_vitals cleanup; apply/npc tile filter; failed USE mark. Residual: full CalculateBlockedByAi rebuild; food-search not_reachable |",
            );
            any |= write_if_changed(&fm_path, &raw, &next);
        }
    }

    // TODO_PORT
    let todo_path = docs.join("TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo_path) {
        if !raw.contains("**PATH-REACH not_reachable") {
            let mut t = raw.clone();
            t = t.replace(
                "Residual: live Player notReachableObjects → apply still hardcodes target_reachable=true",
                "Live path maps → **PATH-REACH** PARTIAL (Player.ai_path_reach + apply filter)",
            );
            t = t.replace(
                "live AI notReachable map / full dropHeldObject",
                "full dropHeldObject / PATH-REACH residual CalculateBlockedByAi",
            );
            t = t.replace(
                "live path-filter map; hungry-cost pair",
                "hungry-cost pair; PATH-REACH residual global rebuild",
            );
            t = t.replace(
                "- [~] **NPC-SCAN-RESID peers_path_nest PARTIAL**",
                "- [~] **PATH-REACH not_reachable_maps PARTIAL** — `AiPathReachMaps` on `Player` + `SimState.blocked_by_ai`; defaults 90s/20s/5s; tick_vitals cleanup; profession apply + npc_ai filter tiles; failed USE/walk mark. Residual: full `CalculateBlockedByAi` from living AI targets; search_best_food not_reachable live\n\
- [~] **NPC-SCAN-RESID peers_path_nest PARTIAL**",
            );
            if !t.contains("**PATH-REACH not_reachable_maps PARTIAL**:") {
                t = t.replace(
                    "| 2026-07-28 | **NPC-SCAN-RESID",
                    "| 2026-07-28 | **PATH-REACH not_reachable_maps PARTIAL**: `ai_path_reach.rs` timed maps; `Player.ai_path_reach` + `SimState.blocked_by_ai`; tick_vitals cleanup; profession_scan + npc_ai tile filter; failed USE/walk mark; tests path_reach_*. Residual: CalculateBlockedByAi rebuild; food-search wire |\n\
| 2026-07-28 | **NPC-SCAN-RESID",
                );
            }
            any |= write_if_changed(&todo_path, &raw, &t);
        }
    }

    // CALL_INDEX
    let ci_path = docs.join("CALL_INDEX.md");
    if let Ok(raw) = std::fs::read_to_string(&ci_path) {
        if !raw.contains("AiPathReachMaps") {
            let next = raw.replace(
                "| `ProfessionPathFilters` / `filter_scan_tiles_path` / `filter_scan_tiles_path_owned` / `closest_by_parent_id_path` / `target_reachable_for_tile` | same | **NPC-SCAN-RESID** isObjectNotReachable/hostile pure |",
                "| `ProfessionPathFilters` / `filter_scan_tiles_path` / `filter_scan_tiles_path_owned` / `closest_by_parent_id_path` / `target_reachable_for_tile` | same | **NPC-SCAN-RESID** isObjectNotReachable/hostile pure |\n\
| `AiPathReachMaps` / `add_not_reachable` / `add_hostile_path` / `cleanup` / `blocked_coords` / `path_filters_from_player` / `apply_path_filters_to_tiles` | `ol-sim/src/ai_path_reach.rs` + `profession_scan` | **PATH-REACH** live notReachableObjects/hostile/blockedByAI |\n\
| `Player.ai_path_reach` / `SimState.blocked_by_ai` | `player.rs` / `lib.rs` | **PATH-REACH** sticky maps + tick_vitals decay |",
            );
            any |= write_if_changed(&ci_path, &raw, &next);
        }
    }

    // QUEUE
    let q_path = docs.join("QUEUE.md");
    if let Ok(raw) = std::fs::read_to_string(&q_path) {
        if raw.contains("| `PATH-REACH` |") {
            let mut t = raw.replace(
                "| `PATH-REACH` | not_reachable_maps | Live notReachableObjects / hostile path maps |\n",
                "",
            );
            t = t.replace(
                "## Done recently (do not re-queue)\n\n",
                "## Done recently (do not re-queue)\n\n**PATH-REACH** PARTIAL (live maps + apply/npc filter) · ",
            );
            any |= write_if_changed(&q_path, &raw, &t);
        }
    }

    any
}
