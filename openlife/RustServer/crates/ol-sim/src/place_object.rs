//! Haxe `WorldMap.PlaceObject` / `TryPlaceObject` / `TransformObject` (**PLACE-OBJECT** / free_tile_search).
//!
//! Free-tile search: expanding random radius, biome blocking, don't-place-behind-tree,
//! grave container swallow, allowReplace non-permanent, optional wall path trim via
//! [`crate::animal_move::calculate_non_blocked_target`].

use crate::animal_move::{calculate_non_blocked_target, is_biome_blocking};
use crate::postload_wire::description_is_orig_grave;
use crate::SimState;
use ol_content::ContentDb;
use ol_world::{name_looks_like_grave, ComplexObject, World};
use rand::Rng;

/// Horse-Drawn Cart (Haxe TransformObject).
pub const HORSE_DRAWN_CART_ID: i32 = 778;
/// Horse-Drawn Tire Cart.
pub const HORSE_DRAWN_TIRE_CART_ID: i32 = 3158;

/// Max free-tile search attempts (Haxe `for (i in 1...10000)`).
pub const PLACE_MAX_ATTEMPTS: i32 = 9999;
/// After this many tries, ignore wall path trim (Haxe `i > 5000`).
pub const PLACE_DROP_WALLS_AFTER: i32 = 5000;

/// Options for [`place_object`] / [`place_object_by_id`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaceObjectOpts {
    /// Haxe `allowReplaceObject` — replace non-permanent occupants.
    pub allow_replace: bool,
    /// Haxe `considerWalls` — path-trim via CalculateNonBlockedTarget from origin.
    pub consider_walls: bool,
}

impl Default for PlaceObjectOpts {
    fn default() -> Self {
        Self {
            allow_replace: false,
            consider_walls: false,
        }
    }
}

impl PlaceObjectOpts {
    pub fn replace() -> Self {
        Self {
            allow_replace: true,
            consider_walls: false,
        }
    }

    /// Haxe grave / non-containable held: `allowReplace=true, considerWalls=true`.
    pub fn grave_or_held() -> Self {
        Self {
            allow_replace: true,
            consider_walls: true,
        }
    }
}

/// Outcome of a successful place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceObjectResult {
    pub x: i32,
    pub y: i32,
    /// Object id that was on the tile before replace (Haxe returns displaced helper).
    pub displaced_id: Option<i32>,
    /// True when existing tile object was pushed into the placed grave's contained.
    pub swallowed_into_grave: bool,
    /// Where the displaced object was re-homed after allowReplace (Haxe continues free search).
    /// // Haxe: WorldMap.PlaceObject — TryPlaceObject returns displaced; loop re-places with allowReplace=false
    pub displaced_rehome: Option<(i32, i32)>,
}

/// Internal TryPlaceObject outcome (placed / need re-home displaced / keep searching).
enum TryPlaceInternal {
    /// `objectToPlace` written; Haxe returns null.
    Placed(PlaceObjectResult),
    /// Placed by replace; Haxe returns displaced helper for continued free-tile search.
    NeedRehome {
        placed: PlaceObjectResult,
        displaced: ComplexObject,
    },
    /// Keep searching (biome / tree / occupied / wall path).
    Fail,
}

// ── Pure helpers ────────────────────────────────────────────────────────────

/// Haxe `ObjectData.isTree` — description contains `"Tree"`.
/// // Haxe: ObjectData.isTree
#[inline]
pub fn is_tree_description(description: &str, name: &str) -> bool {
    description.contains("Tree") || name.contains("Tree")
}

/// Content-backed tree check for object id under `(x, y-1)` south of candidate.
/// // Haxe: getObjectDataAtPosition(x, y-1).isTree()
pub fn is_tree_object(content: &ContentDb, object_id: i32) -> bool {
    if object_id == 0 {
        return false;
    }
    content
        .get(object_id)
        .map(|d| is_tree_description(&d.description, &d.name))
        .unwrap_or(false)
}

/// Haxe `ObjectData.isGrave` — description contains `origGrave`.
/// // Haxe: ObjectData.isGrave
#[inline]
pub fn is_grave_object(content: &ContentDb, object_id: i32) -> bool {
    if object_id == 0 {
        return false;
    }
    if let Some(d) = content.get(object_id) {
        if description_is_orig_grave(&d.description) || d.description.contains("origGrave") {
            return true;
        }
        return name_looks_like_grave(&d.name, &d.description);
    }
    false
}

/// Haxe permanent flag.
#[inline]
pub fn is_permanent_object(content: &ContentDb, object_id: i32) -> bool {
    if object_id == 0 {
        return false;
    }
    content
        .get(object_id)
        .map(|d| d.permanent)
        .unwrap_or(false)
}

/// Haxe `containSize > slotSize` size gate (pure).
/// // Haxe: ObjectHelper.canBePlacedIn containSize/slotSize
#[inline]
pub fn contain_fits_slot(contain_size: f32, slot_size: f32) -> bool {
    contain_size <= slot_size
}

/// Resolve containSize of `item_id` and slotSize of `container_id` from content.
/// // Haxe: ObjectData.containSize / slotSize
#[inline]
pub fn contain_slot_sizes(content: &ContentDb, item_id: i32, container_id: i32) -> (f32, f32) {
    let cs = content
        .get(item_id)
        .map(|d| d.contain_size)
        .unwrap_or(0.0);
    let ss = content
        .get(container_id)
        .map(|d| d.slot_size)
        .unwrap_or(1.0);
    (cs, ss)
}

/// True when `item_id` fits into `container_id` by Haxe containSize/slotSize.
/// Missing content: item defaults 0, container defaults 1 (always fits).
/// // Haxe: ObjectHelper.canBePlacedIn size check
#[inline]
pub fn object_contain_fits_container(content: &ContentDb, item_id: i32, container_id: i32) -> bool {
    let (cs, ss) = contain_slot_sizes(content, item_id, container_id);
    contain_fits_slot(cs, ss)
}

/// Haxe `doTransitionIfPossibleHelper` post-transition fit when USE is on a
/// contained object (`containerSlotSize >= 0`).
///
/// Refuse when `newTarget` is not containable or `containSize > containerSlotSize`.
/// When `container_slot_size < 0` the gate is inactive (normal ground USE).
/// Empty result id 0 always fits (slot clear).
// Haxe: TransitionHelper.doTransitionIfPossibleHelper L1087–1091
#[inline]
pub fn transition_result_fits_container(
    container_slot_size: f32,
    new_target_containable: bool,
    new_target_contain_size: f32,
) -> bool {
    if container_slot_size < 0.0 {
        return true;
    }
    new_target_containable && contain_fits_slot(new_target_contain_size, container_slot_size)
}

/// Content-aware form of [`transition_result_fits_container`].
///
/// Missing `new_target_id` (except 0): treat as not containable when gate active.
// Haxe: TransitionHelper.doTransitionIfPossibleHelper L1087–1091
#[inline]
pub fn transition_result_fits_container_from_content(
    content: &ContentDb,
    container_slot_size: f32,
    new_target_id: i32,
) -> bool {
    if container_slot_size < 0.0 {
        return true;
    }
    // Empty / clear slot always fits (Haxe Empty id 0 is containable after patches).
    if new_target_id == 0 {
        return true;
    }
    match content.get(new_target_id) {
        Some(d) => transition_result_fits_container(
            container_slot_size,
            d.containable,
            d.contain_size,
        ),
        None => false,
    }
}

/// Haxe `ObjectHelper.canBePlacedIn` subset for grave swallow of existing ground id.
///
/// Uses ObjectDef `contain_size` / `slot_size` from content when present.
/// // Haxe: ObjectHelper.canBePlacedIn
pub fn can_be_placed_in_grave(
    content: &ContentDb,
    existing_id: i32,
    grave_id: i32,
    grave_contained_len: usize,
) -> bool {
    let (cs, ss) = contain_slot_sizes(content, existing_id, grave_id);
    can_be_placed_in_grave_sized(
        content,
        existing_id,
        grave_id,
        grave_contained_len,
        Some(cs),
        Some(ss),
    )
}

/// Like [`can_be_placed_in_grave`] with explicit Haxe containSize/slotSize (or `None` to skip).
pub fn can_be_placed_in_grave_sized(
    content: &ContentDb,
    existing_id: i32,
    grave_id: i32,
    grave_contained_len: usize,
    contain_size: Option<f32>,
    slot_size: Option<f32>,
) -> bool {
    if existing_id == 0 {
        return false;
    }
    let Some(exist) = content.get(existing_id) else {
        return false;
    };
    if !exist.containable || exist.permanent {
        return false;
    }
    let slots = content.get(grave_id).map(|g| g.num_slots).unwrap_or(0);
    if slots <= 0 {
        // Content-missing graves (tests): allow swallow of containable non-permanent.
        // Still honour size gate when both sizes provided.
        if let (Some(cs), Some(ss)) = (contain_size, slot_size) {
            return contain_fits_slot(cs, ss);
        }
        return true;
    }
    if grave_contained_len >= slots as usize {
        return false;
    }
    // Haxe: containSize > slotSize → refuse
    if let (Some(cs), Some(ss)) = (contain_size, slot_size) {
        return contain_fits_slot(cs, ss);
    }
    true
}

/// Convert ground tile helper → nested slot for grave swallow (preserves nested cargo).
/// // Haxe: objectToPlace.containedObjects.push(obj) full ObjectHelper
fn complex_as_nested_slot(c: &ComplexObject) -> ol_world::NestedHelper {
    crate::horse_mount::complex_to_nested(c)
}

/// Push full existing helper into grave (flat `contained` + recursive `slots`).
/// // Haxe: WorldMap.TryPlaceObject grave branch containedObjects.push
fn grave_swallow_push(object: &mut ComplexObject, existing: &ComplexObject) {
    if object.slots.is_empty() && !object.contained.is_empty() {
        object.synthesize_slots_from_wire();
    }
    let nest = complex_as_nested_slot(existing);
    object.slots.push(nest);
    object.rebuild_wire_from_slots();
}

/// Haxe `TransformObject` for ground placement of held carts.
///
/// Returns transformed id via transition `(id, -1).newTargetID` when id is 778 or 3158.
/// // Haxe: WorldMap.TransformObject
pub fn transform_placed_object_id(content: &ContentDb, object_id: i32) -> i32 {
    if object_id != HORSE_DRAWN_CART_ID && object_id != HORSE_DRAWN_TIRE_CART_ID {
        return object_id;
    }
    content
        .find_transition(object_id, -1)
        .map(|t| {
            if t.new_target_id != 0 {
                t.new_target_id
            } else {
                object_id
            }
        })
        .unwrap_or(object_id)
}

/// One step of Haxe free-tile distance growth inside the PlaceObject search loop.
///
/// `distance = Math.ceil(i / (20 * distance * distance))`.
/// // Haxe: WorldMap.PlaceObject distance formula
pub fn place_search_distance_step(i: i32, distance: i32) -> i32 {
    let d = distance.max(1) as f64;
    let i = i.max(1) as f64;
    (i / (20.0 * d * d)).ceil() as i32
}

/// Haxe `world.randomInt(distance * 2) - distance` with `randomInt(n) ∈ 0..=n`.
/// // Haxe: WorldMap.randomInt → floor(random * (x+1))
pub fn place_random_offset<R: Rng>(rng: &mut R, distance: i32) -> i32 {
    let dist = distance.max(0);
    let r = rng.gen_range(0..=(dist * 2));
    r - dist
}

/// Pure candidate tile for attempt `i` (1-based) after updating `distance`.
pub fn place_search_candidate<R: Rng>(
    rng: &mut R,
    origin_x: i32,
    origin_y: i32,
    i: i32,
    distance: &mut i32,
) -> (i32, i32) {
    *distance = place_search_distance_step(i, *distance);
    let dx = place_random_offset(rng, *distance);
    let dy = place_random_offset(rng, *distance);
    (origin_x + dx, origin_y + dy)
}

/// Decision for a single TryPlaceObject evaluation (no world mutation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryPlaceKind {
    /// Biome blocks placement — keep searching.
    BiomeBlocked,
    /// considerWalls path fully blocked — keep searching.
    WallPathBlocked,
    /// Object south of tile is a tree — keep searching.
    BehindTree,
    /// Empty tile — place here.
    Empty,
    /// Placing grave and can swallow existing into contained.
    GraveSwallow,
    /// allowReplace and existing non-permanent.
    Replace,
    /// Occupied permanent (or replace disallowed) — keep searching.
    Occupied,
}

/// Pure TryPlaceObject decision at `(x,y)` given existing id and south (y-1) id.
/// // Haxe: WorldMap.TryPlaceObject
pub fn try_place_kind(
    content: &ContentDb,
    biome_blocking: bool,
    existing_id: i32,
    south_id: i32,
    place_id: i32,
    place_is_grave: bool,
    place_contained_len: usize,
    allow_replace: bool,
    wall_path_blocked: bool,
) -> TryPlaceKind {
    if biome_blocking {
        return TryPlaceKind::BiomeBlocked;
    }
    if wall_path_blocked {
        return TryPlaceKind::WallPathBlocked;
    }
    if is_tree_object(content, south_id) {
        return TryPlaceKind::BehindTree;
    }
    if existing_id == 0 {
        return TryPlaceKind::Empty;
    }
    if place_is_grave
        && can_be_placed_in_grave(content, existing_id, place_id, place_contained_len)
    {
        return TryPlaceKind::GraveSwallow;
    }
    if allow_replace && !is_permanent_object(content, existing_id) {
        return TryPlaceKind::Replace;
    }
    TryPlaceKind::Occupied
}

// ── Live PlaceObject ────────────────────────────────────────────────────────

/// Haxe `WorldMap.PlaceObjectById`.
/// // Haxe: WorldMap.PlaceObjectById
pub fn place_object_by_id(
    state: &mut SimState,
    tx: i32,
    ty: i32,
    obj_id: i32,
    opts: PlaceObjectOpts,
) -> Option<PlaceObjectResult> {
    if obj_id == 0 {
        return Some(PlaceObjectResult {
            x: tx,
            y: ty,
            displaced_id: None,
            swallowed_into_grave: false,
            displaced_rehome: None,
        });
    }
    let id = transform_placed_object_id(&state.content, obj_id);
    let co = ComplexObject::new_simple(id);
    place_object(state, tx, ty, co, opts)
}

/// Haxe `WorldMap.PlaceObject` with free-tile search.
///
/// Mutates world; records MX via [`SimState::record_world_change`].
/// On allowReplace, re-homes the displaced helper with a continued free-tile
/// search and `allow_replace=false` (Haxe returns displaced from TryPlaceObject).
/// // Haxe: WorldMap.PlaceObject
pub fn place_object(
    state: &mut SimState,
    tx: i32,
    ty: i32,
    mut object: ComplexObject,
    opts: PlaceObjectOpts,
) -> Option<PlaceObjectResult> {
    let mut rng = rand::thread_rng();
    place_object_with_rng(state, tx, ty, &mut object, opts, &mut rng)
}

/// PlaceObject with injectable RNG (tests).
pub fn place_object_with_rng<R: Rng>(
    state: &mut SimState,
    tx: i32,
    ty: i32,
    object: &mut ComplexObject,
    opts: PlaceObjectOpts,
    rng: &mut R,
) -> Option<PlaceObjectResult> {
    // Transform cart ids on the helper.
    let transformed = transform_placed_object_id(&state.content, object.base_id);
    if transformed != object.base_id {
        object.base_id = transformed;
    }

    let origin_x = tx;
    let origin_y = ty;
    let mut allow_replace = opts.allow_replace;
    let mut consider_walls = opts.consider_walls;
    // Placement of the caller's original object when a displace re-home is in progress.
    let mut primary: Option<PlaceObjectResult> = None;

    // First try origin (Haxe first TryPlaceObject omits considerWalls → false).
    match try_place_object(
        state,
        tx,
        ty,
        object,
        allow_replace,
        false,
        origin_x,
        origin_y,
        rng,
    ) {
        TryPlaceInternal::Placed(res) => {
            if let Some(mut p) = primary.take() {
                p.displaced_rehome = Some((res.x, res.y));
                return Some(p);
            }
            return Some(res);
        }
        TryPlaceInternal::NeedRehome { placed, displaced } => {
            primary = Some(placed);
            *object = displaced;
            allow_replace = false;
        }
        TryPlaceInternal::Fail => {}
    }

    let mut distance = 1_i32;
    for i in 1..=PLACE_MAX_ATTEMPTS {
        // Haxe: if originalObjectToPlace != objectToPlace allowReplaceObject = false
        if primary.is_some() {
            allow_replace = false;
        }

        distance = place_search_distance_step(i, distance);
        let tmp_x = tx + place_random_offset(rng, distance);
        let tmp_y = ty + place_random_offset(rng, distance);

        if i > PLACE_DROP_WALLS_AFTER {
            consider_walls = false;
        }

        match try_place_object(
            state,
            tmp_x,
            tmp_y,
            object,
            allow_replace,
            consider_walls,
            origin_x,
            origin_y,
            rng,
        ) {
            TryPlaceInternal::Placed(res) => {
                if let Some(mut p) = primary {
                    p.displaced_rehome = Some((res.x, res.y));
                    return Some(p);
                }
                return Some(res);
            }
            TryPlaceInternal::NeedRehome { placed, displaced } => {
                // First displace of the original object — continue free search for displaced.
                primary = Some(placed);
                *object = displaced;
                allow_replace = false;
            }
            TryPlaceInternal::Fail => {}
        }
    }

    // Original placed via replace but re-home exhausted attempts (Haxe returns false).
    // Still report primary placement so callers see where the new object landed.
    primary
}

/// Haxe `TryPlaceObject` — place / swallow / replace or fail this candidate.
/// // Haxe: WorldMap.TryPlaceObject
fn try_place_object<R: Rng>(
    state: &mut SimState,
    mut x: i32,
    mut y: i32,
    object: &mut ComplexObject,
    allow_replace: bool,
    consider_walls: bool,
    origin_x: i32,
    origin_y: i32,
    rng: &mut R,
) -> TryPlaceInternal {
    let place_id = object.base_id;
    let place_is_grave = is_grave_object(&state.content, place_id);

    // Snapshot tile state under read lock.
    let (existing_id, south_id, existing_helper, path_xy) = {
        let w = state.world.read().unwrap();
        if is_biome_blocking(&w, x, y) {
            return TryPlaceInternal::Fail;
        }

        let mut px = x;
        let mut py = y;
        if consider_walls {
            match calculate_non_blocked_target(
                &w,
                &state.content,
                rng,
                origin_x,
                origin_y,
                x,
                y,
                false,
            ) {
                Some((nx, ny)) => {
                    px = nx;
                    py = ny;
                }
                None => {
                    // Fully blocked path — Haxe returns objectToPlace (fail try).
                    // Same-tile origin: calculate_non_blocked returns None (no step) → allow.
                    if x != origin_x || y != origin_y {
                        return TryPlaceInternal::Fail;
                    }
                }
            }
        }

        x = px;
        y = py;
        let existing_id = w.get_object(x, y);
        let south_id = w.get_object(x, y - 1);
        let existing_helper = w.get_helper(x, y).cloned();
        (existing_id, south_id, existing_helper, (x, y))
    };

    let (x, y) = path_xy;
    let kind = try_place_kind(
        &state.content,
        false,
        existing_id,
        south_id,
        place_id,
        place_is_grave,
        object.contained.len(),
        allow_replace,
        false,
    );

    match kind {
        TryPlaceKind::Empty => {
            write_placed(state, x, y, object);
            TryPlaceInternal::Placed(PlaceObjectResult {
                x,
                y,
                displaced_id: None,
                swallowed_into_grave: false,
                displaced_rehome: None,
            })
        }
        TryPlaceKind::GraveSwallow => {
            let existing = existing_helper
                .unwrap_or_else(|| ComplexObject::new_simple(existing_id));
            // Haxe: containedObjects.push(full ObjectHelper) — preserve nested/state.
            grave_swallow_push(object, &existing);
            write_placed(state, x, y, object);
            TryPlaceInternal::Placed(PlaceObjectResult {
                x,
                y,
                displaced_id: Some(existing_id),
                swallowed_into_grave: true,
                displaced_rehome: None,
            })
        }
        TryPlaceKind::Replace => {
            // Haxe: setObjectHelper new; return displaced obj for free-tile re-home.
            let displaced = existing_helper
                .unwrap_or_else(|| ComplexObject::new_simple(existing_id));
            write_placed(state, x, y, object);
            TryPlaceInternal::NeedRehome {
                placed: PlaceObjectResult {
                    x,
                    y,
                    displaced_id: Some(existing_id),
                    swallowed_into_grave: false,
                    displaced_rehome: None,
                },
                displaced,
            }
        }
        TryPlaceKind::BiomeBlocked
        | TryPlaceKind::WallPathBlocked
        | TryPlaceKind::BehindTree
        | TryPlaceKind::Occupied => TryPlaceInternal::Fail,
    }
}

fn write_placed(state: &mut SimState, x: i32, y: i32, object: &ComplexObject) {
    let id = object.base_id;
    {
        let mut w = state.world.write().unwrap();
        w.set_object_complex(x, y, object.clone());
    }
    state.record_world_change(x, y, id);
}

/// Convenience: place flat id near origin with free search (no walls).
/// Replaces lite ring search used by older death_polish helpers.
pub fn place_object_near(
    state: &mut SimState,
    cx: i32,
    cy: i32,
    object_id: i32,
    allow_replace: bool,
) -> Option<(i32, i32)> {
    let opts = PlaceObjectOpts {
        allow_replace,
        consider_walls: false,
    };
    place_object_by_id(state, cx, cy, object_id, opts).map(|r| (r.x, r.y))
}

/// Haxe `WorldMap.PlaceObject(tx, ty, grave, true, true)`.
pub fn place_complex_object(
    state: &mut SimState,
    tx: i32,
    ty: i32,
    object: ComplexObject,
    opts: PlaceObjectOpts,
) -> Option<PlaceObjectResult> {
    place_object(state, tx, ty, object, opts)
}

/// Place grave complex with free search + walls (Haxe placeGrave).
pub fn place_grave_object(
    state: &mut SimState,
    cx: i32,
    cy: i32,
    grave: ComplexObject,
) -> Option<(i32, i32)> {
    place_complex_object(state, cx, cy, grave, PlaceObjectOpts::grave_or_held())
        .map(|r| (r.x, r.y))
}

/// World-only try for pure tests (biome + tree + empty/replace) without SimState.
pub fn try_place_flat_on_world(
    world: &mut World,
    content: &ContentDb,
    x: i32,
    y: i32,
    place_id: i32,
    allow_replace: bool,
) -> bool {
    if is_biome_blocking(world, x, y) {
        return false;
    }
    let south = world.get_object(x, y - 1);
    if is_tree_object(content, south) {
        return false;
    }
    let existing = world.get_object(x, y);
    let kind = try_place_kind(
        content,
        false,
        existing,
        south,
        place_id,
        is_grave_object(content, place_id),
        0,
        allow_replace,
        false,
    );
    match kind {
        TryPlaceKind::Empty | TryPlaceKind::Replace | TryPlaceKind::GraveSwallow => {
            world.set_object(x, y, place_id);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::{ObjectDef, Transition};
    use ol_world::{World, OCEAN, SNOWINGREY};
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::sync::Arc;

    fn def(id: i32, name: &str, desc: &str, permanent: bool, containable: bool) -> ObjectDef {
        let mut o = ObjectDef::empty(id);
        o.name = name.into();
        o.description = desc.into();
        o.permanent = permanent;
        o.containable = containable;
        o.num_slots = if name.contains("Grave") || desc.contains("origGrave") {
            6
        } else {
            0
        };
        o
    }

    #[test]
    fn place_search_distance_grows_slowly() {
        // Haxe: distance = ceil(i / (20 * distance * distance)) — can oscillate
        // (e.g. d=1→2 at i=21, then d=2→1 at i=22). Check formula edges, not mono growth.
        let mut d = 1;
        for i in 1..=20 {
            d = place_search_distance_step(i, d);
            assert_eq!(d, 1, "i={i}");
        }
        assert_eq!(place_search_distance_step(21, 1), 2);
        assert_eq!(place_search_distance_step(22, 2), 1);
        // With fixed d=1, radius steps up with attempt index.
        assert_eq!(place_search_distance_step(40, 1), 2);
        assert_eq!(place_search_distance_step(41, 1), 3);
        // Simulated loop reaches d≥2 at least once in first 50 attempts.
        d = 1;
        let mut saw_ge2 = false;
        for i in 1..=50 {
            d = place_search_distance_step(i, d);
            if d >= 2 {
                saw_ge2 = true;
            }
        }
        assert!(saw_ge2, "expected distance ≥2 at some attempt ≤50");
    }

    #[test]
    fn place_random_offset_in_range() {
        let mut rng = StdRng::seed_from_u64(42);
        for dist in [0, 1, 2, 5, 10] {
            for _ in 0..50 {
                let o = place_random_offset(&mut rng, dist);
                assert!(o >= -dist && o <= dist, "offset {o} out of ±{dist}");
            }
        }
    }

    #[test]
    fn is_tree_and_grave_pure() {
        assert!(is_tree_description("Maple Tree", ""));
        assert!(is_tree_description("", "Oak Tree"));
        assert!(!is_tree_description("Bush", "berry"));
        assert!(description_is_orig_grave("Stone +origGrave"));
    }

    #[test]
    fn try_place_kind_matrix() {
        let mut db = ContentDb::default();
        db.objects
            .insert(1, def(1, "Stone", "rock", false, false));
        db.objects
            .insert(2, def(2, "Wall", "stone wall", true, false));
        db.objects
            .insert(33, def(33, "Berry", "food", false, true));
        db.objects
            .insert(87, def(87, "Fresh Grave", "+origGrave", true, false));
        db.objects
            .insert(100, def(100, "Maple Tree", "Maple Tree", true, false));

        assert_eq!(
            try_place_kind(&db, true, 0, 0, 1, false, 0, false, false),
            TryPlaceKind::BiomeBlocked
        );
        assert_eq!(
            try_place_kind(&db, false, 0, 0, 1, false, 0, false, false),
            TryPlaceKind::Empty
        );
        assert_eq!(
            try_place_kind(&db, false, 0, 100, 1, false, 0, false, false),
            TryPlaceKind::BehindTree
        );
        assert_eq!(
            try_place_kind(&db, false, 2, 0, 1, false, 0, true, false),
            TryPlaceKind::Occupied
        );
        assert_eq!(
            try_place_kind(&db, false, 1, 0, 1, false, 0, true, false),
            TryPlaceKind::Replace
        );
        assert_eq!(
            try_place_kind(&db, false, 33, 0, 87, true, 0, true, false),
            TryPlaceKind::GraveSwallow
        );
    }

    #[test]
    fn transform_horse_cart_via_transition() {
        let mut db = ContentDb::default();
        db.objects
            .insert(778, def(778, "Horse-Drawn Cart", "cart", false, false));
        db.objects.insert(
            1422,
            def(1422, "Horse-Drawn Cart", "ground cart", true, false),
        );
        db.transitions.insert(
            (778, -1),
            Transition {
                actor_id: 778,
                target_id: -1,
                new_actor_id: 0,
                new_target_id: 1422,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,
                desired_move_dist: 0,
                actor_min_use_fraction: 0.0,
                target_min_use_fraction: 0.0,
                switch_number_of_uses: false,
                target_number_of_uses: -1,
                is_pickup_or_drop: false,
            },
        );
        assert_eq!(transform_placed_object_id(&db, 778), 1422);
        assert_eq!(transform_placed_object_id(&db, 33), 33);
    }

    #[test]
    fn try_place_flat_skips_ocean_biome() {
        let mut db = ContentDb::default();
        db.objects
            .insert(1, def(1, "Stone", "rock", false, false));
        let mut w = World::new(32, 32, false);
        w.ensure_full_map_chunks();
        w.set_biome(5, 5, OCEAN);
        assert!(!try_place_flat_on_world(&mut w, &db, 5, 5, 1, false));
        w.set_biome(6, 6, 0); // GREEN
        assert!(try_place_flat_on_world(&mut w, &db, 6, 6, 1, false));
        assert_eq!(w.get_object(6, 6), 1);
    }

    #[test]
    fn try_place_flat_skips_behind_tree() {
        let mut db = ContentDb::default();
        db.objects
            .insert(1, def(1, "Stone", "rock", false, false));
        db.objects
            .insert(100, def(100, "Maple Tree", "Maple Tree", true, false));
        let mut w = World::new(32, 32, false);
        w.ensure_full_map_chunks();
        w.set_object(4, 3, 100); // south of (4,4)
        assert!(!try_place_flat_on_world(&mut w, &db, 4, 4, 1, false));
        w.set_object(4, 3, 0);
        assert!(try_place_flat_on_world(&mut w, &db, 4, 4, 1, false));
    }

    #[test]
    fn live_place_finds_empty_when_origin_blocked() {
        let mut db = ContentDb::default();
        db.objects
            .insert(50, def(50, "Held Item", "tool", false, false));
        for id in 200..210 {
            db.objects
                .insert(id, def(id, "Wall", "stone wall", true, false));
        }
        let mut state = SimState::with_default_empty(Arc::new(db));
        {
            let mut w = state.world.write().unwrap();
            for dy in -1..=1 {
                for dx in -1..=1 {
                    w.set_object(10 + dx, 10 + dy, 200);
                }
            }
        }
        let mut rng = StdRng::seed_from_u64(7);
        let mut obj = ComplexObject::new_simple(50);
        let res = place_object_with_rng(
            &mut state,
            10,
            10,
            &mut obj,
            PlaceObjectOpts::default(),
            &mut rng,
        )
        .expect("should find free tile outside permanent block");
        assert_ne!((res.x, res.y), (10, 10));
        assert_eq!(state.world.read().unwrap().get_object(res.x, res.y), 50);
    }

    #[test]
    fn live_place_skips_snowingrey_until_elsewhere() {
        let mut db = ContentDb::default();
        db.objects
            .insert(50, def(50, "Item", "x", false, false));
        let mut state = SimState::with_default_empty(Arc::new(db));
        {
            let mut w = state.world.write().unwrap();
            w.set_biome(0, 0, SNOWINGREY);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    w.set_biome(dx, dy, SNOWINGREY);
                }
            }
            w.set_biome(3, 0, 0); // GREEN free
        }
        let mut rng = StdRng::seed_from_u64(99);
        let mut obj = ComplexObject::new_simple(50);
        let res = place_object_with_rng(
            &mut state,
            0,
            0,
            &mut obj,
            PlaceObjectOpts::default(),
            &mut rng,
        )
        .expect("find non-blocking biome");
        assert!(!is_biome_blocking(
            &state.world.read().unwrap(),
            res.x,
            res.y
        ));
        assert_eq!(state.world.read().unwrap().get_object(res.x, res.y), 50);
    }

    #[test]
    fn live_grave_swallows_containable() {
        let mut db = ContentDb::default();
        db.objects
            .insert(87, def(87, "Fresh Grave", "+origGrave", true, false));
        db.objects
            .insert(33, def(33, "Berry", "food", false, true));
        let mut state = SimState::with_default_empty(Arc::new(db));
        state.world.write().unwrap().set_object(5, 5, 33);
        let mut grave = ComplexObject::new_simple(87);
        let mut rng = StdRng::seed_from_u64(1);
        let res = place_object_with_rng(
            &mut state,
            5,
            5,
            &mut grave,
            PlaceObjectOpts::grave_or_held(),
            &mut rng,
        )
        .expect("place grave");
        assert!(res.swallowed_into_grave);
        let h = state
            .world
            .read()
            .unwrap()
            .get_helper(res.x, res.y)
            .cloned()
            .expect("helper");
        assert_eq!(h.base_id, 87);
        assert!(h.contained.contains(&33));
    }

    #[test]
    fn live_allow_replace_non_permanent() {
        let mut db = ContentDb::default();
        db.objects
            .insert(10, def(10, "Stick", "wood", false, false));
        db.objects
            .insert(20, def(20, "Stone", "rock", false, false));
        let mut state = SimState::with_default_empty(Arc::new(db));
        state.world.write().unwrap().set_object(1, 1, 10);
        let res = place_object_by_id(&mut state, 1, 1, 20, PlaceObjectOpts::replace())
            .expect("replace");
        assert_eq!((res.x, res.y), (1, 1));
        assert_eq!(res.displaced_id, Some(10));
        assert_eq!(state.world.read().unwrap().get_object(1, 1), 20);
        // Displaced stick re-homed onto a free neighbor (not deleted).
        // Haxe: TryPlaceObject returns displaced; PlaceObject continues free search.
        let rehome = res.displaced_rehome.expect("displaced re-home");
        assert_ne!(rehome, (1, 1));
        assert_eq!(state.world.read().unwrap().get_object(rehome.0, rehome.1), 10);
    }

    #[test]
    fn live_allow_replace_rehomes_when_surrounded_by_permanents() {
        // Permanent ring around origin with non-permanent underfoot + one free tile outside.
        let mut db = ContentDb::default();
        db.objects
            .insert(10, def(10, "Stick", "wood", false, false));
        db.objects
            .insert(20, def(20, "Stone", "rock", false, false));
        for id in 200..210 {
            db.objects
                .insert(id, def(id, "Wall", "stone wall", true, false));
        }
        let mut state = SimState::with_default_empty(Arc::new(db));
        {
            let mut w = state.world.write().unwrap();
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        w.set_object(10, 10, 10); // non-permanent underfoot
                    } else {
                        w.set_object(10 + dx, 10 + dy, 200);
                    }
                }
            }
            // Free tile outside ring for re-home / if origin replace needs nowhere... origin is replaceable.
        }
        let mut rng = StdRng::seed_from_u64(11);
        let mut obj = ComplexObject::new_simple(20);
        let res = place_object_with_rng(
            &mut state,
            10,
            10,
            &mut obj,
            PlaceObjectOpts::replace(),
            &mut rng,
        )
        .expect("place with replace");
        assert_eq!((res.x, res.y), (10, 10));
        assert_eq!(res.displaced_id, Some(10));
        assert_eq!(state.world.read().unwrap().get_object(10, 10), 20);
        let rehome = res.displaced_rehome.expect("stick re-homed outside ring");
        assert_eq!(state.world.read().unwrap().get_object(rehome.0, rehome.1), 10);
        // Re-home not on a permanent wall tile.
        assert_ne!(state.world.read().unwrap().get_object(rehome.0, rehome.1), 200);
    }

    #[test]
    fn live_grave_swallows_nested_contained() {
        // Haxe: containedObjects.push(full ObjectHelper) — nested cargo preserved.
        let mut db = ContentDb::default();
        db.objects
            .insert(87, def(87, "Fresh Grave", "+origGrave", true, false));
        db.objects
            .insert(391, def(391, "Basket", "basket", false, true));
        db.objects
            .insert(33, def(33, "Berry", "food", false, true));
        db.objects
            .insert(40, def(40, "Carrot", "food", false, true));
        let mut state = SimState::with_default_empty(Arc::new(db));
        {
            let mut basket = ComplexObject::new_simple(391);
            basket.contained = vec![33, 40];
            basket.nested = vec![vec![100], vec![]];
            basket.synthesize_slots_from_wire();
            state
                .world
                .write()
                .unwrap()
                .set_object_complex(5, 5, basket);
        }
        let mut grave = ComplexObject::new_simple(87);
        let mut rng = StdRng::seed_from_u64(3);
        let res = place_object_with_rng(
            &mut state,
            5,
            5,
            &mut grave,
            PlaceObjectOpts::grave_or_held(),
            &mut rng,
        )
        .expect("place grave");
        assert!(res.swallowed_into_grave);
        let h = state
            .world
            .read()
            .unwrap()
            .get_helper(res.x, res.y)
            .cloned()
            .expect("grave helper");
        assert_eq!(h.base_id, 87);
        assert!(h.contained.contains(&391));
        // Nested basket contents survive on grave.slots[0].contained
        let slot = h
            .slots
            .iter()
            .find(|s| s.id == 391)
            .expect("basket slot");
        let nest_ids: Vec<i32> = slot.contained.iter().map(|c| c.id).collect();
        assert_eq!(nest_ids, vec![33, 40]);
        // One level of nest under berry (100) preserved in NestedHelper tree.
        assert_eq!(
            slot.contained[0]
                .contained
                .iter()
                .map(|c| c.id)
                .collect::<Vec<_>>(),
            vec![100]
        );
    }

    #[test]
    fn live_grave_reject_when_slots_full() {
        let mut db = ContentDb::default();
        let mut gdef = def(87, "Fresh Grave", "+origGrave", true, false);
        gdef.num_slots = 2;
        db.objects.insert(87, gdef);
        db.objects
            .insert(33, def(33, "Berry", "food", false, true));
        db.objects
            .insert(34, def(34, "Berry2", "food", false, true));
        let mut state = SimState::with_default_empty(Arc::new(db));
        state.world.write().unwrap().set_object(5, 5, 34);
        let mut grave = ComplexObject::new_simple(87);
        grave.contained = vec![33, 33]; // already full (num_slots=2)
        let mut rng = StdRng::seed_from_u64(5);
        let res = place_object_with_rng(
            &mut state,
            5,
            5,
            &mut grave,
            PlaceObjectOpts::grave_or_held(),
            &mut rng,
        )
        .expect("grave places elsewhere or via replace");
        // Origin occupied non-permanent → allowReplace can place grave and re-home 34
        // OR if replace path: grave at origin, 34 re-homed — not swallowed.
        if res.swallowed_into_grave {
            panic!("should not swallow when slots full");
        }
        // Grave on map somewhere
        assert_eq!(state.world.read().unwrap().get_object(res.x, res.y), 87);
    }

    #[test]
    fn contain_fits_slot_and_sized_gate() {
        assert!(contain_fits_slot(1.0, 1.0));
        assert!(contain_fits_slot(0.5, 1.0));
        assert!(!contain_fits_slot(2.0, 1.0));
        let mut db = ContentDb::default();
        db.objects
            .insert(33, def(33, "Berry", "food", false, true));
        db.objects
            .insert(87, def(87, "Fresh Grave", "+origGrave", true, false));
        assert!(can_be_placed_in_grave_sized(
            &db, 33, 87, 0, Some(1.0), Some(1.0)
        ));
        assert!(!can_be_placed_in_grave_sized(
            &db, 33, 87, 0, Some(2.0), Some(1.0)
        ));
        // Full slots
        assert!(!can_be_placed_in_grave(&db, 33, 87, 6));
    }

    /// CLOTHING-CONTAIN-SIZE: ObjectDef contain_size/slot_size wire into grave gate.
    #[test]
    fn contain_size_from_object_def_blocks_grave_swallow() {
        let mut db = ContentDb::default();
        let mut berry = def(33, "Berry", "food", false, true);
        berry.contain_size = 2.0;
        db.objects.insert(33, berry);
        let mut grave = def(87, "Fresh Grave", "+origGrave", true, false);
        grave.slot_size = 1.0;
        db.objects.insert(87, grave);
        // Defaults would fit (0<=1); oversized contain_size refuses.
        assert!(!can_be_placed_in_grave(&db, 33, 87, 0));
        assert!(!object_contain_fits_container(&db, 33, 87));
        // Equal sizes fit.
        db.objects.get_mut(&33).unwrap().contain_size = 1.0;
        assert!(can_be_placed_in_grave(&db, 33, 87, 0));
        assert!(object_contain_fits_container(&db, 33, 87));
        let (cs, ss) = contain_slot_sizes(&db, 33, 87);
        assert!((cs - 1.0).abs() < 1e-5);
        assert!((ss - 1.0).abs() < 1e-5);
    }

    /// CLOTHING-CONTAIN-SIZE: USE-on-container post-transition result fit (L1087).
    // Haxe: TransitionHelper.doTransitionIfPossibleHelper L1087–1091
    #[test]
    fn transition_result_fits_container_gate() {
        // Inactive when container_slot_size < 0
        assert!(transition_result_fits_container(-1.0, false, 99.0));
        // Active: must be containable and size-fit
        assert!(transition_result_fits_container(1.0, true, 1.0));
        assert!(transition_result_fits_container(2.0, true, 1.5));
        assert!(!transition_result_fits_container(1.0, false, 0.0));
        assert!(!transition_result_fits_container(1.0, true, 2.0));

        let mut db = ContentDb::default();
        let mut ok = def(10, "Ok", "item", false, true);
        ok.contain_size = 1.0;
        db.objects.insert(10, ok);
        let mut big = def(11, "Big", "item", false, true);
        big.contain_size = 3.0;
        db.objects.insert(11, big);
        let mut perm = def(12, "Perm", "item", true, false);
        perm.contain_size = 0.5;
        db.objects.insert(12, perm);

        assert!(transition_result_fits_container_from_content(&db, -1.0, 11));
        assert!(transition_result_fits_container_from_content(&db, 1.0, 0)); // clear
        assert!(transition_result_fits_container_from_content(&db, 1.0, 10));
        assert!(!transition_result_fits_container_from_content(&db, 1.0, 11));
        assert!(!transition_result_fits_container_from_content(&db, 1.0, 12)); // not containable
        assert!(!transition_result_fits_container_from_content(&db, 1.0, 999)); // missing
    }
}
