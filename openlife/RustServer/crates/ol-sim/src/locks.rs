//! Per-tile lock flags for owned gates/doors (session, not OLW1 yet).

use std::collections::HashSet;

/// Set of locked tile coordinates.
#[derive(Debug, Default, Clone)]
pub struct LockState {
    pub locked: HashSet<(i32, i32)>,
}

impl LockState {
    pub fn lock(&mut self, x: i32, y: i32) {
        self.locked.insert((x, y));
    }

    pub fn unlock(&mut self, x: i32, y: i32) -> bool {
        self.locked.remove(&(x, y))
    }

    pub fn is_locked(&self, x: i32, y: i32) -> bool {
        self.locked.contains(&(x, y))
    }

    pub fn count(&self) -> usize {
        self.locked.len()
    }

    pub fn format_query(&self, x: i32, y: i32) -> String {
        format!(
            "LOCKTILE {x} {y} locked={}",
            if self.is_locked(x, y) { 1 } else { 0 }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_unlock() {
        let mut l = LockState::default();
        l.lock(3, 4);
        assert!(l.is_locked(3, 4));
        assert!(l.unlock(3, 4));
        assert!(!l.is_locked(3, 4));
        assert!(l.format_query(3, 4).contains("locked=0"));
    }
}
