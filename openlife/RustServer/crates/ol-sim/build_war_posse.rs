//! Build-time wire for **SOCIAL-WAR-PERSIST / war_posse_disk** (WPS1).
//!
//! Ensures:
//! - `mod war_posse_persist` + pub use
//! - `SimBootLive.war_posse_share`
//! - sim boot seed + periodic / shutdown mirror
//! - ol-config `war_posse_save_path`
//! - ol-server boot load + autosave + shutdown save
//!
//! Idempotent. Handles CRLF sources.

use std::path::Path;

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

pub fn war_posse_wired(lib: &str, settings_live: &str, config: &str, main: &str) -> bool {
    let disconnect_mirrors = lib.matches("mirror_war_posse_share(&state, &war_posse_share)").count();
    lib.contains("mod war_posse_persist;")
        && lib.contains("save_war_posse")
        && lib.contains("load_war_posse")
        && lib.contains("war_posse_share")
        && lib.contains("mirror_war_posse_share")
        && disconnect_mirrors >= 3 // try_recv disconnect + select None + periodic
        && settings_live.contains("war_posse_share")
        && config.contains("war_posse_save_path")
        && main.contains("load_war_posse")
        && main.contains("save_war_posse")
        && main.contains("war/posse autosaved")
        && main.contains("let war_posse = Arc::clone(&shared_war_posse)")
}

/// Patch ol-sim + ol-config + ol-server. Returns true when fully ready.
pub fn patch_war_posse(src_dir: &Path, workspace: &Path) -> bool {
    let lib_ok = patch_lib(&src_dir.join("lib.rs"));
    let sl_ok = patch_settings_live(&src_dir.join("settings_live.rs"));
    let cfg_path = workspace.join("crates/ol-config/src/lib.rs");
    let cfg_ok = if cfg_path.exists() {
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

    let lib = std::fs::read_to_string(src_dir.join("lib.rs")).unwrap_or_default();
    let sl = std::fs::read_to_string(src_dir.join("settings_live.rs")).unwrap_or_default();
    let cfg = std::fs::read_to_string(&cfg_path).unwrap_or_default();
    let main = std::fs::read_to_string(&main_path).unwrap_or_default();
    war_posse_wired(&lib, &sl, &cfg, &main) || (lib_ok && sl_ok && cfg_ok && main_ok)
}

fn patch_lib(lib_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);
    let mut changed = false;

    // mod
    if !text.contains("mod war_posse_persist;") {
        if text.contains("mod war;\nmod weather;\n") {
            text = text.replacen(
                "mod war;\nmod weather;\n",
                "mod war;\n// SOCIAL-WAR-PERSIST: WPS1 war/posse session disk (war_posse_disk)\nmod war_posse_persist;\nmod weather;\n",
                1,
            );
            changed = true;
        } else if let Some(idx) = text.find("mod war;") {
            let end = idx + "mod war;".len();
            text.insert_str(
                end,
                "\n// SOCIAL-WAR-PERSIST: WPS1 war/posse session disk (war_posse_disk)\nmod war_posse_persist;",
            );
            changed = true;
        } else {
            return false;
        }
    }

    // pub use
    if !text.contains("pub use war_posse_persist::") {
        let needle = "pub use war::{\n    format_war_report, pair_key, WarState, STATUS_ALLIANCE, STATUS_PEACE, STATUS_WAR,\n};\n";
        let export = "pub use war::{\n    format_war_report, pair_key, WarState, STATUS_ALLIANCE, STATUS_PEACE, STATUS_WAR,\n};\npub use war_posse_persist::{\n    apply_war_posse_snapshot, capture_war_posse_snapshot, load_war_posse, save_war_posse,\n    WarPosseShare, WarPosseSnapshot, DEFAULT_WAR_POSSE_FILE, WAR_POSSE_FORMAT_VERSION,\n};\n";
        if text.contains(needle) {
            text = text.replacen(needle, export, 1);
            changed = true;
        } else if text.contains("pub use war::{") {
            if let Some(idx) = text.find("pub use war::{") {
                if let Some(end_rel) = text[idx..].find("};\n") {
                    let end = idx + end_rel + "};\n".len();
                    text.insert_str(
                        end,
                        "pub use war_posse_persist::{\n    apply_war_posse_snapshot, capture_war_posse_snapshot, load_war_posse, save_war_posse,\n    WarPosseShare, WarPosseSnapshot, DEFAULT_WAR_POSSE_FILE, WAR_POSSE_FORMAT_VERSION,\n};\n",
                    );
                    changed = true;
                }
            }
        }
    }

    // Capture share from boot_live before boot is moved.
    if !text.contains("let war_posse_share = boot_live")
        && !text.contains("let war_posse_share =")
    {
        let needle =
            "    let mut hot_reload = None;\n    let live_share = boot_live.as_ref().and_then(|b| b.live_share.clone());\n    if let Some(boot) = boot_live {";
        let insert = "    let mut hot_reload = None;\n    let live_share = boot_live.as_ref().and_then(|b| b.live_share.clone());\n    // SOCIAL-WAR-PERSIST: shared war/posse for autosave mirror (WPS1).\n    let war_posse_share = boot_live.as_ref().and_then(|b| b.war_posse_share.clone());\n    if let Some(boot) = boot_live {";
        if text.contains(needle) {
            text = text.replacen(needle, insert, 1);
            changed = true;
        }
    }

    // Seed war/posse after accounts seed.
    if !text.contains("apply_war_posse_snapshot(&mut state.war") {
        let needle = "            \"sim: loaded accounts from shared book\"\n        );\n    }\n    // Haxe ObjectHelper.InitObjectHelpersAfterRead — after world+accounts loaded.";
        let insert = "            \"sim: loaded accounts from shared book\"\n        );\n    }\n    // SOCIAL-WAR-PERSIST: seed war/posse from boot-loaded WPS1 share.\n    if let Some(ref share) = war_posse_share {\n        let snap = share.read().unwrap().clone();\n        let (wars, posse_edges) = snap.counts();\n        apply_war_posse_snapshot(&mut state.war, &mut state.posse, &snap);\n        if wars > 0 || posse_edges > 0 {\n            info!(\n                wars,\n                posse_edges,\n                \"sim: loaded war/posse from shared WPS1\"\n            );\n        }\n    }\n    // Haxe ObjectHelper.InitObjectHelpersAfterRead — after world+accounts loaded.";
        if text.contains(needle) {
            text = text.replacen(needle, insert, 1);
            changed = true;
        }
    }

    // Mirror helper
    if !text.contains("fn mirror_war_posse_share") {
        let anchor = "fn apply_death_inheritance(state: &mut SimState, deceased_p_id: i32) {\n    death_polish::apply_death_polish(state, deceased_p_id);\n}\n";
        if text.contains(anchor) {
            let helper = r#"fn apply_death_inheritance(state: &mut SimState, deceased_p_id: i32) {
    death_polish::apply_death_polish(state, deceased_p_id);
}

/// Mirror live war/posse into outer autosave Arc (SOCIAL-WAR-PERSIST).
fn mirror_war_posse_share(state: &SimState, share: &Option<WarPosseShare>) {
    if let Some(ref s) = share {
        if let Ok(mut g) = s.write() {
            *g = capture_war_posse_snapshot(&state.war, &state.posse);
        }
    }
}

"#;
            text = text.replacen(anchor, helper, 1);
            changed = true;
        }
    }

    // Always patch any remaining disconnect sites that clone accounts then stop without mirror.
    // Pattern: accounts clone + intent closed, without a mirror line immediately before info.
    let bare_stop = "if let Some(ref shared) = shared_accounts {\n                        *shared.write().unwrap() = state.accounts.clone();\n                    }\n                    info!(\"intent channel closed; sim stopping\");";
    let with_mirror = "if let Some(ref shared) = shared_accounts {\n                        *shared.write().unwrap() = state.accounts.clone();\n                    }\n                    mirror_war_posse_share(&state, &war_posse_share);\n                    info!(\"intent channel closed; sim stopping\");";
    if text.contains(bare_stop) {
        text = text.replace(bare_stop, with_mirror);
        changed = true;
    }
    // Second indent style (select None branch, fewer spaces)
    let bare_stop2 = "if let Some(ref shared) = shared_accounts {\n                                *shared.write().unwrap() = state.accounts.clone();\n                            }\n                            info!(\"intent channel closed; sim stopping\");";
    let with_mirror2 = "if let Some(ref shared) = shared_accounts {\n                                *shared.write().unwrap() = state.accounts.clone();\n                            }\n                            mirror_war_posse_share(&state, &war_posse_share);\n                            info!(\"intent channel closed; sim stopping\");";
    if text.contains(bare_stop2) {
        text = text.replace(bare_stop2, with_mirror2);
        changed = true;
    }

    // Periodic mirror
    let pattern_b = "            if let Some(ref shared) = shared_accounts {\n                *shared.write().unwrap() = state.accounts.clone();\n            }\n        }\n\n        if state.tick.saturating_sub(last_skip_log) >= 200 {";
    let replace_b = "            if let Some(ref shared) = shared_accounts {\n                *shared.write().unwrap() = state.accounts.clone();\n            }\n            mirror_war_posse_share(&state, &war_posse_share);\n        }\n\n        if state.tick.saturating_sub(last_skip_log) >= 200 {";
    if text.contains(pattern_b) {
        text = text.replacen(pattern_b, replace_b, 1);
        changed = true;
    }

    if changed {
        let out = restore_nl(&text, crlf);
        if std::fs::write(lib_path, out).is_err() {
            return false;
        }
    }
    std::fs::read_to_string(lib_path)
        .map(|t| t.contains("mod war_posse_persist;") && t.contains("save_war_posse"))
        .unwrap_or(false)
}

fn patch_settings_live(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("war_posse_share") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);
    let mut changed = false;

    if !text.contains("WarPosseShare") {
        let needle = "use crate::environment::Season;\nuse crate::SimState;\n";
        let insert = "use crate::environment::Season;\nuse crate::war_posse_persist::WarPosseShare;\nuse crate::SimState;\n";
        if text.contains(needle) {
            text = text.replacen(needle, insert, 1);
            changed = true;
        }
    }

    if !text.contains("pub war_posse_share:") {
        let needle = "    /// Haxe `EternalWinter` at boot.\n    pub eternal_winter: bool,\n}\n";
        let insert = "    /// Haxe `EternalWinter` at boot.\n    pub eternal_winter: bool,\n    /// SOCIAL-WAR-PERSIST: shared war/posse snapshot for autosave (WPS1).\n    pub war_posse_share: Option<WarPosseShare>,\n}\n";
        if text.contains(needle) {
            text = text.replacen(needle, insert, 1);
            changed = true;
        }
    }

    if !text.contains("war_posse_share: None") {
        let needle = "            season_length_secs: 450.0,\n            eternal_winter: false,\n        }\n    }\n}";
        let insert = "            season_length_secs: 450.0,\n            eternal_winter: false,\n            war_posse_share: None,\n        }\n    }\n}";
        if text.contains(needle) {
            text = text.replacen(needle, insert, 1);
            changed = true;
        }
    }

    if changed {
        let out = restore_nl(&text, crlf);
        if std::fs::write(path, out).is_err() {
            return false;
        }
    }
    std::fs::read_to_string(path)
        .map(|t| t.contains("war_posse_share"))
        .unwrap_or(false)
}

fn patch_config(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("war_posse_save_path") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);

    let needle = "    /// Score-entry prestige queue (`SES1` / `score_entries_v1.bin`).\n    /// Haxe ScoreEntry had TODO save-to-disk; Rust SES1 is separate from OLA1.\n    pub fn score_entries_save_path(&self) -> PathBuf {\n        self.save_directory.join(\"score_entries_v1.bin\")\n    }\n";
    let insert = "    /// Score-entry prestige queue (`SES1` / `score_entries_v1.bin`).\n    /// Haxe ScoreEntry had TODO save-to-disk; Rust SES1 is separate from OLA1.\n    pub fn score_entries_save_path(&self) -> PathBuf {\n        self.save_directory.join(\"score_entries_v1.bin\")\n    }\n\n    /// Session war/posse (`WPS1` / `war_posse_v1.bin`).\n    /// Haxe had no disk for WAR/POSSE; Rust WPS1 keeps session maps across restart.\n    pub fn war_posse_save_path(&self) -> PathBuf {\n        self.save_directory.join(\"war_posse_v1.bin\")\n    }\n";
    if text.contains(needle) {
        text = text.replacen(needle, insert, 1);
        let out = restore_nl(&text, crlf);
        return std::fs::write(path, out).is_ok();
    }
    if text.contains("score_entries_v1.bin") && !text.contains("war_posse_save_path") {
        if let Some(idx) = text.find("pub fn score_entries_save_path") {
            if let Some(end_rel) = text[idx..].find("\n    }\n\n    /// Self-play") {
                let end = idx + end_rel + "\n    }\n".len();
                text.insert_str(
                    end,
                    "\n    /// Session war/posse (`WPS1` / `war_posse_v1.bin`).\n    /// Haxe had no disk for WAR/POSSE; Rust WPS1 keeps session maps across restart.\n    pub fn war_posse_save_path(&self) -> PathBuf {\n        self.save_directory.join(\"war_posse_v1.bin\")\n    }\n",
                );
                let out = restore_nl(&text, crlf);
                return std::fs::write(path, out).is_ok();
            }
        }
    }
    false
}

fn patch_main(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);
    let mut changed = false;

    // imports
    if !text.contains("load_war_posse") {
        if text.contains("load_score_entries") {
            text = text.replacen(
                "load_score_entries,\n",
                "load_score_entries, load_war_posse,\n",
                1,
            );
            changed = true;
        }
        if text.contains("save_score_entries, AccountBook,") {
            text = text.replacen(
                "save_score_entries, AccountBook,",
                "save_score_entries, save_war_posse, AccountBook,",
                1,
            );
            changed = true;
        } else if text.contains("save_score_entries,") && !text.contains("save_war_posse") {
            text = text.replacen("save_score_entries,", "save_score_entries, save_war_posse,", 1);
            changed = true;
        }
        if !text.contains("WarPosseSnapshot") {
            if text.contains("TwinRegistry, WeatherSnapshot,") {
                text = text.replacen(
                    "TwinRegistry, WeatherSnapshot,",
                    "TwinRegistry, WarPosseSnapshot, WeatherSnapshot,",
                    1,
                );
            } else {
                text = text.replacen(
                    "WeatherSnapshot,",
                    "WarPosseSnapshot, WeatherSnapshot,",
                    1,
                );
            }
            changed = true;
        }
    }

    // Boot load
    if !text.contains("shared_war_posse") {
        let needle = "    // Seed account web view from boot-loaded book so /api/accounts works before first tick.\n    let account_view = Arc::new(RwLock::new(shared_accounts.read().unwrap().snapshot()));";
        let insert = r#"    // SOCIAL-WAR-PERSIST: WPS1 session war/posse (Haxe had no disk).
    let shared_war_posse = Arc::new(RwLock::new({
        let wps_path = cfg.war_posse_save_path();
        match load_war_posse(&wps_path) {
            Ok(snap) => {
                let (wars, posse_edges) = snap.counts();
                if wps_path.exists() {
                    info!(
                        path = %wps_path.display(),
                        wars,
                        posse_edges,
                        "loaded war/posse (WPS1)"
                    );
                } else {
                    info!(
                        path = %wps_path.display(),
                        "no war/posse save — empty session maps"
                    );
                }
                snap
            }
            Err(e) => {
                warn!(
                    error = %e,
                    path = %wps_path.display(),
                    "war/posse load failed; empty session maps"
                );
                WarPosseSnapshot::default()
            }
        }
    }));
    // Seed account web view from boot-loaded book so /api/accounts works before first tick.
    let account_view = Arc::new(RwLock::new(shared_accounts.read().unwrap().snapshot()));"#;
        if text.contains(needle) {
            text = text.replacen(needle, insert, 1);
            changed = true;
        }
    }

    // Attach share to SimBootLive
    if text.contains("shared_war_posse") && !text.contains("war_posse_share:") {
        if text.contains("eternal_winter: live0.eternal_winter,\n        };") {
            text = text.replacen(
                "eternal_winter: live0.eternal_winter,\n        };",
                "eternal_winter: live0.eternal_winter,\n            // SOCIAL-WAR-PERSIST: WPS1 share for sim seed + autosave mirror\n            war_posse_share: Some(Arc::clone(&shared_war_posse)),\n        };",
                1,
            );
            changed = true;
        }
    }

    // Autosave: clone arc into task
    if text.contains("shared_war_posse")
        && !text.contains("let war_posse = Arc::clone(&shared_war_posse)")
    {
        let needle = "        let score_entries_save = cfg.score_entries_save_path();\n        handles.push(tokio::spawn(async move {";
        let insert = "        let score_entries_save = cfg.score_entries_save_path();\n        let war_posse = Arc::clone(&shared_war_posse);\n        let war_posse_save = cfg.war_posse_save_path();\n        handles.push(tokio::spawn(async move {";
        if text.contains(needle) {
            text = text.replacen(needle, insert, 1);
            changed = true;
        }
    }

    // Autosave write body
    if text.contains("shared_war_posse") && !text.contains("war/posse autosaved") {
        let needle = "                // SES1 prestige queue (Haxe ScoreEntry disk TODO).\n                if let Err(e) = save_score_entries(&accounts_snap, &score_entries_save) {\n                    warn!(error = %e, \"score-entry autosave failed\");\n                } else {\n                    any_ok = true;\n                    info!(\n                        path = %score_entries_save.display(),\n                        \"score entries autosaved (SES1)\"\n                    );\n                }\n                if any_ok {";
        let insert = "                // SES1 prestige queue (Haxe ScoreEntry disk TODO).\n                if let Err(e) = save_score_entries(&accounts_snap, &score_entries_save) {\n                    warn!(error = %e, \"score-entry autosave failed\");\n                } else {\n                    any_ok = true;\n                    info!(\n                        path = %score_entries_save.display(),\n                        \"score entries autosaved (SES1)\"\n                    );\n                }\n                // WPS1 session war/posse (SOCIAL-WAR-PERSIST).\n                let war_posse_snap = war_posse.read().unwrap().clone();\n                if let Err(e) = save_war_posse(&war_posse_snap, &war_posse_save) {\n                    warn!(error = %e, \"war/posse autosave failed\");\n                } else {\n                    any_ok = true;\n                    let (wars, posse_edges) = war_posse_snap.counts();\n                    info!(\n                        path = %war_posse_save.display(),\n                        wars,\n                        posse_edges,\n                        \"war/posse autosaved (WPS1)\"\n                    );\n                }\n                if any_ok {";
        if text.contains(needle) {
            text = text.replacen(needle, insert, 1);
            changed = true;
        }
    }

    // Shutdown save
    if text.contains("shared_war_posse") && !text.contains("war/posse saved on shutdown") {
        let needle = "        if let Err(e) = save_score_entries(&*accounts, cfg.score_entries_save_path()) {\n            warn!(error = %e, \"score-entry shutdown save failed\");\n        } else {\n            info!(\n                path = %cfg.score_entries_save_path().display(),\n                \"score entries saved on shutdown (SES1)\"\n            );\n        }\n    }\n\n    for h in handles {";
        let insert = "        if let Err(e) = save_score_entries(&*accounts, cfg.score_entries_save_path()) {\n            warn!(error = %e, \"score-entry shutdown save failed\");\n        } else {\n            info!(\n                path = %cfg.score_entries_save_path().display(),\n                \"score entries saved on shutdown (SES1)\"\n            );\n        }\n    }\n    {\n        let snap = shared_war_posse.read().unwrap().clone();\n        if let Err(e) = save_war_posse(&snap, cfg.war_posse_save_path()) {\n            warn!(error = %e, \"war/posse shutdown save failed\");\n        } else {\n            let (wars, posse_edges) = snap.counts();\n            info!(\n                path = %cfg.war_posse_save_path().display(),\n                wars,\n                posse_edges,\n                \"war/posse saved on shutdown (WPS1)\"\n            );\n        }\n    }\n\n    for h in handles {";
        if text.contains(needle) {
            text = text.replacen(needle, insert, 1);
            changed = true;
        }
    }

    if changed {
        let out = restore_nl(&text, crlf);
        if std::fs::write(path, out).is_err() {
            return false;
        }
    }
    std::fs::read_to_string(path)
        .map(|t| {
            t.contains("load_war_posse")
                && t.contains("save_war_posse")
                && t.contains("war/posse autosaved")
        })
        .unwrap_or(false)
}
