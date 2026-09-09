# SETTINGS-LONG-TAIL / GrownUpAge live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (GrownUpAge promoted; long-tail continues)
- **Haxe:** `ServerSettings.GrownUpAge = 14`; `TimeHelper.updateAge` L700/717 youth vs adult
- **Rust:** `LiveSettings.grown_up_age` → `GameplayKnobs` → `age_step_from_health_live` on `tick_vitals`

## Implemented this fire
1. `grown_up_age` on ServerConfig / LiveSettings / GameplayKnobs (default 14)
2. FIELD_MAP `GrownUpAge` → Live
3. `age_step_from_health_live` takes live GrownUpAge for health-factor adult branch + starve youth/adult split
4. Tests: `age_step_live_grown_up_age_override` + `tick_vitals_starving_uses_live_grown_up_age`

## Residual
- Next: `FoodUseChildFaktor` (Haxe child foodDecay when `age < GrownUpAge && food > 0`; default 1 = no-op)
- Birth cross-species aging mult still passed as `1.0` on the tick path
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables

## Verify
```powershell
cargo test -p ol-config --lib -- GrownUpAge grown_up_age
cargo test -p ol-sim --lib -- age_step_live_grown_up_age_override tick_vitals_starving_uses_live_grown_up_age apply_live_settings_gameplay_knobs
```
