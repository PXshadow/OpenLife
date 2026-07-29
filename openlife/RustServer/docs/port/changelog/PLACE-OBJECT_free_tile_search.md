# PLACE-OBJECT / free_tile_search (2026-07-26)

## Status: **DONE** (core + gap close)

### Haxe
- `WorldMap.PlaceObject` / `TryPlaceObject` / `TransformObject` / `PlaceObjectById`
- Expanding random free-tile search (`distance = ceil(i/(20*d*d))`, `randomInt(2d)-d`)
- `isBiomeBlocking`, don't-place-behind-tree (`y-1` isTree)
- Grave swallow via `canBePlacedIn` (full `ObjectHelper` push including nested)
- `allowReplace` non-permanent: returns displaced helper; outer loop re-places with `allowReplace=false`
- `considerWalls` → `TimeHelper.CalculateNonBlockedTarget`
- Drop walls after 5000 attempts
- Callers: placeGrave, doBabyHelper held drop, TryAnimaEscape bow 798, etc.

### Rust
- `ol-sim/src/place_object.rs` — pure + live helpers (included from `death_polish.rs`)
- **allowReplace re-home**: `TryPlaceInternal::NeedRehome` continues free search for displaced
- **Grave nested swallow**: `grave_swallow_push` → `slots` + `rebuild_wire_from_slots` via `complex_to_nested`
- Pure size gate: `contain_fits_slot` / `can_be_placed_in_grave_sized` (ObjectDef fields residual)
- Wired:
  - `death_polish` place_grave_on_map / non-containable held / rope death target
  - `lib.rs` doBabyHelper droppable held → `place_object_by_id` (allowReplace=false)
  - `lib.rs` bow escape 798 → `place_object_by_id(PlaceObjectOpts::replace())`

### Tests
- `place_object::*` distance formula, kind matrix, ocean/tree, blocked origin, snowingrey
- replace re-home + permanent ring re-home
- grave swallow flat + nested cargo; slots-full reject; sized gate
- existing `death_polish::*` grave tests

### Residual
- ~~ObjectDef `containSize` / `slotSize` not loaded from content~~ → **CLOTHING-CONTAIN-SIZE DONE** (fields + live gates)
- Other Haxe PlaceObject sites: combat wound held-drop / ground arrow, TimeHelper decay/time-transition contained spills, ClearHeldObjectOnground, TransitionHelper fortify/outcomes
- baby bones in arms (Haxe product TODO)

### Matrix
- **PLACE-OBJECT** free_tile_search → DONE (core+gaps)
- **GPI-PLACE-GRAVE** PlaceObject residual closed via place_object
