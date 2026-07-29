//! Build-time patches for **C-SS-AI-IGNORE** + **TH-ALT-OUTCOME**.
//!
//! 1. Wire `apply_default_ai_should_ignore_patches` into binary-cache boot.
//! 2. Wire ContentDb ignore table into ol-sim reverse craft graph + topdown search.
//! 3. Wire `apply_default_alternative_outcome_patches` into binary-cache finish.
//! 4. Run ol-sim `_apply_th_alt_outcome.py` when present (USE path + lib mod).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/binary_cache.rs");
    println!("cargo:rerun-if-changed=src/ai_should_ignore_patches.inc.rs");
    println!("cargo:rerun-if-changed=src/alt_outcome_patches.inc.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    patch_binary_cache_ai_ignore(&manifest.join("src/binary_cache.rs"));
    patch_binary_cache_alt_outcome(&manifest.join("src/binary_cache.rs"));

    // Sibling crate ol-sim (workspace/crates/ol-sim)
    if let Some(crates) = manifest.parent() {
        let sim_src = crates.join("ol-sim/src");
        if sim_src.is_dir() {
            println!("cargo:rerun-if-changed={}", sim_src.join("lib.rs").display());
            println!(
                "cargo:rerun-if-changed={}",
                sim_src.join("craft_topdown.rs").display()
            );
            println!(
                "cargo:rerun-if-changed={}",
                sim_src.join("craft_graph.rs").display()
            );
            println!(
                "cargo:rerun-if-changed={}",
                sim_src.join("use_transition.rs").display()
            );
            println!(
                "cargo:rerun-if-changed={}",
                sim_src.join("alt_outcome.rs").display()
            );
            println!(
                "cargo:rerun-if-changed={}",
                sim_src.join("_apply_th_alt_outcome.py").display()
            );
            patch_sim_lib(&sim_src.join("lib.rs"));
            patch_sim_craft_topdown(&sim_src.join("craft_topdown.rs"));
            // TH-ALT-OUTCOME: apply python wire for use_transition + lib + docs
            let apply = sim_src.join("_apply_th_alt_outcome.py");
            if apply.exists() {
                let _ = Command::new("python")
                    .arg(&apply)
                    .status()
                    .or_else(|_| Command::new("python3").arg(&apply).status());
            }
        }
    }
}

fn patch_binary_cache_ai_ignore(path: &Path) {
    let Ok(raw) = fs::read_to_string(path) else {
        return;
    };
    if raw.contains("apply_default_ai_should_ignore_patches") {
        return;
    }
    let mut next = raw.clone();
    if next.contains("apply_default_horse_transition_patches,") {
        next = next.replace(
            "apply_default_horse_transition_patches,",
            "apply_default_ai_should_ignore_patches,\n    apply_default_horse_transition_patches,",
        );
    }
    if next.contains("apply_default_horse_transition_patches(db);")
        && !next.contains("apply_default_ai_should_ignore_patches(db);")
    {
        next = next.replace(
            "apply_default_horse_transition_patches(db);\n    apply_default_weapon_range_patches(db);",
            "apply_default_horse_transition_patches(db);\n    // C-SS-AI-IGNORE: ServerSettings.PatchTransitions aiShouldIgnore\n    apply_default_ai_should_ignore_patches(db);\n    apply_default_weapon_range_patches(db);",
        );
    }
    if next != raw {
        let _ = fs::write(path, next);
    }
}

fn patch_binary_cache_alt_outcome(path: &Path) {
    let Ok(raw) = fs::read_to_string(path) else {
        return;
    };
    if raw.contains("apply_default_alternative_outcome_patches") {
        return;
    }
    let mut next = raw.clone();
    if next.contains("apply_default_horse_transition_patches,") {
        next = next.replace(
            "apply_default_horse_transition_patches,",
            "apply_default_alternative_outcome_patches,\n    apply_default_horse_transition_patches,",
        );
    }
    if next.contains("apply_default_horse_transition_patches(db);")
        && !next.contains("apply_default_alternative_outcome_patches(db);")
    {
        next = next.replace(
            "apply_default_horse_transition_patches(db);\n    // C-SS-AI-IGNORE",
            "apply_default_horse_transition_patches(db);\n    // TH-ALT-OUTCOME\n    apply_default_alternative_outcome_patches(db);\n    // C-SS-AI-IGNORE",
        );
        if !next.contains("apply_default_alternative_outcome_patches(db);") {
            next = next.replace(
                "apply_default_horse_transition_patches(db);\n    apply_default_ai_should_ignore_patches(db);",
                "apply_default_horse_transition_patches(db);\n    // TH-ALT-OUTCOME\n    apply_default_alternative_outcome_patches(db);\n    apply_default_ai_should_ignore_patches(db);",
            );
        }
    }
    if next != raw {
        let _ = fs::write(path, next);
    }
}

fn patch_sim_lib(path: &Path) {
    let Ok(raw) = fs::read_to_string(path) else {
        return;
    };
    if raw.contains("load_ai_should_ignore_from") {
        return;
    }
    let needle = "let seeded = graph.seed_from_pairs(pairs, cap);";
    if let Some(idx) = raw.find(needle) {
        let mut next = raw.clone();
        next.insert_str(
            idx + needle.len(),
            "\n    // C-SS-AI-IGNORE: content PatchTransitions → craft graph skip edges\n    graph.load_ai_should_ignore_from(content.ai_should_ignore.iter().copied());",
        );
        if !next.contains("ai_ignore = graph.ai_should_ignore_count()") {
            next = next.replace(
                "let seeded = graph.seed_from_pairs(pairs, cap);",
                "let seeded = graph.seed_from_pairs(pairs, cap);",
            );
        }
        let _ = fs::write(path, next);
    }
}

fn patch_sim_craft_topdown(path: &Path) {
    let Ok(raw) = fs::read_to_string(path) else {
        return;
    };
    if raw.contains("ai_should_ignore_edge") {
        return;
    }
    // legacy no-op if already wired by source
    let _ = raw;
}
