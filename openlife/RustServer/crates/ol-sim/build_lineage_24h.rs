//! LINEAGE-24H / starving_window — build-time wire.
//!
//! Idempotent patches:
//! - death path: `note_death_reason_at(sim_time, reason, age)` + `stamp_lineage_death`
//! - eat/horse paths: `get_starving_food_factor_at(state.sim_time)`
//! - python apply for docs when present
//!
//! Pure `world_food_stats` already ships with 24h window helpers.

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

fn replace_all(hay: &mut String, old: &str, new: &str) -> bool {
    if !hay.contains(old) {
        return false;
    }
    *hay = hay.replace(old, new);
    true
}

fn replace_once(hay: &mut String, old: &str, new: &str) -> bool {
    if let Some(i) = hay.find(old) {
        hay.replace_range(i..i + old.len(), new);
        true
    } else {
        false
    }
}

/// True when death stamps + live windowed starving factor are wired.
pub fn lineage_24h_wired(lib: &str, use_trans: &str, wfs: &str) -> bool {
    wfs.contains("LINEAGE-24H")
        && wfs.contains("note_death_reason_at")
        && wfs.contains("get_starving_food_factor_at")
        && wfs.contains("LAST_DAY_YEARS")
        && lib.contains("note_death_reason_at(state.sim_time")
        && lib.contains("stamp_lineage_death")
        && (lib.contains("get_starving_food_factor_at(state.sim_time)")
            || use_trans.contains("get_starving_food_factor_at(state.sim_time)"))
}

pub fn patch_lineage_24h(manifest: &Path, src: &Path, workspace: &Path) -> bool {
    // Prefer python apply (docs + multi-file).
    let py = src.join("_apply_lineage_24h.py");
    if py.exists() {
        let _ = Command::new("python")
            .arg(&py)
            .current_dir(manifest)
            .status()
            .or_else(|_| {
                Command::new("python3")
                    .arg(&py)
                    .current_dir(manifest)
                    .status()
            });
    }

    let lib_path = src.join("lib.rs");
    let use_path = src.join("use_transition.rs");
    let wfs_path = src.join("world_food_stats.rs");
    let Ok(lib_raw) = std::fs::read_to_string(&lib_path) else {
        return false;
    };
    let use_raw = std::fs::read_to_string(&use_path).unwrap_or_default();
    let wfs_raw = std::fs::read_to_string(&wfs_path).unwrap_or_default();
    if lineage_24h_wired(&lib_raw, &use_raw, &wfs_raw) {
        patch_docs(workspace);
        return true;
    }

    let mut changed = false;

    // lib.rs death path
    {
        let crlf = lib_raw.contains("\r\n");
        let mut t = normalize_nl(&lib_raw);
        changed |= replace_once(
            &mut t,
            "        // WORLD-FOOD-FACTOR: feed getStarvingFoodFactor session counters\n\
        // Haxe: Lineage.reasonKilledLastDay['reason_age'|'reason_hunger']\n\
        state.world_food.note_death_reason(&reason);\n",
            "        // LINEAGE-24H: stamp last-day death window + lineage deathTime/reason\n\
        // Haxe: Lineage.reasonKilledLastDay + GenerateLineageStatistics L353\n\
        state.world_food.note_death_reason_at(state.sim_time, &reason, age);\n\
        state.social.stamp_lineage_death(p_id, state.sim_time, &reason, age);\n",
        );
        changed |= replace_all(
            &mut t,
            "state.world_food.get_starving_food_factor()",
            "state.world_food.get_starving_food_factor_at(state.sim_time)",
        );
        if changed {
            let _ = std::fs::write(&lib_path, restore_nl(&t, crlf));
        }
    }

    // use_transition.rs horse eat
    if let Ok(use_raw2) = std::fs::read_to_string(&use_path) {
        let crlf = use_raw2.contains("\r\n");
        let mut t = normalize_nl(&use_raw2);
        if replace_all(
            &mut t,
            "state.world_food.get_starving_food_factor()",
            "state.world_food.get_starving_food_factor_at(state.sim_time)",
        ) {
            let _ = std::fs::write(&use_path, restore_nl(&t, crlf));
            changed = true;
        }
    }

    patch_docs(workspace);

    let lib_after = std::fs::read_to_string(&lib_path).unwrap_or_default();
    let use_after = std::fs::read_to_string(&use_path).unwrap_or_default();
    let wfs_after = std::fs::read_to_string(&wfs_path).unwrap_or_default();
    lineage_24h_wired(&lib_after, &use_after, &wfs_after) || changed
}

fn patch_docs(workspace: &Path) {
    let todo = workspace.join("docs/port/TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo) {
        if !raw.contains("LINEAGE-24H starving_window") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            let _ = replace_once(
                &mut t,
                "Residual: ObjectCounts autosave share wire; lineage last-day reason window; ol-web HTML table hook",
                "Residual: ObjectCounts autosave share wire; ol-web HTML table hook. **LINEAGE-24H DONE** last-day reason window",
            );
            let marker = "## Changelog (port docs)\n\n| Date | Change |\n|------|--------|\n";
            let row = "## Changelog (port docs)\n\n| Date | Change |\n|------|--------|\n\
| 2026-07-29 | **LINEAGE-24H starving_window**: `reasonKilledLastDay` 24h (`yearsSinceDeath < 1440`); death stamps + kid remap; `get_starving_food_factor_at`; lineage death session fields; tests `world_food_stats::last_day_*` |\n";
            let _ = replace_once(&mut t, marker, row);
            if let Some(i) = t.find("Last updated:") {
                if let Some(end) = t[i..].find('\n') {
                    let line = "Last updated: **2026-07-29** (LINEAGE-24H starving_window)";
                    t.replace_range(i..i + end, line);
                }
            }
            let _ = std::fs::write(&todo, restore_nl(&t, crlf));
        }
    }

    let matrix = workspace.join("docs/port/FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&matrix) {
        if !raw.contains("LINEAGE-24H DONE") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            let _ = replace_once(
                &mut t,
                "| S-LIN | `Lineage.hx` | 530 | `lineage_persist`, relations, **`prestige`** | PARTIAL → **CLASS-BONI DONE** | OLN1; PrestigeClass enum + PrestigeClasses names + isNobleOrMore + birth calculatePrestigeClass; archive/delete policies incomplete |",
                "| S-LIN | `Lineage.hx` | 530 | `lineage_persist`, relations, **`prestige`**, **world_food_stats LINEAGE-24H** | PARTIAL → **CLASS-BONI + LINEAGE-24H DONE** | OLN1; PrestigeClass; **reasonKilledLastDay 24h** + death session stamps; archive/delete residual |",
            );
            let _ = replace_all(
                &mut t,
                "lineage last-day window",
                "**LINEAGE-24H DONE**",
            );
            let _ = replace_all(&mut t, "lineage 24h", "**LINEAGE-24H DONE**");
            let _ = std::fs::write(&matrix, restore_nl(&t, crlf));
        }
    }

    let call = workspace.join("docs/port/CALL_INDEX.md");
    if let Ok(raw) = std::fs::read_to_string(&call) {
        if !raw.contains("LINEAGE-24H / starving_window") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            t.push_str(
                "\n## LINEAGE-24H / starving_window (2026-07-29)\n\n\
| Symbol | File | Role |\n\
|--------|------|------|\n\
| `Lineage.GenerateLineageStatistics` / `reasonKilledLastDay` | `server/Lineage.hx` | last-day death reason map |\n\
| `WorldMap.getStarvingFoodFactor` | `server/WorldMap.hx` | hunger/age last-day ratio |\n\
| `note_death_reason_at` / `get_starving_food_factor_at` | `ol-sim/world_food_stats.rs` | 24h stamps + factor |\n\
| `stamp_lineage_death` | `ol-sim/social.rs` | session deathTime/reason |\n\n",
            );
            let _ = std::fs::write(&call, restore_nl(&t, crlf));
        }
    }

    let queue = workspace.join("docs/port/QUEUE.md");
    if let Ok(raw) = std::fs::read_to_string(&queue) {
        if raw.contains("`LINEAGE-24H`") && !raw.contains("~~`LINEAGE-24H`~~") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            let _ = replace_once(
                &mut t,
                "| `LINEAGE-24H` | starving_window | Lineage last-day reason window residual |",
                "| ~~`LINEAGE-24H`~~ | starving_window | **DONE** last-day reason window |",
            );
            let _ = replace_once(
                &mut t,
                "**COMBAT-FEVER-BLEED** DONE",
                "**LINEAGE-24H** DONE · **COMBAT-FEVER-BLEED** DONE",
            );
            let _ = std::fs::write(&queue, restore_nl(&t, crlf));
        }
    }
}
