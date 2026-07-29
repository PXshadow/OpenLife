//! Prestige class (Haxe `openlife.server.Lineage.PrestigeClass` + class boni).
//!
//! Haxe assigns class from living-player percentile ranks of total score via
//! [`prestige_classes_from_living_scores`]. Fixed float thresholds
//! ([`PrestigeClass::from_prestige`]) remain for combat / single-score paths.
//!
//! CLASS-BONI / `prestige_class_table`:
//! - [`PRESTIGE_CLASS_NAMES`] — Haxe `Lineage.PrestigeClasses` index table
//! - [`calculate_class_boni`] — Haxe `GlobalPlayerInstance.calculateClassBoni`
//!
//! NOOB-NOBLE-SPAWN / `spawn_weights`:
//! - [`apply_noob_noble_spawn_weight`] — Haxe TODO L1276 + design L6850
//!   (50% noble birth in first 5 lives)

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

/// Haxe `Lineage.PrestigeClasses` title-case labels indexed by prestige-class int.
///
/// Indices 4 and 5 are Noble aliases (Haxe name table only; enum gaps).
/// // Haxe: Lineage.PrestigeClasses
pub const PRESTIGE_CLASS_NAMES: [&str; 8] = [
    "Not Set",  // 0 NotSet
    "Serf",     // 1
    "Commoner", // 2
    "Noble",    // 3
    "Noble",    // 4 alias
    "Noble",    // 5 alias
    "King",     // 6
    "Emperor",  // 7
];

/// Prestige below this → Serf.
pub const PRESTIGE_SERF_MAX: f32 = 10.0;
/// Prestige below this (and ≥ serf max) → Commoner.
pub const PRESTIGE_COMMONER_MAX: f32 = 50.0;
/// Prestige below this (and ≥ commoner max) → Noble.
pub const PRESTIGE_NOBLE_MAX: f32 = 100.0;
/// Prestige below this (and ≥ noble max) → King; else Emperor.
pub const PRESTIGE_KING_MAX: f32 = 200.0;

/// Haxe `calculateClassBoni` same-class bonus.
pub const CLASS_BONI_SAME: f32 = 2.0;
/// Haxe `calculateClassBoni` Noble↔Serf mismatch mali.
pub const CLASS_BONI_NOBLE_SERF: f32 = -3.0;

/// Haxe design note L6850: first N lives count as "noob" for noble birth weight.
///
/// Count is **completed lives before this birth** (`AccountRecord.lives` is
/// incremented in `on_spawn` **after** birth class is chosen).
// Haxe: GlobalPlayerInstance TODO L1276 / design L6850 "first 5 lifes"
pub const NOOB_NOBLE_MAX_LIVES: u32 = 5;

/// Haxe design note L6850: 50% chance of noble birth while still a noob.
// Haxe: "(new players have a 50% change of noble birth in their first 5 lifes)"
pub const NOOB_NOBLE_BIRTH_CHANCE: f32 = 0.5;

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

    /// Haxe `Lineage.PrestigeClasses` title-case label for this enum value.
    pub fn class_name(self) -> &'static str {
        prestige_class_name_at_index(self.as_i32())
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
    /// Noble aliases 4–5 normalize to [`PrestigeClass::Noble`].
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

/// Haxe `Lineage.PrestigeClasses[index]` lookup (0..=7). Out of range → `"Not Set"`.
// Haxe: Lineage.get_className / PrestigeClasses
pub fn prestige_class_name_at_index(index: i32) -> &'static str {
    if index >= 0 && (index as usize) < PRESTIGE_CLASS_NAMES.len() {
        PRESTIGE_CLASS_NAMES[index as usize]
    } else {
        PRESTIGE_CLASS_NAMES[0]
    }
}

/// Haxe `GlobalPlayerInstance.calculateClassBoni`.
///
/// Birth-fitness class table:
/// - same class → `+2`
/// - Noble ↔ Serf (either direction) → `-3`
/// - otherwise → `0`
///
/// `self_class` is the parent (caller); `other_class` is the child (mother fitness)
/// or mother (father fitness: `p.calculateClassBoni(mother)`).
// Haxe: GlobalPlayerInstance.calculateClassBoni
pub fn calculate_class_boni(self_class: PrestigeClass, other_class: PrestigeClass) -> f32 {
    if self_class == other_class {
        return CLASS_BONI_SAME;
    }
    let noble_serf = matches!(
        (self_class, other_class),
        (PrestigeClass::Noble, PrestigeClass::Serf)
            | (PrestigeClass::Serf, PrestigeClass::Noble)
    );
    if noble_serf {
        CLASS_BONI_NOBLE_SERF
    } else {
        0.0
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
/// Used by [`prestige_classes_from_living_scores`] / scoreboard snapshot for
/// **online** class tags (King/Emperor bands). Birth class uses
/// [`calculate_prestige_class_at_birth`] instead (Haxe `calculatePrestigeClass`).
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

/// Haxe `GlobalPlayerInstance.CalculateNeededPrestige`.
///
/// `lineage_prestiges_asc` must be ordered by ascending **total** prestige
/// (`lineagePrestige + 4 * familyPrestige`); each element is that player's
/// **lineage** prestige float (the value returned as the cutoff).
///
/// Walks 1-indexed count until `count >= n * percent`, then returns that
/// player's lineage prestige. Empty / no match → `999_999.0`.
// Haxe: GlobalPlayerInstance.CalculateNeededPrestige
pub fn calculate_needed_prestige(lineage_prestiges_asc: &[f32], percent: f32) -> f32 {
    let n = lineage_prestiges_asc.len();
    if n == 0 {
        return 999_999.0;
    }
    let threshold = n as f32 * percent;
    let mut count = 0u32;
    for &lp in lineage_prestiges_asc {
        count += 1;
        if (count as f32) < threshold {
            continue;
        }
        return lp;
    }
    999_999.0
}

/// Haxe `GlobalPlayerInstance.calculatePrestigeClass` — class assigned **at birth**
/// before `GetFittestMother` (not the living percentile King/Emperor bands).
///
/// Rules (Haxe):
/// - `n < 2` living → Commoner
/// - sort living by total prestige ascending (`lineage + 4*family`)
/// - if `account_total_score < needed(0.4)` → Serf
/// - if `n < 5` → Commoner (no Noble band)
/// - if `account_total_score < needed(0.8)` → Commoner else Noble
/// - never King / Emperor at birth
///
/// Noob→noble spawn weight is **not** applied here; use
/// [`apply_noob_noble_spawn_weight`] / [`calculate_prestige_class_at_birth_with_noob`].
///
/// `living`: `(lineage_prestige, total_prestige)` per living non-deleted player.
// Haxe: GlobalPlayerInstance.calculatePrestigeClass
pub fn calculate_prestige_class_at_birth(
    account_total_score: f32,
    living: &[(f32, f32)],
) -> PrestigeClass {
    let n = living.len();
    if n < 2 {
        return PrestigeClass::Commoner;
    }
    let mut sorted: Vec<(f32, f32)> = living.to_vec();
    // Ascending total prestige; ties keep relative order (stable).
    sorted.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
    });
    let lineage_asc: Vec<f32> = sorted.iter().map(|(lp, _)| *lp).collect();

    let needed_commoner = calculate_needed_prestige(&lineage_asc, 0.4);
    if account_total_score < needed_commoner {
        return PrestigeClass::Serf;
    }
    if n < 5 {
        return PrestigeClass::Commoner;
    }
    let needed_noble = calculate_needed_prestige(&lineage_asc, 0.8);
    if account_total_score < needed_noble {
        PrestigeClass::Commoner
    } else {
        PrestigeClass::Noble
    }
}

/// True while account has fewer than [`NOOB_NOBLE_MAX_LIVES`] completed lives.
///
/// `account_lives_before_birth` is `AccountRecord.lives` **before** `on_spawn`
/// increments it (so 0 = first life, 4 = fifth life).
// Haxe: GlobalPlayerInstance TODO L1276 / design L6850
#[inline]
pub fn is_noob_for_spawn(account_lives_before_birth: u32) -> bool {
    account_lives_before_birth < NOOB_NOBLE_MAX_LIVES
}

/// NOOB-NOBLE-SPAWN: promote noobs to Noble with [`NOOB_NOBLE_BIRTH_CHANCE`].
///
/// Implements Haxe TODO at `GlobalPlayerInstance` L1276 ("spawn noobs more likely
/// to and as noble") using the design note at L6850:
/// > new players have a 50% change of noble birth in their first 5 lifes
///
/// Rules:
/// - non-noob → `base_class` unchanged
/// - already [`PrestigeClass::is_noble_or_more`] → unchanged (never demote)
/// - noob + `roll < 0.5` → [`PrestigeClass::Noble`]
/// - noob + miss → `base_class` (Serf/Commoner from score table)
///
/// Mother preference for same-class nobles is handled separately by
/// [`calculate_class_boni`] inside birth fitness (Noble child → Noble mother +2).
///
/// `roll` is a uniform sample in `[0, 1)` (caller supplies RNG for testability).
// Haxe: GlobalPlayerInstance TODO L1276 spawn noobs as noble; design L6850
pub fn apply_noob_noble_spawn_weight(
    base_class: PrestigeClass,
    account_lives_before_birth: u32,
    roll: f32,
) -> PrestigeClass {
    if !is_noob_for_spawn(account_lives_before_birth) {
        return base_class;
    }
    if base_class.is_noble_or_more() {
        return base_class;
    }
    if roll.is_finite() && roll < NOOB_NOBLE_BIRTH_CHANCE {
        PrestigeClass::Noble
    } else {
        base_class
    }
}

/// Score-based birth class + noob noble weight in one step.
///
/// `roll` is only consulted when the account is still a noob and base class is
/// not already noble-or-more.
// Haxe: calculatePrestigeClass + TODO L1276 noob noble
pub fn calculate_prestige_class_at_birth_with_noob(
    account_total_score: f32,
    living: &[(f32, f32)],
    account_lives_before_birth: u32,
    roll: f32,
) -> PrestigeClass {
    let base = calculate_prestige_class_at_birth(account_total_score, living);
    apply_noob_noble_spawn_weight(base, account_lives_before_birth, roll)
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
    fn prestige_class_names_table_matches_haxe() {
        // Haxe: ['Not Set', 'Serf', 'Commoner', 'Noble', 'Noble', 'Noble', 'King', 'Emperor']
        assert_eq!(PRESTIGE_CLASS_NAMES.len(), 8);
        assert_eq!(prestige_class_name_at_index(0), "Not Set");
        assert_eq!(prestige_class_name_at_index(1), "Serf");
        assert_eq!(prestige_class_name_at_index(2), "Commoner");
        assert_eq!(prestige_class_name_at_index(3), "Noble");
        assert_eq!(prestige_class_name_at_index(4), "Noble");
        assert_eq!(prestige_class_name_at_index(5), "Noble");
        assert_eq!(prestige_class_name_at_index(6), "King");
        assert_eq!(prestige_class_name_at_index(7), "Emperor");
        assert_eq!(prestige_class_name_at_index(-1), "Not Set");
        assert_eq!(prestige_class_name_at_index(99), "Not Set");
        // Enum class_name uses the table.
        assert_eq!(PrestigeClass::Noble.class_name(), "Noble");
        assert_eq!(PrestigeClass::King.class_name(), "King");
        assert_eq!(PrestigeClass::NotSet.class_name(), "Not Set");
    }

    #[test]
    fn calculate_class_boni_table() {
        // Same class → +2 (including King/King, NotSet/NotSet).
        assert_eq!(
            calculate_class_boni(PrestigeClass::Commoner, PrestigeClass::Commoner),
            CLASS_BONI_SAME
        );
        assert_eq!(
            calculate_class_boni(PrestigeClass::Serf, PrestigeClass::Serf),
            CLASS_BONI_SAME
        );
        assert_eq!(
            calculate_class_boni(PrestigeClass::Noble, PrestigeClass::Noble),
            CLASS_BONI_SAME
        );
        assert_eq!(
            calculate_class_boni(PrestigeClass::King, PrestigeClass::King),
            CLASS_BONI_SAME
        );
        assert_eq!(
            calculate_class_boni(PrestigeClass::Emperor, PrestigeClass::Emperor),
            CLASS_BONI_SAME
        );

        // Noble ↔ Serf → -3.
        assert_eq!(
            calculate_class_boni(PrestigeClass::Noble, PrestigeClass::Serf),
            CLASS_BONI_NOBLE_SERF
        );
        assert_eq!(
            calculate_class_boni(PrestigeClass::Serf, PrestigeClass::Noble),
            CLASS_BONI_NOBLE_SERF
        );

        // Other cross pairs → 0.
        assert_eq!(
            calculate_class_boni(PrestigeClass::Commoner, PrestigeClass::Serf),
            0.0
        );
        assert_eq!(
            calculate_class_boni(PrestigeClass::Commoner, PrestigeClass::Noble),
            0.0
        );
        assert_eq!(
            calculate_class_boni(PrestigeClass::King, PrestigeClass::Serf),
            0.0
        );
        assert_eq!(
            calculate_class_boni(PrestigeClass::Noble, PrestigeClass::King),
            0.0
        );
        assert_eq!(
            calculate_class_boni(PrestigeClass::NotSet, PrestigeClass::Commoner),
            0.0
        );
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

    #[test]
    fn calculate_needed_prestige_haxe_percent() {
        // lineage prestiges already sorted by total prestige asc.
        // n=10, percent 0.4 → first count >= 4 → index 3
        let lp: Vec<f32> = (1..=10).map(|i| i as f32 * 10.0).collect();
        assert!((calculate_needed_prestige(&lp, 0.4) - 40.0).abs() < 1e-5);
        // percent 0.8 → count >= 8 → index 7 → 80
        assert!((calculate_needed_prestige(&lp, 0.8) - 80.0).abs() < 1e-5);
        // n=2, 0.4 → count>=0.8 → first element
        assert!((calculate_needed_prestige(&[5.0, 50.0], 0.4) - 5.0).abs() < 1e-5);
        assert_eq!(calculate_needed_prestige(&[], 0.4), 999_999.0);
    }

    #[test]
    fn calculate_prestige_class_at_birth_haxe_parity() {
        // n < 2 → Commoner always
        assert_eq!(
            calculate_prestige_class_at_birth(0.0, &[]),
            PrestigeClass::Commoner
        );
        assert_eq!(
            calculate_prestige_class_at_birth(0.0, &[(100.0, 100.0)]),
            PrestigeClass::Commoner
        );

        // n=2: sort by total; needed 0.4 = first lineage prestige.
        // living totals 10 and 50 → lineage same; score < 10 → Serf else Commoner (n<5).
        let two = [(10.0, 10.0), (50.0, 50.0)];
        assert_eq!(
            calculate_prestige_class_at_birth(5.0, &two),
            PrestigeClass::Serf
        );
        assert_eq!(
            calculate_prestige_class_at_birth(10.0, &two),
            PrestigeClass::Commoner
        );
        assert_eq!(
            calculate_prestige_class_at_birth(999.0, &two),
            PrestigeClass::Commoner
        );

        // n=5: Noble possible when score >= needed(0.8).
        // lineage prestiges 10,20,30,40,50 sorted by total = same order.
        // 0.4 → count>=2 → 20; 0.8 → count>=4 → 40
        let five: Vec<(f32, f32)> = [10.0, 20.0, 30.0, 40.0, 50.0]
            .into_iter()
            .map(|v| (v, v))
            .collect();
        assert_eq!(
            calculate_prestige_class_at_birth(19.0, &five),
            PrestigeClass::Serf
        );
        assert_eq!(
            calculate_prestige_class_at_birth(20.0, &five),
            PrestigeClass::Commoner
        );
        assert_eq!(
            calculate_prestige_class_at_birth(39.0, &five),
            PrestigeClass::Commoner
        );
        assert_eq!(
            calculate_prestige_class_at_birth(40.0, &five),
            PrestigeClass::Noble
        );
        // Birth never returns King/Emperor.
        assert_ne!(
            calculate_prestige_class_at_birth(1e9, &five),
            PrestigeClass::King
        );
    }

    /// Document intentional delta: living percentile bands ≠ birth 0.4/0.8 cutoffs.
    #[test]
    fn birth_class_vs_living_percentile_documented_delta() {
        // 10 living scores: percentile assigns King/Emperor; birth never does.
        let living: Vec<(f32, f32)> = (1..=10)
            .map(|i| {
                let v = i as f32 * 10.0;
                (v, v)
            })
            .collect();
        let birth = calculate_prestige_class_at_birth(100.0, &living);
        assert!(
            matches!(
                birth,
                PrestigeClass::Serf | PrestigeClass::Commoner | PrestigeClass::Noble
            ),
            "birth class is only Serf/Commoner/Noble, got {birth:?}"
        );
        // Living band for top rank uses Emperor; birth uses Noble max.
        assert_eq!(
            prestige_class_from_percentile(9, 10),
            PrestigeClass::Emperor
        );
        assert_eq!(
            calculate_prestige_class_at_birth(1e9, &living),
            PrestigeClass::Noble
        );
    }

    // --- NOOB-NOBLE-SPAWN / spawn_weights ---

    #[test]
    fn is_noob_for_spawn_first_five_lives() {
        assert!(is_noob_for_spawn(0)); // first life
        assert!(is_noob_for_spawn(1));
        assert!(is_noob_for_spawn(4)); // fifth life
        assert!(!is_noob_for_spawn(5)); // sixth life
        assert!(!is_noob_for_spawn(100));
        assert_eq!(NOOB_NOBLE_MAX_LIVES, 5);
        assert!((NOOB_NOBLE_BIRTH_CHANCE - 0.5).abs() < 1e-6);
    }

    #[test]
    fn noob_roll_promotes_serf_and_commoner_to_noble() {
        // Hit: roll < 0.5
        assert_eq!(
            apply_noob_noble_spawn_weight(PrestigeClass::Serf, 0, 0.0),
            PrestigeClass::Noble
        );
        assert_eq!(
            apply_noob_noble_spawn_weight(PrestigeClass::Serf, 0, 0.499),
            PrestigeClass::Noble
        );
        assert_eq!(
            apply_noob_noble_spawn_weight(PrestigeClass::Commoner, 3, 0.25),
            PrestigeClass::Noble
        );
        // Miss: roll >= 0.5 keeps base
        assert_eq!(
            apply_noob_noble_spawn_weight(PrestigeClass::Serf, 0, 0.5),
            PrestigeClass::Serf
        );
        assert_eq!(
            apply_noob_noble_spawn_weight(PrestigeClass::Commoner, 2, 0.99),
            PrestigeClass::Commoner
        );
    }

    #[test]
    fn noob_never_demotes_noble_or_higher() {
        assert_eq!(
            apply_noob_noble_spawn_weight(PrestigeClass::Noble, 0, 0.99),
            PrestigeClass::Noble
        );
        assert_eq!(
            apply_noob_noble_spawn_weight(PrestigeClass::King, 0, 0.0),
            PrestigeClass::King
        );
        assert_eq!(
            apply_noob_noble_spawn_weight(PrestigeClass::Emperor, 4, 0.0),
            PrestigeClass::Emperor
        );
    }

    #[test]
    fn non_noob_ignores_roll() {
        assert_eq!(
            apply_noob_noble_spawn_weight(PrestigeClass::Serf, 5, 0.0),
            PrestigeClass::Serf
        );
        assert_eq!(
            apply_noob_noble_spawn_weight(PrestigeClass::Commoner, 10, 0.0),
            PrestigeClass::Commoner
        );
    }

    #[test]
    fn noob_nan_roll_does_not_promote() {
        assert_eq!(
            apply_noob_noble_spawn_weight(PrestigeClass::Serf, 0, f32::NAN),
            PrestigeClass::Serf
        );
    }

    #[test]
    fn calculate_prestige_class_at_birth_with_noob_combined() {
        // n=2 → score path Commoner for high score; noob hit → Noble.
        let two = [(10.0, 10.0), (50.0, 50.0)];
        assert_eq!(
            calculate_prestige_class_at_birth_with_noob(999.0, &two, 0, 0.1),
            PrestigeClass::Noble
        );
        // noob miss → Commoner (n<5, score high)
        assert_eq!(
            calculate_prestige_class_at_birth_with_noob(999.0, &two, 0, 0.9),
            PrestigeClass::Commoner
        );
        // Serf score + noob hit → Noble
        assert_eq!(
            calculate_prestige_class_at_birth_with_noob(5.0, &two, 1, 0.0),
            PrestigeClass::Noble
        );
        // Veteran Serf stays Serf even on low roll
        assert_eq!(
            calculate_prestige_class_at_birth_with_noob(5.0, &two, 5, 0.0),
            PrestigeClass::Serf
        );
    }

    /// Noble child + same-class mother boni (documents "to noble mothers" path).
    #[test]
    fn noob_noble_child_class_boni_prefers_noble_mother() {
        // After noob promote → Noble child: Noble mother +2, Serf mother −3.
        assert_eq!(
            calculate_class_boni(PrestigeClass::Noble, PrestigeClass::Noble),
            CLASS_BONI_SAME
        );
        assert_eq!(
            calculate_class_boni(PrestigeClass::Serf, PrestigeClass::Noble),
            CLASS_BONI_NOBLE_SERF
        );
    }
}
