//! Seasonal snow / freeze overlay notes (Haxe snow map subset).
//!
//! Tracks which tiles are snow-covered for path notes and food drain; does not
//! mutate biome ids (those stay in World).

use std::collections::HashSet;

use crate::environment::Season;

/// Extra food drain when standing on snow in winter.
pub const SNOW_FOOD_EXTRA: f32 = 0.03;

/// Move-speed factor on snow.
pub const SNOW_MOVE_FACTOR: f32 = 0.88;

/// Sparse snow cover set.
#[derive(Debug, Default, Clone)]
pub struct SnowCover {
    pub tiles: HashSet<(i32, i32)>,
    /// Global snow intensity 0..1 (season driven).
    pub intensity: f32,
}

impl SnowCover {
    /// Update intensity from season (winter high, summer clear).
    pub fn sync_season(&mut self, season: Season) {
        self.intensity = match season {
            Season::Winter => 1.0,
            Season::Autumn => 0.25,
            Season::Spring => 0.1,
            Season::Summer => 0.0,
        };
        if self.intensity <= 0.0 {
            self.tiles.clear();
        }
    }

    pub fn set_tile(&mut self, x: i32, y: i32, snow: bool) {
        if snow {
            self.tiles.insert((x, y));
        } else {
            self.tiles.remove(&(x, y));
        }
    }

    pub fn is_snow(&self, x: i32, y: i32) -> bool {
        self.intensity > 0.0 && self.tiles.contains(&(x, y))
    }

    /// Auto-cover a ring around players in winter (cheap aesthetic).
    pub fn blanket_near(&mut self, centers: &[(i32, i32)], radius: i32) {
        if self.intensity < 0.5 {
            return;
        }
        for &(cx, cy) in centers {
            for dx in -radius..=radius {
                for dy in -radius..=radius {
                    self.tiles.insert((cx + dx, cy + dy));
                }
            }
        }
    }

    pub fn food_extra_at(&self, x: i32, y: i32) -> f32 {
        if self.is_snow(x, y) {
            SNOW_FOOD_EXTRA * self.intensity
        } else {
            0.0
        }
    }

    pub fn format_query(&self) -> String {
        format!(
            "SNOW intensity={:.2} tiles={}",
            self.intensity,
            self.tiles.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winter_blanket() {
        let mut s = SnowCover::default();
        s.sync_season(Season::Winter);
        s.blanket_near(&[(0, 0)], 1);
        assert!(s.is_snow(0, 0));
        assert!(s.food_extra_at(0, 0) > 0.0);
        s.sync_season(Season::Summer);
        assert!(!s.is_snow(0, 0));
        assert_eq!(s.tiles.len(), 0);
    }

    #[test]
    fn query() {
        let s = SnowCover::default();
        assert!(s.format_query().contains("intensity="));
    }
}
