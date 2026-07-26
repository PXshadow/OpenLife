//! Lineage, following, exile — Haxe Connection bootstrap / social packets subset.
//!
//! Wire tags (server→client):
//! - LN / lineage lines (minimal family chain; Haxe LINEAGE)
//! - FW FOLLOWING: `follower_id leader_id color`
//! - EX EXILED: `target_id exiler_id` lines
//! - LR is LEARNED_TOOL_REPORT (tools), not lineage

use crate::prestige::{prestige_class_wire_token, PrestigeClass};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Shared lineage list for web (`/api/lineages`, `/lineage`).
pub type LineageView = Arc<RwLock<LineageSnapshot>>;

/// One lineage node for JSON / HTML.
#[derive(Debug, Clone, Serialize)]
pub struct LineageEntryView {
    pub id: i32,
    pub name: String,
    pub mother_id: Option<i32>,
    pub father_id: Option<i32>,
    pub generation: i32,
    pub prestige: f32,
    pub prestige_class: String,
}

/// Lineage book snapshot (no SQL; mirrors OLN1 in-memory state).
#[derive(Debug, Clone, Serialize, Default)]
pub struct LineageSnapshot {
    pub lineages: Vec<LineageEntryView>,
    pub count: usize,
    /// On-disk format hint for operators.
    pub format: String,
}

/// Lightweight lineage node (not full Haxe Lineage.hx).
#[derive(Debug, Clone)]
pub struct LineageNode {
    pub id: i32,
    pub name: String,
    pub mother_id: Option<i32>,
    pub father_id: Option<i32>,
    pub generation: i32,
    pub prestige: f32,
    /// Haxe `lineage.prestigeClass` (kept in sync via [`Self::set_prestige`]).
    pub prestige_class: PrestigeClass,
}

impl LineageNode {
    pub fn eve(id: i32, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            mother_id: None,
            father_id: None,
            generation: 0,
            prestige: 0.0,
            prestige_class: PrestigeClass::from_prestige(0.0),
        }
    }

    /// Child born to `mother` (generation + 1, mother_id set).
    pub fn with_mother(id: i32, name: impl Into<String>, mother: &LineageNode) -> Self {
        Self {
            id,
            name: name.into(),
            mother_id: Some(mother.id),
            father_id: None,
            generation: mother.generation.saturating_add(1),
            prestige: 0.0,
            prestige_class: PrestigeClass::from_prestige(0.0),
        }
    }

    /// Update prestige and recompute class (Haxe `calculatePrestigeClass` subset).
    pub fn set_prestige(&mut self, prestige: f32) {
        self.prestige = prestige.max(0.0);
        self.prestige_class = PrestigeClass::from_prestige(self.prestige);
    }

    /// Set living-percentile prestige class without changing prestige float.
    ///
    /// Used by [`crate::SimState::refresh_living_prestige_classes`] (score rank path).
    pub fn set_prestige_class(&mut self, class: PrestigeClass) {
        self.prestige_class = class;
    }

    /// Add delta prestige and recompute class.
    pub fn add_prestige(&mut self, delta: f32) {
        self.set_prestige(self.prestige + delta);
    }

    /// Prestige class for this lineage (cached field).
    pub fn prestige_class(&self) -> PrestigeClass {
        self.prestige_class
    }

    /// Haxe-style compact lineage summary for bootstrap (includes class + prestige).
    pub fn wire_line(&self) -> String {
        let mother = self.mother_id.unwrap_or(self.id);
        let class_tok = prestige_class_wire_token(self.prestige);
        format!(
            "{} eve={} gen={} name={} {}",
            self.id, mother, self.generation, self.name, class_tok
        )
    }
}

#[derive(Debug, Default, Clone)]
pub struct SocialState {
    pub lineages: HashMap<i32, LineageNode>,
    /// follower_p_id → leader_p_id
    pub following: HashMap<i32, i32>,
    /// leader_p_id → set of exiled p_ids
    pub exiles: HashMap<i32, HashSet<i32>>,
    /// badge color index per leader (0..7)
    pub leader_colors: HashMap<i32, i32>,
}

impl SocialState {
    pub fn ensure_lineage(&mut self, p_id: i32, name: &str) {
        self.lineages
            .entry(p_id)
            .or_insert_with(|| LineageNode::eve(p_id, name.to_string()));
    }

    /// Set lineage prestige and recompute prestige class.
    pub fn set_lineage_prestige(&mut self, p_id: i32, prestige: f32) {
        if let Some(n) = self.lineages.get_mut(&p_id) {
            n.set_prestige(prestige);
        }
    }

    /// Set lineage prestige class from living score percentiles (no float change).
    pub fn set_lineage_prestige_class(&mut self, p_id: i32, class: PrestigeClass) {
        if let Some(n) = self.lineages.get_mut(&p_id) {
            n.set_prestige_class(class);
        }
    }

    /// Prestige class for a lineage id (default Serf if missing).
    pub fn prestige_class(&self, p_id: i32) -> PrestigeClass {
        self.lineages
            .get(&p_id)
            .map(|n| n.prestige_class())
            .unwrap_or(PrestigeClass::Serf)
    }

    pub fn set_follow(&mut self, follower: i32, leader: i32) -> Result<(), &'static str> {
        if follower == leader {
            self.following.remove(&follower);
            return Ok(());
        }
        // Reject obvious cycles: leader already follows follower chain back.
        let mut walk = leader;
        let mut guard = 0;
        while let Some(&next) = self.following.get(&walk) {
            if next == follower {
                return Err("circular_follow");
            }
            walk = next;
            guard += 1;
            if guard > 64 {
                break;
            }
        }
        self.following.insert(follower, leader);
        self.leader_colors.entry(leader).or_insert(0);
        Ok(())
    }

    pub fn unfollow(&mut self, follower: i32) {
        self.following.remove(&follower);
    }

    pub fn exile(&mut self, leader: i32, target: i32) {
        self.exiles.entry(leader).or_default().insert(target);
        // Being exiled ends follow relationship both ways.
        if self.following.get(&target) == Some(&leader) {
            self.following.remove(&target);
        }
    }

    pub fn is_exiled_by(&self, leader: i32, target: i32) -> bool {
        self.exiles
            .get(&leader)
            .map(|s| s.contains(&target))
            .unwrap_or(false)
    }

    /// FOLLOWING packet body lines for all known pairs.
    pub fn following_packets(&self) -> Vec<String> {
        self.following
            .iter()
            .map(|(&follower, &leader)| {
                let color = self.leader_colors.get(&leader).copied().unwrap_or(0);
                format!("{follower} {leader} {color}")
            })
            .collect()
    }

    /// EXILED list: space-separated pairs or multi-line.
    pub fn exile_packets(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (&leader, set) in &self.exiles {
            for &target in set {
                out.push(format!("{target} {leader}"));
            }
        }
        out
    }

    pub fn lineage_packets(&self) -> Vec<String> {
        self.lineages.values().map(|n| n.wire_line()).collect()
    }

    /// Snapshot for web lineage page / `/api/lineages`.
    pub fn snapshot(&self) -> LineageSnapshot {
        let mut lineages: Vec<LineageEntryView> = self
            .lineages
            .values()
            .map(|n| LineageEntryView {
                id: n.id,
                name: n.name.clone(),
                mother_id: n.mother_id,
                father_id: n.father_id,
                generation: n.generation,
                prestige: n.prestige,
                prestige_class: n.prestige_class.wire_name().to_string(),
            })
            .collect();
        lineages.sort_by(|a, b| {
            a.generation
                .cmp(&b.generation)
                .then_with(|| a.id.cmp(&b.id))
        });
        let count = lineages.len();
        LineageSnapshot {
            lineages,
            count,
            format: "OLN1".into(),
        }
    }
}

/// Format FOLLOWING server message payload lines.
pub fn format_following_line(follower: i32, leader: i32, color: i32) -> String {
    format!("{follower} {leader} {color}")
}

/// Format EXILED pair.
pub fn format_exile_line(target: i32, exiler: i32) -> String {
    format!("{target} {exiler}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follow_and_cycle_reject() {
        let mut s = SocialState::default();
        s.set_follow(2, 3).unwrap();
        s.set_follow(3, 4).unwrap();
        assert!(s.set_follow(4, 2).is_err());
        assert_eq!(s.following.get(&2), Some(&3));
    }

    #[test]
    fn exile_breaks_follow() {
        let mut s = SocialState::default();
        s.set_follow(5, 6).unwrap();
        s.exile(6, 5);
        assert!(s.is_exiled_by(6, 5));
        assert!(!s.following.contains_key(&5));
    }

    #[test]
    fn lineage_wire() {
        let n = LineageNode::eve(10, "EVE");
        assert!(n.wire_line().contains("10"));
        assert!(n.wire_line().contains("EVE"));
        assert!(n.wire_line().contains("class=serf"));
        assert_eq!(n.prestige_class(), PrestigeClass::Serf);
    }

    #[test]
    fn lineage_prestige_class_updates() {
        let mut n = LineageNode::eve(1, "A");
        n.set_prestige(55.0);
        assert_eq!(n.prestige_class(), PrestigeClass::Noble);
        assert!(n.wire_line().contains("class=noble"));
    }
}
