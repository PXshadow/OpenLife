# 2026-09-03 MIN-PICKUP-AGE

**status:** **DONE**

Haxe `ObjectData.minPickupAge` + `ServerSettings.ReduceAgeNeededToPickupObjects` (default 10) on USE/DROP/REMV.

- **Content:** parse `minPickupAge=` (same comma group as `permanent=`), copy OLC1 `min_pickup_age` onto `ObjectDef`, PatchObjectData 151 last-write 5 (Yew Bow) / 560 → 2 (Knife). Haxe writes 151 twice; port-as-is, do not retarget War Sword 3047.
- **USE:** multi-use `numberOfUses > 1` refuse except gooseberry 30/391; ReduceAge say `I am N years too young`; `oldEnoughForTransitions` (age or description BERRY); `oldEnoughForPickup` (age or empty + speedMult ≥ 0.98).
- **DROP:** after clothing, before isClose: ReduceAge say, multi-use silent, contained silent.
- **REMV:** multi-use silent. **SWAP** has no age gate (Haxe).
- **Live:** `reduce_age_needed_to_pickup_objects` on server.toml / LiveSettings / GameplayKnobs.

Tests: `pickup_age_years_too_young_ceil` / `use_min_pickup_age_reduce_says_too_young` / `use_min_pickup_age_speed_mult_allows_pickup` / `use_min_pickup_age_skips_transition_not_berry` / `use_gooseberry_berry_allows_transition_when_young` / `use_live_reduce_age_zero_refuses` / `drop_min_pickup_age_container_cargo_refuses` / `remv_min_pickup_age_multi_use_refuses` / `apply_min_pickup_age_patches_bow_knife`.

`cargo check -p ol-server --offline` ok.

**CRAFT-LIVE-IO** was already live (`advance_use_held_staging` / npc `pending_use`); not re-implemented.
