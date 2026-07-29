//! Build-time wire for **TIME-ANIMAL-OFFSPRING** / `pop_die_offspring`.
//!
//! - `mod animal_pop` + pub uses
//! - `tick_animals_dt` → `tick_movement_with_pop` (pop baked into AnimalWorld)
//! - Map MX for deaths/births via `apply_animal_pop_map_events`
//! - `spawn_default_animals` captures original population baseline
//! - animal_damage test Animal literal gains `failed_moves`
//! - CALL_INDEX animal section refresh (best-effort)

use std::path::Path;

pub fn animal_pop_wired(lib_text: &str) -> bool {
    lib_text.contains("mod animal_pop;")
        && lib_text.contains("tick_movement_with_pop")
        && lib_text.contains("capture_original_counts")
        && lib_text.contains("apply_animal_pop_map_events")
        && lib_text.contains("tick_animals_dt_full")
}

pub fn patch_animal_pop(src_dir: &Path) -> bool {
    let lib_path = src_dir.join("lib.rs");
    let damage_path = src_dir.join("animal_damage.rs");
    let Ok(mut lib) = std::fs::read_to_string(&lib_path) else {
        return false;
    };
    let mut changed = false;

    // --- mod animal_pop ---
    if !lib.contains("mod animal_pop;") {
        if lib.contains("mod animal_damage;\nmod animals;") {
            lib = lib.replacen(
                "mod animal_damage;\nmod animals;",
                "mod animal_damage;\nmod animal_pop;\nmod animals;",
                1,
            );
            changed = true;
        } else if lib.contains("mod animal_move;\nmod animal_damage;") {
            lib = lib.replacen(
                "mod animal_move;\nmod animal_damage;",
                "mod animal_move;\nmod animal_damage;\nmod animal_pop;",
                1,
            );
            changed = true;
        }
    }

    // --- pub use animals expanded ---
    if !lib.contains("AnimalMovementTick") {
        let old = "pub use animals::{\n    Animal, AnimalKind, AnimalSnapshot, AnimalView, AnimalWorld, AnimalWorldShare,\n    CloseDeadlyAnimal, ANIMAL_THREAT_RANGE, DEADLY_ANIMAL_SEARCH_DIST,\n};";
        let new = "pub use animals::{\n    Animal, AnimalBirthEvent, AnimalDeathEvent, AnimalDeathReason, AnimalDestInfo,\n    AnimalKind, AnimalMovementTick, AnimalSnapshot, AnimalView, AnimalWorld, AnimalWorldShare,\n    CloseDeadlyAnimal, ANIMAL_THREAT_RANGE, DEADLY_ANIMAL_SEARCH_DIST,\n};";
        if lib.contains(old) {
            lib = lib.replacen(old, new, 1);
            changed = true;
        }
    }

    // --- pub use animal_pop ---
    if !lib.contains("pub use animal_pop::") {
        let insert = r#"// Haxe: TimeHelper doAnimalMovement offspring / die-in-place / failedMoves (TIME-ANIMAL-OFFSPRING)
pub use animal_pop::{
    accumulate_failed_moves, apply_low_pop_offspring_boost, apply_overpop_dying_boost,
    apply_rabbit_wrong_place_dying, can_die_pop_fraction, chance_for_animal_dying,
    chance_for_offspring, compute_dying_chance, compute_offspring_chance, count_close_same_parent,
    failed_moves_kills, has_close_same_parent, lonely_death_override, natural_death_allowed,
    offspring_pop_allows, resolve_failed_move, resolve_pop_on_dest, roll_natural_death,
    roll_offspring, PopMoveOutcome, CAN_DIE_POP_FRACTION_DEFAULT, CAN_DIE_POP_FRACTION_RABBIT_WRONG,
    CHANCE_FOR_ANIMAL_DYING, CHANCE_FOR_ANIMAL_DYING_FACTOR_IF_IN_LOVED_BIOME, CHANCE_FOR_OFFSPRING,
    FAILED_MOVES_DEATH_THRESHOLD, MAX_OFFSPRING_FACTOR, MIN_CURRENT_POP_FOR_NATURAL_DEATH,
    OFFSPRING_FACTOR_IF_POP_LOW, OFFSPRING_FACTOR_LOW_POP_BELOW, OFFSPRING_MIN_SEPARATION,
};
"#;
        if let Some(idx) = lib.find("// Haxe: TimeHelper DoAnimalDamage") {
            lib.insert_str(idx, insert);
            changed = true;
        } else if let Some(idx) = lib.find("pub use animal_damage::{") {
            lib.insert_str(idx, insert);
            changed = true;
        }
    }

    // --- spawn_default_animals: capture original counts ---
    if !lib.contains("capture_original_counts") {
        let old = "    info!(\n        n = state.animals.animals.len(),\n        sx,\n        sy,\n        \"sim: default animals spawned on map near play area\"\n    );\n}";
        let new = "    // Haxe originalObjectsCount baseline for offspring/die gates.\n    state.animals.capture_original_counts();\n    info!(\n        n = state.animals.animals.len(),\n        sx,\n        sy,\n        \"sim: default animals spawned on map near play area\"\n    );\n}";
        if lib.contains(old) {
            lib = lib.replacen(old, new, 1);
            changed = true;
        }
    }

    // --- apply_animal_pop_map_events helper ---
    if !lib.contains("fn apply_animal_pop_map_events") {
        if let Some(idx) = lib.find("/// One wander step for all animals using pathfind walkability.") {
            lib.insert_str(idx, APPLY_POP_MAP_EVENTS_FN);
            changed = true;
        }
    }

    // --- tick_animals_dt rewrite ---
    if !lib.contains("tick_animals_dt_full") {
        let marker = "/// Timed animal movement: Haxe `doAnimalMovement` cadence + pathing + chase/biome.";
        if let Some(start) = lib.find(marker) {
            let after = &lib[start..];
            let end_rel = after
                .find("\n/// Haxe `TimeHelper.DoAnimalDamage`")
                .or_else(|| after.find("\nfn apply_animal_path_damages"))
                .unwrap_or(0);
            if end_rel > 50 {
                lib.replace_range(start..start + end_rel, TICK_ANIMALS_DT_NEW);
                changed = true;
            }
        }
    }

    // --- vitals: use full tick + pop map events ---
    if lib.contains("let moves = tick_animals_dt(state, dt);")
        && !lib.contains("apply_animal_pop_map_events(state, outbound")
    {
        lib = lib.replacen(
            "let moves = tick_animals_dt(state, dt);\n    for &(_id, kind, ox, oy, nx, ny) in &moves {",
            "let animal_tick = tick_animals_dt_full(state, dt);\n    let moves = animal_tick.moves.clone();\n    apply_animal_pop_map_events(state, outbound, &animal_tick);\n    for &(_id, kind, ox, oy, nx, ny) in &moves {",
            1,
        );
        changed = true;
    }

    // tick_animals thin wrapper
    if lib.contains(
        "pub fn tick_animals(state: &mut SimState) -> Vec<(i32, AnimalKind, i32, i32, i32, i32)> {\n    tick_animals_dt(state, 100.0)\n}",
    ) && lib.contains("tick_animals_dt_full")
    {
        lib = lib.replacen(
            "pub fn tick_animals(state: &mut SimState) -> Vec<(i32, AnimalKind, i32, i32, i32, i32)> {\n    tick_animals_dt(state, 100.0)\n}",
            "pub fn tick_animals(state: &mut SimState) -> Vec<(i32, AnimalKind, i32, i32, i32, i32)> {\n    tick_animals_dt_full(state, 100.0).moves\n}",
            1,
        );
        changed = true;
    }

    if changed {
        let _ = std::fs::write(&lib_path, &lib);
    }

    // --- animal_damage test Animal literal ---
    if let Ok(mut dmg) = std::fs::read_to_string(&damage_path) {
        if dmg.contains("target: None,\n        };\n        register_animal_hit")
            && !dmg.contains("failed_moves:")
        {
            dmg = dmg.replacen(
                "target: None,\n        };\n        register_animal_hit",
                "target: None,\n            failed_moves: 0.0,\n        };\n        register_animal_hit",
                1,
            );
            let _ = std::fs::write(&damage_path, dmg);
            changed = true;
        }
    }

    // --- CALL_INDEX best-effort ---
    if let Some(docs) = src_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|r| r.join("docs/port/CALL_INDEX.md"))
    {
        if let Ok(mut ci) = std::fs::read_to_string(&docs) {
            if !ci.contains("TIME-ANIMAL-OFFSPRING")
                && ci.contains("## Rust: animal move / chase (`TIME-ANIMAL` / `TIME-ANIMAL-CHASE`)")
            {
                if let Ok(tail) = std::fs::read_to_string(
                    docs.parent()
                        .unwrap()
                        .join("CALL_INDEX_TAIL_ANIMAL.md"),
                ) {
                    if let Some(start) =
                        ci.find("## Rust: animal move / chase (`TIME-ANIMAL` / `TIME-ANIMAL-CHASE`)")
                    {
                        ci.truncate(start);
                        ci.push_str(&tail);
                        let _ = std::fs::write(&docs, ci);
                    }
                }
            }
        }
    }

    let lib_now = std::fs::read_to_string(&lib_path).unwrap_or_default();
    animal_pop_wired(&lib_now) || changed
}

const APPLY_POP_MAP_EVENTS_FN: &str = r#"/// Apply map MX for animal natural death / failedMoves death / offspring births.
///
/// Haxe: die-in-place restores groundObject or `decaysToObj`; offspring places a
/// copy on `fromTx/fromTy`. Entity model clears/places content object ids.
// Haxe: TimeHelper.doAnimalMovement death + offspring map updates
fn apply_animal_pop_map_events(
    state: &mut SimState,
    outbound: &OutboundHub,
    tick: &AnimalMovementTick,
) {
    for death in &tick.deaths {
        let animal_obj = death.kind.object_id();
        let decays = state
            .content
            .get(animal_obj)
            .map(|d| d.decays_to_obj)
            .unwrap_or(0);
        let new_id = if decays > 0 { decays } else { 0 };
        {
            let mut w = state.world.write().unwrap();
            let at = w.get_object(death.x, death.y);
            if at == animal_obj || at == 0 {
                w.set_object(death.x, death.y, new_id);
            }
        }
        let floor = state.world.read().unwrap().get_floor(death.x, death.y) as i32;
        let near = nearby_conn_ids(state, death.x, death.y, NEARBY_RANGE.max(32));
        for &cid in &near {
            let Some(viewer) = state.players.get(&cid) else {
                continue;
            };
            if viewer.deleted || !viewer.connected {
                continue;
            }
            let (rx, ry) = viewer.world_to_client(death.x, death.y);
            outbound.send_urgent(
                cid,
                format_map_change(rx, ry, floor, new_id, -1).into_bytes(),
            );
            send_frame(outbound, cid);
        }
    }
    for birth in &tick.births {
        let animal_obj = birth.kind.object_id();
        {
            let mut w = state.world.write().unwrap();
            if w.get_object(birth.x, birth.y) == 0 {
                w.set_object(birth.x, birth.y, animal_obj);
            }
        }
        let floor = state.world.read().unwrap().get_floor(birth.x, birth.y) as i32;
        let obj = state.world.read().unwrap().get_object(birth.x, birth.y);
        let near = nearby_conn_ids(state, birth.x, birth.y, NEARBY_RANGE.max(32));
        for &cid in &near {
            let Some(viewer) = state.players.get(&cid) else {
                continue;
            };
            if viewer.deleted || !viewer.connected {
                continue;
            }
            let (rx, ry) = viewer.world_to_client(birth.x, birth.y);
            outbound.send_urgent(
                cid,
                format_map_change(rx, ry, floor, obj, -1).into_bytes(),
            );
            send_frame(outbound, cid);
        }
    }
}

"#;

const TICK_ANIMALS_DT_NEW: &str = r#"/// Timed animal movement: Haxe `doAnimalMovement` cadence + pathing + chase/biome
/// + offspring / natural die / failedMoves (**TIME-ANIMAL-OFFSPRING**).
///
/// - Interval: content auto-decay seconds (wolf/boar ~3s, rabbit ~1s)
/// - Chase: deadly animals toward nearest player (season/hits/tables); wolf/grizzly
///   toward closest bone grave in winter/snow; pack alert stamps nearby hits
/// - Loved biome: update lovedTx/Ty; steer home when not in spawn biome
/// - Preferred-biome bias on random candidates; path trim still applies when chasing
/// - Pop: natural die-in-place, offspring on origin, failedMoves>20 stuck death
pub fn tick_animals_dt(state: &mut SimState, dt: f32) -> Vec<(i32, AnimalKind, i32, i32, i32, i32)> {
    tick_animals_dt_full(state, dt).moves
}

/// Full animal tick including deaths/births for map MX wire.
pub fn tick_animals_dt_full(state: &mut SimState, dt: f32) -> AnimalMovementTick {
    let (ww, wh) = {
        let w = state.world.read().unwrap();
        (w.width_tiles, w.height_tiles)
    };
    if ww <= 0 || wh <= 0 || state.animals.animals.is_empty() || dt <= 0.0 {
        return AnimalMovementTick::default();
    }
    let world = Arc::clone(&state.world);
    let content = Arc::clone(&state.content);
    let season = state.environment.season;
    let players: Vec<(i32, i32)> = state
        .players
        .values()
        .filter(|p| !p.deleted && p.held_by == 0)
        .map(|p| (p.x, p.y))
        .collect();
    let interval_for = |kind: AnimalKind| -> f32 {
        let oid = kind.object_id();
        content
            .auto_decays
            .get(&oid)
            .map(|t| {
                if t.auto_decay_seconds > 0.0 {
                    t.auto_decay_seconds
                } else {
                    AnimalWorld::wander_interval(kind)
                }
            })
            .unwrap_or_else(|| AnimalWorld::wander_interval(kind))
    };
    let mut rng = rand::thread_rng();
    // Haxe: WorldMap.cursedGraves global list for GetClosestBoneGrave (fallback local scan).
    let global_bone_graves = world_time::index_positions(&state.world_map_time.cursed_graves);
    state.animals.tick_movement_with_pop(
        &mut rng,
        dt,
        ww,
        wh,
        Some(&interval_for),
        |rng, animals, i| {
            let w = world.read().unwrap();
            let kind = animals[i].kind;
            let parent_id = kind.object_id();
            let ax = animals[i].x;
            let ay = animals[i].y;
            let hits = animals[i].hits;
            let current_biome = w.get_biome(ax, ay);
            let loves_orig = animal_move::is_spawning_in(&content, parent_id, current_biome);
            animal_move::maybe_update_loved_biome(
                loves_orig,
                ax,
                ay,
                &mut animals[i].loved_tx,
                &mut animals[i].loved_ty,
            );
            let loves_current = loves_orig;

            let need_graves = (season == environment::Season::Winter || current_biome == 4)
                && (parent_id == animal_move::PARENT_WOLF
                    || parent_id == animal_move::PARENT_GRIZZLY);
            // Haxe: GetClosestBoneGrave over WorldMap.cursedGraves (global)
            let graves = if need_graves {
                if !global_bone_graves.is_empty() {
                    global_bone_graves.clone()
                } else {
                    animal_move::collect_bone_graves_near(&w, ax, ay, 80)
                }
            } else {
                Vec::new()
            };

            let existing_target = animals[i].target;
            let target_still_grave = existing_target
                .map(|(tx, ty)| animal_move::is_bone_grave(w.get_object(tx, ty)))
                .unwrap_or(false);

            let pack: Vec<(i32, i32, i32, f32)> = animals
                .iter()
                .map(|a| (a.kind.object_id(), a.x, a.y, a.hits))
                .collect();

            let chase = animal_move::resolve_animal_chase(
                parent_id,
                kind.is_deadly(),
                hits,
                season,
                current_biome,
                ax,
                ay,
                &players,
                &graves,
                &pack,
                i,
                existing_target,
                target_still_grave,
            );
            animals[i].target = chase.target;
            if let Some(pi) = chase.pack_alert_index {
                if pi < animals.len() && animals[pi].hits <= 0.0 {
                    animals[pi].hits = animal_move::PACK_ALERT_HITS;
                }
            }

            let rad = {
                let oid = parent_id;
                content
                    .auto_decays
                    .get(&oid)
                    .map(|t| {
                        let mut m = t.move_dist;
                        if m <= 0 {
                            m = AnimalWorld::move_radius(kind);
                        } else if m < 3 {
                            m += 1;
                        }
                        if t.desired_move_dist > 0 {
                            m = m.max(t.desired_move_dist.min(6));
                        }
                        m
                    })
                    .unwrap_or_else(|| AnimalWorld::move_radius(kind))
            };
            let rabbit = matches!(kind, AnimalKind::Rabbit);
            let steer = animal_move::AnimalSteer {
                object_id: parent_id,
                goto_target: chase.goto_target,
                target: chase.target,
                loved_tx: animals[i].loved_tx,
                loved_ty: animals[i].loved_ty,
                loves_current_biome: loves_current,
            };
            let dest = animal_move::pick_animal_destination_steered(
                &w,
                &content,
                rng,
                ax,
                ay,
                ww,
                wh,
                rad,
                rabbit,
                Some(steer),
            )?;
            let target_biome = w.get_biome(dest.0, dest.1);
            let is_preferred =
                animal_move::is_spawning_in(&content, parent_id, target_biome);
            Some(AnimalDestInfo {
                x: dest.0,
                y: dest.1,
                is_preferred_biome: is_preferred,
                rabbit_in_wrong_place: false,
                loves_current_biome: loves_current,
            })
        },
    )
}

"#;
