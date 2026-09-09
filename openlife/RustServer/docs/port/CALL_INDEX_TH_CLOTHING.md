## Rust: clothing transitions (TH-CLOTHING-MATRIX)

| Symbol | File | Role |
|--------|------|------|
| `get_clothing_slot_index` / `is_clothing_string` | `ol-sim/src/clothing_transitions.rs` | Haxe ObjectData.getClothingSlot / isClothing |
| `allow_reset_uses_on_target` | same | Haxe resetNumberOfUses clothing rule |
| `resolve_switch_slot` / `ClothingSlotIds` | same | dual shoe + type match (doSwitchCloths) |
| `try_transition_on_clothing_pure` / `_with_content` | same | tryTranstionOnClothing multi-use |
| `put_into_clothing_nest` / `take_from_clothing_nest` | same | DoContainerStuffOnObj on worn clothing |
| `apply_switch_cloths` / `apply_place_obj_in_clothing` / `apply_sremv_from_clothing` | same | live player mutators |
| `apply_self_clothing` / `SelfClothingPath` / `can_store_held_in_worn_clothing` | same | doSelf: trans → clothing-in-clothing place → switch → place |
| `format_clothing_set` / `crown_say_line` | same | clothing_set string + king/mask say |
| DROP c / SELF / SREMV / UBABY wire | `ol-sim/src/lib.rs` | drop clothingIndex; doSelf drink→eat→clothing; specialRemove; doOnOther |
| re-export | `clothing_cmds` | `#[path]` nest + pub use |
| Tests | `clothing_transitions::*` / clothing_cmds | slot matrix, dual shoe, nest put/take, live switch/place/SELF, clothing-in-clothing, DROP c |
