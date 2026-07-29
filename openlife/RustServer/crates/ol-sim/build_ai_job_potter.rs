//! AI-POTTER: wire `pottery_profession` + fix short_craft apply + sticky fields (idempotent).
//! Also applies **AI-POTTER-L2946** residual `doPotteryOnFire` crafts (pure RS).

use std::path::Path;
use std::process::Command;

// Pure-RS residual crafts (Haxe L2946 TODO make other potter stuff).
#[path = "build_ai_potter_l2946.rs"]
mod potter_l2946_impl;

pub fn already_wired(lib: &str) -> bool {
    lib.contains("mod pottery_profession;")
        && lib.contains("pub use pottery_profession::{")
        && lib.contains("do_pottery")
}

/// Replace broken short_craft body with include of smith-based apply (idempotent).
pub fn patch_pottery_short_craft(src_dir: &Path) -> bool {
    let path = src_dir.join("pottery_profession.rs");
    let Ok(mut text) = std::fs::read_to_string(&path) else {
        return false;
    };

    let include_line = "include!(\"pottery_action_apply.inc.rs\");\n";
    if !text.contains("include!(\"pottery_action_apply.inc.rs\")") {
        if let Some(start) = text
            .find("/// Map shortCraft pottery action through farmer-style apply when actor held.")
            .or_else(|| text.find("/// Map shortCraft pottery action for live USE/DROP"))
            .or_else(|| {
                text.find("pub fn pottery_action_short_craft_apply")
                    .map(|i| text[..i].rfind("///").unwrap_or(i))
            })
        {
            let end = text[start..]
                .find("\n// ── Tests")
                .or_else(|| text[start..].find("\n#[cfg(test)]"))
                .map(|rel| start + rel);
            if let Some(end) = end {
                text = format!("{}{}{}", &text[..start], include_line, &text[end..]);
            } else {
                return false;
            }
        }
    }

    text = text.replace(
        "let dist_deposit = if inp.has_clay_deposit {",
        "let _dist_deposit = if inp.has_clay_deposit {",
    );
    text = text.replace(
        "use crate::farmer_profession::ShortCraftApply;",
        "use crate::smith_profession::SmithApply;",
    );
    text = text.replace("ShortCraftApply::UseOnTarget", "SmithApply::UseOnTarget");
    text = text.replace(
        "ShortCraftApply::SeekOrCraftActor { actor: STONE }",
        "SmithApply::SeekOrCraftActor { actor: STONE }",
    );
    text = text.replace(
        "if counts.get(PILE_OF_CLAY) > 0 || true {\n            // Always attempt pile pull when gate says so (Haxe shortCraft searches).\n            runtime.stage = runtime.stage.max(2.0);\n            return PotteryAction::ShortCraft {\n                actor: 0,\n                target: PILE_OF_CLAY,\n            };\n        }",
        "// Always attempt pile pull when gate says so (Haxe shortCraft searches).\n        runtime.stage = runtime.stage.max(2.0);\n        return PotteryAction::ShortCraft {\n            actor: 0,\n            target: PILE_OF_CLAY,\n        };",
    );

    std::fs::write(&path, text).is_ok()
}

pub fn wire_lib(lib_path: &Path) -> bool {
    let Ok(mut text) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    if already_wired(&text) {
        return true;
    }

    if !text.contains("mod pottery_profession;") {
        if text.contains("mod baker_profession;") {
            text = text.replace(
                "mod baker_profession;\n",
                "mod baker_profession;\n// Haxe: AiBase potter profession family (AI-POTTER)\nmod pottery_profession;\n",
            );
        } else if text.contains("mod smith_profession;") {
            text = text.replace(
                "mod smith_profession;\n",
                "mod smith_profession;\n// Haxe: AiBase potter profession family (AI-POTTER)\nmod pottery_profession;\n",
            );
        } else {
            return false;
        }
    }

    // Note: do not re-export BASKET (292) — already pub-used from horse_mount.
    let use_block = r#"// Haxe: AiBase potter profession (AI-POTTER / pottery_job)
pub use pottery_profession::{
    apply_clay_source_to_gather_input, assign_potter_from_speech, count_potter_peers,
    count_potter_peers_filtered, decide_potter_job, do_pottery, do_pottery_on_fire_action,
    empty_basket_drop_is_deposit_staging, fill_pottery_counts_from_map, gather_clay,
    has_or_become_potter, has_or_become_potter_filtered, is_clay_source_id, is_kiln_id,
    kiln_id_priority, needed_pottery_clay, parse_potter_profession_speech,
    pick_closest_clay_source, pick_closest_clay_source_radius, pick_firing_kiln_near_home,
    pick_kiln_near_home, pick_potter_goal, potter_chebyshev, potter_goal_from_counts_and_rung,
    potter_goal_from_map_and_rung, potter_job_rung_label, potter_max_people_for_dispatch,
    potter_quad_dist, potter_radius_table, pottery_action_short_craft_apply, pottery_action_to_goal,
    pottery_on_fire_counts_from_pottery, resolve_potter_assigned_job, smith_pottery_action_to_pottery,
    try_decide_potter_from_rung, ClaySourceCandidate, GatherClayInput, KilnCandidate,
    PotterPeerSnapshot, PotterProfessionRuntime, PotteryAction, PotteryCounts, PotteryMapObj,
    ADOBE_KILN,
    CLAY, CLAY_DEPOSIT, CLAY_PIT, FIRING_ADOBE_KILN, GATHER_CLAY_FAR_QUAD, GATHER_CLAY_HOME_QUAD,
    GATHER_CLAY_MIN_STOCK, KILN_SEARCH_RADIUS, KILN_WITH_CHARCOAL, NEEDED_CLAY_CAP, PILE_OF_CLAY,
    POTTER_ASSIGNED_MAX_PEOPLE, POTTER_CRITICAL_MAX_PEOPLE, POTTER_DEFAULT_MAX_PEOPLE,
    POTTER_PROFESSION_KEY, POTTERY_CRAFT_SEARCH_RADIUS, SEALED_ADOBE_KILN, WOOD_FILLED_KILN,
};
"#;

    if !text.contains("pub use pottery_profession::{") {
        let anchors = ["pub use baker_profession::{", "pub use smith_profession::{"];
        let mut inserted = false;
        for anchor in anchors {
            if let Some(idx) = text.find(anchor) {
                if let Some(end_rel) = text[idx..].find("\n};\n") {
                    let end = idx + end_rel + "\n};\n".len();
                    text.insert_str(end, use_block);
                    inserted = true;
                    break;
                }
            }
        }
        if !inserted {
            return false;
        }
    }

    std::fs::write(lib_path, text).is_ok()
}

pub fn patch_ai_goals(ai_goals: &Path) -> bool {
    let Ok(mut text) = std::fs::read_to_string(ai_goals) else {
        return false;
    };
    let mut changed = false;

    if !text.contains("POTTER_TARGET_ID") {
        if text.contains("pub const BAKER_TARGET_ID: i32 = 273;") {
            text = text.replacen(
                "pub const BAKER_TARGET_ID: i32 = 273;",
                "pub const BAKER_TARGET_ID: i32 = 273;\n\n/// Default potter craft preference (Clay Bowl; preference only).\n///\n/// Haxe age-rotated pottery + doPottery bias toward bowls/plates.\npub const POTTER_TARGET_ID: i32 = 235;",
                1,
            );
            changed = true;
        }
    }

    if !text.contains("/// Pottery / kiln") {
        if text.contains("/// Baking / oven / pie role (Haxe `BAKER` / `doBaking`).\n    Baker,") {
            text = text.replace(
                "/// Baking / oven / pie role (Haxe `BAKER` / `doBaking`).\n    Baker,",
                "/// Baking / oven / pie role (Haxe `BAKER` / `doBaking`).\n    Baker,\n    /// Pottery / kiln / clay bowls (Haxe `POTTER` / `doPottery`).\n    Potter,",
            );
            changed = true;
        }
    }

    if !text.contains("\"POTTER\" | \"POTTERY\"") {
        if text.contains("\"BAKER\" | \"BAKE\" => Some(Profession::Baker),") {
            text = text.replace(
                "\"BAKER\" | \"BAKE\" => Some(Profession::Baker),",
                "\"BAKER\" | \"BAKE\" => Some(Profession::Baker),\n        \"POTTER\" | \"POTTERY\" => Some(Profession::Potter),",
            );
            changed = true;
        }
    }

    if !text.contains("Profession::Potter => Goal::SeekObject(POTTER_TARGET_ID)") {
        if text.contains("Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID),") {
            text = text.replace(
                "Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID),",
                "Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID),\n        Profession::Potter => Goal::SeekObject(POTTER_TARGET_ID),",
            );
            changed = true;
        }
    }

    if !text.contains("pick_potter_goal") {
        if text.contains(
            "return crate::baker_profession::pick_baker_goal(graph, have, stage);\n    }\n    g\n}",
        ) {
            text = text.replace(
                "return crate::baker_profession::pick_baker_goal(graph, have, stage);\n    }\n    g\n}",
                "return crate::baker_profession::pick_baker_goal(graph, have, stage);\n    }\n    // Potter: clay bowl/plate pipeline (AI-POTTER).\n    if profession == Profession::Potter\n        && held_id == 0\n        && food > HUNGRY_FOOD\n        && matches!(g, Goal::SeekObject(_) | Goal::Explore | Goal::Idle)\n        && !threat_near\n    {\n        return crate::pottery_profession::pick_potter_goal(graph, have);\n    }\n    g\n}",
            );
            changed = true;
        }
    }

    if !text.contains("parse_profession_token(\"potter\")") {
        if text.contains("assert_eq!(parse_profession_token(\"BAKE\"), Some(Profession::Baker));") {
            text = text.replace(
                "assert_eq!(parse_profession_token(\"BAKE\"), Some(Profession::Baker));",
                "assert_eq!(parse_profession_token(\"BAKE\"), Some(Profession::Baker));\n        assert_eq!(parse_profession_token(\"potter\"), Some(Profession::Potter));",
            );
            changed = true;
        }
    }

    if text.contains("Profession::Baker,\n            Profession::Explorer,")
        && !text.contains("Profession::Potter,\n            Profession::Explorer,")
    {
        text = text.replace(
            "Profession::Baker,\n            Profession::Explorer,",
            "Profession::Baker,\n            Profession::Potter,\n            Profession::Explorer,",
        );
        changed = true;
    }

    if changed {
        std::fs::write(ai_goals, text).is_ok()
    } else {
        text.contains("POTTER_TARGET_ID") && text.contains("Profession::Potter")
    }
}

pub fn patch_player(player: &Path) -> bool {
    let Ok(mut text) = std::fs::read_to_string(player) else {
        return false;
    };
    if text.contains("pub pottery_profession:") {
        return true;
    }

    let field = "    /// Sticky AI potter profession stage / last / assigned (Haxe `profession['POTTER']`).\n    // Haxe: AiBase.profession['POTTER'] + lastProfession / assignedProfession\n    pub pottery_profession: crate::PotterProfessionRuntime,\n";

    if !text.contains("pub baker_profession: crate::BakerProfessionRuntime,") {
        return false;
    }
    text = text.replace(
        "pub baker_profession: crate::BakerProfessionRuntime,\n",
        &format!("pub baker_profession: crate::BakerProfessionRuntime,\n{field}"),
    );
    text = text.replace(
        "baker_profession: crate::BakerProfessionRuntime::default(),\n",
        "baker_profession: crate::BakerProfessionRuntime::default(),\n            pottery_profession: crate::PotterProfessionRuntime::default(),\n",
    );

    if !text.contains("fn pottery_profession_sticky") {
        if let Some(idx) = text.find("fn smith_profession_sticky_defaults_and_survives") {
            let insert = r#"
    #[test]
    fn pottery_profession_sticky_defaults_and_survives() {
        let mut p = Player::new(1, 1, "potter@test");
        assert!(!p.pottery_profession.is_last_potter);
        assert_eq!(p.pottery_profession.stage, 0.0);
        assert!(crate::assign_potter_from_speech(&mut p.pottery_profession, "POTTER!"));
        assert!(p.pottery_profession.is_assigned_potter);
        assert!(p.pottery_profession.is_last_potter);
        p.pottery_profession.stage = 10.0;
        assert_eq!(p.pottery_profession.stage, 10.0);
        p.pottery_profession.wipe_on_eat(false);
        assert_eq!(p.pottery_profession.stage, 0.0);
        assert!(!p.pottery_profession.is_last_potter);
    }

"#;
            text.insert_str(idx, insert);
        }
    }

    std::fs::write(player, text).is_ok()
}

pub fn patch_priority_ladder(prio: &Path) -> bool {
    let Ok(mut text) = std::fs::read_to_string(prio) else {
        return false;
    };
    let mut changed = false;

    if !text.contains("POTTER_TARGET_ID") {
        if text.contains("BAKER_TARGET_ID, FARMER_TARGET_ID") {
            text = text.replace(
                "BAKER_TARGET_ID, FARMER_TARGET_ID",
                "BAKER_TARGET_ID, FARMER_TARGET_ID, POTTER_TARGET_ID",
            );
            changed = true;
        } else if text.contains("use super::{Goal, Profession, BAKER_TARGET_ID") {
            text = text.replace(
                "BAKER_TARGET_ID, FARMER_TARGET_ID, HUNGRY_FOOD, SMITH_TARGET_ID",
                "BAKER_TARGET_ID, FARMER_TARGET_ID, HUNGRY_FOOD, POTTER_TARGET_ID, SMITH_TARGET_ID",
            );
            changed = true;
        }
    }

    if !text.contains("Profession::Potter => Goal::SeekObject(POTTER_TARGET_ID)") {
        if text.contains("Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID),\n                Profession::Explorer => Goal::Explore,") {
            text = text.replace(
                "Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID),\n                Profession::Explorer => Goal::Explore,",
                "Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID),\n                Profession::Potter => Goal::SeekObject(POTTER_TARGET_ID),\n                Profession::Explorer => Goal::Explore,",
            );
            changed = true;
        }
    }

    if text.contains("Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID),\n            Profession::Hunter if prey_adjacent")
        && !text.contains("Profession::Potter => Goal::SeekObject(POTTER_TARGET_ID),\n            Profession::Hunter")
    {
        text = text.replace(
            "Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID),\n            Profession::Hunter if prey_adjacent",
            "Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID),\n            // Haxe: AssignedJob POTTER → doPottery(100); AgeRotated pottery → doPottery()\n            Profession::Potter => Goal::SeekObject(POTTER_TARGET_ID),\n            Profession::Hunter if prey_adjacent",
        );
        changed = true;
    }

    if changed {
        std::fs::write(prio, text).is_ok()
    } else {
        text.contains("Profession::Potter") || text.contains("_ => Goal::Explore")
    }
}

pub fn patch_profession_scan(scan: &Path) -> bool {
    let Ok(mut text) = std::fs::read_to_string(scan) else {
        return false;
    };
    if text.contains("// Residual: pottery / sheep have no ProfessionScanKind yet.") {
        text = text.replace(
            "// Residual: pottery / sheep have no ProfessionScanKind yet.\n        AgeRotatedJobKind::Pottery | AgeRotatedJobKind::SheepHerding => None,",
            "// Residual: pottery pure SM in pottery_profession (AI-POTTER); scan USE I/O open.\n        // Sheep still no ProfessionScanKind.\n        AgeRotatedJobKind::Pottery | AgeRotatedJobKind::SheepHerding => None,",
        );
        let _ = std::fs::write(scan, text);
    }
    true
}

pub fn patch_todo_port(todo: &Path) -> bool {
    let Ok(mut text) = std::fs::read_to_string(todo) else {
        return false;
    };
    if text.contains("AI-POTTER pottery_job") {
        return true;
    }
    if text.contains("- [ ] Profession state machine (potter, …; farmer/smith/baker pure SM above)") {
        text = text.replace(
            "- [ ] Profession state machine (potter, …; farmer/smith/baker pure SM above)  \n",
            "- [~] **AI-POTTER pottery_job PARTIAL** — pure `pottery_profession.rs` doPottery/gatherClay/onFire + sticky `Player.pottery_profession` + Profession::Potter; residual live profession_scan USE I/O / nested basket / other crafts  \n- [ ] Profession state machine (shepherd, …; farmer/smith/baker/potter pure SM above)  \n",
        );
        let _ = std::fs::write(todo, text);
        return true;
    }
    false
}

/// AI-POTTER-L2946 residual crafts — pure RS first, python fallback.
fn patch_ai_potter_l2946(src: &Path) -> bool {
    if potter_l2946_impl::patch_all(src) {
        let smith = std::fs::read_to_string(src.join("smith_profession.rs")).unwrap_or_default();
        if potter_l2946_impl::already_wired(&smith) {
            return true;
        }
    }
    // Python fallback
    let py = src.join("_patch_ai_potter_l2946.py");
    if py.exists() {
        let st = Command::new("python")
            .arg(&py)
            .status()
            .or_else(|_| Command::new("python3").arg(&py).status());
        return st.map(|s| s.success()).unwrap_or(false);
    }
    false
}

pub fn patch_all(src: &Path, lib_path: &Path) -> bool {
    let mut ok = true;
    if !patch_pottery_short_craft(src) {
        println!("cargo:warning=AI-POTTER: short_craft patch skipped/failed");
        ok = false;
    }
    if !wire_lib(lib_path) {
        println!("cargo:warning=AI-POTTER: lib.rs wire failed");
        ok = false;
    }
    if !patch_ai_goals(&src.join("ai_goals.rs")) {
        println!("cargo:warning=AI-POTTER: ai_goals patch incomplete");
        ok = false;
    }
    if !patch_player(&src.join("player.rs")) {
        println!("cargo:warning=AI-POTTER: player sticky patch failed");
        ok = false;
    }
    if !patch_priority_ladder(&src.join("priority_ladder.rs")) {
        println!("cargo:warning=AI-POTTER: priority_ladder Potter arm incomplete");
        ok = false;
    }
    let _ = patch_profession_scan(&src.join("profession_scan.rs"));
    // docs/port/TODO_PORT.md relative to ol-sim crate: ../../docs/port
    if let Some(crate_root) = src.parent() {
        if let Some(ws) = crate_root.parent().and_then(|p| p.parent()) {
            let _ = patch_todo_port(&ws.join("docs/port/TODO_PORT.md"));
        }
    }
    // AI-POTTER-L2946 residual doPotteryOnFire other crafts (Haxe L2946 TODO).
    if !patch_ai_potter_l2946(src) {
        println!(
            "cargo:warning=AI-POTTER-L2946: residual crafts not applied — run src/_patch_ai_potter_l2946.py"
        );
    }
    ok
}
