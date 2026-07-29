# NESTED-IN-NESTED-TIMERS / deep_contained (2026-07-26)

## Haxe

- `TimeHelper.DoWorldMapTimeStuff` L1149–1168: first-level `doTimeForObject` on `containedObjects`
- L1150 TODO: `// TODO time in contained objects in contained objects`
- `doTimeForObject` L2191–2232; overflow refuse L2213 when cargo > new `numSlots`

## Rust

| Piece | Path |
|-------|------|
| Pure deep tick | `crates/ol-sim/src/nested_timers.rs` |
| First-level rearm (unchanged contract) | `contained_timers_persist.rs` |
| Map-slice wire | `world_time.rs` → `nested_timers::tick_container_helper_timers` |
| Build wire | `build_contained_timers.rs` + `src/_apply_nested_now.py` |

## Behavior

1. First-level contained timers still use runtime `WorldMapTimeState.contained_timers` + OLW3 slot times.
2. Depth ≥ 2: times live on `NestedHelper.creation_time` / `time_to_change`; recursive walk via `tick_nested_helpers_deep`.
3. Transform keeps cargo when `new.num_slots >= cargo.len()`; else refuse (Haxe L2213).
4. Deep transform/prune sets `changed` → MX `MapTimeChange`.
5. Wire nest rebuilt via `ComplexObject::rebuild_wire_from_slots`.

## Tests

- `nested_timers::tick_nested_helpers_deep_*`
- `nested_timers::tick_container_helper_*`
- `nested_timers::nested_in_nested_*`
- `contained_timers_persist::nested_in_nested_times_stay_on_slots_not_runtime_map`
- `world_time::nested_in_nested_timer_transform_in_map_slice` (deep MX)
- `world_time::nested_in_nested_mid_ttc_survives_map_slice`
- `world_time::first_level_transform_preserves_deep_cargo`
- `world_time::first_level_overflow_refuse_keeps_parent_and_cargo` (L2213)
- `world_time::rearm_then_deep_tick_without_runtime_map_for_depth2`
- `world_time::nested_in_nested_wired_call_site_present`

## Apply wire

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
python crates/ol-sim/src/_apply_nested_now.py
python docs/port/_apply_nested_docs.py
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- nested
```

(Build script also runs nested wire via `patch_contained_timers` → `patch_nested_in_nested`.)

## Residual

- Clothing / held / wound ObjectHelper timers (body NestedHelper; not map-slice)
- Long-term multi-use decay Haxe TODOs (separate from deep map-slice auto-decay)
