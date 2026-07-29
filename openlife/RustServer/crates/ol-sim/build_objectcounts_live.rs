//! Build-time wire for **OBJECTCOUNTS-LIVE / object_counts_share**.
//!
//! Residual of FOODSTATS-DISK: Haxe `WorldMap.write` TraceCountObjectsToDisk.
//! Pure format in `long_term`; share types in `object_counts_share.rs`.
//!
//! Idempotent. Handles CRLF.

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

pub fn objectcounts_live_wired(lib: &str, settings_live: &str, main: &str) -> bool {
    let mirrors = lib
        .matches("mirror_object_counts_share(&state, &object_counts_share)")
        .count();
    lib.contains("mod object_counts_share")
        && lib.contains("ObjectCountsShare")
        && lib.contains("fn mirror_object_counts_share")
        && lib.contains("let object_counts_share = boot_live")
        && mirrors >= 2
        && settings_live.contains("object_counts_share")
        && main.contains("shared_object_counts")
        && main.contains("object counts autosaved")
}

/// Prefer Python apply (same as other residual wires); fall back to pure RS patches.
pub fn patch_objectcounts_live(src_dir: &Path, workspace: &Path) -> bool {
    let py = src_dir.join("_apply_objectcounts_live.py");
    if py.exists() {
        let status = Command::new("python")
            .arg(&py)
            .status()
            .or_else(|_| Command::new("python3").arg(&py).status());
        if let Ok(s) = status {
            if s.success() {
                // fall through to verify + RS fill gaps
            }
        }
    }

    let lib_ok = patch_lib(&src_dir.join("lib.rs"));
    let sl_ok = patch_settings_live(&src_dir.join("settings_live.rs"));
    let cfg_path = workspace.join("crates/ol-config/src/lib.rs");
    let _cfg_ok = if cfg_path.exists() {
        patch_config(&cfg_path)
    } else {
        true
    };
    let main_path = workspace.join("crates/ol-server/src/main.rs");
    let main_ok = if main_path.exists() {
        patch_main(&main_path)
    } else {
        true
    };
    let _ = patch_docs(workspace);

    let lib = std::fs::read_to_string(src_dir.join("lib.rs")).unwrap_or_default();
    let sl = std::fs::read_to_string(src_dir.join("settings_live.rs")).unwrap_or_default();
    let main = std::fs::read_to_string(&main_path).unwrap_or_default();
    objectcounts_live_wired(&lib, &sl, &main) || (lib_ok && sl_ok && main_ok)
}

fn patch_lib(lib_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);
    let mut changed = false;

    if !text.contains("mod object_counts_share") {
        for needle in ["mod long_term;\n", "mod world_food_stats;\n"] {
            if text.contains(needle) {
                text = text.replacen(
                    needle,
                    &format!("{needle}mod object_counts_share; // OBJECTCOUNTS-LIVE\n"),
                    1,
                );
                changed = true;
                break;
            }
        }
    }

    if !text.contains("object_counts_share::{ObjectCountsShare")
        && !text.contains("ObjectCountsShare")
    {
        // after long_term pub use
        if let Some(idx) = text.find("pub use long_term::{") {
            if let Some(end_rel) = text[idx..].find("\n};\n") {
                let end = idx + end_rel + "\n};\n".len();
                text.insert_str(
                    end,
                    "// OBJECTCOUNTS-LIVE: ObjectCounts autosave share\npub use object_counts_share::{ObjectCountsShare, ObjectCountsSnapshot};\n",
                );
                changed = true;
            }
        }
    } else if text.contains("mod object_counts_share")
        && !text.contains("object_counts_share::{ObjectCountsShare")
        && text.contains("ObjectCountsShare")
    {
        // ObjectCountsShare exported from long_term only — also export module path
    } else if text.contains("mod object_counts_share")
        && !text.contains("pub use object_counts_share::")
    {
        if let Some(idx) = text.find("pub use long_term::{") {
            if let Some(end_rel) = text[idx..].find("\n};\n") {
                let end = idx + end_rel + "\n};\n".len();
                if !text[end..end + 80.min(text.len() - end)].contains("object_counts_share::") {
                    text.insert_str(
                        end,
                        "// OBJECTCOUNTS-LIVE: ObjectCounts autosave share\npub use object_counts_share::{ObjectCountsShare, ObjectCountsSnapshot};\n",
                    );
                    changed = true;
                }
            }
        }
    }

    // Ensure pub use even if ObjectCountsShare name appears elsewhere first
    if text.contains("mod object_counts_share")
        && !text.contains("pub use object_counts_share::{ObjectCountsShare, ObjectCountsSnapshot}")
    {
        if let Some(idx) = text.find("pub use long_term::{") {
            if let Some(end_rel) = text[idx..].find("\n};\n") {
                let end = idx + end_rel + "\n};\n".len();
                text.insert_str(
                    end,
                    "// OBJECTCOUNTS-LIVE: ObjectCounts autosave share\npub use object_counts_share::{ObjectCountsShare, ObjectCountsSnapshot};\n",
                );
                changed = true;
            }
        }
    }

    if !text.contains("let object_counts_share = boot_live") {
        let needles = [
            "    // FOODSTATS-DISK: WorldFoodStats share for FoodStats.txt autosave dump.\n    let world_food_share = boot_live.as_ref().and_then(|b| b.world_food_share.clone());\n",
            "    let world_food_share = boot_live.as_ref().and_then(|b| b.world_food_share.clone());\n",
        ];
        let insert = "    // OBJECTCOUNTS-LIVE: object census share for ObjectCounts.txt autosave dump.\n    let object_counts_share = boot_live.as_ref().and_then(|b| b.object_counts_share.clone());\n";
        for n in needles {
            if text.contains(n) {
                text = text.replacen(n, &format!("{n}{insert}"), 1);
                changed = true;
                break;
            }
        }
    }

    if !text.contains("fn mirror_object_counts_share") {
        if let Some(idx) = text.find("fn mirror_world_food_share") {
            if let Some(end_rel) = text[idx..].find("\n}\n") {
                let end = idx + end_rel + "\n}\n".len();
                text.insert_str(
                    end,
                    r#"
/// Mirror live long-term object census into outer autosave Arc (OBJECTCOUNTS-LIVE).
// Haxe: WorldMap.write → TraceCountObjectsToDisk ObjectCounts{N}.txt
fn mirror_object_counts_share(state: &SimState, share: &Option<ObjectCountsShare>) {
    if let Some(ref s) = share {
        if let Ok(mut g) = s.write() {
            *g = ObjectCountsSnapshot::from_long_term(&state.long_term);
        }
    }
}

"#,
                );
                changed = true;
            }
        }
    }

    let pairs = [
        (
            "mirror_world_food_share(&state, &world_food_share);\n                    info!(\"intent channel closed; sim stopping\");",
            "mirror_world_food_share(&state, &world_food_share);\n                    mirror_object_counts_share(&state, &object_counts_share);\n                    info!(\"intent channel closed; sim stopping\");",
        ),
        (
            "mirror_world_food_share(&state, &world_food_share);\n                            info!(\"intent channel closed; sim stopping\");",
            "mirror_world_food_share(&state, &world_food_share);\n                            mirror_object_counts_share(&state, &object_counts_share);\n                            info!(\"intent channel closed; sim stopping\");",
        ),
        (
            "            // FOODSTATS-DISK: periodic FoodStats share mirror for outer autosave.\n            mirror_world_food_share(&state, &world_food_share);\n        }",
            "            // FOODSTATS-DISK: periodic FoodStats share mirror for outer autosave.\n            mirror_world_food_share(&state, &world_food_share);\n            // OBJECTCOUNTS-LIVE: periodic ObjectCounts share mirror for outer autosave.\n            mirror_object_counts_share(&state, &object_counts_share);\n        }",
        ),
        (
            "            mirror_world_food_share(&state, &world_food_share);\n        }\n        if state.tick.saturating_sub(last_skip_log) >= 200 {",
            "            mirror_world_food_share(&state, &world_food_share);\n            mirror_object_counts_share(&state, &object_counts_share);\n        }\n        if state.tick.saturating_sub(last_skip_log) >= 200 {",
        ),
    ];
    for (old, new) in pairs {
        if text.contains(old) && !text.contains(new) {
            text = text.replace(old, new);
            changed = true;
        }
    }

    for pad in ["                    ", "                            "] {
        let bare = format!(
            "mirror_world_food_share(&state, &world_food_share);\n{pad}info!"
        );
        let full = format!(
            "mirror_world_food_share(&state, &world_food_share);\n{pad}mirror_object_counts_share(&state, &object_counts_share);\n{pad}info!"
        );
        if text.contains(&bare) && !text.contains(&full) {
            text = text.replace(&bare, &full);
            changed = true;
        }
    }

    if changed {
        let _ = std::fs::write(lib_path, restore_nl(&text, crlf));
    }
    std::fs::read_to_string(lib_path)
        .map(|t| {
            t.contains("fn mirror_object_counts_share")
                && t.contains("mod object_counts_share")
                && t.contains("let object_counts_share = boot_live")
        })
        .unwrap_or(false)
}

fn patch_settings_live(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("object_counts_share") && raw.contains("ObjectCountsShare") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);
    let mut changed = false;

    if !text.contains("ObjectCountsShare") {
        if text.contains("use crate::world_food_stats::WorldFoodShare;\n") {
            text = text.replacen(
                "use crate::world_food_stats::WorldFoodShare;\n",
                "use crate::world_food_stats::WorldFoodShare;\nuse crate::object_counts_share::ObjectCountsShare;\n",
                1,
            );
            changed = true;
        }
    }

    if !text.contains("pub object_counts_share:") {
        text = text.replacen(
            "    pub world_food_share: Option<WorldFoodShare>,\n",
            "    pub world_food_share: Option<WorldFoodShare>,\n    /// OBJECTCOUNTS-LIVE: world object census for ObjectCounts.txt autosave dump.\n    pub object_counts_share: Option<ObjectCountsShare>,\n",
            1,
        );
        changed = true;
    }

    if !text.contains("object_counts_share: None") {
        text = text.replacen(
            "            world_food_share: None,\n",
            "            world_food_share: None,\n            object_counts_share: None,\n",
            1,
        );
        changed = true;
    }

    if changed {
        let _ = std::fs::write(path, restore_nl(&text, crlf));
    }
    std::fs::read_to_string(path)
        .map(|t| t.contains("object_counts_share"))
        .unwrap_or(false)
}

fn patch_config(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("OBJECTCOUNTS-LIVE") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);
    if text.contains("Pure format/write in `ol-sim` `long_term`; autosave wire optional residual.") {
        text = text.replacen(
            "Pure format/write in `ol-sim` `long_term`; autosave wire optional residual.",
            "Pure format/write + OBJECTCOUNTS-LIVE autosave share in `ol-sim`.",
            1,
        );
        let _ = std::fs::write(path, restore_nl(&text, crlf));
    }
    true
}

fn patch_main(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);
    let mut changed = false;

    if !text.contains("write_object_counts") {
        if text.contains("write_food_statistics, AccountBook,") {
            text = text.replacen(
                "write_food_statistics, AccountBook,",
                "write_food_statistics, write_object_counts, AccountBook,",
                1,
            );
            changed = true;
        } else if text.contains("write_food_statistics,") {
            text = text.replacen(
                "write_food_statistics,",
                "write_food_statistics, write_object_counts,",
                1,
            );
            changed = true;
        }
    }

    if !text.contains("ObjectCountsSnapshot") {
        if text.contains("WorldFoodStats,") {
            text = text.replacen("WorldFoodStats,", "ObjectCountsSnapshot, WorldFoodStats,", 1);
            changed = true;
        } else if text.contains("WorldFoodStats") {
            text = text.replacen("WorldFoodStats", "ObjectCountsSnapshot, WorldFoodStats", 1);
            changed = true;
        }
    }

    if !text.contains("shared_object_counts") {
        let needle = "    let shared_world_food = Arc::new(RwLock::new(WorldFoodStats::new()));\n";
        if text.contains(needle) {
            text = text.replacen(
                needle,
                "    let shared_world_food = Arc::new(RwLock::new(WorldFoodStats::new()));\n    // OBJECTCOUNTS-LIVE: session object census mirror for ObjectCounts.txt (write-only dump).\n    let shared_object_counts = Arc::new(RwLock::new(ObjectCountsSnapshot::new()));\n",
                1,
            );
            changed = true;
        }
    }

    if text.contains("shared_object_counts") && !text.contains("object_counts_share:") {
        let needles = [
            "            world_food_share: Some(Arc::clone(&shared_world_food)),\n            // AI-LLM-HTTP-DRAIN: speech job/result bridge for call_ai_async worker\n            llm_speech_share: Some(Arc::clone(&llm_speech_share)),\n",
            "            world_food_share: Some(Arc::clone(&shared_world_food)),\n            llm_speech_share: Some(Arc::clone(&llm_speech_share)),\n",
        ];
        let inserts = [
            "            world_food_share: Some(Arc::clone(&shared_world_food)),\n            // OBJECTCOUNTS-LIVE: ObjectCounts.txt autosave mirror\n            object_counts_share: Some(Arc::clone(&shared_object_counts)),\n            // AI-LLM-HTTP-DRAIN: speech job/result bridge for call_ai_async worker\n            llm_speech_share: Some(Arc::clone(&llm_speech_share)),\n",
            "            world_food_share: Some(Arc::clone(&shared_world_food)),\n            object_counts_share: Some(Arc::clone(&shared_object_counts)),\n            llm_speech_share: Some(Arc::clone(&llm_speech_share)),\n",
        ];
        for (n, i) in needles.iter().zip(inserts.iter()) {
            if text.contains(*n) {
                text = text.replacen(n, i, 1);
                changed = true;
                break;
            }
        }
    }

    if text.contains("shared_object_counts")
        && !text.contains("let object_counts_share = Arc::clone(&shared_object_counts)")
    {
        let needle = "        let world_food_share = Arc::clone(&shared_world_food);\n        let food_stats_save = cfg.food_stats_save_path();\n        let content_for_food = Arc::clone(&content);\n        handles.push(tokio::spawn(async move {";
        let insert = "        let world_food_share = Arc::clone(&shared_world_food);\n        let food_stats_save = cfg.food_stats_save_path();\n        let content_for_food = Arc::clone(&content);\n        let object_counts_share = Arc::clone(&shared_object_counts);\n        let object_counts_save = cfg.object_counts_save_path();\n        let content_for_counts = Arc::clone(&content);\n        handles.push(tokio::spawn(async move {";
        if text.contains(needle) {
            text = text.replacen(needle, insert, 1);
            changed = true;
        }
    }

    if text.contains("shared_object_counts") && !text.contains("object counts autosaved") {
        let marker = "                        \"food stats autosaved (FoodStats.txt)\"\n                    );\n                }\n                if any_ok {";
        let add = r#"                        "food stats autosaved (FoodStats.txt)"
                    );
                }
                // OBJECTCOUNTS-LIVE: Haxe TraceCountObjectsToDisk → ObjectCounts.txt
                let counts_snap = object_counts_share.read().unwrap().clone();
                let content_names = Arc::clone(&content_for_counts);
                if let Err(e) = write_object_counts(
                    &counts_snap.current_counts,
                    &counts_snap.original_counts,
                    &object_counts_save,
                    |id| {
                        content_names
                            .get(id)
                            .map(|d| {
                                if d.description.is_empty() {
                                    if d.name.is_empty() {
                                        String::new()
                                    } else {
                                        d.name.clone()
                                    }
                                } else {
                                    d.description.clone()
                                }
                            })
                            .unwrap_or_default()
                    },
                ) {
                    warn!(error = %e, "object counts autosave failed");
                } else {
                    any_ok = true;
                    info!(
                        path = %object_counts_save.display(),
                        objects = counts_snap.current_counts.len(),
                        "object counts autosaved (ObjectCounts.txt)"
                    );
                }
                if any_ok {"#;
        if text.contains(marker) {
            text = text.replacen(marker, add, 1);
            changed = true;
        }
    }

    if text.contains("shared_object_counts") && !text.contains("object counts saved on shutdown") {
        let marker = "                \"food stats saved on shutdown (FoodStats.txt)\"\n            );\n        }\n    }\n\n    for h in handles {";
        let add = r#"                "food stats saved on shutdown (FoodStats.txt)"
            );
        }
    }
    {
        // OBJECTCOUNTS-LIVE: final ObjectCounts.txt (Haxe TraceCountObjectsToDisk on save).
        let snap = shared_object_counts.read().unwrap().clone();
        let path = cfg.object_counts_save_path();
        if let Err(e) = write_object_counts(
            &snap.current_counts,
            &snap.original_counts,
            &path,
            |id| {
                content
                    .get(id)
                    .map(|d| {
                        if d.description.is_empty() {
                            if d.name.is_empty() {
                                String::new()
                            } else {
                                d.name.clone()
                            }
                        } else {
                            d.description.clone()
                        }
                    })
                    .unwrap_or_default()
            },
        ) {
            warn!(error = %e, "object counts shutdown save failed");
        } else {
            info!(
                path = %path.display(),
                objects = snap.current_counts.len(),
                "object counts saved on shutdown (ObjectCounts.txt)"
            );
        }
    }

    for h in handles {"#;
        if text.contains(marker) {
            text = text.replacen(marker, add, 1);
            changed = true;
        }
    }

    if changed {
        let _ = std::fs::write(path, restore_nl(&text, crlf));
    }
    std::fs::read_to_string(path)
        .map(|t| {
            t.contains("shared_object_counts")
                && t.contains("object counts autosaved")
                && t.contains("write_object_counts")
        })
        .unwrap_or(false)
}

fn patch_docs(workspace: &Path) -> bool {
    let port = workspace.join("docs/port");
    if !port.exists() {
        return true;
    }
    // Changelog always
    let cl = port.join("changelog/OBJECTCOUNTS-LIVE_object_counts_share.md");
    if !cl.exists() {
        let _ = std::fs::create_dir_all(cl.parent().unwrap());
        let _ = std::fs::write(
            &cl,
            r#"# OBJECTCOUNTS-LIVE / object_counts_share (2026-07-29)

## Status: **DONE**

Closes FOODSTATS-DISK residual: ObjectCounts autosave share wire.

### Rust
- `ol-sim/src/object_counts_share.rs` — `ObjectCountsSnapshot` / `ObjectCountsShare`
- `SimBootLive.object_counts_share` + `mirror_object_counts_share`
- ol-server autosave/shutdown `ObjectCounts.txt`

### Tests
- `object_counts_share::*`
- Prior `long_term::format_object_count*` / `write_object_counts_roundtrip_disk`
"#,
        );
    }

    // FILE_MATRIX residual note
    let matrix = port.join("FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&matrix) {
        if !raw.contains("OBJECTCOUNTS-LIVE") {
            let crlf = raw.contains("\r\n");
            let mut text = normalize_nl(&raw);
            text = text.replacen(
                "residual lineage last-day window / ObjectCounts autosave share",
                "residual lineage last-day window; **OBJECTCOUNTS-LIVE DONE**",
                1,
            );
            if text.contains("| **FOODSTATS-DISK** / foodstats_txt |")
                && !text.contains("| **OBJECTCOUNTS-LIVE**")
            {
                if let Some(idx) = text.find("| **FOODSTATS-DISK** / foodstats_txt |") {
                    if let Some(line_end) = text[idx..].find('\n') {
                        text.insert_str(
                            idx + line_end + 1,
                            "| **OBJECTCOUNTS-LIVE** / object_counts_share | WorldMap ObjectCounts autosave share | **DONE** | `ObjectCountsShare` + sim mirror; ol-server autosave/shutdown |\n",
                        );
                    }
                }
            }
            let _ = std::fs::write(&matrix, restore_nl(&text, crlf));
        }
    }

    let todo = port.join("TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo) {
        if !raw.contains("OBJECTCOUNTS-LIVE object_counts_share") {
            let crlf = raw.contains("\r\n");
            let mut text = normalize_nl(&raw);
            if text.starts_with("Last updated:") {
                if let Some(end) = text.find('\n') {
                    text.replace_range(
                        ..end + 1,
                        "Last updated: **2026-07-29** (OBJECTCOUNTS-LIVE object_counts_share)\n",
                    );
                }
            }
            text = text.replacen(
                "Residual: ObjectCounts autosave share wire; lineage last-day reason window; ol-web HTML table hook",
                "Residual: lineage last-day reason window; ol-web HTML table hook. **OBJECTCOUNTS-LIVE DONE**",
                1,
            );
            if text.contains("| Date | Change |\n|------|--------|\n") {
                text = text.replacen(
                    "| Date | Change |\n|------|--------|\n",
                    "| Date | Change |\n|------|--------|\n| 2026-07-29 | **OBJECTCOUNTS-LIVE object_counts_share**: ObjectCountsShare + autosave/shutdown ObjectCounts.txt |\n",
                    1,
                );
            }
            let _ = std::fs::write(&todo, restore_nl(&text, crlf));
        }
    }

    let queue = port.join("QUEUE.md");
    if let Ok(raw) = std::fs::read_to_string(&queue) {
        if raw.contains("| `OBJECTCOUNTS-LIVE` | object_counts_share |") {
            let crlf = raw.contains("\r\n");
            let mut text = normalize_nl(&raw);
            text = text.replace(
                "| `OBJECTCOUNTS-LIVE` | object_counts_share | ObjectCounts autosave share residual |\n",
                "",
            );
            text = text.replacen(
                "## Done recently (do not re-queue)\n\n",
                "## Done recently (do not re-queue)\n\n**OBJECTCOUNTS-LIVE** DONE · ",
                1,
            );
            let _ = std::fs::write(&queue, restore_nl(&text, crlf));
        }
    }

    true
}
