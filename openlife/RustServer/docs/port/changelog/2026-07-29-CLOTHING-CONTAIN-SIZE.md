# CLOTHING-CONTAIN-SIZE / contain_slot_size (2026-07-29)

## Summary

ObjectDef gains Haxe `containSize` / `slotSize` and live container/clothing gates use them.

| Field | Type | Haxe | Default |
|-------|------|------|---------|
| `contain_size` | f32 | `ObjectData.containSize` | `0.0` |
| `slot_size` | f32 | `ObjectData.slotSize` (text `slotsSize=`) | `1.0` |

Gate rule (port-as-is): **refuse when `containSize > container.slotSize`**.

## Code

- `ol-content` `ObjectDef` + text parse (`containSize=`, `slotsSize=`, alt `slotSize=`)
- `ol-binary` OLC1 **v8** trailer after v7 distances; legacy &lt;8 → defaults 0/1
- `ol-sim/place_object.rs` — `contain_fits_slot`, `contain_slot_sizes`, `object_contain_fits_container`; grave uses content sizes
- `ol-sim/clothing_transitions.rs` — `can_put_into_clothing_sized` + `apply_place_obj_in_clothing`
- `ol-sim/lib.rs` — DROP into container + PUTNEST size refuse (`FAIL SIZE`)

## Tests

- `ol-content::parse_contain_size_and_slot_size`
- `ol-binary::olc1_v8_contain_slot_size_roundtrip` / `olc1_legacy_v7_defaults_contain_slot`
- `clothing_transitions::clothing_contain_size_gate` / `apply_place_obj_in_clothing_size_refuse`
- `place_object::contain_size_from_object_def_blocks_grave_swallow`
- filter: `cargo test -p ol-sim --lib -- contain_size slot_size clothing contain` (130 ok)

## Residual close (same day — gap pass)

- **ServerSettings containSize patches:** `apply_default_contain_size_patches` (desc: Flat Rock / Mechanism / Blowpipe / Crucible / Shears; ids 0,356,2188,2191–2,319,321–2,325,1528,2573–4,2578,300–302) wired in text load + OLC1 `finish_cache_boot`
- **TH L1087 containerSlotSize:** pure `transition_result_fits_container*` + live `apply_use_at_ex(..., container_index)`; NetIntent::Use `index` wired; in-place contained slot update keeps outer id
- Tests: `apply_default_contain_size_patches_*` / `use_on_container_*` / `transition_result_fits*`

## Residual

- RustClient OLC1 max format still v7 until client bake follows v8
- Unify remaining TH/combat PlaceObject spill sites (PLACE-OBJECT residual)
- USE-on-container: full multi-use / loved-food / lock side-effects on contained slots are best-effort; bare no-transition container USE still deferred to REMV/DROP
