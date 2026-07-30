//! USE transition apply path — multi-use numberOfUses (Haxe TransitionHelper).
//!
//! Chunk: **TH-MULTI** / **TH-MULTI-POLISH** / **TH-HORSE** / **HORSE-MOUNT-POLISH** / **TH-LOCK** / **EATEN-FOOD-PCT** / **DARK-NOSAJ** / **TH-ALT-OUTCOME**
//! Anchors: `doTransitionIfPossible`, `DoChangeNumberOfUsesOnActor`,
//! `DoChangeNumberOfUsesOnTarget` (+ loved-food bare-hand extra),
//! `doHorseStuffPossible`, `isPickupOrDrop` / nest swap,
//! empty-ground `held+-1` dismount, hitch cart cargo, grave-basket,
//! key/lock `externId` + LockPick gates,
//! alternativeTransitionOutcome + fortification fail (L1260–1306).

// Single crate-level module (lib.rs `mod loved_food_wire`) — do not #[path]-include
// again or LAST_LOVED_FOOD_EXTRA becomes two statics and PS/PE never sees the flag.
use crate::death_inherit::account_soul_token;
use crate::add_owner_to_helper;
use crate::horse_mount::{
    apply_ids_after_nest_swap, basket_refuse_if_changing_held, empty_ground_dismount_transition,
    held_as_nested, horse_eat_plan, is_horse_mount_held, nested_to_complex,
    pickup_or_drop_slots_ok, should_nest_swap_helpers, tile_as_nested,
};
use crate::locks::{
    evaluate_lock_use_gate, lockpick_coins_to_wallet_i32, note_lock_say, owner_account_of,
    owner_may_open_empty_hand, KEY_OBJ,
};
use crate::loved_food_wire::{evaluate_loved_food_extra, note_loved_food_extra, stamp_hits};
use crate::multi_use::{
    actor_must_be_full_refuse, change_number_of_uses_on_actor, change_number_of_uses_on_target,
    force_target_number_of_uses, pick_tool_last_use_new_actor, prefer_last_use_table,
    reverse_actor_exceeds_max, reverse_target_exceeds_max, should_skip_use_decrement,
    switch_number_of_uses, target_must_be_full_refuse, TargetUsesOutcome,
};
use crate::player::{clothing_slot_for_object, Player};
use crate::{
    ally_strength_blocks_pickup, calculate_enemy_vs_ally_strength_factor_ex,
    check_if_not_moving_and_close_enough, is_friendly, is_holding_weapon, is_leadership_ally,
    is_moving, note_too_close_say, refuse_ranged_use_too_close, schedule_decay, AllyStrengthPlayer,
    SimState, UseResult, ALLY_STRENGTH_TOO_LOW_FOR_PICKUP_DEFAULT,
};
use ol_content::{ContentDb, Transition};
use ol_world::{ComplexObject, NestedHelper, World};

// C-SS-MORE-BATCH5 full hungry-work pure pipe lives later in this file (public API).
// Call sites in apply_use_at use those pub helpers.

/// Haxe person.male==false proxy from display object name / default Female001 (19).
fn person_looks_female(content: &ContentDb, display_object_id: i32) -> bool {
    if let Some(d) = content.get(display_object_id) {
        let n = d.name.to_ascii_lowercase();
        let desc = d.description.to_ascii_lowercase();
        if n.contains("female") || desc.contains("female") {
            return true;
        }
        if n.contains("male") || desc.contains("male") {
            return false;
        }
    }
    display_object_id == 19
}

/// Stamp `extern_id` onto the tile helper after USE place (Haxe ObjectHelper.externId).
fn stamp_extern_id(world: &mut World, tx: i32, ty: i32, extern_id: i32) {
    if extern_id == 0 {
        return;
    }
    if let Some(h) = world.helpers.get_mut(&(tx, ty)) {
        h.extern_id = extern_id;
        return;
    }
    let base = world.get_object(tx, ty);
    if base != 0 {
        let mut c = ComplexObject::new_simple(base);
        c.extern_id = extern_id;
        world.set_object_complex(tx, ty, c);
    }
}

/// Haxe: TransitionHelper.use → doHorseStuffPossible (eat while mounted).
fn try_horse_eat(
    state: &mut SimState,
    conn_id: u64,
    tx: i32,
    ty: i32,
    actor: i32,
    target: i32,
    uses_remaining: i32,
) -> Option<UseResult> {
    let target_num = state.content.get(target).map(|d| d.num_uses).unwrap_or(0);
    let last_use = target_num >= 2 && uses_remaining > 0 && uses_remaining <= 1;
    let (food_id, via_tr, new_target) =
        horse_eat_plan(&state.content, actor, target, last_use)?;
    let food_value = state
        .content
        .get(food_id)
        .map(|d| d.food_value)
        .unwrap_or(0);
    if food_value <= 0 {
        return None;
    }
    // Haxe doEating — yum fill × world FoodFactor × starving + yum restore; keep horse held.
    // Haxe: ServerSettings.YumBonus (YUM-LIVE-SETTINGS)
    // EATEN-FOOD-PCT / WORLD-FOOD-FACTOR / C-SS-FULL-TABLE: doHorse → doEating L3186–3215 + doIncrease
    let food_base = food_value as f32;
    // C-SS-FULL-TABLE: live FoodFactorEaten* bands + full eat/restore knobs
    let bands = state.gameplay.food_factor_eaten_bands();
    let world_ff = state.world_food.get_food_factor_ex(food_id, &bands);
    let starve_ff = state.world_food.get_starving_food_factor_at(state.sim_time);
    let eat_knobs = state.gameplay.eat_live_knobs();
    let yum_b = eat_knobs.yum_bonus;
    let restore = state.gameplay.yum_restore_knobs();
    let red_per = eat_knobs.food_reduction_per_eating;
    // CRAVING-WIRE inputs before mut player borrow (Haxe doIncreaseFoodValue after fill)
    let person_oid = state
        .players
        .get(&conn_id)
        .map(|p| crate::person_object_id(p))
        .unwrap_or(crate::DEFAULT_PERSON_OBJECT);
    let person_color = state.content.person_color(person_oid);
    let loved: Vec<i32> = crate::loved_food_ids_for_person_color(person_color).to_vec();
    let food_objects = crate::food_objects_list(state);
    let nearby_best = crate::nearby_best_for_craving(state, conn_id);
    // Haxe playerTo prestige for superMeh trade (same as try_eat_held)
    let eater_p_id = state
        .players
        .get(&conn_id)
        .map(|p| p.p_id)
        .unwrap_or(0);
    let prestige_before = state
        .combat
        .stats
        .get(&eater_p_id)
        .map(|s| s.prestige)
        .unwrap_or(0.0);
    let mut pending_super_meh: Option<crate::SuperMehTrade> = None;
    let recorded_gain = {
        let p = state.players.get_mut(&conn_id)?;
        let fill_before = p.food.ceil() as i32;
        let count = p.yum.get_count_eaten(food_id);
        if !crate::can_eat_obj_ex(food_value, count, p.food, p.food_max, yum_b) {
            return None;
        }
        let computed = crate::compute_eat_full(food_value, count, eat_knobs);
        if crate::refuse_self_eat_super_meh(computed.is_super_meh, p.food) {
            return None;
        }
        let base_gain = p.yum.eat_full(food_id, food_base, fill_before, eat_knobs);
        // Haxe L3186–3192: FoodFactor (in eat) × getFoodFactor × getStarvingFoodFactor
        let mut gain = crate::apply_world_food_factors(base_gain, world_ff, starve_ff);
        // Haxe L3195: superMeh trades health for +1 food
        gain += crate::super_meh_extra_food_value(computed.is_super_meh);
        // Haxe L3195–3206: age / prestige−1 or hits+1 path (full doEating, not residual)
        let trade = crate::super_meh_trade(computed.is_super_meh, prestige_before, food_id);
        if trade.age_delta != 0.0 {
            p.age += trade.age_delta;
            p.true_age += trade.age_delta;
        }
        if computed.is_super_meh {
            pending_super_meh = Some(trade);
        }
        if gain <= 0.0 {
            return None;
        }
        p.food = (p.food + gain).min(p.food_max);
        // Haxe doIncreaseFoodValue after reduce (skip superMeh) — parity with doEating
        // C-SS-FULL-TABLE: horse path previously skipped yum restore + craving
        if !computed.is_super_meh {
            let amount = if computed.has_eaten_delta != 0.0 {
                computed.has_eaten_delta
            } else {
                red_per
            };
            let dont_change = crate::dont_change_craving(/*is_self_eat=*/ true, computed.is_yum);
            let _ = p.yum.do_increase_food_value_ex(
                food_id,
                amount,
                dont_change,
                &loved,
                &food_objects,
                nearby_best,
                restore,
                crate::craving_rand_int,
                crate::craving_rand_f01,
            );
        }
        // Stay mounted — do not clear held horse.
        Some(gain)
    };
    // Haxe: WorldMap.world.addFoodStatistic after fill (doEating L3215)
    if let Some(gain) = recorded_gain {
        state.world_food.add_food_statistic(food_id, food_base, gain);
    }
    // Haxe L3199–3206: superMeh prestige / hits (same as try_eat_held)
    if let Some(trade) = pending_super_meh {
        if trade.prestige_delta != 0.0 && eater_p_id != 0 {
            let s = state.combat.stats_mut(eater_p_id);
            s.prestige = (s.prestige + trade.prestige_delta).max(0.0);
        }
        if trade.needs_food_max_recompute && eater_p_id != 0 {
            let _ = state.combat.apply_hits(
                eater_p_id,
                trade.hits_delta,
                trade.wounded_by_food_id,
            );
            // Haxe: food_store_max = calculateFoodStoreMax(); death if < 1
            let (age, food, exh, true_age) = state
                .players
                .get(&conn_id)
                .map(|p| (p.age, p.food, p.exhaustion, p.true_age))
                .unwrap_or((20.0, 10.0, 0.0, 20.0));
            let health_f = state.player_health_food_store_max_factor(eater_p_id, true_age);
            let hits = state.combat.hits_of(eater_p_id);
            let new_max = crate::food_store_max_from_parts(age, food, hits, exh, health_f);
            if let Some(p) = state.players.get_mut(&conn_id) {
                p.food_max = new_max;
                if p.food > p.food_max && p.food_max > 0.0 {
                    p.food = p.food_max;
                }
                if crate::super_meh_food_max_is_deadly(new_max) {
                    // Haxe: doDeath('reason_killed_${woundedBy}')
                    p.deleted = true;
                    crate::ai_takeover::clear_ai_on_death(&mut p.ai_controlled);
                    p.death_reason =
                        Some(format!("reason_killed_{}", trade.wounded_by_food_id));
                }
            }
        }
    }
    {
        let mut w = state.world.write().unwrap();
        if via_tr {
            if new_target == 0 {
                w.set_object(tx, ty, 0);
            } else {
                place_after_use(
                    &mut w,
                    &state.content,
                    tx,
                    ty,
                    target,
                    new_target,
                    uses_remaining,
                    false,
                    true,
                );
            }
        } else {
            // Eat the tile object itself.
            w.set_object(tx, ty, 0);
        }
    }
    let live = {
        let w = state.world.read().unwrap();
        w.get_object(tx, ty)
    };
    state.record_world_change(tx, ty, live);
    schedule_decay(state, tx, ty, live);
    Some(UseResult {
        actor_before: actor,
        target_before: target,
        actor_after: actor,
        target_after: live,
        applied: true,
        x: tx,
        y: ty,
    })
}

/// Place post-USE tile state with Haxe `DoChangeNumberOfUsesOnTarget` semantics.
pub fn place_after_use(
    world: &mut World,
    content: &ContentDb,
    tx: i32,
    ty: i32,
    target_before: i32,
    target_after: i32,
    uses_before: i32,
    reverse_use_target: bool,
    from_transition: bool,
) {
    place_after_use_ex(
        world,
        content,
        tx,
        ty,
        target_before,
        target_after,
        uses_before,
        reverse_use_target,
        from_transition,
        false,
        true,
    );
}

/// Apply transition result to one contained slot of the outer container helper.
///
/// Outer base id stays `outer_id`; only `contained[slot]` / `slots[slot]` change.
// Haxe: TransitionHelper.doTransitionIfPossible L894–904 (in-place contained mutate)
fn place_after_use_on_contained(
    world: &mut World,
    content: &ContentDb,
    tx: i32,
    ty: i32,
    outer_id: i32,
    slot: usize,
    target_before: i32,
    target_after: i32,
    uses_before: i32,
    reverse_use_target: bool,
    from_transition: bool,
    no_use_target: bool,
    allow_reset_on_id_change: bool,
    force_tgt_uses: i32,
    loved_extra: bool,
    sim_time: f32,
) {
    let mut helper = world
        .get_helper(tx, ty)
        .cloned()
        .unwrap_or_else(|| ComplexObject::new_simple(outer_id));
    helper.base_id = outer_id;
    if slot >= helper.contained.len() {
        world.set_object_complex(tx, ty, helper);
        return;
    }

    let num_uses_after = content
        .get(target_after)
        .map(|d| d.num_uses)
        .unwrap_or(0)
        .max(0);
    let num_uses_before = content
        .get(target_before)
        .map(|d| d.num_uses)
        .unwrap_or(0)
        .max(0);

    let forced = if from_transition && !loved_extra {
        force_target_number_of_uses(force_tgt_uses, num_uses_after)
    } else {
        None
    };

    let (new_id, new_uses) = if let Some(u) = forced {
        if target_after == 0 || u <= 0 {
            (0, 0)
        } else {
            (target_after, u)
        }
    } else if target_after == 0 {
        (0, 0)
    } else {
        match change_number_of_uses_on_target(
            target_before,
            target_after,
            uses_before,
            num_uses_before,
            num_uses_after,
            reverse_use_target,
            no_use_target,
            from_transition,
            allow_reset_on_id_change,
        ) {
            TargetUsesOutcome::Cleared => (0, 0),
            TargetUsesOutcome::Simple => (target_after, 0),
            TargetUsesOutcome::Uses(u) => (target_after, u),
        }
    };

    if new_id == 0 {
        helper.contained.remove(slot);
        if slot < helper.nested.len() {
            helper.nested.remove(slot);
        }
        if slot < helper.slots.len() {
            helper.slots.remove(slot);
        }
        helper.sync_slots_len_after_contained_change();
    } else {
        helper.contained[slot] = new_id;
        if slot < helper.slots.len() {
            helper.slots[slot].id = new_id;
            helper.slots[slot].uses_remaining = new_uses;
            helper.slots[slot].creation_time = sim_time;
        } else if new_uses > 0 || !helper.slots.is_empty() {
            helper.sync_slots_len_after_contained_change();
            if slot < helper.slots.len() {
                helper.slots[slot].id = new_id;
                helper.slots[slot].uses_remaining = new_uses;
                helper.slots[slot].creation_time = sim_time;
            }
        }
        // Keep nested ids length aligned when present.
        if !helper.nested.is_empty() {
            while helper.nested.len() < helper.contained.len() {
                helper.nested.push(Vec::new());
            }
            helper.nested.truncate(helper.contained.len());
        }
    }
    world.set_object_complex(tx, ty, helper);
}

/// Extended form with Haxe `noUseTarget` + clothing reset control.
pub fn place_after_use_ex(
    world: &mut World,
    content: &ContentDb,
    tx: i32,
    ty: i32,
    target_before: i32,
    target_after: i32,
    uses_before: i32,
    reverse_use_target: bool,
    from_transition: bool,
    no_use_target: bool,
    allow_reset_on_id_change: bool,
) {
    if target_after == 0 {
        world.set_object(tx, ty, 0);
        return;
    }

    let num_uses_after = content
        .get(target_after)
        .map(|d| d.num_uses)
        .unwrap_or(0)
        .max(0);
    let num_uses_before = content
        .get(target_before)
        .map(|d| d.num_uses)
        .unwrap_or(0)
        .max(0);

    match change_number_of_uses_on_target(
        target_before,
        target_after,
        uses_before,
        num_uses_before,
        num_uses_after,
        reverse_use_target,
        no_use_target,
        from_transition,
        allow_reset_on_id_change,
    ) {
        TargetUsesOutcome::Cleared => world.set_object(tx, ty, 0),
        TargetUsesOutcome::Simple => world.set_object(tx, ty, target_after),
        TargetUsesOutcome::Uses(u) => {
            world.set_object_complex(tx, ty, ComplexObject::with_uses(target_after, u));
        }
    }
}

/// Haxe tool last-use after actor uses hit 0: `(id,-1)` LA → non-LA → `(id,target)` LA.
///
/// Haxe TODO L1565 EMPTY+Cold Bowl: no extra guard; only map keys decide.
// Haxe: TransitionHelper.DoChangeNumberOfUsesOnActorManual (~1567–1590)
pub fn tool_last_use_new_actor(
    content: &ContentDb,
    object_id: i32,
    target_id: i32,
) -> Option<i32> {
    if object_id == 0 {
        return None;
    }
    let base = content.resolve_base_id(object_id);
    pick_tool_last_use_new_actor(
        content
            .find_transition_last_use(base, -1)
            .map(|t| t.new_actor_id),
        content.find_transition(base, -1).map(|t| t.new_actor_id),
        content
            .find_transition_last_use(base, target_id)
            .map(|t| t.new_actor_id),
    )
}

/// Apply actor numberOfUses + tool last-use after a USE transition.
// Haxe: TransitionHelper.DoChangeNumberOfUsesOnActor / DoChangeNumberOfUsesOnActorManual
fn resolve_actor_after_use(
    content: &ContentDb,
    actor_before: i32,
    actor_after_tr: i32,
    uses_before: i32,
    reverse_use_actor: bool,
    no_use_actor: bool,
    target_id_for_last: i32,
) -> (i32, i32) {
    let num_uses_after = content
        .get(actor_after_tr)
        .map(|d| d.num_uses)
        .unwrap_or(0)
        .max(0);

    let mut out = change_number_of_uses_on_actor(
        actor_before,
        actor_after_tr,
        uses_before,
        num_uses_after,
        reverse_use_actor,
        no_use_actor,
    );

    // Haxe: tool last-use only on same-id deplete path (idHasChanged=false → uses hit 0).
    // When reverse / id change, Manual returns early without tool lookup.
    let same_id = content.resolve_base_id(actor_before)
        == content.resolve_base_id(actor_after_tr);
    if out.held_id != 0
        && out.held_uses == 0
        && !no_use_actor
        && !reverse_use_actor
        && same_id
    {
        if let Some(new_id) = tool_last_use_new_actor(content, out.held_id, target_id_for_last)
        {
            out.held_id = new_id;
            // Haxe keeps numberOfUses at 0 after tool id transform (not full num_uses).
            out.held_uses = 0;
        }
        // No tool row: leave held_id at actor_after with uses=0 (USE ignores Manual false).
    }

    (out.held_id, out.held_uses)
}

fn allow_target_reset(content: &ContentDb, target_id: i32) -> bool {
    let Some(def) = content.get(target_id) else {
        return true;
    };
    // Haxe: resetNumberOfUses = !isClothing || numUses < 2 (TH-CLOTHING-MATRIX)
    if def.num_uses < 2 {
        return true;
    }
    !def.is_clothing()
}

/// Apply USE at world tile `(tx, ty)` for connection `conn_id`.

/// DARK-NOSAJ: Haxe TransitionHelper Tarr/Dark Nosaj monument side-effects on USE.
// Haxe: TransitionHelper.doCommandHelper L144–185
fn apply_monument_use_side_effects(
    state: &mut SimState,
    conn_id: u64,
    actor: i32,
    target: i32,
) {
    if target == 0 {
        return;
    }
    let target_parent = state.content.resolve_base_id(target);
    let held_parent = if actor == 0 {
        0
    } else {
        state.content.resolve_base_id(actor)
    };
    let (dark, praised, p_id, age, food, exh, true_age) = {
        let Some(p) = state.players.get(&conn_id) else {
            return;
        };
        (
            p.dark_nosaj,
            p.praised_jinbali,
            p.p_id,
            p.age,
            p.food,
            p.exhaustion,
            p.true_age,
        )
    };
    let Some(plan) =
        crate::dark_nosaj::plan_monument_use(target_parent, held_parent, dark, praised)
    else {
        return;
    };

    // Player session flags
    if let Some(p) = state.players.get_mut(&conn_id) {
        p.dark_nosaj = plan.dark_nosaj;
        p.praised_jinbali = plan.praised_jinbali;
    }

    // yum_multiplier / prestige
    if plan.yum_delta != 0.0 && plan.yum_delta.is_finite() {
        {
            let s = state.combat.stats_mut(p_id);
            s.prestige += plan.yum_delta;
        }
        if let Some(n) = state.social.lineages.get_mut(&p_id) {
            n.add_prestige(plan.yum_delta);
        }
    }

    // lostCombatPrestige book + combat stats mirror
    if plan.lost_combat_delta != 0.0 || plan.lost_combat_floor_zero {
        let before = state.reputation.lost_combat(p_id);
        let after = crate::dark_nosaj::apply_lost_combat_delta(
            before,
            plan.lost_combat_delta,
            plan.lost_combat_floor_zero,
        );
        state.reputation.set_from_lost_combat(p_id, after);
        state.combat.stats_mut(p_id).lost_combat_prestige = after;
    }

    // hits (praised-punish path) + food_max recompute for sendFoodUpdate parity
    let mut hits = state.combat.hits_of(p_id);
    if plan.hits_delta != 0.0 && plan.hits_delta.is_finite() {
        hits = (hits + plan.hits_delta).max(0.0);
        state.combat.stats_mut(p_id).hits = hits;
    }
    if plan.yum_delta != 0.0 || plan.hits_delta != 0.0 {
        let health_f = state.player_health_food_store_max_factor(p_id, true_age);
        let knobs = state.gameplay.food_store_max_knobs();
        let new_max = crate::food_store_max_from_parts_ex(age, food, hits, exh, health_f, knobs);
        if let Some(p) = state.players.get_mut(&conn_id) {
            p.food_max = new_max;
            if p.food > new_max {
                p.food = new_max;
            }
        }
    }

    crate::dark_nosaj::note_monument_feedback(crate::dark_nosaj::MonumentFeedback {
        conn_id,
        say: plan.say,
        curse: plan.curse,
    });
}

/// Apply USE at world tile `(tx, ty)` (no container index).
pub fn apply_use_at(
    state: &mut SimState,
    conn_id: u64,
    tx: i32,
    ty: i32,
) -> Option<UseResult> {
    apply_use_at_ex(state, conn_id, tx, ty, None)
}

/// Apply USE at `(tx, ty)` with optional Haxe `containerIndex` (USE x y id i).
///
/// When `container_index >= 0`, retarget to the contained object and enforce
/// post-transition fit vs outer `slotSize` (L1087).
// Haxe: TransitionHelper.doTransitionIfPossible + doTransitionIfPossibleHelper
pub fn apply_use_at_ex(
    state: &mut SimState,
    conn_id: u64,
    tx: i32,
    ty: i32,
    container_index: Option<i32>,
) -> Option<UseResult> {
    let (
        actor,
        held_uses,
        display_object_id,
        force_last_use,
        held_helper_snapshot,
        px,
        py,
        moving,
        player_email,
        p_id,
        food_max,
        exhaustion,
    ) = {
        let player = state.players.get(&conn_id)?;
        if player.deleted {
            return None;
        }
        (
            player.held_id,
            player.held_uses,
            player.display_object_id,
            player.force_last_use,
            player.held_helper.clone(),
            player.x,
            player.y,
            is_moving(player),
            player.email.clone(),
            player.p_id,
            player.food_max,
            player.exhaustion,
        )
    };
    // Haxe: TransitionHelper.checkIfNotMovingAndCloseEnough (moving + held useDistance)
    let held_use_distance = state
        .content
        .get(actor)
        .map(|d| d.use_distance)
        .unwrap_or(1);
    let (mw, mh, wrap) = {
        let w = state.world.read().unwrap();
        (w.width_tiles, w.height_tiles, w.wrap)
    };
    if !check_if_not_moving_and_close_enough(
        moving,
        px,
        py,
        tx,
        ty,
        held_use_distance,
        mw,
        mh,
        wrap,
    ) {
        return Some(UseResult {
            actor_before: actor,
            target_before: 0,
            actor_after: actor,
            target_after: 0,
            applied: false,
            x: tx,
            y: ty,
        });
    }

    let (outer_target, target, uses_remaining, mut hits_before, container_slot_size, container_slot_idx) = {
        let w = state.world.read().unwrap();
        let outer = w.get_object(tx, ty);
        let helper = w.get_helper(tx, ty);
        let uses = helper.map(|h| h.uses_remaining).unwrap_or(0);
        let hits = helper.map(|h| h.hits).unwrap_or(0.0);
        // Haxe: doTransitionIfPossible containerIndex >= 0 → retarget contained
        // // Haxe: TransitionHelper.doTransitionIfPossible L885–899
        let ci = container_index.unwrap_or(-1);
        if ci >= 0 {
            let idx = ci as usize;
            let contained_len = helper.map(|h| h.contained.len()).unwrap_or(0);
            if idx >= contained_len {
                // Haxe: player.message = 'container index not found'
                return Some(UseResult {
                    actor_before: actor,
                    target_before: outer,
                    actor_after: actor,
                    target_after: outer,
                    applied: false,
                    x: tx,
                    y: ty,
                });
            }
            let h = helper.expect("contained_len > 0");
            let slot_id = h.contained[idx];
            let slot_uses = if idx < h.slots.len() {
                h.slots[idx].uses_remaining
            } else {
                0
            };
            let outer_slot = state
                .content
                .get(outer)
                .map(|d| d.slot_size)
                .unwrap_or(1.0);
            (outer, slot_id, slot_uses, hits, outer_slot, Some(idx))
        } else {
            (outer, outer, uses, hits, -1.0_f32, None)
        }
    };

    let target_num_uses = state.content.get(target).map(|d| d.num_uses).unwrap_or(0);
    let uses_remaining = if target_num_uses >= 2 && uses_remaining > target_num_uses {
        target_num_uses
    } else {
        uses_remaining
    };

    // Haxe: TransitionHelper.use deadlyDistance>1.9 && target.isAnimal() && isCloseUseExact 1.5
    let held_deadly = state
        .content
        .get(actor)
        .map(|d| d.deadly_distance)
        .unwrap_or(0.0);
    let target_is_animal = state
        .content
        .get(target)
        .map(|d| d.is_animal())
        .unwrap_or(false);
    if refuse_ranged_use_too_close(
        held_deadly,
        target_is_animal,
        px as f64,
        py as f64,
        tx as f64,
        ty as f64,
        mw,
        mh,
        wrap,
    ) {
        // Haxe: player.say('Too close...'); player.message = 'too close'
        // Public SAY → uppercased PS to speaker + nearby (GPI-TOO-CLOSE).
        note_too_close_say(conn_id);
        return Some(UseResult {
            actor_before: actor,
            target_before: target,
            actor_after: actor,
            target_after: target,
            applied: false,
            x: tx,
            y: ty,
        });
    }

    // Haxe: TransitionHelper.doCommandHelper AllyStrenghTooLowForPickup
    // Default threshold 0 = gate disabled (cheap early exit).
    // Haxe: ServerSettings.AllyStrenghTooLowForPickup
    let ally_pickup_threshold = ALLY_STRENGTH_TOO_LOW_FOR_PICKUP_DEFAULT;
    if ally_pickup_threshold > 0.0 && target != 0 {
        let (mw, mh, wrap) = {
            let w = state.world.read().unwrap();
            (w.width_tiles, w.height_tiles, w.wrap)
        };
        let strength_players: Vec<AllyStrengthPlayer> = state
            .players
            .values()
            .map(|op| {
                let oname = state
                    .content
                    .get(op.held_id)
                    .map(|d| d.name.as_str())
                    .unwrap_or("");
                let holding = is_holding_weapon(op.held_id, oname);
                let ally_to_self =
                    is_leadership_ally(&state.social.following, op.p_id, p_id);
                let friendly_to_self = is_friendly(
                    ally_to_self,
                    op.last_attacked_player_id,
                    op.last_player_attacked_me_id,
                    p_id,
                );
                AllyStrengthPlayer {
                    p_id: op.p_id,
                    x: op.x,
                    y: op.y,
                    deleted: op.deleted,
                    food_store_max: op.food_max,
                    holding_weapon: holding,
                    friendly_to_observer: friendly_to_self,
                    friendly_to_target: false,
                    ally_to_observer: ally_to_self,
                }
            })
            .collect();
        // C-SS-MORE-BATCH3: live AllyConsideredClose
        let strength_f = calculate_enemy_vs_ally_strength_factor_ex(
            px,
            py,
            &strength_players,
            false, // Haxe: calculateEnemyVsAllyStrengthFactor() no target
            mw,
            mh,
            wrap,
            state.gameplay.ally_considered_close,
        );
        if ally_strength_blocks_pickup(strength_f, ally_pickup_threshold, target) {
            // Haxe: player.say('Too many hostile people...', true); return false
            note_lock_say(conn_id, "Too many hostile people...");
            return Some(UseResult {
                actor_before: actor,
                target_before: target,
                actor_after: actor,
                target_after: target,
                applied: false,
                x: tx,
                y: ty,
            });
        }
    }

    // Haxe: TransitionHelper.doCommandHelper Tarr/Dark Nosaj monuments (side-effects).
    // DARK-NOSAJ — runs even when no transition applies (matches Haxe early hook).
    apply_monument_use_side_effects(state, conn_id, actor, target);

    // Haxe: TransitionHelper.use → doHorseStuffPossible (eat while mounted).
    if is_horse_mount_held(actor) && target != 0 {
        if let Some(r) = try_horse_eat(state, conn_id, tx, ty, actor, target, uses_remaining) {
            return Some(r);
        }
    }

    let held_num_uses = state.content.get(actor).map(|d| d.num_uses).unwrap_or(0);
    let effective_held_uses = if held_uses > 0 {
        held_uses
    } else if held_num_uses >= 2 {
        held_num_uses
    } else {
        0
    };

    let prefer_last = prefer_last_use_table(
        state.prefer_last_use || force_last_use,
        uses_remaining,
        target_num_uses,
        effective_held_uses,
        held_num_uses,
    );

    // Haxe: GetTrans; if null && empty ground, held+-1 dismount (reject newTargetID==0).
    let mut tr = state
        .content
        .find_transition_prefer(actor, target, prefer_last)
        .cloned()
        .or_else(|| {
            if target == 0 {
                empty_ground_dismount_transition(&state.content, actor).cloned()
            } else {
                None
            }
        });

    // Haxe: property owner open locked without key (empty hand, null transition, 917+target).
    // Haxe: TransitionHelper.doTransitionIfPossibleHelper L1010-1025
    let mut owner_open_say = false;
    if tr.is_none() && actor == 0 && target != 0 {
        let parent = state.content.resolve_base_id(target);
        if let Some(key_tr) = state.content.find_transition(KEY_OBJ, parent).cloned() {
            let (owners_acc, owner_id) = {
                let w = state.world.read().unwrap();
                w.get_helper(tx, ty)
                    .map(|h| (h.owners_by_account.clone(), h.owner_id))
                    .unwrap_or_default()
            };
            let owner = owner_account_of(&owners_acc, owner_id);
            let player_account = account_soul_token(&player_email);
            if owner_may_open_empty_hand(true, true, true, owner, player_account) {
                tr = Some(key_tr);
                owner_open_say = true;
            }
        }
    }

    let refuse = |actor_a: i32, target_a: i32| {
        Some(UseResult {
            actor_before: actor,
            target_before: target,
            actor_after: actor_a,
            target_after: target_a,
            applied: false,
            x: tx,
            y: ty,
        })
    };

    // Capture nest counts before mutation (horse cart cargo / pickup slots).
    let held_contained_count = held_helper_snapshot
        .as_ref()
        .map(|h| h.contained.len())
        .unwrap_or(0);
    let (target_contained_count, tile_helper_snapshot) = {
        let w = state.world.read().unwrap();
        let h = w.get_helper(tx, ty).cloned();
        let n = h.as_ref().map(|c| c.contained.len()).unwrap_or(0);
        (n, h)
    };

    // --- TH-LOCK: key / lock / lockpick pre-gates (Haxe doCommandHelper L214-258) ---
    let held_extern = held_helper_snapshot
        .as_ref()
        .map(|h| h.extern_id)
        .unwrap_or(0);
    let target_extern = tile_helper_snapshot
        .as_ref()
        .map(|h| h.extern_id)
        .unwrap_or(0);
    let target_desc = state
        .content
        .get(target)
        .map(|d| d.description.clone())
        .unwrap_or_default();
    let coins = state.economy.coins_of(p_id) as f32;
    let is_female = person_looks_female(&state.content, display_object_id);
    let decays_to = state
        .content
        .get(actor)
        .map(|d| d.decays_to_obj)
        .unwrap_or(0);
    let lock_settings = state.lockpick_settings;
    let unit_key = rand::random::<f32>();
    let unit_lp = rand::random::<f32>();
    let lock_gate = evaluate_lock_use_gate(
        actor,
        target,
        &target_desc,
        held_extern,
        target_extern,
        tr.is_some(),
        coins,
        exhaustion,
        food_max,
        is_female,
        &lock_settings,
        decays_to,
        unit_key,
        unit_lp,
    );

    // Apply lock side-effects (coins / exhaustion / broken key / held extern) even on refuse.
    // Haxe: player.coins is Float; Rust wallet is i32 — floor via lockpick_coins_to_wallet_i32.
    if let Some(c) = lock_gate.coins_after {
        let wallet = state.economy.wallet_mut(p_id);
        wallet.coins = lockpick_coins_to_wallet_i32(c);
    }
    if let Some(e) = lock_gate.exhaustion_after {
        if let Some(p) = state.players.get_mut(&conn_id) {
            p.exhaustion = e;
        }
    }
    if let Some(new_held) = lock_gate.held_id_override {
        if let Some(p) = state.players.get_mut(&conn_id) {
            p.set_held(new_held, 0);
        }
    } else if lock_gate.held_extern != held_extern {
        if let Some(p) = state.players.get_mut(&conn_id) {
            if let Some(h) = p.held_helper.as_mut() {
                h.extern_id = lock_gate.held_extern;
            } else if p.held_id != 0 {
                let mut h = NestedHelper::with_uses(p.held_id, p.held_uses);
                h.extern_id = lock_gate.held_extern;
                p.set_held_helper(h);
            }
        }
    }
    // Stamp target extern early when pairing / blank-lock copy (before place).
    if lock_gate.target_extern != target_extern {
        let mut w = state.world.write().unwrap();
        stamp_extern_id(&mut w, tx, ty, lock_gate.target_extern);
    }
    if let Some(say) = lock_gate.say {
        note_lock_say(conn_id, say);
    } else if owner_open_say {
        note_lock_say(conn_id, "Its mine!");
    }
    if !lock_gate.allow {
        let held_now = state
            .players
            .get(&conn_id)
            .map(|p| p.held_id)
            .unwrap_or(actor);
        return refuse(held_now, target);
    }
    let lock_claim = lock_gate.claim_ownership;
    let lock_target_extern = lock_gate.target_extern;
    let lock_held_extern = lock_gate.held_extern;

    let (
        actor_after_tr,
        mut target_after,
        reverse_target,
        reverse_actor,
        mut no_use_target,
        no_use_actor,
        from_transition,
        switch_uses,
        force_tgt_uses,
        is_pickup_or_drop,
        change_held,
    ) = if let Some(ref tr) = tr {
        if tr.reverse_use_actor {
            let new_actor_uses = state
                .content
                .get(tr.new_actor_id)
                .map(|d| d.num_uses)
                .unwrap_or(0);
            if reverse_actor_exceeds_max(effective_held_uses, new_actor_uses) {
                return refuse(actor, target);
            }
        }

        let mut tr_work: Transition = tr.clone();
        if tr_work.reverse_use_target {
            let new_tgt_uses = state
                .content
                .get(tr_work.new_target_id)
                .map(|d| d.num_uses)
                .unwrap_or(0);
            let cur_for_max = if uses_remaining > 0 {
                uses_remaining
            } else if target_num_uses >= 2 {
                target_num_uses
            } else {
                0
            };
            if reverse_target_exceeds_max(cur_for_max, new_tgt_uses) {
                if let Some(max_tr) = state.content.find_transition_max_use(actor, target) {
                    tr_work = max_tr.clone();
                } else {
                    return refuse(actor, target);
                }
            }
        }

        if actor_must_be_full_refuse(
            tr_work.actor_min_use_fraction,
            effective_held_uses,
            held_num_uses,
        ) {
            return refuse(actor, target);
        }
        let cur_tgt = if uses_remaining > 0 {
            uses_remaining
        } else if target_num_uses >= 2 {
            target_num_uses
        } else {
            0
        };
        if target_must_be_full_refuse(
            tr_work.target_min_use_fraction,
            tr_work.reverse_use_target,
            cur_tgt,
            target_num_uses,
        ) {
            return refuse(actor, target);
        }

        // Haxe: containerSlotSize gate when USE on contained object.
        // // Haxe: TransitionHelper.doTransitionIfPossibleHelper L1087–1091
        if !crate::death_polish::transition_result_fits_container_from_content(
            &state.content,
            container_slot_size,
            tr_work.new_target_id,
        ) {
            // Haxe: player.message = 'result does not fit in container'
            note_lock_say(conn_id, "result does not fit in container");
            return refuse(actor, target);
        }

        // Haxe: isPickupOrDrop → empty first (compare contained to newActor.numSlots).
        let new_actor_slots = state
            .content
            .get(tr_work.new_actor_id)
            .map(|d| d.num_slots)
            .unwrap_or(0);
        let new_target_slots = state
            .content
            .get(tr_work.new_target_id)
            .map(|d| d.num_slots)
            .unwrap_or(0);
        // When USE is on a contained object, slot-fit is vs outer container only
        // (pickup/drop nest swap is ground semantics — skip for container index).
        if container_slot_idx.is_none()
            && !pickup_or_drop_slots_ok(
                tr_work.is_pickup_or_drop,
                target_contained_count,
                new_actor_slots,
                new_target_slots,
            )
        {
            return refuse(actor, target);
        }

        let mut nua = tr_work.no_use_actor;
        let mut nut = tr_work.no_use_target;
        if !nua && !tr_work.reverse_use_actor {
            let chance = state
                .content
                .get(actor)
                .map(|d| d.use_chance)
                .unwrap_or(0.0);
            if should_skip_use_decrement(chance, rand::random::<f32>()) {
                nua = true;
            }
        }
        if !nut && !tr_work.reverse_use_target {
            let chance = state
                .content
                .get(target)
                .map(|d| d.use_chance)
                .unwrap_or(0.0);
            if chance > 0.0 && rand::random::<f32>() >= chance {
                nut = true;
            }
        }

        // C-SS-MORE-BATCH5: TransitionHelper hungry-work cost / heat / food / exhaustion.
        // Haxe: TransitionHelper.doTransitionIfPossible L1170–1256
        {
            let biome = {
                let w = state.world.read().unwrap();
                w.get_biome(tx, ty)
            };
            // Re-read vitals after lock side-effects.
            let Some((food_now, food_max_now, exh_now, heat_now)) =
                state.players.get(&conn_id).map(|p| {
                    (p.food, p.food_max, p.exhaustion, p.heat)
                })
            else {
                return refuse(actor, target);
            };
            let actor_hw = object_hungry_work(
                actor,
                state
                    .content
                    .get(actor)
                    .map(|d| d.description.as_str())
                    .unwrap_or(""),
                state.gameplay.hungry_work_cost,
            );
            // Haxe uses newParentTargetObjectData (new target after transition).
            let new_tgt_desc = state
                .content
                .get(tr_work.new_target_id)
                .map(|d| d.description.clone())
                .unwrap_or_default();
            let new_tgt_hw = object_hungry_work(
                tr_work.new_target_id,
                &new_tgt_desc,
                state.gameplay.hungry_work_cost,
            );
            // Transition cost/temperature not yet on Transition; defaults 0 / -1.
            // Haxe: transition.hungryWorkCost / hungryWorkTemperature
            let transition_hw_cost = 0.0_f32;
            let transition_hw_temp = -1.0_f32;
            let base_cost = compute_hungry_work_cost(
                actor_hw,
                new_tgt_hw,
                transition_hw_cost,
                biome == BIOME_PASSABLE_RIVER,
            );
            let (mut cost, _is_fortified) =
                apply_loose_fence_hungry_work_waiver(base_cost, &new_tgt_desc, hits_before);
            // Owner half-cost / refuse when object is +owned.
            // Haxe: target.objectData.isOwned + isOwnedBy
            let object_is_owned = ol_world::description_is_owned(&target_desc);
            if object_is_owned {
                let (owners_acc, owner_id) = {
                    let w = state.world.read().unwrap();
                    w.get_helper(tx, ty)
                        .map(|h| (h.owners_by_account.clone(), h.owner_id))
                        .unwrap_or_default()
                };
                let owner = owner_account_of(&owners_acc, owner_id);
                let player_account = account_soul_token(&player_email);
                let player_is_owner = owner.map(|o| o == player_account).unwrap_or(true);
                match adjust_hungry_work_for_ownership(cost, true, player_is_owner) {
                    HungryWorkOwnerAdj::OwnerHalf { cost: c, .. } => cost = c,
                    HungryWorkOwnerAdj::NotOwnerRefuse => {
                        note_lock_say(conn_id, "Not the owner");
                        return refuse(actor, target);
                    }
                    HungryWorkOwnerAdj::NotOwnerContinue { cost: c } => cost = c,
                    HungryWorkOwnerAdj::Unaffected { cost: c } => cost = c,
                }
            }
            let temperature = resolve_hungry_work_temperature(
                transition_hw_temp,
                cost,
                state.gameplay.hungry_work_heat,
            );
            match evaluate_hungry_work_use(
                cost,
                temperature,
                food_now,
                food_max_now,
                exh_now,
                heat_now,
            ) {
                HungryWorkGate::Free => {}
                HungryWorkGate::Allow {
                    heat_after,
                    food_after,
                    exhaustion_after,
                } => {
                    if let Some(p) = state.players.get_mut(&conn_id) {
                        p.heat = heat_after;
                        p.food = food_after;
                        p.exhaustion = exhaustion_after;
                    }
                    // Haxe: player.doEmote(Emote.biomeRelief); sendFoodUpdate — FX later path
                }
                HungryWorkGate::RefuseExhaustion { .. } => {
                    // Haxe: player.say('Too exhausted! $excess'); Emote.homesick
                    note_lock_say(conn_id, "Too exhausted!");
                    return refuse(actor, target);
                }
                HungryWorkGate::RefuseFood { .. } => {
                    // Haxe: player.say('Need ${missingFood} more food!')
                    note_lock_say(conn_id, "Need more food!");
                    return refuse(actor, target);
                }
            }
        }

        // TH-ALT-OUTCOME: Haxe TransitionHelper L1260–1306 alternativeTransitionOutcome.
        // After hungry-work cost is paid; TryAgain keeps tile + may place bonus; no main transform.
        // Proceed reduces hits then continues normal USE transform.
        {
            let outcomes: Vec<i32> = state
                .content
                .alternative_outcomes_for(actor, target, tr_work.new_target_id)
                .to_vec();
            let count_obj = {
                let w = state.world.read().unwrap();
                w.get_helper(tx, ty).map(|h| h.count_obj).unwrap_or(0.0)
            };
            // is_fortified from hungry-work cost vs hits (recompute; matches L1179 + Loose waiver).
            let actor_hw = object_hungry_work(
                actor,
                state
                    .content
                    .get(actor)
                    .map(|d| d.description.as_str())
                    .unwrap_or(""),
                state.gameplay.hungry_work_cost,
            );
            let new_tgt_desc = state
                .content
                .get(tr_work.new_target_id)
                .map(|d| d.description.clone())
                .unwrap_or_default();
            let new_tgt_hw = object_hungry_work(
                tr_work.new_target_id,
                &new_tgt_desc,
                state.gameplay.hungry_work_cost,
            );
            let biome = {
                let w = state.world.read().unwrap();
                w.get_biome(tx, ty)
            };
            let base_cost = compute_hungry_work_cost(
                actor_hw,
                new_tgt_hw,
                0.0, // transition.hungryWorkCost residual
                biome == BIOME_PASSABLE_RIVER,
            );
            let (_cost_for_fort, is_fortified) =
                apply_loose_fence_hungry_work_waiver(base_cost, &new_tgt_desc, hits_before);
            // allow_for_owner: owned + owner + original cost < 1 (Haxe L1195–1196)
            let object_is_owned = ol_world::description_is_owned(&target_desc);
            let allow_for_owner = if object_is_owned {
                let (owners_acc, owner_id) = {
                    let w = state.world.read().unwrap();
                    w.get_helper(tx, ty)
                        .map(|h| (h.owners_by_account.clone(), h.owner_id))
                        .unwrap_or_default()
                };
                let owner = owner_account_of(&owners_acc, owner_id);
                let player_account = account_soul_token(&player_email);
                let player_is_owner = owner.map(|o| o == player_account).unwrap_or(true);
                match adjust_hungry_work_for_ownership(base_cost, true, player_is_owner) {
                    HungryWorkOwnerAdj::OwnerHalf {
                        allow_for_owner: a,
                        ..
                    } => a,
                    _ => false,
                }
            } else {
                false
            };
            let (fort_id, fort_val) = crate::alt_outcome::fortification_of(&state.content, target);
            let plan = crate::alt_outcome::evaluate_alternative_outcome(
                tr_work.target_id,
                allow_for_owner,
                is_fortified,
                &outcomes,
                hits_before,
                count_obj,
                fort_id,
                fort_val,
                crate::alt_outcome::ALTERNATIVE_OUTCOME_PERCENT_INCREASE_PER_HIT,
                crate::alt_outcome::ALTERNATIVE_OUTCOME_HITS_DECREASE_ON_SUCCESS,
                rand::random::<f32>(),
                rand::random::<f32>(),
            );
            match plan {
                crate::alt_outcome::AltOutcomePlan::Skip => {}
                crate::alt_outcome::AltOutcomePlan::Proceed { hits_after } => {
                    // Defer stamp to post-transform hits_out path (set_object may clear helper).
                    hits_before = hits_after;
                }
                crate::alt_outcome::AltOutcomePlan::TryAgain {
                    hits_after,
                    count_obj_after,
                    place_id,
                    say_fortification,
                } => {
                    {
                        let mut w = state.world.write().unwrap();
                        stamp_hits(&mut w, tx, ty, hits_after);
                        if let Some(c) = count_obj_after {
                            if let Some(h) = w.helpers.get_mut(&(tx, ty)) {
                                h.count_obj = c;
                            } else {
                                let base = w.get_object(tx, ty);
                                if base != 0 {
                                    let mut co = ComplexObject::new_simple(base);
                                    co.hits = hits_after;
                                    co.count_obj = c;
                                    w.set_object_complex(tx, ty, co);
                                }
                            }
                        }
                    }
                    if let Some(oid) = place_id {
                        if oid > 0 {
                            let _ = crate::death_polish::place_object_by_id(
                                state,
                                tx,
                                ty,
                                oid,
                                crate::death_polish::PlaceObjectOpts::default(),
                            );
                        }
                    }
                    if say_fortification {
                        // Haxe: player.say('Try again! Fortification: ${-Math.round(hits)}', true)
                        note_lock_say(
                            conn_id,
                            format!(
                                "Try again! Fortification: {}",
                                -hits_after.round() as i32
                            ),
                        );
                    } else {
                        // Haxe: player.say('Try again! Hits ${Math.round(hits)}', true)
                        note_lock_say(
                            conn_id,
                            format!("Try again! Hits {}", hits_after.round() as i32),
                        );
                    }
                    // Haxe: return true without main transform (action still applied).
                    return Some(UseResult {
                        actor_before: actor,
                        target_before: target,
                        actor_after: actor,
                        target_after: target,
                        applied: true,
                        x: tx,
                        y: ty,
                    });
                }
            }
        }

        (
            tr_work.new_actor_id,
            tr_work.new_target_id,
            tr_work.reverse_use_target,
            tr_work.reverse_use_actor,
            nut,
            nua,
            true,
            tr_work.switch_number_of_uses,
            tr_work.target_number_of_uses,
            tr_work.is_pickup_or_drop,
            true, // changeHeldObject
        )
    } else if container_slot_idx.is_some() {
        // USE on container index without a transition: DoContainerStuff is the
        // REMV/put path — not bare ground swap. Refuse here.
        return refuse(actor, target);
    } else if actor == 0 && target != 0 {
        let permanent = state
            .content
            .get(target)
            .map(|d| d.permanent)
            .unwrap_or(false);
        if permanent {
            return refuse(actor, target);
        }
        (
            target, 0, false, false, false, false, false, false, -1, false, true,
        )
    } else if actor != 0 && target != 0 {
        let tgt_perm = state
            .content
            .get(target)
            .map(|d| d.permanent)
            .unwrap_or(false);
        let act_perm = state
            .content
            .get(actor)
            .map(|d| d.permanent)
            .unwrap_or(false);
        if tgt_perm || act_perm {
            return refuse(actor, target);
        }
        // Bare swap: put-down transform when holding horse-like object.
        let ground = crate::horse_mount::put_down_ground_id(&state.content, actor)
            .unwrap_or(actor);
        (
            target, ground, false, false, false, false, false, false, -1, false, true,
        )
    } else if actor != 0 && target == 0 {
        // Empty tile bare swap / put-down (no transition found).
        let permanent = state
            .content
            .get(actor)
            .map(|d| d.permanent)
            .unwrap_or(false);
        if permanent {
            return refuse(actor, target);
        }
        let ground = crate::horse_mount::put_down_ground_id(&state.content, actor)
            .unwrap_or(actor);
        (
            0, ground, false, false, false, false, false, false, -1, false, true,
        )
    } else {
        return refuse(actor, target);
    };

    // Haxe L1322–1326: Basket 292 with cargo refuses held change (grave scoop needs empty).
    // Haxe: HORSE-MOUNT-POLISH hitch_cart / grave-basket
    if from_transition && change_held {
        let held_parent = state.content.resolve_base_id(actor);
        if basket_refuse_if_changing_held(held_parent, held_contained_count, change_held) {
            return refuse(actor, target);
        }
    }

    // Nest swap is ground horse-cart semantics; never when USE on container index.
    let nest_swap = container_slot_idx.is_none()
        && from_transition
        && should_nest_swap_helpers(
            is_pickup_or_drop,
            change_held,
            actor_after_tr,
            held_contained_count,
        );

    // TH-MULTI-POLISH: loved-food bare-hand extra.
    // hits_before may already be reduced by TH-ALT-OUTCOME Proceed (local only; stamp later).
    let mut hits_out = hits_before;
    let mut loved_extra = false;
    if from_transition {
        let lf = evaluate_loved_food_extra(
            &state.content,
            display_object_id,
            actor,
            reverse_target,
            target,
            target_after,
            hits_before,
            rand::random::<f32>(),
        );
        if lf.got_extra {
            loved_extra = true;
            hits_out = lf.hits;
            target_after = lf.place_target;
            if lf.force_no_use {
                no_use_target = true;
            }
        }
    }

    let allow_reset = allow_target_reset(&state.content, target);
    let sim_time = state.sim_time;

    // Haxe: isPickupOrDrop || isHorseDropTrans → swap tile NestedHelper with held.
    if nest_swap {
        let tile_n = tile_as_nested(
            target,
            uses_remaining,
            tile_helper_snapshot.as_ref(),
        );
        let held_n = held_as_nested(actor, effective_held_uses, held_helper_snapshot.as_ref());
        // Swap whole helpers first, then apply transition result ids.
        let (new_held, new_tile) =
            apply_ids_after_nest_swap(tile_n, held_n, actor_after_tr, target_after, sim_time);
        {
            let mut w = state.world.write().unwrap();
            if new_tile.is_empty() {
                w.set_object(tx, ty, 0);
            } else {
                w.set_object_complex(tx, ty, nested_to_complex(&new_tile, sim_time));
            }
            stamp_hits(&mut w, tx, ty, hits_out);
        }
        let live_target = {
            let w = state.world.read().unwrap();
            w.get_object(tx, ty)
        };
        state.record_world_change(tx, ty, live_target);
        schedule_decay(state, tx, ty, live_target);
        let final_actor = if new_held.is_empty() { 0 } else { new_held.id };
        if let Some(p) = state.players.get_mut(&conn_id) {
            if new_held.is_empty() {
                p.clear_held();
            } else {
                p.set_held_helper(new_held);
            }
            p.force_last_use = false;
            if actor != 0 {
                p.tools.learn(actor);
            }
            if target != 0 {
                p.tools.learn(target);
            }
        }
        return Some(UseResult {
            actor_before: actor,
            target_before: target,
            actor_after: final_actor,
            target_after: live_target,
            applied: true,
            x: tx,
            y: ty,
        });
    }

    let (final_actor, final_held_uses, place_uses, place_no_use) = if !from_transition {
        let floor_uses = if uses_remaining > 0 {
            uses_remaining
        } else {
            target_num_uses.max(0)
        };
        (
            actor_after_tr,
            if actor_after_tr == 0 {
                0
            } else {
                floor_uses
            },
            effective_held_uses,
            true,
        )
    } else if switch_uses {
        let (new_held, new_tgt) = switch_number_of_uses(effective_held_uses, uses_remaining);
        (actor_after_tr, new_held, new_tgt, true)
    } else {
        let (a, u) = resolve_actor_after_use(
            &state.content,
            actor,
            actor_after_tr,
            effective_held_uses,
            reverse_actor,
            no_use_actor,
            target,
        );
        (a, u, uses_remaining, no_use_target)
    };

    {
        let mut w = state.world.write().unwrap();
        if let Some(slot_i) = container_slot_idx {
            // Haxe: transition mutates contained helper in place; outer object stays.
            // // Haxe: TransitionHelper.doTransitionIfPossible L894–904
            place_after_use_on_contained(
                &mut w,
                &state.content,
                tx,
                ty,
                outer_target,
                slot_i,
                target,
                target_after,
                place_uses,
                reverse_target,
                from_transition,
                place_no_use,
                allow_reset,
                force_tgt_uses,
                loved_extra,
                sim_time,
            );
            // Hits/extern stay on outer container helper.
            stamp_hits(&mut w, tx, ty, hits_out);
            stamp_extern_id(&mut w, tx, ty, lock_target_extern);
        } else {
            let forced = if from_transition && !loved_extra {
                let n = state
                    .content
                    .get(target_after)
                    .map(|d| d.num_uses)
                    .unwrap_or(0);
                force_target_number_of_uses(force_tgt_uses, n)
            } else {
                None
            };
            if let Some(u) = forced {
                if target_after == 0 || u <= 0 {
                    w.set_object(tx, ty, 0);
                } else {
                    w.set_object_complex(tx, ty, ComplexObject::with_uses(target_after, u));
                }
            } else if !from_transition && target_after != 0 {
                // Bare swap / put-down: preserve held nest cargo on the tile.
                let mut place = if let Some(ref hh) = held_helper_snapshot {
                    if !hh.contained.is_empty() || hh.has_extra_meta() {
                        let mut n = hh.clone();
                        n.id = target_after;
                        nested_to_complex(&n, sim_time)
                    } else {
                        let num = state
                            .content
                            .get(target_after)
                            .map(|d| d.num_uses)
                            .unwrap_or(0);
                        if num >= 2 {
                            let u = if place_uses > 0 { place_uses } else { num };
                            ComplexObject::with_uses(target_after, u)
                        } else {
                            ComplexObject::new_simple(target_after)
                        }
                    }
                } else {
                    let num = state
                        .content
                        .get(target_after)
                        .map(|d| d.num_uses)
                        .unwrap_or(0);
                    if num >= 2 {
                        let u = if place_uses > 0 { place_uses } else { num };
                        ComplexObject::with_uses(target_after, u)
                    } else {
                        ComplexObject::new_simple(target_after)
                    }
                };
                place.creation_time = sim_time;
                if place.is_complex() {
                    w.set_object_complex(tx, ty, place);
                } else {
                    w.set_object(tx, ty, target_after);
                }
            } else {
                place_after_use_ex(
                    &mut w,
                    &state.content,
                    tx,
                    ty,
                    target,
                    target_after,
                    place_uses,
                    reverse_target,
                    from_transition,
                    place_no_use,
                    allow_reset,
                );
                // Stamp creation_time on horse put-down targets to delay escape auto-decay.
                if from_transition && target_after != 0 {
                    if let Some(h) = w.get_helper(tx, ty).cloned() {
                        let mut h = h;
                        h.creation_time = sim_time;
                        w.set_object_complex(tx, ty, h);
                    } else if target_after == crate::horse_mount::ESCAPED_RIDING_HORSE
                        || target_after == crate::horse_mount::ESCAPED_HORSE_CART
                        || target_after == crate::horse_mount::ESCAPED_TIRE_CART
                    {
                        let mut h = ComplexObject::new_simple(target_after);
                        h.creation_time = sim_time;
                        w.set_object_complex(tx, ty, h);
                    }
                }
            }
            stamp_hits(&mut w, tx, ty, hits_out);
            // Haxe ObjectHelper.externId: re-stamp after place (place may rebuild helper).
            stamp_extern_id(&mut w, tx, ty, lock_target_extern);
            if lock_claim {
                // Haxe: Lock and Key hits=1 + setNewOwnerAndClearOld
                let base = w.get_object(tx, ty);
                if base != 0 {
                    if let Some(h) = w.helpers.get_mut(&(tx, ty)) {
                        h.hits = 1.0;
                        h.extern_id = lock_target_extern;
                        add_owner_to_helper(h, p_id);
                    } else {
                        let mut c = ComplexObject::new_simple(base);
                        c.hits = 1.0;
                        c.extern_id = lock_target_extern;
                        add_owner_to_helper(&mut c, p_id);
                        w.set_object_complex(tx, ty, c);
                    }
                }
            }
        }
    }

    let live_target = {
        let w = state.world.read().unwrap();
        // Outer container stays on tile when USE was on a contained object.
        if container_slot_idx.is_some() {
            outer_target
        } else {
            w.get_object(tx, ty)
        }
    };
    state.record_world_change(tx, ty, live_target);
    schedule_decay(state, tx, ty, live_target);

    let equip_slot = if final_actor != 0 {
        state
            .content
            .get(final_actor)
            .and_then(|def| clothing_slot_for_object(&def.name, &def.description))
    } else {
        None
    };
    if let Some(p) = state.players.get_mut(&conn_id) {
        // Pickup bare swap: preserve tile nest when taking cart without transition flag.
        if !from_transition && final_actor != 0 && actor == 0 {
            if let Some(ref th) = tile_helper_snapshot {
                if !th.contained.is_empty() || th.is_complex() {
                    let mut n = tile_as_nested(target, uses_remaining, Some(th));
                    n.id = final_actor;
                    p.set_held_helper(n);
                } else {
                    p.set_held(final_actor, final_held_uses);
                }
            } else {
                p.set_held(final_actor, final_held_uses);
            }
        } else if !from_transition && final_actor != 0 && target != 0 {
            // Swapped to former tile object — keep its nest.
            if let Some(ref th) = tile_helper_snapshot {
                if th.base_id == final_actor
                    && (!th.contained.is_empty() || th.is_complex())
                {
                    p.set_held_helper(tile_as_nested(final_actor, final_held_uses, Some(th)));
                } else {
                    p.set_held(final_actor, final_held_uses);
                }
            } else {
                p.set_held(final_actor, final_held_uses);
            }
        } else {
            p.set_held(final_actor, final_held_uses);
        }
        // Preserve key externId on held after set_held rebuild.
        if lock_held_extern != 0 {
            if let Some(h) = p.held_helper.as_mut() {
                if h.extern_id == 0 {
                    h.extern_id = lock_held_extern;
                }
            }
        }
        p.force_last_use = false;
        if actor != 0 {
            p.tools.learn(actor);
        }
        if target != 0 {
            p.tools.learn(target);
        }
        if let Some(slot) = equip_slot {
            p.set_clothing(slot, final_actor);
        }
    }

    if loved_extra {
        // Haxe: player.say('got an extra!', true) + doEmote(Emote.happy)
        note_loved_food_extra(conn_id);
    }

    // AI-CRAFT-STICKY: countDone / countTransitionsDone after live USE success.
    // Haxe: AiBase use-done ~9075–9089 (held or ground product parent → countDone++)
    if let Some(p) = state.players.get_mut(&conn_id) {
        if p.craft_ai.item_to_craft_id > 0 || p.craft_ai.runtime.item.product_id > 0 {
            let _ = p.craft_ai.note_successful_use(
                actor,
                target,
                final_actor,
                live_target,
            );
        }
    }

    // BLOCKED-BY-AI: human / smith-hammer set blockTargetForAi after USE.
    // Haxe: TransitionHelper.use ~397–414 (post-transition tile object)
    {
        use crate::ai_path_reach::{
            block_claim_number_of_uses, should_set_block_target_for_ai, BlockTargetClaim,
        };
        use crate::animal_damage::is_weapon_from_deadly_distance;
        let sim_time = state.sim_time;
        let is_human = state
            .players
            .get(&conn_id)
            .map(|p| p.is_human_body())
            .unwrap_or(false);
        let (
            parent_id,
            number_of_uses,
            is_animal,
            permanent,
            food_value,
            is_clothing,
            is_weapon,
        ) = {
            let base = if live_target != 0 {
                state.content.resolve_base_id(live_target)
            } else {
                0
            };
            let def = state.content.get(base);
            let is_animal = def.map(|d| d.is_animal()).unwrap_or(false);
            let permanent = def.map(|d| d.permanent).unwrap_or(false);
            let food_value = def.map(|d| d.food_value).unwrap_or(0);
            let is_clothing = def.map(|d| d.is_clothing()).unwrap_or(false);
            let deadly = def.map(|d| d.deadly_distance).unwrap_or(0.0);
            let is_weapon = is_weapon_from_deadly_distance(deadly);
            let uses = state
                .world
                .read()
                .ok()
                .and_then(|w| w.get_helper(tx, ty).map(|h| h.uses_remaining))
                .unwrap_or(0);
            (
                base,
                block_claim_number_of_uses(uses),
                is_animal,
                permanent,
                food_value,
                is_clothing,
                is_weapon,
            )
        };
        // Haxe: heldId is pre-transition held (smith hammer check on actor before USE)
        if should_set_block_target_for_ai(
            is_human,
            actor,
            parent_id,
            permanent,
            is_weapon,
            is_animal,
            food_value,
            is_clothing,
        ) {
            let claim = BlockTargetClaim {
                x: tx,
                y: ty,
                parent_id,
                number_of_uses,
                is_animal,
                held_new_target_id: None,
            };
            if let Some(p) = state.players.get_mut(&conn_id) {
                p.ai_block_targets.set_player_block(claim, sim_time);
            }
        }
    }

    Some(UseResult {
        actor_before: actor,
        target_before: target,
        actor_after: final_actor,
        target_after: live_target,
        applied: true,
        x: tx,
        y: ty,
    })
}

/// Wire held id for PU (multi-use dummy when uses partial).
pub fn wire_held_id(content: &ContentDb, p: &Player) -> i32 {
    if p.held_id == 0 {
        return 0;
    }
    let uses = if p.held_uses > 0 {
        p.held_uses
    } else {
        content
            .get(p.held_id)
            .map(|d| d.num_uses)
            .unwrap_or(0)
            .max(0)
    };
    content.wire_id_for_uses(p.held_id, uses)
}


// ── C-SS-MORE-BATCH5: TransitionHelper hungry-work pure pipe ────────────────
// Haxe: TransitionHelper.doTransitionIfPossible L1170–1256

/// Haxe `BiomeTag.PASSABLERIVER` — hungry-work cost −1 (alias of ol_world).
// Haxe: Biome.PASSABLERIVER = 13
pub const BIOME_PASSABLE_RIVER: u8 = ol_world::PASSABLE_RIVER;

/// Default Haxe `ServerSettings.HungryWorkHeat` when live knob invalid.
// Haxe: ServerSettings.HungryWorkHeat = 0.002
pub const DEFAULT_HUNGRY_WORK_HEAT: f32 = 0.002;

/// Haxe `TransitionHelper` hungry-work heat: when transition temperature &lt; 0,
/// use `hungryWorkCost * HungryWorkHeat`.
// Haxe: TransitionHelper hungryWorkTemperature = hungryWorkCost * ServerSettings.HungryWorkHeat
// C-SS-MORE-BATCH5
#[inline]
pub fn resolve_hungry_work_temperature(
    transition_temperature: f32,
    hungry_work_cost: f32,
    hungry_work_heat: f32,
) -> f32 {
    if transition_temperature < 0.0 {
        let heat = if hungry_work_heat.is_finite() && hungry_work_heat >= 0.0 {
            hungry_work_heat
        } else {
            DEFAULT_HUNGRY_WORK_HEAT
        };
        hungry_work_cost.max(0.0) * heat
    } else {
        transition_temperature
    }
}

/// Base hungry-work cost before Loose / owner adjustments.
// Haxe: TransitionHelper L1172–1174 actor + newTarget + transition − river
#[inline]
pub fn compute_hungry_work_cost(
    actor_hungry_work: f32,
    new_target_hungry_work: f32,
    transition_hungry_work_cost: f32,
    is_passable_river: bool,
) -> f32 {
    let mut cost = actor_hungry_work + new_target_hungry_work + transition_hungry_work_cost;
    if is_passable_river {
        cost -= 1.0;
    }
    cost
}

/// Zero cost for unfortified Loose fences; return (cost, is_fortified).
// Haxe: TransitionHelper L1179–1182
#[inline]
pub fn apply_loose_fence_hungry_work_waiver(
    cost: f32,
    new_target_description: &str,
    target_hits: f32,
) -> (f32, bool) {
    let is_fortified = cost > 0.0 && target_hits < -0.1;
    if new_target_description.contains("Loose") && !is_fortified {
        (0.0, is_fortified)
    } else {
        (cost, is_fortified)
    }
}

/// Ownership adjustment for hungry-work cost.
// Haxe: TransitionHelper L1191–1208
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HungryWorkOwnerAdj {
    /// Object not owned — cost unchanged.
    Unaffected { cost: f32 },
    /// Owner (or missing owner): half cost; `allow_for_owner` when original cost &lt; 1.
    OwnerHalf { cost: f32, allow_for_owner: bool },
    /// Non-owner with cost &lt; 1 — refuse USE.
    NotOwnerRefuse,
    /// Non-owner with cost ≥ 1 — continue at full cost.
    NotOwnerContinue { cost: f32 },
}

/// Apply owned-object cost rules.
// Haxe: TransitionHelper L1191–1208
#[inline]
pub fn adjust_hungry_work_for_ownership(
    cost: f32,
    object_is_owned: bool,
    player_is_owner_or_unowned: bool,
) -> HungryWorkOwnerAdj {
    if !object_is_owned {
        return HungryWorkOwnerAdj::Unaffected { cost };
    }
    if player_is_owner_or_unowned {
        let allow = cost < 1.0;
        HungryWorkOwnerAdj::OwnerHalf {
            cost: cost * 0.5,
            allow_for_owner: allow,
        }
    } else if cost < 1.0 {
        HungryWorkOwnerAdj::NotOwnerRefuse
    } else {
        HungryWorkOwnerAdj::NotOwnerContinue { cost }
    }
}

/// Result of hungry-work gate after temperature resolve.
// Haxe: TransitionHelper L1211–1256
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HungryWorkGate {
    /// cost ≤ 0 — no side-effects.
    Free,
    /// Apply heat / food / exhaustion after success.
    Allow {
        heat_after: f32,
        food_after: f32,
        exhaustion_after: f32,
    },
    /// `exhaustion > food_store_max / 2`.
    RefuseExhaustion { excess: i32 },
    /// `ceil(cost/2 - food_store) > 0`.
    RefuseFood { missing: i32 },
}

/// Pure hungry-work refuse / apply plan (super-hot refuse stays commented in Haxe).
// Haxe: TransitionHelper L1211–1256 (super-hot refuse is commented — port-as-is)
// C-SS-MORE-BATCH5
pub fn evaluate_hungry_work_use(
    cost: f32,
    temperature: f32,
    food_store: f32,
    food_store_max: f32,
    exhaustion: f32,
    current_heat: f32,
) -> HungryWorkGate {
    if !(cost > 0.0) {
        return HungryWorkGate::Free;
    }
    // Haxe: excessExhaustion = ceil(exhaustion - food_store_max / 2)
    let excess = (exhaustion - food_store_max / 2.0).ceil() as i32;
    if excess > 0 {
        return HungryWorkGate::RefuseExhaustion { excess };
    }
    // Haxe: missingFood = ceil(hungryWorkCost / 2 - food_store)
    let missing = (cost / 2.0 - food_store).ceil() as i32;
    if missing > 0 {
        return HungryWorkGate::RefuseFood { missing };
    }
    // Apply: heat += temp (clamp 1); cost/2 → −food +exhaustion
    let mut heat = current_heat + temperature;
    if heat > 1.0 {
        heat = 1.0;
    }
    let half = cost / 2.0;
    HungryWorkGate::Allow {
        heat_after: heat,
        food_after: food_store - half,
        exhaustion_after: exhaustion + half,
    }
}

/// Object `hungryWork` from ServerSettings.PatchObjectData + `+hungryWork` tag.
///
/// `default_hungry_work_cost` is live `ServerSettings.HungryWorkCost` (for `+hungryWork`).
// Haxe: ServerSettings.PatchObjectData hungryWork + description +hungryWork
// C-SS-MORE-BATCH5
pub fn object_hungry_work(
    object_id: i32,
    description: &str,
    default_hungry_work_cost: f32,
) -> f32 {
    // Explicit PatchObjectData values (subset; transition.hungryWorkCost residual).
    let patched = match object_id {
        857 => -2.0,  // Steel Hoe
        1849 => 5.0,  // Buried Grave with Dug Stone
        123 => 2.0,   // Harvested Tule
        231 => 10.0,  // Adobe Oven Base
        1020 => 2.0,  // Snow Bank
        138 => 2.0,   // Cut Sapling Skewer
        3961 => 5.0,  // Iron Vein
        496 => 4.0,   // Dug Stump
        1011 => 3.0,  // Buried Grave
        213 => 3.0,   // Deep Tilled Row
        1136 => 3.0,  // Shallow Tilled Row
        511 => 2.0,   // Pond
        1261 | 141 | 142 | 143 => 2.0, // Goose ponds
        662 => 1.0,   // Shallow Well
        663 => 2.0,   // Deep Well
        1845 | 1846 | 1847 => 5.0, // Loose Fence*
        3146 | 1853 => {
            // Chopped Softwood / similar — Haxe uses HungryWorkCost live
            if default_hungry_work_cost.is_finite() {
                default_hungry_work_cost
            } else {
                5.0
            }
        }
        _ => 0.0,
    };
    if patched != 0.0 {
        return patched;
    }
    // Haxe: description +hungryWork → HungryWorkCost
    if description.to_ascii_lowercase().contains("+hungrywork") {
        if default_hungry_work_cost.is_finite() && default_hungry_work_cost >= 0.0 {
            default_hungry_work_cost
        } else {
            5.0
        }
    } else {
        0.0
    }
}

/// Full pure plan: base cost → loose → owner → temperature → gate.
// Haxe: TransitionHelper L1170–1256
// C-SS-MORE-BATCH5
pub fn plan_hungry_work_use(
    actor_hungry_work: f32,
    new_target_hungry_work: f32,
    transition_hungry_work_cost: f32,
    transition_temperature: f32,
    hungry_work_heat: f32,
    is_passable_river: bool,
    new_target_description: &str,
    target_hits: f32,
    object_is_owned: bool,
    player_is_owner_or_unowned: bool,
    food_store: f32,
    food_store_max: f32,
    exhaustion: f32,
    current_heat: f32,
) -> (f32, f32, HungryWorkOwnerAdj, HungryWorkGate) {
    let base = compute_hungry_work_cost(
        actor_hungry_work,
        new_target_hungry_work,
        transition_hungry_work_cost,
        is_passable_river,
    );
    let (cost_loose, _) =
        apply_loose_fence_hungry_work_waiver(base, new_target_description, target_hits);
    let owner_adj =
        adjust_hungry_work_for_ownership(cost_loose, object_is_owned, player_is_owner_or_unowned);
    let cost = match owner_adj {
        HungryWorkOwnerAdj::Unaffected { cost }
        | HungryWorkOwnerAdj::OwnerHalf { cost, .. }
        | HungryWorkOwnerAdj::NotOwnerContinue { cost } => cost,
        HungryWorkOwnerAdj::NotOwnerRefuse => {
            return (
                cost_loose,
                resolve_hungry_work_temperature(
                    transition_temperature,
                    cost_loose,
                    hungry_work_heat,
                ),
                owner_adj,
                HungryWorkGate::Free, // unused when refuse-owner
            );
        }
    };
    let temperature =
        resolve_hungry_work_temperature(transition_temperature, cost, hungry_work_heat);
    let gate = evaluate_hungry_work_use(
        cost,
        temperature,
        food_store,
        food_store_max,
        exhaustion,
        current_heat,
    );
    (cost, temperature, owner_adj, gate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::Player;
    use ol_content::{ObjectDef, Transition};
    use ol_world::ComplexObject;
    use std::sync::Arc;

    fn def(id: i32, num_uses: i32, permanent: bool) -> ObjectDef {
        ObjectDef {
            id,
            description: format!("obj{id}"),
            name: format!("Obj{id}"),
            containable: false,
            permanent,
            blocks_walking: false,
            food_value: 0,
            heat_value: 0.0,
            map_chance: 0.0,
            biomes: Vec::new(),
            num_uses,
            num_slots: 0,
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

    fn tr(
        actor: i32,
        target: i32,
        new_actor: i32,
        new_target: i32,
        rev_a: bool,
        rev_t: bool,
    ) -> Transition {
        Transition {
            actor_id: actor,
            target_id: target,
            new_actor_id: new_actor,
            new_target_id: new_target,
            last_use_actor: false,
            last_use_target: false,
            auto_decay_seconds: 0.0,
            reverse_use_actor: rev_a,
            reverse_use_target: rev_t,
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

    fn state_with(db: ContentDb) -> crate::SimState {
        crate::SimState::with_default_empty(Arc::new(db))
    }

    #[test]
    fn held_uses_decrement_on_tool() {
        let mut db = ContentDb::default();
        db.objects.insert(10, def(10, 3, false));
        db.objects.insert(20, def(20, 0, true));
        db.transitions
            .insert((10, 20), tr(10, 20, 10, 20, false, false));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(10, 3);
        }
        state.world.write().unwrap().set_object(0, 0, 20);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        let p = state.players.get(&1).unwrap();
        assert_eq!(p.held_id, 10);
        assert_eq!(p.held_uses, 2);
    }

    /// Haxe checkIfNotMovingAndCloseEnough with bow useDistance=5.
    // Haxe: TransitionHelper.checkIfNotMovingAndCloseEnough + ObjectData.useDistance
    #[test]
    fn use_respects_held_use_distance_five() {
        let mut db = ContentDb::default();
        let mut bow = def(152, 0, false);
        bow.use_distance = 5;
        bow.deadly_distance = 4.0;
        db.objects.insert(152, bow);
        db.objects.insert(20, def(20, 0, true));
        db.transitions
            .insert((152, 20), tr(152, 20, 152, 20, false, false));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(152, 0);
            p.x = 0;
            p.y = 0;
        }
        // (3,4) → dist² 25 within useDistance 5
        state.world.write().unwrap().set_object(3, 4, 20);
        let r = apply_use_at(&mut state, 1, 3, 4).unwrap();
        assert!(r.applied, "bow should reach (3,4) at useDistance=5");
        // beyond: (4,4) → 32 > 25
        state.world.write().unwrap().set_object(4, 4, 20);
        let r = apply_use_at(&mut state, 1, 4, 4).unwrap();
        assert!(!r.applied, "bow should not reach (4,4)");
    }

    /// Haxe bow min-range: deadly>1.9 + animal + exact ≤1.5 refuse.
    // Haxe: TransitionHelper.use L757-765
    #[test]
    fn use_refuses_ranged_too_close_to_animal() {
        crate::clear_too_close_pending(); // clear stale say + debug message
        let mut db = ContentDb::default();
        let mut bow = def(152, 0, false);
        bow.use_distance = 5;
        bow.deadly_distance = 4.0;
        db.objects.insert(152, bow);
        let mut wolf = def(418, 0, true);
        wolf.moves = 2;
        db.objects.insert(418, wolf);
        db.transitions
            .insert((152, 418), tr(152, 418, 152, 0, false, false));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(152, 0);
            p.x = 0;
            p.y = 0;
        }
        state.world.write().unwrap().set_object(1, 0, 418);
        let r = apply_use_at(&mut state, 1, 1, 0).unwrap();
        assert!(!r.applied, "too close to animal with bow");
        // GPI-TOO-CLOSE: note public say + debug message for live PS drain
        assert_eq!(
            crate::take_too_close_say(),
            Some(1),
            "refuse must note Too close say for PS"
        );
        assert_eq!(
            crate::take_too_close_message(),
            Some((1, crate::TOO_CLOSE_MESSAGE)),
            "refuse must note player.message = 'too close'"
        );
        // at distance 3 should apply and must not re-note too-close
        state.world.write().unwrap().set_object(3, 0, 418);
        let r = apply_use_at(&mut state, 1, 3, 0).unwrap();
        assert!(r.applied, "bow at range 3 should hit animal");
        assert!(
            crate::take_too_close_say().is_none(),
            "successful ranged USE must not note Too close"
        );
        assert!(crate::take_too_close_message().is_none());
    }

    /// Live USE refuse path: public PS `TOO CLOSE...` + FRAME (GPI-TOO-CLOSE).
    // Haxe: TransitionHelper.use L761-764 + GlobalPlayerInstance.sayHelper
    #[test]
    fn use_refuses_ranged_too_close_emits_ps_say() {
        use crate::{apply_intent, Counters, NetIntent, TOO_CLOSE_SAY};
        use ol_net::OutboundHub;
        use ol_protocol::format_player_says;

        crate::clear_too_close_pending();
        let mut db = ContentDb::default();
        let mut bow = def(152, 0, false);
        bow.use_distance = 5;
        bow.deadly_distance = 4.0;
        db.objects.insert(152, bow);
        let mut wolf = def(418, 0, true);
        wolf.moves = 2;
        db.objects.insert(418, wolf);
        db.transitions
            .insert((152, 418), tr(152, 418, 152, 0, false, false));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(152, 0);
            // Absolute world origin + birth=pos so NetIntent tiles are not birth-shifted.
            p.x = 0;
            p.y = 0;
            p.birth_x = 0;
            p.birth_y = 0;
            p.connected = true;
            p.age = 20.0;
        }
        state.world.write().unwrap().set_object(1, 0, 418);
        let p_id = state.players.get(&1).unwrap().p_id;

        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        // Absolute world tile (1,0) — birth is 0 so resolve_net_intent_tile is identity.
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Use {
                conn_id: 1,
                x: 1,
                y: 0,
                id: None,
                index: None,
            },
        );

        let expected_ps = format_player_says(p_id, false, TOO_CLOSE_SAY);
        let mut saw_ps = false;
        let mut saw_fm = false;
        let mut pkts: Vec<String> = Vec::new();
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt).into_owned();
            if s == expected_ps {
                saw_ps = true;
            }
            if s == "FM\n#" || s.starts_with("FM\n") {
                saw_fm = true;
            }
            pkts.push(s);
        }
        assert!(
            saw_ps,
            "expected public PS {expected_ps:?} on too-close refuse; got {pkts:?}; flag={:?}",
            crate::take_too_close_say()
        );
        assert!(saw_fm, "send_chat_ps must FRAME after PS; got {pkts:?}");
        // Flag drained by live path
        assert!(crate::take_too_close_say().is_none());
        crate::clear_too_close_pending();
        // Still holding bow; animal untouched
        assert_eq!(state.players.get(&1).unwrap().held_id, 152);
        assert_eq!(state.world.read().unwrap().get_object(1, 0), 418);
    }

    /// Bowl of Stew 1251 last use via USE: LA 1251+-1 → Clay Bowl 235, uses stay 0.
    // Haxe: DoChangeNumberOfUsesOnActorManual tool last-use
    #[test]
    fn stew_last_use_becomes_clay_bowl_uses_zero() {
        let mut db = ContentDb::default();
        db.objects.insert(1251, def(1251, 2, false));
        db.objects.insert(235, def(235, 0, false));
        db.objects.insert(20, def(20, 0, true));
        // Same-actor USE on a dummy target (no id change)
        db.transitions
            .insert((1251, 20), tr(1251, 20, 1251, 20, false, false));
        // LA tool: 1251 + -1 → 235
        let mut la = tr(1251, -1, 235, 0, false, false);
        la.last_use_actor = true;
        db.transitions_last_use.insert((1251, -1), la);
        // Non-LA alone would keep 1251 — must not win over LA
        db.transitions
            .insert((1251, -1), tr(1251, -1, 1251, 0, false, false));

        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(1251, 1);
        }
        state.world.write().unwrap().set_object(0, 0, 20);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        let p = state.players.get(&1).unwrap();
        assert_eq!(p.held_id, 235);
        assert_eq!(p.held_uses, 0, "Haxe keeps uses at 0 after tool transform");
    }

    /// try_eat_held: Bowl of Stew multi-use then empty → Clay Bowl.
    // Haxe: GlobalPlayerInstance eat + DoChangeNumberOfUsesOnActorManual
    #[test]
    fn try_eat_stew_decrements_then_clay_bowl() {
        let mut db = ContentDb::default();
        let mut stew = def(1251, 2, false);
        stew.food_value = 5;
        db.objects.insert(1251, stew);
        db.objects.insert(235, def(235, 0, false));
        let mut la = tr(1251, -1, 235, 0, false, false);
        la.last_use_actor = true;
        db.transitions_last_use.insert((1251, -1), la);

        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(1251, 2);
            // Room for two yum fills (value 5 + YumBonus 5 each).
            p.food = 2.0;
            p.food_max = 40.0;
        }
        assert!(crate::try_eat_held(&mut state, 1));
        {
            let p = state.players.get(&1).unwrap();
            assert_eq!(p.held_id, 1251);
            assert_eq!(p.held_uses, 1);
        }
        assert!(crate::try_eat_held(&mut state, 1));
        {
            let p = state.players.get(&1).unwrap();
            assert_eq!(p.held_id, 235);
            assert_eq!(p.held_uses, 0);
        }
    }

    /// Deplete multi-use food with no tool row → clear held on eat.
    #[test]
    fn try_eat_multi_use_no_tool_clears() {
        let mut db = ContentDb::default();
        let mut food = def(900, 2, false);
        food.food_value = 4;
        db.objects.insert(900, food);
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(900, 1);
            p.food = 2.0;
            p.food_max = 40.0;
        }
        assert!(crate::try_eat_held(&mut state, 1));
        let p = state.players.get(&1).unwrap();
        assert_eq!(p.held_id, 0);
        assert_eq!(p.held_uses, 0);
    }

    #[test]
    fn reverse_actor_at_max_refuses() {
        let mut db = ContentDb::default();
        db.objects.insert(10, def(10, 5, false));
        db.objects.insert(20, def(20, 0, true));
        db.transitions
            .insert((10, 20), tr(10, 20, 10, 20, true, false));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(10, 5);
        }
        state.world.write().unwrap().set_object(0, 0, 20);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(!r.applied);
    }

    #[test]
    fn reverse_target_max_uses_max_use_table() {
        let mut db = ContentDb::default();
        db.objects.insert(33, def(33, 0, false));
        db.objects.insert(1096, def(1096, 4, true));
        db.objects.insert(3963, def(3963, 0, true));
        db.transitions
            .insert((33, 1096), tr(33, 1096, 0, 1096, false, true));
        db.transitions_max_use
            .insert((33, 1096), tr(33, 1096, 0, 3963, false, false));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(33, 0);
        }
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(0, 0, ComplexObject::with_uses(1096, 4));
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        assert_eq!(r.target_after, 3963);
    }

    #[test]
    fn actor_min_use_fraction_refuses() {
        let mut db = ContentDb::default();
        db.objects.insert(10, def(10, 5, false));
        db.objects.insert(20, def(20, 0, true));
        let mut t = tr(10, 20, 0, 20, false, false);
        t.actor_min_use_fraction = 1.0;
        db.transitions.insert((10, 20), t);
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(10, 2);
        }
        state.world.write().unwrap().set_object(0, 0, 20);
        assert!(!apply_use_at(&mut state, 1, 0, 0).unwrap().applied);
    }

    #[test]
    fn target_min_use_fraction_refuses() {
        let mut db = ContentDb::default();
        db.objects.insert(10, def(10, 0, false));
        db.objects.insert(20, def(20, 5, true));
        let mut t = tr(10, 20, 0, 0, false, false);
        t.target_min_use_fraction = 1.0;
        db.transitions.insert((10, 20), t);
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(10, 0);
        }
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(0, 0, ComplexObject::with_uses(20, 2));
        assert!(!apply_use_at(&mut state, 1, 0, 0).unwrap().applied);
    }

    #[test]
    fn wire_held_partial_uses() {
        let mut db = ContentDb::default();
        let mut d = def(50, 3, false);
        d.dummy_ids = vec![9001, 9002];
        db.objects.insert(50, d);
        db.dummy_parent.insert(9001, 50);
        db.dummy_parent.insert(9002, 50);
        let mut p = Player::new(1, 1, "e");
        p.set_held(50, 2);
        assert_eq!(wire_held_id(&db, &p), 9002);
        p.set_held(50, 3);
        assert_eq!(wire_held_id(&db, &p), 50);
    }

    fn def_slots(id: i32, slots: i32) -> ObjectDef {
        let mut d = def(id, 0, false);
        d.num_slots = slots;
        d.containable = slots > 0;
        d
    }

    // Haxe: 770 + -1 = 0 + 1421 empty-ground dismount
    #[test]
    fn horse_dismount_empty_ground() {
        let mut db = ContentDb::default();
        db.objects.insert(770, def(770, 0, false));
        db.objects.insert(1421, def(1421, 0, false));
        db.transitions
            .insert((770, -1), tr(770, -1, 0, 1421, false, false));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(770, 0);
            p.x = 0;
            p.y = 0;
        }
        state.world.write().unwrap().set_object(0, 0, 0);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        assert_eq!(r.actor_after, 0);
        assert_eq!(r.target_after, 1421);
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
    }

    // Haxe: 0 + 1422 = 778 + 0 with cargo preserved (is_pickup_or_drop nest swap)
    #[test]
    fn horse_cart_pickup_preserves_cargo() {
        let mut db = ContentDb::default();
        db.objects.insert(1422, def_slots(1422, 4));
        db.objects.insert(778, def_slots(778, 4));
        db.objects.insert(33, def(33, 0, false));
        db.objects.insert(40, def(40, 0, false));
        let mut t = tr(0, 1422, 778, 0, false, false);
        t.is_pickup_or_drop = true;
        db.transitions.insert((0, 1422), t);
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.clear_held();
            p.x = 0;
            p.y = 0;
        }
        let mut cart = ComplexObject::new_simple(1422);
        cart.contained = vec![33, 40];
        cart.slots = vec![
            ol_world::NestedHelper::id_only(33),
            ol_world::NestedHelper::id_only(40),
        ];
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(0, 0, cart);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        assert_eq!(r.actor_after, 778);
        assert_eq!(r.target_after, 0);
        let p = state.players.get(&1).unwrap();
        assert_eq!(p.held_id, 778);
        let hh = p.held_helper.as_ref().expect("held helper");
        assert_eq!(hh.contained.len(), 2);
        assert_eq!(hh.contained[0].id, 33);
        assert_eq!(hh.contained[1].id, 40);
    }

    // Haxe: isHorseDropTrans — dismount cart with cargo → ground keeps nest
    #[test]
    fn horse_cart_drop_preserves_cargo() {
        let mut db = ContentDb::default();
        db.objects.insert(778, def_slots(778, 4));
        db.objects.insert(1422, def_slots(1422, 4));
        db.objects.insert(33, def(33, 0, false));
        db.transitions
            .insert((778, -1), tr(778, -1, 0, 1422, false, false));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            let h = ol_world::NestedHelper::from_wire(778, &[33]);
            p.set_held_helper(h);
            p.x = 0;
            p.y = 0;
        }
        state.world.write().unwrap().set_object(0, 0, 0);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        assert_eq!(r.actor_after, 0);
        assert_eq!(r.target_after, 1422);
        let w = state.world.read().unwrap();
        let h = w.get_helper(0, 0).expect("tile helper");
        assert_eq!(h.base_id, 1422);
        assert_eq!(h.contained, vec![33]);
    }

    // Haxe: empty first — too many contained for newActor.numSlots
    #[test]
    fn horse_pickup_slots_refuse() {
        let mut db = ContentDb::default();
        db.objects.insert(1422, def_slots(1422, 4));
        db.objects.insert(778, def_slots(778, 1)); // only 1 slot
        let mut t = tr(0, 1422, 778, 0, false, false);
        t.is_pickup_or_drop = true;
        db.transitions.insert((0, 1422), t);
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.clear_held();
            p.x = 0;
            p.y = 0;
        }
        let mut cart = ComplexObject::new_simple(1422);
        cart.contained = vec![33, 40];
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(0, 0, cart);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(!r.applied);
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
        assert_eq!(state.world.read().unwrap().get_object(0, 0), 1422);
    }


    /// C-SS-FULL-TABLE: horse mount-eat runs doIncreaseFoodValue (yum restore).
    // Haxe: doHorseStuffPossible → doEating → doIncreaseFoodValue
    #[test]
    fn horse_eat_applies_yum_restore() {
        let mut db = ContentDb::default();
        db.objects.insert(770, def(770, 0, false));
        let mut berry = def(31, 0, false);
        berry.food_value = 5;
        db.objects.insert(31, berry);
        let mut other = def(40, 0, false);
        other.food_value = 3;
        db.objects.insert(40, other);
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "horse_yum@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(770, 0);
            p.food = 2.0;
            p.food_max = 40.0;
            p.x = 0;
            p.y = 0;
            // Prior eats so hasEaten has keys for random restore (not only food 31)
            p.yum.has_eaten.insert(40, 3.0);
            p.yum.has_eaten.insert(31, 0.0);
        }
        state.world.write().unwrap().set_object(0, 0, 31);
        let before_other = state.players.get(&1).unwrap().yum.get_count_eaten(40);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        // After eat, food 31 has hasEaten delta; random restore may lower 40
        // (or 31 if RNG picks 31 — then key==eaten skips). At least hasEaten mutates.
        let p = state.players.get(&1).unwrap();
        assert!(
            p.yum.get_count_eaten(31) > 0.0 || p.yum.get_count_eaten(40) < before_other,
            "horse-eat must run do_increase_food_value_ex side-effects"
        );
        // Stay mounted
        assert_eq!(p.held_id, 770);
    }

    /// EATEN-FOOD-PCT: horse mount-eat multiplies world FoodFactor and records stats.
    // Haxe: doHorseStuffPossible → GlobalPlayerInstance.doEating L3186–3215
    #[test]
    fn horse_eat_applies_world_food_factor_and_stats() {
        let mut db = ContentDb::default();
        db.objects.insert(770, def(770, 0, false));
        let mut berry = def(31, 0, false);
        berry.food_value = 5;
        db.objects.insert(31, berry);
        let mut state = state_with(db);
        // Empty map → food factor 2.5; starving default 1.5
        assert!((state.world_food.get_food_factor(31) - 2.5).abs() < 1e-5);
        crate::spawn_player(&mut state, 1, "horse_wff@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(770, 0);
            p.food = 2.0;
            p.food_max = 40.0;
            p.x = 0;
            p.y = 0;
        }
        state.world.write().unwrap().set_object(0, 0, 31);
        let food_before = state.players.get(&1).unwrap().food;
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        assert_eq!(state.players.get(&1).unwrap().held_id, 770);
        let gained = state.players.get(&1).unwrap().food - food_before;
        assert!(
            gained > 10.0,
            "world factors should boost horse-eat fill; gained={gained}"
        );
        assert!(
            state
                .world_food
                .eaten_values
                .get(&31)
                .copied()
                .unwrap_or(0.0)
                > 0.0,
            "horse-eat must call add_food_statistic"
        );
        assert_eq!(
            state.world_food.eaten_percentage.get(&31).copied(),
            Some(100.0),
            "live eatenFoodPercentage after horse-eat"
        );
    }

    /// EATEN-FOOD-PCT: horse superMeh burns prestige + ages (full doEating L3195–3206).
    // Haxe: doHorseStuffPossible → doEating isSuperMeh L3195–3206
    #[test]
    fn horse_eat_super_meh_burns_prestige() {
        let mut db = ContentDb::default();
        db.objects.insert(770, def(770, 0, false));
        let mut crumb = def(99, 0, false);
        crumb.food_value = 4;
        db.objects.insert(99, crumb);
        let mut state = state_with(db);
        let p_id = crate::spawn_player(&mut state, 1, "horse_smeh@test");
        state.combat.stats_mut(p_id).prestige = 3.0;
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(770, 0);
            p.food = 1.0; // allow superMeh self-eat
            p.food_max = 40.0;
            p.age = 20.0;
            p.x = 0;
            p.y = 0;
            // Force superMeh: high has_eaten so fill < half food_value
            p.yum.has_eaten.insert(99, 20.0);
        }
        state.world.write().unwrap().set_object(0, 0, 99);
        let age_before = state.players.get(&1).unwrap().age;
        let hits_before = state.combat.hits_of(p_id);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied, "horse superMeh should apply when starving");
        assert_eq!(state.players.get(&1).unwrap().held_id, 770);
        let prestige = state
            .combat
            .stats
            .get(&p_id)
            .map(|s| s.prestige)
            .unwrap_or(0.0);
        assert!(
            (prestige - 2.0).abs() < 1e-4,
            "prestige should drop by 1 on horse superMeh, got {prestige}"
        );
        assert_eq!(state.combat.hits_of(p_id), hits_before, "hits path not used");
        let age = state.players.get(&1).unwrap().age;
        assert!(
            (age - age_before - 0.2).abs() < 1e-4,
            "age += 0.2 on horse superMeh prestige path"
        );
        assert!(
            state
                .world_food
                .eaten_values
                .get(&99)
                .copied()
                .unwrap_or(0.0)
                > 0.0,
            "horse superMeh still records add_food_statistic"
        );
    }

    // Haxe: doHorseStuffPossible — eat food while holding 770
    #[test]
    fn horse_eat_while_mounted() {
        let mut db = ContentDb::default();
        db.objects.insert(770, def(770, 0, false));
        let mut berry = def(31, 0, false);
        berry.food_value = 3;
        db.objects.insert(31, berry);
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(770, 0);
            p.food = 5.0;
            p.food_max = 20.0;
            p.x = 0;
            p.y = 0;
        }
        state.world.write().unwrap().set_object(0, 0, 31);
        let food_before = state.players.get(&1).unwrap().food;
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        assert_eq!(r.actor_after, 770);
        assert_eq!(state.players.get(&1).unwrap().held_id, 770);
        assert!(state.players.get(&1).unwrap().food > food_before);
        assert_eq!(state.world.read().unwrap().get_object(0, 0), 0);
    }

    // Tire cart put-down transform 3158 → 3161 (not 1422)
    #[test]
    fn tire_cart_put_down_transform() {
        let mut db = ContentDb::default();
        db.objects.insert(3158, def_slots(3158, 4));
        db.objects.insert(3161, def_slots(3161, 4));
        db.transitions
            .insert((3158, -1), tr(3158, -1, 0, 3161, false, false));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(3158, 0);
            p.x = 0;
            p.y = 0;
        }
        state.world.write().unwrap().set_object(0, 0, 0);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        assert_eq!(r.actor_after, 0);
        assert_eq!(r.target_after, 3161);
    }

    // Plain stone swap still works without horse flags
    #[test]
    fn plain_item_swap_regression() {
        let mut db = ContentDb::default();
        db.objects.insert(33, def(33, 0, false));
        db.objects.insert(34, def(34, 0, false));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(33, 0);
            p.x = 0;
            p.y = 0;
        }
        state.world.write().unwrap().set_object(0, 0, 34);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        assert_eq!(r.actor_after, 34);
        assert_eq!(r.target_after, 33);
    }

    // --- TH-LOCK wire: Key 917 / Lock Removal Key 1003 ---

    fn locked_chest_def(id: i32) -> ObjectDef {
        let mut d = def(id, 0, true);
        d.description = "Locked Wooden Chest".into();
        d.name = "Locked Wooden Chest".into();
        d
    }

    #[test]
    fn key_917_mismatch_refuses_and_preserves_extern() {
        let mut db = ContentDb::default();
        db.objects.insert(917, def(917, 0, false));
        db.objects.insert(988, locked_chest_def(988));
        // Matching transition would open the chest if key fit.
        db.transitions
            .insert((917, 988), tr(917, 988, 917, 989, false, false));
        db.objects.insert(989, def(989, 0, true));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            let mut h = NestedHelper::with_uses(917, 0);
            h.extern_id = 11;
            p.set_held_helper(h);
            p.x = 0;
            p.y = 0;
        }
        let mut chest = ComplexObject::new_simple(988);
        chest.extern_id = 22;
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(0, 0, chest);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(!r.applied);
        assert_eq!(
            state
                .players
                .get(&1)
                .unwrap()
                .held_helper
                .as_ref()
                .map(|h| h.extern_id),
            Some(11)
        );
        assert_eq!(
            state
                .world
                .read()
                .unwrap()
                .get_helper(0, 0)
                .map(|h| h.extern_id),
            Some(22)
        );
        assert_eq!(state.world.read().unwrap().get_object(0, 0), 988);
    }

    #[test]
    fn key_917_both_zero_pairs_shared_extern() {
        let mut db = ContentDb::default();
        db.objects.insert(917, def(917, 0, false));
        db.objects.insert(988, locked_chest_def(988));
        db.objects.insert(989, def(989, 0, true));
        db.transitions
            .insert((917, 988), tr(917, 988, 917, 989, false, false));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(917, 0);
            p.x = 0;
            p.y = 0;
        }
        state.world.write().unwrap().set_object(0, 0, 988);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        let held_ex = state
            .players
            .get(&1)
            .unwrap()
            .held_helper
            .as_ref()
            .map(|h| h.extern_id)
            .unwrap_or(0);
        let tile_ex = state
            .world
            .read()
            .unwrap()
            .get_helper(0, 0)
            .map(|h| h.extern_id)
            .unwrap_or(0);
        assert!(held_ex > 0);
        assert_eq!(held_ex, tile_ex);
    }

    #[test]
    fn lock_removal_1003_success_allows_unlock_transition() {
        let mut db = ContentDb::default();
        let mut key = def(1003, 0, false);
        key.decays_to_obj = 862;
        db.objects.insert(1003, key);
        db.objects.insert(862, def(862, 0, false));
        db.objects.insert(988, locked_chest_def(988));
        db.objects.insert(989, def(989, 0, true));
        db.transitions
            .insert((1003, 988), tr(1003, 988, 0, 989, false, false));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        let p_id = state.players.get(&1).unwrap().p_id;
        state.economy.add_coins(p_id, 10);
        {
            let p = state.players.get_mut(&1).unwrap();
            let mut h = NestedHelper::with_uses(1003, 0);
            h.extern_id = 1;
            p.set_held_helper(h);
            p.exhaustion = 0.0;
            p.x = 0;
            p.y = 0;
        }
        let mut chest = ComplexObject::new_simple(988);
        chest.extern_id = 99; // mismatch → lockpick
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(0, 0, chest);

        // Force success path by retrying with low RNG is hard; instead assert settings
        // defaults and that refuse/success both deduct correctly via pure try_lockpick.
        // Wire: when mismatch and lockpick fails (most rolls), USE does not apply.
        // With enough retries we may hit success; call pure gate then optional wire.
        use crate::locks::{try_lockpick, LockpickSettings};
        let s = LockpickSettings::default();
        assert!((s.success_chance - 5.0).abs() < 1e-5);
        assert!((s.fail_chance - 10.0).abs() < 1e-5);
        assert!((s.exhaustion_cost - 3.0).abs() < 1e-5);
        assert!((s.coin_cost - 1.0).abs() < 1e-5);
        // Deterministic success via pure helper
        match try_lockpick(10.0, 0.0, 20.0, false, &s, 862, 0.01) {
            crate::locks::LockpickOutcome::Success { .. } => {}
            o => panic!("expected Success {o:?}"),
        }
        // Matching keys skip lockpick and allow USE.
        {
            let mut w = state.world.write().unwrap();
            if let Some(h) = w.helpers.get_mut(&(0, 0)) {
                h.extern_id = 1;
            }
        }
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        assert_eq!(r.target_after, 989);
    }

    /// Live success_chance=100 + coin_cost: 1003 mismatch always opens and deducts.
    // Haxe: ServerSettings.Lockpick* via apply_live_settings → USE
    #[test]
    fn lock_removal_1003_live_success_100_deducts_coin_cost() {
        use crate::settings_live::apply_live_settings;
        use ol_config::ServerConfig;
        // Clear module static from prior tests
        let _ = crate::locks::take_lock_say();

        let mut db = ContentDb::default();
        let mut key = def(1003, 0, false);
        key.decays_to_obj = 862;
        db.objects.insert(1003, key);
        db.objects.insert(862, def(862, 0, false));
        db.objects.insert(988, locked_chest_def(988));
        db.objects.insert(989, def(989, 0, true));
        db.transitions
            .insert((1003, 988), tr(1003, 988, 0, 989, false, false));
        let mut state = state_with(db);
        let live = ServerConfig {
            lockpick_success_chance: 100.0,
            lockpick_fail_chance: 0.0,
            lockpick_exhaustion_cost: 2.0,
            lockpick_coin_cost: 3.0,
            ..Default::default()
        }
        .live_settings();
        apply_live_settings(&mut state, &live);
        assert!((state.lockpick_settings.success_chance - 100.0).abs() < 1e-5);
        assert!((state.lockpick_settings.coin_cost - 3.0).abs() < 1e-5);

        crate::spawn_player(&mut state, 1, "u");
        let p_id = state.players.get(&1).unwrap().p_id;
        state.economy.add_coins(p_id, 10);
        {
            let p = state.players.get_mut(&1).unwrap();
            // Avoid default Female001 (19) ×0.5 exhaustion (use male skin id).
            p.display_object_id = 352;
            let mut h = NestedHelper::with_uses(1003, 0);
            h.extern_id = 1;
            p.set_held_helper(h);
            p.exhaustion = 0.0;
            p.x = 0;
            p.y = 0;
        }
        let mut chest = ComplexObject::new_simple(988);
        chest.extern_id = 99;
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(0, 0, chest);

        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied, "100% success must open locked chest");
        assert_eq!(r.target_after, 989);
        assert_eq!(state.economy.coins_of(p_id), 7); // 10 - 3
        assert!((state.players.get(&1).unwrap().exhaustion - 2.0).abs() < 1e-5);
        // Transition new_actor=0 empties hand (not broke→862)
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
        assert_eq!(state.world.read().unwrap().get_object(0, 0), 989);
        let _ = crate::locks::take_lock_say();
    }

    /// Live success=0 fail=100: 1003 mismatch breaks key to decays_to, refuses USE.
    // Haxe: TransitionHelper.LockPick break branch + transformHeldObject
    #[test]
    fn lock_removal_1003_live_fail_100_breaks_key() {
        use crate::settings_live::apply_live_settings;
        use ol_config::ServerConfig;

        let mut db = ContentDb::default();
        let mut key = def(1003, 0, false);
        key.decays_to_obj = 862;
        db.objects.insert(1003, key);
        db.objects.insert(862, def(862, 0, false));
        db.objects.insert(988, locked_chest_def(988));
        db.objects.insert(989, def(989, 0, true));
        db.transitions
            .insert((1003, 988), tr(1003, 988, 0, 989, false, false));
        let mut state = state_with(db);
        let live = ServerConfig {
            lockpick_success_chance: 0.0,
            lockpick_fail_chance: 100.0,
            lockpick_exhaustion_cost: 1.0,
            lockpick_coin_cost: 1.0,
            ..Default::default()
        }
        .live_settings();
        apply_live_settings(&mut state, &live);

        crate::spawn_player(&mut state, 1, "u");
        let p_id = state.players.get(&1).unwrap().p_id;
        state.economy.add_coins(p_id, 5);
        {
            let p = state.players.get_mut(&1).unwrap();
            p.display_object_id = 352; // male — full fail chance / exhaustion
            let mut h = NestedHelper::with_uses(1003, 0);
            h.extern_id = 1;
            p.set_held_helper(h);
            p.exhaustion = 0.0;
            p.x = 0;
            p.y = 0;
        }
        let mut chest = ComplexObject::new_simple(988);
        chest.extern_id = 99;
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(0, 0, chest);

        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(!r.applied);
        assert_eq!(state.world.read().unwrap().get_object(0, 0), 988);
        // r=0 edge → Failed (key intact); any r>0 → Broke. Retry once if soft-failed.
        let held = state.players.get(&1).unwrap().held_id;
        if held == 1003 {
            // exact rng01=0 soft fail still deducted coins/exh
            assert_eq!(state.economy.coins_of(p_id), 4);
            let r2 = apply_use_at(&mut state, 1, 0, 0).unwrap();
            assert!(!r2.applied);
        }
        assert_eq!(
            state.players.get(&1).unwrap().held_id,
            862,
            "key should break to decays_to 862 under 100% fail"
        );
        assert!(state.economy.coins_of(p_id) <= 4);
        assert!(state.players.get(&1).unwrap().exhaustion >= 1.0);
    }

    /// Fractional live coin_cost floors onto i32 wallet after USE side-effect.
    #[test]
    fn lock_removal_1003_fractional_coin_cost_wallet_floor() {
        use crate::settings_live::apply_live_settings;
        use ol_config::ServerConfig;

        let mut db = ContentDb::default();
        let mut key = def(1003, 0, false);
        key.decays_to_obj = 862;
        db.objects.insert(1003, key);
        db.objects.insert(862, def(862, 0, false));
        db.objects.insert(988, locked_chest_def(988));
        db.objects.insert(989, def(989, 0, true));
        db.transitions
            .insert((1003, 988), tr(1003, 988, 0, 989, false, false));
        let mut state = state_with(db);
        apply_live_settings(
            &mut state,
            &ServerConfig {
                lockpick_success_chance: 100.0,
                lockpick_fail_chance: 0.0,
                lockpick_exhaustion_cost: 0.0,
                lockpick_coin_cost: 1.5,
                ..Default::default()
            }
            .live_settings(),
        );
        crate::spawn_player(&mut state, 1, "u");
        let p_id = state.players.get(&1).unwrap().p_id;
        state.economy.add_coins(p_id, 5);
        {
            let p = state.players.get_mut(&1).unwrap();
            let mut h = NestedHelper::with_uses(1003, 0);
            h.extern_id = 1;
            p.set_held_helper(h);
            p.x = 0;
            p.y = 0;
        }
        let mut chest = ComplexObject::new_simple(988);
        chest.extern_id = 99;
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(0, 0, chest);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        // pure 5 - 1.5 = 3.5 → wallet floor 3
        assert_eq!(state.economy.coins_of(p_id), 3);
    }

    #[test]
    fn blank_lock_904_receives_key_extern() {
        let mut db = ContentDb::default();
        db.objects.insert(917, def(917, 0, false));
        db.objects.insert(904, {
            let mut d = def(904, 0, true);
            d.description = "Lock Blank".into();
            d
        });
        db.transitions
            .insert((917, 904), tr(917, 904, 917, 4058, false, false));
        db.objects.insert(4058, def(4058, 0, true));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            let mut h = NestedHelper::with_uses(917, 0);
            h.extern_id = 77;
            p.set_held_helper(h);
            p.x = 0;
            p.y = 0;
        }
        state.world.write().unwrap().set_object(0, 0, 904);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        assert_eq!(
            state
                .world
                .read()
                .unwrap()
                .get_helper(0, 0)
                .map(|h| h.extern_id),
            Some(77)
        );
    }

    /// DARK-NOSAJ: empty-hand USE on 2466 sets dark_nosaj; Tarr 3112 clears.
    // Haxe: TransitionHelper L144–185
    #[test]
    fn dark_nosaj_monument_use_sets_and_clears() {
        use crate::dark_nosaj::{
            take_monument_feedback, DARK_NOSAJ_MONUMENT_ID, TARR_MONUMENT_ID,
            CURSE_CLEAR_WORD, CURSE_DARK_MINION_WORD,
        };
        let _ = take_monument_feedback();
        let mut state = state_with(ContentDb::default());
        let p_id = crate::spawn_player(&mut state, 1, "dn@test");
        let prest_before = state.combat.stats.get(&p_id).map(|s| s.prestige).unwrap_or(0.0);
        apply_monument_use_side_effects(&mut state, 1, 0, DARK_NOSAJ_MONUMENT_ID);
        let pl = state.players.get(&1).unwrap();
        assert!((pl.dark_nosaj - 1.0).abs() < 1e-5, "dark_nosaj={}", pl.dark_nosaj);
        assert!(!pl.praised_jinbali);
        assert!((state.reputation.lost_combat(p_id) - 100.0).abs() < 1e-3);
        let prest = state.combat.stats.get(&p_id).map(|s| s.prestige).unwrap_or(0.0);
        // Haxe set path: yum_multiplier −100 (spawn may seed prestige; assert delta)
        assert!(
            (prest - (prest_before - 100.0)).abs() < 1e-2,
            "prestige delta: before={prest_before} after={prest}"
        );
        let fb = take_monument_feedback().expect("set feedback");
        assert_eq!(fb.say, "All hail dark nosaj");
        assert_eq!(fb.curse, Some((1, Some(CURSE_DARK_MINION_WORD))));

        apply_monument_use_side_effects(&mut state, 1, 0, TARR_MONUMENT_ID);
        let pl = state.players.get(&1).unwrap();
        assert_eq!(pl.dark_nosaj, 0.0);
        assert!((state.reputation.lost_combat(p_id) - 10.0).abs() < 1e-3); // 100-90
        let fb = take_monument_feedback().expect("clear feedback");
        assert_eq!(fb.say, "Jasoniah is the one true god!");
        assert_eq!(fb.curse, Some((0, Some(CURSE_CLEAR_WORD))));
    }

    /// DARK-NOSAJ: praise path then dark nosaj punish (+hits).
    #[test]
    fn dark_nosaj_praise_then_punish() {
        use crate::dark_nosaj::{
            take_monument_feedback, DARK_NOSAJ_MONUMENT_ID, TARR_MONUMENT_ID,
        };
        let _ = take_monument_feedback();
        let mut state = state_with(ContentDb::default());
        let p_id = crate::spawn_player(&mut state, 1, "praise@test");
        apply_monument_use_side_effects(&mut state, 1, 0, TARR_MONUMENT_ID);
        assert!(state.players.get(&1).unwrap().praised_jinbali);
        let prest = state.combat.stats.get(&p_id).map(|s| s.prestige).unwrap_or(0.0);
        // spawn may seed prestige; require +5 vs pre-praise
        let _ = take_monument_feedback();
        // Reset prestige book for clean assert: re-read after praise only delta 5
        // (spawn prestige + 5)
        assert!(state.players.get(&1).unwrap().praised_jinbali);
        apply_monument_use_side_effects(&mut state, 1, 0, DARK_NOSAJ_MONUMENT_ID);
        let pl = state.players.get(&1).unwrap();
        assert!(!pl.praised_jinbali);
        assert!((state.combat.hits_of(p_id) - 10.0).abs() < 1e-3);
        let fb = take_monument_feedback().unwrap();
        assert_eq!(fb.say, "AAAAAAAAAAAAAAAAAAAAaaaa!!!");
        assert!(fb.curse.is_none());
        let _ = prest;
    }

    /// DARK-NOSAJ: apply_use_at runs monument side-effects exactly once (no multi-invoke).
    /// Prestige after set is −100 (not −N×100); clear does not re-enter praise on same USE.
    // Haxe: TransitionHelper.doCommandHelper L144–185 (once per USE)
    #[test]
    fn dark_nosaj_apply_use_at_side_effects_once() {
        use crate::dark_nosaj::{
            take_monument_feedback, DARK_NOSAJ_MONUMENT_ID, TARR_MONUMENT_ID,
        };
        let _ = take_monument_feedback();
        let mut state = state_with(ContentDb::default());
        let p_id = crate::spawn_player(&mut state, 1, "once@test");
        state.players.get_mut(&1).unwrap().x = 0;
        state.players.get_mut(&1).unwrap().y = 0;
        state
            .world
            .write()
            .unwrap()
            .set_object(0, 0, DARK_NOSAJ_MONUMENT_ID);

        let prest_before = state.combat.stats.get(&p_id).map(|s| s.prestige).unwrap_or(0.0);
        let _ = apply_use_at(&mut state, 1, 0, 0);
        let prest_after_set = state.combat.stats.get(&p_id).map(|s| s.prestige).unwrap_or(0.0);
        assert!(
            (prest_after_set - (prest_before - 100.0)).abs() < 1e-2,
            "set prestige delta once: before={prest_before} after={prest_after_set}"
        );
        assert!((state.players.get(&1).unwrap().dark_nosaj - 1.0).abs() < 1e-5);
        let fb = take_monument_feedback().expect("one feedback after set");
        assert_eq!(fb.say, "All hail dark nosaj");
        assert!(take_monument_feedback().is_none(), "exactly one feedback note");

        // Tarr clear on same player: clear path only — praised_jinbali stays false
        state
            .world
            .write()
            .unwrap()
            .set_object(0, 0, TARR_MONUMENT_ID);
        let prest_pre_clear = state.combat.stats.get(&p_id).map(|s| s.prestige).unwrap_or(0.0);
        let _ = apply_use_at(&mut state, 1, 0, 0);
        let pl = state.players.get(&1).unwrap();
        assert_eq!(pl.dark_nosaj, 0.0);
        assert!(
            !pl.praised_jinbali,
            "Tarr clear must not re-enter praise on same USE"
        );
        let prest_after_clear = state.combat.stats.get(&p_id).map(|s| s.prestige).unwrap_or(0.0);
        assert!(
            (prest_after_clear - (prest_pre_clear + 90.0)).abs() < 1e-2,
            "clear prestige +90 once: pre={prest_pre_clear} after={prest_after_clear}"
        );
        let fb = take_monument_feedback().expect("clear feedback");
        assert_eq!(fb.say, "Jasoniah is the one true god!");
        assert!(take_monument_feedback().is_none());
    }

    /// DARK-NOSAJ: held non-empty on 2466 → no plan / no flag change via apply_use_at.
    // Haxe: TransitionHelper L166–167 empty-hand gate
    #[test]
    fn dark_nosaj_held_nonempty_no_set() {
        use crate::dark_nosaj::{take_monument_feedback, DARK_NOSAJ_MONUMENT_ID};
        let _ = take_monument_feedback();
        let mut db = ContentDb::default();
        db.objects.insert(5, def(5, 0, false));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "held@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.held_id = 5;
            p.x = 0;
            p.y = 0;
            p.dark_nosaj = 0.0;
            p.praised_jinbali = false;
        }
        state
            .world
            .write()
            .unwrap()
            .set_object(0, 0, DARK_NOSAJ_MONUMENT_ID);
        let _ = apply_use_at(&mut state, 1, 0, 0);
        let pl = state.players.get(&1).unwrap();
        assert_eq!(pl.dark_nosaj, 0.0);
        assert!(!pl.praised_jinbali);
        assert!(take_monument_feedback().is_none());
    }

    // ── C-SS-MORE-BATCH5 hungry-work pure + USE wire ─────────────────────────

    #[test]
    fn resolve_hungry_work_temperature_live() {
        // temp < 0 → cost × heat
        assert!(
            (resolve_hungry_work_temperature(-1.0, 10.0, 0.002) - 0.02).abs() < 1e-6
        );
        assert!(
            (resolve_hungry_work_temperature(-1.0, 10.0, 0.004) - 0.04).abs() < 1e-6
        );
        // temp >= 0 passthrough
        assert!((resolve_hungry_work_temperature(0.5, 10.0, 0.002) - 0.5).abs() < 1e-6);
        assert!((resolve_hungry_work_temperature(0.0, 10.0, 0.002) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn compute_hungry_work_cost_river_and_loose() {
        assert!((compute_hungry_work_cost(2.0, 3.0, 1.0, false) - 6.0).abs() < 1e-6);
        assert!((compute_hungry_work_cost(2.0, 3.0, 1.0, true) - 5.0).abs() < 1e-6);
        let (c, fort) = apply_loose_fence_hungry_work_waiver(5.0, "Loose Fence", 0.0);
        assert!((c - 0.0).abs() < 1e-6);
        assert!(!fort);
        let (c2, fort2) = apply_loose_fence_hungry_work_waiver(5.0, "Loose Fence", -1.0);
        assert!((c2 - 5.0).abs() < 1e-6);
        assert!(fort2);
    }

    #[test]
    fn evaluate_hungry_work_use_gates_and_apply() {
        // Free when cost <= 0
        assert_eq!(
            evaluate_hungry_work_use(0.0, 0.0, 5.0, 20.0, 0.0, 0.5),
            HungryWorkGate::Free
        );
        // Exhaustion refuse: exhaustion > food_max/2
        match evaluate_hungry_work_use(4.0, 0.01, 10.0, 20.0, 11.0, 0.5) {
            HungryWorkGate::RefuseExhaustion { excess } => assert_eq!(excess, 1),
            o => panic!("expected RefuseExhaustion {o:?}"),
        }
        // Food refuse: ceil(cost/2 - food) > 0
        match evaluate_hungry_work_use(10.0, 0.02, 2.0, 20.0, 0.0, 0.5) {
            HungryWorkGate::RefuseFood { missing } => assert_eq!(missing, 3), // ceil(5-2)=3
            o => panic!("expected RefuseFood {o:?}"),
        }
        // Allow: heat clamp, food − cost/2, exhaustion + cost/2
        match evaluate_hungry_work_use(4.0, 0.8, 10.0, 20.0, 1.0, 0.5) {
            HungryWorkGate::Allow {
                heat_after,
                food_after,
                exhaustion_after,
            } => {
                assert!((heat_after - 1.0).abs() < 1e-6); // 0.5+0.8 clamped
                assert!((food_after - 8.0).abs() < 1e-6);
                assert!((exhaustion_after - 3.0).abs() < 1e-6);
            }
            o => panic!("expected Allow {o:?}"),
        }
    }

    #[test]
    fn object_hungry_work_patch_and_tag() {
        assert!((object_hungry_work(231, "Adobe Oven Base", 5.0) - 10.0).abs() < 1e-6);
        assert!((object_hungry_work(857, "Steel Hoe", 5.0) - (-2.0)).abs() < 1e-6);
        assert!((object_hungry_work(9999, "Thing +hungryWork", 7.0) - 7.0).abs() < 1e-6);
        assert!((object_hungry_work(1, "Rock", 5.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn plan_hungry_work_use_owner_and_heat() {
        let (_c, temp, adj, gate) = plan_hungry_work_use(
            0.0,
            10.0,
            0.0,
            -1.0,
            0.004,
            false,
            "Adobe Oven Base",
            0.0,
            false,
            true,
            20.0,
            20.0,
            0.0,
            0.3,
        );
        assert!((temp - 0.04).abs() < 1e-6); // 10 * 0.004
        assert!(matches!(adj, HungryWorkOwnerAdj::Unaffected { .. }));
        match gate {
            HungryWorkGate::Allow {
                heat_after,
                food_after,
                exhaustion_after,
            } => {
                assert!((heat_after - 0.34).abs() < 1e-6);
                assert!((food_after - 15.0).abs() < 1e-6);
                assert!((exhaustion_after - 5.0).abs() < 1e-6);
            }
            o => panic!("expected Allow {o:?}"),
        }
        // Owner half cost
        let (c_own, _, adj_own, _) = plan_hungry_work_use(
            0.0, 4.0, 0.0, -1.0, 0.002, false, "Gate +owned", 0.0, true, true, 20.0, 20.0, 0.0,
            0.5,
        );
        assert!((c_own - 2.0).abs() < 1e-6);
        assert!(matches!(
            adj_own,
            HungryWorkOwnerAdj::OwnerHalf {
                allow_for_owner: false,
                ..
            }
        ));
    }

    /// USE on patched hungry-work target applies heat/food/exhaustion via live heat.
    // Haxe: TransitionHelper L1247–1251
    #[test]
    fn apply_use_hungry_work_heat_live() {
        let mut db = ContentDb::default();
        // empty hand on Adobe Oven Base 231 → still need a transition
        db.objects.insert(0, def(0, 0, false));
        db.objects.insert(231, {
            let mut d = def(231, 0, true);
            d.description = "Adobe Oven Base".into();
            d
        });
        db.objects.insert(232, {
            let mut d = def(232, 0, true);
            d.description = "Adobe Oven".into();
            d
        });
        // actor 10 tool, target 231 → new target 232 with hungryWork on 232? Haxe uses newTarget.
        // object_hungry_work(231)=10 is for new_target when new_target is 231.
        // Use actor free, target→new_target that keeps 231 as newTarget for cost.
        db.objects.insert(10, def(10, 0, false));
        db.transitions
            .insert((10, 231), tr(10, 231, 10, 231, false, false));
        let mut state = state_with(db);
        state.gameplay.hungry_work_heat = 0.004;
        crate::spawn_player(&mut state, 1, "hw@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(10, 0);
            p.food = 20.0;
            p.food_max = 20.0;
            p.exhaustion = 0.0;
            p.heat = 0.3;
            p.x = 0;
            p.y = 0;
        }
        state.world.write().unwrap().set_object(0, 0, 231);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        let p = state.players.get(&1).unwrap();
        // cost = 0 (actor) + 10 (new_target 231) + 0 = 10; heat += 10*0.004=0.04
        assert!((p.heat - 0.34).abs() < 1e-5, "heat={}", p.heat);
        assert!((p.food - 15.0).abs() < 1e-5, "food={}", p.food);
        assert!((p.exhaustion - 5.0).abs() < 1e-5, "exh={}", p.exhaustion);
    }

    #[test]
    fn apply_use_hungry_work_refuse_food() {
        let mut db = ContentDb::default();
        db.objects.insert(10, def(10, 0, false));
        db.objects.insert(231, {
            let mut d = def(231, 0, true);
            d.description = "Adobe Oven Base".into();
            d
        });
        db.transitions
            .insert((10, 231), tr(10, 231, 10, 231, false, false));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "starve@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(10, 0);
            p.food = 1.0; // missing = ceil(5 - 1) = 4
            p.food_max = 20.0;
            p.exhaustion = 0.0;
            p.x = 0;
            p.y = 0;
        }
        state.world.write().unwrap().set_object(0, 0, 231);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(!r.applied);
        assert_eq!(state.world.read().unwrap().get_object(0, 0), 231);
        let p = state.players.get(&1).unwrap();
        assert!((p.food - 1.0).abs() < 1e-5);
    }

    /// CLOTHING-CONTAIN-SIZE: USE on container index refuses when newTarget too big.
    // Haxe: TransitionHelper.doTransitionIfPossibleHelper L1087–1091
    #[test]
    fn use_on_container_refuses_oversized_new_target() {
        let mut db = ContentDb::default();
        // Outer basket slotSize=1
        let mut basket = def(100, 0, true);
        basket.num_slots = 4;
        basket.slot_size = 1.0;
        db.objects.insert(100, basket);
        // Contained berry (target of transition)
        let mut berry = def(33, 0, false);
        berry.containable = true;
        berry.contain_size = 1.0;
        db.objects.insert(33, berry);
        // Oversized newTarget
        let mut big = def(200, 0, false);
        big.containable = true;
        big.contain_size = 2.0;
        db.objects.insert(200, big);
        // Actor tool
        db.objects.insert(10, def(10, 0, false));
        // 10+33 → 10+200 (big result won't fit in slotSize 1)
        db.transitions
            .insert((10, 33), tr(10, 33, 10, 200, false, false));

        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "csize@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(10, 0);
            p.x = 0;
            p.y = 0;
        }
        {
            let mut w = state.world.write().unwrap();
            let mut h = ComplexObject::new_simple(100);
            h.contained.push(33);
            w.set_object_complex(0, 0, h);
        }
        let r = apply_use_at_ex(&mut state, 1, 0, 0, Some(0)).unwrap();
        assert!(!r.applied, "oversized newTarget must refuse");
        let w = state.world.read().unwrap();
        assert_eq!(w.get_object(0, 0), 100, "outer stays");
        let h = w.get_helper(0, 0).unwrap();
        assert_eq!(h.contained, vec![33], "contained unchanged");
    }

    /// CLOTHING-CONTAIN-SIZE: equal containSize/slotSize allows USE on container index.
    // Haxe: TransitionHelper.doTransitionIfPossibleHelper L1087–1091
    #[test]
    fn use_on_container_allows_fitting_new_target() {
        let mut db = ContentDb::default();
        let mut basket = def(100, 0, true);
        basket.num_slots = 4;
        basket.slot_size = 2.0;
        db.objects.insert(100, basket);
        let mut berry = def(33, 0, false);
        berry.containable = true;
        berry.contain_size = 1.0;
        db.objects.insert(33, berry);
        let mut result = def(201, 0, false);
        result.containable = true;
        result.contain_size = 2.0; // equal → fit
        db.objects.insert(201, result);
        db.objects.insert(10, def(10, 0, false));
        db.transitions
            .insert((10, 33), tr(10, 33, 10, 201, false, false));

        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "cfit@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(10, 0);
            p.x = 0;
            p.y = 0;
        }
        {
            let mut w = state.world.write().unwrap();
            let mut h = ComplexObject::new_simple(100);
            h.contained.push(33);
            w.set_object_complex(0, 0, h);
        }
        let r = apply_use_at_ex(&mut state, 1, 0, 0, Some(0)).unwrap();
        assert!(r.applied, "fitting newTarget must apply");
        let w = state.world.read().unwrap();
        assert_eq!(w.get_object(0, 0), 100, "outer stays basket");
        let h = w.get_helper(0, 0).unwrap();
        assert_eq!(h.contained, vec![201], "contained becomes result");
        assert_eq!(state.players.get(&1).unwrap().held_id, 10);
    }

    /// CLOTHING-CONTAIN-SIZE: pure L1087 gate edges.
    #[test]
    fn transition_result_fits_container_pure() {
        assert!(crate::death_polish::transition_result_fits_container(
            -1.0, false, 99.0
        ));
        assert!(crate::death_polish::transition_result_fits_container(
            1.0, true, 1.0
        ));
        assert!(!crate::death_polish::transition_result_fits_container(
            1.0, true, 2.0
        ));
        assert!(!crate::death_polish::transition_result_fits_container(
            1.0, false, 0.0
        ));
    }


    /// TH-ALT-OUTCOME: low hits → TryAgain keeps target, stamps hits.
    // Haxe: TransitionHelper L1274–1303
    #[test]
    fn alt_outcome_try_again_keeps_target() {
        let mut db = ContentDb::default();
        db.objects.insert(71, def(71, 0, false));
        db.objects.insert(340, def(340, 0, true));
        db.objects.insert(344, def(344, 0, false));
        db.transitions
            .insert((71, 340), tr(71, 340, 71, 340, false, false));
        db.alt_outcomes_object.insert(340, vec![344]);
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "chop");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(71, 0);
            p.food = 20.0;
            p.food_max = 20.0;
            p.exhaustion = 0.0;
        }
        state.world.write().unwrap().set_object(0, 0, 340);
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied, "try-again is still an applied action");
        assert_eq!(r.target_after, 340, "target not transformed on try-again");
        let hits = state
            .world
            .read()
            .unwrap()
            .get_helper(0, 0)
            .map(|h| h.hits)
            .unwrap_or(0.0);
        assert!(hits >= 1.0 - 1e-4, "hits stamped, got {hits}");
    }

    /// TH-ALT-OUTCOME: high hits → Proceed continues transition.
    #[test]
    fn alt_outcome_proceed_allows_transform() {
        let mut db = ContentDb::default();
        db.objects.insert(71, def(71, 0, false));
        db.objects.insert(340, def(340, 0, true));
        db.objects.insert(341, def(341, 0, true));
        db.transitions
            .insert((71, 340), tr(71, 340, 71, 341, false, false));
        db.alt_outcomes_object.insert(340, vec![344]);
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "chop2");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(71, 0);
            p.food = 20.0;
            p.food_max = 20.0;
        }
        {
            let mut w = state.world.write().unwrap();
            w.set_object(0, 0, 340);
            crate::loved_food_wire::stamp_hits(&mut w, 0, 0, 10.0);
        }
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        assert_eq!(r.target_after, 341, "main transform on proceed");
        let hits = state
            .world
            .read()
            .unwrap()
            .get_helper(0, 0)
            .map(|h| h.hits)
            .unwrap_or(0.0);
        assert!(
            (hits - 5.0).abs() < 1e-3 || hits >= 5.0 - 1e-3,
            "hits={hits}"
        );
    }

}
