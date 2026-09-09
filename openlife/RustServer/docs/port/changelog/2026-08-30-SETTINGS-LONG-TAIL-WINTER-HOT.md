# SETTINGS-LONG-TAIL / WinterWildFoodDecayChance + HotSeasonTemperatureFactor live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (two live leftovers promoted; long-tail continues)
- **Haxe:** `WinterWildFoodDecayChance = 1.5`; `HotSeasonTemperatureFactor = 0.75`
- **Rust:** LiveSettings → GameplayKnobs → map-time `seasonal_chances_ex` / `apply_season_temperature_factors_ex`

## Implemented this fire
1. **WinterWildFoodDecayChance** on live `seasonal_chances_ex` / `do_world_map_time_stuff_ex` (compiled fallback 1.5)
2. **HotSeasonTemperatureFactor** on live `apply_season_temperature_factors_ex` / `initialize_tile_temperature_ex` / `update_tile_temperature_lerp_ex` (compiled fallback 0.75)
3. FIELD_MAP Live + server.toml + apply_live_settings
4. Cold season still compiled (`ColdSeasonTemperatureFactor`)

## Residual
- **ColdSeasonTemperatureFactor** still ModuleConst on `apply_season_temperature_factors` negative impact
- Skip **GrowNewPlantsFromExistingFactor** until a live offspring-from-plant path
- Skip **MaxPlayersBeforeStartingAsChild** until spawnAsEve pairing is live

## Verify
```powershell
cargo test -p ol-config --lib -- winter_wild -- --test-threads=1
cargo test -p ol-sim --lib -- seasonal_chances -- --test-threads=1
cargo test -p ol-sim --lib -- settings_live -- --test-threads=1
```
