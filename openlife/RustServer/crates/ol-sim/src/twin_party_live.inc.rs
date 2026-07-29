/// Join twin-code waiting queue; when full, birth all members together.
// FERTILITY-TWINS twin_sockets — protocol twin_code_hash / twin_count
pub fn apply_twin_join(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    p_id: i32,
    email: &str,
    code: &str,
    twin_count: i32,
) {
    let outcome = state
        .twin_wait
        .join(code, twin_count, conn_id, email, state.sim_time);
    match outcome {
        TwinJoinOutcome::Waiting { have, need } => {
            // TWIN-PARTY-RESID: include short code hash in wait PS
            let line = format!(
                "{p_id} {}",
                format_twin_wait_ps_code(have, need, code)
            );
            send_ps_reply(outbound, conn_id, &line);
            info!(conn_id, have, need, code = %code, "sim: twin wait");
        }
        TwinJoinOutcome::AlreadyWaiting { have, need } => {
            let line = format!(
                "{p_id} {}",
                format_twin_wait_ps_code(have, need, code)
            );
            send_ps_reply(outbound, conn_id, &line);
        }
        TwinJoinOutcome::Ready(party) => {
            process_ready_twin_party(state, outbound, party);
        }
        TwinJoinOutcome::InvalidCount => {
            send_ps_reply(
                outbound,
                conn_id,
                &format!("{p_id} TWINJOIN FAIL bad_count"),
            );
        }
        TwinJoinOutcome::EmptyCode => {
            send_ps_reply(
                outbound,
                conn_id,
                &format!("{p_id} TWINJOIN FAIL empty_code"),
            );
        }
        TwinJoinOutcome::CountMismatch { expected } => {
            send_ps_reply(
                outbound,
                conn_id,
                &format!("{p_id} TWINJOIN FAIL count_mismatch expected={expected}"),
            );
        }
    }
}

/// Birth a ready twin party: convert living members to age-0 babies at one mother
/// (or twin-Eve cluster). Protocol product for twin_code_hash / twin_count.
// OHOL twins plan: same mother or twin Eves; identical gender via shared po_id
fn process_ready_twin_party(
    state: &mut SimState,
    outbound: &OutboundHub,
    party: ReadyTwinParty,
) {
    let ready_line = format_twin_party_ready(&party);
    info!(
        code = %party.code_hash,
        count = party.twin_count,
        members = party.members.len(),
        "sim: twin party ready"
    );

    // (conn, p_id, x, y, home_x, home_y, family_name, display_name, can_hold)
    let mother_info: Option<(u64, i32, i32, i32, i32, i32, String, String, bool)> = {
        let mut found = None;
        for (&cid, pl) in state.players.iter() {
            if pl.deleted {
                continue;
            }
            let po = person_object_id(pl);
            let (name, desc) = state
                .content
                .get(po)
                .map(|d| (d.name.as_str(), d.description.as_str()))
                .unwrap_or(("", ""));
            let female = person_looks_female(po, name, desc);
            // C-SS-MORE-BATCH4: live MinAgeFertile / MaxAgeFertile
            if !is_fertile_ex(
                false,
                pl.age,
                female,
                state.gameplay.min_age_fertile,
                state.gameplay.max_age_fertile,
            ) {
                continue;
            }
            if party.members.iter().any(|m| m.conn_id == cid) {
                continue;
            }
            found = Some((
                cid,
                pl.p_id,
                pl.x,
                pl.y,
                pl.home_x,
                pl.home_y,
                pl.family_name.clone(),
                pl.display_name(),
                pl.can_hold_baby(),
            ));
            break;
        }
        found
    };

    let twin_po = DEFAULT_PERSON_OBJECT;
    let (eve_x, eve_y) = {
        let w = state.world.read().unwrap();
        find_playable_spawn(&w, (state.spawn_x, state.spawn_y))
    };

    let mut first_baby: Option<(u64, i32)> = None;
    let mut born_pids: Vec<i32> = Vec::new();

    for (i, member) in party.members.iter().enumerate() {
        send_ps_reply(outbound, member.conn_id, &ready_line);

        if !state.players.contains_key(&member.conn_id) {
            let _ = spawn_player(state, member.conn_id, &member.email);
        }

        // Scope mut borrow so lineage/social updates can re-borrow state.
        let converted = if let Some(pl) = state.players.get_mut(&member.conn_id) {
            if pl.deleted {
                None
            } else {
                if let Some((_, _, mx, my, hx, hy, ref fam, _, _)) = mother_info {
                    pl.x = mx;
                    pl.y = my;
                    pl.set_birth_origin(mx, my);
                    pl.home_x = hx;
                    pl.home_y = hy;
                    if !fam.is_empty() {
                        pl.family_name = fam.clone();
                    }
                } else {
                    pl.x = eve_x + (i as i32 % 2);
                    pl.y = eve_y + (i as i32 / 2);
                    pl.set_birth_origin(pl.x, pl.y);
                    pl.home_x = pl.x;
                    pl.home_y = pl.y;
                }
                pl.age = 0.0;
                pl.true_age = 0.01;
                pl.food = START_FOOD;
                pl.food_max = MAX_FOOD;
                pl.display_object_id = twin_po;
                pl.held_id = 0;
                pl.held_by = 0;
                pl.holding_player_id = 0;
                Some((pl.p_id, pl.display_name(), pl.x, pl.y))
            }
        } else {
            None
        };
        let Some((baby_p_id, baby_name, bx, by)) = converted else {
            continue;
        };

        if let Some((_, mid, _, _, _, _, _, ref mother_name, _)) = mother_info {
            state.social.ensure_lineage(mid, mother_name);
            let mother_node = state
                .social
                .lineages
                .get(&mid)
                .cloned()
                .unwrap_or_else(|| LineageNode::eve(mid, mother_name.clone()));
            let child_node = LineageNode::with_mother(baby_p_id, baby_name, &mother_node);
            state.social.lineages.insert(baby_p_id, child_node);
            state.markers.set_mother_marker(baby_p_id, bx, by, mid);
        } else {
            state.social.ensure_lineage(baby_p_id, &baby_name);
        }

        if first_baby.is_none() {
            first_baby = Some((member.conn_id, baby_p_id));
        }
        born_pids.push(baby_p_id);

        state.publish_player_view(member.conn_id);
        send_forced_player_update(state, outbound, member.conn_id, Some(1));
        let mother_id = mother_info.as_ref().map(|m| m.1).unwrap_or(0);
        let line = format!(
            "{baby_p_id} TWINBORN OK party={} mother={mother_id}",
            party.code_hash
        );
        send_ps_reply(outbound, member.conn_id, &line);
        state.push_event(format!(
            "TWINBORN {baby_p_id} code={} mother={mother_id}",
            party.code_hash
        ));
    }

    // TWIN-PARTY-RESID: same-server heart-link for living party members
    if born_pids.len() >= 2 {
        state.twin_heart.register_party(&born_pids);
    }

    if let Some((mc, mid, _, _, _, _, _, _, can_hold0)) = mother_info {
        for _ in 0..party.members.len().max(1) {
            state.fertility.complete_birth(mid, state.sim_time);
        }
        if can_hold0 {
            if let Some((baby_conn, baby_p_id)) = first_baby {
                let can_hold = state
                    .players
                    .get(&mc)
                    .map(|m| m.can_hold_baby() && m.holding_player_id == 0)
                    .unwrap_or(false);
                if can_hold {
                    if let Some(m) = state.players.get_mut(&mc) {
                        m.start_holding(baby_p_id);
                    }
                    if let Some(b) = state.players.get_mut(&baby_conn) {
                        b.held_by = mid;
                    }
                }
            }
        }
    }
}

/// OHOL twins plan #10: murder of a party member wounds remaining twins with a
/// "broken heart" (max wound stacks + food cut) so they die soon after.
///
/// Only **murder** (`reason_killed` / `reason_killed_<id>`, not legal/suicide/hunger).
/// Call from combat kill paths after the deceased is marked deleted.
// TWIN-PARTY-RESID twin_wait_edges / heart-link
// OHOL forum: "If one of your party members dies of murder, the rest get wounded
// with a broken heart and die soon after."
pub fn apply_twin_heart_link_on_murder(
    state: &mut SimState,
    outbound: &OutboundHub,
    deceased_p_id: i32,
) {
    let siblings = state.twin_heart.on_member_death(deceased_p_id);
    if siblings.is_empty() {
        return;
    }
    let line = format_twin_heart_ps(deceased_p_id);
    for sib_pid in siblings {
        let Some((&conn_id, _)) = state
            .players
            .iter()
            .find(|(_, pl)| pl.p_id == sib_pid && !pl.deleted)
        else {
            continue;
        };
        let _ = state
            .combat
            .apply_wound(sib_pid, BROKEN_HEART_WOUND_STACKS);
        if let Some(s) = state.combat.stats.get_mut(&sib_pid) {
            if s.wounded_by == 0 {
                s.wounded_by = deceased_p_id.max(1);
            }
        }
        if let Some(pl) = state.players.get_mut(&conn_id) {
            pl.food = (pl.food * 0.25).min(4.0).max(0.0);
        }
        let p_id = state.players.get(&conn_id).map(|p| p.p_id).unwrap_or(sib_pid);
        send_ps_reply(outbound, conn_id, &format!("{p_id} {line}"));
        state.publish_player_view(conn_id);
        send_forced_player_update(state, outbound, conn_id, Some(1));
        info!(
            deceased = deceased_p_id,
            sibling = sib_pid,
            "sim: twin broken-heart link"
        );
        state.push_event(format!("TWINHEART {sib_pid} from {deceased_p_id}"));
    }
}
