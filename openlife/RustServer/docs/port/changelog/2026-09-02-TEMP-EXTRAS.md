# 2026-09-02 updateTemperature extras (color / biome-love / storedWater / held-by)

**status:** **DONE**

## Closed

- Haxe `getTemperatureShiftForColor` (Black +0.1, Brown +0.05, White −0.05, Ginger −0.1) subtracted from tile ambient.
- Loved-biome temperature boni: `biomeLoveFactor/10` capped at `TemperatureMaxLovedBiomeImpact` 0.1; warms when both heat and ambient &lt; 0.5, cools when both &gt; 0.5; negative love ignored.
- `storedWater` evaporative cool when `heat > 0.6` (`dt * stored * (heat-0.6) * 0.05`); drains reserve. Drink already fills `Player.stored_water`.
- Held baby: ambient = holder `heat` (Haxe `heldByPlayer.heat`).
- Live `tick_vitals` passes person color, living parent colors, `held_by`, and writes back `stored_water`.

## Tests

- `temperature_shift_for_person_colors` / `ginger_color_shift_warms_snow_ambient_vs_black`
- `biome_love_warms_when_cold_and_ignores_hate`
- `stored_water_cools_hot_and_drains_reserve` / `tick_vitals_stored_water_cools_hot_player`
- `held_by_overrides_ambient_to_holder_heat` / `tick_vitals_held_by_uses_holder_heat`
