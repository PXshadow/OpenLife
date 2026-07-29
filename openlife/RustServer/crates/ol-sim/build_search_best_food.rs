//! SEARCH-BEST-FOOD / ai_food_search build-time wire (included from build.rs).
//!
//! Also piggybacks **DO-COMMANDS**, **HEALTH-AGE-FOOD**, **WORLD-FOOD-FACTOR**,
//! and **AI-FOOD-FAIL-MARK**.
//!
//! Idempotent. Handles CRLF sources.

use std::path::Path;

#[path = "build_health_age_food.rs"]
mod health_age_food_wire;

#[path = "build_world_food_factor.rs"]
mod world_food_factor_wire;

#[path = "build_ai_food_fail_mark.rs"]
mod ai_food_fail_mark_wire;

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

fn replace_once(hay: &mut String, old: &str, new: &str) -> bool {
    if let Some(i) = hay.find(old) {
        hay.replace_range(i..i + old.len(), new);
        true
    } else {
        false
    }
}

pub fn search_best_food_wired(lib: &str) -> bool {
    lib.contains("mod search_best_food;")
        && lib.contains("search_best_food_full")
        && lib.contains("pub use search_best_food::")
        && (lib.contains("SEARCH-BEST-FOOD") || lib.contains("search_best_food_live.inc.rs"))
}

const FN_OLD: &str = r#"/// Haxe `AiHelper.SearchBestFood` lite — scan radius for best food by score.
///
/// Returns `(food_id, food_value, tx, ty, quad_dist)` or None.
pub fn search_best_food_nearby(
    state: &SimState,
    conn_id: u64,
    radius: i32,
) -> Option<(i32, i32, i32, i32, f32)> {
    let p = state.players.get(&conn_id)?;
    if p.deleted {
        return None;
    }
    let (px, py) = (p.x, p.y);
    let food_store = p.food;
    let food_max = p.food_max;
    let craving = p.yum.currently_craving;
    let mut cands: Vec<FoodCandidate> = Vec::new();
    {
        let world = state.world.read().ok()?;
        for ty in (py - radius)..=(py + radius) {
            for tx in (px - radius)..=(px + radius) {
                let id = world.get_object(tx, ty);
                if id == 0 {
                    continue;
                }
                let base = state.content.resolve_base_id(id);
                let Some(def) = state.content.get(base) else {
                    continue;
                };
                if def.food_value <= 0 {
                    continue;
                }
                let count = p.yum.get_count_eaten(base);
                cands.push(FoodCandidate {
                    food_id: base,
                    food_value: def.food_value,
                    tx,
                    ty,
                    count_eaten: count,
                });
            }
        }
    }
    let i = pick_best_food(&cands, px, py, food_store, food_max, craving)?;
    let c = cands[i];
    let dx = (c.tx - px) as f32;
    let dy = (c.ty - py) as f32;
    let quad = dx * dx + dy * dy;
    Some((c.food_id, c.food_value, c.tx, c.ty, quad))
}"#;

const FN_NEW: &str = r#"// Haxe: AiHelper.SearchBestFood full live scan (SEARCH-BEST-FOOD / ai_food_search)
include!("search_best_food_live.inc.rs");"#;

const DO_BLOCK: &str = r#"        // DO-COMMANDS / say_commands — Haxe GlobalPlayerInstance.doCommands natural language.
        // I EXILE/BANN, I REDEEM, I FOLLOW, I HIRE, ORDER,, I GIVE, OWN THIS, HOME!
        if parse_do_command(&upper).is_some() {
            let candidates: Vec<NameCandidate> = state
                .players
                .values()
                .map(|pl| {
                    let pc = state.social.prestige_class(pl.p_id).as_i32();
                    NameCandidate::from_player(pl, pc)
                })
                .collect();
            let lost = state
                .combat
                .stats
                .get(&p.p_id)
                .map(|s| s.lost_combat_prestige)
                .unwrap_or(0.0);
            let speaker = p.clone();
            let fx = apply_do_commands_live(
                &upper,
                &speaker,
                conn_id,
                &mut state.social,
                &mut state.economy,
                &mut state.players,
                &state.world,
                lost,
                &candidates,
            );
            for (c, line) in &fx.private_ps {
                send_ps_reply(outbound, *c, line);
            }
            let near = nearby_conn_ids(state, speaker.x, speaker.y, NEARBY_RANGE);
            for line in &fx.following_lines {
                send_nearby(
                    outbound,
                    &near,
                    format_server_message("FW", &[line.as_str()]).into_bytes(),
                );
            }
            for line in &fx.exile_lines {
                send_nearby(
                    outbound,
                    &near,
                    format_server_message("EX", &[line.as_str()]).into_bytes(),
                );
            }
            for (c, msg) in &fx.order_global {
                send_ps_reply(outbound, *c, msg);
            }
            if upper.starts_with("I GIVE ") || upper.starts_with("I HIRE") {
                for id in state.economy.wallets.keys().copied().collect::<Vec<_>>() {
                    let coins = state.economy.coins_of(id);
                    state.scoreboard.set_coins(id, coins);
                }
            }
            if fx.recognized && !fx.broadcast_chat {
                info!(conn_id, text = %text, "sim: DO-COMMANDS handled (no chat broadcast)");
                return;
            }
            if fx.recognized && fx.broadcast_chat {
                info!(conn_id, text = %text, "sim: DO-COMMANDS + chat broadcast");
            }
        }
"#;

fn patch_do_commands_inline(src: &Path, workspace: &Path) -> bool {
    let path = src.join("lib.rs");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("mod do_commands_wire;") {
        if replace_once(
            &mut t,
            "mod speech;\n",
            "mod speech;\n// Haxe: GlobalPlayerInstance.doCommands (DO-COMMANDS / say_commands)\nmod do_commands_wire;\n",
        ) {
            changed = true;
        }
    }

    if !t.contains("pub use do_commands_wire::") {
        if let Some(i) = t.find("pub use speech::{") {
            if let Some(j) = t[i..].find("};\n") {
                let end = i + j + 3;
                let replacement = r#"pub use speech::{
    chat_range_for_age as speech_chat_range_for_age, closest_owned_tile, compute_hire_cost,
    do_command_broadcasts_chat, extract_command_name, find_player_by_name, format_exile_say_result,
    format_follow_say_result, format_give_say_result, format_hire_say_result,
    format_home_bang_result, format_order_global, format_own_this_result, format_redeem_say_result,
    hire_age_ok, hire_angry_ok, hire_class_ok, is_follow_self_name, is_home_oven_id,
    parse_do_command, parse_own_this_name, parse_roman_coin_amount, pick_nearest_home_oven,
    ADULT_CHAT_RANGE, DoCommand, HIRE_COST, HIRE_COST_INCREASE_PER_PERSON, HOME_OVEN_IDS,
    HOME_SEARCH_MAX_QUAD, MUMBLE_CHAT_RANGE, SHOUT_CHAT_RANGE, SpeechVolume, WHISPER_CHAT_RANGE,
};
pub use do_commands_wire::{apply_do_commands_live, DoCommandEffects, NameCandidate};
"#;
                t.replace_range(i..end, replacement);
                changed = true;
            }
        }
    }

    if !t.contains("DO-COMMANDS / say_commands") {
        let anchor = "        if upper.starts_with(\"PAY \") {";
        if t.contains(anchor) {
            if replace_once(&mut t, anchor, &format!("{DO_BLOCK}\n{anchor}")) {
                changed = true;
            }
        }
    }

    if changed {
        let _ = std::fs::write(&path, restore_nl(&t, crlf));
    }

    // docs (best-effort)
    let port = workspace.join("docs").join("port");
    if let Ok(raw) = std::fs::read_to_string(port.join("TODO_PORT.md")) {
        let crlf = raw.contains("\r\n");
        let mut td = normalize_nl(&raw);
        let old = "- [ ] Full Haxe `doCommands` list parity (MAKE for AI hire, etc.)  \n";
        let new = "- [x] **DO-COMMANDS** say_commands — pure `speech::parse_do_command` + live `do_commands_wire` (`I EXILE`/`I FOLLOW`/`I HIRE`/`I GIVE`/`ORDER,`/`OWN THIS`/`HOME!`); residual delayed follow confirm + AiBase MAKE/CRAFT  \n";
        if replace_once(&mut td, old, new) {
            let _ = std::fs::write(port.join("TODO_PORT.md"), restore_nl(&td, crlf));
        }
    }
    if let Ok(raw) = std::fs::read_to_string(port.join("QUEUE.md")) {
        let crlf = raw.contains("\r\n");
        let mut td = normalize_nl(&raw);
        let _ = replace_once(
            &mut td,
            "| 52 | `DO-COMMANDS` | say_commands | **running** |",
            "| 52 | ~~`DO-COMMANDS`~~ | say_commands | **DONE** (core) |",
        );
        let _ = replace_once(
            &mut td,
            "| `DO-COMMANDS` | workflow (new) | say_commands |",
            "| ~~`DO-COMMANDS`~~ | done | say_commands **DONE** (core) |",
        );
        let _ = std::fs::write(port.join("QUEUE.md"), restore_nl(&td, crlf));
    }
    if let Ok(raw) = std::fs::read_to_string(port.join("FILE_MATRIX.md")) {
        let crlf = raw.contains("\r\n");
        let mut td = normalize_nl(&raw);
        if td.contains("GPI-SAY") && !td.contains("**DO-COMMANDS**") {
            if let Some(i) = td.find("| GPI-SAY") {
                if let Some(end) = td[i..].find('\n') {
                    let row = "| GPI-SAY / **MUTE-SAY** / **DO-COMMANDS** | sayHelper + mute + doCommands | PARTIAL → mute DONE + **doCommands core DONE** | pure `speech` + `do_commands_wire`; residual delayed follow / MAKE hear |\n";
                    td.replace_range(i..i + end + 1, row);
                    let _ = std::fs::write(port.join("FILE_MATRIX.md"), restore_nl(&td, crlf));
                }
            }
        }
    }

    std::fs::read_to_string(&path)
        .map(|s| s.contains("mod do_commands_wire;") && s.contains("apply_do_commands_live"))
        .unwrap_or(false)
}

pub fn patch_search_best_food(manifest: &Path, src: &Path, workspace: &Path) -> bool {
    // AI-FOOD-FAIL-MARK piggyback (always attempt; idempotent)
    let _ = ai_food_fail_mark_wire::patch_all(src, workspace);

    let lib_path = src.join("lib.rs");
    let Ok(raw) = std::fs::read_to_string(&lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("mod search_best_food;") {
        if replace_once(
            &mut t,
            "mod yum;\nmod food_fill;",
            "mod yum;\n// Haxe: AiHelper.SearchBestFood / processFood (SEARCH-BEST-FOOD / ai_food_search)\nmod search_best_food;\nmod food_fill;",
        ) {
            changed = true;
        }
    }

    if !t.contains("pub use search_best_food::") {
        let old = "pub use yum::{\n\
    append_distance_suffix, can_eat_obj, can_feed_to_me_obj, compute_eat, dont_change_craving,\n\
    format_display_food_label, format_display_food_text, hold_food_emote, is_holding_meh,\n\
    is_holding_yum, is_obj_meh, is_obj_super_meh, is_obj_yum, loved_food_ids_for_biome,\n\
    loved_food_ids_for_person_color, pick_best_food, refuse_self_eat_super_meh,\n\
    should_display_best_food, starving_factor, CravingWire, EatCompute, FoodCandidate,\n\
    NearbyBestFood, PendingDisplayFood, YumState, FOOD_FACTOR, FOOD_REDUCTION_FAKTOR_MEH,\n\
    FOOD_REDUCTION_PER_EATING, HEALTH_LOST_MEH, HEALTH_LOST_SUPER_MEH, LOVED_FOOD_RESTORE,\n\
    YUM_BONUS, YUM_FOOD_RESTORE, YUM_NEW_CRAVING_CHANCE,\n\
};\n\
pub use food_fill::{";
        let new = "pub use yum::{\n\
    append_distance_suffix, can_eat_obj, can_feed_to_me_obj, compute_eat, dont_change_craving,\n\
    format_display_food_label, format_display_food_text, hold_food_emote, is_holding_meh,\n\
    is_holding_yum, is_obj_meh, is_obj_super_meh, is_obj_yum, loved_food_ids_for_biome,\n\
    loved_food_ids_for_person_color, pick_best_food, refuse_self_eat_super_meh,\n\
    should_display_best_food, starving_factor, CravingWire, EatCompute, FoodCandidate,\n\
    NearbyBestFood, PendingDisplayFood, YumState, FOOD_FACTOR, FOOD_REDUCTION_FAKTOR_MEH,\n\
    FOOD_REDUCTION_PER_EATING, HEALTH_LOST_MEH, HEALTH_LOST_SUPER_MEH, LOVED_FOOD_RESTORE,\n\
    YUM_BONUS, YUM_FOOD_RESTORE, YUM_NEW_CRAVING_CHANCE,\n\
};\n\
// Haxe: AiHelper.SearchBestFood (SEARCH-BEST-FOOD / ai_food_search)\n\
pub use search_best_food::{\n\
    container_blocks_remove, count_parent_in_radius, food_factor_from_eaten_percentage,\n\
    is_dangerous_near, pick_best_search_food, process_food, to_best_hit, AiFoodSearchFlags,\n\
    BestFoodHit, ProcessFoodOpts, ProcessFoodScore, SearchFoodCand, SearchFoodCounters,\n\
    BLOCKS_REMOVE_CONTAINER_IDS, CARROT_ID, CARROT_PILE_ID, CARROT_ROW_ID, COOKED_GOOSE_ID,\n\
    DRIED_CORN_ID, FOOD_DANGER_RADIUS, FOOD_STOCK_COUNT_RADIUS, FRUITING_PEPPER_ID,\n\
    HOT_PEPPER_ID, ONION_ID, PILE_SHUCKED_CORN_ID, RIPE_ONIONS_ID, SEARCH_BEST_FOOD_RADIUS,\n\
    SHUCKED_CORN_ID, WILD_ONION_ID,\n\
};\n\
pub use food_fill::{";
        if replace_once(&mut t, old, new) {
            changed = true;
        }
    }

    if !t.contains("search_best_food_full") && !t.contains("search_best_food_live.inc.rs") {
        if replace_once(&mut t, FN_OLD, FN_NEW) {
            changed = true;
        }
    }

    if changed {
        let out = restore_nl(&t, crlf);
        if std::fs::write(&lib_path, out).is_err() {
            return false;
        }
    }

    let yum_path = src.join("yum.rs");
    if let Ok(raw_y) = std::fs::read_to_string(&yum_path) {
        let crlf_y = raw_y.contains("\r\n");
        let mut y = normalize_nl(&raw_y);
        let old_s = "    let starving = if food_store < 0.5 {\n\
        2.0\n\
    } else if food_store < 3.0 {\n\
        4.0\n\
    } else {\n\
        16.0\n\
    };";
        let new_s = "    // Full Haxe cascade via starving_factor (includes < -1 / < -1.5)\n\
    let starving = starving_factor(food_store);";
        if y.contains(old_s) && replace_once(&mut y, old_s, new_s) {
            let _ = std::fs::write(&yum_path, restore_nl(&y, crlf_y));
            changed = true;
        }
    }

    // SEARCH-BEST-FOOD python fallback
    if !search_best_food_wired(&std::fs::read_to_string(&lib_path).unwrap_or_default()) {
        let py = manifest.join("_apply_search_best_food.py");
        if py.is_file() {
            let _ = std::process::Command::new("python")
                .arg(&py)
                .current_dir(manifest)
                .status();
            let _ = std::process::Command::new("python3")
                .arg(&py)
                .current_dir(manifest)
                .status();
        }
    }

    // DO-COMMANDS piggyback (always attempt)
    {
        let apply_dc = src.join("_apply_do_commands.py");
        if apply_dc.is_file() {
            let _ = std::process::Command::new("python")
                .arg(&apply_dc)
                .current_dir(manifest)
                .status();
            let _ = std::process::Command::new("python3")
                .arg(&apply_dc)
                .current_dir(manifest)
                .status();
        }
        let _ = patch_do_commands_inline(src, workspace);
    }

    // HEALTH-AGE-FOOD / health_food_max piggyback (always attempt)
    let haf = health_age_food_wire::patch_health_age_food(manifest, src, workspace);
    if !haf {
        // Also try python helper
        let py = manifest.join("_patch_health_age_food.py");
        if py.is_file() {
            let _ = std::process::Command::new("python")
                .arg(&py)
                .current_dir(manifest)
                .status();
            let _ = std::process::Command::new("python3")
                .arg(&py)
                .current_dir(manifest)
                .status();
            let _ = health_age_food_wire::patch_health_age_food(manifest, src, workspace);
        }
    }

    // WORLD-FOOD-FACTOR / food_factor piggyback (always attempt)
    let wff = world_food_factor_wire::patch_world_food_factor(manifest, src, workspace);
    if !wff {
        let py = src.join("_apply_world_food_factor.py");
        if py.is_file() {
            let _ = std::process::Command::new("python")
                .arg(&py)
                .current_dir(manifest)
                .status();
            let _ = std::process::Command::new("python3")
                .arg(&py)
                .current_dir(manifest)
                .status();
            let _ = world_food_factor_wire::patch_world_food_factor(manifest, src, workspace);
        }
    }

    let lib_now = std::fs::read_to_string(&lib_path).unwrap_or_default();
    search_best_food_wired(&lib_now) || changed
}
