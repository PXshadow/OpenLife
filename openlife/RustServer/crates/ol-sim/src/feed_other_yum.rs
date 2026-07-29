//! FEED-OTHER-YUM / feed_full_eat pure helpers.
//!
//! Haxe `GlobalPlayerInstance.doEating` when `playerFrom != playerTo`:
//! feeder yum prestige share, age/ill gates, drugs fever resist, post-eat emotes,
//! and responsible_id for FX/PU.
//!
//! Live path: `feed_other_full_eat` in `lib.rs` (include / build wire).

/// Haxe feed-other feeder yum prestige share (`gainedPrestige * 0.2`).
// Haxe: GlobalPlayerInstance.doEating L3151–3152
// FEED-OTHER-YUM / feed_full_eat
pub const FEED_OTHER_FEEDER_PRESTIGE_SHARE: f32 = 0.2;

/// Haxe `ServerSettings.MinAgeToEat` (years) — feeder/self must be ≥ this to eat/feed.
// Haxe: ServerSettings.MinAgeToEat / doEating L3045–3047
pub const FEED_MIN_AGE_TO_EAT: f32 = 3.0;

/// Haxe `ServerSettings.AllowEatingOrFeedingIfIll` — default false blocks feeder with yellow fever.
// Haxe: ServerSettings.AllowEatingOrFeedingIfIll / doEating L3050–3055
pub const ALLOW_EATING_OR_FEEDING_IF_ILL: bool = false;

/// Haxe `ServerSettings.ResistanceAgainstFeverForEatingMushrooms` (isDrugs post-eat).
// Haxe: ServerSettings.ResistanceAgainstFeverForEatingMushrooms / doEating L3161–3163
pub const RESISTANCE_AGAINST_FEVER_FOR_EATING_MUSHROOMS: f32 = 0.2;

/// Prestige delta for feeder when feed-other is yum (0 if meh/superMeh or non-yum).
///
/// Haxe: `playerFrom.addHealthAndPrestige(gainedPrestige * 0.2)` only on yum path.
// Haxe: GlobalPlayerInstance.doEating L3151–3152
// FEED-OTHER-YUM
pub fn feed_other_feeder_prestige_delta(eater_health_delta: f32, is_yum: bool) -> f32 {
    if is_yum && eater_health_delta.is_finite() && eater_health_delta > 0.0 {
        eater_health_delta * FEED_OTHER_FEEDER_PRESTIGE_SHARE
    } else {
        0.0
    }
}

/// Feeder age + yellow-fever gates before doEating fill.
///
/// Haxe: `playerFrom.age < MinAgeToEat` → false; `!AllowEatingOrFeedingIfIll && hasYellowFever` → false.
// Haxe: GlobalPlayerInstance.doEating L3045–3056
// FEED-OTHER-YUM
pub fn feeder_may_eat_or_feed(
    feeder_age: f32,
    feeder_has_yellow_fever: bool,
    min_age_to_eat: f32,
    allow_if_ill: bool,
) -> Result<(), &'static str> {
    if !feeder_age.is_finite() || feeder_age < min_age_to_eat {
        return Err("too young");
    }
    if !allow_if_ill && feeder_has_yellow_fever {
        return Err("too ill");
    }
    Ok(())
}

/// Same as [`feeder_may_eat_or_feed`] with Haxe default MinAge / AllowIfIll knobs.
// Haxe: GlobalPlayerInstance.doEating L3045–3056
pub fn feeder_may_eat_or_feed_default(
    feeder_age: f32,
    feeder_has_yellow_fever: bool,
) -> Result<(), &'static str> {
    feeder_may_eat_or_feed(
        feeder_age,
        feeder_has_yellow_fever,
        FEED_MIN_AGE_TO_EAT,
        ALLOW_EATING_OR_FEEDING_IF_ILL,
    )
}

/// Pure isDrugs fever resistance: `(new_yellowfever_count, new_fever_time_to_change)`.
///
/// Haxe: `yellowfeverCount += resistance`; `fever.timeToChange *= 1 - resistance` when fever set.
// Haxe: GlobalPlayerInstance.doEating L3161–3163
// FEED-OTHER-YUM
pub fn apply_drugs_fever_resistance(
    yellowfever_count: f32,
    fever_time_to_change: Option<f32>,
    resistance: f32,
) -> (f32, Option<f32>) {
    let r = if resistance.is_finite() && resistance > 0.0 {
        resistance
    } else {
        0.0
    };
    let new_count = if yellowfever_count.is_finite() {
        yellowfever_count + r
    } else {
        r
    };
    let new_ttc = fever_time_to_change.map(|t| {
        if t.is_finite() {
            t * (1.0 - r)
        } else {
            t
        }
    });
    (new_count, new_ttc)
}

/// Default-resistance variant (Haxe `ResistanceAgainstFeverForEatingMushrooms` = 0.2).
// Haxe: GlobalPlayerInstance.doEating L3161–3163
pub fn apply_drugs_fever_resistance_default(
    yellowfever_count: f32,
    fever_time_to_change: Option<f32>,
) -> (f32, Option<f32>) {
    apply_drugs_fever_resistance(
        yellowfever_count,
        fever_time_to_change,
        RESISTANCE_AGAINST_FEVER_FOR_EATING_MUSHROOMS,
    )
}

/// Haxe `responsible_id`: self-eat → `-1`, feed-other → feeder `p_id`.
// Haxe: GlobalPlayerInstance.doEating L3175
// FEED-OTHER-YUM
pub fn feed_other_responsible_id(feeder_p_id: i32, eater_p_id: i32) -> i32 {
    if feeder_p_id == eater_p_id || feeder_p_id == 0 {
        -1
    } else {
        feeder_p_id
    }
}

/// Post-eat emote for eater after doEating (Haxe L3239–3245).
// Haxe: GlobalPlayerInstance.doEating L3239–3245
// FEED-OTHER-YUM
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedPostEmote {
    /// Craving food eaten (`Emote.miamFood`).
    MiamFood,
    /// Yum non-craving (`Emote.happy`).
    Happy,
    /// SuperMeh (`Emote.ill`).
    Ill,
    /// Meh non-super (`Emote.sad`).
    Sad,
}

impl FeedPostEmote {
    /// Catalog name for [`crate::emotes::emote_by_name`].
    pub fn emote_name(self) -> &'static str {
        match self {
            Self::MiamFood => "MIAMFOOD",
            Self::Happy => "HAPPY",
            Self::Ill => "ILL",
            Self::Sad => "SAD",
        }
    }
}

/// Select eater post-eat emote from craving / yum / superMeh flags.
// Haxe: GlobalPlayerInstance.doEating L3239–3245
// FEED-OTHER-YUM
pub fn feed_other_eater_post_emote(
    is_craving: bool,
    is_yum: bool,
    is_super_meh: bool,
) -> FeedPostEmote {
    if is_craving {
        FeedPostEmote::MiamFood
    } else if is_yum {
        FeedPostEmote::Happy
    } else if is_super_meh {
        FeedPostEmote::Ill
    } else {
        FeedPostEmote::Sad
    }
}

/// Feeder gets happy emote only when eater ate a craving object (Haxe L3240–3241).
// Haxe: GlobalPlayerInstance.doEating L3240–3241
// FEED-OTHER-YUM
pub fn feed_other_feeder_happy_on_craving(is_craving: bool) -> bool {
    is_craving
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FEED-OTHER-YUM: feeder gets 20% of eater yum prestige; meh/superMeh → 0.
    // Haxe: GlobalPlayerInstance.doEating L3151–3156
    #[test]
    fn feed_other_feeder_prestige_delta_yum_share() {
        assert!((feed_other_feeder_prestige_delta(1.0, true) - 0.2).abs() < 1e-5);
        assert!((feed_other_feeder_prestige_delta(5.0, true) - 1.0).abs() < 1e-5);
        assert_eq!(feed_other_feeder_prestige_delta(1.0, false), 0.0);
        assert_eq!(feed_other_feeder_prestige_delta(-2.0, true), 0.0);
        assert_eq!(feed_other_feeder_prestige_delta(0.0, true), 0.0);
    }

    /// Exact feeder share = 0.2 × eater health_delta on yum.
    // Haxe: GlobalPlayerInstance.doEating L3150–3152
    #[test]
    fn feed_other_feeder_prestige_delta_exact_fraction() {
        let eater_delta = 3.0; // foodEaten hasEaten delta
        let d = feed_other_feeder_prestige_delta(eater_delta, true);
        assert!((d - eater_delta * FEED_OTHER_FEEDER_PRESTIGE_SHARE).abs() < 1e-6);
        // meh health_lost → feeder 0
        assert_eq!(feed_other_feeder_prestige_delta(-1.0, false), 0.0);
        assert_eq!(feed_other_feeder_prestige_delta(-2.0, false), 0.0);
    }

    /// Feeder MinAgeToEat + AllowEatingOrFeedingIfIll gates.
    // Haxe: GlobalPlayerInstance.doEating L3045–3056
    #[test]
    fn feeder_may_eat_or_feed_age_and_ill_gates() {
        assert!(feeder_may_eat_or_feed_default(3.0, false).is_ok());
        assert!(feeder_may_eat_or_feed_default(20.0, false).is_ok());
        assert_eq!(
            feeder_may_eat_or_feed_default(2.9, false),
            Err("too young")
        );
        assert_eq!(
            feeder_may_eat_or_feed_default(20.0, true),
            Err("too ill")
        );
        // allow_if_ill=true permits fevered feeder
        assert!(feeder_may_eat_or_feed(20.0, true, 3.0, true).is_ok());
        // age gate still wins when allow_if_ill
        assert_eq!(
            feeder_may_eat_or_feed(1.0, true, 3.0, true),
            Err("too young")
        );
    }

    /// isDrugs: yellowfeverCount += 0.2; fever TTC *= 0.8.
    // Haxe: GlobalPlayerInstance.doEating L3161–3163
    #[test]
    fn apply_drugs_fever_resistance_default_math() {
        let (c, ttc) = apply_drugs_fever_resistance_default(1.0, Some(10.0));
        assert!((c - 1.2).abs() < 1e-5);
        assert!((ttc.unwrap() - 8.0).abs() < 1e-5);
        let (c2, ttc2) = apply_drugs_fever_resistance_default(0.0, None);
        assert!((c2 - 0.2).abs() < 1e-5);
        assert!(ttc2.is_none());
    }

    /// responsible_id self vs other.
    // Haxe: GlobalPlayerInstance.doEating L3175
    #[test]
    fn feed_other_responsible_id_self_vs_other() {
        assert_eq!(feed_other_responsible_id(5, 5), -1);
        assert_eq!(feed_other_responsible_id(7, 5), 7);
        assert_eq!(feed_other_responsible_id(0, 5), -1);
    }

    /// Post-eat emote priority: craving > yum > superMeh > meh.
    // Haxe: GlobalPlayerInstance.doEating L3239–3245
    #[test]
    fn feed_other_eater_post_emote_priority() {
        assert_eq!(
            feed_other_eater_post_emote(true, true, false),
            FeedPostEmote::MiamFood
        );
        assert_eq!(
            feed_other_eater_post_emote(false, true, false),
            FeedPostEmote::Happy
        );
        assert_eq!(
            feed_other_eater_post_emote(false, false, true),
            FeedPostEmote::Ill
        );
        assert_eq!(
            feed_other_eater_post_emote(false, false, false),
            FeedPostEmote::Sad
        );
        assert!(feed_other_feeder_happy_on_craving(true));
        assert!(!feed_other_feeder_happy_on_craving(false));
        assert_eq!(FeedPostEmote::MiamFood.emote_name(), "MIAMFOOD");
    }
}
