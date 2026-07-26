//! Move-speed composition notes (Haxe moveSpeed factors subset).

use crate::fire::FireState;
use crate::snow::{SnowCover, SNOW_MOVE_FACTOR};
use crate::weather::Weather;
use crate::{RIDE_MOVE_SPEED, WALK_MOVE_SPEED};

/// Per-item ballast penalty applied to move speed (held + backpack count).
///
/// Slight: 2% slower per carried object. Full load (1 held + 8 backpack) = −18%.
pub const BALLAST_PER_ITEM: f32 = 0.02;

/// Floor for the ballast multiplier so a full pack never freezes movement.
pub const BALLAST_MULT_MIN: f32 = 0.5;

/// Count ballast items: 1 if hands hold something, plus backpack slots filled.
pub fn weight_item_count(held_id: i32, backpack_len: usize) -> u32 {
    let held = if held_id != 0 { 1u32 } else { 0 };
    held + backpack_len as u32
}

/// Multiplier from ballast item count (`1.0` with empty hands and pack).
pub fn ballast_speed_mult(items: u32) -> f32 {
    if items == 0 {
        return 1.0;
    }
    (1.0 - BALLAST_PER_ITEM * items as f32).max(BALLAST_MULT_MIN)
}

/// Compose reported move speed from base walk/ride + weather + snow + fire + ballast.
///
/// `ballast_items` is held (0/1) + backpack count from [`weight_item_count`].
pub fn compose_move_speed(
    riding: bool,
    weather: &Weather,
    snow: &SnowCover,
    fire: &FireState,
    x: i32,
    y: i32,
    ballast_items: u32,
) -> f32 {
    let mut speed = if riding {
        RIDE_MOVE_SPEED
    } else {
        WALK_MOVE_SPEED
    };
    speed *= weather.kind.move_speed_factor();
    if snow.is_snow(x, y) {
        speed *= SNOW_MOVE_FACTOR;
    }
    if fire.is_burning(x, y) {
        speed *= 0.8;
    }
    speed *= ballast_speed_mult(ballast_items);
    speed
}

/// `SAY ?SPEED` body without leading p_id: composed move speed note.
///
/// Format: `SPEED move=X.XX` from [`compose_move_speed`].
pub fn format_speed_query(speed: f32) -> String {
    format!("SPEED move={speed:.2}")
}

/// `SAY ?WEIGHT` body without leading p_id: carried item count.
///
/// Format: `WEIGHT n items` (held + backpack slots).
pub fn format_weight_query(items: u32) -> String {
    format!("WEIGHT {items} items")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::Season;
    use crate::snow::SnowCover;
    use crate::weather::{Weather, WeatherKind};

    #[test]
    fn weight_item_count_held_and_backpack() {
        assert_eq!(weight_item_count(0, 0), 0);
        assert_eq!(weight_item_count(33, 0), 1);
        assert_eq!(weight_item_count(0, 3), 3);
        assert_eq!(weight_item_count(10, 5), 6);
    }

    #[test]
    fn ballast_mult_slight_per_item() {
        assert!((ballast_speed_mult(0) - 1.0).abs() < 1e-6);
        assert!((ballast_speed_mult(1) - 0.98).abs() < 1e-6);
        assert!((ballast_speed_mult(5) - 0.90).abs() < 1e-6);
        // 1 held + 8 pack = 9 → 0.82
        assert!((ballast_speed_mult(9) - 0.82).abs() < 1e-6);
        // floor
        assert!((ballast_speed_mult(100) - BALLAST_MULT_MIN).abs() < 1e-6);
    }

    #[test]
    fn ride_faster_than_walk() {
        let w = Weather::default();
        let s = SnowCover::default();
        let f = FireState::default();
        let walk = compose_move_speed(false, &w, &s, &f, 0, 0, 0);
        let ride = compose_move_speed(true, &w, &s, &f, 0, 0, 0);
        assert!(ride > walk);
    }

    #[test]
    fn storm_slows() {
        let w = Weather::new(WeatherKind::Storm, 10.0);
        let s = SnowCover::default();
        let f = FireState::default();
        let slow = compose_move_speed(false, &w, &s, &f, 0, 0, 0);
        assert!(slow < WALK_MOVE_SPEED);
    }

    #[test]
    fn snow_slows() {
        let w = Weather::default();
        let mut s = SnowCover::default();
        s.sync_season(Season::Winter);
        s.set_tile(1, 1, true);
        let f = FireState::default();
        let v = compose_move_speed(false, &w, &s, &f, 1, 1, 0);
        assert!(v < WALK_MOVE_SPEED);
    }

    #[test]
    fn fire_slows() {
        let w = Weather::default();
        let s = SnowCover::default();
        let mut f = FireState::default();
        f.ignite(2, 2, 10.0, 1.0);
        let v = compose_move_speed(false, &w, &s, &f, 2, 2, 0);
        assert!((v - WALK_MOVE_SPEED * 0.8).abs() < 0.001);
    }

    #[test]
    fn ballast_slows_slightly() {
        let w = Weather::default();
        let s = SnowCover::default();
        let f = FireState::default();
        let empty = compose_move_speed(false, &w, &s, &f, 0, 0, 0);
        let heavy = compose_move_speed(false, &w, &s, &f, 0, 0, 5);
        assert!((empty - WALK_MOVE_SPEED).abs() < 0.001);
        assert!((heavy - WALK_MOVE_SPEED * 0.90).abs() < 0.001);
        assert!(heavy < empty);
    }

    #[test]
    fn format_speed_query_shape() {
        let s = format_speed_query(WALK_MOVE_SPEED);
        assert_eq!(s, format!("SPEED move={WALK_MOVE_SPEED:.2}"));
        assert!(format_speed_query(5.0).contains("5.00"));
    }

    #[test]
    fn format_weight_query_shape() {
        assert_eq!(format_weight_query(0), "WEIGHT 0 items");
        assert_eq!(format_weight_query(1), "WEIGHT 1 items");
        assert_eq!(format_weight_query(9), "WEIGHT 9 items");
    }
}
