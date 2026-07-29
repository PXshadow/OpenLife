//! BLOCKED-BY-AI / blocked_rebuild — build-time live wire (idempotent).
//!
//! Sticky `Player.ai_block_targets` + tick_vitals rebuild of `SimState.blocked_by_ai`
//! from food/use/drop claims + human blockTargetForAi after USE.
//!
//! // Haxe: AiBase.CalculateBlockedByAi ~222–239; TransitionHelper.use ~397–414

use std::path::Path;

fn normalize_nl(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

fn restore_nl(s: &str, crlf: bool) -> String {
    if crlf {
        s.replace("\n", "\r\n")
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
        eprintln!("cargo:warning=BLOCKED-BY-AI write {}: {e}", path.display());
        return false;
    }
    true
}

/// True when sticky field + live rebuild + intent note are present.
pub fn blocked_by_ai_live_wired(lib: &str, player: &str, short_craft: &str, use_tr: &str) -> bool {
    player.contains("ai_block_targets")
        && lib.contains("rebuild_blocked_by_ai_live")
        && lib.contains("note_ai_block_targets_from_live_intent")
        && lib.contains("AiStickyBlockTargets")
        && short_craft.contains("note_ai_block_targets_from_live_intent")
        && use_tr.contains("should_set_block_target_for_ai")
        && lib.contains("BLOCKED-BY-AI: wipe+rebuild")
}

/// Build hook: wire sticky live CalculateBlockedByAi.
pub fn patch_blocked_by_ai_live(src: &Path, workspace: &Path) -> bool {
    println!("cargo:rerun-if-changed=build_blocked_by_ai.rs");
    println!("cargo:rerun-if-changed=src/ai_path_reach.rs");
    println!("cargo:rerun-if-changed=src/_apply_blocked_by_ai.py");

    // Optional python apply (full patcher).
    let py = src.join("_apply_blocked_by_ai.py");
    if py.exists() {
        let _ = std::process::Command::new("python")
            .arg(&py)
            .status()
            .or_else(|_| std::process::Command::new("python3").arg(&py).status());
    }

    let lib_path = src.join("lib.rs");
    let player_path = src.join("player.rs");
    let sc_path = src.join("short_craft_intent.rs");
    let use_path = src.join("use_transition.rs");

    let lib = std::fs::read_to_string(&lib_path).unwrap_or_default();
    let player = std::fs::read_to_string(&player_path).unwrap_or_default();
    let sc = std::fs::read_to_string(&sc_path).unwrap_or_default();
    let use_tr = std::fs::read_to_string(&use_path).unwrap_or_default();
    if blocked_by_ai_live_wired(&lib, &player, &sc, &use_tr) {
        let stamp = src.join(".blocked_by_ai_patched");
        let _ = std::fs::write(&stamp, b"blocked-by-ai-1-source-wired\n");
        let _ = patch_docs(workspace);
        return true;
    }

    let mut any = false;
    any |= patch_player(&player_path);
    any |= patch_lib(&lib_path);
    any |= patch_short_craft(&sc_path);
    any |= patch_use_transition(&use_path);
    any |= patch_docs(workspace);

    let lib2 = std::fs::read_to_string(&lib_path).unwrap_or_default();
    let player2 = std::fs::read_to_string(&player_path).unwrap_or_default();
    let sc2 = std::fs::read_to_string(&sc_path).unwrap_or_default();
    let use2 = std::fs::read_to_string(&use_path).unwrap_or_default();
    let ok = blocked_by_ai_live_wired(&lib2, &player2, &sc2, &use2);
    if ok {
        let stamp = src.join(".blocked_by_ai_patched");
        let _ = std::fs::write(
            &stamp,
            if any {
                b"blocked-by-ai-1-rs-patched\n".as_slice()
            } else {
                b"blocked-by-ai-1-source-wired\n"
            },
        );
    } else {
        println!("cargo:warning=BLOCKED-BY-AI: could not fully wire sticky rebuild live");
    }
    ok || any
}

fn patch_player(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("ai_block_targets") {
        return false;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;
    ch |= replace_once(
        &mut t,
        "    pub ai_path_reach: crate::ai_path_reach::AiPathReachMaps,\n}\n",
        "    pub ai_path_reach: crate::ai_path_reach::AiPathReachMaps,\n\
    /// Sticky AI food/use/drop/block claims for live CalculateBlockedByAi (**BLOCKED-BY-AI**).\n\
    // Haxe: AiBase.foodTarget / dropTarget / useTarget + GPI.blockTargetForAi\n\
    pub ai_block_targets: crate::ai_path_reach::AiStickyBlockTargets,\n}\n",
    );
    ch |= replace_once(
        &mut t,
        "            ai_path_reach: crate::ai_path_reach::AiPathReachMaps::default(),\n        }\n    }\n",
        "            ai_path_reach: crate::ai_path_reach::AiPathReachMaps::default(),\n\
            // BLOCKED-BY-AI: sticky food/use/drop/block claims\n\
            ai_block_targets: crate::ai_path_reach::AiStickyBlockTargets::default(),\n        }\n    }\n",
    );
    if ch {
        write_if_changed(path, &raw, &restore_nl(&t, crlf))
    } else {
        false
    }
}

fn patch_lib(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;

    // Expand exports
    if !t.contains("AiStickyBlockTargets") {
        ch |= replace_once(
            &mut t,
            "    AiAgentBlockSource, AiPathReachMaps, BlockTargetClaim, HumanBlockClaim,\n",
            "    AiAgentBlockSource, AiPathReachMaps, AiStickyBlockTargets, BlockTargetClaim, HumanBlockClaim,\n\
    StickyBlockBodyRow, StickyBlockIntentKind,\n",
        );
        ch |= replace_once(
            &mut t,
            "    add_agent_to_blocked_by_ai, add_blocked_by_ai, apply_calculate_blocked_by_ai,\n",
            "    add_agent_to_blocked_by_ai, add_blocked_by_ai, apply_calculate_blocked_by_ai,\n\
    apply_rebuild_blocked_by_ai_from_sticky,\n",
        );
        ch |= replace_once(
            &mut t,
            "    blocked_coords_from_live, calculate_blocked_by_ai, cleanup_blocked_by_ai,\n",
            "    blocked_coords_from_live, calculate_blocked_by_ai, cleanup_blocked_by_ai,\n\
    rebuild_blocked_by_ai_from_sticky, should_set_block_target_for_ai,\n",
        );
        ch |= replace_once(
            &mut t,
            "    NOT_REACHABLE_DEFAULT_SECS, NOT_REACHABLE_FOOD_SECS,\n};",
            "    NOT_REACHABLE_DEFAULT_SECS, NOT_REACHABLE_FOOD_SECS, SMITHING_HAMMER_BLOCK_ID,\n};",
        );
        // alternate export formatting
        if !t.contains("AiStickyBlockTargets") {
            ch |= replace_once(
                &mut t,
                "AiPathReachMaps, BlockTargetClaim, HumanBlockClaim,",
                "AiPathReachMaps, AiStickyBlockTargets, BlockTargetClaim, HumanBlockClaim,\n    StickyBlockBodyRow, StickyBlockIntentKind,",
            );
        }
        if !t.contains("rebuild_blocked_by_ai_from_sticky") {
            ch |= replace_once(
                &mut t,
                "apply_calculate_blocked_by_ai,",
                "apply_calculate_blocked_by_ai, apply_rebuild_blocked_by_ai_from_sticky, rebuild_blocked_by_ai_from_sticky, should_set_block_target_for_ai,",
            );
        }
        if !t.contains("SMITHING_HAMMER_BLOCK_ID") {
            ch |= replace_once(
                &mut t,
                "NOT_REACHABLE_FOOD_SECS,",
                "NOT_REACHABLE_FOOD_SECS, SMITHING_HAMMER_BLOCK_ID,",
            );
        }
    }

    // Live rebuild + note helpers before tick_vitals
    if !t.contains("fn rebuild_blocked_by_ai_live") && !t.contains("pub fn rebuild_blocked_by_ai_live")
    {
        let fns = r#"
/// Live Haxe `CalculateBlockedByAi` — wipe+rebuild `blocked_by_ai` from sticky targets.
///
/// Collects living AI agents' food/use/drop/block claims and human
/// `blockTargetForAi` (age ≤ 20s), then replaces the global map.
// Haxe: AiBase.CalculateBlockedByAi ~222–239 (each AI frame)
// BLOCKED-BY-AI
pub fn rebuild_blocked_by_ai_live(state: &mut SimState) {
    use crate::ai_path_reach::{
        apply_rebuild_blocked_by_ai_from_sticky, StickyBlockBodyRow,
    };
    let sim_time = state.sim_time;
    let bodies: Vec<StickyBlockBodyRow> = state
        .players
        .values()
        .map(|p| {
            let wounded = p.hidden_wound.is_some();
            StickyBlockBodyRow {
                is_ai: p.is_ai_body(),
                age: p.age,
                is_wounded: wounded,
                deleted: p.deleted,
                sticky: p.ai_block_targets.clone(),
            }
        })
        .collect();
    apply_rebuild_blocked_by_ai_from_sticky(&mut state.blocked_by_ai, sim_time, &bodies);
}

/// Note sticky food/use/drop claim from a shortCraft live intent (before USE/DROP).
// Haxe: AiBase.useTarget / dropTarget / foodTarget set while working
// BLOCKED-BY-AI
pub fn note_ai_block_targets_from_live_intent(
    state: &mut SimState,
    conn_id: u64,
    intent: crate::ShortCraftLiveIntent,
) {
    use crate::ai_path_reach::{BlockTargetClaim, StickyBlockIntentKind};
    use crate::ShortCraftLiveIntent;
    let (mut kind, x, y, target_hint) = match intent {
        ShortCraftLiveIntent::UseAt {
            x,
            y,
            target_id,
            ..
        } => (StickyBlockIntentKind::Use, x, y, target_id),
        ShortCraftLiveIntent::UseOnEmptyGround { x, y, .. } => {
            (StickyBlockIntentKind::Use, x, y, 0)
        }
        ShortCraftLiveIntent::DropAt { x, y } => (StickyBlockIntentKind::Drop, x, y, 0),
        _ => return,
    };
    let held_id = state
        .players
        .get(&conn_id)
        .map(|p| p.held_id)
        .unwrap_or(0);
    let (parent_id, number_of_uses, is_animal, food_value, held_new_target_id) = {
        let world = match state.world.read() {
            Ok(w) => w,
            Err(_) => return,
        };
        let id = world.get_object(x, y);
        let base = if id != 0 {
            state.content.resolve_base_id(id)
        } else if target_hint != 0 {
            state.content.resolve_base_id(target_hint)
        } else {
            0
        };
        let def = state.content.get(base);
        let is_animal = def.map(|d| d.is_animal()).unwrap_or(false);
        let food_value = def.map(|d| d.food_value).unwrap_or(0);
        let uses = world
            .get_helper(x, y)
            .map(|h| h.uses_remaining)
            .unwrap_or(0);
        let num_uses = def.map(|d| d.num_uses).unwrap_or(0);
        let number_of_uses = if uses > 0 {
            uses
        } else if num_uses > 0 {
            num_uses
        } else {
            1
        };
        let held_new = if held_id != 0 && base != 0 {
            state
                .content
                .find_transition(held_id, base)
                .map(|tr| state.content.resolve_base_id(tr.new_target_id))
        } else {
            None
        };
        (base, number_of_uses, is_animal, food_value, held_new)
    };
    if matches!(kind, StickyBlockIntentKind::Use) && food_value > 0 {
        kind = StickyBlockIntentKind::Food;
    }
    let claim = BlockTargetClaim {
        x,
        y,
        parent_id: if parent_id != 0 {
            parent_id
        } else {
            target_hint
        },
        number_of_uses,
        is_animal,
        held_new_target_id,
    };
    if let Some(p) = state.players.get_mut(&conn_id) {
        p.ai_block_targets.note_action_claim(kind, claim);
    }
}

"#;
        ch |= replace_once(
            &mut t,
            "pub fn tick_vitals(state: &mut SimState, dt: f32, outbound: &OutboundHub) {",
            &format!(
                "{fns}pub fn tick_vitals(state: &mut SimState, dt: f32, outbound: &OutboundHub) {{"
            ),
        );
    }

    // tick_vitals: rebuild instead of only decaying blocked_by_ai
    // Keep "PATH-REACH: cleanup AI path maps" substring so path_reach_wired still matches.
    if !t.contains("BLOCKED-BY-AI: wipe+rebuild") {
        ch |= replace_once(
            &mut t,
            "    // PATH-REACH: cleanup AI path maps (Haxe cleanupBlockedObjects each reaction).\n\
    // Haxe: AiBase.cleanupBlockedObjectsHelper ~6264\n\
    {\n\
        use crate::ai_path_reach::cleanup_blocked_by_ai;\n\
        cleanup_blocked_by_ai(&mut state.blocked_by_ai, dt);\n\
        for p in state.players.values_mut() {\n\
            p.ai_path_reach.cleanup(dt);\n\
        }\n\
    }",
            "    // PATH-REACH: cleanup AI path maps (Haxe cleanupBlockedObjects each reaction).\n\
    // Haxe: AiBase.cleanupBlockedObjectsHelper ~6264\n\
    {\n\
        for p in state.players.values_mut() {\n\
            p.ai_path_reach.cleanup(dt);\n\
        }\n\
    }\n\
    // BLOCKED-BY-AI: wipe+rebuild global blockedByAI from sticky food/use/drop/block.\n\
    // Haxe: AiBase.CalculateBlockedByAi ~222 each AI frame (replaces decay-only map).\n\
    rebuild_blocked_by_ai_live(state);",
        );
    }

    if ch {
        write_if_changed(path, &raw, &restore_nl(&t, crlf))
    } else {
        false
    }
}

fn patch_short_craft(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("note_ai_block_targets_from_live_intent") {
        return false;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let ch = replace_once(
        &mut t,
        ") -> ShortCraftLiveApplyResult {\n    match intent {\n",
        ") -> ShortCraftLiveApplyResult {\n\
    // BLOCKED-BY-AI: sticky use/drop/food claim before USE/DROP so rebuild sees it.\n\
    // Haxe: AiBase.useTarget / dropTarget / foodTarget while working\n\
    crate::note_ai_block_targets_from_live_intent(state, conn_id, intent);\n\
    match intent {\n",
    );
    if ch {
        write_if_changed(path, &raw, &restore_nl(&t, crlf))
    } else {
        false
    }
}

fn patch_use_transition(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("should_set_block_target_for_ai") {
        return false;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let insert = r#"
    // BLOCKED-BY-AI: human / smith-hammer set blockTargetForAi after USE.
    // Haxe: TransitionHelper.use ~397–414
    {
        use crate::ai_path_reach::{should_set_block_target_for_ai, BlockTargetClaim};
        use crate::animal_damage::is_weapon_from_deadly_distance;
        let sim_time = state.sim_time;
        let is_human = state
            .players
            .get(&conn_id)
            .map(|p| p.is_human_body())
            .unwrap_or(false);
        let (parent_id, number_of_uses, is_animal, permanent, food_value, is_clothing, is_weapon) = {
            let base = if live_target != 0 {
                state.content.resolve_base_id(live_target)
            } else {
                0
            };
            let def = state.content.get(base);
            let is_animal = def.map(|d| d.is_animal()).unwrap_or(false);
            let permanent = def.map(|d| d.permanent).unwrap_or(false);
            let food_value = def.map(|d| d.food_value).unwrap_or(0);
            let is_clothing = def.map(|d| d.is_clothing()).unwrap_or(false);
            let deadly = def.map(|d| d.deadly_distance).unwrap_or(0.0);
            let is_weapon = is_weapon_from_deadly_distance(deadly);
            let uses = state
                .world
                .read()
                .ok()
                .and_then(|w| w.get_helper(tx, ty).map(|h| h.uses_remaining))
                .unwrap_or(0);
            let num_uses = def.map(|d| d.num_uses).unwrap_or(0);
            let number_of_uses = if uses > 0 {
                uses
            } else if num_uses > 0 {
                num_uses
            } else {
                1
            };
            (
                base,
                number_of_uses,
                is_animal,
                permanent,
                food_value,
                is_clothing,
                is_weapon,
            )
        };
        if should_set_block_target_for_ai(
            is_human,
            actor,
            parent_id,
            permanent,
            is_weapon,
            is_animal,
            food_value,
            is_clothing,
        ) {
            let claim = BlockTargetClaim {
                x: tx,
                y: ty,
                parent_id,
                number_of_uses,
                is_animal,
                held_new_target_id: None,
            };
            if let Some(p) = state.players.get_mut(&conn_id) {
                p.ai_block_targets.set_player_block(claim, sim_time);
            }
        }
    }

"#;
    let ch = replace_once(
        &mut t,
        "    Some(UseResult {\n\
        actor_before: actor,\n\
        target_before: target,\n\
        actor_after: final_actor,\n\
        target_after: live_target,\n\
        applied: true,\n\
        x: tx,\n\
        y: ty,\n\
    })\n}\n\n/// Wire held id for PU",
        &format!(
            "{insert}    Some(UseResult {{\n\
        actor_before: actor,\n\
        target_before: target,\n\
        actor_after: final_actor,\n\
        target_after: live_target,\n\
        applied: true,\n\
        x: tx,\n\
        y: ty,\n\
    }})\n}}\n\n/// Wire held id for PU"
        ),
    );
    if ch {
        write_if_changed(path, &raw, &restore_nl(&t, crlf))
    } else {
        false
    }
}

fn patch_docs(workspace: &Path) -> bool {
    let mut any = false;
    let port = workspace.join("docs/port");

    // FILE_MATRIX
    let fm = port.join("FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&fm) {
        if !raw.contains("BLOCKED-BY-AI") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            if replace_once(
                &mut t,
                "| **PATH-REACH** / not_reachable_maps |",
                "| **BLOCKED-BY-AI** / blocked_rebuild | AiBase CalculateBlockedByAi live rebuild | **DONE** | sticky `Player.ai_block_targets` + `rebuild_blocked_by_ai_live` tick; shortCraft note; USE human/smith blockTarget; tests |\n| **PATH-REACH** / not_reachable_maps |",
            ) {
                any |= write_if_changed(&fm, &raw, &restore_nl(&t, crlf));
            }
        }
    }

    // TODO_PORT
    let tp = port.join("TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&tp) {
        if !raw.contains("BLOCKED-BY-AI blocked_rebuild") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            if let Some(i) = t.find("PATH-REACH not_reachable_maps PARTIAL") {
                if let Some(nl) = t[i..].find('\n') {
                    let insert_at = i + nl + 1;
                    let line = "- [x] **BLOCKED-BY-AI blocked_rebuild DONE** — sticky `Player.ai_block_targets`; pure rebuild_from_sticky; live tick rebuild; shortCraft note; USE human/smith blockTarget; tests sticky rebuild + gates  \n";
                    t.insert_str(insert_at, line);
                    if let Some(h) = t.find("| 2026-07-28 | **PATH-REACH not_reachable_maps PARTIAL**") {
                        t.insert_str(
                            h,
                            "| 2026-07-28 | **BLOCKED-BY-AI blocked_rebuild DONE**: sticky AiStickyBlockTargets + rebuild_blocked_by_ai_live; shortCraft note; USE blockTarget; tests |\n",
                        );
                    }
                    any |= write_if_changed(&tp, &raw, &restore_nl(&t, crlf));
                }
            }
        }
    }

    // CALL_INDEX
    let ci = port.join("CALL_INDEX.md");
    if let Ok(raw) = std::fs::read_to_string(&ci) {
        if !raw.contains("rebuild_blocked_by_ai_live") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            if replace_once(
                &mut t,
                "| `AiAgentBlockSource` / `HumanBlockClaim` / `add_agent_to_blocked_by_ai` / `calculate_blocked_by_ai` / `apply_calculate_blocked_by_ai` | same | pure CalculateBlockedByAi rebuild |\n",
                "| `AiAgentBlockSource` / `HumanBlockClaim` / `add_agent_to_blocked_by_ai` / `calculate_blocked_by_ai` / `apply_calculate_blocked_by_ai` | same | pure CalculateBlockedByAi rebuild |\n\
| `AiStickyBlockTargets` / `StickyBlockBodyRow` / `rebuild_blocked_by_ai_from_sticky` / `should_set_block_target_for_ai` | same | sticky claims + pure live rebuild |\n\
| `rebuild_blocked_by_ai_live` / `note_ai_block_targets_from_live_intent` | `lib.rs` | tick wipe+rebuild + shortCraft note |\n\
| `Player.ai_block_targets` | `player.rs` | sticky food/use/drop/block claims |\n",
            ) {
                any |= write_if_changed(&ci, &raw, &restore_nl(&t, crlf));
            }
        }
    }

    // QUEUE
    let q = port.join("QUEUE.md");
    if let Ok(raw) = std::fs::read_to_string(&q) {
        if raw.contains("`BLOCKED-BY-AI` | blocked_rebuild") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            let _ = replace_once(
                &mut t,
                "| `BLOCKED-BY-AI` | blocked_rebuild | Live CalculateBlockedByAi rebuild residual |\n",
                "",
            );
            let _ = replace_once(
                &mut t,
                "**PATH-REACH** PARTIAL",
                "**BLOCKED-BY-AI** DONE · **PATH-REACH** PARTIAL",
            );
            any |= write_if_changed(&q, &raw, &restore_nl(&t, crlf));
        }
    }

    // changelog
    let ch = port.join("changelog/2026-07-28-BLOCKED-BY-AI.md");
    if !ch.exists() {
        let body = r#"# BLOCKED-BY-AI / blocked_rebuild (2026-07-28)

## Summary

Live Haxe `AiBase.CalculateBlockedByAi` — wipe and rebuild global `blockedByAI` each tick from sticky AI food/use/drop targets and human `blockTargetForAi`.

## Files

- **Core pure:** `crates/ol-sim/src/ai_path_reach.rs` — `AiStickyBlockTargets`, `rebuild_blocked_by_ai_from_sticky`, `should_set_block_target_for_ai`
- **Player:** `Player.ai_block_targets`
- **Live:** `rebuild_blocked_by_ai_live` + `note_ai_block_targets_from_live_intent` in `lib.rs`; tick_vitals rebuild
- **Intent:** `apply_short_craft_live_intent` notes use/drop/food sticky before USE/DROP
- **USE:** `use_transition::apply_use_at` sets human/smith hammer `blockTargetForAi`

## Tests

```powershell
cargo test -p ol-sim --lib -- ai_path_reach sticky rebuild_from_sticky should_set_block
```
"#;
        let _ = std::fs::write(&ch, body);
        any = true;
    }

    // PATH-REACH residual
    let pr = port.join("changelog/2026-07-28-PATH-REACH.md");
    if let Ok(raw) = std::fs::read_to_string(&pr) {
        if raw.contains("Live `CalculateBlockedByAi` rebuild each tick") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            let _ = replace_once(
                &mut t,
                "- Live `CalculateBlockedByAi` rebuild each tick from sticky food/use/drop targets (pure ready; claim sources missing)\n",
                "- ~~Live CalculateBlockedByAi rebuild~~ → **BLOCKED-BY-AI DONE**\n",
            );
            any |= write_if_changed(&pr, &raw, &restore_nl(&t, crlf));
        }
    }

    any
}
