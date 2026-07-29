//! Build-time wire for SCORE-ENTRY / score_disk (Haxe ScoreEntry.hx).

use std::path::Path;

pub fn score_entry_wired(lib: &str, death_polish: &str) -> bool {
    lib.contains("mod score_entry;")
        && lib.contains("save_score_entries")
        && lib.contains("process_player_score_entries")
        && death_polish.contains("create_score_entry_for_dead_relative")
}

/// Idempotent pure-Rust patches for score_entry module + death/tick wires.
pub fn patch_score_entry(src: &Path) -> bool {
    let lib_path = src.join("lib.rs");
    let dp_path = src.join("death_polish.rs");
    let mut ok = true;
    ok &= patch_lib(&lib_path);
    ok &= patch_death_polish(&dp_path);
    ok &= patch_account_persist(&src.join("account_persist.rs"));
    ok
}

fn patch_lib(lib_path: &Path) -> bool {
    let Ok(mut text) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    let mut changed = false;

    if !text.contains("mod score_entry;") {
        if text.contains("mod score;\n") {
            text = text.replacen(
                "mod score;\n",
                "mod score;\n// Haxe: openlife.server.ScoreEntry prestige queue + SES1 disk (SCORE-ENTRY)\nmod score_entry;\n",
                1,
            );
            changed = true;
        } else if text.contains("mod player_soul;\n") {
            text = text.replacen(
                "mod player_soul;\n",
                "mod player_soul;\nmod score_entry;\n",
                1,
            );
            changed = true;
        }
    }

    if !text.contains("pub use score_entry::") {
        let insert_after = "pub use score::{\n    compute_score, PrestigePlayerRow, PrestigeSnapshot, PrestigeView, ScoreEntry, Scoreboard,\n    SCORE_PER_DEATH, SCORE_PER_KILL,\n};\n";
        let export = "pub use score::{\n    compute_score, PrestigePlayerRow, PrestigeSnapshot, PrestigeView, ScoreEntry, Scoreboard,\n    SCORE_PER_DEATH, SCORE_PER_KILL,\n};\npub use score_entry::{\n    create_new_score_entry, create_score_entry_for_cursed_grave, create_score_entry_for_dead_relative,\n    create_score_entry_if_grave, format_global_message_text, grave_is_non_bone, load_score_entries,\n    process_score_entry, save_score_entries, should_process_score_entry, AccountScoreEntry,\n    DeadRelativePlayer, MotherLineNode, ProcessScoreResult, ANCESTOR_PRESTIGE_FACTOR,\n    CURSED_GRAVE_MALI, DEFAULT_SCORE_ENTRY_FILE, OLD_GRAVE_DECAY_MALI, OLD_GRAVE_OBJECT_ID,\n    SCORE_ENTRY_FORMAT_VERSION,\n};\n";
        if text.contains(insert_after) {
            text = text.replacen(insert_after, export, 1);
            changed = true;
        }
    }

    if !text.contains("AccountScoreEntry")
        && text.contains(
            "pub use accounts::{\n    normalize_email, AccountBook, AccountBookSnapshot, AccountRecord, AccountSummary, AccountView,\n};",
        )
    {
        text = text.replacen(
            "pub use accounts::{\n    normalize_email, AccountBook, AccountBookSnapshot, AccountRecord, AccountSummary, AccountView,\n};",
            "pub use accounts::{\n    normalize_email, AccountBook, AccountBookSnapshot, AccountRecord, AccountScoreEntry,\n    AccountSummary, AccountView,\n};",
            1,
        );
        changed = true;
    }

    if !text.contains("process_player_score_entries") {
        let marker =
            "    // Continuous breast-feeding (Haxe TimeHelper isHoldingChildInBreastFeedingAgeAndCanFeed).";
        if text.contains(marker) {
            let block = r#"    // Haxe TimeHelper → ScoreEntry.ProcessScoreEntry (trueAge % 5 == 0).
    process_player_score_entries(state, outbound);

    // Continuous breast-feeding (Haxe TimeHelper isHoldingChildInBreastFeedingAgeAndCanFeed)."#;
            text = text.replacen(marker, block, 1);
            changed = true;
        }
    }

    if !text.contains("fn process_player_score_entries") {
        let anchor = "fn apply_death_inheritance(state: &mut SimState, deceased_p_id: i32) {\n    death_polish::apply_death_polish(state, deceased_p_id);\n}\n";
        if text.contains(anchor) {
            let helper = r#"fn apply_death_inheritance(state: &mut SimState, deceased_p_id: i32) {
    death_polish::apply_death_polish(state, deceased_p_id);
}

/// Haxe `ScoreEntry.ProcessScoreEntry` for all living humans (age gate inside).
/// // Haxe: TimeHelper → ScoreEntry.ProcessScoreEntry
fn process_player_score_entries(state: &mut SimState, outbound: &OutboundHub) {
    let targets: Vec<(u64, i32, String, f32, f32)> = state
        .players
        .iter()
        .filter(|(_, p)| !p.deleted && p.connected)
        .map(|(&cid, p)| {
            let prestige = state
                .social
                .lineages
                .get(&p.p_id)
                .map(|n| n.prestige)
                .or_else(|| state.combat.stats.get(&p.p_id).map(|s| s.prestige))
                .unwrap_or(0.0);
            (cid, p.p_id, p.email.clone(), p.age, prestige)
        })
        .collect();
    for (cid, p_id, email, age, prestige) in targets {
        if !score_entry::should_process_score_entry(age) {
            continue;
        }
        let Some(result) = state.accounts.process_score_entry_for(&email, prestige) else {
            continue;
        };
        // Haxe addPrestige
        if let Some(n) = state.social.lineages.get_mut(&p_id) {
            n.add_prestige(result.prestige_delta);
        }
        if let Some(s) = state.combat.stats.get_mut(&p_id) {
            s.prestige = (s.prestige + result.prestige_delta).max(0.0);
        }
        let msg = score_entry::format_global_message_text(&result.message);
        if !msg.is_empty() {
            let pkt = format_server_message("GM", &[&msg]).into_bytes();
            outbound.send(cid, pkt);
        }
    }
}
"#;
            text = text.replacen(anchor, helper, 1);
            changed = true;
        }
    }

    if changed && std::fs::write(lib_path, text).is_err() {
        return false;
    }
    std::fs::read_to_string(lib_path)
        .map(|t| t.contains("mod score_entry;") && t.contains("process_player_score_entries"))
        .unwrap_or(false)
}

fn patch_death_polish(path: &Path) -> bool {
    let Ok(mut text) = std::fs::read_to_string(path) else {
        return false;
    };
    if text.contains("create_score_entry_for_dead_relative")
        && text.contains("fn apply_dead_relative_score_entry")
    {
        return true;
    }
    let mut changed = false;

    if text.contains(
        "//! 4. [`apply_inherit_coins`](crate::death_inherit::apply_inherit_coins) (+ grave residual)\n",
    ) && !text.contains("CreateScoreEntryForDeadRelative")
    {
        text = text.replacen(
            "//! 4. [`apply_inherit_coins`](crate::death_inherit::apply_inherit_coins) (+ grave residual)\n",
            "//! 4. [`apply_inherit_coins`](crate::death_inherit::apply_inherit_coins) (+ grave residual)\n//! 5. [`create_score_entry_for_dead_relative`](crate::score_entry) (SCORE-ENTRY)\n",
            1,
        );
        changed = true;
    }

    if !text.contains("use crate::score_entry::") {
        text = text.replacen(
            "use crate::relations::root_eve_id;\n",
            "use crate::relations::root_eve_id;\nuse crate::score_entry::{\n    create_score_entry_for_dead_relative, DeadRelativePlayer, MotherLineNode,\n    ANCESTOR_PRESTIGE_FACTOR,\n};\nuse crate::animal_move::is_bone_grave;\n",
            1,
        );
        changed = true;
    }

    let end_marker = "    if let (Some((x, y)), Some(g)) = (grave_xy, grave_helper) {\n        state.world.write().unwrap().set_object_complex(x, y, g);\n    }\n}\n\n#[cfg(test)]";
    let with_score = r#"    if let (Some((x, y)), Some(g)) = (grave_xy, grave_helper) {
        state.world.write().unwrap().set_object_complex(x, y, g);
    }

    // Haxe: ScoreEntry.CreateScoreEntryForDeadRelative(this)
    apply_dead_relative_score_entry(state, deceased_p_id, &deceased_email);
}

/// Haxe `ScoreEntry.CreateScoreEntryForDeadRelative` live wire.
/// // Haxe: ScoreEntry.CreateScoreEntryForDeadRelative
fn apply_dead_relative_score_entry(state: &mut SimState, deceased_p_id: i32, deceased_email: &str) {
    let prestige = state
        .social
        .lineages
        .get(&deceased_p_id)
        .map(|n| n.prestige)
        .or_else(|| state.combat.stats.get(&deceased_p_id).map(|s| s.prestige))
        .unwrap_or(0.0);
    let (name, family, mother_id) = {
        let p = state.players.values().find(|p| p.p_id == deceased_p_id);
        let name = p
            .map(|p| {
                if p.first_name.is_empty() {
                    format!("P{}", p.p_id)
                } else {
                    p.first_name.clone()
                }
            })
            .or_else(|| {
                state
                    .social
                    .lineages
                    .get(&deceased_p_id)
                    .map(|n| n.name.clone())
            })
            .unwrap_or_else(|| format!("P{deceased_p_id}"));
        let family = p
            .map(|p| p.family_name.clone())
            .unwrap_or_else(|| "SNOW".into());
        let mother = state
            .social
            .lineages
            .get(&deceased_p_id)
            .and_then(|n| n.mother_id);
        (name, family, mother)
    };
    let player = DeadRelativePlayer {
        p_id: deceased_p_id,
        account_email: deceased_email.to_string(),
        prestige,
        name,
        family_name: family,
        mother_lineage_id: mother_id,
    };

    // Snapshot lineage + account emails for pure walk.
    let lineages: HashMap<i32, (Option<i32>, String)> = state
        .social
        .lineages
        .iter()
        .map(|(&id, n)| {
            let email = email_for_lineage_id(state, id);
            (id, (n.mother_id, email))
        })
        .collect();
    let grave_flags: HashMap<String, bool> = {
        let world = state.world.read().unwrap();
        let mut m = HashMap::new();
        for (email, rec) in &state.accounts.by_email {
            let mut has_non_bone = false;
            for &(x, y) in &rec.graves {
                let id = world.get_object(x, y);
                if id > 0 && !is_bone_grave(id) {
                    has_non_bone = true;
                    break;
                }
            }
            m.insert(email.clone(), has_non_bone);
        }
        m
    };

    let mut seed = deceased_p_id
        .wrapping_mul(1103515245)
        .wrapping_add(state.sim_time.to_bits() as i32);
    let entry = create_score_entry_for_dead_relative(
        &player,
        &|id| {
            let (mother_id, email) = lineages.get(&id)?;
            Some(MotherLineNode {
                player_id: id,
                account_email: email.clone(),
                has_non_bone_grave: grave_flags.get(email).copied().unwrap_or(false),
                mother_id: *mother_id,
            })
        },
        || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let u = (seed as u32 >> 8) as f32 / 16_777_216.0;
            u.clamp(0.0, 0.999_999)
        },
        ANCESTOR_PRESTIGE_FACTOR,
    );
    if let Some(e) = entry {
        state.accounts.push_score_entry(e);
        state.push_event(format!("{deceased_p_id} SCORE_ENTRY ancestor award"));
    }
}

fn email_for_lineage_id(state: &SimState, p_id: i32) -> String {
    if let Some(p) = state.players.values().find(|p| p.p_id == p_id) {
        if !p.email.is_empty() {
            return p.email.clone();
        }
    }
    for r in state.accounts.by_email.values() {
        if r.last_p_id == p_id {
            return r.email.clone();
        }
    }
    format!("pid{p_id}@inherit.local")
}

#[cfg(test)]"#;

    if text.contains(end_marker) {
        text = text.replacen(end_marker, with_score, 1);
        changed = true;
    }

    if changed {
        let _ = std::fs::write(path, &text);
    }
    std::fs::read_to_string(path)
        .map(|t| t.contains("create_score_entry_for_dead_relative"))
        .unwrap_or(false)
}

fn patch_account_persist(path: &Path) -> bool {
    let Ok(mut text) = std::fs::read_to_string(path) else {
        return false;
    };
    if text.contains("score_entries: Vec::new()") {
        return true;
    }
    if text.contains("graves: Vec::new(),\n    })") {
        text = text.replacen(
            "graves: Vec::new(),\n    })",
            "graves: Vec::new(),\n        // Haxe ScoreEntry queue — SES1 via score_entry.rs (not OLA1).\n        score_entries: Vec::new(),\n    })",
            1,
        );
        let _ = std::fs::write(path, text);
    }
    std::fs::read_to_string(path)
        .map(|t| t.contains("score_entries: Vec::new()"))
        .unwrap_or(false)
}
