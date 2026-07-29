//! Build-time wire for GPI-DEATH-POLISH (`grave_soul_leader`).
//! Also stamps PLACE-OBJECT free_tile_search lib re-exports + port docs (piggyback).

use std::path::Path;
use std::process::Command;

pub fn death_polish_wired(text: &str) -> bool {
    text.contains("death_polish::apply_death_polish")
        && text.contains("mod death_polish;")
        && text.contains("food_death_wire(wb, nursing)")
        && text.contains("place_grave_with_soul")
}

/// Prefer Python script (mirrors repo scripts/); fall back to pure Rust replaces.
pub fn patch_lib_gpi_death_polish(lib_path: &Path, workspace: &Path) -> bool {
    if let Ok(text) = std::fs::read_to_string(lib_path) {
        if death_polish_wired(&text) {
            // Source already wired — still ensure PLACE-OBJECT free_tile exports + docs.
            let _ = patch_place_object_exports_and_docs(lib_path);
            return true;
        }
    }
    let script = workspace.join("scripts/patch_gpi_death_polish.py");
    if script.exists() {
        let py = Command::new("python")
            .arg(&script)
            .status()
            .or_else(|_| Command::new("python3").arg(&script).status())
            .or_else(|_| Command::new("py").arg("-3").arg(&script).status());
        if let Ok(s) = py {
            if s.success() {
                if let Ok(text) = std::fs::read_to_string(lib_path) {
                    if death_polish_wired(&text) {
                        let _ = patch_place_object_exports_and_docs(lib_path);
                        return true;
                    }
                }
            }
        }
    }
    let ok = patch_lib_rust_only(lib_path);
    if ok {
        let _ = patch_place_object_exports_and_docs(lib_path);
    }
    ok
}

/// PLACE-OBJECT free_tile_search: expand death_polish re-exports + stamp port docs.
fn patch_place_object_exports_and_docs(lib_path: &Path) -> bool {
    let mut ok = true;
    if let Ok(mut text) = std::fs::read_to_string(lib_path) {
        if !text.contains("place_search_distance_step") {
            if text.contains("MURDER_GRAVE_ID,\n};") {
                text = text.replacen(
                    "MURDER_GRAVE_ID,\n};",
                    "MURDER_GRAVE_ID,\n    // PLACE-OBJECT free_tile_search\n    can_be_placed_in_grave, is_grave_object, is_permanent_object, is_tree_description, is_tree_object,\n    place_complex_object, place_object, place_object_by_id, place_object_near, place_object_with_rng,\n    place_random_offset, place_search_candidate, place_search_distance_step, transform_placed_object_id,\n    try_place_flat_on_world, try_place_kind, PlaceObjectOpts, PlaceObjectResult, TryPlaceKind,\n    HORSE_DRAWN_CART_ID, HORSE_DRAWN_TIRE_CART_ID, PLACE_DROP_WALLS_AFTER, PLACE_MAX_ATTEMPTS,\n};",
                    1,
                );
                ok &= std::fs::write(lib_path, &text).is_ok();
            }
        }
    }

    // docs/port relative to ol-sim crate: ../../docs/port
    let Some(src) = lib_path.parent() else {
        return ok;
    };
    let Some(crate_root) = src.parent() else {
        return ok;
    };
    let Some(crates_dir) = crate_root.parent() else {
        return ok;
    };
    let Some(rust_server) = crates_dir.parent() else {
        return ok;
    };
    let docs = rust_server.join("docs").join("port");

    // TODO_PORT
    let todo = docs.join("TODO_PORT.md");
    if let Ok(mut t) = std::fs::read_to_string(&todo) {
        let old = "  - Residual: full WorldMap.PlaceObject wall/biome search; baby bones in arms Haxe TODO; ScoreEntry dead-relative → **SCORE-ENTRY DONE**";
        let new = "  - Residual: baby bones in arms Haxe TODO; ScoreEntry dead-relative → **SCORE-ENTRY DONE**\n- [x] **PLACE-OBJECT free_tile_search** — `place_object.rs` PlaceObject/TryPlaceObject (expanding random free tile, biome block, behind-tree, grave swallow, allowReplace, considerWalls via CalculateNonBlockedTarget); death_polish grave/held wired; residual: containSize/slotSize, unify remaining TH/TimeHelper callers, baby-hold lite ring";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            t = t.replacen(
                "Last updated: **2026-07-26** (NOOB-NOBLE-SPAWN spawn_weights)",
                "Last updated: **2026-07-26** (PLACE-OBJECT free_tile_search)",
                1,
            );
            if !t.contains("| 2026-07-26 | **PLACE-OBJECT free_tile_search**") {
                if let Some(idx) = t.find("| 2026-07-26 | **WORLD-FOOD-FACTOR") {
                    t.insert_str(
                        idx,
                        "| 2026-07-26 | **PLACE-OBJECT free_tile_search**: pure+live `place_object.rs` (Haxe PlaceObject/TryPlaceObject distance growth, biome block, behind-tree, grave swallow, allowReplace, considerWalls); death_polish place_grave_on_map + non-containable held; tests place_object::*; residual containSize + unify TH/TimeHelper callers |\n",
                    );
                } else if let Some(idx) = t.find("| 2026-07-26 |") {
                    t.insert_str(
                        idx,
                        "| 2026-07-26 | **PLACE-OBJECT free_tile_search**: pure+live `place_object.rs` (Haxe PlaceObject free tile + walls/biome); death_polish wired; tests place_object::* |\n",
                    );
                }
            }
            ok &= std::fs::write(&todo, t).is_ok();
        }
    }

    // FILE_MATRIX
    let matrix = docs.join("FILE_MATRIX.md");
    if let Ok(mut t) = std::fs::read_to_string(&matrix) {
        let old = "residual: full PlaceObject wall/biome search, baby-bones-in-arms TODO";
        let new = "**PlaceObject free_tile via place_object.rs**; residual: baby-bones-in-arms TODO";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
        }
        if !t.contains("**PLACE-OBJECT** / free_tile_search") {
            if let Some(idx) = t.find("| **GPI-PLACE-GRAVE**") {
                if let Some(eol) = t[idx..].find('\n') {
                    let insert_at = idx + eol + 1;
                    t.insert_str(
                        insert_at,
                        "| **PLACE-OBJECT** / free_tile_search | WorldMap.PlaceObject expanding free tile + walls/biome | **DONE** (core) | `place_object.rs` pure TryPlaceKind + live free search; TransformObject carts; death_polish wired; residual containSize/slotSize; unify remaining PlaceObject call sites |\n",
                    );
                }
            }
        }
        t = t.replacen(
            "Last reviewed: **2026-07-26** (NOOB-NOBLE-SPAWN spawn_weights)",
            "Last reviewed: **2026-07-26** (PLACE-OBJECT free_tile_search)",
            1,
        );
        ok &= std::fs::write(&matrix, t).is_ok();
    }

    // CALL_INDEX
    let call = docs.join("CALL_INDEX.md");
    if let Ok(mut t) = std::fs::read_to_string(&call) {
        if !t.contains("WorldMap.PlaceObject") {
            let anchor = "| `WorldMap.isBiomeBlocking` | same | movement gate |\n";
            let insert = "| `WorldMap.isBiomeBlocking` | same | movement gate |\n| `WorldMap.PlaceObject` / `TryPlaceObject` / `TransformObject` | same | free-tile place + grave swallow → `ol-sim/place_object.rs` |\n| Rust `place_object` / `try_place_kind` / `place_search_distance_step` | `ol-sim/place_object.rs` | PLACE-OBJECT free_tile_search |\n";
            if t.contains(anchor) {
                t = t.replacen(anchor, insert, 1);
                ok &= std::fs::write(&call, t).is_ok();
            }
        }
    }

    ok
}

fn patch_lib_rust_only(lib_path: &Path) -> bool {
    let Ok(mut text) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    if death_polish_wired(&text) {
        return true;
    }

    if !text.contains("mod death_polish;") {
        text = text.replacen(
            "mod death_inherit;\n",
            "mod death_inherit;\nmod death_polish;\n",
            1,
        );
    }

    let old_exp = "pub use death_cause::{\n    combat_death, combat_death_wire, format_cause_query, format_death_event,\n    format_death_event_tag, hunger_death_wire, killed_by_object_wire,\n    parse_killed_object_id, DeathCause,\n};\npub use death_inherit::{apply_inherit_coins, format_inherit_events, InheritContext, InheritTransfer};";
    let new_exp = "pub use death_cause::{\n    combat_death, combat_death_wire, food_death_wire, format_cause_query, format_death_event,\n    format_death_event_tag, hunger_death_wire, killed_by_object_wire,\n    parse_killed_object_id, DeathCause,\n};\npub use death_inherit::{\n    account_soul_token, add_owner_to_helper, apply_inherit_coins, apply_inherit_ownership_on_helpers,\n    choose_new_leader, count_leadership_power, format_inherit_events, format_leader_succession_event,\n    format_ownership_events, remove_owner_from_helper, stamp_grave_soul, InheritContext,\n    InheritTransfer, LeaderSuccession, OwnershipTransfer,\n};\npub use death_polish::{apply_death_polish, place_grave_with_soul};";
    if text.contains(old_exp) {
        text = text.replacen(old_exp, new_exp, 1);
    } else if !text.contains("food_death_wire") {
        text = text.replacen(
            "format_death_event_tag, hunger_death_wire, killed_by_object_wire,",
            "food_death_wire, format_death_event_tag, hunger_death_wire, killed_by_object_wire,",
            1,
        );
        if !text.contains("pub use death_polish::") {
            text = text.replacen(
                "pub use death_inherit::{apply_inherit_coins, format_inherit_events, InheritContext, InheritTransfer};\npub use economy::INHERIT_COINS_FACTOR;",
                "pub use death_inherit::{
    account_soul_token, add_owner_to_helper, apply_inherit_coins, apply_inherit_ownership_on_helpers,
    choose_new_leader, count_leadership_power, format_inherit_events, format_leader_succession_event,
    format_ownership_events, remove_owner_from_helper, stamp_grave_soul, InheritContext,
    InheritTransfer, LeaderSuccession, OwnershipTransfer,
};
pub use death_polish::{apply_death_polish, place_grave_with_soul};
pub use economy::INHERIT_COINS_FACTOR;",
                1,
            );
        }
    }

    let new_fn = "/// GPI-DEATH-POLISH grave_soul_leader — Haxe doDeathHelper polish:\n/// ChooseNewLeader + InheritOwnership + InheritCoins (grave residual + soul).\nfn apply_death_inheritance(state: &mut SimState, deceased_p_id: i32) {\n    death_polish::apply_death_polish(state, deceased_p_id);\n}\n";
    if !text.contains("death_polish::apply_death_polish") {
        let start_markers = [
            "/// GPI-DEATH-POLISH grave_soul_leader",
            "/// Haxe `GlobalPlayerInstance.InheritCoins`",
        ];
        let end = "\n}\n\n/// Max Chebyshev ring radius when scattering loot on death / DROPALL.";
        for sm in start_markers {
            if let Some(start) = text.find(sm) {
                if let Some(end_rel) = text[start..].find(end) {
                    let end_abs = start + end_rel;
                    text = format!("{}{}{}", &text[..start], new_fn, &text[end_abs + 1..]);
                    break;
                }
            }
        }
    }

    let old_hunger = "            // Haxe TimeHelper: woundedBy != 0 → reason_killed_${woundedBy}\n            let wb = wounded_by_pid.get(&p.p_id).copied().unwrap_or(0);\n            p.death_reason = Some(hunger_death_wire(wb));";
    let new_hunger = "            // Haxe TimeHelper: woundedBy != 0 → reason_killed_${woundedBy}\n            // GPI-DEATH-POLISH: holding baby → reason_nursing_hunger.\n            let wb = wounded_by_pid.get(&p.p_id).copied().unwrap_or(0);\n            let nursing = p.holding_player_id != 0;\n            p.death_reason = Some(food_death_wire(wb, nursing));";
    if text.contains(old_hunger) {
        text = text.replacen(old_hunger, new_hunger, 1);
    }

    let old_grave = "        // Hunger/age: place content-resolved grave when non-zero.\n        if cause.is_natural() {\n            let gid = state.grave_object_id;\n            if gid != 0 {\n                if let Some((x, y)) = death_xy {\n                    state.world.write().unwrap().set_object(x, y, gid);\n                    state.record_world_change(x, y, gid);\n                    state.specials.insert(x, y, SpecialKind::Grave);\n                }\n            }\n        }";
    let new_grave = "        // Hunger/age: place content-resolved grave when non-zero (GPI-DEATH-POLISH soul).\n        if cause.is_natural() {\n            let gid = state.grave_object_id;\n            if gid != 0 {\n                if let Some((x, y)) = death_xy {\n                    place_grave_with_soul(state, x, y, gid, p_id, &email);\n                }\n            }\n        }";
    if text.contains(old_grave) {
        text = text.replacen(old_grave, new_grave, 1);
    }

    // Integration tests already source-wired in lib.rs; do not re-inject.

    if std::fs::write(lib_path, &text).is_err() {
        return false;
    }
    death_polish_wired(&std::fs::read_to_string(lib_path).unwrap_or_default())
}
