# SETTINGS-LONG-TAIL / ColdSeasonTemperatureFactor live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **DONE** for ColdSeason (HotSeason already live)
- **Haxe:** `ColdSeasonTemperatureFactor = 0.75` (`ServerSettings.hx`; GPI / TemperatureHandler / TimeHelper negative `seasonImpact`)
- **Rust:** LiveSettings → GameplayKnobs → `apply_season_temperature_factors_ex` + player tile init

## Implemented
1. `cold_season_temperature_factor` on ServerConfig / LiveSettings / GameplayKnobs (Haxe default 0.75)
2. FIELD_MAP Live + `server.toml`
3. `apply_season_temperature_factors_ex(impact, hot, cold)` — negative impact uses live cold
4. Player vitals path: `update_player_temperature_ex` / `ensure_tile_temperature_ex`
5. Map-time `do_world_map_time_stuff_ex` / lerp / initialize `_ex`

## Residual
- `GrowNewPlantsFromExistingFactor` until offspring-from-plant path
- `MaxPlayersBeforeStartingAsChild` until spawnAsEve pairing
- Next port: [`PRIORITY.md`](../PRIORITY.md) **CRAFT-LIVE-IO** `useHeldObjOnTarget`

## Verify
```powershell
cargo test -p ol-config --lib -- winter_wild -- --test-threads=1
cargo test -p ol-sim --lib -- apply_season_temperature_factors_ex -- --test-threads=1
cargo test -p ol-sim --lib -- ensure_tile_temperature_ex_live_cold -- --test-threads=1
cargo test -p ol-sim --lib -- settings_live -- --test-threads=1
```
