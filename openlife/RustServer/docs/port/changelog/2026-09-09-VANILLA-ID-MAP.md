# 2026-09-09 VANILLA-ID-MAP

**status:** **DONE**

Haxe `Server.mapIdToVanillaId` / `VanillaObjIdMap` / `InitVanillaObjectIdMap` for vanilla clients.

- `lastVanillaID < 1` (default -1) mapping off; ids `<= lastVanillaID` pass through
- OpenLife-only ids: `+VanillaId N` map or 0; dummy ids `> lastOpenLifeID` offset
- `lastOpenLifeID` stamped before dummy allocation (text) / min dummy-1 (OLC1)
- LOGIN `client_tag` containing `OpenLife` keeps raw ids; others get patched MX/MC
- World storage stays raw OpenLife ids (wire-only remap)

Tests: `vanilla_id::*` / `force_mc_vanilla_client_patches_object_id` / `force_mc_openlife_client_keeps_raw_id` / `apply_live_settings_vanilla_id_map`

`cargo check -p ol-server --offline` Finished.

Residual: ol-net login_bootstrap MC is unpatched; sim `force_send_map_chunk` on login is the authority. PU held ids not remapped (Haxe leftover is MX/MC only). Next **HALF-PENALTY-STRONG**.
