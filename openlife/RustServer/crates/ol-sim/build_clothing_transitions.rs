//! Build-time wire for **TH-CLOTHING-MATRIX** / `clothing_transitions`.
//!
//! Module lives under `clothing_cmds` via `#[path]` (always compiled).
//! This patch:
//! - re-exports matrix helpers from `lib.rs`
//! - DROP clothingIndex → place into worn clothing
//! - SELF / SREMV raw tag handlers
//! - use_transition clothing reset uses `ObjectDef::is_clothing`

use std::path::Path;

pub fn clothing_transitions_wired(lib_text: &str) -> bool {
    (lib_text.contains("pub use clothing_cmds::")
        && lib_text.contains("apply_self_clothing")
        || lib_text.contains("pub use clothing_transitions::")
        || lib_text.contains("clothing_cmds::apply_self_clothing"))
        && lib_text.contains("DROP into clothing container")
        && lib_text.contains("eq_ignore_ascii_case(\"SELF\")")
        && lib_text.contains("eq_ignore_ascii_case(\"SREMV\")")
}

pub fn patch_clothing_transitions(src_dir: &Path) -> bool {
    let lib_path = src_dir.join("lib.rs");
    let use_path = src_dir.join("use_transition.rs");
    let Ok(mut lib) = std::fs::read_to_string(&lib_path) else {
        return false;
    };
    let mut changed = false;

    // pub use clothing matrix helpers (module is under clothing_cmds)
    if !lib.contains("apply_self_clothing")
        || (!lib.contains("pub use clothing_cmds::{")
            && !lib.contains("pub use clothing_transitions::{")
            && !lib.contains("clothing_cmds::apply_self_clothing"))
    {
        if !lib.contains("apply_self_clothing") {
            let export = r#"// TH-CLOTHING-MATRIX re-exports (module: clothing_cmds::clothing_transitions)
pub use clothing_cmds::{
    allow_reset_uses_on_target, apply_place_obj_in_clothing, apply_self_clothing,
    apply_sremv_from_clothing, apply_switch_cloths, apply_transition_on_clothing,
    can_put_into_clothing, clothing_slot_from_def, crown_say_line, format_clothing_set,
    get_clothing_slot_index, is_clothing_string, other_player_accepts_cloth,
    put_into_clothing_nest, resolve_switch_slot, switch_clothing_index_full,
    take_from_clothing_nest, try_transition_on_clothing_pure,
    try_transition_on_clothing_with_content, ClothingSlotIds, ClothingTransitionIn,
    ClothingTransitionOut, SelfClothingPath, CLOTHING_INDEX_LABELS, MAX_AGE_CLOTH_OTHERS,
};
"#;
            if lib.contains("pub use nested_body::{") {
                lib = lib.replacen(
                    "pub use nested_body::{",
                    &format!("{export}pub use nested_body::{{"),
                    1,
                );
                changed = true;
            }
        }
    }

    if !lib.contains("DROP into clothing container") {
        let marker = "    let held = state.players.get(&conn_id).map(|p| p.held_id).unwrap_or(0);\n    if held == 0 {\n        return;\n    }\n    // Floor-only: skip ground place (do not put roads/floors on object layer).";
        let insert = r#"    let held = state.players.get(&conn_id).map(|p| p.held_id).unwrap_or(0);
    if held == 0 {
        return;
    }
    // Haxe TransitionHelper.drop: clothingIndex >= 0 → doPlaceObjInClothing(isDrop=true)
    // // Haxe: TH-CLOTHING-MATRIX
    if let Some(ci) = c {
        if ci >= 0 {
            let content = state.content.clone();
            let ok = state
                .players
                .get_mut(&conn_id)
                .map(|p| clothing_cmds::apply_place_obj_in_clothing(p, &content, ci, true).is_ok())
                .unwrap_or(false);
            if ok {
                state.publish_player_view(conn_id);
                send_player_update_and_frame(state, outbound, conn_id);
                info!(conn_id, clothing_index = ci, "sim: DROP into clothing container");
                return;
            }
            send_player_update_and_frame(state, outbound, conn_id);
            return;
        }
    }
    // Floor-only: skip ground place (do not put roads/floors on object layer)."#;
        if lib.contains(marker) {
            lib = lib.replacen(marker, insert, 1);
            changed = true;
        }
    }

    if !lib.contains("eq_ignore_ascii_case(\"SELF\")") {
        let marker = "            } else {\n                touch_afk_activity(state, conn_id);\n                debug!(conn_id, %tag, %payload, \"sim: raw intent\");\n            }";
        let insert = r#"            } else if tag.eq_ignore_ascii_case("SELF") {
                // Haxe GlobalPlayerInstance.self / doSelf clothing branch (TH-CLOTHING-MATRIX)
                touch_afk_activity(state, conn_id);
                // SELF x y i# — i = clothing slot (-1 auto)
                let mut parts = payload.split_whitespace();
                let _sx = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                let _sy = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                let clothing_slot = parts
                    .next()
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(-1);
                let content = state.content.clone();
                let mut say_line: Option<String> = None;
                let applied = if let Some(p) = state.players.get_mut(&conn_id) {
                    if p.deleted {
                        false
                    } else {
                        match clothing_cmds::apply_self_clothing(p, &content, clothing_slot) {
                            Ok((_path, say)) => {
                                if let Some(s) = say {
                                    say_line = Some(s.to_string());
                                }
                                true
                            }
                            Err(_) => false,
                        }
                    }
                } else {
                    false
                };
                if applied {
                    state.publish_player_view(conn_id);
                    if let Some(msg) = say_line {
                        send_ps_reply(outbound, conn_id, &msg);
                    }
                }
                send_player_update_and_frame(state, outbound, conn_id);
                info!(conn_id, clothing_slot, applied, "sim: SELF clothing");
            } else if tag.eq_ignore_ascii_case("SREMV") {
                // Haxe specialRemove / specialRemoveHelper (TH-CLOTHING-MATRIX)
                touch_afk_activity(state, conn_id);
                // SREMV x y c i#
                let mut parts = payload.split_whitespace();
                let _x = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                let _y = parts.next().and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                let clothing_slot = parts
                    .next()
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(-1);
                let index = parts.next().and_then(|s| s.parse::<i32>().ok());
                let ok = state
                    .players
                    .get_mut(&conn_id)
                    .map(|p| {
                        clothing_cmds::apply_sremv_from_clothing(p, clothing_slot, index).is_ok()
                    })
                    .unwrap_or(false);
                if ok {
                    state.publish_player_view(conn_id);
                    info!(conn_id, clothing_slot, ?index, "sim: SREMV from clothing");
                }
                send_player_update_and_frame(state, outbound, conn_id);
            } else {
                touch_afk_activity(state, conn_id);
                debug!(conn_id, %tag, %payload, "sim: raw intent");
            }"#;
        if lib.contains(marker) {
            lib = lib.replacen(marker, insert, 1);
            changed = true;
        }
    }

    if changed {
        let _ = std::fs::write(&lib_path, &lib);
    }

    // use_transition: allow_target_reset uses is_clothing
    if let Ok(mut ut) = std::fs::read_to_string(&use_path) {
        if !ut.contains("!def.is_clothing()") {
            if let Some(start) = ut.find("fn allow_target_reset(") {
                if let Some(rel_end) = ut[start..].find("\n}\n\n/// Apply USE") {
                    let end = start + rel_end + 2;
                    let replacement = "fn allow_target_reset(content: &ContentDb, target_id: i32) -> bool {\n    let Some(def) = content.get(target_id) else {\n        return true;\n    };\n    // Haxe: resetNumberOfUses = !isClothing || numUses < 2 (TH-CLOTHING-MATRIX)\n    if def.num_uses < 2 {\n        return true;\n    }\n    !def.is_clothing()\n}";
                    ut.replace_range(start..end, replacement);
                    let _ = std::fs::write(&use_path, ut);
                    changed = true;
                }
            }
        }
    }

    clothing_transitions_wired(&std::fs::read_to_string(&lib_path).unwrap_or_default()) || changed
}
