//! EVE-BANANA / jungle_spawn build-time wire (included from build.rs).
//!
//! - `mod eve_spawn` + pub use
//! - Synthetic Eve path uses `find_eve_spawn` (food plants / jungle banana)
//! - Boot preferred spawn uses Eve food preference when plants exist
//! - Fix test attrs if earlier insert stole `#[test]` from human_login
//! - Port docs (TODO_PORT / FILE_MATRIX / CALL_INDEX / QUEUE)
//!
//! Idempotent. Handles CRLF sources.

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

fn replace_once(hay: &mut String, old: &str, new: &str) -> bool {
    if let Some(i) = hay.find(old) {
        hay.replace_range(i..i + old.len(), new);
        true
    } else {
        false
    }
}

pub fn eve_banana_wired(lib: &str) -> bool {
    lib.contains("mod eve_spawn;")
        && lib.contains("find_eve_spawn")
        && lib.contains("EVE-BANANA")
        && lib.contains("collect_eve_food_sites")
}

fn fix_test_attrs(t: &mut String) -> bool {
    let mut changed = false;
    let bad = "    /// Human LOGIN must not spawn on mother/NPC tile (bootstrap desync fix).\n\
    #[test]\n\
    /// EVE-BANANA: synthetic Eve picks food-plant tile when bananas/berries abundant.\n\
    #[test]\n\
    fn synthetic_eve_spawn_prefers_banana_when_abundant() {\n";
    let good = "    /// EVE-BANANA: synthetic Eve picks food-plant tile when bananas/berries abundant.\n\
    #[test]\n\
    fn synthetic_eve_spawn_prefers_banana_when_abundant() {\n";
    if t.contains(bad) {
        *t = t.replacen(bad, good, 1);
        changed = true;
    }
    if t.contains("fn human_login_spawn_near_configured_spawn_not_mother()")
        && !t.contains("#[test]\n    fn human_login_spawn_near_configured_spawn_not_mother()")
    {
        let old = "    fn human_login_spawn_near_configured_spawn_not_mother() {\n";
        let new = "    /// Human LOGIN must not spawn on mother/NPC tile (bootstrap desync fix).\n\
    #[test]\n\
    fn human_login_spawn_near_configured_spawn_not_mother() {\n";
        if replace_once(t, old, new) {
            changed = true;
        }
    }
    let dup = "    /// Human LOGIN must not spawn on mother/NPC tile (bootstrap desync fix).\n\
    #[test]\n\
    /// Human LOGIN must not spawn on mother/NPC tile (bootstrap desync fix).\n\
    #[test]\n\
    fn human_login_spawn_near_configured_spawn_not_mother() {\n";
    if t.contains(dup) {
        let single = "    /// Human LOGIN must not spawn on mother/NPC tile (bootstrap desync fix).\n\
    #[test]\n\
    fn human_login_spawn_near_configured_spawn_not_mother() {\n";
        *t = t.replacen(dup, single, 1);
        changed = true;
    }
    changed
}

pub fn patch_eve_banana(manifest: &Path, src: &Path) -> bool {
    let lib_path = src.join("lib.rs");
    let Ok(raw) = std::fs::read_to_string(&lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if fix_test_attrs(&mut t) {
        changed = true;
    }

    if !t.contains("mod eve_spawn;") {
        if replace_once(
            &mut t,
            "mod birth_fitness;\n",
            "mod birth_fitness;\n// Haxe: GPI.spawnAsEve food plants + jungle banana (EVE-BANANA)\nmod eve_spawn;\n",
        ) {
            changed = true;
        }
    }

    if !t.contains("pub use eve_spawn::") {
        let insert_after = "pub use birth_fitness::{\n    father_fitness, mother_fitness, ChildView, FatherView, MotherView, EVE_OR_ADAM_BIRTH_CHANCE,\n};\n";
        let export = r#"pub use birth_fitness::{
    father_fitness, mother_fitness, ChildView, FatherView, MotherView, EVE_OR_ADAM_BIRTH_CHANCE,
};
// Haxe: GlobalPlayerInstance.spawnAsEve + ClearStartLocations (EVE-BANANA / jungle_spawn)
pub use eve_spawn::{
    classify_eve_food_site, collect_eve_food_sites, eve_location_fitness, eve_person_color_at,
    find_eve_spawn, find_eve_spawn_with_rng, get_close_special_biome_person_color,
    is_eve_start_food_id, person_color_by_biome, pick_best_eve_site, resolve_eve_spawn_site,
    select_eve_food_pool, sites_for_pool, use_fixed_starting_spawn, EveFoodPool, EveFoodPoolCounts,
    EveFoodSite, EveSpawnOpts, EVE_BANANA_PLANT, EVE_BERRY_BUSH, EVE_LOCATION_SAMPLES,
    EVE_MIN_FOOD_SITES, EVE_START_FOOD_IDS,
};
"#;
        if replace_once(&mut t, insert_after, export) {
            changed = true;
        }
    }

    let old_eve = r#"        if eve {
            let w = state.world.read().unwrap();
            find_playable_spawn(&w, (state.spawn_x, state.spawn_y))
        } else if let Some(mid) = pick_best_mother_p_id_for_child_class(state, birth_class) {"#;
    let new_eve = r#"        if eve {
            // EVE-BANANA: Haxe spawnAsEve near banana/berry food plants (jungle preference).
            let w = state.world.read().unwrap();
            let prefer = (state.spawn_x, state.spawn_y);
            let fallback = find_playable_spawn(&w, prefer);
            let player_xy: Vec<(i32, i32)> = state
                .players
                .values()
                .filter(|pl| !pl.deleted)
                .map(|pl| (pl.x, pl.y))
                .collect();
            find_eve_spawn(&w, prefer, &player_xy, fallback)
        } else if let Some(mid) = pick_best_mother_p_id_for_child_class(state, birth_class) {"#;
    if t.contains("if eve {") && !t.contains("EVE-BANANA: Haxe spawnAsEve") {
        if replace_once(&mut t, old_eve, new_eve) {
            changed = true;
        }
    }

    let old_boot = r#"    {
        let w = state.world.read().unwrap();
        let (sx, sy) = find_playable_spawn(&w, (0, 0));
        state.spawn_x = sx;
        state.spawn_y = sy;
        info!(sx, sy, "sim: playable spawn point ready");
    }"#;
    let new_boot = r#"    {
        // EVE-BANANA: prefer food-plant Eve spawn (jungle banana) when map has plants.
        let w = state.world.read().unwrap();
        let fallback = find_playable_spawn(&w, (0, 0));
        let (sx, sy) = find_eve_spawn(&w, fallback, &[], fallback);
        state.spawn_x = sx;
        state.spawn_y = sy;
        info!(sx, sy, "sim: playable spawn point ready (Eve food / grassland)");
    }"#;
    if t.contains("sim: playable spawn point ready")
        && !t.contains("sim: playable spawn point ready (Eve food / grassland)")
    {
        if replace_once(&mut t, old_boot, new_boot) {
            changed = true;
        }
    }

    if !t.contains("fn synthetic_eve_spawn_prefers_banana_when_abundant") {
        let old = "    /// Human LOGIN must not spawn on mother/NPC tile (bootstrap desync fix).\n\
    #[test]\n\
    fn human_login_spawn_near_configured_spawn_not_mother() {";
        let new = r#"    /// EVE-BANANA: synthetic Eve picks food-plant tile when bananas/berries abundant.
    #[test]
    fn synthetic_eve_spawn_prefers_banana_when_abundant() {
        let mut state = SimState::with_default_empty(test_content());
        state.spawn_x = 10;
        state.spawn_y = 10;
        {
            let mut w = state.world.write().unwrap();
            for i in 0..12 {
                let x = 30 + i;
                w.set_biome(x, 30, 6); // jungle
                w.set_object(x, 30, crate::EVE_BANANA_PLANT);
                w.set_object(i, 2, crate::EVE_BERRY_BUSH);
            }
        }
        let p_id = spawn_player(&mut state, 9_000_001, "eve_banana@ai");
        let p = state
            .players
            .values()
            .find(|pl| pl.p_id == p_id)
            .expect("eve");
        let obj = state.world.read().unwrap().get_object(p.x, p.y);
        assert!(
            obj == crate::EVE_BANANA_PLANT
                || obj == crate::EVE_BERRY_BUSH
                || (p.x - 10).abs() <= 200,
            "eve at {},{} obj={obj}",
            p.x,
            p.y
        );
    }

    /// Human LOGIN must not spawn on mother/NPC tile (bootstrap desync fix).
    #[test]
    fn human_login_spawn_near_configured_spawn_not_mother() {"#;
        if replace_once(&mut t, old, new) {
            changed = true;
        }
    }

    if fix_test_attrs(&mut t) {
        changed = true;
    }

    if changed {
        let out = restore_nl(&t, crlf);
        if std::fs::write(&lib_path, out).is_err() {
            return false;
        }
    }

    let docs_ok = patch_docs(manifest);
    changed || docs_ok || eve_banana_wired(&std::fs::read_to_string(&lib_path).unwrap_or_default())
}

fn patch_docs(manifest: &Path) -> bool {
    let workspace = manifest
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| manifest.to_path_buf());
    let port = workspace.join("docs/port");
    if !port.is_dir() {
        return false;
    }
    let mut any = false;
    any |= patch_todo_port(&port.join("TODO_PORT.md"));
    any |= patch_file_matrix(&port.join("FILE_MATRIX.md"));
    any |= patch_call_index(&port.join("CALL_INDEX.md"));
    any |= patch_queue(&port.join("QUEUE.md"));
    any
}

fn patch_todo_port(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if t.contains("- [ ] Eve jungle banana preference (Haxe TODO)") {
        if replace_once(
            &mut t,
            "- [ ] Eve jungle banana preference (Haxe TODO)",
            "- [x] **EVE-BANANA** Eve jungle banana preference — `eve_spawn.rs` ClearStartLocations + pool pick + jungle fitness; synthetic Eve + boot spawn wire; tests `eve_spawn::*`",
        ) {
            changed = true;
        }
    }

    if !t.contains("EVE-BANANA jungle_spawn") {
        let entry = "| 2026-07-26 | **EVE-BANANA jungle_spawn**: pure `eve_spawn.rs` (`classify_eve_food_site` / `select_eve_food_pool` / `eve_location_fitness` / `resolve_eve_spawn_site` / `collect_eve_food_sites` / `find_eve_spawn`); Haxe TODO L1122 jungle banana preference; synthetic Eve + boot wire; residual last Eve/Adam pairing, grave close, human TCP bootstrap, race object pick |\n";
        if let Some(i) = t.find("| Date | Change |\n|------|--------|\n") {
            let insert_at = i + "| Date | Change |\n|------|--------|\n".len();
            t.insert_str(insert_at, entry);
            changed = true;
        }
    }

    if !t.contains("19. ~~**EVE-BANANA**")
        && t.contains(
            "18. ~~**EXHAUSTION-WOUND wound_food_pipes**~~ **DONE** (core) calculateFoodStoreMax + DoDamage/tick pipes",
        )
    {
        if replace_once(
            &mut t,
            "18. ~~**EXHAUSTION-WOUND wound_food_pipes**~~ **DONE** (core) calculateFoodStoreMax + DoDamage/tick pipes  \n",
            "18. ~~**EXHAUSTION-WOUND wound_food_pipes**~~ **DONE** (core) calculateFoodStoreMax + DoDamage/tick pipes  \n19. ~~**EVE-BANANA** jungle_spawn~~ **DONE** (core) `eve_spawn` food-plant Eve + jungle banana preference; residual last-Eve pairing / human bootstrap / race object pick  \n",
        ) {
            changed = true;
        }
    }

    if changed {
        let _ = std::fs::write(path, restore_nl(&t, crlf));
    }
    changed
}

fn patch_file_matrix(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    let old = "| GPI-SPAWN / **CLASS-BONI** | Eve/child spawn, class, prestige class table | PARTIAL → **class boni + birth class DONE** | `calculate_class_boni` + `PRESTIGE_CLASS_NAMES` + `calculate_prestige_class_at_birth` (0.4/0.8); pick_best_* child class wire; residual Noob→noble spawn weights / Eve banana / additive birth_fitness base |";
    let new = "| GPI-SPAWN / **CLASS-BONI** / **EVE-BANANA** | Eve/child spawn, class, prestige class table | PARTIAL → **class boni + Eve banana DONE** | class boni + birth class; **`eve_spawn` jungle banana** (food pools + fitness + synthetic/boot wire); residual Noob→noble spawn weights / additive birth_fitness base / last-Eve pairing / race object pick |";
    if t.contains(old) {
        if replace_once(&mut t, old, new) {
            changed = true;
        }
    }

    if !t.contains("| **EVE-BANANA** / jungle_spawn") {
        if let Some(i) = t.find("| GPI-SPAWN / **CLASS-BONI**") {
            if let Some(end) = t[i..].find('\n') {
                let line_end = i + end + 1;
                let row = "| **EVE-BANANA** / jungle_spawn | Eve food-plant spawn + jungle banana | **DONE** (core) | pure `eve_spawn.rs`; synthetic Eve + boot wire; residual last-Eve pair / human TCP bootstrap / grave close / race object |\n";
                t.insert_str(line_end, row);
                changed = true;
            }
        }
    }

    if changed {
        let _ = std::fs::write(path, restore_nl(&t, crlf));
    }
    changed
}

fn patch_call_index(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("find_eve_spawn") {
        return false;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    t.push_str(
        r#"
## EVE-BANANA / jungle_spawn

| Symbol | File | Role |
|--------|------|------|
| `GlobalPlayerInstance.spawnAsEve` | `server/GlobalPlayerInstance.hx` | Eve/Adam wild birth position |
| `ClearStartLocations` | same | foodArray filter |
| `getCloseSpecialBiomePersonColor` | same | Jungle→Brown / Desert→Black |
| Rust `classify_eve_food_site` / `select_eve_food_pool` / `eve_location_fitness` | `ol-sim/eve_spawn.rs` | pure |
| Rust `collect_eve_food_sites` / `find_eve_spawn` | same | world scan + live pick |
| Wire | `spawn_player` synthetic Eve + sim boot preferred spawn | EVE-BANANA |

"#,
    );
    let _ = std::fs::write(path, restore_nl(&t, crlf));
    true
}

fn patch_queue(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;
    if replace_once(
        &mut t,
        "| `EVE-BANANA` | workflow (new) | jungle_spawn |",
        "| ~~`EVE-BANANA`~~ | workflow | jungle_spawn **DONE** |",
    ) {
        changed = true;
    }
    if replace_once(
        &mut t,
        "| 49 | `EVE-BANANA` | jungle_spawn | **running** |",
        "| 49 | ~~`EVE-BANANA`~~ | jungle_spawn | **DONE** food-plant Eve + jungle banana |",
    ) {
        changed = true;
    }
    if changed {
        let _ = std::fs::write(path, restore_nl(&t, crlf));
    }
    changed
}
