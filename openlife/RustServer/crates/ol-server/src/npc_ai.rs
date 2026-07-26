//! Basic AI NPC scheduler (Haxe `AiBase.RunAi` shape — single thread).
//!
//! Priority: eat if hungry → seek food → craft (bottom-up valuation) → explore.
//! Activity logged in RAM and flushed every 30s ([`npc_activity`]).

use crate::npc_activity::{
    NpcActivityEvent, NpcActivityKind, NpcActivityLog, NpcStuckTracker,
};
use ol_content::ContentDb;
use ol_metrics::{Counters, ScopeTimer};
use ol_net::NetIntent;
use ol_sim::{
    best_craft, evaluate_nearby_crafts, is_walkable, next_step, CraftProfession, NearbyObj,
    PlayerSnapshot, DEFAULT_CRAFT_RADIUS, DEFAULT_WALK_SPEED, INTERACTION_SEC,
};
use ol_world::World;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Reserved NPC conn id base (above self-play).
pub const NPC_CONN_BASE: u64 = 9_100_000;

#[derive(Debug, Clone)]
pub struct NpcConfig {
    pub enabled: bool,
    pub min: u32,
    pub max: u32,
    pub think_period_ticks: u32,
    pub observe_radius: i32,
    pub craft_radius: i32,
}

impl Default for NpcConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min: 3,
            max: 40,
            think_period_ticks: 10,
            observe_radius: 16,
            craft_radius: DEFAULT_CRAFT_RADIUS,
        }
    }
}

fn profession_for_index(i: u32) -> CraftProfession {
    match i % 3 {
        0 => CraftProfession::Forager,
        1 => CraftProfession::Farmer,
        _ => CraftProfession::Hunter,
    }
}

fn collect_nearby(world: &World, px: i32, py: i32, radius: i32) -> Vec<NearbyObj> {
    let mut out = Vec::new();
    let r = radius.max(1).min(60);
    for dy in -r..=r {
        for dx in -r..=r {
            let x = px + dx;
            let y = py + dy;
            let id = world.get_object(x, y);
            if id != 0 {
                out.push(NearbyObj { id, x, y });
            }
        }
    }
    out
}

fn food_at(content: &ContentDb, id: i32) -> i32 {
    content.get(id).map(|d| d.food_value).unwrap_or(0)
}

/// Find nearest edible ground object (food_value > 0).
fn nearest_food(
    content: &ContentDb,
    nearby: &[NearbyObj],
    px: i32,
    py: i32,
) -> Option<NearbyObj> {
    nearby
        .iter()
        .filter(|o| food_at(content, o.id) > 0)
        .min_by_key(|o| (o.x - px).abs().max((o.y - py).abs()))
        .copied()
}

fn log_ev(
    log: &NpcActivityLog,
    conn_id: u64,
    p: &PlayerSnapshot,
    kind: NpcActivityKind,
    cpu_us: u32,
    game_ms: u32,
    detail: impl Into<String>,
) {
    log.push(NpcActivityEvent {
        wall_unix_ms: 0,
        conn_id,
        p_id: p.p_id,
        kind,
        cpu_us,
        game_ms,
        age: p.age,
        food: p.food,
        x: p.x,
        y: p.y,
        held_id: p.held_id,
        detail: detail.into(),
    });
}

/// Run single AI scheduler thread loop (async task).
pub async fn run_npc_scheduler(
    cfg: NpcConfig,
    intent_tx: tokio::sync::mpsc::Sender<NetIntent>,
    world: Arc<RwLock<World>>,
    content: Arc<ContentDb>,
    player_views: Arc<RwLock<HashMap<u64, PlayerSnapshot>>>,
    counters: Arc<Counters>,
    activity: Arc<NpcActivityLog>,
) {
    if !cfg.enabled {
        info!("npc scheduler idle (npc_enabled=false)");
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    }

    let min = cfg.min.max(1);
    let max = cfg.max.max(min);
    let mut target_pop = min;
    let think_period = cfg.think_period_ticks.max(1);
    let radius = cfg.observe_radius.max(4);
    let craft_radius = cfg.craft_radius.max(8).min(80);

    let labels = ["npc-forager", "npc-farmer", "npc-hunter"];
    for i in 0..min {
        let conn_id = NPC_CONN_BASE + i as u64;
        let email = format!("{}@local", labels[i as usize % labels.len()]);
        let _ = intent_tx
            .send(NetIntent::Login {
                conn_id,
                reconnect: false,
                email,
                client_tag: "client_npc".into(),
            })
            .await;
        info!(conn_id, "npc: login requested");
    }

    let mut tick: u64 = 0;
    let mut active: u32 = min;
    let mut stuck_map: HashMap<u64, NpcStuckTracker> = HashMap::new();
    /// conn → (craft_key, remaining_cooldown_thinks)
    let mut craft_blacklist: HashMap<u64, HashMap<String, u32>> = HashMap::new();
    /// conn → (target_xy, best_dist_seen) for progress tracking
    let mut craft_progress: HashMap<u64, ((i32, i32), i32)> = HashMap::new();
    info!(
        min,
        max, think_period, radius, craft_radius, "npc scheduler started (eat+craft+activity log)"
    );

    loop {
        tokio::time::sleep(Duration::from_millis(200)).await;
        tick = tick.wrapping_add(1);
        activity.try_flush();

        let intents = counters.intents_applied.load(Ordering::Relaxed);
        let skips = counters.skip_ticks.load(Ordering::Relaxed);
        if skips > 0 && tick % 50 == 0 && active > min {
            active = active.saturating_sub(1).max(min);
            target_pop = active;
        } else if tick % 100 == 0 && active < max && intents < 100_000 {
            if active >= target_pop && active < max {
                active += 1;
                target_pop = active;
                let conn_id = NPC_CONN_BASE + (active - 1) as u64;
                let email = format!("npc-{active}@local");
                let _ = intent_tx
                    .send(NetIntent::Login {
                        conn_id,
                        reconnect: false,
                        email,
                        client_tag: "client_npc".into(),
                    })
                    .await;
                info!(conn_id, active, "npc: grown population");
            }
        }

        for i in 0..active {
            let conn_id = NPC_CONN_BASE + i as u64;
            if (tick as u32 + i) % think_period != 0 {
                continue;
            }
            let timer = ScopeTimer::start();
            let snap = player_views
                .read()
                .ok()
                .and_then(|g| g.get(&conn_id).cloned());
            let Some(p) = snap else {
                continue;
            };

            let tracker = stuck_map.entry(conn_id).or_default();

            // Death detection.
            if p.deleted {
                if !tracker.was_deleted {
                    tracker.was_deleted = true;
                    log_ev(
                        &activity,
                        conn_id,
                        &p,
                        NpcActivityKind::Death,
                        0,
                        0,
                        format!(
                            "age={:.1} food={:.1} reason=deleted_or_starved held={}",
                            p.age, p.food, p.held_id
                        ),
                    );
                    // Respawn: request new login same conn (sim may replace body).
                    let email = format!("npc-re-{}@local", i);
                    let _ = intent_tx.try_send(NetIntent::Login {
                        conn_id,
                        reconnect: false,
                        email,
                        client_tag: "client_npc".into(),
                    });
                }
                continue;
            }
            tracker.was_deleted = false;
            tracker.note_position(p.x, p.y);

            if p.moving {
                continue;
            }

            let profession = profession_for_index(i);
            let hungry = p.food < p.food_max * 0.45;
            let starving = p.food < p.food_max * 0.25;
            let food_need = if p.food_max > 0.1 {
                ((p.food_max - p.food) / p.food_max).clamp(0.0, 2.0)
            } else {
                0.0
            };

            let ally_count = {
                let views = player_views.read().unwrap();
                views
                    .values()
                    .filter(|o| {
                        !o.deleted
                            && o.conn_id != conn_id
                            && (o.x - p.x).abs().max((o.y - p.y).abs()) <= craft_radius
                    })
                    .count() as u32
            };

            let nearby = {
                let w = world.read().unwrap();
                collect_nearby(&w, p.x, p.y, craft_radius)
            };

            let mut acted = false;
            let mut detail = String::new();
            let mut kind = NpcActivityKind::Think;
            let mut game_ms = 200u32;

            // --- 1. Eat held food if hungry ---
            if !acted && hungry && p.held_id != 0 && food_at(&content, p.held_id) > 0 {
                // USE on self-tile fails transition → sim try_eat_held.
                if intent_tx
                    .try_send(NetIntent::Use {
                        conn_id,
                        x: p.x,
                        y: p.y,
                        id: None,
                        index: None,
                    })
                    .is_ok()
                {
                    kind = NpcActivityKind::Eat;
                    detail = format!("eat_held={}", p.held_id);
                    game_ms = 500;
                    acted = true;
                }
            }

            // --- 2. Seek / pick food on ground if hungry ---
            if !acted && (hungry || starving) {
                if let Some(food) = nearest_food(&content, &nearby, p.x, p.y) {
                    let dist = (food.x - p.x).abs().max((food.y - p.y).abs());
                    if dist <= 1 {
                        if intent_tx
                            .try_send(NetIntent::Use {
                                conn_id,
                                x: food.x,
                                y: food.y,
                                id: None,
                                index: None,
                            })
                            .is_ok()
                        {
                            kind = NpcActivityKind::SeekFood;
                            detail = format!(
                                "use_food id={} fv={}",
                                food.id,
                                food_at(&content, food.id)
                            );
                            game_ms = 500;
                            acted = true;
                        }
                    } else {
                        let step = {
                            let w = world.read().unwrap();
                            next_step(&w, p.x, p.y, food.x, food.y, &|nx, ny| {
                                is_walkable(&w, &content, nx, ny)
                            })
                        };
                        if let Some((dx, dy)) = step {
                            if intent_tx
                                .try_send(NetIntent::Move {
                                    conn_id,
                                    xs: p.x,
                                    ys: p.y,
                                    deltas: vec![(dx, dy)],
                                    seq: None,
                                })
                                .is_ok()
                            {
                                kind = NpcActivityKind::SeekFood;
                                detail = format!("walk_food id={} @{},{}", food.id, food.x, food.y);
                                game_ms = 250;
                                acted = true;
                            }
                        }
                    }
                }
            }

            // Decay craft blacklists.
            if let Some(bl) = craft_blacklist.get_mut(&conn_id) {
                bl.retain(|_, n| {
                    *n = n.saturating_sub(1);
                    *n > 0
                });
            }

            // --- 3. Bottom-up craft valuation (tools/food priority in craft_value) ---
            // When hungry, skip long walks (logs showed dist=36+ then starve/blacklist).
            if !acted && !starving {
                let max_craft_dist = if hungry {
                    12
                } else if p.food < p.food_max * 0.6 {
                    24
                } else {
                    craft_radius
                };
                let options = evaluate_nearby_crafts(
                    &content,
                    p.x,
                    p.y,
                    p.held_id,
                    &nearby,
                    profession,
                    hungry,
                    food_need,
                    ally_count,
                    DEFAULT_WALK_SPEED,
                    INTERACTION_SEC,
                    max_craft_dist.min(craft_radius),
                );
                let blocked = craft_blacklist.get(&conn_id).cloned().unwrap_or_default();
                let best = options.iter().find(|o| {
                    if o.net_score <= 0.1 {
                        return false;
                    }
                    let key = format!("{}+{}", o.actor_id, o.target_id);
                    if blocked.contains_key(&key) {
                        return false;
                    }
                    let (gx, gy) = if o.actor_id != 0
                        && o.actor_id != p.held_id
                        && (o.actor_x != p.x || o.actor_y != p.y)
                    {
                        (o.actor_x, o.actor_y)
                    } else {
                        (o.target_x, o.target_y)
                    };
                    let dist = (gx - p.x).abs().max((gy - p.y).abs());
                    dist <= max_craft_dist
                });
                if let Some(best) = best {
                    let key = format!("{}+{}", best.actor_id, best.target_id);
                    tracker.note_craft_key(key.clone());
                    log_ev(
                        &activity,
                        conn_id,
                        &p,
                        NpcActivityKind::CraftPlan,
                        0,
                        (best.time_cost_sec * 1000.0) as u32,
                        format!(
                            "plan {} score={:.1} time={:.1}s prod={}/{} in={:.1}",
                            key,
                            best.net_score,
                            best.time_cost_sec,
                            best.new_actor_id,
                            best.new_target_id,
                            best.input_value
                        ),
                    );

                    let (gx, gy) = if best.actor_id != 0
                        && best.actor_id != p.held_id
                        && (best.actor_x != p.x || best.actor_y != p.y)
                    {
                        (best.actor_x, best.actor_y)
                    } else {
                        (best.target_x, best.target_y)
                    };
                    let dist = (gx - p.x).abs().max((gy - p.y).abs());

                    // Abandon only if distance gets *worse*, or stuck long without improvement.
                    let abandon = if let Some(((tx, ty), best_d)) = craft_progress.get(&conn_id).copied()
                    {
                        if tx == gx && ty == gy {
                            if dist < best_d {
                                craft_progress.insert(conn_id, ((gx, gy), dist));
                                false
                            } else if dist > best_d + 2 {
                                true // wandered away
                            } else {
                                // Same or slight stall: allow more multi-step walks.
                                tracker.same_action_count >= 15
                            }
                        } else {
                            craft_progress.insert(conn_id, ((gx, gy), dist));
                            false
                        }
                    } else {
                        craft_progress.insert(conn_id, ((gx, gy), dist));
                        false
                    };
                    // Prefer USE when adjacent even if craft_loop flagged (arrival after walk spam).
                    if dist <= 1 {
                        if intent_tx
                            .try_send(NetIntent::Use {
                                conn_id,
                                x: best.target_x,
                                y: best.target_y,
                                id: None,
                                index: None,
                            })
                            .is_ok()
                        {
                            kind = NpcActivityKind::Craft;
                            detail = format!(
                                "use craft {}→{}/{} score={:.1}",
                                key, best.new_actor_id, best.new_target_id, best.net_score
                            );
                            game_ms = (best.time_cost_sec * 1000.0) as u32;
                            acted = true;
                            craft_progress.remove(&conn_id);
                            tracker.craft_ring.clear();
                            tracker.same_action_count = 0;
                        }
                    } else if abandon || tracker.craft_loop() {
                        craft_blacklist
                            .entry(conn_id)
                            .or_default()
                            .insert(key.clone(), 25);
                        craft_progress.remove(&conn_id);
                        tracker.craft_ring.clear();
                        log_ev(
                            &activity,
                            conn_id,
                            &p,
                            NpcActivityKind::StuckCycle,
                            0,
                            0,
                            format!("blacklist craft {key} dist={dist}"),
                        );
                    } else {
                        // Multi-step path toward goal (up to 6 tiles) for timed_movement.
                        let mut deltas = Vec::new();
                        {
                            let w = world.read().unwrap();
                            let mut cx = p.x;
                            let mut cy = p.y;
                            for _ in 0..6 {
                                if let Some((dx, dy)) = next_step(
                                    &w,
                                    cx,
                                    cy,
                                    gx,
                                    gy,
                                    &|nx, ny| is_walkable(&w, &content, nx, ny),
                                ) {
                                    deltas.push((dx, dy));
                                    cx += dx;
                                    cy += dy;
                                    if cx == gx && cy == gy {
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            }
                        }
                        // Fallback: greedy step if pathfind fails (blocked/no path).
                        if deltas.is_empty() {
                            let sdx = (gx - p.x).signum();
                            let sdy = (gy - p.y).signum();
                            let w = world.read().unwrap();
                            for (dx, dy) in [(sdx, 0), (0, sdy), (sdx, sdy)] {
                                if dx == 0 && dy == 0 {
                                    continue;
                                }
                                if is_walkable(&w, &content, p.x + dx, p.y + dy) {
                                    deltas.push((dx, dy));
                                    break;
                                }
                            }
                        }
                        if !deltas.is_empty()
                            && intent_tx
                                .try_send(NetIntent::Move {
                                    conn_id,
                                    xs: p.x,
                                    ys: p.y,
                                    deltas,
                                    seq: None,
                                })
                                .is_ok()
                        {
                            kind = NpcActivityKind::Craft;
                            detail = format!("walk_craft {} @{},{} dist={}", key, gx, gy, dist);
                            game_ms = 250;
                            acted = true;
                        }
                    }
                }
            }

            // --- 4. Feed kids (NURSE/FEED) when holding a baby + food ---
            // Holding baby is modeled as holding_player_id on sim; snapshot may not expose it.
            // Use SAY NURSE when holding food and another young player is adjacent.
            if !acted && p.held_id != 0 && food_at(&content, p.held_id) > 0 {
                let baby_near = {
                    let views = player_views.read().unwrap();
                    views.values().find(|o| {
                        !o.deleted
                            && o.conn_id != conn_id
                            && o.age < 3.0
                            && (o.x - p.x).abs().max((o.y - p.y).abs()) <= 1
                    }).map(|o| o.p_id)
                };
                if baby_near.is_some() {
                    if intent_tx
                        .try_send(NetIntent::Raw {
                            conn_id,
                            tag: "SAY".into(),
                            payload: "NURSE".into(),
                        })
                        .is_ok()
                    {
                        kind = NpcActivityKind::Feed;
                        detail = format!("nurse baby held_food={}", p.held_id);
                        game_ms = 500;
                        acted = true;
                    }
                }
            }

            // --- 5. Combat: HIT nearby non-allied low-food adults when hunter ---
            if !acted && matches!(profession, CraftProfession::Hunter) && !hungry {
                let prey = {
                    let views = player_views.read().unwrap();
                    views
                        .values()
                        .filter(|o| {
                            !o.deleted
                                && o.conn_id != conn_id
                                && o.age >= 14.0
                                && (o.x - p.x).abs().max((o.y - p.y).abs()) <= 2
                                && !o.email.contains("npc-forager")
                        })
                        .min_by_key(|o| (o.x - p.x).abs().max((o.y - p.y).abs()))
                        .map(|o| o.p_id)
                };
                if let Some(tid) = prey {
                    if intent_tx
                        .try_send(NetIntent::Raw {
                            conn_id,
                            tag: "SAY".into(),
                            payload: format!("HIT {tid}"),
                        })
                        .is_ok()
                    {
                        kind = NpcActivityKind::Combat;
                        detail = format!("hit p_id={tid}");
                        game_ms = 400;
                        acted = true;
                    }
                }
            }

            // --- 6. Explore ---
            if !acted {
                let step = {
                    let w = world.read().unwrap();
                    let candidates = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1)];
                    let mut chosen = None;
                    for &(dx, dy) in &candidates {
                        if is_walkable(&w, &content, p.x + dx, p.y + dy) {
                            chosen = Some((dx, dy));
                            break;
                        }
                    }
                    chosen.or_else(|| {
                        next_step(&w, p.x, p.y, p.x + 3, p.y, &|nx, ny| {
                            is_walkable(&w, &content, nx, ny)
                        })
                    })
                };
                if let Some((dx, dy)) = step {
                    if intent_tx
                        .try_send(NetIntent::Move {
                            conn_id,
                            xs: p.x,
                            ys: p.y,
                            deltas: vec![(dx, dy)],
                            seq: None,
                        })
                        .is_ok()
                    {
                        kind = NpcActivityKind::Explore;
                        detail = format!("explore {},{}", dx, dy);
                        game_ms = 250;
                        acted = true;
                    }
                }
            }

            if !acted {
                kind = NpcActivityKind::Error;
                detail = "no_action".into();
            }

            tracker.note_action(&detail);
            if tracker.is_stuck() {
                let why = if tracker.position_cycle() {
                    "pos_cycle"
                } else if tracker.craft_loop() {
                    "craft_loop"
                } else if tracker.same_pos_count >= 12 {
                    "pos_stuck"
                } else {
                    "action_spam"
                };
                log_ev(
                    &activity,
                    conn_id,
                    &p,
                    if tracker.position_cycle() || tracker.craft_loop() {
                        NpcActivityKind::StuckCycle
                    } else {
                        NpcActivityKind::Stuck
                    },
                    timer.elapsed().as_micros() as u32,
                    0,
                    format!(
                        "{} detail={} crafts={:?}",
                        why, detail, tracker.craft_ring
                    ),
                );
                // Nudge: random step to break cycle.
                let _ = intent_tx.try_send(NetIntent::Move {
                    conn_id,
                    xs: p.x,
                    ys: p.y,
                    deltas: vec![(1, 0), (0, 1)],
                    seq: None,
                });
                tracker.same_pos_count = 0;
                tracker.same_action_count = 0;
            }

            let cpu = timer.elapsed().as_micros() as u32;
            log_ev(&activity, conn_id, &p, kind, cpu, game_ms, detail);
            debug!(conn_id, ?kind, "npc think");

            counters
                .ai_cpu_us
                .fetch_add(cpu as u64, Ordering::Relaxed);
            counters.ai_thinks.fetch_add(1, Ordering::Relaxed);
            let dt_ms = 200u64.saturating_mul(active as u64).max(200);
            counters
                .ai_sim_time_ms
                .fetch_add(dt_ms / active.max(1) as u64, Ordering::Relaxed);
        }
    }
}
