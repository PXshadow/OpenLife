# MOSQUITO-MAPCHANCE / BiomeAnimalHitChance live

## Chunk
- **matrix_id:** `MOSQUITO-MAPCHANCE`
- **status:** **DONE** (content patch already landed; live hit-chance now LiveSettings)
- **Haxe:** `ServerSettings.BiomeAnimalHitChance` (0.0); `DoDamage` ~4575–4582; `PatchObjectData` 2156 `mapChance*=0.3` + SWAMP biomes
- **Rust:** `LiveSettings.biome_animal_hit_chance` → `GameplayKnobs` → animal path miss gate; `apply_default_mosquito_map_chance_patches`

## Implemented this fire
1. `biome_animal_hit_chance` on ServerConfig / LiveSettings / GameplayKnobs (default 0.0)
2. FIELD_MAP Live row `BiomeAnimalHitChance`
3. Live `DoAnimalDamage` uses `state.gameplay.biome_animal_hit_chance` (not only the module const)

## Already present
- 2156 mapChance ×0.3 + SWAMP biome + `rebuild_biome_spawn_tables` at content load / OLC1 finish

## Residual
- `displayBiomeAnimal` / “im save here from …” say (Haxe DoDamage ~4578)
- Jungle-escape `BiomeAnimalHitChance` on non-path GPI DoDamage (weapon/player-null already gated on path)

## Verify
```powershell
cargo test -p ol-config --lib -- live_settings
cargo test -p ol-sim --lib -- apply_live_settings_gameplay_knobs biome_animal_hit
cargo test -p ol-content --lib -- mosquito_map_chance
```
