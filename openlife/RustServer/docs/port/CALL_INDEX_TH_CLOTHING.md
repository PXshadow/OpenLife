## Rust: clothing transitions (TH-CLOTHING-MATRIX)

| Symbol | File | Role |
|--------|------|------|
| `get_clothing_slot_index` / `is_clothing_string` | `ol-sim/src/clothing_transitions.rs` | Haxe ObjectData.getClothingSlot / isClothing |
| `allow_reset_uses_on_target` | same | Haxe resetNumberOfUses clothing rule |
| `resolve_switch_slot` / `ClothingSlotIds` | same | dual shoe + type match (doSwitchCloths) |
| `try_transition_on_clothing_pure` / `_with_content` | same | tryTranstionOnClothing multi-use |
| `put_into_clothing_nest` / `take_from_clothing_nest` | same | DoContainerStuffOnObj on worn clothing |
| `apply_switch_cloths` / `apply_place_obj_in_clothing` / `apply_sremv_from_clothing` | same | live player mutators |
| `apply_self_clothing` / `SelfClothingPath` | same | doSelf clothing order: trans → switch → place |
| `format_clothing_set` / `crown_say_line` | same | clothing_set string + king/mask say |
| DROP c / SELF / SREMV wire | `ol-sim/src/lib.rs` (build_clothing_transitions) | TransitionHelper.drop clothingIndex; GPI.self; specialRemove |
| re-export | `clothing_cmds` | `#[path]` nest + pub use |
| Tests | `clothing_transitions::*` / clothing_cmds | slot matrix, dual shoe, nest put/take, live switch/place/SELF |
