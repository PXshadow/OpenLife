//! Test/admin environment force helpers (not full VOG).
//!
//! Thin facades over [`Environment`], [`Apocalypse`], and [`Weather`] so
//! external crates / future admin net paths share one entry surface.

use crate::apocalypse::Apocalypse;
use crate::environment::{Environment, Season};
use crate::weather::{parse_weather_kind, Weather, WeatherKind};

/// Parse season token (`SPRING|SUMMER|AUTUMN|FALL|WINTER`).
pub fn parse_season(s: &str) -> Option<Season> {
    Season::parse(s)
}

/// Force season on environment (resets elapsed, recomputes temperature).
pub fn set_season(env: &mut Environment, season: Season) {
    env.set_season(season);
}

/// Force hour of day [0, 24) (recomputes night cooling).
pub fn set_hour(env: &mut Environment, hour: f32) {
    env.set_hour(hour);
}

/// Start apocalypse warning → active cycle.
pub fn start_apoc(apoc: &mut Apocalypse) {
    apoc.trigger();
}

/// End apocalypse (reset to Idle).
pub fn end_apoc(apoc: &mut Apocalypse) {
    apoc.end();
}

/// Set weather by kind name. Returns `false` if the kind is unknown.
pub fn set_weather(weather: &mut Weather, kind: &str, secs: f32) -> bool {
    match parse_weather_kind(kind) {
        Some(k) => {
            weather.set(k, secs);
            true
        }
        None => false,
    }
}

/// Wire name for a weather kind.
pub fn weather_kind_name(k: WeatherKind) -> &'static str {
    k.wire_name()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apocalypse::ApocalypsePhase;
    use crate::environment::Season;

    #[test]
    fn season_hour_apoc() {
        let mut e = Environment::default();
        set_season(&mut e, Season::Winter);
        assert_eq!(e.season, Season::Winter);
        assert_eq!(e.season_elapsed, 0.0);
        set_hour(&mut e, 25.0);
        assert!((e.hour_of_day - 1.0).abs() < 1e-3);
        let mut a = Apocalypse::default();
        start_apoc(&mut a);
        assert_eq!(a.phase, ApocalypsePhase::Warning);
        end_apoc(&mut a);
        assert_eq!(a.phase, ApocalypsePhase::Idle);
        let mut w = Weather::default();
        assert!(set_weather(&mut w, "storm", 30.0));
        assert_eq!(w.kind, WeatherKind::Storm);
        assert!(!set_weather(&mut w, "not-a-kind", 1.0));
    }

    #[test]
    fn parse_seasons() {
        assert_eq!(parse_season("fall"), Some(Season::Autumn));
        assert_eq!(parse_season("WINTER"), Some(Season::Winter));
        assert_eq!(parse_season("nope"), None);
    }

    #[test]
    fn weather_kind_names() {
        assert_eq!(weather_kind_name(WeatherKind::Clear), "clear");
    }
}
