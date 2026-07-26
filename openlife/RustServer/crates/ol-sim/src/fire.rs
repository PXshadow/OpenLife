//! Fire / flood tile hazards (Haxe map hazard subset).
//!
//! Pure timers: burning tiles deal periodic food drain to players standing on them.

use std::collections::HashMap;

/// Seconds a fire burns by default.
pub const DEFAULT_FIRE_SECS: f32 = 30.0;

/// Extra food drain per second while standing on fire.
pub const FIRE_FOOD_DRAIN: f32 = 0.15;

/// One burning tile.
#[derive(Debug, Clone, Copy)]
pub struct FireTile {
    pub remaining: f32,
    pub intensity: f32,
}

/// Active fire map (x,y) → fire.
#[derive(Debug, Default, Clone)]
pub struct FireState {
    pub tiles: HashMap<(i32, i32), FireTile>,
}

impl FireState {
    pub fn ignite(&mut self, x: i32, y: i32, secs: f32, intensity: f32) {
        self.tiles.insert(
            (x, y),
            FireTile {
                remaining: secs.max(0.1),
                intensity: intensity.max(0.1),
            },
        );
    }

    pub fn extinguish(&mut self, x: i32, y: i32) -> bool {
        self.tiles.remove(&(x, y)).is_some()
    }

    pub fn is_burning(&self, x: i32, y: i32) -> bool {
        self.tiles.contains_key(&(x, y))
    }

    /// Drain/sec for a player on (x,y).
    pub fn drain_at(&self, x: i32, y: i32) -> f32 {
        self.tiles
            .get(&(x, y))
            .map(|f| FIRE_FOOD_DRAIN * f.intensity)
            .unwrap_or(0.0)
    }

    /// Advance timers; remove expired.
    pub fn tick(&mut self, dt: f32) {
        let mut dead = Vec::new();
        for (k, f) in self.tiles.iter_mut() {
            f.remaining -= dt;
            if f.remaining <= 0.0 {
                dead.push(*k);
            }
        }
        for k in dead {
            self.tiles.remove(&k);
        }
    }

    pub fn count(&self) -> usize {
        self.tiles.len()
    }

    pub fn format_query(&self) -> String {
        format!("FIRE tiles={}", self.count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burn_and_expire() {
        let mut f = FireState::default();
        f.ignite(1, 2, 1.0, 1.0);
        assert!(f.is_burning(1, 2));
        assert!(f.drain_at(1, 2) > 0.0);
        f.tick(0.5);
        assert!(f.is_burning(1, 2));
        f.tick(0.6);
        assert!(!f.is_burning(1, 2));
        assert_eq!(f.format_query(), "FIRE tiles=0");
    }

    #[test]
    fn extinguish() {
        let mut f = FireState::default();
        f.ignite(0, 0, 10.0, 2.0);
        assert!(f.extinguish(0, 0));
        assert!(!f.extinguish(0, 0));
    }
}
