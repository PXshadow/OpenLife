//! GPI-DEATH-POLISH + GPI-PLACE-GRAVE — death helper + placeGrave richness.
//!
//! Haxe `GlobalPlayerInstance.doDeathHelper` order (subset):
//! 1. [`apply_death_hold_detach`] drop held player / Baby Bones 1920 in carrier arms
//! 2. [`choose_new_leader`](crate::death_inherit::choose_new_leader)
//! 3. [`place_grave_for_player`] / placeGrave (this module)
//! 3. InheritOwnership on world helpers (+ refresh `Player.owning`)
//! 4. [`apply_inherit_coins`](crate::death_inherit::apply_inherit_coins) (+ grave residual)
//! 5. [`create_score_entry_for_dead_relative`](crate::score_entry) (SCORE-ENTRY)
//!
//! PLACE-OBJECT free_tile_search lives in [`place_object_impl`] (sibling `place_object.rs`).

// Haxe: WorldMap.PlaceObject / TryPlaceObject (PLACE-OBJECT / free_tile_search)
#[path = "place_object.rs"]
mod place_object_impl;
pub use place_object_impl::{
    can_be_placed_in_grave, can_be_placed_in_grave_sized, contain_fits_slot, contain_slot_sizes,
    is_grave_object, is_permanent_object, is_tree_description, is_tree_object,
    object_contain_fits_container, place_complex_object, place_object, place_object_by_id,
    place_object_near, place_object_with_rng, place_random_offset, place_search_candidate,
    place_search_distance_step, transform_placed_object_id, transition_result_fits_container,
    transition_result_fits_container_from_content, try_place_flat_on_world, try_place_kind,
    PlaceObjectOpts, PlaceObjectResult, TryPlaceKind, HORSE_DRAWN_CART_ID,
    HORSE_DRAWN_TIRE_CART_ID, PLACE_DROP_WALLS_AFTER, PLACE_MAX_ATTEMPTS,
};

use crate::animal_move::is_bone_grave;
use crate::death_inherit::{
    add_owner_to_helper, apply_inherit_coins, choose_new_leader, count_leadership_power,
    format_inherit_events, format_leader_succession_event, format_ownership_events,
    remove_owner_from_helper, stamp_grave_soul, InheritContext, OwnershipTransfer,
};
use crate::mutation::SpecialKind;
use crate::player::ClothingSlot;
use crate::relations::root_eve_id;
use crate::score_entry::{
    create_score_entry_for_dead_relative, DeadRelativePlayer, MotherLineNode,
};
use crate::SimState;
use ol_content::ContentDb;
use ol_net::OutboundHub;
use ol_protocol::{
    format_grave_info, format_grave_old, format_grave_old_line, format_owner_list,
};
use ol_world::ComplexObject;
use std::collections::{HashMap, HashSet};

// ── Haxe grave object ids (ObjectData / placeGrave) ─────────────────────────

/// Haxe `1920` Baby Bones — held in carrier arms when a held player dies.
// Haxe: ObjectData Baby Bones; GPI.doDeathHelper L4014 TODO
pub const BABY_BONES_ID: i32 = 1920;
/// Haxe `3053` Baby Bone Pile — age &lt; MinAgeToEat.
pub const BABY_BONE_PILE_ID: i32 = 3053;
/// Haxe `752` Murder Grave — adult holding a wound object.
pub const MURDER_GRAVE_ID: i32 = 752;
/// Haxe `87` Fresh Grave — default adult death.
pub const FRESH_GRAVE_ID: i32 = 87;
/// Haxe `ServerSettings.MinAgeToEat` (years) — baby bone pile threshold.
pub const GRAVE_MIN_AGE_TO_EAT: f32 = 3.0;

/// Result of a successful [`place_grave_for_player`] / placeGrave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceGraveResult {
    pub x: i32,
    pub y: i32,
    pub grave_id: i32,
    pub creator_p_id: i32,
    /// Clothing + containable held packed into the grave (not wounds).
    pub contained: Vec<i32>,
}

// ── Pure select / wound helpers ─────────────────────────────────────────────

pub use ol_combat_rules::{
    is_arrow_wound_description, is_arrow_wound_object, is_wound_description, is_wound_object,
};

/// Haxe `placeGrave` grave id selection: baby 3053 / murder 752 / fresh 87.
/// // Haxe: GlobalPlayerInstance.placeGrave (age + heldObject.isWound)
pub fn select_grave_object_id(age: f32, held_is_wound: bool) -> i32 {
    select_grave_object_id_with_min_age(age, held_is_wound, GRAVE_MIN_AGE_TO_EAT)
}

/// Same as [`select_grave_object_id`] with explicit MinAgeToEat.
pub fn select_grave_object_id_with_min_age(
    age: f32,
    held_is_wound: bool,
    min_age_to_eat: f32,
) -> i32 {
    if age < min_age_to_eat {
        BABY_BONE_PILE_ID
    } else if held_is_wound {
        MURDER_GRAVE_ID
    } else {
        FRESH_GRAVE_ID
    }
}

/// Prefer Haxe select id when present in content; else name-contains-grave fallback.
///
/// When content is empty / partial (unit tests), returns `0` so callers skip placement
/// and keep full ground scatter of loot.
pub fn resolve_place_grave_id(content: &ContentDb, age: f32, held_is_wound: bool) -> i32 {
    resolve_place_grave_id_with_min_age(content, age, held_is_wound, GRAVE_MIN_AGE_TO_EAT)
}

/// Same as [`resolve_place_grave_id`] with live `ServerSettings.MinAgeToEat`.
// Haxe: ServerSettings.MinAgeToEat — C-SS-MIN-AGE-AI
pub fn resolve_place_grave_id_with_min_age(
    content: &ContentDb,
    age: f32,
    held_is_wound: bool,
    min_age_to_eat: f32,
) -> i32 {
    let preferred = select_grave_object_id_with_min_age(age, held_is_wound, min_age_to_eat);
    if content.objects.contains_key(&preferred) {
        return preferred;
    }
    // Name-based content resolution (legacy resolve_grave_object_id path).
    crate::resolve_grave_object_id(content)
}

/// Haxe-shaped `GRAVE` wire body for event logs / tests: `x y creator_p_id`.
/// // Haxe: Connection.SendGraveInfoToAll → GRAVE x y getCreatorId()
pub fn format_grave_place_log(x: i32, y: i32, creator_p_id: i32) -> String {
    format!("GRAVE {x} {y} {creator_p_id}")
}

// ── Placement helpers (PLACE-OBJECT free_tile_search) ────────────────────────

fn object_containable(content: &ContentDb, id: i32) -> bool {
    content
        .objects
        .get(&id)
        .map(|o| o.containable)
        .unwrap_or(false)
}

/// Place grave complex via full Haxe PlaceObject free-tile / wall / biome search.
/// // Haxe: WorldMap.PlaceObject(tx, ty, grave, true, true)
fn place_grave_on_map(
    state: &mut SimState,
    cx: i32,
    cy: i32,
    grave: ComplexObject,
) -> Option<(i32, i32)> {
    place_complex_object(state, cx, cy, grave, PlaceObjectOpts::grave_or_held()).map(|r| (r.x, r.y))
}

// ── Public placeGrave port ──────────────────────────────────────────────────

/// Place a content grave with soul stamp at `(x, y)` when `grave_id != 0`.
///
/// Also records the tile on the soft account (`PlayerAccount.graves` subset).
/// // Haxe: GlobalPlayerInstance.placeGrave + account.graves push
pub fn place_grave_with_soul(
    state: &mut SimState,
    x: i32,
    y: i32,
    grave_id: i32,
    p_id: i32,
    email: &str,
) {
    if grave_id == 0 {
        return;
    }
    let mut grave = ComplexObject::new_simple(grave_id);
    let aid = crate::death_inherit::account_id_ensure(&mut state.accounts, email);
    stamp_grave_soul(&mut grave, p_id, aid);
    state.world.write().unwrap().set_object_complex(x, y, grave);
    state.record_world_change(x, y, grave_id);
    state.specials.insert(x, y, SpecialKind::Grave);
    // Haxe: account.graves push (session; rewired by InitObjectHelpersAfterRead)
    state.accounts.record_grave(email, x, y);
}

/// Full Haxe `placeGrave` for a living/dying player id.
///
/// Selects 3053/752/87, runs held `(held,-1)` death transition, packs clothing +
/// containable held into the grave, places non-containables nearby, stamps soul,
/// records account graves, and emits a `GRAVE` event log line.
///
/// Returns `None` when content cannot resolve a grave id (tests without graves).
/// // Haxe: GlobalPlayerInstance.placeGrave
pub fn place_grave_for_player(state: &mut SimState, p_id: i32) -> Option<PlaceGraveResult> {
    let conn = state
        .players
        .iter()
        .find(|(_, pl)| pl.p_id == p_id)
        .map(|(&c, _)| c)?;
    place_grave_for_conn(state, conn)
}

/// [`place_grave_for_player`] by connection id.
pub fn place_grave_for_conn(state: &mut SimState, conn_id: u64) -> Option<PlaceGraveResult> {
    let (p_id, email, age, mut held_id, cx, cy, clothing) = {
        let pl = state.players.get(&conn_id)?;
        let clothing = [
            pl.clothing(ClothingSlot::Hat),
            pl.clothing(ClothingSlot::Chest),
            pl.clothing(ClothingSlot::Shoes),
        ];
        (
            pl.p_id,
            pl.email.clone(),
            pl.age,
            pl.held_id,
            pl.x,
            pl.y,
            clothing,
        )
    };

    let held_is_wound = is_wound_object(&state.content, held_id);
    // C-SS-MIN-AGE-AI: live MinAgeToEat for baby bone pile threshold
    let grave_id = resolve_place_grave_id_with_min_age(
        &state.content,
        age,
        held_is_wound,
        state.gameplay.min_age_to_eat,
    );
    if grave_id == 0 {
        return None;
    }

    // ── Held death transition GetTransition(held, -1) ─────────────────────
    // Haxe: placeGrave heldTransition (rope animals etc.)
    if held_id != 0 {
        if let Some(tr) = state.content.find_transition(held_id, -1) {
            let new_target = tr.new_target_id;
            let new_actor = tr.new_actor_id;
            if new_target != 0 {
                let _ = place_object_near(state, cx, cy, new_target, false);
            }
            held_id = new_actor;
            if let Some(pl) = state.players.get_mut(&conn_id) {
                if new_actor == 0 {
                    pl.clear_held();
                } else {
                    pl.set_held(new_actor, 0);
                }
            }
        }
    }

    // Re-read wound after transform (actor may change).
    let held_is_wound = is_wound_object(&state.content, held_id);
    let mut contained: Vec<i32> = Vec::new();

    // ── Held → grave or ground (never pack wounds) ────────────────────────
    // Haxe: if held && !isWound → containable push / else PlaceObject
    if held_id != 0 {
        if held_is_wound {
            if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.clear_held();
            }
        } else if object_containable(&state.content, held_id) {
            contained.push(held_id);
            if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.clear_held();
            }
        } else {
            // Haxe: PlaceObject(..., allowReplace=true, considerWalls=true)
            let _ = place_complex_object(
                state,
                cx,
                cy,
                ComplexObject::new_simple(held_id),
                PlaceObjectOpts::grave_or_held(),
            );
            if let Some(pl) = state.players.get_mut(&conn_id) {
                pl.clear_held();
            }
        }
    }

    // ── Clothing → grave.containedObjects ─────────────────────────────────
    // Haxe: for clothingObjects if id != 0 push (no need to clear — body dead)
    for id in clothing {
        if id != 0 {
            contained.push(id);
        }
    }
    if let Some(pl) = state.players.get_mut(&conn_id) {
        for slot in [ClothingSlot::Hat, ClothingSlot::Chest, ClothingSlot::Shoes] {
            pl.set_clothing(slot, 0);
        }
    }

    let mut grave = ComplexObject::new_simple(grave_id);
    grave.contained = contained.clone();
    let aid = crate::death_inherit::account_id_ensure(&mut state.accounts, &email);
    stamp_grave_soul(&mut grave, p_id, aid);

    let (gx, gy) = place_grave_on_map(state, cx, cy, grave)?;
    state.specials.insert(gx, gy, SpecialKind::Grave);
    state.accounts.record_grave(&email, gx, gy);
    state.push_event(format_grave_place_log(gx, gy, p_id));

    Some(PlaceGraveResult {
        x: gx,
        y: gy,
        grave_id,
        creator_p_id: p_id,
        contained,
    })
}

/// Haxe `Connection.SendGraveInfoToAll` — birth-relative `GRAVE x y creator`.
/// // Haxe: Connection.SendGraveInfoToAll
pub fn send_grave_info_to_all(
    state: &SimState,
    outbound: &OutboundHub,
    world_x: i32,
    world_y: i32,
    creator_p_id: i32,
) {
    for pl in state.players.values() {
        if !pl.connected || pl.deleted {
            continue;
        }
        let (x, y) = pl.world_to_client(world_x, world_y);
        let pkt = format_grave_info(x, y, creator_p_id).into_bytes();
        outbound.send(pl.conn_id, pkt);
    }
}

/// Haxe `getFullName(true)` for GO: spaces → `_`; empty name is `~`.
// Haxe: Lineage.getFullName L526–533; protocol GRAVE_OLD name
pub fn grave_old_underscored_name(name: &str, class_name: &str) -> String {
    let n = name.trim();
    if n.is_empty() {
        return "~".into();
    }
    let full = if class_name.is_empty() {
        n.to_string()
    } else {
        format!("{n} {class_name}")
    };
    full.replace(' ', "_")
}

/// Haxe `Lineage.getDeadSince` — floor((sim_time − deathTime) / 60 years).
// Haxe: Lineage.getDeadSince L536–540 (seconds / 60)
pub fn grave_dead_since_years(sim_time: f32, death_sim_time: f32) -> i32 {
    let secs = (sim_time - death_sim_time).max(0.0);
    (secs / crate::age_curves::SECONDS_PER_YEAR).floor() as i32
}

/// Client `GRAVE x y` → GO (`sendGraveInfoHelper`). Silent if no creator lineage.
// Haxe: Connection.sendGraveInfoHelper L1317–1329
pub fn send_grave_old_query(
    state: &crate::SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    client_x: i32,
    client_y: i32,
    world_x: i32,
    world_y: i32,
) {
    let creator = {
        let Ok(world) = state.world.read() else {
            return;
        };
        let Some(h) = world.get_helper(world_x, world_y) else {
            return;
        };
        if !h.living_owners.is_empty() {
            h.living_owners[0]
        } else {
            h.owner_id
        }
    };
    if creator <= 0 {
        return;
    }
    let Some(node) = state.social.lineages.get(&creator) else {
        return;
    };
    let po_id = if node.po_id > 0 {
        node.po_id
    } else {
        state
            .players
            .values()
            .find(|p| p.p_id == creator)
            .map(crate::person_object_id)
            .unwrap_or(0)
    };
    let dead = grave_dead_since_years(state.sim_time, node.death_sim_time);
    let name = grave_old_underscored_name(&node.name, node.prestige_class.class_name());
    let lineage = crate::social::create_lineage_string(&state.social.lineages, creator, false);
    let body = format_grave_old_line(client_x, client_y, creator, po_id, dead, &name, &lineage);
    outbound.send(conn_id, format_grave_old(&body).into_bytes());
}

/// Client `OWNER x y` → OW (`sendOwners`). Silent if no owners and not `isOwned`.
/// Unowned `+owned` / `+tempOwned` / `+followerOwned` tiles claim the querier.
// Haxe: Connection.sendOwners L1236–1258
pub fn send_owners_query(
    state: &mut crate::SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    client_x: i32,
    client_y: i32,
    world_x: i32,
    world_y: i32,
) {
    let Some(p) = state.players.get(&conn_id) else {
        return;
    };
    if p.deleted {
        return;
    }
    let p_id = p.p_id;
    let obj_id = match state.world.read() {
        Ok(w) => w.get_object(world_x, world_y),
        Err(_) => return,
    };
    if obj_id == 0 {
        return;
    }
    let desc = state
        .content
        .get(obj_id)
        .map(|d| {
            if d.description.is_empty() {
                d.name.clone()
            } else {
                d.description.clone()
            }
        })
        .unwrap_or_default();
    let is_owned = ol_world::object_data_is_owned(&desc);
    let owners = {
        let mut world = match state.world.write() {
            Ok(w) => w,
            Err(_) => return,
        };
        let mut helper = world
            .get_helper(world_x, world_y)
            .cloned()
            .unwrap_or_else(|| ComplexObject::new_simple(obj_id));
        if helper.living_owners.is_empty() {
            if !is_owned {
                return;
            }
            add_owner_to_helper(&mut helper, p_id);
            world.set_object_complex(world_x, world_y, helper.clone());
        }
        helper.living_owners.clone()
    };
    outbound.send(
        conn_id,
        format_owner_list(client_x, client_y, &owners).into_bytes(),
    );
}

/// Place grave (if resolvable) then leave remaining death loot for scatter.
///
/// When `outbound` is `Some`, fans out Haxe `GRAVE` to connected clients.
pub fn place_grave_on_death(
    state: &mut SimState,
    outbound: Option<&OutboundHub>,
    conn_id: u64,
) -> Option<PlaceGraveResult> {
    let res = place_grave_for_conn(state, conn_id)?;
    if let Some(ob) = outbound {
        send_grave_info_to_all(state, ob, res.x, res.y, res.creator_p_id);
    }
    Some(res)
}

/// Pid variant of [`place_grave_on_death`].
pub fn place_grave_on_death_pid(
    state: &mut SimState,
    outbound: Option<&OutboundHub>,
    p_id: i32,
) -> Option<PlaceGraveResult> {
    let conn = state
        .players
        .iter()
        .find(|(_, pl)| pl.p_id == p_id)
        .map(|(&c, _)| c)?;
    place_grave_on_death(state, outbound, conn)
}

/// Haxe `doDeathHelper` L4001–4015: drop a held player; if **this** body was held,
/// detach and put Baby Bones in the carrier's hands (Haxe TODO L4014).
// Haxe: GlobalPlayerInstance.doDeathHelper L4001-4015
pub fn apply_death_hold_detach(state: &mut SimState, deceased_p_id: i32) {
    let (holding_id, held_by, dx, dy) = match state.players.values().find(|p| p.p_id == deceased_p_id)
    {
        Some(p) => (p.holding_player_id, p.held_by, p.x, p.y),
        None => return,
    };
    // Dying carrier: dropPlayer onto the death tile (Haxe dropPlayer(this.x, this.y)).
    if holding_id != 0 {
        if let Some(p) = state.players.values_mut().find(|p| p.p_id == deceased_p_id) {
            let _ = p.release_holding();
        }
        if let Some(baby) = state
            .players
            .values_mut()
            .find(|p| p.p_id == holding_id && !p.deleted)
        {
            baby.held_by = 0;
            baby.x = dx;
            baby.y = dy;
        }
    }
    // Dying held body: detach, then Baby Bones 1920 in carrier arms (not empty).
    if held_by != 0 {
        if let Some(p) = state.players.values_mut().find(|p| p.p_id == deceased_p_id) {
            p.held_by = 0;
        }
        if let Some(carrier) = state
            .players
            .values_mut()
            .find(|p| p.p_id == held_by && !p.deleted)
        {
            if carrier.holding_player_id == deceased_p_id {
                let _ = carrier.release_holding();
            }
            carrier.set_held(BABY_BONES_ID, 0);
        }
    }
}

/// Next Eve origin from a death tile (Haxe `ServerSettings.startingGx/Gy`).
///
/// Session-only (Haxe mutates statics; not written back to ServerSettings.txt).
/// [`SimState::spawn_x`] / [`SimState::spawn_y`] is the Eve fallback.
// Haxe: GlobalPlayerInstance.doDeathHelper L3995–3996
pub fn stamp_starting_gx_from_death(state: &mut SimState, death_tx: i32, death_ty: i32) {
    state.spawn_x = death_tx;
    state.spawn_y = death_ty;
}

/// Full Haxe-shaped death inheritance polish for one deceased player.
///
/// Call after the body is marked deleted and (when applicable) after the grave
/// object is placed on the death tile.
///
/// // Haxe: GlobalPlayerInstance.doDeathHelper
pub fn apply_death_polish(state: &mut SimState, deceased_p_id: i32) {
    // Haxe L3995–3996 reads this.tx/ty before dropPlayer / held detach.
    let death_origin = state
        .players
        .values()
        .find(|p| p.p_id == deceased_p_id)
        .map(|p| (p.x, p.y));
    apply_death_hold_detach(state, deceased_p_id);
    let (deceased_email, death_xy, follow_leader, owning_tiles, age_years, true_age_years, death_reason, killed_by) = {
        let p = state.players.values().find(|p| p.p_id == deceased_p_id);
        let email = p
            .map(|p| p.email.clone())
            .unwrap_or_else(|| format!("pid{deceased_p_id}@inherit.local"));
        let xy = death_origin.or_else(|| p.map(|p| (p.x, p.y)));
        let follow = state.social.following.get(&deceased_p_id).copied();
        let owning = p.map(|p| p.owning.clone()).unwrap_or_default();
        let age = p.map(|p| p.age).unwrap_or(0.0);
        let true_age = p.map(|p| p.true_age).unwrap_or(0.0);
        let reason = p
            .and_then(|p| p.death_reason.clone())
            .unwrap_or_default();
        let killed_by = p
            .map(|p| p.last_player_attacked_me_id)
            .unwrap_or(0);
        (email, xy, follow, owning, age, true_age, reason, killed_by)
    };
    // LINEAGE-24H / REP / COINS: Haxe doDeathHelper deathTime + reputation + coins
    // snapshot *before* InheritCoins.
    // Haxe: GPI.doDeathHelper L3977–3983
    let lost_combat = state
        .combat
        .stats
        .get(&deceased_p_id)
        .map(|s| s.lost_combat_prestige)
        .unwrap_or(0.0);
    let coins = state.economy.coins_of(deceased_p_id) as f32;
    state.social.stamp_lineage_death(
        deceased_p_id,
        state.sim_time,
        &death_reason,
        age_years,
        true_age_years,
        lost_combat,
        coins,
        killed_by,
    );

    // ── ChooseNewLeader (Haxe doDeathHelper before placeGrave) ─────────────
    // Haxe: GlobalPlayerInstance.ChooseNewLeader / countLeadershipPower
    {
        let mut power: HashMap<i32, f32> = HashMap::new();
        for pl in state.players.values() {
            if pl.deleted || pl.p_id == deceased_p_id {
                continue;
            }
            let prestige = state
                .social
                .lineages
                .get(&pl.p_id)
                .map(|n| n.prestige)
                .or_else(|| state.combat.stats.get(&pl.p_id).map(|s| s.prestige))
                .unwrap_or(0.0);
            let class = state.social.prestige_class(pl.p_id);
            let coins = state.economy.coins_of(pl.p_id);
            // Haxe: account.familyPrestige[lineage.myEveId]
            let founder = root_eve_id(&state.social, pl.p_id);
            let family_prestige = state.accounts.family_prestige_for(&pl.email, founder);
            power.insert(
                pl.p_id,
                count_leadership_power(prestige, coins, family_prestige, class),
            );
        }
        let succ = choose_new_leader(
            deceased_p_id,
            follow_leader,
            &mut state.social.following,
            &state.social.exiles,
            &power,
        );
        if let Some(line) = format_leader_succession_event(deceased_p_id, &succ) {
            state.push_event(line);
        }
    }

    // STARTING-GX-DEATH: after ChooseNewLeader, before placeGrave/dropPlayer.
    // Haxe: GlobalPlayerInstance.doDeathHelper L3995–3996
    if let Some((gx, gy)) = death_xy {
        stamp_starting_gx_from_death(state, gx, gy);
    }

    // ── InheritOwnership (Haxe player.owning + world helpers scan) ─────────
    // Haxe: GlobalPlayerInstance.InheritOwnership
    {
        let mut world = state.world.write().unwrap();
        let mut owned_keys: HashSet<(i32, i32)> = world
            .helpers
            .iter()
            .filter(|(_, h)| h.is_owner(deceased_p_id))
            .map(|(&k, _)| k)
            .collect();
        for tile in &owning_tiles {
            owned_keys.insert(*tile);
        }
        let mut transfers = Vec::new();
        for (x, y) in owned_keys {
            let Some(h) = world.helpers.get_mut(&(x, y)) else {
                continue;
            };
            if !remove_owner_from_helper(h, deceased_p_id) {
                continue;
            }
            let mut new_owner = 0i32;
            if h.living_owners.is_empty() {
                if let Some(lid) = follow_leader.filter(|&id| id != 0 && id != deceased_p_id) {
                    add_owner_to_helper(h, lid);
                    new_owner = lid;
                }
            }
            transfers.push(OwnershipTransfer { x, y, new_owner });
        }
        drop(world);

        // Refresh Player.owning: clear deceased; add transferred tiles to new owner.
        if let Some(p) = state.players.values_mut().find(|p| p.p_id == deceased_p_id) {
            p.owning.clear();
        }
        for t in &transfers {
            if t.new_owner != 0 {
                if let Some(p) = state.players.values_mut().find(|p| p.p_id == t.new_owner) {
                    if !p.owning.contains(&(t.x, t.y)) {
                        p.owning.push((t.x, t.y));
                    }
                }
            }
        }
        for line in format_ownership_events(deceased_p_id, &transfers) {
            state.push_event(line);
        }
    }

    // Living = not deleted and not the deceased (deceased may already be marked deleted).
    let living: HashMap<i32, bool> = state
        .players
        .values()
        .filter(|p| p.p_id != deceased_p_id && !p.deleted)
        .map(|p| (p.p_id, true))
        .collect();
    let social = state.social.clone();
    let allies = state.allies.clone();

    // Death-tile grave helper for residual coins (Haxe TODO "store coins in grave").
    // Prefer SpecialKind::Grave (any select id: 87/752/3053) over name-resolved id alone.
    let grave_xy = death_xy.filter(|&(x, y)| {
        if state.specials.kind_at(x, y) == Some(SpecialKind::Grave) {
            return true;
        }
        let gid = state.grave_object_id;
        gid != 0 && state.world.read().unwrap().get_object(x, y) == gid
    });

    let mut grave_helper = grave_xy.and_then(|(x, y)| {
        let w = state.world.read().unwrap();
        w.get_helper(x, y).cloned().or_else(|| {
            let id = w.get_object(x, y);
            if id != 0 {
                Some(ComplexObject::new_simple(id))
            } else {
                None
            }
        })
    });
    if let Some(ref mut g) = grave_helper {
        let aid = crate::death_inherit::account_id_ensure(&mut state.accounts, &deceased_email);
        stamp_grave_soul(g, deceased_p_id, aid);
    }

    let mut ctx = InheritContext {
        deceased_p_id,
        deceased_email: &deceased_email,
        living: &living,
        social: &social,
        allies: &allies,
        accounts: &mut state.accounts,
        economy: &mut state.economy,
        scoreboard: &mut state.scoreboard,
        grave: grave_helper.as_mut(),
        // C-SS-MORE-BATCH4: live InheritCoinsFactor
        inherit_coins_factor: state.gameplay.inherit_coins_factor,
    };
    let transfers = apply_inherit_coins(&mut ctx);
    for line in format_inherit_events(deceased_p_id, &transfers) {
        state.push_event(line);
    }
    if let (Some((x, y)), Some(g)) = (grave_xy, grave_helper) {
        state.world.write().unwrap().set_object_complex(x, y, g);
    }

    // Haxe: ScoreEntry.CreateScoreEntryForDeadRelative(this)
    apply_dead_relative_score_entry(state, deceased_p_id, &deceased_email);
}

/// Haxe `ScoreEntry.CreateScoreEntryForDeadRelative` live wire.
/// // Haxe: ScoreEntry.CreateScoreEntryForDeadRelative
fn apply_dead_relative_score_entry(state: &mut SimState, deceased_p_id: i32, deceased_email: &str) {
    let prestige = state
        .social
        .lineages
        .get(&deceased_p_id)
        .map(|n| n.prestige)
        .or_else(|| state.combat.stats.get(&deceased_p_id).map(|s| s.prestige))
        .unwrap_or(0.0);
    let (name, family, mother_id) = {
        let p = state.players.values().find(|p| p.p_id == deceased_p_id);
        let name = p
            .map(|p| {
                if p.first_name.is_empty() {
                    p.name_for_say()
                } else {
                    p.first_name.clone()
                }
            })
            .or_else(|| {
                state
                    .social
                    .lineages
                    .get(&deceased_p_id)
                    .map(|n| n.name.clone())
            })
            .unwrap_or_else(|| format!("P{deceased_p_id}"));
        let family = p
            .map(|p| p.family_name.clone())
            .unwrap_or_else(|| "SNOW".into());
        let mother = state
            .social
            .lineages
            .get(&deceased_p_id)
            .and_then(|n| n.mother_id);
        (name, family, mother)
    };
    let player = DeadRelativePlayer {
        p_id: deceased_p_id,
        account_email: deceased_email.to_string(),
        prestige,
        name,
        family_name: family,
        mother_lineage_id: mother_id,
    };

    // Snapshot lineage + account emails for pure walk.
    let lineages: HashMap<i32, (Option<i32>, String)> = state
        .social
        .lineages
        .iter()
        .map(|(&id, n)| {
            let email = email_for_lineage_id(state, id);
            (id, (n.mother_id, email))
        })
        .collect();
    // Haxe Lineage.get_grave: creatorId == ancestor.myId, then !isBoneGrave.
    // // Haxe: Lineage.get_grave L674-678 + ScoreEntry L79
    let grave_for_creator: HashMap<i32, bool> = {
        let world = state.world.read().unwrap();
        let mut m = HashMap::new();
        for (&id, (_mother, email)) in &lineages {
            let has = state
                .accounts
                .get(email)
                .map(|rec| {
                    rec.graves.iter().any(|&(x, y)| {
                        let obj_id = world.get_object(x, y);
                        if obj_id <= 0 || is_bone_grave(obj_id) {
                            return false;
                        }
                        let creator = world
                            .get_helper(x, y)
                            .and_then(ol_world::helper_creator_player_id)
                            .or_else(|| {
                                world
                                    .get_helper(x, y)
                                    .map(|h| h.owner_id)
                                    .filter(|&c| c != 0)
                            })
                            .unwrap_or(0);
                        creator == id
                    })
                })
                .unwrap_or(false);
            m.insert(id, has);
        }
        m
    };

    let mut seed = deceased_p_id
        .wrapping_mul(1103515245)
        .wrapping_add(state.sim_time.to_bits() as i32);
    let entry = create_score_entry_for_dead_relative(
        &player,
        &|id| {
            let (mother_id, email) = lineages.get(&id)?;
            Some(MotherLineNode {
                player_id: id,
                account_email: email.clone(),
                has_non_bone_grave: grave_for_creator.get(&id).copied().unwrap_or(false),
                mother_id: *mother_id,
            })
        },
        || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let u = (seed as u32 >> 8) as f32 / 16_777_216.0;
            u.clamp(0.0, 0.999_999)
        },
        state.gameplay.ancestor_prestige_factor,
    );
    if let Some(e) = entry {
        state.accounts.push_score_entry(e);
        state.push_event(format!("{deceased_p_id} SCORE_ENTRY ancestor award"));
    }
}

fn email_for_lineage_id(state: &SimState, p_id: i32) -> String {
    if let Some(p) = state.players.values().find(|p| p.p_id == p_id) {
        if !p.email.is_empty() {
            return p.email.clone();
        }
    }
    for r in state.accounts.by_email.values() {
        if r.last_p_id == p_id {
            return r.email.clone();
        }
    }
    format!("pid{p_id}@inherit.local")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::{ContentDb, ObjectDef, Transition};
    use std::sync::Arc;

    fn def(id: i32, name: &str, desc: &str, containable: bool, permanent: bool) -> ObjectDef {
        let mut o = ObjectDef::empty(id);
        o.name = name.into();
        o.description = desc.into();
        o.containable = containable;
        o.permanent = permanent;
        o
    }

    #[test]
    fn grave_old_name_and_dead_since() {
        assert_eq!(grave_old_underscored_name("", "Commoner"), "~");
        assert_eq!(
            grave_old_underscored_name("Ada Snow", "Commoner"),
            "Ada_Snow_Commoner"
        );
        assert_eq!(grave_dead_since_years(180.0, 0.0), 3);
        assert_eq!(grave_dead_since_years(60.0, 60.0), 0);
    }

    /// BABY-BONES-ARMS: held death puts Baby Bones 1920 in the carrier's hands.
    // Haxe: GPI.doDeathHelper L4014 TODO
    #[test]
    fn held_baby_death_places_bones_in_carrier_arms() {
        let mut state = SimState::with_default_empty(Arc::new(ContentDb::default()));
        {
            use crate::player::Player;
            let mut mom = Player::new(1, 1, "mom@bones");
            mom.holding_player_id = 2;
            mom.x = 3;
            mom.y = 4;
            state.players.insert(1, mom);
            let mut baby = Player::new(2, 2, "baby@bones");
            baby.held_by = 1;
            baby.age = 0.5;
            baby.deleted = true;
            baby.x = 3;
            baby.y = 4;
            state.players.insert(2, baby);
        }
        apply_death_polish(&mut state, 2);
        let mom = state.players.get(&1).unwrap();
        assert_eq!(mom.held_id, BABY_BONES_ID);
        assert_eq!(mom.holding_player_id, 0);
        assert_eq!(state.players.get(&2).unwrap().held_by, 0);
    }

    /// BABY-BONES-ARMS: unheld death must not hand bones to a bystander.
    #[test]
    fn unheld_death_does_not_give_bystander_bones() {
        let mut state = SimState::with_default_empty(Arc::new(ContentDb::default()));
        {
            use crate::player::Player;
            let mut by = Player::new(1, 1, "by@bones");
            by.x = 3;
            by.y = 4;
            state.players.insert(1, by);
            let mut other = Player::new(2, 2, "die@bones");
            other.deleted = true;
            other.x = 3;
            other.y = 4;
            state.players.insert(2, other);
        }
        apply_death_polish(&mut state, 2);
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
        assert_eq!(state.players.get(&1).unwrap().holding_player_id, 0);
    }

    /// STARTING-GX-DEATH: death tile becomes next Eve `spawn_x/y` (Haxe startingGx/Gy).
    // Haxe: GPI.doDeathHelper L3995–3996
    #[test]
    fn death_stamps_starting_gx_gy_for_next_eve() {
        let mut state = SimState::with_default_empty(Arc::new(ContentDb::default()));
        state.spawn_x = 235;
        state.spawn_y = 150;
        {
            use crate::player::Player;
            let mut p = Player::new(2, 2, "die@gx");
            p.deleted = true;
            p.x = 12;
            p.y = 34;
            state.players.insert(2, p);
        }
        apply_death_polish(&mut state, 2);
        assert_eq!(state.spawn_x, 12, "startingGx = death tx");
        assert_eq!(state.spawn_y, 34, "startingGy = death ty");
    }

    #[test]
    fn select_grave_baby_adult_murder() {
        assert_eq!(select_grave_object_id(0.5, false), BABY_BONE_PILE_ID);
        assert_eq!(select_grave_object_id(2.9, true), BABY_BONE_PILE_ID);
        assert_eq!(select_grave_object_id(3.0, false), FRESH_GRAVE_ID);
        assert_eq!(select_grave_object_id(20.0, true), MURDER_GRAVE_ID);
        assert_eq!(select_grave_object_id(20.0, false), FRESH_GRAVE_ID);
    }

    #[test]
    fn select_grave_live_min_age_override() {
        // age 4 < live min 5 → baby; age 4 >= default 3 → adult if const used
        assert_eq!(
            select_grave_object_id_with_min_age(4.0, false, 5.0),
            BABY_BONE_PILE_ID
        );
        assert_eq!(
            select_grave_object_id_with_min_age(4.0, true, 5.0),
            BABY_BONE_PILE_ID
        );
        assert_eq!(
            select_grave_object_id_with_min_age(5.0, true, 5.0),
            MURDER_GRAVE_ID
        );
    }

    #[test]
    fn place_grave_live_min_age_baby_threshold() {
        let mut db = ContentDb::default();
        db.objects
            .insert(3053, def(3053, "Baby Bone Pile", "bones", false, true));
        db.objects
            .insert(87, def(87, "Fresh Grave", "+origGrave", false, true));
        let mut state = SimState::with_default_empty(Arc::new(db));
        // Live MinAgeToEat = 5 → age 4 is still baby bone pile
        state.gameplay.min_age_to_eat = 5.0;
        {
            use crate::player::Player;
            let mut p = Player::new(1, 4, "live@min");
            p.p_id = 4;
            p.age = 4.0;
            p.x = 1;
            p.y = 1;
            p.connected = true;
            state.players.insert(1, p);
        }
        let res = place_grave_for_player(&mut state, 4).expect("baby grave");
        assert_eq!(res.grave_id, 3053);
        // age 5 with min 5 → adult fresh
        state.players.get_mut(&1).unwrap().age = 5.0;
        state.players.get_mut(&1).unwrap().x = 2;
        state.players.get_mut(&1).unwrap().y = 2;
        let res2 = place_grave_for_player(&mut state, 4).expect("adult grave");
        assert_eq!(res2.grave_id, 87);
    }

    #[test]
    fn is_wound_description_matches_haxe() {
        assert!(is_wound_description("Arrow Wound"));
        assert!(is_wound_description("Snake Bite"));
        assert!(is_wound_description("Hog Cut"));
        assert!(!is_wound_description("Gooseberry"));
        assert!(!is_wound_description("Fresh Grave"));
    }

    #[test]
    fn resolve_place_prefers_haxe_id_when_in_content() {
        let mut db = ContentDb::default();
        db.objects
            .insert(87, def(87, "Fresh Grave", "grave", false, true));
        db.objects
            .insert(752, def(752, "Murder Grave", "grave", false, true));
        db.objects
            .insert(3053, def(3053, "Baby Bone Pile", "bones", false, true));
        assert_eq!(resolve_place_grave_id(&db, 1.0, false), 3053);
        assert_eq!(resolve_place_grave_id(&db, 20.0, true), 752);
        assert_eq!(resolve_place_grave_id(&db, 20.0, false), 87);
    }

    #[test]
    fn resolve_place_falls_back_to_name_grave() {
        let mut db = ContentDb::default();
        db.objects
            .insert(77, def(77, "Grave", "stone grave", false, true));
        assert_eq!(resolve_place_grave_id(&db, 20.0, false), 77);
        assert_eq!(resolve_place_grave_id(&db, 1.0, false), 77);
    }

    #[test]
    fn resolve_place_empty_content_is_zero() {
        let db = ContentDb::default();
        assert_eq!(resolve_place_grave_id(&db, 20.0, false), 0);
    }

    #[test]
    fn place_grave_packs_clothing_and_containable_held() {
        let mut db = ContentDb::default();
        db.objects
            .insert(87, def(87, "Fresh Grave", "+origGrave", false, true));
        db.objects.insert(33, def(33, "Berry", "food", true, false));
        db.objects
            .insert(40, def(40, "Hat", "clothing", true, false));
        let mut state = SimState::with_default_empty(Arc::new(db));
        let p_id = {
            use crate::player::Player;
            let mut p = Player::new(1, 7, "pack@grave");
            p.p_id = 7;
            p.age = 20.0;
            p.x = 10;
            p.y = 11;
            p.held_id = 33;
            p.hat = 40;
            p.chest = 41;
            p.connected = true;
            state.players.insert(1, p);
            7
        };
        let res = place_grave_for_player(&mut state, p_id).expect("grave");
        assert_eq!(res.grave_id, 87);
        assert_eq!(res.x, 10);
        assert_eq!(res.y, 11);
        assert!(res.contained.contains(&33), "held containable in grave");
        assert!(res.contained.contains(&40), "hat in grave");
        assert!(res.contained.contains(&41), "chest in grave");
        let g = state
            .world
            .read()
            .unwrap()
            .get_helper(10, 11)
            .cloned()
            .expect("helper");
        assert_eq!(g.base_id, 87);
        assert!(g.contained.contains(&33));
        assert!(g.contained.contains(&40));
        let pl = state.players.get(&1).unwrap();
        assert_eq!(pl.held_id, 0);
        assert_eq!(pl.hat, 0);
        assert!(state.event_log.iter().any(|e| e == "GRAVE 10 11 7"));
    }

    #[test]
    fn place_grave_murder_skips_wound_in_contained() {
        let mut db = ContentDb::default();
        db.objects
            .insert(752, def(752, "Murder Grave", "+origGrave", false, true));
        db.objects
            .insert(87, def(87, "Fresh Grave", "+origGrave", false, true));
        db.objects
            .insert(560, def(560, "Knife Wound", "Knife Wound", false, false));
        let mut state = SimState::with_default_empty(Arc::new(db));
        {
            use crate::player::Player;
            let mut p = Player::new(1, 9, "murder@x");
            p.p_id = 9;
            p.age = 25.0;
            p.x = 2;
            p.y = 3;
            p.held_id = 560;
            p.connected = true;
            state.players.insert(1, p);
        }
        let res = place_grave_for_player(&mut state, 9).expect("murder grave");
        assert_eq!(res.grave_id, 752);
        assert!(!res.contained.contains(&560), "wound must not enter grave");
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
        assert_eq!(state.world.read().unwrap().get_object(2, 3), 752);
    }

    #[test]
    fn place_grave_baby_bone_pile() {
        let mut db = ContentDb::default();
        db.objects
            .insert(3053, def(3053, "Baby Bone Pile", "bones", false, true));
        let mut state = SimState::with_default_empty(Arc::new(db));
        {
            use crate::player::Player;
            let mut p = Player::new(1, 3, "baby@x");
            p.p_id = 3;
            p.age = 1.5;
            p.x = 0;
            p.y = 0;
            p.connected = true;
            state.players.insert(1, p);
        }
        let res = place_grave_for_player(&mut state, 3).expect("baby grave");
        assert_eq!(res.grave_id, 3053);
    }

    #[test]
    fn held_death_transition_places_new_target() {
        let mut db = ContentDb::default();
        db.objects
            .insert(87, def(87, "Fresh Grave", "+origGrave", false, true));
        // Rope animal 126 → ground 127, actor 0 (cleared).
        db.objects
            .insert(126, def(126, "Domestic Cow", "cow +rope", false, false));
        db.objects
            .insert(127, def(127, "Domestic Cow", "cow", false, true));
        db.transitions.insert(
            (126, -1),
            Transition {
                actor_id: 126,
                target_id: -1,
                new_actor_id: 0,
                new_target_id: 127,
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
                hungry_work_cost: 0.0,
                hungry_work_temperature: -1.0,
                coin_cost: 0,
                is_forbidden: false,
            },
        );
        let mut state = SimState::with_default_empty(Arc::new(db));
        {
            use crate::player::Player;
            let mut p = Player::new(1, 5, "rope@x");
            p.p_id = 5;
            p.age = 20.0;
            p.x = 8;
            p.y = 8;
            p.held_id = 126;
            p.connected = true;
            state.players.insert(1, p);
        }
        let res = place_grave_for_player(&mut state, 5).expect("grave");
        assert_eq!(res.grave_id, 87);
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
        // newTarget cow on death tile or ring (grave may take death tile).
        let w = state.world.read().unwrap();
        let mut found_cow = w.get_object(8, 8) == 127;
        for dy in -2..=2 {
            for dx in -2..=2 {
                if w.get_object(8 + dx, 8 + dy) == 127 {
                    found_cow = true;
                }
            }
        }
        // Cow may have been swallowed into grave if non-permanent — 127 is permanent.
        assert!(
            found_cow
                || w.get_helper(res.x, res.y)
                    .map(|g| g.contained.contains(&127))
                    .unwrap_or(false),
            "rope death transition should place newTarget 127"
        );
    }

    #[test]
    fn format_grave_place_log_shape() {
        assert_eq!(format_grave_place_log(1, 2, 7), "GRAVE 1 2 7");
    }
}
