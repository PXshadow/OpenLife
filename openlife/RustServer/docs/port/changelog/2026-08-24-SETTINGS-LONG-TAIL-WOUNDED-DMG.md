# SETTINGS-LONG-TAIL / TargetWoundedDamageFactor live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (TargetWoundedDamageFactor promoted; long-tail continues)
- **Haxe:** `ServerSettings.TargetWoundedDamageFactor = 0.2`; `GlobalPlayerInstance.DoDamage` L4669 (`targetPlayer.isWounded()`)
- **Rust:** `LiveSettings.target_wounded_damage_factor` → `GameplayKnobs` → `target_wounded_damage_mul` on HIT `org_damage` and animal-path applied damage

## Implemented this fire
1. `target_wounded_damage_factor` on ServerConfig / LiveSettings / GameplayKnobs (default 0.2)
2. FIELD_MAP `TargetWoundedDamageFactor` → Live
3. HIT multiplies org_damage when target `is_wounded_held` (held wound ≠ hidden)
4. Animal path applies the same target mul (`attacker == null` still hits this Haxe line)
5. Tests: `target_wounded_damage_mul_defaults_and_live` + `apply_live_settings_gameplay_knobs`

## Residual
- Next leftover live-path ModuleConst: **MaleDamageFactor** (DoDamage attacker, Haxe L4623; default 1.2)
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables
- HIT still does not multiply live CursedReceive/Make on org_damage (helpers exist)

## Verify
```powershell
cargo test -p ol-config --lib
cargo test -p ol-sim --lib -- target_wounded_damage_mul_defaults_and_live apply_live_settings_gameplay_knobs
```
