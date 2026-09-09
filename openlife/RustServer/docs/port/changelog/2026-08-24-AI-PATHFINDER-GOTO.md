# AI-PATHFINDER-GOTO — live next_step = PathfinderNew

## Haxe

`AiHelper.GotoHelper` uses `PathfinderNew.CreatePath` (`MapData.RAD=32`, 8-connected).

## Rust

| Symbol | Change |
|--------|--------|
| `next_step` | now `next_step_new` (PathfinderNew) |
| `goto_path_outcome` | reachability via `find_path_new` |
| `GOTO_COLLISION_RAD` | 16 → 32 (Haxe RAD) |
| `try_ai_follow_path_to` | `find_path_new` for follow MOVE |

`find_path` / `path_steps` / `SAY STEPS` remain 4-conn A*.

## Tests

```
cargo test -p ol-sim --lib -- pathfind::
```

## Residual

SAY PATH now 8-conn (debug). Timeout → **AI-PATHFINDER-TIMEOUT**; `WriteMapToFile` omitted.
