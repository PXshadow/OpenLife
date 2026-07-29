Haxe anchors: `TimeHelper.doTimeForObject`, `ObjectHelper.creationTimeInTicks` / `timeToChange` / `numberOfUses` on contained; ReadFromFile clamp.

## Rust: animal move / chase / pop (`TIME-ANIMAL` / `TIME-ANIMAL-CHASE` / `TIME-ANIMAL-OFFSPRING`)

| Symbol | File | Role |
|--------|------|------|
| `tick_animals` / `tick_animals_dt` / `tick_animals_dt_full` | `ol-sim/src/lib.rs` | doAnimalMovement cadence + chase + **pop die/offspring/failedMoves** |
| `apply_animal_pop_map_events` | same | MX for natural/failedMoves death + offspring birth tiles |
| `AnimalWorld::tick_wander_timed_ex` | `animals.rs` | timer/hits decay; legacy no-pop path |
| `AnimalWorld::tick_movement_with_pop` | same | full pop/die/failedMoves + moves/births/deaths |
| `Animal.failed_moves` / `loved_tx` / `loved_ty` / `target` | same | Haxe ObjectHelper fields |
| `AnimalWorld::original_counts` / `capture_original_counts` | same | Haxe originalObjectsCount baseline |
| `resolve_animal_chase` | `animal_move.rs` | Winter/SNOW bone grave; deadly player lock; pack alert index |
| `deadly_chase_gate` | same | season/hits/`animalsDontChase`/`chasingAnimals` |
| `get_closest_player_at` | same | Haxe GetClosestPlayerAt (Euclidean quad-dist) |
| `get_closest_bone_grave` / `is_bone_grave` / `collect_bone_graves_near` | same | GetClosestBoneGrave / IsBoneGrave |
| `is_spawning_in` | same | ObjectData.isSpawningIn (biomes + countsOrGrowsAs) |
| `pick_animal_destination_steered` | same | preferred-biome bias + gotoTarget/gotoLovedBiome best-quad |
| `calculate_non_blocked_target` / `can_animal_end_up_here` | same | path trim + land rules |
| `resolve_pop_on_dest` / `resolve_failed_move` | `animal_pop.rs` | natural die + offspring rolls; failedMoves>20 |
| `chance_for_offspring` / `chance_for_animal_dying` / gates | same | ServerSettings Chance* pure |
| `resolve_animal_path_damage` / escape helpers | `animal_damage.rs` | DoAnimalDamage / TryAnimaEscape (prior chunk) |
| Tests | `animal_pop::*` / `animal_move::*` / `animal_damage::*` / `animals::*` | pop gates, chase, path blocks, failedMoves kill |

## Rust: config hot-reload (`CONFIG-SETTINGS` / server_settings_hot_reload)

| Symbol | File | Role |
|--------|------|------|
| `ServerConfig::live_settings` | `RustServer/crates/ol-config` | runtime-safe knob snapshot |
| `ServerConfig::season_length_secs` | same | Haxe SeasonDuration years × 60 |
| `HotReloadTracker::new` / `poll` / `force_reload` | same | mtime + due-tick re-read of `server.toml` |
| `LiveSettings` | same | live field set (speed/move/season/npc/…) |
| `apply_live_settings` / `enforce_eternal_winter` | `ol-sim/src/settings_live.rs` | apply onto `SimState` |
| `haxe_next_season_duration_years` / `haxe_season_hardness` / `haxe_next_season_length_secs` | same | Haxe DoSeason re-seed pure helpers |
| `reseed_season_length_after_roll` / `is_hard_season` | same | post-roll length from `season_duration_base_secs` |
| `SimState::season_duration_base_secs` | `ol-sim/src/lib.rs` | Haxe SeasonDuration base for next roll |
| `intent_budget_from_live` | `settings_live.rs` | intent drain from live knobs |
| `SimBootLive` | same | boot package for `run_sim_loop_with_views` |
| `NpcConfig::from_live` | `ol-server/src/npc_ai.rs` | LiveSettings → NPC knobs each ~200 ms wake |
| `run_npc_scheduler(live_share, …)` | same | same-wake hot-reload (no 2 s copy task) |
| build wire | `ol-sim/build_settings_live.rs` | lib.rs + main + npc_ai at cargo build |
| Tests | `ol-config` `hot_reload_*` / `settings_live::*` / `npc_config_from_live_*` | tracker + season formula + NPC map |
