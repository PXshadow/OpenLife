//! Haxe `ObjectHelper.InitObjectHelpersAfterRead` → sim wire (**NESTED-OLW1-POLISH postload_wire**).
//!
//! After OLW + OLA1 load (and when players are present), rewire:
//! - grave helpers → `AccountRecord.graves` (via `account_soul_token` / `owners_by_account`)
//! - owned helpers (`+owned`) → `Player.owning` tile list + prune dead/missing `living_owners`
//! - deleted owners → Haxe `removeOwner` (strip account tokens too)
//! - creator lineages → `LineageNode.owns_object = true`
//!
//! Pure core lives in `ol_world::postload_owners`. This module applies results to
//! [`SimState`] accounts, players, world helpers, and [`SpecialIndex`] graves.
//!
//! ## Intentional delta
//! Haxe with `LoadPlayers=false` prunes every living owner (empty player map).
//! Rust does not persist players across restarts; cold boot with an empty player
//! map **keeps** disk `living_owners` so mid-session ownership survives a world
//! reload that is not accompanied by a player wipe. When any players are loaded,
//! missing/deleted ids are pruned exactly like Haxe.

use crate::death_inherit::account_soul_token;
use crate::mutation::SpecialKind;
use crate::SimState;
use ol_world::{
    init_object_helpers_after_read, name_looks_like_grave, ComplexObject, GraveAccountLink,
    LivingOwnerStatus, PlayerOwningLink,
};
use std::collections::HashMap;
use tracing::info;


// CONTAINED-TIMERS-PERSIST pure helpers (NestedHelper slots ↔ runtime map)
#[path = "contained_timers_persist.rs"]
mod contained_timers_inner;

/// Rebuild [`crate::WorldMapTimeState::contained_timers`] after OLW load.
///
/// Haxe restores `ObjectHelper.creationTimeInTicks` + `timeToChange` on each
/// contained object via disk. Rust stores those on NestedHelper slots (OLW3) and
/// re-arms the runtime parallel map here (chunk **CONTAINED-TIMERS-PERSIST** /
/// `rearm_after_load`).
pub fn arm_contained_timers_for_loaded_world(state: &mut SimState) {
    let world = state.world.read().unwrap();
    let sim_time = state.sim_time;
    let map = contained_timers_inner::rebuild_contained_timers_from_world(&world, sim_time);
    let stats = contained_timers_inner::rearm_stats(&world, &map);
    drop(world);
    state.world_map_time.contained_timers = map;
    if stats.tiles > 0 {
        info!(
            tiles = stats.tiles,
            slots = stats.slots,
            persisted_ttc = stats.with_persisted_ttc,
            "contained timers re-armed after load"
        );
    }
}

/// Stats from one post-load rewire pass (logging / tests).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PostloadWireStats {
    pub helpers_scanned: usize,
    pub graves_linked: usize,
    pub owning_linked: usize,
    pub owners_pruned: usize,
    pub lineages_marked: usize,
}

/// Content + Haxe `ObjectData.isGrave` (`origGrave` in description) flag for one base id.
pub fn content_grave_meta(state: &SimState, base_id: i32) -> (String, String, bool) {
    if let Some(def) = state.content.get(base_id) {
        let is_grave = description_is_orig_grave(&def.description)
            || name_looks_like_grave(&def.name, &def.description);
        (def.name.clone(), def.description.clone(), is_grave)
    } else {
        (String::new(), String::new(), false)
    }
}

/// Haxe `ObjectData.isGrave` — description contains `origGrave`.
#[inline]
pub fn description_is_orig_grave(description: &str) -> bool {
    description.to_ascii_lowercase().contains("origgrave")
}

/// Build account-soul-token → email keys for grave rewire.
pub fn account_token_index(state: &SimState) -> HashMap<i32, String> {
    let mut map = HashMap::new();
    for email in state.accounts.by_email.keys() {
        let tok = account_soul_token(email);
        if tok != 0 {
            map.insert(tok, email.clone());
        }
    }
    map
}

/// Living-player predicate for Haxe prune (with Rust cold-boot delta).
///
/// * Known player + not deleted → alive  
/// * Known player + deleted → dead  
/// * Unknown id + **no** players loaded → keep (see module intentional delta)  
/// * Unknown id + some players loaded → missing (Haxe prune)
pub fn player_alive_for_postload(state: &SimState, p_id: i32) -> bool {
    if p_id == 0 {
        return false;
    }
    if let Some(p) = state.players.values().find(|p| p.p_id == p_id) {
        return !p.deleted;
    }
    state.players.is_empty()
}

/// Haxe living-owner status for postload (Alive / Deleted / Missing / Keep).
pub fn player_status_for_postload(state: &SimState, p_id: i32) -> LivingOwnerStatus {
    if p_id == 0 {
        return LivingOwnerStatus::Missing;
    }
    if let Some(p) = state.players.values().find(|p| p.p_id == p_id) {
        if p.deleted {
            return LivingOwnerStatus::Deleted;
        }
        return LivingOwnerStatus::Alive;
    }
    // Cold boot (no players): keep disk living_owners.
    if state.players.is_empty() {
        LivingOwnerStatus::Keep
    } else {
        LivingOwnerStatus::Missing
    }
}

/// Account soul token for a living player id (email → token), if known.
pub fn account_token_for_player(state: &SimState, p_id: i32) -> Option<i32> {
    let email = state
        .players
        .values()
        .find(|p| p.p_id == p_id)
        .map(|p| p.email.as_str())?;
    let tok = account_soul_token(email);
    if tok == 0 {
        None
    } else {
        Some(tok)
    }
}

/// Apply one grave account link into the account book + specials index.
pub fn apply_grave_account_link(
    state: &mut SimState,
    token_to_email: &HashMap<i32, String>,
    link: &GraveAccountLink,
) -> bool {
    let Some(email) = token_to_email.get(&link.account_id) else {
        return false;
    };
    let rec = state.accounts.ensure(email);
    let tile = (link.tx, link.ty);
    if !rec.graves.contains(&tile) {
        rec.graves.push(tile);
    }
    state
        .specials
        .insert(link.tx, link.ty, SpecialKind::Grave);
    true
}

/// Apply one player owning link into `Player.owning`.
pub fn apply_player_owning_link(state: &mut SimState, link: &PlayerOwningLink) -> bool {
    let Some(p) = state
        .players
        .values_mut()
        .find(|p| p.p_id == link.player_id && !p.deleted)
    else {
        return false;
    };
    let tile = (link.tx, link.ty);
    if !p.owning.contains(&tile) {
        p.owning.push(tile);
    }
    true
}

/// Haxe `Lineage.ownsObject = true` for creator lineages with map objects.
pub fn apply_lineage_owns_object(state: &mut SimState, creator_player_id: i32) -> bool {
    if creator_player_id == 0 {
        return false;
    }
    // Ensure node exists when lineage book already has the creator (post-load
    // players/lineages may already be present; do not invent names here).
    if let Some(n) = state.social.lineages.get_mut(&creator_player_id) {
        if !n.owns_object {
            n.owns_object = true;
            return true;
        }
        return false;
    }
    false
}

/// Write pruned helper snapshot back into the world map.
fn write_helpers_back(state: &mut SimState, helpers: Vec<(i32, i32, ComplexObject)>) {
    let mut world = state.world.write().unwrap();
    for (tx, ty, co) in helpers {
        if co.base_id == 0 {
            continue;
        }
        world.set_object_complex(tx, ty, co);
    }
}

/// Haxe `ObjectHelper.InitObjectHelpersAfterRead` against current world + accounts + players.
///
/// Call after OLA1/OLW seed at sim boot, and optionally after bulk player load.
pub fn apply_init_object_helpers_after_read(state: &mut SimState) -> PostloadWireStats {
    let mut helpers: Vec<(i32, i32, ComplexObject)> = {
        let world = state.world.read().unwrap();
        world
            .helpers
            .iter()
            .map(|(&(tx, ty), h)| (tx, ty, h.clone()))
            .collect()
    };
    let helpers_scanned = helpers.len();
    if helpers_scanned == 0 {
        arm_contained_timers_for_loaded_world(state);
        return PostloadWireStats::default();
    }

    // Snapshot status without holding player map during pure rewire.
    let status_map: HashMap<i32, LivingOwnerStatus> = state
        .players
        .values()
        .map(|p| {
            (
                p.p_id,
                if p.deleted {
                    LivingOwnerStatus::Deleted
                } else {
                    LivingOwnerStatus::Alive
                },
            )
        })
        .collect();
    let any_players = !status_map.is_empty();

    // p_id → account soul token (for removeOwner on deleted).
    let token_by_pid: HashMap<i32, i32> = state
        .players
        .values()
        .filter_map(|p| {
            let tok = account_soul_token(&p.email);
            if tok != 0 {
                Some((p.p_id, tok))
            } else {
                None
            }
        })
        .collect();

    // Content name/desc/is_grave snapshot (base_id → meta).
    let mut name_cache: HashMap<i32, (String, String, bool)> = HashMap::new();
    for (_, _, co) in &helpers {
        if co.base_id != 0 && !name_cache.contains_key(&co.base_id) {
            name_cache.insert(co.base_id, content_grave_meta(state, co.base_id));
        }
    }

    let before_owners: Vec<Vec<i32>> = helpers
        .iter()
        .map(|(_, _, h)| h.living_owners.clone())
        .collect();

    let (graves, owning, lineage_links) = init_object_helpers_after_read(
        &mut helpers,
        |base_id| {
            name_cache
                .get(&base_id)
                .cloned()
                .unwrap_or_else(|| (String::new(), String::new(), false))
        },
        |p_id| {
            if p_id == 0 {
                return LivingOwnerStatus::Missing;
            }
            if let Some(&st) = status_map.get(&p_id) {
                return st;
            }
            // Cold boot (no players): keep disk living_owners.
            if !any_players {
                LivingOwnerStatus::Keep
            } else {
                LivingOwnerStatus::Missing
            }
        },
        |p_id| token_by_pid.get(&p_id).copied(),
    );

    let mut owners_pruned = 0usize;
    for (i, (_, _, h)) in helpers.iter().enumerate() {
        if h.living_owners != before_owners[i] {
            owners_pruned += before_owners[i]
                .iter()
                .filter(|id| !h.living_owners.contains(id))
                .count();
        }
    }

    write_helpers_back(state, helpers);

    let token_to_email = account_token_index(state);
    let mut graves_linked = 0usize;
    for g in &graves {
        if apply_grave_account_link(state, &token_to_email, g) {
            graves_linked += 1;
        }
    }

    let mut owning_linked = 0usize;
    for o in &owning {
        if apply_player_owning_link(state, o) {
            owning_linked += 1;
        }
    }

    // Haxe: creatorLinage.ownsObject = true for helpers with getLinage().
    let mut lineages_marked = 0usize;
    for l in &lineage_links {
        if apply_lineage_owns_object(state, l.creator_player_id) {
            lineages_marked += 1;
        }
    }

    let stats = PostloadWireStats {
        helpers_scanned,
        graves_linked,
        owning_linked,
        owners_pruned,
        lineages_marked,
    };
    info!(
        helpers = stats.helpers_scanned,
        graves = stats.graves_linked,
        owning = stats.owning_linked,
        pruned = stats.owners_pruned,
        lineages = stats.lineages_marked,
        "postload: InitObjectHelpersAfterRead applied"
    );
    // Contained ObjectHelper timers (OLW3 slots → runtime map).
    arm_contained_timers_for_loaded_world(state);
    stats
}

/// Rebuild `Player.owning` for one living player from current world helpers
/// (session spawn / reconnect when p_id may still appear on disk helpers).
pub fn rebuild_player_owning_from_world(state: &mut SimState, p_id: i32) {
    if p_id == 0 {
        return;
    }
    let tiles: Vec<(i32, i32)> = {
        let world = state.world.read().unwrap();
        world
            .helpers
            .iter()
            .filter(|(_, h)| h.is_owner(p_id))
            .map(|(&(tx, ty), _)| (tx, ty))
            .collect()
    };
    if let Some(p) = state.players.values_mut().find(|p| p.p_id == p_id) {
        p.owning = tiles;
    }
}

/// Rebuild all `AccountRecord.graves` from world helpers (token match only).
///
/// Used when accounts load after world, or to refresh without full owner prune.
pub fn rebuild_account_graves_from_world(state: &mut SimState) -> usize {
    for rec in state.accounts.by_email.values_mut() {
        rec.graves.clear();
    }
    let links: Vec<GraveAccountLink> = {
        let world = state.world.read().unwrap();
        let mut out = Vec::new();
        for (&(tx, ty), h) in &world.helpers {
            if h.base_id == 0 || h.owners_by_account.is_empty() {
                continue;
            }
            let (name, desc, content_grave) = content_grave_meta(state, h.base_id);
            let is_grave = content_grave || name_looks_like_grave(&name, &desc);
            if !is_grave {
                continue;
            }
            for &aid in &h.owners_by_account {
                if aid != 0 {
                    out.push(GraveAccountLink {
                        account_id: aid,
                        tx,
                        ty,
                        base_id: h.base_id,
                    });
                }
            }
        }
        out
    };
    let token_to_email = account_token_index(state);
    let mut linked = 0usize;
    for g in &links {
        if apply_grave_account_link(state, &token_to_email, g) {
            linked += 1;
        }
    }
    linked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::AccountBook;
    use crate::death_inherit::{account_soul_token, stamp_grave_soul};
    use crate::player::Player;
    use crate::social::LineageNode;
    use ol_content::{ContentDb, ObjectDef};
    use ol_world::{ComplexObject, World};
    use std::sync::{Arc, RwLock};

    fn state_with_grave_content() -> SimState {
        let mut db = ContentDb::default();
        let mut grave = ObjectDef::empty(87);
        grave.name = "Fresh Grave".into();
        grave.description = "Fresh Grave +origGrave".into();
        db.objects.insert(87, grave);
        let mut chest = ObjectDef::empty(100);
        chest.name = "Chest".into();
        // Haxe isOwned requires +owned
        chest.description = "Chest +owned".into();
        db.objects.insert(100, chest);
        let mut box_ = ObjectDef::empty(50);
        box_.name = "Box".into();
        box_.description = "Box +owned".into();
        db.objects.insert(50, box_);
        // Non-owned content (living_owners on disk must not rewire).
        let mut plain = ObjectDef::empty(33);
        plain.name = "Basket".into();
        plain.description = "Basket".into();
        db.objects.insert(33, plain);
        SimState::new(
            Arc::new(RwLock::new(World::new(32, 32, true))),
            Arc::new(db),
        )
    }

    #[test]
    fn description_orig_grave() {
        assert!(description_is_orig_grave("Stone +origGrave"));
        assert!(!description_is_orig_grave("Basket"));
    }

    #[test]
    fn cold_boot_preserves_living_owners_links_graves() {
        let mut state = state_with_grave_content();
        let email = "hero@test.com";
        state.accounts.ensure(email);
        let token = account_soul_token(email);

        // Owned chest with living owner 7 (no player loaded).
        let mut chest = ComplexObject::new_simple(100);
        chest.owner_id = 7;
        chest.living_owners = vec![7, 8];
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(2, 3, chest);

        // Grave stamped with account soul token.
        let mut grave = ComplexObject::new_simple(87);
        stamp_grave_soul(&mut grave, 42, email);
        assert!(grave.owners_by_account.contains(&token));
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(5, 6, grave);

        let stats = apply_init_object_helpers_after_read(&mut state);
        assert_eq!(stats.helpers_scanned, 2);
        // Cold boot: living owners kept (no prune of 7/8).
        let w = state.world.read().unwrap();
        let chest = w.get_helper(2, 3).unwrap();
        assert_eq!(chest.living_owners, vec![7, 8]);
        drop(w);

        let rec = state.accounts.get(email).unwrap();
        assert!(
            rec.graves.contains(&(5, 6)),
            "grave must link to account: {:?}",
            rec.graves
        );
        assert_eq!(stats.graves_linked, 1);
        assert_eq!(state.specials.kind_at(5, 6), Some(SpecialKind::Grave));
    }

    #[test]
    fn with_players_prunes_missing_and_fills_owning() {
        let mut state = state_with_grave_content();
        // Player 1 alive; player 2 deleted; 9 never exists.
        let mut p1 = Player::new(1, 10, "a@b.c");
        p1.deleted = false;
        state.players.insert(10, p1);
        let mut p2 = Player::new(2, 20, "c@d.e");
        p2.deleted = true;
        state.players.insert(20, p2);

        let mut co = ComplexObject::new_simple(50);
        co.owner_id = 1;
        co.living_owners = vec![1, 2, 9];
        // Account tokens for owners (soul of each email)
        co.owners_by_account = vec![
            account_soul_token("a@b.c"),
            account_soul_token("c@d.e"),
        ];
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(1, 1, co);

        let stats = apply_init_object_helpers_after_read(&mut state);
        let w = state.world.read().unwrap();
        let h = w.get_helper(1, 1).unwrap();
        assert_eq!(h.living_owners, vec![1]);
        assert_eq!(h.owner_id, 1);
        // Deleted p_id=2 → removeOwner strips their account token
        assert_eq!(h.owners_by_account, vec![account_soul_token("a@b.c")]);
        drop(w);

        let p1 = state.players.get(&10).unwrap();
        assert!(p1.owning.contains(&(1, 1)));
        assert!(stats.owning_linked >= 1);
        assert!(stats.owners_pruned >= 2);
    }

    #[test]
    fn grave_with_living_owners_not_pruned() {
        let mut state = state_with_grave_content();
        let mut p1 = Player::new(1, 10, "a@b.c");
        p1.deleted = false;
        state.players.insert(10, p1);
        // Missing player 99 on grave living_owners — Haxe does not prune graves.
        let mut grave = ComplexObject::new_simple(87);
        grave.living_owners = vec![1, 99];
        grave.owner_id = 1;
        stamp_grave_soul(&mut grave, 1, "a@b.c");
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(2, 2, grave);

        let _ = apply_init_object_helpers_after_read(&mut state);
        let w = state.world.read().unwrap();
        let h = w.get_helper(2, 2).unwrap();
        assert_eq!(h.living_owners, vec![1, 99]);
    }

    #[test]
    fn without_owned_tag_skips_owning_link() {
        let mut state = state_with_grave_content();
        let mut p1 = Player::new(1, 10, "a@b.c");
        p1.deleted = false;
        state.players.insert(10, p1);
        // Base 33 is Basket without +owned
        let mut co = ComplexObject::new_simple(33);
        co.owner_id = 1;
        co.living_owners = vec![1, 9];
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(3, 3, co);

        let stats = apply_init_object_helpers_after_read(&mut state);
        let w = state.world.read().unwrap();
        let h = w.get_helper(3, 3).unwrap();
        assert_eq!(h.living_owners, vec![1, 9], "no +owned → no prune");
        drop(w);
        assert!(!state.players.get(&10).unwrap().owning.contains(&(3, 3)));
        assert_eq!(stats.owning_linked, 0);
    }

    #[test]
    fn marks_lineage_owns_object() {
        let mut state = state_with_grave_content();
        let mut p1 = Player::new(1, 10, "a@b.c");
        p1.deleted = false;
        state.players.insert(10, p1);
        state.social.lineages.insert(1, LineageNode::eve(1, "Hero"));

        let mut co = ComplexObject::new_simple(50);
        co.owner_id = 1;
        co.living_owners = vec![1];
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(0, 0, co);

        let stats = apply_init_object_helpers_after_read(&mut state);
        assert!(
            state.social.lineages.get(&1).unwrap().owns_object,
            "creator lineage must be marked owns_object"
        );
        assert!(stats.lineages_marked >= 1);
    }

    #[test]
    fn rebuild_player_owning_scans_helpers() {
        let mut state = state_with_grave_content();
        let p = Player::new(3, 30, "x@y.z");
        state.players.insert(30, p);

        let co = ComplexObject::with_owner(33, 3);
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(4, 5, co);

        rebuild_player_owning_from_world(&mut state, 3);
        assert_eq!(state.players.get(&30).unwrap().owning, vec![(4, 5)]);
    }

    #[test]
    fn account_token_index_roundtrip() {
        let mut book = AccountBook::default();
        book.ensure("Ada@X.com");
        let mut state = state_with_grave_content();
        state.accounts = book;
        let idx = account_token_index(&state);
        let tok = account_soul_token("ada@x.com");
        assert_eq!(idx.get(&tok).map(|s| s.as_str()), Some("ada@x.com"));
    }

    #[test]
    fn rebuild_account_graves_only() {
        let mut state = state_with_grave_content();
        let email = "g@h.i";
        state.accounts.ensure(email);
        let mut grave = ComplexObject::new_simple(87);
        stamp_grave_soul(&mut grave, 1, email);
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(8, 9, grave);
        let n = rebuild_account_graves_from_world(&mut state);
        assert_eq!(n, 1);
        assert!(state.accounts.get(email).unwrap().graves.contains(&(8, 9)));
    }

    #[test]
    fn player_status_deleted_vs_missing() {
        let mut state = state_with_grave_content();
        let mut p = Player::new(5, 50, "d@e.f");
        p.deleted = true;
        state.players.insert(50, p);
        assert_eq!(
            player_status_for_postload(&state, 5),
            LivingOwnerStatus::Deleted
        );
        assert_eq!(
            player_status_for_postload(&state, 99),
            LivingOwnerStatus::Missing
        );
        state.players.clear();
        assert_eq!(
            player_status_for_postload(&state, 99),
            LivingOwnerStatus::Keep
        );
    }
    #[test]
    fn rearm_contained_timers_from_olw3_slots() {
        let mut state = state_with_grave_content();
        {
            let mut world = state.world.write().unwrap();
            let mut h = ComplexObject::new_simple(391);
            h.contained = vec![50];
            let mut slot = ol_world::NestedHelper::id_only(50);
            slot.creation_time = 20.0;
            slot.time_to_change = 40.0;
            h.slots = vec![slot];
            world.set_object_complex(3, 4, h);
        }
        assert!(state.world_map_time.contained_timers.is_empty());
        // sim_time after creation so ReadFromFile clamp does not rewrite progress.
        state.sim_time = 100.0;
        arm_contained_timers_for_loaded_world(&mut state);
        let ts = state
            .world_map_time
            .contained_timers
            .get(&(3, 4))
            .expect("re-armed tile");
        assert_eq!(ts, &[(20.0, 40.0)]);
    }


}
