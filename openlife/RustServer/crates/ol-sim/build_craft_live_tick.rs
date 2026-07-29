//! CRAFT-LIVE-TICK: wire crate-root re-exports + strip inline tests to include (idempotent).
//! Also wires **DROP-HELD-AI** `drop_held_ai` path submodule + crate-root re-exports.
//! Also runs **NPC-SCAN-FULL** python applicator when present (multi_profession_scan).
//! Also runs **AI-CRAFT-MULTI** python applicator + pure-Rust doc/lib re-export wire.
//! Also runs **AI-SHEPHERD-MID** pure-Rust + python applicator (sheep mid / wet 625 / Profession::Shepherd).
//! Also runs **DROP-HELD-TABLE** table_prefer apply (ShouldDropOnTable / quiver snapshot).
//! Also runs **AI-CRAFT-STICKY** Player.craft_ai sticky runtime wire.
//! Also runs **PREFER-SHORT-WAIT** BusyMoving → Wait + PreferShortCraft craft_actor.
//! Also runs **AI-FARM-STICKY** Player.farm_profession BASICFARMER live weight write.
//! Also runs **AI-CRAFT-TOPDOWN** DoTransitionSearch filters + scan gates.

use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "build_ai_shepherd_mid.rs"]
mod build_ai_shepherd_mid;
#[path = "build_ai_shepherd_mid_farm_to.rs"]
mod build_ai_shepherd_mid_farm_to;
#[path = "build_drop_held_table.rs"]
mod build_drop_held_table;
#[path = "build_ai_craft_sticky.rs"]
mod build_ai_craft_sticky;
#[path = "build_prefer_short_wait.rs"]
mod build_prefer_short_wait;
#[path = "build_ai_farm_sticky.rs"]
mod build_ai_farm_sticky;
#[path = "build_ai_craft_topdown.rs"]
mod build_ai_craft_topdown;

// Body shared with previous CRAFT-LIVE-TICK implementation (profession_scan / drop_held / craft multi).
include!("build_craft_live_tick_body.inc.rs");

/// Run AI-SHEPHERD-MID pure-Rust patcher (primary) + optional python.
fn run_shepherd_mid_apply(src: &Path) -> bool {
    if build_ai_shepherd_mid::already_wired(src) {
        let _ = build_ai_shepherd_mid_farm_to::patch(src);
        return true;
    }
    let crate_dir = src.parent().unwrap_or(src);
    let workspace = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(crate_dir);
    let rs_ok = build_ai_shepherd_mid::patch_all(src, workspace);
    let _ = build_ai_shepherd_mid_farm_to::patch(src);
    if rs_ok || build_ai_shepherd_mid::already_wired(src) {
        println!("cargo:warning=AI-SHEPHERD-MID: sheep_mid_sites applied (pure Rust)");
        return true;
    }
    for name in ["_run_patch_now.py", "_shepherd_mid_patch_core.py", "_apply_shepherd_mid.py"] {
        let apply = src.join(name);
        if !apply.exists() {
            continue;
        }
        let st = Command::new("python")
            .arg(&apply)
            .status()
            .or_else(|_| Command::new("python3").arg(&apply).status());
        let _ = build_ai_shepherd_mid_farm_to::patch(src);
        if matches!(st, Ok(s) if s.success()) && build_ai_shepherd_mid::already_wired(src) {
            println!("cargo:warning=AI-SHEPHERD-MID: sheep_mid_sites applied via {name}");
            return true;
        }
    }
    println!("cargo:warning=AI-SHEPHERD-MID: could not fully wire sheep mid sites");
    false
}

/// Run NPC-SCAN-FULL python applicator (idempotent multi_profession_scan wire).
fn run_npc_scan_full_apply(src: &Path) -> bool {
    let _ = run_shepherd_mid_apply(src);

    let apply = src.join("_patch_npc_scan_full.py");
    if !apply.exists() {
        return false;
    }
    let st = Command::new("python")
        .arg(&apply)
        .status()
        .or_else(|_| Command::new("python3").arg(&apply).status());
    match st {
        Ok(s) if s.success() => {
            let scan_ok = std::fs::read_to_string(src.join("profession_scan.rs"))
                .map(|t| {
                    t.contains("ProfessionScanKind::Pottery")
                        || t.contains("pottery_profession_scan_tick")
                })
                .unwrap_or(false);
            if scan_ok {
                println!("cargo:warning=NPC-SCAN-FULL: multi_profession_scan applied");
            }
            scan_ok
        }
        _ => false,
    }
}

fn run_drop_held_table_apply(src: &Path, workspace: &Path) -> bool {
    let drop = std::fs::read_to_string(src.join("drop_held_ai.rs")).unwrap_or_default();
    let player = std::fs::read_to_string(src.join("player.rs")).unwrap_or_default();
    let lib = std::fs::read_to_string(src.join("lib.rs")).unwrap_or_default();
    if build_drop_held_table::drop_held_table_wired(&drop, &player, &lib) {
        return true;
    }
    build_drop_held_table::patch_drop_held_table(src, workspace)
}

fn run_ai_craft_sticky_apply(src: &Path, workspace: &Path) -> bool {
    if build_ai_craft_sticky::already_wired(src) {
        return true;
    }
    build_ai_craft_sticky::patch_all(src, workspace)
}

fn run_prefer_short_wait_apply(src: &Path, workspace: &Path) -> bool {
    let drop = std::fs::read_to_string(src.join("drop_held_ai.rs")).unwrap_or_default();
    let sci = std::fs::read_to_string(src.join("short_craft_intent.rs")).unwrap_or_default();
    let lib = std::fs::read_to_string(src.join("lib.rs")).unwrap_or_default();
    let ps = std::fs::read_to_string(src.join("profession_scan.rs")).unwrap_or_default();
    if build_prefer_short_wait::prefer_short_wait_wired(&drop, &sci, &lib, &ps) {
        return true;
    }
    build_prefer_short_wait::patch_prefer_short_wait(src, workspace)
}

fn run_ai_farm_sticky_apply(src: &Path, workspace: &Path) -> bool {
    if build_ai_farm_sticky::already_wired(src) {
        return true;
    }
    build_ai_farm_sticky::patch_all(src, workspace)
}

/// AI-CRAFT-TOPDOWN: DoTransitionSearch filters + hostile/unreachable scan.
fn run_ai_craft_topdown_apply(src: &Path, workspace: &Path) -> bool {
    if build_ai_craft_topdown::already_wired(src) {
        let _ = build_ai_craft_topdown::patch_all(src, workspace);
        return true;
    }
    let ok = build_ai_craft_topdown::patch_all(src, workspace);
    if ok || build_ai_craft_topdown::already_wired(src) {
        println!("cargo:warning=AI-CRAFT-TOPDOWN: DoTransitionSearch filters applied");
        true
    } else {
        println!("cargo:warning=AI-CRAFT-TOPDOWN: could not wire craft_topdown into craft_item");
        false
    }
}

/// Full CRAFT-LIVE-TICK + DROP-HELD-AI + NPC-SCAN-FULL + AI-CRAFT-MULTI + AI-SHEPHERD-MID
/// + DROP-HELD-TABLE + AI-CRAFT-STICKY + PREFER-SHORT-WAIT + AI-FARM-STICKY
/// + AI-CRAFT-TOPDOWN patch.
pub fn patch_all_craft_live_tick(src: &Path, lib_path: &PathBuf) -> bool {
    let sm = run_shepherd_mid_apply(src);
    let a = patch_profession_scan_tests(src);
    let b = patch_lib_craft_live_tick(lib_path);
    let c = patch_short_craft_drop_held_submod(src);
    let d = patch_lib_drop_held_ai(lib_path);
    let e = run_npc_scan_full_apply(src);
    let f = patch_lib_craft_item_multi(lib_path);
    let _ = run_ai_craft_multi_apply(src);
    let workspace = lib_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf());
    let g = workspace
        .as_ref()
        .map(|w| patch_docs_craft_item_multi(w))
        .unwrap_or(false);
    let h = workspace
        .as_ref()
        .map(|w| run_drop_held_table_apply(src, w))
        .unwrap_or_else(|| run_drop_held_table_apply(src, src));
    let sticky = workspace
        .as_ref()
        .map(|w| run_ai_craft_sticky_apply(src, w))
        .unwrap_or_else(|| run_ai_craft_sticky_apply(src, src));
    let psw = workspace
        .as_ref()
        .map(|w| run_prefer_short_wait_apply(src, w))
        .unwrap_or_else(|| run_prefer_short_wait_apply(src, src));
    let farm_sticky = workspace
        .as_ref()
        .map(|w| run_ai_farm_sticky_apply(src, w))
        .unwrap_or_else(|| run_ai_farm_sticky_apply(src, src));
    let topdown = workspace
        .as_ref()
        .map(|w| run_ai_craft_topdown_apply(src, w))
        .unwrap_or_else(|| run_ai_craft_topdown_apply(src, src));
    let d2 = patch_lib_drop_held_ai(lib_path);
    sm || a || b || c || d || e || f || g || h || sticky || psw || farm_sticky || topdown || d2
        || craft_live_tick_wired(lib_path)
        || drop_held_ai_wired(lib_path)
        || craft_item_multi_wired(lib_path)
        || build_ai_craft_sticky::already_wired(src)
        || build_ai_farm_sticky::already_wired(src)
        || build_ai_craft_topdown::already_wired(src)
}
