# 2026-09-09 AI-KNIFE-STUFF

**status:** **DONE**

Haxe mid `doKnifeStuff` (~876) after `isHandlingFire()` (~635). Held Knife **560** opportunist (not assigned TAILOR/BAKER).

- Held not 560 -> none (no GetOrCraft knife)
- Else `shortCraft(560, target, 20, false)`: bread **1470**, dough plate **1468**, Dead Wolf **422**, Dead Bear **643**
- `craftActorIfNeeded=false` so missing knife does not enqueue craft 560
- Ladder `KNIFE_STUFF` after HandlingFire on MidPriorityTasks / CriticalMisc; also hungry ConsiderMakeFood ~8549
- `mid_tasks_pending` when held knife + target in r=20

Tests: `held_not_knife_returns_none_even_with_bread` / `held_knife_and_bread_in_r20_uses` / `craft_actor_if_needed_false_does_not_enqueue_craft_knife` / `bread_then_dough_then_wolf_then_bear` / `farm_profession_scan_tick_knife_stuff_uses_bread_not_bear` / `apply_profession_scan_tick_knife_stuff_uses_held_knife_on_bread`

`cargo test -p ol-sim --lib -- knife_stuff` then `cargo check -p ol-server --offline`.

Residual: baker `knife_bread_stage` / smith knife remain assigned-job paths. Next **AI-ATTACK-PLAYER**.
