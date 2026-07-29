//! Player sticky multi-tick craft runtime (**AI-CRAFT-STICKY** / craft_runtime).
//!
//! Haxe `AiBase.itemToCraft` + `failedCraftings` + `itemToCraftId` + `craftingTasks`
//! + `lastActorId` + `calledCraftItem` + `itemToCraftName` live on the AI player across ticks.
//! Pure multi-step decisions stay in [`crate::get_or_craft::craft_item`]; this module
//! owns the session sticky shell on [`crate::Player`].
//!
//! // Haxe: AiBase.itemToCraft / failedCraftings / itemToCraftId / craftingTasks
//! // Haxe: AiBase.newBorn clears failedCraftings + itemToCraft
//! // Haxe: craftItemHelper product-change → addTask when countDone < count
//! // Haxe: doTimeStuffHelper sticky continue + craftingTasks ~667–680
//! // Haxe: USE done countDone / countTransitionsDone ~9077–9089
//! // Haxe: doMakeCraftCommand itemToCraftName ~8339

use crate::craft_graph::ReverseCraftGraph;
use crate::get_or_craft::craft_item::{
    craft_item_with_runtime, CraftAiRuntime, CraftItemDecision, CraftLiveExpandOpts,
    CraftScanFilters, CraftWorldObj,
    FailedCraftings, ItemToCraftState,
};
use crate::get_or_craft::{
    expand_craft_item_live_sticky_scan, resolve_seek_or_craft_live_ex, GetOrCraftWorldObj,
};
use crate::short_craft_intent::ShortCraftLiveIntent;

/// Sticky multi-tick craft state stored on [`crate::Player`].
// Haxe: AiBase.itemToCraft + failedCraftings + itemToCraftId + craftingTasks + lastActorId + calledCraftItem + itemToCraftName
#[derive(Debug, Clone)]
pub struct PlayerCraftAi {
    /// Multi-step IntemToCraft + failedCraftings + lastActorId + calledCraftItem.
    pub runtime: CraftAiRuntime,
    /// Haxe `itemToCraftId` — active product task id (`-1` = none).
    pub item_to_craft_id: i32,
    /// Haxe `craftingTasks` — interrupted / queued product ids.
    pub crafting_tasks: Vec<i32>,
    /// Haxe `itemToCraftName` — human MAKE order name (say Making/Failed/Finished).
    pub item_to_craft_name: Option<String>,
}

impl Default for PlayerCraftAi {
    fn default() -> Self {
        Self::new()
    }
}

/// Outcome of a successful USE that was tracked against sticky craft progress.
// Haxe: AiBase use-done path ~9075–9089
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteUseOutcome {
    /// No active sticky product — ignored.
    Ignored,
    /// Transition recorded; product not yet held/on ground.
    TransitionOnly,
    /// Product parent appeared held or on ground → `count_done` incremented.
    ProductCountInc {
        count_done: i32,
        /// Cleared human order name when finished-say would fire (Haxe itemToCraftName).
        finished_name: Option<String>,
    },
}

/// Priority-ladder sensor flags derived from sticky craft state.
// Haxe: doTimeStuffHelper itemToCraftId continue + craftingTasks.length > 0 ~667–680
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StickyCraftSensorFlags {
    /// Unfinished sticky product (`itemToCraftId > 0 && countDone < count`).
    pub unfinished_sticky: bool,
    /// Queued interrupted products (`craftingTasks` non-empty).
    pub has_craft_queue: bool,
}

impl StickyCraftSensorFlags {
    /// True when either unfinished sticky or queue should drive CraftQueue band.
    pub fn any_craft_work(self) -> bool {
        self.unfinished_sticky || self.has_craft_queue
    }
}

/// What sticky craft product to pursue this AI tick (after begin_tick).
// Haxe: doTimeStuffHelper ~667–680
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StickyCraftTickChoice {
    None,
    /// Continue unfinished `itemToCraftId`.
    Continue { product_id: i32 },
    /// Shifted from `craftingTasks` (re-push on fail via [`PlayerCraftAi::requeue_current_task`]).
    FromQueue { product_id: i32 },
}

impl StickyCraftTickChoice {
    pub fn product_id(self) -> Option<i32> {
        match self {
            Self::None => None,
            Self::Continue { product_id } | Self::FromQueue { product_id } => Some(product_id),
        }
    }

    pub fn from_queue(self) -> bool {
        matches!(self, Self::FromQueue { .. })
    }
}

impl PlayerCraftAi {
    pub fn new() -> Self {
        Self {
            runtime: CraftAiRuntime::new(),
            item_to_craft_id: -1,
            crafting_tasks: Vec::new(),
            item_to_craft_name: None,
        }
    }

    /// Haxe `newBorn` — wipe sticky craft state on birth / reset.
    // Haxe: AiBase.newBorn → itemToCraftId=-1; itemToCraft=new; failedCraftings=new Map
    // Delta: also clears craftingTasks + lastActorId + name (stricter; clean new life).
    pub fn wipe_on_birth(&mut self) {
        *self = Self::new();
    }

    /// Haxe `resetTargets` — clear sticky trans actor/target only.
    // Haxe: AiBase.resetTargets → itemToCraft.transActor/transTarget = null
    pub fn reset_targets(&mut self) {
        self.runtime.item.clear_trans();
    }

    /// Haxe `calledCraftItem = false` each doTimeStuff entry.
    pub fn begin_tick(&mut self) {
        self.runtime.clear_tick_guard();
    }

    /// Haxe `addTask(taskId, atEnd)`.
    // Haxe: AiBase.addTask — skip <1 and duplicates
    pub fn add_task(&mut self, task_id: i32, at_end: bool) {
        if task_id < 1 {
            return;
        }
        if self.crafting_tasks.contains(&task_id) {
            return;
        }
        if at_end {
            self.crafting_tasks.push(task_id);
        } else {
            self.crafting_tasks.insert(0, task_id);
        }
    }

    /// True when sticky product still has unfinished count (continue craft path).
    // Haxe: itemToCraftId > 0 && itemToCraft.countDone < itemToCraft.count
    // Also true when id set but IntemToCraft not yet bound (doMakeCraft / first tick; Haxe count defaults 0).
    pub fn should_continue_sticky_craft(&self) -> bool {
        if self.item_to_craft_id <= 0 {
            return false;
        }
        if self.runtime.item.product_id != self.item_to_craft_id {
            return true;
        }
        // count==0 means craftItemHelper has not initialized this product yet.
        if self.runtime.item.count <= 0 {
            return true;
        }
        self.runtime.item.count_done < self.runtime.item.count
    }

    /// Sensor flags for priority ladder CraftQueue band.
    // Haxe: itemToCraftId continue + craftingTasks.length
    pub fn sticky_craft_sensor_flags(&self) -> StickyCraftSensorFlags {
        StickyCraftSensorFlags {
            unfinished_sticky: self.should_continue_sticky_craft(),
            has_craft_queue: !self.crafting_tasks.is_empty(),
        }
    }

    /// Prepare sticky for a new `product_id` (re-queue interrupted prior product).
    // Haxe: craftItemHelper when itemToCraft.itemToCraft.parentId != objId ~6677–6690
    pub fn prepare_for_product(&mut self, product_id: i32) {
        let prev = self.runtime.item.product_id;
        if prev > 0 && prev != product_id {
            // Interrupted unfinished craft → re-queue prior product.
            if self.runtime.item.count_done < self.runtime.item.count {
                self.add_task(prev, true);
            }
        }
        if product_id > 0 {
            // Reset IntemToCraft fields so stale trans pair cannot linger without craft_item_helper.
            if self.runtime.item.product_id != product_id {
                self.runtime.item.reset_for_product(product_id);
            }
            self.item_to_craft_id = product_id;
        }
    }

    /// Set active craft id without re-queue (e.g. craving path).
    pub fn set_item_to_craft_id(&mut self, id: i32) {
        self.item_to_craft_id = if id > 0 { id } else { -1 };
    }

    /// Haxe `doMakeCraftCommand` — set sticky product from human MAKE order.
    // Haxe: AiBase.doMakeCraftCommand ~8339–8348
    /// Returns optional say text `"Making {name}"` when `name` is provided and not silent.
    pub fn do_make_craft_command(
        &mut self,
        product_id: i32,
        name: Option<String>,
        silent: bool,
    ) -> Option<String> {
        if product_id <= 0 {
            return None;
        }
        self.item_to_craft_id = product_id;
        self.item_to_craft_name = name.clone();
        if silent {
            return None;
        }
        name.map(|n| format!("Making {n}"))
    }

    /// Pop next queued craft task into `item_to_craft_id` (round-robin style).
    // Haxe: craftingTasks.shift + craftItem + push back on fail path
    pub fn take_next_crafting_task(&mut self) -> Option<i32> {
        if self.crafting_tasks.is_empty() {
            return None;
        }
        let id = self.crafting_tasks.remove(0);
        self.item_to_craft_id = id;
        Some(id)
    }

    /// Re-queue current task at end (Haxe failed craftItem still re-pushes).
    pub fn requeue_current_task(&mut self) {
        let id = self.item_to_craft_id;
        if id > 0 {
            self.add_task(id, true);
        }
    }

    /// On craft fail: re-queue if taken from queue, clear human name with Failed say.
    // Haxe: craftItemHelper fail say + craftingTasks.push ~6732 / ~679
    pub fn on_craft_fail_from_choice(&mut self, choice: StickyCraftTickChoice) -> Option<String> {
        if choice.from_queue() {
            self.requeue_current_task();
        }
        // Haxe: if (itemToCraftName != null) say Failed; clear
        self.item_to_craft_name
            .take()
            .map(|n| format!("Failed to craft {n}"))
    }

    /// Haxe USE-done path: bookkeeping + countDone when product appears held/ground.
    // Haxe: AiBase ~9075–9089
    pub fn note_successful_use(
        &mut self,
        use_actor_id: i32,
        use_target_id: i32,
        held_parent_after: i32,
        ground_parent_after: i32,
    ) -> NoteUseOutcome {
        let product = if self.runtime.item.product_id > 0 {
            self.runtime.item.product_id
        } else if self.item_to_craft_id > 0 {
            self.item_to_craft_id
        } else {
            return NoteUseOutcome::Ignored;
        };

        // Haxe: itemToCraft.done = true; countTransitionsDone += 1; last* ids
        self.runtime.item.count_transitions_done =
            self.runtime.item.count_transitions_done.saturating_add(1);
        self.runtime.item.last_actor_id = use_actor_id;
        self.runtime.item.last_target_id = use_target_id;
        self.runtime.item.last_new_actor_id = held_parent_after;
        self.runtime.item.last_new_target_id = ground_parent_after;
        self.runtime.item.clear_trans();

        // if object to create is held by player or is on ground, then count as done
        if held_parent_after == product || ground_parent_after == product {
            self.runtime.item.count_done = self.runtime.item.count_done.saturating_add(1);
            // Haxe: Finished say when name matches; clear name either way when product done + name set
            let finished_name = self.item_to_craft_name.take();
            NoteUseOutcome::ProductCountInc {
                count_done: self.runtime.item.count_done,
                finished_name,
            }
        } else {
            NoteUseOutcome::TransitionOnly
        }
    }

    /// Accessors for pure craft_item_with_runtime.
    pub fn runtime_mut(&mut self) -> &mut CraftAiRuntime {
        &mut self.runtime
    }

    pub fn runtime(&self) -> &CraftAiRuntime {
        &self.runtime
    }

    pub fn item(&self) -> &ItemToCraftState {
        &self.runtime.item
    }

    pub fn item_mut(&mut self) -> &mut ItemToCraftState {
        &mut self.runtime.item
    }
}

/// Begin tick + pick sticky continue or next crafting task (Haxe doTimeStuffHelper).
// Haxe: calledCraftItem=false; itemToCraftId continue; craftingTasks.shift
pub fn select_sticky_craft_for_tick(craft_ai: &mut PlayerCraftAi) -> StickyCraftTickChoice {
    craft_ai.begin_tick();
    if craft_ai.should_continue_sticky_craft() {
        return StickyCraftTickChoice::Continue {
            product_id: craft_ai.item_to_craft_id,
        };
    }
    if let Some(id) = craft_ai.take_next_crafting_task() {
        return StickyCraftTickChoice::FromQueue { product_id: id };
    }
    StickyCraftTickChoice::None
}

/// Merge sticky craft flags into ladder craft-queue / critical-craft sensors.
// Haxe: unfinished sticky before clothing craft; tasks → craft queue band
pub fn apply_sticky_flags_to_craft_sensors(
    unfinished_sticky: bool,
    has_task_queue: bool,
    critical_craft_pending: &mut bool,
    has_craft_queue: &mut bool,
) {
    // Unfinished sticky continues high in the craft band (same slot as Haxe pre-queue continue).
    if unfinished_sticky {
        *has_craft_queue = true;
    }
    if has_task_queue {
        *has_craft_queue = true;
    }
    // Do not force critical_craft_pending — reserved for smith early sticky.
    let _ = critical_craft_pending;
}

/// Sticky multi-tick craftItem on [`PlayerCraftAi`].
// Haxe: craftItem + itemToCraft / failedCraftings / itemToCraftId
pub fn craft_item_with_player_craft_ai(
    objs: &[CraftWorldObj],
    product_id: i32,
    player_x: i32,
    player_y: i32,
    held_id: i32,
    opts: &CraftLiveExpandOpts,
    craft_ai: &mut PlayerCraftAi,
    graph: &ReverseCraftGraph,
    pile_id_for: Option<&dyn Fn(i32) -> i32>,
) -> CraftItemDecision {
    craft_ai.prepare_for_product(product_id);
    craft_item_with_runtime(
        objs,
        product_id,
        player_x,
        player_y,
        held_id,
        opts,
        craft_ai.runtime_mut(),
        graph,
        pile_id_for,
    )
}

/// Expand CraftItem staging using player sticky craft runtime.
// Haxe: craftItem multi-step with sticky itemToCraft / failedCraftings
pub fn expand_craft_item_player_sticky(
    object_id: i32,
    objs: &[GetOrCraftWorldObj],
    player_x: i32,
    player_y: i32,
    held_id: i32,
    pile_id_for: &dyn Fn(i32) -> i32,
    empty_drop: Option<(i32, i32)>,
    graph: &ReverseCraftGraph,
    opts: &CraftLiveExpandOpts,
    craft_ai: &mut PlayerCraftAi,
) -> ShortCraftLiveIntent {
    expand_craft_item_player_sticky_scan(
        object_id,
        objs,
        player_x,
        player_y,
        held_id,
        pile_id_for,
        empty_drop,
        graph,
        opts,
        craft_ai,
        CraftScanFilters::default(),
    )
}

/// Sticky expand with path-reach / hostile scan filters.
// Haxe: craftItem + isObjectNotReachable / isObjectWithHostilePath (AI-CRAFT-LIVE-RESID)
pub fn expand_craft_item_player_sticky_scan(
    object_id: i32,
    objs: &[GetOrCraftWorldObj],
    player_x: i32,
    player_y: i32,
    held_id: i32,
    pile_id_for: &dyn Fn(i32) -> i32,
    empty_drop: Option<(i32, i32)>,
    graph: &ReverseCraftGraph,
    opts: &CraftLiveExpandOpts,
    craft_ai: &mut PlayerCraftAi,
    scan: CraftScanFilters<'_>,
) -> ShortCraftLiveIntent {
    craft_ai.prepare_for_product(object_id);
    expand_craft_item_live_sticky_scan(
        object_id,
        objs,
        player_x,
        player_y,
        held_id,
        pile_id_for,
        empty_drop,
        graph,
        opts,
        craft_ai.runtime_mut(),
        scan,
    )
}

/// Resolve SeekOrCraft / CraftItem with player sticky runtime.
// Haxe: craftItem + failedCraftings / itemToCraft sticky across ticks
pub fn resolve_seek_or_craft_player_sticky(
    intent: ShortCraftLiveIntent,
    objs: &[GetOrCraftWorldObj],
    player_x: i32,
    player_y: i32,
    held_id: i32,
    target: Option<(i32, i32)>,
    pile_id_for: &dyn Fn(i32) -> i32,
    empty_drop: Option<(i32, i32)>,
    graph: Option<&ReverseCraftGraph>,
    have: Option<&std::collections::HashSet<i32>>,
    opts: &CraftLiveExpandOpts,
    craft_ai: &mut PlayerCraftAi,
) -> ShortCraftLiveIntent {
    if let ShortCraftLiveIntent::CraftItem { object_id } = intent {
        if object_id > 0 {
            craft_ai.prepare_for_product(object_id);
        }
    }
    resolve_seek_or_craft_live_ex(
        intent,
        objs,
        player_x,
        player_y,
        held_id,
        target,
        pile_id_for,
        empty_drop,
        graph,
        have,
        opts,
        Some(craft_ai.runtime_mut()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_graph() -> ReverseCraftGraph {
        let mut g = ReverseCraftGraph::new();
        g.insert(1, 2, 3, 0);
        g.insert(3, 4, 5, 0);
        g
    }

    #[test]
    fn newborn_wipes_failed_and_tasks() {
        let mut ai = PlayerCraftAi::new();
        ai.item_to_craft_id = 99;
        ai.item_to_craft_name = Some("Bow".into());
        ai.add_task(5, true);
        ai.runtime.failed.record_fail(5, 10.0);
        ai.runtime.item = ItemToCraftState::new(5);
        ai.runtime.item.count_done = 1;
        ai.wipe_on_birth();
        assert_eq!(ai.item_to_craft_id, -1);
        assert!(ai.crafting_tasks.is_empty());
        assert!(ai.runtime.failed.last_fail_sec.is_empty());
        assert_eq!(ai.runtime.item.product_id, 0);
        assert_eq!(ai.runtime.last_actor_id, -1);
        assert!(ai.item_to_craft_name.is_none());
    }

    #[test]
    fn add_task_dedup_and_order() {
        let mut ai = PlayerCraftAi::new();
        ai.add_task(0, true); // ignored
        ai.add_task(10, true);
        ai.add_task(10, true); // dedup
        ai.add_task(20, false); // front
        assert_eq!(ai.crafting_tasks, vec![20, 10]);
    }

    #[test]
    fn prepare_for_product_requeues_interrupted() {
        let mut ai = PlayerCraftAi::new();
        ai.runtime.item = ItemToCraftState::new(7);
        ai.runtime.item.count = 2;
        ai.runtime.item.count_done = 0; // unfinished
        ai.runtime.item.trans_actor_id = Some(1);
        ai.item_to_craft_id = 7;
        ai.prepare_for_product(11);
        assert_eq!(ai.item_to_craft_id, 11);
        assert!(ai.crafting_tasks.contains(&7));
        // Product change resets stale trans pair.
        assert_eq!(ai.runtime.item.product_id, 11);
        assert!(ai.runtime.item.trans_actor_id.is_none());
        assert_eq!(ai.runtime.item.count_done, 0);
    }

    #[test]
    fn prepare_skips_requeue_when_count_done() {
        let mut ai = PlayerCraftAi::new();
        ai.runtime.item = ItemToCraftState::new(7);
        ai.runtime.item.count = 1;
        ai.runtime.item.count_done = 1; // finished
        ai.prepare_for_product(11);
        assert!(!ai.crafting_tasks.contains(&7));
        assert_eq!(ai.item_to_craft_id, 11);
    }

    #[test]
    fn sticky_failed_cooldown_across_player_craft_ai() {
        let g = sample_graph();
        let mut ai = PlayerCraftAi::new();
        let opts = CraftLiveExpandOpts::default().with_now(100.0);
        let d1 = craft_item_with_player_craft_ai(&[], 5, 0, 0, 0, &opts, &mut ai, &g, None);
        assert!(matches!(
            d1,
            CraftItemDecision::Failed | CraftItemDecision::SeekIngredient { .. }
        ));
        assert_eq!(ai.item_to_craft_id, 5);
        // Force fail record if seek path
        if !matches!(d1, CraftItemDecision::Failed) {
            ai.runtime.failed.record_fail(5, 100.0);
        }
        let opts2 = CraftLiveExpandOpts::default().with_now(110.0);
        let d2 = craft_item_with_player_craft_ai(&[], 5, 0, 0, 0, &opts2, &mut ai, &g, None);
        assert_eq!(d2, CraftItemDecision::Cooldown);
    }

    #[test]
    fn should_continue_when_count_incomplete() {
        let mut ai = PlayerCraftAi::new();
        ai.item_to_craft_id = 3;
        ai.runtime.item = ItemToCraftState::new(3);
        ai.runtime.item.count = 2;
        ai.runtime.item.count_done = 0;
        assert!(ai.should_continue_sticky_craft());
        ai.runtime.item.count_done = 2;
        assert!(!ai.should_continue_sticky_craft());
    }

    #[test]
    fn take_next_crafting_task_shifts_queue() {
        let mut ai = PlayerCraftAi::new();
        ai.add_task(3, true);
        ai.add_task(5, true);
        assert_eq!(ai.take_next_crafting_task(), Some(3));
        assert_eq!(ai.item_to_craft_id, 3);
        assert_eq!(ai.crafting_tasks, vec![5]);
    }

    #[test]
    fn failed_craftings_type_alias_surface() {
        // Ensure FailedCraftings remains usable from sticky shell.
        let mut f = FailedCraftings::new();
        f.record_fail(1, 0.0);
        assert!(f.is_cooling_down(1, 1.0));
    }

    #[test]
    fn note_successful_use_increments_count_done_when_product_held() {
        let mut ai = PlayerCraftAi::new();
        ai.item_to_craft_id = 83;
        ai.item_to_craft_name = Some("Fire".into());
        ai.runtime.item = ItemToCraftState::new(83);
        ai.runtime.item.count = 2;
        ai.runtime.item.count_done = 0;
        let o = ai.note_successful_use(71, 72, 83, 0);
        assert_eq!(
            o,
            NoteUseOutcome::ProductCountInc {
                count_done: 1,
                finished_name: Some("Fire".into()),
            }
        );
        assert_eq!(ai.runtime.item.count_done, 1);
        assert_eq!(ai.runtime.item.count_transitions_done, 1);
        assert_eq!(ai.runtime.item.last_actor_id, 71);
        assert_eq!(ai.runtime.item.last_new_actor_id, 83);
        assert!(ai.item_to_craft_name.is_none());
        assert!(ai.should_continue_sticky_craft());
        // Second unit completes sticky continue.
        let o2 = ai.note_successful_use(71, 72, 83, 0);
        assert_eq!(
            o2,
            NoteUseOutcome::ProductCountInc {
                count_done: 2,
                finished_name: None,
            }
        );
        assert!(!ai.should_continue_sticky_craft());
    }

    #[test]
    fn note_successful_use_transition_only_when_intermediate() {
        let mut ai = PlayerCraftAi::new();
        ai.item_to_craft_id = 83;
        ai.runtime.item = ItemToCraftState::new(83);
        ai.runtime.item.count = 1;
        let o = ai.note_successful_use(71, 72, 80, 78); // intermediate
        assert_eq!(o, NoteUseOutcome::TransitionOnly);
        assert_eq!(ai.runtime.item.count_done, 0);
        assert_eq!(ai.runtime.item.count_transitions_done, 1);
        assert_eq!(ai.runtime.item.last_new_target_id, 78);
    }

    #[test]
    fn note_successful_use_product_on_ground() {
        let mut ai = PlayerCraftAi::new();
        ai.item_to_craft_id = 5;
        ai.runtime.item = ItemToCraftState::new(5);
        let o = ai.note_successful_use(1, 2, 0, 5);
        assert!(matches!(o, NoteUseOutcome::ProductCountInc { count_done: 1, .. }));
    }

    #[test]
    fn do_make_craft_command_sets_id_and_name() {
        let mut ai = PlayerCraftAi::new();
        let say = ai.do_make_craft_command(152, Some("Bow and Arrow".into()), false);
        assert_eq!(ai.item_to_craft_id, 152);
        assert_eq!(ai.item_to_craft_name.as_deref(), Some("Bow and Arrow"));
        assert_eq!(say.as_deref(), Some("Making Bow and Arrow"));
        let silent = ai.do_make_craft_command(71, Some("Hatchet".into()), true);
        assert!(silent.is_none());
        assert_eq!(ai.item_to_craft_id, 71);
    }

    #[test]
    fn select_sticky_prefers_continue_then_queue() {
        let mut ai = PlayerCraftAi::new();
        ai.runtime.called_craft_item = true;
        ai.item_to_craft_id = 10;
        ai.runtime.item = ItemToCraftState::new(10);
        ai.runtime.item.count = 2;
        ai.runtime.item.count_done = 0;
        ai.add_task(20, true);
        let c = select_sticky_craft_for_tick(&mut ai);
        assert_eq!(c, StickyCraftTickChoice::Continue { product_id: 10 });
        assert!(!ai.runtime.called_craft_item); // begin_tick cleared guard
        // Finish sticky
        ai.runtime.item.count_done = 2;
        let c2 = select_sticky_craft_for_tick(&mut ai);
        assert_eq!(c2, StickyCraftTickChoice::FromQueue { product_id: 20 });
        assert_eq!(ai.item_to_craft_id, 20);
        assert!(ai.crafting_tasks.is_empty());
    }

    #[test]
    fn on_craft_fail_requeues_from_queue_and_clears_name() {
        let mut ai = PlayerCraftAi::new();
        ai.item_to_craft_id = 9;
        ai.item_to_craft_name = Some("Rope".into());
        let choice = StickyCraftTickChoice::FromQueue { product_id: 9 };
        let say = ai.on_craft_fail_from_choice(choice);
        assert_eq!(say.as_deref(), Some("Failed to craft Rope"));
        assert!(ai.crafting_tasks.contains(&9));
        assert!(ai.item_to_craft_name.is_none());
    }

    #[test]
    fn sticky_sensor_flags_for_ladder() {
        let mut ai = PlayerCraftAi::new();
        assert!(!ai.sticky_craft_sensor_flags().any_craft_work());
        ai.add_task(3, true);
        assert!(ai.sticky_craft_sensor_flags().has_craft_queue);
        ai.item_to_craft_id = 3;
        ai.runtime.item = ItemToCraftState::new(3);
        ai.runtime.item.count = 1;
        ai.runtime.item.count_done = 0;
        let f = ai.sticky_craft_sensor_flags();
        assert!(f.unfinished_sticky);
        assert!(f.any_craft_work());
        let mut crit = false;
        let mut q = false;
        apply_sticky_flags_to_craft_sensors(f.unfinished_sticky, f.has_craft_queue, &mut crit, &mut q);
        assert!(q);
        assert!(!crit); // smith-only critical not forced
    }
}
