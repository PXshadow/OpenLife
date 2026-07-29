//! OLC1-DISTANCES: keep RustClient compile-compatible when OLC1 write path is v7+.
//! Also stamps port docs / weapon-patch comment (idempotent).

use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/olc1.rs");

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    // ── client content_binary.rs ────────────────────────────────────────────
    let client_rs = manifest_dir.join("../../../../RustClient/src/content_binary.rs");
    if let Ok(client_rs) = client_rs.canonicalize() {
        if client_rs.is_file() {
            println!("cargo:rerun-if-changed={}", client_rs.display());
            patch_client(&client_rs);
        }
    }

    // ── port docs ───────────────────────────────────────────────────────────
    let port = manifest_dir.join("../../docs/port");
    if port.is_dir() {
        patch_file_matrix(&port.join("FILE_MATRIX.md"));
        patch_todo_port(&port.join("TODO_PORT.md"));
        patch_call_index(&port.join("CALL_INDEX.md"));
    }

    // ── weapon-patch doc comment in ol-content ───────────────────────────────
    let tail = manifest_dir.join("../ol-content/src/lib_tail.inc.rs");
    if let Ok(tail) = tail.canonicalize() {
        replace_in_file(
            &tail,
            "/// Safe if id missing. Binary cache lacks these fields until OLC1 format bumps;\n/// patches keep bows at range 5 and deadly 4 so USE min-range works.",
            "/// Safe if id missing. OLC1 v7 stores use/deadly/moves; patches still override\n/// weapon ids (bows range 5 / deadly 4) so USE min-range works on text or old cache.",
        );
    }
}

fn replace_in_file(path: &Path, from: &str, to: &str) {
    let Ok(orig) = std::fs::read_to_string(path) else {
        return;
    };
    if !orig.contains(from) {
        return;
    }
    let text = orig.replace(from, to);
    if text != orig {
        let _ = std::fs::write(path, text);
    }
}

fn patch_client(client_rs: &Path) {
    let Ok(orig) = std::fs::read_to_string(client_rs) else {
        return;
    };
    let mut text = orig.clone();

    // Variable-dummy materialize gate must stay on format ≥ 6 forever (not write-path version).
    text = text.replace(
        "    if fmt < OLC1_FORMAT_VERSION {\n        assign_variable_dummies(db);",
        "    if fmt < 6u32 {\n        assign_variable_dummies(db);",
    );
    text = text.replace(
        "    if fmt < OLC1_FORMAT_VERSION_V6 {\n        assign_variable_dummies(db);",
        "    if fmt < 6u32 {\n        assign_variable_dummies(db);",
    );

    // Client bake: OLC1 v7 trailer from ClientObjectDef (object-file values).
    if !text.contains("deadly_distance: def.deadly_distance") {
        let needle =
            "        variable_dummy_ids: def.variable_dummy_ids.clone(),\n    }\n}\n\nfn client_sprite_to_bin";
        let insert = "        variable_dummy_ids: def.variable_dummy_ids.clone(),\n\
        // OLC1 v7 — Haxe ObjectData.deadlyDistance / useDistance / moves.\n\
        deadly_distance: def.deadly_distance,\n\
        use_distance: def.use_distance,\n\
        moves: def.moves,\n\
    }\n}\n\nfn client_sprite_to_bin";
        if text.contains(needle) {
            text = text.replace(needle, insert);
        }
        // Upgrade legacy hardcoded defaults if still present.
        text = text.replace(
            "        // OLC1 v7 trailer (Haxe deadlyDistance / useDistance / moves).\n\
        deadly_distance: 0.0,\n\
        use_distance: 1,\n\
        moves: 0,\n",
            "        // OLC1 v7 trailer — Haxe ObjectData.writeToFile deadlyDistance/useDistance + moves.\n\
        deadly_distance: def.deadly_distance,\n\
        use_distance: def.use_distance,\n\
        moves: def.moves,\n",
        );
    }

    text = text.replace(
        "/// Accepts format 1..=6. Runtime-materializes multi-use + variable dummy object records.",
        "/// Accepts format 1..=7. Runtime-materializes multi-use + variable dummy object records.",
    );

    if !text.contains("Format 7 adds") {
        text = text.replace(
            "//!   Format 6 adds `variableDummyIDs` lists (P4#26 C++ `autoGenerateVariableObjects`).\n",
            "//!   Format 6 adds `variableDummyIDs` lists (P4#26 C++ `autoGenerateVariableObjects`).\n\
//!   Format 7 adds `deadlyDistance` / `useDistance` / `moves` (server IS-CLOSE / animals).\n",
        );
    }

    if text != orig {
        let _ = std::fs::write(client_rs, text);
    }
}

fn patch_file_matrix(path: &Path) {
    let Ok(orig) = std::fs::read_to_string(path) else {
        return;
    };
    let mut text = orig.replace(
        "Last reviewed: **2026-07-28** (FOODSTATS-DISK foodstats_txt) (NOOB-NOBLE-SPAWN spawn_weights)",
        "Last reviewed: **2026-07-28** (OLC1-DISTANCES binary_use_dist) (FOODSTATS-DISK foodstats_txt)",
    );
    text = text.replace(
        "| D-OD | `data/object/ObjectData.hx` | `ol-content` | PARTIAL → **useDistance/deadlyDistance/moves + weapon patches** |",
        "| D-OD | `data/object/ObjectData.hx` | `ol-content` + `ol-binary` OLC1 | PARTIAL → **text + OLC1 v7 distances DONE** | useDistance/deadlyDistance/moves text + weapon/animal patches + **OLC1 v7 encode/load** |",
    );
    text = text.replace(
        "weapon+animal patches; USE/DROP/REMV wired; residual: OLC1 binary field encode, Connection MaxDistance fans, Too close say/PS |",
        "weapon+animal patches; USE/DROP/REMV wired; **OLC1 v7 distance trailer DONE**; residual: Connection MaxDistance fans, Too close say/PS |",
    );
    if !text.contains("**OLC1-DISTANCES**") {
        text = text.replace(
            "weapon+animal patches; USE/DROP/REMV wired; **OLC1 v7 distance trailer DONE**; residual: Connection MaxDistance fans, Too close say/PS |\n",
            "weapon+animal patches; USE/DROP/REMV wired; **OLC1 v7 distance trailer DONE**; residual: Connection MaxDistance fans, Too close say/PS |\n\
| **OLC1-DISTANCES** / binary_use_dist | OLC1 v7 use/deadly/moves | **DONE** | `ol-binary` OLC1_FORMAT_VERSION=7; encode deadly_distance(f32)+use_distance(i32)+moves(i32); `ol-content` maps into ObjectDef; tests `olc1_v7_*` |\n",
        );
    }
    if text != orig {
        let _ = std::fs::write(path, text);
    }
}

fn patch_todo_port(path: &Path) {
    let Ok(orig) = std::fs::read_to_string(path) else {
        return;
    };
    let mut text = orig.replace(
        "Last updated: **2026-07-28** (FOODSTATS-DISK foodstats_txt) (NOOB-NOBLE-SPAWN spawn_weights)",
        "Last updated: **2026-07-28** (OLC1-DISTANCES binary_use_dist) (FOODSTATS-DISK foodstats_txt) (NOOB-NOBLE-SPAWN spawn_weights)",
    );
    if !text.contains("OLC1-DISTANCES / binary_use_dist") {
        text = text.replace(
            "- [x] **IS-CLOSE / action_range** — `check_if_not_moving_and_close_enough` + held `ObjectDef.use_distance` (clamp ≥1) + torus wrap `in_use_range_ex`; `is_close_use_exact_*_wrap`; USE bow min-range (`deadlyDistance>1.9` + animal + exact≤1.5); DROP/REMV same range gate; content load + weapon/animal patches; tests `action_range_*` / `use_respects_held_use_distance_five` / `use_refuses_ranged_too_close_to_animal`  \n",
            "- [x] **IS-CLOSE / action_range** — `check_if_not_moving_and_close_enough` + held `ObjectDef.use_distance` (clamp ≥1) + torus wrap `in_use_range_ex`; `is_close_use_exact_*_wrap`; USE bow min-range (`deadlyDistance>1.9` + animal + exact≤1.5); DROP/REMV same range gate; content load + weapon/animal patches; tests `action_range_*` / `use_respects_held_use_distance_five` / `use_refuses_ranged_too_close_to_animal`  \n\
- [x] **OLC1-DISTANCES / binary_use_dist** — OLC1 format **v7** trailer `deadly_distance`(f32) + `use_distance`(i32) + `moves`(i32); `ol-binary` encode/parse; `ol-content` `olc1_record_to_object` → ObjectDef; legacy &lt;7 defaults 0/1/0; tests `olc1_v7_roundtrip_minimal` / `olc1_v7_distances_server_load` / `olc1_legacy_v6_defaults_distances`  \n",
        );
    }
    if text != orig {
        let _ = std::fs::write(path, text);
    }
}

fn patch_call_index(path: &Path) {
    let Ok(orig) = std::fs::read_to_string(path) else {
        return;
    };
    if orig.contains("OLC1-DISTANCES") {
        return;
    }
    let old = "| Rust `ObjectDef.use_distance` / `deadly_distance` / `moves` / `is_animal` | `ol-content` | content fields + weapon/animal patches |\n";
    let new = "| Rust `ObjectDef.use_distance` / `deadly_distance` / `moves` / `is_animal` | `ol-content` | content fields + weapon/animal patches |\n\
| Rust OLC1 v7 `deadly_distance`/`use_distance`/`moves` encode/load | `ol-binary` + `ol-content/binary_cache` | **OLC1-DISTANCES** binary_use_dist |\n";
    if orig.contains(old) {
        let _ = std::fs::write(path, orig.replace(old, new));
    }
}
