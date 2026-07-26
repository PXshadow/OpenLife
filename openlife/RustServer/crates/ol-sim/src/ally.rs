//! Personal ally / friend set (Haxe non-exile social subset).
//!
//! Symmetric friendship is optional; this stores directed ally links only.

use std::collections::{HashMap, HashSet};

/// Ally book: p_id → set of allied p_ids.
#[derive(Debug, Default, Clone)]
pub struct AllyState {
    pub allies: HashMap<i32, HashSet<i32>>,
}

impl AllyState {
    pub fn add(&mut self, from: i32, to: i32) -> Result<(), &'static str> {
        if from == to || to == 0 || from == 0 {
            return Err("BAD");
        }
        self.allies.entry(from).or_default().insert(to);
        Ok(())
    }

    pub fn remove(&mut self, from: i32, to: i32) -> bool {
        self.allies
            .get_mut(&from)
            .map(|s| s.remove(&to))
            .unwrap_or(false)
    }

    pub fn is_ally(&self, from: i32, to: i32) -> bool {
        self.allies
            .get(&from)
            .map(|s| s.contains(&to))
            .unwrap_or(false)
    }

    /// True if either direction is allied (soft mutual).
    pub fn is_mutual_or_either(&self, a: i32, b: i32) -> bool {
        self.is_ally(a, b) || self.is_ally(b, a)
    }

    /// Sorted ally ids for `from`.
    pub fn list(&self, from: i32) -> Vec<i32> {
        let mut v: Vec<i32> = self
            .allies
            .get(&from)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default();
        v.sort_unstable();
        v
    }

    /// Chat body for `SAY ?ALLY` without leading p_id.
    pub fn format_query(&self, from: i32) -> String {
        let list = self.list(from);
        if list.is_empty() {
            "ALLY none".into()
        } else {
            let ids = list
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            format!("ALLY {ids}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_list_remove() {
        let mut a = AllyState::default();
        assert!(a.add(1, 2).is_ok());
        assert!(a.add(1, 3).is_ok());
        assert_eq!(a.add(1, 1), Err("BAD"));
        assert!(a.is_ally(1, 2));
        assert_eq!(a.list(1), vec![2, 3]);
        assert!(a.format_query(1).contains("2"));
        assert!(a.remove(1, 2));
        assert!(!a.is_ally(1, 2));
        assert_eq!(a.format_query(9), "ALLY none");
    }

    #[test]
    fn either_direction() {
        let mut a = AllyState::default();
        a.add(1, 2).unwrap();
        assert!(a.is_mutual_or_either(1, 2));
        assert!(a.is_mutual_or_either(2, 1));
    }
}
