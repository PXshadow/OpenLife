//! In-process self-play agent: exercises MOVE / USE / DROP / eat against the live sim.
//!
//! Priority each tick: stay alive → free hands → walk (always visible motion) → use nearby.
//! Target preference is biased by [`ol_sim::pick_goal_ext`] / [`ol_sim::pick_goal_smith_craft`]
//! (profession + threat/prey/biome).
//! - Forager on grassland → [`Goal::Harvest`] → `SAY HARVEST`
//! - Hunter with prey adjacent → [`Goal::Hunt`] → `SAY HUNT`
//! - Smith expands wants via craft graph `products_using(iron)`
//! Farmer/Smith SeekObject goals consult a reverse craft graph for intermediate ingredients
//! and keep held craft ingredients instead of dropping (USE partner when found).
//! Threat/prey prefer live [`ol_sim::AnimalWorld`] via Arc share (same pattern as craft graph).
//!
//! Agents: Forager on [`SELFPLAY_CONN_ID`], Farmer on +1, Hunter on +2 (optional triple).

use ol_content::ContentDb;
use ol_metrics::Counters;
use ol_net::NetIntent;
use ol_sim::{
    apply_job_flags_to_live_input, consider_animals_for_goto, force_drop_at_feet,
    infer_baker_pipeline_stage, infer_smith_stage_from_have, is_grassland, is_walkable, next_step,
    next_step_consider_animals, pick_baker_goal, pick_farmer_goal, pick_goal_ext,
    pick_goal_from_live_sensors, pick_smith_profession_goal, scan_world_radius,
    self_clothing_raw_payload, smart_drop_held_from_sensors, update_is_hungry, AnimalWorld,
    CloseDeadlyAnimal, DropHeldSensorExtras, FarmProfession, Goal, LiveSensorInput, PlayerSnapshot,
    Profession, ProfessionStickySnapshot, ReverseCraftGraph, ShortCraftLiveIntent,
    ANIMAL_THREAT_RANGE, BAKER_TARGET_ID, DEADLY_ANIMAL_SEARCH_DIST, FARMER_TARGET_ID, HUNT_RANGE,
    MIN_AGE_TO_EAT, SMITH_IRON_ID, SMITH_TARGET_ID,
};
use ol_world::World;
use rand::{Rng, SeedableRng};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::info;

/// Reserved conn id for the primary self-play agent (high so it won't clash with TCP).
pub const SELFPLAY_CONN_ID: u64 = 9_000_001;

/// Second agent conn id (Farmer by default when dual-spawned).
pub const SELFPLAY_CONN_ID_2: u64 = SELFPLAY_CONN_ID + 1;

/// Third agent conn id (Hunter when triple-spawned).
pub const SELFPLAY_CONN_ID_3: u64 = SELFPLAY_CONN_ID + 2;

/// Identity for one self-play agent loop.
#[derive(Debug, Clone)]
pub struct SelfplayAgent {
    pub conn_id: u64,
    pub profession: Profession,
    pub email: String,
    pub client_tag: String,
    pub label: String,
}

impl SelfplayAgent {
    pub fn forager() -> Self {
        Self {
            conn_id: SELFPLAY_CONN_ID,
            profession: Profession::Forager,
            email: "selfplay@local".into(),
            client_tag: "client_selfplay".into(),
            label: "forager".into(),
        }
    }

    pub fn farmer() -> Self {
        Self {
            conn_id: SELFPLAY_CONN_ID_2,
            profession: Profession::Farmer,
            email: "selfplay-farmer@local".into(),
            client_tag: "client_selfplay_farmer".into(),
            label: "farmer".into(),
        }
    }

    pub fn hunter() -> Self {
        Self {
            conn_id: SELFPLAY_CONN_ID_3,
            profession: Profession::Hunter,
            email: "selfplay-hunter@local".into(),
            client_tag: "client_selfplay_hunter".into(),
            label: "hunter".into(),
        }
    }
}

/// Map selfplay profession role → sticky snapshot for ladder job sensors.
// Haxe: assignedProfession / lastProfession for farm/smith/baker
fn selfplay_sticky_for_profession(profession: Profession, age: f32) -> ProfessionStickySnapshot {
    match profession {
        Profession::Farmer => ProfessionStickySnapshot {
            farm_assigned: Some(FarmProfession::BasicFarmer),
            farm_last: Some(FarmProfession::BasicFarmer),
            age,
            ..Default::default()
        },
        Profession::Smith => ProfessionStickySnapshot {
            smith_assigned: true,
            smith_last: true,
            age,
            ..Default::default()
        },
        Profession::Baker => ProfessionStickySnapshot {
            baker_assigned: true,
            baker_last: true,
            age,
            ..Default::default()
        },
        Profession::Shepherd => ProfessionStickySnapshot {
            shepherd_assigned: true,
            shepherd_last: true,
            age,
            ..Default::default()
        },
        // Forager/Hunter/Explorer/Potter: age-rotated or dedicated sticky elsewhere.
        _ => ProfessionStickySnapshot {
            age,
            ..Default::default()
        },
    }
}

pub async fn run_selfplay_agent(
    agent: SelfplayAgent,
    intent_tx: tokio::sync::mpsc::Sender<NetIntent>,
    world: Arc<RwLock<World>>,
    content: Arc<ContentDb>,
    log: Arc<RwLock<Vec<String>>>,
    pos: Arc<RwLock<(i32, i32)>>,
    player_views: Arc<RwLock<HashMap<u64, PlayerSnapshot>>>,
    craft_graph: Arc<ReverseCraftGraph>,
    // Live animal world mirror published by sim (read-only here).
    animals: Arc<RwLock<AnimalWorld>>,
    // Optional process counters (unstick gate metric).
    counters: Option<Arc<Counters>>,
) {
    // Stagger agents slightly so logins don't race the same tick.
    let startup_ms = match agent.conn_id {
        SELFPLAY_CONN_ID => 400,
        SELFPLAY_CONN_ID_2 => 700,
        _ => 1000,
    };
    tokio::time::sleep(Duration::from_millis(startup_ms)).await;

    let conn_id = agent.conn_id;
    let profession = agent.profession;
    let mut rng = rand::rngs::StdRng::from_entropy();
    let mut x = 0i32;
    let mut y = 0i32;
    let mut last_use: Option<(i32, i32, u64)> = None; // tile + tick, avoid spam
    let mut stuck_use_count = 0u32;
    let mut tick: u64 = 0;
    let mut saw_alive = false;
    // Haxe `AiBase.isHungry` hysteresis across ticks (AI-PRIO-LIVE).
    let mut was_hungry = false;
    // After successful USE, skip act for waitingTime analog (ms).
    let mut post_use_wait_until: Option<std::time::Instant> = None;

    login_and_spawn(
        &agent,
        &intent_tx,
        &world,
        &log,
        &pos,
        &mut x,
        &mut y,
        &mut rng,
    )
    .await;

    loop {
        tokio::time::sleep(Duration::from_millis(400)).await;
        tick += 1;

        // Prefer sim truth for position / held / death.
        let snap = player_views
            .read()
            .ok()
            .and_then(|g| g.get(&conn_id).cloned());

        if let Some(ref p) = snap {
            if p.deleted {
                push_log(
                    &log,
                    &format!("[{}] t{tick} died — respawn", agent.label),
                );
                login_and_spawn(
                    &agent,
                    &intent_tx,
                    &world,
                    &log,
                    &pos,
                    &mut x,
                    &mut y,
                    &mut rng,
                )
                .await;
                last_use = None;
                stuck_use_count = 0;
                saw_alive = false;
                continue;
            }
            saw_alive = true;
            x = p.x;
            y = p.y;
            *pos.write().unwrap() = (x, y);
        } else if saw_alive {
            push_log(
                &log,
                &format!("[{}] t{tick} lost player snapshot — respawn", agent.label),
            );
            login_and_spawn(
                &agent,
                &intent_tx,
                &world,
                &log,
                &pos,
                &mut x,
                &mut y,
                &mut rng,
            )
            .await;
            last_use = None;
            stuck_use_count = 0;
            saw_alive = false;
            continue;
        }

        let held = snap.as_ref().map(|p| p.held_id).unwrap_or(0);
        let food = snap.as_ref().map(|p| p.food).unwrap_or(10.0);
        let food_max = snap.as_ref().map(|p| p.food_max).unwrap_or(20.0);
        let age = snap.as_ref().map(|p| p.age).unwrap_or(20.0);
        let heat = snap.as_ref().map(|p| p.heat).unwrap_or(0.5);
        // held_by > 0 ⇒ being carried; mother presence via held_by when young (partial).
        let held_by = snap.as_ref().map(|p| p.held_by).unwrap_or(0);
        let has_mother = held_by > 0 && age < MIN_AGE_TO_EAT;
        let nearby_food = has_nearby_food(&world, &content, x, y);
        // Prefer live AnimalWorld GetCloseDeadlyAnimal (moves²); fall back to Chebyshev + map.
        let deadly = sense_deadly_animal(&animals, x, y);
        let (animal_threat_cheby, animal_prey_near, animal_flee_away) =
            sense_animals(&animals, x, y);
        let animal_prey_adjacent = sense_prey_adjacent(&animals, x, y);
        let map_threat = has_nearby_named(&world, &content, x, y, &["wolf"]);
        let map_prey_adj = has_nearby_named_range(
            &world,
            &content,
            x,
            y,
            &["rabbit", "boar", "deer", "hare"],
            HUNT_RANGE,
        );
        let animal_threat = deadly.is_some() || animal_threat_cheby || map_threat;
        let threat_near = animal_threat;
        let prey_adjacent = animal_prey_adjacent || map_prey_adj;
        let standing_biome = {
            let w = world.read().unwrap();
            w.get_biome(x, y)
        };
        let on_grassland = is_grassland(standing_biome);
        let mut have = HashSet::new();
        if held != 0 {
            have.insert(held);
        }
        // Prefer live ladder when threat / mother / superbad heat (AI-PRIO-LIVE).
        let use_live_ladder = threat_near || has_mother || heat < 0.1 || heat > 0.9;
        let mut goal = if use_live_ladder {
            let deadly_animal = deadly
                .map(|d| (d.x, d.y, d.dist_quad))
                .or_else(|| {
                    if animal_threat_cheby || map_threat {
                        // Outside moves² but Chebyshev/map still sees wolf — force near dist.
                        Some((x, y, 50.0))
                    } else {
                        None
                    }
                });
            let mut input = LiveSensorInput {
                held_id: held,
                food,
                food_max,
                was_hungry, // previous-tick hysteresis input
                age,
                heat,
                has_mother,
                follow_player: has_mother,
                nearby_food,
                deadly_animal,
                held_by_other: held_by > 0 && age < MIN_AGE_TO_EAT,
                ..Default::default()
            };
            // NPC-CRAFT-LADDER: job sensors from sticky profession role (selfplay has no
            // Player.farm_profession — map Profession → assigned sticky snapshot).
            // Haxe: assignedProfession / lastProfession → AssignedJob / AgeRotatedJob
            let sticky = selfplay_sticky_for_profession(profession, age);
            apply_job_flags_to_live_input(&mut input, &sticky);
            let (rung, g, bundle) =
                pick_goal_from_live_sensors(&input, profession, prey_adjacent, on_grassland);
            was_hungry = bundle.is_hungry;
            if tick % 11 == 1 {
                push_log(
                    &log,
                    &format!(
                        "[{}] t{tick} LIVE_SENSORS rung={} goal={} threat={threat_near} mother={has_mother} heat={heat:.2} food={food:.1}/{food_max:.1}",
                        agent.label,
                        rung.as_label(),
                        g.as_label(),
                    ),
                );
            }
            g
        } else {
            // Keep hungry hysteresis even on thin pick_goal_* path.
            was_hungry = update_is_hungry(was_hungry, food, food_max, held);
            if matches!(profession, Profession::Smith | Profession::Baker) {
                let smith_stage = if profession == Profession::Smith {
                    infer_smith_stage_from_have(&have)
                } else {
                    0.0
                };
                ol_sim::pick_goal_smith_craft_at_stage(
                    profession,
                    held,
                    food,
                    nearby_food,
                    threat_near,
                    prey_adjacent,
                    on_grassland,
                    &craft_graph,
                    &have,
                    SMITH_IRON_ID,
                    smith_stage,
                )
            } else {
                pick_goal_ext(
                    profession,
                    held,
                    food,
                    nearby_food,
                    threat_near,
                    prey_adjacent,
                    on_grassland,
                    0,
                )
            }
        };
        // Evidence: Hunter (and others) flee real AnimalWorld wolves.
        if matches!(goal, Goal::Flee) && animal_threat && tick % 5 == 1 {
            push_log(
                &log,
                &format!(
                    "[{}] t{tick} FLEE animal_threat deadly={} cheby_range={ANIMAL_THREAT_RANGE} food={food:.1} at ({x},{y})",
                    agent.label,
                    deadly.is_some(),
                ),
            );
        }
        if matches!(goal, Goal::Harvest) && tick % 7 == 1 {
            push_log(
                &log,
                &format!(
                    "[{}] t{tick} HARVEST intent grassland biome={standing_biome} food={food:.1} at ({x},{y})",
                    agent.label
                ),
            );
        }
        if matches!(goal, Goal::Hunt) && tick % 5 == 1 {
            push_log(
                &log,
                &format!(
                    "[{}] t{tick} HUNT adjacent prey animal={animal_prey_adjacent} map={map_prey_adj} at ({x},{y})",
                    agent.label
                ),
            );
        }
        // Craft plan: reverse-craft toward profession product via intermediate ingredients.
        // Farmer/Smith always; also when a SeekObject goal is already set.
        // Smith: expand want via products_using(iron) → first smith product / path.
        let craft_want = match (profession, goal) {
            (Profession::Farmer, _) => Some(FARMER_TARGET_ID),
            (Profession::Baker, _) => Some(BAKER_TARGET_ID),
            (Profession::Smith, _) => {
                // Prefer first product from iron reverse edges, else default.
                let targets = ol_sim::smith_product_targets(&craft_graph, SMITH_IRON_ID);
                Some(
                    targets
                        .into_iter()
                        .find(|p| !have.contains(p))
                        .unwrap_or(SMITH_TARGET_ID),
                )
            }
            (_, Goal::SeekObject(w)) if w != 0 => Some(w),
            _ => None,
        };
        if let Some(want) = craft_want {
            // Prefer intermediate ingredients when seeking the product.
            if matches!(goal, Goal::SeekObject(w) if w == want)
                || matches!(profession, Profession::Farmer | Profession::Smith | Profession::Baker)
                    && matches!(goal, Goal::SeekObject(_) | Goal::Explore | Goal::Idle)
                    && held == 0
            {
                if profession == Profession::Smith {
                    // AI-JOB-SMITH: infer stage from held/have so ladder advances past ore
                    let smith_stage = infer_smith_stage_from_have(&have);
                    goal = pick_smith_profession_goal(&craft_graph, &have, smith_stage);
                    if tick % 8 == 0 {
                        push_log(
                            &log,
                            &format!(
                                "[{}] t{tick} smith-iron-plan want={want} stage={smith_stage} goal={goal:?}",
                                agent.label
                            ),
                        );
                    }
                } else if profession == Profession::Farmer {
                    // Haxe: AI-JOB-FARM pipeline + reverse-craft intermediates (not only 242).
                    goal = pick_farmer_goal(&craft_graph, &have);
                } else if profession == Profession::Baker {
                    // Haxe: AI-JOB-BAKER oven/pie/bread pipeline (stage from inventory)
                    let baker_stage = infer_baker_pipeline_stage(&have);
                    goal = pick_baker_goal(&craft_graph, &have, baker_stage);
                    if tick % 8 == 0 {
                        push_log(
                            &log,
                            &format!(
                                "[{}] t{tick} baker-plan want={want} stage={baker_stage:.1} goal={goal:?}",
                                agent.label
                            ),
                        );
                    }
                } else if let Some(ing) = craft_graph.seek_ingredient_for(want, &have) {
                    if ing != want {
                        goal = Goal::SeekObject(ing);
                        if tick % 8 == 0 {
                            push_log(
                                &log,
                                &format!(
                                    "[{}] t{tick} craft-plan want={want} seek_ing={ing}",
                                    agent.label
                                ),
                            );
                        }
                    } else if matches!(goal, Goal::Explore | Goal::Idle) && held == 0 {
                        goal = Goal::SeekObject(want);
                    }
                } else if matches!(goal, Goal::Explore | Goal::Idle) && held == 0 {
                    // No path yet — still bias toward profession product.
                    if matches!(profession, Profession::Farmer | Profession::Smith | Profession::Baker) {
                        goal = Goal::SeekObject(want);
                    }
                }
            }
            // Holding a useful craft ingredient: seek partner instead of SeekObject root.
            if held != 0 && craft_graph.held_is_craft_ingredient(want, held) {
                if let Some(partner) = craft_graph.partner_for_held(want, held, &have) {
                    goal = Goal::SeekObject(partner);
                    if tick % 8 == 0 {
                        push_log(
                            &log,
                            &format!(
                                "[{}] t{tick} craft-partner want={want} held={held} seek={partner}",
                                agent.label
                            ),
                        );
                    }
                }
            }
        }

        // Suppress unused warning when animal_prey_near only used for logging path.
        let _ = animal_prey_near;

        // Profession auto-intents: SAY HARVEST / SAY HUNT when goal says so.
        if held == 0 {
            if matches!(goal, Goal::Harvest) {
                let _ = intent_tx
                    .send(NetIntent::Raw {
                        conn_id,
                        tag: "SAY".into(),
                        payload: "HARVEST".into(),
                    })
                    .await;
                push_log(
                    &log,
                    &format!("[{}] t{tick} SAY HARVEST at ({x},{y})", agent.label),
                );
            } else if matches!(goal, Goal::Hunt) && prey_adjacent {
                let _ = intent_tx
                    .send(NetIntent::Raw {
                        conn_id,
                        tag: "SAY".into(),
                        payload: "HUNT".into(),
                    })
                    .await;
                push_log(
                    &log,
                    &format!("[{}] t{tick} SAY HUNT at ({x},{y})", agent.label),
                );
            }
        }

        // 1) Eat if holding food and low on food; keep craft ingredients; else drop.
        if held != 0 {
            let is_food = content
                .get(held)
                .map(|d| d.food_value > 0)
                .unwrap_or(false);
            if is_food && food < 15.0 {
                let _ = intent_tx
                    .send(NetIntent::Use {
                        conn_id,
                        x,
                        y,
                        id: None,
                        index: None,
                    })
                    .await;
                push_log(
                    &log,
                    &format!(
                        "[{}] t{tick} EAT held={held} food={food:.1} at ({x},{y})",
                        agent.label
                    ),
                );
            } else if !is_food {
                let keep_craft = craft_want
                    .map(|want| craft_graph.held_is_craft_ingredient(want, held))
                    .unwrap_or(false);
                if keep_craft {
                    // USE held on nearby partner object when in range (craft step).
                    if let Some(want) = craft_want {
                        let mut have = HashSet::new();
                        have.insert(held);
                        if let Some(partner) = craft_graph.partner_for_held(want, held, &have) {
                            if let Some((tx, ty, _)) =
                                find_object_id_near(&world, x, y, partner, 2)
                            {
                                let _ = intent_tx
                                    .send(NetIntent::Use {
                                        conn_id,
                                        x: tx,
                                        y: ty,
                                        id: Some(partner),
                                        index: None,
                                    })
                                    .await;
                                push_log(
                                    &log,
                                    &format!(
                                        "[{}] t{tick} CRAFT-USE held={held} on {partner}@({tx},{ty}) want={want}",
                                        agent.label
                                    ),
                                );
                            }
                        }
                    }
                    // else keep holding and walk toward partner (handled below).
                } else {
                    // Haxe: dropHeldObject smart — peels at feet; else container/empty (DROP-HELD-LIVE)
                    let tiles = {
                        let w = world.read().unwrap();
                        scan_world_radius(&w, Some(content.as_ref()), x, y, 12)
                    };
                    let max_home = if force_drop_at_feet(held) { 1.0 } else { 40.0 };
                    // PREFER-SHORT-WAIT: pass agent moving if known; selfplay agents are usually stationary
                    let agent_moving = false;
                    let intent = smart_drop_held_from_sensors(
                        held,
                        1,
                        x,
                        y,
                        x,
                        y,
                        food,
                        agent_moving,
                        false,
                        max_home,
                        &tiles,
                        DropHeldSensorExtras::default(),
                    );
                    match intent {
                        ShortCraftLiveIntent::DropAt { x: tx, y: ty } => {
                            let _ = intent_tx
                                .send(NetIntent::Drop {
                                    conn_id,
                                    x: tx,
                                    y: ty,
                                    c: None,
                                })
                                .await;
                            push_log(
                                &log,
                                &format!(
                                    "[{}] t{tick} SMART-DROP held={held} at ({tx},{ty})",
                                    agent.label
                                ),
                            );
                        }
                        ShortCraftLiveIntent::UseAt { x: tx, y: ty, .. }
                        | ShortCraftLiveIntent::UseOnEmptyGround { x: tx, y: ty, .. } => {
                            let _ = intent_tx
                                .send(NetIntent::Use {
                                    conn_id,
                                    x: tx,
                                    y: ty,
                                    id: None,
                                    index: None,
                                })
                                .await;
                            push_log(
                                &log,
                                &format!(
                                    "[{}] t{tick} SMART-DROP-USE held={held} at ({tx},{ty})",
                                    agent.label
                                ),
                            );
                        }
                        ShortCraftLiveIntent::SelfClothing { slot } => {
                            let _ = intent_tx
                                .send(NetIntent::Raw {
                                    conn_id,
                                    tag: "SELF".into(),
                                    payload: self_clothing_raw_payload(slot),
                                })
                                .await;
                            push_log(
                                &log,
                                &format!(
                                    "[{}] t{tick} SMART-SELF clothing slot={slot} held={held}",
                                    agent.label
                                ),
                            );
                        }
                        ShortCraftLiveIntent::Goto { x: tx, y: ty } => {
                            if let Some((dx, dy)) = {
                                let w = world.read().unwrap();
                                next_step(&w, x, y, tx, ty, &|nx, ny| {
                                    is_walkable(&w, &content, nx, ny)
                                })
                            } {
                                let _ = intent_tx
                                    .send(NetIntent::Move {
                                        conn_id,
                                        xs: x,
                                        ys: y,
                                        deltas: vec![(dx, dy)],
                                        seq: None,
                                    })
                                    .await;
                                push_log(
                                    &log,
                                    &format!(
                                        "[{}] t{tick} SMART-DROP-GOTO held={held} toward ({tx},{ty})",
                                        agent.label
                                    ),
                                );
                            }
                        }
                        // Haxe: isMoving return true — hold tick, no feet-drop fallback (PREFER-SHORT-WAIT)
                        ShortCraftLiveIntent::Wait => {
                            push_log(
                                &log,
                                &format!(
                                    "[{}] t{tick} SMART-DROP-WAIT held={held} busy_moving",
                                    agent.label
                                ),
                            );
                        }
                        _ => {
                            // Fallback: container neighbor then empty ground.
                            let drop_at = find_container_neighbor(&world, &content, x, y)
                                .or_else(|| find_empty_neighbor(&world, x, y));
                            if let Some((tx, ty)) = drop_at {
                                let _ = intent_tx
                                    .send(NetIntent::Drop {
                                        conn_id,
                                        x: tx,
                                        y: ty,
                                        c: None,
                                    })
                                    .await;
                                push_log(
                                    &log,
                                    &format!(
                                        "[{}] t{tick} DROP held={held} at ({tx},{ty})",
                                        agent.label
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        } else if tick % 11 == 0 {
            // Occasionally try REMV from a nearby container.
            if let Some((tx, ty)) = find_container_neighbor(&world, &content, x, y) {
                let _ = intent_tx
                    .send(NetIntent::Raw {
                        conn_id,
                        tag: "REMV".into(),
                        payload: format!("{tx} {ty}"),
                    })
                    .await;
                push_log(
                    &log,
                    &format!("[{}] t{tick} REMV try at ({tx},{ty})", agent.label),
                );
            }
        }
        if tick % 17 == 0 {
            let say = if tick % 68 == 0 {
                "RECIPE".to_string()
            } else if tick % 51 == 0 {
                "NEXTCRAFT".to_string()
            } else if tick % 34 == 0 {
                "?TEMP".to_string()
            } else {
                format!("hello {} t{tick}", agent.label)
            };
            let _ = intent_tx
                .send(NetIntent::Raw {
                    conn_id,
                    tag: "SAY".into(),
                    payload: say.clone(),
                })
                .await;
            push_log(&log, &format!("[{}] t{tick} SAY {say}", agent.label));
        }

        // Sync local coords from server snapshot (do not optimistic-teleport).
        if let Some(ref p) = snap {
            x = p.x;
            y = p.y;
            *pos.write().unwrap() = (x, y);
        }
        let snapshot_moving = snap.as_ref().map(|p| p.moving).unwrap_or(false);
        let done_seq = snap.as_ref().map(|p| p.done_moving_seq).unwrap_or(0);

        // 2) Walk only when not mid-path (honor PlayerSnapshot.moving).
        // SeekFood: prefer highest food_value object within 8 tiles, then pathfind.
        let target = match goal {
            Goal::Explore | Goal::Idle | Goal::Flee | Goal::Harvest => None,
            Goal::SeekFood => find_best_food(
                &world,
                &content,
                x,
                y,
                8,
                last_use.map(|(a, b, _)| (a, b)),
            )
            .or_else(|| {
                find_target(
                    &world,
                    &content,
                    x,
                    y,
                    last_use.map(|(a, b, _)| (a, b)),
                    goal,
                )
            }),
            Goal::SeekObject(_) | Goal::Hunt => find_target(
                &world,
                &content,
                x,
                y,
                last_use.map(|(a, b, _)| (a, b)),
                goal,
            ),
        };
        // Flee: prefer opposite of live AnimalWorld wolf dir; else map wolf object.
        let flee_dir = if matches!(goal, Goal::Flee) {
            animal_flee_away.or_else(|| {
                find_threat_dir(&world, &content, x, y)
                    .map(|(dx, dy)| (-dx.signum(), -dy.signum()))
            })
        } else {
            None
        };
        let (dx, dy) = if let Some((fdx, fdy)) = flee_dir {
            if fdx == 0 && fdy == 0 {
                random_step(&mut rng)
            } else {
                (fdx, fdy)
            }
        } else if let Some((tx, ty, _id)) = target {
            let dist = (tx - x).abs().max((ty - y).abs());
            if dist <= 1 {
                // Always keep moving so agents stay visible.
                random_step(&mut rng)
            } else {
                // Pathfind around blocks_walking; SeekFood uses animal dual-pass footprints.
                // Haxe: gotoAdv considerAnimals for food walk (AI-GOTO-FOOD)
                let food_store = snap.as_ref().map(|p| p.food).unwrap_or(0.0);
                let seek_food = matches!(goal, Goal::SeekFood);
                let step = {
                    let w = world.read().unwrap();
                    let content = content.clone();
                    if seek_food {
                        let consider = consider_animals_for_goto(true, 0.0, food_store);
                        next_step_consider_animals(&w, &content, x, y, tx, ty, consider)
                    } else {
                        next_step(&w, x, y, tx, ty, &|nx, ny| {
                            is_walkable(&w, &content, nx, ny)
                        })
                    }
                };
                step.unwrap_or_else(|| {
                    let dx = (tx - x).signum();
                    let dy = (ty - y).signum();
                    if dx == 0 && dy == 0 {
                        random_step(&mut rng)
                    } else {
                        (dx, dy)
                    }
                })
            }
        } else {
            random_step(&mut rng)
        };

        if snapshot_moving {
            // Wait for path complete; no destination KA while moving (K11 + agent).
            if tick % 5 == 0 {
                push_log(
                    &log,
                    &format!(
                        "[{}] t{tick} wait moving at ({x},{y}) done_seq={done_seq}",
                        agent.label
                    ),
                );
            }
        } else if dx != 0 || dy != 0 {
            let xs = x;
            let ys = y;
            // Do not optimistic-update local x,y to path end; follow snapshot next loop.
            // Do **not** send KeepAlive after MOVE: with timed_movement=false, instant MOVE
            // lands at the destination, then KA with start coords snaps the player back
            // (move_path is None so mid-path KA ignore does not apply).
            let _ = intent_tx
                .send(NetIntent::Move {
                    conn_id,
                    xs,
                    ys,
                    deltas: vec![(dx, dy)],
                    seq: None,
                })
                .await;
            if tick % 3 == 0 {
                if let Some((tx, ty, id)) = target {
                    push_log(
                        &log,
                        &format!(
                            "[{}] t{tick} walk ({xs},{ys})+({},{}) seek {id}@({tx},{ty})",
                            agent.label, dx, dy
                        ),
                    );
                } else {
                    push_log(
                        &log,
                        &format!("[{}] t{tick} explore step ({xs},{ys})", agent.label),
                    );
                }
            }
        }

        // Post-USE waitingTime analog: skip further USE this loop window.
        if let Some(until) = post_use_wait_until {
            if std::time::Instant::now() < until {
                continue;
            }
            post_use_wait_until = None;
        }

        // 3) USE adjacent interesting object if hands empty and goal wants an object.
        let hands_empty = player_views
            .read()
            .ok()
            .and_then(|g| g.get(&conn_id).map(|p| p.held_id == 0))
            .unwrap_or(held == 0);

        // Harvest / Hunt use SAY intents above; object USE for food/craft/prey objects.
        let want_use = matches!(
            goal,
            Goal::SeekFood | Goal::SeekObject(_) | Goal::Hunt | Goal::Harvest
        );
        if hands_empty && want_use {
            let use_target = match goal {
                Goal::SeekFood => find_best_food(&world, &content, x, y, 8, None)
                    .or_else(|| find_target(&world, &content, x, y, None, goal)),
                _ => find_target(&world, &content, x, y, None, goal),
            };
            if let Some((tx, ty, id)) = use_target {
                // Squared-Euclidean adjacency (d=1): diagonal not in range.
                let dxu = (tx - x) as i64;
                let dyu = (ty - y) as i64;
                let in_range = dxu * dxu + dyu * dyu <= 1;
                let same_spam = matches!(last_use, Some((lx, ly, t0)) if lx == tx && ly == ty && tick.saturating_sub(t0) < 6);
                // Skip USE while moving.
                if snapshot_moving {
                    // wait
                } else if same_spam {
                    // Design §1.7: same_spam is **throttle only** — do not force unstick walk.
                    // Multi-use forage must re-USE the same tile without random MOVE.
                    if tick % 8 == 0 {
                        push_log(
                            &log,
                            &format!(
                                "[{}] t{tick} throttle same_spam obj {id}@({tx},{ty})",
                                agent.label
                            ),
                        );
                    }
                } else if in_range && stuck_use_count < 3 {
                    let obj_before = {
                        let w = world.read().unwrap();
                        w.get_object(tx, ty)
                    };
                    let uses_before = {
                        let w = world.read().unwrap();
                        w.get_helper(tx, ty).map(|h| h.uses_remaining).unwrap_or(0)
                    };
                    // Multi-use nearly exhausted: force last-use transition via SAY LASTUSE.
                    let nearly_exhausted = multi_use_nearly_exhausted(&world, &content, tx, ty, id);
                    if nearly_exhausted {
                        let _ = intent_tx
                            .send(NetIntent::Raw {
                                conn_id,
                                tag: "SAY".into(),
                                payload: "LASTUSE".into(),
                            })
                            .await;
                        push_log(
                            &log,
                            &format!(
                                "[{}] t{tick} LASTUSE force near-exhaust obj {id}@({tx},{ty})",
                                agent.label
                            ),
                        );
                    }
                    let _ = intent_tx
                        .send(NetIntent::Use {
                            conn_id,
                            x: tx,
                            y: ty,
                            id: Some(id),
                            index: None,
                        })
                        .await;
                    if nearly_exhausted {
                        let _ = intent_tx
                            .send(NetIntent::Use {
                                conn_id,
                                x: tx,
                                y: ty,
                                id: Some(id),
                                index: None,
                            })
                            .await;
                    }
                    last_use = Some((tx, ty, tick));
                    // Wait for sim (~waitingTime) then check progress via snapshot + world.
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    post_use_wait_until =
                        Some(std::time::Instant::now() + Duration::from_millis(200));
                    let obj_after = {
                        let w = world.read().unwrap();
                        w.get_object(tx, ty)
                    };
                    let uses_after = {
                        let w = world.read().unwrap();
                        w.get_helper(tx, ty).map(|h| h.uses_remaining).unwrap_or(0)
                    };
                    let held_after = player_views
                        .read()
                        .ok()
                        .and_then(|g| g.get(&conn_id).map(|p| p.held_id))
                        .unwrap_or(held);
                    // Progress: object id change, uses burn, or held change.
                    let progressed = obj_after != obj_before
                        || uses_after != uses_before
                        || held_after != held;
                    if progressed {
                        stuck_use_count = 0;
                    } else {
                        stuck_use_count += 1;
                    }
                    push_log(
                        &log,
                        &format!(
                            "[{}] t{tick} USE obj {id} at ({tx},{ty}) from ({x},{y}) goal={goal:?}",
                            agent.label
                        ),
                    );
                } else if stuck_use_count >= 3 {
                    // Forced unstick only on stuck_count (not same_spam).
                    stuck_use_count = 0;
                    last_use = None;
                    if !snapshot_moving {
                        let (jx, jy) = random_step(&mut rng);
                        let xs = x;
                        let ys = y;
                        let _ = intent_tx
                            .send(NetIntent::Move {
                                conn_id,
                                xs,
                                ys,
                                deltas: vec![(jx, jy)],
                                seq: None,
                            })
                            .await;
                        if let Some(ref c) = counters {
                            c.selfplay_unstick_total.fetch_add(1, Ordering::Relaxed);
                        }
                        push_log(
                            &log,
                            &format!(
                                "[{}] t{tick} unstick walk reason=stuck_count → step ({jx},{jy})",
                                agent.label
                            ),
                        );
                    }
                }
            }
        } else {
            stuck_use_count = 0;
        }

        // Clear stuck counter when we moved far from last use.
        if let Some((lx, ly, _)) = last_use {
            if (lx - x).abs().max((ly - y).abs()) > 3 {
                stuck_use_count = 0;
            }
        }

        if tick % 20 == 0 {
            info!(
                agent = %agent.label,
                conn_id,
                ?profession,
                tick,
                x,
                y,
                held,
                food,
                ?goal,
                "selfplay heartbeat"
            );
        }
    }
}

async fn login_and_spawn(
    agent: &SelfplayAgent,
    intent_tx: &tokio::sync::mpsc::Sender<NetIntent>,
    world: &Arc<RwLock<World>>,
    log: &Arc<RwLock<Vec<String>>>,
    pos: &Arc<RwLock<(i32, i32)>>,
    x: &mut i32,
    y: &mut i32,
    rng: &mut impl Rng,
) {
    let conn_id = agent.conn_id;
    let _ = intent_tx
        .send(NetIntent::Login {
            conn_id,
            reconnect: false,
            email: agent.email.clone(),
            client_tag: agent.client_tag.clone(),
        })
        .await;
    push_log(
        log,
        &format!("[{}] agent login {}", agent.label, agent.email),
    );

    // Pick a green-ish spawn; fall back to random walk from center.
    // Offset secondary agents so they don't stack on the forager.
    let spawn_offset = if conn_id == SELFPLAY_CONN_ID {
        0
    } else {
        12 + ((conn_id - SELFPLAY_CONN_ID) as i32) * 8
    };
    {
        let w = world.read().unwrap();
        let cx = (w.width_tiles / 2).max(0) + spawn_offset;
        let cy = (w.height_tiles / 2).max(0);
        let mut found = false;
        'search: for r in 0..100 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let tx = cx + dx;
                    let ty = cy + dy;
                    if w.get_biome(tx, ty) == 0 {
                        *x = tx;
                        *y = ty;
                        found = true;
                        break 'search;
                    }
                }
            }
        }
        if !found {
            *x = cx + rng.gen_range(-20..=20);
            *y = cy + rng.gen_range(-20..=20);
            if w.width_tiles > 0 {
                *x = x.rem_euclid(w.width_tiles);
                *y = y.rem_euclid(w.height_tiles);
            }
        }
    }
    *pos.write().unwrap() = (*x, *y);
    // Empty MOVE is a no-op; omit KA so we never fight sim-authoritative spawn coords.
    let _ = intent_tx
        .send(NetIntent::Move {
            conn_id,
            xs: *x,
            ys: *y,
            deltas: vec![],
            seq: None,
        })
        .await;
    push_log(log, &format!("[{}] spawn at ({x},{y})", agent.label));
}

fn wrap_xy(world: &Arc<RwLock<World>>, x: i32, y: i32) -> (i32, i32) {
    let w = world.read().unwrap();
    if w.width_tiles > 0 && w.height_tiles > 0 {
        (x.rem_euclid(w.width_tiles), y.rem_euclid(w.height_tiles))
    } else {
        (x, y)
    }
}

fn random_step(rng: &mut impl Rng) -> (i32, i32) {
    // Always non-zero so the agent visibly moves.
    match rng.gen_range(0..8) {
        0 => (1, 0),
        1 => (-1, 0),
        2 => (0, 1),
        3 => (0, -1),
        4 => (1, 1),
        5 => (1, -1),
        6 => (-1, 1),
        _ => (-1, -1),
    }
}

fn find_empty_neighbor(world: &Arc<RwLock<World>>, x: i32, y: i32) -> Option<(i32, i32)> {
    let w = world.read().unwrap();
    for oy in -1..=1 {
        for ox in -1..=1 {
            if ox == 0 && oy == 0 {
                continue;
            }
            let tx = x + ox;
            let ty = y + oy;
            if w.get_object(tx, ty) == 0 {
                return Some((tx, ty));
            }
        }
    }
    None
}

fn find_container_neighbor(
    world: &Arc<RwLock<World>>,
    content: &ContentDb,
    x: i32,
    y: i32,
) -> Option<(i32, i32)> {
    let w = world.read().unwrap();
    for oy in -2..=2 {
        for ox in -2..=2 {
            let tx = x + ox;
            let ty = y + oy;
            let id = w.get_object(tx, ty);
            if id == 0 {
                continue;
            }
            if content.get(id).map(|d| d.is_container()).unwrap_or(false) {
                return Some((tx, ty));
            }
            // Also treat helpers with contained items as containers.
            if w
                .get_helper(tx, ty)
                .map(|h| !h.contained.is_empty())
                .unwrap_or(false)
            {
                return Some((tx, ty));
            }
        }
    }
    None
}

/// Prefer objects matching `goal`; skip `avoid` tile so we don't glue to one wheat forever.
fn find_target(
    world: &Arc<RwLock<World>>,
    content: &ContentDb,
    x: i32,
    y: i32,
    avoid: Option<(i32, i32)>,
    goal: Goal,
) -> Option<(i32, i32, i32)> {
    let w = world.read().unwrap();
    let mut best: Option<(i32, i32, i32, i32)> = None; // rank, tx, ty, id
    for r in 1i32..=16 {
        for oy in -r..=r {
            let oy: i32 = oy;
            for ox in -r..=r {
                let ox: i32 = ox;
                if ox.abs() != r && oy.abs() != r {
                    continue;
                }
                let tx = x + ox;
                let ty = y + oy;
                if avoid == Some((tx, ty)) {
                    continue;
                }
                let id = w.get_object(tx, ty);
                if id == 0 {
                    continue;
                }
                let interest = object_interest(content, id, goal);
                if interest <= 0 {
                    continue;
                }
                // lower is better: distance minus interest
                let rank = ox.abs() + oy.abs() - interest;
                if best.map(|b| rank < b.0).unwrap_or(true) {
                    best = Some((rank, tx, ty, id));
                }
            }
        }
        if best.is_some() {
            break;
        }
    }
    best.map(|(_, tx, ty, id)| (tx, ty, id))
}

/// Find nearest tile with exact object id within Chebyshev `r` (for craft partner USE).
fn find_object_id_near(
    world: &Arc<RwLock<World>>,
    x: i32,
    y: i32,
    want_id: i32,
    r: i32,
) -> Option<(i32, i32, i32)> {
    if want_id == 0 {
        return None;
    }
    let w = world.read().unwrap();
    let mut best: Option<(i32, i32, i32, i32)> = None; // dist, tx, ty, id
    for oy in -r..=r {
        for ox in -r..=r {
            let tx = x + ox;
            let ty = y + oy;
            let id = w.get_object(tx, ty);
            if id != want_id {
                continue;
            }
            let dist = ox.abs().max(oy.abs());
            if best.map(|b| dist < b.0).unwrap_or(true) {
                best = Some((dist, tx, ty, id));
            }
        }
    }
    best.map(|(_, tx, ty, id)| (tx, ty, id))
}

/// Scan radius `r` for best food using **shared** SearchBestFood pure scoring.
///
/// Skips `avoid` so agents unstick from a single bush. Same scorer as NPC hungry
/// seek and live `search_best_food_full` (`ol_player_helper::pick_best_search_food`).
/// Used by Forager [`Goal::SeekFood`].
fn find_best_food(
    world: &Arc<RwLock<World>>,
    content: &ContentDb,
    x: i32,
    y: i32,
    r: i32,
    avoid: Option<(i32, i32)>,
) -> Option<(i32, i32, i32)> {
    use ol_player_helper::{
        pick_best_search_food, AiFoodSearchFlags, ProcessFoodOpts, SearchFoodCand,
    };
    let w = world.read().unwrap();
    let mut cands: Vec<SearchFoodCand> = Vec::new();
    let mut stock: Vec<(i32, i32, i32, i32)> = Vec::new();
    for oy in -r..=r {
        for ox in -r..=r {
            let tx = x + ox;
            let ty = y + oy;
            if avoid == Some((tx, ty)) {
                continue;
            }
            let id = w.get_object(tx, ty);
            if id == 0 {
                continue;
            }
            let base = content.resolve_base_id(id);
            let Some(def) = content.get(base) else {
                continue;
            };
            let uses = if def.num_uses > 0 { def.num_uses } else { 1 };
            stock.push((tx, ty, base, uses));
            if def.food_value <= 0 {
                continue;
            }
            cands.push(SearchFoodCand {
                parent_id: base,
                food_id: base,
                food_value: def.food_value,
                tx,
                ty,
                count_eaten: 0.0,
                number_of_uses: uses,
                index_in_container: -1,
                is_dangerous: false,
                not_reachable: false,
                food_factor: 1.0,
            });
        }
    }
    drop(w);
    // Neutral stomach so pure gates still rank by distance/value/seed rules.
    let mut opts = ProcessFoodOpts::human(x, y, 5.0, 40.0, 0);
    opts.ai = Some(AiFoodSearchFlags::default());
    let (idx, _score) = pick_best_search_food(&cands, &opts, &stock)?;
    let c = &cands[idx];
    Some((c.tx, c.ty, c.food_id))
}

/// True when tile object is multi-use and nearly exhausted (`uses_remaining` in 1..=2).
fn multi_use_nearly_exhausted(
    world: &Arc<RwLock<World>>,
    content: &ContentDb,
    tx: i32,
    ty: i32,
    id: i32,
) -> bool {
    let num_uses = content.get(id).map(|d| d.num_uses).unwrap_or(0);
    if num_uses <= 1 {
        return false;
    }
    let w = world.read().unwrap();
    let uses = w
        .get_helper(tx, ty)
        .map(|h| h.uses_remaining)
        .unwrap_or(num_uses);
    uses > 0 && uses <= 2
}

fn has_nearby_food(world: &Arc<RwLock<World>>, content: &ContentDb, x: i32, y: i32) -> bool {
    let w = world.read().unwrap();
    for oy in -8..=8 {
        for ox in -8..=8 {
            let id = w.get_object(x + ox, y + oy);
            if id == 0 {
                continue;
            }
            if content.get(id).map(|d| d.food_value > 0).unwrap_or(false) {
                return true;
            }
            if let Some(def) = content.get(id) {
                let n = def.name.to_lowercase();
                if n.contains("berry")
                    || n.contains("carrot")
                    || n.contains("onion")
                    || n.contains("goose")
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Read live animal sensors from shared [`AnimalWorld`] (sim-published).
///
/// Returns `(threat_near, prey_near, flee_step)` where `flee_step` is a cardinal /
/// diagonal unit step away from the nearest wolf when threatened.
fn sense_animals(
    animals: &Arc<RwLock<AnimalWorld>>,
    x: i32,
    y: i32,
) -> (bool, bool, Option<(i32, i32)>) {
    let Ok(aw) = animals.read() else {
        return (false, false, None);
    };
    let threat = aw.nearby_threat(x, y, ANIMAL_THREAT_RANGE);
    let prey = aw.nearby_prey(x, y, ANIMAL_THREAT_RANGE);
    let flee = if threat {
        aw.nearest_threat_dir(x, y, ANIMAL_THREAT_RANGE).map(|(dx, dy)| {
            let ax = -dx.signum();
            let ay = -dy.signum();
            if ax == 0 && ay == 0 {
                // Standing on the wolf tile — step any cardinal.
                (1, 0)
            } else {
                (ax, ay)
            }
        })
    } else {
        None
    };
    (threat, prey, flee)
}

/// Haxe `GetCloseDeadlyAnimal` against live AnimalWorld (moves² filter).
fn sense_deadly_animal(
    animals: &Arc<RwLock<AnimalWorld>>,
    x: i32,
    y: i32,
) -> Option<CloseDeadlyAnimal> {
    let Ok(aw) = animals.read() else {
        return None;
    };
    aw.get_close_deadly_animal(x, y, DEADLY_ANIMAL_SEARCH_DIST)
}

/// True when live prey is within [`HUNT_RANGE`] (adjacent for `SAY HUNT`).
fn sense_prey_adjacent(animals: &Arc<RwLock<AnimalWorld>>, x: i32, y: i32) -> bool {
    let Ok(aw) = animals.read() else {
        return false;
    };
    aw.nearby_prey(x, y, HUNT_RANGE)
}

/// Cheap name-substring scan in a small radius (threat / prey sensors).
fn has_nearby_named(
    world: &Arc<RwLock<World>>,
    content: &ContentDb,
    x: i32,
    y: i32,
    needles: &[&str],
) -> bool {
    has_nearby_named_range(world, content, x, y, needles, 10)
}

/// Name-substring scan within Chebyshev `range` (inclusive).
fn has_nearby_named_range(
    world: &Arc<RwLock<World>>,
    content: &ContentDb,
    x: i32,
    y: i32,
    needles: &[&str],
    range: i32,
) -> bool {
    let w = world.read().unwrap();
    let r = range.max(0);
    for oy in -r..=r {
        for ox in -r..=r {
            let id = w.get_object(x + ox, y + oy);
            if id == 0 {
                continue;
            }
            if let Some(def) = content.get(id) {
                let n = def.name.to_lowercase();
                if needles.iter().any(|k| n.contains(k)) {
                    return true;
                }
            }
        }
    }
    false
}

/// Direction (dx, dy) toward nearest object whose name contains "wolf", if any.
fn find_threat_dir(
    world: &Arc<RwLock<World>>,
    content: &ContentDb,
    x: i32,
    y: i32,
) -> Option<(i32, i32)> {
    let w = world.read().unwrap();
    let mut best: Option<(i32, i32, i32)> = None; // dist, dx, dy
    for oy in -10..=10 {
        for ox in -10..=10 {
            if ox == 0 && oy == 0 {
                continue;
            }
            let id = w.get_object(x + ox, y + oy);
            if id == 0 {
                continue;
            }
            let Some(def) = content.get(id) else {
                continue;
            };
            if !def.name.to_lowercase().contains("wolf") {
                continue;
            }
            let d = ox.abs() + oy.abs();
            if best.map(|b| d < b.0).unwrap_or(true) {
                best = Some((d, ox, oy));
            }
        }
    }
    best.map(|(_, dx, dy)| (dx, dy))
}

fn object_interest(content: &ContentDb, id: i32, goal: Goal) -> i32 {
    let Some(def) = content.get(id) else {
        return 0;
    };
    let mut s = 0;
    let n = def.name.to_lowercase();
    let is_food = def.food_value > 0
        || n.contains("berry")
        || n.contains("goose")
        || n.contains("carrot")
        || n.contains("onion");

    match goal {
        Goal::SeekFood => {
            if def.food_value > 0 {
                // Prefer higher food_value when find_target fallback is used.
                s += 12 + def.food_value.min(20);
            }
            if is_food {
                s += 8;
            }
            // Weak fallback so empty biomes still pick something edible-ish.
            if n.contains("wheat") || n.contains("milkweed") {
                s += 2;
            }
        }
        Goal::SeekObject(want) => {
            if id == want {
                s += 20;
            }
            // Soft profession-ish names when exact id is rare on the map.
            if n.contains("wheat") || n.contains("soil") || n.contains("seed") {
                s += 4;
            }
            if n.contains("iron") || n.contains("forge") || n.contains("hammer") {
                s += 4;
            }
            if is_food {
                s += 1;
            }
        }
        Goal::Explore | Goal::Idle | Goal::Flee | Goal::Harvest => {
            // Not used for targeting (caller skips), but keep a neutral score.
            if is_food {
                s += 3;
            }
        }
        Goal::Hunt => {
            // Prefer animals / meat-ish names until dedicated prey map exists.
            if n.contains("rabbit")
                || n.contains("boar")
                || n.contains("wolf")
                || n.contains("deer")
                || n.contains("mouflon")
            {
                s += 16;
            }
            if n.contains("meat") || n.contains("carcass") {
                s += 10;
            }
            if is_food {
                s += 2;
            }
        }
    }
    s
}

fn push_log(log: &Arc<RwLock<Vec<String>>>, line: &str) {
    if let Ok(mut g) = log.write() {
        g.push(line.to_string());
        if g.len() > 200 {
            let drain = g.len() - 200;
            g.drain(0..drain);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::{ContentDb, ObjectDef};
    use ol_world::ComplexObject;

    fn food_def(id: i32, food_value: i32, num_uses: i32) -> ObjectDef {
        ObjectDef {
            id,
            description: format!("food{id}"),
            name: format!("Food{id}"),
            containable: false,
            permanent: false,
            blocks_walking: false,
            food_value,
            heat_value: 0.0,
            map_chance: 0.0,
            biomes: Vec::new(),
            num_uses,
            num_slots: 0,
            floor: false,
        dummy_ids: Vec::new(),
        use_chance: 0.0,
        speed_mult: 1.0,
        winter_decay_factor: 0.0,
        spring_regrow_factor: 0.0,
        decay_factor: 1.0,
        decays_to_obj: 0,
        r_value: 0.0,
        clothing: "n".into(),
        counts_or_grows_as: 0,
        crafting_steps: 0,
        use_distance: 1,
        deadly_distance: 0.0,
        moves: 0,
        damage: 0.0,
        damage_protection_factor: 1.0,
        wound_factor: 0.5,
        male: false,
        contain_size: 0.0,
        slot_size: 1.0,
        }
    }

    #[test]
    fn find_best_food_picks_highest_food_value_within_8() {
        let mut db = ContentDb::default();
        db.objects.insert(10, food_def(10, 2, 0));
        db.objects.insert(20, food_def(20, 9, 0));
        db.objects.insert(30, food_def(30, 5, 0));
        let content = Arc::new(db);
        let world = Arc::new(RwLock::new(World::new(64, 64, false)));
        {
            let mut w = world.write().unwrap();
            w.set_object(5, 5, 10); // fv=2 near center
            w.set_object(8, 5, 20); // fv=9 within 8 of (5,5)
            w.set_object(20, 20, 30); // fv=5 outside radius 8 from (5,5)
        }
        let best = find_best_food(&world, &content, 5, 5, 8, None).expect("food");
        assert_eq!(best.2, 20, "should pick highest food_value in radius");
        assert_eq!((best.0, best.1), (8, 5));
    }

    #[test]
    fn sense_animals_threat_from_shared_world() {
        use ol_sim::AnimalKind;
        let share = Arc::new(RwLock::new(AnimalWorld::new()));
        {
            let mut aw = share.write().unwrap();
            aw.spawn(AnimalKind::Wolf, 12, 10);
            aw.spawn(AnimalKind::Rabbit, 11, 10);
        }
        let (threat, prey, flee) = sense_animals(&share, 10, 10);
        assert!(threat, "wolf within threat range");
        assert!(prey, "rabbit within range");
        // Flee should step away from wolf at +2,0 → step -1,0
        assert_eq!(flee, Some((-1, 0)));
        // No threat when far
        let (t2, _, f2) = sense_animals(&share, 50, 50);
        assert!(!t2);
        assert!(f2.is_none());
    }

    #[test]
    fn hunter_pick_goal_flees_real_wolf_via_sense() {
        use ol_sim::{pick_goal_ext, AnimalKind, Profession};
        let share = Arc::new(RwLock::new(AnimalWorld::new()));
        share.write().unwrap().spawn(AnimalKind::Wolf, 10, 10);
        let (threat, _prey, _) = sense_animals(&share, 10, 10);
        assert!(threat);
        let prey_adj = sense_prey_adjacent(&share, 10, 10);
        let goal = pick_goal_ext(
            Profession::Hunter,
            0,
            15.0,
            false,
            threat,
            prey_adj,
            false,
            0,
        );
        assert_eq!(goal, Goal::Flee);
    }

    #[test]
    fn hunter_pick_goal_hunts_when_prey_adjacent() {
        use ol_sim::{pick_goal_ext, AnimalKind, Profession};
        let share = Arc::new(RwLock::new(AnimalWorld::new()));
        share.write().unwrap().spawn(AnimalKind::Rabbit, 10, 10);
        let prey_adj = sense_prey_adjacent(&share, 10, 10);
        assert!(prey_adj);
        let goal = pick_goal_ext(
            Profession::Hunter,
            0,
            15.0,
            false,
            false,
            prey_adj,
            false,
            0,
        );
        assert_eq!(goal, Goal::Hunt);
        // Far prey is not adjacent
        share.write().unwrap().animals[0].x = 20;
        assert!(!sense_prey_adjacent(&share, 10, 10));
    }

    #[test]
    fn forager_harvest_goal_on_grassland() {
        use ol_sim::{pick_goal_ext, Profession};
        let g = pick_goal_ext(Profession::Forager, 0, 15.0, true, false, false, true, 0);
        assert_eq!(g, Goal::Harvest);
    }

    #[test]
    fn multi_use_nearly_exhausted_detects_low_uses() {
        let mut db = ContentDb::default();
        db.objects.insert(50, food_def(50, 1, 5));
        let content = Arc::new(db);
        let world = Arc::new(RwLock::new(World::new(16, 16, false)));
        {
            let mut w = world.write().unwrap();
            w.set_object_complex(1, 1, ComplexObject::with_uses(50, 2));
            w.set_object_complex(2, 2, ComplexObject::with_uses(50, 4));
        }
        assert!(multi_use_nearly_exhausted(&world, &content, 1, 1, 50));
        assert!(!multi_use_nearly_exhausted(&world, &content, 2, 2, 50));
    }
}


/// Pure decision: force unstick only on stuck_count (not same_spam).
pub fn should_force_unstick(stuck_use_count: u32, threshold: u32) -> bool {
    stuck_use_count >= threshold
}

/// Pure decision: same_spam only throttles USE.
pub fn should_throttle_same_spam(
    last_use: Option<(i32, i32, u64)>,
    tx: i32,
    ty: i32,
    tick: u64,
    window: u64,
) -> bool {
    matches!(last_use, Some((lx, ly, t0)) if lx == tx && ly == ty && tick.saturating_sub(t0) < window)
}

/// Agents must not send KeepAlive after MOVE (instant path authority).
pub fn should_send_ka_after_move() -> bool {
    false
}

#[cfg(test)]
mod decision_tests {
    use super::*;

    #[test]
    fn unstick_only_on_stuck_count() {
        assert!(!should_force_unstick(2, 3));
        assert!(should_force_unstick(3, 3));
    }

    #[test]
    fn same_spam_throttle() {
        assert!(should_throttle_same_spam(Some((1, 2, 10)), 1, 2, 12, 6));
        assert!(!should_throttle_same_spam(Some((1, 2, 10)), 1, 2, 20, 6));
        assert!(!should_throttle_same_spam(None, 1, 2, 12, 6));
    }

    #[test]
    fn no_ka_after_move() {
        assert!(!should_send_ka_after_move());
    }
}
