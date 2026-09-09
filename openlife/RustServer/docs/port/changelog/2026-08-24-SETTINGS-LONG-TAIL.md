# SETTINGS-LONG-TAIL / StartingEveAge live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (one ModuleConst promoted; long-tail continues)
- **Haxe:** `ServerSettings.StartingEveAge = 14`; `GlobalPlayerInstance.spawnAsEve` `age` / `trueAge`
- **Rust:** `LiveSettings.starting_eve_age` → `GameplayKnobs` → `spawn_player` / revive

## Implemented this fire
1. `starting_eve_age` on ServerConfig / LiveSettings / GameplayKnobs (default 14)
2. FIELD_MAP `StartingEveAge` ModuleConst → Live
3. Live Eve spawn + deleted-player revive set `age` and `true_age` from the knob (hot-reloadable)

## Residual
- `ObjDecayChance` / `FloorDecayChance` still ModuleConst (`long_term` chance math)
- `CursedGraveTime` still ModuleConst (`world_time` sharp-stone extra secs)
- Other ~16 critical ModuleConst + ~170 Haxe long-tail
- `Player::new` still defaults 14.0 until spawn overwrites (Haxe field initializer)

## Verify
```powershell
cargo test -p ol-config --lib -- StartingEveAge starting_eve_age live_settings
cargo test -p ol-sim --lib -- spawn_player_uses_live_starting_eve_age apply_live_settings_gameplay_knobs
```
