# SETTINGS-LONG-TAIL / AgingFactorWhileStarvingToDeath live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (AgingFactorWhileStarvingToDeath promoted; long-tail continues)
- **Haxe:** `ServerSettings.AgingFactorWhileStarvingToDeath = 0.5`; `TimeHelper.updateAge` L716–722 (youth ×factor, adult ×1/factor when `food_store < 0`)
- **Rust:** `LiveSettings.aging_factor_while_starving` → `GameplayKnobs` → `age_step_from_health_live` on `tick_vitals`

## Implemented this fire
1. `aging_factor_while_starving` on ServerConfig / LiveSettings / GameplayKnobs (default 0.5)
2. FIELD_MAP `AgingFactorWhileStarvingToDeath` → Live
3. `tick_vitals` uses Haxe `updateAge` (`trueAge` wall-clock + display age × ageingFactor) instead of linear `AGE_YEARS_PER_SEC`
4. Tests: `age_step_live_starve_factor_override` + `tick_vitals_starving_youth_uses_live_aging_factor`

## Residual
- Next: `GrownUpAge` (still ModuleConst 14 in `age_step_from_health_live` youth/adult branch)
- Birth cross-species aging mult still passed as `1.0` on the tick path
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables

## Verify
```powershell
cargo test -p ol-config --lib -- AgingFactorWhileStarvingToDeath aging_factor_while_starving
cargo test -p ol-sim --lib -- age_step_live_starve_factor_override tick_vitals_starving_youth_uses_live_aging_factor apply_live_settings_gameplay_knobs at_old_age_threshold at_max_age_not_yet_dead age_death_over_max
```
