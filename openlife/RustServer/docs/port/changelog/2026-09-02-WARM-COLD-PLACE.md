# 2026-09-02 warmPlace / coldPlace fitness

**status:** **DONE**

## Closed

- Haxe `warmPlace` / `coldPlace` remembered tiles on `Player` (session + PLB1 `storedInt` warmTx/Ty coldTx/Ty).
- Seed when felt temp > 0.55 / < 0.45; replace only in desert/jungle or snow/passable-river using torus-quad fitness (`/ (quad+25)` vs `/ 25`; water ×2).
- Birth inherits mother's places.
- NPC superbad heat (`>0.9` / `<0.1`) walks to cold/warm place (`plan_temp_place_goto`) after eat-held.

## Tests

- `warm_place_seeds_and_prefers_closer_desert` / `cold_place_water_fitness_doubles`
- `desert_tick_records_warm_place` / `tick_vitals_desert_seeds_warm_place`
