## Rust: farmer profession (AI-JOB-FARM / AI-JOB-FARM-WIRE / AI-JOB-FARM-LIVE)

| Symbol | File | Role |
|--------|------|------|
| `has_or_become_profession` / `FarmProfessionRuntime` / `FarmTaskState` | `ol-sim/src/farmer_profession.rs` | sticky last + max caps + task hysteresis |
| `Player.farm_profession` / `Player.farm_task` | `ol-sim/src/player.rs` | sticky runtime + task hysteresis across ticks (AI-JOB-FARM-LIVE) |
| `parse_farm_profession_speech` / `assign_farm_from_speech` | `farmer_profession.rs` | `FARMER!`/`WHEAT!`→BASIC, `CARROT!`/`BERRY!`/`ROW!`/`SOIL!`/`WATER!` → assigned+last |
| `keep_bushes_alive` / `keep_bushes_alive_count` / `KEEP_BUSHES_ALIVE_*` | same | Haxe keepBushesAlive: living bushes &lt;20 → ShortCraft(1137,389) |
| `do_critical_farm_slice` | same | age-gated bushes + basic + carrot (doCriticalStuff farm slice) |
| `short_craft_apply` / `ShortCraftApply` / `ShortCraftInput` / `farm_action_short_craft_apply` | same | pure shortCraft edges: USE/drop/seek, snow/ocean, weak skewer, carrot row, maxNewActor |
| `decide_farm_job` / `do_basic_farming` / `do_carrot_farming` / `do_berry_farming` / `do_advanced_farming_step` | same | job sequences → `FarmAction` |
| `do_plant` / `do_harvest_wheat` / `do_harvest_corn` / `do_watering_on` | same | hysteresis helpers |
| `do_prepare_soil` / `do_prepare_rows` / `do_composting` | same | soil/rows/compost; rows call keep_bushes_alive when dying present |
| `pick_farmer_goal` / `farmer_pipeline_targets` | same | reverse-craft pipeline seek |
| `age_rotated_farm_profession` / `resolve_farm_assigned_job` | same | AssignedJob / age job 0–1 |
| `fill_farm_counts_from_map` / `fill_farm_counts_from_map_with_floor` / `FarmMapObj` | `farmer_profession` + `farm_spatial_inc.rs` | bulk home-radius snapshot (exclusive square + optional IsIgnoredFloor) |
| `count_close_objects_at` / `count_close_objects_ex` / `count_close_objects_with_piles` / `CountCloseOpts` | same | Haxe CountCloseObjects: parent +1, pile +uses, specials 233/300 |
| `in_count_close_square` / `is_ignored_floor` / `count_close_pile_specials` / `pile_obj_id_from_table` | same | half-open square; IsIgnoredFloor; pile resolve |
| `count_corn_seeds_near` / `farm_radius_table` / `soil_units_from_map` | same | countCorn r=20 (held only 1115/1120/1247); radii; soil units |
| `try_decide_farm_from_rung` / `farm_action_to_goal` / `farm_job_rung_label` | same | ladder bridge + goal map |
| `farm_goal_from_map_and_rung` / `farm_goal_from_counts_and_rung` | same | fill→decide→goal for live tick / ladder consumers |
| `AI_IGNORED_FLOOR_IDS` / `WET_CLAY_BOWL_ID` / `BIG_CHARCOAL_PILE_ID` / `HUGE_CHARCOAL_PILE_ID` | same | Haxe floor ignore + CountClose specials |
| selfplay farmer plan | `ol-server/src/selfplay.rs` | uses `pick_farmer_goal` (not map-fill yet) |
| build wire | `ol-sim/build_ai_job_farm_wire.rs` | splice include + lib export at cargo build |
