//! Combat reputation score (float), **separate from** prestige / PrestigeClass.
//!
//! Haxe maps this via `GlobalPlayerInstance.lostCombatPrestige` (positive = bad)
//! and stores lineage `reputation = lostCombatPrestige * (-1)` (higher = better).
//! Labels match `PlayerSoul.getCombatPrestigeLabel`.
//!
//! **REPUTATION-HIT:** pure helpers for the post-`DoDamage` float update in
//! `GlobalPlayerInstance.kill` (`attackWasLegit` / illegal guilt / child-elder-ally…).

use std::collections::HashMap;

/// Seven reputation levels from combat prestige (Haxe label table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReputationLabel {
    SuperGood,
    Good,
    FairlyGood,
    Neutral,
    FairlyBad,
    Bad,
    SuperBad,
}

impl ReputationLabel {
    /// Wire / chat token (`super_good`, `neutral`, …).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SuperGood => "super_good",
            Self::Good => "good",
            Self::FairlyGood => "fairly_good",
            Self::Neutral => "neutral",
            Self::FairlyBad => "fairly_bad",
            Self::Bad => "bad",
            Self::SuperBad => "super_bad",
        }
    }

    /// Haxe `getCombatPrestigeLabel` human phrase.
    pub fn display(self) -> &'static str {
        match self {
            Self::SuperGood => "super good",
            Self::Good => "good",
            Self::FairlyGood => "fairly good",
            Self::Neutral => "neutral",
            Self::FairlyBad => "fairly bad",
            Self::Bad => "bad",
            Self::SuperBad => "super bad",
        }
    }
}

/// Map Haxe `lostCombatPrestige` (positive = bad) to a label.
///
/// Thresholds (from `PlayerSoul.getCombatPrestigeLabel`):
/// - `> 50` → super bad
/// - `≥ 20` → bad
/// - `> 4` → fairly bad
/// - `> -4` → neutral
/// - `≥ -20` → fairly good
/// - `≥ -50` → good
/// - else → super good
pub fn label_from_lost_combat(lost_combat: f32) -> ReputationLabel {
    if !lost_combat.is_finite() {
        return ReputationLabel::Neutral;
    }
    if lost_combat > 50.0 {
        ReputationLabel::SuperBad
    } else if lost_combat >= 20.0 {
        ReputationLabel::Bad
    } else if lost_combat > 4.0 {
        ReputationLabel::FairlyBad
    } else if lost_combat > -4.0 {
        ReputationLabel::Neutral
    } else if lost_combat >= -20.0 {
        ReputationLabel::FairlyGood
    } else if lost_combat >= -50.0 {
        ReputationLabel::Good
    } else {
        ReputationLabel::SuperGood
    }
}

/// Convert lost-combat float to stored lineage reputation (`* -1`).
///
/// Higher reputation is better (inverse of lost combat prestige).
pub fn reputation_from_lost_combat(lost_combat: f32) -> f32 {
    if !lost_combat.is_finite() {
        return 0.0;
    }
    -lost_combat
}

/// Inverse of [`reputation_from_lost_combat`].
pub fn lost_combat_from_reputation(reputation: f32) -> f32 {
    if !reputation.is_finite() {
        return 0.0;
    }
    -reputation
}

/// Label from the **stored reputation** float (higher = better).
pub fn label_from_reputation(reputation: f32) -> ReputationLabel {
    label_from_lost_combat(lost_combat_from_reputation(reputation))
}

/// True when lost combat prestige is high enough that AI treats the player as
/// dangerous (Haxe uses thresholds around 1 / 4 / 5).
pub fn is_dangerous_lost_combat(lost_combat: f32) -> bool {
    lost_combat.is_finite() && lost_combat > 1.0
}

// --- REPUTATION-HIT constants (Haxe ServerSettings defaults) ---

/// Haxe `ServerSettings.MinAgeToEat` — child prestige-cost threshold (years).
// Haxe: ServerSettings.MinAgeToEat
pub const MIN_AGE_TO_EAT_YEARS: f32 = 3.0;

/// Haxe elderly threshold `trueAge > 50`.
// Haxe: targetPlayer.trueAge > 50
pub const ELDERLY_AGE_YEARS: f32 = 50.0;

/// Haxe `PrestigeCostPerDamageForChild`.
pub const PRESTIGE_COST_PER_DAMAGE_CHILD: f32 = 5.0;
/// Haxe `PrestigeCostPerDamageForElderly`.
pub const PRESTIGE_COST_PER_DAMAGE_ELDERLY: f32 = 1.0;
/// Haxe `PrestigeCostPerDamageForAlly`.
pub const PRESTIGE_COST_PER_DAMAGE_ALLY: f32 = 1.0;
/// Haxe `PrestigeCostPerDamageForCloseRelatives`.
pub const PRESTIGE_COST_PER_DAMAGE_CLOSE_RELATIVE: f32 = 0.5;
/// Haxe `PrestigeCostPerDamageForWomenWithoutWeapon`.
pub const PRESTIGE_COST_PER_DAMAGE_WOMAN_UNARMED: f32 = 0.5;

/// Devil Mask clothing object id (Haxe 3213) — multiplies prestige-cost damage ×5,
/// skips health/prestige speech (`addHealthAndPrestige` + GM).
// Haxe: getClothingById(3213) Devil Mask
pub const DEVIL_MASK_CLOTHING_ID: i32 = 3213;

/// Haxe `ServerSettings.CombatReputationRestorePerYear` default.
/// Calm restore rate: subtract `(rate * dt) / 60` from lostCombatPrestige per tick.
// Haxe: ServerSettings.CombatReputationRestorePerYear
pub const COMBAT_REPUTATION_RESTORE_PER_YEAR: f32 = 2.0;

/// Live / test overrides for category prestige-cost multipliers (Haxe ServerSettings).
// Haxe: PrestigeCostPerDamageFor* ServerSettings
// PRESTIGE-ALLY-COST
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrestigeCostFactors {
    pub child: f32,
    pub elderly: f32,
    pub ally: f32,
    pub close_relative: f32,
    pub woman_unarmed: f32,
    /// Haxe `MinAgeToEat` — child prestige-cost age threshold (years).
    // C-SS-MORE-BATCH3
    pub min_age_to_eat: f32,
}

impl Default for PrestigeCostFactors {
    fn default() -> Self {
        Self {
            child: PRESTIGE_COST_PER_DAMAGE_CHILD,
            elderly: PRESTIGE_COST_PER_DAMAGE_ELDERLY,
            ally: PRESTIGE_COST_PER_DAMAGE_ALLY,
            close_relative: PRESTIGE_COST_PER_DAMAGE_CLOSE_RELATIVE,
            woman_unarmed: PRESTIGE_COST_PER_DAMAGE_WOMAN_UNARMED,
            min_age_to_eat: MIN_AGE_TO_EAT_YEARS,
        }
    }
}

impl PrestigeCostFactors {
    /// Sanitize non-finite entries back to Haxe defaults.
    pub fn sanitized(self) -> Self {
        let pick = |v: f32, d: f32| if v.is_finite() && v >= 0.0 { v } else { d };
        Self {
            child: pick(self.child, PRESTIGE_COST_PER_DAMAGE_CHILD),
            elderly: pick(self.elderly, PRESTIGE_COST_PER_DAMAGE_ELDERLY),
            ally: pick(self.ally, PRESTIGE_COST_PER_DAMAGE_ALLY),
            close_relative: pick(self.close_relative, PRESTIGE_COST_PER_DAMAGE_CLOSE_RELATIVE),
            woman_unarmed: pick(self.woman_unarmed, PRESTIGE_COST_PER_DAMAGE_WOMAN_UNARMED),
            // C-SS-MORE-BATCH3
            min_age_to_eat: pick(self.min_age_to_eat, MIN_AGE_TO_EAT_YEARS),
        }
    }
}

/// Category branch that produced [`HitReputationDelta::prestige_cost`] (Haxe kill L4527–4562).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrestigeCostCategory {
    #[default]
    None,
    Child,
    Elderly,
    Ally,
    CloseRelative,
    WomanUnarmed,
}

impl PrestigeCostCategory {
    /// Haxe GM phrase fragment after "attacking ".
    pub fn message_phrase(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Child => Some("a child"),
            Self::Elderly => Some("elder"),
            Self::Ally => Some("ally"),
            Self::CloseRelative => Some("close relative"),
            // Haxe typo "women" kept for wire parity
            Self::WomanUnarmed => Some("a women without weapon"),
        }
    }
}

/// Haxe `connection.sendGlobalMessage('Lost $prestigeCost prestige for attacking …')`.
// Haxe: GlobalPlayerInstance.kill category GM
pub fn format_prestige_cost_global_message(
    prestige_cost: f32,
    category: PrestigeCostCategory,
    target_name: &str,
) -> Option<String> {
    let phrase = category.message_phrase()?;
    if !prestige_cost.is_finite() || prestige_cost <= 0.0 {
        return None;
    }
    // Haxe ceil'd cost displays as whole number when integer.
    let cost_disp = if (prestige_cost - prestige_cost.round()).abs() < 1e-4 {
        format!("{}", prestige_cost.round() as i32)
    } else {
        format!("{prestige_cost}")
    };
    Some(format!(
        "Lost {cost_disp} prestige for attacking {phrase} {target_name}!"
    ))
}

/// Haxe TimeHelper calm restore: amount to **subtract** from `lostCombatPrestige`.
///
/// Conditions: `angryTime >= 0 && lostCombatPrestige > 0 && darkNosaj < 1`.
/// Delta = `(CombatReputationRestorePerYear * dt) / 60`.
// Haxe: TimeHelper L404–405
pub fn combat_reputation_restore_delta(
    angry_time: f32,
    lost_combat: f32,
    dark_nosaj: f32,
    dt_secs: f32,
) -> f32 {
    combat_reputation_restore_delta_ex(
        angry_time,
        lost_combat,
        dark_nosaj,
        dt_secs,
        COMBAT_REPUTATION_RESTORE_PER_YEAR,
    )
}

/// Live-rate variant of [`combat_reputation_restore_delta`].
///
/// `restore_per_year` = Haxe `CombatReputationRestorePerYear` (live via GameplayKnobs).
// Haxe: TimeHelper L404–405 CombatReputationRestorePerYear
// C-SS-TAIL-KNOBS
pub fn combat_reputation_restore_delta_ex(
    angry_time: f32,
    lost_combat: f32,
    dark_nosaj: f32,
    dt_secs: f32,
    restore_per_year: f32,
) -> f32 {
    if !dt_secs.is_finite() || dt_secs <= 0.0 {
        return 0.0;
    }
    if !lost_combat.is_finite() || lost_combat <= 0.0 {
        return 0.0;
    }
    if !angry_time.is_finite() || angry_time < 0.0 {
        return 0.0;
    }
    if !dark_nosaj.is_finite() || dark_nosaj >= 1.0 {
        return 0.0;
    }
    let rate = if restore_per_year.is_finite() && restore_per_year >= 0.0 {
        restore_per_year
    } else {
        COMBAT_REPUTATION_RESTORE_PER_YEAR
    };
    (rate * dt_secs) / 60.0
}

/// Haxe `attackWasLegit = damage < 2 * targetPlayer.lostCombatPrestige`.
// Haxe: GlobalPlayerInstance.kill attackWasLegit
pub fn attack_was_legit(damage: f32, target_lost_combat: f32) -> bool {
    if !damage.is_finite() || !target_lost_combat.is_finite() {
        return false;
    }
    damage < 2.0 * target_lost_combat
}

/// Inputs for post-hit combat reputation (Haxe kill after DoDamage).
#[derive(Debug, Clone, Copy)]
pub struct HitReputationInput {
    /// Applied wound/kill damage from DoDamage.
    pub damage: f32,
    /// Target's current lostCombatPrestige (positive = bad).
    pub target_lost_combat: f32,
    /// Target currently holding a weapon (`isHoldingWeapon`).
    pub target_holding_weapon: bool,
    /// Haxe lineage prestigeClass as int (Serf=1 …); higher = more prestigious.
    pub attacker_prestige_class: i32,
    pub target_prestige_class: i32,
    /// Haxe `trueAge` (years).
    pub target_true_age: f32,
    /// Target is ally of attacker after any mid-hit exile.
    pub target_is_ally: bool,
    pub target_is_close_relative: bool,
    pub target_is_female: bool,
    pub target_is_cursed: bool,
    /// Attacker wears Devil Mask 3213.
    pub attacker_has_red_mask: bool,
}

impl Default for HitReputationInput {
    fn default() -> Self {
        Self {
            damage: 0.0,
            target_lost_combat: 0.0,
            target_holding_weapon: false,
            attacker_prestige_class: 2, // Commoner
            target_prestige_class: 2,
            target_true_age: 20.0,
            target_is_ally: false,
            target_is_close_relative: false,
            target_is_female: false,
            target_is_cursed: false,
            attacker_has_red_mask: false,
        }
    }
}

/// Deltas to apply after a connecting hit (positive lost-delta = worse reputation).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitReputationDelta {
    /// Added to attacker `lostCombatPrestige` (negative = recover).
    pub attacker_lost_delta: f32,
    /// Added to target `lostCombatPrestige` (negative = recover).
    pub target_lost_delta: f32,
    pub attack_was_legit: bool,
    /// Extra prestige cost from child/elder/ally/relative/woman branch (ceil'd).
    /// Applied to score prestige + GM when not wearing Devil Mask (live wire).
    pub prestige_cost: f32,
    /// Which category branch produced `prestige_cost` (for GM text).
    pub prestige_cost_category: PrestigeCostCategory,
    /// True when illegal guilt path ran (target unarmed).
    pub illegal_guilt_applied: bool,
}

/// Compute lostCombatPrestige deltas for one connecting HIT (Haxe kill L4504–4561).
///
/// - **Legit** (`damage < 2 * target.lost`): both recover `damage/2`.
/// - **Illegal + target unarmed**: attacker guilt `damage` (or `×0.5` if higher class)
///   plus optional category `ceil(damage * factor)` (red mask multiplies damage for
///   category cost only after base guilt).
/// - **Illegal + target armed**: no float change (duel path).
// Haxe: GlobalPlayerInstance.kill prestigeCost / lostCombatPrestige after DoDamage
pub fn compute_hit_reputation(input: &HitReputationInput) -> HitReputationDelta {
    compute_hit_reputation_with_factors(input, &PrestigeCostFactors::default())
}

/// Same as [`compute_hit_reputation`] with live/test [`PrestigeCostFactors`].
// Haxe: ServerSettings.PrestigeCostPerDamageFor*
// PRESTIGE-ALLY-COST
pub fn compute_hit_reputation_with_factors(
    input: &HitReputationInput,
    factors: &PrestigeCostFactors,
) -> HitReputationDelta {
    let factors = factors.sanitized();
    let damage = if input.damage.is_finite() {
        input.damage.max(0.0)
    } else {
        0.0
    };
    if damage <= 0.0 {
        return HitReputationDelta {
            attacker_lost_delta: 0.0,
            target_lost_delta: 0.0,
            attack_was_legit: false,
            prestige_cost: 0.0,
            prestige_cost_category: PrestigeCostCategory::None,
            illegal_guilt_applied: false,
        };
    }

    let legit = attack_was_legit(damage, input.target_lost_combat);
    if legit {
        let recover = damage / 2.0;
        return HitReputationDelta {
            attacker_lost_delta: -recover,
            target_lost_delta: -recover,
            attack_was_legit: true,
            prestige_cost: 0.0,
            prestige_cost_category: PrestigeCostCategory::None,
            illegal_guilt_applied: false,
        };
    }

    // Not legit — only guilt if target is unarmed.
    if input.target_holding_weapon {
        return HitReputationDelta {
            attacker_lost_delta: 0.0,
            target_lost_delta: 0.0,
            attack_was_legit: false,
            prestige_cost: 0.0,
            prestige_cost_category: PrestigeCostCategory::None,
            illegal_guilt_applied: false,
        };
    }

    // Haxe: isHigherPrestigeClass = targetClass < attackerClass → half guilt
    let is_higher = input.target_prestige_class < input.attacker_prestige_class;
    let mut attacker_lost = if is_higher {
        damage * 0.5
    } else {
        damage
    };

    // Red mask: category costs use 5× damage; base guilt already applied above.
    let mut cost_damage = damage;
    if input.attacker_has_red_mask {
        cost_damage *= 5.0;
    }

    let mut prestige_cost = 0.0_f32;
    let mut prestige_cost_category = PrestigeCostCategory::None;
    // Haxe L4525: TODO count as ally if exile happened not long ago ??? (open both sides)
    // C-SS-MORE-BATCH3: MinAgeToEat via factors.min_age_to_eat (default 3)
    let min_age = if factors.min_age_to_eat.is_finite() && factors.min_age_to_eat >= 0.0 {
        factors.min_age_to_eat
    } else {
        MIN_AGE_TO_EAT_YEARS
    };
    if input.target_true_age < min_age {
        prestige_cost = (cost_damage * factors.child).ceil();
        prestige_cost_category = PrestigeCostCategory::Child;
    } else if input.target_true_age > ELDERLY_AGE_YEARS && !input.target_is_cursed {
        prestige_cost = (cost_damage * factors.elderly).ceil();
        prestige_cost_category = PrestigeCostCategory::Elderly;
    } else if input.target_is_ally && !input.target_is_cursed {
        // PRESTIGE-ALLY-COST: PrestigeCostPerDamageForAlly (live via factors.ally)
        prestige_cost = (cost_damage * factors.ally).ceil();
        prestige_cost_category = PrestigeCostCategory::Ally;
    } else if input.target_is_close_relative && !input.target_is_cursed {
        prestige_cost = (cost_damage * factors.close_relative).ceil();
        prestige_cost_category = PrestigeCostCategory::CloseRelative;
    } else if input.target_is_female && !input.target_is_cursed {
        prestige_cost = (cost_damage * factors.woman_unarmed).ceil();
        prestige_cost_category = PrestigeCostCategory::WomanUnarmed;
    }

    if prestige_cost.is_finite() && prestige_cost > 0.0 {
        attacker_lost += prestige_cost;
    } else {
        prestige_cost = 0.0;
        prestige_cost_category = PrestigeCostCategory::None;
    }

    HitReputationDelta {
        attacker_lost_delta: attacker_lost,
        target_lost_delta: 0.0,
        attack_was_legit: false,
        prestige_cost,
        prestige_cost_category,
        illegal_guilt_applied: true,
    }
}

/// Session-local reputation book (p_id → reputation float, higher = better).
///
/// Independent of [`crate::prestige::PrestigeClass`] and economy trade prestige.
#[derive(Debug, Clone, Default)]
pub struct ReputationBook {
    scores: HashMap<i32, f32>,
}

impl ReputationBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Current reputation (0.0 if unknown).
    pub fn get(&self, p_id: i32) -> f32 {
        self.scores.get(&p_id).copied().unwrap_or(0.0)
    }

    /// Set absolute reputation score.
    pub fn set(&mut self, p_id: i32, reputation: f32) {
        let v = if reputation.is_finite() {
            reputation
        } else {
            0.0
        };
        self.scores.insert(p_id, v);
    }

    /// Add delta to reputation (positive delta improves reputation).
    pub fn add(&mut self, p_id: i32, delta: f32) {
        if !delta.is_finite() {
            return;
        }
        let cur = self.get(p_id);
        self.set(p_id, cur + delta);
    }

    /// Record lost-combat float (Haxe field); stores inverted reputation.
    pub fn set_from_lost_combat(&mut self, p_id: i32, lost_combat: f32) {
        self.set(p_id, reputation_from_lost_combat(lost_combat));
    }

    /// Lost-combat view of stored score (positive = bad).
    pub fn lost_combat(&self, p_id: i32) -> f32 {
        lost_combat_from_reputation(self.get(p_id))
    }

    /// Label for a player (neutral if unknown / 0).
    pub fn label(&self, p_id: i32) -> ReputationLabel {
        label_from_reputation(self.get(p_id))
    }

    /// Apply illegal combat damage: attacker gains bad reputation.
    ///
    /// `damage` is the wound/kill magnitude. Higher prestige class multiplies
    /// guilt by 0.5 in Haxe when attacking down; this pure helper takes a
    /// precomputed `guilt_scale` (default 1.0).
    pub fn apply_illegal_hit(&mut self, attacker: i32, damage: f32, guilt_scale: f32) {
        if !damage.is_finite() || damage <= 0.0 {
            return;
        }
        let scale = if guilt_scale.is_finite() && guilt_scale > 0.0 {
            guilt_scale
        } else {
            1.0
        };
        // lost_combat += damage * scale → reputation -= damage * scale
        self.add(attacker, -(damage * scale));
    }

    /// Legal hit: both sides recover some lost combat prestige (Haxe halves).
    pub fn apply_legal_hit(&mut self, a: i32, b: i32, damage: f32) {
        if !damage.is_finite() || damage <= 0.0 {
            return;
        }
        let recover = damage / 2.0;
        // lost_combat -= damage/2 → reputation += damage/2
        self.add(a, recover);
        self.add(b, recover);
    }

    /// Apply [`compute_hit_reputation`] deltas to the book.
    ///
    /// `lost_delta` positive → reputation decreases (`reputation = -lost`).
    // Haxe: this.lostCombatPrestige += … / target.lostCombatPrestige -= …
    pub fn apply_hit_delta(
        &mut self,
        attacker: i32,
        target: i32,
        delta: &HitReputationDelta,
    ) {
        if delta.attacker_lost_delta.is_finite() && delta.attacker_lost_delta != 0.0 {
            // lost += d → rep -= d
            self.add(attacker, -delta.attacker_lost_delta);
        }
        if delta.target_lost_delta.is_finite() && delta.target_lost_delta != 0.0 {
            self.add(target, -delta.target_lost_delta);
        }
    }

    pub fn remove(&mut self, p_id: i32) {
        self.scores.remove(&p_id);
    }

    pub fn len(&self) -> usize {
        self.scores.len()
    }

    pub fn is_empty(&self) -> bool {
        self.scores.is_empty()
    }
}

/// `REP score=N.N label=neutral lost=N.N` body (no leading p_id).
pub fn format_reputation_query(book: &ReputationBook, p_id: i32) -> String {
    let score = book.get(p_id);
    let label = book.label(p_id);
    let lost = book.lost_combat(p_id);
    format!(
        "REP score={score:.1} label={} lost={lost:.1}",
        label.as_str()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_match_haxe_lost_combat_table() {
        assert_eq!(label_from_lost_combat(51.0), ReputationLabel::SuperBad);
        assert_eq!(label_from_lost_combat(20.0), ReputationLabel::Bad);
        assert_eq!(label_from_lost_combat(19.9), ReputationLabel::FairlyBad);
        assert_eq!(label_from_lost_combat(4.1), ReputationLabel::FairlyBad);
        assert_eq!(label_from_lost_combat(4.0), ReputationLabel::Neutral);
        assert_eq!(label_from_lost_combat(0.0), ReputationLabel::Neutral);
        assert_eq!(label_from_lost_combat(-3.9), ReputationLabel::Neutral);
        assert_eq!(label_from_lost_combat(-4.0), ReputationLabel::FairlyGood);
        assert_eq!(label_from_lost_combat(-20.0), ReputationLabel::FairlyGood);
        assert_eq!(label_from_lost_combat(-20.1), ReputationLabel::Good);
        assert_eq!(label_from_lost_combat(-50.0), ReputationLabel::Good);
        assert_eq!(label_from_lost_combat(-50.1), ReputationLabel::SuperGood);
        assert_eq!(label_from_lost_combat(f32::NAN), ReputationLabel::Neutral);
    }

    #[test]
    fn reputation_is_negative_lost_combat() {
        assert_eq!(reputation_from_lost_combat(10.0), -10.0);
        assert_eq!(reputation_from_lost_combat(-3.0), 3.0);
        assert_eq!(lost_combat_from_reputation(7.5), -7.5);
        assert_eq!(
            label_from_reputation(25.0),
            label_from_lost_combat(-25.0)
        );
    }

    #[test]
    fn separate_from_zero_default() {
        let book = ReputationBook::new();
        assert_eq!(book.get(1), 0.0);
        assert_eq!(book.label(1), ReputationLabel::Neutral);
        assert!(!is_dangerous_lost_combat(book.lost_combat(1)));
    }

    #[test]
    fn illegal_hit_worsens_attacker_only() {
        let mut book = ReputationBook::new();
        book.apply_illegal_hit(1, 10.0, 1.0);
        assert_eq!(book.get(1), -10.0);
        assert_eq!(book.lost_combat(1), 10.0);
        assert_eq!(book.label(1), ReputationLabel::FairlyBad);
        assert_eq!(book.get(2), 0.0);
        // Higher-class guilt scale 0.5
        book.apply_illegal_hit(3, 10.0, 0.5);
        assert_eq!(book.get(3), -5.0);
    }

    #[test]
    fn legal_hit_recovers_both() {
        let mut book = ReputationBook::new();
        book.set_from_lost_combat(1, 8.0); // rep = -8
        book.set_from_lost_combat(2, 4.0); // rep = -4
        book.apply_legal_hit(1, 2, 4.0); // +2 each
        assert_eq!(book.get(1), -6.0);
        assert_eq!(book.get(2), -2.0);
    }

    #[test]
    fn format_query_and_remove() {
        let mut book = ReputationBook::new();
        book.set(5, 30.0);
        let q = format_reputation_query(&book, 5);
        assert!(q.contains("score=30.0"), "{q}");
        assert!(q.contains("label=good") || q.contains("label=fairly_good"), "{q}");
        assert!(q.starts_with("REP "), "{q}");
        book.remove(5);
        assert!(book.is_empty());
    }

    #[test]
    fn dangerous_threshold() {
        assert!(!is_dangerous_lost_combat(1.0));
        assert!(is_dangerous_lost_combat(1.01));
        assert!(is_dangerous_lost_combat(5.0));
    }

    // --- REPUTATION-HIT pure ---

    #[test]
    fn attack_was_legit_uses_double_target_lost() {
        // damage < 2 * target_lost
        assert!(attack_was_legit(9.0, 5.0)); // 9 < 10
        assert!(!attack_was_legit(10.0, 5.0)); // 10 < 10 false
        assert!(!attack_was_legit(1.0, 0.0));
        assert!(!attack_was_legit(f32::NAN, 5.0));
    }

    #[test]
    fn compute_legit_recovers_both_half_damage() {
        let d = compute_hit_reputation(&HitReputationInput {
            damage: 4.0,
            target_lost_combat: 3.0, // 4 < 6 → legit
            ..Default::default()
        });
        assert!(d.attack_was_legit);
        assert!((d.attacker_lost_delta - (-2.0)).abs() < 1e-5);
        assert!((d.target_lost_delta - (-2.0)).abs() < 1e-5);
        assert!(!d.illegal_guilt_applied);
    }

    #[test]
    fn compute_illegal_unarmed_full_guilt() {
        let d = compute_hit_reputation(&HitReputationInput {
            damage: 6.0,
            target_lost_combat: 0.0,
            target_holding_weapon: false,
            attacker_prestige_class: 2,
            target_prestige_class: 2,
            target_true_age: 25.0,
            target_is_female: false,
            ..Default::default()
        });
        assert!(!d.attack_was_legit);
        assert!(d.illegal_guilt_applied);
        // equal class → full damage; no category (adult male non-ally)
        assert!((d.attacker_lost_delta - 6.0).abs() < 1e-5);
        assert_eq!(d.target_lost_delta, 0.0);
    }

    #[test]
    fn compute_illegal_higher_class_half_guilt() {
        // attacker Noble(3) > target Serf(1)
        let d = compute_hit_reputation(&HitReputationInput {
            damage: 10.0,
            target_lost_combat: 0.0,
            attacker_prestige_class: 3,
            target_prestige_class: 1,
            target_true_age: 30.0,
            ..Default::default()
        });
        assert!((d.attacker_lost_delta - 5.0).abs() < 1e-5);
    }

    #[test]
    fn compute_illegal_armed_target_no_change() {
        let d = compute_hit_reputation(&HitReputationInput {
            damage: 8.0,
            target_lost_combat: 0.0,
            target_holding_weapon: true,
            ..Default::default()
        });
        assert!(!d.illegal_guilt_applied);
        assert_eq!(d.attacker_lost_delta, 0.0);
        assert_eq!(d.target_lost_delta, 0.0);
    }

    #[test]
    fn compute_child_prestige_cost_ceil() {
        // damage 1.2 → guilt 1.2 + ceil(1.2 * 5) = 1.2 + 6 = 7.2
        let d = compute_hit_reputation(&HitReputationInput {
            damage: 1.2,
            target_lost_combat: 0.0,
            target_true_age: 2.0,
            ..Default::default()
        });
        assert!((d.prestige_cost - 6.0).abs() < 1e-5);
        assert!((d.attacker_lost_delta - 7.2).abs() < 1e-4);
    }

    #[test]
    fn compute_ally_and_woman_and_elder_branches() {
        let ally = compute_hit_reputation(&HitReputationInput {
            damage: 2.0,
            target_is_ally: true,
            target_true_age: 20.0,
            ..Default::default()
        });
        // guilt 2 + ceil(2 * 1) = 2 + 2 = 4
        assert!((ally.prestige_cost - 2.0).abs() < 1e-5);
        assert!((ally.attacker_lost_delta - 4.0).abs() < 1e-5);

        let woman = compute_hit_reputation(&HitReputationInput {
            damage: 2.0,
            target_is_female: true,
            target_true_age: 20.0,
            ..Default::default()
        });
        // guilt 2 + ceil(2 * 0.5) = 2 + 1 = 3
        assert!((woman.prestige_cost - 1.0).abs() < 1e-5);
        assert!((woman.attacker_lost_delta - 3.0).abs() < 1e-5);

        let elder = compute_hit_reputation(&HitReputationInput {
            damage: 2.0,
            target_true_age: 51.0,
            ..Default::default()
        });
        // guilt 2 + ceil(2 * 1) = 4
        assert!((elder.prestige_cost - 2.0).abs() < 1e-5);
        assert!((elder.attacker_lost_delta - 4.0).abs() < 1e-5);

        // cursed woman: no category cost
        let cursed = compute_hit_reputation(&HitReputationInput {
            damage: 2.0,
            target_is_female: true,
            target_is_cursed: true,
            target_true_age: 20.0,
            ..Default::default()
        });
        assert_eq!(cursed.prestige_cost, 0.0);
        assert!((cursed.attacker_lost_delta - 2.0).abs() < 1e-5);
    }

    #[test]
    fn compute_red_mask_multiplies_category_only() {
        // base guilt still damage; category uses ×5
        let d = compute_hit_reputation(&HitReputationInput {
            damage: 2.0,
            target_true_age: 2.0, // child
            attacker_has_red_mask: true,
            ..Default::default()
        });
        // guilt 2 + ceil(10 * 5) = 2 + 50 = 52
        assert!((d.prestige_cost - 50.0).abs() < 1e-5);
        assert!((d.attacker_lost_delta - 52.0).abs() < 1e-5);
    }

    #[test]
    fn book_apply_hit_delta_inverts_to_reputation() {
        let mut book = ReputationBook::new();
        let d = compute_hit_reputation(&HitReputationInput {
            damage: 4.0,
            target_lost_combat: 0.0,
            target_true_age: 25.0,
            ..Default::default()
        });
        book.apply_hit_delta(1, 2, &d);
        assert!((book.get(1) - (-4.0)).abs() < 1e-5);
        assert_eq!(book.get(2), 0.0);
        assert!((book.lost_combat(1) - 4.0).abs() < 1e-5);

        // legit recovery
        book.set_from_lost_combat(10, 10.0);
        book.set_from_lost_combat(11, 10.0);
        let legit = compute_hit_reputation(&HitReputationInput {
            damage: 4.0,
            target_lost_combat: 10.0,
            ..Default::default()
        });
        book.apply_hit_delta(10, 11, &legit);
        assert!((book.lost_combat(10) - 8.0).abs() < 1e-5);
        assert!((book.lost_combat(11) - 8.0).abs() < 1e-5);
    }

    #[test]
    fn prestige_cost_category_child_and_gm_text() {
        let d = compute_hit_reputation(&HitReputationInput {
            damage: 1.2,
            target_true_age: 2.0,
            ..Default::default()
        });
        assert_eq!(d.prestige_cost_category, PrestigeCostCategory::Child);
        let msg = format_prestige_cost_global_message(d.prestige_cost, d.prestige_cost_category, "Kid")
            .expect("msg");
        assert!(msg.contains("Lost 6 prestige"), "{msg}");
        assert!(msg.contains("a child Kid"), "{msg}");
    }

    /// PRESTIGE-ALLY-COST: ally category + GM text + live factor override.
    // Haxe: PrestigeCostPerDamageForAlly + sendGlobalMessage ally
    #[test]
    fn prestige_cost_category_ally_and_gm_text() {
        let d = compute_hit_reputation(&HitReputationInput {
            damage: 2.0,
            target_is_ally: true,
            target_true_age: 20.0,
            ..Default::default()
        });
        assert_eq!(d.prestige_cost_category, PrestigeCostCategory::Ally);
        assert!((d.prestige_cost - 2.0).abs() < 1e-5);
        let msg = format_prestige_cost_global_message(
            d.prestige_cost,
            d.prestige_cost_category,
            "Buddy",
        )
        .expect("msg");
        assert!(msg.contains("Lost 2 prestige"), "{msg}");
        assert!(msg.contains("ally Buddy"), "{msg}");

        let mut factors = PrestigeCostFactors::default();
        factors.ally = 3.0;
        let live = compute_hit_reputation_with_factors(
            &HitReputationInput {
                damage: 2.0,
                target_is_ally: true,
                target_true_age: 20.0,
                ..Default::default()
            },
            &factors,
        );
        assert!((live.prestige_cost - 6.0).abs() < 1e-5);
        assert_eq!(live.prestige_cost_category, PrestigeCostCategory::Ally);

        let cursed = compute_hit_reputation(&HitReputationInput {
            damage: 2.0,
            target_is_ally: true,
            target_is_cursed: true,
            target_true_age: 20.0,
            ..Default::default()
        });
        assert_eq!(cursed.prestige_cost, 0.0);
        assert_eq!(cursed.prestige_cost_category, PrestigeCostCategory::None);
    }

    /// C-SS-MORE: live child/elderly/close_relative/woman_unarmed factor overrides (ceil parity).
    // Haxe: ServerSettings.PrestigeCostPerDamageForChild/Elderly/CloseRelatives/WomenWithoutWeapon
    #[test]
    fn prestige_cost_live_non_ally_factor_overrides_ceil() {
        // Child: damage 1.2 * factor 2.0 → ceil 3 (default factor 5 → ceil 6)
        let mut factors = PrestigeCostFactors::default();
        factors.child = 2.0;
        let child = compute_hit_reputation_with_factors(
            &HitReputationInput {
                damage: 1.2,
                target_true_age: 2.0,
                ..Default::default()
            },
            &factors,
        );
        assert_eq!(child.prestige_cost_category, PrestigeCostCategory::Child);
        assert!((child.prestige_cost - 3.0).abs() < 1e-5);

        // C-SS-MORE-BATCH3: live MinAgeToEat raises child threshold
        factors.min_age_to_eat = 5.0;
        let older_child = compute_hit_reputation_with_factors(
            &HitReputationInput {
                damage: 1.0,
                target_true_age: 4.0, // was not child at default 3, is child at 5
                ..Default::default()
            },
            &factors,
        );
        assert_eq!(
            older_child.prestige_cost_category,
            PrestigeCostCategory::Child
        );

        // Elderly: damage 2.5 * factor 0.5 → ceil 2 (default 1 → ceil 3)
        factors = PrestigeCostFactors::default();
        factors.elderly = 0.5;
        let elderly = compute_hit_reputation_with_factors(
            &HitReputationInput {
                damage: 2.5,
                target_true_age: 55.0,
                ..Default::default()
            },
            &factors,
        );
        assert_eq!(elderly.prestige_cost_category, PrestigeCostCategory::Elderly);
        assert!((elderly.prestige_cost - 2.0).abs() < 1e-5);

        // Close relative: damage 3.0 * factor 2.0 → ceil 6 (default 0.5 → ceil 2)
        factors = PrestigeCostFactors::default();
        factors.close_relative = 2.0;
        let rel = compute_hit_reputation_with_factors(
            &HitReputationInput {
                damage: 3.0,
                target_true_age: 20.0,
                target_is_close_relative: true,
                ..Default::default()
            },
            &factors,
        );
        assert_eq!(rel.prestige_cost_category, PrestigeCostCategory::CloseRelative);
        assert!((rel.prestige_cost - 6.0).abs() < 1e-5);

        // Woman unarmed: damage 1.5 * factor 2.0 → ceil 3 (default 0.5 → ceil 1)
        factors = PrestigeCostFactors::default();
        factors.woman_unarmed = 2.0;
        let woman = compute_hit_reputation_with_factors(
            &HitReputationInput {
                damage: 1.5,
                target_true_age: 20.0,
                target_is_female: true,
                ..Default::default()
            },
            &factors,
        );
        assert_eq!(
            woman.prestige_cost_category,
            PrestigeCostCategory::WomanUnarmed
        );
        assert!((woman.prestige_cost - 3.0).abs() < 1e-5);

        // Default child still ceil(1.2*5)=6
        let def = compute_hit_reputation(&HitReputationInput {
            damage: 1.2,
            target_true_age: 2.0,
            ..Default::default()
        });
        assert!((def.prestige_cost - 6.0).abs() < 1e-5);
    }

    #[test]
    fn combat_reputation_restore_only_when_calm_and_lost() {
        // rate 2/year → (2 * 60) / 60 = 2.0 per 60s
        let d = combat_reputation_restore_delta(1.0, 5.0, 0.0, 60.0);
        assert!((d - 2.0).abs() < 1e-5);
        // angry (negative angryTime) → no restore
        assert_eq!(combat_reputation_restore_delta(-1.0, 5.0, 0.0, 60.0), 0.0);
        // zero lost → no restore
        assert_eq!(combat_reputation_restore_delta(1.0, 0.0, 0.0, 60.0), 0.0);
        // darkNosaj >= 1 → no restore
        assert_eq!(combat_reputation_restore_delta(1.0, 5.0, 1.0, 60.0), 0.0);
        // C-SS-TAIL-KNOBS live rate: dt=60 → delta == live_rate
        let live = combat_reputation_restore_delta_ex(1.0, 5.0, 0.0, 60.0, 3.5);
        assert!((live - 3.5).abs() < 1e-5);
        let zero_rate = combat_reputation_restore_delta_ex(1.0, 5.0, 0.0, 60.0, 0.0);
        assert_eq!(zero_rate, 0.0);
    }
}
