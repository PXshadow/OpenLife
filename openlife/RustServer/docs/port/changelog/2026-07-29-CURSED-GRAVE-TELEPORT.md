# CURSED-GRAVE-TELEPORT / tcg_tv_teleport

## Summary

Port Haxe admin SAY `!TCG`/`!CURSEDGRAVE` and `!TV`/`!VILLAGE`: closest-unblocked pick from global `cursed_graves` / `ovens` indexes → `blockedTeleportLocations` cycle → `doTeleport` snap + JumpToNonBlocked + VOG/MC/force PU.

## Behavior

1. Parse bang (exact `!TCG`/`!TV` or contains `!CURSEDGRAVE`/`!VILLAGE`).
2. Gate: `Player.godmode` (stands in for Haxe `account.canUseServerCommands`) → else PS `NOT ALLOWED!`.
3. Pick closest linear-index not in `blocked_teleport_locations`.
4. Empty → not-found say; all blocked → clear list + `Tried all locations. Start again!`.
5. Found → push blocked + `apply_do_teleport` (absolute snap, JumpToNonBlocked if blocked, cancel_movement VOG).

## Surfaces

| Layer | Path |
|-------|------|
| Pure | `ol-sim/src/teleport_cmd.rs` |
| Player field | `Player.blocked_teleport_locations` |
| Live | `apply_do_teleport` / `try_apply_teleport_bang` in `lib.rs` SAY path |
| Indexes | `WorldMapTimeState.cursed_graves` / `.ovens` (CURSED-GRAVES-INDEX) |

## Residuals

- Torus-wrap distance (Haxe transformX/Y) vs absolute quad
- `map_linear_index` y−1 delta vs Haxe WorldMap.index
- AiHelper.SearchNewHome still local oven scan
- CursedGraveTime not LiveSettings-wired
- coinCost `!TG` personal-grave path out of chunk

## Tests

```text
cargo test -p ol-sim --lib -- teleport_cmd say_tcg say_tv
```
