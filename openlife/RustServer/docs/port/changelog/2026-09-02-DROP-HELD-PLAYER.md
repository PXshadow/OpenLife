# 2026-09-02 DROP-HELD-PLAYER

**status:** **DONE**

Haxe `GPI.drop` / `GPI.swap`: if `heldPlayer != null` → `dropPlayer(x,y)` (not object DROP / JUMP wiggle).

Live `apply_drop_player` (`dropPlayerHelper`): range 1 (squared Euclidean + wrap); `isBlocked` = `blocksWalking` then biome (boat on water open); place held player on click tile; clear hold links; PU both.

DROP checks holding player **before** clothing/object DROP. SWAP holding-player now uses `apply_drop_player` instead of JUMP.

Tests: `drop_releases_held_player_at_tile` (too-far keeps hold; adjacent lands on tile) / `swap_drops_held_player_at_tile`.

`cargo check -p ol-server --offline` ok.
