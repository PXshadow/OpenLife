# SETTINGS-LONG-TAIL / alt-outcome + animal-move chances live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (USE alt-outcome + animal-move chances promoted; long-tail continues)
- **Haxe:** `AlternativeOutcomePercentIncreasePerHit=10`, `AlternativeOutcomeHitsDecreaseOnSucess=5`, `ChanceThatAnimalsCanPassBlockingBiome=0.03`, `chancePreferredBiome=0.8`
- **Rust:** LiveSettings → GameplayKnobs → USE `evaluate_alternative_outcome` + `tick_animals_dt` `pick_animal_destination_ex`

## Implemented this fire
1. Four knobs on ServerConfig / LiveSettings / GameplayKnobs
2. FIELD_MAP Live (Haxe name `chancePreferredBiome` kept)
3. Live USE alt-outcome reads gameplay instead of ModuleConst
4. Animal wander uses `AnimalMoveChanceKnobs` on pass-blocking + preferred-biome
5. Tests: `apply_live_settings_gameplay_knobs` + existing alt_outcome / chance_preferred_biome

## Residual
- **CombatAngryTimeMinimum** still needs TimeHelper angry recovery tick
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables

## Verify
```powershell
cargo test -p ol-config --lib
cargo test -p ol-sim --lib -- apply_live_settings_gameplay_knobs alt_outcome chance_preferred_biome
```
