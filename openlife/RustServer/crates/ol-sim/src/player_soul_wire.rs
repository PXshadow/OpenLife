//! Live-wire helpers for AI-SOUL-WIRE (`Player.soul` / SoulView assembly).
//!
//! Pure functions used by `SimState::soul_view_for` and unit tests.
//! Included from [`super`] (`player_soul.rs`).

/// Haxe `ServerSettings.TemperatureImpactBelow` — base half-width for super hot/cold.
// Haxe: ServerSettings.TemperatureImpactBelow
pub const TEMPERATURE_IMPACT_BELOW: f32 = 0.6;
/// Haxe `ServerSettings.TemperatureImpactColorFactor`.
// Haxe: ServerSettings.TemperatureImpactColorFactor
pub const TEMPERATURE_IMPACT_COLOR_FACTOR: f32 = 0.5;

/// Haxe `PersonColor` race ids (ObjectData.person) — match `multi_use::PERSON_*`.
pub const PERSON_COLOR_BLACK: i32 = 1;
pub const PERSON_COLOR_BROWN: i32 = 3;
pub const PERSON_COLOR_WHITE: i32 = 4;
pub const PERSON_COLOR_GINGER: i32 = 6;

/// Haxe `GlobalPlayerInstance.isAngryOrTerrified` — `angryTime < 1`.
// Haxe: GlobalPlayerInstance.isAngryOrTerrified
#[inline]
pub fn is_angry_or_terrified(angry_time: f32) -> bool {
    angry_time < 1.0
}

/// Haxe person.male==false proxy from display object name / default Female001 (19).
///
/// When `content_male` is `Some`, use Haxe `ObjectData.male` directly (`!male` ⇒ female).
// Haxe: GlobalPlayerInstance.isFemale / ObjectData.male
pub fn person_looks_female(
    display_object_id: i32,
    name: &str,
    description: &str,
) -> bool {
    person_is_female(display_object_id, name, description, None)
}

/// Female check with optional content `ObjectData.male` flag.
// Haxe: GlobalPlayerInstance.isFemale
pub fn person_is_female(
    display_object_id: i32,
    name: &str,
    description: &str,
    content_male: Option<bool>,
) -> bool {
    if let Some(male) = content_male {
        return !male;
    }
    let n = name.to_ascii_lowercase();
    let desc = description.to_ascii_lowercase();
    if n.contains("female") || desc.contains("female") {
        return true;
    }
    if n.contains("male") || desc.contains("male") {
        return false;
    }
    // Haxe default person object Female001 = 19
    display_object_id == 19 || display_object_id == 0
}

/// Email heuristics for AI / NPC / selfplay (matches move-speed is_ai paths).
pub fn email_looks_ai(email: &str) -> bool {
    let email_l = email.to_ascii_lowercase();
    email_l.contains("ai@")
        || email_l.starts_with("ai_")
        || email_l.contains("npc")
        || email_l.contains("selfplay")
}

/// Title-case Haxe `SeasonNames` token for soul season line.
pub fn season_display_name(season_token: &str) -> String {
    match season_token.trim().to_ascii_uppercase().as_str() {
        "SPRING" => "Spring".into(),
        "SUMMER" => "Summer".into(),
        "AUTUMN" | "FALL" => "Autumn".into(),
        "WINTER" => "Winter".into(),
        other if other.is_empty() => "DONT KNOW".into(),
        other => {
            let mut c = other.chars();
            match c.next() {
                Some(f) => format!("{}{}", f.to_uppercase(), c.as_str().to_ascii_lowercase()),
                None => "DONT KNOW".into(),
            }
        }
    }
}

/// Haxe `TimeHelper.SeasonText` shape: optional hard prefix + season name.
///
/// `hardness` is the **pre-square** Haxe `SeasonHardness` when known; use `1.0` for mild.
// Haxe: TimeHelper.DoSeason SeasonText
pub fn haxe_season_text(season_token: &str, hardness: f32) -> String {
    let name = season_display_name(season_token);
    let is_hard_season = matches!(
        season_token.trim().to_ascii_uppercase().as_str(),
        "WINTER" | "SUMMER"
    );
    let hard = is_hard_season && hardness > 1.25;
    if !hard {
        return name;
    }
    if hardness > 1.4 {
        format!("A very hard  {name}")
    } else {
        // Haxe: 'A hard ' + name → double-space in golden ("A hard  Winter")
        format!("A hard  {name}")
    }
}

/// Pre-square hardness from unit random + optional hard-season square (Haxe DoSeason).
///
/// Returns `(season_text, operational_hardness)`.
/// SeasonText uses pre-square hardness; operational hardness is squared only when
/// Winter/Summer and pre > 1.25 (Haxe bumps +0.1 before square when pre > 1.4).
// Haxe: TimeHelper.DoSeason SeasonHardness + SeasonText
pub fn haxe_season_roll_text_and_hardness(
    season_token: &str,
    unit_random: f32,
) -> (String, f32) {
    let u = if unit_random.is_finite() {
        unit_random.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let mut pre = u + 0.5; // [0.5, 1.5]
    // SeasonText from pre-square value (before +0.1 / square).
    let text = haxe_season_text(season_token, pre);
    let is_ws = matches!(
        season_token.trim().to_ascii_uppercase().as_str(),
        "WINTER" | "SUMMER"
    );
    let hard_season = is_ws && pre > 1.25;
    if hard_season && pre > 1.4 {
        pre += 0.1; // Haxe: make it even harder before square
    }
    let operational = if hard_season { pre * pre } else { pre };
    (text, operational)
}

/// Assigned / last profession strings from sticky smith/baker/farm + free GPI strings.
///
/// Priority: free Haxe `assignedProfession`/`lastProfession` when non-empty, else
/// smith flags, else baker flags, else farm keys (`FarmProfession::as_str`).
///
/// Returns `(assigned, last)` for [`super::get_profession_text`].
// Haxe: GlobalPlayerInstance.assignedProfession / lastProfession + AiBase profession map
pub fn sticky_profession_pair(
    smith_assigned: bool,
    smith_last: bool,
    baker_assigned: bool,
    baker_last: bool,
    farm_assigned: Option<&str>,
    farm_last: Option<&str>,
    free_assigned: Option<&str>,
    free_last: Option<&str>,
) -> (Option<String>, Option<String>) {
    let assigned = non_empty_owned(free_assigned).or_else(|| {
        if smith_assigned {
            Some("SMITH".into())
        } else if baker_assigned {
            Some("BAKER".into())
        } else {
            non_empty_owned(farm_assigned)
        }
    });
    let last = non_empty_owned(free_last).or_else(|| {
        if smith_last {
            Some("SMITH".into())
        } else if baker_last {
            Some("BAKER".into())
        } else {
            non_empty_owned(farm_last)
        }
    });
    (assigned, last)
}

#[inline]
fn non_empty_owned(s: Option<&str>) -> Option<String> {
    s.map(str::trim).filter(|t| !t.is_empty()).map(|t| t.to_string())
}

/// Haxe `GlobalPlayerInstance.isSuperHot` with person-color thresholds.
///
/// Base threshold `0.5 + 0.5 * TemperatureImpactBelow` (= 0.8 at default 0.6),
/// then Black/Brown/White raise the bar (harder to be "super hot").
// Haxe: GlobalPlayerInstance.isSuperHot
pub fn is_super_hot_for_person(heat: f32, person_color: i32) -> bool {
    if !heat.is_finite() {
        return false;
    }
    let mut too_hot = 0.5 + 0.5 * TEMPERATURE_IMPACT_BELOW;
    let factor = TEMPERATURE_IMPACT_COLOR_FACTOR;
    match person_color {
        PERSON_COLOR_BLACK => too_hot += 0.2 * factor,
        PERSON_COLOR_BROWN => too_hot += 0.1 * factor,
        PERSON_COLOR_WHITE => too_hot += 0.05 * factor,
        _ => {}
    }
    heat > too_hot
}

/// Haxe `GlobalPlayerInstance.isSuperCold` with person-color thresholds.
///
/// Base threshold `0.5 - 0.5 * TemperatureImpactBelow` (= 0.2 at default 0.6),
/// then Ginger/White/Brown lower the bar (harder to be "super cold").
// Haxe: GlobalPlayerInstance.isSuperCold
pub fn is_super_cold_for_person(heat: f32, person_color: i32) -> bool {
    if !heat.is_finite() {
        return false;
    }
    let mut too_cold = 0.5 - 0.5 * TEMPERATURE_IMPACT_BELOW;
    let factor = TEMPERATURE_IMPACT_COLOR_FACTOR;
    match person_color {
        PERSON_COLOR_GINGER => too_cold -= 0.2 * factor,
        PERSON_COLOR_WHITE => too_cold -= 0.1 * factor,
        PERSON_COLOR_BROWN => too_cold -= 0.05 * factor,
        _ => {}
    }
    heat < too_cold
}

/// Format `"First Family"` display for parent links.
pub fn parent_display_name(first: &str, family: &str) -> String {
    format!("{} {}", first.trim(), family.trim())
}

/// Home option: Haxe treats (0,0) as unset.
#[inline]
pub fn home_option(home_x: i32, home_y: i32) -> Option<(i32, i32)> {
    if home_x == 0 && home_y == 0 {
        None
    } else {
        Some((home_x, home_y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angry_or_terrified_threshold() {
        assert!(is_angry_or_terrified(0.0));
        assert!(is_angry_or_terrified(0.9));
        assert!(!is_angry_or_terrified(1.0));
        assert!(!is_angry_or_terrified(5.0));
    }

    #[test]
    fn person_looks_female_default_19() {
        assert!(person_looks_female(19, "Female001", ""));
        assert!(person_looks_female(0, "unknown", ""));
        assert!(!person_looks_female(20, "Male01", ""));
        assert!(person_looks_female(99, "Something female", ""));
    }

    #[test]
    fn person_is_female_content_male_flag() {
        // Haxe ObjectData.male wins over name heuristic
        assert!(!person_is_female(19, "Female001", "", Some(true)));
        assert!(person_is_female(20, "Male01", "", Some(false)));
    }

    #[test]
    fn email_ai_heuristics() {
        assert!(email_looks_ai("ai@test"));
        assert!(email_looks_ai("npc1@local"));
        assert!(email_looks_ai("selfplay_0@x"));
        assert!(!email_looks_ai("human@example.com"));
    }

    #[test]
    fn season_text_hard_prefix() {
        assert_eq!(haxe_season_text("WINTER", 1.0), "Winter");
        assert_eq!(haxe_season_text("WINTER", 1.3), "A hard  Winter");
        assert_eq!(haxe_season_text("SUMMER", 1.5), "A very hard  Summer");
        assert_eq!(haxe_season_text("SPRING", 2.0), "Spring");
    }

    #[test]
    fn season_roll_hard_and_very_hard() {
        // unit 0.8 → pre 1.3 → hard Winter, operational 1.69
        let (text, op) = haxe_season_roll_text_and_hardness("WINTER", 0.8);
        assert_eq!(text, "A hard  Winter");
        assert!((op - 1.3 * 1.3).abs() < 1e-4, "op={op}");
        // unit 0.95 → pre 1.45 → +0.1 → 1.55 → very hard, op = 1.55^2
        let (text2, op2) = haxe_season_roll_text_and_hardness("SUMMER", 0.95);
        assert_eq!(text2, "A very hard  Summer");
        assert!((op2 - 1.55 * 1.55).abs() < 1e-4, "op2={op2}");
        // Spring never hard-prefixed
        let (text3, op3) = haxe_season_roll_text_and_hardness("SPRING", 1.0);
        assert_eq!(text3, "Spring");
        assert!((op3 - 1.5).abs() < 1e-4);
    }

    #[test]
    fn sticky_profession_pair_smith_baker() {
        let (a, l) = sticky_profession_pair(true, true, false, false, None, None, None, None);
        assert_eq!(a.as_deref(), Some("SMITH"));
        assert_eq!(l.as_deref(), Some("SMITH"));
        let (a2, l2) = sticky_profession_pair(false, false, true, true, None, None, None, None);
        assert_eq!(a2.as_deref(), Some("BAKER"));
        assert_eq!(l2.as_deref(), Some("BAKER"));
        let (a3, l3) = sticky_profession_pair(true, false, false, true, None, None, None, None);
        assert_eq!(a3.as_deref(), Some("SMITH"));
        assert_eq!(l3.as_deref(), Some("BAKER"));
    }

    #[test]
    fn sticky_profession_pair_farm_and_free() {
        let (a, l) = sticky_profession_pair(
            false,
            false,
            false,
            false,
            Some("BASICFARMER"),
            Some("CARROTFARMER"),
            None,
            None,
        );
        assert_eq!(a.as_deref(), Some("BASICFARMER"));
        assert_eq!(l.as_deref(), Some("CARROTFARMER"));
        // free strings override sticky
        let (a2, l2) = sticky_profession_pair(
            true,
            true,
            false,
            false,
            Some("BASICFARMER"),
            Some("BASICFARMER"),
            Some("SHEPHERD"),
            Some("HUNTER"),
        );
        assert_eq!(a2.as_deref(), Some("SHEPHERD"));
        assert_eq!(l2.as_deref(), Some("HUNTER"));
        // smith wins over farm when free unset
        let (a3, _) = sticky_profession_pair(
            true,
            false,
            false,
            false,
            Some("BASICFARMER"),
            None,
            None,
            None,
        );
        assert_eq!(a3.as_deref(), Some("SMITH"));
    }

    #[test]
    fn super_hot_cold_person_color_thresholds() {
        // Neutral color: tooHot=0.8, tooCold=0.2
        assert!(!is_super_hot_for_person(0.8, 0));
        assert!(is_super_hot_for_person(0.81, 0));
        assert!(!is_super_cold_for_person(0.2, 0));
        assert!(is_super_cold_for_person(0.19, 0));
        // Black: tooHot = 0.8 + 0.1 = 0.9
        assert!(!is_super_hot_for_person(0.89, PERSON_COLOR_BLACK));
        assert!(is_super_hot_for_person(0.91, PERSON_COLOR_BLACK));
        // Ginger: tooCold = 0.2 - 0.1 = 0.1
        assert!(!is_super_cold_for_person(0.1, PERSON_COLOR_GINGER));
        assert!(is_super_cold_for_person(0.09, PERSON_COLOR_GINGER));
        // Brown hot: 0.8 + 0.05 = 0.85
        assert!(!is_super_hot_for_person(0.85, PERSON_COLOR_BROWN));
        assert!(is_super_hot_for_person(0.86, PERSON_COLOR_BROWN));
    }

    #[test]
    fn home_option_unset_at_origin() {
        assert_eq!(home_option(0, 0), None);
        assert_eq!(home_option(5, 0), Some((5, 0)));
    }
}
