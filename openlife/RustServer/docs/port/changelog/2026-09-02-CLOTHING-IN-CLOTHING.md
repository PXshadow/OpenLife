# 2026-09-02 CLOTHING-IN-CLOTHING

**status:** **DONE**

## Closed

- Haxe GPI `doSwitchCloths` L3515 TODO: store clothes in clothes (backpack) while wearing.
- Live SELF: if the requested slot is a worn container that can accept held clothing (different native slot), `doPlaceObjInClothing` runs **before** `doSwitchCloths` so `SELF x y 5` with a hat nests the hat instead of redirect-equipping it.
- Live DROP: `clothingIndex >= 0` calls `doPlaceObjInClothing(isDrop=true)` **before** not-moving / close-enough / ground place (Haxe `TransitionHelper.drop`). Failed clothing DROP does not fall through to the tile.
- Containable / containSize gates stay Haxe `DoContainerStuffOnObj` (no clothing bypass). Same-slot backpack still switches. Full backpack / non-containable clothing still equips.

## Tests

- `self_stores_hat_in_worn_backpack`
- `self_minus_one_still_equips_hat_while_wearing_backpack`
- `self_same_slot_backpack_still_switches`
- `self_full_backpack_falls_back_to_equip`
- `self_non_containable_clothing_still_equips`
- `drop_hat_into_worn_backpack_swaps_or_inserts`
- `drop_c_into_worn_backpack_not_ground`

`cargo check -p ol-server --offline` ok.
