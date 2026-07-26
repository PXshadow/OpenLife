//! Weather overlay on top of seasons (Haxe map-weather subset).
//!
//! Weather modulates food drain and move notes; biome/season still come from
//! [`crate::environment::Environment`].

use crate::environment::Season;
use serde::Serialize;
use std::sync::{Arc, RwLock};

/// Shared weather snapshot for web / metrics (EnvView-style).
pub type WeatherView = Arc<RwLock<WeatherSnapshot>>;

/// JSON-friendly weather snapshot for `/api/weather`.
#[derive(Debug, Clone, Serialize)]
pub struct WeatherSnapshot {
    pub kind: String,
    pub remaining_secs: f32,
    pub food_drain_mult: f32,
    pub move_speed_factor: f32,
}

impl Default for WeatherSnapshot {
    fn default() -> Self {
        Self {
            kind: "clear".into(),
            remaining_secs: 0.0,
            food_drain_mult: 1.0,
            move_speed_factor: 1.0,
        }
    }
}

/// Discrete weather kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WeatherKind {
    #[default]
    Clear,
    Rain,
    Storm,
    Snow,
    Heatwave,
    Fog,
}

impl WeatherKind {
    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Clear => "clear",
            Self::Rain => "rain",
            Self::Storm => "storm",
            Self::Snow => "snow",
            Self::Heatwave => "heatwave",
            Self::Fog => "fog",
        }
    }

    /// Extra food drain multiplier contribution (added to 1.0 base outside).
    pub fn food_drain_extra(self) -> f32 {
        match self {
            Self::Clear => 0.0,
            Self::Rain => 0.02,
            Self::Storm => 0.05,
            Self::Snow => 0.04,
            Self::Heatwave => 0.06,
            Self::Fog => 0.01,
        }
    }

    /// Move-speed factor note (1.0 = normal). Fog/storm slow slightly.
    pub fn move_speed_factor(self) -> f32 {
        match self {
            Self::Clear | Self::Heatwave => 1.0,
            Self::Rain => 0.95,
            Self::Snow => 0.90,
            Self::Storm => 0.85,
            Self::Fog => 0.92,
        }
    }
}

/// Live weather state with optional timed transitions.
#[derive(Debug, Clone)]
pub struct Weather {
    pub kind: WeatherKind,
    /// Seconds remaining in current kind before auto-pick (0 = sticky).
    pub remaining_secs: f32,
}

impl Default for Weather {
    fn default() -> Self {
        Self {
            kind: WeatherKind::Clear,
            remaining_secs: 0.0,
        }
    }
}

impl Weather {
    pub fn new(kind: WeatherKind, duration_secs: f32) -> Self {
        Self {
            kind,
            remaining_secs: duration_secs.max(0.0),
        }
    }

    /// Advance weather clock; when duration elapses, pick season-biased clear/rain/snow.
    pub fn tick(&mut self, dt: f32, season: Season) {
        if self.remaining_secs <= 0.0 {
            return;
        }
        self.remaining_secs = (self.remaining_secs - dt).max(0.0);
        if self.remaining_secs <= 0.0 {
            self.kind = default_for_season(season);
            self.remaining_secs = 0.0;
        }
    }

    /// Force a weather change (VOG / SAY WEATHER style).
    pub fn set(&mut self, kind: WeatherKind, duration_secs: f32) {
        self.kind = kind;
        self.remaining_secs = duration_secs.max(0.0);
    }

    /// Food drain multiplier: `1.0 + kind.extra`.
    pub fn food_drain_mult(&self) -> f32 {
        1.0 + self.kind.food_drain_extra()
    }

    /// Chat body for `SAY ?WEATHER` (without leading p_id).
    pub fn query_text(&self) -> String {
        if self.remaining_secs > 0.0 {
            format!(
                "WEATHER {} remaining={:.0}s drain={:.2}",
                self.kind.wire_name(),
                self.remaining_secs,
                self.food_drain_mult()
            )
        } else {
            format!(
                "WEATHER {} drain={:.2}",
                self.kind.wire_name(),
                self.food_drain_mult()
            )
        }
    }

    /// Snapshot for web / shared `WeatherView`.
    pub fn snapshot(&self) -> WeatherSnapshot {
        WeatherSnapshot {
            kind: self.kind.wire_name().to_string(),
            remaining_secs: self.remaining_secs,
            food_drain_mult: self.food_drain_mult(),
            move_speed_factor: self.kind.move_speed_factor(),
        }
    }
}

/// Season-biased default when a timed weather expires.
pub fn default_for_season(season: Season) -> WeatherKind {
    match season {
        Season::Winter => WeatherKind::Snow,
        Season::Summer => WeatherKind::Clear,
        Season::Spring => WeatherKind::Rain,
        Season::Autumn => WeatherKind::Clear,
    }
}

/// Parse weather kind from SAY text token.
pub fn parse_weather_kind(s: &str) -> Option<WeatherKind> {
    match s.trim().to_ascii_lowercase().as_str() {
        "clear" | "sun" | "sunny" => Some(WeatherKind::Clear),
        "rain" | "rainy" => Some(WeatherKind::Rain),
        "storm" | "thunder" => Some(WeatherKind::Storm),
        "snow" | "snowy" => Some(WeatherKind::Snow),
        "heat" | "heatwave" | "hot" => Some(WeatherKind::Heatwave),
        "fog" | "foggy" | "mist" => Some(WeatherKind::Fog),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_is_neutral() {
        let w = Weather::default();
        assert_eq!(w.kind, WeatherKind::Clear);
        assert_eq!(w.food_drain_mult(), 1.0);
        assert_eq!(w.kind.move_speed_factor(), 1.0);
    }

    #[test]
    fn storm_drains_and_slows() {
        let w = Weather::new(WeatherKind::Storm, 60.0);
        assert!(w.food_drain_mult() > 1.0);
        assert!(w.kind.move_speed_factor() < 1.0);
        let q = w.query_text();
        assert!(q.contains("storm"));
        assert!(q.contains("remaining="));
    }

    #[test]
    fn tick_expires_to_season_default() {
        let mut w = Weather::new(WeatherKind::Storm, 1.0);
        w.tick(0.5, Season::Winter);
        assert_eq!(w.kind, WeatherKind::Storm);
        w.tick(0.6, Season::Winter);
        assert_eq!(w.kind, WeatherKind::Snow);
        assert_eq!(w.remaining_secs, 0.0);
    }

    #[test]
    fn parse_kinds() {
        assert_eq!(parse_weather_kind("RAIN"), Some(WeatherKind::Rain));
        assert_eq!(parse_weather_kind("heatwave"), Some(WeatherKind::Heatwave));
        assert_eq!(parse_weather_kind("nope"), None);
    }

    #[test]
    fn season_defaults() {
        assert_eq!(default_for_season(Season::Winter), WeatherKind::Snow);
        assert_eq!(default_for_season(Season::Spring), WeatherKind::Rain);
    }

    #[test]
    fn set_forces_kind_and_duration() {
        let mut w = Weather::default();
        w.set(WeatherKind::Rain, 90.0);
        assert_eq!(w.kind, WeatherKind::Rain);
        assert!((w.remaining_secs - 90.0).abs() < 1e-6);
        let q = w.query_text();
        assert!(q.contains("rain"), "{q}");
        assert!(q.contains("remaining=90"), "{q}");
        w.set(WeatherKind::Clear, 0.0);
        assert_eq!(w.kind, WeatherKind::Clear);
        assert_eq!(w.remaining_secs, 0.0);
    }
}
