## COMBAT-MOSQUITO-KIND / mosquito_animal — **DONE** (core)

| Symbol | Location | Notes |
|--------|----------|-------|
| Haxe `Biome.getBiomeAnimals(JUNGLE)=[2156]` | `server/Biome.hx` | jungle animal id |
| Haxe `doesRealDamage` / `moskitoDamageFactor` / fever | `GlobalPlayerInstance.DoDamage` | parent≠2156; jungle love |
| Haxe `DoAnimalDamage` path | `TimeHelper.hx` | deadlyDistance+damage (mosquito still bites) |
| Haxe `isAnimal` excludes 2156 | `ObjectData.hx` | not isDeadlyAnimal → no chase |
| Rust `AnimalKind::Mosquito` | `ol-sim/animals.rs` | object_id 2156; snapshot.mosquito; `is_deadly_animal=false` |
| Rust combat profile damage=1 | `ol-sim/animal_damage.rs` | path deadly; not AI deadly |
| Rust pure jungle love + moskito scale | `ol-sim/hunt.rs` | `jungle_biome_love_for_mosquito` / `moskito_damage_factor_from_love` / `scale_damage_by_moskito_factor` |
| Live fever loves_jungle | `lib.rs` `player_jungle_biome_love` + `apply_mosquito_fever_candidate` | person color + living parents + floor |
| Live path moskito scale | `lib.rs` `apply_animal_path_damages` | `!does_real_damage` → scale by moskito factor |
| Live chase / USE escape | `lib.rs` | `kind.is_deadly_animal()` (mosquito excluded) |
| Spawn seeds | `lib.rs` `spawn_default_animals` | +2 `AnimalKind::Mosquito` |
| Re-exports | `lib.rs` `pub use hunt::{jungle_biome_love_…}` | crate-public pure helpers |

**Residual (deferred):** content `mapChance*=0.3` + SWAMP biomes push for 2156; `BiomeAnimalHitChance` / `isAnimalNotDeadlyForMe` jungle-escape for 2156; Haxe yellowFever emote=7 PE index (FEVER-EMOTE residual).
