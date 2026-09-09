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
| Pure biome animals / deadly-for-me | `ol-sim/animal_damage.rs` | `biome_animals_for_loved_biome` / `is_animal_*_deadly_for_me` |
| Live BiomeAnimalHitChance gate | `lib.rs` `apply_animal_path_damages` | miss before path damage roll (default chance 0) |
| Content mapChance / SWAMP | `ol-content` `apply_default_mosquito_map_chance_patches` | 2156 `*=0.3` + SWAMP + `rebuild_biome_spawn_tables` |

**MOSQUITO-MAPCHANCE DONE.** Residual: live fever NestedHelper / yellowFever emote=7 PE (FEVER-EMOTE); animal-zero wound equip path still thin vs full WEAPON-ANIMAL-ZERO.
