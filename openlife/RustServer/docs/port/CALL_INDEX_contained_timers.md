## Rust: contained timers (CONTAINED-TIMERS-PERSIST / rearm_after_load)

| Symbol | File | Role |
|--------|------|------|
| `clamp_creation_to_sim_time` | `ol-sim/src/contained_timers_persist.rs` | Haxe `ObjectHelper.ReadFromFile` L268 clamp |
| `timer_from_nested_slot` / `timers_from_helper_for_rearm` | same | NestedHelper → (creation, ttc); fresh seed when missing |
| `uses_from_nested_slot` / `uses_from_helper_for_rearm` | same | NestedHelper.uses_remaining → last-use path |
| `rebuild_contained_timers_from_world` | same | scan helpers → runtime timer map after OLW (first level) |
| `apply_contained_timers_to_slots` / `apply_contained_uses_to_slots` | same | stamp NestedHelper for OLW3 save (map-slice) |
| `rearm_stats` / `ContainedTimerRearmStats` | same | tiles/slots/persisted_ttc counts |
| `arm_contained_timers_for_loaded_world` | `postload_wire.rs` / `lib.rs` (build wire) | fill `WorldMapTimeState.contained_timers` after load |
| `do_time_for_contained` | `world_time.rs` | Haxe `doTimeForObject` pure (already TIME-WORLD) |
| map-slice contained | `world_time.rs` `do_world_map_time_stuff` | **`nested_timers::tick_container_helper_timers`** (first-level + deep NestedHelper); MX on change |
| `tick_nested_helpers_deep` / `tick_container_helper_timers` | `ol-sim/src/nested_timers.rs` | **NESTED-IN-NESTED-TIMERS** recursive NestedHelper timers (Haxe L1150) |
| `NESTED_TIMER_MAX_DEPTH` | same | recursion safety (default 8) |
| build wire | `ol-sim/build_contained_timers.rs` | CT rearm + nested-in-nested; `nested_in_nested_wired` requires call site |
| script | `src/_apply_nested_now.py` | offline wire (also run from build) |
| Tests deep | `nested_timers::*` / `world_time::nested_in_nested_*` / `first_level_*` | deep transform, mid-ttc, overflow refuse L2213, cargo keep, call-site |

Haxe anchors: `TimeHelper.doTimeForObject`, `ObjectHelper.creationTimeInTicks` / `timeToChange` / `numberOfUses` on contained; ReadFromFile clamp; DoWorldMapTimeStuff L1150 nested-in-nested.

Deep: `tick_nested_helpers_deep` + **live map-slice** `nested_timers::tick_container_helper_timers` (**NESTED-IN-NESTED-TIMERS** DONE). Residuals: clothing/held/wounds ObjectHelper timers (out of map rearm scope); TIME-LONG multi-use container decay (Haxe L1479).
