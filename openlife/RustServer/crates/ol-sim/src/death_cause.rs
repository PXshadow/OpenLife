//! Death reason taxonomy (Haxe-style `reason_*` wire tags).
//!
//! Pure classification for event logs, graves, and metrics. Does not mutate
//! players — callers set `Player::death_reason` and call these helpers.

/// Canonical death causes used by the sim (subset of OHOL / Open Life tags).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeathCause {
    /// Food drained to zero (`reason_hunger`).
    Hunger,
    /// Age exceeded max (`reason_age`).
    Age,
    /// Combat kill (illegal / default) (`reason_killed`).
    Killed,
    /// Combat kill treated as legal (exile/posse) (`reason_killed_legal`).
    KilledLegal,
    /// Voluntary `SAY DIE` / client DIE (`reason_suicide`).
    Suicide,
    /// Unrecognized or empty string.
    Unknown,
}

impl DeathCause {
    /// Parse a stored / wire reason string (case-sensitive `reason_*` tags).
    pub fn from_reason(s: &str) -> Self {
        match s.trim() {
            "reason_hunger" => Self::Hunger,
            "reason_age" => Self::Age,
            "reason_killed" => Self::Killed,
            "reason_killed_legal" => Self::KilledLegal,
            "reason_suicide" => Self::Suicide,
            _ => Self::Unknown,
        }
    }

    /// Wire / storage tag written onto `Player::death_reason`.
    pub fn wire_tag(self) -> &'static str {
        match self {
            Self::Hunger => "reason_hunger",
            Self::Age => "reason_age",
            Self::Killed => "reason_killed",
            Self::KilledLegal => "reason_killed_legal",
            Self::Suicide => "reason_suicide",
            Self::Unknown => "reason_unknown",
        }
    }

    /// Short human label for logs / admin UI.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Hunger => "hunger",
            Self::Age => "old age",
            Self::Killed => "killed",
            Self::KilledLegal => "killed (legal)",
            Self::Suicide => "suicide",
            Self::Unknown => "unknown",
        }
    }

    /// True if this death should count as a PvP kill for scoreboards.
    pub fn is_pvp_kill(self) -> bool {
        matches!(self, Self::Killed | Self::KilledLegal)
    }

    /// True if this is a natural / environmental death (not combat or suicide).
    pub fn is_natural(self) -> bool {
        matches!(self, Self::Hunger | Self::Age)
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

/// `DEATH <p_id> <reason_tag>` event-log line (matches existing sim event log).
pub fn format_death_event(p_id: i32, cause: DeathCause) -> String {
    format!("DEATH {p_id} {}", cause.wire_tag())
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
        assert_eq!(DeathCause::from_reason("reason_age"), DeathCause::Age);
        assert_eq!(DeathCause::from_reason("reason_killed"), DeathCause::Killed);
        assert_eq!(
            DeathCause::from_reason("reason_killed_legal"),
            DeathCause::KilledLegal
        );
        assert_eq!(
            DeathCause::from_reason("reason_suicide"),
            DeathCause::Suicide
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
            DeathCause::Age,
            DeathCause::Killed,
            DeathCause::KilledLegal,
            DeathCause::Suicide,
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
        assert!(DeathCause::Age.is_natural());
        assert!(!DeathCause::Killed.is_natural());
        assert!(DeathCause::Killed.is_pvp_kill());
        assert!(DeathCause::KilledLegal.is_pvp_kill());
        assert!(DeathCause::KilledLegal.is_legal_kill());
        assert!(!DeathCause::Killed.is_legal_kill());
        assert!(!DeathCause::Suicide.is_pvp_kill());
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
    }
}
