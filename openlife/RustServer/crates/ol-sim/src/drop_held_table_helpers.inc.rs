// DROP-HELD-TABLE / table_prefer helpers (included from drop_held_ai.rs after force_drop_at_feet)
// Haxe: AiHelper.ShouldDropOnTable / isSmallFoodToStore / container factors
// NOTE: quiver_from_clothing_* live after QuiverClothing (see build patcher).

/// Omelette — Haxe `AiHelper.dropOnTable`.
// Haxe: dropOnTable = [1285]
pub const OMELETTE: i32 = 1285;
/// Table — preferred container for omelette.
// Haxe: Table 3371
pub const TABLE: i32 = 3371;
/// Wooden Slot Box — most preferred small-food store.
// Haxe: Wooden Slot Box 3065
pub const WOODEN_SLOT_BOX: i32 = 3065;
/// Haxe `AiHelper.dropOnTable` parent ids.
pub const DROP_ON_TABLE_IDS: &[i32] = &[OMELETTE];
/// Haxe `AiHelper.pies` (baked pies for store + same-food prefer).
// Haxe: pies = [272, 803, 273, 274, 275, 276, 277, 278] == baker COOKED_PIES
pub const BAKED_PIE_IDS: &[i32] = COOKED_PIES;
/// Haxe `AiHelper.smallCookedFood`.
// Haxe: smallCookedFood = [570]
pub const SMALL_COOKED_FOOD_IDS: &[i32] = &[COOKED_MUTTON];

/// Haxe `AiHelper.ShouldDropOnTable` — omelette stages on tables.
// Haxe: AiHelper.ShouldDropOnTable ~30–32
#[inline]
pub fn should_drop_on_table(parent_id: i32) -> bool {
    parent_id > 0 && DROP_ON_TABLE_IDS.contains(&parent_id)
}

/// Haxe `AiHelper.IsBakedPie`.
// Haxe: AiHelper.IsBakedPie ~34–36
#[inline]
pub fn is_baked_pie(parent_id: i32) -> bool {
    parent_id > 0 && BAKED_PIE_IDS.contains(&parent_id)
}

/// Haxe `AiHelper.isSmallFoodToStore` — pies + cooked mutton into boxes/baskets.
// Haxe: AiHelper.isSmallFoodToStore ~38–40
#[inline]
pub fn is_small_food_to_store(parent_id: i32) -> bool {
    is_baked_pie(parent_id) || (parent_id > 0 && SMALL_COOKED_FOOD_IDS.contains(&parent_id))
}

/// True when empty-search may USE-drop into a free-slot container.
// Haxe: allowDropInContainer && (ShouldDropOnTable || isSmallFoodToStore)
#[inline]
pub fn allows_drop_in_container(held_id: i32) -> bool {
    should_drop_on_table(held_id) || is_small_food_to_store(held_id)
}

/// Distance adjust for container prefer (Haxe quadDistance factor math).
// Haxe: if factor<1 quad-=4; then *=factor (or /= if ≤0)
#[inline]
pub fn adjust_container_drop_score(quad_distance: f32, factor: f32) -> f32 {
    let mut d = quad_distance;
    if factor < 1.0 {
        d -= 4.0;
    }
    if d > 0.0 {
        d * factor
    } else if factor != 0.0 {
        d / factor
    } else {
        d
    }
}

/// Haxe container prefer factor for table/small-food held (None = not eligible).
// Haxe: GetClosestObjectToPositionHelper considerContainer factors ~232–272
pub fn container_prefer_factor(held_id: i32, container: &ScanTile) -> Option<f32> {
    if !allows_drop_in_container(held_id) || !container.has_free_slot() {
        return None;
    }
    let parent = container.parent_id;
    if should_drop_on_table(held_id) {
        let factor = if parent == TABLE { 0.25 } else { 1.0 };
        return Some(factor);
    }
    let mut factor = if parent == WOODEN_SLOT_BOX {
        0.25
    } else if parent == BASKET {
        0.5
    } else {
        0.8
    };
    if is_baked_pie(held_id) {
        if tile_contains_baked_pie(container) {
            factor *= 0.5;
        }
    } else if held_id == COOKED_MUTTON && container.contains_parent(COOKED_MUTTON) {
        factor *= 0.5;
    }
    Some(factor)
}

/// True when any nested slot is a baked pie parent id.
// Haxe: IsBakedPie(contained.parentId)
#[inline]
fn tile_contains_baked_pie(t: &ScanTile) -> bool {
    if is_baked_pie(t.contains_id) {
        return true;
    }
    t.contains_extra.iter().any(|&id| is_baked_pie(id))
}

/// Squared euclidean distance (Haxe quadDistance helper, no world wrap).
#[inline]
fn quad_distance_xy(ax: i32, ay: i32, bx: i32, by: i32) -> f32 {
    let dx = (bx - ax) as f32;
    let dy = (by - ay) as f32;
    dx * dx + dy * dy
}
