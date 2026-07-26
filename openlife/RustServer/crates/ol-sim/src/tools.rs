//! Tool slots / learned tools (Haxe TOOL_SLOTS / LEARNED_TOOL_REPORT subset).
//!
//! Wire:
//! - TS `used total` via [`ToolSlots::wire_slots`]
//! - LR space-separated object ids via [`ToolSlots::learned_list`] /
//!   `ol_protocol::format_learned_tool_report`

use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct ToolSlots {
    pub used: i32,
    pub total: i32,
    pub learned: HashSet<i32>,
    pub experts: HashSet<i32>,
}

impl Default for ToolSlots {
    fn default() -> Self {
        Self {
            used: 0,
            total: 1000,
            learned: HashSet::new(),
            experts: HashSet::new(),
        }
    }
}

impl ToolSlots {
    pub fn wire_slots(&self) -> String {
        format!("{} {}", self.used, self.total)
    }

    pub fn learn(&mut self, object_id: i32) {
        if self.learned.insert(object_id) {
            self.used = self.learned.len() as i32;
        }
    }

    pub fn mark_expert(&mut self, object_id: i32) {
        self.learn(object_id);
        self.experts.insert(object_id);
    }

    /// Clear all learned/expert tools and reset `used` (test helper / `SAY FORGETTOOLS`).
    pub fn forget_all(&mut self) {
        self.learned.clear();
        self.experts.clear();
        self.used = 0;
    }

    /// Sorted space-separated object ids for LEARNED_TOOL_REPORT (LR) payload.
    pub fn learned_list(&self) -> String {
        let mut v: Vec<_> = self.learned.iter().copied().collect();
        v.sort_unstable();
        v.iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Sorted learned object ids (stable for wire / tests).
    pub fn learned_ids_sorted(&self) -> Vec<i32> {
        let mut v: Vec<_> = self.learned.iter().copied().collect();
        v.sort_unstable();
        v
    }

    /// Human-readable `?TOOLS` reply body (without player id): wire slots + learned count.
    pub fn query_text(&self) -> String {
        format!(
            "TOOLS {} learned={}",
            self.wire_slots(),
            self.learned.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_protocol::{format_learned_tool_report, format_tool_slots};

    #[test]
    fn learn_increments_used() {
        let mut t = ToolSlots::default();
        t.learn(334);
        t.learn(334);
        t.learn(12);
        assert_eq!(t.used, 2);
        assert!(t.wire_slots().starts_with("2 "));
        assert_eq!(t.learned_list(), "12 334");
        assert_eq!(
            format_learned_tool_report(&t.learned_ids_sorted()),
            "LR\n12 334\n#"
        );
        assert_eq!(format_tool_slots(t.used, t.total), "TS\n2 1000\n#");
    }

    #[test]
    fn query_text_includes_wire_slots_and_learned_count() {
        let mut t = ToolSlots::default();
        t.learn(334);
        t.learn(12);
        let q = t.query_text();
        assert!(q.starts_with("TOOLS "));
        assert!(q.contains(t.wire_slots().as_str()), "got {q}");
        assert!(q.contains("learned=2"), "got {q}");
        assert_eq!(q, "TOOLS 2 1000 learned=2");
    }

    #[test]
    fn forget_all_clears_learned_and_used() {
        let mut t = ToolSlots::default();
        t.learn(334);
        t.mark_expert(12);
        assert_eq!(t.used, 2);
        assert!(!t.learned.is_empty());
        assert!(!t.experts.is_empty());
        t.forget_all();
        assert_eq!(t.used, 0);
        assert!(t.learned.is_empty());
        assert!(t.experts.is_empty());
        assert_eq!(t.query_text(), "TOOLS 0 1000 learned=0");
        assert_eq!(t.learned_list(), "");
    }
}
