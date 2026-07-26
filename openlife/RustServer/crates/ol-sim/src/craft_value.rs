//! Bottom-up craft valuation for AI / self-play (time cost vs object value).
//!
//! Step 1: given nearby objects, score each craftable transition by
//! movement time to actor + to target + interaction time, minus value of
//! spent inputs, plus value of products (hunger / profession weighted).
//! Step 2: scan radius for craftable pairs (default 50 tiles).

use ol_content::{ContentDb, ObjectDef};
use std::collections::HashMap;

/// Default interaction time (seconds) for a USE / craft step.
pub const INTERACTION_SEC: f32 = 0.5;
/// Default search radius for craft planning (tiles).
pub const DEFAULT_CRAFT_RADIUS: i32 = 50;
/// Default walk speed (tiles/s) when player speed unknown — matches WALK_MOVE_SPEED.
pub const DEFAULT_WALK_SPEED: f32 = 3.75;
/// Counts above this for a non-food object drive its scarcity value near zero.
pub const ABUNDANCE_SOFT_CAP: u32 = 5;

/// AI profession bias for valuation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CraftProfession {
    Forager,
    Farmer,
    Hunter,
    Smith,
    Explorer,
    Generic,
}

/// One nearby ground object (or held treated as at player).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NearbyObj {
    pub id: i32,
    pub x: i32,
    pub y: i32,
}

/// A scored craft opportunity (actor on target → products).
#[derive(Debug, Clone)]
pub struct CraftOption {
    pub actor_id: i32,
    pub target_id: i32,
    pub new_actor_id: i32,
    pub new_target_id: i32,
    pub actor_x: i32,
    pub actor_y: i32,
    pub target_x: i32,
    pub target_y: i32,
    /// Estimated time (seconds) from player position.
    pub time_cost_sec: f32,
    /// Net score: product values − spent inputs − time_penalty.
    pub net_score: f32,
    pub product_value: f32,
    pub input_value: f32,
}

/// Chebyshev distance (Haxe-ish tile distance for planning).
pub fn tile_dist(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs().max((ay - by).abs())
}

/// Movement time in seconds at constant speed (tiles/s).
pub fn move_time_sec(ax: i32, ay: i32, bx: i32, by: i32, speed: f32) -> f32 {
    let d = tile_dist(ax, ay, bx, by) as f32;
    let sp = if speed > 0.05 { speed } else { DEFAULT_WALK_SPEED };
    d / sp
}

/// Time to walk to A, interact, walk to B, interact (or same tile once).
///
/// If the actor is already at the player (held / bare hands), only walk to the
/// target once + one interaction (Haxe USE with empty/held actor).
pub fn craft_time_cost_sec(
    px: i32,
    py: i32,
    actor_xy: (i32, i32),
    target_xy: (i32, i32),
    speed: f32,
    interaction_sec: f32,
) -> f32 {
    let (ax, ay) = actor_xy;
    let (tx, ty) = target_xy;
    let actor_at_player = ax == px && ay == py;
    if actor_at_player || (ax == tx && ay == ty) {
        return move_time_sec(px, py, tx, ty, speed) + interaction_sec;
    }
    // Path: player → actor tile → pick → target tile → use.
    let to_actor = move_time_sec(px, py, ax, ay, speed);
    let actor_to_target = move_time_sec(ax, ay, tx, ty, speed);
    to_actor + actor_to_target + interaction_sec * 2.0
}

/// Scarcity multiplier: high when count is 0, near 0 when count ≥ soft_cap.
pub fn scarcity_mult(count: u32, soft_cap: u32) -> f32 {
    let cap = soft_cap.max(1) as f32;
    if count == 0 {
        return 1.5;
    }
    if count as f32 >= cap {
        return 0.05;
    }
    1.0 - (count as f32 / cap) * 0.95
}

/// Base tool-ish score from name tags.
fn toolish_bonus(def: &ObjectDef) -> f32 {
    let n = def.name.to_ascii_lowercase();
    let d = def.description.to_ascii_lowercase();
    let mut b = 0.0;
    if n.contains("knife")
        || n.contains("axe")
        || n.contains("hoe")
        || n.contains("bow")
        || n.contains("spear")
        || n.contains("hammer")
        || d.contains("+tool")
    {
        b += 8.0;
    }
    if n.contains("basket") || n.contains("bowl") || n.contains("pot") {
        b += 3.0;
    }
    b
}

/// Value of one object for an agent state.
pub fn object_value(
    content: &ContentDb,
    id: i32,
    profession: CraftProfession,
    hungry: bool,
    food_need: f32,
    count_nearby: u32,
    soft_cap: u32,
) -> f32 {
    if id <= 0 {
        return 0.0;
    }
    let Some(def) = content.get(id) else {
        return 0.5 * scarcity_mult(count_nearby, soft_cap);
    };
    let scarcity = scarcity_mult(count_nearby, soft_cap);
    let mut v = 1.0 + toolish_bonus(def);

    // Food: Haxe-like foodValue, self counts double when hungry.
    if def.food_value > 0 {
        let mut food = def.food_value as f32;
        if hungry {
            food *= 1.0 + food_need.clamp(0.0, 2.0);
            food *= 2.0; // self double
        }
        // Profession: forager likes wild food more.
        match profession {
            CraftProfession::Forager => food *= 1.3,
            CraftProfession::Farmer => food *= 1.15,
            CraftProfession::Hunter => food *= 0.9,
            _ => {}
        }
        v += food;
    } else {
        // Non-food tools/materials priority early game.
        match profession {
            CraftProfession::Smith => {
                if def.name.to_ascii_lowercase().contains("stone")
                    || def.name.to_ascii_lowercase().contains("iron")
                {
                    v *= 1.4;
                }
            }
            CraftProfession::Farmer => {
                if def.name.to_ascii_lowercase().contains("seed")
                    || def.name.to_ascii_lowercase().contains("wheat")
                    || def.name.to_ascii_lowercase().contains("soil")
                {
                    v *= 1.35;
                }
            }
            CraftProfession::Hunter => {
                if def.name.to_ascii_lowercase().contains("bow")
                    || def.name.to_ascii_lowercase().contains("arrow")
                    || def.name.to_ascii_lowercase().contains("spear")
                {
                    v *= 1.4;
                }
            }
            CraftProfession::Forager => {
                if def.food_value == 0 {
                    v *= 1.05;
                }
            }
            _ => {}
        }
    }

    v * scarcity
}

/// Food contribution for nearby allies (each ally full weight; self already doubled in object_value).
pub fn ally_food_bonus(food_value: i32, ally_count_in_radius: u32) -> f32 {
    if food_value <= 0 {
        return 0.0;
    }
    food_value as f32 * ally_count_in_radius as f32
}

/// Count how many of each object id appear nearby.
pub fn count_ids(nearby: &[NearbyObj]) -> HashMap<i32, u32> {
    let mut m = HashMap::new();
    for o in nearby {
        *m.entry(o.id).or_insert(0) += 1;
    }
    m
}

/// Enumerate craft options from held + nearby objects (bottom-up).
///
/// For each (actor, target) pair where a transition exists and both are
/// available within radius (held counts as at player), score time vs value.
pub fn evaluate_nearby_crafts(
    content: &ContentDb,
    px: i32,
    py: i32,
    held_id: i32,
    nearby: &[NearbyObj],
    profession: CraftProfession,
    hungry: bool,
    food_need: f32,
    ally_count: u32,
    speed: f32,
    interaction_sec: f32,
    radius: i32,
) -> Vec<CraftOption> {
    let counts = count_ids(nearby);
    let mut options = Vec::new();

    // Candidate actors: empty hands (0) at player, held item, nearby ground objects.
    let mut actors: Vec<(i32, i32, i32)> = Vec::new(); // id, x, y
    actors.push((0, px, py)); // bare hands — Haxe actor 0
    if held_id != 0 {
        actors.push((held_id, px, py));
    }
    for o in nearby {
        if tile_dist(px, py, o.x, o.y) <= radius {
            actors.push((o.id, o.x, o.y));
        }
    }

    // Targets: nearby tiles (and empty target 0 for self-craft held,0).
    let mut targets: Vec<(i32, i32, i32)> = nearby
        .iter()
        .filter(|o| tile_dist(px, py, o.x, o.y) <= radius)
        .map(|o| (o.id, o.x, o.y))
        .collect();
    // Empty-ground craft: target 0 at player for (held, 0) transitions.
    targets.push((0, px, py));

    let mut seen = std::collections::HashSet::new();

    for &(aid, ax, ay) in &actors {
        for &(tid, tx, ty) in &targets {
            // Avoid using same physical instance as both when held is also on ground.
            if aid == held_id && ax == px && ay == py && tid == held_id && tx == px && ty == py {
                continue;
            }
            let key = (aid, tid, ax, ay, tx, ty);
            if !seen.insert(key) {
                continue;
            }
            let Some(tr) = content.find_transition(aid, tid) else {
                continue;
            };
            let time = craft_time_cost_sec(px, py, (ax, ay), (tx, ty), speed, interaction_sec);
            let cnt_a = if aid == held_id {
                counts.get(&aid).copied().unwrap_or(0).saturating_add(1)
            } else {
                counts.get(&aid).copied().unwrap_or(0)
            };
            let cnt_t = counts.get(&tid).copied().unwrap_or(0);
            let cnt_na = counts.get(&tr.new_actor_id).copied().unwrap_or(0);
            let cnt_nt = counts.get(&tr.new_target_id).copied().unwrap_or(0);

            let mut in_v = object_value(
                content,
                aid,
                profession,
                hungry,
                food_need,
                cnt_a.saturating_sub(1),
                ABUNDANCE_SOFT_CAP,
            );
            if tid != 0 {
                // Target consumed only if new_target differs and isn't permanent ground.
                if tr.new_target_id != tid {
                    in_v += object_value(
                        content,
                        tid,
                        profession,
                        hungry,
                        food_need,
                        cnt_t.saturating_sub(1),
                        ABUNDANCE_SOFT_CAP,
                    ) * 0.5; // partial — often target transforms not destroyed
                }
            }

            let mut out_v = object_value(
                content,
                tr.new_actor_id,
                profession,
                hungry,
                food_need,
                cnt_na,
                ABUNDANCE_SOFT_CAP,
            );
            out_v += object_value(
                content,
                tr.new_target_id,
                profession,
                hungry,
                food_need,
                cnt_nt,
                ABUNDANCE_SOFT_CAP,
            );
            // Ally food bonus on edible products.
            if let Some(def) = content.get(tr.new_actor_id) {
                out_v += ally_food_bonus(def.food_value, ally_count);
            }
            if let Some(def) = content.get(tr.new_target_id) {
                out_v += ally_food_bonus(def.food_value, ally_count);
            }

            // Time penalty: 1 score unit ≈ 2 seconds of work.
            let time_penalty = time * 0.5;
            let net = out_v - in_v - time_penalty;
            options.push(CraftOption {
                actor_id: aid,
                target_id: tid,
                new_actor_id: tr.new_actor_id,
                new_target_id: tr.new_target_id,
                actor_x: ax,
                actor_y: ay,
                target_x: tx,
                target_y: ty,
                time_cost_sec: time,
                net_score: net,
                product_value: out_v,
                input_value: in_v,
            });
        }
    }

    options.sort_by(|a, b| {
        b.net_score
            .partial_cmp(&a.net_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    options
}

/// Best craft option if any with positive score.
pub fn best_craft(options: &[CraftOption]) -> Option<&CraftOption> {
    options.iter().find(|o| o.net_score > 0.1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::{ObjectDef, Transition};

    fn db_with_food_and_branch() -> ContentDb {
        let mut db = ContentDb::default();
        db.objects.insert(
            64,
            ObjectDef {
                id: 64,
                name: "Straight Branch".into(),
                description: "Straight Branch".into(),
                food_value: 0,
                ..ObjectDef::empty(64)
            },
        );
        db.objects.insert(
            404,
            ObjectDef {
                id: 404,
                name: "Wild Carrot".into(),
                description: "Wild Carrot".into(),
                food_value: 5,
                ..ObjectDef::empty(404)
            },
        );
        db.objects.insert(
            36,
            ObjectDef {
                id: 36,
                name: "Seeding Wild Carrot".into(),
                description: "Seeding Wild Carrot".into(),
                food_value: 0,
                ..ObjectDef::empty(36)
            },
        );
        // bare hand + seeding carrot → seed head + wild carrot (from real 0_36.txt)
        db.transitions.insert(
            (0, 36),
            Transition {
                actor_id: 0,
                target_id: 36,
                new_actor_id: 395,
                new_target_id: 404,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,

            desired_move_dist: 0,
            },
        );
        db.objects.insert(
            395,
            ObjectDef {
                id: 395,
                name: "Carrot Seed Head".into(),
                description: "Carrot Seed Head".into(),
                food_value: 0,
                ..ObjectDef::empty(395)
            },
        );
        db
    }

    #[test]
    fn scarcity_high_when_missing() {
        assert!(scarcity_mult(0, 5) > scarcity_mult(3, 5));
        assert!(scarcity_mult(5, 5) < 0.1);
    }

    #[test]
    fn hungry_doubles_food_priority() {
        let db = db_with_food_and_branch();
        let full = object_value(&db, 404, CraftProfession::Forager, false, 0.0, 0, 5);
        let hungry = object_value(&db, 404, CraftProfession::Forager, true, 1.0, 0, 5);
        assert!(hungry > full * 1.5);
    }

    #[test]
    fn craft_time_includes_interaction() {
        // Same tile / held at player → one approach + one interaction.
        let t = craft_time_cost_sec(0, 0, (0, 0), (10, 0), 3.75, 0.5);
        assert!((t - (10.0 / 3.75 + 0.5)).abs() < 0.01);
        // Distinct actor tile then target: two legs + two interactions.
        let t2 = craft_time_cost_sec(0, 0, (5, 0), (10, 0), 3.75, 0.5);
        let expect = 5.0 / 3.75 + 5.0 / 3.75 + 1.0;
        assert!((t2 - expect).abs() < 0.01);
    }

    #[test]
    fn evaluate_finds_carrot_harvest() {
        let db = db_with_food_and_branch();
        let nearby = vec![NearbyObj {
            id: 36,
            x: 5,
            y: 0,
        }];
        let opts = evaluate_nearby_crafts(
            &db,
            0,
            0,
            0, // empty hands
            &nearby,
            CraftProfession::Forager,
            true,
            1.0,
            0,
            3.75,
            0.5,
            50,
        );
        assert!(!opts.is_empty());
        let best = best_craft(&opts).expect("positive craft");
        assert_eq!(best.actor_id, 0);
        assert_eq!(best.target_id, 36);
        assert_eq!(best.new_target_id, 404);
        assert!(best.net_score > 0.0);
    }

    #[test]
    fn abundance_reduces_value() {
        let db = db_with_food_and_branch();
        let rare = object_value(&db, 64, CraftProfession::Generic, false, 0.0, 0, 5);
        let many = object_value(&db, 64, CraftProfession::Generic, false, 0.0, 8, 5);
        assert!(rare > many * 5.0);
    }
}
