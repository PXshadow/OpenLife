# AI-POTTER-RESID / pottery_resid

**Date:** 2026-07-29  
**Status:** DONE (EmptyBasketAtHome extract + wet-nozzle cleanup)

## Haxe

- `AiBase.gatherClay` EmptyBasketAtHome ~L3007–3021  
  - `dropIsAUse = false` + `dropTarget = basket` (TODO empty basket ???)  
  - empty hands → `isDropingItem` → `myPlayer.drop` → `DoContainerStuffOnObj` takes first contained (clay)
- `AiBase.cleanUp` wet-nozzle ~L1031–1041 (outside L2946 pottery-on-fire TODO)  
  - `shortCraft(285, 285, 30, false)` when count>1 or (count>0 && held 285)  
  - `shortCraft(0, 2110, 20)` Clay with Nozzle → Wet Clay Nozzle

## Rust

### EmptyBasketAtHome

1. Pure `empty_basket_at_home_is_drop_extract(held_id)` — true when empty hands  
2. Live `pottery_action_to_live_intent(EmptyBasketAtHome)` → **DropAt** on clay-in-basket near home (r=10 then scan) — not UseAt (USE would pick up whole basket)  
3. Live `apply_drop` empty-hand path: container take index 0 (`empty_hand_container_take_index`) + non-permanent floor swap

### Wet-nozzle cleanup

1. Pure `wet_nozzle_cleanup_action(count_ground_285, held_id, has_2110)`  
2. Constants: `CLAY_WITH_NOZZLE=2110`, search/count radii  
3. Wired early in `do_pottery` after charcoal basket / before fire kiln

## Tests

- `pottery_profession::tests::empty_basket_at_home_drop_extract_and_gather`
- `pottery_profession::tests::wet_nozzle_cleanup_merge_and_clay_with_nozzle`
- `short_craft_intent::profession_scan::tests::empty_basket_at_home_live_is_drop_extract`
- pottery filter: 52 passed

## Residual (out of this chunk)

- Full Haxe `cleanUp` body (pileUp stone/straw/corn, skewers, cleanUpBowls)
- age `% 3` cleanUp gate (general AI, not potter-only)
- General cleanUp live tick outside pottery profession

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- empty_basket wet_nozzle pottery -- --test-threads=1
```
