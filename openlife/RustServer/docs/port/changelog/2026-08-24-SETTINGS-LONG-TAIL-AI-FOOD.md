# SETTINGS-LONG-TAIL / AIFoodUseFactor* live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (AIFoodUseFactorSerf/Commoner/Noble promoted; long-tail continues)
- **Haxe:** `ServerSettings.AIFoodUseFactorSerf/Commoner/Noble = 0.8/0.9/1`; `TimeHelper.updateFoodAndDoHealing` L868–872 (`player.isAi()` then class if/else)
- **Rust:** `LiveSettings.ai_food_use_factor_*` → `GameplayKnobs` → `ai_class_food_use_mult` on `tick_vitals` drain

## Implemented this fire
1. `ai_food_use_factor_serf/commoner/noble` on ServerConfig / LiveSettings / GameplayKnobs (defaults 0.8/0.9/1)
2. FIELD_MAP `AIFoodUseFactor*` → Live
3. AI food drain uses live class knobs; King/Emperor/NotSet stay `1.0` (Haxe if/else, unlike AI speed)
4. Tests: `ai_class_food_use_mult_haxe_gate` + `tick_vitals_ai_food_use_faktor_live` + `tick_vitals_human_skips_ai_food_use_faktor`

## Residual
- Next leftover live-path ModuleConst: **EveFoodUseFactor** (same foodDecay block, Haxe L976; default 1)
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables
- Birth cross-species aging mult still passed as `1.0` on the tick path (`AgingFactorHumanBornToAi` / `AgingFactorAiBornToHuman`)

## Verify
```powershell
cargo test -p ol-config --lib -- AIFoodUseFactor ai_food_use_factor
cargo test -p ol-sim --lib -- ai_class_food_use_mult_haxe_gate tick_vitals_ai_food_use_faktor_live tick_vitals_human_skips_ai_food_use_faktor apply_live_settings_gameplay_knobs
```
