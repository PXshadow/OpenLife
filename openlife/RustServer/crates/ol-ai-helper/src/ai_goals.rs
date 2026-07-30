//! Minimal AI profession goals for self-play (Haxe `AiBase` profession subset).
//!
//! Pure decision logic: survival first, then profession-biased goals. No world I/O.
//!
//! Full ordered ladder matching Haxe `doTimeStuffHelper` lives in
//! [`priority_ladder`] (chunk **AI-PRIO**). This module keeps the thin
//! self-play `pick_goal*` API used by `ol-server` selfplay/NPC loops.

use crate::professions::is_grassland;
use ol_ai_crafting::craft_graph::ReverseCraftGraph;
use std::collections::HashSet;

// Haxe: AiBase.doTimeStuffHelper priority ladder (AI-PRIO / priority_ladder)
// OL-AI-SPLIT: sibling file in ol-ai/src/
#[path = "priority_ladder.rs"]
pub mod priority_ladder;

pub use priority_ladder::{
    age_job_index, age_rotated_job_kind, age_rotated_job_sequence, apply_escape_to_sensors,
    baby_hungry_follow_tiles, check_is_hungry_and_eat_effects, child_with_mother_follow_tiles,
    compute_do_stuff, effective_do_stuff, escape_context_from_threats, escape_side_effects,
    escape_target_xy, fill_live_sensors, get_close_deadly_player, get_close_player_target,
    goal_from_rung, is_child_and_has_mother, is_child_and_has_mother_ex, is_deadly_player_candidate,
    is_hungry_simple, is_moving_to_player_needed, is_superbad_temp, ordered_follow_max_tiles,
    wounded_follow_tiles, pick_goal_from_ladder, pick_goal_from_live_sensors, pick_goal_with_sensors,
    player_quad_dist, resolve_escape_threat, resolve_priority_rung, sensors_from_ext,
    sensors_from_ext_ex, sensors_from_simple,
    should_attempt_escape, skip_escape_for_hunt, threat_is_far_for_temp, threat_quad_from_deadly,
    update_is_hungry, AgeRotatedJobKind, CloseDeadlyPlayer, ClosePlayerTarget, DeadlyPlayerCandidate,
    EscapeContext, EscapeSideEffects, EscapeThreat, HungryEatEffects, LiveSensorBundle,
    LiveSensorExtras, LiveSensorInput, PlayerTargetCandidate, PriorityBand, PriorityRung,
    PrioritySensors, BLUE_MASK_HOME_QUAD_MAX, DEADLY_PLAYER_ANGRY_ACTIVE,
    DEADLY_PLAYER_SEARCH_DIST, DEADLY_PLAYER_SEARCH_DIST_AI, DEVIL_MASK_ID, ESCAPE_ANGRY_TIME_IGNORE,
    ESCAPE_DID_NOT_REACH_FOOD_MAX, ESCAPE_DIST, ESCAPE_FOOD_CRIT_SKIP, ESCAPE_HUNT_MIN_AGE,
    ESCAPE_PLAYER_DIST_MAX, EXILE_HOME_QUAD_DANGER, GOBLIN_MASK_ID, HUNGRY_ENTER_FLOOR,
    HUNGRY_ENTER_FRAC, HUNGRY_LEAVE_FRAC, MAX_CHILD_AGE_BREASTFEED, MIN_AGE_TO_EAT,
    PLAYER_TARGET_SEARCH_DIST, SMITHING_HAMMER_ID,
};

/// Self-play / NPC profession role (subset of Haxe profession map keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Profession {
    Forager,
    Farmer,
    Smith,
    Explorer,
    /// Combat / prey-seeking role (Haxe hunt subset).
    Hunter,
    /// Baking / oven / pie role (Haxe `BAKER` / `doBaking`).
    Baker,
    /// Pottery / kiln / clay bowls (Haxe `POTTER` / `doPottery`).
    Potter,
    /// Shepherd / sheep herding (Haxe `SHEPHERD` / `isSheepHerding`).
    // Haxe: AI-SHEPHERD-MID Profession::Shepherd ladder goal
    Shepherd,
}

/// High-level goal chosen each AI decision tick.
///
/// AI-PRIO bands Follow/Feed/Craft/Job map onto these via [`goal_from_rung`]
/// (Follow→Explore path bias, Feed→SeekFood, Craft/Job→SeekObject / profession defaults)
/// so the action layer stays stable until AI-FOLLOW / AI-JOB bodies land.
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
        "BAKER" | "BAKE" => Some(Profession::Baker),
        "POTTER" | "POTTERY" => Some(Profession::Potter),
        "SHEPHERD" | "SHEEP" => Some(Profession::Shepherd),
        _ => None,
    }
}

/// Food at or below this value prioritizes [`Goal::SeekFood`].
pub const HUNGRY_FOOD: f32 = 8.0;

/// Default farmer craft preference (wheat-ish; preference only, not content-bound).
pub const FARMER_TARGET_ID: i32 = 242;

/// Default smith craft preference (iron-ish product; preference only).
pub const SMITH_TARGET_ID: i32 = 314;

/// Default baker craft preference (Cooked Carrot Pie; preference only).
///
/// Haxe age-rotated baking + thin self-play bias toward cooked pies.
pub const BAKER_TARGET_ID: i32 = 273;

/// Default potter craft preference (Clay Bowl; preference only).
///
/// Haxe age-rotated pottery + doPottery bias toward bowls/plates.
pub const POTTER_TARGET_ID: i32 = 235;

/// Default shepherd craft preference (Domestic Sheep; preference only).
///
/// Haxe age-rotated sheep herding + isSheepHerding bias toward domestic sheep.
// Haxe: AI-SHEPHERD-MID
pub const SHEPHERD_TARGET_ID: i32 = 575;

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
/// 3. Profession defaults (forager seeks nearby food, farmer/smith/baker seek objects, explorer wanders)
///
/// For the full Haxe `doTimeStuffHelper` rung order see
/// [`priority_ladder::resolve_priority_rung`] / [`priority_ladder::pick_goal_from_ladder`].
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
/// 6. Profession defaults (forager food/explore, farmer/smith/baker objects, …)
///
/// Note: this is the **thin self-play** ladder. The full AiBase skeleton is
/// [`priority_ladder`].
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
        Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID),
        Profession::Potter => Goal::SeekObject(POTTER_TARGET_ID),
        Profession::Shepherd => Goal::SeekObject(SHEPHERD_TARGET_ID),
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

/// Craft-aware goal pick: smith iron products, farmer pipeline, baker pie/oven pipeline.
///
/// Smith uses stage `0.0` — prefer [`pick_goal_smith_craft_at_stage`] when
/// [`crate::SmithProfessionRuntime::stage`] is sticky.
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
    pick_goal_smith_craft_at_stage(
        profession,
        held_id,
        food,
        nearby_food,
        threat_near,
        prey_adjacent,
        on_grassland,
        graph,
        have,
        iron_id,
        0.0,
    )
}

/// Like [`pick_goal_smith_craft`] with smith profession stage (Haxe `profession['SMITH']`).
// Haxe: doSmithing stage ladder + pick_smith_profession_goal
pub fn pick_goal_smith_craft_at_stage(
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
    smith_stage: f32,
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
        // AI-JOB-SMITH: stage-aware pipeline then iron reverse-craft fallback
        let _ = iron_id;
        return crate::smith_profession::pick_smith_profession_goal(graph, have, smith_stage);
    }
    // Also expand smith when base would Explore/Idle with empty hands and fed.
    if profession == Profession::Smith
        && held_id == 0
        && food > HUNGRY_FOOD
        && matches!(g, Goal::Explore | Goal::Idle | Goal::SeekObject(_))
        && !threat_near
    {
        return crate::smith_profession::pick_smith_profession_goal(graph, have, smith_stage);
    }
    // Farmer: expand toward crop pipeline intermediates via reverse craft (AI-JOB-FARM).
    if profession == Profession::Farmer
        && held_id == 0
        && food > HUNGRY_FOOD
        && matches!(g, Goal::SeekObject(_) | Goal::Explore | Goal::Idle)
        && !threat_near
    {
        return crate::farmer_profession::pick_farmer_goal(graph, have);
    }
    // Baker: oven / pie / bread pipeline (AI-JOB-BAKER); stage inferred from inventory.
    if profession == Profession::Baker
        && held_id == 0
        && food > HUNGRY_FOOD
        && matches!(g, Goal::SeekObject(_) | Goal::Explore | Goal::Idle)
        && !threat_near
    {
        let stage = crate::baker_profession::infer_baker_pipeline_stage(have);
        return crate::baker_profession::pick_baker_goal(graph, have, stage);
    }
    // Potter: clay bowl/plate pipeline (AI-POTTER).
    if profession == Profession::Potter
        && held_id == 0
        && food > HUNGRY_FOOD
        && matches!(g, Goal::SeekObject(_) | Goal::Explore | Goal::Idle)
        && !threat_near
    {
        return crate::pottery_profession::pick_potter_goal(graph, have);
    }
    // Shepherd: sheep / lamb / milk pipeline (AI-SHEPHERD-MID).
    if profession == Profession::Shepherd
        && held_id == 0
        && food > HUNGRY_FOOD
        && matches!(g, Goal::SeekObject(_) | Goal::Explore | Goal::Idle)
        && !threat_near
    {
        return crate::shepherd_profession::pick_shepherd_goal(graph, have);
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
        assert_eq!(parse_profession_token("baker"), Some(Profession::Baker));
        assert_eq!(parse_profession_token("BAKE"), Some(Profession::Baker));
        assert_eq!(parse_profession_token("potter"), Some(Profession::Potter));
        assert_eq!(parse_profession_token("shepherd"), Some(Profession::Shepherd));
        assert_eq!(parse_profession_token("SHEEP"), Some(Profession::Shepherd));
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
            Profession::Baker,
            Profession::Potter,
            Profession::Shepherd,
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
                true,
                false,
                false,
                true,
                0,
            ),
            Goal::Harvest
        );
        assert_eq!(
            pick_goal_ext(Profession::Forager, 1, 15.0, true, false, false, true, 0),
            Goal::Idle
        );
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
        assert_eq!(
            pick_goal_with_biome(
                Profession::Forager,
                0,
                15.0,
                true,
                false,
                false,
                9
            ),
            Goal::SeekFood
        );
    }

    #[test]
    fn farmer_smith_baker_seek_profession_objects() {
        assert_eq!(
            pick_goal(Profession::Farmer, 0, 15.0, false),
            Goal::SeekObject(FARMER_TARGET_ID)
        );
        assert_eq!(
            pick_goal(Profession::Smith, 0, 15.0, true),
            Goal::SeekObject(SMITH_TARGET_ID)
        );
        assert_eq!(
            pick_goal(Profession::Baker, 0, 15.0, false),
            Goal::SeekObject(BAKER_TARGET_ID)
        );
        assert_eq!(
            pick_goal(Profession::Shepherd, 0, 15.0, false),
            Goal::SeekObject(SHEPHERD_TARGET_ID)
        );
        assert_eq!(
            pick_goal(Profession::Potter, 0, 15.0, false),
            Goal::SeekObject(POTTER_TARGET_ID)
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
        assert_eq!(
            pick_goal_ext(Profession::Farmer, 0, 15.0, false, false, true, false, 0),
            Goal::SeekObject(FARMER_TARGET_ID)
        );
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

    /// Prey/threat flags are sensors from the adapter (animals live in ol-sim).
    #[test]
    fn hunter_uses_threat_and_prey_sensor_flags() {
        assert_eq!(
            pick_goal_ext(Profession::Hunter, 0, 15.0, false, false, false, false, 0),
            Goal::Explore
        );
        assert_eq!(
            pick_goal_ext(Profession::Hunter, 0, 15.0, false, false, true, false, 0),
            Goal::Hunt
        );
        assert_eq!(
            pick_goal_ext(Profession::Hunter, 0, 15.0, false, true, true, false, 0),
            Goal::Flee
        );
    }

    fn iron_smith_graph() -> ReverseCraftGraph {
        let mut g = ReverseCraftGraph::new();
        g.insert(SMITH_IRON_ID, 441, 684, 0);
        g.insert(SMITH_IRON_ID, 455, 334, 0);
        g.insert(298, 303, 319, 0);
        g
    }

    #[test]
    fn smith_product_targets_from_products_using_iron() {
        let g = iron_smith_graph();
        let targets = smith_product_targets(&g, SMITH_IRON_ID);
        assert!(targets.contains(&684), "got {targets:?}");
        assert!(targets.contains(&334), "got {targets:?}");
        assert!(!targets.is_empty());
        assert!(targets.len() <= SMITH_PRODUCT_CAP);
    }

    #[test]
    fn pick_smith_goal_prefers_iron_product_or_ingredient() {
        let g = iron_smith_graph();
        let empty = HashSet::new();
        let goal = pick_smith_goal(&g, &empty, SMITH_IRON_ID);
        match goal {
            Goal::SeekObject(id) => {
                assert!(
                    id == SMITH_IRON_ID || id == 441 || id == 455 || id == 684 || id == 334,
                    "unexpected seek {id}"
                );
            }
            o => panic!("expected SeekObject, got {o:?}"),
        }
        let have: HashSet<i32> = [SMITH_IRON_ID, 441].into_iter().collect();
        let goal = pick_smith_goal(&g, &have, SMITH_IRON_ID);
        match goal {
            Goal::SeekObject(id) => {
                assert!(
                    id == 684 || id == 455 || id == 334 || id == 441,
                    "unexpected seek {id}"
                );
            }
            o => panic!("expected SeekObject, got {o:?}"),
        }
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
        // Stage-0 pipeline prefers iron ore before reverse-craft products.
        assert!(matches!(goal, Goal::SeekObject(_)), "got {goal:?}");
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
        // Stage 5+ seeks mining pick path (not iron ore 290).
        let stage5 = pick_goal_smith_craft_at_stage(
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
            5.0,
        );
        match stage5 {
            Goal::SeekObject(id) => {
                assert_ne!(id, 290, "stage5+ must not seek iron ore");
                // 684 pick, or reverse-craft ingredients (314 iron / 441 hammer)
                assert!(
                    id == 684 || id == 314 || id == 441,
                    "stage5 expected pick path, got {id}"
                );
            }
            o => panic!("expected SeekObject, got {o:?}"),
        }
    }

    #[test]
    fn pick_goal_smith_craft_expands_baker_pipeline() {
        let g = ReverseCraftGraph::new();
        let empty = HashSet::new();
        let goal = pick_goal_smith_craft(
            Profession::Baker,
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
        // Empty inventory → stage 0 wants clay plate first
        assert_eq!(goal, Goal::SeekObject(236)); // CLAY_PLATE
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
