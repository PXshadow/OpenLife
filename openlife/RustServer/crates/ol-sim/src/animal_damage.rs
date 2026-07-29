//! Haxe `TimeHelper` animal **damage / escape** pure rules (`TIME-ANIMAL` / `damage_escape`).
//!
//! Anchors:
//! - `DoAnimalDamage` / `DoAnimalDamageHelper` — hurt players on animal move path
//! - `TryAnimaEscape` — chance animal flees when attacked
//! - `MakeAnimalsRunAway` — nearby animals speed up when a player steps
//! - `GlobalPlayerInstance.DoDamage` animal factor branch (`attacker == null`)
//!
//! **COMBAT-MOSQUITO-KIND**: Mosquito Swarm 2156 path damage + fever (not isDeadlyAnimal).

use crate::animals::{Animal, AnimalKind, AnimalWorld};
use crate::combat::CombatState;
use crate::environment::Season;

/// Haxe `ServerSettings.AnimalDamageFactor`.
pub const ANIMAL_DAMAGE_FACTOR: f32 = 1.5;
/// Haxe `ServerSettings.AnimalDamageFactorInWinter`.
pub const ANIMAL_DAMAGE_FACTOR_IN_WINTER: f32 = 2.0;
/// Haxe `ServerSettings.AnimalDamageFactorIfAttacked` (when `animal.hits > 0`).
pub const ANIMAL_DAMAGE_FACTOR_IF_ATTACKED: f32 = 1.5;
/// Haxe `ServerSettings.AnimalDeadlyDistanceFactor` (patched onto deadly animals).
pub const ANIMAL_DEADLY_DISTANCE: f32 = 0.5;
/// Haxe `ObjectData.animalEscapeFactor` default.
pub const DEFAULT_ANIMAL_ESCAPE_FACTOR: f32 = 0.7;
/// Haxe `MakeAnimalsRunAway` default `searchDistance`.
pub const RUN_AWAY_SEARCH_DISTANCE: i32 = 1;
/// Haxe `timeToChange /= 5` when fleeing / running away.
pub const ESCAPE_TIMER_DIVISOR: f32 = 5.0;
/// Haxe `animal.hits -= 0.005` per movement attempt.
pub const HITS_DECAY_PER_MOVE: f32 = 0.005;
/// Haxe `animalEscapeFactor - target.hits * 0.25`.
pub const ESCAPE_HITS_PENALTY: f32 = 0.25;
/// Bow + quiver clothing parent ids halve escape factor (harder to escape).
pub const ARROW_QUIVER_ID: i32 = 3948;
pub const EMPTY_ARROW_QUIVER_ID: i32 = 874;
/// Bow and Arrow held id (Haxe `weapon.id == 152`).
pub const BOW_AND_ARROW_ID: i32 = 152;
/// Haxe `TryAnimaEscape` bow swap → Bloody Yew Bow.
pub const BLOODY_YEW_BOW_ID: i32 = 749;
/// Haxe `PlaceObject(..., 798)` arrow wound on escape tile.
pub const ARROW_WOUND_OBJECT_ID: i32 = 798;
/// After a successful path bite, keep timer short (Haxe skips full re-arm when damage>0).
pub const POST_HIT_TIMER_CAP: f32 = 0.35;

/// Combat profile for a wild animal kind (ServerSettings object patches).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimalCombatProfile {
    pub deadly_distance: f32,
    pub damage: f32,
    pub escape_factor: f32,
}

impl AnimalKind {
    /// Haxe combat table for sim animal kinds (path damage / deadlyDistance).
    ///
    /// - Rabbit (3566): not deadly (`damage=0`)
    /// - Wolf (418): damage 3, deadlyDistance 0.5
    /// - Boar (1323): damage 3, deadlyDistance 0.5
    /// - Mosquito (2156): damage 1, deadlyDistance 0.5 — non-real fever path
    // Haxe: ServerSettings.PatchObjectData 2156 damage=1 deadlyDistance factor
    // COMBAT-MOSQUITO-KIND
    pub fn combat_profile(self) -> AnimalCombatProfile {
        match self {
            AnimalKind::Rabbit => AnimalCombatProfile {
                deadly_distance: 0.0,
                damage: 0.0,
                escape_factor: DEFAULT_ANIMAL_ESCAPE_FACTOR,
            },
            AnimalKind::Wolf => AnimalCombatProfile {
                deadly_distance: ANIMAL_DEADLY_DISTANCE,
                damage: 3.0,
                escape_factor: DEFAULT_ANIMAL_ESCAPE_FACTOR,
            },
            AnimalKind::Boar => AnimalCombatProfile {
                deadly_distance: ANIMAL_DEADLY_DISTANCE,
                damage: 3.0,
                escape_factor: DEFAULT_ANIMAL_ESCAPE_FACTOR,
            },
            AnimalKind::Mosquito => AnimalCombatProfile {
                deadly_distance: ANIMAL_DEADLY_DISTANCE,
                damage: 1.0,
                escape_factor: DEFAULT_ANIMAL_ESCAPE_FACTOR,
            },
        }
    }

    /// True when the animal can hurt players on its move path.
    ///
    /// Mosquito is path-deadly but not Haxe `isDeadlyAnimal` (chase/AI use
    /// [`AnimalKind::is_deadly_animal`]).
    pub fn is_deadly(self) -> bool {
        let p = self.combat_profile();
        p.deadly_distance > 0.0 && p.damage > 0.0
    }

    /// Haxe `ObjectData.isDomesticAnimal` — biome[0]==GREEN domestic herd.
    ///
    /// Current sim kinds are wild only; returns false until domestic kinds land.
    #[inline]
    pub fn is_domestic(self) -> bool {
        false
    }
}

/// Haxe animal-branch org damage before roll:
/// `damage * AnimalDamageFactor [* Winter] [* IfAttacked]`.
pub fn org_animal_damage(base_damage: f32, season: Season, animal_hits: f32) -> f32 {
    let mut org = base_damage.max(0.0) * ANIMAL_DAMAGE_FACTOR;
    if season == Season::Winter {
        org *= ANIMAL_DAMAGE_FACTOR_IN_WINTER;
    }
    if animal_hits > 0.0 {
        org *= ANIMAL_DAMAGE_FACTOR_IF_ATTACKED;
    }
    org
}

/// Haxe damage roll: `(org/2) + org * rng01` (same shape as combat weapons).
#[inline]
pub fn roll_damage(org: f32, rng01: f32) -> f32 {
    CombatState::roll_base_damage(org, rng01)
}

/// Decay Haxe `animal.hits` after a movement attempt (`hits -= 0.005`, clamp ≥ 0).
pub fn decay_animal_hits(animal: &mut Animal) {
    if animal.hits > 0.0 {
        animal.hits = (animal.hits - HITS_DECAY_PER_MOVE).max(0.0);
    }
}

/// Walk path cells from `(from_x, from_y)` toward `(to_x, to_y)` inclusive of both
/// ends (Haxe `DoAnimalDamageHelper` step loop, max 10 steps).
pub fn path_cells(from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> Vec<(i32, i32)> {
    let mut out = Vec::with_capacity(11);
    let mut tmp_x = from_x;
    let mut tmp_y = from_y;
    out.push((tmp_x, tmp_y));
    for _ in 0..10 {
        if tmp_x == to_x && tmp_y == to_y {
            break;
        }
        if to_x > tmp_x {
            tmp_x += 1;
        } else if to_x < tmp_x {
            tmp_x -= 1;
        }
        if to_y > tmp_y {
            tmp_y += 1;
        } else if to_y < tmp_y {
            tmp_y -= 1;
        }
        out.push((tmp_x, tmp_y));
        if tmp_x == to_x && tmp_y == to_y {
            break;
        }
    }
    out
}

/// Haxe `isCloseUseExact`: Euclidean tile distance ≤ `deadly_distance`.
#[inline]
pub fn player_in_deadly_range(
    player_x: i32,
    player_y: i32,
    cell_x: i32,
    cell_y: i32,
    deadly_distance: f32,
) -> bool {
    if deadly_distance <= 0.0 {
        return false;
    }
    let dx = (player_x - cell_x) as f32;
    let dy = (player_y - cell_y) as f32;
    dx * dx + dy * dy <= deadly_distance * deadly_distance
}

/// One eligible player for animal path damage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageTarget {
    pub p_id: i32,
    pub x: i32,
    pub y: i32,
}

/// First player on the animal path within deadly range (Haxe returns after first hit).
///
/// `players` should already exclude deleted / held-by-mother bodies.
pub fn first_player_on_path(
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
    deadly_distance: f32,
    players: &[DamageTarget],
) -> Option<DamageTarget> {
    if deadly_distance <= 0.0 || players.is_empty() {
        return None;
    }
    for (cx, cy) in path_cells(from_x, from_y, to_x, to_y) {
        for p in players {
            if player_in_deadly_range(p.x, p.y, cx, cy, deadly_distance) {
                return Some(*p);
            }
        }
    }
    None
}

/// Result of applying animal path damage to a player (pure numbers).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimalDamageHit {
    pub target_p_id: i32,
    pub org_damage: f32,
    pub rolled_damage: f32,
    pub applied_damage: f32,
}

/// Compute Haxe-style animal damage vs one player (protection applied).
pub fn compute_animal_damage_on_player(
    kind: AnimalKind,
    animal_hits: f32,
    season: Season,
    clothing_insulation: f32,
    floor_insulation: f32,
    weapon_protection: f32,
    target_food_max: f32,
    rng01: f32,
) -> Option<(f32 /*org*/, f32 /*applied*/)> {
    let profile = kind.combat_profile();
    if profile.damage <= 0.0 || profile.deadly_distance <= 0.0 {
        return None;
    }
    let org = org_animal_damage(profile.damage, season, animal_hits);
    let base = roll_damage(org, rng01);
    let raw = CombatState::apply_protection(
        base,
        clothing_insulation,
        floor_insulation,
        weapon_protection,
    );
    // Haxe: cap vs calculateNotReducedFoodStoreMax (~20), not live reduced food_max
    let _ = target_food_max; // kept for API stability; cap uses not-reduced base
    let applied = CombatState::cap_damage_default(raw);
    Some((org, applied))
}

/// Full `DoAnimalDamageHelper` pure outcome: first player on path + damage numbers.
pub fn resolve_animal_path_damage(
    kind: AnimalKind,
    animal_hits: f32,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
    season: Season,
    players: &[DamageTarget],
    // clothing, floor, weapon_prot, food_max for the hit player
    target_stats: impl Fn(i32) -> (f32, f32, f32, f32),
    rng01: f32,
) -> Option<AnimalDamageHit> {
    let profile = kind.combat_profile();
    if !kind.is_deadly() {
        return None;
    }
    let target = first_player_on_path(
        from_x,
        from_y,
        to_x,
        to_y,
        profile.deadly_distance,
        players,
    )?;
    let (cloth, floor, wprot, food_max) = target_stats(target.p_id);
    let (org, applied) = compute_animal_damage_on_player(
        kind,
        animal_hits,
        season,
        cloth,
        floor,
        wprot,
        food_max,
        rng01,
    )?;
    let rolled = roll_damage(org, rng01);
    Some(AnimalDamageHit {
        target_p_id: target.p_id,
        org_damage: org,
        rolled_damage: rolled,
        applied_damage: applied,
    })
}

/// Inputs for Haxe `TryAnimaEscape`.
#[derive(Debug, Clone, Copy)]
pub struct EscapeAttempt {
    pub weapon_escape_factor: f32,
    pub animal_hits: f32,
    pub using_bow_and_arrow: bool,
    /// True if clothing includes Arrow Quiver / Empty Arrow Quiver.
    pub has_quiver: bool,
    pub rng01: f32,
}

/// Outcome of an escape roll (before forcing a move transition).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeRoll {
    /// `random > factor` — animal stays / attack may continue.
    NoEscape,
    /// Animal should flee (speed timer + force wander).
    Escape,
}

/// Effective escape factor after hits + quiver (Haxe `TryAnimaEscape`).
pub fn effective_escape_factor(weapon_escape_factor: f32, animal_hits: f32, has_quiver: bool) -> f32 {
    let mut f = weapon_escape_factor - animal_hits * ESCAPE_HITS_PENALTY;
    if has_quiver {
        f /= 2.0;
    }
    f
}

/// Haxe `TryAnimaEscape` RNG gate (does not mutate timers).
///
/// Note: Haxe still increments `target.hits` before this roll.
pub fn try_animal_escape_roll(attempt: EscapeAttempt) -> EscapeRoll {
    let factor = effective_escape_factor(
        attempt.weapon_escape_factor,
        attempt.animal_hits,
        attempt.has_quiver && attempt.using_bow_and_arrow,
    );
    if attempt.rng01 > factor {
        EscapeRoll::NoEscape
    } else {
        EscapeRoll::Escape
    }
}

/// Haxe early-out: domestic + no weapon → skip escape (return false without hits++).
///
/// Callers should skip `register_animal_hit` / roll when this is true.
#[inline]
pub fn skip_escape_for_domestic(is_domestic: bool, holding_weapon: bool) -> bool {
    is_domestic && !holding_weapon
}

/// Haxe `MakeAnimalsRunAway` domestic skip when player holds no weapon.
#[inline]
pub fn skip_run_away_for_domestic(is_domestic: bool, holding_weapon: bool) -> bool {
    is_domestic && !holding_weapon
}

/// Haxe `ServerSettings.PatchObjectData` weapon deadlyDistance patches (subset).
/// Knife 560, War Sword 3047, Bow 152 / 1624, bloody variants 750 / 3048 / 749.
// Haxe: ServerSettings.PatchObjectData deadlyDistance weapons
pub const KNOWN_WEAPON_OBJECT_IDS: &[i32] = &[
    BOW_AND_ARROW_ID, // 152
    1624,             // Bow and Arrow with Note
    560,              // Knife
    3047,             // War Sword
    750,              // Bloody Knife
    3048,             // Bloody War Sword
    BLOODY_YEW_BOW_ID, // 749
];

/// Haxe `ObjectData.isWeapon` pure: `deadlyDistance > 0`.
// Haxe: ObjectData.isWeapon
#[inline]
pub fn is_weapon_from_deadly_distance(deadly_distance: f32) -> bool {
    deadly_distance.is_finite() && deadly_distance > 0.0
}

/// Haxe `isHoldingWeapon` / `ObjectData.isWeapon`.
///
/// Prefer known patched weapon ids + name heuristic when content `deadlyDistance`
/// is not loaded (content DB does not yet carry deadlyDistance for all objects).
// Haxe: GlobalPlayerInstance.isHoldingWeapon
pub fn is_holding_weapon(held_id: i32, held_name: &str) -> bool {
    if held_id == 0 {
        return false;
    }
    if KNOWN_WEAPON_OBJECT_IDS.contains(&held_id) {
        return true;
    }
    let n = held_name.to_ascii_lowercase();
    n.contains("bow")
        || n.contains("arrow")
        || n.contains("sword")
        || n.contains("knife")
        || n.contains("spear")
        || n.contains("lance")
        || n.contains("axe")
        || n.contains("club")
        || n.contains("hammer")
        || n.contains("blade")
}

/// Default weapon escape factor (content field missing → 0.7).
pub fn weapon_escape_factor(held_id: i32, held_name: &str) -> f32 {
    if held_id == 0 {
        return DEFAULT_ANIMAL_ESCAPE_FACTOR;
    }
    let n = held_name.to_ascii_lowercase();
    // Slightly lower factor (harder escape) for bows; knives mid; bare default.
    if n.contains("bow") || n.contains("arrow") || held_id == BOW_AND_ARROW_ID {
        0.55
    } else if n.contains("spear") {
        0.6
    } else if n.contains("knife") || n.contains("sword") || n.contains("axe") {
        0.65
    } else {
        DEFAULT_ANIMAL_ESCAPE_FACTOR
    }
}

/// Speed up animal flee timer (Haxe `timeToChange /= 5`).
pub fn accelerate_flee_timer(move_timer: f32) -> f32 {
    (move_timer / ESCAPE_TIMER_DIVISOR).max(0.05)
}

/// Apply flee acceleration to one animal (escape / run-away).
pub fn accelerate_animal_flee(animal: &mut Animal) {
    animal.move_timer = accelerate_flee_timer(animal.move_timer);
}

/// Haxe: after successful path damage, skip full `calculateTimeToChange` re-arm.
/// Cap the already-rearmed timer so the animal can bite again soon.
pub fn preserve_short_timer_after_hit(move_timer: f32) -> f32 {
    move_timer.min(POST_HIT_TIMER_CAP).max(0.05)
}

/// Apply [`preserve_short_timer_after_hit`] to a live animal.
pub fn apply_post_hit_timer(animal: &mut Animal) {
    animal.move_timer = preserve_short_timer_after_hit(animal.move_timer);
}

/// Haxe exclusive for-range: `for t in base-d ... base+d` → `[base-d, base+d)`.
///
/// At `search_distance=1` this is a 2×2 box (player tile + −X/−Y), not full Chebyshev 3×3.
#[inline]
pub fn in_run_away_range(
    animal_x: i32,
    animal_y: i32,
    player_x: i32,
    player_y: i32,
    search_distance: i32,
) -> bool {
    let d = search_distance.max(0);
    animal_x >= player_x - d
        && animal_x < player_x + d
        && animal_y >= player_y - d
        && animal_y < player_y + d
}

/// Haxe `MakeAnimalsRunAway`: animals in exclusive `[player±d)` box get timer /= 5.
///
/// Domestic animals are skipped when `player_holding_weapon` is false.
/// Returns animal ids that were accelerated.
pub fn make_animals_run_away(
    animals: &mut AnimalWorld,
    player_x: i32,
    player_y: i32,
    search_distance: i32,
) -> Vec<i32> {
    make_animals_run_away_ex(animals, player_x, player_y, search_distance, true)
}

/// Like [`make_animals_run_away`] with explicit weapon flag (domestic skip).
pub fn make_animals_run_away_ex(
    animals: &mut AnimalWorld,
    player_x: i32,
    player_y: i32,
    search_distance: i32,
    player_holding_weapon: bool,
) -> Vec<i32> {
    let mut accelerated = Vec::new();
    for a in &mut animals.animals {
        if skip_run_away_for_domestic(a.kind.is_domestic(), player_holding_weapon) {
            continue;
        }
        if in_run_away_range(a.x, a.y, player_x, player_y, search_distance) {
            accelerate_animal_flee(a);
            accelerated.push(a.id);
        }
    }
    accelerated
}

/// Register a successful strike on an animal (Haxe `target.hits += 1`).
pub fn register_animal_hit(animal: &mut Animal) {
    animal.hits += 1.0;
}

/// True when any clothing slot is Arrow Quiver / Empty Arrow Quiver.
///
/// Haxe scans full `clothingObjects`; pass all known slots (hat/chest/shoes/…).
pub fn clothing_has_quiver_ids(clothing_ids: &[i32]) -> bool {
    clothing_ids
        .iter()
        .any(|&id| id == ARROW_QUIVER_ID || id == EMPTY_ARROW_QUIVER_ID)
}

/// Clothing slots that count as quiver for escape (hat/chest/shoes convenience).
pub fn clothing_has_quiver(hat: i32, chest: i32, shoes: i32) -> bool {
    clothing_has_quiver_ids(&[hat, chest, shoes])
}

/// Haxe `TryAnimaEscape` `weapon.timeToChange = 2` after swap to Bloody Yew Bow.
pub const BOW_ESCAPE_BLOODY_TTC: f32 = 2.0;

/// Pure bow-escape side effects (Haxe `TryAnimaEscape` usingBowAndArrow branch).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BowEscapeEffects {
    /// Held becomes Bloody Yew Bow (749).
    pub new_held_id: i32,
    /// Place arrow wound object id at animal tile.
    pub wound_object_id: i32,
    pub place_x: i32,
    pub place_y: i32,
    /// Haxe `weapon.timeToChange = 2` cool-down / neverDrop window.
    pub time_to_change: f32,
}

/// When bow escape succeeds, swap held to bloody bow and place arrow wound (798).
// Haxe: TimeHelper.TryAnimaEscape bow → 749, timeToChange=2, PlaceObject 798
pub fn bow_escape_effects(
    using_bow_and_arrow: bool,
    animal_x: i32,
    animal_y: i32,
) -> Option<BowEscapeEffects> {
    if !using_bow_and_arrow {
        return None;
    }
    Some(BowEscapeEffects {
        new_held_id: BLOODY_YEW_BOW_ID,
        wound_object_id: ARROW_WOUND_OBJECT_ID,
        place_x: animal_x,
        place_y: animal_y,
        time_to_change: BOW_ESCAPE_BLOODY_TTC,
    })
}

/// Full TryAnimaEscape decision: domestic gate → hits++ → roll → flee/bow effects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EscapeOutcome {
    /// Domestic + bare hands: no hit register, no damage path change.
    SkippedDomestic,
    /// Roll failed: animal stays (hunt/damage may continue).
    Stayed {
        animal_hits_after: f32,
    },
    /// Escape: accelerate flee; optional bow side effects.
    Escaped {
        animal_hits_after: f32,
        bow: Option<BowEscapeEffects>,
    },
}

/// Pure `TryAnimaEscape` pipeline (caller mutates animal from outcome).
pub fn resolve_animal_escape(
    is_domestic: bool,
    holding_weapon: bool,
    weapon_escape_factor: f32,
    animal_hits_before: f32,
    using_bow_and_arrow: bool,
    has_quiver: bool,
    animal_x: i32,
    animal_y: i32,
    rng01: f32,
) -> EscapeOutcome {
    // Haxe: domestic + no weapon returns false *before* hits += 1.
    if skip_escape_for_domestic(is_domestic, holding_weapon) {
        return EscapeOutcome::SkippedDomestic;
    }
    let animal_hits_after = animal_hits_before + 1.0;
    let roll = try_animal_escape_roll(EscapeAttempt {
        weapon_escape_factor,
        animal_hits: animal_hits_after,
        using_bow_and_arrow,
        has_quiver,
        rng01,
    });
    match roll {
        EscapeRoll::NoEscape => EscapeOutcome::Stayed { animal_hits_after },
        EscapeRoll::Escape => EscapeOutcome::Escaped {
            animal_hits_after,
            bow: bow_escape_effects(using_bow_and_arrow, animal_x, animal_y),
        },
    }
}

/// Apply escape outcome to animal (hits + flee timer). Returns bow effects if any.
pub fn apply_escape_outcome(animal: &mut Animal, outcome: EscapeOutcome) -> Option<BowEscapeEffects> {
    match outcome {
        EscapeOutcome::SkippedDomestic => None,
        EscapeOutcome::Stayed { animal_hits_after } => {
            animal.hits = animal_hits_after;
            None
        }
        EscapeOutcome::Escaped {
            animal_hits_after,
            bow,
        } => {
            animal.hits = animal_hits_after;
            accelerate_animal_flee(animal);
            bow
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animals::AnimalWorld;

    #[test]
    fn wolf_and_boar_deadly_rabbit_not() {
        assert!(AnimalKind::Wolf.is_deadly());
        assert!(AnimalKind::Boar.is_deadly());
        assert!(!AnimalKind::Rabbit.is_deadly());
        assert_eq!(AnimalKind::Wolf.combat_profile().damage, 3.0);
        assert_eq!(
            AnimalKind::Wolf.combat_profile().deadly_distance,
            ANIMAL_DEADLY_DISTANCE
        );
        assert!(!AnimalKind::Wolf.is_domestic());
        // COMBAT-MOSQUITO-KIND
        assert!(AnimalKind::Mosquito.is_deadly());
        assert_eq!(AnimalKind::Mosquito.combat_profile().damage, 1.0);
        assert_eq!(
            AnimalKind::Mosquito.combat_profile().deadly_distance,
            ANIMAL_DEADLY_DISTANCE
        );
        assert!(!AnimalKind::Mosquito.is_deadly_animal());
        assert!(!AnimalKind::Mosquito.is_deadly_for_ai());
    }

    #[test]
    fn mosquito_path_damage_hits_same_tile() {
        let players = [DamageTarget {
            p_id: 9,
            x: 4,
            y: 4,
        }];
        let hit = resolve_animal_path_damage(
            AnimalKind::Mosquito,
            0.0,
            4,
            4,
            4,
            4,
            Season::Spring,
            &players,
            |_id| (0.0, 0.0, 1.0, 20.0),
            0.5,
        )
        .expect("mosquito damages on same tile");
        assert_eq!(hit.target_p_id, 9);
        assert!(hit.applied_damage > 0.0);
        // org = 1.0 * AnimalDamageFactor 1.5 = 1.5
        assert!((hit.org_damage - 1.5).abs() < 1e-4);
    }

    #[test]
    fn org_damage_factors_winter_and_attacked() {
        let base = 3.0;
        let spring = org_animal_damage(base, Season::Spring, 0.0);
        assert!((spring - base * ANIMAL_DAMAGE_FACTOR).abs() < 1e-5);
        let winter = org_animal_damage(base, Season::Winter, 0.0);
        assert!((winter - spring * ANIMAL_DAMAGE_FACTOR_IN_WINTER).abs() < 1e-5);
        let attacked = org_animal_damage(base, Season::Spring, 1.0);
        assert!((attacked - spring * ANIMAL_DAMAGE_FACTOR_IF_ATTACKED).abs() < 1e-5);
    }

    #[test]
    fn path_cells_steps_diagonally() {
        let cells = path_cells(0, 0, 2, 2);
        assert_eq!(cells, vec![(0, 0), (1, 1), (2, 2)]);
        let same = path_cells(3, 3, 3, 3);
        assert_eq!(same, vec![(3, 3)]);
    }

    #[test]
    fn animal_damage_applies_when_adjacent_same_tile() {
        // deadlyDistance 0.5 → only same tile (Euclidean).
        let players = [DamageTarget {
            p_id: 7,
            x: 5,
            y: 5,
        }];
        // Path ends on player tile.
        let hit = first_player_on_path(3, 5, 5, 5, ANIMAL_DEADLY_DISTANCE, &players);
        assert_eq!(hit.map(|p| p.p_id), Some(7));
        // Path misses player.
        let miss = first_player_on_path(0, 0, 1, 0, ANIMAL_DEADLY_DISTANCE, &players);
        assert!(miss.is_none());
        // Adjacent integer tile is outside 0.5 Euclidean.
        let adj = first_player_on_path(4, 5, 4, 5, ANIMAL_DEADLY_DISTANCE, &players);
        assert!(adj.is_none());
    }

    #[test]
    fn resolve_path_damage_hits_player_on_endpoint() {
        let players = [DamageTarget {
            p_id: 1,
            x: 2,
            y: 2,
        }];
        let hit = resolve_animal_path_damage(
            AnimalKind::Wolf,
            0.0,
            0,
            2,
            2,
            2,
            Season::Spring,
            &players,
            |_id| (0.0, 0.0, 1.0, 20.0),
            0.5,
        )
        .expect("wolf damages player on path");
        assert_eq!(hit.target_p_id, 1);
        assert!(hit.applied_damage > 0.0);
        assert!(hit.org_damage > 0.0);
        // Rabbit never damages.
        assert!(resolve_animal_path_damage(
            AnimalKind::Rabbit,
            0.0,
            0,
            2,
            2,
            2,
            Season::Spring,
            &players,
            |_id| (0.0, 0.0, 1.0, 20.0),
            0.5,
        )
        .is_none());
    }

    #[test]
    fn escape_roll_respects_factor_and_hits() {
        // factor 0.7, rng 0.8 → no escape
        assert_eq!(
            try_animal_escape_roll(EscapeAttempt {
                weapon_escape_factor: 0.7,
                animal_hits: 0.0,
                using_bow_and_arrow: false,
                has_quiver: false,
                rng01: 0.8,
            }),
            EscapeRoll::NoEscape
        );
        // rng 0.5 < 0.7 → escape
        assert_eq!(
            try_animal_escape_roll(EscapeAttempt {
                weapon_escape_factor: 0.7,
                animal_hits: 0.0,
                using_bow_and_arrow: false,
                has_quiver: false,
                rng01: 0.5,
            }),
            EscapeRoll::Escape
        );
        // hits reduce factor: 0.7 - 2*0.25 = 0.2; rng 0.3 → no escape
        assert_eq!(
            try_animal_escape_roll(EscapeAttempt {
                weapon_escape_factor: 0.7,
                animal_hits: 2.0,
                using_bow_and_arrow: false,
                has_quiver: false,
                rng01: 0.3,
            }),
            EscapeRoll::NoEscape
        );
    }

    #[test]
    fn quiver_halves_escape_with_bow() {
        let f = effective_escape_factor(0.6, 0.0, true);
        assert!((f - 0.3).abs() < 1e-5);
        // has_quiver only counts when bow is used in roll helper
        assert_eq!(
            try_animal_escape_roll(EscapeAttempt {
                weapon_escape_factor: 0.6,
                animal_hits: 0.0,
                using_bow_and_arrow: true,
                has_quiver: true,
                rng01: 0.4, // 0.4 > 0.3 → no escape
            }),
            EscapeRoll::NoEscape
        );
    }

    #[test]
    fn make_animals_run_away_accelerates_nearby() {
        let mut w = AnimalWorld::new();
        // Exclusive range at d=1: animal on player tile is in; +1,+0 is out.
        let near = w.spawn(AnimalKind::Rabbit, 1, 1);
        let edge_out = w.spawn(AnimalKind::Wolf, 2, 1); // px+1 excluded
        let far = w.spawn(AnimalKind::Wolf, 10, 10);
        w.animals[0].move_timer = 5.0;
        w.animals[1].move_timer = 5.0;
        w.animals[2].move_timer = 5.0;
        let ids = make_animals_run_away(&mut w, 1, 1, RUN_AWAY_SEARCH_DISTANCE);
        assert!(ids.contains(&near));
        assert!(!ids.contains(&edge_out));
        assert!(!ids.contains(&far));
        assert!((w.animals[0].move_timer - 1.0).abs() < 1e-4); // 5/5
        assert!((w.animals[1].move_timer - 5.0).abs() < 1e-4);
        assert!((w.animals[2].move_timer - 5.0).abs() < 1e-4);
    }

    #[test]
    fn run_away_range_is_haxe_exclusive_2x2() {
        // searchDistance=1 → tiles [px-1,px] × [py-1,py]
        assert!(in_run_away_range(5, 5, 5, 5, 1));
        assert!(in_run_away_range(4, 4, 5, 5, 1));
        assert!(in_run_away_range(4, 5, 5, 5, 1));
        assert!(in_run_away_range(5, 4, 5, 5, 1));
        assert!(!in_run_away_range(6, 5, 5, 5, 1));
        assert!(!in_run_away_range(5, 6, 5, 5, 1));
        assert!(!in_run_away_range(6, 6, 5, 5, 1));
    }

    #[test]
    fn hits_decay_and_register() {
        let mut a = Animal {
            id: 0,
            x: 0,
            y: 0,
            kind: AnimalKind::Wolf,
            hp: 20,
            move_timer: 1.0,
            hits: 1.0,
            loved_tx: 0,
            loved_ty: 0,
            target: None,
            failed_moves: 0.0,
            object_id: AnimalKind::Wolf.object_id(),
        };
        register_animal_hit(&mut a);
        assert!((a.hits - 2.0).abs() < 1e-5);
        decay_animal_hits(&mut a);
        assert!((a.hits - (2.0 - HITS_DECAY_PER_MOVE)).abs() < 1e-5);
    }

    #[test]
    fn clothing_quiver_detect() {
        assert!(clothing_has_quiver(ARROW_QUIVER_ID, 0, 0));
        assert!(clothing_has_quiver(0, EMPTY_ARROW_QUIVER_ID, 0));
        assert!(!clothing_has_quiver(1, 2, 3));
        // Full clothing scan (extra slots).
        assert!(clothing_has_quiver_ids(&[0, 0, 0, ARROW_QUIVER_ID]));
        assert!(!clothing_has_quiver_ids(&[1, 2, 3, 4]));
    }

    #[test]
    fn domestic_skip_escape_and_run_away() {
        assert!(skip_escape_for_domestic(true, false));
        assert!(!skip_escape_for_domestic(true, true));
        assert!(!skip_escape_for_domestic(false, false));
        assert!(skip_run_away_for_domestic(true, false));
        assert!(!skip_run_away_for_domestic(false, false));
        match resolve_animal_escape(true, false, 0.7, 0.0, false, false, 0, 0, 0.1) {
            EscapeOutcome::SkippedDomestic => {}
            o => panic!("expected skip, got {o:?}"),
        }
    }

    #[test]
    fn bow_escape_places_wound_and_bloody_bow() {
        let e = bow_escape_effects(true, 3, 4).expect("bow effects");
        assert_eq!(e.new_held_id, BLOODY_YEW_BOW_ID);
        assert_eq!(e.wound_object_id, ARROW_WOUND_OBJECT_ID);
        assert_eq!((e.place_x, e.place_y), (3, 4));
        assert!((e.time_to_change - BOW_ESCAPE_BLOODY_TTC).abs() < 1e-6);
        assert!(bow_escape_effects(false, 0, 0).is_none());

        let out = resolve_animal_escape(
            false,
            true,
            0.7,
            0.0,
            true,
            false,
            9,
            8,
            0.1, // escape
        );
        match out {
            EscapeOutcome::Escaped {
                bow: Some(b),
                animal_hits_after,
            } => {
                assert!((animal_hits_after - 1.0).abs() < 1e-5);
                assert_eq!(b.new_held_id, BLOODY_YEW_BOW_ID);
                assert_eq!(b.place_x, 9);
                assert!((b.time_to_change - 2.0).abs() < 1e-6);
            }
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn post_hit_timer_capped() {
        assert!((preserve_short_timer_after_hit(3.0) - POST_HIT_TIMER_CAP).abs() < 1e-5);
        assert!((preserve_short_timer_after_hit(0.1) - 0.1).abs() < 1e-5);
    }

    #[test]
    fn is_holding_weapon_name_heuristic() {
        assert!(!is_holding_weapon(0, ""));
        assert!(is_holding_weapon(BOW_AND_ARROW_ID, "Bow and Arrow"));
        assert!(is_holding_weapon(1, "Flint Knife"));
        assert!(!is_holding_weapon(99, "Gooseberry"));
        // PatchObjectData known ids without name
        assert!(is_holding_weapon(560, ""));
        assert!(is_holding_weapon(3047, ""));
        assert!(is_holding_weapon(750, ""));
        assert!(is_weapon_from_deadly_distance(1.5));
        assert!(!is_weapon_from_deadly_distance(0.0));
    }
}
