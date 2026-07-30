//! Craft-aware profession goal expansion (used to live in `ai_goals`).
//!
//! Lives here so `ol-ai-helper` does not depend on profession crates.

use ol_ai_crafting::craft_graph::ReverseCraftGraph;
use ol_ai_helper::ai_goals::{pick_goal_ext, Goal, Profession, HUNGRY_FOOD};
use std::collections::HashSet;

use crate::baker_profession::{infer_baker_pipeline_stage, pick_baker_goal};
use crate::farmer_profession::pick_farmer_goal;
use crate::pottery_profession::pick_potter_goal;
use crate::shepherd_profession::pick_shepherd_goal;
use crate::smith_profession::pick_smith_profession_goal;

/// Craft-aware goal pick: smith iron products, farmer pipeline, baker pie/oven pipeline.
///
/// Smith uses stage `0.0` — prefer [`pick_goal_smith_craft_at_stage`] when stage is sticky.
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
        let _ = iron_id;
        return pick_smith_profession_goal(graph, have, smith_stage);
    }
    if profession == Profession::Smith
        && held_id == 0
        && food > HUNGRY_FOOD
        && matches!(g, Goal::Explore | Goal::Idle | Goal::SeekObject(_))
        && !threat_near
    {
        return pick_smith_profession_goal(graph, have, smith_stage);
    }
    if profession == Profession::Farmer
        && held_id == 0
        && food > HUNGRY_FOOD
        && matches!(g, Goal::SeekObject(_) | Goal::Explore | Goal::Idle)
        && !threat_near
    {
        return pick_farmer_goal(graph, have);
    }
    if profession == Profession::Baker
        && held_id == 0
        && food > HUNGRY_FOOD
        && matches!(g, Goal::SeekObject(_) | Goal::Explore | Goal::Idle)
        && !threat_near
    {
        let stage = infer_baker_pipeline_stage(have);
        return pick_baker_goal(graph, have, stage);
    }
    if profession == Profession::Potter
        && held_id == 0
        && food > HUNGRY_FOOD
        && matches!(g, Goal::SeekObject(_) | Goal::Explore | Goal::Idle)
        && !threat_near
    {
        return pick_potter_goal(graph, have);
    }
    if profession == Profession::Shepherd
        && held_id == 0
        && food > HUNGRY_FOOD
        && matches!(g, Goal::SeekObject(_) | Goal::Explore | Goal::Idle)
        && !threat_near
    {
        return pick_shepherd_goal(graph, have);
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_ai_helper::ai_goals::SMITH_IRON_ID;

    fn iron_smith_graph() -> ReverseCraftGraph {
        let mut g = ReverseCraftGraph::new();
        g.insert(314, 0, 441, 0);
        g.insert(441, 0, 455, 0);
        g.insert(314, 0, 684, 0);
        g
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
        assert_eq!(goal, Goal::SeekObject(236)); // CLAY_PLATE
    }
}
