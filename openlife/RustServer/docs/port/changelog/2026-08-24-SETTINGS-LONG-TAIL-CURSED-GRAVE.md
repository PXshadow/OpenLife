# SETTINGS-LONG-TAIL / CursedGraveTime live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (CursedGraveTime promoted; long-tail continues)
- **Haxe:** `ServerSettings.CursedGraveTime = 12`; `TimeHelper.doTimeTransitionHelper` L2123–2137
- **Rust:** `LiveSettings.cursed_grave_time` → `GameplayKnobs` → `container_overflow_delay_ex` → `tick_auto_decays`

## Implemented this fire
1. `cursed_grave_time` on ServerConfig / LiveSettings / GameplayKnobs (default 12 hours)
2. FIELD_MAP `CursedGraveTime` Live
3. `cursed_grave_sharp_stone_extra_secs` / `container_overflow_delay_ex` / `place_popped_contained_near`
4. Live `tick_auto_decays`: if cargo would overflow new target slots, delay (+20s, +hours×3600 if sharp stone) and pop to a neighbor instead of transforming

## Residual
- `CreateScoreEntryForCursedGrave` not called from overflow pop
- TIME-WORLD `do_world_map_time_stuff` still not on `tick_vitals`
- Next ModuleConst: `AnimalDecayFactor` (content-load patch table)

## Verify
```powershell
cargo test -p ol-config --lib -- CursedGrave cursed_grave_time field_map
cargo test -p ol-sim --lib -- container_overflow_sharp_stone_extends auto_decay_overflow_pops_and_uses_live_cursed_grave_time apply_live_settings_gameplay_knobs
```
