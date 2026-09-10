//! # Food eating (Haxe GPI `tryEat` / `doEating` / yum fill)
//!
//! **Canonical home for the player eat path.** New eat / yum-fill / multi-use
//! actor-on-eat logic belongs here — not in temperature handling or world-time
//! decay.
//!
//! ## Haxe map
//!
//! | Haxe | Rust |
//! |------|------|
//! | `GlobalPlayerInstance` eat (~3219) | [`try_eat_held`] |
//! | `DoChangeNumberOfUsesOnActorManual` on eat | `multi_use::change_number_of_uses_on_actor` |
//! | Yum / meh / `doIncreaseFoodValue` pure | [`crate::yum`], [`crate::food_fill`] |
//! | Feed other | [`try_do_eating`] (UBABY + SAY FEED/NURSE; helpers in [`crate::feed_other_yum`]) |
//! | SearchBestFood (display / AI) | [`crate::search_best_food`] — **search**, not eat apply |
//!
//! ## What belongs here
//!
//! - Eating the held object (fill food store, advance uses, last-use transform)
//! - Orchestration that applies yum fill after a successful eat
//!
//! ## What does **not** belong here
//!
//! - Body / tile temperature → [`crate::temperature_handler`]
//! - World map auto-decay / animals → [`crate::world_time`]
//! - Food **statistics** disk/HTML → [`crate::world_food_stats`]
//! - AI “seek food” goals → AI crates / `search_best_food`

use std::cell::RefCell;

use tracing::info;

use crate::feed_other_yum::{
    feed_other_eater_post_emote, feed_other_feeder_happy_on_craving,
};
use crate::health_prestige::{
    is_eve_or_adam_name, prestige_fan_deltas, PRESTIGE_LEADER_CHAIN_DEPTH,
};
use crate::multi_use;
use crate::use_transition;
use crate::SimState;

thread_local! {
    static PENDING_EAT_EMOTES: RefCell<Vec<(u64, i32)>> = RefCell::new(Vec::new());
    static PENDING_EAT_SAYS: RefCell<Vec<(u64, String)>> = RefCell::new(Vec::new());
}

/// Queue PE after doEating (`conn_id`, emot index). Consumed by eat/FEED fan-out.
// Haxe: GlobalPlayerInstance.doEating L3239–3245 doEmote
// FEED-OTHER-EMOTE
pub fn note_eat_emote(conn_id: u64, index: i32) {
    PENDING_EAT_EMOTES.with(|c| c.borrow_mut().push((conn_id, index)));
}

/// Take pending doEating PE list (eater then optional feeder).
// FEED-OTHER-EMOTE
pub fn take_eat_emotes() -> Vec<(u64, i32)> {
    PENDING_EAT_EMOTES.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

/// Queue private say after doEating refuse (`I need better food!`).
// Haxe: GlobalPlayerInstance.doEating L3103 say(..., toSelf)
// EAT-REFUSE-EMOTE
pub fn note_eat_say(conn_id: u64, text: impl Into<String>) {
    PENDING_EAT_SAYS.with(|c| c.borrow_mut().push((conn_id, text.into())));
}

/// Take pending doEating refuse says.
// EAT-REFUSE-EMOTE
pub fn take_eat_says() -> Vec<(u64, String)> {
    PENDING_EAT_SAYS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

fn emote_index_named(name: &str) -> Option<i32> {
    crate::emotes::emote_by_name(name).map(|e| e.index)
}

/// Haxe doEating post-eat PE: eater miam/happy/ill/sad; feeder happy on craving.
// Haxe: GlobalPlayerInstance.doEating L3239–3245
// FEED-OTHER-EMOTE / HORSE-EAT-EMOTE
pub(crate) fn queue_do_eating_post_emotes(
    from_conn: u64,
    to_conn: u64,
    computed: crate::yum::EatCompute,
) {
    let _ = take_eat_emotes();
    let kind = feed_other_eater_post_emote(
        computed.is_craving,
        computed.is_yum,
        computed.is_super_meh,
    );
    if let Some(idx) = emote_index_named(kind.emote_name()) {
        note_eat_emote(to_conn, idx);
    }
    if feed_other_feeder_happy_on_craving(computed.is_craving) {
        if let Some(idx) = emote_index_named("HAPPY") {
            note_eat_emote(from_conn, idx);
        }
    }
}

/// SuperMeh / meh-feed / too-full refuse PE (clears prior pending eat PE).
// Haxe: GlobalPlayerInstance.doEating L3070–3111
// EAT-REFUSE-EMOTE
pub(crate) fn begin_eat_refuse() {
    let _ = take_eat_emotes();
    let _ = take_eat_says();
}

pub(crate) fn queue_named_eat_emote(conn_id: u64, name: &str) {
    if let Some(idx) = emote_index_named(name) {
        note_eat_emote(conn_id, idx);
    }
}

/// Eat held food. Multi-use follows Haxe `DoChangeNumberOfUsesOnActorManual`
/// (`idHasChanged=false`, `reverseUse=false`, last-use target `-1`).
///
/// Returns true if edible food was consumed (yum fill applied).
// Haxe: GlobalPlayerInstance eat ~3219–3224
pub fn try_eat_held(state: &mut SimState, conn_id: u64) -> bool {
    try_do_eating(state, conn_id, conn_id)
}

/// Haxe `doEating(playerFrom, playerTo)` — self-eat, UBABY, or SAY FEED/NURSE.
///
/// Fill uses [`compute_eat_full`] × world FoodFactor × starving (same as horse
/// mount-eat). Yum / drugs / prestige apply to `to_conn`; held uses consume on
/// `from_conn`.
// Haxe: GlobalPlayerInstance.doEating L3040–3246
// FEED-OTHER-EAT
pub fn try_do_eating(state: &mut SimState, from_conn: u64, to_conn: u64) -> bool {
    let (held, uses, age, ill, feeder_pid) = match state.players.get(&from_conn) {
        Some(p) if !p.deleted => (
            p.held_id,
            p.held_uses,
            p.age,
            crate::nested_body::is_yellow_fever(p.fever.as_ref()),
            p.p_id,
        ),
        _ => return false,
    };
    if let Err(why) = crate::feed_other_yum::feeder_may_eat_or_feed(
        age,
        ill,
        state.gameplay.min_age_to_eat,
        state.gameplay.allow_eating_or_feeding_if_ill,
    ) {
        if why == "too ill" {
            // Haxe L3050–3055: say + eater yellowFever PE.
            // EAT-ILL-EMOTE
            begin_eat_refuse();
            note_eat_say(from_conn, "I am too ill!");
            queue_named_eat_emote(to_conn, "YELLOWFEVER");
        }
        return false;
    }
    if held == 0 {
        return false;
    }
    // Haxe: heldObjData.dummyParent ?? held
    let food_id = state.content.resolve_base_id(held);
    let Some(def) = state
        .content
        .get(food_id)
        .or_else(|| state.content.get(held))
    else {
        return false;
    };
    let food_value = def.food_value;
    if food_value < 1 {
        return false;
    }
    let num_uses = def.num_uses.max(0);
    let food_base = food_value as f32;

    let (eater_food, eater_max, count_eaten, eater_pid, eater_fever, person_oid) =
        match state.players.get(&to_conn) {
            Some(p) if !p.deleted => (
                p.food,
                p.food_max,
                p.yum.get_count_eaten(food_id),
                p.p_id,
                crate::nested_body::is_yellow_fever(p.fever.as_ref()),
                crate::person_object_id(p),
            ),
            _ => return false,
        };

    let eat_knobs = state.gameplay.eat_live_knobs();
    let yum_b = eat_knobs.yum_bonus;
    let is_self = from_conn == to_conn;
    // Haxe canFeedToMeObj: Psilocybe (837) only if eater has yellow fever.
    if !is_self
        && food_id == crate::yum::PSILOCYBE_MUSHROOM_ID
        && !eater_fever
    {
        return false;
    }
    // Haxe L3070: too full → Emote.refuseFood on eater.
    let room = eater_max - eater_food;
    let need = (food_value as f32 / 4.0).ceil();
    if !room.is_finite() || room < need {
        begin_eat_refuse();
        queue_named_eat_emote(to_conn, "REFUSEFOOD");
        return false;
    }
    let computed = crate::compute_eat_full(food_value, count_eaten, eat_knobs);
    // Haxe L3096–3106: superMeh + food_store > 5.
    if crate::refuse_self_eat_super_meh(computed.is_super_meh, eater_food) {
        begin_eat_refuse();
        if is_self {
            queue_named_eat_emote(to_conn, "ILL");
            note_eat_say(to_conn, "I need better food!");
        } else {
            queue_named_eat_emote(from_conn, "SAD");
        }
        return false;
    }
    // Haxe L3108–3111: feed-other meh only if starving (food_store ≤ 2).
    if !is_self && !computed.is_yum && eater_food > crate::yum::MEH_FEED_REFUSE_FOOD_STORE {
        begin_eat_refuse();
        queue_named_eat_emote(from_conn, "SAD");
        return false;
    }

    let bands = state.gameplay.food_factor_eaten_bands();
    let world_ff = state.world_food.get_food_factor_ex(food_id, &bands);
    let starve_ff = state.world_food.get_starving_food_factor_at(state.sim_time);
    let restore = state.gameplay.yum_restore_knobs();
    let red_per = eat_knobs.food_reduction_per_eating;
    let person_color = state.content.person_color(person_oid);
    let loved: Vec<i32> = crate::loved_food_ids_for_person_color(person_color).to_vec();
    let food_objects = crate::food_objects_list(state);
    let nearby_best = crate::nearby_best_for_craving(state, to_conn);
    let prestige_before = state
        .combat
        .stats
        .get(&eater_pid)
        .map(|s| s.prestige)
        .unwrap_or(0.0);
    let resist = state.gameplay.resistance_against_fever_for_eating_mushrooms;

    // Haxe: DoChangeNumberOfUsesOnActorManual(player, false, false, -1)
    let mut out =
        multi_use::change_number_of_uses_on_actor(held, held, uses, num_uses, false, false);
    if out.held_id != 0 && out.held_uses == 0 {
        if let Some(new_id) =
            use_transition::tool_last_use_new_actor(&state.content, out.held_id, -1)
        {
            out.held_id = new_id;
            out.held_uses = 0;
        } else {
            // Manual returned false → setHeldObject(null)
            out.held_id = 0;
            out.held_uses = 0;
        }
    }

    let mut pending_super_meh: Option<crate::SuperMehTrade> = None;
    let recorded_gain = {
        let Some(p) = state.players.get_mut(&to_conn) else {
            return false;
        };
        let fill_before = p.food.ceil() as i32;
        let base_gain = p.yum.eat_full(food_id, food_base, fill_before, eat_knobs);
        // Haxe L3186–3192: FoodFactor (in eat) × getFoodFactor × getStarvingFoodFactor
        let mut gain = crate::apply_world_food_factors(base_gain, world_ff, starve_ff);
        gain += crate::super_meh_extra_food_value(computed.is_super_meh);
        let trade = crate::super_meh_trade(computed.is_super_meh, prestige_before, food_id);
        if trade.age_delta != 0.0 {
            p.age += trade.age_delta;
            p.true_age += trade.age_delta;
        }
        if computed.is_super_meh {
            pending_super_meh = Some(trade);
        }
        // Haxe addFood: overflow → yum_bonus extra pips (not discarded).
        let (nf, nb) = crate::food_store_max::add_food(p.food, p.food_max, gain, p.yum.yum_bonus);
        p.food = nf;
        p.yum.yum_bonus = nb;
        if !computed.is_super_meh {
            let amount = if computed.has_eaten_delta != 0.0 {
                computed.has_eaten_delta
            } else {
                red_per
            };
            let dont_change = crate::dont_change_craving(is_self, computed.is_yum);
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
        if crate::horse_mount::is_drugs(food_id) {
            let ttc = p.fever.as_ref().map(|f| f.time_to_change);
            let (count, new_ttc) = crate::feed_other_yum::apply_drugs_fever_resistance(
                p.yellowfever_count,
                ttc,
                resist,
            );
            p.yellowfever_count = count;
            if let (Some(f), Some(t)) = (p.fever.as_mut(), new_ttc) {
                f.time_to_change = t;
            }
        }
        gain
    };
    state
        .world_food
        .add_food_statistic(food_id, food_base, recorded_gain);
    apply_super_meh_trade_on_eater(state, to_conn, eater_pid, pending_super_meh);
    if let Some(p) = state.players.get_mut(&from_conn) {
        p.set_held(out.held_id, out.held_uses);
    }
    apply_eat_health_prestige(state, from_conn, to_conn, computed.health_delta, computed.is_yum);
    if let Some(p) = state.players.get_mut(&to_conn) {
        // Haxe: playerTo.responsible_id = self ? -1 : playerFrom.p_id
        p.yum.responsible_id =
            crate::feed_other_yum::feed_other_responsible_id(feeder_pid, eater_pid);
    }
    queue_do_eating_post_emotes(from_conn, to_conn, computed);
    info!(
        from_conn,
        to_conn,
        held,
        food_id,
        gain = recorded_gain,
        new_held = out.held_id,
        new_uses = out.held_uses,
        "sim: ate/fed food"
    );
    true
}

/// SuperMeh age already applied on the eater; prestige / hits / food_max / death.
// Haxe: GlobalPlayerInstance.doEating L3195–3206
fn apply_super_meh_trade_on_eater(
    state: &mut SimState,
    eater_conn: u64,
    eater_p_id: i32,
    trade: Option<crate::SuperMehTrade>,
) {
    let Some(trade) = trade else {
        return;
    };
    if trade.prestige_delta != 0.0 && eater_p_id != 0 {
        let s = state.combat.stats_mut(eater_p_id);
        s.prestige = (s.prestige + trade.prestige_delta).max(0.0);
    }
    if !trade.needs_food_max_recompute || eater_p_id == 0 {
        return;
    }
    let _ = state
        .combat
        .apply_hits(eater_p_id, trade.hits_delta, trade.wounded_by_food_id);
    let (age, food, exh, true_age) = state
        .players
        .get(&eater_conn)
        .map(|p| (p.age, p.food, p.exhaustion, p.true_age))
        .unwrap_or((20.0, 10.0, 0.0, 20.0));
    let health_f = state.player_health_food_store_max_factor(eater_p_id, true_age);
    let hits = state.combat.hits_of(eater_p_id);
    let knobs = state.gameplay.food_store_max_knobs();
    let new_max = crate::food_store_max_from_parts_ex(age, food, hits, exh, health_f, knobs);
    if let Some(p) = state.players.get_mut(&eater_conn) {
        p.food_max = new_max;
        if p.food > p.food_max && p.food_max > 0.0 {
            p.food = p.food_max;
        }
        if crate::super_meh_food_max_is_deadly(new_max) {
            p.deleted = true;
            crate::ai_takeover::clear_ai_on_death(&mut p.ai_controlled);
            p.death_reason = Some(format!("reason_killed_{}", trade.wounded_by_food_id));
        }
    }
}

/// Haxe `doEating` → `addHealthAndPrestige` for eater, plus feeder 0.2 yum share.
///
/// Dark Nosaj blocks the whole prestige path. Negative count (meh) still hits
/// self yum/prestige, then returns before coins + family/leader fan.
// Haxe: GlobalPlayerInstance.doEating L3144–3156 + addHealthAndPrestige L5997
// HEALTH-PRESTIGE-FAN
pub fn apply_eat_health_prestige(
    state: &mut SimState,
    from_conn: u64,
    to_conn: u64,
    health_delta: f32,
    is_yum: bool,
) {
    apply_add_health_and_prestige(state, to_conn, health_delta, true);
    if from_conn == to_conn {
        return;
    }
    let share = crate::feed_other_yum::feed_other_feeder_prestige_delta(health_delta, is_yum);
    if share != 0.0 {
        apply_add_health_and_prestige(state, from_conn, share, true);
    }
}

/// HIT / non-eat prestige debit by `p_id` (Haxe `addHealthAndPrestige(-prestigeCost, false)`).
// Haxe: GlobalPlayerInstance.kill L4530 isFood=false
// HIT-PRESTIGE-COST
pub(crate) fn apply_add_health_and_prestige_by_pid(
    state: &mut SimState,
    p_id: i32,
    count: f32,
) {
    let Some((&conn, _)) = state.players.iter().find(|(_, p)| p.p_id == p_id) else {
        return;
    };
    apply_add_health_and_prestige(state, conn, count, false);
}

/// Haxe `addHealthAndPrestige(count)` — self yum + coins + clothing family/leader fan.
// Haxe: GlobalPlayerInstance.addHealthAndPrestige L5997–6101
pub(crate) fn apply_add_health_and_prestige(
    state: &mut SimState,
    conn: u64,
    count: f32,
    is_food: bool,
) {
    if !count.is_finite() {
        return;
    }
    let (pid, giver_factor) = {
        let Some(p) = state.players.get(&conn) else {
            return;
        };
        if crate::dark_nosaj::blocks_health_and_prestige(p.dark_nosaj) {
            return;
        }
        (
            p.p_id,
            live_clothing_prestige_factor(&state.content, p),
        )
    };
    let giver_eve = family_eve_of(state, pid);
    let fan = collect_prestige_fan(state, pid, giver_eve);
    add_yum_prestige(state, pid, count);
    if is_food {
        if let Some(n) = state.social.lineages.get_mut(&pid) {
            n.prestige_from.add_eating(count);
        }
    }
    if count <= 0.0 {
        return;
    }
    credit_prestige_coins(state, pid, count);
    let deltas = prestige_fan_deltas(
        count,
        fan.mother,
        fan.father,
        &fan.grandparents,
        &fan.children,
        fan.sibling,
        &fan.leaders,
        giver_factor,
    );
    for d in deltas {
        add_yum_prestige(state, d.p_id, d.prestige);
        if d.coins > 0.0 {
            credit_prestige_coins(state, d.p_id, d.coins);
        }
        if let Some(n) = state.social.lineages.get_mut(&d.p_id) {
            match d.kind {
                crate::health_prestige::PrestigeFanKind::Parent => {
                    n.prestige_from.add_children(d.prestige);
                }
                crate::health_prestige::PrestigeFanKind::Grandparent => {
                    n.prestige_from.add_grandkids(d.prestige);
                }
                crate::health_prestige::PrestigeFanKind::Child => {
                    n.prestige_from.add_parents(d.prestige);
                }
                crate::health_prestige::PrestigeFanKind::Sibling => {
                    n.prestige_from.add_siblings(d.prestige);
                }
                crate::health_prestige::PrestigeFanKind::Leader => {
                    n.prestige_from.add_followers(d.prestige);
                }
            }
        }
    }
}

fn credit_prestige_coins(state: &mut SimState, p_id: i32, amount: f32) {
    let n = state.economy.credit_floored_coins(p_id, amount);
    if n > 0 {
        let c = state.economy.coins_of(p_id);
        state.scoreboard.set_coins(p_id, c);
    }
}

pub(crate) fn add_yum_prestige(state: &mut SimState, p_id: i32, delta: f32) {
    if !delta.is_finite() || delta == 0.0 {
        return;
    }
    state.combat.stats_mut(p_id).prestige += delta;
    if let Some(n) = state.social.lineages.get_mut(&p_id) {
        // Haxe yum_multiplier has no floor; class uses non-negative prestige.
        n.prestige = if (n.prestige + delta).is_finite() {
            n.prestige + delta
        } else {
            n.prestige
        };
        n.prestige_class = crate::prestige::PrestigeClass::from_prestige(n.prestige.max(0.0));
    }
}

fn live_clothing_prestige_factor(content: &ol_content::ContentDb, p: &crate::Player) -> f32 {
    crate::health_prestige::clothing_prestige_factor_from_content(
        &p.clothing_parent_ids(),
        is_eve_or_adam_name(&p.first_name),
        |id| content.get(id).map(|d| d.get_prestige_factor()),
    )
}

fn live_clothing_extra_prestige(content: &ol_content::ContentDb, p: &crate::Player) -> f32 {
    crate::health_prestige::clothing_extra_prestige_factor(
        &p.clothing_parent_ids(),
        |id| {
            content
                .get(id)
                .map(|d| d.extra_prestige_factor)
                .unwrap_or(0.0)
        },
    )
}

fn family_eve_of(state: &SimState, p_id: i32) -> i32 {
    state
        .social
        .lineages
        .get(&p_id)
        .map(|n| n.family_eve_id())
        .unwrap_or(0)
}

fn clothing_factor_for_pid(state: &SimState, p_id: i32) -> Option<f32> {
    state
        .players
        .values()
        .find(|p| p.p_id == p_id)
        .map(|p| live_clothing_prestige_factor(&state.content, p))
}

struct PrestigeFanInputs {
    mother: Option<(i32, f32)>,
    father: Option<(i32, f32)>,
    grandparents: Vec<(i32, f32)>,
    children: Vec<(i32, f32)>,
    sibling: Option<(i32, f32)>,
    leaders: Vec<(i32, f32, f32, bool, bool, bool)>,
}

/// Snapshot living relatives / follow chain for [`prestige_fan_deltas`].
// Haxe: addHealthAndPrestige mother/father/children/followPlayer
fn collect_prestige_fan(state: &SimState, eater: i32, giver_eve: i32) -> PrestigeFanInputs {
    let node = state.social.lineages.get(&eater);
    let mother_id = node.and_then(|n| n.mother_id).filter(|&id| id > 0);
    let father_id = node.and_then(|n| n.father_id).filter(|&id| id > 0);
    let mother = mother_id.and_then(|id| clothing_factor_for_pid(state, id).map(|f| (id, f)));
    let father = father_id.and_then(|id| clothing_factor_for_pid(state, id).map(|f| (id, f)));

    let mut grandparents = Vec::new();
    if mother.is_some() {
        if let Some(mid) = mother_id {
            if let Some(mn) = state.social.lineages.get(&mid) {
                if let Some(id) = mn.mother_id.filter(|&i| i > 0) {
                    if let Some(f) = clothing_factor_for_pid(state, id) {
                        grandparents.push((id, f));
                    }
                }
                if let Some(id) = mn.father_id.filter(|&i| i > 0) {
                    if let Some(f) = clothing_factor_for_pid(state, id) {
                        grandparents.push((id, f));
                    }
                }
            }
        }
    }
    if father.is_some() {
        if let Some(fid) = father_id {
            if let Some(fn_) = state.social.lineages.get(&fid) {
                if let Some(id) = fn_.mother_id.filter(|&i| i > 0) {
                    if let Some(f) = clothing_factor_for_pid(state, id) {
                        grandparents.push((id, f));
                    }
                }
                if let Some(id) = fn_.father_id.filter(|&i| i > 0) {
                    if let Some(f) = clothing_factor_for_pid(state, id) {
                        grandparents.push((id, f));
                    }
                }
            }
        }
    }

    let mut children: Vec<(i32, f32)> = state
        .players
        .values()
        .filter(|p| !p.deleted && p.p_id != eater)
        .filter_map(|p| {
            let n = state.social.lineages.get(&p.p_id)?;
            if n.mother_id == Some(eater) || n.father_id == Some(eater) {
                Some((p.p_id, live_clothing_prestige_factor(&state.content, p)))
            } else {
                None
            }
        })
        .collect();
    children.sort_unstable_by_key(|c| c.0);
    // Haxe picks `children[random]` as "sibling" when mother != null.
    // Deterministic: lowest p_id (Haxe WorldMap.calculateRandomInt residual).
    let sibling = if mother.is_some() && !children.is_empty() {
        Some(children[0])
    } else {
        None
    };

    let mut leaders = Vec::new();
    let mut walk = state.social.following.get(&eater).copied();
    for _ in 0..PRESTIGE_LEADER_CHAIN_DEPTH {
        let Some(lid) = walk.filter(|&id| id > 0) else {
            break;
        };
        let exiled = state.social.is_exiled_by(lid, eater);
        let Some(lp) = state.players.values().find(|p| p.p_id == lid) else {
            break;
        };
        let same_family = giver_eve > 0 && giver_eve == family_eve_of(state, lid);
        leaders.push((
            lid,
            live_clothing_prestige_factor(&state.content, lp),
            live_clothing_extra_prestige(&state.content, lp),
            same_family,
            lp.is_cursed,
            exiled,
        ));
        if exiled {
            break;
        }
        walk = state.social.following.get(&lid).copied();
        if walk == Some(lid) {
            break;
        }
    }

    PrestigeFanInputs {
        mother,
        father,
        grandparents,
        children,
        sibling,
        leaders,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::{ContentDb, ObjectDef};
    use ol_world::NestedHelper;
    use std::sync::Arc;

    fn food_def(id: i32, food_value: i32) -> ObjectDef {
        ObjectDef {
            id,
            food_value,
            ..Default::default()
        }
    }

    #[test]
    fn try_eat_held_blocks_yellow_fever_unless_allow_if_ill() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        crate::spawn_player(&mut state, 1, "ill@t");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 20.0;
            p.food = 2.0;
            p.food_max = 40.0;
            p.set_held(31, 0);
            p.fever = Some(NestedHelper::id_only(crate::nested_body::YELLOW_FEVER_ID));
        }
        let _ = take_eat_emotes();
        let _ = take_eat_says();
        assert!(!try_eat_held(&mut state, 1));
        assert_eq!(state.players.get(&1).unwrap().held_id, 31);
        assert_eq!(
            take_eat_emotes(),
            vec![(1, crate::emotes::emote_by_name("YELLOWFEVER").unwrap().index)]
        );
        assert_eq!(
            take_eat_says(),
            vec![(1, "I am too ill!".to_string())]
        );
        state.gameplay.allow_eating_or_feeding_if_ill = true;
        assert!(try_eat_held(&mut state, 1));
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
    }

    /// EAT-ILL-EMOTE: feed-other ill feeder → eater yellowFever PE + feeder say.
    // Haxe: GlobalPlayerInstance.doEating L3050–3055
    #[test]
    fn try_do_eating_ill_feeder_eater_yellow_fever_pe() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        setup_feed_other(&mut state, 31);
        {
            let p = state.players.get_mut(&1).unwrap();
            p.fever = Some(NestedHelper::id_only(crate::nested_body::YELLOW_FEVER_ID));
        }
        let _ = take_eat_emotes();
        let _ = take_eat_says();
        assert!(!try_do_eating(&mut state, 1, 2));
        assert_eq!(state.players.get(&1).unwrap().held_id, 31);
        let yf = crate::emotes::emote_by_name("YELLOWFEVER").unwrap().index;
        assert_eq!(take_eat_emotes(), vec![(2, yf)]);
        assert_eq!(
            take_eat_says(),
            vec![(1, "I am too ill!".to_string())]
        );
    }

    /// EAT-ILL-EMOTE: SAY FEED ill feeder fans eater YELLOWFEVER PE + feeder say.
    // Haxe: GlobalPlayerInstance.doEating L3050–3055
    #[test]
    fn say_feed_ill_feeder_emits_yellow_fever_pe() {
        use crate::{apply_intent, Counters};
        use ol_net::{NetIntent, OutboundHub};

        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(Arc::new(db));
        let a = crate::spawn_player(&mut state, 1, "ill@a");
        let b = crate::spawn_player(&mut state, 2, "ill@b");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 20.0;
            p.x = 0;
            p.y = 0;
            p.set_held(31, 0);
            p.fever = Some(NestedHelper::id_only(crate::nested_body::YELLOW_FEVER_ID));
        }
        {
            let p = state.players.get_mut(&2).unwrap();
            p.age = 20.0;
            p.x = 1;
            p.y = 0;
            p.food = 2.0;
            p.food_max = 200.0;
        }
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("FEED {b}"),
            },
        );
        let yf = crate::emotes::emote_by_name("YELLOWFEVER").unwrap().index;
        let want_pe = format!("\n{b} {yf}");
        let mut saw_pe = false;
        let mut saw_say = false;
        for rx in [&mut rx1, &mut rx2] {
            while let Ok(pkt) = rx.try_recv() {
                let s = String::from_utf8_lossy(&pkt);
                if s.starts_with("PE\n") && s.contains(&want_pe) {
                    saw_pe = true;
                }
                if s.contains("I am too ill!") {
                    saw_say = true;
                }
            }
        }
        assert!(saw_pe, "expected PE YELLOWFEVER for eater {b}");
        assert!(saw_say, "expected 'I am too ill!' from feeder {a}");
        assert_eq!(state.players.get(&1).unwrap().held_id, 31);
    }

    #[test]
    fn try_eat_held_drugs_uses_live_fever_resistance() {
        let mut db = ContentDb::default();
        db.objects.insert(
            crate::horse_mount::PSILOCYBE,
            food_def(crate::horse_mount::PSILOCYBE, 3),
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        crate::spawn_player(&mut state, 1, "drug@t");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 20.0;
            p.food = 2.0;
            p.food_max = 40.0;
            p.set_held(crate::horse_mount::PSILOCYBE, 0);
            let mut fever = NestedHelper::id_only(crate::nested_body::YELLOW_FEVER_ID);
            fever.time_to_change = 10.0;
            p.fever = Some(fever);
            p.yellowfever_count = 1.0;
        }
        state.gameplay.allow_eating_or_feeding_if_ill = true;
        state.gameplay.resistance_against_fever_for_eating_mushrooms = 0.5;
        assert!(try_eat_held(&mut state, 1));
        let p = state.players.get(&1).unwrap();
        assert!((p.yellowfever_count - 1.5).abs() < 1e-5);
        assert!((p.fever.as_ref().unwrap().time_to_change - 5.0).abs() < 1e-5);
    }

    /// WALLET-I32-FLOOR: yum eat credits Math.floor(health_delta) coins.
    // Haxe: addHealthAndPrestige this.coins += count
    #[test]
    fn try_eat_held_credits_floored_prestige_coins() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        let pid = crate::spawn_player(&mut state, 1, "yum@coins");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 20.0;
            p.food = 2.0;
            p.food_max = 40.0;
            p.set_held(31, 0);
        }
        let expected = crate::economy::wallet_floor_f32(
            crate::yum::compute_eat(5, 0.0).health_delta,
        );
        assert!(expected >= 1, "fresh yum should credit at least 1 coin");
        assert!(try_eat_held(&mut state, 1));
        assert_eq!(state.economy.coins_of(pid), expected);
        assert_eq!(state.scoreboard.entry(pid).unwrap().coins, expected);
    }

    #[test]
    fn try_eat_held_dark_nosaj_skips_prestige_coins() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        let pid = crate::spawn_player(&mut state, 1, "dark@coins");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 20.0;
            p.food = 2.0;
            p.food_max = 40.0;
            p.dark_nosaj = 1.0;
            p.set_held(31, 0);
        }
        assert!(try_eat_held(&mut state, 1));
        assert_eq!(state.economy.coins_of(pid), 0);
    }

    #[test]
    fn try_eat_held_meh_does_not_debit_coins() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        let pid = crate::spawn_player(&mut state, 1, "meh@coins");
        state.economy.credit_floored_coins(pid, 4.0);
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 20.0;
            p.food = 2.0;
            p.food_max = 40.0;
            p.set_held(31, 0);
            p.yum.reduce_food_value(31, 20.0);
        }
        assert!(try_eat_held(&mut state, 1));
        assert_eq!(state.economy.coins_of(pid), 4);
    }

    fn wear_two_slots(p: &mut crate::Player, a: i32, b: i32) {
        p.first_name = "KID".into();
        p.hat = a;
        p.chest = b;
        p.clothing_helpers[0] = Some(NestedHelper::id_only(a));
        p.clothing_helpers[1] = Some(NestedHelper::id_only(b));
    }

    fn link_mother(state: &mut SimState, child: i32, mother: i32) {
        let mom = state
            .social
            .lineages
            .get(&mother)
            .cloned()
            .unwrap_or_else(|| crate::LineageNode::eve(mother, "MOM"));
        let mut kid = crate::LineageNode::with_mother(child, "KID", &mom);
        kid.alive = true;
        state.social.lineages.insert(child, kid);
        state.social.ensure_lineage(mother, "MOM");
    }

    /// HEALTH-PRESTIGE-FAN: yum eat adds health_delta to self yum/prestige.
    // Haxe: addHealthAndPrestige this.yum_multiplier += count
    #[test]
    fn try_eat_held_adds_self_prestige() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        let pid = crate::spawn_player(&mut state, 1, "yum@pr");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 20.0;
            p.food = 2.0;
            p.food_max = 40.0;
            p.set_held(31, 0);
        }
        let before = state.player_prestige(pid);
        let delta = crate::yum::compute_eat(5, 0.0).health_delta;
        assert!(delta > 0.0);
        assert!(try_eat_held(&mut state, 1));
        assert!((state.player_prestige(pid) - (before + delta)).abs() < 1e-4);
    }

    #[test]
    fn dark_nosaj_blocks_eat_health_prestige() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        let pid = crate::spawn_player(&mut state, 1, "dark@pr");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 20.0;
            p.food = 2.0;
            p.food_max = 40.0;
            p.dark_nosaj = 1.0;
            p.set_held(31, 0);
        }
        let before = state.player_prestige(pid);
        assert!(try_eat_held(&mut state, 1));
        assert!((state.player_prestige(pid) - before).abs() < 1e-5);
        assert_eq!(state.economy.coins_of(pid), 0);
    }

    /// Mother gets count * total_clothing_factor / 4 (two worn slots → factor 1).
    // Haxe: addHealthAndPrestige mother.yum_multiplier += tmpCount * clothingFactor / 4
    #[test]
    fn try_eat_held_fans_mother_quarter() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        let mom = crate::spawn_player(&mut state, 1, "mom@fan");
        let kid = crate::spawn_player(&mut state, 2, "kid@fan");
        link_mother(&mut state, kid, mom);
        // Human Eve-pair spawn may auto-follow; isolate parent share.
        state.social.unfollow(kid);
        {
            let p = state.players.get_mut(&1).unwrap();
            wear_two_slots(p, 100, 200);
            p.first_name = "MOM".into();
        }
        {
            let p = state.players.get_mut(&2).unwrap();
            p.age = 20.0;
            p.food = 2.0;
            p.food_max = 40.0;
            p.set_held(31, 0);
            wear_two_slots(p, 100, 200);
        }
        let mom_before = state.player_prestige(mom);
        let delta = crate::yum::compute_eat(5, 0.0).health_delta;
        assert!(try_eat_held(&mut state, 2));
        // giver 1.0, mother 1.0 → total 1.0 → mother += delta/4
        let got = state.player_prestige(mom) - mom_before;
        assert!(
            (got - delta / 4.0).abs() < 1e-4,
            "mother fan {got} want {}",
            delta / 4.0
        );
        assert_eq!(state.economy.coins_of(mom), 0);
        let kid_eat = state
            .social
            .lineages
            .get(&kid)
            .map(|n| n.prestige_from.eating)
            .unwrap_or(0.0);
        assert!(
            (kid_eat - delta).abs() < 1e-4,
            "eater prestigeFromEating {kid_eat} want {delta}"
        );
        let mom_kids = state
            .social
            .lineages
            .get(&mom)
            .map(|n| n.prestige_from.children)
            .unwrap_or(0.0);
        assert!(
            (mom_kids - delta / 4.0).abs() < 1e-4,
            "mother prestigeFromChildren {mom_kids}"
        );
    }

    /// Follow leader gets count/4 * factor coins+prestige (same family).
    // Haxe: addHealthAndPrestige leader.yum_multiplier/coins += tmpCount * leaderFactor
    #[test]
    fn try_eat_held_fans_leader_coins() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        let lead = crate::spawn_player(&mut state, 1, "lead@fan");
        let kid = crate::spawn_player(&mut state, 2, "kid@lead");
        link_mother(&mut state, kid, lead);
        state.social.set_follow(kid, lead).unwrap();
        {
            let p = state.players.get_mut(&1).unwrap();
            wear_two_slots(p, 100, 200);
            p.first_name = "MOM".into();
        }
        {
            let p = state.players.get_mut(&2).unwrap();
            p.age = 20.0;
            p.food = 2.0;
            p.food_max = 40.0;
            p.set_held(31, 0);
            wear_two_slots(p, 100, 200);
        }
        let lead_before = state.player_prestige(lead);
        let coins_before = state.economy.coins_of(lead);
        let delta = crate::yum::compute_eat(5, 0.0).health_delta;
        assert!(try_eat_held(&mut state, 2));
        // parent /4 plus leader tmp=delta/4 * factor 1 → extra delta/4
        let got = state.player_prestige(lead) - lead_before;
        assert!(
            (got - delta / 2.0).abs() < 1e-4,
            "parent+leader fan {got} want {}",
            delta / 2.0
        );
        let leader_coins = crate::economy::wallet_floor_f32(delta / 4.0);
        assert_eq!(
            state.economy.coins_of(lead) - coins_before,
            leader_coins
        );
    }

    /// PRESTIGE-CLOTH-FACTOR: crown extraPrestigeFactor adds to leader factor.
    // Haxe: calculateClothingPrestigeFactorForLeader extraPrestigeFactor
    #[test]
    fn try_eat_held_leader_crown_extra_prestige() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut crown = ObjectDef::empty(693);
        crown.clothing = "h".into();
        crown.prestige_factor = 1.5;
        crown.extra_prestige_factor = 0.2;
        db.objects.insert(693, crown);
        let mut state = SimState::with_default_empty(Arc::new(db));
        let lead = crate::spawn_player(&mut state, 1, "lead@crown");
        let kid = crate::spawn_player(&mut state, 2, "kid@crown");
        link_mother(&mut state, kid, lead);
        state.social.set_follow(kid, lead).unwrap();
        {
            let p = state.players.get_mut(&1).unwrap();
            wear_two_slots(p, 693, 0);
            p.first_name = "MOM".into();
        }
        {
            let p = state.players.get_mut(&2).unwrap();
            p.age = 20.0;
            p.food = 2.0;
            p.food_max = 40.0;
            p.set_held(31, 0);
            wear_two_slots(p, 100, 200);
        }
        let lead_before = state.player_prestige(lead);
        let delta = crate::yum::compute_eat(5, 0.0).health_delta;
        assert!(try_eat_held(&mut state, 2));
        // kid unknown slots → 1.0; crown hat getPrestigeFactor 0.6
        // parent: total 0.8 → delta*0.8/4
        // leader: (0.8 + extra 0.2) * delta/4
        let want = delta * 0.8 / 4.0 + delta / 4.0 * 1.0;
        let got = state.player_prestige(lead) - lead_before;
        assert!(
            (got - want).abs() < 1e-4,
            "crown extra fan {got} want {want}"
        );
    }

    fn setup_feed_other(state: &mut SimState, food_id: i32) {
        crate::spawn_player(state, 1, "feeder@eat");
        crate::spawn_player(state, 2, "eater@eat");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 20.0;
            p.set_held(food_id, 0);
        }
        {
            let p = state.players.get_mut(&2).unwrap();
            p.age = 20.0;
            p.food = 2.0;
            p.food_max = 200.0;
        }
    }

    /// FEED-OTHER-EAT: fill is compute_eat_full × world × starving, not raw food_value.
    // Haxe: GlobalPlayerInstance.doEating L3087–3192
    #[test]
    fn try_do_eating_feed_other_fill_uses_compute_eat_full() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        setup_feed_other(&mut state, 31);
        let knobs = state.gameplay.eat_live_knobs();
        let fill = crate::compute_eat_full(5, 0.0, knobs).fill;
        let world_ff = state.world_food.get_food_factor(31);
        let starve_ff = state.world_food.get_starving_food_factor_at(state.sim_time);
        let want = crate::apply_world_food_factors(fill, world_ff, starve_ff);
        assert!(
            (want - 5.0).abs() > 1.0,
            "yum+world fill must exceed raw food_value; want={want}"
        );
        assert!(try_do_eating(&mut state, 1, 2));
        let eater = state.players.get(&2).unwrap();
        assert!(
            (eater.food - (2.0 + want)).abs() < 1e-3,
            "eater food {} want {}",
            eater.food,
            2.0 + want
        );
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
        assert!(
            state
                .world_food
                .eaten_values
                .get(&31)
                .copied()
                .unwrap_or(0.0)
                > 0.0,
            "feed-other must add_food_statistic"
        );
        assert!((eater.yum.get_count_eaten(31) - 1.0).abs() < 1e-4);
    }

    /// Haxe addFood: eating past food_store_max stores overflow as yum_bonus extra pips.
    #[test]
    fn try_do_eating_overflow_goes_to_yum_bonus() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        setup_feed_other(&mut state, 31);
        {
            let p = state.players.get_mut(&2).unwrap();
            p.food = 18.0;
            p.food_max = 20.0;
            p.yum.yum_bonus = 0.0;
        }
        let knobs = state.gameplay.eat_live_knobs();
        let fill = crate::compute_eat_full(5, 0.0, knobs).fill;
        let world_ff = state.world_food.get_food_factor(31);
        let starve_ff = state.world_food.get_starving_food_factor_at(state.sim_time);
        let want = crate::apply_world_food_factors(fill, world_ff, starve_ff);
        assert!(try_do_eating(&mut state, 1, 2));
        let eater = state.players.get(&2).unwrap();
        let (want_food, want_yum) = crate::food_store_max::add_food(18.0, 20.0, want, 0.0);
        assert!((eater.food - want_food).abs() < 1e-3, "food {}", eater.food);
        assert!(
            (eater.yum.yum_bonus - want_yum).abs() < 1e-3,
            "yum_bonus {} want {want_yum}",
            eater.yum.yum_bonus
        );
        assert!(
            want_yum > 0.0,
            "overflow eat must store extra pips as yum_bonus"
        );
        assert!((eater.yum.get_count_eaten(31) - 1.0).abs() < 1e-4);
    }

    /// Craving (count_eaten < 0) extra fill + hasEaten increment matches compute_eat.
    #[test]
    fn try_do_eating_craving_fill_and_count_eaten() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        setup_feed_other(&mut state, 31);
        {
            let p = state.players.get_mut(&2).unwrap();
            p.food = 2.0;
            p.food_max = 200.0;
            p.yum.has_eaten.insert(31, -4.0);
        }
        let computed = crate::compute_eat_full(5, -4.0, state.gameplay.eat_live_knobs());
        assert!(computed.is_craving);
        let world_ff = state.world_food.get_food_factor(31);
        let starve_ff = state.world_food.get_starving_food_factor_at(state.sim_time);
        let want = crate::apply_world_food_factors(computed.fill, world_ff, starve_ff);
        assert!(try_do_eating(&mut state, 1, 2));
        let eater = state.players.get(&2).unwrap();
        assert!(
            (eater.food - (2.0 + want)).abs() < 1e-3,
            "craving fill {} want {}",
            eater.food,
            2.0 + want
        );
        // -4 + has_eaten_delta (2)
        assert!(
            (eater.yum.get_count_eaten(31) - (-4.0 + computed.has_eaten_delta)).abs() < 1e-4,
            "count_eaten {}",
            eater.yum.get_count_eaten(31)
        );
    }

    /// FEED-OTHER-EAT: live YumBonus changes feed-other fill.
    // Haxe: ServerSettings.YumBonus / doEating L3087
    #[test]
    fn try_do_eating_feed_other_uses_live_yum_bonus() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        setup_feed_other(&mut state, 31);
        state.gameplay.yum_bonus = 7.0;
        let knobs = state.gameplay.eat_live_knobs();
        let fill = crate::compute_eat_full(5, 0.0, knobs).fill;
        let world_ff = state.world_food.get_food_factor(31);
        let starve_ff = state.world_food.get_starving_food_factor_at(state.sim_time);
        let want = crate::apply_world_food_factors(fill, world_ff, starve_ff);
        assert!(try_do_eating(&mut state, 1, 2));
        let got = state.players.get(&2).unwrap().food - 2.0;
        assert!(
            (got - want).abs() < 1e-3,
            "live yum_bonus fill {got} want {want}"
        );
        let default_fill = crate::yum::compute_eat(5, 0.0).fill;
        let default_want = crate::apply_world_food_factors(default_fill, world_ff, starve_ff);
        assert!(
            (want - default_want).abs() > 0.5,
            "yum_bonus 7 must differ from default 5 after world factors"
        );
    }

    /// FEED-OTHER-EAT: meh feed refused when eater food_store > 2.
    // Haxe: GlobalPlayerInstance.doEating L3108–3112 / canFeedToMeObj
    #[test]
    fn try_do_eating_feed_other_refuses_meh_when_not_starving() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        setup_feed_other(&mut state, 31);
        {
            let p = state.players.get_mut(&2).unwrap();
            p.food = 10.0;
            p.yum.reduce_food_value(31, 20.0);
        }
        assert!(!try_do_eating(&mut state, 1, 2));
        assert_eq!(state.players.get(&1).unwrap().held_id, 31);
        assert!((state.players.get(&2).unwrap().food - 10.0).abs() < 1e-5);
    }

    /// FEED-OTHER-EAT: feeder too young (MinAgeToEat) cannot feed.
    // Haxe: GlobalPlayerInstance.doEating L3045–3047
    #[test]
    fn try_do_eating_feed_other_too_young_feeder() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        setup_feed_other(&mut state, 31);
        state.players.get_mut(&1).unwrap().age = 1.0;
        assert!(!try_do_eating(&mut state, 1, 2));
        assert_eq!(state.players.get(&1).unwrap().held_id, 31);
    }

    /// FEED-OTHER-EAT: yum feeder prestige share 0.2 × eater health_delta.
    // Haxe: GlobalPlayerInstance.doEating L3150–3152
    #[test]
    fn try_do_eating_feed_other_feeder_yum_prestige_share() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        setup_feed_other(&mut state, 31);
        let feeder_pid = state.players.get(&1).unwrap().p_id;
        let eater_pid = state.players.get(&2).unwrap().p_id;
        let before_f = state.player_prestige(feeder_pid);
        let before_e = state.player_prestige(eater_pid);
        let computed = crate::compute_eat_full(5, 0.0, state.gameplay.eat_live_knobs());
        assert!(computed.is_yum);
        assert!(try_do_eating(&mut state, 1, 2));
        let eater_got = state.player_prestige(eater_pid) - before_e;
        let feeder_got = state.player_prestige(feeder_pid) - before_f;
        assert!((eater_got - computed.health_delta).abs() < 1e-4);
        let want_share = computed.health_delta * crate::FEED_OTHER_FEEDER_PRESTIGE_SHARE;
        assert!(
            feeder_got + 1e-4 >= want_share,
            "feeder share {feeder_got} want at least {want_share}"
        );
        // Family/leader yum fan may add a little on top of the 0.2 feed-other share.
        assert!(
            (feeder_got - want_share).abs() < computed.health_delta.abs().max(0.25),
            "feeder share {feeder_got} vs {want_share}"
        );
    }

    fn pe_index(name: &str) -> i32 {
        crate::emotes::emote_by_name(name).unwrap().index
    }

    /// FEED-OTHER-EMOTE: yum feed-other → eater HAPPY; feeder no PE.
    // Haxe: GlobalPlayerInstance.doEating L3243
    #[test]
    fn try_do_eating_feed_other_emote_yum_happy() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        setup_feed_other(&mut state, 31);
        let _ = take_eat_emotes();
        assert!(try_do_eating(&mut state, 1, 2));
        assert_eq!(take_eat_emotes(), vec![(2, pe_index("HAPPY"))]);
    }

    /// FEED-OTHER-EMOTE: meh (starving) → eater SAD.
    // Haxe: GlobalPlayerInstance.doEating L3245
    #[test]
    fn try_do_eating_feed_other_emote_meh_sad() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        setup_feed_other(&mut state, 31);
        {
            let p = state.players.get_mut(&2).unwrap();
            p.food = 2.0;
            p.yum.reduce_food_value(31, 5.0);
        }
        let _ = take_eat_emotes();
        assert!(try_do_eating(&mut state, 1, 2));
        assert_eq!(take_eat_emotes(), vec![(2, pe_index("SAD"))]);
    }

    /// FEED-OTHER-EMOTE: superMeh (food ≤ 5) → eater ILL.
    // Haxe: GlobalPlayerInstance.doEating L3244
    #[test]
    fn try_do_eating_feed_other_emote_super_meh_ill() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        setup_feed_other(&mut state, 31);
        {
            let p = state.players.get_mut(&2).unwrap();
            p.food = 2.0;
            p.yum.reduce_food_value(31, 20.0);
        }
        let _ = take_eat_emotes();
        assert!(try_do_eating(&mut state, 1, 2));
        assert_eq!(take_eat_emotes(), vec![(2, pe_index("ILL"))]);
    }

    /// FEED-OTHER-EMOTE: craving → eater MIAMFOOD + feeder HAPPY.
    // Haxe: GlobalPlayerInstance.doEating L3239–3241
    #[test]
    fn try_do_eating_feed_other_emote_craving_miam_and_feeder_happy() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        setup_feed_other(&mut state, 31);
        state
            .players
            .get_mut(&2)
            .unwrap()
            .yum
            .has_eaten
            .insert(31, -4.0);
        let _ = take_eat_emotes();
        assert!(try_do_eating(&mut state, 1, 2));
        assert_eq!(
            take_eat_emotes(),
            vec![(2, pe_index("MIAMFOOD")), (1, pe_index("HAPPY"))]
        );
    }

    /// FEED-OTHER-EMOTE: self yum eat → HAPPY only (no extra feeder).
    // Haxe: GlobalPlayerInstance.doEating L3243
    #[test]
    fn try_eat_held_emote_yum_happy() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        crate::spawn_player(&mut state, 1, "self@pe");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 20.0;
            p.food = 2.0;
            p.food_max = 200.0;
            p.set_held(31, 0);
        }
        let _ = take_eat_emotes();
        assert!(try_eat_held(&mut state, 1));
        assert_eq!(take_eat_emotes(), vec![(1, pe_index("HAPPY"))]);
    }

    /// EAT-REFUSE-EMOTE: self superMeh + food > 5 → ILL + toSelf say.
    // Haxe: GlobalPlayerInstance.doEating L3101–3103
    #[test]
    fn try_eat_held_refuse_super_meh_ill_and_say() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        crate::spawn_player(&mut state, 1, "sm@self");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 20.0;
            p.food = 10.0;
            p.food_max = 200.0;
            p.set_held(31, 0);
            p.yum.reduce_food_value(31, 20.0);
        }
        let _ = take_eat_emotes();
        let _ = take_eat_says();
        assert!(!try_eat_held(&mut state, 1));
        assert_eq!(state.players.get(&1).unwrap().held_id, 31);
        assert_eq!(take_eat_emotes(), vec![(1, pe_index("ILL"))]);
        assert_eq!(
            take_eat_says(),
            vec![(1, "I need better food!".to_string())]
        );
    }

    /// EAT-REFUSE-EMOTE: feed-other superMeh + food > 5 → feeder SAD.
    // Haxe: GlobalPlayerInstance.doEating L3105
    #[test]
    fn try_do_eating_refuse_super_meh_feeder_sad() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        setup_feed_other(&mut state, 31);
        {
            let p = state.players.get_mut(&2).unwrap();
            p.food = 10.0;
            p.yum.reduce_food_value(31, 20.0);
        }
        let _ = take_eat_emotes();
        assert!(!try_do_eating(&mut state, 1, 2));
        assert_eq!(take_eat_emotes(), vec![(1, pe_index("SAD"))]);
        assert!(take_eat_says().is_empty());
    }

    /// EAT-REFUSE-EMOTE: feed-other meh (not super) + food > 2 → feeder SAD.
    // Haxe: GlobalPlayerInstance.doEating L3108–3111
    #[test]
    fn try_do_eating_refuse_meh_feed_feeder_sad() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        setup_feed_other(&mut state, 31);
        {
            let p = state.players.get_mut(&2).unwrap();
            p.food = 10.0;
            p.yum.reduce_food_value(31, 5.0);
        }
        let _ = take_eat_emotes();
        assert!(!try_do_eating(&mut state, 1, 2));
        assert_eq!(take_eat_emotes(), vec![(1, pe_index("SAD"))]);
    }

    /// EAT-REFUSE-EMOTE: SAY FEED meh refuse fans feeder SAD PE.
    // Haxe: GlobalPlayerInstance.doEating L3110
    #[test]
    fn say_feed_refuse_meh_emits_feeder_sad_pe() {
        use crate::{apply_intent, Counters};
        use ol_net::{NetIntent, OutboundHub};

        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(Arc::new(db));
        let a = crate::spawn_player(&mut state, 1, "ref@a");
        let b = crate::spawn_player(&mut state, 2, "ref@b");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 20.0;
            p.x = 0;
            p.y = 0;
            p.set_held(31, 0);
        }
        {
            let p = state.players.get_mut(&2).unwrap();
            p.age = 20.0;
            p.x = 1;
            p.y = 0;
            p.food = 10.0;
            p.food_max = 200.0;
            p.yum.reduce_food_value(31, 5.0);
        }
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("FEED {b}"),
            },
        );
        let want = format!("\n{a} {}", pe_index("SAD"));
        let mut saw = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PE\n") && s.contains(&want) {
                saw = true;
            }
        }
        assert!(saw, "expected PE SAD for feeder {a}");
        assert_eq!(state.players.get(&1).unwrap().held_id, 31);
    }

    /// FEED-OTHER-EMOTE: SAY FEED fans eater HAPPY PE to nearby.
    // Haxe: GlobalPlayerInstance.doEating L3243 doEmote
    #[test]
    fn say_feed_emits_eater_happy_pe() {
        use crate::{apply_intent, Counters};
        use ol_net::{NetIntent, OutboundHub};

        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(Arc::new(db));
        let a = crate::spawn_player(&mut state, 1, "pe@a");
        let b = crate::spawn_player(&mut state, 2, "pe@b");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 20.0;
            p.x = 0;
            p.y = 0;
            p.set_held(31, 0);
        }
        {
            let p = state.players.get_mut(&2).unwrap();
            p.age = 20.0;
            p.x = 1;
            p.y = 0;
            p.food = 2.0;
            p.food_max = 200.0;
        }
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("FEED {b}"),
            },
        );
        let want = format!("\n{b} {}", pe_index("HAPPY"));
        let mut saw = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PE\n") && s.contains(&want) {
                saw = true;
            }
        }
        assert!(saw, "expected PE HAPPY for eater {b}");
        let _ = a;
    }

    /// EAT-RESPONSIBLE-PU: feed-other stamps feeder p_id; self-eat stays -1.
    // Haxe: GlobalPlayerInstance.doEating L3175
    #[test]
    fn try_do_eating_feed_other_sets_responsible_id() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        setup_feed_other(&mut state, 31);
        let feeder_pid = state.players.get(&1).unwrap().p_id;
        assert!(try_do_eating(&mut state, 1, 2));
        assert_eq!(state.players.get(&2).unwrap().yum.responsible_id, feeder_pid);
        assert_eq!(
            crate::feed_other_yum::feed_other_responsible_id(feeder_pid, state.players.get(&2).unwrap().p_id),
            feeder_pid
        );
    }

    /// EAT-RESPONSIBLE-PU: self-eat responsible_id is -1.
    // Haxe: GlobalPlayerInstance.doEating L3175
    #[test]
    fn try_eat_held_responsible_id_self() {
        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let mut state = SimState::with_default_empty(Arc::new(db));
        crate::spawn_player(&mut state, 1, "self@rid");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 20.0;
            p.food = 2.0;
            p.food_max = 200.0;
            p.set_held(31, 0);
        }
        assert!(try_eat_held(&mut state, 1));
        assert_eq!(state.players.get(&1).unwrap().yum.responsible_id, -1);
    }

    /// EAT-RESPONSIBLE-PU: SAY FEED FX + PU carry feeder p_id.
    // Haxe: sendFoodUpdate + PlayerInstance.toData responsible_id
    #[test]
    fn say_feed_fx_pu_responsible_id() {
        use crate::{apply_intent, Counters};
        use ol_net::{NetIntent, OutboundHub};

        let mut db = ContentDb::default();
        db.objects.insert(31, food_def(31, 5));
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(Arc::new(db));
        let a = crate::spawn_player(&mut state, 1, "rid@a");
        let b = crate::spawn_player(&mut state, 2, "rid@b");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 20.0;
            p.x = 0;
            p.y = 0;
            p.set_held(31, 0);
        }
        {
            let p = state.players.get_mut(&2).unwrap();
            p.age = 20.0;
            p.x = 1;
            p.y = 0;
            p.food = 2.0;
            p.food_max = 200.0;
        }
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("FEED {b}"),
            },
        );
        let mut saw_fx = false;
        let mut saw_pu = false;
        let needle = format!(" 1 31 {a} ");
        for rx in [&mut rx1, &mut rx2] {
            while let Ok(pkt) = rx.try_recv() {
                let s = String::from_utf8_lossy(&pkt);
                if s.starts_with("FX\n") {
                    let fields: Vec<&str> =
                        s.lines().nth(1).unwrap_or("").split_whitespace().collect();
                    if fields.len() >= 6 && fields[5] == a.to_string() {
                        saw_fx = true;
                    }
                }
                if s.starts_with("PU\n") && s.contains(&needle) {
                    saw_pu = true;
                }
            }
        }
        assert_eq!(state.players.get(&2).unwrap().yum.responsible_id, a);
        assert!(saw_fx, "expected FX responsible_id={a}");
        assert!(saw_pu, "expected PU just_ate last_ate responsible={a}");
        let _ = b;
    }
}
