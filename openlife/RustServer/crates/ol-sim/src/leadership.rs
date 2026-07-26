//! Leadership election over follow graphs (Haxe FOLLOWING leaders subset).
//!
//! Pure: leaders are nodes with inbound follows; ranked by follower count.

use std::collections::HashMap;

/// One elected leader: `leader_p_id` with `follower_count` direct followers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeaderEntry {
    pub leader_id: i32,
    pub followers: usize,
}

/// Default top-N for `?LEADER`.
pub const LEADER_QUERY_LIMIT: usize = 10;

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

/// `SAY ?LEADER` chat reply body (without leading player id).
///
/// Format: `LEADER id:n id:n …` or `LEADER none` when empty.
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
}
