//! Live wire for Haxe `GlobalPlayerInstance.doCommands` natural-language SAY forms.
//!
//! // Haxe: GlobalPlayerInstance.doCommands / processFollowCommand / processHireCommand
//! Pure parsers live in [`crate::speech`]; this module mutates sim state.
//! LEADERSHIP-UX: delayed `newFollower` confirm + FOLLOWER map pins + FW -1/top color.

use crate::ai_takeover::player_is_ai;
use crate::economy::Economy;
use crate::emotes::{emote_by_name, EmoteEntry};
use crate::leadership::format_map_location_says_body;
use crate::move_live_gates::is_friendly;
use crate::relations::{get_top_leader, is_close_relative, is_leadership_ally, top_leader};
use crate::settings_live::GameplayKnobs;
use crate::social::{
    format_exile_line, format_follow_pending_global, format_following_for_player,
    format_i_follow_now, format_i_follow_soon, format_you_have_new_follower, SocialState,
    TIME_CONFIRM_NEW_FOLLOWER,
};
use crate::speech::{
    closest_owned_tile, compute_hire_cost, do_command_broadcasts_chat, find_player_by_name,
    format_exile_say_result, format_follow_say_result, format_give_say_result,
    format_hire_say_result, format_home_bang_result, format_order_global, format_own_this_result,
    format_redeem_say_result, hire_age_ok, hire_angry_ok, hire_class_ok, is_follow_self_name,
    is_home_oven_id, parse_do_command, parse_roman_coin_amount, pick_nearest_home_oven, DoCommand,
    HIRE_COST, HIRE_COST_INCREASE_PER_PERSON, HOME_SEARCH_MAX_QUAD,
};
use crate::Player;
use ol_world::{ComplexObject, World};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Side-effects from applying one doCommands form.
#[derive(Debug, Default)]
pub struct DoCommandEffects {
    /// Private PS lines: (conn_id, body).
    pub private_ps: Vec<(u64, String)>,
    /// FW FOLLOWING body lines to fan nearby (or all when [`Self::fan_following_all`]).
    pub following_lines: Vec<String>,
    /// EX exile body lines to fan nearby.
    pub exile_lines: Vec<String>,
    /// Global ORDER messages to each conn under the speaker's leadership.
    pub order_global: Vec<(u64, String)>,
    /// When true, also broadcast the original SAY as normal chat (Haxe return true).
    pub broadcast_chat: bool,
    /// When true, command was recognized.
    pub recognized: bool,
    /// Haxe hire: `lostCombatPrestige -= ceil(lost/10)` after successful pay.
    // Haxe: processHireCommand combatPrestigeImppact regain
    pub combat_prestige_regain: f32,
    /// Haxe `Connection.SendFollowingToAll` — fan FW to every connection, not nearby only.
    // Haxe: Connection.SendFollowingToAll
    pub fan_following_all: bool,
    /// PE emotes: (conn_id, emot_index).
    pub emotes: Vec<(u64, i32)>,
    /// Spoken SAY bodies to rebroadcast nearby: (speaker_p_id, text).
    pub spoken_says: Vec<(i32, String)>,
}

/// Live knobs for delayed follow confirm + hire coin costs (FOLLOW-HIRE-DELAY).
///
/// Haxe: `ServerSettings.TimeConfirmNewFollower` / `HireCost` / `HireCostIncreasePerPerson`.
/// Defaults match module consts; hot-reload via [`GameplayKnobs`].
/// Note: **I HIRE is immediate** (`setFollowPlayer`); only I FOLLOW uses TimeConfirm.
// Haxe: ServerSettings.TimeConfirmNewFollower / HireCost*
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FollowHireLiveKnobs {
    /// Seconds until delayed `I FOLLOW` sticks (not used by hire — hire is immediate).
    pub time_confirm_new_follower: f32,
    pub hire_cost: i32,
    pub hire_cost_increase_per_person: i32,
}

impl Default for FollowHireLiveKnobs {
    fn default() -> Self {
        Self {
            time_confirm_new_follower: TIME_CONFIRM_NEW_FOLLOWER,
            hire_cost: HIRE_COST,
            hire_cost_increase_per_person: HIRE_COST_INCREASE_PER_PERSON,
        }
    }
}

impl FollowHireLiveKnobs {
    /// Snapshot from live gameplay knobs (sanitized floors).
    // Haxe: ServerSettings statics mid-session
    pub fn from_gameplay(gp: &GameplayKnobs) -> Self {
        Self {
            time_confirm_new_follower: if gp.time_confirm_new_follower.is_finite()
                && gp.time_confirm_new_follower > 0.0
            {
                gp.time_confirm_new_follower
            } else {
                TIME_CONFIRM_NEW_FOLLOWER
            },
            hire_cost: if gp.hire_cost.is_finite() && gp.hire_cost >= 0.0 {
                gp.hire_cost.round() as i32
            } else {
                HIRE_COST
            },
            hire_cost_increase_per_person: if gp.hire_cost_increase_per_person.is_finite()
                && gp.hire_cost_increase_per_person >= 0.0
            {
                gp.hire_cost_increase_per_person.round() as i32
            } else {
                HIRE_COST_INCREASE_PER_PERSON
            },
        }
    }
}

/// Snapshot of other players for name lookup (built by caller).
#[derive(Debug, Clone)]
pub struct NameCandidate {
    pub p_id: i32,
    pub conn_id: u64,
    pub first_name: String,
    pub x: i32,
    pub y: i32,
    pub deleted: bool,
    pub age: f32,
    pub angry_time: f32,
    pub ai_controlled: bool,
    pub connected: bool,
    pub email: String,
    pub display_object_id: i32,
    /// Haxe `ObjectData.person` race for `po_id` (0 = unset / non-person).
    /// Used by hire cost foreign-color ×2 — not raw display id equality.
    // Haxe: GlobalPlayerInstance.getColor / ObjectData.person
    pub person_color: i32,
    pub home_x: i32,
    pub home_y: i32,
    pub prestige_class: i32,
}

impl NameCandidate {
    pub fn from_player(p: &Player, prestige_class: i32) -> Self {
        Self::from_player_with_person_color(p, prestige_class, 0)
    }

    /// Build candidate with Haxe `ObjectData.person` race (from ContentDb).
    // Haxe: getColor() → ObjectData.person
    pub fn from_player_with_person_color(
        p: &Player,
        prestige_class: i32,
        person_color: i32,
    ) -> Self {
        Self {
            p_id: p.p_id,
            conn_id: p.conn_id,
            first_name: p.first_name.clone(),
            x: p.x,
            y: p.y,
            deleted: p.deleted,
            age: p.age,
            angry_time: p.angry_time,
            ai_controlled: p.ai_controlled,
            connected: p.connected,
            email: p.email.clone(),
            display_object_id: p.display_object_id,
            person_color,
            home_x: p.home_x,
            home_y: p.home_y,
            prestige_class,
        }
    }
}

/// Apply natural-language doCommands when `upper` matches a form.
// Haxe: GlobalPlayerInstance.doCommands
#[allow(clippy::too_many_arguments)]
pub fn apply_do_commands_live(
    upper: &str,
    speaker: &Player,
    speaker_conn: u64,
    social: &mut SocialState,
    economy: &mut Economy,
    players: &mut HashMap<u64, Player>,
    world: &Arc<RwLock<World>>,
    lost_combat_prestige: f32,
    candidates: &[NameCandidate],
    knobs: FollowHireLiveKnobs,
) -> DoCommandEffects {
    let mut fx = DoCommandEffects::default();
    let Some(cmd) = parse_do_command(upper) else {
        return fx;
    };
    fx.recognized = true;
    fx.broadcast_chat = do_command_broadcasts_chat(&cmd);

    let cand_tuples: Vec<(i32, &str, i32, i32, bool)> = candidates
        .iter()
        .map(|c| (c.p_id, c.first_name.as_str(), c.x, c.y, c.deleted))
        .collect();

    let lookup = |name: &str| -> Option<&NameCandidate> {
        let id = find_player_by_name(
            speaker.p_id,
            speaker.x,
            speaker.y,
            name,
            &cand_tuples,
            6,
        )?;
        candidates.iter().find(|c| c.p_id == id)
    };

    match cmd {
        DoCommand::FollowMy => {}
        DoCommand::Exile { name } => match lookup(&name) {
            None => fx.private_ps.push((
                speaker_conn,
                format_exile_say_result(speaker.p_id, &name, false, "not_found"),
            )),
            Some(t) => {
                if social.is_exiled_by(speaker.p_id, t.p_id) {
                    fx.private_ps.push((
                        speaker_conn,
                        format_exile_say_result(speaker.p_id, &name, false, "already"),
                    ));
                } else {
                    social.exile(speaker.p_id, t.p_id);
                    fx.exile_lines
                        .push(format_exile_line(t.p_id, speaker.p_id));
                    fx.private_ps.push((
                        speaker_conn,
                        format_exile_say_result(speaker.p_id, &name, true, ""),
                    ));
                }
            }
        },
        DoCommand::Redeem { name } => match lookup(&name) {
            None => fx.private_ps.push((
                speaker_conn,
                format_redeem_say_result(speaker.p_id, &name, false, "not_found"),
            )),
            Some(t) => {
                let n = social.redeem(speaker.p_id, t.p_id);
                if n == 0 {
                    fx.private_ps.push((
                        speaker_conn,
                        format_redeem_say_result(speaker.p_id, &name, false, "not_exiled"),
                    ));
                } else {
                    fx.private_ps.push((
                        speaker_conn,
                        format_redeem_say_result(speaker.p_id, &name, true, ""),
                    ));
                }
            }
        },
        DoCommand::Follow { name } => {
            // Haxe: GlobalPlayerInstance.processFollowCommand
            // FOLLOW-HIRE-DELAY: live TimeConfirmNewFollower
            process_follow_command(
                &name,
                speaker,
                speaker_conn,
                social,
                players,
                candidates,
                knobs.time_confirm_new_follower,
                &mut fx,
            );
        }
        DoCommand::Hire { name } => {
            // Haxe: processHireCommand — immediate setFollowPlayer (not delayed newFollower).
            // FOLLOW-HIRE-DELAY: hire stays immediate; TimeConfirm only gates I FOLLOW.
            if name.is_empty() {
                fx.private_ps.push((
                    speaker_conn,
                    format_hire_say_result(speaker.p_id, "?", false, "not_found"),
                ));
            } else {
                match lookup(&name) {
                    None => fx.private_ps.push((
                        speaker_conn,
                        format_hire_say_result(speaker.p_id, &name, false, "not_found"),
                    )),
                    Some(t) => match try_hire(
                        speaker,
                        t,
                        social,
                        economy,
                        players,
                        lost_combat_prestige,
                        candidates,
                        knobs.hire_cost,
                        knobs.hire_cost_increase_per_person,
                    ) {
                        Ok((cost, prestige_regain)) => {
                            fx.combat_prestige_regain = prestige_regain;
                            // Haxe: Connection.SendFollowingToAll(player)
                            let deleted: HashSet<i32> = candidates
                                .iter()
                                .filter(|c| c.deleted)
                                .map(|c| c.p_id)
                                .collect();
                            let top = get_top_leader(
                                &social.following,
                                social,
                                &deleted,
                                t.p_id,
                                None,
                            )
                            .unwrap_or(t.p_id);
                            fx.following_lines
                                .push(format_following_for_player(social, t.p_id, top));
                            fx.fan_following_all = true;
                            if let Some(e) = emote_index("HAPPY") {
                                fx.emotes.push((speaker_conn, e));
                                if t.conn_id != 0 {
                                    fx.emotes.push((t.conn_id, e));
                                }
                            }
                            // Haxe: this.say('I hire …'); player.say('… hired me!')
                            fx.spoken_says.push((
                                speaker.p_id,
                                format!("I hire {} for {} coins!", t.first_name, cost),
                            ));
                            fx.spoken_says.push((
                                t.p_id,
                                format!("{} hired me!", speaker.first_name),
                            ));
                            fx.private_ps.push((
                                speaker_conn,
                                format_hire_say_result(
                                    speaker.p_id,
                                    &name,
                                    true,
                                    &format!("cost={cost}"),
                                ),
                            ));
                        }
                        Err(reason) => fx.private_ps.push((
                            speaker_conn,
                            format_hire_say_result(speaker.p_id, &name, false, reason),
                        )),
                    },
                }
            }
        }
        DoCommand::Order { text } => {
            let msg = format_order_global(&text);
            fx.order_global.push((speaker_conn, msg.clone()));
            for c in candidates {
                if c.deleted || c.p_id == speaker.p_id {
                    continue;
                }
                if top_leader(&social.following, c.p_id) == speaker.p_id {
                    fx.order_global.push((c.conn_id, msg.clone()));
                }
            }
            fx.private_ps
                .push((speaker_conn, format!("{} ORDER OK", speaker.p_id)));
        }
        DoCommand::Give { name, coin_token } => {
            let amount = parse_roman_coin_amount(&coin_token);
            match lookup(&name) {
                None => fx.private_ps.push((
                    speaker_conn,
                    format_give_say_result(speaker.p_id, 0, amount, false, "not_found"),
                )),
                Some(t) if amount <= 0 => fx.private_ps.push((
                    speaker_conn,
                    format_give_say_result(speaker.p_id, t.p_id, amount, false, "bad_amount"),
                )),
                Some(t) => {
                    let ok = economy.gift(speaker.p_id, t.p_id, amount);
                    fx.private_ps.push((
                        speaker_conn,
                        format_give_say_result(
                            speaker.p_id,
                            t.p_id,
                            amount,
                            ok,
                            if ok { "" } else { "need_coins" },
                        ),
                    ));
                    if ok {
                        if let Some(tp) = players.values().find(|p| p.p_id == t.p_id) {
                            fx.private_ps.push((
                                tp.conn_id,
                                format!(
                                    "{} RECV {} coins from {}",
                                    t.p_id, amount, speaker.first_name
                                ),
                            ));
                        }
                    }
                }
            }
        }
        DoCommand::OwnThis { name } => match lookup(&name) {
            None => fx.private_ps.push((
                speaker_conn,
                format_own_this_result(speaker.p_id, 0, false, "not_found"),
            )),
            Some(t) => {
                let owning = players
                    .get(&speaker_conn)
                    .map(|p| p.owning.clone())
                    .unwrap_or_default();
                match closest_owned_tile(speaker.x, speaker.y, &owning) {
                    None => fx.private_ps.push((
                        speaker_conn,
                        format_own_this_result(speaker.p_id, t.p_id, false, "no_property"),
                    )),
                    Some((ox, oy)) => {
                        let mut ok = false;
                        let mut already = false;
                        if let Ok(mut w) = world.write() {
                            if let Some(mut h) = w.get_helper(ox, oy).cloned() {
                                if h.owner_id == t.p_id {
                                    already = true;
                                } else {
                                    h.owner_id = t.p_id;
                                    w.set_object_complex(ox, oy, h);
                                    ok = true;
                                }
                            } else {
                                let id = w.get_object(ox, oy);
                                if id != 0 {
                                    let h = ComplexObject::with_owner(id, t.p_id);
                                    w.set_object_complex(ox, oy, h);
                                    ok = true;
                                }
                            }
                        }
                        if already {
                            fx.private_ps.push((
                                speaker_conn,
                                format_own_this_result(speaker.p_id, t.p_id, false, "already"),
                            ));
                        } else if ok {
                            for pl in players.values_mut() {
                                if pl.p_id == t.p_id && !pl.owning.contains(&(ox, oy)) {
                                    pl.owning.push((ox, oy));
                                }
                            }
                            fx.private_ps.push((
                                speaker_conn,
                                format_own_this_result(speaker.p_id, t.p_id, true, ""),
                            ));
                        } else {
                            fx.private_ps.push((
                                speaker_conn,
                                format_own_this_result(speaker.p_id, t.p_id, false, "no_property"),
                            ));
                        }
                    }
                }
            }
        },
        DoCommand::HomeBang => {
            let mut ovens: Vec<(i32, i32, bool)> = Vec::new();
            if let Ok(w) = world.read() {
                let r = 80i32;
                for y in (speaker.y - r)..=(speaker.y + r) {
                    for x in (speaker.x - r)..=(speaker.x + r) {
                        let id = w.get_object(x, y);
                        if is_home_oven_id(id) {
                            ovens.push((x, y, w.get_floor(x, y) > 0));
                        }
                    }
                }
            }
            match pick_nearest_home_oven(speaker.x, speaker.y, &ovens, HOME_SEARCH_MAX_QUAD) {
                None => fx.private_ps.push((
                    speaker_conn,
                    format_home_bang_result(speaker.p_id, 0, 0, false, "no_oven"),
                )),
                Some((hx, hy)) => {
                    if let Some(pl) = players.get_mut(&speaker_conn) {
                        let same = pl.home_x == hx && pl.home_y == hy;
                        pl.home_x = hx;
                        pl.home_y = hy;
                        let leader_id = pl.p_id;
                        let following = social.following.clone();
                        for other in players.values_mut() {
                            if other.p_id == leader_id || other.deleted {
                                continue;
                            }
                            if following.get(&other.p_id) == Some(&leader_id)
                                && player_is_ai(
                                    other.connected,
                                    other.ai_controlled,
                                    &other.email,
                                )
                            {
                                other.home_x = hx;
                                other.home_y = hy;
                            }
                        }
                        fx.private_ps.push((
                            speaker_conn,
                            format_home_bang_result(
                                speaker.p_id,
                                hx,
                                hy,
                                true,
                                if same { "same" } else { "" },
                            ),
                        ));
                    }
                }
            }
        }
    }

    fx
}

/// Haxe `processFollowCommand` — ME unfollows immediately; named follow schedules
/// delayed confirm on top+direct leaders (`newFollower*` / `TimeConfirmNewFollower`).
// Haxe: GlobalPlayerInstance.processFollowCommand
fn process_follow_command(
    name: &str,
    speaker: &Player,
    speaker_conn: u64,
    social: &mut SocialState,
    players: &mut HashMap<u64, Player>,
    candidates: &[NameCandidate],
    confirm_secs: f32,
    fx: &mut DoCommandEffects,
) {
    if is_follow_self_name(name) {
        // Haxe: followPlayer = null; SendFollowingToAll; say I FOLLOW ME; happy
        social.unfollow(speaker.p_id);
        let deleted: HashSet<i32> = candidates
            .iter()
            .filter(|c| c.deleted)
            .map(|c| c.p_id)
            .collect();
        let top = get_top_leader(&social.following, social, &deleted, speaker.p_id, None)
            .unwrap_or(speaker.p_id);
        fx.following_lines
            .push(format_following_for_player(social, speaker.p_id, top));
        fx.fan_following_all = true;
        fx.private_ps.push((
            speaker_conn,
            format!("{} YOU_FOLLOW_NOW_NO_ONE!", speaker.p_id),
        ));
        fx.spoken_says
            .push((speaker.p_id, "I FOLLOW ME!".into()));
        if let Some(e) = emote_index("HAPPY") {
            fx.emotes.push((speaker_conn, e));
        }
        fx.private_ps.push((
            speaker_conn,
            format_follow_say_result(speaker.p_id, "ME", true, ""),
        ));
        return;
    }

    let cand_tuples: Vec<(i32, &str, i32, i32, bool)> = candidates
        .iter()
        .map(|c| (c.p_id, c.first_name.as_str(), c.x, c.y, c.deleted))
        .collect();
    let target = find_player_by_name(
        speaker.p_id,
        speaker.x,
        speaker.y,
        name,
        &cand_tuples,
        6,
    )
    .and_then(|id| candidates.iter().find(|c| c.p_id == id));

    let Some(t) = target else {
        fx.private_ps.push((
            speaker_conn,
            format_follow_say_result(speaker.p_id, name, false, "not_found"),
        ));
        return;
    };
    if t.p_id == speaker.p_id {
        fx.private_ps.push((
            speaker_conn,
            format_follow_say_result(speaker.p_id, name, false, "not_found"),
        ));
        return;
    }

    // Haxe: getLeaderWhoExiled gate
    if let Some(_exiler) = social.leader_who_exiled(speaker.p_id, t.p_id) {
        fx.private_ps.push((
            speaker_conn,
            format_follow_say_result(speaker.p_id, name, false, "exiled"),
        ));
        return;
    }

    if social.following.get(&speaker.p_id) == Some(&t.p_id) {
        fx.private_ps.push((
            speaker_conn,
            format_follow_say_result(speaker.p_id, name, false, "already"),
        ));
        return;
    }

    let deleted: HashSet<i32> = candidates
        .iter()
        .filter(|c| c.deleted)
        .map(|c| c.p_id)
        .collect();

    // Probe circular via temp follow + getTopLeader null (Haxe tmpFollow pattern).
    // Haxe: set followPlayer = player; leader = getTopLeader(); restore
    let tmp = social.following.get(&speaker.p_id).copied();
    social.following.insert(speaker.p_id, t.p_id);
    let top_opt = get_top_leader(&social.following, social, &deleted, speaker.p_id, None);
    match tmp {
        Some(prev) => {
            social.following.insert(speaker.p_id, prev);
        }
        None => {
            social.following.remove(&speaker.p_id);
        }
    }
    let Some(top_leader_id) = top_opt else {
        fx.private_ps.push((
            speaker_conn,
            format_follow_say_result(speaker.p_id, name, false, "circular_follow"),
        ));
        return;
    };

    // Busy-slot gates: top leader and direct target each hold at most one pending.
    // Haxe: leader.newFollower / player.newFollower
    let (top_busy, top_busy_is_self, top_busy_time) =
        pending_busy_of(players, top_leader_id, speaker.p_id);
    if top_busy {
        let secs = top_busy_time.ceil() as i32;
        let msg = if top_busy_is_self {
            format!(
                "{} Leader will accept you in {secs} seconds...",
                speaker.p_id
            )
        } else {
            format!(
                "{} Top leader is considering some one else. Try in {secs} seconds...",
                speaker.p_id
            )
        };
        fx.private_ps.push((speaker_conn, msg));
        return;
    }
    if t.p_id != top_leader_id {
        let (dir_busy, _, dir_time) = pending_busy_of(players, t.p_id, speaker.p_id);
        if dir_busy {
            let secs = dir_time.ceil() as i32;
            fx.private_ps.push((
                speaker_conn,
                format!(
                    "{} {} is considering some one else. Try in {secs} seconds...",
                    speaker.p_id, t.first_name
                ),
            ));
            return;
        }
    }

    // FOLLOW-HIRE-DELAY: live TimeConfirmNewFollower (default 15).
    let confirm_t = if confirm_secs.is_finite() && confirm_secs > 0.0 {
        confirm_secs
    } else {
        TIME_CONFIRM_NEW_FOLLOWER
    };
    let family = players
        .values()
        .find(|p| p.p_id == t.p_id)
        .map(|p| p.family_name.clone())
        .unwrap_or_default();

    // Schedule on top leader + direct target (Haxe both get the pending).
    for host_id in [top_leader_id, t.p_id] {
        for pl in players.values_mut() {
            if pl.p_id == host_id {
                pl.new_follower_id = speaker.p_id;
                pl.new_follower_for_id = t.p_id;
                pl.new_follower_time = confirm_t;
                break;
            }
        }
    }

    fx.private_ps.push((
        speaker_conn,
        format!(
            "{} {}",
            speaker.p_id,
            format_follow_pending_global(confirm_t, &t.first_name, &family)
        ),
    ));
    fx.private_ps.push((
        speaker_conn,
        format_follow_say_result(speaker.p_id, name, true, "pending"),
    ));

    // FOLLOWER map pin + YOU_HAVE_A_NEW_FOLLOWER + hubba on top (and direct if mid-chain).
    // Pin target = new follower (requestor) — Haxe literally pins self; sensible UX is follower.
    // Haxe: leader.connection.sendMapLocation / YOU_HAVE_A_NEW_FOLLOWER / doEmote hubba
    notify_new_follower_request(
        players,
        top_leader_id,
        speaker,
        fx,
    );
    if t.p_id != top_leader_id {
        notify_new_follower_request(players, t.p_id, speaker, fx);
    }

    if let Some(e) = emote_index("HAPPY") {
        fx.emotes.push((speaker_conn, e));
    }
    fx.spoken_says
        .push((speaker.p_id, format_i_follow_soon(&t.first_name)));
}

fn pending_busy_of(
    players: &HashMap<u64, Player>,
    host_p_id: i32,
    speaker_p_id: i32,
) -> (bool, bool, f32) {
    for pl in players.values() {
        if pl.p_id == host_p_id && pl.new_follower_id != 0 {
            return (
                true,
                pl.new_follower_id == speaker_p_id,
                pl.new_follower_time,
            );
        }
    }
    (false, false, 0.0)
}

fn notify_new_follower_request(
    players: &HashMap<u64, Player>,
    host_p_id: i32,
    follower: &Player,
    fx: &mut DoCommandEffects,
) {
    let Some(host) = players.values().find(|p| p.p_id == host_p_id) else {
        return;
    };
    // Haxe sendMapLocation: transformX/Y = target.tx − viewer.birth (gx/gy), not current pos.
    // Haxe: Connection.sendMapLocation / WorldMap.transformX
    let (rel_x, rel_y) = crate::map_location_pins::follow_request_pin_rel(
        host.birth_x,
        host.birth_y,
        follower.x,
        follower.y,
    );
    let body = format_map_location_says_body(
        "FOLLOWER",
        "follower",
        follower.p_id,
        rel_x,
        rel_y,
    );
    fx.private_ps
        .push((host.conn_id, format!("{}/0 {body}", host.p_id)));
    fx.private_ps.push((
        host.conn_id,
        format!(
            "{} {}",
            host.p_id,
            format_you_have_new_follower(&follower.first_name, &follower.family_name)
        ),
    ));
    if let Some(e) = emote_index("HUBBA") {
        fx.emotes.push((host.conn_id, e));
    }
}

fn emote_index(name: &str) -> Option<i32> {
    emote_by_name(name).map(|e: &EmoteEntry| e.index)
}

/// Side-effects from delayed follow confirm tick.
// Haxe: TimeHelper.DoTimeStuffForPlayer newFollowerTime
#[derive(Debug, Default)]
pub struct PendingFollowTickEffects {
    pub following_lines: Vec<String>,
    pub private_ps: Vec<(u64, String)>,
    pub spoken_says: Vec<(i32, String)>,
    /// Always fan FW to all connections (SendFollowingToAll).
    pub fan_following_all: bool,
}

/// Countdown + confirm Haxe `newFollowerTime` for all players.
///
/// When time hits ≤0 with a pending request: re-check exile, `set_follow`, FW, clear slots.
// Haxe: TimeHelper L415–441 newFollowerTime_confirm
pub fn tick_pending_new_followers(
    players: &mut HashMap<u64, Player>,
    social: &mut SocialState,
    dt: f32,
) -> PendingFollowTickEffects {
    let mut fx = PendingFollowTickEffects::default();
    if dt <= 0.0 {
        return fx;
    }

    // Snapshot who needs confirm this frame (process after countdown).
    let mut to_confirm: Vec<(i32, i32, i32)> = Vec::new(); // (host, follower, follow_for)
    for pl in players.values_mut() {
        if pl.new_follower_time > 0.0 {
            pl.new_follower_time = (pl.new_follower_time - dt).max(0.0);
        }
        if pl.new_follower_time <= 0.0 && pl.new_follower_id != 0 {
            to_confirm.push((
                pl.p_id,
                pl.new_follower_id,
                pl.new_follower_for_id,
            ));
        }
    }

    let deleted: HashSet<i32> = players
        .values()
        .filter(|p| p.deleted)
        .map(|p| p.p_id)
        .collect();

    for (host_id, follower_id, follow_for_id) in to_confirm {
        // Skip if already cleared by a paired host earlier this tick.
        let still_pending = players
            .values()
            .any(|p| p.p_id == host_id && p.new_follower_id == follower_id);
        if !still_pending {
            continue;
        }

        // Haxe: exileLeader = newFollower.getLeaderWhoExiled(player)
        // Re-check vs direct follow target (newFollowerFor); host may be top leader.
        let not_exiled = social
            .leader_who_exiled(follower_id, follow_for_id)
            .is_none();
        let already = social.following.get(&follower_id) == Some(&follow_for_id);

        if not_exiled && !already && follow_for_id != 0 {
            // Haxe: setFollowPlayer(newFollowerFor) — probes getTopLeader != null
            let ok = try_set_follow_player(social, &deleted, follower_id, follow_for_id);
            if ok {
                let top = get_top_leader(
                    &social.following,
                    social,
                    &deleted,
                    follower_id,
                    None,
                )
                .unwrap_or(follower_id);
                fx.following_lines
                    .push(format_following_for_player(social, follower_id, top));
                fx.fan_following_all = true;

                // Haxe: `player.newFollower.say('I follow now ${player.name}…')`
                // `player` is the host whose timer fired (top or direct), not newFollowerFor.
                // Haxe: TimeHelper.DoTimeStuffForPlayer newFollowerTime confirm
                let (lname, lfam) = players
                    .values()
                    .find(|p| p.p_id == host_id)
                    .map(|p| (p.first_name.clone(), p.family_name.clone()))
                    .unwrap_or_else(|| ("?".into(), String::new()));
                let f_conn = players
                    .values()
                    .find(|p| p.p_id == follower_id)
                    .map(|p| p.conn_id)
                    .unwrap_or(0);
                fx.spoken_says
                    .push((follower_id, format_i_follow_now(&lname, &lfam)));
                if f_conn != 0 {
                    fx.private_ps.push((
                        f_conn,
                        format!("{follower_id} You follow now {lname} {lfam}"),
                    ));
                }
            }
        }

        // Clear pending on host and on newFollowerFor (Haxe clears both).
        clear_pending_follower_slots(players, host_id, follow_for_id);
    }

    fx
}

/// Haxe `setFollowPlayer`: set follow then refuse if getTopLeader is null (circular).
// Haxe: GlobalPlayerInstance.setFollowPlayer
fn try_set_follow_player(
    social: &mut SocialState,
    deleted: &HashSet<i32>,
    follower: i32,
    leader: i32,
) -> bool {
    let tmp = social.following.get(&follower).copied();
    if social.set_follow(follower, leader).is_err() {
        return false;
    }
    if get_top_leader(&social.following, social, deleted, follower, None).is_none() {
        match tmp {
            Some(prev) => {
                let _ = social.set_follow(follower, prev);
            }
            None => social.unfollow(follower),
        }
        return false;
    }
    true
}

fn clear_pending_follower_slots(
    players: &mut HashMap<u64, Player>,
    host_id: i32,
    follow_for_id: i32,
) {
    for pl in players.values_mut() {
        if pl.p_id == host_id || pl.p_id == follow_for_id {
            pl.new_follower_id = 0;
            pl.new_follower_for_id = 0;
            pl.new_follower_time = 0.0;
        }
    }
}

/// Returns `(coin_cost, combat_prestige_regain)` on success.
// Haxe: GlobalPlayerInstance.processHireCommand
fn try_hire(
    speaker: &Player,
    target: &NameCandidate,
    social: &mut SocialState,
    economy: &mut Economy,
    players: &mut HashMap<u64, Player>,
    lost_combat_prestige: f32,
    all: &[NameCandidate],
    hire_cost: i32,
    hire_cost_increase: i32,
) -> Result<(i32, f32), &'static str> {
    if !player_is_ai(target.connected, target.ai_controlled, &target.email) {
        return Err("human");
    }
    let prev_boss = social.hired_boss(target.p_id);
    if prev_boss == speaker.p_id {
        return Err("already_mine");
    }
    if prev_boss != 0 {
        if all.iter().any(|c| c.p_id == prev_boss && !c.deleted) {
            return Err("hired_other");
        }
    }
    if social.following.get(&target.p_id) == Some(&speaker.p_id) {
        return Err("follows_already");
    }
    if top_leader(&social.following, target.p_id) == speaker.p_id {
        return Err("already_follower");
    }
    // Haxe: processHireCommand getLeaderWhoExiled gate
    if social.leader_who_exiled(speaker.p_id, target.p_id).is_some() {
        return Err("exiled");
    }
    if !hire_angry_ok(target.angry_time) {
        return Err("too_angry");
    }
    hire_age_ok(target.age)?;
    let hirer_class = social.prestige_class(speaker.p_id).as_i32();
    let target_class = target.prestige_class;
    if !hire_class_ok(hirer_class, target_class) {
        return Err("class");
    }
    let ages: HashMap<i32, f32> = all.iter().map(|c| (c.p_id, c.age)).collect();
    let deleted: HashSet<i32> = all.iter().filter(|c| c.deleted).map(|c| c.p_id).collect();
    let hired_count = social.count_hired(speaker.p_id, &ages, &deleted);
    // Haxe: `player.isFriendly(this)` — ally + no mutual last-attack with hirer.
    // Haxe: GlobalPlayerInstance.isFriendly / processHireCommand neededCoins ×2
    let ally = is_leadership_ally(&social.following, target.p_id, speaker.p_id);
    let (tgt_last_atk, tgt_last_atk_me) = players
        .values()
        .find(|p| p.p_id == target.p_id)
        .map(|p| (p.last_attacked_player_id, p.last_player_attacked_me_id))
        .unwrap_or((0, 0));
    let friendly = is_friendly(ally, tgt_last_atk, tgt_last_atk_me, speaker.p_id);
    // Haxe: `player.getColor() != this.getColor()` — ObjectData.person race, not po_id.
    // Haxe: GlobalPlayerInstance.getColor
    let speaker_color = all
        .iter()
        .find(|c| c.p_id == speaker.p_id)
        .map(|c| c.person_color)
        .unwrap_or(0);
    let same_color = speaker_color == target.person_color;
    let close_rel = is_close_relative(social, speaker.p_id, target.p_id);
    let base = hire_cost.max(0);
    let increase = hire_cost_increase.max(0);
    let cost = compute_hire_cost(
        base,
        increase,
        target_class,
        friendly,
        same_color,
        close_rel,
        hired_count,
        lost_combat_prestige,
    );
    // Haxe: combatPrestigeImppact = ceil(lostCombatPrestige / 10); regain after pay
    let combat_impact = (lost_combat_prestige / 10.0).ceil().max(0.0);
    if economy.coins_of(speaker.p_id) < cost {
        return Err("need_coins");
    }
    // Haxe: setFollowPlayer immediate (not delayed newFollower) + circular probe
    // FOLLOW-HIRE-DELAY: hire still immediate (intentional Haxe parity)
    if !try_set_follow_player(social, &deleted, target.p_id, speaker.p_id) {
        return Err("circular_follow");
    }
    if !economy.gift(speaker.p_id, target.p_id, cost) {
        social.unfollow(target.p_id);
        return Err("need_coins");
    }
    social.set_hired(target.p_id, speaker.p_id);
    for pl in players.values_mut() {
        if pl.p_id == target.p_id {
            pl.home_x = speaker.home_x;
            pl.home_y = speaker.home_y;
            break;
        }
    }
    Ok((cost, combat_impact))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::speech::{do_command_broadcasts_chat, parse_do_command, DoCommand};
    use std::sync::{Arc, RwLock};

    #[test]
    fn parse_wired_forms() {
        assert!(parse_do_command("I EXILE BOB").is_some());
        assert!(parse_do_command("I HIRE BOB").is_some());
        assert!(parse_do_command("HOME!").is_some());
        assert!(!do_command_broadcasts_chat(&DoCommand::Redeem {
            name: "X".into()
        }));
    }

    fn cand(p: &Player, class: i32) -> NameCandidate {
        NameCandidate::from_player(p, class)
    }

    /// Ensure lineage exists before set_lineage_prestige_class (no-op without node).
    fn set_class(social: &mut SocialState, p_id: i32, name: &str, class: crate::prestige::PrestigeClass) {
        social.ensure_lineage(p_id, name);
        social.set_lineage_prestige_class(p_id, class);
    }

    #[test]
    fn follow_schedules_pending_not_immediate() {
        // Haxe: processFollowCommand sets newFollower*; following map unchanged until timer
        let mut social = SocialState::default();
        let mut economy = Economy::default();
        let mut players = HashMap::new();
        let mut a = Player::new(1, 1, "a@x");
        a.first_name = "ALICE".into();
        a.x = 0;
        a.y = 0;
        let mut b = Player::new(2, 2, "b@x");
        b.first_name = "BOB".into();
        b.x = 1;
        b.y = 0;
        let cands = vec![cand(&a, 0), cand(&b, 0)];
        players.insert(1, a.clone());
        players.insert(2, b.clone());
        let world = Arc::new(RwLock::new(World::new(32, 32, false)));
        let fx = apply_do_commands_live(
            "I FOLLOW BOB",
            &a,
            1,
            &mut social,
            &mut economy,
            &mut players,
            &world,
            0.0,
            &cands,
            FollowHireLiveKnobs::default(),
        );
        assert!(fx.recognized);
        assert!(
            social.following.get(&1).is_none(),
            "must not set_follow until confirm"
        );
        assert!(fx.following_lines.is_empty());
        let bob = players.get(&2).unwrap();
        assert_eq!(bob.new_follower_id, 1);
        assert_eq!(bob.new_follower_for_id, 2);
        assert!((bob.new_follower_time - TIME_CONFIRM_NEW_FOLLOWER).abs() < 1e-3);
        assert!(fx
            .private_ps
            .iter()
            .any(|(_, s)| s.contains("FOLLOWER") || s.contains("seconds")));
        assert!(fx
            .spoken_says
            .iter()
            .any(|(_, s)| s.contains("FOLLOW SOON")));
    }

    #[test]
    fn follow_busy_slot_refuses_second() {
        let mut social = SocialState::default();
        let mut economy = Economy::default();
        let mut players = HashMap::new();
        let mut a = Player::new(1, 1, "a@x");
        a.first_name = "ALICE".into();
        let mut b = Player::new(2, 2, "b@x");
        b.first_name = "BOB".into();
        b.x = 1;
        let mut c = Player::new(3, 3, "c@x");
        c.first_name = "CAROL".into();
        c.x = 1;
        // Bob already considering Alice
        b.new_follower_id = 1;
        b.new_follower_for_id = 2;
        b.new_follower_time = 10.0;
        let cands = vec![cand(&a, 0), cand(&b, 0), cand(&c, 0)];
        players.insert(1, a.clone());
        players.insert(2, b);
        players.insert(3, c.clone());
        let world = Arc::new(RwLock::new(World::new(32, 32, false)));
        let fx = apply_do_commands_live(
            "I FOLLOW BOB",
            &c,
            3,
            &mut social,
            &mut economy,
            &mut players,
            &world,
            0.0,
            &cands,
            FollowHireLiveKnobs::default(),
        );
        assert!(social.following.get(&3).is_none());
        assert!(fx
            .private_ps
            .iter()
            .any(|(_, s)| s.contains("considering") || s.contains("Try in")));
    }

    #[test]
    fn follow_confirm_after_timer_sets_follow() {
        let mut social = SocialState::default();
        let mut players = HashMap::new();
        let mut a = Player::new(1, 1, "a@x");
        a.first_name = "ALICE".into();
        a.new_follower_id = 0;
        let mut b = Player::new(2, 2, "b@x");
        b.first_name = "BOB".into();
        // pending: Alice → Bob
        b.new_follower_id = 1;
        b.new_follower_for_id = 2;
        b.new_follower_time = 0.5;
        players.insert(1, a);
        players.insert(2, b);
        let fx = tick_pending_new_followers(&mut players, &mut social, 1.0);
        assert_eq!(social.following.get(&1), Some(&2));
        assert!(!fx.following_lines.is_empty());
        assert!(fx.fan_following_all);
        assert_eq!(players.get(&2).unwrap().new_follower_id, 0);
    }

    #[test]
    fn follow_confirm_aborts_on_exile() {
        let mut social = SocialState::default();
        social.exile(2, 1); // Bob exiled Alice
        let mut players = HashMap::new();
        let mut a = Player::new(1, 1, "a@x");
        a.first_name = "ALICE".into();
        let mut b = Player::new(2, 2, "b@x");
        b.first_name = "BOB".into();
        b.new_follower_id = 1;
        b.new_follower_for_id = 2;
        b.new_follower_time = 0.1;
        players.insert(1, a);
        players.insert(2, b);
        let fx = tick_pending_new_followers(&mut players, &mut social, 1.0);
        assert!(social.following.get(&1).is_none());
        assert!(fx.following_lines.is_empty());
        assert_eq!(players.get(&2).unwrap().new_follower_id, 0);
    }

    #[test]
    fn follow_me_fw_leader_minus_one() {
        let mut social = SocialState::default();
        social.set_follow(1, 2).unwrap();
        let mut economy = Economy::default();
        let mut players = HashMap::new();
        let mut a = Player::new(1, 1, "a@x");
        a.first_name = "ALICE".into();
        let mut b = Player::new(2, 2, "b@x");
        b.first_name = "BOB".into();
        let cands = vec![cand(&a, 0), cand(&b, 0)];
        players.insert(1, a.clone());
        players.insert(2, b);
        let world = Arc::new(RwLock::new(World::new(32, 32, false)));
        let fx = apply_do_commands_live(
            "I FOLLOW ME",
            &a,
            1,
            &mut social,
            &mut economy,
            &mut players,
            &world,
            0.0,
            &cands,
            FollowHireLiveKnobs::default(),
        );
        assert!(!social.following.contains_key(&1));
        assert!(fx.fan_following_all);
        assert!(
            fx.following_lines.iter().any(|l| l.contains(" -1 ")),
            "lines={:?}",
            fx.following_lines
        );
    }

    /// FOLLOW-HIRE-DELAY: live TimeConfirmNewFollower used when scheduling pending.
    // Haxe: ServerSettings.TimeConfirmNewFollower
    #[test]
    fn follow_uses_live_time_confirm() {
        let mut social = SocialState::default();
        let mut economy = Economy::default();
        let mut players = HashMap::new();
        let mut a = Player::new(1, 1, "a@x");
        a.first_name = "ALICE".into();
        a.x = 0;
        a.y = 0;
        let mut b = Player::new(2, 2, "b@x");
        b.first_name = "BOB".into();
        b.x = 1;
        b.y = 0;
        let cands = vec![cand(&a, 0), cand(&b, 0)];
        players.insert(1, a.clone());
        players.insert(2, b.clone());
        let world = Arc::new(RwLock::new(World::new(32, 32, false)));
        let knobs = FollowHireLiveKnobs {
            time_confirm_new_follower: 7.0,
            ..Default::default()
        };
        let fx = apply_do_commands_live(
            "I FOLLOW BOB",
            &a,
            1,
            &mut social,
            &mut economy,
            &mut players,
            &world,
            0.0,
            &cands,
            knobs,
        );
        assert!(fx.recognized);
        assert!(social.following.get(&1).is_none());
        let bob = players.get(&2).unwrap();
        assert!((bob.new_follower_time - 7.0).abs() < 1e-3);
    }

    /// FOLLOW-HIRE-DELAY: I HIRE is immediate (Haxe setFollowPlayer, not newFollower delay).
    // Haxe: processHireCommand setFollowPlayer immediate
    #[test]
    fn hire_is_immediate_not_delayed() {
        let mut social = SocialState::default();
        let mut economy = Economy::default();
        economy.add_coins(1, 100);
        let mut players = HashMap::new();
        let mut boss = Player::new(1, 1, "boss@x");
        boss.first_name = "BOSS".into();
        boss.x = 0;
        boss.y = 0;
        boss.age = 20.0;
        boss.home_x = 5;
        boss.home_y = 5;
        let mut worker = Player::new(2, 2, "npc-w@local");
        worker.first_name = "WORKER".into();
        worker.x = 1;
        worker.y = 0;
        worker.age = 20.0;
        worker.ai_controlled = true;
        worker.connected = false;
        worker.email = "npc-w@local".into();
        set_class(&mut social, 1, "BOSS", crate::prestige::PrestigeClass::Commoner);
        set_class(&mut social, 2, "WORKER", crate::prestige::PrestigeClass::Commoner);
        let cands = vec![cand(&boss, 2), cand(&worker, 2)];
        players.insert(1, boss.clone());
        players.insert(2, worker.clone());
        let world = Arc::new(RwLock::new(World::new(32, 32, false)));
        let knobs = FollowHireLiveKnobs {
            hire_cost: 10,
            hire_cost_increase_per_person: 10,
            ..Default::default()
        };
        let fx = apply_do_commands_live(
            "I HIRE WORKER",
            &boss,
            1,
            &mut social,
            &mut economy,
            &mut players,
            &world,
            0.0,
            &cands,
            knobs,
        );
        assert!(fx.recognized);
        // Immediate follow — no pending newFollower slot
        assert_eq!(social.following.get(&2), Some(&1));
        assert_eq!(social.hired_boss(2), 1);
        assert_eq!(players.get(&2).unwrap().new_follower_id, 0);
        assert!(fx.fan_following_all);
        assert!(!fx.following_lines.is_empty());
        assert!(fx.spoken_says.iter().any(|(_, s)| s.contains("hire")));
        assert_eq!(players.get(&2).unwrap().home_x, 5);
        assert_eq!(players.get(&2).unwrap().home_y, 5);
    }

    /// FOLLOW-HIRE-DELAY: live HireCost multiplies into coin transfer.
    #[test]
    fn hire_uses_live_hire_cost() {
        let mut social = SocialState::default();
        let mut economy = Economy::default();
        economy.add_coins(1, 100);
        let mut players = HashMap::new();
        let mut boss = Player::new(1, 1, "boss@x");
        boss.first_name = "BOSS".into();
        boss.age = 20.0;
        let mut worker = Player::new(2, 2, "npc-w@local");
        worker.first_name = "WORKER".into();
        worker.x = 1;
        worker.age = 20.0;
        worker.ai_controlled = true;
        worker.connected = false;
        worker.email = "npc-w@local".into();
        set_class(&mut social, 1, "BOSS", crate::prestige::PrestigeClass::Noble);
        set_class(&mut social, 2, "WORKER", crate::prestige::PrestigeClass::Serf);
        let cands = vec![cand(&boss, 1), cand(&worker, 1)];
        players.insert(1, boss.clone());
        players.insert(2, worker);
        let world = Arc::new(RwLock::new(World::new(32, 32, false)));
        let knobs = FollowHireLiveKnobs {
            hire_cost: 30,
            hire_cost_increase_per_person: 0,
            ..Default::default()
        };
        let _fx = apply_do_commands_live(
            "I HIRE WORKER",
            &boss,
            1,
            &mut social,
            &mut economy,
            &mut players,
            &world,
            0.0,
            &cands,
            knobs,
        );
        assert_eq!(economy.coins_of(1), 70); // 100 - 30
        assert_eq!(economy.coins_of(2), 30);
    }

    #[test]
    fn follow_hire_knobs_from_gameplay() {
        let mut gp = GameplayKnobs::default();
        gp.time_confirm_new_follower = 3.5;
        gp.hire_cost = 12.4;
        gp.hire_cost_increase_per_person = 8.6;
        let k = FollowHireLiveKnobs::from_gameplay(&gp);
        assert!((k.time_confirm_new_follower - 3.5).abs() < 1e-3);
        assert_eq!(k.hire_cost, 12);
        assert_eq!(k.hire_cost_increase_per_person, 9);
    }

    /// Haxe: `isFriendly` false when lastAttacked breaks ally → foreign ×2 hire cost.
    // Haxe: processHireCommand neededCoins isFriendly + getColor
    #[test]
    fn hire_cost_hostile_ally_attacked() {
        let mut social = SocialState::default();
        // Shared top leader makes leadership allies.
        social.set_follow(1, 99).unwrap();
        social.set_follow(2, 99).unwrap();
        let mut economy = Economy::default();
        economy.add_coins(1, 200);
        let mut players = HashMap::new();
        let mut boss = Player::new(1, 1, "boss@x");
        boss.first_name = "BOSS".into();
        boss.age = 20.0;
        let mut worker = Player::new(2, 2, "npc-w@local");
        worker.first_name = "WORKER".into();
        worker.x = 1;
        worker.age = 20.0;
        worker.ai_controlled = true;
        worker.connected = false;
        worker.email = "npc-w@local".into();
        // Target attacked hirer → not friendly even though leadership ally.
        worker.last_attacked_player_id = 1;
        let mut top = Player::new(99, 99, "top@x");
        top.first_name = "TOP".into();
        // Same person race → only attack breaks friendly (not color).
        // Different person race would also ×2; here force via attack only + color ×2 path:
        // isFriendly false AND different color → ×2.
        let mut boss_c = cand(&boss, 1);
        boss_c.person_color = 4; // White
        let mut worker_c = cand(&worker, 1);
        worker_c.person_color = 1; // Black — foreign color
        let cands = vec![boss_c, worker_c, cand(&top, 0)];
        players.insert(1, boss.clone());
        players.insert(2, worker);
        players.insert(99, top);
        let world = Arc::new(RwLock::new(World::new(32, 32, false)));
        set_class(&mut social, 1, "BOSS", crate::prestige::PrestigeClass::Serf);
        set_class(&mut social, 2, "WORKER", crate::prestige::PrestigeClass::Serf);
        let knobs = FollowHireLiveKnobs {
            hire_cost: 10,
            hire_cost_increase_per_person: 0,
            ..Default::default()
        };
        let _fx = apply_do_commands_live(
            "I HIRE WORKER",
            &boss,
            1,
            &mut social,
            &mut economy,
            &mut players,
            &world,
            0.0,
            &cands,
            knobs,
        );
        // Serf base 10; not friendly + different color → ×2 = 20
        assert_eq!(economy.coins_of(1), 180);
        assert_eq!(economy.coins_of(2), 20);
        assert_eq!(social.hired_boss(2), 1);
    }

    /// Hire color uses ObjectData.person race, not display_object_id equality.
    // Haxe: GlobalPlayerInstance.getColor
    #[test]
    fn hire_cost_person_color_not_display_id() {
        let mut social = SocialState::default();
        let mut economy = Economy::default();
        economy.add_coins(1, 200);
        let mut players = HashMap::new();
        let mut boss = Player::new(1, 1, "boss@x");
        boss.first_name = "BOSS".into();
        boss.age = 20.0;
        boss.display_object_id = 100; // different display ids
        let mut worker = Player::new(2, 2, "npc-w@local");
        worker.first_name = "WORKER".into();
        worker.x = 1;
        worker.age = 20.0;
        worker.ai_controlled = true;
        worker.connected = false;
        worker.email = "npc-w@local".into();
        worker.display_object_id = 200;
        // Same person race → no foreign-color ×2 despite different display ids.
        let mut boss_c = cand(&boss, 1);
        boss_c.person_color = 4;
        let mut worker_c = cand(&worker, 1);
        worker_c.person_color = 4;
        let cands = vec![boss_c, worker_c];
        players.insert(1, boss.clone());
        players.insert(2, worker);
        let world = Arc::new(RwLock::new(World::new(32, 32, false)));
        set_class(&mut social, 1, "BOSS", crate::prestige::PrestigeClass::Serf);
        set_class(&mut social, 2, "WORKER", crate::prestige::PrestigeClass::Serf);
        let knobs = FollowHireLiveKnobs {
            hire_cost: 10,
            hire_cost_increase_per_person: 0,
            ..Default::default()
        };
        let _fx = apply_do_commands_live(
            "I HIRE WORKER",
            &boss,
            1,
            &mut social,
            &mut economy,
            &mut players,
            &world,
            0.0,
            &cands,
            knobs,
        );
        // not ally, but same person color → no ×2; cost stays 10
        assert_eq!(economy.coins_of(1), 190);
        assert_eq!(economy.coins_of(2), 10);
    }

    /// Confirm speech uses host (timer owner) name per Haxe TimeHelper.
    // Haxe: TimeHelper newFollowerTime `player.name` (host)
    #[test]
    fn follow_confirm_say_uses_host_name() {
        let mut social = SocialState::default();
        let mut players = HashMap::new();
        let mut a = Player::new(1, 1, "a@x");
        a.first_name = "ALICE".into();
        a.family_name = "SMITH".into();
        let mut b = Player::new(2, 2, "b@x");
        b.first_name = "BOB".into();
        b.family_name = "JONES".into();
        // pending: Alice → Bob; host is Bob
        b.new_follower_id = 1;
        b.new_follower_for_id = 2;
        b.new_follower_time = 0.1;
        players.insert(1, a);
        players.insert(2, b);
        let fx = tick_pending_new_followers(&mut players, &mut social, 1.0);
        assert_eq!(social.following.get(&1), Some(&2));
        assert!(
            fx.spoken_says
                .iter()
                .any(|(_, s)| s.contains("BOB") && s.contains("JONES")),
            "spoken={:?}",
            fx.spoken_says
        );
    }

    /// Hire success fans FW to all + happy emotes (already core; regression).
    // Haxe: processHireCommand SendFollowingToAll + doEmote happy
    #[test]
    fn hire_success_fan_following_all() {
        let mut social = SocialState::default();
        let mut economy = Economy::default();
        economy.add_coins(1, 100);
        let mut players = HashMap::new();
        let mut boss = Player::new(1, 1, "boss@x");
        boss.first_name = "BOSS".into();
        boss.age = 20.0;
        let mut worker = Player::new(2, 2, "npc-w@local");
        worker.first_name = "WORKER".into();
        worker.x = 1;
        worker.age = 20.0;
        worker.ai_controlled = true;
        worker.connected = false;
        worker.email = "npc-w@local".into();
        set_class(&mut social, 1, "BOSS", crate::prestige::PrestigeClass::Commoner);
        set_class(&mut social, 2, "WORKER", crate::prestige::PrestigeClass::Commoner);
        let mut boss_c = cand(&boss, 2);
        boss_c.person_color = 4;
        let mut worker_c = cand(&worker, 2);
        worker_c.person_color = 4;
        let cands = vec![boss_c, worker_c];
        players.insert(1, boss.clone());
        players.insert(2, worker);
        let world = Arc::new(RwLock::new(World::new(32, 32, false)));
        let fx = apply_do_commands_live(
            "I HIRE WORKER",
            &boss,
            1,
            &mut social,
            &mut economy,
            &mut players,
            &world,
            0.0,
            &cands,
            FollowHireLiveKnobs::default(),
        );
        assert!(fx.fan_following_all);
        assert!(!fx.following_lines.is_empty());
        assert!(fx.spoken_says.iter().any(|(_, s)| s.contains("hire")));
        assert!(fx.spoken_says.iter().any(|(_, s)| s.contains("hired me")));
    }

    /// try_set_follow_player refuses circular top (hire path).
    // Haxe: setFollowPlayer getTopLeader null
    #[test]
    fn hire_set_follow_refuses_exile_circular_top() {
        let mut social = SocialState::default();
        // Boss already follows worker → hiring worker would cycle.
        social.set_follow(1, 2).unwrap();
        let mut economy = Economy::default();
        economy.add_coins(1, 100);
        let mut players = HashMap::new();
        let mut boss = Player::new(1, 1, "boss@x");
        boss.first_name = "BOSS".into();
        boss.age = 20.0;
        let mut worker = Player::new(2, 2, "npc-w@local");
        worker.first_name = "WORKER".into();
        worker.x = 1;
        worker.age = 20.0;
        worker.ai_controlled = true;
        worker.connected = false;
        worker.email = "npc-w@local".into();
        set_class(&mut social, 1, "BOSS", crate::prestige::PrestigeClass::Commoner);
        set_class(&mut social, 2, "WORKER", crate::prestige::PrestigeClass::Commoner);
        let cands = vec![cand(&boss, 2), cand(&worker, 2)];
        players.insert(1, boss.clone());
        players.insert(2, worker);
        let world = Arc::new(RwLock::new(World::new(32, 32, false)));
        let fx = apply_do_commands_live(
            "I HIRE WORKER",
            &boss,
            1,
            &mut social,
            &mut economy,
            &mut players,
            &world,
            0.0,
            &cands,
            FollowHireLiveKnobs::default(),
        );
        assert!(
            fx.private_ps
                .iter()
                .any(|(_, s)| s.contains("circular") || s.contains("FAIL")),
            "ps={:?}",
            fx.private_ps
        );
        assert_ne!(social.hired_boss(2), 1);
        assert_eq!(economy.coins_of(1), 100); // no spend on refuse
    }
}
