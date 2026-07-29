//! Haxe `AiBase.isPickingupFood` pure state machine (**AI-PICKUP-FOOD**).
//!
//! Closes residual from AI-GOTO-FOOD: drop held before move, permanent→USE,
//! non-permanent ground food→DROP, container→REMV, fail→30s notReachable.
//!
//! // Haxe: AiBase.isPickingupFood ~8610–8706

use crate::ai_path_reach::{food_pickup_success_reset_did_not_reach, StickyFoodTarget};

// ── Pure gates ──────────────────────────────────────────────────────────────

/// Haxe drop distance before walking to food (`food_store < 0.5 ? 5 : 10`).
// Haxe: AiBase.isPickingupFood ~8635
#[inline]
pub fn food_pickup_drop_dist(food_store: f32) -> f32 {
    if food_store < 0.5 {
        5.0
    } else {
        10.0
    }
}

/// Haxe `isUse` for food target: permanent object **or** foodValue &lt; 1.
// Haxe: AiBase.isPickingupFood ~8624
// `foodTarget.isPermanent() || foodTarget.objectData.foodValue < 1`
#[inline]
pub fn food_pickup_is_use(is_permanent: bool, food_value: i32) -> bool {
    is_permanent || food_value < 1
}

/// Haxe `foodTarget.indexInContainer >= 0`.
// Haxe: AiBase.isPickingupFood ~8666
#[inline]
pub fn food_pickup_in_container(index_in_container: i32) -> bool {
    index_in_container >= 0
}

/// Chebyshev tile distance (npc SeekFood adjacency uses the same metric).
#[inline]
pub fn food_pickup_tile_dist(px: i32, py: i32, fx: i32, fy: i32) -> i32 {
    (px - fx).abs().max((py - fy).abs())
}

// ── Plan enum ───────────────────────────────────────────────────────────────

/// Next step of Haxe `isPickingupFood` (caller emits NetIntent / dropHeld).
// Haxe: AiBase.isPickingupFood ~8610–8706
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IsPickingupFoodPlan {
    /// `foodTarget == null` — not in pickup SM this tick.
    Inactive,
    /// Held parent matches food parent → clear sticky, not busy with pickup.
    // Haxe: heldObject.parentId == foodTarget.parentId → foodTarget=null; return false
    ClearedAlreadyHeld,
    /// `isEatableCheckAgain` failed → clear sticky; tick consumed.
    // Haxe: ~8618–8621 return true
    ClearedUneatable,
    /// Drop held before walk (`dropHeldObject(dropDist)`).
    // Haxe: ~8635–8639
    DropHeldBeforeMove { max_distance_to_home: f32 },
    /// Player still pathing — hold tick.
    // Haxe: isMoving → return true ~8644
    BusyMoving,
    /// Walk toward food (caller: `plan_goto_obj` + dual-pass Goto).
    // Haxe: distance > 1 → gotoObj ~8646–8654
    GotoFood,
    /// Holding a baby → drop player at feet before pickup.
    // Haxe: getHeldPlayer / dropPlayer ~8657–8663
    DropHeldPlayer,
    /// At range: empty hands for USE or container REMV (`dropHeldObject(0)`).
    // Haxe: (isUse || isInContainer) && isHoldingObject ~8671–8674
    DropHeldForPickup,
    /// REMV pie/food from basket etc.
    // Haxe: isInContainer && !isUse → remove ~8684–8685
    Remv { x: i32, y: i32, index: i32 },
    /// Permanent bush / low foodValue → USE.
    // Haxe: isUse → use ~8688
    Use { x: i32, y: i32 },
    /// Ground edible (berry bowl style) → empty-hand DROP on tile.
    // Haxe: else drop ~8689
    DropOnFood { x: i32, y: i32 },
}

/// Sensor snapshot for one `isPickingupFood` plan.
// Haxe: fields read inside isPickingupFood
#[derive(Debug, Clone, Copy)]
pub struct IsPickingupFoodInput {
    /// Sticky food present.
    pub has_food_target: bool,
    pub food_x: i32,
    pub food_y: i32,
    pub food_parent_id: i32,
    /// `-1` = ground; `>=0` = container slot.
    pub food_index_in_container: i32,
    pub food_is_permanent: bool,
    /// Target objectData.foodValue (not player's food_store).
    pub food_value: i32,
    /// Tile / contained food still edible (`isEatableCheckAgain`).
    pub tile_still_eatable: bool,
    /// Held object parent id (`0` = empty hands).
    pub held_parent_id: i32,
    /// Haxe `isHoldingObject()` (non-zero held that counts as holding).
    pub is_holding_object: bool,
    /// Haxe `isMoving()`.
    pub is_moving: bool,
    pub player_x: i32,
    pub player_y: i32,
    /// Player food_store (for dropDist only).
    pub food_store: f32,
    /// Haxe `getHeldPlayer() != null`.
    pub holding_player: bool,
    /// When true and holding: plan returns DropHeld\* (dropHeld would act).
    /// Set false after a failed drop attempt so SM can proceed with held item.
    pub drop_held_would_act: bool,
}

impl IsPickingupFoodInput {
    /// Build from sticky food + live sensors.
    pub fn from_sticky(
        sticky: StickyFoodTarget,
        food_is_permanent: bool,
        food_value: i32,
        tile_still_eatable: bool,
        held_parent_id: i32,
        is_holding_object: bool,
        is_moving: bool,
        player_x: i32,
        player_y: i32,
        food_store: f32,
        holding_player: bool,
        drop_held_would_act: bool,
    ) -> Self {
        Self {
            has_food_target: true,
            food_x: sticky.x,
            food_y: sticky.y,
            food_parent_id: sticky.parent_id,
            food_index_in_container: sticky.index_in_container,
            food_is_permanent,
            food_value,
            tile_still_eatable,
            held_parent_id,
            is_holding_object,
            is_moving,
            player_x,
            player_y,
            food_store,
            holding_player,
            drop_held_would_act,
        }
    }
}

/// Pure `isPickingupFood` next action.
// Haxe: AiBase.isPickingupFood ~8610–8706
pub fn plan_is_picking_up_food(inp: &IsPickingupFoodInput) -> IsPickingupFoodPlan {
    if !inp.has_food_target {
        return IsPickingupFoodPlan::Inactive;
    }

    // Already holding the food we wanted.
    // Haxe: heldObject.parentId == foodTarget.parentId
    if inp.held_parent_id > 0 && inp.held_parent_id == inp.food_parent_id {
        return IsPickingupFoodPlan::ClearedAlreadyHeld;
    }

    // Someone ate it / multi-use gone.
    // Haxe: isEatableCheckAgain == false
    if !inp.tile_still_eatable {
        return IsPickingupFoodPlan::ClearedUneatable;
    }

    let is_use = food_pickup_is_use(inp.food_is_permanent, inp.food_value);
    let in_container = food_pickup_in_container(inp.food_index_in_container);

    // Drop held before move (Haxe always attempts when holding).
    // Haxe: dropDist = food_store < 0.5 ? 5 : 10; dropHeldObject(dropDist)
    if inp.is_holding_object && inp.drop_held_would_act {
        return IsPickingupFoodPlan::DropHeldBeforeMove {
            max_distance_to_home: food_pickup_drop_dist(inp.food_store),
        };
    }

    if inp.is_moving {
        return IsPickingupFoodPlan::BusyMoving;
    }

    let dist = food_pickup_tile_dist(inp.player_x, inp.player_y, inp.food_x, inp.food_y);
    if dist > 1 {
        return IsPickingupFoodPlan::GotoFood;
    }

    // At range: drop baby first.
    // Haxe: getHeldPlayer / dropPlayer
    if inp.holding_player {
        return IsPickingupFoodPlan::DropHeldPlayer;
    }

    // Empty hands for USE or container take.
    // Haxe: (isUse || isInContainer) && isHoldingObject → dropHeldObject(0)
    // (If drop_held_would_act was true we already returned DropHeldBeforeMove above.)
    // When drop failed earlier, is_holding may still be true — still need empty hands.
    if (is_use || in_container) && inp.is_holding_object {
        return IsPickingupFoodPlan::DropHeldForPickup;
    }

    // Final pickup action.
    // Haxe: isInContainer && !isUse → remove; isUse → use; else drop
    if in_container && !is_use {
        return IsPickingupFoodPlan::Remv {
            x: inp.food_x,
            y: inp.food_y,
            index: inp.food_index_in_container,
        };
    }
    if is_use {
        return IsPickingupFoodPlan::Use {
            x: inp.food_x,
            y: inp.food_y,
        };
    }
    IsPickingupFoodPlan::DropOnFood {
        x: inp.food_x,
        y: inp.food_y,
    }
}

/// Whether the plan consumes the AI tick as "busy with food pickup" (Haxe `return true`).
// Haxe: isPickingupFood returns Bool — false only for null target / already held
#[inline]
pub fn is_picking_up_food_busy(plan: IsPickingupFoodPlan) -> bool {
    !matches!(
        plan,
        IsPickingupFoodPlan::Inactive | IsPickingupFoodPlan::ClearedAlreadyHeld
    )
}

/// After successful REMV/USE/DROP: reset didNotReachFood + clear sticky.
// Haxe: ~8703–8704
pub fn food_pickup_action_success_reset() -> f32 {
    food_pickup_success_reset_did_not_reach()
}

// ── Sticky helpers ──────────────────────────────────────────────────────────

/// Enrich sticky with container slot (Haxe ObjectHelper.indexInContainer).
#[inline]
pub fn sticky_food_with_container(
    x: i32,
    y: i32,
    parent_id: i32,
    index_in_container: i32,
) -> StickyFoodTarget {
    StickyFoodTarget::with_container(x, y, parent_id, index_in_container)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn base_inp() -> IsPickingupFoodInput {
        IsPickingupFoodInput {
            has_food_target: true,
            food_x: 5,
            food_y: 5,
            food_parent_id: 31, // gooseberry-ish
            food_index_in_container: -1,
            food_is_permanent: false,
            food_value: 3,
            tile_still_eatable: true,
            held_parent_id: 0,
            is_holding_object: false,
            is_moving: false,
            player_x: 5,
            player_y: 5,
            food_store: 10.0,
            holding_player: false,
            drop_held_would_act: true,
        }
    }

    #[test]
    fn drop_dist_starving_vs_ok() {
        assert!((food_pickup_drop_dist(0.4) - 5.0).abs() < 0.01);
        assert!((food_pickup_drop_dist(0.5) - 10.0).abs() < 0.01);
        assert!((food_pickup_drop_dist(12.0) - 10.0).abs() < 0.01);
    }

    #[test]
    fn is_use_permanent_or_low_food_value() {
        assert!(food_pickup_is_use(true, 5));
        assert!(food_pickup_is_use(false, 0));
        assert!(!food_pickup_is_use(false, 3));
    }

    #[test]
    fn inactive_and_already_held() {
        let mut inp = base_inp();
        inp.has_food_target = false;
        assert_eq!(
            plan_is_picking_up_food(&inp),
            IsPickingupFoodPlan::Inactive
        );
        assert!(!is_picking_up_food_busy(IsPickingupFoodPlan::Inactive));

        inp.has_food_target = true;
        inp.held_parent_id = 31;
        assert_eq!(
            plan_is_picking_up_food(&inp),
            IsPickingupFoodPlan::ClearedAlreadyHeld
        );
        assert!(!is_picking_up_food_busy(
            IsPickingupFoodPlan::ClearedAlreadyHeld
        ));
    }

    #[test]
    fn uneatable_clears() {
        let mut inp = base_inp();
        inp.tile_still_eatable = false;
        assert_eq!(
            plan_is_picking_up_food(&inp),
            IsPickingupFoodPlan::ClearedUneatable
        );
        assert!(is_picking_up_food_busy(
            IsPickingupFoodPlan::ClearedUneatable
        ));
    }

    #[test]
    fn drop_held_before_move_when_holding() {
        let mut inp = base_inp();
        inp.is_holding_object = true;
        inp.held_parent_id = 34; // sharp stone
        inp.player_x = 0;
        inp.player_y = 0; // far from food
        inp.food_store = 0.2;
        match plan_is_picking_up_food(&inp) {
            IsPickingupFoodPlan::DropHeldBeforeMove {
                max_distance_to_home,
            } => {
                assert!((max_distance_to_home - 5.0).abs() < 0.01);
            }
            other => panic!("expected DropHeldBeforeMove, got {other:?}"),
        }
        inp.food_store = 8.0;
        match plan_is_picking_up_food(&inp) {
            IsPickingupFoodPlan::DropHeldBeforeMove {
                max_distance_to_home,
            } => {
                assert!((max_distance_to_home - 10.0).abs() < 0.01);
            }
            other => panic!("expected DropHeldBeforeMove, got {other:?}"),
        }
    }

    #[test]
    fn busy_moving_and_goto() {
        let mut inp = base_inp();
        inp.is_moving = true;
        inp.player_x = 0;
        assert_eq!(
            plan_is_picking_up_food(&inp),
            IsPickingupFoodPlan::BusyMoving
        );

        inp.is_moving = false;
        inp.player_x = 0;
        inp.player_y = 0;
        assert_eq!(plan_is_picking_up_food(&inp), IsPickingupFoodPlan::GotoFood);
    }

    #[test]
    fn drop_held_player_at_range() {
        let mut inp = base_inp();
        inp.holding_player = true;
        assert_eq!(
            plan_is_picking_up_food(&inp),
            IsPickingupFoodPlan::DropHeldPlayer
        );
    }

    #[test]
    fn permanent_use_and_ground_drop() {
        let mut inp = base_inp();
        // Permanent bush → USE
        inp.food_is_permanent = true;
        inp.food_value = 4;
        assert_eq!(
            plan_is_picking_up_food(&inp),
            IsPickingupFoodPlan::Use { x: 5, y: 5 }
        );

        // Ground edible → DROP
        inp.food_is_permanent = false;
        inp.food_value = 3;
        assert_eq!(
            plan_is_picking_up_food(&inp),
            IsPickingupFoodPlan::DropOnFood { x: 5, y: 5 }
        );
    }

    #[test]
    fn container_remv_and_drop_held_for_pickup() {
        let mut inp = base_inp();
        inp.food_index_in_container = 2;
        inp.food_is_permanent = false;
        inp.food_value = 5;
        // Empty hands → REMV
        assert_eq!(
            plan_is_picking_up_food(&inp),
            IsPickingupFoodPlan::Remv {
                x: 5,
                y: 5,
                index: 2
            }
        );

        // Holding → must drop first for container
        inp.is_holding_object = true;
        inp.held_parent_id = 99;
        // drop_held_would_act true → DropHeldBeforeMove first (even at range)
        assert!(matches!(
            plan_is_picking_up_food(&inp),
            IsPickingupFoodPlan::DropHeldBeforeMove { .. }
        ));
        // After drop attempt failed: force drop for pickup
        inp.drop_held_would_act = false;
        assert_eq!(
            plan_is_picking_up_food(&inp),
            IsPickingupFoodPlan::DropHeldForPickup
        );
    }

    #[test]
    fn use_with_held_forces_drop() {
        let mut inp = base_inp();
        inp.food_is_permanent = true;
        inp.is_holding_object = true;
        inp.held_parent_id = 34;
        inp.drop_held_would_act = false; // already tried dropHeld before move
        assert_eq!(
            plan_is_picking_up_food(&inp),
            IsPickingupFoodPlan::DropHeldForPickup
        );
    }

    #[test]
    fn success_reset() {
        assert_eq!(food_pickup_action_success_reset(), 0.0);
    }

    #[test]
    fn sticky_container_ctor() {
        let s = sticky_food_with_container(1, 2, 40, 3);
        assert_eq!(s.x, 1);
        assert_eq!(s.y, 2);
        assert_eq!(s.parent_id, 40);
        assert_eq!(s.index_in_container, 3);
        assert!(s.in_container());
        assert!(!StickyFoodTarget::new(0, 0, 1).in_container());
    }
}
