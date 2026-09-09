# SETTINGS-LONG-TAIL / EveFoodUseFactor live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (EveFoodUseFactor promoted; long-tail continues)
- **Haxe:** `ServerSettings.EveFoodUseFactor = 1`; `TimeHelper.updateFoodAndDoHealing` L976 (`isEveOrAdam() && !isWounded()`)
- **Rust:** `LiveSettings.eve_food_use_factor` → `GameplayKnobs` → `eve_food_use_mult` on `tick_vitals` drain

## Implemented this fire
1. `eve_food_use_factor` on ServerConfig / LiveSettings / GameplayKnobs (default 1)
2. FIELD_MAP `EveFoodUseFactor` → Live
3. Unwounded first-name EVE/ADAM food drain uses live knob (after bleed/hazard extras, Haxe order)
4. Tests: `eve_food_use_mult_haxe_gate` + `tick_vitals_eve_food_use_faktor_live` + `tick_vitals_non_eve_skips_eve_food_use_faktor`

## Residual
- Next leftover live-path ModuleConst: **AgingFactorHumanBornToAi** / **AgingFactorAiBornToHuman** (`tick_vitals` still passes `1.0` as birth cross-species aging mult)
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables
- EveDamageFactor still unused on the combat path

## Verify
```powershell
cargo test -p ol-config --lib
cargo test -p ol-sim --lib -- eve_food_use_mult_haxe_gate tick_vitals_eve_food_use_faktor_live tick_vitals_non_eve_skips_eve_food_use_faktor apply_live_settings_gameplay_knobs
```
