# SETTINGS-LONG-TAIL / WeaponDamageFactor live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (WeaponDamageFactor promoted; long-tail continues)
- **Haxe:** `ServerSettings.WeaponDamageFactor = 1`; `GlobalPlayerInstance.DoDamage` L4590 (`attacker != null`)
- **Rust:** `LiveSettings.weapon_damage_factor` → `GameplayKnobs` → `weapon_damage_mul` on HIT `org_damage`

## Implemented this fire
1. `weapon_damage_factor` on ServerConfig / LiveSettings / GameplayKnobs (default 1)
2. FIELD_MAP `WeaponDamageFactor` → Live
3. HIT multiplies org_damage by live WeaponDamageFactor before male/eve/wounded muls
4. Animal path skipped (`attacker == null` uses AnimalDamageFactor*)
5. Tests: `weapon_damage_mul_defaults_and_live` + `apply_live_settings_gameplay_knobs`

## Residual
- Next leftover: **CursedReceive/Make HIT wire** (Live knobs + helpers exist; HIT does not multiply yet)
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables

## Verify
```powershell
cargo test -p ol-config --lib
cargo test -p ol-sim --lib -- weapon_damage_mul_defaults_and_live apply_live_settings_gameplay_knobs
```
