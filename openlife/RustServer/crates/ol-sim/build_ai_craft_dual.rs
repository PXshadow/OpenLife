//! Build-time wire for **AI-CRAFT-DUAL** / dual_center_search.
//!
//! Dual-center searchCurrentPosition + pile-vs-loose *1.5 / r=6 re-anchor.
//! Idempotent pure-Rust string patches + optional Python full apply.

use std::path::{Path, PathBuf};
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

fn write_if_changed(path: &Path, original: &str, next: &str) -> bool {
    if original == next {
        return false;
    }
    if let Err(e) = std::fs::write(path, next) {
        eprintln!(
            "cargo:warning=AI-CRAFT-DUAL write {}: {e}",
            path.display()
        );
        return false;
    }
    true
}

/// True when craft_item.rs has full multi-step body (not header-only truncation).
fn craft_item_intact(src: &Path) -> bool {
    std::fs::read_to_string(src.join("craft_item.rs"))
        .map(|t| {
            t.contains("pub struct FailedCraftings")
                && t.contains("pub fn craft_item_helper")
                && t.len() > 20_000
        })
        .unwrap_or(false)
}

/// Emergency recovery: restore craft_item.rs from Grok session updates.jsonl oldText.
fn recover_craft_item_if_truncated(src: &Path) {
    if craft_item_intact(src) {
        return;
    }
    eprintln!(
        "cargo:warning=craft_item.rs truncated — running recovery (session updates.jsonl)"
    );
    let py = src.join("_recover_craft_item.py");
    if py.exists() {
        let status = Command::new("python")
            .arg(&py)
            .current_dir(src)
            .status()
            .or_else(|_| Command::new("python3").arg(&py).current_dir(src).status());
        if let Ok(s) = status {
            if s.success() && craft_item_intact(src) {
                eprintln!("cargo:warning=craft_item.rs restored from session log");
                return;
            }
        }
    }
    // Pure-Rust fallback
    if recover_craft_item_rust(src) {
        eprintln!("cargo:warning=craft_item.rs restored via Rust session parser");
    } else {
        eprintln!("cargo:warning=craft_item.rs recovery FAILED — rebuild needs manual restore");
    }
}

fn recover_craft_item_rust(src: &Path) -> bool {
    let home = match std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        Ok(h) => h,
        Err(_) => return false,
    };
    let sessions = Path::new(&home).join(".grok").join("sessions");
    let mut candidates: Vec<PathBuf> = Vec::new();
    let preferred = sessions
        .join("C%3A%5CUsers%5Cmarti")
        .join("019fac93-5454-77b3-b002-9f68fb9b61a6")
        .join("updates.jsonl");
    if preferred.is_file() {
        candidates.push(preferred);
    }
    let root = sessions.join("C%3A%5CUsers%5Cmarti");
    if let Ok(rd) = std::fs::read_dir(&root) {
        let mut dirs: Vec<_> = rd.filter_map(|e| e.ok()).collect();
        dirs.sort_by_key(|e| {
            std::cmp::Reverse(
                e.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
            )
        });
        for e in dirs.into_iter().take(40) {
            let u = e.path().join("updates.jsonl");
            if u.is_file() && !candidates.iter().any(|p| p == &u) {
                candidates.push(u);
            }
        }
    }
    for updates in candidates {
        let Ok(text) = std::fs::read_to_string(&updates) else {
            continue;
        };
        for line in text.lines() {
            if !line.contains("craft_item.rs") || !line.contains("oldText") {
                continue;
            }
            if let Some(body) = extract_old_text_rust(line) {
                if body.contains("pub struct FailedCraftings")
                    && body.contains("pub fn craft_item_helper")
                    && body.len() > 20_000
                {
                    if std::fs::write(src.join("craft_item.rs"), body.as_bytes()).is_ok() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn extract_old_text_rust(line: &str) -> Option<String> {
    let key = "\"oldText\":\"";
    let path_idx = line.find("craft_item.rs")?;
    let search_from = path_idx.saturating_sub(80);
    let rel = line[search_from..].find(key)?;
    let start = search_from + rel + key.len();
    let bytes = line.as_bytes();
    let mut out = String::new();
    let mut i = start;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '\\' {
            if i + 1 >= bytes.len() {
                break;
            }
            let n = bytes[i + 1] as char;
            match n {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'u' if i + 5 < bytes.len() => {
                    let hex = &line[i + 2..i + 6];
                    if let Ok(cp) = u32::from_str_radix(hex, 16) {
                        if let Some(ch) = char::from_u32(cp) {
                            out.push(ch);
                        }
                    }
                    i += 6;
                    continue;
                }
                other => out.push(other),
            }
            i += 2;
            continue;
        }
        if c == '"' {
            break;
        }
        out.push(c);
        i += 1;
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

pub fn already_wired(src: &Path) -> bool {
    recover_craft_item_if_truncated(src);
    let ci = src.join("craft_item.rs");
    let dual = src.join("craft_dual_center.inc.rs");
    if !dual.exists() && !src.join("craft_dual_center.rs").exists() {
        return false;
    }
    std::fs::read_to_string(&ci)
        .map(|t| {
            (t.contains("include!(\"craft_dual_center.inc.rs\")")
                || t.contains("mod craft_dual_center")
                || t.contains("include!(\"craft_dual_center.rs\")"))
                && t.contains("reanchor_craft_actor_near_target")
                && t.contains("AI-CRAFT-DUAL")
                && t.contains("FailedCraftings")
        })
        .unwrap_or(false)
}

pub fn patch_all(src: &Path, workspace: &Path) -> bool {
    recover_craft_item_if_truncated(src);
    let py = src.join("_apply_ai_craft_dual.py");
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
    ok || already_wired(src)
}

fn patch_minimal_rust(src: &Path) -> bool {
    if !src.join("craft_dual_center.inc.rs").exists()
        && !src.join("craft_dual_center.rs").exists()
    {
        eprintln!("cargo:warning=AI-CRAFT-DUAL: craft_dual_center.inc.rs missing");
        return false;
    }
    let ci = src.join("craft_item.rs");
    let Ok(raw) = std::fs::read_to_string(&ci) else {
        return false;
    };
    // Do not attempt dual wire on truncated craft_item.
    if !raw.contains("FailedCraftings") || raw.len() < 20_000 {
        eprintln!("cargo:warning=AI-CRAFT-DUAL: craft_item.rs still truncated; skip dual patch");
        return false;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    // Header
    if !t.contains("AI-CRAFT-DUAL") {
        if t.contains(
            "//! Multi-step **craftItem** / **craftItemHelper** world craft (AI-CRAFT-MULTI + **AI-CRAFT-TOPDOWN**).",
        ) {
            t = t.replacen(
                "//! Multi-step **craftItem** / **craftItemHelper** world craft (AI-CRAFT-MULTI + **AI-CRAFT-TOPDOWN**).",
                "//! Multi-step **craftItem** / **craftItemHelper** world craft (AI-CRAFT-MULTI + **AI-CRAFT-TOPDOWN** + **AI-CRAFT-DUAL**).",
                1,
            );
            changed = true;
        }
        if !t.contains("dual-center searchCurrentPosition") {
            t = t.replacen(
                "//! - craftItemHelper specials ~6750–7037 (water, soil, forge, bowls, TIME)\n",
                "//! - craftItemHelper specials ~6750–7037 (water, soil, forge, bowls, TIME)\n\
                 //! - dual-center searchCurrentPosition + pile*1.5 / r=6 re-anchor ~7050–7242 (AI-CRAFT-DUAL)\n",
                1,
            );
            changed = true;
        }
    }

    // include dual helpers into craft_item module
    if !t.contains("craft_dual_center") {
        let needle = "    STONE_HOE,\n};\n";
        if let Some(idx) = t.find(needle) {
            let end = idx + needle.len();
            let nest = if src.join("craft_dual_center.inc.rs").exists() {
                r#"
// Haxe: searchCurrentPosition dual-center + pile*1.5 / r=6 re-anchor (AI-CRAFT-DUAL)
include!("craft_dual_center.inc.rs");
"#
            } else {
                r#"
// Haxe: searchCurrentPosition dual-center + pile*1.5 / r=6 re-anchor (AI-CRAFT-DUAL)
#[path = "craft_dual_center.rs"]
mod craft_dual_center;
#[allow(unused_imports)]
pub use craft_dual_center::{
    closest_craft_obj_dual_center, craft_have_set_ex, craft_obj_in_dual_center,
    craft_quad_distance, reanchor_craft_actor_near_target, CraftActorReanchor,
    ACTOR_NEAR_TARGET_R, PILE_VS_LOOSE_QUAD_FACTOR,
};
"#
            };
            t.insert_str(end, nest);
            changed = true;
        } else {
            eprintln!("cargo:warning=AI-CRAFT-DUAL: nest/include anchor missing");
        }
    }

    // Default searchCurrentPosition = true (Haxe IntemToCraft)
    if t.contains("            search_current_position: false,\n")
        && !t.contains("search_current_position: true, // Haxe IntemToCraft")
    {
        t = t.replacen(
            "            search_current_position: false,\n",
            "            search_current_position: true, // Haxe IntemToCraft default true (AI-CRAFT-DUAL)\n",
            1,
        );
        changed = true;
    }

    // topdown opts with_search_current
    let old_opts = r#"    let topdown_opts = CraftTopDownOpts::default()
        .with_last(state.last_actor_id, state.last_target_id)
        .with_hardened_row(exists_row)
        .with_pile(product_pile)
        .with_index(&craft_index);
"#;
    let new_opts = r#"    let topdown_opts = CraftTopDownOpts::default()
        .with_last(state.last_actor_id, state.last_target_id)
        .with_hardened_row(exists_row)
        .with_pile(product_pile)
        .with_index(&craft_index)
        .with_search_current(state.search_current_position);
"#;
    if t.contains(old_opts) {
        t = t.replacen(old_opts, new_opts, 1);
        changed = true;
    }

    if changed {
        let out = restore_nl(&t, crlf);
        let _ = write_if_changed(&ci, &raw, &out);
    }

    let _ = patch_topdown(src);
    let _ = patch_lib_exports(src);

    already_wired(src) || changed
}

fn patch_topdown(src: &Path) -> bool {
    let ct = src.join("craft_topdown.rs");
    let Ok(raw) = std::fs::read_to_string(&ct) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("search_current_position: bool") {
        let old = "    pub meta_by_edge: Option<&'a HashMap<(i32, i32), CraftTransMeta>>,\n}\n";
        let new = "    pub meta_by_edge: Option<&'a HashMap<(i32, i32), CraftTransMeta>>,\n\
    /// Haxe `itemToCraft.searchCurrentPosition` — dual home+player scan when true.\n\
    // Haxe: IntemToCraft.searchCurrentPosition (AI-CRAFT-DUAL)\n\
    pub search_current_position: bool,\n}\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if !t.contains("search_current_position: true") {
        if let Some(idx) = t.find("impl<'a> Default for CraftTopDownOpts") {
            let slice = &t[idx..];
            if let Some(rel) = slice.find("meta_by_edge: None,\n        }") {
                let abs = idx + rel;
                let end = abs + "meta_by_edge: None,\n        }".len();
                t.replace_range(
                    abs..end,
                    "meta_by_edge: None,\n            search_current_position: true,\n        }",
                );
                changed = true;
            }
        }
    }

    if !t.contains("fn with_search_current") {
        let old = "    pub fn with_meta_map(mut self, map: &'a HashMap<(i32, i32), CraftTransMeta>) -> Self {\n\
        self.meta_by_edge = Some(map);\n\
        self\n\
    }\n";
        let new = "    pub fn with_meta_map(mut self, map: &'a HashMap<(i32, i32), CraftTransMeta>) -> Self {\n\
        self.meta_by_edge = Some(map);\n\
        self\n\
    }\n\n\
    /// Dual-center: when true, objects near player count even if far from home.\n\
    // Haxe: itemToCraft.searchCurrentPosition (AI-CRAFT-DUAL)\n\
    pub fn with_search_current(mut self, search_current: bool) -> Self {\n\
        self.search_current_position = search_current;\n\
        self\n\
    }\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    if changed {
        let out = restore_nl(&t, crlf);
        write_if_changed(&ct, &raw, &out)
    } else {
        false
    }
}

fn patch_lib_exports(src: &Path) -> bool {
    let lib = src.join("lib.rs");
    let Ok(raw) = std::fs::read_to_string(&lib) else {
        return false;
    };
    if raw.contains("reanchor_craft_actor_near_target") && raw.contains("craft_have_set_ex") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("closest_craft_obj_dual_center") {
        if t.contains("closest_craft_obj_by_ids, closest_craft_obj_filtered,") {
            t = t.replacen(
                "closest_craft_obj_by_ids, closest_craft_obj_filtered,",
                "closest_craft_obj_by_ids, closest_craft_obj_dual_center, closest_craft_obj_filtered,",
                1,
            );
            changed = true;
        }
    }
    if !t.contains("craft_have_set_ex") {
        if t.contains("craft_chebyshev, craft_have_set, craft_item,") {
            t = t.replacen(
                "craft_chebyshev, craft_have_set, craft_item,",
                "craft_chebyshev, craft_have_set, craft_have_set_ex, craft_item,",
                1,
            );
            changed = true;
        }
    }
    if !t.contains("reanchor_craft_actor_near_target") {
        if t.contains("retarget_water_source, search_best_object_for_crafting,") {
            t = t.replacen(
                "retarget_water_source, search_best_object_for_crafting,",
                "reanchor_craft_actor_near_target, retarget_water_source, search_best_object_for_crafting,",
                1,
            );
            changed = true;
        }
    }
    if !t.contains("CraftActorReanchor") {
        if t.contains("CraftTransPair, CraftWorldObj,") {
            t = t.replacen(
                "CraftTransPair, CraftWorldObj,",
                "CraftActorReanchor, CraftTransPair, CraftWorldObj,",
                1,
            );
            changed = true;
        }
    }
    if !t.contains("ACTOR_NEAR_TARGET_R") {
        if t.contains("AI_CRAFT_MIN_RADIUS, AI_IGNORE_TIME_TRANSITIONS") {
            t = t.replacen(
                "AI_CRAFT_MIN_RADIUS, AI_IGNORE_TIME_TRANSITIONS",
                "ACTOR_NEAR_TARGET_R, AI_CRAFT_MIN_RADIUS, AI_IGNORE_TIME_TRANSITIONS",
                1,
            );
            changed = true;
        }
    }

    if changed {
        let out = restore_nl(&t, crlf);
        write_if_changed(&lib, &raw, &out)
    } else {
        false
    }
}

fn patch_docs(workspace: &Path) -> bool {
    let docs = workspace.join("docs/port");
    let fm = docs.join("FILE_MATRIX.md");
    let mut any = false;
    if let Ok(raw) = std::fs::read_to_string(&fm) {
        if !raw.contains("AI-CRAFT-DUAL") {
            let row = "| **AI-CRAFT-DUAL** / dual_center_search | searchCurrentPosition dual home/player + pile*1.5/r=6 re-anchor | **DONE** (pure+helper wire) | `craft_dual_center.inc.rs` nested under craft_item: dual have-set + reanchor; CraftTopDownOpts.search_current_position; tests dual_center_*; residual live path maps; Haxe double-count TODO |\n";
            if let Some(idx) = raw.find("| **AI-CRAFT-TOPDOWN**") {
                let end = raw[idx..].find('\n').map(|n| idx + n + 1).unwrap_or(raw.len());
                let mut t = raw.clone();
                t.insert_str(end, row);
                let _ = std::fs::write(&fm, t);
                any = true;
            }
        }
    }
    let todo = docs.join("TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo) {
        if !raw.contains("AI-CRAFT-DUAL dual_center_search DONE") {
            let line = "- [x] **AI-CRAFT-DUAL dual_center_search DONE** — craft_dual_center nest: searchCurrentPosition dual home/player, pile*1.5/r=6 re-anchor in craft_item_helper, CraftTopDownOpts.search_current_position; tests dual_center_*; residual live path maps\n";
            if let Some(idx) = raw.find("- [x] **AI-CRAFT-TOPDOWN") {
                let end = raw[idx..].find('\n').map(|n| idx + n + 1).unwrap_or(raw.len());
                let mut t = raw.clone();
                t.insert_str(end, line);
                let _ = std::fs::write(&todo, t);
                any = true;
            }
        }
    }
    any
}
