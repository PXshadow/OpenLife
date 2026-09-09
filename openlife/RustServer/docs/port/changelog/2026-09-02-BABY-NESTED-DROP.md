# 2026-09-02 BABY-NESTED-DROP

**status:** **DONE**

Haxe `doBabyHelper`: if `target.heldPlayer != null` → `target.dropPlayer(this.x, this.y)`; if still holding, refuse pickup.

Live `apply_do_baby_hold` uses `needs_force_drop_nested_hold` then `apply_drop_player` at the carrier tile. Nested child lands on that tile; pickup continues. HOLD/BABY share the path.

Tests: `baby_nested_hold_force_drops_then_picks_up` / `needs_force_drop_nested_hold_haxe`.

`cargo check -p ol-server --offline` ok.
