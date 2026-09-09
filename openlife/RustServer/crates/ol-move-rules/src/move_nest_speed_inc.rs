// Included from move_speed.rs after shoes_soften_backpack_product (MOVE-NEST-SPEED).
// Expects NestedHelper in scope + contained_obj_speed_mult / shoes_soften_backpack_product.

/// Speed mult product for one contained id via content `speedMult` clamp.
#[inline]
fn id_contained_speed_mult_ex(content: &ContentDb, id: i32, min_reduction: f32) -> f32 {
    if id <= 0 {
        return 1.0;
    }
    // Missing content → treat as speedMult 1.0 → clamp upper MinSpeedReduction (Haxe objectData default).
    let sm = content
        .get(id)
        .map(|d| d.speed_mult)
        .unwrap_or(1.0);
    contained_obj_speed_mult_ex(sm, min_reduction)
}

/// Haxe `p.getPackpack().containedObjects` product (one level only).
///
/// Unlike held nest, backpack cargo does **not** scan a sub-nest under each slot.
/// Empty / missing pack → `1.0`.
// Haxe: MoveHelper.calculateSpeed L164-168 + GlobalPlayerInstance.getPackpack
pub fn backpack_nest_speed_product(content: &ContentDb, pack: Option<&NestedHelper>) -> f32 {
    backpack_nest_speed_product_ex(content, pack, MIN_SPEED_REDUCTION_PER_CONTAINED)
}

/// Live-knob variant of [`backpack_nest_speed_product`].
// SETTINGS-LONG-TAIL
pub fn backpack_nest_speed_product_ex(
    content: &ContentDb,
    pack: Option<&NestedHelper>,
    min_reduction: f32,
) -> f32 {
    let Some(p) = pack else {
        return 1.0;
    };
    let mut prod = 1.0f32;
    for obj in &p.contained {
        prod *= id_contained_speed_mult_ex(content, obj.id, min_reduction);
    }
    prod
}

/// Prefer Haxe clothing backpack nest (`clothingObjects[5]`) when the slot is equipped;
/// otherwise fall back to flat `Player.backpack` ids (SAY STORE legacy).
///
/// When `clothing_pack` is `Some` with `id > 0`, only nest `contained` contributes
/// (even if empty — matches Haxe empty packpack). Flat ids are ignored in that case
/// so cargo that lives only on the clothing nest is not missed, and dual representations
/// do not double-count.
// Haxe: MoveHelper.calculateSpeed L164-168 getPackpack().containedObjects
pub fn resolve_backpack_speed_product(
    content: &ContentDb,
    flat_backpack: &[i32],
    clothing_pack: Option<&NestedHelper>,
) -> f32 {
    resolve_backpack_speed_product_ex(
        content,
        flat_backpack,
        clothing_pack,
        MIN_SPEED_REDUCTION_PER_CONTAINED,
    )
}

/// Live-knob variant of [`resolve_backpack_speed_product`].
// SETTINGS-LONG-TAIL
pub fn resolve_backpack_speed_product_ex(
    content: &ContentDb,
    flat_backpack: &[i32],
    clothing_pack: Option<&NestedHelper>,
    min_reduction: f32,
) -> f32 {
    match clothing_pack {
        Some(pack) if pack.id > 0 => {
            backpack_nest_speed_product_ex(content, Some(pack), min_reduction)
        }
        _ => backpack_speed_product_ex(content, flat_backpack, min_reduction),
    }
}

/// Haxe `p.heldObject.containedObjects` + one nested level under each (MOVE-NEST-SPEED).
///
/// ```text
/// for obj in held.containedObjects:
///   mult *= calculateObjSpeedMult(obj)
///   for sub in obj.containedObjects:
///     mult *= calculateObjSpeedMult(sub)
/// ```
///
/// Applied **after** backpack product and shoes-√ soften (Haxe order L166-179).
/// Deeper than one sub-level is not scanned (Haxe stops at one nest under held cargo).
// Haxe: MoveHelper.calculateSpeed L173-179
pub fn held_nest_speed_product(content: &ContentDb, held: Option<&NestedHelper>) -> f32 {
    held_nest_speed_product_ex(content, held, MIN_SPEED_REDUCTION_PER_CONTAINED)
}

/// Live-knob variant of [`held_nest_speed_product`].
// SETTINGS-LONG-TAIL
pub fn held_nest_speed_product_ex(
    content: &ContentDb,
    held: Option<&NestedHelper>,
    min_reduction: f32,
) -> f32 {
    let Some(h) = held else {
        return 1.0;
    };
    let mut p = 1.0f32;
    for obj in &h.contained {
        p *= id_contained_speed_mult_ex(content, obj.id, min_reduction);
        for sub in &obj.contained {
            p *= id_contained_speed_mult_ex(content, sub.id, min_reduction);
            // depth-2 under sub intentionally ignored (Haxe one nest only)
        }
    }
    p
}

/// Combine backpack product (shoes-softened) with held-nest product (no shoes soften).
///
/// Haxe: shoes √ on backpack only, then `*= held nest product`.
// Haxe: MoveHelper.calculateSpeed L166-179
#[inline]
pub fn combine_backpack_and_held_nest(
    backpack_product: f32,
    held_nest_product: f32,
    has_both_shoes: bool,
) -> f32 {
    let pack = shoes_soften_backpack_product(backpack_product, has_both_shoes);
    let nest = if held_nest_product.is_finite() && held_nest_product > 0.0 {
        held_nest_product
    } else {
        1.0
    };
    pack * nest
}
