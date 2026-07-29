## Rust: farmer profession (AI-JOB-FARM)

| Symbol | File | Role |
|--------|------|------|
| `has_or_become_profession` / `FarmProfessionRuntime` / `FarmTaskState` | `ol-sim/src/farmer_profession.rs` | sticky last + max caps + task hysteresis |
| `parse_farm_profession_speech` | same | `FARMER!`/`WHEAT!`→BASIC, `CARROT!`/`BERRY!`/`ADVANCED!`/`SOIL!`/`ROW!`/`WATER!` |
| `decide_farm_job` / `do_basic_farming` / `do_carrot_farming` / `do_berry_farming` / `do_advanced_farming_step` | same | job sequences → `FarmAction` |
| `do_plant` / `do_harvest_wheat` / `do_harvest_corn` / `do_watering_on` | same | hysteresis helpers |
| `do_prepare_soil` / `do_prepare_rows` / `do_composting` | same | soil/rows/compost (SoilMaker/RowMaker) |
| `pick_farmer_goal` / `farmer_pipeline_targets` | same | reverse-craft pipeline seek |
| `age_rotated_farm_profession` / `resolve_farm_assigned_job` | same | AssignedJob / age job 0–1 |
| selfplay farmer plan | `ol-server/src/selfplay.rs` | uses `pick_farmer_goal` |

---

## Rust: baker profession (AI-JOB-BAKER / AI-JOB-BAKER-LIVE)

| Symbol | File | Role |
|--------|------|------|
| `has_or_become_baker` / `count_baker_peers_filtered` / `BakerPeerSnapshot` | `ol-sim/src/baker_profession.rs` | sticky last + peer filters (age/wound/home/follow) |
| `parse_baker_profession_speech` / `resolve_baker_assigned_job` | same | `BAKER!` speech + AssignedJob |
| `do_baking` / `decide_baker_job` / `BakeAction` / `BakeCounts` | same | pure doBakingHelper → craft/shortCraft |
| `pre_profession_dough` / `hot_oven_bake` / `knife_bread_stage` / `make_raw_pies` | same | dough, hot oven pies (mutton max=4), knife bread, pie hysteresis |
| `bake_action_short_craft_apply` / `baker_short_craft_limits` | same | shortCraft USE/drop/seek + maxNewActor |
| `fill_bake_counts_from_map*` / `BakeMapObj` (uses/floor) | same | CountCloseObjects pile + IsIgnoredFloor fill |
| `make_seats_and_cleanup_ex` / `consider_drop_near_oven` | same | craft 2828 BOWLFILLER; drop home/oven anchor |
| `pick_oven_parent` / `OvenState` / `is_oven_id` | same | Hot 250 → Burning 249 → Adobe 237 / Wood-filled 247 |
| `pick_baker_goal` / `baker_pipeline_targets` / `bake_action_to_goal` | same | reverse-craft pipeline seek |
| `Profession::Baker` / `BAKER_TARGET_ID` | `ol-sim/src/ai_goals.rs` | thin self-play profession (Cooked Carrot Pie 273) |
| selfplay baker plan | `ol-server/src/selfplay.rs` | uses `pick_baker_goal` when `Profession::Baker` |

---

## Rust: world persist / NestedHelper (NESTED-OLW1)

| Symbol | File | Role |
|--------|------|------|
| `NestedHelper` | `ol-world/src/lib.rs` | recursive contained meta (Haxe ObjectHelper under containedObjects) |
| `ComplexObject.slots` / `rebuild_wire_from_slots` / `synthesize_slots_from_wire` | same | OLW3 slots ↔ wire contained/nested |
| `transform_to_dummy` | same | Haxe ObjectHelper.TransformToDummy |
| `write_world` / `read_world` / `WORLD_FORMAT_VERSION=3` | `ol-world/src/persist.rs` | OLW1 magic; save v3; load v1–v3 |
| `write_nested_helper` / `read_nested_helper` | same | Haxe WriteToFile / ReadFromFile recursive |
| `save_world_file` / `load_world_file` / `rotate_world_backups` | same | disk I/O + `.bak.N` |
| `init_object_helpers_after_read` / `apply_helper_postload` | `ol-world/src/postload_owners.rs` | pure InitObjectHelpersAfterRead (+owned gate, grave no-prune, removeOwner) |
| `apply_init_object_helpers_after_read` | `ol-sim/src/postload_wire.rs` | sim boot: graves + owning + lineage.owns_object |
| `rebuild_player_owning_from_world` / `rebuild_account_graves_from_world` | same | spawn re-scan / account-only graves refresh |
| `account_token_index` / `description_is_orig_grave` / `player_status_for_postload` | same | soul-token map + origGrave + Alive/Deleted/Missing/Keep |
| `LineageNode.owns_object` | `ol-sim/src/social.rs` | Haxe `Lineage.ownsObject` (session; set by postload) |
| `container_put` / `container_take` / `*_nested` | `ol-world/src/lib.rs` | runtime nest; keeps slots parallel when tracked |
| `encode_map_object_string_nested` / `parse_map_object_string` | same | wire `base,c:sub` one level |

## Rust: long-term decay (TIME-LONG)

| Symbol | File | Role |
|--------|------|------|
| `do_world_long_term_time_stuff` | `RustServer/crates/ol-sim/src/long_term.rs` | DoWorldLongTermTimeStuff (tick-wired) |
| `floor_decay_chance` / `object_decay_chance` | same | DecayFloor / DecayObject pure chance |
| `resolve_object_decay_to` / `floor_decay_result` | same | decay products (content + trash 618) |

## Rust: animal move / chase (`TIME-ANIMAL` / `TIME-ANIMAL-CHASE`)

| Symbol | File | Role |
|--------|------|------|
| `tick_animals` / `tick_animals_dt` | `ol-sim/src/lib.rs` | Haxe doAnimalMovement cadence + chase/biome steer + path damage |
| `AnimalWorld::tick_wander_timed_ex` | `animals.rs` | timer/hits decay; pick_dest gets `&mut [Animal]` for pack alert |
| `Animal.loved_tx` / `loved_ty` / `target` | same | Haxe ObjectHelper lovedTx/Ty + target |
| `resolve_animal_chase` | `animal_move.rs` | Winter/SNOW bone grave; deadly player lock; pack alert index |
| `deadly_chase_gate` | same | season/hits/`animalsDontChase`/`chasingAnimals` |
| `get_closest_player_at` | same | Haxe GetClosestPlayerAt (Euclidean quad-dist) |
| `get_closest_bone_grave` / `is_bone_grave` / `collect_bone_graves_near` | same | GetClosestBoneGrave / IsBoneGrave |
| `is_spawning_in` | same | ObjectData.isSpawningIn (biomes + countsOrGrowsAs) |
| `pick_animal_destination_steered` | same | preferred-biome bias + gotoTarget/gotoLovedBiome best-quad |
| `calculate_non_blocked_target` / `can_animal_end_up_here` | same | path trim + land rules |
| `resolve_animal_path_damage` / escape helpers | `animal_damage.rs` | DoAnimalDamage / TryAnimaEscape (prior chunk) |
| Tests | `animal_move::*` / `animal_damage::*` / `animals::*` | chase gates, steer, pack alert, path blocks |
