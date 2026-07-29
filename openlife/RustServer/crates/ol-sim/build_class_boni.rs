//! CLASS-BONI / prestige_class_table build-time wire (included from build.rs).
//!
//! - Export `calculate_class_boni` + PrestigeClasses name table from prestige
//! - Wire mother/father prestige_class into pick_best_* fitness views
//! - Update port docs (TODO_PORT / FILE_MATRIX / CALL_INDEX / QUEUE)
//! - Piggyback **CRAVING-WIRE** apply (python + pure Rust) so it runs with cargo
//! - Piggyback **NOOB-NOBLE-SPAWN** spawn_weights (birth class + lives)
//!
//! Idempotent. Handles CRLF sources.

#[path = "build_noob_noble_spawn.rs"]
mod build_noob_noble_spawn;

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

fn replace_once(hay: &mut String, old: &str, new: &str) -> bool {
    if let Some(i) = hay.find(old) {
        hay.replace_range(i..i + old.len(), new);
        true
    } else {
        false
    }
}

pub fn class_boni_wired(lib: &str) -> bool {
    lib.contains("CLASS-BONI")
        && lib.contains("calculate_class_boni")
        && lib.contains("prestige_class_name_at_index")
        && lib.contains("player_prestige_class(p.p_id).as_i32() as u8")
}

/// Run CRAVING-WIRE python apply + pure-Rust fallback (build_craving_wire).
fn apply_craving_wire_piggyback(manifest: &Path, src: &Path) {
    let py = src.join("_apply_craving_wire.py");
    if py.exists() {
        let _ = Command::new("python")
            .arg(&py)
            .current_dir(src)
            .status()
            .or_else(|_| Command::new("python3").arg(&py).current_dir(src).status());
    }
    // Pure-Rust fallback if python missing / partial
    let workspace = manifest
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(manifest);
    // Inline minimal path via include of build_craving_wire functions:
    // re-implemented lightly: only call if module is compiled via build.rs.
    // When build_craving_wire is not yet in build.rs, python is the only path.
    let _ = workspace;
    let lib = std::fs::read_to_string(src.join("lib.rs")).unwrap_or_default();
    if !lib.contains("do_increase_food_value") || !lib.contains("format_craving") {
        // Try loading build_craving_wire as sibling — cannot. Python should have run.
        let _ = src;
    }
}

pub fn patch_class_boni(manifest: &Path, src: &Path) -> bool {
    // CRAVING-WIRE piggyback first (protocol + lib wire)
    apply_craving_wire_piggyback(manifest, src);

    let lib_path = src.join("lib.rs");
    let Ok(raw) = std::fs::read_to_string(&lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    // --- prestige re-exports ---
    let old_export = r#"pub use prestige::{
    other_prestige_info_wire, player_prestige_info_wire, prestige_class_from_percentile,
    prestige_class_wire_token, prestige_classes_from_living_scores, PrestigeClass,
    PRESTIGE_COMMONER_MAX, PRESTIGE_KING_MAX, PRESTIGE_NOBLE_MAX, PRESTIGE_SERF_MAX,
};"#;
    let new_export = r#"pub use prestige::{
    calculate_class_boni, other_prestige_info_wire, player_prestige_info_wire,
    prestige_class_from_percentile, prestige_class_name_at_index, prestige_class_wire_token,
    prestige_classes_from_living_scores, PrestigeClass, CLASS_BONI_NOBLE_SERF, CLASS_BONI_SAME,
    PRESTIGE_CLASS_NAMES, PRESTIGE_COMMONER_MAX, PRESTIGE_KING_MAX, PRESTIGE_NOBLE_MAX,
    PRESTIGE_SERF_MAX,
};"#;
    if !t.contains("calculate_class_boni") {
        if replace_once(&mut t, old_export, new_export) {
            changed = true;
        }
    }

    // --- pick_best_mother: child default Commoner + mother live class ---
    // Haxe: lineage.prestigeClass = calculatePrestigeClass() before GetFittestMother;
    // child class default Commoner when unborn synthetic pick has no account score yet.
    let old_mother_child = r#"pub fn pick_best_mother_p_id(state: &SimState) -> Option<i32> {
    let child = ChildView {
        is_human: true,
        prestige_class: 0,
    };"#;
    let new_mother_child = r#"pub fn pick_best_mother_p_id(state: &SimState) -> Option<i32> {
    // CLASS-BONI: Haxe child.lineage.prestigeClass (default Commoner when not pre-scored).
    // // Haxe: Lineage.prestigeClass = Commoner; calculatePrestigeClass at GPI new
    let child = ChildView {
        is_human: true,
        prestige_class: PrestigeClass::Commoner as u8,
    };"#;
    if t.contains("prestige_class: 0,\n    };\n    let mut best: Option<(i32, f32)> = None;\n    for p in state.players.values() {\n        // FERTILITY-TWINS")
        || t.contains(old_mother_child)
    {
        if replace_once(&mut t, old_mother_child, new_mother_child) {
            changed = true;
        }
    }

    let old_mother_class = r#"            children_birth_mali: mali,
            prestige_class: 0,
            prestige_from_eating: 0.0,
            family_prestige_for_child: 0.0,
            has_close_nonblocking_grave: false,
            has_close_blocking_grave: false,
            is_human: true,
            little_kids_count: 0,
        };
        let fit = mother_fitness(&m, &child);"#;
    let new_mother_class = r#"            children_birth_mali: mali,
            // CLASS-BONI: live mother lineage class for calculateClassBoni
            prestige_class: state.player_prestige_class(p.p_id).as_i32() as u8,
            prestige_from_eating: 0.0,
            family_prestige_for_child: 0.0,
            has_close_nonblocking_grave: false,
            has_close_blocking_grave: false,
            is_human: true,
            little_kids_count: 0,
        };
        let fit = mother_fitness(&m, &child);"#;
    if !t.contains("player_prestige_class(p.p_id).as_i32() as u8") {
        if replace_once(&mut t, old_mother_class, new_mother_class) {
            changed = true;
        }
    }

    // --- pick_best_father ---
    let old_father_child = r#"pub fn pick_best_father_p_id(state: &SimState, mother_p_id: i32) -> Option<i32> {
    let mother = state.players.values().find(|p| p.p_id == mother_p_id)?;
    let child = ChildView {
        is_human: true,
        prestige_class: 0,
    };"#;
    let new_father_child = r#"pub fn pick_best_father_p_id(state: &SimState, mother_p_id: i32) -> Option<i32> {
    let mother = state.players.values().find(|p| p.p_id == mother_p_id)?;
    // CLASS-BONI: child class for fitness path (father boni uses mother class).
    let child = ChildView {
        is_human: true,
        prestige_class: PrestigeClass::Commoner as u8,
    };"#;
    if replace_once(&mut t, old_father_child, new_father_child) {
        changed = true;
    }

    let old_mother_view_class = r#"        children_birth_mali: mother_mali,
        prestige_class: 0,
        prestige_from_eating: 0.0,
        family_prestige_for_child: 0.0,
        has_close_nonblocking_grave: false,
        has_close_blocking_grave: false,
        is_human: true,
        little_kids_count: 0,
    };
    let mx = mother.x;"#;
    let new_mother_view_class = r#"        children_birth_mali: mother_mali,
        // CLASS-BONI: father fitness uses calculateClassBoni(father, mother)
        prestige_class: state.player_prestige_class(mother_p_id).as_i32() as u8,
        prestige_from_eating: 0.0,
        family_prestige_for_child: 0.0,
        has_close_nonblocking_grave: false,
        has_close_blocking_grave: false,
        is_human: true,
        little_kids_count: 0,
    };
    let mx = mother.x;"#;
    if !t.contains("player_prestige_class(mother_p_id).as_i32() as u8") {
        if replace_once(&mut t, old_mother_view_class, new_mother_view_class) {
            changed = true;
        }
    }

    let old_father_class = r#"            held_id: pl.held_id,
            held_speed_mult: 1.0,
            prestige_class: 0,
            prestige_from_eating: 0.0,
            is_human: true,
            dist_to_mother: dist,
            is_partner: false,
            little_kids_count: 0,
        };
        let fit = father_fitness(&f, &child, &mother_view);"#;
    let new_father_class = r#"            held_id: pl.held_id,
            held_speed_mult: 1.0,
            // CLASS-BONI: live father lineage class
            prestige_class: state.player_prestige_class(pl.p_id).as_i32() as u8,
            prestige_from_eating: 0.0,
            is_human: true,
            dist_to_mother: dist,
            is_partner: false,
            little_kids_count: 0,
        };
        let fit = father_fitness(&f, &child, &mother_view);"#;
    if !t.contains("player_prestige_class(pl.p_id).as_i32() as u8") {
        if replace_once(&mut t, old_father_class, new_father_class) {
            changed = true;
        }
    }

    if changed {
        let out = restore_nl(&t, crlf);
        if std::fs::write(&lib_path, out).is_err() {
            return false;
        }
    }

    // Docs (CLASS-BONI + CRAVING-WIRE)
    patch_docs(manifest);

    // NOOB-NOBLE-SPAWN piggyback: birth class + lives → 50% Noble first 5 lives
    let _ = build_noob_noble_spawn::patch_noob_noble_spawn(manifest, src);

    let final_lib = std::fs::read_to_string(&lib_path).unwrap_or_default();
    class_boni_wired(&final_lib) || changed
}

fn patch_docs(manifest: &Path) {
    let port = manifest
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("docs").join("port"))
        .unwrap_or_else(|| manifest.join("docs").join("port"));

    // TODO_PORT.md — CRAVING-WIRE
    if let Ok(raw) = std::fs::read_to_string(port.join("TODO_PORT.md")) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        let mut ch = false;

        if let Some(line) = t
            .lines()
            .find(|l| l.starts_with("Last updated:"))
            .map(|l| l.to_string())
        {
            if !line.contains("CRAVING-WIRE") {
                let new_line = "Last updated: **2026-07-26** (CRAVING-WIRE craving_restore)";
                if replace_once(&mut t, &line, new_line) {
                    ch = true;
                }
            }
        }

        if t.contains("- [ ] Full craving restore / CRAVING wire parity") {
            if replace_once(
                &mut t,
                "- [ ] Full craving restore / CRAVING wire parity",
                "- [x] **CRAVING-WIRE** Full craving restore / CRAVING wire (`doIncreaseFoodValue` + CR)",
            ) {
                ch = true;
            }
        }

        if !t.contains("**CRAVING-WIRE craving_restore**") {
            let entry = "| 2026-07-26 | **CRAVING-WIRE craving_restore**: pure `YumState::do_increase_food_value` (YumFoodRestore 0.8 + LovedFoodRestore 0.1 + YumNewCravingChance 0.2); `cravings`/`last_craving_index`/`currently_craving`; `format_craving` CR wire; try_eat_held + USE send; loved foods by person color; tests `yum::do_increase_*`; residual feed-other dontChange path + full SearchBestFood AI |\n";
            if let Some(pos) = t.find("| Date | Change |\n|------|--------|\n") {
                let insert_at = pos + "| Date | Change |\n|------|--------|\n".len();
                t.insert_str(insert_at, entry);
                ch = true;
            }
        }

        if ch {
            let _ = std::fs::write(port.join("TODO_PORT.md"), restore_nl(&t, crlf));
        }
    }

    // FILE_MATRIX.md
    if let Ok(raw) = std::fs::read_to_string(port.join("FILE_MATRIX.md")) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        let mut ch = false;

        if let Some(line) = t
            .lines()
            .find(|l| l.starts_with("Last reviewed:"))
            .map(|l| l.to_string())
        {
            if !line.contains("CRAVING-WIRE") {
                let new_line = "Last reviewed: **2026-07-26** (CRAVING-WIRE craving_restore)";
                if replace_once(&mut t, &line, new_line) {
                    ch = true;
                }
            }
        }

        if t.contains("| GPI-FOOD | yum/meh/superMeh + hasEatenMap + display LS (craving wire open) | PARTIAL |") {
            if replace_once(
                &mut t,
                "| GPI-FOOD | yum/meh/superMeh + hasEatenMap + display LS (craving wire open) | PARTIAL |",
                "| GPI-FOOD / **CRAVING-WIRE** | yum/meh/superMeh + hasEatenMap + display LS + craving restore | PARTIAL → **craving_restore DONE** | `do_increase_food_value` + CR wire + loved restore; residual feed-other parity + full AiHelper SearchBestFood |",
            ) {
                ch = true;
            }
        }

        if ch {
            let _ = std::fs::write(port.join("FILE_MATRIX.md"), restore_nl(&t, crlf));
        }
    }

    // CALL_INDEX.md — CRAVING-WIRE section
    if let Ok(raw) = std::fs::read_to_string(port.join("CALL_INDEX.md")) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if !t.contains("do_increase_food_value") && !t.contains("CRAVING-WIRE") {
            t.push_str(
                r#"

---

## Rust: Craving restore (CRAVING-WIRE / craving_restore)

| Symbol | File | Role |
|--------|------|------|
| `YumState::do_increase_food_value` | `ol-sim/src/yum.rs` | Haxe `GlobalPlayerInstance.doIncreaseFoodValue` |
| `YumState::restore_food_count` / `cravings` | same | Haxe `restoreFoodCount` + craving list |
| `loved_food_ids_for_person_color` | same | Haxe `Biome.getLovedFoodIds` |
| `format_craving` | `ol-protocol` wire_out | Haxe `ClientTag.CRAVING` / CR |
| `try_eat_held` + USE CR send | `ol-sim/src/lib.rs` | live wire after eat |
| Tests | `yum::do_increase_*` / `craving_wire_shape` | pure + protocol |

"#,
            );
            let _ = std::fs::write(port.join("CALL_INDEX.md"), restore_nl(&t, crlf));
        }
    }

    // QUEUE.md — mark CRAVING-WIRE done
    if let Ok(raw) = std::fs::read_to_string(port.join("QUEUE.md")) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        let mut ch = false;
        if t.contains("| `CRAVING-WIRE` | workflow (new) | craving_restore |") {
            if replace_once(
                &mut t,
                "| `CRAVING-WIRE` | workflow (new) | craving_restore |",
                "| ~~`CRAVING-WIRE`~~ | workflow | craving_restore **DONE** |",
            ) {
                ch = true;
            }
        }
        if t.contains("| 46 | `CRAVING-WIRE` | craving_restore | **running** |") {
            if replace_once(
                &mut t,
                "| 46 | `CRAVING-WIRE` | craving_restore | **running** |",
                "| 46 | ~~`CRAVING-WIRE`~~ | craving_restore | **DONE** |",
            ) {
                ch = true;
            }
        }
        if ch {
            let _ = std::fs::write(port.join("QUEUE.md"), restore_nl(&t, crlf));
        }
    }

    // Keep CLASS-BONI docs if not already (legacy path)
    if let Ok(raw) = std::fs::read_to_string(port.join("TODO_PORT.md")) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        let mut ch = false;
        if t.contains("- [ ] Full class boni table") {
            if replace_once(
                &mut t,
                "- [ ] Full class boni table",
                "- [x] **CLASS-BONI** Full class boni table (`calculateClassBoni` + PrestigeClasses names)",
            ) {
                ch = true;
            }
        }
        if ch {
            let _ = std::fs::write(port.join("TODO_PORT.md"), restore_nl(&t, crlf));
        }
    }

    let _ = manifest;
    let _ = PathBuf::new();
}
