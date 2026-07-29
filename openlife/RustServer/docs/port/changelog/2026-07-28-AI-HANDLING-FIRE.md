# AI-HANDLING-FIRE / is_handling_fire

- **matrix_id:** `AI-HANDLING-FIRE`
- **status:** DONE
- **date:** 2026-07-28

## Closed

1. pure `handling_fire.rs` — `is_handling_fire` / full expand / `FireFoodDispatchPath`
2. nested makeFireFood(2) near coals; makeFireFood(3) on hot-coals place + kindling
3. FIREKEEPER sticky `Player.fire_keeper_profession`
4. `ProfessionScanKind::HandlingFire` mid early + assigned/last
5. `late_make_fire_food_scan_tick` makeFireFood(1) **wired** on Low/Mid/AgeRotated/CriticalMisc/ConsiderMakeFood after empty steps
6. Fire 82 fuel cascade: firewood when stocked else `is_handling_fire_fire_fuel_tail` (butt log >10 / kindling)
7. Winter: `ProfessionScanInput.is_winter` from `Environment::Winter` → kindling first on Fire 82
8. DoBaking(2) live expands via `do_baking` (`expand_handling_fire_do_baking`)
9. Temperature rung → HandlingFire max=2; ConsiderMakeFood → early isHandlingFire + late fire food
10. tests pure + sticky + plan + late peer cap + winter live scan

## Residual

- full multi-peer `getBestAiForObjByProfession` distance pick (sticky/weight/peer heuristic live)
- `Player.fire_place` sticky tile coords (scan recomputes GetCloseFire)
- Haxe `itemToCraft.maxSearchRadius=30` craft side-effect around nested makeFireFood(3)
