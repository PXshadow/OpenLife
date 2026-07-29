# Call index slice — WEAPON-ANIMAL-ZERO / animal_wound_zero

Appended from `haxe-port-chunk` (matrix_id WEAPON-ANIMAL-ZERO). Merge into [CALL_INDEX.md](CALL_INDEX.md) when convenient.

## Haxe

| Symbol | File | Role |
|--------|------|------|
| `TimeHelper.DoAnimalDamage` / `DoAnimalDamageHelper` | `server/TimeHelper.hx` | path cells → `GlobalPlayerInstance.doDamage(animal)` |
| `GlobalPlayerInstance.DoDamage` attacker==null | `server/GlobalPlayerInstance.hx` | GetTransition(animal,0) → doWound equip/ground + fromObj.id=newActor TTC |
| `TransitionImporter.GetTransition(animal,0,LA)` | `data/transition/TransitionImporter.hx` | prefer LA then non-LA |
| `takeCoins` early return | `server/GlobalPlayerInstance.hx` | no-op when attacker null |
| Animal retaliate bloody weapon | same L4792–4814 | **commented out** — skip |

## Rust

| Symbol | File | Role |
|--------|------|------|
| `resolve_animal_zero_transition` / `plan_animal_zero_wound_from_content` | `ol-sim/src/weapon_wound.rs` | pure animal+0 plan |
| `plan_animal_zero_residual` / `animal_zero_cooldown_factor` | same | newActor TTC (`-1`×WeaponCoolDownFactor*) |
| `force_no_coins_on_equip` | same | takeCoins always false on animal path |
| `apply_animal_path_damages` | `ol-sim/src/lib.rs` | after wander: damage + animal zero wire |
| `apply_animal_zero_wound_hit` / residual plan | same | equip/ground + map transform + MX |
| `Animal.map_object_id` / `apply_zero_residual` | `ol-sim/src/animals.rs` | attacking form id + move_timer |
| `AnimalDeathEvent.object_id` | same | pop map clear uses live form |
| Tests | `weapon_wound::animal_zero_*`, `animals::animal_zero_*` | boar/snake/wolf + residual transform |

## Residuals (out of core)

- BiomeAnimalHitChance / isAnimalNotDeadlyForMe miss path
- Rattle Snake 764 not in AnimalKind (pure shoes factor only)
- Ground ComplexObject ttc=2 stamp, mosquito fever, bleed DPS, wallet takeCoins
