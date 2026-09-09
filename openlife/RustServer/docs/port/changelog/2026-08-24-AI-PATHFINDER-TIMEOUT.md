# AI-PATHFINDER-TIMEOUT — PathfinderNew 100ms brute abort

## Haxe

- `PathfinderNew.timeOut = 100` ms
- `CreatePathBruteForceInCircle` ~172: `(Sys.time()-startTime)*1000 > timeOut` → break → null path
- `WriteMapToFile` debug dump (`DebugWritePathToFile`); full-grid `CreatePathBruteForce` unused

## Rust

| Symbol | Role |
|--------|------|
| `PATHFINDER_NEW_TIMEOUT_MS` | 100 |
| `PATHFINDER_NEW_MAX_EXPANSIONS` | 64 (`2 * RAD`) |
| `PathBudget` | `{ max_ms, max_expansions }` live 100ms + 64 |
| `create_path_with_budget` | brute abort like Haxe |
| `create_path` | `PathBudget::LIVE` |
| `find_path_new_with_budget` | world-window wrapper |

`WriteMapToFile` omitted (no disk write). Unused full-grid brute omitted.

## Tests

```
cargo test -p ol-ai-pathing --lib -- pathfinder
cargo test -p ol-sim --lib -- next_step_new find_path_new
```

## Residual

1. `WriteMapToFile` debug dump (skip)
2. Haxe unused full-grid brute (skip)
3. AI-SAY-HELPER live fan-out (**AI-SAY-HELPER-FAN**)
