# AI-FIREFOOD-RUNG / firefood_job

- **matrix_id:** `AI-FIREFOOD-RUNG`
- **status:** DONE
- **date:** 2026-07-28

## Closed

Haxe `doTimeStuffHelper` assigned/last `FIREFOODMAKER` → `makeFireFood(100)` ladder path:

1. `ProfessionScanKind::FireFood`
2. `ProfessionStickySnapshot::{fire_food_assigned, fire_food_last}` + `from_runtimes_ex` fire arg
3. `plan_assigned_job_steps` assigned + last fallthrough
4. `fire_food_profession_scan_tick` + `fire_food_peers_from_players(_ex)` + `peer_count_for_kind`
5. `profession_scan_tick` / ladder / apply writeback `Player.fire_food_profession`
6. `mod fire_food_rung` + `try_decide_fire_food_from_rung` pub use
7. npc_ai scan radius for FireFood

## Residual (out of this chunk)

- late / hungry / `isHandlingFire` makeFireFood(1/2/3)
- popcorn BowlFiller peer pick
- `fill_fire_food_counts` hot_coals_is_fire_place always false
- dedicated npc sticky role for FIREFOODMAKER (beyond craft profession default)
