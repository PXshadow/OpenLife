# 2026-09-05 AI-HANDLE-TEMP

**status:** **DONE**

Haxe `AiBase.handleTemperature` drink / GetOrCraft water / close biome / firePlace / arrive-relax / fail-cool / fail-warm (kindling 72 then `isHandlingFire(2)`). Superbad remembered-place goto stays as fallback.

- `handle_temperature.rs` pure SM
- `Player.ai_handling_temperature` / `ai_temp_just_arrived` / `ai_last_heat`
- Live `apply_handle_temperature_tick` on Temperature + ChildWithMother rungs
- npc 1b full SM (SELF drink, craft water, biome goto)

Tests: `handle_temperature::*` / `apply_handle_temperature_tick_*`

`cargo check -p ol-server --offline` ok.
