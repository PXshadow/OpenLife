# SETTINGS-LONG-TAIL / CursedReceive + CursedMake HIT wire

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (HIT now reads live cursed damage knobs; long-tail continues)
- **Haxe:** `DoDamage` L4628–4629 (`attacker != null`): target `isCursed` × CursedReceiveDamageFactor (1.2); attacker `isCursed` × CursedMakeDamageFactor (0.5)
- **Rust:** `GameplayKnobs.cursed_*_damage_factor` → `cursed_receive_damage_mul` / `cursed_make_damage_mul` on HIT `org_damage`

## Implemented this fire
1. HIT captures killer/target `is_cursed`
2. Multiplies org_damage after MaleDamageFactor, before EveDamageFactor (Haxe order)
3. Animal path skipped (`attacker == null`)
4. Tests: existing `cursed_damage_mul_defaults_and_live` + HIT compile path

## Residual
- Next leftover live-path ModuleConst: **GraveBlockingDistance** / **MaxPlayersBeforeActivatingGraveCurse** (same-path siblings)
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables

## Verify
```powershell
cargo test -p ol-sim --lib -- cursed_damage_mul_defaults_and_live say_hit_wounds_then_kills_kill_one_shot
```
