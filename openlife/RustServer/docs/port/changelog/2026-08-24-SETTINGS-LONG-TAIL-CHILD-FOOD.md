# SETTINGS-LONG-TAIL / FoodUseChildFaktor live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (FoodUseChildFaktor promoted; long-tail continues)
- **Haxe:** `ServerSettings.FoodUseChildFaktor = 1`; `TimeHelper.updateFoodAndDoHealing` L866 (`age < GrownUpAge && food_store > 0`)
- **Rust:** `LiveSettings.food_use_child_faktor` → `GameplayKnobs` → `child_food_use_mult` on `tick_vitals` drain

## Implemented this fire
1. `food_use_child_faktor` on ServerConfig / LiveSettings / GameplayKnobs (default 1)
2. FIELD_MAP `FoodUseChildFaktor` → Live
3. Child food drain uses live GrownUpAge + live child factor (default still a no-op)
4. Tests: `child_food_use_mult_haxe_gate` + `tick_vitals_child_food_use_faktor_live` (×2)

## Residual
- Next leftover live-path ModuleConst (DoorIds / AiIgnoredFloorIds tables, or Haxe AIFoodUseFactor* on same foodDecay block)
- Birth cross-species aging mult still passed as `1.0` on the tick path

## Verify
```powershell
cargo test -p ol-config --lib -- FoodUseChildFaktor food_use_child_faktor
cargo test -p ol-sim --lib -- child_food_use_mult_haxe_gate tick_vitals_child_food_use_faktor_live apply_live_settings_gameplay_knobs old_age_increases_food_drain
```
