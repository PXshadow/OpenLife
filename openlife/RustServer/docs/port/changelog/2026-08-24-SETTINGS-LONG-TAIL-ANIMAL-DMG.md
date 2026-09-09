# SETTINGS-LONG-TAIL / AnimalDamageFactor live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (AnimalDamageFactor + winter/attacked siblings promoted; long-tail continues)
- **Haxe:** `ServerSettings.AnimalDamageFactor = 1.5` / `InWinter = 2` / `IfAttacked = 1.5`; `GlobalPlayerInstance.DoDamage` L4590–4593 (`attacker == null`)
- **Rust:** `LiveSettings.animal_damage_factor*` → `GameplayKnobs` → `org_animal_damage_ex` on animal path DoDamage

## Implemented this fire
1. `animal_damage_factor` / `_in_winter` / `_if_attacked` on ServerConfig / LiveSettings / GameplayKnobs (defaults 1.5 / 2 / 1.5)
2. FIELD_MAP AnimalDamageFactor* → Live
3. Live animal path uses `resolve_animal_path_damage_ex` + `animal_damage_factor_knobs()`
4. Tests: `org_damage_factors_live_override` + `apply_live_settings_gameplay_knobs`

## Residual
- Next leftover live-path ModuleConst: **WeaponDamageFactor** (same DoDamage line, attacker != null; default 1)
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables
- HIT still does not multiply live CursedReceive/Make on org_damage (helpers exist)

## Verify
```powershell
cargo test -p ol-config --lib
cargo test -p ol-sim --lib -- org_damage_factors_live_override apply_live_settings_gameplay_knobs
```
