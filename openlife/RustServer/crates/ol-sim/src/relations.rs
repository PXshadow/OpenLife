//! Mother / child / spouse relationship queries (Haxe lineage relation subset).

use crate::social::SocialState;

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
            .insert(12, LineageNode::with_mother(12, "Grand", &child));

        assert_eq!(format_relation_query(&s, 10, 10), "REL 10 10 EVE");
        assert!(!format_relation_query(&s, 11, 11).contains("EVE"));
        assert_eq!(format_relation_query(&s, 11, 11), "REL 11 11 self");

        assert_eq!(format_gen_query(&s, 10), "GEN 10 0");
        assert_eq!(format_gen_query(&s, 11), "GEN 11 1");
        assert_eq!(format_gen_query(&s, 12), "GEN 12 2");
        assert_eq!(format_gen_query(&s, 99), "GEN 99 0");
    }
}
