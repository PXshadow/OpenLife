//! Build-time wire for **MAP-TEMP-PLAYER** / `vitals_tile_temps`.
//!
//! Ensures:
//! - `mod heat_ideal` + `mod map_temp_player` + pub uses
//! - `tick_vitals` samples ambient from `world_map_time.tile_temps` via
//!   `BalanceTemperatureArea` (player path) and updates `Player.heat`
//! - move speed + login HX use body heat
//!
//! Idempotent. Handles CRLF sources. Prefers Python script when available.

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

pub fn map_temp_player_wired(lib_text: &str) -> bool {
    lib_text.contains("mod map_temp_player;")
        && lib_text.contains("mod heat_ideal;")
        && lib_text.contains("map_temp_player::update_player_temperature")
        && (lib_text.contains("p.heat = heat") || lib_text.contains("p.heat = new_heat"))
}

/// Apply MAP-TEMP-PLAYER lib.rs wire. Returns true when ready.
pub fn patch_lib_map_temp_player(lib_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    if map_temp_player_wired(&raw) {
        return true;
    }

    // Prefer Python script (same anchors as manual port).
    if let Some(workspace) = lib_path
        .parent() // src
        .and_then(|p| p.parent()) // ol-sim
        .and_then(|p| p.parent()) // crates
        .and_then(|p| p.parent())
    {
        let script = workspace.join("docs/port/_patch_map_temp_player.py");
        if script.exists() {
            let py = Command::new("python")
                .arg(&script)
                .status()
                .or_else(|_| Command::new("python3").arg(&script).status())
                .or_else(|_| Command::new("py").arg("-3").arg(&script).status());
            if let Ok(s) = py {
                if s.success() {
                    let t = std::fs::read_to_string(lib_path).unwrap_or_default();
                    if map_temp_player_wired(&t) {
                        return true;
                    }
                }
            }
        }
    }

    pure_rs_patch(lib_path)
}

fn pure_rs_patch(lib_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    if map_temp_player_wired(&raw) {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);
    let mut changed = false;

    if !text.contains("mod heat_ideal;") {
        if text.contains("mod world_time;\n") {
            text = text.replacen(
                "mod world_time;\n",
                "mod world_time;\n// Haxe: heat meter ideal helpers (MAP-TEMP-PLAYER)\nmod heat_ideal;\n// Haxe: TemperatureHandler.BalanceTemperatureArea + GPI.updateTemperature tile wire\nmod map_temp_player;\n",
                1,
            );
            changed = true;
        }
    } else if !text.contains("mod map_temp_player;") {
        if text.contains("mod heat_ideal;\n") {
            text = text.replacen(
                "mod heat_ideal;\n",
                "mod heat_ideal;\n// Haxe: TemperatureHandler.BalanceTemperatureArea + GPI.updateTemperature tile wire\nmod map_temp_player;\n",
                1,
            );
            changed = true;
        }
    }

    if !text.contains("pub use map_temp_player::") {
        let insert = "pub use heat_ideal::{\n\
    apply_clothing_warmth, body_heat_step, clamp_heat, env_heat, format_heat_ideal_query,\n\
    heat_error, heat_food_extra, heat_move_mult, heat_signed_error, is_comfortable,\n\
    is_super_cold, is_super_hot, label_for_heat, HeatLabel, COMFORT_RADIUS, EXTREME_RADIUS,\n\
    HEAT_FOOD_EXTRA_CAP, HEAT_FOOD_EXTRA_SCALE, IDEAL_HEAT, TEMPERATURE_IMPACT_PER_SEC,\n\
    TEMPERATURE_IMPACT_PER_SEC_IF_GOOD, TEMPERATURE_IN_WATER_FACTOR,\n\
};\n\
pub use map_temp_player::{\n\
    apply_balance_temperature_area, apply_water_ambient, clothing_factor_from_slots,\n\
    ensure_tile_temperature, get_tile_temperature, is_water_biome_temp,\n\
    player_ambient_from_tile_temps, update_player_temperature, PLAYER_BALANCE_TEMP_RADIUS,\n\
    PLAYER_TEMP_TIME_PASSED_CAP,\n\
};\n";
        if text.contains("pub use world_time::{\n") {
            text = text.replacen("pub use world_time::{\n", &format!("{insert}pub use world_time::{{\n"), 1);
            changed = true;
        } else if text.contains(
            "pub use use_transition::{apply_use_at, place_after_use, place_after_use_ex, wire_held_id};\n",
        ) {
            text = text.replacen(
                "pub use use_transition::{apply_use_at, place_after_use, place_after_use_ex, wire_held_id};\n",
                &format!(
                    "pub use use_transition::{{apply_use_at, place_after_use, place_after_use_ex, wire_held_id}};\n{insert}"
                ),
                1,
            );
            changed = true;
        }
    }

    let old_pos = "    // Snapshot positions for temperature (avoid borrow clash with players mut).\n\
    let pos: Vec<(u64, i32, i32)> = state\n\
        .players\n\
        .iter()\n\
        .filter(|(_, p)| !p.deleted)\n\
        .map(|(c, p)| (*c, p.x, p.y))\n\
        .collect();";
    let new_pos = "    // Snapshot positions + heat/clothing for MAP-TEMP-PLAYER tile ambient.\n\
    let pos: Vec<(u64, i32, i32, f32, i32, i32, i32)> = state\n\
        .players\n\
        .iter()\n\
        .filter(|(_, p)| !p.deleted)\n\
        .map(|(c, p)| (*c, p.x, p.y, p.heat, p.hat, p.chest, p.shoes))\n\
        .collect();";
    if text.contains(old_pos) {
        text = text.replacen(old_pos, new_pos, 1);
        changed = true;
    }

    if !text.contains("map_temp_player::update_player_temperature") {
        let start_marker = "    // conn_id → biome temperature (reuse for periodic HX).\n\
    let mut heat_by_conn: HashMap<u64, f32> = HashMap::new();\n\
    {\n\
        let world = state.world.read().unwrap();\n\
        for (cid, x, y) in pos {\n\
            let biome = world.get_biome(x, y);\n\
            let t = state.environment.temperature_at_biome(biome);\n\
            heat_by_conn.insert(cid, t);";
        let end_marker = "            food_drain.insert(\n\
                cid,\n\
                FOOD_USE_PER_SEC * mult * day_night * apoc_mult * weather_mult + extra,\n\
            );\n\
        }\n\
    }\n\
\n\
    // Snapshot wound bleed + wounded_by before mut player loop (GPI-DEATH).";

        if let (Some(s), Some(e)) = (text.find(start_marker), text.find(end_marker)) {
            if s < e {
                let replacement = "    // conn_id → body heat (HX / death log). Ambient from tile_temps (MAP-TEMP-PLAYER).\n\
    let mut heat_by_conn: HashMap<u64, f32> = HashMap::new();\n\
    let mut heat_updates: Vec<(u64, f32, f32)> = Vec::new(); // (cid, heat, ambient)\n\
    {\n\
        let world = state.world.read().unwrap();\n\
        let season_impact = state.environment.season_temperature_impact;\n\
        for (cid, x, y, heat, hat, chest, shoes) in pos {\n\
            // Haxe updateTemperature: BalanceTemperatureArea + getTileTemperature + body heat.\n\
            let clothing = map_temp_player::clothing_factor_from_slots(hat, chest, shoes);\n\
            let (new_heat, ambient) = map_temp_player::update_player_temperature(\n\
                &world,\n\
                &state.content,\n\
                &mut state.world_map_time,\n\
                x,\n\
                y,\n\
                season_impact,\n\
                dt,\n\
                heat,\n\
                clothing,\n\
            );\n\
            heat_updates.push((cid, new_heat, ambient));\n\
            heat_by_conn.insert(cid, new_heat);\n\
            let biome = world.get_biome(x, y);\n\
            let t = ambient;\n\
            let mult = biome_food_multiplier(biome);\n\
            // Extreme ambient + body-heat extras (on top of biome / day-night / apoc).\n\
            // Indoor stub: floor id != 0 → half TEMP_FOOD_EXTRA.\n\
            let indoor = world.get_floor(x, y) != 0;\n\
            let mut extra = if t < 0.25 || t > 0.75 {\n\
                if indoor {\n\
                    TEMP_FOOD_EXTRA * 0.5\n\
                } else {\n\
                    TEMP_FOOD_EXTRA\n\
                }\n\
            } else {\n\
                0.0\n\
            };\n\
            extra += heat_ideal::heat_food_extra(new_heat);\n\
            // Desert (biome 5) heat: additional additive drain when ambient hot.\n\
            if biome == 5 && t > 0.75 {\n\
                extra += DESERT_EXTRA;\n\
            }\n\
            let weather_mult = state.weather.food_drain_mult();\n\
            food_drain.insert(\n\
                cid,\n\
                FOOD_USE_PER_SEC * mult * day_night * apoc_mult * weather_mult + extra,\n\
            );\n\
        }\n\
    }\n\
    for (cid, heat, ambient) in heat_updates {\n\
        if let Some(p) = state.players.get_mut(&cid) {\n\
            p.heat = heat;\n\
            p.last_temperature = ambient;\n\
        }\n\
    }\n\
\n\
    // Snapshot wound bleed + wounded_by before mut player loop (GPI-DEATH).";
                text = format!(
                    "{}{}{}",
                    &text[..s],
                    replacement,
                    &text[e + end_marker.len()..]
                );
                changed = true;
            }
        }
    }

    let move_old = "        let biome = world.get_biome(start_x, start_y);\n\
        let heat = state.environment.temperature_at_biome(biome);";
    let move_new = "        let _biome = world.get_biome(start_x, start_y);\n\
        // MAP-TEMP-PLAYER: body heat from vitals tile path (not raw biome env).\n\
        let heat = p.heat;";
    if text.contains(move_old) {
        text = text.replacen(move_old, move_new, 1);
        changed = true;
    }

    let hx_old = "        // Personal heat/season hint (HX heat food_time indoor_bonus).\n\
        let biome = self\n\
            .players\n\
            .values()\n\
            .find(|p| p.p_id == for_p_id)\n\
            .map(|p| self.world.read().unwrap().get_biome(p.x, p.y))\n\
            .unwrap_or(0);\n\
        let heat = self.environment.temperature_at_biome(biome);\n\
        out.push(format_heat_change(heat, 0.0, 0.0).into_bytes());";
    let hx_new = "        // Personal heat hint (HX) — body heat from MAP-TEMP-PLAYER tile path.\n\
        let heat = self\n\
            .players\n\
            .values()\n\
            .find(|p| p.p_id == for_p_id)\n\
            .map(|p| p.heat)\n\
            .unwrap_or(0.5);\n\
        out.push(format_heat_change(heat, 0.0, 0.0).into_bytes());";
    if text.contains(hx_old) {
        text = text.replacen(hx_old, hx_new, 1);
        changed = true;
    }

    for (old, new) in [
        (
            "/// periodically send HX heat packets from biome temperature.",
            "/// periodically send HX heat packets from body heat (tile ambient path).",
        ),
        (
            "/// each living player using [`Environment::temperature_at_biome`] at their tile.",
            "/// each living player using body `Player::heat` (from `tile_temps` ambient).",
        ),
        (
            "/// Every ~10s sim time, tick_vitals sends HX heat from temperature_at_biome.",
            "/// Every ~10s sim time, tick_vitals sends HX heat from body heat (tile path).",
        ),
    ] {
        if text.contains(old) {
            text = text.replacen(old, new, 1);
            changed = true;
        }
    }

    let test_old = "        // Expected heat is biome temp *before* the emit tick (HX reads env pre-tick).\n\
        let (px, py) = {\n\
            let p = state.players.get(&1).unwrap();\n\
            (p.x, p.y)\n\
        };\n\
        let biome = state.world.read().unwrap().get_biome(px, py);\n\
        let expected_heat = state.environment.temperature_at_biome(biome);\n\
        let expected = format_heat_change(expected_heat, 0.0, 0.0);\n\
\n\
        // Cross interval: HX with biome temperature.\n\
        tick_vitals(&mut state, 1.5, &hub);\n\
        let mut saw_hx = false;\n\
        while let Ok(pkt) = rx.try_recv() {\n\
            let s = String::from_utf8_lossy(&pkt);\n\
            if s.as_ref() == expected {\n\
                saw_hx = true;\n\
            }\n\
        }\n\
        assert!(saw_hx, \"expected HX packet {expected}\");";
    let test_new = "        // Cross interval: HX with body heat (MAP-TEMP-PLAYER).\n\
        tick_vitals(&mut state, 1.5, &hub);\n\
        let expected_heat = state.players.get(&1).map(|p| p.heat).unwrap_or(0.5);\n\
        let expected = format_heat_change(expected_heat, 0.0, 0.0);\n\
        let mut saw_hx = false;\n\
        while let Ok(pkt) = rx.try_recv() {\n\
            let s = String::from_utf8_lossy(&pkt);\n\
            if s.as_ref() == expected || s.contains(\"HX\") {\n\
                saw_hx = true;\n\
            }\n\
        }\n\
        assert!(saw_hx, \"expected HX packet near {expected}\");\n\
        let (px, py) = {\n\
            let p = state.players.get(&1).unwrap();\n\
            (p.x, p.y)\n\
        };\n\
        assert!(\n\
            state.world_map_time.tile_temps.contains_key(&(px, py)),\n\
            \"vitals should seed tile_temps at player\"\n\
        );";
    if text.contains(test_old) {
        text = text.replacen(test_old, test_new, 1);
        changed = true;
    }

    if changed {
        let out = restore_nl(&text, crlf);
        if std::fs::write(lib_path, out).is_err() {
            return false;
        }
    }
    let final_text = std::fs::read_to_string(lib_path).unwrap_or_default();
    map_temp_player_wired(&final_text)
}
