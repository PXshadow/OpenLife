# 2026-09-03 AI-BLOCK-CMD

**status:** **DONE**

Haxe `doCommandHelper` `blockTargetForAi` after DROP/SWAP/REMV (tile at command xy; post-command held for smith hammer 441). USE applied path already live. Shared `note_block_target_for_ai_after_command`. Food/permanent/weapon/animal/clothing still skip.

Tests: `drop_sets_player_block_for_ai` / `drop_food_does_not_block_for_ai` / `swap_sets_player_block_for_ai` / `remv_permanent_container_does_not_block_for_ai`.

`cargo check -p ol-server --offline` ok.
