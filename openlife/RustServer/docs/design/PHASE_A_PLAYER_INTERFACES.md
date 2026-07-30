# Phase A — PlayerWriteInterface / PlayerReadInterface

**Status:** done (2026-07-30)  
**Tree:** git-tracked `openlife/RustServer`  
**Reference copy:** `C:\OhOl\RustServer-ai-split` (lookup only)

## Naming

| Name | Role |
|------|------|
| **`PlayerWriteInterface`** | Shared **write** API for humans and AI → enqueues `NetIntent` (command payload) |
| **`PlayerReadInterface`** | Fast **read** façade: world + self body + best-food |
| **`NetIntent`** | Command *enum* consumed by `apply_intent` (rename to `PlayerCommand` later optional) |
| Sub-traits | `WorldView`, `PlayerView`, `FoodSearch` (building blocks of read) |

Deprecated aliases (compat): `PlayerCommands` = `PlayerWriteInterface`, `IntentSink` = `CommandSink`.

## Crate graph (Phase A)

```text
ol-ai-api     traits + DTOs only  (→ ol-net)
ol-ai         pure AI + re-exports ol-ai-api
ol-sim        implements read adapters (ai_adapters.rs)
ol-server     NPC: write via PlayerWriteInterface; food via FoodSearch r=40
```

## What landed

1. **`crates/ol-ai-api`**
   - `PlayerWriteInterface` / `CommandSink`
   - `PlayerReadInterface` + `PlayerReadHandles`
   - `WorldView`, `PlayerView`, `FoodSearch`
   - `BestFoodQuery` / `BestFoodHit`, `DEFAULT_FOOD_SEARCH_RADIUS = 40`

2. **`ol-ai`** re-exports the API; local trait modules removed. (Later: façade also re-exports helper / pathing / crafting / professions.)

3. **`ol-sim/src/ai_adapters.rs`** (now `mod` + `pub use`)
   - `WorldViewRef`, `PlayerRef`, `PlayerSnapshotView`
   - `SimFoodSearch` / `best_food_for_ai` (ground scan; default r=40)
   - `SimPlayerRead` implements full `PlayerReadInterface`

4. **`npc_ai`**
   - `NpcWriteTx` implements write interface
   - `NpcNearbyFoodSearch` implements `FoodSearch` for pre-scanned nearby tiles
   - Seek food uses `best_food_default` (radius **40**)
   - USE/MOVE/DROP/REMV helpers go through write interface

## Hard rules (enforced by structure)

- AI **must not** mutate `World` / `Player` directly for gameplay.
- Writes = `PlayerWriteInterface` → same `NetIntent` as TCP clients.
- Reads = `PlayerReadInterface` / sub-traits (AI-fast; not TCP).

## Next (Phase B)

- `ol-player-helper` for pure food scoring shared with players
- Upgrade `SimFoodSearch` to full `search_best_food_live.inc.rs` when included in `lib.rs`
- Migrate remaining raw `try_send(NetIntent::…)` in `npc_ai` to write helpers
