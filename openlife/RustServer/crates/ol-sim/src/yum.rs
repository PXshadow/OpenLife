//! YUM chain / food history (Haxe yum system simplified).

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct YumState {
    pub history: VecDeque<i32>,
    pub capacity: usize,
    pub yum_bonus: f32,
    /// Last food object id eaten (`last_ate_id` on FX / PU).
    pub just_ate_id: i32,
    /// Haxe `just_ate` flag: 1 while the eat PU is in flight, then cleared.
    pub just_ate: bool,
    /// Food store (ceil) before the last eat — FX `last_ate_fill_max`.
    pub last_ate_fill_max: i32,
}

impl Default for YumState {
    fn default() -> Self {
        Self {
            history: VecDeque::with_capacity(8),
            capacity: 8,
            yum_bonus: 0.0,
            just_ate_id: 0,
            just_ate: false,
            last_ate_fill_max: 0,
        }
    }
}

impl YumState {
    /// Record eating food object. Bonus if not recently eaten same id.
    /// `fill_before` is ceil(food_store) before the gain is applied.
    pub fn eat(&mut self, food_id: i32, base_value: f32, fill_before: i32) -> f32 {
        let recent = self.history.iter().any(|&id| id == food_id);
        let mult = if recent { 1.0 } else { 1.5 };
        self.just_ate_id = food_id;
        self.just_ate = true;
        self.last_ate_fill_max = fill_before;
        self.history.push_back(food_id);
        while self.history.len() > self.capacity {
            self.history.pop_front();
        }
        if !recent {
            self.yum_bonus += 0.1;
        } else {
            self.yum_bonus = (self.yum_bonus - 0.05).max(0.0);
        }
        base_value * mult + self.yum_bonus
    }

    /// Clear the transient `just_ate` flag after PU/FX fan-out (Haxe post-PU).
    pub fn clear_just_ate_flag(&mut self) {
        self.just_ate = false;
    }

    /// Wire ints for FX / PU.
    pub fn just_ate_flag(&self) -> i32 {
        if self.just_ate {
            1
        } else {
            0
        }
    }

    pub fn yum_bonus_ceil(&self) -> i32 {
        self.yum_bonus.ceil() as i32
    }

    /// Human-readable `?YUM` reply body (without player id).
    pub fn query_text(&self) -> String {
        format!(
            "YUM bonus={} history={}",
            self.yum_bonus,
            self.history.len()
        )
    }

    /// Clear YUM history / bonus / just-ate (SAY CLEAR YUM / RESET YUM).
    pub fn clear(&mut self) {
        self.history.clear();
        self.yum_bonus = 0.0;
        self.just_ate_id = 0;
        self.just_ate = false;
        self.last_ate_fill_max = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yum_bonus_on_variety() {
        let mut y = YumState::default();
        let a = y.eat(33, 3.0, 5);
        let b = y.eat(33, 3.0, 8);
        assert!(a > b); // first of kind better than repeat
        let c = y.eat(40, 3.0, 10);
        assert!(c >= b);
        assert_eq!(y.just_ate_id, 40);
        assert!(y.just_ate);
        assert_eq!(y.last_ate_fill_max, 10);
        assert_eq!(y.history.len(), 3);
    }

    #[test]
    fn query_text_includes_bonus_and_history_len() {
        let mut y = YumState::default();
        let _ = y.eat(33, 3.0, 0);
        let t = y.query_text();
        assert!(t.contains("bonus="));
        assert!(t.contains("history=1"));
        assert!(t.starts_with("YUM "));
    }

    #[test]
    fn clear_resets_history_bonus_and_just_ate() {
        let mut y = YumState::default();
        let _ = y.eat(33, 3.0, 5);
        let _ = y.eat(40, 3.0, 8);
        assert!(!y.history.is_empty());
        assert!(y.yum_bonus > 0.0);
        assert!(y.just_ate);
        y.clear();
        assert!(y.history.is_empty());
        assert_eq!(y.yum_bonus, 0.0);
        assert_eq!(y.just_ate_id, 0);
        assert!(!y.just_ate);
        assert_eq!(y.last_ate_fill_max, 0);
        assert_eq!(y.query_text(), "YUM bonus=0 history=0");
    }
}
