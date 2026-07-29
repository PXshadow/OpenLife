# PO-MAX-DISTANCE / close_say_range

**Date:** 2026-07-29  
**Mode:** implement  
**Status:** **DONE**

## Haxe

- `ServerSettings.MaxDistanceToBeConsideredAsCloseForSay = 20`
- `Connection.sendSayToAllClose` → `isClose(..., MaxDistanceToBeConsideredAsCloseForSay)`
- Distinct from `MaxDistanceToBeConsideredAsClose` (PU, often 2e6) / movement 30 / map 10
- Confirm path: `TimeHelper` newFollowerTime → `player.newFollower.say(...)` → same CloseForSay

## Rust

| Piece | Location |
|-------|----------|
| Adult say radius | `speech::ADULT_CHAT_RANGE` = **20** (+ `MAX_DISTANCE_CLOSE_FOR_SAY`) |
| Age soft scale | `chat_range_for_age` infants 8 / children 16 / adult+elder **20** (NaN/non-finite → 20) |
| Interest cull | `NEARBY_RANGE` = **24** unchanged (PU/MX) |
| LiveSettings | **ModuleConst residual** (intentional; not hot-reloaded) |
| Say fans | AI/scripted/LLM/`send_chat_ps` + do-commands + **pending newFollower spoken_says** + social-pin count + coins/moskitos says |
| FIELD_MAP | `MaxDistanceToBeConsideredAsCloseForSay` → ModuleConst |

## Tests

- `speech::tests::age_brackets` asserts 20
- `say_adult_close_for_say_range_twenty` live PS gate cheby 22 vs 20
- `pending_follower_spoken_says_close_for_say_range` delayed confirm PS gate
- `chat_range_for_age_nan_matches_speech` live vs pure NaN parity
- `mumble::mumble_narrower_than_adult`

## Residual

- Euclidean vs Chebyshev metric on `isClose` (product-wide distance metric; out of this chunk)
- `MaxDistanceToBeConsideredAsClose` product 2e6 vs practical `NEARBY_RANGE` (PO-FAR intentional)
- MuteBook/DEAF filters on `send_chat_ps` (Rust product; Haxe distance-only)
- Young age soft scale (Rust product; Haxe always CloseForSay 20)

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- age_brackets -- --test-threads=1
cargo test -p ol-sim --lib -- say_adult_close_for_say -- --test-threads=1
cargo test -p ol-sim --lib -- pending_follower_spoken_says -- --test-threads=1
cargo test -p ol-sim --lib -- chat_range_for_age_nan -- --test-threads=1
cargo test -p ol-sim --lib -- mumble_narrower -- --test-threads=1
```
