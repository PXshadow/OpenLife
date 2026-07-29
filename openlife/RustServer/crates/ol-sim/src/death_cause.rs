//! Death reason taxonomy (Haxe-style `reason_*` wire tags).
//!
//! Pure classification for event logs, graves, and metrics. Does not mutate
//! players — callers set `Player::death_reason` and call these helpers.
//!
//! Haxe: `GlobalPlayerInstance.doDeath` / `doDeathHelper` reason strings:
//! - `reason_hunger`
//! - `reason_nursing_hunger`
//! - `reason_age`
//! - `reason_suicide` (Rust DIE / SAY DIE)
//! - `reason_killed` / `reason_killed_<objectId>` (woundedBy object)
//! - `reason_killed_legal` (Rust combat legality extension)
//! - `reason_disconnected`
//! - `reason_unknown`

/// Canonical death causes used by the sim (subset of OHOL / Open Life tags).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeathCause {
    /// Food drained to zero (`reason_hunger`).
    Hunger,
    /// Starved while nursing a hungry baby (`reason_nursing_hunger`).
    NursingHunger,
    /// Age exceeded max (`reason_age`).
    Age,
    /// Combat kill (illegal / default) (`reason_killed` or `reason_killed_<id>`).
    Killed,
    /// Combat kill treated as legal (exile/posse) (`reason_killed_legal`).
    KilledLegal,
    /// Voluntary `SAY DIE` / client DIE (`reason_suicide`).
    Suicide,
    /// Client disconnect death (`reason_disconnected`).
    Disconnected,
    /// Unrecognized or empty string.
    Unknown,
}

impl DeathCause {
    /// Parse a stored / wire reason string (case-sensitive `reason_*` tags).
    ///
    /// Accepts bare `reason_killed` and Haxe `reason_killed_<objectId>` as [`Self::Killed`].
    /// `reason_killed_legal` is checked before the `reason_killed_` prefix.
    pub fn from_reason(s: &str) -> Self {
        let t = s.trim();
        match t {
            "reason_hunger" => Self::Hunger,
            "reason_nursing_hunger" => Self::NursingHunger,
            "reason_age" => Self::Age,
            "reason_killed" => Self::Killed,
            "reason_killed_legal" => Self::KilledLegal,
            "reason_suicide" => Self::Suicide,
            "reason_disconnected" => Self::Disconnected,
            "reason_unknown" => Self::Unknown,
            _ if t.starts_with("reason_killed_") => Self::Killed,
            _ => Self::Unknown,
        }
    }

    /// Wire / storage tag written onto `Player::death_reason` for enum-only causes.
    ///
    /// For object-tagged kills use [`killed_by_object_wire`] instead.
    pub fn wire_tag(self) -> &'static str {
        match self {
            Self::Hunger => "reason_hunger",
            Self::NursingHunger => "reason_nursing_hunger",
            Self::Age => "reason_age",
            Self::Killed => "reason_killed",
            Self::KilledLegal => "reason_killed_legal",
            Self::Suicide => "reason_suicide",
            Self::Disconnected => "reason_disconnected",
            Self::Unknown => "reason_unknown",
        }
    }

    /// Short human label for logs / admin UI.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Hunger => "hunger",
            Self::NursingHunger => "nursing hunger",
            Self::Age => "old age",
            Self::Killed => "killed",
            Self::KilledLegal => "killed (legal)",
            Self::Suicide => "suicide",
            Self::Disconnected => "disconnected",
            Self::Unknown => "unknown",
        }
    }

    /// True if this death should count as a PvP kill for scoreboards.
    pub fn is_pvp_kill(self) -> bool {
        matches!(self, Self::Killed | Self::KilledLegal)
    }

    /// True if this is a natural / environmental death (not combat or suicide).
    pub fn is_natural(self) -> bool {
        matches!(self, Self::Hunger | Self::NursingHunger | Self::Age)
    }

    /// True if the kill was legal under crime / exile rules.
    pub fn is_legal_kill(self) -> bool {
        matches!(self, Self::KilledLegal)
    }
}

/// Build combat death cause from legality flag (matches KILL / HIT paths).
pub fn combat_death(legal: bool) -> DeathCause {
    if legal {
        DeathCause::KilledLegal
    } else {
        DeathCause::Killed
    }
}

/// Haxe `reason_killed_${woundedBy}` when `object_id != 0`, else bare `reason_killed`.
pub fn killed_by_object_wire(object_id: i32) -> String {
    if object_id != 0 {
        format!("reason_killed_{object_id}")
    } else {
        DeathCause::Killed.wire_tag().into()
    }
}

/// Wire tag for a combat death (legal flag + optional wounding object).
///
/// Legal kills keep `reason_killed_legal` for scoreboard classification; illegal
/// kills use Haxe object-id form when `wounded_by != 0`.
pub fn combat_death_wire(legal: bool, wounded_by: i32) -> String {
    if legal {
        DeathCause::KilledLegal.wire_tag().into()
    } else {
        killed_by_object_wire(wounded_by)
    }
}

/// Haxe TimeHelper food death: hunger if unwounded, else `reason_killed_${woundedBy}`.
pub fn hunger_death_wire(wounded_by: i32) -> String {
    food_death_wire(wounded_by, false)
}

/// Food death wire with optional nursing flag (GPI-DEATH-POLISH).
///
/// Priority: wounded object kill tag > `reason_nursing_hunger` when holding/nursing
/// a baby > bare `reason_hunger`.
pub fn food_death_wire(wounded_by: i32, nursing: bool) -> String {
    if wounded_by != 0 {
        killed_by_object_wire(wounded_by)
    } else if nursing {
        DeathCause::NursingHunger.wire_tag().into()
    } else {
        DeathCause::Hunger.wire_tag().into()
    }
}

/// Parse trailing object id from `reason_killed_<id>` (not from `reason_killed_legal`).
pub fn parse_killed_object_id(reason: &str) -> Option<i32> {
    let t = reason.trim();
    if t == "reason_killed_legal" || t == "reason_killed" {
        return None;
    }
    t.strip_prefix("reason_killed_")
        .and_then(|rest| rest.parse::<i32>().ok())
        .filter(|&id| id != 0)
}

/// `DEATH <p_id> <reason_tag>` event-log line (matches existing sim event log).
pub fn format_death_event(p_id: i32, cause: DeathCause) -> String {
    format!("DEATH {p_id} {}", cause.wire_tag())
}

/// `DEATH <p_id> <reason_tag>` with an arbitrary wire tag (e.g. `reason_killed_752`).
pub fn format_death_event_tag(p_id: i32, reason_tag: &str) -> String {
    format!("DEATH {p_id} {reason_tag}")
}

/// `CAUSE hunger|age|… tag=reason_*` query body (no leading p_id).
pub fn format_cause_query(cause: DeathCause) -> String {
    format!("CAUSE {} tag={}", cause.display_name(), cause.wire_tag())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_known_tags() {
        assert_eq!(DeathCause::from_reason("reason_hunger"), DeathCause::Hunger);
        assert_eq!(
            DeathCause::from_reason("reason_nursing_hunger"),
            DeathCause::NursingHunger
        );
        assert_eq!(DeathCause::from_reason("reason_age"), DeathCause::Age);
        assert_eq!(DeathCause::from_reason("reason_killed"), DeathCause::Killed);
        assert_eq!(
            DeathCause::from_reason("reason_killed_legal"),
            DeathCause::KilledLegal
        );
        assert_eq!(
            DeathCause::from_reason("reason_killed_752"),
            DeathCause::Killed
        );
        assert_eq!(
            DeathCause::from_reason("reason_killed_0"),
            DeathCause::Killed
        );
        assert_eq!(
            DeathCause::from_reason("reason_suicide"),
            DeathCause::Suicide
        );
        assert_eq!(
            DeathCause::from_reason("reason_disconnected"),
            DeathCause::Disconnected
        );
        assert_eq!(DeathCause::from_reason("bogus"), DeathCause::Unknown);
        assert_eq!(DeathCause::from_reason(""), DeathCause::Unknown);
        assert_eq!(
            DeathCause::from_reason("  reason_hunger  "),
            DeathCause::Hunger
        );
    }

    #[test]
    fn wire_roundtrip() {
        for c in [
            DeathCause::Hunger,
            DeathCause::NursingHunger,
            DeathCause::Age,
            DeathCause::Killed,
            DeathCause::KilledLegal,
            DeathCause::Suicide,
            DeathCause::Disconnected,
        ] {
            assert_eq!(DeathCause::from_reason(c.wire_tag()), c);
        }
        // Unknown's wire tag is not reverse-parsed as a known tag by design
        // (sim never stores reason_unknown today).
        assert_eq!(
            DeathCause::from_reason(DeathCause::Unknown.wire_tag()),
            DeathCause::Unknown
        );
    }

    #[test]
    fn classification_helpers() {
        assert!(DeathCause::Hunger.is_natural());
        assert!(DeathCause::NursingHunger.is_natural());
        assert!(DeathCause::Age.is_natural());
        assert!(!DeathCause::Killed.is_natural());
        assert!(DeathCause::Killed.is_pvp_kill());
        assert!(DeathCause::KilledLegal.is_pvp_kill());
        assert!(DeathCause::KilledLegal.is_legal_kill());
        assert!(!DeathCause::Killed.is_legal_kill());
        assert!(!DeathCause::Suicide.is_pvp_kill());
        assert!(!DeathCause::Disconnected.is_pvp_kill());
    }

    #[test]
    fn combat_and_formatters() {
        assert_eq!(combat_death(true), DeathCause::KilledLegal);
        assert_eq!(combat_death(false), DeathCause::Killed);
        assert_eq!(
            format_death_event(42, DeathCause::Hunger),
            "DEATH 42 reason_hunger"
        );
        let q = format_cause_query(DeathCause::Age);
        assert!(q.contains("old age"));
        assert!(q.contains("reason_age"));
        assert_eq!(killed_by_object_wire(752), "reason_killed_752");
        assert_eq!(killed_by_object_wire(0), "reason_killed");
        assert_eq!(hunger_death_wire(0), "reason_hunger");
        assert_eq!(hunger_death_wire(87), "reason_killed_87");
        assert_eq!(food_death_wire(0, false), "reason_hunger");
        assert_eq!(food_death_wire(0, true), "reason_nursing_hunger");
        assert_eq!(food_death_wire(87, true), "reason_killed_87"); // wound wins
        assert_eq!(combat_death_wire(true, 99), "reason_killed_legal");
        assert_eq!(combat_death_wire(false, 99), "reason_killed_99");
        assert_eq!(combat_death_wire(false, 0), "reason_killed");
        assert_eq!(parse_killed_object_id("reason_killed_752"), Some(752));
        assert_eq!(parse_killed_object_id("reason_killed"), None);
        assert_eq!(parse_killed_object_id("reason_killed_legal"), None);
        assert_eq!(
            format_death_event_tag(3, "reason_killed_752"),
            "DEATH 3 reason_killed_752"
        );
    }
}
