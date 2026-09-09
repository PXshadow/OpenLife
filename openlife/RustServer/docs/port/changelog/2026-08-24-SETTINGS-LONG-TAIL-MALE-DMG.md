# SETTINGS-LONG-TAIL / MaleDamageFactor live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (MaleDamageFactor promoted; long-tail continues)
- **Haxe:** `ServerSettings.MaleDamageFactor = 1.2`; `GlobalPlayerInstance.DoDamage` L4623 (`attacker.isMale()`, inside `attacker != null`)
- **Rust:** `LiveSettings.male_damage_factor` → `GameplayKnobs` → `male_damage_mul` on HIT `org_damage` when `!player_is_female`

## Implemented this fire
1. `male_damage_factor` on ServerConfig / LiveSettings / GameplayKnobs (default 1.2)
2. FIELD_MAP `MaleDamageFactor` → Live
3. HIT multiplies org_damage when attacker is male (Haxe `ObjectData.male` / `player_is_female`)
4. Animal path skipped (`attacker == null` in Haxe)
5. Tests: `male_damage_mul_defaults_and_live` + `apply_live_settings_gameplay_knobs`

## Residual
- Next leftover live-path ModuleConst: **AnimalDamageFactor** (and winter/attacked siblings on the same animal DoDamage path)
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables (no TOML unless a live path needs override)
- HIT still does not multiply live CursedReceive/Make on org_damage (helpers exist)

## Verify
```powershell
cargo test -p ol-config --lib
cargo test -p ol-sim --lib -- male_damage_mul_defaults_and_live apply_live_settings_gameplay_knobs
```
