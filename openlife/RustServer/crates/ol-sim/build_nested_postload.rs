//! Build-time wire for **NESTED-OLW1-POLISH postload_wire**.
//!
//! Ensures:
//! - `mod postload_wire` + `pub use postload_wire::{…}`
//! - `Player.owning` field + `Player::new` init
//! - sim boot calls `apply_init_object_helpers_after_read` after OLA1 seed
//! - `spawn_player` rebuilds owning from world helpers
//!
//! Idempotent. Handles CRLF sources.

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

pub fn postload_wire_ready(lib_text: &str, player_text: &str) -> bool {
    lib_text.contains("mod postload_wire;")
        && lib_text.contains("apply_init_object_helpers_after_read")
        && lib_text.contains("rebuild_player_owning_from_world")
        && player_text.contains("pub owning:")
}

/// Prefer Python script; fall back to pure Rust replaces.
pub fn patch_nested_postload(src_dir: &Path, workspace: &Path) -> bool {
    let lib_path = src_dir.join("lib.rs");
    let player_path = src_dir.join("player.rs");
    let lib_text = std::fs::read_to_string(&lib_path).unwrap_or_default();
    let player_text = std::fs::read_to_string(&player_path).unwrap_or_default();
    if postload_wire_ready(&lib_text, &player_text) {
        return true;
    }

    let script = workspace.join("scripts/patch_nested_postload_wire.py");
    if script.exists() {
        let py = Command::new("python")
            .arg(&script)
            .status()
            .or_else(|_| Command::new("python3").arg(&script).status())
            .or_else(|_| Command::new("py").arg("-3").arg(&script).status());
        if let Ok(s) = py {
            if s.success() {
                let lib_text = std::fs::read_to_string(&lib_path).unwrap_or_default();
                let player_text = std::fs::read_to_string(&player_path).unwrap_or_default();
                if postload_wire_ready(&lib_text, &player_text) {
                    return true;
                }
            }
        }
    }

    let p_ok = patch_player_owning(&player_path);
    let l_ok = patch_lib_postload(&lib_path);
    let lib_text = std::fs::read_to_string(&lib_path).unwrap_or_default();
    let player_text = std::fs::read_to_string(&player_path).unwrap_or_default();
    postload_wire_ready(&lib_text, &player_text) || (p_ok && l_ok)
}

fn patch_player_owning(player_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(player_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);
    if text.contains("pub owning:") {
        return true;
    }

    let field_old = "    /// Sim-time of last successful lite profession action\n\
    /// (`HARVEST` / `FISH` / `MINE` / `DIG` / `CHOP`).\n\
    /// Initialized negative so the first action is not on cooldown.\n\
    pub last_prof_action_time: f32,\n\
}";
    let field_new = "    /// Sim-time of last successful lite profession action\n\
    /// (`HARVEST` / `FISH` / `MINE` / `DIG` / `CHOP`).\n\
    /// Initialized negative so the first action is not on cooldown.\n\
    pub last_prof_action_time: f32,\n\
    /// Haxe `GlobalPlayerInstance.owning` — absolute world tiles of owned objects\n\
    /// (filled by `InitObjectHelpersAfterRead` / CLAIM / DROP ownership).\n\
    pub owning: Vec<(i32, i32)>,\n\
}";
    if !text.contains(field_old) {
        return false;
    }
    text = text.replacen(field_old, field_new, 1);

    let init_old = "            holding_player_id: 0,\n\
            held_by: 0,\n\
            last_prof_action_time: -crate::professions::PROF_ACTION_COOLDOWN_SECS,\n\
        }\n\
    }";
    let init_new = "            holding_player_id: 0,\n\
            held_by: 0,\n\
            last_prof_action_time: -crate::professions::PROF_ACTION_COOLDOWN_SECS,\n\
            owning: Vec::new(),\n\
        }\n\
    }";
    if !text.contains(init_old) {
        return false;
    }
    text = text.replacen(init_old, init_new, 1);

    let out = restore_nl(&text, crlf);
    std::fs::write(player_path, out).is_ok()
}

fn patch_lib_postload(lib_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);
    let mut changed = false;

    if !text.contains("mod postload_wire;") {
        // Prefer after long_term; fall back after death_polish / mutation.
        let anchors = [
            (
                "mod long_term;\n",
                "mod long_term;\n// Haxe: ObjectHelper.InitObjectHelpersAfterRead (NESTED-OLW1-POLISH postload_wire)\nmod postload_wire;\n",
            ),
            (
                "mod death_polish;\n",
                "mod death_polish;\nmod postload_wire;\n",
            ),
            (
                "mod mutation;\n",
                "mod mutation;\nmod postload_wire;\n",
            ),
        ];
        for (old, new) in anchors {
            if text.contains(old) && !text.contains("mod postload_wire;") {
                text = text.replacen(old, new, 1);
                changed = true;
                break;
            }
        }
    }

    if !text.contains("pub use postload_wire::") {
        let insert = "pub use postload_wire::{\n\
    apply_grave_account_link, apply_init_object_helpers_after_read,\n\
    apply_player_owning_link, content_grave_meta, description_is_orig_grave,\n\
    player_alive_for_postload, rebuild_account_graves_from_world,\n\
    rebuild_player_owning_from_world, account_token_index, PostloadWireStats,\n\
};\n";
        let anchors = [
            "pub use long_term::{\n",
            "pub use mutation::{SpecialIndex, SpecialKind};\n",
            "pub use death_polish::{apply_death_polish, place_grave_with_soul};\n",
        ];
        for a in anchors {
            if text.contains(a) {
                text = text.replacen(a, &format!("{insert}{a}"), 1);
                changed = true;
                break;
            }
        }
    }

    if !text.contains("apply_init_object_helpers_after_read(&mut state)") {
        let boot_old = "    // Seed soft accounts from boot-loaded OLA1 (if any).\n\
    if let Some(ref shared) = shared_accounts {\n\
        state.accounts = shared.read().unwrap().clone();\n\
        info!(\n\
            accounts = state.accounts.len(),\n\
            \"sim: loaded accounts from shared book\"\n\
        );\n\
    }\n\
    // Natural spawn / OLW load never went through USE — arm decay timers now.\n\
    arm_decays_for_loaded_world(&mut state);";
        let boot_new = "    // Seed soft accounts from boot-loaded OLA1 (if any).\n\
    if let Some(ref shared) = shared_accounts {\n\
        state.accounts = shared.read().unwrap().clone();\n\
        info!(\n\
            accounts = state.accounts.len(),\n\
            \"sim: loaded accounts from shared book\"\n\
        );\n\
    }\n\
    // Haxe ObjectHelper.InitObjectHelpersAfterRead — after world+accounts loaded.\n\
    // Rewire graves→accounts and owned helpers→player.owning; prune dead owners.\n\
    {\n\
        let stats = apply_init_object_helpers_after_read(&mut state);\n\
        if stats.helpers_scanned > 0 {\n\
            info!(\n\
                helpers = stats.helpers_scanned,\n\
                graves = stats.graves_linked,\n\
                owning = stats.owning_linked,\n\
                pruned = stats.owners_pruned,\n\
                \"sim: postload InitObjectHelpersAfterRead\"\n\
            );\n\
        }\n\
    }\n\
    // Natural spawn / OLW load never went through USE — arm decay timers now.\n\
    arm_decays_for_loaded_world(&mut state);";
        if text.contains(boot_old) {
            text = text.replacen(boot_old, boot_new, 1);
            changed = true;
        }
    }

    if !text.contains("rebuild_player_owning_from_world(state, p_id)") {
        let spawn_old = "    state.accounts.on_spawn(email, p_id, &display);\n\
    state.players.insert(conn_id, p);\n\
    if let Some(mid) = mother_link {";
        let spawn_new = "    state.accounts.on_spawn(email, p_id, &display);\n\
    state.players.insert(conn_id, p);\n\
    // Haxe owning filled by InitObjectHelpersAfterRead; also re-scan on spawn.\n\
    rebuild_player_owning_from_world(state, p_id);\n\
    if let Some(mid) = mother_link {";
        if text.contains(spawn_old) {
            text = text.replacen(spawn_old, spawn_new, 1);
            changed = true;
        }
    }

    if !changed && text.contains("mod postload_wire;") {
        // Already partially wired.
        let out = restore_nl(&text, crlf);
        let _ = std::fs::write(lib_path, out);
        return text.contains("apply_init_object_helpers_after_read");
    }
    if !changed {
        return false;
    }
    let out = restore_nl(&text, crlf);
    std::fs::write(lib_path, out).is_ok()
}
