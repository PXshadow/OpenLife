# 2026-09-09 AI-ATTACK-PLAYER

**status:** **DONE**

Haxe `AiBase.attackPlayer` (~5822) on `PriorityRung::Combat`. Live getWeapon + stand-off walk + `kill`.

- null / wounded target / food_store < -2 / self wounded / age < MinAiAgeForCombat (8) -> none
- `getWeapon(false)`: already holding weapon skips; quiver SELF slot 5; dropHeld(5) for 4151; pickup knife/sword/bow r=40; else GetOrCraft 152 (or 148 if empty quiver-with-bow 4149)
- exact quad vs deadlyDistance: too far or (range>1.9 && quad<1.5) -> goto stand-off tile
- else `KILL` (client-relative) + clear didNotReachFood
- Sensors `combat_target` from GetCloseDeadlyPlayer / GetClosePlayerTarget; skip-escape when armed age>8
- Live `apply_profession_scan_from_sensors` Combat rung; npc_ai after pickup food

Tests: `null_wounded_food_age_gates` / `empty_hand_picks_up_closest_knife` / `no_ground_weapon_seeks_bow` / `knife_in_range_kills_adjacent` / `knife_too_far_gotos_standoff_on_target_tile` / `bow_too_close_gotos_standoff` / `bow_in_range_kills` / `attack_player_action_maps_kill_and_pickup` / `apply_profession_scan_from_sensors_attack_player_kills_adjacent`

`cargo test -p ol-sim --lib -- attack_player` then `cargo check -p ol-server --offline`.

Residual: GetOrCraftItem craft graph for missing bow; mask-only GetClosePlayerTarget family/top-leader polish; killAnimal still **AI-KILL-ANIMAL**. Next **KNOCKOUT-PICKUP**.
