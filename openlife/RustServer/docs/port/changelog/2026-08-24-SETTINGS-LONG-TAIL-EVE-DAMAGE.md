# SETTINGS-LONG-TAIL / EveDamageFactor live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (EveDamageFactor promoted; long-tail continues)
- **Haxe:** `ServerSettings.EveDamageFactor = 1`; `GlobalPlayerInstance.DoDamage` L4630 (attacker) + L4667 (target)
- **Rust:** `LiveSettings.eve_damage_factor` → `GameplayKnobs` → `eve_pair_damage_mul` on HIT `org_damage`; `eve_damage_mul` on animal-path target

## Implemented this fire
1. `eve_damage_factor` on ServerConfig / LiveSettings / GameplayKnobs (default 1)
2. FIELD_MAP `EveDamageFactor` → Live
3. HIT multiplies org_damage by attacker Eve × target Eve (Haxe both sites)
4. Animal path (`attacker == null`) still multiplies the target
5. Tests: `eve_damage_mul_defaults_and_live` + `apply_live_settings_gameplay_knobs`

## Residual
- Next leftover live-path ModuleConst: **TargetWoundedDamageFactor** (same DoDamage block, Haxe L4669; default 0.2)
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables
- HIT still does not multiply live CursedReceive/Make on org_damage (helpers exist)

## Verify
```powershell
cargo test -p ol-config --lib
cargo test -p ol-sim --lib -- eve_damage_mul_defaults_and_live apply_live_settings_gameplay_knobs
```
