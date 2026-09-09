# SETTINGS-LONG-TAIL / CombatAngryTimeBeforeAttack live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (CombatAngryTimeBeforeAttack promoted; long-tail continues)
- **Haxe:** `ServerSettings.CombatAngryTimeBeforeAttack = 5`; GPI spawn `angryTime`; `TimeHelper.UpdateEmotes` L662 soft combat
- **Rust:** `LiveSettings.combat_angry_time_before_attack` → `GameplayKnobs` → spawn/revive/child `angry_time` + `combat_angry_before_attack_ex` on fever_pe

## Implemented this fire
1. `combat_angry_time_before_attack` on ServerConfig / LiveSettings / GameplayKnobs (default 5)
2. FIELD_MAP `CombatAngryTimeBeforeAttack` → Live
3. `spawn_player` / revive / `spawn_child` set `angry_time` from live knobs
4. fever_pe soft combat uses `combat_angry_before_attack_ex`
5. Tests: `spawn_player_uses_live_combat_angry_time_before_attack` + `soft_combat_uses_live_combat_angry_before_attack` + `apply_live_settings_gameplay_knobs`

## Residual
- Next leftover: **CombatAngryTimeMinimum** only if TimeHelper angry recovery is ported (HIT subtracts damage; no tick clamp yet). DoorIds / AiIgnoredFloorIds remain ModuleConst tables.

## Verify
```powershell
cargo test -p ol-config --lib
cargo test -p ol-sim --lib -- spawn_player_uses_live_combat_angry_time_before_attack soft_combat_uses_live_combat_angry_before_attack apply_live_settings_gameplay_knobs
```
