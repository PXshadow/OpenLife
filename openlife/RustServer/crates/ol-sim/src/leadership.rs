//! Leadership election over follow graphs (Haxe FOLLOWING leaders subset).
//!
//! Pure: leaders are nodes with inbound follows; ranked by follower count.
//!
//! **LEADER-RANGE / leader_break** — Haxe `Connection.sendToMePlayerInfo` +
//! `sendLeader` / `sendDirectLeader` / `LEAD` / `!LEADER`:
//! vanilla client breaks if `/LEADER` is used while the top leader is out of
//! PU range, so far **top leaders still receive PU** (no PO for them).

use std::collections::HashMap;

/// One elected leader: `leader_p_id` with `follower_count` direct followers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeaderEntry {
    pub leader_id: i32,
    pub followers: usize,
}

/// Default top-N for `?LEADER` ranking query.
pub const LEADER_QUERY_LIMIT: usize = 10;

/// Haxe `ServerSettings.MaxDistanceToBeConsideredAsClose` **product** default when
/// range clamp is on (commented as `20` in Haxe; live value is often huge to disable).
///
/// Used by [`decide_player_info_range`] squared-Euclidean `isClose` gate.
// Haxe: ServerSettings.MaxDistanceToBeConsideredAsClose
pub const MAX_DISTANCE_PU_CLOSE: i32 = 20;

/// How many followers currently point at `leader_id`.
pub fn follower_count(following: &HashMap<i32, i32>, leader_id: i32) -> usize {
    following.values().filter(|&&l| l == leader_id).count()
}

/// True if anyone follows `p_id` (i.e. they are a leader of at least one).
pub fn is_leader(following: &HashMap<i32, i32>, p_id: i32) -> bool {
    following.values().any(|&l| l == p_id)
}

/// Rank leaders from a follower → leader map.
///
/// Counts how many followers point at each leader, sorts by follower count
/// descending (tie-break: lower leader id first), returns top `limit` entries.
pub fn rank_leaders(following: &HashMap<i32, i32>, limit: usize) -> Vec<LeaderEntry> {
    if limit == 0 || following.is_empty() {
        return Vec::new();
    }
    let mut counts: HashMap<i32, usize> = HashMap::new();
    for &leader in following.values() {
        *counts.entry(leader).or_default() += 1;
    }
    let mut entries: Vec<LeaderEntry> = counts
        .into_iter()
        .map(|(leader_id, followers)| LeaderEntry {
            leader_id,
            followers,
        })
        .collect();
    entries.sort_by(|a, b| {
        b.followers
            .cmp(&a.followers)
            .then_with(|| a.leader_id.cmp(&b.leader_id))
    });
    entries.truncate(limit);
    entries
}

/// `SAY LEADER` ranking chat reply body (without leading player id).
///
/// Format: `LEADER id:n id:n …` or `LEADER none` when empty.
///
/// Note: Haxe `?L` / `?LEADER` personal power say is
/// [`format_leader_power_say`]; this is the follow-graph rank list.
pub fn format_leader_query(following: &HashMap<i32, i32>, limit: usize) -> String {
    let ranked = rank_leaders(following, limit);
    if ranked.is_empty() {
        return "LEADER none".into();
    }
    let parts: Vec<String> = ranked
        .iter()
        .map(|e| format!("{}:{}", e.leader_id, e.followers))
        .collect();
    format!("LEADER {}", parts.join(" "))
}

/// True if `follower_id` currently follows `leader_id` directly (not via chain).
pub fn is_direct_follower(
    following: &HashMap<i32, i32>,
    leader_id: i32,
    follower_id: i32,
) -> bool {
    following.get(&follower_id) == Some(&leader_id)
}

/// Private PS body delivered to the ordered follower (no PS framing).
///
/// Format: `{leader_id} ORDER {text}`
pub fn format_order_delivery(leader_id: i32, text: &str) -> String {
    format!("{leader_id} ORDER {text}")
}

/// Speaker private-PS result for `SAY ORDER`.
///
/// OK: `{speaker} ORDER {target} OK`  
/// FAIL: `{speaker} ORDER FAIL {reason}`
pub fn format_order_result(speaker_id: i32, target_id: i32, ok: bool, reason: &str) -> String {
    if ok {
        format!("{speaker_id} ORDER {target_id} OK")
    } else {
        format!("{speaker_id} ORDER FAIL {reason}")
    }
}

/// Notify leader that a follower acknowledged: `{follower_id} OBEY`
pub fn format_obey_notify(follower_id: i32) -> String {
    format!("{follower_id} OBEY")
}

/// Speaker private-PS result for `SAY OBEY`.
pub fn format_obey_result(speaker_id: i32, ok: bool, reason: &str) -> String {
    if ok {
        format!("{speaker_id} OBEY OK")
    } else {
        format!("{speaker_id} OBEY FAIL {reason}")
    }
}

/// Notify former leader that a follower broke follow: `{follower_id} DISOBEY`
pub fn format_disobey_notify(follower_id: i32) -> String {
    format!("{follower_id} DISOBEY")
}

/// Speaker private-PS result for `SAY DISOBEY`.
pub fn format_disobey_result(speaker_id: i32, ok: bool, reason: &str) -> String {
    if ok {
        format!("{speaker_id} DISOBEY OK")
    } else {
        format!("{speaker_id} DISOBEY FAIL {reason}")
    }
}

// ─── LEADER-RANGE / leader_break ────────────────────────────────────────────

/// Outcome of Haxe `Connection.sendToMePlayerInfo` range gate (L424–430).
// Haxe: Connection.sendToMePlayerInfo
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerInfoRangeDecision {
    /// Within range → send PU (+ NAME).
    SendUpdate,
    /// Out of range **but** `playerToSend == topLeader` → still send PU
    /// (vanilla client LEADER-out-of-range break workaround).
    SendUpdateLeaderExempt,
    /// Out of range and not top leader → `PLAYER_OUT_OF_RANGE` (PO), skip PU.
    SendOutOfRange,
}

/// Squared-Euclidean close check on the plane (no wrap).
///
/// `max_distance <= 0` is treated as always-close (matches “clamp disabled”).
// Haxe: GlobalPlayerInstance.isClose → AiHelper.CalculateDistance
#[inline]
pub fn is_close_pu(
    viewer_x: i32,
    viewer_y: i32,
    target_x: i32,
    target_y: i32,
    max_distance: i32,
) -> bool {
    is_close_pu_wrap(
        viewer_x, viewer_y, target_x, target_y, max_distance, 0, 0, false,
    )
}

/// Squared-Euclidean close check with optional torus shortest-path.
///
/// Haxe `sendToMePlayerInfo` feeds `WorldMap.transformX/Y` (wrap relative) into
/// `isClose`; Rust stores world coords, so wrap here is equivalent for distance.
// Haxe: Connection.sendToMePlayerInfo L422-428 + WorldMap.transformX/Y
// Haxe: GlobalPlayerInstance.isClose L1779 TODO round map — wrap when map wraps
#[inline]
pub fn is_close_pu_wrap(
    viewer_x: i32,
    viewer_y: i32,
    target_x: i32,
    target_y: i32,
    max_distance: i32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> bool {
    if max_distance <= 0 {
        return true;
    }
    let (dx, dy) =
        crate::math_wrap::wrap_delta(viewer_x, viewer_y, target_x, target_y, map_w, map_h, wrap);
    let max = max_distance as i64;
    let dx = dx as i64;
    let dy = dy as i64;
    dx * dx + dy * dy <= max * max
}

/// Haxe `sendToMePlayerInfo` range + top-leader exception.
///
/// ```text
/// if (playerToSend != topLeader && !isClose(...)) → PO + skip
/// else → PU
/// ```
///
/// When `target_p_id == top_leader_p_id` (including self-as-leader), out-of-range
/// still yields [`PlayerInfoRangeDecision::SendUpdateLeaderExempt`].
// Haxe: Connection.sendToMePlayerInfo L424-430 TODO vanilla client breaks
pub fn decide_player_info_range(
    viewer_x: i32,
    viewer_y: i32,
    target_x: i32,
    target_y: i32,
    target_p_id: i32,
    top_leader_p_id: i32,
    max_distance: i32,
) -> PlayerInfoRangeDecision {
    decide_player_info_range_wrap(
        viewer_x,
        viewer_y,
        target_x,
        target_y,
        target_p_id,
        top_leader_p_id,
        max_distance,
        0,
        0,
        false,
    )
}

/// Same as [`decide_player_info_range`] with torus-aware `isClose`.
// Haxe: Connection.sendToMePlayerInfo L422-428 transformX/Y + isClose
pub fn decide_player_info_range_wrap(
    viewer_x: i32,
    viewer_y: i32,
    target_x: i32,
    target_y: i32,
    target_p_id: i32,
    top_leader_p_id: i32,
    max_distance: i32,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> PlayerInfoRangeDecision {
    let close = is_close_pu_wrap(
        viewer_x,
        viewer_y,
        target_x,
        target_y,
        max_distance,
        map_w,
        map_h,
        wrap,
    );
    if close {
        return PlayerInfoRangeDecision::SendUpdate;
    }
    if target_p_id == top_leader_p_id {
        return PlayerInfoRangeDecision::SendUpdateLeaderExempt;
    }
    PlayerInfoRangeDecision::SendOutOfRange
}

/// True when `subject_p_id` is the follow-chain top leader of `viewer_p_id`
/// (or is the viewer themself when unfollowed).
// Haxe: getTopLeader; used for inverted PU fan-out leader exemption
pub fn is_top_leader_of(
    following: &HashMap<i32, i32>,
    viewer_p_id: i32,
    subject_p_id: i32,
) -> bool {
    if viewer_p_id == 0 || subject_p_id == 0 {
        return false;
    }
    crate::relations::top_leader(following, viewer_p_id) == subject_p_id
}

/// Haxe `Connection.sendMapLocation` **body** (inside PLAYER_SAYS text).
///
/// Full PS line: `{viewer_p_id}/0 {text1} *{text2} {target_p_id} *map {rel_x} {rel_y}`
// Haxe: Connection.sendMapLocation
pub fn format_map_location_says_body(
    text1: &str,
    text2: &str,
    target_p_id: i32,
    rel_x: i32,
    rel_y: i32,
) -> String {
    format!("{text1} *{text2} {target_p_id} *map {rel_x} {rel_y}")
}

/// Haxe `sendLeader` / `sendDirectLeader` map-pin body (`LEADER *leader …`).
// Haxe: Connection.sendLeaderHelper / sendDirectLeader
pub fn format_leader_map_location_body(leader_p_id: i32, rel_x: i32, rel_y: i32) -> String {
    format_map_location_says_body("LEADER", "leader", leader_p_id, rel_x, rel_y)
}

/// Haxe `!L` / `!LEADER` / `?L` spoken line (private say):
/// `{name} {familyName} Power: {ceil(power)}`
// Haxe: GlobalPlayerInstance doCommands !LEADER L5727+
pub fn format_leader_power_say(name: &str, family_name: &str, power: f32) -> String {
    let power_ceil = power.ceil() as i32;
    if family_name.is_empty() {
        format!("{name} Power: {power_ceil}")
    } else {
        format!("{name} {family_name} Power: {power_ceil}")
    }
}

/// Haxe “No leader!” when follow/top leader is missing (self is always a leader
/// for top chain; used for **direct** follow when `followPlayer == null`).
pub fn format_no_leader_say() -> &'static str {
    "No leader!"
}

/// Resolve direct follow target for `!DL` / `sendDirectLeader`.
///
/// Returns `None` when the player does not follow anyone.
// Haxe: player.followPlayer
pub fn direct_follow_leader(following: &HashMap<i32, i32>, p_id: i32) -> Option<i32> {
    following.get(&p_id).copied().filter(|&l| l != p_id && l != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_following_is_none() {
        let m = HashMap::new();
        assert!(rank_leaders(&m, 10).is_empty());
        assert_eq!(format_leader_query(&m, 10), "LEADER none");
        assert!(!is_leader(&m, 1));
        assert_eq!(follower_count(&m, 1), 0);
    }

    #[test]
    fn ranks_by_follower_count() {
        let mut m = HashMap::new();
        m.insert(2, 10);
        m.insert(3, 10);
        m.insert(4, 10);
        m.insert(5, 20);
        m.insert(6, 20);
        let ranked = rank_leaders(&m, 10);
        assert_eq!(ranked[0].leader_id, 10);
        assert_eq!(ranked[0].followers, 3);
        assert_eq!(ranked[1].leader_id, 20);
        assert_eq!(ranked[1].followers, 2);
        assert!(is_leader(&m, 10));
        assert_eq!(follower_count(&m, 10), 3);
    }

    #[test]
    fn tie_break_lower_id_first() {
        let mut m = HashMap::new();
        m.insert(1, 30);
        m.insert(2, 20);
        // both have 1 follower — lower id first
        let ranked = rank_leaders(&m, 10);
        assert_eq!(ranked[0].leader_id, 20);
        assert_eq!(ranked[1].leader_id, 30);
    }

    #[test]
    fn respects_limit() {
        let mut m = HashMap::new();
        for i in 0..5 {
            m.insert(100 + i, i);
        }
        assert_eq!(rank_leaders(&m, 2).len(), 2);
        assert_eq!(rank_leaders(&m, 0).len(), 0);
    }

    #[test]
    fn format_includes_counts() {
        let mut m = HashMap::new();
        m.insert(2, 7);
        m.insert(3, 7);
        let s = format_leader_query(&m, 5);
        assert_eq!(s, "LEADER 7:2");
    }

    #[test]
    fn direct_follower_gate() {
        let mut m = HashMap::new();
        m.insert(2, 10); // 2 follows 10
        m.insert(3, 2); // 3 follows 2 (chain under 10)
        assert!(is_direct_follower(&m, 10, 2));
        assert!(!is_direct_follower(&m, 10, 3)); // not direct
        assert!(!is_direct_follower(&m, 2, 10));
        assert!(!is_direct_follower(&m, 10, 99));
        assert!(!is_direct_follower(&HashMap::new(), 1, 2));
    }

    #[test]
    fn order_obey_disobey_formatters() {
        assert_eq!(
            format_order_delivery(7, "MOVE NORTH"),
            "7 ORDER MOVE NORTH"
        );
        assert_eq!(format_order_result(7, 2, true, ""), "7 ORDER 2 OK");
        assert_eq!(
            format_order_result(7, 2, false, "not_leader"),
            "7 ORDER FAIL not_leader"
        );
        assert_eq!(format_obey_notify(2), "2 OBEY");
        assert_eq!(format_obey_result(2, true, ""), "2 OBEY OK");
        assert_eq!(
            format_obey_result(2, false, "no_leader"),
            "2 OBEY FAIL no_leader"
        );
        assert_eq!(format_disobey_notify(2), "2 DISOBEY");
        assert_eq!(format_disobey_result(2, true, ""), "2 DISOBEY OK");
        assert_eq!(
            format_disobey_result(2, false, "no_leader"),
            "2 DISOBEY FAIL no_leader"
        );
    }

    // ── LEADER-RANGE ────────────────────────────────────────────────────────

    #[test]
    fn is_close_pu_squared_euclidean() {
        // dist 0
        assert!(is_close_pu(0, 0, 0, 0, 1));
        // (3,4) → 25 <= 25
        assert!(is_close_pu(0, 0, 3, 4, 5));
        // (3,4) → 25 > 16
        assert!(!is_close_pu(0, 0, 3, 4, 4));
        // max_distance <= 0 → always close
        assert!(is_close_pu(0, 0, 9999, 9999, 0));
        assert!(is_close_pu(0, 0, 9999, 9999, -1));
    }

    /// Haxe transformX/Y: edge pair on torus is 1 tile, not map_w-1.
    // Haxe: WorldMap.transformX/Y + isClose with MaxDistance clamp 20
    #[test]
    fn is_close_pu_wrap_torus_edge_within_nearby() {
        // Plane: (0,0)→(99,0) is far for max=2
        assert!(!is_close_pu(0, 0, 99, 0, 2));
        // Torus 100×100: shortest dx = -1 → close
        assert!(is_close_pu_wrap(0, 0, 99, 0, 2, 100, 100, true));
        // Diagonal wrap (0,0)↔(99,99) → cheby 1, euclid √2 ≤ 2
        assert!(is_close_pu_wrap(0, 0, 99, 99, 2, 100, 100, true));
        // Still far when wrap off
        assert!(!is_close_pu_wrap(0, 0, 99, 0, 2, 100, 100, false));
        // Far even with wrap when mid-map
        assert!(!is_close_pu_wrap(0, 0, 50, 0, 2, 100, 100, true));
    }

    #[test]
    fn decide_player_info_range_wrap_torus_in_range() {
        // Across wrap edge, non-leader → InRange (SendUpdate), not PO
        assert_eq!(
            decide_player_info_range_wrap(0, 0, 99, 0, 2, 1, 20, 100, 100, true),
            PlayerInfoRangeDecision::SendUpdate
        );
        // Same coords without wrap → PO
        assert_eq!(
            decide_player_info_range_wrap(0, 0, 99, 0, 2, 1, 20, 100, 100, false),
            PlayerInfoRangeDecision::SendOutOfRange
        );
    }

    #[test]
    fn decide_player_info_range_leader_exempt() {
        // Far non-leader → PO
        assert_eq!(
            decide_player_info_range(0, 0, 100, 100, 2, 1, 20),
            PlayerInfoRangeDecision::SendOutOfRange
        );
        // Far top leader → still PU (client LEADER break workaround)
        assert_eq!(
            decide_player_info_range(0, 0, 100, 100, 1, 1, 20),
            PlayerInfoRangeDecision::SendUpdateLeaderExempt
        );
        // Close non-leader → PU
        assert_eq!(
            decide_player_info_range(0, 0, 1, 0, 2, 1, 20),
            PlayerInfoRangeDecision::SendUpdate
        );
        // Close leader → normal SendUpdate (not exempt variant)
        assert_eq!(
            decide_player_info_range(0, 0, 1, 0, 1, 1, 20),
            PlayerInfoRangeDecision::SendUpdate
        );
        // Default Haxe live max (2e6) keeps everyone close
        assert_eq!(
            decide_player_info_range(0, 0, 100_000, 100_000, 2, 1, 2_000_000),
            PlayerInfoRangeDecision::SendUpdate
        );
    }

    #[test]
    fn is_top_leader_of_chain() {
        let mut m = HashMap::new();
        m.insert(2, 10);
        m.insert(3, 2);
        assert!(is_top_leader_of(&m, 3, 10));
        assert!(is_top_leader_of(&m, 2, 10));
        assert!(!is_top_leader_of(&m, 3, 2)); // 2 is mid, top is 10
        assert!(is_top_leader_of(&m, 10, 10)); // unfollowed → self
        assert!(!is_top_leader_of(&m, 3, 99));
    }

    #[test]
    fn map_location_and_power_say_format() {
        assert_eq!(
            format_leader_map_location_body(42, 3, -7),
            "LEADER *leader 42 *map 3 -7"
        );
        assert_eq!(
            format_map_location_says_body("FOLLOWER", "follower", 5, 1, 2),
            "FOLLOWER *follower 5 *map 1 2"
        );
        assert_eq!(
            format_leader_power_say("Ada", "SNOW", 12.1),
            "Ada SNOW Power: 13"
        );
        assert_eq!(format_leader_power_say("Eve", "", 0.0), "Eve Power: 0");
        assert_eq!(format_no_leader_say(), "No leader!");
        assert_eq!(direct_follow_leader(&HashMap::new(), 1), None);
        let mut m = HashMap::new();
        m.insert(1, 9);
        assert_eq!(direct_follow_leader(&m, 1), Some(9));
    }
}
