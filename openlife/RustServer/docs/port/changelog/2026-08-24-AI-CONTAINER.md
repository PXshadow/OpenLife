# AI-CONTAINER — skip nonempty containers in craft scan

## Haxe

`AiBase.addObjectsForCrafting` ~7275–7282: if `numSlots > 0` and `containedObjects.length > 0`, skip. Comment: `TODO consider contained objects` (Haxe itself limited).

## Rust

| Symbol | Role |
|--------|------|
| `ScanTile::is_nonempty_container` | num_slots > 0 && contained_count > 0 |
| `nonempty_container_tiles_from_scan` | coords for CraftScanFilters |
| `npc_enqueue_get_or_craft_ex(..., nonempty_containers)` | skip those tiles in multi-step |
| npc_ai | live scan → enqueue |

## Tests

```
cargo test -p ol-sim --lib -- nonempty_container npc_enqueue_ex_nonempty
```

## Residual

Search *inside* containers (Haxe TODO). Drop-into-container already **DROP-HELD-AI**.
