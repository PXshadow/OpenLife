# 2026-09-02 clothing rValue insulation / heat-protection matrix

**status:** **DONE**

## Closed

- Haxe `ObjectData.getInsulation` / `getHeatProtection` (`parts` h/t/b/p=0.4, s=0.2; backpack/`p` heat-protection 0).
- `calculateClothingInsulation` / `HeatProtection` summed over all 6 worn slots.
- Ambient: `+ insulation * 0.5 * TemperatureClothingFactor` (skip in water); heat-protection cannot pull below 0.5.
- `clothingFactor`: insulation when ambient < 0.5, else heat-protection + `TemperatureNaturalHeatInsulation` (0.5).
- Live `tick_vitals` uses `clothing_parent_ids()` + content rValue (not hat/chest/shoes presence stub).
- Compiled Haxe defaults: ClothingFactor 0.1, InsulationFactor 5, NaturalHeatInsulation 0.5 (not LiveSettings this fire).

## Tests

- `clothing_get_insulation_and_heat_protection`
- `clothing_ambient_warms_and_skips_water` / `clothing_body_factor_insulation_vs_heat_protection`
- `clothing_sums_use_rvalue_matrix_and_backpack_no_heat_protection`
- `wool_hat_slows_snow_cooling`
- `tick_vitals_clothing_rvalue_slows_snow_cooling`
