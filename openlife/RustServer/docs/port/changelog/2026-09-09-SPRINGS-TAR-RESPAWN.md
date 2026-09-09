# 2026-09-09 SPRINGS-TAR-RESPAWN

**status:** **DONE**

Haxe `ServerSettings.CanObjectRespawn`: Natural Spring 3030 / Tarry Spot 2285 / Dug Big Rock 503 never respawn.

- `can_object_respawn` already skipped DecayObject
- `should_try_respawn_object_ex` (Haxe RespawnObjects) now skips the blacklist
- `respawn_from_original_roll_ex3` (original-tile growback) now skips the blacklist

Tests: `can_object_respawn_blacklist`

`cargo check -p ol-server --offline` Finished.

Residual: Shallow Well 662 still decays to 3030 (Haxe id-table). Next **WELLS-OIL-DECAY**.
