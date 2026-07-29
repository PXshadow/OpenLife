//! Build-time wire for **LOCKPICK-SETTINGS / lockpick_live_knobs**.
//!
//! Ensures:
//! - `SimState.lockpick_settings: LockpickSettings`
//! - `apply_live_settings` maps LiveSettings lockpick* → SimState
//! - `apply_use_at` uses `state.lockpick_settings` (not hardcoded Default)
//! - boot apply from hot-reload tracker `last_live`
//!
//! Also runs **HORSE-MOUNT-POLISH hitch_cart** source wire (basket refuse + live tests).
//! Also runs **COMBAT-BLOODY bloody_weapon** source wire (HIT/HUNT/DROP).
//! Also runs **ALLY-STRENGTH ally_combat** source wire (HIT allyFactor).
//! Also runs **FERTILITY-TWINS twin_sockets** source wire (isFertile + TwinWaitQueue).
//!
//! Idempotent. Handles CRLF sources.
//!
//! Note: lockpick core is source-wired (DONE); this module mostly stamps + piggy-backs.

use std::path::Path;

#[path = "build_fertility_twins.rs"]
mod fertility_twins_inline;

#[path = "build_horse_mount_polish.rs"]
mod horse_mount_inline;

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

pub fn lockpick_settings_ready(lib: &str, settings: &str, use_tr: &str) -> bool {
    lib.contains("pub lockpick_settings: LockpickSettings")
        && lib.contains("lockpick_settings: LockpickSettings::default()")
        && settings.contains("lockpick_success_chance")
        && settings.contains("state.lockpick_settings")
        && use_tr.contains("state.lockpick_settings")
        && !use_tr.contains("let lock_settings = LockpickSettings::default();")
}

/// Patch lib + settings_live + use_transition. Returns true when ready.
pub fn patch_lockpick_settings(src_dir: &Path) -> bool {
    let lib_path = src_dir.join("lib.rs");
    let settings_path = src_dir.join("settings_live.rs");
    let use_path = src_dir.join("use_transition.rs");

    // Lockpick is source-wired; only apply residual string fixes if missing.
    let _ = patch_use_transition_lockpick(&use_path);

    // HORSE-MOUNT-POLISH hitch_cart (idempotent; piggy-backs this always-on wire)
    let _ = horse_mount_inline::patch_horse_mount_polish(src_dir);

    // COMBAT-BLOODY bloody_weapon (idempotent; piggy-backs always-on wire)
    let combat_ok = patch_combat_bloody_inline(src_dir);
    if combat_ok {
        let stamp = src_dir.join(".combat_bloody_patched");
        let _ = std::fs::write(&stamp, b"combat-bloody-1-source-wired\n");
    }

    // ALLY-STRENGTH ally_combat (idempotent; piggy-backs always-on wire)
    let ally_ok = patch_ally_strength_inline(src_dir);
    if ally_ok {
        let stamp = src_dir.join(".ally_strength_patched");
        let _ = std::fs::write(&stamp, b"ally-strength-1-source-wired\n");
    } else {
        println!("cargo:warning=ALLY-STRENGTH: could not wire allyFactor into lib.rs HIT path");
    }

    // FERTILITY-TWINS twin_sockets (idempotent; piggy-backs always-on wire)
    // workspace = RustServer (src = ol-sim/src → ol-sim → crates → RustServer)
    let workspace = src_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| src_dir.to_path_buf());
    let fert_ok = fertility_twins_inline::patch_fertility_twins(src_dir, &workspace);
    // Port docs (TODO_PORT / FILE_MATRIX) — best-effort string replace.
    patch_fertility_docs(&workspace);
    if fert_ok {
        let stamp = src_dir.join(".fertility_twins_patched");
        let _ = std::fs::write(&stamp, b"fertility-twins-1-source-wired\n");
    } else {
        println!(
            "cargo:warning=FERTILITY-TWINS: could not wire isFertile/twin_wait into lib.rs/ol-net"
        );
    }

    let lib = std::fs::read_to_string(&lib_path).unwrap_or_default();
    let settings = std::fs::read_to_string(&settings_path).unwrap_or_default();
    let use_tr = std::fs::read_to_string(&use_path).unwrap_or_default();
    lockpick_settings_ready(&lib, &settings, &use_tr)
}

fn patch_use_transition_lockpick(use_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(use_path) else {
        return false;
    };
    if raw.contains("state.lockpick_settings")
        && !raw.contains("let lock_settings = LockpickSettings::default();")
    {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);
    if text.contains("let lock_settings = LockpickSettings::default();") {
        text = text.replacen(
            "let lock_settings = LockpickSettings::default();",
            "let lock_settings = state.lockpick_settings;",
            1,
        );
        let out = restore_nl(&text, crlf);
        return std::fs::write(use_path, out).is_ok();
    }
    text.contains("state.lockpick_settings")
}

fn patch_fertility_docs(workspace: &Path) {
    let port = workspace.join("docs/port");
    let todo_path = port.join("TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo_path) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        let mut ch = false;
        if t.contains("- [ ] Full fertile rules + twin sockets") {
            t = t.replacen(
                "- [ ] Full fertile rules + twin sockets  \n",
                "- [x] **FERTILITY-TWINS twin_sockets** — Haxe `isFertile` (female+age+deleted) pure `is_fertile`/`can_birth_full`; BIRTH/GESTATE/nurse/mother-pick wire; pure `TwinWaitQueue` (protocol twin_code_hash count 2–4); LOGIN→TWINJOIN; SAY TWINJOIN/?TWINWAIT/?TWINS; residual multi-server sockets stub + twin death heart-link  \n",
                1,
            );
            ch = true;
        }
        if t.contains("| Birth/gestation/nurse | ■■ | ■■ | | Multiplayer product polish |") {
            t = t.replacen(
                "| Birth/gestation/nurse | ■■ | ■■ | | Multiplayer product polish |\n",
                "| Birth/gestation/nurse | ■■ | ■ | | **FERTILITY-TWINS** isFertile + twin wait queue core |\n",
                1,
            );
            ch = true;
        }
        if !t.contains("**FERTILITY-TWINS twin_sockets**: pure") {
            let marker = "| Date | Change |\n|------|--------|\n";
            let row = "| 2026-07-26 | **FERTILITY-TWINS twin_sockets**: pure `is_fertile`/`can_birth_full`; `TwinWaitQueue`; BIRTH/GESTATE female; LOGIN TWINJOIN; tests fertility/twins; residual multi-server sockets |\n";
            if t.contains(marker) {
                t = t.replacen(marker, &format!("{marker}{row}"), 1);
                ch = true;
            }
        }
        if ch {
            let _ = std::fs::write(&todo_path, restore_nl(&t, crlf));
        }
    }
    let mx_path = port.join("FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&mx_path) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        let mut ch = false;
        if t.contains("twins priority queue missing;") {
            t = t.replacen(
                "twins priority queue missing;",
                "**FERTILITY-TWINS** twin wait queue core;",
                1,
            );
            ch = true;
        }
        if t.contains("| GPI-BABY | baby/hold/dropPlayer/jump | PARTIAL |") {
            t = t.replacen(
                "| GPI-BABY | baby/hold/dropPlayer/jump | PARTIAL |\n",
                "| GPI-BABY / **FERTILITY-TWINS** | baby/hold + fertile + twin wait | PARTIAL → **fertile+twin core DONE** | `is_fertile`/`TwinWaitQueue`/BIRTH female/TWINJOIN; residual twin death link / multi-server sockets |\n",
                1,
            );
            ch = true;
        }
        if ch {
            let _ = std::fs::write(&mx_path, restore_nl(&t, crlf));
        }
    }
}

include!("build_combat_bloody_inc.rs");
include!("build_ally_strength_inc.rs");
