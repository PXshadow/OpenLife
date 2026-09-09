# AI-PATHFINDER-NEW — CreatePath core

## Haxe

- `PathfinderNew.CreatePath` ~131–148
- `CreateDirectPath` ~352–454 (greedy 8-conn, reverse `factor=-1`)
- `CreatePathBruteForceInCircle` + `makePathOneTile` ~155–260 (wave meet)
- `AddPathFromCrossing` / `CreatePathFromMap` ~456–555
- `AiHelper.GotoHelper` uses PathfinderNew (`MapData.RAD=32`)

## Rust

| Symbol | Role |
|--------|------|
| `ol-ai-pathing::pathfinder_new::create_path` | pure CreatePath |
| `create_direct_path` / `path_to_steps` | greedy paint + dx/dy |
| `find_path_new` / `next_step_new` | `pathfind.rs` world-window wrappers (`PATHFINDER_NEW_DEFAULT_RADIUS=32`) |

Live `next_step` remains 4-connected A*.

## Tests

```
cargo test -p ol-ai-pathing -- pathfinder_new
cargo test -p ol-sim --lib -- next_step_new find_path_new
```

## Residual

1. Live Goto / `next_step` still A* → **AI-PATHFINDER-GOTO**
2. 100ms wall-clock timeout
3. Unused `CreatePathBruteForce` (full grid, not circle)
4. `WriteMapToFile` / old-vs-new timing
