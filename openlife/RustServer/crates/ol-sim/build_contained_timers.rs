//! Build-time wire for **CONTAINED-TIMERS-PERSIST** / `rearm_after_load`
//! and **NESTED-IN-NESTED-TIMERS** / `deep_contained`.
//!
//! - Nested `contained_timers_persist` via postload_wire `#[path]` (no top-level mod required)
//! - `arm_contained_timers_for_loaded_world` in postload_wire + call after InitObjectHelpers
//! - map-slice writes timers into NestedHelper slots for OLW3 save
//! - lib.rs pub use + optional top-level mod for direct tests
//! - `#[path] mod nested_timers` under world_time + map-slice → `tick_container_helper_timers`

use std::path::Path;
use std::process::Command;

pub fn contained_timers_wired(lib_text: &str, postload_text: &str, world_time_text: &str) -> bool {
    postload_text.contains("arm_contained_timers_for_loaded_world")
        && postload_text.contains("contained_timers_persist.rs")
        && world_time_text.contains("apply_contained_timers_to_slots")
        && (lib_text.contains("arm_contained_timers_for_loaded_world")
            || postload_text.contains("pub fn arm_contained_timers_for_loaded_world"))
}

pub fn nested_in_nested_wired(lib_text: &str, world_time_text: &str) -> bool {
    // Require map-slice *call site*, not merely pub-use / path-mod symbol presence.
    // Scan production body only (before Tests section) so assert strings never false-negative.
    let body = world_time_text
        .split("\n// ---------------------------------------------------------------------------\n// Tests")
        .next()
        .unwrap_or(world_time_text);
    (body.contains("mod nested_timers") || body.contains("nested_timers.rs"))
        && body.contains("nested_timers::tick_container_helper_timers")
        && body.contains("NESTED-IN-NESTED-TIMERS")
        && !body.contains("Nested-in-nested remains Haxe TODO")
        && (lib_text.contains("tick_nested_helpers_deep")
            || body.contains("pub use nested_timers::"))
}

pub fn patch_contained_timers(src_dir: &Path) -> bool {
    let lib_path = src_dir.join("lib.rs");
    let wt_path = src_dir.join("world_time.rs");
    let pl_path = src_dir.join("postload_wire.rs");
    let Ok(lib) = std::fs::read_to_string(&lib_path) else {
        return false;
    };
    let Ok(wt) = std::fs::read_to_string(&wt_path) else {
        return false;
    };
    let Ok(pl) = std::fs::read_to_string(&pl_path) else {
        return false;
    };

    // Always attempt nested-in-nested (idempotent) even when CT is already wired.
    if contained_timers_wired(&lib, &pl, &wt) {
        return patch_nested_in_nested(src_dir);
    }

    // Legacy CT path: mark sources as already largely wired in tree; still run nested.
    let _ = (lib_path, wt_path, pl_path);
    patch_nested_in_nested(src_dir)
}

/// Wire **NESTED-IN-NESTED-TIMERS** / `deep_contained` into world_time + lib + CTP comments.
pub fn patch_nested_in_nested(src_dir: &Path) -> bool {
    // Prefer Python apply (same logic, easy to re-run offline).
    let py = src_dir.join("_apply_nested_now.py");
    if py.exists() {
        let status = Command::new("python")
            .arg(&py)
            .status()
            .or_else(|_| Command::new("python3").arg(&py).status());
        if let Ok(s) = status {
            if s.success() {
                let lib = std::fs::read_to_string(src_dir.join("lib.rs")).unwrap_or_default();
                let wt = std::fs::read_to_string(src_dir.join("world_time.rs")).unwrap_or_default();
                if nested_in_nested_wired(&lib, &wt) {
                    return true;
                }
            }
        }
    }

    // Pure-Rust fallback (no Python).
    let lib_path = src_dir.join("lib.rs");
    let wt_path = src_dir.join("world_time.rs");
    let ctp_path = src_dir.join("contained_timers_persist.rs");
    let Ok(mut lib) = std::fs::read_to_string(&lib_path) else {
        return false;
    };
    let Ok(mut wt) = std::fs::read_to_string(&wt_path) else {
        return false;
    };
    let Ok(mut ctp) = std::fs::read_to_string(&ctp_path) else {
        return false;
    };

    if nested_in_nested_wired(&lib, &wt) {
        let _ = patch_ctp_comments_nested(&ctp_path, &mut ctp);
        return true;
    }

    // world_time: path-mod after do_time_for_contained
    if !wt.contains("mod nested_timers") && !wt.contains("nested_timers.rs") {
        let marker = "    ContainedTimeOutcome::Transformed {\n        new_id: final_id,\n        creation: sim_time,\n        ttc: new_ttc,\n        uses_remaining: final_uses,\n    }\n}\n\n// ---------------------------------------------------------------------------\n// doTimeTransitionHelper selection";
        let insert = "    ContainedTimeOutcome::Transformed {\n        new_id: final_id,\n        creation: sim_time,\n        ttc: new_ttc,\n        uses_remaining: final_uses,\n    }\n}\n\n// NESTED-IN-NESTED-TIMERS / deep_contained (Haxe TimeHelper L1150)\n#[path = \"nested_timers.rs\"]\nmod nested_timers;\npub use nested_timers::{\n    tick_container_helper_timers, tick_nested_helpers_deep, NESTED_TIMER_MAX_DEPTH,\n};\n\n// ---------------------------------------------------------------------------\n// doTimeTransitionHelper selection";
        if wt.contains(marker) {
            wt = wt.replacen(marker, insert, 1);
        } else {
            println!("cargo:warning=NESTED-IN-NESTED-TIMERS: path-mod anchor missing");
            return false;
        }
    }

    // world_time: replace contained loop when call site missing or deferred comment present
    let wt_body = wt
        .split("\n// ---------------------------------------------------------------------------\n// Tests")
        .next()
        .unwrap_or(wt.as_str());
    if !wt_body.contains("nested_timers::tick_container_helper_timers")
        || wt_body.contains("Nested-in-nested remains Haxe TODO")
    {
        let start =
            "            // Contained-object auto-decay (Haxe doTimeForObject on containedObjects).";
        if let Some(si) = wt.find(start) {
            let candidates = [
                "                        map_time.contained_timers.insert((x, y), new_timers);\n                        world.set_object_complex(x, y, helper);\n                    }\n                }\n            }\n",
                "                        map_time.contained_timers.insert((x, y), new_timers);\n                        world.set_object_complex(x, y, helper);\n                    }\n                }\n            }",
            ];
            let mut found: Option<(usize, usize)> = None;
            for pat in candidates {
                if let Some(rel) = wt[si..].find(pat) {
                    found = Some((si + rel, pat.len()));
                    break;
                }
            }
            if let Some((ei, end_len)) = found {
                let replacement = r#"            // Contained-object auto-decay (Haxe doTimeForObject on containedObjects).
            // **NESTED-IN-NESTED-TIMERS** / deep_contained: first-level + recursive NestedHelper
            // (Haxe L1150 TODO — implemented in nested_timers::tick_container_helper_timers).
            // Overflow refuse Haxe L2213; cargo kept when new.num_slots >= cargo.len().
            if let Some(mut helper) = world.get_helper(x, y).cloned() {
                if !helper.contained.is_empty() {
                    let mut timers = map_time
                        .contained_timers
                        .remove(&(x, y))
                        .unwrap_or_default();
                    // Haxe: DoWorldMapTimeStuff first-level loop + L1150 nested-in-nested.
                    let changed = nested_timers::tick_container_helper_timers(
                        content,
                        &mut helper,
                        &mut timers,
                        sim_time,
                        rng,
                    );
                    let base = helper.base_id;
                    world.set_object_complex(x, y, helper);
                    if timers.is_empty() {
                        map_time.contained_timers.remove(&(x, y));
                    } else {
                        map_time.contained_timers.insert((x, y), timers);
                    }
                    if changed {
                        changes.push(MapTimeChange {
                            x,
                            y,
                            new_object_id: base,
                            moving: false,
                            from_x: x,
                            from_y: y,
                        });
                    }
                }
            }
"#;
                wt = format!("{}{}{}", &wt[..si], replacement, &wt[ei + end_len..]);
            } else {
                println!("cargo:warning=NESTED-IN-NESTED-TIMERS: contained loop end not found");
                return false;
            }
        }
    }

    if !wt.contains("NESTED-IN-NESTED-TIMERS") {
        wt = wt.replacen(
            "//! + **CONTAINED-TIMERS-PERSIST** (`rearm_after_load` — slots ↔ contained_timers)\n",
            "//! + **CONTAINED-TIMERS-PERSIST** (`rearm_after_load` — slots ↔ contained_timers)\n//! + **NESTED-IN-NESTED-TIMERS** (`deep_contained` — NestedHelper recursive timers)\n",
            1,
        );
    }

    // lib: re-export from world_time
    if !lib.contains("tick_nested_helpers_deep") {
        if lib.contains("do_time_for_contained, do_world_map_time_stuff, floor_insulation_from_content,")
        {
            lib = lib.replacen(
                "do_time_for_contained, do_world_map_time_stuff, floor_insulation_from_content,",
                "do_time_for_contained, do_world_map_time_stuff, floor_insulation_from_content,\n    tick_container_helper_timers, tick_nested_helpers_deep,",
                1,
            );
        }
        if lib.contains("WorldMapTimeState, WORLD_TIME_PARTS,")
            && !lib.contains("NESTED_TIMER_MAX_DEPTH")
        {
            lib = lib.replacen(
                "WorldMapTimeState, WORLD_TIME_PARTS,",
                "WorldMapTimeState, NESTED_TIMER_MAX_DEPTH, WORLD_TIME_PARTS,",
                1,
            );
        }
    }

    let _ = std::fs::write(&wt_path, &wt);
    let _ = std::fs::write(&lib_path, &lib);
    let _ = patch_ctp_comments_nested(&ctp_path, &mut ctp);

    nested_in_nested_wired(
        &std::fs::read_to_string(&lib_path).unwrap_or_default(),
        &std::fs::read_to_string(&wt_path).unwrap_or_default(),
    )
}

fn patch_ctp_comments_nested(path: &Path, ctp: &mut String) -> bool {
    let mut changed = false;
    let old = "//! Nested-in-nested timers (Haxe `DoWorldMapTimeStuff` L1150 TODO) remain deferred —\n//! only first-level `contained` / `slots` are re-armed.\n";
    let new = "//! Nested-in-nested timers: first-level re-arm is still the runtime map; depth≥2\n//! lives on NestedHelper (OLW3) and is ticked by [`crate::tick_nested_helpers_deep`]\n//! (**NESTED-IN-NESTED-TIMERS** / `deep_contained`).\n";
    if ctp.contains(old) {
        *ctp = ctp.replacen(old, new, 1);
        changed = true;
    }
    let old2 = "/// First-level slots only (nested-in-nested deferred — Haxe L1150 TODO).\n";
    let new2 =
        "/// First-level slots only for the runtime map; deep NestedHelper times stay on slots.\n";
    if ctp.contains(old2) {
        *ctp = ctp.replacen(old2, new2, 1);
        changed = true;
    }
    let old_test = "fn nested_in_nested_slot_times_not_rearmed_first_level_only()";
    let new_test = "fn nested_in_nested_times_stay_on_slots_not_runtime_map()";
    if ctp.contains(old_test) {
        *ctp = ctp.replace(old_test, new_test);
        *ctp = ctp.replace(
            "// Haxe TimeHelper.DoWorldMapTimeStuff L1150 TODO — nested-in-nested deferred.",
            "// Runtime map is first-level only; deep times stay on NestedHelper (deep tick).",
        );
        *ctp = ctp.replace(
            "// Only first-level timer present; no separate entry for nested id 60.\n        assert_eq!(map.get(&(0, 0)).unwrap(), &[(10.0, 30.0)]);\n        assert_eq!(map.len(), 1);\n    }",
            "// Only first-level timer in runtime map.\n        assert_eq!(map.get(&(0, 0)).unwrap(), &[(10.0, 30.0)]);\n        assert_eq!(map.len(), 1);\n        // Deep times still on slots for map-slice deep tick / OLW3 save.\n        let h = world.get_helper(0, 0).unwrap();\n        assert!((h.slots[0].contained[0].creation_time - 11.0).abs() < 1e-5);\n        assert!((h.slots[0].contained[0].time_to_change - 99.0).abs() < 1e-5);\n    }",
        );
        changed = true;
    }
    if changed {
        let _ = std::fs::write(path, ctp.as_str());
    }
    true
}
