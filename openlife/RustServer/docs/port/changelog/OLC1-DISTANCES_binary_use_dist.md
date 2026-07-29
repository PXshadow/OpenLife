# OLC1-DISTANCES / binary_use_dist (2026-07-28)

## Summary

OLC1 write path **v7** trailer + client bake fidelity + animal deadly on ObjectDef:

| Field | Type | Haxe | Default (format &lt; 7) |
|-------|------|------|-------------------------|
| `deadly_distance` | f32 | `ObjectData.deadlyDistance` | `0.0` |
| `use_distance` | i32 | `ObjectData.useDistance` | `1` |
| `moves` | i32 | `ObjectData.moves` | `0` |

Haxe binary `writeToFile`/`readFromFile` store deadly+use only (not moves). Rust OLC1 v7 also stores `moves` for IS-CLOSE animals without waiting for a move tick.

## Code

- `ol-binary` `olc1.rs` — encode/decode + `OLC1_FORMAT_VERSION = 7`
- `ol-content` `binary_cache.rs` — `olc1_record_to_object` → `ObjectDef`
- `ol-content` `apply_default_weapon_range_patches` — bows 5/4, knives 1.5
- `ol-content` `apply_default_animal_deadly_distance_patches` — `AnimalDeadlyDistanceFactor` 0.5
- `ol-content` `apply_animal_moves_from_transitions` — stamp `moves` from time-move
- RustClient `ClientObjectDef` — parse `useDistance`/`deadlyDistance`/`moves` from object text
- RustClient `client_object_to_olc1` / `olc1_to_client_object` — real values (not defaults)

## Tests

- `ol-binary::olc1_v7_roundtrip_minimal`
- `ol-binary::olc1_legacy_v6_defaults_distances`
- `ol-content::olc1_v7_distances_server_load`
- `ol-content::finish_cache_boot_weapon_and_animal_distance_patches`
- `ol-content::parse_use_and_deadly_distance_and_weapon_patches`
- `ol-content::effective_use_distance_and_is_animal`
- client `olc1_v7_client_bake_distances_roundtrip`
- client `parse_use_deadly_distance_and_moves`

## Residual (out of chunk)

- Connection MaxDistance fans (IS-CLOSE residual, not OLC1)
- ~~“Too close” say/PS~~ → **GPI-TOO-CLOSE DONE** (`note_too_close_say` + live public PS)
- Rebake existing on-disk client caches once to pick up non-default object-file distances
