# SETTINGS-LONG-TAIL / CombatExhaustionCostPerAttack HIT + ExhaustionYellowFeverPerSec live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (HIT exhaustion + yellow-fever drain live; long-tail continues)
- **Haxe:** `CombatExhaustionCostPerAttack = 0.1`; `ExhaustionYellowFeverPerSec = 0.1`
- **Rust:** LiveSettings → GameplayKnobs → HIT/KILL attacker exhaustion + `yellow_fever_food_drain_ex` on tick_vitals

## Implemented this fire
1. Skipped **CombatAngryTimeMinimum** (still no TimeHelper angry recovery tick)
2. Wired existing **CombatExhaustionCostPerAttack** on live HIT and SAY KILL (Haxe `kill()`)
3. **ExhaustionYellowFeverPerSec** on live yellow-fever food drain (+ existing heat delta)
4. FIELD_MAP Live + server.toml + apply_live_settings

## Residual
- Next used live-path ModuleConst (HIT / food leftovers)
- **CombatAngryTimeMinimum** still needs TimeHelper angry recovery tick
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables

## Verify
```powershell
cargo test -p ol-config --lib -- field_map_critical_live_count live_critical_includes_gameplay_batch force_reload_reports_all_live_keys
cargo test -p ol-sim --lib -- apply_live_settings_gameplay_knobs yellow_fever_food tick_vitals_yellow_fever old_age_increases_food_drain
```
