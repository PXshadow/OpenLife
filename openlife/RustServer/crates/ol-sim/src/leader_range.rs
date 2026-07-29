//! LEADER-RANGE / leader_break + **PO-FAR-PLAYERS** / player_out_of_range.
//!
//! Haxe: `Connection.sendLeader` / `sendDirectLeader` / `Server` case LEAD /
//! `GlobalPlayerInstance` `!L`/`!LEADER`/`?L`/`!DL` / `sendToMePlayerInfo` top-leader
//! PU exemption (vanilla client breaks if leader is out of range).
//!
//! **PO-FAR-PLAYERS**: Haxe `Connection.SendToMeAllClosePlayers` +
//! `sendToMePlayerInfo` — viewer-centric sweep that sends **PO** for every far
//! non-leader and **PU+NAME** for close players / top leader (even when far).

use crate::clothing_transitions::format_clothing_set;
use crate::death_inherit::count_leadership_power;
use crate::leadership::{
    decide_player_info_range, decide_player_info_range_wrap, direct_follow_leader,
    format_leader_map_location_body, format_leader_power_say, format_no_leader_say,
    is_top_leader_of, PlayerInfoRangeDecision, MAX_DISTANCE_PU_CLOSE,
};
use crate::move_path::steps_to_client_path_deltas;
use crate::relations::top_leader;
use crate::SimState;
use ol_net::OutboundHub;
use ol_protocol::{
    format_player_moves_start, format_player_out_of_range, format_player_says,
    format_player_update_line_full_clothing, format_server_message,
};
use std::collections::HashMap;

/// Conn ids within Chebyshev `range` of `(x,y)`, **plus** any connected player
/// whose follow-chain top leader is `subject_p_id` (LEADER-RANGE exemption).
///
/// Haxe inverted fan-out: when broadcasting subject PU, far followers still need
/// the leader body so `/LEADER` / map pin does not break the vanilla client.
// Haxe: Connection.sendToMePlayerInfo topLeader exception
pub fn nearby_conn_ids_for_player_update(
    state: &SimState,
    x: i32,
    y: i32,
    subject_p_id: i32,
    range: i32,
) -> Vec<u64> {
    let mut ids = crate::nearby_conn_ids(state, x, y, range);
    if subject_p_id == 0 {
        return ids;
    }
    for (&cid, p) in &state.players {
        if !p.connected || p.deleted {
            continue;
        }
        if ids.contains(&cid) {
            continue;
        }
        if is_top_leader_of(&state.social.following, p.p_id, subject_p_id) {
            ids.push(cid);
        }
    }
    ids
}

/// Pure PO decision for one viewer watching one subject (tests + future bootstrap).
// Haxe: Connection.sendToMePlayerInfo L424-430
pub fn player_info_range_for_viewer(
    viewer_x: i32,
    viewer_y: i32,
    subject_x: i32,
    subject_y: i32,
    subject_p_id: i32,
    following: &HashMap<i32, i32>,
    viewer_p_id: i32,
    max_distance: i32,
) -> PlayerInfoRangeDecision {
    let tl = top_leader(following, viewer_p_id);
    decide_player_info_range(
        viewer_x,
        viewer_y,
        subject_x,
        subject_y,
        subject_p_id,
        tl,
        max_distance,
    )
}

/// Default max distance for pure range checks (product 20 when clamp is on).
pub fn default_pu_max_distance() -> i32 {
    MAX_DISTANCE_PU_CLOSE
}

/// Format full PO wire packet for a single far non-leader player.
pub fn po_packet_for(p_id: i32) -> Vec<u8> {
    format_player_out_of_range(&[p_id]).into_bytes()
}

// ─── PO-FAR-PLAYERS / player_out_of_range ────────────────────────────────────

/// One subject snapshot for pure viewer-centric `SendToMeAllClosePlayers` decisions.
// Haxe: Connection.sendToMePlayerInfo inputs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerInfoSubject {
    pub p_id: i32,
    pub x: i32,
    pub y: i32,
    pub deleted: bool,
    /// Haxe `isHeld()` — baby in arms (heldByPlayer != null).
    pub held: bool,
    pub moving: bool,
}

/// Wire action for one subject under a viewer (`SendToMeAllClosePlayers`).
// Haxe: Connection.sendToMePlayerInfo L414-448
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerSubjectAction {
    /// Deleted or held — neither PU nor PO.
    Skip,
    /// Far non-leader → `PLAYER_OUT_OF_RANGE` (PO).
    OutOfRange,
    /// Close or top-leader exempt.
    /// `include_pu`: false when subject is moving and `send_moving=false`
    /// (Haxe still sends NAME after; PU/PM skipped to avoid display bugs).
    InRange { include_pu: bool },
}

/// Haxe `sendToMePlayerInfo` range + held/deleted + moving gates (pure, plane).
// Haxe: Connection.sendToMePlayerInfo L414-448
pub fn decide_viewer_subject(
    viewer_x: i32,
    viewer_y: i32,
    subject: &PlayerInfoSubject,
    top_leader_p_id: i32,
    max_distance: i32,
    send_moving: bool,
) -> ViewerSubjectAction {
    decide_viewer_subject_wrap(
        viewer_x,
        viewer_y,
        subject,
        top_leader_p_id,
        max_distance,
        send_moving,
        0,
        0,
        false,
    )
}

/// Same as [`decide_viewer_subject`] with torus-aware distance (Haxe transformX/Y).
// Haxe: Connection.sendToMePlayerInfo L422-428 WorldMap.transformX/Y + isClose
pub fn decide_viewer_subject_wrap(
    viewer_x: i32,
    viewer_y: i32,
    subject: &PlayerInfoSubject,
    top_leader_p_id: i32,
    max_distance: i32,
    send_moving: bool,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> ViewerSubjectAction {
    if subject.deleted || subject.held {
        return ViewerSubjectAction::Skip;
    }
    match decide_player_info_range_wrap(
        viewer_x,
        viewer_y,
        subject.x,
        subject.y,
        subject.p_id,
        top_leader_p_id,
        max_distance,
        map_w,
        map_h,
        wrap,
    ) {
        PlayerInfoRangeDecision::SendOutOfRange => ViewerSubjectAction::OutOfRange,
        PlayerInfoRangeDecision::SendUpdate | PlayerInfoRangeDecision::SendUpdateLeaderExempt => {
            // Moving + !sendMoving → no PU/PM (Haxe still sends NAME later).
            let include_pu = !subject.moving || send_moving;
            ViewerSubjectAction::InRange { include_pu }
        }
    }
}

/// Collect p_ids that should receive PO for this viewer (far non-leaders, not held/deleted).
// Haxe: SendToMeAllClosePlayers → sendToMePlayerInfo PO branch
pub fn collect_far_non_leader_p_ids(
    viewer_x: i32,
    viewer_y: i32,
    top_leader_p_id: i32,
    subjects: &[PlayerInfoSubject],
    max_distance: i32,
) -> Vec<i32> {
    collect_far_non_leader_p_ids_wrap(
        viewer_x,
        viewer_y,
        top_leader_p_id,
        subjects,
        max_distance,
        0,
        0,
        false,
    )
}

/// Same as [`collect_far_non_leader_p_ids`] with torus wrap.
// Haxe: SendToMeAllClosePlayers + transformX/Y
pub fn collect_far_non_leader_p_ids_wrap(
    viewer_x: i32,
    viewer_y: i32,
    top_leader_p_id: i32,
    subjects: &[PlayerInfoSubject],
    max_distance: i32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> Vec<i32> {
    let mut out = Vec::new();
    for s in subjects {
        if matches!(
            decide_viewer_subject_wrap(
                viewer_x,
                viewer_y,
                s,
                top_leader_p_id,
                max_distance,
                true, // send_moving irrelevant for PO branch (range check first)
                map_w,
                map_h,
                wrap,
            ),
            ViewerSubjectAction::OutOfRange
        ) {
            out.push(s.p_id);
        }
    }
    out
}

/// Haxe `ServerSettings.SendMoveEveryXTicks` product default (`-1` = disabled).
/// When `> 0`, TimeHelper re-runs `sendToMeAllClosePlayers(false, false)` each N ticks.
// Haxe: ServerSettings.SendMoveEveryXTicks L261
pub const SEND_MOVE_EVERY_X_TICKS: i32 = -1;

/// Whether this tick should run the periodic viewer roster refresh.
// Haxe: TimeHelper.DoTimeStuff L132-135
#[inline]
pub fn should_refresh_close_players(tick: u64, every_x_ticks: i32) -> bool {
    every_x_ticks > 0 && tick % (every_x_ticks as u64) == 0
}

/// Live max-distance for viewer-centric PU/PO gate.
///
/// - `broadcast_all_updates` → `0` (always close; no PO) — full roster PU.
/// - else → [`crate::NEARBY_RANGE`] Euclidean `isClose` (practical interest cull).
///
/// Haxe product often sets `MaxDistanceToBeConsideredAsClose = 2_000_000` (PO rare);
/// tests pass an explicit clamp (e.g. 20).
// Haxe: ServerSettings.MaxDistanceToBeConsideredAsClose
pub fn pu_close_max_distance(state: &SimState) -> i32 {
    if state.broadcast_all_updates {
        0
    } else {
        crate::NEARBY_RANGE
    }
}

/// Haxe `Connection.SendToMeAllClosePlayers` — viewer-centric full roster fan.
///
/// For each living non-held subject:
/// - far **and** not viewer's top leader → **PO** `p_id`
/// - else → **PU** (+ **PM** when moving and `send_moving_player`) + **NM**
/// Ends with **FM** on the viewer connection.
///
/// Called on LOGIN (`send_moving_player=true`) and optional periodic refresh
/// (`send_moving_player=false` when `SendMoveEveryXTicks > 0`).
// Haxe: Connection.SendToMeAllClosePlayers L393-411 / sendToMePlayerInfo L414-448
pub fn send_to_me_all_close_players(
    state: &SimState,
    outbound: &OutboundHub,
    viewer_conn: u64,
    send_moving_player: bool,
) {
    let Some(viewer) = state.players.get(&viewer_conn) else {
        return;
    };
    if viewer.deleted {
        return;
    }
    let viewer_p_id = viewer.p_id;
    let viewer_x = viewer.x;
    let viewer_y = viewer.y;
    let following = &state.social.following;
    let top_leader_p_id = top_leader(following, viewer_p_id);
    let max_distance = pu_close_max_distance(state);
    // Haxe: transformX/Y before isClose — use world torus dims when wrap is on.
    let (map_w, map_h, wrap) = {
        let w = state.world.read().unwrap();
        (w.width_tiles, w.height_tiles, w.wrap)
    };

    // Snapshot subjects (avoid borrow issues while formatting).
    let subjects: Vec<_> = state
        .players
        .values()
        .filter(|p| !p.deleted)
        .map(|p| {
            (
                p.p_id,
                p.x,
                p.y,
                p.held_by != 0,
                p.moving || p.move_path.is_some(),
                p.first_name.clone(),
                p.family_name.clone(),
                crate::person_object_id(p),
                p.held_id,
                p.age,
                p.done_moving_seq,
                format_clothing_set(p),
                p.move_path.as_ref().map(|mp| {
                    (
                        mp.start_x,
                        mp.start_y,
                        mp.total_sec,
                        mp.trunc,
                        mp.remaining.iter().copied().collect::<Vec<_>>(),
                    )
                }),
            )
        })
        .collect();

    let mut far_po_ids: Vec<i32> = Vec::new();

    for (p_id, sx, sy, held, moving, first, family, po, held_id, age, seq, clothing, path_opt) in
        subjects
    {
        let subject = PlayerInfoSubject {
            p_id,
            x: sx,
            y: sy,
            deleted: false,
            held,
            moving,
        };
        match decide_viewer_subject_wrap(
            viewer_x,
            viewer_y,
            &subject,
            top_leader_p_id,
            max_distance,
            send_moving_player,
            map_w,
            map_h,
            wrap,
        ) {
            ViewerSubjectAction::Skip => {}
            ViewerSubjectAction::OutOfRange => {
                // Haxe: one PO per far player; multi-id PO is valid on the client.
                far_po_ids.push(p_id);
            }
            ViewerSubjectAction::InRange { include_pu } => {
                if include_pu {
                    // Relative coords for this viewer (Haxe transformX/Y).
                    let (rx, ry) = state
                        .players
                        .get(&viewer_conn)
                        .map(|v| v.world_to_client(sx, sy))
                        .unwrap_or((sx, sy));
                    // Speed from live player if still present.
                    let spd = state
                        .players
                        .values()
                        .find(|pl| pl.p_id == p_id)
                        .map(|pl| crate::player_move_speed(state, pl))
                        .unwrap_or(crate::WALK_MOVE_SPEED);
                    let force = if p_id == viewer_p_id { 1 } else { 0 };
                    let pu = format_player_update_line_full_clothing(
                        p_id,
                        po,
                        held_id,
                        rx,
                        ry,
                        age,
                        spd,
                        0,
                        0,
                        force,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        -1,
                        seq.max(1),
                        &clothing,
                    );
                    outbound.send(
                        viewer_conn,
                        format_server_message("PU", &[&pu]).into_bytes(),
                    );
                    // Moving + sendMoving: also PM (Haxe generateRelativeMoveUpdateString).
                    if moving {
                        if let Some((start_x, start_y, total, trunc, steps)) = path_opt {
                            let (pm_rx, pm_ry) = state
                                .players
                                .get(&viewer_conn)
                                .map(|v| v.world_to_client(start_x, start_y))
                                .unwrap_or((start_x, start_y));
                            let wire_deltas = steps_to_client_path_deltas(&steps);
                            let pm = format_player_moves_start(
                                p_id,
                                pm_rx,
                                pm_ry,
                                total,
                                total,
                                trunc,
                                &wire_deltas,
                            );
                            outbound.send(viewer_conn, pm.into_bytes());
                        }
                    }
                }
                // Haxe always sends NAME after the moving branch (even when PU skipped).
                let nm_line = format!("{p_id} {first} {family}");
                outbound.send(
                    viewer_conn,
                    format_server_message("NM", &[&nm_line]).into_bytes(),
                );
            }
        }
    }

    if !far_po_ids.is_empty() {
        outbound.send(
            viewer_conn,
            format_player_out_of_range(&far_po_ids).into_bytes(),
        );
    }
    // Haxe: player.connection.send(FRAME) after the loop.
    outbound.send(viewer_conn, format_server_message("FM", &[]).into_bytes());
}

/// Periodic / bulk: `SendToMeAllClosePlayers` for every connected non-deleted viewer.
///
/// Haxe TimeHelper uses `sendMovingPlayer=false` so movers skip PU/PM (NAME still sent).
// Haxe: TimeHelper.DoTimeStuff L132-135 sendToMeAllClosePlayers(false, false)
pub fn send_to_me_all_close_players_all_viewers(
    state: &SimState,
    outbound: &OutboundHub,
    send_moving_player: bool,
) {
    let cids: Vec<u64> = state
        .players
        .iter()
        .filter(|(_, p)| p.connected && !p.deleted)
        .map(|(&c, _)| c)
        .collect();
    for cid in cids {
        send_to_me_all_close_players(state, outbound, cid, send_moving_player);
    }
}

/// Snapshot needed to build LEAD / !LEADER replies without holding more borrows.
#[derive(Debug, Clone)]
pub struct LeaderPinSnapshot {
    pub viewer_p_id: i32,
    pub leader_p_id: i32,
    pub leader_first: String,
    pub leader_family: String,
    pub power: f32,
    pub rel_x: i32,
    pub rel_y: i32,
}

/// Resolve top leader (or direct follow when `direct`) for map pin + power say.
pub fn resolve_leader_pin(
    state: &SimState,
    viewer_conn: u64,
    direct: bool,
) -> Result<LeaderPinSnapshot, &'static str> {
    let viewer = state
        .players
        .get(&viewer_conn)
        .filter(|p| !p.deleted)
        .ok_or("no_player")?;
    let viewer_p_id = viewer.p_id;
    let leader_p_id = if direct {
        match direct_follow_leader(&state.social.following, viewer_p_id) {
            Some(id) => id,
            None => return Err("no_direct_leader"),
        }
    } else {
        top_leader(&state.social.following, viewer_p_id)
    };

    let leader = state
        .players
        .values()
        .find(|p| p.p_id == leader_p_id && !p.deleted)
        .ok_or("leader_missing")?;

    let prestige = state.player_prestige(leader_p_id);
    let class = state.player_prestige_class(leader_p_id);
    let coins = state.economy.coins_of(leader_p_id);
    let founder = crate::relations::root_eve_id(&state.social, leader_p_id);
    let family_prestige = state.accounts.family_prestige_for(&leader.email, founder);
    let power = count_leadership_power(prestige, coins, family_prestige, class);
    let (rel_x, rel_y) = viewer.world_to_client(leader.x, leader.y);

    Ok(LeaderPinSnapshot {
        viewer_p_id,
        leader_p_id,
        leader_first: leader.first_name.clone(),
        leader_family: leader.family_name.clone(),
        power,
        rel_x,
        rel_y,
    })
}

/// Haxe `sendLeader` / `sendDirectLeader` — PS map pin only.
// Haxe: Connection.sendLeaderHelper / sendDirectLeader
pub fn send_leader_map_pin(outbound: &OutboundHub, conn_id: u64, pin: &LeaderPinSnapshot) {
    let body = format_leader_map_location_body(pin.leader_p_id, pin.rel_x, pin.rel_y);
    let ps = format_player_says(pin.viewer_p_id, false, &body);
    outbound.send(conn_id, ps.into_bytes());
    outbound.send(conn_id, format_server_message("FM", &[]).into_bytes());
}

/// Haxe `!L` / `!LEADER` — power say + map pin.
// Haxe: GlobalPlayerInstance doCommands !LEADER
pub fn send_leader_power_and_pin(outbound: &OutboundHub, conn_id: u64, pin: &LeaderPinSnapshot) {
    let say = format_leader_power_say(&pin.leader_first, &pin.leader_family, pin.power);
    let ps_say = format_player_says(pin.viewer_p_id, false, &say);
    outbound.send(conn_id, ps_say.into_bytes());
    send_leader_map_pin(outbound, conn_id, pin);
}

/// Haxe `?L` / `?LEADER` personal — power say only (no map).
pub fn send_leader_power_only(outbound: &OutboundHub, conn_id: u64, pin: &LeaderPinSnapshot) {
    let say = format_leader_power_say(&pin.leader_first, &pin.leader_family, pin.power);
    let ps_say = format_player_says(pin.viewer_p_id, false, &say);
    outbound.send(conn_id, ps_say.into_bytes());
    outbound.send(conn_id, format_server_message("FM", &[]).into_bytes());
}

/// "No leader!" private PS.
pub fn send_no_leader(outbound: &OutboundHub, conn_id: u64, viewer_p_id: i32) {
    let ps = format_player_says(viewer_p_id, false, format_no_leader_say());
    outbound.send(conn_id, ps.into_bytes());
    outbound.send(conn_id, format_server_message("FM", &[]).into_bytes());
}

/// Apply LEAD tag or !L / !LEADER / ?L / !DL / ?DL style commands.
///
/// - `direct`: use followPlayer (direct) vs getTopLeader
/// - `with_map`: send map pin (LEAD / !L / !DL)
/// - `with_power`: send power say (?L / !L / ?DL / !DL)
pub fn apply_leader_query(
    state: &SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    direct: bool,
    with_map: bool,
    with_power: bool,
) {
    match resolve_leader_pin(state, conn_id, direct) {
        Ok(pin) => {
            if with_power && with_map {
                send_leader_power_and_pin(outbound, conn_id, &pin);
            } else if with_power {
                send_leader_power_only(outbound, conn_id, &pin);
            } else if with_map {
                send_leader_map_pin(outbound, conn_id, &pin);
            }
        }
        Err("no_direct_leader") | Err("leader_missing") => {
            let vid = state
                .players
                .get(&conn_id)
                .map(|p| p.p_id)
                .unwrap_or(0);
            send_no_leader(outbound, conn_id, vid);
        }
        Err(_) => {}
    }
}

/// Parse SPEECH for LEADER-RANGE personal commands (not ranking `LEADER id:n`).
///
/// Returns `Some((direct, with_map, with_power))` when handled.
pub fn parse_leader_personal_command(upper: &str) -> Option<(bool, bool, bool)> {
    // Exact short forms
    if upper == "!L" || upper == "!LEADER" || upper.starts_with("!LEADER ") {
        return Some((false, true, true)); // top + map + power
    }
    if upper == "?L" {
        return Some((false, false, true)); // top + power only
    }
    // Personal ?LEADER name-power (Haxe); ranking uses bare LEADER without ?
    // Avoid stealing ?LEADERBOARD — only exact ?LEADER
    if upper == "?LEADER" {
        return Some((false, false, true));
    }
    if upper == "!DL" || upper == "!DLEADER" || upper.starts_with("!DLEADER ") {
        return Some((true, true, true));
    }
    if upper == "?DL" || upper == "?DLEADER" || upper.starts_with("?DLEADER ") {
        return Some((true, false, true));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::leadership::{decide_player_info_range, PlayerInfoRangeDecision};
    use crate::spawn_player;
    use ol_content::ContentDb;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn parse_leader_personal_forms() {
        assert_eq!(
            parse_leader_personal_command("!L"),
            Some((false, true, true))
        );
        assert_eq!(
            parse_leader_personal_command("?L"),
            Some((false, false, true))
        );
        assert_eq!(
            parse_leader_personal_command("!DL"),
            Some((true, true, true))
        );
        assert_eq!(parse_leader_personal_command("?LEADERBOARD"), None);
        assert_eq!(
            parse_leader_personal_command("?LEADER"),
            Some((false, false, true))
        );
    }

    #[test]
    fn po_packet_shape() {
        let s = String::from_utf8(po_packet_for(42)).unwrap();
        assert_eq!(s, "PO\n42\n#");
    }

    #[test]
    fn range_decision_uses_top_leader() {
        let mut following = HashMap::new();
        following.insert(2, 10);
        // viewer 2, subject 99 far → PO
        assert_eq!(
            player_info_range_for_viewer(0, 0, 100, 100, 99, &following, 2, 20),
            PlayerInfoRangeDecision::SendOutOfRange
        );
        // subject is top leader 10 far → exempt
        assert_eq!(
            player_info_range_for_viewer(0, 0, 100, 100, 10, &following, 2, 20),
            PlayerInfoRangeDecision::SendUpdateLeaderExempt
        );
        let _ = decide_player_info_range(0, 0, 0, 0, 1, 1, 20);
    }

    // ─── PO-FAR-PLAYERS pure ───────────────────────────────────────────────

    #[test]
    fn decide_viewer_subject_far_non_leader_is_po() {
        let s = PlayerInfoSubject {
            p_id: 99,
            x: 100,
            y: 100,
            deleted: false,
            held: false,
            moving: false,
        };
        assert_eq!(
            decide_viewer_subject(0, 0, &s, 1, 20, true),
            ViewerSubjectAction::OutOfRange
        );
    }

    #[test]
    fn decide_viewer_subject_far_top_leader_is_pu() {
        let s = PlayerInfoSubject {
            p_id: 1,
            x: 100,
            y: 100,
            deleted: false,
            held: false,
            moving: false,
        };
        assert_eq!(
            decide_viewer_subject(0, 0, &s, 1, 20, true),
            ViewerSubjectAction::InRange { include_pu: true }
        );
    }

    #[test]
    fn decide_viewer_subject_held_skipped() {
        let s = PlayerInfoSubject {
            p_id: 5,
            x: 0,
            y: 0,
            deleted: false,
            held: true,
            moving: false,
        };
        assert_eq!(
            decide_viewer_subject(0, 0, &s, 1, 20, true),
            ViewerSubjectAction::Skip
        );
    }

    #[test]
    fn decide_viewer_subject_moving_no_send_skips_pu() {
        let s = PlayerInfoSubject {
            p_id: 3,
            x: 1,
            y: 0,
            deleted: false,
            held: false,
            moving: true,
        };
        assert_eq!(
            decide_viewer_subject(0, 0, &s, 1, 20, false),
            ViewerSubjectAction::InRange { include_pu: false }
        );
        assert_eq!(
            decide_viewer_subject(0, 0, &s, 1, 20, true),
            ViewerSubjectAction::InRange { include_pu: true }
        );
    }

    #[test]
    fn collect_far_non_leader_p_ids_filters() {
        let subjects = [
            PlayerInfoSubject {
                p_id: 1, // top leader far — exempt
                x: 100,
                y: 100,
                deleted: false,
                held: false,
                moving: false,
            },
            PlayerInfoSubject {
                p_id: 2, // close
                x: 1,
                y: 0,
                deleted: false,
                held: false,
                moving: false,
            },
            PlayerInfoSubject {
                p_id: 3, // far non-leader
                x: 100,
                y: 0,
                deleted: false,
                held: false,
                moving: false,
            },
            PlayerInfoSubject {
                p_id: 4, // far but held
                x: 200,
                y: 0,
                deleted: false,
                held: true,
                moving: false,
            },
            PlayerInfoSubject {
                p_id: 5, // far deleted
                x: 200,
                y: 200,
                deleted: true,
                held: false,
                moving: false,
            },
        ];
        let far = collect_far_non_leader_p_ids(0, 0, 1, &subjects, 20);
        assert_eq!(far, vec![3]);
    }

    #[test]
    fn max_distance_zero_means_no_po() {
        let s = PlayerInfoSubject {
            p_id: 99,
            x: 1_000_000,
            y: 1_000_000,
            deleted: false,
            held: false,
            moving: false,
        };
        assert_eq!(
            decide_viewer_subject(0, 0, &s, 1, 0, true),
            ViewerSubjectAction::InRange { include_pu: true }
        );
    }

    /// Live wire: far non-leader gets PO; far top leader gets PU (LEADER-RANGE).
    #[test]
    fn send_to_me_all_close_players_po_for_far_non_leaders() {
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut state = SimState::with_default_empty(Arc::new(ContentDb::default()));
        // Interest cull on so far subjects get PO.
        state.broadcast_all_updates = false;
        spawn_player(&mut state, 1, "view@t");
        spawn_player(&mut state, 2, "far@t");
        spawn_player(&mut state, 3, "lead@t");
        {
            let v = state.players.get_mut(&1).unwrap();
            v.x = 0;
            v.y = 0;
            v.connected = true;
        }
        {
            let f = state.players.get_mut(&2).unwrap();
            // Mid-map: still far on plane *and* after torus wrap (512×512 default).
            f.x = 256;
            f.y = 256;
            f.connected = true;
        }
        {
            let l = state.players.get_mut(&3).unwrap();
            l.x = 200;
            l.y = 200;
            l.connected = true;
            l.first_name = "LEAD".into();
        }
        let viewer_pid = state.players.get(&1).unwrap().p_id;
        let far_pid = state.players.get(&2).unwrap().p_id;
        let lead_pid = state.players.get(&3).unwrap().p_id;
        let _ = state.social.set_follow(viewer_pid, lead_pid);

        send_to_me_all_close_players(&state, &hub, 1, true);

        let mut saw_po_far = false;
        let mut saw_po_leader = false;
        let mut saw_pu_leader = false;
        let mut saw_fm = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PO\n") {
                if s.contains(&far_pid.to_string()) {
                    saw_po_far = true;
                }
                // top leader must NOT appear as a PO id token
                let body = s.trim_start_matches("PO\n").trim_end_matches("\n#");
                if body
                    .split_whitespace()
                    .any(|t| t == lead_pid.to_string())
                {
                    saw_po_leader = true;
                }
            }
            if s.starts_with("PU\n") && s.contains(&format!("{lead_pid} ")) {
                saw_pu_leader = true;
            }
            if s.starts_with("FM\n") || s == "FM\n#" {
                saw_fm = true;
            }
        }
        assert!(saw_po_far, "far non-leader must receive PO");
        assert!(!saw_po_leader, "top leader must not be PO'd when far");
        assert!(saw_pu_leader, "far top leader still gets PU (LEADER-RANGE)");
        assert!(saw_fm, "SendToMeAllClosePlayers ends with FRAME");
    }

    #[test]
    fn pu_close_max_distance_broadcast_all_disables_po() {
        let mut state = SimState::with_default_empty(Arc::new(ContentDb::default()));
        state.broadcast_all_updates = true;
        assert_eq!(pu_close_max_distance(&state), 0);
        state.broadcast_all_updates = false;
        assert_eq!(pu_close_max_distance(&state), crate::NEARBY_RANGE);
    }

    /// Torus edge within NEARBY_RANGE after wrap → InRange (not PO).
    // Haxe: transformX/Y before isClose when max_distance is small (20–24)
    #[test]
    fn decide_viewer_subject_wrap_torus_edge_in_range() {
        let s = PlayerInfoSubject {
            p_id: 99,
            x: 511, // default empty world is 512×512 wrap
            y: 0,
            deleted: false,
            held: false,
            moving: false,
        };
        // Plane: far → PO
        assert_eq!(
            decide_viewer_subject(0, 0, &s, 1, 24, true),
            ViewerSubjectAction::OutOfRange
        );
        // Torus: dx=-1 → close
        assert_eq!(
            decide_viewer_subject_wrap(0, 0, &s, 1, 24, true, 512, 512, true),
            ViewerSubjectAction::InRange { include_pu: true }
        );
        let far = collect_far_non_leader_p_ids_wrap(0, 0, 1, &[s], 24, 512, 512, true);
        assert!(far.is_empty(), "edge neighbor must not be PO on wrap map");
    }

    #[test]
    fn should_refresh_close_players_gate() {
        // Haxe default SendMoveEveryXTicks = -1 → never
        assert!(!should_refresh_close_players(0, SEND_MOVE_EVERY_X_TICKS));
        assert!(!should_refresh_close_players(90, -1));
        assert!(!should_refresh_close_players(90, 0));
        // every 90 ticks
        assert!(should_refresh_close_players(0, 90));
        assert!(!should_refresh_close_players(1, 90));
        assert!(should_refresh_close_players(90, 90));
        assert!(should_refresh_close_players(180, 90));
    }

    /// Live: torus edge subject gets PU not PO when world.wrap.
    #[test]
    fn send_to_me_all_close_players_torus_edge_not_po() {
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut state = SimState::with_default_empty(Arc::new(ContentDb::default()));
        // 512×512 wrap is default empty world
        state.broadcast_all_updates = false;
        spawn_player(&mut state, 1, "view@t");
        spawn_player(&mut state, 2, "edge@t");
        {
            let v = state.players.get_mut(&1).unwrap();
            v.x = 0;
            v.y = 0;
            v.connected = true;
        }
        {
            let e = state.players.get_mut(&2).unwrap();
            e.x = 511; // one tile across wrap from 0
            e.y = 0;
            e.connected = true;
        }
        let edge_pid = state.players.get(&2).unwrap().p_id;

        send_to_me_all_close_players(&state, &hub, 1, true);

        let mut saw_po_edge = false;
        let mut saw_pu_edge = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PO\n") {
                let body = s.trim_start_matches("PO\n").trim_end_matches("\n#");
                if body
                    .split_whitespace()
                    .any(|t| t == edge_pid.to_string())
                {
                    saw_po_edge = true;
                }
            }
            if s.starts_with("PU\n") && s.contains(&format!("{edge_pid} ")) {
                saw_pu_edge = true;
            }
        }
        assert!(!saw_po_edge, "wrap-edge neighbor must not be PO");
        assert!(saw_pu_edge, "wrap-edge neighbor must get PU");
    }

    /// send_moving=false skips PU for movers but still NM; used by periodic refresh.
    #[test]
    fn send_to_me_all_close_players_send_moving_false_skips_mover_pu() {
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut state = SimState::with_default_empty(Arc::new(ContentDb::default()));
        state.broadcast_all_updates = false;
        spawn_player(&mut state, 1, "view@t");
        spawn_player(&mut state, 2, "move@t");
        {
            let v = state.players.get_mut(&1).unwrap();
            v.x = 0;
            v.y = 0;
            v.connected = true;
        }
        {
            let m = state.players.get_mut(&2).unwrap();
            m.x = 1;
            m.y = 0;
            m.connected = true;
            m.moving = true;
        }
        let mover_pid = state.players.get(&2).unwrap().p_id;

        send_to_me_all_close_players(&state, &hub, 1, false);

        let mut saw_pu_mover = false;
        let mut saw_nm_mover = false;
        while let Ok(pkt) = rx1.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.starts_with("PU\n") && s.contains(&format!("{mover_pid} ")) {
                saw_pu_mover = true;
            }
            if s.starts_with("NM\n") && s.contains(&format!("{mover_pid} ")) {
                saw_nm_mover = true;
            }
        }
        assert!(!saw_pu_mover, "moving + !send_moving must skip PU");
        assert!(saw_nm_mover, "NAME still sent for in-range mover");
    }
}
