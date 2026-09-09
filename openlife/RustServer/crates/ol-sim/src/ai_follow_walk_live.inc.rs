// AI-FOLLOW-WALK live pathfind + continuous tick (included from lib.rs)
// + AI-FOLLOW-ACQUIRE empty-sticky child-mother / AutoFollowPlayer closest
// Haxe: AiBase.isMovingToPlayer / startFollowingPlayer Goto / ally Goto(speaker)

/// Process deferred `orderedToDrop` (Haxe AI frame start before escape/jobs).
// Haxe: AiBase.doTimeStuffHelper orderedToDrop L481–484
// AI-SAY-HELPER / AI-LLM-APPLY
fn tick_ordered_ai_drop(state: &mut SimState, outbound: &OutboundHub) {
    // Snapshot then clear flag first (Haxe: orderedToDrop = false; dropHeldObject(0))
    let jobs: Vec<(u64, i32, i32)> = state
        .players
        .iter_mut()
        .filter(|(_, p)| !p.deleted && p.ai_ordered_to_drop)
        .map(|(&cid, p)| {
            p.ai_ordered_to_drop = false;
            (cid, p.x, p.y, p.held_id)
        })
        .filter(|(_, _, _, held)| *held != 0)
        .map(|(cid, x, y, _)| (cid, x, y))
        .collect();
    for (cid, x, y) in jobs {
        apply_drop(state, outbound, cid, x, y, None);
    }
}

/// Pathfind AI toward absolute goal and start timed MOVE path (follow / ally Goto).
// Haxe: myPlayer.gotoAdv / Goto → MoveHelper path
// AI-FOLLOW-WALK
fn try_ai_follow_path_to(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    goal_x: i32,
    goal_y: i32,
) -> bool {
    let (sx, sy, p_id, moving) = match state.players.get(&conn_id) {
        Some(p) if !p.deleted => (p.x, p.y, p.p_id, p.moving || p.move_path.is_some()),
        _ => return false,
    };
    if sx == goal_x && sy == goal_y {
        return false;
    }
    // Continuous tick: do not thrash mid-path; repath when stationary.
    if moving {
        return true;
    }
    let content = state.content.clone();
    let allies = state.allies.clone();
    let steps = {
        let world = state.world.read().unwrap();
        find_path_new(
            sx,
            sy,
            goal_x,
            goal_y,
            &|x, y| {
                is_walkable_for_player(&world, &content, x, y, p_id, &|a, b| {
                    allies.is_mutual_or_either(a, b)
                })
            },
            pathfind::GOTO_COLLISION_RAD,
        )
    };
    let Some(steps) = steps else {
        return false;
    };
    if steps.is_empty() {
        return false;
    }
    let capped = truncate_follow_path_steps(&steps, FOLLOW_PATH_STEP_CAP);
    let deltas = steps_to_client_path_deltas(&capped);
    match apply_move_path_start(state, outbound, conn_id, sx, sy, &deltas, None) {
        Ok(()) => true,
        Err(_) => {
            let dx = (goal_x - sx).signum();
            let dy = (goal_y - sy).signum();
            let step = if dx != 0 { (dx, 0) } else { (0, dy) };
            if step.0 == 0 && step.1 == 0 {
                return false;
            }
            apply_move_path_start(state, outbound, conn_id, sx, sy, &[step], None).is_ok()
        }
    }
}

/// True when this body is driven by AI (permanent NPC, takeover, selfplay).
// Haxe: myPlayer.isAi() / ServerAi body
fn player_is_ai_follow_body(p: &Player) -> bool {
    p.ai_controlled
        || email_looks_ai(&p.email)
        || p.email.to_ascii_lowercase().contains("npc")
        || p.email.to_ascii_lowercase().contains("selfplay")
}

/// Empty-sticky acquire: child-mother `getFollowPlayer` or AutoFollowPlayer closest.
// Haxe: AiBase.isMovingToPlayer playerToFollow == null branch L8287–8296
// AI-FOLLOW-ACQUIRE / auto_follow
fn tick_ai_follow_acquire(state: &mut SimState, auto_follow_player_enabled: bool) {
    use crate::ai_llm_apply::ai_follow_walk::{
        resolve_auto_follow_acquire_ex, AutoFollowCandidate, AUTO_FOLLOW_PLAYER_DEFAULT,
    };
    let _ = AUTO_FOLLOW_PLAYER_DEFAULT; // documented default; live passes explicit flag

    let ai_jobs: Vec<(u64, i32, i32, i32, f32)> = state
        .players
        .iter()
        .filter(|(_, p)| {
            !p.deleted
                && p.ai_follow_p_id <= 0
                && player_is_ai_follow_body(p)
                && state.social.hired_boss(p.p_id) == 0
        })
        .map(|(c, p)| (*c, p.p_id, p.x, p.y, p.age))
        .collect();
    if ai_jobs.is_empty() {
        return;
    }

    // Leadership followPlayer map + deleted lookup for mother gate.
    let following = state.social.following.clone();
    let deleted_by_pid: std::collections::HashMap<i32, bool> = state
        .players
        .values()
        .map(|p| (p.p_id, p.deleted))
        .collect();

    // Closest-player candidates only needed when AutoFollowPlayer is on.
    let candidates: Vec<AutoFollowCandidate> = if auto_follow_player_enabled {
        state
            .players
            .values()
            .filter(|p| !p.deleted && p.p_id > 0)
            .map(|p| AutoFollowCandidate {
                p_id: p.p_id,
                x: p.x,
                y: p.y,
                // Haxe: Connection.getConnections = humans; getAis = AI bodies
                is_human: p.is_human_body(),
                deleted: false,
            })
            .collect()
    } else {
        Vec::new()
    };

    for (conn_id, ai_p_id, ax, ay, age) in ai_jobs {
        let leader = direct_follow_leader(&following, ai_p_id);
        let leader_deleted = leader
            .map(|id| deleted_by_pid.get(&id).copied().unwrap_or(true))
            .unwrap_or(true);
        // Haxe isMovingToPlayer(…, followHuman: true) default
        let only_human = true;
        let plan = resolve_auto_follow_acquire_ex(
            0,
            age,
            ax,
            ay,
            ai_p_id,
            leader,
            leader_deleted,
            auto_follow_player_enabled,
            only_human,
            &candidates,
            state.gameplay.min_age_to_eat,
        );
        if let Some(acq) = plan {
            if let Some(p) = state.players.get_mut(&conn_id) {
                // Loose follow (autoStopFollow stays default true) — Haxe just assigns
                p.ai_follow_p_id = acq.follow_p_id;
                let _ = acq.source; // ChildMother | ClosestPlayer (audit / future say)
            }
        }
    }
}

/// Haxe `searchNewHomeIfNeeded` — silent home reassignment on AI think tick.
/// Always side-effect only (Haxe returns false; does not consume the think).
// Haxe: AiBase.doTimeStuffHelper L601; searchNewHomeIfNeeded ~8174–8196
// AI-HOME-TICK
fn tick_search_new_home_if_needed(state: &mut SimState) {
    use crate::do_commands_wire::collect_home_search_ovens;
    use crate::speech::{
        apply_new_home_if_needed, count_home_population, search_new_home_if_needed_ex,
        AI_MIGRATE_VILLAGE_POPULATION_SIZE, HOME_POP_MAX_AGE_MINUS_2, HOME_POP_STARVING_FACTOR,
        HOME_SEARCH_MAX_QUAD,
    };

    let ai_ids: Vec<u64> = state
        .players
        .iter()
        .filter(|(_, p)| !p.deleted && player_is_ai_follow_body(p))
        .filter(|(_, p)| !p.moving && p.move_path.is_none())
        .map(|(&c, _)| c)
        .collect();
    if ai_ids.is_empty() {
        return;
    }

    let oven_tiles: Vec<(i32, i32)> = state.world_map_time.ovens.values().copied().collect();
    let min_age = state.gameplay.min_age_to_eat;
    let peers: Vec<(i32, i32, f32, f32, bool)> = state
        .players
        .values()
        .filter(|p| player_is_ai_follow_body(p))
        .map(|p| (p.home_x, p.home_y, p.age, p.food, p.deleted))
        .collect();

    let global_rows = if oven_tiles.is_empty() {
        None
    } else if let Ok(w) = state.world.read() {
        Some(collect_home_search_ovens(
            &w,
            &oven_tiles,
            &state.world_map_time.original_biomes,
            0,
            0,
        ))
    } else {
        Some(Vec::new())
    };

    for conn_id in ai_ids {
        let Some(p) = state.players.get(&conn_id) else {
            continue;
        };
        let (sx, sy, hx, hy, food) = (p.x, p.y, p.home_x, p.home_y, p.food);
        let home_obj_id = match state.world.read() {
            Ok(w) => w.get_object(hx, hy),
            Err(_) => 0,
        };
        let pop = count_home_population(
            hx,
            hy,
            &peers,
            min_age,
            HOME_POP_MAX_AGE_MINUS_2,
            HOME_POP_STARVING_FACTOR,
        );
        let ovens = match &global_rows {
            Some(rows) => rows.clone(),
            None => match state.world.read() {
                Ok(w) => collect_home_search_ovens(
                    &w,
                    &[],
                    &state.world_map_time.original_biomes,
                    sx,
                    sy,
                ),
                Err(_) => Vec::new(),
            },
        };
        let (map_w, map_h) = match state.world.read() {
            Ok(w) if w.wrap => (w.width_tiles, w.height_tiles),
            _ => (0, 0),
        };
        let new_home = search_new_home_if_needed_ex(
            food,
            pop,
            home_obj_id,
            AI_MIGRATE_VILLAGE_POPULATION_SIZE,
            sx,
            sy,
            &ovens,
            HOME_SEARCH_MAX_QUAD,
            map_w,
            map_h,
        );
        if let Some(p) = state.players.get_mut(&conn_id) {
            apply_new_home_if_needed(&mut p.home_x, &mut p.home_y, new_home);
        }
    }
}

/// Live Haxe `DoNaming` I AM family (shared by SAY and `tick_found_family`).
// Haxe: NamingHelper.DoNaming L40–173 (AI-NAMING-IAM)
fn apply_do_naming_iam_live(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    text: &str,
) -> crate::naming::DoNamingIam {
    use crate::naming::{
        plan_do_naming_iam_ex, should_migrate_found_family_follower, DoNamingIam,
        NAMING_FOUND_FAMILY_COST,
    };
    use crate::relations::{is_close_relative, is_same_family};

    let Some((self_p_id, current_family, display_object_id, hx, hy, email)) = (|| {
        let p = state.players.get(&conn_id)?;
        if p.deleted {
            return None;
        }
        Some((
            p.p_id,
            p.family_name.clone(),
            p.display_object_id,
            p.home_x,
            p.home_y,
            p.email.clone(),
        ))
    })() else {
        return DoNamingIam::NotNaming;
    };
    let is_founder = state
        .social
        .lineages
        .get(&self_p_id)
        .map(|n| n.is_family_founder())
        .unwrap_or(false);
    let eve_id = state
        .social
        .lineages
        .get(&self_p_id)
        .map(|n| n.family_eve_id())
        .filter(|&e| e > 0)
        .unwrap_or_else(|| crate::relations::root_eve_id(&state.social, self_p_id));
    let same_account = eve_id != self_p_id
        && state
            .players
            .values()
            .any(|o| o.p_id == eve_id && o.email == email);
    let prestige = state
        .social
        .lineages
        .get(&self_p_id)
        .map(|n| n.prestige)
        .or_else(|| state.combat.stats.get(&self_p_id).map(|s| s.prestige))
        .unwrap_or(0.0);
    let coins = state.economy.coins_of(self_p_id) as f32;
    let same_family_followers = state
        .players
        .values()
        .filter(|o| {
            !o.deleted
                && o.p_id != self_p_id
                && state.social.is_follower_from(o.p_id, self_p_id)
                && is_same_family(&state.social, o.p_id, self_p_id)
        })
        .count() as i32;
    let used: Vec<String> = state
        .players
        .values()
        .filter(|o| !o.deleted)
        .map(|o| o.family_name.clone())
        .collect();
    let used_refs: Vec<&str> = used.iter().map(|s| s.as_str()).collect();
    let plan = plan_do_naming_iam_ex(
        text,
        &current_family,
        is_founder,
        same_account,
        prestige,
        same_family_followers,
        coins,
        &used_refs,
        &state.gameplay.starting_family_name,
        state.gameplay.found_family_needed_prestige,
        state.gameplay.found_family_cost,
        state.gameplay.found_family_needed_followers,
    );
    match &plan {
        DoNamingIam::NotNaming => {}
        DoNamingIam::Reject { private_say } => {
            send_ps_reply(outbound, conn_id, &format!("{self_p_id} {private_say}"));
        }
        DoNamingIam::Apply {
            family_name,
            found_new,
        } => {
            let family_name = family_name.clone();
            let found_new = *found_new;
            let self_color = state.content.person_color(display_object_id);
            let mut rename: Vec<i32> = Vec::new();
            if found_new {
                // Haxe: dynasty = myDynastyId < 1 ? old Eve : myDynastyId
                // Haxe: NamingHelper.DoNaming L135–141
                let old_founder = state
                    .social
                    .lineages
                    .get(&self_p_id)
                    .map(|n| {
                        if n.my_eve_id > 0 {
                            n.my_eve_id
                        } else {
                            n.family_eve_id()
                        }
                    })
                    .filter(|&e| e > 0)
                    .unwrap_or(eve_id);
                let existing_dynasty = state
                    .social
                    .lineages
                    .get(&self_p_id)
                    .map(|n| n.my_dynasty_id)
                    .unwrap_or(-1);
                let dynasty = if existing_dynasty < 1 {
                    old_founder
                } else {
                    existing_dynasty
                };
                let cands: Vec<(i32, bool, bool, bool)> = state
                    .players
                    .values()
                    .filter(|o| !o.deleted && o.p_id != self_p_id)
                    .filter(|o| {
                        state.social.is_follower_from(o.p_id, self_p_id)
                            && is_same_family(&state.social, o.p_id, self_p_id)
                    })
                    .map(|o| {
                        let close = is_close_relative(&state.social, o.p_id, self_p_id);
                        let same_home = o.home_x == hx && o.home_y == hy;
                        let diff_color =
                            state.content.person_color(o.display_object_id) != self_color;
                        (o.p_id, close, same_home, diff_color)
                    })
                    .collect();
                let mut migrated = 0i32;
                for (pid, close, same_home, diff_color) in cands {
                    // Residual vs WorldMap.randomFloat: same-home roll 1.5 else 0.
                    let roll = if same_home { 1.5 } else { 0.0 };
                    if !should_migrate_found_family_follower(
                        close, same_home, diff_color, migrated, roll,
                    ) {
                        continue;
                    }
                    if let Some(n) = state.social.lineages.get_mut(&pid) {
                        n.stamp_dynasty_id(dynasty);
                        n.my_eve_id = self_p_id;
                    }
                    rename.push(pid);
                    migrated += 1;
                }
                if let Some(n) = state.social.lineages.get_mut(&self_p_id) {
                    n.stamp_dynasty_id(dynasty);
                    n.my_eve_id = self_p_id;
                }
                // Haxe: familyPrestige[newFounder] = familyPrestige[oldFounder]
                state
                    .accounts
                    .copy_family_prestige(&email, old_founder, self_p_id);
                for &pid in &rename {
                    if pid == self_p_id {
                        continue;
                    }
                    if let Some(em) = state
                        .players
                        .values()
                        .find(|o| o.p_id == pid)
                        .map(|o| o.email.clone())
                    {
                        state
                            .accounts
                            .copy_family_prestige(&em, old_founder, self_p_id);
                    }
                }
                let cost = if state.gameplay.found_family_cost.is_finite()
                    && state.gameplay.found_family_cost >= 0.0
                {
                    state.gameplay.found_family_cost
                } else {
                    NAMING_FOUND_FAMILY_COST
                };
                state.economy.add_coins(self_p_id, -(cost as i32));
                rename.push(self_p_id);
            } else {
                // Haxe setFamilyName writes eveLineage.myFamilyName (whole eve).
                for o in state.players.values() {
                    if o.deleted {
                        continue;
                    }
                    let e = state
                        .social
                        .lineages
                        .get(&o.p_id)
                        .map(|n| n.family_eve_id())
                        .filter(|&e| e > 0)
                        .unwrap_or_else(|| {
                            crate::relations::root_eve_id(&state.social, o.p_id)
                        });
                    if e == eve_id {
                        rename.push(o.p_id);
                    }
                }
                if !rename.contains(&self_p_id) {
                    rename.push(self_p_id);
                }
            }
            let mut nm_rows: Vec<(i32, String, String)> = Vec::new();
            for pl in state.players.values_mut() {
                if rename.contains(&pl.p_id) {
                    pl.family_name = family_name.clone();
                    nm_rows.push((pl.p_id, pl.first_name.clone(), family_name.clone()));
                }
            }
            // Haxe setFamilyName writes eve.myFamilyName; WriteLineages dumps getter.
            // LINEAGE-FAMILY-NAME
            for &pid in &rename {
                state.social.stamp_lineage_family_name(pid, &family_name);
            }
            if eve_id > 0 {
                state
                    .social
                    .stamp_lineage_family_name(eve_id, &family_name);
            }
            let all_conn: Vec<u64> = state
                .players
                .iter()
                .filter(|(_, p)| p.connected && !p.deleted)
                .map(|(&c, _)| c)
                .collect();
            for (pid, first, last) in &nm_rows {
                let is_ai = state
                    .players
                    .values()
                    .find(|p| p.p_id == *pid)
                    .map(|p| p.is_ai_body())
                    .unwrap_or(false);
                let line = crate::format_player_nm_line_ex(
                    &state.social.lineages,
                    *pid,
                    first,
                    last,
                    is_ai,
                );
                let pkt = format_server_message("NM", &[&line]).into_bytes();
                for &cid in &all_conn {
                    outbound.send_urgent(cid, pkt.clone());
                }
            }
        }
    }
    plan
}

/// Live Haxe `DoNaming` YOU ARE first-name (held else closest r=5).
// Haxe: NamingHelper.DoNaming L45–198 (AI-NAMING-YOU-ARE)
fn apply_do_naming_you_are_live(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    text: &str,
) -> crate::naming::DoNamingYouAre {
    use crate::naming::{
        pick_you_are_target, plan_do_naming_you_are_ex_gender, DoNamingYouAre, YOU_ARE_CLOSE_TILES,
    };
    use ol_protocol::format_name_message;

    if !crate::naming::is_you_are_say(text) {
        return DoNamingYouAre::NotNaming;
    }
    let Some((self_p_id, sx, sy, mut held_p_id)) = (|| {
        let p = state.players.get(&conn_id)?;
        if p.deleted {
            return None;
        }
        Some((p.p_id, p.x, p.y, p.holding_player_id))
    })() else {
        return DoNamingYouAre::NotNaming;
    };
    // Haxe: heldPlayer else getClosestPlayer(5); held_by if holding_player_id unset.
    if held_p_id <= 0
        || !state
            .players
            .values()
            .any(|o| !o.deleted && o.p_id == held_p_id)
    {
        held_p_id = state
            .players
            .values()
            .find(|o| !o.deleted && o.held_by == self_p_id)
            .map(|o| o.p_id)
            .unwrap_or(0);
    }
    let others: Vec<(i32, i32, i32)> = state
        .players
        .values()
        .filter(|o| !o.deleted && o.p_id != self_p_id)
        .map(|o| (o.p_id, o.x, o.y))
        .collect();
    let Some(target_pid) = pick_you_are_target(
        self_p_id,
        held_p_id,
        sx,
        sy,
        &others,
        YOU_ARE_CLOSE_TILES,
    ) else {
        return DoNamingYouAre::NotNaming;
    };
    let Some(target_conn) = state
        .players
        .iter()
        .find(|(_, o)| o.p_id == target_pid)
        .map(|(&c, _)| c)
    else {
        return DoNamingYouAre::NotNaming;
    };
    let (target_first, used): (String, Vec<String>) = {
        let first = state
            .players
            .get(&target_conn)
            .map(|o| o.first_name.clone())
            .unwrap_or_default();
        let used = state
            .players
            .values()
            .filter(|o| !o.deleted)
            .map(|o| o.first_name.clone())
            .collect();
        (first, used)
    };
    let used_refs: Vec<&str> = used.iter().map(|s| s.as_str()).collect();
    let female = state
        .players
        .get(&target_conn)
        .map(|tp| crate::player_is_female(state, tp))
        .unwrap_or(false);
    let plan = plan_do_naming_you_are_ex_gender(
        text,
        &target_first,
        &used_refs,
        &state.gameplay.starting_name,
        female,
    );
    if let DoNamingYouAre::Apply { first_name } = &plan {
        let first_name = first_name.clone();
        let (age, family, tx, ty, email, display) = {
            let Some(pl) = state.players.get_mut(&target_conn) else {
                return DoNamingYouAre::NotNaming;
            };
            pl.first_name = first_name.clone();
            (
                pl.age,
                pl.family_name.clone(),
                pl.x,
                pl.y,
                pl.email.clone(),
                pl.display_name(),
            )
        };
        if let Some(node) = state.social.lineages.get_mut(&target_pid) {
            node.name = display.clone();
        }
        state.scoreboard.set_name(target_pid, &display);
        state.accounts.ensure(&email).last_name = display.clone();
        // Haxe: Connection.sendNameToAll
        let nm = format_name_message(target_pid, &first_name, &family).into_bytes();
        let all_conn: Vec<u64> = state
            .players
            .iter()
            .filter(|(_, p)| p.connected && !p.deleted)
            .map(|(&c, _)| c)
            .collect();
        for &cid in &all_conn {
            outbound.send_urgent(cid, nm.clone());
        }
        if age > 3.0 {
            let announce = format!("{first_name} {family}");
            let near = nearby_conn_ids(state, tx, ty, nearby_range(state));
            send_chat_ps(
                state,
                outbound,
                target_conn,
                target_pid,
                &announce,
                &near,
            );
        }
        // Haxe: p.doEmote(Emote.happy); if p != targetPlayer target also happy
        // AI-YOU-ARE-EMOTE
        let mut happy: Vec<(i32, i32, i32)> = vec![(self_p_id, sx, sy)];
        if target_pid != self_p_id {
            happy.push((target_pid, tx, ty));
        }
        for (pid, x, y) in happy {
            let pe = format_player_emot(pid, 0).into_bytes();
            let near = nearby_conn_ids(state, x, y, nearby_range(state));
            send_nearby(outbound, &near, pe);
            for &nid in &near {
                send_frame(outbound, nid);
            }
        }
    }
    plan
}

/// Haxe `foundFamily` — SAY `I AM {name}` (+ optional `I FOLLOW ME`); not a think-consuming rung.
// Haxe: AiBase.doTimeStuffHelper L602; foundFamily ~8199–8237
// AI-FOUND-FAMILY
fn tick_found_family(state: &mut SimState, outbound: &OutboundHub) {
    use crate::ai_follow_walk::{plan_found_family_ex, FOUND_FAMILY_FOLLOW_ME_SAY};
    use crate::relations::is_same_family;

    let ai_ids: Vec<u64> = state
        .players
        .iter()
        .filter(|(_, p)| !p.deleted && player_is_ai_follow_body(p))
        .filter(|(_, p)| !p.moving && p.move_path.is_none())
        .map(|(&c, _)| c)
        .collect();
    for conn_id in ai_ids {
        let Some(p) = state.players.get(&conn_id) else {
            continue;
        };
        let self_p_id = p.p_id;
        let is_founder = state
            .social
            .lineages
            .get(&self_p_id)
            .map(|n| n.is_family_founder())
            .unwrap_or(false);
        let follow_p_id = state
            .social
            .following
            .get(&self_p_id)
            .copied()
            .unwrap_or(0);
        let self_color = state.content.person_color(p.display_object_id);
        let leader_color = if follow_p_id > 0 {
            state
                .players
                .values()
                .find(|o| o.p_id == follow_p_id && !o.deleted)
                .map(|o| state.content.person_color(o.display_object_id))
        } else {
            None
        };
        let eve_id = state
            .social
            .lineages
            .get(&self_p_id)
            .map(|n| n.family_eve_id())
            .filter(|&e| e > 0)
            .unwrap_or_else(|| crate::relations::root_eve_id(&state.social, self_p_id));
        let same_account = eve_id != self_p_id
            && state
                .players
                .values()
                .any(|o| o.p_id == eve_id && o.email == p.email);
        let prestige = state
            .social
            .lineages
            .get(&self_p_id)
            .map(|n| n.prestige)
            .or_else(|| state.combat.stats.get(&self_p_id).map(|s| s.prestige))
            .unwrap_or(0.0);
        let coins = state.economy.coins_of(self_p_id) as f32;
        let family_name = p.family_name.clone();
        let (sx, sy, age) = (p.x, p.y, p.age);
        let same_family_followers = state
            .players
            .values()
            .filter(|o| {
                !o.deleted
                    && o.p_id != self_p_id
                    && state.social.is_follower_from(o.p_id, self_p_id)
                    && is_same_family(&state.social, o.p_id, self_p_id)
            })
            .count() as i32;
        let used: Vec<String> = state
            .players
            .values()
            .filter(|o| !o.deleted)
            .map(|o| o.family_name.clone())
            .collect();
        let used_refs: Vec<&str> = used.iter().map(|s| s.as_str()).collect();
        let break_roll = rand::random::<f32>();
        let Some(plan) = plan_found_family_ex(
            self_p_id,
            is_founder,
            same_account,
            follow_p_id,
            self_color,
            leader_color,
            prestige,
            same_family_followers,
            coins,
            &family_name,
            &used_refs,
            break_roll,
            state.gameplay.found_family_needed_prestige,
            state.gameplay.found_family_cost,
            state.gameplay.found_family_needed_followers,
            state.gameplay.found_family_break_alliance_chance,
        ) else {
            continue;
        };
        // Haxe: foundFamily only SAYS; DoNaming applies name / coins / myEveId.
        let _ = apply_do_naming_iam_live(state, outbound, conn_id, &plan.iam_say);
        let near = nearby_conn_ids(state, sx, sy, crate::say_close_range(state, age));
        send_chat_ps(state, outbound, conn_id, self_p_id, &plan.iam_say, &near);
        if plan.follow_me {
            state.social.following.remove(&self_p_id);
            send_chat_ps(
                state,
                outbound,
                conn_id,
                self_p_id,
                FOUND_FAMILY_FOLLOW_ME_SAY,
                &near,
            );
        }
    }
}

/// Haxe `allyUp` — hired workers never switch to most-powerful at home.
// Haxe: AiBase.allyUp L8250 hiredByPlayer != null return (AI-ALLY-UP-HIRE)
fn tick_ai_ally_up(state: &mut SimState, outbound: &OutboundHub) {
    use crate::ai_follow_walk::{
        plan_ally_up, pick_most_powerful_at_home, AllyUpBest, ALLY_UP_COOLDOWN_SECS,
    };
    let now = state.sim_time;
    let ai_ids: Vec<u64> = state
        .players
        .iter()
        .filter(|(_, p)| !p.deleted && player_is_ai_follow_body(p))
        .map(|(&c, _)| c)
        .collect();
    if ai_ids.is_empty() {
        return;
    }
    let following = state.social.following.clone();
    for conn_id in ai_ids {
        let Some(p) = state.players.get(&conn_id) else {
            continue;
        };
        if now - p.ai_last_leader_check_sim < ALLY_UP_COOLDOWN_SECS
            && p.ai_last_leader_check_sim > 0.0
        {
            continue;
        }
        let self_p_id = p.p_id;
        let hired = state.social.hired_boss(self_p_id);
        let age = p.age;
        let has_home = p.home_x != 0 || p.home_y != 0;
        let follow_p_id = following.get(&self_p_id).copied().unwrap_or(0);
        let hx = p.home_x;
        let hy = p.home_y;
        let sx = p.x;
        let sy = p.y;
        if let Some(p) = state.players.get_mut(&conn_id) {
            p.ai_last_leader_check_sim = now;
        }
        if hired > 0 {
            continue;
        }
        let cands: Vec<(i32, i32, i32, f32, i32, i32, bool)> = state
            .players
            .values()
            .map(|o| {
                let class = state.social.prestige_class(o.p_id).as_i32();
                let coins = state.economy.coins_of(o.p_id);
                (o.p_id, o.home_x, o.home_y, 0.0, coins, class, o.deleted)
            })
            .collect();
        let Some(best_id) = pick_most_powerful_at_home(hx, hy, self_p_id, &cands) else {
            continue;
        };
        let Some(best_p) = state.players.values().find(|o| o.p_id == best_id) else {
            continue;
        };
        let dx = best_p.x - sx;
        let dy = best_p.y - sy;
        let quad = dx * dx + dy * dy;
        let follow_same_home = if follow_p_id > 0 {
            state
                .players
                .values()
                .find(|o| o.p_id == follow_p_id)
                .map(|f| f.home_x == hx && f.home_y == hy)
                .unwrap_or(false)
        } else {
            false
        };
        let already_ally = crate::relations::is_leadership_ally(&following, self_p_id, best_id);
        let best = AllyUpBest {
            p_id: best_id,
            name: best_p.first_name.clone(),
            quad_dist: quad,
            pending_new_follower: best_p.new_follower_id != 0,
            already_ally,
        };
        let Some(say) = plan_ally_up(
            0.0, // already passed cooldown
            now,
            has_home,
            0,
            age,
            follow_p_id,
            follow_same_home,
            false,
            self_p_id,
            Some(&best),
        ) else {
            continue;
        };
        let near = nearby_conn_ids(state, sx, sy, crate::say_close_range(state, age));
        send_chat_ps(state, outbound, conn_id, self_p_id, &say, &near);
        let _ = say;
    }
}

/// Continuous follow walk for AI sticky `ai_follow_p_id` (Haxe isMovingToPlayer).
// Haxe: AiBase.doTimeStuffHelper sticky clear → isMovingToPlayer(acquire+walk)
// AI-FOLLOW-WALK + AI-FOLLOW-ACQUIRE
fn tick_ai_follow_walk(state: &mut SimState, outbound: &OutboundHub) {
    use crate::ai_goals::priority_ladder::is_hungry_simple;
    use crate::ai_llm_apply::ai_follow_walk::{
        follow_max_tiles_for_context_ex, should_say_follow_target_name_ex,
    };

    let now = state.sim_time;

    // Haxe: doTimeStuffHelper L560–568 sticky auto-clear BEFORE isMovingToPlayer
    // (ordered 5min timeout / age>MinAgeToEat*2 clear). Must run before acquire so
    // adult AutoFollow can re-latch same tick after age clear (Haxe order).
    {
        let clear_jobs: Vec<(u64, i32, bool, f32, f32)> = state
            .players
            .iter()
            .filter(|(_, p)| !p.deleted && p.ai_follow_p_id > 0)
            .filter(|(_, p)| player_is_ai_follow_body(p))
            .map(|(c, p)| {
                (
                    *c,
                    p.ai_follow_p_id,
                    p.ai_auto_stop_follow,
                    p.ai_follow_started_sim_time,
                    p.age,
                )
            })
            .collect();
        for (conn_id, follow_p_id, auto_stop, started, age) in clear_jobs {
            let mut sticky = AiFollowSticky {
                follow_p_id,
                auto_stop_follow: auto_stop,
                follow_started_sim_time: started,
            };
            // C-SS-MIN-AGE-AI: live MinAgeToEat * 2 clear gate
            let clear = plan_follow_sticky_clear_ex(
                &sticky,
                age,
                now,
                state.gameplay.min_age_to_eat,
            );
            apply_follow_sticky_clear(&mut sticky, clear);
            if let Some(p) = state.players.get_mut(&conn_id) {
                p.ai_auto_stop_follow = sticky.auto_stop_follow;
                p.ai_follow_p_id = sticky.follow_p_id;
            }
        }
    }

    // AI-FOLLOW-ACQUIRE: fill empty sticky before walk (child-mother / AutoFollowPlayer)
    // Haxe: isMovingToPlayer when playerToFollow == null
    // LiveSettings / GameplayKnobs.auto_follow_player (Haxe ServerSettings.AutoFollowPlayer)
    let auto_follow = state.gameplay.auto_follow_player;
    tick_ai_follow_acquire(state, auto_follow);
    // Haxe: searchNewHomeIfNeeded after isMoving, before foundFamily/allyUp
    tick_search_new_home_if_needed(state);
    tick_found_family(state, outbound);
    tick_ai_ally_up(state, outbound);

    let following = state.social.following.clone();
    let ai_jobs: Vec<(
        u64,
        i32,
        i32,
        i32,
        f32,
        bool,
        bool,
        bool,
        bool,
        bool,
        bool,
    )> = {
        let content = &state.content;
        state
            .players
            .iter()
            .filter(|(_, p)| !p.deleted && p.ai_follow_p_id > 0)
            .filter(|(_, p)| player_is_ai_follow_body(p))
            .map(|(c, p)| {
                // Haxe: isWounded() || hasYellowFever()
                let held_wound =
                    p.is_wounded_held(is_wound_object(content, p.held_id));
                let wounded_or_fever =
                    held_wound || p.hidden_wound.is_some() || p.fever.is_some();
                (
                    *c,
                    p.p_id,
                    p.x,
                    p.y,
                    p.age,
                    p.ai_auto_stop_follow,
                    p.moving || p.move_path.is_some(),
                    is_hungry_simple(p.food),
                    p.ai_is_nice_baby,
                    wounded_or_fever,
                    p.ai_debug_say,
                )
            })
            .collect()
    };
    if ai_jobs.is_empty() {
        return;
    }
    let by_pid: std::collections::HashMap<i32, (i32, i32, bool, String)> = state
        .players
        .values()
        .map(|p| (p.p_id, (p.x, p.y, p.deleted, p.display_name())))
        .collect();

    for (
        conn_id,
        ai_p_id,
        ax,
        ay,
        age,
        auto_stop,
        moving,
        hungry,
        nice_baby,
        wounded_or_fever,
        debug_say,
    ) in ai_jobs
    {
        let follow_p_id = state
            .players
            .get(&conn_id)
            .map(|p| p.ai_follow_p_id)
            .unwrap_or(0);
        if follow_p_id <= 0 {
            continue;
        }
        let target_row = by_pid.get(&follow_p_id);
        let target = target_row.map(|&(x, y, deleted, _)| FollowTargetSnap {
            p_id: follow_p_id,
            x,
            y,
            deleted,
        });
        // Living leadership mother for child band (same social.following as acquire).
        let leader = direct_follow_leader(&following, ai_p_id);
        let has_living_mother = leader
            .map(|id| {
                by_pid
                    .get(&id)
                    .map(|(_, _, deleted, _)| !*deleted)
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        // Haxe doTimeStuffHelper specialized distance bands before general 5/10
        // C-SS-MIN-AGE-AI: live MinAgeToEat for baby/child follow bands
        let max_tiles = follow_max_tiles_for_context_ex(
            age,
            hungry,
            has_living_mother,
            nice_baby,
            wounded_or_fever,
            auto_stop,
            state.gameplay.min_age_to_eat,
        );
        let seed = follow_seed(now, ai_p_id);
        let decision = decide_follow_walk(follow_p_id, target, ax, ay, max_tiles, seed);
        match decision {
            FollowWalkDecision::TargetDeleted => {
                if let Some(p) = state.players.get_mut(&conn_id) {
                    p.ai_follow_p_id = 0;
                    p.ai_auto_stop_follow = true;
                }
            }
            FollowWalkDecision::WalkTo { goal_x, goal_y } => {
                if !moving {
                    let _ = try_ai_follow_path_to(state, outbound, conn_id, goal_x, goal_y);
                }
                // Haxe: isMovingToPlayer L8318 say target name while walking
                if should_say_follow_target_name_ex(age, debug_say, state.gameplay.min_age_to_eat) {
                    if let Some((_, _, _, name)) = target_row {
                        if !name.is_empty() {
                            // PO-MAX-DISTANCE: CloseForSay 20
                            let near = nearby_conn_ids(state, ax, ay, ADULT_CHAT_RANGE);
                            send_chat_ps(state, outbound, conn_id, ai_p_id, name, &near);
                        }
                    }
                }
            }
            FollowWalkDecision::CloseEnough | FollowWalkDecision::NoTarget => {}
        }
    }
}
