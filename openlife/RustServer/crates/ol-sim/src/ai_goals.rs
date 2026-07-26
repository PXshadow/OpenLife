//! Minimal AI profession goals for self-play (Haxe `AiBase` profession subset).
//!
//! Pure decision logic: survival first, then profession-biased goals. No world I/O.

use crate::craft_graph::ReverseCraftGraph;
use crate::professions::is_grassland;
use std::collections::HashSet;

/// Self-play / NPC profession role (subset of Haxe profession map keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Profession {
    Forager,
    Farmer,
    Smith,
    Explorer,
    /// Combat / prey-seeking role (Haxe hunt subset).
    Hunter,
}

/// High-level goal chosen each AI decision tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Goal {
    /// Path toward edible objects / eat held food.
    SeekFood,
    /// Path toward a specific object id (craft / profession target).
    SeekObject(i32),
    /// Wander without a fixed target.
    Explore,
    /// Hands busy or nothing useful — hold still preference for action layer.
    Idle,
    /// Move away from a nearby threat.
    Flee,
    /// Path toward / engage nearby prey (Hunter); action layer issues `SAY HUNT` when adjacent.
    Hunt,
    /// Forager grassland gather via `SAY HARVEST` (empty hands + grassland biome).
    Harvest,
}

impl Goal {
    /// Stable debug label for `SAY SEEKING` / self-play logs (no leading prefix).
    pub fn as_label(self) -> String {
        match self {
            Goal::SeekFood => "SEEKFOOD".into(),
            Goal::SeekObject(id) => format!("SEEKOBJECT {id}"),
            Goal::Explore => "EXPLORE".into(),
            Goal::Idle => "IDLE".into(),
            Goal::Flee => "FLEE".into(),
            Goal::Hunt => "HUNT".into(),
            Goal::Harvest => "HARVEST".into(),
        }
    }
}

/// `SAY SEEKING` body without leading p_id: `SEEKING {label}`.
///
/// Optional for human players; primary use is AI / self-play goal self-debug.
pub fn format_seeking_query(goal: Goal) -> String {
    format!("SEEKING {}", goal.as_label())
}

/// Parse optional profession token for `SAY SEEKING [PROF]` (default Forager).
pub fn parse_profession_token(s: &str) -> Option<Profession> {
    match s.trim().to_ascii_uppercase().as_str() {
        "FORAGER" | "FORAGE" => Some(Profession::Forager),
        "FARMER" | "FARM" => Some(Profession::Farmer),
        "SMITH" => Some(Profession::Smith),
        "EXPLORER" | "EXPLORE" => Some(Profession::Explorer),
        "HUNTER" | "HUNT" => Some(Profession::Hunter),
        _ => None,
    }
}

/// Food at or below this value prioritizes [`Goal::SeekFood`].
pub const HUNGRY_FOOD: f32 = 8.0;

/// Default farmer craft preference (wheat-ish; preference only, not content-bound).
pub const FARMER_TARGET_ID: i32 = 242;

/// Default smith craft preference (iron-ish product; preference only).
pub const SMITH_TARGET_ID: i32 = 314;

/// Iron ingredient id for smith reverse-craft expansion via [`ReverseCraftGraph::products_using`].
///
/// OHOL iron ore / iron family preference (same id as default smith target when graph is empty).
pub const SMITH_IRON_ID: i32 = 314;

/// Max products from `products_using(iron)` considered as smith goals.
pub const SMITH_PRODUCT_CAP: usize = 8;

/// Product ids the Smith should work toward, expanded from craft graph.
///
/// Uses [`ReverseCraftGraph::products_using`] on `iron_id`, always including
/// [`SMITH_TARGET_ID`] as fallback when the graph has no iron edges.
pub fn smith_product_targets(graph: &ReverseCraftGraph, iron_id: i32) -> Vec<i32> {
    let mut v = graph.products_using(iron_id);
    if !v.contains(&SMITH_TARGET_ID) {
        v.push(SMITH_TARGET_ID);
    }
    if iron_id != SMITH_TARGET_ID && iron_id > 0 && !v.contains(&iron_id) {
        // Prefer seeking iron itself when nothing lists it as ingredient yet.
        // (Only as last-resort seek when products list is just the fallback.)
    }
    v.truncate(SMITH_PRODUCT_CAP);
    v
}

/// Pick a smith [`Goal::SeekObject`] from reverse-craft products that use iron.
///
/// Preference:
/// 1. First product from [`smith_product_targets`] not in `have` → seek ingredient or product
/// 2. Else [`SMITH_TARGET_ID`]
pub fn pick_smith_goal(graph: &ReverseCraftGraph, have: &HashSet<i32>, iron_id: i32) -> Goal {
    let targets = smith_product_targets(graph, iron_id);
    for &want in &targets {
        if have.contains(&want) {
            continue;
        }
        if let Some(ing) = graph.seek_ingredient_for(want, have) {
            return Goal::SeekObject(ing);
        }
        return Goal::SeekObject(want);
    }
    // All products owned or empty graph — bias toward default smith target / iron.
    if !have.contains(&iron_id) && iron_id > 0 {
        return Goal::SeekObject(iron_id);
    }
    Goal::SeekObject(SMITH_TARGET_ID)
}

/// Pick the next high-level goal from profession + local sensors.
///
/// Priority:
/// 1. Hungry → [`Goal::SeekFood`]
/// 2. Holding something while not hungry → [`Goal::Idle`] (eat/drop elsewhere)
/// 3. Profession defaults (forager seeks nearby food, farmer/smith seek objects, explorer wanders)
pub fn pick_goal(
    profession: Profession,
    held_id: i32,
    food: f32,
    nearby_food: bool,
) -> Goal {
    // No biome sensor: grassland harvest requires [`pick_goal_ext`].
    pick_goal_ext(
        profession,
        held_id,
        food,
        nearby_food,
        false,
        false,
        false,
        0,
    )
}

/// Extended goal picker with threat / prey / biome sensors.
///
/// Priority (before profession defaults):
/// 1. `threat_near` and food &gt; [`HUNGRY_FOOD`] / 2 → [`Goal::Flee`]
/// 2. Hungry (`food <= HUNGRY_FOOD`) → [`Goal::SeekFood`]
/// 3. Holding → [`Goal::Idle`]
/// 4. [`Profession::Hunter`] with `prey_adjacent` and fed → [`Goal::Hunt`]
/// 5. [`Profession::Forager`] empty hands + grassland biome → [`Goal::Harvest`]
/// 6. Profession defaults (forager food/explore, farmer/smith objects, …)
///
/// `biome` is the standing biome id (0 = grassland). Pass `0` when unknown
/// (forager then falls back to nearby-food / explore without Harvest).
///
/// `prey_adjacent` should use hunt range (Chebyshev 1); far prey does not force Hunt
/// (action layer cannot `SAY HUNT` until adjacent).
pub fn pick_goal_ext(
    profession: Profession,
    held_id: i32,
    food: f32,
    nearby_food: bool,
    threat_near: bool,
    prey_adjacent: bool,
    on_grassland: bool,
    // reserved for future craft-aware smith without graph (unused; prefer pick_smith_goal)
    _smith_want: i32,
) -> Goal {
    // Flee when threatened and not critically starving (food > half hungry threshold).
    if threat_near && food > HUNGRY_FOOD / 2.0 {
        return Goal::Flee;
    }

    if food <= HUNGRY_FOOD {
        return Goal::SeekFood;
    }

    if held_id != 0 {
        return Goal::Idle;
    }

    // Hunter engages only when prey is adjacent (SAY HUNT range).
    if profession == Profession::Hunter && prey_adjacent {
        return Goal::Hunt;
    }

    // Forager prefers HARVEST on grassland with empty hands (auto intent / SAY HARVEST).
    if profession == Profession::Forager && on_grassland {
        return Goal::Harvest;
    }

    match profession {
        Profession::Forager => {
            if nearby_food {
                Goal::SeekFood
            } else {
                Goal::Explore
            }
        }
        Profession::Farmer => Goal::SeekObject(FARMER_TARGET_ID),
        Profession::Smith => Goal::SeekObject(SMITH_TARGET_ID),
        Profession::Explorer => Goal::Explore,
        Profession::Hunter => {
            // Prey not adjacent: forage-like wander (seek food if seen, else explore).
            if nearby_food {
                Goal::SeekFood
            } else {
                Goal::Explore
            }
        }
    }
}

/// Convenience: biome id → grassland flag for [`pick_goal_ext`].
pub fn pick_goal_with_biome(
    profession: Profession,
    held_id: i32,
    food: f32,
    nearby_food: bool,
    threat_near: bool,
    prey_adjacent: bool,
    biome: u8,
) -> Goal {
    pick_goal_ext(
        profession,
        held_id,
        food,
        nearby_food,
        threat_near,
        prey_adjacent,
        is_grassland(biome),
        0,
    )
}

/// Smith-aware goal pick: uses craft graph `products_using(iron)` when profession is Smith.
pub fn pick_goal_smith_craft(
    profession: Profession,
    held_id: i32,
    food: f32,
    nearby_food: bool,
    threat_near: bool,
    prey_adjacent: bool,
    on_grassland: bool,
    graph: &ReverseCraftGraph,
    have: &HashSet<i32>,
    iron_id: i32,
) -> Goal {
    let g = pick_goal_ext(
        profession,
        held_id,
        food,
        nearby_food,
        threat_near,
        prey_adjacent,
        on_grassland,
        0,
    );
    if profession == Profession::Smith && matches!(g, Goal::SeekObject(_)) {
        return pick_smith_goal(graph, have, iron_id);
    }
    // Also expand smith when base would Explore/Idle with empty hands and fed.
    if profession == Profession::Smith
        && held_id == 0
        && food > HUNGRY_FOOD
        && matches!(g, Goal::Explore | Goal::Idle | Goal::SeekObject(_))
        && !threat_near
    {
        return pick_smith_goal(graph, have, iron_id);
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goal_as_label_and_format_seeking() {
        assert_eq!(Goal::SeekFood.as_label(), "SEEKFOOD");
        assert_eq!(Goal::SeekObject(242).as_label(), "SEEKOBJECT 242");
        assert_eq!(Goal::Explore.as_label(), "EXPLORE");
        assert_eq!(Goal::Idle.as_label(), "IDLE");
        assert_eq!(Goal::Flee.as_label(), "FLEE");
        assert_eq!(Goal::Hunt.as_label(), "HUNT");
        assert_eq!(Goal::Harvest.as_label(), "HARVEST");
        assert_eq!(
            format_seeking_query(Goal::SeekObject(314)),
            "SEEKING SEEKOBJECT 314"
        );
        assert_eq!(format_seeking_query(Goal::SeekFood), "SEEKING SEEKFOOD");
        assert_eq!(format_seeking_query(Goal::Harvest), "SEEKING HARVEST");
    }

    #[test]
    fn parse_profession_token_names() {
        assert_eq!(parse_profession_token("farmer"), Some(Profession::Farmer));
        assert_eq!(parse_profession_token("HUNTER"), Some(Profession::Hunter));
        assert_eq!(parse_profession_token("smith"), Some(Profession::Smith));
        assert_eq!(parse_profession_token("nope"), None);
    }

    #[test]
    fn hungry_always_seeks_food() {
        for prof in [
            Profession::Forager,
            Profession::Farmer,
            Profession::Smith,
            Profession::Explorer,
            Profession::Hunter,
        ] {
            assert_eq!(pick_goal(prof, 0, HUNGRY_FOOD, false), Goal::SeekFood);
            assert_eq!(pick_goal(prof, 99, 1.0, true), Goal::SeekFood);
            assert_eq!(pick_goal(prof, 0, 0.0, false), Goal::SeekFood);
        }
    }

    #[test]
    fn holding_while_fed_is_idle() {
        assert_eq!(
            pick_goal(Profession::Forager, 33, 15.0, true),
            Goal::Idle
        );
        assert_eq!(
            pick_goal(Profession::Smith, 314, 12.0, false),
            Goal::Idle
        );
    }

    #[test]
    fn forager_seeks_nearby_food_or_explores() {
        // Without grassland flag, base pick_goal has on_grassland=false.
        assert_eq!(
            pick_goal(Profession::Forager, 0, 15.0, true),
            Goal::SeekFood
        );
        assert_eq!(
            pick_goal(Profession::Forager, 0, 15.0, false),
            Goal::Explore
        );
    }

    #[test]
    fn forager_prefers_harvest_on_grassland_empty_hands() {
        assert_eq!(
            pick_goal_ext(
                Profession::Forager,
                0,
                15.0,
                true, // nearby food ignored when grassland harvest applies
                false,
                false,
                true,
                0,
            ),
            Goal::Harvest
        );
        // Holding blocks harvest
        assert_eq!(
            pick_goal_ext(Profession::Forager, 1, 15.0, true, false, false, true, 0),
            Goal::Idle
        );
        // Hungry blocks harvest
        assert_eq!(
            pick_goal_ext(
                Profession::Forager,
                0,
                HUNGRY_FOOD,
                true,
                false,
                false,
                true,
                0
            ),
            Goal::SeekFood
        );
        // Non-grassland forager still uses nearby_food
        assert_eq!(
            pick_goal_with_biome(
                Profession::Forager,
                0,
                15.0,
                true,
                false,
                false,
                9 // ocean
            ),
            Goal::SeekFood
        );
    }

    #[test]
    fn farmer_and_smith_seek_profession_objects() {
        assert_eq!(
            pick_goal(Profession::Farmer, 0, 15.0, false),
            Goal::SeekObject(FARMER_TARGET_ID)
        );
        assert_eq!(
            pick_goal(Profession::Smith, 0, 15.0, true),
            Goal::SeekObject(SMITH_TARGET_ID)
        );
    }

    #[test]
    fn explorer_explores_when_fed_and_empty_hands() {
        assert_eq!(
            pick_goal(Profession::Explorer, 0, 15.0, true),
            Goal::Explore
        );
        assert_eq!(
            pick_goal(Profession::Explorer, 0, 10.0, false),
            Goal::Explore
        );
    }

    #[test]
    fn above_hungry_threshold_allows_profession() {
        let g = pick_goal(Profession::Explorer, 0, HUNGRY_FOOD + 0.01, false);
        assert_eq!(g, Goal::Explore);
        let g = pick_goal(Profession::Farmer, 0, HUNGRY_FOOD + 1.0, false);
        assert_eq!(g, Goal::SeekObject(FARMER_TARGET_ID));
    }

    #[test]
    fn pick_goal_ext_flees_when_threat_and_not_critical() {
        assert_eq!(
            pick_goal_ext(Profession::Forager, 0, 5.0, false, true, false, false, 0),
            Goal::Flee
        );
        assert_eq!(
            pick_goal_ext(Profession::Hunter, 0, 15.0, true, true, true, false, 0),
            Goal::Flee
        );
    }

    #[test]
    fn pick_goal_ext_critically_hungry_seeks_food_over_flee() {
        assert_eq!(
            pick_goal_ext(
                Profession::Forager,
                0,
                HUNGRY_FOOD / 2.0,
                false,
                true,
                false,
                false,
                0
            ),
            Goal::SeekFood
        );
        assert_eq!(
            pick_goal_ext(Profession::Forager, 0, 1.0, true, true, false, false, 0),
            Goal::SeekFood
        );
    }

    #[test]
    fn hunter_hunts_when_prey_adjacent_and_fed() {
        assert_eq!(
            pick_goal_ext(Profession::Hunter, 0, 15.0, false, false, true, false, 0),
            Goal::Hunt
        );
        // Hungry hunter seeks food, not hunt
        assert_eq!(
            pick_goal_ext(
                Profession::Hunter,
                0,
                HUNGRY_FOOD,
                false,
                false,
                true,
                false,
                0
            ),
            Goal::SeekFood
        );
        // Non-hunter ignores prey
        assert_eq!(
            pick_goal_ext(Profession::Farmer, 0, 15.0, false, false, true, false, 0),
            Goal::SeekObject(FARMER_TARGET_ID)
        );
        // Prey not adjacent → explore (not Hunt)
        assert_eq!(
            pick_goal_ext(Profession::Hunter, 0, 15.0, false, false, false, false, 0),
            Goal::Explore
        );
    }

    #[test]
    fn hunter_idle_when_holding_even_with_prey() {
        assert_eq!(
            pick_goal_ext(Profession::Hunter, 33, 15.0, false, false, true, false, 0),
            Goal::Idle
        );
    }

    /// Self-play contract: Hunter Flee/Hunt decisions use [`crate::AnimalWorld::nearby_threat`].
    #[test]
    fn hunter_uses_animal_nearby_threat_when_accessible() {
        use crate::animals::{AnimalKind, AnimalWorld, ANIMAL_THREAT_RANGE};
        use crate::hunt::HUNT_RANGE;

        let mut animals = AnimalWorld::new();
        let (px, py) = (10, 10);
        assert!(!animals.nearby_threat(px, py, ANIMAL_THREAT_RANGE));
        let threat = animals.nearby_threat(px, py, ANIMAL_THREAT_RANGE);
        let prey_adj = animals.nearby_prey(px, py, HUNT_RANGE);
        // No prey adjacent → explore
        assert_eq!(
            pick_goal_ext(Profession::Hunter, 0, 15.0, false, threat, prey_adj, false, 0),
            Goal::Explore
        );

        // Rabbit adjacent → Hunt
        animals.spawn(AnimalKind::Rabbit, px, py);
        let prey_adj = animals.nearby_prey(px, py, HUNT_RANGE);
        assert!(prey_adj);
        assert_eq!(
            pick_goal_ext(Profession::Hunter, 0, 15.0, false, false, prey_adj, false, 0),
            Goal::Hunt
        );

        // Wolf within threat range → flee (even if prey also near).
        animals.spawn(AnimalKind::Wolf, px + ANIMAL_THREAT_RANGE, py);
        let threat = animals.nearby_threat(px, py, ANIMAL_THREAT_RANGE);
        assert!(threat);
        assert_eq!(
            pick_goal_ext(Profession::Hunter, 0, 15.0, false, threat, true, false, 0),
            Goal::Flee
        );

        // Rabbit far (not adjacent) is not Hunt.
        animals.animals.clear();
        animals.spawn(AnimalKind::Rabbit, px + 3, py);
        let prey_adj = animals.nearby_prey(px, py, HUNT_RANGE);
        assert!(!prey_adj);
        assert_eq!(
            pick_goal_ext(Profession::Hunter, 0, 15.0, false, false, prey_adj, false, 0),
            Goal::Explore
        );
    }

    fn iron_smith_graph() -> ReverseCraftGraph {
        let mut g = ReverseCraftGraph::new();
        // iron 314 + hammer 441 → steel pick 684
        g.insert(SMITH_IRON_ID, 441, 684, 0);
        // iron 314 + chisel 455 → steel axe 334
        g.insert(SMITH_IRON_ID, 455, 334, 0);
        // charcoal 298 + forge 303 → hot iron 319 (also iron product chain)
        g.insert(298, 303, 319, 0);
        g
    }

    #[test]
    fn smith_product_targets_from_products_using_iron() {
        let g = iron_smith_graph();
        let targets = smith_product_targets(&g, SMITH_IRON_ID);
        assert!(targets.contains(&684), "got {targets:?}");
        assert!(targets.contains(&334), "got {targets:?}");
        // Always includes fallback SMITH_TARGET_ID when not already a product
        // 314 may appear as iron ingredient only — still in list if products_using misses it.
        assert!(!targets.is_empty());
        assert!(targets.len() <= SMITH_PRODUCT_CAP);
    }

    #[test]
    fn pick_smith_goal_prefers_iron_product_or_ingredient() {
        let g = iron_smith_graph();
        let empty = HashSet::new();
        let goal = pick_smith_goal(&g, &empty, SMITH_IRON_ID);
        // Should seek iron (314) as first missing ingredient for product 334 or 684.
        match goal {
            Goal::SeekObject(id) => {
                assert!(
                    id == SMITH_IRON_ID || id == 441 || id == 455 || id == 684 || id == 334,
                    "unexpected seek {id}"
                );
            }
            o => panic!("expected SeekObject, got {o:?}"),
        }
        // Have iron + hammer → path can produce 684; or still seek other product ingredients.
        let have: HashSet<i32> = [SMITH_IRON_ID, 441].into_iter().collect();
        let goal = pick_smith_goal(&g, &have, SMITH_IRON_ID);
        match goal {
            Goal::SeekObject(id) => {
                // 684 (product from iron+hammer), 455 (missing for axe), or 334 (axe product).
                assert!(
                    id == 684 || id == 455 || id == 334 || id == 441,
                    "unexpected seek {id}"
                );
            }
            o => panic!("expected SeekObject, got {o:?}"),
        }
        // Have iron + hammer + chisel → both products craftable; seek first missing product.
        let have_all: HashSet<i32> = [SMITH_IRON_ID, 441, 455].into_iter().collect();
        let goal = pick_smith_goal(&g, &have_all, SMITH_IRON_ID);
        match goal {
            Goal::SeekObject(id) => {
                assert!(id == 334 || id == 684, "expected a steel product, got {id}");
            }
            o => panic!("expected SeekObject, got {o:?}"),
        }
    }

    #[test]
    fn pick_goal_smith_craft_expands_products_using() {
        let g = iron_smith_graph();
        let empty = HashSet::new();
        let goal = pick_goal_smith_craft(
            Profession::Smith,
            0,
            15.0,
            false,
            false,
            false,
            false,
            &g,
            &empty,
            SMITH_IRON_ID,
        );
        assert!(matches!(goal, Goal::SeekObject(_)), "got {goal:?}");
        // Hungry still overrides
        assert_eq!(
            pick_goal_smith_craft(
                Profession::Smith,
                0,
                1.0,
                false,
                false,
                false,
                false,
                &g,
                &empty,
                SMITH_IRON_ID,
            ),
            Goal::SeekFood
        );
    }

    #[test]
    fn empty_craft_graph_smith_falls_back_to_default() {
        let g = ReverseCraftGraph::new();
        let empty = HashSet::new();
        assert_eq!(
            pick_smith_goal(&g, &empty, SMITH_IRON_ID),
            Goal::SeekObject(SMITH_IRON_ID)
        );
        let have: HashSet<i32> = [SMITH_IRON_ID].into_iter().collect();
        assert_eq!(
            pick_smith_goal(&g, &have, SMITH_IRON_ID),
            Goal::SeekObject(SMITH_TARGET_ID)
        );
    }
}
