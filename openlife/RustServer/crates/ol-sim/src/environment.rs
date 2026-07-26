//! Seasons + day cycle + temperature (Haxe TimeHelper subset + day/night).
//!
//! Haxe `TimeHelper` seasons: Spring / Summer / Autumn / Winter.
//! Day/night is a lightweight extension: `hour_of_day` cycles 0..24 and
//! drives `day_night_multiplier` (food / temp cost).

use serde::Serialize;
use std::sync::{Arc, RwLock};

/// Shared snapshot for web viewer / metrics.
pub type EnvView = Arc<RwLock<EnvSnapshot>>;

#[derive(Debug, Clone, Serialize)]
pub struct EnvSnapshot {
    pub season: String,
    pub temperature: f32,
    pub season_impact: f32,
    pub season_elapsed: f32,
    /// Current hour in the 0..24 game day.
    pub hour_of_day: f32,
    /// Coarse phase: DAWN | DAY | DUSK | NIGHT.
    pub day_phase: String,
    /// Multiplier on food drain / cold stress (>1 at night).
    pub day_night_multiplier: f32,
}

impl Default for EnvSnapshot {
    fn default() -> Self {
        Self {
            season: "SPRING".into(),
            temperature: 0.5,
            season_impact: 0.0,
            season_elapsed: 0.0,
            hour_of_day: 12.0,
            day_phase: "DAY".into(),
            day_night_multiplier: 1.0,
        }
    }
}

/// Season cycle matching Open Life season names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub fn as_str(self) -> &'static str {
        match self {
            Season::Spring => "SPRING",
            Season::Summer => "SUMMER",
            Season::Autumn => "AUTUMN",
            Season::Winter => "WINTER",
        }
    }

    /// Parse season token from SAY `SETSEASON` (case-insensitive).
    ///
    /// Accepts `SPRING|SUMMER|AUTUMN|FALL|WINTER`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "SPRING" => Some(Self::Spring),
            "SUMMER" => Some(Self::Summer),
            "AUTUMN" | "FALL" => Some(Self::Autumn),
            "WINTER" => Some(Self::Winter),
            _ => None,
        }
    }

    /// Advance one season.
    pub fn next(self) -> Self {
        match self {
            Season::Spring => Season::Summer,
            Season::Summer => Season::Autumn,
            Season::Autumn => Season::Winter,
            Season::Winter => Season::Spring,
        }
    }
}

/// Coarse day phase derived from `hour_of_day`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayPhase {
    Dawn,
    Day,
    Dusk,
    Night,
}

impl DayPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            DayPhase::Dawn => "DAWN",
            DayPhase::Day => "DAY",
            DayPhase::Dusk => "DUSK",
            DayPhase::Night => "NIGHT",
        }
    }

    /// From continuous hour in [0, 24).
    pub fn from_hour(hour: f32) -> Self {
        let h = hour.rem_euclid(24.0);
        if (5.0..7.0).contains(&h) {
            DayPhase::Dawn
        } else if (7.0..18.0).contains(&h) {
            DayPhase::Day
        } else if (18.0..20.0).contains(&h) {
            DayPhase::Dusk
        } else {
            DayPhase::Night
        }
    }
}

#[derive(Debug, Clone)]
pub struct Environment {
    pub season: Season,
    /// Seconds into current season.
    pub season_elapsed: f32,
    /// Seconds per season (default 120s for fast self-play).
    pub season_length: f32,
    /// Base temperature 0..1 (0 cold, 1 hot).
    pub temperature: f32,
    /// Season impact on temperature (−1..1).
    pub season_temperature_impact: f32,
    pub hot_factor: f32,
    pub cold_factor: f32,
    /// Current hour of the game day, range [0, 24).
    pub hour_of_day: f32,
    /// Real seconds for a full 24-hour game day (default 240s).
    pub day_length: f32,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            season: Season::Spring,
            season_elapsed: 0.0,
            season_length: 120.0,
            temperature: 0.5,
            season_temperature_impact: 0.0,
            hot_factor: 1.0,
            cold_factor: 1.0,
            hour_of_day: 12.0,
            day_length: 240.0,
        }
    }
}

impl Environment {
    /// Recompute season impact + temperature from current season/elapsed/hour (no clock advance).
    fn recompute_temperature(&mut self) {
        // Smooth seasonal bias (Haxe SeasonTemperatureImpact shape, simplified).
        let phase = self.season_elapsed / self.season_length.max(1.0);
        self.season_temperature_impact = match self.season {
            Season::Spring => -0.1 + 0.2 * phase,
            Season::Summer => 0.3 + 0.2 * phase,
            Season::Autumn => 0.2 - 0.4 * phase,
            Season::Winter => -0.3 - 0.2 * phase,
        };
        let impact = if self.season_temperature_impact > 0.0 {
            self.season_temperature_impact * self.hot_factor
        } else {
            self.season_temperature_impact * self.cold_factor
        };
        // Night is cooler.
        let night_cool = match self.day_phase() {
            DayPhase::Night => -0.10,
            DayPhase::Dawn | DayPhase::Dusk => -0.05,
            DayPhase::Day => 0.0,
        };
        self.temperature = (0.5 + impact * 0.5 + night_cool).clamp(0.0, 1.0);
    }

    /// Force season (`SAY SETSEASON`); resets season elapsed and recomputes temp.
    pub fn set_season(&mut self, season: Season) {
        self.season = season;
        self.season_elapsed = 0.0;
        self.recompute_temperature();
    }

    /// Force hour of day in [0, 24) (`SAY SETHOUR`); recomputes night cooling.
    pub fn set_hour(&mut self, hour: f32) {
        self.hour_of_day = hour.rem_euclid(24.0);
        self.recompute_temperature();
    }

    /// Advance season + day clock. Returns `true` when the season rolled over.
    pub fn tick(&mut self, dt: f32) -> bool {
        let mut season_changed = false;
        self.season_elapsed += dt;
        if self.season_elapsed >= self.season_length {
            self.season_elapsed = 0.0;
            self.season = self.season.next();
            season_changed = true;
        }
        // Temp uses current hour (pre-advance), matching prior tick order.
        self.recompute_temperature();

        // Advance clock: day_length real seconds ⇒ 24 game hours.
        let hours_per_sec = 24.0 / self.day_length.max(1.0);
        self.hour_of_day = (self.hour_of_day + dt * hours_per_sec).rem_euclid(24.0);
        season_changed
    }

    pub fn day_phase(&self) -> DayPhase {
        DayPhase::from_hour(self.hour_of_day)
    }

    /// Food / stress multiplier: night costs more, day is baseline 1.0.
    pub fn day_night_multiplier(&self) -> f32 {
        match self.day_phase() {
            DayPhase::Day => 1.0,
            DayPhase::Dawn | DayPhase::Dusk => 1.05,
            DayPhase::Night => 1.15,
        }
    }

    /// Effective temperature at biome (ocean/snow colder).
    pub fn temperature_at_biome(&self, biome: u8) -> f32 {
        let mut t = self.temperature;
        match biome {
            4 | 21 => t -= 0.25, // snow
            9 | 17 => t -= 0.15, // ocean/river
            5 => t += 0.15,      // desert
            6 | 15 => t += 0.1,  // jungle
            _ => {}
        }
        t.clamp(0.0, 1.0)
    }

    pub fn season_query_text(&self) -> String {
        let impact = if self.season_temperature_impact > 0.0 {
            self.season_temperature_impact * self.hot_factor
        } else {
            self.season_temperature_impact * self.cold_factor
        };
        format!(
            "{} {:.2}",
            self.season.as_str(),
            (impact * 100.0).round() / 100.0
        )
    }

    pub fn temp_query_text(&self, biome: u8) -> String {
        format!(
            "TEMP {:.2} SEASON {} {}",
            self.temperature_at_biome(biome),
            self.season.as_str(),
            self.day_phase().as_str()
        )
    }

    /// Text for `SAY ?TIME` (without leading player id).
    pub fn time_query_text(&self) -> String {
        format!(
            "TIME {:.2} {}",
            self.hour_of_day,
            self.day_phase().as_str()
        )
    }

    pub fn snapshot(&self) -> EnvSnapshot {
        EnvSnapshot {
            season: self.season.as_str().to_string(),
            temperature: self.temperature,
            season_impact: self.season_temperature_impact,
            season_elapsed: self.season_elapsed,
            hour_of_day: self.hour_of_day,
            day_phase: self.day_phase().as_str().to_string(),
            day_night_multiplier: self.day_night_multiplier(),
        }
    }
}

/// Clothing warmth bonus for food/temp drain reduction (Haxe clothing subset).
///
/// Each non-zero slot (hat / chest / shoes) adds `+0.5`, max `+1.5`.
/// Callers can scale cold food drain by this bonus later.
pub fn clothing_temp_bonus(hat: i32, chest: i32, shoes: i32) -> f32 {
    let mut b = 0.0_f32;
    if hat != 0 {
        b += 0.5;
    }
    if chest != 0 {
        b += 0.5;
    }
    if shoes != 0 {
        b += 0.5;
    }
    b
}

/// Food drain multiplier when standing on ocean (biome 9) or river (biome 17).
pub const OCEAN_RIVER_FOOD_DRAIN_MULT: f32 = 1.2;

/// Ocean biome id (wet / SWIM note).
pub const BIOME_OCEAN: u8 = 9;
/// River biome id (wet / SWIM note).
pub const BIOME_RIVER: u8 = 17;

/// True for ocean/river tiles where SWIM notes report extra food drain.
pub fn is_swim_biome(biome: u8) -> bool {
    biome == BIOME_OCEAN || biome == BIOME_RIVER
}

/// Multiplier on base food drain by standing biome.
/// Jungle is easier (slower hunger); snow is harder (faster hunger).
/// Ocean/river use [`OCEAN_RIVER_FOOD_DRAIN_MULT`] (faster hunger while wet).
pub fn biome_food_multiplier(biome: u8) -> f32 {
    match biome {
        4 | 21 => 1.25, // snow / snow-in-grey
        6 | 15 => 0.85, // jungle / border jungle
        5 => 1.10,      // desert
        9 | 17 => OCEAN_RIVER_FOOD_DRAIN_MULT, // ocean / river
        _ => 1.0,       // green and other temperate biomes
    }
}

/// `SAY ?BIOMEFOOD` body without leading p_id: `BIOMEFOOD biome=N mult=X.XX`.
pub fn format_biomefood_query(biome: u8) -> String {
    let mult = biome_food_multiplier(biome);
    format!("BIOMEFOOD biome={biome} mult={mult:.2}")
}

/// `SAY ?SWIM` / SWIM note body without leading p_id.
///
/// Format: `SWIM biome=N wet=0|1 food_mult=X.XX` — wet when ocean/river
/// ([`OCEAN_RIVER_FOOD_DRAIN_MULT`] already applied in vitals via biome mult).
pub fn format_swim_query(biome: u8) -> String {
    let mult = biome_food_multiplier(biome);
    let wet = if is_swim_biome(biome) { 1 } else { 0 };
    format!("SWIM biome={biome} wet={wet} food_mult={mult:.2}")
}

/// `SAY ?WARM` body without leading p_id: clothing warmth bonus.
///
/// Format: `WARM bonus=X.XX` from [`clothing_temp_bonus`].
pub fn format_warm_query(hat: i32, chest: i32, shoes: i32) -> String {
    let bonus = clothing_temp_bonus(hat, chest, shoes);
    format!("WARM bonus={bonus:.2}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn season_advances() {
        let mut e = Environment {
            season_length: 1.0,
            ..Default::default()
        };
        assert_eq!(e.season, Season::Spring);
        assert!(e.tick(1.1), "season rollover should return true");
        assert_eq!(e.season, Season::Summer);
        assert!(!e.tick(0.1), "mid-season tick should not report change");
        assert_eq!(e.season, Season::Summer);
    }

    #[test]
    fn snow_biome_colder() {
        let e = Environment {
            temperature: 0.5,
            ..Default::default()
        };
        assert!(e.temperature_at_biome(4) < e.temperature_at_biome(0));
    }

    #[test]
    fn biome_food_multipliers_jungle_easier_snow_harder() {
        assert_eq!(biome_food_multiplier(0), 1.0); // green
        assert!(biome_food_multiplier(6) < 1.0); // jungle easier
        assert!(biome_food_multiplier(15) < 1.0); // border jungle
        assert!(biome_food_multiplier(4) > 1.0); // snow harder
        assert!(biome_food_multiplier(21) > 1.0); // snow-in-grey
        assert!(biome_food_multiplier(4) > biome_food_multiplier(0));
        assert!(biome_food_multiplier(6) < biome_food_multiplier(0));
        assert!(biome_food_multiplier(5) > 1.0); // desert
    }

    #[test]
    fn format_biomefood_query_shape() {
        let s = format_biomefood_query(0);
        assert_eq!(s, "BIOMEFOOD biome=0 mult=1.00");
        let snow = format_biomefood_query(4);
        assert!(snow.contains("biome=4"), "got {snow}");
        assert!(snow.contains("mult=1.25"), "got {snow}");
        let jungle = format_biomefood_query(6);
        assert!(jungle.contains("mult=0.85"), "got {jungle}");
    }

    #[test]
    fn format_warm_query_reports_clothing_temp_bonus() {
        assert_eq!(format_warm_query(0, 0, 0), "WARM bonus=0.00");
        assert_eq!(format_warm_query(1, 0, 0), "WARM bonus=0.50");
        assert_eq!(format_warm_query(1, 2, 3), "WARM bonus=1.50");
    }

    #[test]
    fn ocean_river_food_drain_mult_is_1_2() {
        assert_eq!(OCEAN_RIVER_FOOD_DRAIN_MULT, 1.2);
        assert_eq!(biome_food_multiplier(9), OCEAN_RIVER_FOOD_DRAIN_MULT); // ocean
        assert_eq!(biome_food_multiplier(17), OCEAN_RIVER_FOOD_DRAIN_MULT); // river
        assert!(biome_food_multiplier(9) > biome_food_multiplier(0));
        assert!(biome_food_multiplier(17) > biome_food_multiplier(0));
    }

    #[test]
    fn format_swim_query_wet_on_ocean_dry_on_green() {
        assert!(is_swim_biome(BIOME_OCEAN));
        assert!(is_swim_biome(BIOME_RIVER));
        assert!(!is_swim_biome(0));
        let ocean = format_swim_query(BIOME_OCEAN);
        assert_eq!(
            ocean,
            format!(
                "SWIM biome={} wet=1 food_mult={:.2}",
                BIOME_OCEAN, OCEAN_RIVER_FOOD_DRAIN_MULT
            )
        );
        let dry = format_swim_query(0);
        assert_eq!(dry, "SWIM biome=0 wet=0 food_mult=1.00");
    }

    #[test]
    fn day_phase_from_hour() {
        assert_eq!(DayPhase::from_hour(6.0), DayPhase::Dawn);
        assert_eq!(DayPhase::from_hour(12.0), DayPhase::Day);
        assert_eq!(DayPhase::from_hour(19.0), DayPhase::Dusk);
        assert_eq!(DayPhase::from_hour(0.0), DayPhase::Night);
        assert_eq!(DayPhase::from_hour(23.5), DayPhase::Night);
        assert_eq!(DayPhase::from_hour(3.0), DayPhase::Night);
    }

    #[test]
    fn hour_of_day_advances_and_wraps() {
        let mut e = Environment {
            hour_of_day: 23.0,
            day_length: 24.0, // 1 real second = 1 game hour
            season_length: 10_000.0,
            ..Default::default()
        };
        e.tick(1.0);
        assert!((e.hour_of_day - 0.0).abs() < 1e-4, "hour={}", e.hour_of_day);
        e.tick(12.0);
        assert!((e.hour_of_day - 12.0).abs() < 1e-4);
        assert_eq!(e.day_phase(), DayPhase::Day);
    }

    #[test]
    fn day_night_multiplier_night_harder_than_day() {
        let mut day = Environment {
            hour_of_day: 12.0,
            ..Default::default()
        };
        let night = Environment {
            hour_of_day: 0.0,
            ..Default::default()
        };
        assert_eq!(day.day_phase(), DayPhase::Day);
        assert_eq!(night.day_phase(), DayPhase::Night);
        assert_eq!(day.day_night_multiplier(), 1.0);
        assert!(night.day_night_multiplier() > day.day_night_multiplier());
        // Twilight between day and night.
        day.hour_of_day = 6.0;
        assert_eq!(day.day_phase(), DayPhase::Dawn);
        assert!(day.day_night_multiplier() > 1.0);
        assert!(day.day_night_multiplier() < night.day_night_multiplier());
    }

    #[test]
    fn night_cools_temperature_after_tick() {
        let mut day = Environment {
            hour_of_day: 12.0,
            day_length: 10_000.0,
            season_length: 10_000.0,
            ..Default::default()
        };
        let mut night = Environment {
            hour_of_day: 0.0,
            day_length: 10_000.0,
            season_length: 10_000.0,
            ..Default::default()
        };
        day.tick(0.01);
        night.tick(0.01);
        assert!(
            night.temperature < day.temperature,
            "night={} day={}",
            night.temperature,
            day.temperature
        );
    }

    #[test]
    fn snapshot_includes_day_fields() {
        let e = Environment {
            hour_of_day: 0.5,
            ..Default::default()
        };
        let s = e.snapshot();
        assert!((s.hour_of_day - 0.5).abs() < 1e-6);
        assert_eq!(s.day_phase, "NIGHT");
        assert!((s.day_night_multiplier - 1.15).abs() < 1e-6);
    }

    #[test]
    fn time_query_text_includes_hour_and_phase() {
        let day = Environment {
            hour_of_day: 12.0,
            ..Default::default()
        };
        assert_eq!(day.time_query_text(), "TIME 12.00 DAY");
        let night = Environment {
            hour_of_day: 0.5,
            ..Default::default()
        };
        assert_eq!(night.time_query_text(), "TIME 0.50 NIGHT");
        let dawn = Environment {
            hour_of_day: 6.0,
            ..Default::default()
        };
        assert_eq!(dawn.time_query_text(), "TIME 6.00 DAWN");
    }

    #[test]
    fn season_parse_tokens() {
        assert_eq!(Season::parse("SPRING"), Some(Season::Spring));
        assert_eq!(Season::parse("summer"), Some(Season::Summer));
        assert_eq!(Season::parse("Autumn"), Some(Season::Autumn));
        assert_eq!(Season::parse("FALL"), Some(Season::Autumn));
        assert_eq!(Season::parse("WINTER"), Some(Season::Winter));
        assert_eq!(Season::parse("nope"), None);
    }

    #[test]
    fn set_season_forces_and_resets_elapsed() {
        let mut e = Environment {
            season: Season::Spring,
            season_elapsed: 50.0,
            hour_of_day: 12.0,
            day_length: 10_000.0,
            season_length: 10_000.0,
            ..Default::default()
        };
        e.set_season(Season::Winter);
        assert_eq!(e.season, Season::Winter);
        assert_eq!(e.season_elapsed, 0.0);
        // Winter starts cooler than spring baseline after recompute.
        assert!(e.temperature < 0.5, "winter temp={}", e.temperature);
        assert!(e.season_query_text().starts_with("WINTER"));
    }

    #[test]
    fn set_hour_forces_phase() {
        let mut e = Environment {
            hour_of_day: 12.0,
            day_length: 10_000.0,
            season_length: 10_000.0,
            ..Default::default()
        };
        e.set_hour(0.0);
        assert!((e.hour_of_day - 0.0).abs() < 1e-6);
        assert_eq!(e.day_phase(), DayPhase::Night);
        assert_eq!(e.time_query_text(), "TIME 0.00 NIGHT");
        e.set_hour(25.5); // wraps via rem_euclid
        assert!((e.hour_of_day - 1.5).abs() < 1e-4);
        e.set_hour(19.0);
        assert_eq!(e.day_phase(), DayPhase::Dusk);
    }

    #[test]
    fn clothing_temp_bonus_half_per_nonempty_slot() {
        assert_eq!(clothing_temp_bonus(0, 0, 0), 0.0);
        assert_eq!(clothing_temp_bonus(1, 0, 0), 0.5);
        assert_eq!(clothing_temp_bonus(0, 2, 0), 0.5);
        assert_eq!(clothing_temp_bonus(0, 0, 3), 0.5);
        assert_eq!(clothing_temp_bonus(10, 20, 0), 1.0);
        assert_eq!(clothing_temp_bonus(1, 2, 3), 1.5);
    }
}
