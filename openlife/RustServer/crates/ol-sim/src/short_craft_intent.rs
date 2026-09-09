//! Live shortCraft → USE/DROP intent mapping (CRAFT-LIVE-IO).
//!
//! Bridges pure [`ShortCraftApply`] / [`SmithApply`] decisions to concrete tile
//! USE/DROP (or seek/craft staging) for AI/NPC ticks and profession live I/O.
//!
//! Also hosts **CRAFT-LIVE-TICK** world-scan via [`profession_scan`] submodule
//! (crate-root re-export via build wire when present).
//!
//! **AI-CRAFT-GRAPH-IO**: pure GetOrCraftItem / GetItem lives in [`crate::get_or_craft`];
//! resolve SeekOrCraft/CraftItem via `resolve_seek_or_craft_live` before
//! [`apply_short_craft_live_intent`] (staging-only without world objects).
//!
//! Haxe anchors: `AiBase.shortCraft` / `shortCraftOnTarget` / `useHeldObjOnTarget`
//! / `shortCraftOnGround` / `dropHeldObject` / `checkHungryWorkCostById` /
//! `GetOrCraftItem` / `GetItem`.

use crate::farmer_profession::{
    short_craft_apply_resolved, ShortCraftApply, ShortCraftInput, BASKET_OF_SOIL, WEAK_SKEWER,
};
use crate::smith_profession::{
    short_craft_on_ground_apply, SmithApply, FIRING_KILN, FLOOR_PLACE_ACTOR_IDS,
};
use crate::{apply_drop, apply_use_at, SimState, UseResult};
use ol_net::OutboundHub;

// ── Live intent enum ────────────────────────────────────────────────────────

/// Concrete next step after pure shortCraft / smith apply (tile-aware).
///
/// Profession ticks fill coords from world scan, then either emit NetIntent or
/// call [`apply_short_craft_live_intent`].
// Haxe: useHeldObjOnTarget / dropHeldObject / GetOrCraftItem / shortCraftOnGround
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortCraftLiveIntent {
    /// USE held on target tile (Haxe `useHeldObjOnTarget` → client USE x y).
    UseAt {
        x: i32,
        y: i32,
        target_id: i32,
        actor_id: i32,
    },
    /// DROP held at empty / staging tile (Haxe `dropHeldObject` simplified).
    DropAt { x: i32, y: i32 },
    /// Walk toward tile before placing (Haxe `gotoObj` dropOnStart / dropHeld Goto).
    // Haxe: myPlayer.gotoObj(target) in dropHeldObject when dropOnStart
    Goto { x: i32, y: i32 },
    /// Store held into clothing (Haxe `myPlayer.self(0,0,slot)` quiver slot 5).
    // Haxe: storeInQuiver → self(0, 0, 5); live via NetIntent::Raw SELF
    SelfClothing { slot: i32 },
    /// USE held on empty ground (shortCraftOnGround when held == target).
    UseOnEmptyGround { x: i32, y: i32, held: i32 },
    /// Need actor — seek and optionally craft (AI-CRAFT residual).
    SeekOrCraft { actor: i32, craft_if_needed: bool },
    /// shortCraftOnGround: need to hold ground-use object first.
    SeekGroundActor { target: i32 },
    /// craftItem residual (AI-CRAFT).
    CraftItem { object_id: i32 },
    /// Holding craft-drop object too far from forge → walk to forge.
    GotoForge {
        object_id: i32,
        forge_x: i32,
        forge_y: i32,
    },
    /// Pickup free object near forge then re-enter drop path.
    PickupNearForge { object_id: i32, x: i32, y: i32 },
    /// Defer pottery (seek kiln).
    DeferPottery,
    /// Hungry work cost refused.
    RefuseHungry,
    /// Hold AI tick while pathing (Haxe isMoving return true / dropHeld BusyMoving).
    // Haxe: dropHeldObject dropOnStart isMoving → return true (PREFER-SHORT-WAIT)
    Wait,
    /// Stage `removeFromContainerTarget` this tick (Haxe `removeItemFromContainer` returns true).
    // Haxe: AiBase.removeItemFromContainer ~1455 (AI-REMOVE-CONTAINER)
    StageRemoveFromContainer { x: i32, y: i32, expected_parent: i32 },
    /// REMV last contained at tile (Haxe `myPlayer.remove`).
    Remv { x: i32, y: i32 },
    /// Haxe `doOnOther` / UBABY feed starving (AI-JOB-FOODSERVER).
    FeedOther { target_p_id: i32, target_conn: u64 },
    /// Held not feedable — live SearchBestFood(target, self).
    SeekFeedFood { target_p_id: i32, target_conn: u64 },
    /// Haxe `forceStopOnNextTile` while pathing toward feed target.
    ForceStopWait,
    /// Haxe `myPlayer.kill(tx-gx, ty-gy, id)` (AI-ATTACK-PLAYER).
    // Haxe: AiBase.attackPlayer ~5872
    Kill {
        target_p_id: i32,
        x: i32,
        y: i32,
    },
    /// Other refuse / none / abort / unreachable coords.
    None,
}

impl ShortCraftLiveIntent {
    /// True when this intent is a live USE or DROP at known tiles.
    pub fn is_wire_action(self) -> bool {
        matches!(
            self,
            Self::UseAt { .. }
                | Self::DropAt { .. }
                | Self::UseOnEmptyGround { .. }
                | Self::Remv { .. }
        )
    }

    /// Map to NetIntent-shaped coords for USE (None if not a use).
    pub fn use_xy(self) -> Option<(i32, i32)> {
        match self {
            Self::UseAt { x, y, .. } | Self::UseOnEmptyGround { x, y, .. } => Some((x, y)),
            _ => None,
        }
    }

    /// Map to DROP coords (None if not a drop).
    pub fn drop_xy(self) -> Option<(i32, i32)> {
        match self {
            Self::DropAt { x, y } => Some((x, y)),
            _ => None,
        }
    }
}

/// Spatial context filled by profession tick / AI from world scan.
// Haxe: getClosestObjectById / GetClosestObjectToTarget / forge / oven anchors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortCraftIntentCtx {
    /// Target object tile for UseOnTarget (Haxe ObjectHelper.tx/ty).
    pub target_x: i32,
    pub target_y: i32,
    /// Empty ground tile for DropHeld / generic shortCraftOnGround.
    pub empty_drop_x: i32,
    pub empty_drop_y: i32,
    /// Preferred empty near well (Basket of Soil 336).
    pub empty_near_well_x: Option<i32>,
    pub empty_near_well_y: Option<i32>,
    /// Preferred empty near home (336 fallback).
    pub empty_near_home_x: Option<i32>,
    pub empty_near_home_y: Option<i32>,
    /// Forge drop tile (smith DropNearForge).
    pub forge_x: i32,
    pub forge_y: i32,
    /// Pickup candidate tile near forge (0,0 = unknown → intent None coords).
    pub forge_pickup_x: i32,
    pub forge_pickup_y: i32,
    /// When false, UseAt is suppressed (Haxe isObjectNotReachable / hostile path).
    pub target_reachable: bool,
}

impl ShortCraftIntentCtx {
    /// Minimal context: all actions at one target tile; drop on same tile.
    pub fn at_target(tx: i32, ty: i32) -> Self {
        Self {
            target_x: tx,
            target_y: ty,
            empty_drop_x: tx,
            empty_drop_y: ty,
            empty_near_well_x: None,
            empty_near_well_y: None,
            empty_near_home_x: None,
            empty_near_home_y: None,
            forge_x: tx,
            forge_y: ty,
            forge_pickup_x: tx,
            forge_pickup_y: ty,
            target_reachable: true,
        }
    }

    /// Well/home empty tiles for Basket of Soil 336 shortCraftOnGround.
    pub fn with_soil_anchors(
        mut self,
        well_empty: Option<(i32, i32)>,
        home_empty: Option<(i32, i32)>,
    ) -> Self {
        if let Some((x, y)) = well_empty {
            self.empty_near_well_x = Some(x);
            self.empty_near_well_y = Some(y);
        }
        if let Some((x, y)) = home_empty {
            self.empty_near_home_x = Some(x);
            self.empty_near_home_y = Some(y);
        }
        self
    }
}

// ── useHeldObjOnTarget staging (CRAFT-LIVE-IO) ───────────────────────────────

/// Haxe milkweed family may change 50→51→52 without cancelling use.
// Haxe: AiBase.isUsingItem milkweed exception
#[inline]
pub fn is_milkweed_use_family(parent_id: i32) -> bool {
    matches!(parent_id, 50 | 51 | 52)
}

/// Staged `useHeldObjOnTarget` (Haxe `useTarget` / `useActor` / `expectedUseTarget`).
// Haxe: AiBase.useHeldObjOnTarget ~1391–1405
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseHeldStaging {
    pub tx: i32,
    pub ty: i32,
    /// Expected world parent at (`tx`,`ty`).
    pub expected_target_parent: i32,
    /// Held parent that must be in-hand (0 = empty-hand use / pickup).
    pub use_actor_parent: i32,
    /// Haxe `useIsDropInContainer` (basket fill / store-in-container).
    pub use_is_drop_in_container: bool,
}

impl UseHeldStaging {
    pub fn new(
        tx: i32,
        ty: i32,
        expected_target_parent: i32,
        use_actor_parent: i32,
        use_is_drop_in_container: bool,
    ) -> Self {
        Self {
            tx,
            ty,
            expected_target_parent,
            use_actor_parent,
            use_is_drop_in_container,
        }
    }
}

/// Next step for a staged use (Haxe `isUsingItem`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseHeldAdvance {
    /// Adjacent + held matches → `myPlayer.use`.
    UseNow { x: i32, y: i32 },
    /// Chebyshev > 1 → `gotoObj`.
    Goto { x: i32, y: i32 },
    /// Path in progress (Haxe `isMoving` return true).
    Wait,
    /// Expected actor is empty (0) but still holding → drop first.
    DropHeld,
    /// Cancel staging (target gone/changed, wrong actor, blocked container).
    Cancel,
}

/// Stage a use if reachable + hungry-work / floor / container-drop allow.
///
/// `lookup` is Haxe `checkHungryWorkCostById` (None = skip food/transition gate).
// Haxe: useHeldObjOnTarget + checkHungryWorkCost
pub fn stage_use_held_on_target(
    tx: i32,
    ty: i32,
    expected_target_parent: i32,
    held_parent: i32,
    use_is_drop_in_container: bool,
    reachable: bool,
    food_store: f32,
    lookup: Option<&crate::smith_profession::HungryWorkCostLookup>,
) -> Option<UseHeldStaging> {
    if !reachable {
        return None;
    }
    if let Some(lu) = lookup {
        let mut lu = *lu;
        lu.use_is_drop_in_container = use_is_drop_in_container;
        if !crate::smith_profession::check_hungry_work_cost_lookup(held_parent, food_store, &lu) {
            return None;
        }
    }
    Some(UseHeldStaging::new(
        tx,
        ty,
        expected_target_parent,
        held_parent,
        use_is_drop_in_container,
    ))
}

/// Advance staged use for this think tick.
// Haxe: AiBase.isUsingItem ~8896–9038
pub fn advance_use_held_staging(
    staging: UseHeldStaging,
    world_parent: i32,
    held_parent: i32,
    player_x: i32,
    player_y: i32,
    is_moving: bool,
    target_contained_n: usize,
) -> UseHeldAdvance {
    if world_parent == 0 {
        return UseHeldAdvance::Cancel;
    }
    let expected = staging.expected_target_parent;
    if world_parent != expected
        && !(is_milkweed_use_family(world_parent) && is_milkweed_use_family(expected))
    {
        return UseHeldAdvance::Cancel;
    }
    // Haxe: useIsDropInContainer == false && containedObjects.length > 0 → CancleUse
    if !staging.use_is_drop_in_container && target_contained_n > 0 {
        return UseHeldAdvance::Cancel;
    }
    if held_parent != staging.use_actor_parent {
        if staging.use_actor_parent == 0 && held_parent != 0 {
            return UseHeldAdvance::DropHeld;
        }
        return UseHeldAdvance::Cancel;
    }
    if is_moving {
        return UseHeldAdvance::Wait;
    }
    // Haxe isUsingItem CalculateQuadDistanceToObject > 1 → goto (isClose d=1)
    let dx = staging.tx - player_x;
    let dy = staging.ty - player_y;
    if dx * dx + dy * dy > 1 {
        UseHeldAdvance::Goto {
            x: staging.tx,
            y: staging.ty,
        }
    } else {
        UseHeldAdvance::UseNow {
            x: staging.tx,
            y: staging.ty,
        }
    }
}

/// Read world + player and advance [`Player::craft_ai.use_held`] if staged.
pub fn advance_player_use_held_staging(
    state: &crate::SimState,
    conn_id: u64,
) -> Option<UseHeldAdvance> {
    let p = state.players.get(&conn_id)?;
    let staging = p.craft_ai.use_held?;
    let held_parent = if p.held_id != 0 {
        state.content.resolve_base_id(p.held_id)
    } else {
        0
    };
    let is_moving = p.moving || p.move_path.is_some();
    let (world_parent, contained_n) = {
        let w = state.world.read().ok()?;
        let id = w.get_object(staging.tx, staging.ty);
        let parent = if id != 0 {
            state.content.resolve_base_id(id)
        } else {
            0
        };
        let n = w
            .get_helper(staging.tx, staging.ty)
            .map(|h| h.contained.len())
            .unwrap_or(0);
        (parent, n)
    };
    Some(advance_use_held_staging(
        staging,
        world_parent,
        held_parent,
        p.x,
        p.y,
        is_moving,
        contained_n,
    ))
}

/// Read world + player and advance [`Player::craft_ai.remove_from_container`].
// Haxe: AiBase.isRemovingFromContainer ~9140
pub fn advance_player_remove_from_container_staging(
    state: &crate::SimState,
    conn_id: u64,
) -> Option<crate::RemoveFromContainerAdvance> {
    let p = state.players.get(&conn_id)?;
    let staging = p.craft_ai.remove_from_container?;
    let is_moving = p.moving || p.move_path.is_some();
    let hidden = p.is_holding_hidden_wound();
    let holding_player = p.holding_player_id != 0;
    let (world_parent, contained_n) = {
        let w = state.world.read().ok()?;
        let id = w.get_object(staging.tx, staging.ty);
        let parent = if id != 0 {
            state.content.resolve_base_id(id)
        } else {
            0
        };
        let n = w
            .get_helper(staging.tx, staging.ty)
            .map(|h| h.contained.len() as i32)
            .unwrap_or(0);
        (parent, n)
    };
    Some(crate::advance_remove_from_container(
        Some(staging),
        world_parent,
        contained_n,
        p.held_id,
        hidden,
        holding_player,
        p.x,
        p.y,
        is_moving,
        true,
    ))
}

/// Apply one `isRemovingFromContainer` tick. `None` if no sticky.
// Haxe: AiBase.isRemovingFromContainer ~9140
pub fn apply_player_remove_from_container_tick(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
) -> Option<ShortCraftLiveApplyResult> {
    let adv = advance_player_remove_from_container_staging(state, conn_id)?;
    use crate::RemoveFromContainerAdvance;
    match adv {
        RemoveFromContainerAdvance::Idle => None,
        RemoveFromContainerAdvance::Cancel | RemoveFromContainerAdvance::GotoFailed { .. } => {
            if let Some(p) = state.players.get_mut(&conn_id) {
                p.craft_ai.remove_from_container = None;
            }
            Some(ShortCraftLiveApplyResult::Failed)
        }
        RemoveFromContainerAdvance::Wait => {
            Some(ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Wait))
        }
        RemoveFromContainerAdvance::DropHeld | RemoveFromContainerAdvance::DropPlayer => {
            let (px, py) = state
                .players
                .get(&conn_id)
                .map(|p| (p.x, p.y))
                .unwrap_or((0, 0));
            Some(apply_short_craft_live_intent(
                state,
                outbound,
                conn_id,
                ShortCraftLiveIntent::DropAt { x: px, y: py },
            ))
        }
        RemoveFromContainerAdvance::Goto { x, y } => {
            Some(ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { x, y }))
        }
        RemoveFromContainerAdvance::RemvNow { x, y } => Some(apply_short_craft_live_intent(
            state,
            outbound,
            conn_id,
            ShortCraftLiveIntent::Remv { x, y },
        )),
    }
}

/// Map an advance into a live intent (Cancel → None).
pub fn use_held_advance_to_live_intent(adv: UseHeldAdvance) -> ShortCraftLiveIntent {
    match adv {
        UseHeldAdvance::UseNow { x, y } => ShortCraftLiveIntent::UseAt {
            x,
            y,
            target_id: 0,
            actor_id: 0,
        },
        UseHeldAdvance::Goto { x, y } => ShortCraftLiveIntent::Goto { x, y },
        UseHeldAdvance::Wait => ShortCraftLiveIntent::Wait,
        UseHeldAdvance::DropHeld => ShortCraftLiveIntent::DropAt { x: 0, y: 0 },
        UseHeldAdvance::Cancel => ShortCraftLiveIntent::None,
    }
}

// ── Pure mappers ────────────────────────────────────────────────────────────

/// Map [`ShortCraftApply`] + spatial ctx → live USE/DROP/seek intent.
// Haxe: shortCraftOnTarget → useHeldObjOnTarget / dropHeldObject / GetOrCraftItem
pub fn short_craft_apply_to_live_intent(
    apply: ShortCraftApply,
    ctx: &ShortCraftIntentCtx,
) -> ShortCraftLiveIntent {
    match apply {
        ShortCraftApply::UseOnTarget { actor, target } => {
            if !ctx.target_reachable {
                return ShortCraftLiveIntent::None;
            }
            ShortCraftLiveIntent::UseAt {
                x: ctx.target_x,
                y: ctx.target_y,
                target_id: target,
                actor_id: actor,
            }
        }
        ShortCraftApply::DropHeld => ShortCraftLiveIntent::DropAt {
            x: ctx.empty_drop_x,
            y: ctx.empty_drop_y,
        },
        ShortCraftApply::SeekOrCraftActor {
            actor,
            craft_if_needed,
        } => ShortCraftLiveIntent::SeekOrCraft {
            actor,
            craft_if_needed,
        },
        ShortCraftApply::PreferWeakSkewer => {
            // Caller should use short_craft_apply_resolved; surface seek weak as hint.
            ShortCraftLiveIntent::SeekOrCraft {
                actor: WEAK_SKEWER,
                craft_if_needed: false,
            }
        }
        ShortCraftApply::RefuseHungry => ShortCraftLiveIntent::RefuseHungry,
        ShortCraftApply::Refuse => ShortCraftLiveIntent::None,
    }
}

/// Full farm/baker shortCraft path: pure apply (resolved weak skewer) → live intent.
// Haxe: AiBase.shortCraft / shortCraftOnTarget
pub fn short_craft_to_live_intent(
    inp: ShortCraftInput,
    ctx: &ShortCraftIntentCtx,
) -> ShortCraftLiveIntent {
    let apply = short_craft_apply_resolved(inp);
    short_craft_apply_to_live_intent(apply, ctx)
}

/// Map [`SmithApply`] + spatial ctx → live intent (USE/DROP at forge or target).
// Haxe: smith shortCraft / shortCraftOnGround / GetCraftAndDropItemsCloseToObj
pub fn smith_apply_to_live_intent(
    apply: SmithApply,
    ctx: &ShortCraftIntentCtx,
) -> ShortCraftLiveIntent {
    match apply {
        SmithApply::None | SmithApply::Abort | SmithApply::Refuse => ShortCraftLiveIntent::None,
        SmithApply::RefuseHungryCost => ShortCraftLiveIntent::RefuseHungry,
        SmithApply::UseOnTarget { actor, target } => {
            if !ctx.target_reachable {
                return ShortCraftLiveIntent::None;
            }
            ShortCraftLiveIntent::UseAt {
                x: ctx.target_x,
                y: ctx.target_y,
                target_id: target,
                actor_id: actor,
            }
        }
        SmithApply::DropHeld => ShortCraftLiveIntent::DropAt {
            x: ctx.empty_drop_x,
            y: ctx.empty_drop_y,
        },
        SmithApply::SeekOrCraftActor { actor } => ShortCraftLiveIntent::SeekOrCraft {
            actor,
            craft_if_needed: true,
        },
        SmithApply::UseOnEmptyGround { held } => {
            let (x, y) = pick_ground_use_tile(
                held,
                opt_xy(ctx.empty_near_well_x, ctx.empty_near_well_y),
                opt_xy(ctx.empty_near_home_x, ctx.empty_near_home_y),
                Some((ctx.empty_drop_x, ctx.empty_drop_y)),
            )
            .unwrap_or((ctx.empty_drop_x, ctx.empty_drop_y));
            ShortCraftLiveIntent::UseOnEmptyGround { x, y, held }
        }
        SmithApply::SeekOrGetGroundActor { target } => {
            ShortCraftLiveIntent::SeekGroundActor { target }
        }
        SmithApply::CraftItem { object_id } => ShortCraftLiveIntent::CraftItem { object_id },
        SmithApply::GotoForgeForDrop { object_id } => ShortCraftLiveIntent::GotoForge {
            object_id,
            forge_x: ctx.forge_x,
            forge_y: ctx.forge_y,
        },
        SmithApply::DropNearForge { .. } => ShortCraftLiveIntent::DropAt {
            x: ctx.forge_x,
            y: ctx.forge_y,
        },
        SmithApply::PickupNearForge { object_id } => ShortCraftLiveIntent::PickupNearForge {
            object_id,
            x: ctx.forge_pickup_x,
            y: ctx.forge_pickup_y,
        },
        // Haxe: smith Goal::SeekObject(FIRING_KILN) when pottery body is empty
        SmithApply::DeferPottery => ShortCraftLiveIntent::SeekOrCraft {
            actor: FIRING_KILN,
            craft_if_needed: false,
        },
    }
}

fn opt_xy(x: Option<i32>, y: Option<i32>) -> Option<(i32, i32)> {
    match (x, y) {
        (Some(x), Some(y)) => Some((x, y)),
        _ => None,
    }
}

/// Haxe `shortCraftOnGround` empty-tile pick: Basket of Soil 336 prefers well then home.
// Haxe: AiBase.shortCraftOnGround ~2699–2707
pub fn pick_ground_use_tile(
    held_id: i32,
    empty_near_well: Option<(i32, i32)>,
    empty_near_home: Option<(i32, i32)>,
    any_empty: Option<(i32, i32)>,
) -> Option<(i32, i32)> {
    if held_id == BASKET_OF_SOIL {
        empty_near_well.or(empty_near_home).or(any_empty)
    } else {
        any_empty
    }
}

/// Pure shortCraftOnGround with 336 tile preference baked into live intent.
// Haxe: AiBase.shortCraftOnGround ~2692
pub fn short_craft_on_ground_to_live_intent(
    held_id: i32,
    target: i32,
    ctx: &ShortCraftIntentCtx,
) -> ShortCraftLiveIntent {
    let apply = short_craft_on_ground_apply(held_id, target);
    smith_apply_to_live_intent(apply, ctx)
}

/// Haxe `myPlayer.remove` last contained (AI live, no protocol tag).
// Haxe: AiBase.isRemovingFromContainer myPlayer.remove ~9211
fn apply_ai_remv_last(
    state: &mut SimState,
    conn_id: u64,
    x: i32,
    y: i32,
) -> ShortCraftLiveApplyResult {
    let Some(p) = state.players.get(&conn_id) else {
        return ShortCraftLiveApplyResult::Failed;
    };
    if p.held_id != 0 && !p.is_holding_hidden_wound() {
        return ShortCraftLiveApplyResult::Failed;
    }
    let taken = {
        let mut w = match state.world.write() {
            Ok(g) => g,
            Err(_) => return ShortCraftLiveApplyResult::Failed,
        };
        w.container_take(x, y, None)
    };
    if let Some(id) = taken {
        if let Some(p) = state.players.get_mut(&conn_id) {
            p.held_id = id;
            p.craft_ai.remove_from_container = None;
        }
        ShortCraftLiveApplyResult::Dropped
    } else {
        if let Some(p) = state.players.get_mut(&conn_id) {
            p.craft_ai.remove_from_container = None;
            p.ai_path_reach.add_not_reachable(x, y, 90.0);
        }
        ShortCraftLiveApplyResult::Failed
    }
}

// ── Live apply into sim (USE / DROP) ────────────────────────────────────────

/// Result of applying a wire-capable [`ShortCraftLiveIntent`].
#[derive(Debug, Clone, PartialEq)]
pub enum ShortCraftLiveApplyResult {
    /// `apply_use_at` ran.
    Used(UseResult),
    /// `apply_drop` ran.
    Dropped,
    /// Staging / refuse — not a wire action this tick.
    Staging(ShortCraftLiveIntent),
    /// Player missing / USE returned None.
    Failed,
}

/// Apply UseAt / DropAt / UseOnEmptyGround via sim `apply_use_at` / `apply_drop`.
///
/// Seek/craft/refuse / Goto / SelfClothing intents return
/// [`ShortCraftLiveApplyResult::Staging`] (npc_ai enqueues MOVE / Raw SELF).
/// Callers should run [`crate::resolve_seek_or_craft_live`] first when world
/// objects are available so SeekOrCraft → DropAt/UseAt before this apply.
// Haxe: useHeldObjOnTarget staging → server USE; dropHeldObject → DROP
// Haxe: GetOrCraftItem dropIsAUse / dropTarget (resolved via get_or_craft)
pub fn apply_short_craft_live_intent(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    intent: ShortCraftLiveIntent,
) -> ShortCraftLiveApplyResult {
    // BLOCKED-BY-AI: sticky use/drop/food claim before USE/DROP so rebuild sees it.
    // Haxe: AiBase.useTarget / dropTarget / foodTarget while working
    crate::note_ai_block_targets_from_live_intent(state, conn_id, intent);
    let apply_r = match intent {
        ShortCraftLiveIntent::UseAt {
            x,
            y,
            target_id,
            actor_id,
        } => {
            let (px, py, held) = state
                .players
                .get(&conn_id)
                .map(|p| (p.x, p.y, p.held_id))
                .unwrap_or((x, y, actor_id));
            let expected = if target_id != 0 {
                state.content.resolve_base_id(target_id)
            } else {
                let id = state.world.read().ok().map(|w| w.get_object(x, y)).unwrap_or(0);
                if id != 0 {
                    state.content.resolve_base_id(id)
                } else {
                    0
                }
            };
            let actor = if actor_id != 0 { actor_id } else { held };
            if let Some(p) = state.players.get_mut(&conn_id) {
                p.craft_ai.use_held = Some(UseHeldStaging::new(x, y, expected, actor, false));
            }
            // Haxe isClose d=1 (squared); diagonal is not UseNow
            if !crate::in_use_range(px, py, x, y, 1) {
                ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto { x, y })
            } else {
                match apply_use_at(state, conn_id, x, y) {
                    Some(r) => {
                        if let Some(p) = state.players.get_mut(&conn_id) {
                            p.craft_ai.use_held = None;
                        }
                        ShortCraftLiveApplyResult::Used(r)
                    }
                    None => ShortCraftLiveApplyResult::Failed,
                }
            }
        }
        ShortCraftLiveIntent::UseOnEmptyGround { x, y, .. } => {
            match apply_use_at(state, conn_id, x, y) {
                Some(r) => ShortCraftLiveApplyResult::Used(r),
                None => ShortCraftLiveApplyResult::Failed,
            }
        }
        ShortCraftLiveIntent::DropAt { x, y } => {
            let held_before = state.players.get(&conn_id).map(|p| p.held_id).unwrap_or(0);
            apply_drop(state, outbound, conn_id, x, y, None);
            let held_after = state.players.get(&conn_id).map(|p| p.held_id).unwrap_or(0);
            // AI-FOOD-FAIL-MARK: DROP fail on food sticky → 30s (held unchanged).
            // Haxe: isPickingupFood drop done==false ~8689–8699
            if held_after == held_before {
                crate::mark_path_fail_after_food_pickup_action_live(state, conn_id, x, y);
            }
            ShortCraftLiveApplyResult::Dropped
        }
        ShortCraftLiveIntent::StageRemoveFromContainer {
            x,
            y,
            expected_parent,
        } => {
            if let Some(p) = state.players.get_mut(&conn_id) {
                p.craft_ai.remove_from_container =
                    crate::stage_remove_item_from_container(x, y, expected_parent, true, false);
            }
            ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Wait)
        }
        ShortCraftLiveIntent::Remv { x, y } => {
            apply_ai_remv_last(state, conn_id, x, y)
        }
        ShortCraftLiveIntent::FeedOther { target_conn, .. } => {
            if crate::try_do_eating(state, conn_id, target_conn) {
                ShortCraftLiveApplyResult::Dropped
            } else {
                ShortCraftLiveApplyResult::Failed
            }
        }
        ShortCraftLiveIntent::SeekFeedFood { target_conn, .. } => {
            let hit = crate::search_best_food_full(
                state,
                target_conn,
                crate::FOODSERVER_FOOD_SEARCH_RADIUS,
                Some(conn_id),
                Some(crate::search_best_food::AiFoodSearchFlags::default()),
                true,
            );
            match hit {
                Some(h) => {
                    let (px, py) = state
                        .players
                        .get(&conn_id)
                        .map(|p| (p.x, p.y))
                        .unwrap_or((h.tx, h.ty));
                    if crate::in_use_range(px, py, h.tx, h.ty, 1) {
                        apply_short_craft_live_intent(
                            state,
                            outbound,
                            conn_id,
                            ShortCraftLiveIntent::UseAt {
                                x: h.tx,
                                y: h.ty,
                                target_id: h.food_id,
                                actor_id: 0,
                            },
                        )
                    } else {
                        ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Goto {
                            x: h.tx,
                            y: h.ty,
                        })
                    }
                }
                None => {
                    if let Some(p) = state.players.get_mut(&conn_id) {
                        p.foodserver_profession.clear_target();
                    }
                    ShortCraftLiveApplyResult::Failed
                }
            }
        }
        ShortCraftLiveIntent::ForceStopWait => {
            if let Some(p) = state.players.get_mut(&conn_id) {
                p.force_stop_on_next_tile = true;
            }
            ShortCraftLiveApplyResult::Staging(ShortCraftLiveIntent::Wait)
        }
        other => ShortCraftLiveApplyResult::Staging(other),
    };
    // AI-FOOD-FAIL-MARK / PATH-REACH: food sticky or empty-hand edible → 30s; else age-gate.
    // Haxe: isPickingupFood fail ~8698; isUsingObject fail ~9133 (callers may also mark).
    // Idempotent re-mark OK; food path clears sticky so second call falls to age-gate only if needed.
    if matches!(apply_r, ShortCraftLiveApplyResult::Failed)
        || matches!(&apply_r, ShortCraftLiveApplyResult::Used(r) if !r.applied)
    {
        if let Some((x, y)) = intent.use_xy() {
            crate::mark_path_fail_after_use_live(state, conn_id, x, y);
        }
    }
    apply_r
}

// ── DROP-HELD-LIVE bridge (parent of profession_scan + drop_held_ai) ─────────

/// Smart profession DropHeld → live intent (avoids sibling cycle profession↔drop_held).
// Haxe: dropHeldObject(allowAllPiles) from pottery gather / farm-smith shortCraft DropHeld
pub fn smart_drop_held_profession(
    tiles: &[profession_scan::ScanTile],
    held_id: i32,
    held_uses: i32,
    player_x: i32,
    player_y: i32,
    home_x: i32,
    home_y: i32,
    food_store: f32,
    allow_all_piles: bool,
    max_distance_to_home: f32,
    held_contains_clay: bool,
) -> ShortCraftLiveIntent {
    smart_drop_held_profession_ex(
        tiles,
        held_id,
        held_uses,
        player_x,
        player_y,
        home_x,
        home_y,
        food_store,
        allow_all_piles,
        max_distance_to_home,
        held_contains_clay,
        false,
        &[0; 6],
        &[0; 6],
    )
}

/// Like [`smart_drop_held_profession`] with Haxe `isMoving` (BusyMoving → Wait).
// Haxe: dropHeldObject dropOnStart if (myPlayer.isMoving()) return true
pub fn smart_drop_held_profession_ex(
    tiles: &[profession_scan::ScanTile],
    held_id: i32,
    held_uses: i32,
    player_x: i32,
    player_y: i32,
    home_x: i32,
    home_y: i32,
    food_store: f32,
    allow_all_piles: bool,
    max_distance_to_home: f32,
    held_contains_clay: bool,
    is_moving: bool,
    clothing: &[i32],
    clothing_uses: &[i32],
) -> ShortCraftLiveIntent {
    smart_drop_held_profession_ex_content(
        tiles,
        held_id,
        held_uses,
        player_x,
        player_y,
        home_x,
        home_y,
        food_store,
        allow_all_piles,
        max_distance_to_home,
        held_contains_clay,
        is_moving,
        clothing,
        clothing_uses,
        None,
    )
}

/// Profession DropHeld with ContentDb maxNewActor `trans.newActorID` count.
// Haxe: shortCraftOnTarget GetTransition + CountCloseObjects(newActorID, 30)
pub fn smart_drop_held_profession_ex_content(
    tiles: &[profession_scan::ScanTile],
    held_id: i32,
    held_uses: i32,
    player_x: i32,
    player_y: i32,
    home_x: i32,
    home_y: i32,
    food_store: f32,
    allow_all_piles: bool,
    max_distance_to_home: f32,
    held_contains_clay: bool,
    is_moving: bool,
    clothing: &[i32],
    clothing_uses: &[i32],
    content: Option<&ol_content::ContentDb>,
) -> ShortCraftLiveIntent {
    let mut extras = drop_held_ai::DropHeldSensorExtras::default();
    extras.held_contains_clay = held_contains_clay;
    // Haxe: dropHeldObject → storeInQuiver clothingObjects scan (DROP-HELD-QUIVER)
    extras.quiver = drop_held_ai::quiver_from_clothing_snapshot(clothing, clothing_uses);
    drop_held_ai::smart_drop_held_from_sensors_ex(
        held_id,
        held_uses,
        player_x,
        player_y,
        home_x,
        home_y,
        food_store,
        is_moving,
        allow_all_piles,
        max_distance_to_home,
        tiles,
        extras,
        content,
    )
}

/// True when live intent needs walk/USE/DROP/SELF enqueue or hold-tick Wait.
// Haxe: dropHeldObject return true (including isMoving wait) — not refuse/none
#[inline]
pub fn drop_held_live_intent_actionable(intent: ShortCraftLiveIntent) -> bool {
    matches!(
        intent,
        ShortCraftLiveIntent::UseAt { .. }
            | ShortCraftLiveIntent::DropAt { .. }
            | ShortCraftLiveIntent::Goto { .. }
            | ShortCraftLiveIntent::SelfClothing { .. }
            | ShortCraftLiveIntent::UseOnEmptyGround { .. }
            | ShortCraftLiveIntent::SeekOrCraft { .. }
            // Haxe: isMoving return true — consume tick, do not fall through (PREFER-SHORT-WAIT)
            | ShortCraftLiveIntent::Wait
            | ShortCraftLiveIntent::Kill { .. }
    )
}

/// True when intent is a hold-tick (no wire, no seek) — BusyMoving / TIME wait.
// Haxe: dropHeldObject isMoving return true; craftItem TIME WaitTime
#[inline]
pub fn live_intent_is_wait(intent: ShortCraftLiveIntent) -> bool {
    matches!(intent, ShortCraftLiveIntent::Wait)
}

/// Convenience: pure shortCraft → live intent → apply when wire-capable.
// Haxe: shortCraftOnTarget held path
pub fn short_craft_intent_use(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    inp: ShortCraftInput,
    ctx: &ShortCraftIntentCtx,
) -> ShortCraftLiveApplyResult {
    let intent = short_craft_to_live_intent(inp, ctx);
    apply_short_craft_live_intent(state, outbound, conn_id, intent)
}

// ── Re-exports for hungry floor constants (tests / callers) ─────────────────

/// True when actor is a floor-place special (96/470/881).
#[inline]
pub fn is_floor_place_actor(actor_id: i32) -> bool {
    FLOOR_PLACE_ACTOR_IDS.contains(&actor_id)
}

// ── CRAFT-LIVE-TICK: profession world-scan submodule ────────────────────────
// Compiles even before crate-root `mod profession_scan` is wired; build may also
// add a crate-root alias. Prefer crate-root exports once build patch lands.

/// Profession world-scan → shortCraft USE/DROP (**CRAFT-LIVE-TICK**).
// Haxe: AiBase shortCraft + CountCloseObjects + doFarming/doSmithing/doBaking
#[path = "profession_scan.rs"]
pub mod profession_scan;

/// Smart dropHeldObject oven/forge/soil/quiver/piles (**DROP-HELD-AI**).
// Haxe: AiBase.dropHeldObject / storeInQuiver / UseUpDough
#[path = "drop_held_ai.rs"]
pub mod drop_held_ai;

// ── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baker_profession::{baker_short_craft_apply, HOT_OVEN, RAW_MUTTON};
    use crate::farmer_profession::{
        new_actor_count_with_held, short_craft_apply, short_craft_apply_resolved, BOWL_OF_SOIL,
        DYING_BUSH, SKEWER, WEAK_SKEWER,
    };
    use crate::smith_profession::{
        check_hungry_work_cost_by_id, check_hungry_work_cost_lookup,
        craft_and_drop_near_forge_apply, smith_action_apply, HungryWorkCostLookup, SmithAction,
        SmithApplyInput, FLAT_ROCK, HOT_IRON_BLOOM_FLAT, SMITHING_HAMMER,
    };

    #[test]
    fn use_held_staging_goto_when_far_use_when_close() {
        let st = UseHeldStaging::new(10, 10, 391, 253, false);
        assert_eq!(
            advance_use_held_staging(st, 391, 253, 0, 0, false, 0),
            UseHeldAdvance::Goto { x: 10, y: 10 }
        );
        assert_eq!(
            advance_use_held_staging(st, 391, 253, 10, 10, false, 0),
            UseHeldAdvance::UseNow { x: 10, y: 10 }
        );
        assert_eq!(
            advance_use_held_staging(st, 391, 253, 9, 10, false, 0),
            UseHeldAdvance::UseNow { x: 10, y: 10 }
        );
        // Diagonal: Chebyshev 1 would UseNow; Haxe quad 2 > 1 → Goto
        assert_eq!(
            advance_use_held_staging(st, 391, 253, 9, 9, false, 0),
            UseHeldAdvance::Goto { x: 10, y: 10 }
        );
        assert_eq!(
            advance_use_held_staging(st, 391, 253, 10, 10, true, 0),
            UseHeldAdvance::Wait
        );
    }

    #[test]
    fn use_held_staging_cancels_on_target_change_except_milkweed() {
        let st = UseHeldStaging::new(3, 4, 50, 0, false);
        assert_eq!(
            advance_use_held_staging(st, 51, 0, 3, 4, false, 0),
            UseHeldAdvance::UseNow { x: 3, y: 4 }
        );
        let bush = UseHeldStaging::new(3, 4, 391, 253, false);
        assert_eq!(
            advance_use_held_staging(bush, 82, 253, 3, 4, false, 0),
            UseHeldAdvance::Cancel
        );
        assert_eq!(
            advance_use_held_staging(bush, 0, 253, 3, 4, false, 0),
            UseHeldAdvance::Cancel
        );
    }

    #[test]
    fn use_held_staging_wrong_actor_and_container() {
        let st = UseHeldStaging::new(1, 1, 292, 126, false);
        assert_eq!(
            advance_use_held_staging(st, 292, 33, 1, 1, false, 0),
            UseHeldAdvance::Cancel
        );
        let empty_actor = UseHeldStaging::new(1, 1, 357, 0, false);
        assert_eq!(
            advance_use_held_staging(empty_actor, 357, 33, 1, 1, false, 0),
            UseHeldAdvance::DropHeld
        );
        assert_eq!(
            advance_use_held_staging(st, 292, 126, 1, 1, false, 2),
            UseHeldAdvance::Cancel
        );
        let drop_in = UseHeldStaging::new(1, 1, 292, 126, true);
        assert_eq!(
            advance_use_held_staging(drop_in, 292, 126, 1, 1, false, 2),
            UseHeldAdvance::UseNow { x: 1, y: 1 }
        );
    }

    #[test]
    fn stage_use_held_respects_hungry_and_reachable() {
        use crate::smith_profession::HungryWorkCostLookup;
        let lu = HungryWorkCostLookup::from_transition_cost(2.0);
        assert!(stage_use_held_on_target(1, 1, 391, 253, false, true, 5.0, Some(&lu)).is_some());
        assert!(stage_use_held_on_target(1, 1, 391, 253, false, true, 0.5, Some(&lu)).is_none());
        assert!(stage_use_held_on_target(1, 1, 391, 253, false, false, 5.0, Some(&lu)).is_none());
    }

    #[test]
    fn short_craft_intent_use_soil_on_dying_bush() {
        let inp = ShortCraftInput {
            try_weak_skewer_first: false,
            ..ShortCraftInput::basic(BOWL_OF_SOIL, BOWL_OF_SOIL, DYING_BUSH)
        };
        let ctx = ShortCraftIntentCtx::at_target(10, 12);
        let intent = short_craft_to_live_intent(inp, &ctx);
        assert_eq!(
            intent,
            ShortCraftLiveIntent::UseAt {
                x: 10,
                y: 12,
                target_id: DYING_BUSH,
                actor_id: BOWL_OF_SOIL,
            }
        );
        assert!(intent.is_wire_action());
        assert_eq!(intent.use_xy(), Some((10, 12)));
    }

    #[test]
    fn short_craft_intent_drop_held() {
        let inp = ShortCraftInput {
            try_weak_skewer_first: false,
            ..ShortCraftInput::basic(BOWL_OF_SOIL, 0, DYING_BUSH)
        };
        let mut ctx = ShortCraftIntentCtx::at_target(5, 5);
        ctx.empty_drop_x = 6;
        ctx.empty_drop_y = 7;
        let intent = short_craft_to_live_intent(inp, &ctx);
        assert_eq!(intent, ShortCraftLiveIntent::DropAt { x: 6, y: 7 });
        assert_eq!(intent.drop_xy(), Some((6, 7)));
    }

    #[test]
    fn short_craft_hungry_refuse_farm() {
        let inp = ShortCraftInput {
            try_weak_skewer_first: false,
            food_store: 0.5,
            transition_hungry_cost: 2.0,
            ..ShortCraftInput::basic(BOWL_OF_SOIL, BOWL_OF_SOIL, DYING_BUSH)
        };
        let ctx = ShortCraftIntentCtx::at_target(1, 1);
        assert_eq!(
            short_craft_to_live_intent(inp, &ctx),
            ShortCraftLiveIntent::RefuseHungry
        );
        // food gate helper
        assert!(!check_hungry_work_cost_by_id(0.5, 2.0));
        assert!(check_hungry_work_cost_by_id(5.0, 2.0));
    }

    #[test]
    fn short_craft_weak_skewer_reenter() {
        // Holding weak skewer → Use with 852 (Haxe tries 852 first and succeeds).
        let inp = ShortCraftInput {
            try_weak_skewer_first: true,
            ..ShortCraftInput::basic(WEAK_SKEWER, SKEWER, DYING_BUSH)
        };
        assert_eq!(short_craft_apply(inp), ShortCraftApply::PreferWeakSkewer);
        assert_eq!(
            short_craft_apply_resolved(inp),
            ShortCraftApply::UseOnTarget {
                actor: WEAK_SKEWER,
                target: DYING_BUSH,
            }
        );
        let ctx = ShortCraftIntentCtx::at_target(3, 4);
        assert_eq!(
            short_craft_to_live_intent(inp, &ctx),
            ShortCraftLiveIntent::UseAt {
                x: 3,
                y: 4,
                target_id: DYING_BUSH,
                actor_id: WEAK_SKEWER,
            }
        );

        // Not holding weak or skewer → weak path seeks 852 craft=false, then falls to 139 seek.
        let seek_inp = ShortCraftInput {
            try_weak_skewer_first: true,
            ..ShortCraftInput::basic(0, SKEWER, DYING_BUSH)
        };
        // Weak first: SeekOrCraft 852 craft=false (success) — Haxe returns true on GetItem start.
        assert_eq!(
            short_craft_apply_resolved(seek_inp),
            ShortCraftApply::SeekOrCraftActor {
                actor: WEAK_SKEWER,
                craft_if_needed: false,
            }
        );
    }

    #[test]
    fn short_craft_max_new_actor_with_held() {
        assert_eq!(new_actor_count_with_held(3, 100, 100), 4);
        assert_eq!(new_actor_count_with_held(3, 50, 100), 3);
        let count = new_actor_count_with_held(4, 999, 999);
        let refused = short_craft_apply(ShortCraftInput {
            try_weak_skewer_first: false,
            new_actor_count: count,
            max_new_actor: 4,
            ..ShortCraftInput::basic(BOWL_OF_SOIL, BOWL_OF_SOIL, DYING_BUSH)
        });
        // near=4 + held=1 → 5 >= 4 refuse; but we used count=5 from held
        assert_eq!(count, 5);
        assert_eq!(refused, ShortCraftApply::Refuse);
        // 3 near + held matching → 4 at cap refuse
        let at_cap = new_actor_count_with_held(3, 200, 200);
        assert_eq!(at_cap, 4);
        assert_eq!(
            short_craft_apply(ShortCraftInput {
                try_weak_skewer_first: false,
                new_actor_count: at_cap,
                max_new_actor: 4,
                ..ShortCraftInput::basic(BOWL_OF_SOIL, BOWL_OF_SOIL, DYING_BUSH)
            }),
            ShortCraftApply::Refuse
        );
        // 2 near + held → 3 allow USE
        let under = new_actor_count_with_held(2, 200, 200);
        assert_eq!(
            short_craft_apply(ShortCraftInput {
                try_weak_skewer_first: false,
                new_actor_count: under,
                max_new_actor: 4,
                ..ShortCraftInput::basic(BOWL_OF_SOIL, BOWL_OF_SOIL, DYING_BUSH)
            }),
            ShortCraftApply::UseOnTarget {
                actor: BOWL_OF_SOIL,
                target: DYING_BUSH,
            }
        );
    }

    #[test]
    fn short_craft_on_ground_336_prefers_well_home() {
        // Prefer well empty over arbitrary empty.
        assert_eq!(
            pick_ground_use_tile(BASKET_OF_SOIL, Some((1, 2)), Some((3, 4)), Some((9, 9))),
            Some((1, 2))
        );
        // No well → home
        assert_eq!(
            pick_ground_use_tile(BASKET_OF_SOIL, None, Some((3, 4)), Some((9, 9))),
            Some((3, 4))
        );
        // Neither → any empty
        assert_eq!(
            pick_ground_use_tile(BASKET_OF_SOIL, None, None, Some((9, 9))),
            Some((9, 9))
        );
        // Non-336 uses any empty only
        assert_eq!(
            pick_ground_use_tile(100, Some((1, 2)), Some((3, 4)), Some((9, 9))),
            Some((9, 9))
        );

        let ctx =
            ShortCraftIntentCtx::at_target(0, 0).with_soil_anchors(Some((8, 8)), Some((2, 2)));
        let mut ctx = ctx;
        ctx.empty_drop_x = 99;
        ctx.empty_drop_y = 99;
        let intent = short_craft_on_ground_to_live_intent(BASKET_OF_SOIL, BASKET_OF_SOIL, &ctx);
        assert_eq!(
            intent,
            ShortCraftLiveIntent::UseOnEmptyGround {
                x: 8,
                y: 8,
                held: BASKET_OF_SOIL,
            }
        );
    }

    #[test]
    fn smith_action_apply_to_net_intent_drop_near_forge_and_use() {
        let sc = SmithAction::ShortCraft {
            actor: SMITHING_HAMMER,
            target: HOT_IRON_BLOOM_FLAT,
        };
        let apply = smith_action_apply(sc, &SmithApplyInput::basic(SMITHING_HAMMER));
        let ctx = ShortCraftIntentCtx::at_target(20, 21);
        assert_eq!(
            smith_apply_to_live_intent(apply, &ctx),
            ShortCraftLiveIntent::UseAt {
                x: 20,
                y: 21,
                target_id: HOT_IRON_BLOOM_FLAT,
                actor_id: SMITHING_HAMMER,
            }
        );

        // DropNearForge → DROP at forge coords
        let drop_apply = craft_and_drop_near_forge_apply(FLAT_ROCK, 2, 0, FLAT_ROCK, 2, false);
        assert!(matches!(drop_apply, SmithApply::DropNearForge { .. }));
        let mut forge_ctx = ShortCraftIntentCtx::at_target(0, 0);
        forge_ctx.forge_x = 15;
        forge_ctx.forge_y = 16;
        assert_eq!(
            smith_apply_to_live_intent(drop_apply, &forge_ctx),
            ShortCraftLiveIntent::DropAt { x: 15, y: 16 }
        );

        // UseOnEmptyGround
        let ground = short_craft_on_ground_apply(FLAT_ROCK, FLAT_ROCK);
        assert_eq!(
            smith_apply_to_live_intent(ground, &forge_ctx),
            ShortCraftLiveIntent::UseOnEmptyGround {
                x: forge_ctx.empty_drop_x,
                y: forge_ctx.empty_drop_y,
                held: FLAT_ROCK,
            }
        );
    }

    #[test]
    fn bake_mutton_max_new_actor_four() {
        // 4 nearby cooked → refuse; 3 allow UseOnTarget
        assert_eq!(
            baker_short_craft_apply(RAW_MUTTON, RAW_MUTTON, HOT_OVEN, 4),
            ShortCraftApply::Refuse
        );
        assert_eq!(
            baker_short_craft_apply(RAW_MUTTON, RAW_MUTTON, HOT_OVEN, 3),
            ShortCraftApply::UseOnTarget {
                actor: RAW_MUTTON,
                target: HOT_OVEN,
            }
        );
        let ctx = ShortCraftIntentCtx::at_target(4, 5);
        let ok = short_craft_apply_to_live_intent(
            baker_short_craft_apply(RAW_MUTTON, RAW_MUTTON, HOT_OVEN, 3),
            &ctx,
        );
        assert_eq!(
            ok,
            ShortCraftLiveIntent::UseAt {
                x: 4,
                y: 5,
                target_id: HOT_OVEN,
                actor_id: RAW_MUTTON,
            }
        );
    }

    #[test]
    fn hungry_work_lookup_floor_and_container() {
        // No transition, floor place actor 96 on floor-allow target
        let floor = HungryWorkCostLookup {
            transition_found: false,
            transition_hungry_cost: 0.0,
            target_allow_floor: true,
            target_num_slots: 0,
            held_containable: false,
            use_is_drop_in_container: false,
        };
        assert!(check_hungry_work_cost_lookup(96, 0.0, &floor));
        assert!(!check_hungry_work_cost_lookup(1, 0.0, &floor)); // not floor actor

        // Container drop allow without transition
        let cont = HungryWorkCostLookup {
            transition_found: false,
            transition_hungry_cost: 0.0,
            target_allow_floor: false,
            target_num_slots: 4,
            held_containable: true,
            use_is_drop_in_container: true,
        };
        assert!(check_hungry_work_cost_lookup(33, 0.0, &cont));

        // Transition with cost
        let tr = HungryWorkCostLookup::from_transition_cost(3.0);
        assert!(!check_hungry_work_cost_lookup(1, 2.0, &tr));
        assert!(check_hungry_work_cost_lookup(1, 5.0, &tr));

        assert!(is_floor_place_actor(470));
        assert!(!is_floor_place_actor(1));
    }

    #[test]
    fn unreachable_target_suppresses_use() {
        let mut ctx = ShortCraftIntentCtx::at_target(1, 1);
        ctx.target_reachable = false;
        let apply = ShortCraftApply::UseOnTarget {
            actor: BOWL_OF_SOIL,
            target: DYING_BUSH,
        };
        assert_eq!(
            short_craft_apply_to_live_intent(apply, &ctx),
            ShortCraftLiveIntent::None
        );
    }

    #[test]
    fn craft_actor_if_needed_false_on_seek() {
        let inp = ShortCraftInput {
            try_weak_skewer_first: false,
            craft_actor_if_needed: false,
            ..ShortCraftInput::basic(0, BOWL_OF_SOIL, DYING_BUSH)
        };
        assert_eq!(
            short_craft_apply(inp),
            ShortCraftApply::SeekOrCraftActor {
                actor: BOWL_OF_SOIL,
                craft_if_needed: false,
            }
        );
        let ctx = ShortCraftIntentCtx::at_target(0, 0);
        assert_eq!(
            short_craft_apply_to_live_intent(short_craft_apply(inp), &ctx),
            ShortCraftLiveIntent::SeekOrCraft {
                actor: BOWL_OF_SOIL,
                craft_if_needed: false,
            }
        );
    }

    #[test]
    fn wait_intent_is_actionable_hold_tick() {
        // Haxe: isMoving return true — had_action, no wire
        assert!(drop_held_live_intent_actionable(ShortCraftLiveIntent::Wait));
        assert!(live_intent_is_wait(ShortCraftLiveIntent::Wait));
        assert!(!ShortCraftLiveIntent::Wait.is_wire_action());
        assert!(!drop_held_live_intent_actionable(
            ShortCraftLiveIntent::None
        ));
    }
}
