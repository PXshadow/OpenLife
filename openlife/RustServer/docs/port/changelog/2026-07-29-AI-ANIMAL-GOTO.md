# AI-ANIMAL-GOTO / animal_goto_marks (2026-07-29)

## Summary

Haxe `AiHelper.gotoAdv` dual-pass animal path failure: path with deadly-animal footprints fails, then recheck without animals — if that succeeds mark `hostile_path` (20s), else `not_reachable` (90s).

| Haxe | Rust |
|------|------|
| `considerAnimals = checkIfDangerous && didNotReachFood < 5 && food_store > -1` | `consider_animals_for_goto` |
| `Goto(…, considerAnimals)` then `Goto(…, false, move=false)` | `goto_path_outcome` |
| `CreateCollisionChunkHelper` moves footprint | `collect_deadly_animal_blocked_*` |
| `addHostilePath` / `addNotReachable` | `mark_goto_path_fail(blocked_by_animal)` |
| `gotoObj` receding distance > 100 | pure `receding_goto_should_abort` (no live sticky yet) |

## Files

- `ol-sim/src/ai_path_reach.rs` — pure gates + existing mark helpers
- `ol-sim/src/pathfind.rs` — animal collision + dual-pass outcome + next_step
- `ol-server/src/npc_ai.rs` — profession walk uses animal-aware step + dual-pass mark
- `ol-sim/src/lib.rs` — re-exports

## Tests

```powershell
cargo test -p ol-sim --lib -- pathfind:: ai_path_reach::
```

## Residual

- `Player.lastGotoObj` / `lastGotoObjDistance` live receding abort
- `resetTargets()` on gotoAdv fail
- Full `isAnimalNotDeadlyForMe` biome love / hits / weapon nuance
- Food/explore/seek walk dual-pass (only profession walk-fail wired)
- `didNotReachFood` sticky on NPC (gate uses 0)
