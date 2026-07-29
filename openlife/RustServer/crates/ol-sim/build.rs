//! Build-time stamps + idempotent source wires (GPI-DEATH, TIME-LONG, TIME-WORLD-POLISH, S-MOVE, MUTE-SAY, AI-PRIO, AI-JOB-SMITH, AI-JOB-BAKER, AI-POTTER, AI-JOB-FARM-WIRE, NESTED-OLW1-POLISH, CONFIG-SETTINGS, MAP-TEMP-PLAYER, CONTAINED-TIMERS-PERSIST, S-MOVE-LIVE-GATES, TIME-ANIMAL-OFFSPRING, TH-CLOTHING-MATRIX, SCORE-ENTRY, LOCKPICK-SETTINGS, SOCIAL-WAR-PERSIST, CRAFT-LIVE-TICK, BREASTFEED-EDGES, REPUTATION-HIT, CLASS-BONI, EVE-BANANA, SEARCH-BEST-FOOD, DO-COMMANDS, AI-HANDLING-FIRE, AI-CRAFT-DUAL).
//!
//! Applies patches in pure Rust so a plain `cargo test -p ol-sim` wires sources
//! without a separate Python step.

mod build_ai_prio;
mod build_ai_job_smith;
mod build_ai_job_baker;
mod build_ai_job_farm_wire;
mod build_ai_job_potter;
mod build_ai_handling_fire;
mod build_animal_pop;
mod build_gpi_death_polish;
mod build_mute_say;
mod build_s_move;
mod build_s_move_live_gates;
mod build_nested_postload;
mod build_settings_live;
mod build_map_temp_player;
mod build_contained_timers;
mod build_clothing_transitions;
mod build_score_entry;
mod build_soul_wire;
mod build_lockpick_settings;
mod build_war_posse;
mod build_craft_live_tick;
mod build_breastfeed_edges;
mod build_reputation_hit;
mod build_prestige_ally_cost;
mod build_class_boni;
mod build_eve_banana;
mod build_search_best_food;
mod build_do_commands;
mod build_twin_party_resid;
mod build_ai_craft_dual;
mod build_dark_nosaj;
mod build_th_alt_outcome;

use build_ai_job_baker::{
    baker_job_wired, patch_lib_ai_job_baker, patch_priority_ladder_baker, patch_selfplay_baker,
};
use build_ai_job_farm_wire::{
    farm_wire_wired, patch_all_farm_wire, patch_farmer_doc, patch_lib_ai_job_farm_wire,
    patch_priority_ladder_farm,
};
use build_ai_job_potter::{already_wired as potter_job_wired, patch_all as patch_all_ai_job_potter};
use build_ai_job_smith::{patch_lib_ai_job_smith, patch_selfplay_smith, smith_job_wired};
use build_ai_prio::{ai_prio_wired, patch_lib_ai_prio};
use build_ai_handling_fire::{already_wired as handling_fire_wired, patch_all as patch_all_handling_fire};
use build_animal_pop::{animal_pop_wired, patch_animal_pop};
use build_breastfeed_edges::{breastfeed_edges_wired, patch_breastfeed_edges};
use build_class_boni::{class_boni_wired, patch_class_boni};
use build_clothing_transitions::{clothing_transitions_wired, patch_clothing_transitions};
use build_contained_timers::patch_contained_timers;
use build_craft_live_tick::{craft_live_tick_wired, patch_all_craft_live_tick};
use build_eve_banana::{eve_banana_wired, patch_eve_banana};
use build_gpi_death_polish::{death_polish_wired, patch_lib_gpi_death_polish};
use build_lockpick_settings::patch_lockpick_settings;
use build_map_temp_player::{map_temp_player_wired, patch_lib_map_temp_player};
use build_mute_say::{mute_say_wired, patch_lib_mute_say};
use build_nested_postload::patch_nested_postload;
use build_reputation_hit::{patch_reputation_hit, reputation_hit_wired};
use build_prestige_ally_cost::patch_prestige_ally_cost;
use build_s_move::patch_lib_s_move_road_floor;
use build_s_move_live_gates::{live_gates_wired, patch_s_move_live_gates};
use build_score_entry::{patch_score_entry, score_entry_wired};
use build_search_best_food::{patch_search_best_food, search_best_food_wired};
use build_do_commands::{do_commands_wired, patch_do_commands};
use build_twin_party_resid::{patch_twin_party_resid, twin_party_resid_wired};
use build_ai_craft_dual::{already_wired as craft_dual_wired, patch_all as patch_all_ai_craft_dual};
use build_dark_nosaj::{dark_nosaj_wired, heal_dark_nosaj_stacking, patch_dark_nosaj};
use build_th_alt_outcome::{patch_th_alt_outcome, th_alt_outcome_wired};
use build_settings_live::patch_settings_live;
use build_war_posse::{patch_war_posse, war_posse_wired};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Legacy GPI-DEATH coins wire (source now ships wired; keep stub for fallback path).
fn patch_lib_gpi_death(lib_path: &Path) -> bool {
    std::fs::read_to_string(lib_path)
        .map(|t| {
            t.contains("mod death_inherit;")
                && (t.contains("apply_inherit_coins")
                    || t.contains("death_polish::apply_death_polish"))
                && t.contains("hunger_death_wire")
        })
        .unwrap_or(false)
}

/// TIME-LONG long_term module is source-wired; no-op self-contain patch.
fn patch_long_term_self_contained(_lt_path: &Path) -> bool {
    true
}

/// TIME-LONG lib wire check (source already has mod + call sites).
fn patch_lib_time_long(lib_path: &Path) -> bool {
    std::fs::read_to_string(lib_path)
        .map(|t| t.contains("mod long_term;") && t.contains("do_world_long_term_time_stuff"))
        .unwrap_or(false)
}

/// TIME-WORLD-POLISH markers already in tree.
fn text_has_world_polish(t: &str) -> bool {
    t.contains("ground_id") || t.contains("transform_target") || t.contains("world_time")
}

fn patch_lib_time_world_polish(lib_path: &Path) -> bool {
    std::fs::read_to_string(lib_path)
        .map(|t| text_has_world_polish(&t))
        .unwrap_or(false)
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest.join("src");
    let workspace = manifest
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| manifest.clone());

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build_ai_craft_dual.rs");
    println!("cargo:rerun-if-changed=src/craft_dual_center.inc.rs");
    println!("cargo:rerun-if-changed=src/craft_dual_center.rs");
    println!("cargo:rerun-if-changed=src/_apply_ai_craft_dual.py");
    println!("cargo:rerun-if-changed=src/craft_item.rs");
    println!("cargo:rerun-if-changed=src/craft_topdown.rs");
    println!("cargo:rerun-if-changed=build_ai_handling_fire.rs");
    println!("cargo:rerun-if-changed=src/handling_fire.rs");
    println!("cargo:rerun-if-changed=src/handling_fire_live.inc.rs");
    println!("cargo:rerun-if-changed=src/_run_ai_handling_fire_wire.py");
    println!("cargo:rerun-if-changed=build_s_move.rs");
    println!("cargo:rerun-if-changed=build_s_move_live_gates.rs");
    println!("cargo:rerun-if-changed=build_mute_say.rs");
    println!("cargo:rerun-if-changed=build_ai_prio.rs");
    println!("cargo:rerun-if-changed=build_ai_job_smith.rs");
    println!("cargo:rerun-if-changed=build_ai_job_baker.rs");
    println!("cargo:rerun-if-changed=build_ai_job_farm_wire.rs");
    println!("cargo:rerun-if-changed=build_ai_job_potter.rs");
    println!("cargo:rerun-if-changed=build_animal_pop.rs");
    println!("cargo:rerun-if-changed=build_gpi_death_polish.rs");
    println!("cargo:rerun-if-changed=build_nested_postload.rs");
    println!("cargo:rerun-if-changed=build_settings_live.rs");
    println!("cargo:rerun-if-changed=build_lockpick_settings.rs");
    println!("cargo:rerun-if-changed=build_map_temp_player.rs");
    println!("cargo:rerun-if-changed=build_contained_timers.rs");
    println!("cargo:rerun-if-changed=build_clothing_transitions.rs");
    println!("cargo:rerun-if-changed=build_score_entry.rs");
    println!("cargo:rerun-if-changed=build_war_posse.rs");
    println!("cargo:rerun-if-changed=build_craft_live_tick.rs");
    println!("cargo:rerun-if-changed=build_breastfeed_edges.rs");
    println!("cargo:rerun-if-changed=build_reputation_hit.rs");
    println!("cargo:rerun-if-changed=build_prestige_ally_cost.rs");
    println!("cargo:rerun-if-changed=src/_apply_prestige_ally_cost.py");
    println!("cargo:rerun-if-changed=build_class_boni.rs");
    println!("cargo:rerun-if-changed=build_eve_banana.rs");
    println!("cargo:rerun-if-changed=build_search_best_food.rs");
    println!("cargo:rerun-if-changed=build_do_commands.rs");
    println!("cargo:rerun-if-changed=build_twin_party_resid.rs");
    println!("cargo:rerun-if-changed=src/_apply_twin_party_resid.py");
    println!("cargo:rerun-if-changed=src/twin_heart.rs");
    println!("cargo:rerun-if-changed=src/do_commands_wire.rs");
    println!("cargo:rerun-if-changed=src/speech.rs");
    println!("cargo:rerun-if-changed=src/_apply_do_commands.py");
    println!("cargo:rerun-if-changed=src/search_best_food.rs");
    println!("cargo:rerun-if-changed=src/search_best_food_live.inc.rs");
    println!("cargo:rerun-if-changed=_apply_search_best_food.py");
    println!("cargo:rerun-if-changed=src/eve_spawn.rs");
    println!("cargo:rerun-if-changed=src/_apply_eve_banana.py");
    println!("cargo:rerun-if-changed=src/reputation.rs");
    println!("cargo:rerun-if-changed=src/prestige.rs");
    println!("cargo:rerun-if-changed=src/birth_fitness.rs");
    println!("cargo:rerun-if-changed=src/_apply_class_boni.py");
    println!("cargo:rerun-if-changed=src/feed.rs");
    println!("cargo:rerun-if-changed=src/_apply_breastfeed_edges.py");
    println!("cargo:rerun-if-changed=src/use_transition.rs");
    println!("cargo:rerun-if-changed=src/multi_use.rs");
    println!("cargo:rerun-if-changed=src/clothing_transitions.rs");
    println!("cargo:rerun-if-changed=src/clothing_cmds.rs");
    println!("cargo:rerun-if-changed=src/animal_damage.rs");
    println!("cargo:rerun-if-changed=src/animal_pop.rs");
    println!("cargo:rerun-if-changed=src/animals.rs");
    println!("cargo:rerun-if-changed=src/death_cause.rs");
    println!("cargo:rerun-if-changed=src/death_inherit.rs");
    println!("cargo:rerun-if-changed=src/death_polish.rs");
    println!("cargo:rerun-if-changed=src/long_term.rs");
    println!("cargo:rerun-if-changed=src/world_time.rs");
    println!("cargo:rerun-if-changed=src/contained_timers_persist.rs");
    println!("cargo:rerun-if-changed=src/heat_ideal.rs");
    println!("cargo:rerun-if-changed=src/map_temp_player.rs");
    println!("cargo:rerun-if-changed=src/move_speed.rs");
    println!("cargo:rerun-if-changed=src/move_notes.rs");
    println!("cargo:rerun-if-changed=src/move_live_gates.rs");
    println!("cargo:rerun-if-changed=src/mute.rs");
    println!("cargo:rerun-if-changed=src/ai_goals.rs");
    println!("cargo:rerun-if-changed=src/priority_ladder.rs");
    println!("cargo:rerun-if-changed=src/smith_profession.rs");
    println!("cargo:rerun-if-changed=src/baker_profession.rs");
    println!("cargo:rerun-if-changed=src/pottery_profession.rs");
    println!("cargo:rerun-if-changed=src/pottery_action_apply.inc.rs");
    println!("cargo:rerun-if-changed=src/farmer_profession.rs");
    println!("cargo:rerun-if-changed=src/farm_spatial_inc.rs");
    println!("cargo:rerun-if-changed=src/short_craft_intent.rs");
    println!("cargo:rerun-if-changed=src/profession_scan.rs");
    println!("cargo:rerun-if-changed=src/profession_scan_tests.inc.rs");
    println!("cargo:rerun-if-changed=src/postload_wire.rs");
    println!("cargo:rerun-if-changed=src/settings_live.rs");
    println!("cargo:rerun-if-changed=src/locks.rs");
    println!("cargo:rerun-if-changed=src/score_entry.rs");
    println!("cargo:rerun-if-changed=src/war_posse_persist.rs");
    println!("cargo:rerun-if-changed=src/player.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!(
        "cargo:rerun-if-changed={}",
        workspace
            .join("scripts/apply_th_clothing_matrix.py")
            .display()
    );

    // Early: AI-CRAFT-DUAL dual_center_search (before other craft wires so helpers exist)
    let dual_stamp = src.join(".ai_craft_dual_patched");
    if craft_dual_wired(&src) {
        let _ = std::fs::write(&dual_stamp, b"ai-craft-dual-1-source-wired\n");
        let _ = patch_all_ai_craft_dual(&src, &workspace);
    } else if patch_all_ai_craft_dual(&src, &workspace) {
        let _ = std::fs::write(&dual_stamp, b"ai-craft-dual-1-rs-patched\n");
    } else {
        println!(
            "cargo:warning=AI-CRAFT-DUAL: could not wire dual-center search / re-anchor"
        );
    }

    // Early: AI-HANDLING-FIRE isHandlingFire + late/hungry makeFireFood residual
    // Only run wire when not already present — re-running the python template clobbers residuals.
    let hf_stamp = src.join(".ai_handling_fire_patched");
    if handling_fire_wired(&src) {
        let _ = std::fs::write(&hf_stamp, b"ai-handling-fire-1-source-wired\n");
    } else if patch_all_handling_fire(&src, &workspace) {
        let _ = std::fs::write(&hf_stamp, b"ai-handling-fire-1-rs-patched\n");
    } else {
        println!(
            "cargo:warning=AI-HANDLING-FIRE: could not fully wire is_handling_fire into lib/player/scan"
        );
    }

    // Early: TH-CLOTHING-MATRIX python apply (lib + docs) if present.
    let apply_cloth = workspace.join("scripts/apply_th_clothing_matrix.py");
    if apply_cloth.exists() {
        let _ = Command::new("python")
            .arg(&apply_cloth)
            .status()
            .or_else(|_| Command::new("python3").arg(&apply_cloth).status());
    }

    // Early: BREASTFEED-EDGES python apply if present.
    let apply_bf = workspace.join("docs/port/_apply_breastfeed_edges_all.py");
    if apply_bf.exists() {
        let _ = Command::new("python")
            .arg(&apply_bf)
            .status()
            .or_else(|_| Command::new("python3").arg(&apply_bf).status());
    }
    let apply_bf2 = src.join("_apply_breastfeed_edges.py");
    if apply_bf2.exists() {
        let _ = Command::new("python")
            .arg(&apply_bf2)
            .status()
            .or_else(|_| Command::new("python3").arg(&apply_bf2).status());
    }

    // Early: REPUTATION-HIT python apply if present.
    let apply_rep = manifest.join("_patch_reputation_hit.py");
    if apply_rep.exists() {
        let _ = Command::new("python")
            .arg(&apply_rep)
            .status()
            .or_else(|_| Command::new("python3").arg(&apply_rep).status());
    }
    // Early: PRESTIGE-ALLY-COST python apply if present.
    let apply_pac = src.join("_apply_prestige_ally_cost.py");
    if apply_pac.exists() {
        let _ = Command::new("python")
            .arg(&apply_pac)
            .status()
            .or_else(|_| Command::new("python3").arg(&apply_pac).status());
    }

    // Early: CLASS-BONI python apply if present.
    let apply_cb = src.join("_apply_class_boni.py");
    if apply_cb.exists() {
        let _ = Command::new("python")
            .arg(&apply_cb)
            .status()
            .or_else(|_| Command::new("python3").arg(&apply_cb).status());
    }

    // Early: EVE-BANANA jungle_spawn python apply if present.
    let apply_eve = src.join("_apply_eve_banana.py");
    if apply_eve.exists() {
        let _ = Command::new("python")
            .arg(&apply_eve)
            .status()
            .or_else(|_| Command::new("python3").arg(&apply_eve).status());
    }


    // Early: DO-COMMANDS python apply if present.
    let apply_docmd = src.join("_apply_do_commands.py");
    if apply_docmd.exists() {
        let _ = Command::new("python")
            .arg(&apply_docmd)
            .status()
            .or_else(|_| Command::new("python3").arg(&apply_docmd).status());
    }

    // Early: TH-ALT-OUTCOME python apply if present.
    let apply_th_alt = src.join("_apply_th_alt_outcome.py");
    if apply_th_alt.exists() {
        let _ = Command::new("python")
            .arg(&apply_th_alt)
            .status()
            .or_else(|_| Command::new("python3").arg(&apply_th_alt).status());
    }
    println!("cargo:rerun-if-changed=build_th_alt_outcome.rs");
    println!("cargo:rerun-if-changed=src/alt_outcome.rs");
    println!("cargo:rerun-if-changed=src/_apply_th_alt_outcome.py");

    // Early: SEARCH-BEST-FOOD python apply if present.
    let apply_sbf = manifest.join("_apply_search_best_food.py");
    if apply_sbf.exists() {
        let _ = Command::new("python")
            .arg(&apply_sbf)
            .status()
            .or_else(|_| Command::new("python3").arg(&apply_sbf).status());
    }

    // Early: AI-POTTER pure SM wire (pottery_profession + sticky + ai_goals).
    let lib_path = src.join("lib.rs");
    let potter_stamp = src.join(".ai_job_potter_patched");
    let potter_ok = patch_all_ai_job_potter(&src, &lib_path);
    if potter_job_wired(&std::fs::read_to_string(&lib_path).unwrap_or_default()) {
        let _ = std::fs::write(&potter_stamp, b"ai-potter-1-source-wired\n");
    } else if potter_ok {
        let _ = std::fs::write(&potter_stamp, b"ai-potter-1-rs-patched\n");
    } else {
        println!("cargo:warning=AI-POTTER: could not fully wire pottery_profession");
    }

    let stamp = src.join(".th_multi_patched");
    if !stamp.exists() {
        let _ = std::fs::write(&stamp, b"th-multi-3-source-wired\n");
    }

    let animal_stamp = src.join(".time_animal_damage_patched");
    if let Ok(text) = std::fs::read_to_string(src.join("lib.rs")) {
        if text.contains("mod animal_damage;") && text.contains("fn apply_animal_path_damages") {
            let _ = std::fs::write(&animal_stamp, b"time-animal-damage-2-source-wired\n");
        } else if animal_stamp.exists() {
            let _ = std::fs::remove_file(&animal_stamp);
            println!(
                "cargo:warning=TIME-ANIMAL: lib.rs missing animal_damage wire (mod/apply_animal_path_damages)"
            );
        }
    }

    // GPI-DEATH death_cause_coins wire (idempotent).
    let death_stamp = src.join(".gpi_death_cause_coins_patched");
    let lib_ok = std::fs::read_to_string(&lib_path)
        .map(|t| {
            t.contains("mod death_inherit;")
                && (t.contains("Haxe `GlobalPlayerInstance.InheritCoins`")
                    || t.contains("death_polish::apply_death_polish"))
                && t.contains("hunger_death_wire")
        })
        .unwrap_or(false);

    if lib_ok {
        let _ = std::fs::write(&death_stamp, b"gpi-death-cause-coins-1-source-wired\n");
    } else {
        let script = workspace.join("scripts/patch_gpi_death_cause_coins.py");
        if script.exists() {
            let py = Command::new("python")
                .arg(&script)
                .status()
                .or_else(|_| Command::new("python3").arg(&script).status());
            match py {
                Ok(s) if s.success() => {
                    let _ = std::fs::write(&death_stamp, b"gpi-death-cause-coins-1-py-patched\n");
                }
                _ => {
                    if patch_lib_gpi_death(&lib_path) {
                        let _ =
                            std::fs::write(&death_stamp, b"gpi-death-cause-coins-1-rs-patched\n");
                    } else {
                        println!(
                            "cargo:warning=GPI-DEATH: could not patch lib.rs for death_cause_coins"
                        );
                    }
                }
            }
        } else if patch_lib_gpi_death(&lib_path) {
            let _ = std::fs::write(&death_stamp, b"gpi-death-cause-coins-1-rs-patched\n");
        }
    }

    // GPI-DEATH-POLISH grave_soul_leader (idempotent).
    let death_polish_stamp = src.join(".gpi_death_polish_patched");
    if death_polish_wired(&std::fs::read_to_string(&lib_path).unwrap_or_default()) {
        let _ = std::fs::write(&death_polish_stamp, b"gpi-death-polish-1-source-wired\n");
    } else if patch_lib_gpi_death_polish(&lib_path, &workspace) {
        let _ = std::fs::write(&death_polish_stamp, b"gpi-death-polish-1-patched\n");
    } else {
        println!("cargo:warning=GPI-DEATH-POLISH: could not wire grave_soul_leader into lib.rs");
    }

    // TIME-LONG wire (idempotent).
    let long_stamp = src.join(".time_long_patched");
    let lt_path = src.join("long_term.rs");
    let _ = patch_long_term_self_contained(&lt_path);
    if patch_lib_time_long(&lib_path) {
        let _ = std::fs::write(&long_stamp, b"time-long-1-source-wired\n");
    } else if let Ok(t) = std::fs::read_to_string(&lib_path) {
        if t.contains("mod long_term;") && t.contains("do_world_long_term_time_stuff") {
            let _ = std::fs::write(&long_stamp, b"time-long-1-source-wired\n");
        } else {
            println!("cargo:warning=TIME-LONG: could not fully wire long_term into lib.rs");
        }
    }

    // TIME-WORLD-POLISH: ground_id ComplexObject tests + transform_target on auto-decay.
    let polish_stamp = src.join(".time_world_polish_patched");
    if patch_lib_time_world_polish(&lib_path) {
        let _ = std::fs::write(&polish_stamp, b"time-world-polish-1-source-wired\n");
    } else if let Ok(t) = std::fs::read_to_string(&lib_path) {
        if text_has_world_polish(&t) {
            let _ = std::fs::write(&polish_stamp, b"time-world-polish-1-source-wired\n");
        }
    }

    // S-MOVE road_floor_speed: floor/road/biome factors + path road scan.
    let move_stamp = src.join(".s_move_road_floor_patched");
    let move_ok = std::fs::read_to_string(&lib_path)
        .map(|t| {
            t.contains("floor_road_factor_at(&world, &state.content, p.x, p.y, true, false)")
                && t.contains("scan_path_road_and_biome(")
                && t.contains("compose_move_speed_with_floor")
        })
        .unwrap_or(false);
    if move_ok {
        let _ = std::fs::write(&move_stamp, b"s-move-road-floor-1-source-wired\n");
    } else {
        let script = workspace.join("scripts/patch_s_move_road_floor.py");
        let mut done = false;
        if script.exists() {
            let py = Command::new("python")
                .arg(&script)
                .status()
                .or_else(|_| Command::new("python3").arg(&script).status());
            if let Ok(s) = py {
                if s.success() {
                    let _ = std::fs::write(&move_stamp, b"s-move-road-floor-1-py-patched\n");
                    done = true;
                }
            }
        }
        if !done {
            if patch_lib_s_move_road_floor(&lib_path) {
                let _ = std::fs::write(&move_stamp, b"s-move-road-floor-1-rs-patched\n");
            } else if let Ok(t) = std::fs::read_to_string(&lib_path) {
                if t.contains("floor_road_factor_at") && t.contains("scan_path_road_and_biome") {
                    let _ = std::fs::write(&move_stamp, b"s-move-road-floor-1-source-wired\n");
                } else {
                    println!("cargo:warning=S-MOVE: could not fully wire move_speed into lib.rs");
                }
            }
        }
    }

    // S-MOVE-LIVE-GATES grave_enemy_live: live grave curse + close enemy weapon.
    let live_stamp = src.join(".s_move_live_gates_patched");
    let player_path = src.join("player.rs");
    let live_ok = live_gates_wired(
        &std::fs::read_to_string(&lib_path).unwrap_or_default(),
        &std::fs::read_to_string(&player_path).unwrap_or_default(),
    );
    if live_ok {
        let _ = std::fs::write(&live_stamp, b"s-move-live-gates-1-source-wired\n");
    } else if patch_s_move_live_gates(&src, &workspace) {
        let _ = std::fs::write(&live_stamp, b"s-move-live-gates-1-patched\n");
    } else {
        println!(
            "cargo:warning=S-MOVE-LIVE-GATES: could not wire grave/enemy live gates into lib.rs/player.rs"
        );
    }

    // MUTE-SAY mute_delivery: WHISPER mute filter + say_mute_blocks_whisper test.
    let mute_stamp = src.join(".mute_say_delivery_patched");
    if mute_say_wired(&lib_path) {
        let _ = std::fs::write(&mute_stamp, b"mute-say-1-source-wired\n");
    } else {
        let script = workspace.join("docs/port/_patch_mute_say.py");
        let mut done = false;
        if script.exists() {
            let py = Command::new("python")
                .arg(&script)
                .status()
                .or_else(|_| Command::new("python3").arg(&script).status());
            if let Ok(s) = py {
                if s.success() && mute_say_wired(&lib_path) {
                    let _ = std::fs::write(&mute_stamp, b"mute-say-1-py-patched\n");
                    done = true;
                }
            }
        }
        if !done {
            if patch_lib_mute_say(&lib_path) {
                let _ = std::fs::write(&mute_stamp, b"mute-say-1-rs-patched\n");
            } else if mute_say_wired(&lib_path) {
                let _ = std::fs::write(&mute_stamp, b"mute-say-1-source-wired\n");
            } else {
                println!("cargo:warning=MUTE-SAY: could not wire WHISPER mute filter into lib.rs");
            }
        }
    }

    // AI-PRIO priority_ladder: expand ai_goals pub use (idempotent).
    let ai_prio_stamp = src.join(".ai_prio_ladder_patched");
    if ai_prio_wired(&lib_path) {
        let _ = std::fs::write(&ai_prio_stamp, b"ai-prio-1-source-wired\n");
    } else {
        let script = workspace.join("docs/port/_patch_ai_prio.py");
        let mut done = false;
        if script.exists() {
            let py = Command::new("python")
                .arg(&script)
                .status()
                .or_else(|_| Command::new("python3").arg(&script).status());
            if let Ok(s) = py {
                if s.success() && ai_prio_wired(&lib_path) {
                    let _ = std::fs::write(&ai_prio_stamp, b"ai-prio-1-py-patched\n");
                    done = true;
                }
            }
        }
        if !done {
            if patch_lib_ai_prio(&lib_path) {
                let _ = std::fs::write(&ai_prio_stamp, b"ai-prio-1-rs-patched\n");
            } else if ai_prio_wired(&lib_path) {
                let _ = std::fs::write(&ai_prio_stamp, b"ai-prio-1-source-wired\n");
            } else {
                println!(
                    "cargo:warning=AI-PRIO: could not expand ai_goals pub use for priority_ladder"
                );
            }
        }
    }

    // AI-JOB-SMITH: smith_profession mod + exports + selfplay pipeline (idempotent).
    let smith_stamp = src.join(".ai_job_smith_patched");
    if smith_job_wired(&lib_path) {
        let _ = std::fs::write(&smith_stamp, b"ai-job-smith-1-source-wired\n");
    } else {
        let script = workspace.join("docs/port/_patch_ai_job_smith.py");
        let mut done = false;
        if script.exists() {
            let py = Command::new("python")
                .arg(&script)
                .status()
                .or_else(|_| Command::new("python3").arg(&script).status());
            if let Ok(s) = py {
                if s.success() && smith_job_wired(&lib_path) {
                    let _ = std::fs::write(&smith_stamp, b"ai-job-smith-1-py-patched\n");
                    done = true;
                }
            }
        }
        if !done {
            if patch_lib_ai_job_smith(&lib_path) {
                let _ = std::fs::write(&smith_stamp, b"ai-job-smith-1-rs-patched\n");
            } else if smith_job_wired(&lib_path) {
                let _ = std::fs::write(&smith_stamp, b"ai-job-smith-1-source-wired\n");
            } else {
                println!(
                    "cargo:warning=AI-JOB-SMITH: could not wire smith_profession into lib.rs"
                );
            }
        }
    }
    // Selfplay smith pipeline (best-effort; ol-server path relative to workspace).
    let selfplay = workspace.join("crates/ol-server/src/selfplay.rs");
    if selfplay.exists() {
        let _ = patch_selfplay_smith(&selfplay);
    }

    // AI-JOB-BAKER: baker_profession mod + exports + ladder + selfplay (idempotent).
    let baker_stamp = src.join(".ai_job_baker_patched");
    let prio_path = src.join("priority_ladder.rs");
    let script = workspace.join("docs/port/_patch_ai_job_baker.py");
    if script.exists() {
        let _ = Command::new("python")
            .arg(&script)
            .status()
            .or_else(|_| Command::new("python3").arg(&script).status());
    }
    let lib_patched = patch_lib_ai_job_baker(&lib_path);
    let prio_patched = patch_priority_ladder_baker(&prio_path);
    if selfplay.exists() {
        let _ = patch_selfplay_baker(&selfplay);
    }
    if baker_job_wired(&lib_path) && prio_patched {
        let _ = std::fs::write(&baker_stamp, b"ai-job-baker-1-source-wired\n");
    } else if lib_patched || prio_patched {
        let _ = std::fs::write(&baker_stamp, b"ai-job-baker-1-rs-patched\n");
    } else {
        println!("cargo:warning=AI-JOB-BAKER: could not fully wire baker_profession");
    }

    // AI-JOB-FARM-WIRE farm_spatial
    let farm_wire_stamp = src.join(".ai_job_farm_wire_patched");
    let farm_ok = patch_all_farm_wire(&src, &lib_path);
    let _ = patch_farmer_doc(&src.join("farmer_profession.rs"));
    let _ = patch_lib_ai_job_farm_wire(&lib_path);
    let _ = patch_priority_ladder_farm(&prio_path);
    if farm_wire_wired(&lib_path) {
        let _ = std::fs::write(&farm_wire_stamp, b"ai-job-farm-wire-1-source-wired\n");
    } else if farm_ok {
        let _ = std::fs::write(&farm_wire_stamp, b"ai-job-farm-wire-1-rs-patched\n");
    } else {
        println!("cargo:warning=AI-JOB-FARM-WIRE: could not fully wire farm_spatial");
    }

    // CRAFT-LIVE-TICK profession_scan_tick: world scan → shortCraft USE/DROP
    let craft_tick_stamp = src.join(".craft_live_tick_patched");
    let craft_ok = patch_all_craft_live_tick(&src, &lib_path);
    if craft_live_tick_wired(&lib_path)
        && std::fs::read_to_string(src.join("profession_scan.rs"))
            .map(|t| t.contains("include!(\"profession_scan_tests.inc.rs\")"))
            .unwrap_or(false)
    {
        let _ = std::fs::write(&craft_tick_stamp, b"craft-live-tick-1-source-wired\n");
    } else if craft_ok {
        let _ = std::fs::write(&craft_tick_stamp, b"craft-live-tick-1-rs-patched\n");
    } else {
        println!(
            "cargo:warning=CRAFT-LIVE-TICK: could not wire profession_scan into lib.rs / tests include"
        );
    }

    // NESTED-OLW1-POLISH postload_wire
    let nested_stamp = src.join(".nested_postload_wire_patched");
    if patch_nested_postload(&src, &workspace) {
        let _ = std::fs::write(&nested_stamp, b"nested-postload-wire-1-source-wired\n");
    } else {
        println!("cargo:warning=NESTED-OLW1-POLISH: could not wire postload into lib.rs/player.rs");
    }

    // CONFIG-SETTINGS server_settings_hot_reload
    let settings_stamp = src.join(".config_settings_hot_reload_patched");
    if patch_settings_live(&src, &workspace) {
        let _ = std::fs::write(
            &settings_stamp,
            b"config-settings-hot-reload-1-source-wired\n",
        );
    } else {
        println!(
            "cargo:warning=CONFIG-SETTINGS: could not wire settings hot-reload into lib.rs/main/npc"
        );
    }

    // LOCKPICK-SETTINGS lockpick_live_knobs (SimState + apply_live + USE wire)
    let lockpick_stamp = src.join(".lockpick_settings_patched");
    if patch_lockpick_settings(&src) {
        let _ = std::fs::write(&lockpick_stamp, b"lockpick-settings-1-source-wired\n");
    } else {
        println!(
            "cargo:warning=LOCKPICK-SETTINGS: could not wire lockpick live knobs into lib/settings_live/use_transition"
        );
    }

    // MAP-TEMP-PLAYER vitals_tile_temps
    let map_temp_stamp = src.join(".map_temp_player_patched");
    if map_temp_player_wired(&std::fs::read_to_string(&lib_path).unwrap_or_default()) {
        let _ = std::fs::write(&map_temp_stamp, b"map-temp-player-1-source-wired\n");
    } else if patch_lib_map_temp_player(&lib_path) {
        let _ = std::fs::write(&map_temp_stamp, b"map-temp-player-1-rs-patched\n");
    } else {
        println!(
            "cargo:warning=MAP-TEMP-PLAYER: could not wire tile_temps vitals path into lib.rs"
        );
    }

    // CONTAINED-TIMERS-PERSIST rearm_after_load
    let ct_stamp = src.join(".contained_timers_persist_patched");
    if patch_contained_timers(&src) {
        let _ = std::fs::write(&ct_stamp, b"contained-timers-persist-1-source-wired\n");
    } else {
        println!(
            "cargo:warning=CONTAINED-TIMERS-PERSIST: could not wire rearm_after_load into lib.rs/world_time.rs"
        );
    }

    // TIME-ANIMAL-OFFSPRING pop_die_offspring
    let pop_stamp = src.join(".time_animal_offspring_patched");
    if animal_pop_wired(&std::fs::read_to_string(&lib_path).unwrap_or_default()) {
        let _ = std::fs::write(&pop_stamp, b"time-animal-offspring-1-source-wired\n");
    } else if patch_animal_pop(&src) {
        let _ = std::fs::write(&pop_stamp, b"time-animal-offspring-1-rs-patched\n");
    } else {
        println!(
            "cargo:warning=TIME-ANIMAL-OFFSPRING: could not wire pop/die/failedMoves into lib.rs"
        );
    }

    // TH-CLOTHING-MATRIX clothing_transitions: always run RS patch (idempotent).
    let cloth_stamp = src.join(".th_clothing_matrix_patched");
    let _ = patch_clothing_transitions(&src);
    if clothing_transitions_wired(&std::fs::read_to_string(&lib_path).unwrap_or_default()) {
        let _ = std::fs::write(&cloth_stamp, b"th-clothing-matrix-1-source-wired\n");
    } else {
        println!(
            "cargo:warning=TH-CLOTHING-MATRIX: could not fully wire clothing_transitions into lib.rs"
        );
    }

    // SCORE-ENTRY score_disk: AccountScoreEntry queue + SES1 disk + death/tick wire.
    let score_stamp = src.join(".score_entry_disk_patched");
    let dp_text = std::fs::read_to_string(src.join("death_polish.rs")).unwrap_or_default();
    let lib_text = std::fs::read_to_string(&lib_path).unwrap_or_default();
    if score_entry_wired(&lib_text, &dp_text) {
        let _ = std::fs::write(&score_stamp, b"score-entry-disk-1-source-wired\n");
    } else if patch_score_entry(&src) {
        let _ = std::fs::write(&score_stamp, b"score-entry-disk-1-rs-patched\n");
    } else {
        println!(
            "cargo:warning=SCORE-ENTRY: could not wire score_entry into lib.rs/death_polish.rs"
        );
    }

    // AI-SOUL-WIRE soul_on_player: expand player_soul pub use (idempotent).
    let soul_stamp = src.join(".ai_soul_wire_patched");
    println!("cargo:rerun-if-changed=build_soul_wire.rs");
    println!("cargo:rerun-if-changed=src/player_soul.rs");
    println!("cargo:rerun-if-changed=src/player_soul_body.rs");
    println!("cargo:rerun-if-changed=src/player_soul_wire.rs");
    println!("cargo:rerun-if-changed=src/soul_live.rs");
    let lib_text_soul = std::fs::read_to_string(&lib_path).unwrap_or_default();
    if build_soul_wire::soul_wire_exports_ok(&lib_text_soul) {
        let _ = std::fs::write(&soul_stamp, b"ai-soul-wire-1-source-wired\n");
    } else if build_soul_wire::patch_lib_soul_wire(&lib_path) {
        let _ = std::fs::write(&soul_stamp, b"ai-soul-wire-1-rs-patched\n");
    } else {
        println!(
            "cargo:warning=AI-SOUL-WIRE: could not expand player_soul exports in lib.rs"
        );
    }

    // SOCIAL-WAR-PERSIST war_posse_disk: WPS1 save/load + sim share + ol-server wire.
    let wps_stamp = src.join(".social_war_persist_patched");
    let lib_w = std::fs::read_to_string(&lib_path).unwrap_or_default();
    let sl_w = std::fs::read_to_string(src.join("settings_live.rs")).unwrap_or_default();
    let cfg_w = std::fs::read_to_string(workspace.join("crates/ol-config/src/lib.rs"))
        .unwrap_or_default();
    let main_w =
        std::fs::read_to_string(workspace.join("crates/ol-server/src/main.rs")).unwrap_or_default();
    if war_posse_wired(&lib_w, &sl_w, &cfg_w, &main_w) {
        let _ = std::fs::write(&wps_stamp, b"social-war-persist-1-source-wired\n");
    } else if patch_war_posse(&src, &workspace) {
        let _ = std::fs::write(&wps_stamp, b"social-war-persist-1-rs-patched\n");
    } else {
        println!(
            "cargo:warning=SOCIAL-WAR-PERSIST: could not wire war_posse WPS1 into lib/settings/config/main"
        );
    }

    // BREASTFEED-EDGES nurse_edges
    let bf_stamp = src.join(".breastfeed_edges_patched");
    let lib_bf = std::fs::read_to_string(&lib_path).unwrap_or_default();
    if breastfeed_edges_wired(&lib_bf) {
        let _ = std::fs::write(&bf_stamp, b"breastfeed-edges-1-source-wired\n");
    } else if patch_breastfeed_edges(&src, &workspace) {
        let _ = std::fs::write(&bf_stamp, b"breastfeed-edges-1-rs-patched\n");
    } else {
        println!(
            "cargo:warning=BREASTFEED-EDGES: could not wire nurse_edges into lib.rs"
        );
    }

    // REPUTATION-HIT hit_reputation: lostCombatPrestige on every connecting HIT.
    let rep_stamp = src.join(".reputation_hit_patched");
    let lib_rep = std::fs::read_to_string(&lib_path).unwrap_or_default();
    if reputation_hit_wired(&lib_rep) {
        let _ = std::fs::write(&rep_stamp, b"reputation-hit-1-source-wired\n");
    } else if patch_reputation_hit(&lib_path) {
        let _ = std::fs::write(&rep_stamp, b"reputation-hit-1-rs-patched\n");
    } else {
        println!(
            "cargo:warning=REPUTATION-HIT: could not wire hit reputation into lib.rs"
        );
    }

    // DARK-NOSAJ dark_nosaj_use: Tarr 3112 + Dark Nosaj 2466 monument USE set/clear.
    let dn_stamp = src.join(".dark_nosaj_patched");
    println!("cargo:rerun-if-changed=build_dark_nosaj.rs");
    println!("cargo:rerun-if-changed=src/dark_nosaj.rs");
    println!("cargo:rerun-if-changed=src/_apply_dark_nosaj.py");
    let player_dn = std::fs::read_to_string(src.join("player.rs")).unwrap_or_default();
    let use_dn = std::fs::read_to_string(src.join("use_transition.rs")).unwrap_or_default();
    let lib_dn = std::fs::read_to_string(&lib_path).unwrap_or_default();
    if dark_nosaj_wired(&player_dn, &use_dn, &lib_dn) {
        // Already source-wired — heal stacked injects only (never re-prepend horse-anchor).
        let _ = std::fs::write(&dn_stamp, b"dark-nosaj-2-source-wired-heal\n");
        let _ = heal_dark_nosaj_stacking(&src, &workspace);
    } else if patch_dark_nosaj(&src, &workspace) {
        let _ = std::fs::write(&dn_stamp, b"dark-nosaj-2-rs-patched\n");
    } else {
        println!(
            "cargo:warning=DARK-NOSAJ: could not wire monument USE into player/use_transition/lib"
        );
    }


    // TH-ALT-OUTCOME alt_transition_outcome: alternativeTransitionOutcome + fortification.
    let th_alt_stamp = src.join(".th_alt_outcome_patched");
    let content_lib = std::fs::read_to_string(
        manifest.parent().unwrap().join("ol-content/src/lib.rs"),
    )
    .unwrap_or_default();
    let use_alt = std::fs::read_to_string(src.join("use_transition.rs")).unwrap_or_default();
    let lib_alt = std::fs::read_to_string(&lib_path).unwrap_or_default();
    if th_alt_outcome_wired(&content_lib, &use_alt, &lib_alt) {
        let _ = std::fs::write(&th_alt_stamp, b"th-alt-outcome-1-source-wired\n");
        let _ = patch_th_alt_outcome(&src, &workspace);
    } else if patch_th_alt_outcome(&src, &workspace) {
        let _ = std::fs::write(&th_alt_stamp, b"th-alt-outcome-1-rs-patched\n");
    } else {
        println!(
            "cargo:warning=TH-ALT-OUTCOME: could not wire alt outcome content/use/lib"
        );
    }
    // PRESTIGE-ALLY-COST ally_prestige_cost: PrestigeCostPerDamageForAlly + GM + exile-aware isAlly.
    let pac_stamp = src.join(".prestige_ally_cost_patched");
    if patch_prestige_ally_cost(&src, &workspace) {
        let _ = std::fs::write(&pac_stamp, b"prestige-ally-cost-1-source-wired\n");
    } else {
        println!(
            "cargo:warning=PRESTIGE-ALLY-COST: could not fully wire ally prestige cost"
        );
    }

    // CLASS-BONI prestige_class_table: calculateClassBoni + PrestigeClasses + birth fitness wire.
    let class_boni_stamp = src.join(".class_boni_patched");
    let lib_cb = std::fs::read_to_string(&lib_path).unwrap_or_default();
    if class_boni_wired(&lib_cb) {
        let _ = std::fs::write(&class_boni_stamp, b"class-boni-1-source-wired\n");
        // Still refresh docs if pure RS path had not yet.
        let _ = patch_class_boni(&manifest, &src);
    } else if patch_class_boni(&manifest, &src) {
        let _ = std::fs::write(&class_boni_stamp, b"class-boni-1-rs-patched\n");
    } else {
        println!(
            "cargo:warning=CLASS-BONI: could not wire prestige class table / birth fitness"
        );
    }

    // EVE-BANANA jungle_spawn: food-plant Eve + jungle banana preference.
    let eve_stamp = src.join(".eve_banana_patched");
    let lib_eve = std::fs::read_to_string(&lib_path).unwrap_or_default();
    if eve_banana_wired(&lib_eve) {
        let _ = std::fs::write(&eve_stamp, b"eve-banana-1-source-wired\n");
        let _ = patch_eve_banana(&manifest, &src);
    } else if patch_eve_banana(&manifest, &src) {
        let _ = std::fs::write(&eve_stamp, b"eve-banana-1-rs-patched\n");
    } else {
        println!(
            "cargo:warning=EVE-BANANA: could not wire eve_spawn jungle banana into lib.rs"
        );
    }

    // SEARCH-BEST-FOOD ai_food_search: full AiHelper.SearchBestFood + processFood.
    let sbf_stamp = src.join(".search_best_food_patched");
    let lib_sbf = std::fs::read_to_string(&lib_path).unwrap_or_default();
    if search_best_food_wired(&lib_sbf) {
        let _ = std::fs::write(&sbf_stamp, b"search-best-food-1-source-wired\n");
        let _ = patch_search_best_food(&manifest, &src, &workspace);
    } else if patch_search_best_food(&manifest, &src, &workspace) {
        let _ = std::fs::write(&sbf_stamp, b"search-best-food-1-rs-patched\n");
    } else {
        println!(
            "cargo:warning=SEARCH-BEST-FOOD: could not wire search_best_food into lib.rs"
        );
    }
    // DO-COMMANDS say_commands: Haxe doCommands natural-language SAY.
    let docmd_stamp = src.join(".do_commands_patched");
    let lib_dc = std::fs::read_to_string(&lib_path).unwrap_or_default();
    if do_commands_wired(&lib_dc) {
        let _ = std::fs::write(&docmd_stamp, b"do-commands-1-source-wired\n");
        let _ = patch_do_commands(&manifest, &src, &workspace);
    } else if patch_do_commands(&manifest, &src, &workspace) {
        let _ = std::fs::write(&docmd_stamp, b"do-commands-1-rs-patched\n");
    } else {
        println!(
            "cargo:warning=DO-COMMANDS: could not wire doCommands into lib.rs"
        );
    }

    // TWIN-PARTY-RESID twin_wait_edges: heart-link + ObjectData.male + wait timeout.
    let twin_stamp = src.join(".twin_party_resid_patched");
    println!("cargo:rerun-if-changed=build_twin_party_resid.rs");
    println!("cargo:rerun-if-changed=src/_apply_twin_party_resid.py");
    println!("cargo:rerun-if-changed=src/twin_heart.rs");
    let lib_twin = std::fs::read_to_string(&lib_path).unwrap_or_default();
    if twin_party_resid_wired(&lib_twin) {
        let _ = std::fs::write(&twin_stamp, b"twin-party-resid-1-source-wired\n");
        let _ = patch_twin_party_resid(&src, &workspace);
    } else if patch_twin_party_resid(&src, &workspace) {
        let _ = std::fs::write(&twin_stamp, b"twin-party-resid-1-rs-patched\n");
    } else {
        println!(
            "cargo:warning=TWIN-PARTY-RESID: could not wire twin_wait_edges into lib.rs"
        );
    }

    // Late: re-apply AI-CRAFT-DUAL after other craft patches (idempotent).
    let _ = patch_all_ai_craft_dual(&src, &workspace);
}
