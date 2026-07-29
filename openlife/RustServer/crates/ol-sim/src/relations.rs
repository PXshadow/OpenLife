//! Mother / child / spouse relationship queries (Haxe lineage relation subset).
//!
//! Also: close-relative / same-family / living-children helpers for death coin
//! inheritance (Haxe `GlobalPlayerInstance.isCloseRelative` / `getAllChildren`).

use crate::social::SocialState;
use std::collections::HashMap;

/// Relation label between two player ids via lineage mother links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    Self_,
    Mother,
    Child,
    Sibling,
    None,
}

impl Relation {
    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Self_ => "self",
            Self::Mother => "mother",
            Self::Child => "child",
            Self::Sibling => "sibling",
            Self::None => "none",
        }
    }
}

/// True when the player has a lineage node with `mother_id == None` (Eve / founder).
pub fn is_eve(social: &SocialState, p_id: i32) -> bool {
    social
        .lineages
        .get(&p_id)
        .map(|n| n.mother_id.is_none())
        .unwrap_or(false)
}

/// Walk mother_id chain to root Eve id (self if no mother). Guarded against cycles.
pub fn root_eve_id(social: &SocialState, p_id: i32) -> i32 {
    let mut cur = p_id;
    let mut guard = 0;
    while let Some(n) = social.lineages.get(&cur) {
        match n.mother_id {
            Some(m) if m != cur => {
                cur = m;
                guard += 1;
                if guard > 64 {
                    break;
                }
            }
            _ => break,
        }
    }
    cur
}

/// Haxe `isSameFamily`: shared Eve root via mother lineage.
pub fn is_same_family(social: &SocialState, a: i32, b: i32) -> bool {
    if a == b {
        return true;
    }
    if !social.lineages.contains_key(&a) || !social.lineages.contains_key(&b) {
        return false;
    }
    root_eve_id(social, a) == root_eve_id(social, b)
}

/// Haxe `isCloseRelative` (mother/father/child/sibling/grand; mother-links primarily).
///
/// Father links are included when `father_id` is set on either node.
pub fn is_close_relative(social: &SocialState, a: i32, b: i32) -> bool {
    if a == b {
        return false;
    }
    let Some(na) = social.lineages.get(&a) else {
        return false;
    };
    let Some(nb) = social.lineages.get(&b) else {
        return false;
    };
    // Parent / child
    if na.mother_id == Some(b) || na.father_id == Some(b) {
        return true;
    }
    if nb.mother_id == Some(a) || nb.father_id == Some(a) {
        return true;
    }
    // Sibling (same mother or same father when both set)
    if let (Some(ma), Some(mb)) = (na.mother_id, nb.mother_id) {
        if ma == mb {
            return true;
        }
    }
    if let (Some(fa), Some(fb)) = (na.father_id, nb.father_id) {
        if fa == fb {
            return true;
        }
    }
    // Grandparent / grandkid via mother chain
    if let Some(m) = na.mother_id {
        if social.lineages.get(&m).and_then(|n| n.mother_id) == Some(b) {
            return true;
        }
        if social.lineages.get(&m).and_then(|n| n.father_id) == Some(b) {
            return true;
        }
    }
    if let Some(m) = nb.mother_id {
        if social.lineages.get(&m).and_then(|n| n.mother_id) == Some(a) {
            return true;
        }
        if social.lineages.get(&m).and_then(|n| n.father_id) == Some(a) {
            return true;
        }
    }
    if let Some(f) = na.father_id {
        if social.lineages.get(&f).and_then(|n| n.father_id) == Some(b) {
            return true;
        }
        if social.lineages.get(&f).and_then(|n| n.mother_id) == Some(b) {
            return true;
        }
    }
    if let Some(f) = nb.father_id {
        if social.lineages.get(&f).and_then(|n| n.father_id) == Some(a) {
            return true;
        }
        if social.lineages.get(&f).and_then(|n| n.mother_id) == Some(a) {
            return true;
        }
    }
    false
}

/// Haxe `getAllChildren(onlyLiving)` — mother_id **or** father_id points at parent.
///
/// `living` is a set of p_ids considered alive (not deleted). When `only_living`,
/// only ids present in `living` are returned.
pub fn living_children_of(
    social: &SocialState,
    parent: i32,
    living: &HashMap<i32, bool>,
    only_living: bool,
) -> Vec<i32> {
    let mut kids: Vec<i32> = social
        .lineages
        .iter()
        .filter(|(id, n)| {
            if **id == parent {
                return false;
            }
            let is_child = n.mother_id == Some(parent) || n.father_id == Some(parent);
            if !is_child {
                return false;
            }
            if only_living {
                living.get(id).copied().unwrap_or(false)
            } else {
                true
            }
        })
        .map(|(id, _)| *id)
        .collect();
    kids.sort_unstable();
    kids
}

/// Follow chain top leader (Haxe `getTopLeader` subset). Returns self if unfollowed.
///
/// Does **not** apply exile / deleted / circular→null rules — use
/// [`get_top_leader`] for full Haxe parity (LEADERSHIP-UX).
pub fn top_leader(following: &HashMap<i32, i32>, p_id: i32) -> i32 {
    let mut cur = p_id;
    let mut guard = 0;
    while let Some(&next) = following.get(&cur) {
        if next == cur {
            break;
        }
        cur = next;
        guard += 1;
        if guard > 64 {
            break;
        }
    }
    cur
}

/// Haxe `GlobalPlayerInstance.getTopLeader` — full walk with exile edges, deleted
/// leaders, optional stop-with, and **circular → `None`** (depth &gt; 10).
///
/// - No follow → `Some(self)`
/// - Leader deleted → stop at last living
/// - `exiler` has exiled `p_id` or `p_id` has exiled `exiler` → stop at lastLeader
/// - Cycle / depth exceeded → `None` (caller refuses follow probe)
// Haxe: GlobalPlayerInstance.getTopLeader
pub fn get_top_leader(
    following: &HashMap<i32, i32>,
    social: &SocialState,
    deleted: &std::collections::HashSet<i32>,
    p_id: i32,
    stop_with: Option<i32>,
) -> Option<i32> {
    let Some(&first) = following.get(&p_id) else {
        return Some(p_id); // is his own leader
    };
    if first == p_id || first == 0 {
        return Some(p_id);
    }

    let mut last_leader = p_id;
    let mut leader = first;
    for _ in 0..10 {
        if deleted.contains(&leader) {
            // Haxe: TODO check why no new leader was chosen
            return Some(last_leader);
        }
        // Haxe: this.exiledByPlayers.exists(leader) → is exiled by leader
        if social.is_exiled_by(leader, p_id) {
            return Some(last_leader);
        }
        // Haxe: leader.exiledByPlayers.exists(this) → player exiled leader
        if social.is_exiled_by(p_id, leader) {
            return Some(last_leader);
        }
        match following.get(&leader) {
            None | Some(&0) => return Some(leader),
            Some(&next) if next == leader => return Some(leader),
            Some(&next) => {
                if stop_with == Some(leader) {
                    return Some(leader);
                }
                last_leader = leader;
                leader = next;
            }
        }
    }
    // Circular / too deep
    None
}

/// Convenience: [`get_top_leader`] falling back to `p_id` when circular (None).
// Haxe: getTopLeader used where null is treated as missing/self
pub fn get_top_leader_or_self(
    following: &HashMap<i32, i32>,
    social: &SocialState,
    deleted: &std::collections::HashSet<i32>,
    p_id: i32,
) -> i32 {
    get_top_leader(following, social, deleted, p_id, None).unwrap_or(p_id)
}

/// Follow-graph-only ally check (shared [`top_leader`]; **ignores exile edges**).
///
/// Prefer [`is_ally`] for kill / prestige / HIT paths that must match Haxe `isAlly`.
// Haxe: isAlly without exile (approx; top_leader only)
pub fn is_leadership_ally(following: &HashMap<i32, i32>, a: i32, b: i32) -> bool {
    if a == 0 || b == 0 {
        return false;
    }
    top_leader(following, a) == top_leader(following, b)
}

/// Haxe `isAlly`: same top leader via full [`get_top_leader`] (exile + deleted).
///
/// Used by **PRESTIGE-ALLY-COST** / kill after mid-hit exile so multi-hop followers
/// stop counting as allies once the attacker has exiled them (Haxe L4540).
// Haxe: GlobalPlayerInstance.isAlly
// PRESTIGE-ALLY-COST
pub fn is_ally(
    following: &HashMap<i32, i32>,
    social: &SocialState,
    deleted: &std::collections::HashSet<i32>,
    a: i32,
    b: i32,
) -> bool {
    if a == 0 || b == 0 {
        return false;
    }
    // Haxe: this.getTopLeader() == target.getTopLeader() (null == null → true)
    get_top_leader(following, social, deleted, a, None)
        == get_top_leader(following, social, deleted, b, None)
}

/// Infer relation from lineage mother_id fields.
pub fn relation_of(social: &SocialState, a: i32, b: i32) -> Relation {
    if a == b {
        return Relation::Self_;
    }
    let ma = social.lineages.get(&a).and_then(|n| n.mother_id);
    let mb = social.lineages.get(&b).and_then(|n| n.mother_id);
    if ma == Some(b) {
        return Relation::Mother; // b is mother of a
    }
    if mb == Some(a) {
        return Relation::Child; // b is child of a
    }
    if let (Some(x), Some(y)) = (ma, mb) {
        if x == y {
            return Relation::Sibling;
        }
    }
    Relation::None
}

/// Chat body `REL a b label` (or `REL a a EVE` for self Eve; `… EVE` when either is Eve).
pub fn format_relation_query(social: &SocialState, a: i32, b: i32) -> String {
    let r = relation_of(social, a, b);
    // Self-query on an Eve/founder: label is EVE (mother_id None).
    if r == Relation::Self_ && is_eve(social, a) {
        return format!("REL {a} {b} EVE");
    }
    let label = r.wire_name();
    if is_eve(social, a) || is_eve(social, b) {
        format!("REL {a} {b} {label} EVE")
    } else {
        format!("REL {a} {b} {label}")
    }
}

/// Chat body `GEN p_id generation` from [`crate::social::LineageNode::generation`].
///
/// Missing lineage → generation `0`.
pub fn format_gen_query(social: &SocialState, p_id: i32) -> String {
    let gen = social
        .lineages
        .get(&p_id)
        .map(|n| n.generation)
        .unwrap_or(0);
    format!("GEN {p_id} {gen}")
}

/// List children of mother as `CHILDREN id id …` or `CHILDREN none`.
pub fn format_children_query(social: &SocialState, mother: i32) -> String {
    let mut kids: Vec<i32> = social
        .lineages
        .iter()
        .filter(|(_, n)| n.mother_id == Some(mother))
        .map(|(id, _)| *id)
        .collect();
    kids.sort_unstable();
    if kids.is_empty() {
        "CHILDREN none".into()
    } else {
        let s = kids
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        format!("CHILDREN {s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::social::{LineageNode, SocialState};

    #[test]
    fn mother_child_sibling() {
        let mut s = SocialState::default();
        s.lineages.insert(1, LineageNode::eve(1, "Mom"));
        let mom = s.lineages.get(&1).unwrap().clone();
        s.lineages
            .insert(2, LineageNode::with_mother(2, "A", &mom));
        s.lineages
            .insert(3, LineageNode::with_mother(3, "B", &mom));
        assert_eq!(relation_of(&s, 2, 1), Relation::Mother);
        assert_eq!(relation_of(&s, 1, 2), Relation::Child);
        assert_eq!(relation_of(&s, 2, 3), Relation::Sibling);
        assert!(format_children_query(&s, 1).contains("2"));
        assert!(format_relation_query(&s, 2, 1).contains("mother"));
        // Eve mother: relation includes EVE marker.
        assert!(format_relation_query(&s, 2, 1).contains("EVE"));
        assert!(is_eve(&s, 1));
        assert!(!is_eve(&s, 2));
        assert!(is_same_family(&s, 2, 3));
        assert!(is_close_relative(&s, 2, 1));
        assert!(is_close_relative(&s, 2, 3));
        assert!(is_close_relative(&s, 1, 2));
    }

    #[test]
    fn eve_self_rel_and_gen_depth() {
        let mut s = SocialState::default();
        s.lineages.insert(10, LineageNode::eve(10, "EVE"));
        let eve = s.lineages.get(&10).unwrap().clone();
        s.lineages
            .insert(11, LineageNode::with_mother(11, "Child", &eve));
        let child = s.lineages.get(&11).unwrap().clone();
        s.lineages
            .insert(12, LineageNode::with_mother(12, "Grand", &eve));
        // Fix grand: should be child of 11
        s.lineages
            .insert(12, LineageNode::with_mother(12, "Grand", &child));

        assert_eq!(format_relation_query(&s, 10, 10), "REL 10 10 EVE");
        assert!(!format_relation_query(&s, 11, 11).contains("EVE"));
        assert_eq!(format_relation_query(&s, 11, 11), "REL 11 11 self");

        assert_eq!(format_gen_query(&s, 10), "GEN 10 0");
        assert_eq!(format_gen_query(&s, 11), "GEN 11 1");
        assert_eq!(format_gen_query(&s, 12), "GEN 12 2");
        assert_eq!(format_gen_query(&s, 99), "GEN 99 0");

        assert_eq!(root_eve_id(&s, 12), 10);
        assert!(is_close_relative(&s, 12, 10)); // grandparent
        assert!(is_close_relative(&s, 10, 12)); // grandkid
    }

    #[test]
    fn living_children_and_top_leader() {
        let mut s = SocialState::default();
        s.lineages.insert(1, LineageNode::eve(1, "Mom"));
        let mom = s.lineages.get(&1).unwrap().clone();
        s.lineages
            .insert(2, LineageNode::with_mother(2, "A", &mom));
        s.lineages
            .insert(3, LineageNode::with_mother(3, "B", &mom));
        let mut living = HashMap::new();
        living.insert(2, true);
        living.insert(3, false);
        assert_eq!(living_children_of(&s, 1, &living, true), vec![2]);
        assert_eq!(living_children_of(&s, 1, &living, false), vec![2, 3]);

        let mut following = HashMap::new();
        following.insert(2, 1);
        following.insert(3, 1);
        assert_eq!(top_leader(&following, 2), 1);
        assert!(is_leadership_ally(&following, 2, 3));
        assert!(!is_leadership_ally(&following, 2, 9));

        let empty_del = std::collections::HashSet::new();
        assert_eq!(
            get_top_leader(&following, &s, &empty_del, 2, None),
            Some(1)
        );
        assert_eq!(
            get_top_leader(&following, &s, &empty_del, 1, None),
            Some(1)
        );
        assert!(is_ally(&following, &s, &empty_del, 2, 3));
    }

    #[test]
    fn get_top_leader_exile_deleted_circular() {
        use std::collections::HashSet;
        let mut s = SocialState::default();
        let mut following = HashMap::new();
        // 3 → 2 → 1
        following.insert(3, 2);
        following.insert(2, 1);
        let empty = HashSet::new();
        assert_eq!(get_top_leader(&following, &s, &empty, 3, None), Some(1));

        // Exile edge: 1 exiled 3 → stop before 1
        s.exile(1, 3);
        assert_eq!(get_top_leader(&following, &s, &empty, 3, None), Some(2));
        s.exiles.clear();

        // 3 exiled 1 → also stop
        s.exile(3, 1);
        assert_eq!(get_top_leader(&following, &s, &empty, 3, None), Some(2));
        s.exiles.clear();

        // Deleted mid-leader
        let mut del = HashSet::new();
        del.insert(2);
        assert_eq!(get_top_leader(&following, &s, &del, 3, None), Some(3));

        // Circular → None
        following.clear();
        following.insert(1, 2);
        following.insert(2, 3);
        following.insert(3, 1);
        assert_eq!(get_top_leader(&following, &s, &empty, 1, None), None);
    }

    /// PRESTIGE-ALLY-COST: multi-hop follower stop counting as ally after leader exile.
    // Haxe: isAlly after exile(target) mid kill
    #[test]
    fn is_ally_breaks_on_multi_hop_exile() {
        use std::collections::HashSet;
        let mut s = SocialState::default();
        let mut following = HashMap::new();
        // 3 → 2 → 1 (leader)
        following.insert(3, 2);
        following.insert(2, 1);
        let empty = HashSet::new();
        assert!(is_ally(&following, &s, &empty, 1, 3));
        assert!(is_leadership_ally(&following, 1, 3));

        // Leader 1 exiles 3 — follow edges remain (Haxe only stamps exiledByPlayers).
        // social.exile may also drop *direct* follow of leader; multi-hop keeps 3→2.
        s.exiles.entry(1).or_default().insert(3);
        // Follow-graph-only still says ally; exile-aware isAlly does not.
        assert!(is_leadership_ally(&following, 1, 3));
        assert!(
            !is_ally(&following, &s, &empty, 1, 3),
            "after exile, multi-hop follower must not be isAlly of leader"
        );

        // Peer allies under same leader remain allies when one peer exiles the other
        // (exiler is not on the victim's follow chain to top).
        s.exiles.clear();
        following.clear();
        following.insert(10, 1); // A
        following.insert(11, 1); // B
        assert!(is_ally(&following, &s, &empty, 10, 11));
        s.exiles.entry(10).or_default().insert(11);
        assert!(
            is_ally(&following, &s, &empty, 10, 11),
            "peer exile does not break shared top leader"
        );
    }
}
