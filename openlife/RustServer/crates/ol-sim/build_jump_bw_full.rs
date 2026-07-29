//! Build-time wire for **JUMP-BW-FULL** / `jump_bw_full`.
//!
//! Ports Haxe `GlobalPlayerInstance.jump`, `MoveHelper.JumpToNonBlocked`,
//! exhausted jump say, client JUMP ignore-xy, AI JUMP! full path.
//! Idempotent: prefers Python apply script, pure-Rust fallback for mod + exports.

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

/// True when pure module + live apply + JUMP client wire markers present.
pub fn jump_bw_full_wired(lib_text: &str, jump_bw_exists: bool) -> bool {
    jump_bw_exists
        && lib_text.contains("mod jump_bw;")
        && lib_text.contains("plan_player_jump")
        && lib_text.contains("apply_player_jump")
        && lib_text.contains("JUMP-BW-FULL")
        && (lib_text.contains("try_jump_to_non_blocked")
            || lib_text.contains("JUMP_EXHAUSTED_SAY"))
}

fn try_run_python(script: &Path) -> bool {
    if !script.exists() {
        return false;
    }
    let py = Command::new("python")
        .arg(script)
        .status()
        .or_else(|_| Command::new("python3").arg(script).status())
        .or_else(|_| Command::new("py").arg("-3").arg(script).status());
    matches!(py, Ok(s) if s.success())
}

/// Apply JUMP-BW-FULL. Returns true when ready.
pub fn patch_jump_bw_full(src: &Path, workspace: &Path) -> bool {
    let lib_path = src.join("lib.rs");
    let jump_path = src.join("jump_bw.rs");
    let jump_exists = jump_path.exists();

    // Prefer Python one-shot (live JUMP/AI/MOVE + docs).
    let py = src.join("_apply_jump_bw_full.py");
    let _ = try_run_python(&py);
    let py2 = workspace.join("docs/port/_apply_jump_bw_full.py");
    let _ = try_run_python(&py2);

    let lib_t = std::fs::read_to_string(&lib_path).unwrap_or_default();
    if jump_bw_full_wired(&lib_t, jump_exists) {
        return true;
    }

    let _ = patch_lib_rs_minimal(&lib_path);
    let lib_f = std::fs::read_to_string(&lib_path).unwrap_or_default();
    jump_bw_full_wired(&lib_f, jump_path.exists())
}

/// Minimal pure-Rust: mod + re-exports only (live path needs Python or manual).
fn patch_lib_rs_minimal(lib_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let orig = t.clone();

    if !t.contains("mod jump_bw;") {
        if t.contains("mod move_path;") {
            t = t.replacen("mod move_path;", "mod move_path;\nmod jump_bw;", 1);
        }
    }

    if !t.contains("pub use jump_bw::") {
        let anchor = "pub use move_path::{\n";
        if t.contains(anchor) {
            let insert = concat!(
                "// JUMP-BW-FULL: GlobalPlayerInstance.jump + JumpToNonBlocked pure\n",
                "pub use jump_bw::{\n",
                "    jump_not_held_emits_bw, jump_should_say_exhausted, plan_jump_to_non_blocked,\n",
                "    plan_player_jump, JumpAction, JUMP_EXHAUSTED_SAY, JUMP_TO_NON_BLOCKED_OFFSETS,\n",
                "};\n",
            );
            t = t.replacen(anchor, &format!("{insert}{anchor}"), 1);
        }
    }

    if t != orig {
        let out = restore_nl(&t, crlf);
        let _ = std::fs::write(lib_path, out);
        return true;
    }
    t.contains("mod jump_bw;")
}
