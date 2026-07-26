//! Prestige class (Haxe `openlife.server.Lineage.PrestigeClass` subset).
//!
//! Haxe assigns class from living-player percentile ranks of total score via
//! [`prestige_classes_from_living_scores`]. Fixed float thresholds
//! ([`PrestigeClass::from_prestige`]) remain for combat / single-score paths.

use std::collections::HashMap;

/// Haxe `PrestigeClass` int tags (gaps 4–5 reserved as Noble aliases in Haxe name table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PrestigeClass {
    NotSet = 0,
    Serf = 1,
    #[default]
    Commoner = 2,
    Noble = 3,
    King = 6,
    Emperor = 7,
}

/// Prestige below this → Serf.
pub const PRESTIGE_SERF_MAX: f32 = 10.0;
/// Prestige below this (and ≥ serf max) → Commoner.
pub const PRESTIGE_COMMONER_MAX: f32 = 50.0;
/// Prestige below this (and ≥ commoner max) → Noble.
pub const PRESTIGE_NOBLE_MAX: f32 = 100.0;
/// Prestige below this (and ≥ noble max) → King; else Emperor.
pub const PRESTIGE_KING_MAX: f32 = 200.0;

impl PrestigeClass {
    /// Assign class from a prestige float (sim threshold mapping).
    pub fn from_prestige(prestige: f32) -> Self {
        if !prestige.is_finite() || prestige < PRESTIGE_SERF_MAX {
            Self::Serf
        } else if prestige < PRESTIGE_COMMONER_MAX {
            Self::Commoner
        } else if prestige < PRESTIGE_NOBLE_MAX {
            Self::Noble
        } else if prestige < PRESTIGE_KING_MAX {
            Self::King
        } else {
            Self::Emperor
        }
    }

    /// Haxe wire / display name (lowercase, matches `PlayerSoul.getPrestigeClassName`).
    pub fn wire_name(self) -> &'static str {
        match self {
            Self::NotSet => "commoner",
            Self::Serf => "serf",
            Self::Commoner => "commoner",
            Self::Noble => "noble",
            Self::King => "king",
            Self::Emperor => "emperor",
        }
    }

    /// Haxe `Lineage.PrestigeClasses` title-case label.
    pub fn class_name(self) -> &'static str {
        match self {
            Self::NotSet => "Not Set",
            Self::Serf => "Serf",
            Self::Commoner => "Commoner",
            Self::Noble => "Noble",
            Self::King => "King",
            Self::Emperor => "Emperor",
        }
    }

    /// Haxe `Lineage.isNobleOrMore`.
    pub fn is_noble_or_more(self) -> bool {
        (self as u8) >= (Self::Noble as u8)
    }

    /// Haxe int discriminant.
    pub fn as_i32(self) -> i32 {
        self as u8 as i32
    }

    /// Parse Haxe int tag; unknown values map to `NotSet`.
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => Self::NotSet,
            1 => Self::Serf,
            2 => Self::Commoner,
            3 | 4 | 5 => Self::Noble, // Haxe name table aliases 4–5 as Noble
            6 => Self::King,
            7 => Self::Emperor,
            _ => Self::NotSet,
        }
    }
}

/// Player-info wire string (Haxe `PlayerSoul` first-person prestige line).
pub fn player_prestige_info_wire(prestige: f32) -> String {
    let class = PrestigeClass::from_prestige(prestige);
    format!(
        "You are a {} with prestige {}.",
        class.wire_name(),
        prestige.round() as i32
    )
}

/// Third-person player-info wire string (Haxe `PlayerSoul` look-at prestige line).
pub fn other_prestige_info_wire(prestige: f32) -> String {
    let class = PrestigeClass::from_prestige(prestige);
    format!(
        "They are a {} with prestige {}.",
        class.wire_name(),
        prestige.round() as i32
    )
}

/// Compact class+prestige token for lineage / bootstrap wire lines.
pub fn prestige_class_wire_token(prestige: f32) -> String {
    let class = PrestigeClass::from_prestige(prestige);
    format!("class={} prestige={}", class.wire_name(), prestige)
}

/// Prestige class from rank among living players.
///
/// `rank_index` 0 = lowest score. Percentile bands (fraction = rank_index / n):
/// bottom 20% Serf, next 30% Commoner, next 25% Noble, next 15% King, top 10% Emperor.
/// Single player (`n == 1`) → Commoner. `n == 0` → Commoner (unused).
pub fn prestige_class_from_percentile(rank_index: usize, n: usize) -> PrestigeClass {
    if n <= 1 {
        return PrestigeClass::Commoner;
    }
    let frac = rank_index as f64 / n as f64;
    if frac < 0.20 {
        PrestigeClass::Serf
    } else if frac < 0.50 {
        PrestigeClass::Commoner
    } else if frac < 0.75 {
        PrestigeClass::Noble
    } else if frac < 0.90 {
        PrestigeClass::King
    } else {
        PrestigeClass::Emperor
    }
}

/// Rank scores of living players and assign [`PrestigeClass`] by percentile.
///
/// `scores`: list of `(p_id, total_score)`. Higher score → higher class.
/// Ties: stable by ascending `p_id`. Empty input → empty map. Alone → Commoner.
pub fn prestige_classes_from_living_scores(scores: &[(i32, i32)]) -> HashMap<i32, PrestigeClass> {
    if scores.is_empty() {
        return HashMap::new();
    }
    let mut sorted: Vec<(i32, i32)> = scores.to_vec();
    // Ascending score (rank 0 = lowest); ties broken by p_id for stability.
    sorted.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    let n = sorted.len();
    sorted
        .into_iter()
        .enumerate()
        .map(|(rank, (p_id, _))| (p_id, prestige_class_from_percentile(rank, n)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_prestige_thresholds() {
        assert_eq!(PrestigeClass::from_prestige(0.0), PrestigeClass::Serf);
        assert_eq!(PrestigeClass::from_prestige(9.9), PrestigeClass::Serf);
        assert_eq!(PrestigeClass::from_prestige(10.0), PrestigeClass::Commoner);
        assert_eq!(PrestigeClass::from_prestige(49.9), PrestigeClass::Commoner);
        assert_eq!(PrestigeClass::from_prestige(50.0), PrestigeClass::Noble);
        assert_eq!(PrestigeClass::from_prestige(99.9), PrestigeClass::Noble);
        assert_eq!(PrestigeClass::from_prestige(100.0), PrestigeClass::King);
        assert_eq!(PrestigeClass::from_prestige(199.9), PrestigeClass::King);
        assert_eq!(PrestigeClass::from_prestige(200.0), PrestigeClass::Emperor);
        assert_eq!(PrestigeClass::from_prestige(f32::NAN), PrestigeClass::Serf);
    }

    #[test]
    fn wire_names_match_haxe() {
        assert_eq!(PrestigeClass::Serf.wire_name(), "serf");
        assert_eq!(PrestigeClass::Commoner.wire_name(), "commoner");
        assert_eq!(PrestigeClass::Noble.wire_name(), "noble");
        assert_eq!(PrestigeClass::King.wire_name(), "king");
        assert_eq!(PrestigeClass::Emperor.wire_name(), "emperor");
        assert_eq!(PrestigeClass::NotSet.wire_name(), "commoner");
    }

    #[test]
    fn i32_roundtrip_and_noble_aliases() {
        for c in [
            PrestigeClass::NotSet,
            PrestigeClass::Serf,
            PrestigeClass::Commoner,
            PrestigeClass::Noble,
            PrestigeClass::King,
            PrestigeClass::Emperor,
        ] {
            assert_eq!(PrestigeClass::from_i32(c.as_i32()), c);
        }
        assert_eq!(PrestigeClass::from_i32(4), PrestigeClass::Noble);
        assert_eq!(PrestigeClass::from_i32(5), PrestigeClass::Noble);
    }

    #[test]
    fn is_noble_or_more() {
        assert!(!PrestigeClass::Serf.is_noble_or_more());
        assert!(!PrestigeClass::Commoner.is_noble_or_more());
        assert!(PrestigeClass::Noble.is_noble_or_more());
        assert!(PrestigeClass::King.is_noble_or_more());
        assert!(PrestigeClass::Emperor.is_noble_or_more());
    }

    #[test]
    fn player_info_wire() {
        let s = player_prestige_info_wire(12.4);
        assert!(s.contains("commoner"));
        assert!(s.contains("12"));
        assert!(s.starts_with("You are a "));

        let t = other_prestige_info_wire(55.0);
        assert!(t.contains("noble"));
        assert!(t.starts_with("They are a "));
    }

    #[test]
    fn class_wire_token() {
        let tok = prestige_class_wire_token(5.0);
        assert!(tok.contains("class=serf"));
        assert!(tok.contains("prestige=5"));
    }

    #[test]
    fn living_scores_empty_and_alone() {
        assert!(prestige_classes_from_living_scores(&[]).is_empty());
        let alone = prestige_classes_from_living_scores(&[(7, 999)]);
        assert_eq!(alone.get(&7), Some(&PrestigeClass::Commoner));
        assert_eq!(prestige_class_from_percentile(0, 1), PrestigeClass::Commoner);
    }

    #[test]
    fn living_scores_ten_players_spread_classes() {
        // Increasing scores p_id 1..=10 → ranks 0..=9 by score order.
        let scores: Vec<(i32, i32)> = (1..=10).map(|i| (i, i * 10)).collect();
        let map = prestige_classes_from_living_scores(&scores);
        assert_eq!(map.len(), 10);
        // rank = p_id - 1 (lowest score first)
        assert_eq!(map[&1], PrestigeClass::Serf); // 0/10 = 0.0
        assert_eq!(map[&2], PrestigeClass::Serf); // 0.1
        assert_eq!(map[&3], PrestigeClass::Commoner); // 0.2
        assert_eq!(map[&5], PrestigeClass::Commoner); // 0.4
        assert_eq!(map[&6], PrestigeClass::Noble); // 0.5
        assert_eq!(map[&8], PrestigeClass::Noble); // 0.7
        assert_eq!(map[&9], PrestigeClass::King); // 0.8
        assert_eq!(map[&10], PrestigeClass::Emperor); // 0.9
        // Spread: at least Serf, Commoner, Noble, King, Emperor present.
        let classes: std::collections::HashSet<_> = map.values().copied().collect();
        assert!(classes.contains(&PrestigeClass::Serf));
        assert!(classes.contains(&PrestigeClass::Commoner));
        assert!(classes.contains(&PrestigeClass::Noble));
        assert!(classes.contains(&PrestigeClass::King));
        assert!(classes.contains(&PrestigeClass::Emperor));
    }

    #[test]
    fn living_scores_ties_stable_by_p_id() {
        // Same score: lower p_id ranks lower (Serf-ish first).
        let map = prestige_classes_from_living_scores(&[(3, 50), (1, 50), (2, 50)]);
        // ranks: p_id 1, 2, 3 → 0/3, 1/3, 2/3 → Serf, Commoner, Noble
        assert_eq!(map[&1], PrestigeClass::Serf);
        assert_eq!(map[&2], PrestigeClass::Commoner);
        assert_eq!(map[&3], PrestigeClass::Noble);
    }
}
