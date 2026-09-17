// AI-ATTACK-PLAYER: live getWeapon + range walk + kill from profession_scan.rs

use crate::attack_player::{
    attack_player, deadly_distance_for_held, AttackPlayerAction, AttackPlayerClothing,
    AttackPlayerInput, AttackPlayerTarget, GetWeaponAction, WeaponTile, GET_WEAPON_DROP_HOME,
    MIN_AI_AGE_FOR_COMBAT, WEAPON_SEARCH_DIST,
};
use crate::relations::is_ally;
use ol_ai::{
    get_close_deadly_player, get_close_player_target, CloseDeadlyPlayer, DeadlyPlayerCandidate,
    PlayerTargetCandidate, DEVIL_MASK_ID, GOBLIN_MASK_ID, PLAYER_TARGET_SEARCH_DIST,
};
use ol_net::NetIntent;

/// Map [`GetWeaponAction`] / [`AttackPlayerAction`] to a live intent.
// Haxe: AiBase.attackPlayer / getWeapon → USE / DROP / SELF / goto / kill
pub fn attack_player_action_to_live_intent(
    action: AttackPlayerAction,
    held_id: i32,
) -> ShortCraftLiveIntent {
    match action {
        AttackPlayerAction::None => ShortCraftLiveIntent::None,
        AttackPlayerAction::Goto { x, y } => ShortCraftLiveIntent::Goto { x, y },
        AttackPlayerAction::Kill {
            target_p_id,
            tx,
            ty,
        } => ShortCraftLiveIntent::Kill {
            target_p_id,
            x: tx,
            y: ty,
        },
        AttackPlayerAction::GetWeapon(gw) => match gw {
            GetWeaponAction::None => ShortCraftLiveIntent::None,
            GetWeaponAction::GoHome { x, y } => ShortCraftLiveIntent::Goto { x, y },
            GetWeaponAction::Wait => ShortCraftLiveIntent::Wait,
            GetWeaponAction::SelfClothing { slot } => ShortCraftLiveIntent::SelfClothing { slot },
            GetWeaponAction::DropHeld => ShortCraftLiveIntent::DropAt { x: 0, y: 0 },
            GetWeaponAction::Pickup { x, y, id } => {
                if held_id == 0 {
                    ShortCraftLiveIntent::UseAt {
                        x,
                        y,
                        target_id: id,
                        actor_id: 0,
                    }
                } else {
                    ShortCraftLiveIntent::DropAt { x, y }
                }
            }
            GetWeaponAction::SeekOrCraft { actor } => ShortCraftLiveIntent::SeekOrCraft {
                actor,
                craft_if_needed: true,
            },
        },
    }
}

fn clothing_from_player(p: &crate::Player) -> AttackPlayerClothing {
    let ids = p.clothing_parent_ids();
    let uses = p.clothing_uses_remaining();
    let q = crate::quiver_from_clothing_snapshot(&ids, &uses);
    AttackPlayerClothing {
        arrow_quiver: q.arrow_quiver,
        quiver_with_bow: q.quiver_with_bow,
        empty_quiver_with_bow: q.empty_quiver_with_bow,
        can_add_to_quiver: q.can_add,
    }
}

fn weapon_tiles_from_scan(tiles: &[ScanTile]) -> Vec<WeaponTile> {
    tiles
        .iter()
        .filter(|t| t.parent_id != 0)
        .map(|t| (t.parent_id, t.x, t.y, t.is_permanent))
        .collect()
}

fn player_is_wounded(state: &crate::SimState, p: &crate::Player) -> bool {
    p.is_wounded_held(is_wound_object(&state.content, p.held_id))
}

/// Closest Haxe `playerTarget` = deadlyPlayer else GetClosePlayerTarget.
// Haxe: AiBase.doTimeStuffHelper ~589
pub fn combat_player_target_from_state(
    state: &crate::SimState,
    conn_id: u64,
) -> Option<AttackPlayerTarget> {
    let self_p = state.players.get(&conn_id)?;
    let self_id = self_p.p_id;
    let self_x = self_p.x;
    let self_y = self_p.y;
    let angry = self_p.angry_time;
    let home_x = self_p.home_x;
    let home_y = self_p.home_y;
    let clothing = self_p.clothing_parent_ids();
    let has_red = clothing.contains(&DEVIL_MASK_ID);
    let has_blue = clothing.contains(&GOBLIN_MASK_ID);
    let age = self_p.age;

    let deleted: std::collections::HashSet<i32> = state
        .players
        .values()
        .filter(|p| p.deleted)
        .map(|p| p.p_id)
        .collect();
    let mut deadly_cands: Vec<DeadlyPlayerCandidate> = Vec::new();
    let mut mask_cands: Vec<PlayerTargetCandidate> = Vec::new();
    for o in state.players.values() {
        if o.conn_id == conn_id || o.p_id == 0 || o.deleted {
            continue;
        }
        let name = state
            .content
            .get(o.held_id)
            .map(|d| d.name.as_str())
            .unwrap_or("");
        let holding = crate::is_holding_weapon(o.held_id, name);
        let attacked = self_p.last_attacked_player_id == o.p_id
            || self_p.last_player_attacked_me_id == o.p_id
            || o.last_attacked_player_id == self_id
            || o.last_player_attacked_me_id == self_id;
        let bloody = crate::weapons::is_bloody_weapon(o.held_id)
            || name.to_ascii_lowercase().contains("bloody");
        let lost = state
            .combat
            .stats
            .get(&o.p_id)
            .map(|s| s.lost_combat_prestige)
            .unwrap_or(0.0);
        deadly_cands.push(DeadlyPlayerCandidate {
            p_id: o.p_id,
            x: o.x,
            y: o.y,
            deleted: o.deleted,
            age: o.age,
            angry_time: o.angry_time,
            lost_combat_prestige: lost,
            is_cursed: o.is_cursed,
            is_ai: crate::ai_takeover::player_is_ai(o.connected, o.ai_controlled, &o.email),
            holding_weapon: holding,
            held_is_bloody: bloody,
            exiled_by_observer_leaders: false,
            is_friendly: !attacked,
        });
        mask_cands.push(PlayerTargetCandidate {
            p_id: o.p_id,
            x: o.x,
            y: o.y,
            deleted: o.deleted,
            age: o.age,
            is_same_family: false,
            is_ally: is_ally(
                &state.social.following,
                &state.social,
                &deleted,
                self_id,
                o.p_id,
            ),
            is_top_leader: crate::leadership::direct_follow_leader(
                &state.social.following,
                o.p_id,
            )
            .is_none(),
            lost_combat_prestige: lost,
            is_cursed: o.is_cursed,
            target_home_quad: {
                let dx = o.x - home_x;
                let dy = o.y - home_y;
                (dx * dx + dy * dy) as f32
            },
        });
    }

    let hit = get_close_deadly_player(
        self_x,
        self_y,
        angry,
        home_x,
        home_y,
        ol_ai::DEADLY_PLAYER_SEARCH_DIST_AI,
        &deadly_cands,
    )
    .or_else(|| {
        get_close_player_target(
            has_red,
            has_blue,
            self_p.dark_nosaj,
            age,
            self_x,
            self_y,
            PLAYER_TARGET_SEARCH_DIST,
            &mask_cands,
        )
        .map(|t| CloseDeadlyPlayer {
            p_id: t.p_id,
            x: t.x,
            y: t.y,
            dist: t.dist,
            angry_time: 0.0,
            lost_combat_prestige: 0.0,
        })
    })?;

    let other = state.players.values().find(|p| p.p_id == hit.p_id && !p.deleted)?;
    let (ex, ey) = other.exact_xy();
    Some(AttackPlayerTarget {
        p_id: other.p_id,
        x: other.x,
        y: other.y,
        exact_x: ex,
        exact_y: ey,
        wounded: player_is_wounded(state, other),
    })
}

/// Plan `attackPlayer` from a world scan (no mutation).
// Haxe: doStuff && attackPlayer(playerTarget)
pub fn attack_player_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    target: Option<AttackPlayerTarget>,
) -> ProfessionScanTickResult {
    let nearby = weapon_tiles_from_scan(tiles);
    let parent = inp.held_id;
    let content_dd = inp
        .content
        .as_ref()
        .and_then(|c| c.get(inp.held_id).map(|d| d.deadly_distance))
        .unwrap_or(0.0);
    let held_name = inp
        .content
        .as_ref()
        .and_then(|c| c.get(inp.held_id).map(|d| d.name.clone()))
        .unwrap_or_default();
    let plan_inp = AttackPlayerInput {
        target,
        food_store: inp.food_store,
        self_wounded: false,
        age: inp.age,
        min_ai_age_for_combat: MIN_AI_AGE_FOR_COMBAT,
        holding_weapon: crate::is_holding_weapon(inp.held_id, &held_name),
        held_id: inp.held_id,
        held_parent_id: parent,
        is_moving: inp.is_moving,
        player_x: inp.player_x,
        player_y: inp.player_y,
        exact_x: inp.player_x as f64,
        exact_y: inp.player_y as f64,
        home_x: inp.home_x,
        home_y: inp.home_y,
        deadly_distance: deadly_distance_for_held(inp.held_id, content_dd),
        clothing: AttackPlayerClothing::from_ids(&inp.clothing),
        weapon_tiles: &nearby,
    };
    let action = attack_player(&plan_inp);
    if !action.is_some() {
        return ProfessionScanTickResult::none();
    }
    let mut intent = attack_player_action_to_live_intent(action, inp.held_id);
    if matches!(
        action,
        AttackPlayerAction::GetWeapon(GetWeaponAction::DropHeld)
    ) {
        intent = super::smart_drop_held_profession_ex_content(
            tiles,
            inp.held_id,
            inp.held_uses,
            inp.player_x,
            inp.player_y,
            inp.home_x,
            inp.home_y,
            inp.food_store,
            false,
            GET_WEAPON_DROP_HOME,
            inp.held_contains_clay,
            inp.is_moving,
            &inp.clothing,
            &inp.clothing_uses,
            inp.content.as_deref(),
        );
    }
    ProfessionScanTickResult {
        had_action: !matches!(intent, ShortCraftLiveIntent::None),
        intent,
    }
}

/// Live Combat rung: getWeapon + stand-off walk + `KILL`.
// Haxe: AiBase.doTimeStuffHelper ~591 attackPlayer
pub fn apply_attack_player_tick(
    state: &mut crate::SimState,
    outbound: &OutboundHub,
    conn_id: u64,
) -> ShortCraftLiveApplyResult {
    let target = combat_player_target_from_state(state, conn_id);
    let Some(p) = state.players.get(&conn_id) else {
        return ShortCraftLiveApplyResult::Failed;
    };
    let px = p.x;
    let py = p.y;
    let held_id = p.held_id;
    let held_uses = p.held_uses;
    let food = p.food;
    let age = p.age;
    let moving = p.moving || p.move_path.is_some();
    let clothing = p.clothing_parent_ids();
    let clothing_uses = p.clothing_uses_remaining();
    let clay = held_contains_clay_from_player(p);
    let self_wounded = player_is_wounded(state, p);
    let (ex, ey) = p.exact_xy();
    let home_x = if p.home_x != 0 || p.home_y != 0 {
        p.home_x
    } else {
        p.x
    };
    let home_y = if p.home_x != 0 || p.home_y != 0 {
        p.home_y
    } else {
        p.y
    };
    let cloth_flags = clothing_from_player(p);
    let parent = state
        .content
        .dummy_parent
        .get(&held_id)
        .copied()
        .unwrap_or(held_id);
    let held_name = state
        .content
        .get(held_id)
        .map(|d| d.name.clone())
        .unwrap_or_default();
    let content_dd = state
        .content
        .get(held_id)
        .map(|d| d.deadly_distance)
        .unwrap_or(0.0);
    let holding_weapon = crate::is_holding_weapon(held_id, &held_name);
    let deadly_distance = deadly_distance_for_held(held_id, content_dd);
    let (cx, cy) = p.world_to_client(
        target.map(|t| t.x).unwrap_or(px),
        target.map(|t| t.y).unwrap_or(py),
    );
    let tiles = {
        let w = state.world.read().unwrap();
        scan_world_radius(&w, Some(&state.content), px, py, WEAPON_SEARCH_DIST)
    };
    let nearby = weapon_tiles_from_scan(&tiles);
    let plan_inp = AttackPlayerInput {
        target,
        food_store: food,
        self_wounded,
        age,
        min_ai_age_for_combat: MIN_AI_AGE_FOR_COMBAT,
        holding_weapon,
        held_id,
        held_parent_id: parent,
        is_moving: moving,
        player_x: px,
        player_y: py,
        exact_x: ex,
        exact_y: ey,
        home_x,
        home_y,
        deadly_distance,
        clothing: cloth_flags,
        weapon_tiles: &nearby,
    };
    let action = attack_player(&plan_inp);
    if !action.is_some() {
        return ShortCraftLiveApplyResult::Failed;
    }
    if matches!(
        action,
        AttackPlayerAction::GetWeapon(GetWeaponAction::DropHeld)
    ) {
        let intent = super::smart_drop_held_profession_ex_content(
            &tiles,
            held_id,
            held_uses,
            px,
            py,
            home_x,
            home_y,
            food,
            false,
            GET_WEAPON_DROP_HOME,
            clay,
            moving,
            &clothing,
            &clothing_uses,
            Some(&state.content),
        );
        return apply_short_craft_live_intent(state, outbound, conn_id, intent);
    }
    let intent = attack_player_action_to_live_intent(action, held_id);
    if let ShortCraftLiveIntent::Kill { target_p_id, x, y } = intent {
        let (kx, ky) = state
            .players
            .get(&conn_id)
            .map(|pl| pl.world_to_client(x, y))
            .unwrap_or((cx, cy));
        crate::apply_intent(
            state,
            &ol_metrics::Counters::new(),
            outbound,
            NetIntent::Raw {
                conn_id,
                tag: "KILL".into(),
                payload: format!("{kx} {ky} {target_p_id}"),
            },
        );
        if let Some(pl) = state.players.get_mut(&conn_id) {
            pl.ai_did_not_reach_food = 0.0;
        }
        return ShortCraftLiveApplyResult::Dropped;
    }
    let r = apply_short_craft_live_intent(state, outbound, conn_id, intent);
    if matches!(action, AttackPlayerAction::Goto { .. })
        && !matches!(r, ShortCraftLiveApplyResult::Failed)
    {
        // Haxe L5862: if (done) didNotReachAnimalTarget = 0
        if let Some(pl) = state.players.get_mut(&conn_id) {
            pl.ai_did_not_reach_animal_target = 0;
        }
    }
    r
}
