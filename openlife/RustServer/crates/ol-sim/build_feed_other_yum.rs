//! Build-time wire for **FEED-OTHER-YUM / feed_full_eat**.
//!
//! - `mod feed_other_yum` + pure feeder prestige share
//! - `include!("feed_other_yum_live.inc.rs")` → `feed_other_full_eat`
//! - FEED / NURSE call sites use full compute_eat path
//! - integration tests + docs
//!
//! Idempotent. Runs Python apply when present.

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

fn replace_once(hay: &mut String, old: &str, new: &str) -> bool {
    if let Some(i) = hay.find(old) {
        hay.replace_range(i..i + old.len(), new);
        true
    } else {
        false
    }
}

/// True when live FEED-other path uses full compute_eat + prestige health.
pub fn feed_other_yum_wired(lib: &str) -> bool {
    lib.contains("mod feed_other_yum")
        && lib.contains("feed_other_yum_live.inc.rs")
        && lib.contains("fn feed_other_full_eat")
        && lib.contains("FEED-OTHER-YUM")
        && lib.contains("feed_other_full_eat(")
}

pub fn patch_feed_other_yum(src_dir: &Path, workspace: &Path) -> bool {
    let py = src_dir.join("_apply_feed_other_yum.py");
    if py.exists() {
        let _ = Command::new("python")
            .arg(&py)
            .current_dir(src_dir.parent().unwrap_or(src_dir))
            .status()
            .or_else(|_| {
                Command::new("python3")
                    .arg(&py)
                    .current_dir(src_dir.parent().unwrap_or(src_dir))
                    .status()
            });
    }

    let lib_path = src_dir.join("lib.rs");
    let ok = patch_lib(&lib_path);
    patch_docs(workspace);
    ok && feed_other_yum_wired(&std::fs::read_to_string(&lib_path).unwrap_or_default())
}

fn patch_lib(lib_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    if feed_other_yum_wired(&raw) {
        // still ensure tests if missing
        if raw.contains("feed_other_applies_compute_eat_yum_fill") {
            return true;
        }
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    // 1. mod feed_other_yum
    if !t.contains("mod feed_other_yum") {
        if replace_once(
            &mut t,
            "mod feed;\n",
            "mod feed;\n// Haxe: GlobalPlayerInstance.doEating feed-other (FEED-OTHER-YUM)\nmod feed_other_yum;\n",
        ) {
            changed = true;
        } else if replace_once(
            &mut t,
            "mod feed;\r\n",
            "mod feed;\n// Haxe: GlobalPlayerInstance.doEating feed-other (FEED-OTHER-YUM)\nmod feed_other_yum;\n",
        ) {
            changed = true;
        }
    }

    // 2. pub use
    if !t.contains("feed_other_feeder_prestige_delta")
        || !t.contains("pub use feed_other_yum::")
    {
        if !t.contains("pub use feed_other_yum::") {
            if replace_once(
                &mut t,
                "pub use feed::{\n",
                "// Haxe: doEating feed-other feeder prestige share (FEED-OTHER-YUM)\npub use feed_other_yum::{\n    feed_other_feeder_prestige_delta, FEED_OTHER_FEEDER_PRESTIGE_SHARE,\n};\npub use feed::{\n",
            ) {
                changed = true;
            }
        }
    }

    // 3. include live helpers near search_best_food live or after apply_feed_other_craving
    if !t.contains("feed_other_yum_live.inc.rs") {
        // Prefer include right after apply_feed_other_craving closes
        let markers = [
            "    true\n}\n\n\n/// True if any clothing slot / helper holds `id` (Haxe getClothingById).",
            "    true\n}\n\n/// True if any clothing slot / helper holds `id` (Haxe getClothingById).",
            "include!(\"search_best_food_live.inc.rs\");\n",
        ];
        let inserts = [
            "    true\n}\n\n// FEED-OTHER-YUM / feed_full_eat live doEating feed-other\ninclude!(\"feed_other_yum_live.inc.rs\");\n\n/// True if any clothing slot / helper holds `id` (Haxe getClothingById).",
            "    true\n}\n\n// FEED-OTHER-YUM / feed_full_eat live doEating feed-other\ninclude!(\"feed_other_yum_live.inc.rs\");\n\n/// True if any clothing slot / helper holds `id` (Haxe getClothingById).",
            "include!(\"search_best_food_live.inc.rs\");\n// FEED-OTHER-YUM / feed_full_eat live doEating feed-other\ninclude!(\"feed_other_yum_live.inc.rs\");\n",
        ];
        for (m, ins) in markers.iter().zip(inserts.iter()) {
            if t.contains(m) && !t.contains("feed_other_yum_live.inc.rs") {
                if replace_once(&mut t, m, ins) {
                    changed = true;
                    break;
                }
            }
        }
    }

    // 4. NURSE held-food path
    let nurse_old = r#"                    // WORLD-FOOD-FACTOR: Haxe doEating multiplies world×starving for feed-other
                    let feed_food_id = state.content.resolve_base_id(held_id);
                    let feed_food_val = state
                        .content
                        .get(feed_food_id)
                        .map(|d| d.food_value)
                        .unwrap_or(0);
                    let base_fill = if feed_food_val > 0 {
                        feed_food_val as f32
                    } else {
                        held_food_value
                    };
                    let scaled_fill =
                        feed_fill_with_world_factors(state, feed_food_id, base_fill);
                    let (new_food, leftover) =
                        apply_feed_amounts(scaled_fill, t_food, t_max);
                    let transferred = scaled_fill - leftover;
                    if transferred <= 0.0 {
                        let line = format!("{} {} FAIL full", feeder_id, upper);
                        send_ps_reply(outbound, conn_id, &line);
                    } else {
                        if let Some(feeder) = state.players.get_mut(&conn_id) {
                            feeder.held_id = 0;
                        }
                        if let Some(tp) = state.players.get_mut(&t_conn) {
                            tp.food = new_food;
                        }
                        // Haxe: WorldMap.addFoodStatistic after fill (doEating L3215)
                        state.world_food.add_food_statistic(
                            feed_food_id,
                            base_fill,
                            transferred,
                        );
                        // CRAVING-WIRE: feed-other hasEaten + CR (dontChange always)
                        let _ = apply_feed_other_craving(
                            state,
                            t_conn,
                            feed_food_id,
                            feed_food_val,
                        );
                        send_craving_and_display_after_eat(state, outbound, t_conn);
                        state.publish_player_view(conn_id);
                        state.publish_player_view(t_conn);
                        let near = nearby_conn_ids(state, feeder_x, feeder_y, NEARBY_RANGE);
                        if let Some(tp) = state.players.get(&t_conn) {
                            let fx = food_change_for_player(state, tp);
                            send_nearby(outbound, &near, fx.into_bytes());
                        }
                        let line = format!(
                            "{} {} {} OK food={:.2}",
                            feeder_id, upper, baby_p_id, new_food
                        );
                        send_nearby_ps_lines(outbound, &near, &line);
                    }
"#;
    let nurse_new = r#"                    // FEED-OTHER-YUM: full doEating compute_eat + world×starving + prestige
                    let feed_food_id = state.content.resolve_base_id(held_id);
                    let feed_food_val = state
                        .content
                        .get(feed_food_id)
                        .map(|d| d.food_value)
                        .unwrap_or(0);
                    let food_val = if feed_food_val > 0 {
                        feed_food_val
                    } else {
                        held_food_value.round().max(0.0) as i32
                    };
                    let _ = (t_food, t_max);
                    match feed_other_full_eat(state, conn_id, t_conn, feed_food_id, food_val) {
                        Err(reason) => {
                            let line = format!("{} {} FAIL {}", feeder_id, upper, reason);
                            send_ps_reply(outbound, conn_id, &line);
                        }
                        Ok(_gain) => {
                            if let Some(feeder) = state.players.get_mut(&conn_id) {
                                feeder.held_id = 0;
                            }
                            send_craving_and_display_after_eat(state, outbound, t_conn);
                            state.publish_player_view(conn_id);
                            state.publish_player_view(t_conn);
                            let near =
                                nearby_conn_ids(state, feeder_x, feeder_y, NEARBY_RANGE);
                            let new_food = state
                                .players
                                .get(&t_conn)
                                .map(|tp| tp.food)
                                .unwrap_or(t_food);
                            if let Some(tp) = state.players.get(&t_conn) {
                                let fx = food_change_for_player(state, tp);
                                send_nearby(outbound, &near, fx.into_bytes());
                            }
                            let line = format!(
                                "{} {} {} OK food={:.2}",
                                feeder_id, upper, baby_p_id, new_food
                            );
                            send_nearby_ps_lines(outbound, &near, &line);
                        }
                    }
"#;
    if t.contains(nurse_old) {
        t = t.replacen(nurse_old, nurse_new, 1);
        changed = true;
    }

    // 5. FEED <p_id> path
    let feed_old = r#"                        // WORLD-FOOD-FACTOR: Haxe doEating multiplies world×starving for feed-other
                        let feed_food_id = state.content.resolve_base_id(held_id);
                        let feed_food_val = state
                            .content
                            .get(feed_food_id)
                            .map(|d| d.food_value)
                            .unwrap_or(0);
                        let base_fill = if feed_food_val > 0 {
                            feed_food_val as f32
                        } else {
                            held_food_value
                        };
                        let scaled_fill =
                            feed_fill_with_world_factors(state, feed_food_id, base_fill);
                        let (new_food, leftover) =
                            apply_feed_amounts(scaled_fill, t_food, t_max);
                        let transferred = scaled_fill - leftover;
                        if transferred <= 0.0 {
                            let line = format!("{} FEED {} FAIL full", feeder_id, target_id);
                            send_ps_reply(outbound, conn_id, &line);
                        } else {
                            // Poisoned food: apply sick to target on successful FEED.
                            let held_name = held_object_name(state, held_id);
                            let apply_sick =
                                should_sicken_on_feed(&held_name, held_is_food);
                            // Consume held food item (discrete object); update target food.
                            if let Some(feeder) = state.players.get_mut(&conn_id) {
                                feeder.held_id = 0;
                            }
                            if let Some(tp) = state.players.get_mut(&t_conn) {
                                tp.food = new_food;
                                if apply_sick {
                                    tp.sick = true;
                                }
                            }
                            // Haxe: WorldMap.addFoodStatistic after fill (doEating L3215)
                            state.world_food.add_food_statistic(
                                feed_food_id,
                                base_fill,
                                transferred,
                            );
                            // CRAVING-WIRE: Haxe playerTo.doIncreaseFoodValue(…, dontChange=true)
                            let _ = apply_feed_other_craving(
                                state,
                                t_conn,
                                feed_food_id,
                                feed_food_val,
                            );
                            send_craving_and_display_after_eat(state, outbound, t_conn);
"#;
    let feed_new = r#"                        // FEED-OTHER-YUM: full doEating compute_eat + prestige health
                        let feed_food_id = state.content.resolve_base_id(held_id);
                        let feed_food_val = state
                            .content
                            .get(feed_food_id)
                            .map(|d| d.food_value)
                            .unwrap_or(0);
                        let food_val = if feed_food_val > 0 {
                            feed_food_val
                        } else {
                            held_food_value.round().max(0.0) as i32
                        };
                        let _ = (t_food, t_max);
                        match feed_other_full_eat(
                            state,
                            conn_id,
                            t_conn,
                            feed_food_id,
                            food_val,
                        ) {
                            Err(reason) => {
                                let line = format!(
                                    "{} FEED {} FAIL {}",
                                    feeder_id, target_id, reason
                                );
                                send_ps_reply(outbound, conn_id, &line);
                            }
                            Ok(_gain) => {
                            // Poisoned food: apply sick to target on successful FEED.
                            let held_name = held_object_name(state, held_id);
                            let apply_sick =
                                should_sicken_on_feed(&held_name, held_is_food);
                            if let Some(feeder) = state.players.get_mut(&conn_id) {
                                feeder.held_id = 0;
                            }
                            if apply_sick {
                                if let Some(tp) = state.players.get_mut(&t_conn) {
                                    tp.sick = true;
                                }
                            }
                            let new_food = state
                                .players
                                .get(&t_conn)
                                .map(|tp| tp.food)
                                .unwrap_or(t_food);
                            send_craving_and_display_after_eat(state, outbound, t_conn);
"#;
    // Heal half-applied FEED close (open patch missed, close still applied).
    if t.contains("} // Ok feed_other_full_eat") && !t.contains("match feed_other_full_eat(") {
        let heal_broken = r#"                            send_nearby_ps_lines(outbound, &near, &line);
                            } // Ok feed_other_full_eat
                        } // match feed_other_full_eat
                    }
                    Err(reason) => {
                        let line = format!("{} FEED {} FAIL {}", feeder_id, target_id, reason);"#;
        let heal_fixed = r#"                            send_nearby_ps_lines(outbound, &near, &line);
                        }
                    }
                    Err(reason) => {
                        let line = format!("{} FEED {} FAIL {}", feeder_id, target_id, reason);"#;
        if t.contains(heal_broken) {
            t = t.replacen(heal_broken, heal_fixed, 1);
            changed = true;
        }
    }

    if t.contains(feed_old) {
        t = t.replacen(feed_old, feed_new, 1);
        changed = true;
        // Only close-match when open patch landed (otherwise re-breaks simple FEED).
        if t.contains("match feed_other_full_eat(") {
            let close_old = r#"                            send_nearby_ps_lines(outbound, &near, &line);
                        }
                    }
                    Err(reason) => {
                        let line = format!("{} FEED {} FAIL {}", feeder_id, target_id, reason);"#;
            let close_new = r#"                            send_nearby_ps_lines(outbound, &near, &line);
                            } // Ok feed_other_full_eat
                        } // match feed_other_full_eat
                    }
                    Err(reason) => {
                        let line = format!("{} FEED {} FAIL {}", feeder_id, target_id, reason);"#;
            if t.contains(close_old) {
                t = t.replacen(close_old, close_new, 1);
                changed = true;
            }
        }
    }

    // 6. Integration tests
    if !t.contains("feed_other_applies_compute_eat_yum_fill") {
        let tests = r#"
    /// FEED-OTHER-YUM: first-yum FEED fill uses compute_eat (food_value + YumBonus) × world.
    // Haxe: GlobalPlayerInstance.doEating L3087–3192 playerFrom != playerTo
    #[test]
    fn feed_other_applies_compute_eat_yum_fill() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut db = ContentDb::default();
        db.objects.insert(
            33,
            ObjectDef {
                id: 33,
                description: "Gooseberry".into(),
                name: "Gooseberry".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 5,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
                dummy_ids: Vec::new(),
                use_chance: 0.0,
                speed_mult: 1.0,
                winter_decay_factor: 0.0,
                spring_regrow_factor: 0.0,
                decay_factor: 1.0,
                decays_to_obj: 0,
                r_value: 0.0,
                clothing: "n".into(),
                counts_or_grows_as: 0,
                crafting_steps: 0,
                use_distance: 1,
                deadly_distance: 0.0,
                moves: 0,
            damage: 0.0,
            damage_protection_factor: 1.0,
            wound_factor: 0.5,
            male: false,
            contain_size: 0.0,
            slot_size: 1.0,
            },
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        let _a = spawn_player(&mut state, 1, "foy_a@test");
        let b = spawn_player(&mut state, 2, "foy_b@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 0;
            p.y = 0;
            p.held_id = 33;
        }
        {
            let p = state.players.get_mut(&2).unwrap();
            p.x = 1;
            p.y = 0;
            p.food = 2.0;
            p.food_max = 40.0;
        }
        let food_before = state.players.get(&2).unwrap().food;
        let feeder_id = state.players.get(&1).unwrap().p_id;
        let eater_id = state.players.get(&2).unwrap().p_id;
        let prest_feeder_before = state
            .combat
            .stats
            .get(&feeder_id)
            .map(|s| s.prestige)
            .unwrap_or(0.0);
        let prest_eater_before = state
            .combat
            .stats
            .get(&eater_id)
            .map(|s| s.prestige)
            .unwrap_or(0.0);
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("FEED {b}"),
            },
        );
        let gained = state.players.get(&2).unwrap().food - food_before;
        assert!(
            gained > 20.0,
            "FEED yum fill should use compute_eat (base+YumBonus)×world; gained={gained}"
        );
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
        let prest_eater = state
            .combat
            .stats
            .get(&eater_id)
            .map(|s| s.prestige)
            .unwrap_or(0.0);
        let prest_feeder = state
            .combat
            .stats
            .get(&feeder_id)
            .map(|s| s.prestige)
            .unwrap_or(0.0);
        assert!(
            prest_eater > prest_eater_before,
            "eater yum prestige should rise"
        );
        assert!(
            prest_feeder > prest_feeder_before,
            "feeder should get 0.2 yum prestige share"
        );
    }

    /// FEED-OTHER-YUM: meh food refused when eater food_store > 2.
    // Haxe: GlobalPlayerInstance.doEating L3108–3111 / canFeedToMeObj
    #[test]
    fn feed_other_refuses_meh_when_not_starving() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut db = ContentDb::default();
        db.objects.insert(
            33,
            ObjectDef {
                id: 33,
                description: "Gooseberry".into(),
                name: "Gooseberry".into(),
                containable: true,
                permanent: false,
                blocks_walking: false,
                food_value: 5,
                heat_value: 0.0,
                map_chance: 0.0,
                biomes: Vec::new(),
                num_uses: 0,
                num_slots: 0,
                floor: false,
                dummy_ids: Vec::new(),
                use_chance: 0.0,
                speed_mult: 1.0,
                winter_decay_factor: 0.0,
                spring_regrow_factor: 0.0,
                decay_factor: 1.0,
                decays_to_obj: 0,
                r_value: 0.0,
                clothing: "n".into(),
                counts_or_grows_as: 0,
                crafting_steps: 0,
                use_distance: 1,
                deadly_distance: 0.0,
                moves: 0,
            damage: 0.0,
            damage_protection_factor: 1.0,
            wound_factor: 0.5,
            male: false,
            contain_size: 0.0,
            slot_size: 1.0,
            },
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        let _a = spawn_player(&mut state, 1, "meh_feed_a@test");
        let b = spawn_player(&mut state, 2, "meh_feed_b@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.x = 0;
            p.y = 0;
            p.held_id = 33;
        }
        {
            let p = state.players.get_mut(&2).unwrap();
            p.x = 1;
            p.y = 0;
            p.food = 10.0;
            p.food_max = 40.0;
            p.yum.has_eaten.insert(33, 5.0);
        }
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("FEED {b}"),
            },
        );
        assert_eq!(state.players.get(&1).unwrap().held_id, 33);
        assert!((state.players.get(&2).unwrap().food - 10.0).abs() < 1e-4);
        let mut saw_refuse = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("FAIL refuse") || s.contains("FAIL full") {
                saw_refuse = true;
            }
        }
        assert!(saw_refuse, "expected FEED FAIL refuse for meh when not starving");
    }

"#;
        if let Some(i) = t.find("fn feed_other_applies_world_food_factor_and_stats()") {
            let anchors = [
                "\n    /// WORLD-FOOD-FACTOR: superMeh with prestige burns",
                "\n    fn try_eat_held_super_meh_burns_prestige",
                "\n    /// CRAVING-WIRE: FEED-other keeps eater craving",
            ];
            for a in anchors {
                if let Some(rel) = t[i..].find(a) {
                    t.insert_str(i + rel, tests);
                    changed = true;
                    break;
                }
            }
        }
    }

    if changed {
        let out = restore_nl(&t, crlf);
        let _ = std::fs::write(lib_path, out);
    }
    let after = std::fs::read_to_string(lib_path).unwrap_or_default();
    // Wired enough for cargo to compile feed path if include + mod present
    after.contains("mod feed_other_yum")
        && after.contains("feed_other_yum_live.inc.rs")
        && after.contains("feed_other_full_eat")
}

fn patch_docs(workspace: &Path) {
    let port = workspace.join("docs/port");
    let todo = port.join("TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo) {
        let mut t = raw;
        let mut ch = false;
        if t.contains("- [ ] Feed-other full yum fill path") {
            t = t.replace(
                "- [ ] Feed-other full yum fill path (world factors now applied; still skips full yum/meh prestige health on FEED)  \n",
                "- [x] **FEED-OTHER-YUM feed_full_eat** — full `compute_eat_full` on FEED/NURSE + canFeedToMe meh refuse + yum/meh prestige health + feeder 0.2 share + superMeh trade; residual full `addHealthAndPrestige` parent/coin fan  \n",
            );
            ch = true;
        }
        if t.contains("Last updated:") && !t.contains("FEED-OTHER-YUM feed_full_eat") {
            if let Some(line) = t.lines().find(|l| l.starts_with("Last updated:")) {
                let new_line = "Last updated: **2026-07-29** (FEED-OTHER-YUM feed_full_eat) (AI-PROVIDER llm_http) (FOODSTATS-DISK foodstats_txt) (NOOB-NOBLE-SPAWN spawn_weights)";
                t = t.replacen(line, new_line, 1);
                ch = true;
            }
        }
        if ch {
            let _ = std::fs::write(&todo, t);
        }
    }

    let matrix = port.join("FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&matrix) {
        let mut t = raw;
        let mut ch = false;
        if t.contains("Residual: feed-other fill still raw food_value (not full compute_eat)") {
            t = t.replace(
                "Residual: feed-other fill still raw food_value (not full compute_eat)",
                "**FEED-OTHER-YUM DONE** full compute_eat on FEED/NURSE; residual parent prestige fan",
            );
            ch = true;
        }
        if t.contains("residual full feed yum fill / lineage 24h") {
            t = t.replace(
                "residual full feed yum fill / lineage 24h",
                "**FEED-OTHER-YUM DONE** / lineage 24h",
            );
            ch = true;
        }
        if !t.contains("| **FEED-OTHER-YUM**") {
            if let Some(i) = t.find("| GPI-FOOD / **CRAVING-WIRE**") {
                let row = "| **FEED-OTHER-YUM** / feed_full_eat | FEED/NURSE full doEating compute_eat yum/meh prestige | **DONE** (core) | `feed_other_full_eat` + `can_feed_to_me_obj_ex_yum` + `compute_eat_full` ×world×starving + CR + eater health_delta + feeder 0.2 + superMeh trade; tests `feed_other_applies_compute_eat_yum_fill` / `feed_other_refuses_meh_*` / pure `feed_other_feeder_prestige_delta_*`; residual full addHealthAndPrestige parent/coin fan |\n";
                t.insert_str(i, row);
                ch = true;
            }
        }
        if ch {
            let _ = std::fs::write(&matrix, t);
        }
    }

    let call = port.join("CALL_INDEX.md");
    if let Ok(raw) = std::fs::read_to_string(&call) {
        if !raw.contains("feed_other_full_eat") {
            let mut t = raw;
            t.push_str(
                r#"
### FEED-OTHER-YUM / feed_full_eat
| Symbol | Path | Notes |
|--------|------|-------|
| `doEating` (feed-other) | `server/GlobalPlayerInstance.hx` L3041–3247 | playerFrom ≠ playerTo |
| `feed_other_full_eat` | `ol-sim/feed_other_yum_live.inc.rs` | full compute_eat + prestige + superMeh |
| `feed_other_feeder_prestige_delta` | `ol-sim/feed_other_yum.rs` | yum feeder ×0.2 |
| `can_feed_to_me_obj_ex_yum` | `ol-sim/yum.rs` | meh refuse food>2 |
| Tests | `feed_other_applies_compute_eat_yum_fill` / `feed_other_refuses_meh_*` | live + pure |
"#,
            );
            let _ = std::fs::write(&call, t);
        }
    }

    let queue = port.join("QUEUE.md");
    if let Ok(raw) = std::fs::read_to_string(&queue) {
        let mut t = raw;
        let row = "| `FEED-OTHER-YUM` | feed_full_eat | FEED-other full compute_eat yum residual |\n";
        if t.contains(row) {
            t = t.replace(row, "");
            if !t.contains("**FEED-OTHER-YUM**") {
                t = t.replace(
                    "**AI-PROVIDER** llm_http DONE ·",
                    "**FEED-OTHER-YUM** feed_full_eat DONE · **AI-PROVIDER** llm_http DONE ·",
                );
            }
            let _ = std::fs::write(&queue, t);
        }
    }
}
