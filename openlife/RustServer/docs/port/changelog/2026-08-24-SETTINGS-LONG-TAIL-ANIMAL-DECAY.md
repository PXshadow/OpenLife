# SETTINGS-LONG-TAIL / AnimalDecayFactor live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (AnimalDecayFactor promoted; long-tail continues)
- **Haxe:** `ServerSettings.AnimalDecayFactor = 0.05`; `PatchObjectData` horse-cart / domestic / wolf `decayFactor`
- **Rust:** `LiveSettings.animal_decay_factor` → `GameplayKnobs` → `DecayChanceKnobs` / `decay_factor_for_object`

## Implemented this fire
1. `animal_decay_factor` on ServerConfig / LiveSettings / GameplayKnobs (default 0.05)
2. FIELD_MAP `AnimalDecayFactor` ModuleConst → Live
3. `apply_animal_decay_factor_patches` (content load still uses 0.05)
4. Long-term `try_decay_object` uses live factor for patched animal ids

## Residual
- Content `ObjectDef.decay_factor` stays 0.05 until reload (no Arc clone re-patch on hot-reload)
- TIME-LONG still not on `tick_vitals`
- Next ModuleConst: `ObjDecayFactorForPermanentObjs`

## Verify
```powershell
cargo test -p ol-config --lib -- AnimalDecay animal_decay_factor field_map
cargo test -p ol-content --lib -- decay_patches_apply_to_existing_objects
cargo test -p ol-sim --lib -- decay_factor_for_object_uses_live_animal_knob apply_live_settings_gameplay_knobs
```
