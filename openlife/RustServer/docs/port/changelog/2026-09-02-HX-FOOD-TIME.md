# 2026-09-02 HX foodDrainTime (HEAT food_time)

**status:** **DONE**

## Closed

- Haxe GPI `updateTemperature` `foodUsePerSecond` / `foodDrainTime` on the HEAT (`HX`) wire: `heat foodDrainTime 0`.
- `temperatureFoodFactor = heat >= 0.5 ? heat : 1-heat`; super-hot/cold (person-color gates) add `damageFactor * (Hits + Exhaustion)` to the divisor (`×2` when heat > 0.95 / < 0.05).
- Live `tick_vitals` + login bootstrap HX; `Player.food_use_per_second` written and PLB1 persist.
- Indoor bonus stays 0. Biome/day/weather extras stay on the food-eating drain path (not this packet).

## Tests

- `heat_food_drain_time_ideal_is_twenty` / `heat_food_drain_time_superhot_scales_damage`
- `tick_vitals_emits_hx_heat_every_interval` / `social_bootstrap_hx_includes_food_drain_time`
- persist `multi_player_record_round_trip` food_use_per_second
