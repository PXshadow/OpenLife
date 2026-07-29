//! Horse mount / dismount helpers (Haxe TransitionHelper horse paths).
//!
//! Chunk: **TH-HORSE** / **HORSE-MOUNT** / **HORSE-MOUNT-POLISH** (`hitch_cart`)
//! Anchors: `doHorseStuffPossible`, `swapHandAndFloorObject`, `isHorseDropTrans`,
//! `isPickupOrDrop` nest swap, empty-ground `held+-1` dismount,
//! hitched cart hitch/unhitch, grave-basket `isPickupOrDrop`.

use ol_content::{ContentDb, Transition};
use ol_world::{ComplexObject, NestedHelper};

/// Riding Horse.
pub const RIDING_HORSE: i32 = 770;
/// Horse-Drawn Cart.
pub const HORSE_CART: i32 = 778;
/// Horse-Drawn Tire Cart.
pub const HORSE_TIRE_CART: i32 = 3158;
/// Escaped Riding Horse (just released).
pub const ESCAPED_RIDING_HORSE: i32 = 1421;
/// Escaped Horse-Drawn Cart (just released).
pub const ESCAPED_HORSE_CART: i32 = 1422;
/// Escaped Horse-Drawn Tire Cart (just released).
pub const ESCAPED_TIRE_CART: i32 = 3161;
/// Hitched Horse-Drawn Cart.
// Haxe: 778 + 4154 = 0 + 779; 0 + 779 = 778 + 4154
pub const HITCHED_HORSE_CART: i32 = 779;
/// Hitched Horse-Drawn Tire Cart.
// Haxe: PatchTransitions 3158+4154 → newTarget 3159
pub const HITCHED_TIRE_CART: i32 = 3159;
/// Hitching Post.
pub const HITCHING_POST: i32 = 4154;
/// Fence (also hitch target for tire cart patch).
pub const HITCH_FENCE: i32 = 550;
/// Basket (grave scoop / nest-swap parent).
// Haxe: TransitionHelper L1323 parentId 292; PatchTransitions 292+grave
pub const BASKET: i32 = 292;
/// Basket of Bones (put-down isPickupOrDrop).
pub const BASKET_OF_BONES: i32 = 356;
/// Fresh Grave / Grave / Old Grave / Bone Pile (basket nest keys).
pub const FRESH_GRAVE: i32 = 87;
pub const GRAVE: i32 = 88;
pub const OLD_GRAVE: i32 = 89;
pub const BONE_PILE: i32 = 357;
/// Psilocybe Mushroom.
pub const PSILOCYBE: i32 = 837;
/// Wormless Soil Pit with Mushroom.
pub const MUSHROOM_PIT: i32 = 838;

/// Held ids that allow eat-while-mounted (Haxe `doHorseStuffPossible` gate).
// Haxe: TransitionHelper.doHorseStuffPossible
#[inline]
pub fn is_horse_mount_held(held_id: i32) -> bool {
    matches!(held_id, RIDING_HORSE | HORSE_CART | HORSE_TIRE_CART)
}

/// Drawn cart held ids that hitch onto a post/fence (not bare riding horse).
// Haxe: 778+4154→779, 3158+4154→3159 (patched)
#[inline]
pub fn is_horse_cart_held(held_id: i32) -> bool {
    matches!(held_id, HORSE_CART | HORSE_TIRE_CART)
}

/// Tile / result ids that are hitched carts (unhitch via empty-hand USE).
#[inline]
pub fn is_hitched_cart(object_id: i32) -> bool {
    matches!(object_id, HITCHED_HORSE_CART | HITCHED_TIRE_CART)
}

/// Hitching post or fence (Haxe tire-cart fence patch uses 550).
#[inline]
pub fn is_hitch_anchor(object_id: i32) -> bool {
    matches!(object_id, HITCHING_POST | HITCH_FENCE)
}

/// Expected hitched result id for a held cart + hitch anchor (content may override).
///
/// Defaults match vanilla + `apply_default_horse_transition_patches`:
/// - 778 + * → 779
/// - 3158 + * → 3159
// Haxe: TransitionHelper comments + ServerSettings PatchTransitions tire hitch
#[inline]
pub fn default_hitched_id_for_cart(held_cart_id: i32) -> Option<i32> {
    match held_cart_id {
        HORSE_CART => Some(HITCHED_HORSE_CART),
        HORSE_TIRE_CART => Some(HITCHED_TIRE_CART),
        _ => None,
    }
}

/// Expected unhitched held cart id for a hitched tile object.
#[inline]
pub fn default_cart_id_from_hitched(hitched_id: i32) -> Option<i32> {
    match hitched_id {
        HITCHED_HORSE_CART => Some(HORSE_CART),
        HITCHED_TIRE_CART => Some(HORSE_TIRE_CART),
        _ => None,
    }
}

/// Grave / bone-pile targets used with basket `is_pickup_or_drop` patches.
// Haxe: ServerSettings.PatchTransitions graves block
#[inline]
pub fn is_grave_basket_target(target_id: i32) -> bool {
    matches!(
        target_id,
        FRESH_GRAVE | GRAVE | OLD_GRAVE | BONE_PILE
    )
}

/// Haxe `ObjectData.isDrugs` — blocked while mounted.
// Haxe: ObjectData.isDrugs
#[inline]
pub fn is_drugs(object_id: i32) -> bool {
    matches!(object_id, PSILOCYBE | MUSHROOM_PIT)
}

/// Haxe `isHorseDropTrans`: held has cargo and transition empties the hand.
// Haxe: TransitionHelper.doTransitionIfPossibleHelper (~1317)
#[inline]
pub fn is_horse_drop_trans(
    change_held: bool,
    new_actor_id: i32,
    held_contained_count: usize,
) -> bool {
    change_held && new_actor_id == 0 && held_contained_count > 0
}

/// Whether USE should nest-swap tile helper ↔ held helper.
// Haxe: isPickupOrDrop || isHorseDropTrans
#[inline]
pub fn should_nest_swap_helpers(
    is_pickup_or_drop: bool,
    change_held: bool,
    new_actor_id: i32,
    held_contained_count: usize,
) -> bool {
    is_pickup_or_drop
        || is_horse_drop_trans(change_held, new_actor_id, held_contained_count)
}

/// Haxe: Basket (292) with cargo must not change held via a normal transition.
///
/// Refuse when `change_held` and held parent is basket with any contained items.
/// Grave scoop uses `is_pickup_or_drop` only with empty basket (cargo would refuse here first).
// Haxe: TransitionHelper.doTransitionIfPossibleHelper L1322–1326
#[inline]
pub fn basket_refuse_if_changing_held(
    held_parent_id: i32,
    held_contained_count: usize,
    change_held: bool,
) -> bool {
    change_held && held_parent_id == BASKET && held_contained_count > 0
}

/// Slot gate before horse pickup/drop (Haxe "empty first").
///
/// When `is_pickup_or_drop`, compare target contained count to **new actor** slots;
/// otherwise to **new target** slots.
// Haxe: TransitionHelper.doTransitionIfPossibleHelper (~1139–1153)
pub fn pickup_or_drop_slots_ok(
    is_pickup_or_drop: bool,
    target_contained: usize,
    new_actor_num_slots: i32,
    new_target_num_slots: i32,
) -> bool {
    let slots = if is_pickup_or_drop {
        new_actor_num_slots
    } else {
        new_target_num_slots
    };
    (target_contained as i32) <= slots.max(0)
}

/// Put-down ground transform: held + -1 (or held + 0) with `newActorID==0`.
///
/// Returns the escaped/ground id to place (e.g. 770→1421, 778→1422, 3158→3161).
// Haxe: TransitionHelper.swapHandAndFloorObject (~683–698)
pub fn put_down_ground_id(content: &ContentDb, held_id: i32) -> Option<i32> {
    if held_id <= 0 {
        return None;
    }
    let tr = content
        .find_transition(held_id, -1)
        .or_else(|| content.find_transition(held_id, 0))?;
    if tr.new_actor_id == 0 && tr.new_target_id != 0 {
        Some(tr.new_target_id)
    } else {
        None
    }
}

/// Empty-ground dismount transition: held + -1 (reject when newTargetID==0).
///
/// Also tries held + 0 (ServerSettings synthetic riding-horse path).
// Haxe: TransitionHelper.doTransitionIfPossibleHelper (~948–957)
pub fn empty_ground_dismount_transition(
    content: &ContentDb,
    held_id: i32,
) -> Option<&Transition> {
    if held_id <= 0 {
        return None;
    }
    if let Some(tr) = content.find_transition(held_id, -1) {
        if tr.new_target_id != 0 {
            return Some(tr);
        }
        // Haxe: if newTargetID==0 discard (e.g. clay bowl time-style).
    }
    if let Some(tr) = content.find_transition(held_id, 0) {
        if tr.new_actor_id == 0 && tr.new_target_id != 0 {
            return Some(tr);
        }
    }
    None
}

/// Resolve whether a mounted player can attempt horse-eat on `target_id`.
///
/// Returns `Some((food_id, via_transition, new_target_id))`:
/// - `via_transition=false`: eat the tile object itself (`food_value>0`)
/// - `via_transition=true`: bare-hand 0+target yields food as new actor
// Haxe: TransitionHelper.doHorseStuffPossible
pub fn horse_eat_plan(
    content: &ContentDb,
    held_id: i32,
    target_id: i32,
    target_is_last_use: bool,
) -> Option<(i32, bool, i32)> {
    if !is_horse_mount_held(held_id) || target_id <= 0 {
        return None;
    }
    if is_drugs(target_id) {
        return None;
    }
    let food_val = content.get(target_id).map(|d| d.food_value).unwrap_or(0);
    if food_val >= 1 {
        return Some((target_id, false, 0));
    }
    // Prefer last-use harvest when target is exhausted multi-use.
    let tr = if target_is_last_use {
        content
            .find_transition_prefer(0, target_id, true)
            .or_else(|| content.find_transition(0, target_id))
    } else {
        content.find_transition(0, target_id)
    }?;
    let new_actor = tr.new_actor_id;
    if new_actor <= 0 {
        return None;
    }
    let actor_food = content.get(new_actor).map(|d| d.food_value).unwrap_or(0);
    if actor_food < 1 {
        return None;
    }
    Some((new_actor, true, tr.new_target_id))
}

/// Convert world tile helper → held NestedHelper (preserves cargo).
pub fn complex_to_nested(c: &ComplexObject) -> NestedHelper {
    let mut h = NestedHelper::with_uses(c.base_id, c.uses_remaining);
    h.creation_time = c.creation_time;
    h.time_to_change = c.time_to_change;
    h.hits = c.hits;
    h.coins = c.coins;
    h.text = c.text.clone();
    h.extern_id = c.extern_id;
    h.count_obj = c.count_obj;
    h.living_owners = c.living_owners.clone();
    h.owners_by_account = c.owners_by_account.clone();
    if !c.slots.is_empty() {
        h.contained = c.slots.clone();
    } else if !c.contained.is_empty() {
        h.contained = c
            .contained
            .iter()
            .enumerate()
            .map(|(i, &id)| {
                let nest = c.nested.get(i).map(|v| v.as_slice()).unwrap_or(&[]);
                NestedHelper::from_wire(id, nest)
            })
            .collect();
    }
    h
}

/// Convert held NestedHelper → world ComplexObject; stamps `creation_time` to `sim_time`.
// Haxe: creationTimeInTicks = TimeHelper.tick (prevents instant escape decay)
pub fn nested_to_complex(n: &NestedHelper, sim_time: f32) -> ComplexObject {
    let mut c = if n.uses_remaining > 0 {
        ComplexObject::with_uses(n.id, n.uses_remaining)
    } else {
        ComplexObject::new_simple(n.id)
    };
    c.creation_time = sim_time;
    c.time_to_change = n.time_to_change;
    c.hits = n.hits;
    c.coins = n.coins;
    c.text = n.text.clone();
    c.extern_id = n.extern_id;
    c.count_obj = n.count_obj;
    c.living_owners = n.living_owners.clone();
    c.owners_by_account = n.owners_by_account.clone();
    if let Some(&oid) = c.living_owners.first() {
        c.owner_id = oid;
    }
    if !n.contained.is_empty() {
        c.slots = n.contained.clone();
        c.rebuild_wire_from_slots();
    }
    c
}

/// Flat held id → NestedHelper (no cargo).
pub fn held_as_nested(held_id: i32, held_uses: i32, helper: Option<&NestedHelper>) -> NestedHelper {
    if let Some(h) = helper {
        if h.id == held_id || held_id == 0 {
            return h.clone();
        }
    }
    if held_id == 0 {
        NestedHelper::empty()
    } else {
        NestedHelper::with_uses(held_id, held_uses)
    }
}

/// Tile helper or bare id → NestedHelper.
pub fn tile_as_nested(
    target_id: i32,
    uses_remaining: i32,
    helper: Option<&ComplexObject>,
) -> NestedHelper {
    if let Some(h) = helper {
        if h.base_id == target_id || target_id == 0 {
            return complex_to_nested(h);
        }
    }
    if target_id == 0 {
        NestedHelper::empty()
    } else {
        NestedHelper::with_uses(target_id, uses_remaining)
    }
}

/// After nest swap, transform each side's id to transition results (keep contained).
pub fn apply_ids_after_nest_swap(
    mut held: NestedHelper,
    mut tile: NestedHelper,
    new_actor_id: i32,
    new_target_id: i32,
    sim_time: f32,
) -> (NestedHelper, NestedHelper) {
    held.id = new_actor_id;
    if new_actor_id == 0 {
        held = NestedHelper::empty();
    } else {
        held.creation_time = sim_time;
    }
    tile.id = new_target_id;
    if new_target_id == 0 {
        tile = NestedHelper::empty();
    } else {
        tile.creation_time = sim_time;
    }
    (held, tile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::{ObjectDef, Transition};

    fn def(id: i32, food: i32, slots: i32) -> ObjectDef {
        ObjectDef {
            id,
            description: format!("obj{id}"),
            name: format!("Obj{id}"),
            containable: slots > 0,
            permanent: false,
            blocks_walking: false,
            food_value: food,
            heat_value: 0.0,
            map_chance: 0.0,
            biomes: Vec::new(),
            num_uses: 0,
            num_slots: slots,
            floor: false,
            dummy_ids: Vec::new(),
            use_chance: 0.0,
            speed_mult: 1.0,
            winter_decay_factor: 0.0,
            spring_regrow_factor: 0.0,
            decay_factor: 1.0,
            decays_to_obj: 0,
            r_value: 0.0,
            clothing: "n".into(),
            counts_or_grows_as: 0,
            crafting_steps: 0,
        use_distance: 1,
        deadly_distance: 0.0,
        moves: 0,
        damage: 0.0,
        damage_protection_factor: 1.0,
        wound_factor: 0.5,
        male: false,
        contain_size: 0.0,
        slot_size: 1.0,
        }
    }

    fn tr(a: i32, t: i32, na: i32, nt: i32) -> Transition {
        Transition {
            actor_id: a,
            target_id: t,
            new_actor_id: na,
            new_target_id: nt,
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
        }
    }

    #[test]
    fn mount_held_and_drugs() {
        assert!(is_horse_mount_held(770));
        assert!(is_horse_mount_held(778));
        assert!(is_horse_mount_held(3158));
        assert!(!is_horse_mount_held(33));
        assert!(is_drugs(837));
        assert!(is_drugs(838));
        assert!(!is_drugs(31));
    }

    #[test]
    fn hitch_cart_id_helpers() {
        assert!(is_horse_cart_held(778));
        assert!(is_horse_cart_held(3158));
        assert!(!is_horse_cart_held(770));
        assert!(is_hitched_cart(779));
        assert!(is_hitched_cart(3159));
        assert!(!is_hitched_cart(778));
        assert!(is_hitch_anchor(4154));
        assert!(is_hitch_anchor(550));
        assert_eq!(default_hitched_id_for_cart(778), Some(779));
        assert_eq!(default_hitched_id_for_cart(3158), Some(3159));
        assert_eq!(default_cart_id_from_hitched(779), Some(778));
        assert_eq!(default_cart_id_from_hitched(3159), Some(3158));
        assert!(is_grave_basket_target(87));
        assert!(is_grave_basket_target(357));
        assert!(!is_grave_basket_target(33));
    }

    #[test]
    fn basket_refuse_gate() {
        // Empty basket may change held (grave scoop).
        assert!(!basket_refuse_if_changing_held(BASKET, 0, true));
        // Basket with cargo refuses changeHeld.
        assert!(basket_refuse_if_changing_held(BASKET, 1, true));
        // Non-basket ok.
        assert!(!basket_refuse_if_changing_held(778, 2, true));
        // No change held → ok even with cargo.
        assert!(!basket_refuse_if_changing_held(BASKET, 2, false));
    }

    #[test]
    fn horse_drop_and_nest_swap_flags() {
        assert!(is_horse_drop_trans(true, 0, 2));
        assert!(!is_horse_drop_trans(true, 0, 0));
        assert!(!is_horse_drop_trans(false, 0, 2));
        assert!(should_nest_swap_helpers(true, false, 778, 0));
        assert!(should_nest_swap_helpers(false, true, 0, 3));
        assert!(!should_nest_swap_helpers(false, true, 0, 0));
        // Hitch with cargo: not pickup flag, but isHorseDropTrans → nest swap
        assert!(should_nest_swap_helpers(false, true, 0, 2));
        // Unhitch pickup flag alone
        assert!(should_nest_swap_helpers(true, true, 778, 0));
    }

    #[test]
    fn slots_empty_first() {
        // Pickup: cargo 3, new actor has 2 slots → refuse
        assert!(!pickup_or_drop_slots_ok(true, 3, 2, 10));
        assert!(pickup_or_drop_slots_ok(true, 2, 2, 10));
        // Non-pickup uses new target slots
        assert!(!pickup_or_drop_slots_ok(false, 3, 10, 2));
        assert!(pickup_or_drop_slots_ok(false, 1, 0, 2));
    }

    #[test]
    fn put_down_and_dismount_lookup() {
        let mut db = ContentDb::default();
        db.objects.insert(770, def(770, 0, 0));
        db.objects.insert(1421, def(1421, 0, 0));
        db.objects.insert(3158, def(3158, 0, 4));
        db.objects.insert(3161, def(3161, 0, 4));
        db.objects.insert(778, def(778, 0, 4));
        db.objects.insert(1422, def(1422, 0, 4));
        db.transitions.insert((770, -1), tr(770, -1, 0, 1421));
        db.transitions.insert((3158, -1), tr(3158, -1, 0, 1422));
        // Patch tire cart
        apply_default_horse_transition_patches_local(&mut db);

        assert_eq!(put_down_ground_id(&db, 770), Some(1421));
        assert_eq!(put_down_ground_id(&db, 3158), Some(3161));
        let d = empty_ground_dismount_transition(&db, 770).unwrap();
        assert_eq!(d.new_target_id, 1421);
        assert_eq!(d.new_actor_id, 0);
        // Clay-style: newTargetID==0 discarded
        db.transitions.insert((235, -1), tr(235, -1, 382, 0));
        assert!(empty_ground_dismount_transition(&db, 235).is_none());
    }

    /// Local copy of patch only for tire put-down (unit test without full content).
    fn apply_default_horse_transition_patches_local(db: &mut ContentDb) {
        if let Some(t) = db.transitions.get_mut(&(3158, -1)) {
            t.new_target_id = 3161;
        }
        let key = (770, 0);
        if !db.transitions.contains_key(&key) {
            db.transitions.insert(key, tr(770, 0, 0, 1421));
        }
    }

    #[test]
    fn horse_eat_plan_food_and_drugs() {
        let mut db = ContentDb::default();
        db.objects.insert(770, def(770, 0, 0));
        db.objects.insert(31, def(31, 3, 0)); // gooseberry
        db.objects.insert(837, def(837, 1, 0));
        assert_eq!(
            horse_eat_plan(&db, 770, 31, false),
            Some((31, false, 0))
        );
        assert!(horse_eat_plan(&db, 770, 837, false).is_none());
        assert!(horse_eat_plan(&db, 33, 31, false).is_none());
        // Harvest via 0+target
        db.objects.insert(30, def(30, 0, 0));
        db.objects.insert(279, def(279, 0, 0));
        db.transitions.insert((0, 30), tr(0, 30, 31, 30));
        assert_eq!(
            horse_eat_plan(&db, 778, 30, false),
            Some((31, true, 30))
        );
    }

    #[test]
    fn nest_swap_preserves_cargo() {
        let mut tile = ComplexObject::new_simple(1422);
        tile.contained = vec![33, 40];
        tile.slots = vec![NestedHelper::id_only(33), NestedHelper::id_only(40)];
        let nest = complex_to_nested(&tile);
        assert_eq!(nest.id, 1422);
        assert_eq!(nest.contained.len(), 2);
        assert_eq!(nest.contained[0].id, 33);

        let held = NestedHelper::empty();
        let (new_held, new_tile) =
            apply_ids_after_nest_swap(nest, held, 778, 0, 12.5);
        assert_eq!(new_held.id, 778);
        assert_eq!(new_held.contained.len(), 2);
        assert!(new_tile.is_empty());
        let cx = nested_to_complex(&new_held, 12.5);
        assert_eq!(cx.base_id, 778);
        assert_eq!(cx.contained, vec![33, 40]);
        assert_eq!(cx.creation_time, 12.5);
    }

    #[test]
    fn hitch_nest_swap_cargo_to_hitched_cart() {
        // Haxe: 778+cargo + 4154 → 0 + 779 with cargo (isHorseDropTrans nest swap).
        let mut held = NestedHelper::from_wire(778, &[33, 40]);
        held.creation_time = 1.0;
        let tile = NestedHelper::id_only(4154);
        // Argument order matches use_transition: (tile→held base, held→tile base)
        let (new_held, new_tile) =
            apply_ids_after_nest_swap(tile, held, 0, 779, 5.0);
        assert!(new_held.is_empty());
        assert_eq!(new_tile.id, 779);
        assert_eq!(new_tile.contained.len(), 2);
        assert_eq!(new_tile.contained[0].id, 33);
        assert_eq!(new_tile.creation_time, 5.0);
    }

    #[test]
    fn unhitch_nest_swap_cargo_to_held_cart() {
        // Haxe: 0 + 779+cargo → 778+cargo + 4154 (is_pickup_or_drop).
        let tile = NestedHelper::from_wire(779, &[33]);
        let held = NestedHelper::empty();
        let (new_held, new_tile) =
            apply_ids_after_nest_swap(tile, held, 778, 4154, 9.0);
        assert_eq!(new_held.id, 778);
        assert_eq!(new_held.contained.len(), 1);
        assert_eq!(new_held.contained[0].id, 33);
        assert_eq!(new_tile.id, 4154);
        assert!(new_tile.contained.is_empty());
    }

    #[test]
    fn horse_patch_marks_pickup_and_hitch() {
        let mut db = ContentDb::default();
        db.transitions.insert((0, 1422), {
            let mut t = tr(0, 1422, 778, 0);
            t.is_pickup_or_drop = false;
            t
        });
        db.transitions.insert((0, 779), tr(0, 779, 778, 4154));
        db.transitions.insert((0, 3161), tr(0, 3161, 778, 0));
        db.transitions.insert((0, 3159), tr(0, 3159, 778, 4154));
        db.transitions.insert((3158, -1), tr(3158, -1, 0, 1422));
        db.transitions.insert((3158, 4154), tr(3158, 4154, 0, 779));
        db.transitions.insert((292, 87), tr(292, 87, 356, 88));
        ol_content::apply_default_horse_transition_patches(&mut db);
        assert!(db.transitions.get(&(0, 1422)).unwrap().is_pickup_or_drop);
        assert!(db.transitions.get(&(0, 779)).unwrap().is_pickup_or_drop);
        assert!(db.transitions.get(&(0, 3159)).unwrap().is_pickup_or_drop);
        assert!(db.transitions.get(&(292, 87)).unwrap().is_pickup_or_drop);
        assert_eq!(db.transitions.get(&(0, 3161)).unwrap().new_actor_id, 3158);
        assert_eq!(db.transitions.get(&(3158, -1)).unwrap().new_target_id, 3161);
        assert_eq!(db.transitions.get(&(3158, 4154)).unwrap().new_target_id, 3159);
        assert!(db.transitions.contains_key(&(770, 0)));
    }

    // HORSE-MOUNT-POLISH hitch_cart live USE tests
    include!("horse_mount_live_tests.inc.rs");
}
