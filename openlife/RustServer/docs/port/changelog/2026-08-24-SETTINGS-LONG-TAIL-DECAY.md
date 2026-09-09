# SETTINGS-LONG-TAIL / ObjDecayChance + FloorDecayChance live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (another ModuleConst pair promoted; long-tail continues)
- **Haxe:** `ServerSettings.ObjDecayChance = 0.00005`; `FloorDecayChance = 0.00001`; `TimeHelper.DecayFloor` / `DecayObject`
- **Rust:** `LiveSettings.obj_decay_chance` / `floor_decay_chance` → `GameplayKnobs` → `DecayChanceKnobs` / `*_ex`

## Implemented this fire
1. `obj_decay_chance` + `floor_decay_chance` on ServerConfig / LiveSettings / GameplayKnobs
2. FIELD_MAP `ObjDecayChance` / `FloorDecayChance` ModuleConst → Live
3. `floor_decay_chance_ex` / `object_decay_chance_ex` + `do_world_long_term_time_stuff_ex`

## Residual
- TIME-LONG tick wire: `do_world_long_term_time_stuff` is not called from `tick_vitals` (`SimState` has no `LongTermState`)
- `CursedGraveTime` still ModuleConst
- Other decay *Factor tables still ModuleConst (`AnimalDecayFactor`, wall/food/clothing/permanent)

## Verify
```powershell
cargo test -p ol-config --lib -- ObjDecay FloorDecay live_settings field_map
cargo test -p ol-sim --lib -- floor_decay_chance_ex_live_override object_decay_uses_decay_factor apply_live_settings_gameplay_knobs
```
