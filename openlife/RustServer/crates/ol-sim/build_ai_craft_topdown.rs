//! AI-CRAFT-TOPDOWN / craft_topdown: wire DoTransitionSearch filters (idempotent).
//!
//! // Haxe: AiBase.searchBestTransitionTopDown / DoTransitionSearch

use std::path::{Path, PathBuf};
use std::process::Command;

pub fn already_wired(src: &Path) -> bool {
    let ci = src.join("craft_item.rs");
    let ct = src.join("craft_topdown.rs");
    if !ct.exists() {
        return false;
    }
    std::fs::read_to_string(&ci)
        .map(|t| {
            t.contains("mod craft_topdown")
                && t.contains("search_best_object_for_crafting_topdown")
                && t.contains("CraftObjectIndex::from_objs")
        })
        .unwrap_or(false)
}

pub fn patch_all(src: &Path, workspace: &Path) -> bool {
    if already_wired(src) {
        let _ = patch_docs(workspace);
        return true;
    }
    let py = src.join("_apply_ai_craft_topdown.py");
    if py.exists() {
        let status = Command::new("python")
            .arg(&py)
            .current_dir(src)
            .status()
            .or_else(|_| Command::new("python3").arg(&py).current_dir(src).status());
        if let Ok(s) = status {
            if s.success() && already_wired(src) {
                let _ = patch_docs(workspace);
                return true;
            }
        }
    }
    let ok = patch_minimal_rust(src);
    if ok {
        let _ = patch_docs(workspace);
    }
    ok
}

fn normalize_nl(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn restore_nl(s: &str, crlf: bool) -> String {
    if crlf {
        s.replace('\n', "\r\n")
    } else {
        s.to_string()
    }
}

/// Pure-Rust wire: nest craft_topdown + delegate search + helper opts.
fn patch_minimal_rust(src: &Path) -> bool {
    let ci = src.join("craft_item.rs");
    let Ok(raw) = std::fs::read_to_string(&ci) else {
        return false;
    };
    if !src.join("craft_topdown.rs").exists() {
        return false;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    // Header
    if !t.contains("AI-CRAFT-TOPDOWN") {
        t = t.replacen(
            "//! Multi-step **craftItem** / **craftItemHelper** world craft (AI-CRAFT-MULTI).",
            "//! Multi-step **craftItem** / **craftItemHelper** world craft (AI-CRAFT-MULTI + **AI-CRAFT-TOPDOWN**).",
            1,
        );
        t = t.replacen(
            "//! against reverse-graph + world object snapshot (not full TransitionImporter\n//! top-down with `aiShouldIgnore` / time-transition filters — residual).",
            "//! against reverse-graph + world object snapshot, with top-down `DoTransitionSearch`\n//! filters and hostile/unreachable scan gates ([`craft_topdown`]).",
            1,
        );
        if !t.contains("searchBestTransitionTopDown") {
            t = t.replacen(
                "//! - `AiBase.searchBestObjectForCrafting` ~7132–7186\n",
                "//! - `AiBase.searchBestObjectForCrafting` ~7132–7186\n//! - `AiBase.searchBestTransitionTopDown` / `DoTransitionSearch` ~7696–8039\n",
                1,
            );
        }
        changed = true;
    }

    // Nest mod
    if !t.contains("mod craft_topdown") {
        let anchor = "use crate::craft_graph::ReverseCraftGraph;\nuse crate::short_craft_intent::ShortCraftLiveIntent;\n";
        if t.contains(anchor) {
            let insert = concat!(
                "use crate::craft_graph::ReverseCraftGraph;\n",
                "use crate::short_craft_intent::ShortCraftLiveIntent;\n\n",
                "// Haxe: searchBestTransitionTopDown / DoTransitionSearch (AI-CRAFT-TOPDOWN)\n",
                "#[path = \"craft_topdown.rs\"]\n",
                "mod craft_topdown;\n",
                "pub use craft_topdown::{\n",
                "    auto_decay_time_base_seconds, closest_craft_obj_filtered, craft_obj_passes_scan_filters,\n",
                "    do_transition_search_skip_reason, effective_ai_should_ignore,\n",
                "    hardened_row_forces_hoe_soil_ignore, search_best_object_for_crafting_topdown,\n",
                "    should_skip_craft_edge, should_skip_transition_top_down,\n",
                "    time_transition_exceeds_ai_ignore, CraftObjectIndex, CraftScanFilters,\n",
                "    CraftTopDownOpts, CraftTransMeta, TransSkipReason, AI_CRAFT_MIN_COUNT_RADIUS_CAP,\n",
                "    AI_IGNORE_TIME_TRANSITIONS_LONGER_THAN, HARDENED_ROW, STEEL_HOE, STONE_HOE,\n",
                "};\n",
            );
            t = t.replacen(anchor, insert, 1);
            changed = true;
        }
    }

    // Replace search_best_object_for_crafting body with topdown delegate
    if t.contains("pub fn search_best_object_for_crafting(")
        && !t.contains("search_best_object_for_crafting_topdown(\n        product_id,")
    {
        let markers = [
            "// Haxe: searchBestObjectForCrafting + searchBestTransitionTopDown simplified\n",
            "// ── searchBestObjectForCrafting (reverse-graph first cut) ───────────────────\n",
        ];
        for marker in markers {
            if let Some(start) = t.find(marker) {
                if let Some(rel) = t[start..].find("\nfn find_best_pair_in_radius") {
                    let end = start + rel;
                    let new_fns = r#"// Haxe: searchBestObjectForCrafting + searchBestTransitionTopDown (+ AI-CRAFT-TOPDOWN filters)
pub fn search_best_object_for_crafting(
    product_id: i32,
    objs: &[CraftWorldObj],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    max_search_radius: i32,
    graph: &ReverseCraftGraph,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
) -> Option<CraftTransPair> {
    search_best_object_for_crafting_topdown(
        product_id,
        objs,
        held_id,
        player_x,
        player_y,
        home,
        max_search_radius,
        graph,
        pile_id_for,
        &CraftTopDownOpts::default(),
    )
}

/// Filtered craft search with last-transition / scan opts.
// Haxe: searchBestObjectForCrafting + DoTransitionSearch filters
pub fn search_best_object_for_crafting_ex(
    product_id: i32,
    objs: &[CraftWorldObj],
    held_id: i32,
    player_x: i32,
    player_y: i32,
    home: Option<(i32, i32)>,
    max_search_radius: i32,
    graph: &ReverseCraftGraph,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
    opts: &CraftTopDownOpts<'_>,
) -> Option<CraftTransPair> {
    search_best_object_for_crafting_topdown(
        product_id,
        objs,
        held_id,
        player_x,
        player_y,
        home,
        max_search_radius,
        graph,
        pile_id_for,
        opts,
    )
}

"#;
                    t = format!("{}{}{}", &t[..start], new_fns, &t[end..]);
                    changed = true;
                    break;
                }
            }
        }
    }

    // Helper uses topdown opts with sticky last transition
    if !t.contains("CraftObjectIndex::from_objs") {
        let old = "    // Search best multi-step pair.\n    let pair = search_best_object_for_crafting(\n        product_id,\n        objs,\n        inp.held_id,\n        inp.player_x,\n        inp.player_y,\n        home,\n        max_r,\n        graph,\n        pile_id_for,\n    );\n";
        let new = "    // Search best multi-step pair (AI-CRAFT-TOPDOWN filters + sticky last transition).\n    // Haxe: searchBestObjectForCrafting + DoTransitionSearch lastActor/Target undo\n    let craft_index = CraftObjectIndex::from_objs(objs, None);\n    let exists_row = objs.iter().any(|o| o.parent_id == HARDENED_ROW);\n    let topdown_opts = CraftTopDownOpts::default()\n        .with_last(state.last_actor_id, state.last_target_id)\n        .with_hardened_row(exists_row)\n        .with_index(&craft_index);\n    let pair = search_best_object_for_crafting_ex(\n        product_id,\n        objs,\n        inp.held_id,\n        inp.player_x,\n        inp.player_y,\n        home,\n        max_r,\n        graph,\n        pile_id_for,\n        &topdown_opts,\n    );\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if changed {
        let _ = std::fs::write(&ci, restore_nl(&t, crlf));
    }

    // lib exports
    let lib = src.join("lib.rs");
    if let Ok(raw) = std::fs::read_to_string(&lib) {
        if !raw.contains("CraftTransMeta") && raw.contains("search_best_object_for_crafting,") {
            let raw2 = raw.replacen(
                "search_best_object_for_crafting,",
                "search_best_object_for_crafting, search_best_object_for_crafting_ex,\n    search_best_object_for_crafting_topdown, should_skip_transition_top_down,\n    closest_craft_obj_filtered, CraftObjectIndex, CraftScanFilters,\n    CraftTopDownOpts, CraftTransMeta, TransSkipReason,\n    AI_IGNORE_TIME_TRANSITIONS_LONGER_THAN, HARDENED_ROW,",
                1,
            );
            let _ = std::fs::write(&lib, raw2);
        }
    }

    // get_or_craft re-exports
    let goc = src.join("get_or_craft.rs");
    if let Ok(raw) = std::fs::read_to_string(&goc) {
        if !raw.contains("search_best_object_for_crafting_ex")
            && raw.contains("search_best_object_for_crafting,\n")
        {
            let raw2 = raw.replacen(
                "    first_missing_ingredient, resolve_craft_item_live, search_best_object_for_crafting,\n",
                "    first_missing_ingredient, resolve_craft_item_live, search_best_object_for_crafting,\n    search_best_object_for_crafting_ex, search_best_object_for_crafting_topdown,\n    CraftObjectIndex, CraftScanFilters, CraftTopDownOpts, CraftTransMeta,\n",
                1,
            );
            let _ = std::fs::write(&goc, raw2);
        }
    }

    already_wired(src)
}

fn patch_docs(workspace: &Path) -> bool {
    let docs = workspace.join("docs").join("port");
    if !docs.exists() {
        return false;
    }
    // FILE_MATRIX
    let matrix = docs.join("FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&matrix) {
        if !raw.contains("AI-CRAFT-TOPDOWN") {
            let row = "| **AI-CRAFT-TOPDOWN** / craft_topdown | searchBestTransitionTopDown / DoTransitionSearch filters | **DONE** (core) | `craft_topdown.rs` nested in craft_item — aiShouldIgnore, time>120s, undo-last, reverseUse full, minUseFraction, max/min, hardened-row hoe+soil, hostile/unreachable scan; search→topdown; tests craft_topdown::*. Residual: full BFS wantedObjs; live Transition meta; ObjectDef aiCraftMax/Min |\n";
            if let Some(idx) = raw.find("| **AI-CRAFT-MULTI**") {
                if let Some(eol) = raw[idx..].find('\n') {
                    let at = idx + eol + 1;
                    let t = format!("{}{}{}", &raw[..at], row, &raw[at..]);
                    let _ = std::fs::write(&matrix, t);
                }
            }
        }
    }
    // TODO_PORT
    let todo = docs.join("TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo) {
        if !raw.contains("AI-CRAFT-TOPDOWN craft_topdown") {
            let line = "- [x] **AI-CRAFT-TOPDOWN craft_topdown DONE** — `craft_topdown.rs` DoTransitionSearch filters (aiShouldIgnore, time>120, undo-last, reverseUse full, minUseFraction, max/min, hardened-row) + scan hostile/unreachable; search→topdown; tests craft_topdown::*. Residual: BFS wantedObjs; Transition content meta; ObjectDef aiCraftMax/Min |\n";
            if let Some(idx) = raw.find("- [~] **AI-CRAFT-MULTI craft_item_live PARTIAL**") {
                let t = format!("{}{}{}", &raw[..idx], line, &raw[idx..]);
                let _ = std::fs::write(&todo, t);
            }
        }
    }
    // CALL_INDEX
    let call = docs.join("CALL_INDEX.md");
    if let Ok(raw) = std::fs::read_to_string(&call) {
        if !raw.contains("AI-CRAFT-TOPDOWN") {
            let block = "\n## Rust: craft top-down filters (AI-CRAFT-TOPDOWN)\n\n| Symbol | File | Role |\n|--------|------|------|\n| `do_transition_search_skip_reason` / `CraftTransMeta` | `ol-sim/src/craft_topdown.rs` | pure DoTransitionSearch skip gates |\n| `search_best_object_for_crafting_topdown` / `_ex` | same | filtered reverse-graph craft pair |\n| `closest_craft_obj_filtered` / `CraftScanFilters` | same | hostile/unreachable/full pile scan |\n| `hardened_row_forces_hoe_soil_ignore` / HARDENED_ROW | same | dynamic hoe+soil ignore |\n| `AI_IGNORE_TIME_TRANSITIONS_LONGER_THAN` | same | AiIgnoreTimeTransitionsLongerThen=120 |\n\n";
            if let Some(idx) = raw.find("## Rust: GetOrCraftItem pure I/O") {
                let t = format!("{}{}{}", &raw[..idx], block, &raw[idx..]);
                let _ = std::fs::write(&call, t);
            }
        }
    }
    // changelog
    let cl = docs.join("changelog").join("2026-07-29-AI-CRAFT-TOPDOWN.md");
    if !cl.exists() {
        let body = "# AI-CRAFT-TOPDOWN / craft_topdown (2026-07-29)\n\n## Status: DONE (core)\n\nPure DoTransitionSearch filters + scan gates in `craft_topdown.rs`; wired via craft_item search.\n\n```powershell\ncargo test -p ol-sim --lib -- craft_topdown\ncargo test -p ol-sim --lib -- craft_item\n```\n";
        let _ = std::fs::write(&cl, body);
    }
    // QUEUE
    let queue = docs.join("QUEUE.md");
    if let Ok(raw) = std::fs::read_to_string(&queue) {
        if raw.contains("| `AI-CRAFT-TOPDOWN` | craft_topdown |") {
            let t = raw.replace(
                "| `AI-CRAFT-TOPDOWN` | craft_topdown | craftItem top-down transition search residual |",
                "| ~~`AI-CRAFT-TOPDOWN`~~ | craft_topdown | **DONE** core DoTransitionSearch filters |",
            );
            let _ = std::fs::write(&queue, t);
        }
    }
    true
}

pub fn src_dir(manifest: &Path) -> PathBuf {
    manifest.join("src")
}
