# TH-ALT-OUTCOME / alt_transition_outcome

## Summary

Port Haxe `TransitionHelper` alternative transition outcome + fortification fail path (L1260–1306).

## Behavior

1. Resolve outcomes: transition list if non-empty, else new-target (or target) object list.
2. Gate: `targetID != 884`, `!allowForOwner`, outcomes non-empty **or** fortified (`cost>0 && hits<-0.1`).
3. Roll `rng + hits/10`; if `< 1` → **TryAgain**: hits+=1, optional PlaceObject bonus/fort material, **no** main transform, action applied; private say.
4. Else **Proceed**: hits −= 5 (local → final stamp), continue normal USE transform.

## Surfaces

| Layer | Path |
|-------|------|
| Pure | `ol-sim/src/alt_outcome.rs` |
| Content tables | `ol-content` `alt_outcomes_*` + `apply_default_alternative_outcome_patches` (`alt_outcome_patches.inc.rs`) |
| OLC1 boot | `binary_cache::finish_cache_boot` calls same patches |
| Live USE | `use_transition::apply_use_at` after hungry-work block |
| Say | `locks::note_lock_say` now accepts `impl Into<String>` for dynamic "Try again! Hits N" |

## Content patches (Haxe ServerSettings)

- Fort values: stone/shaft/adobe/boards; fortificationObjId on walls/fences/doors
- Transition push(0): 684+895/896/897 ancient walls, 462+2757/2759 springy doors
- Trees 340/342/3146 → fire wood/butt log; mines/ore pits; shovel/stump/clay; pick+gold/bear cave

## Tests

- `cargo test -p ol-sim --lib alt_outcome` — pure + live TryAgain/Proceed
- `cargo test -p ol-content --lib alt_outcome` — patches + resolve

## Residuals

- LiveSettings `AlternativeOutcomePercentIncreasePerHit` / `HitsDecreaseOnSucess` (hardcoded 10 / 5)
- `Transition.coinCost` deduct before gate (not on Transition yet)
- `Transition.hungryWorkCost` for is_fortified when only transition-level cost
- Fortify-apply USE (spend fort material, hits−=value, countObj++) L190–212
- Haxe TODOs: reduce tool on fail; piles for bonus drops; prob categories instead of push weights
