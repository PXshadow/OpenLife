# 2026-09-05 AI-REMOVE-CONTAINER

**status:** **DONE**

Haxe `AiBase.isRemovingFromContainer` / `removeItemFromContainer`. Sticky `removeFromContainerTarget` + `expectedContainer`. First tick stages; later ticks drop / goto / REMV (always clear after remove attempt).

- `remove_from_container.rs` pure SM
- `PlayerCraftAi.remove_from_container` + npc sticky
- Graves cargo → `StageRemoveFromContainer` (not USE)
- Live `apply_player_remove_from_container_tick` after isUsingItem; handleDeath busy path; npc before eat

Tests: `remove_from_container::*` / `apply_profession_scan_tick_stages_remove_from_container` / `apply_remove_from_container_tick_remvs_when_adjacent`

`cargo check -p ol-server --offline` ok.
