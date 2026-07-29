//! Build-time wire for **BREASTFEED-EDGES / nurse_edges**.
//!
//! Pure-Rust idempotent patches of `src/lib.rs` (exports, continuous nurse,
//! NURSE drain, HOLD doBaby edges, live tests). Also runs Python apply when present.

use std::path::Path;
use std::process::Command;

fn normalize_nl(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn restore_nl(s: String, crlf: bool) -> String {
    if crlf {
        s.replace('\n', "\r\n")
    } else {
        s
    }
}

pub fn breastfeed_edges_wired(lib: &str) -> bool {
    lib.contains("get_max_child_feeding")
        && lib.contains("BREASTFEED-EDGES")
        && lib.contains("can_pickup_breastfeed_age")
        && lib.contains("PICKUP_EXHAUSTION_GAIN")
        && lib.contains("nurse_hits_heal")
        && lib.contains("should_set_follow_on_hold")
}

fn replace_once(text: &mut String, old: &str, new: &str) -> bool {
    if let Some(i) = text.find(old) {
        text.replace_range(i..i + old.len(), new);
        true
    } else {
        false
    }
}

/// Idempotent patch of `src/lib.rs`.
pub fn patch_breastfeed_edges(src_dir: &Path, workspace: &Path) -> bool {
    let lib_path = src_dir.join("lib.rs");
    let Ok(raw) = std::fs::read_to_string(&lib_path) else {
        return false;
    };
    if breastfeed_edges_wired(&raw) {
        // Still try docs python if docs not updated — skip
        let _ = workspace;
        return true;
    }

    // Prefer Python apply when available (full parity + tests).
    let py = src_dir.join("_apply_breastfeed_edges.py");
    if py.exists() {
        let status = Command::new("python")
            .arg(&py)
            .current_dir(src_dir)
            .status()
            .or_else(|_| {
                Command::new("python3")
                    .arg(&py)
                    .current_dir(src_dir)
                    .status()
            });
        if let Ok(s) = status {
            if s.success() {
                if let Ok(t) = std::fs::read_to_string(&lib_path) {
                    if breastfeed_edges_wired(&t) {
                        patch_docs(workspace);
                        return true;
                    }
                }
            }
        }
    }

    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    // --- pub use feed ---
    let old_use = "pub use feed::{\n\
    apply_feed_amounts, breastfeed_tick, can_breastfeed, can_feed, name_looks_like_food,\n\
    pickup_feed_amounts, FEED_RANGE, FOOD_RESTORE_FACTOR_WHILE_FEEDING,\n\
    MAX_CHILD_AGE_BREAST_FEEDING, PICKUP_FEEDING_FOOD_RESTORE,\n\
};";
    let new_use = "pub use feed::{\n\
    apply_feed_amounts, breastfeed_tick, can_breastfeed, can_feed, can_nurse_age,\n\
    can_pickup_breastfeed_age, can_pickup_player_ages, get_max_child_feeding,\n\
    name_looks_like_food, nurse_hits_heal, pickup_feed_amounts, should_set_follow_on_hold,\n\
    FEED_RANGE, FOOD_RESTORE_FACTOR_WHILE_FEEDING, MAX_AGE_FOR_PICKUP_FROM_OTHERS,\n\
    MAX_CHILD_AGE_BREAST_FEEDING, MIN_MAX_CHILD_FEEDING, NURSE_HITS_HEAL_PER_SEC,\n\
    PICKUP_EXHAUSTION_GAIN, PICKUP_FEEDING_FOOD_RESTORE,\n\
};";
    if replace_once(&mut t, old_use, new_use) {
        changed = true;
    } else if !t.contains("get_max_child_feeding") {
        if let Some(start) = t.find("pub use feed::{") {
            if let Some(rel_end) = t[start..].find("};") {
                let end = start + rel_end + 2;
                t.replace_range(start..end, new_use);
                changed = true;
            }
        }
    }

    // --- NURSE bare hands drain ---
    let old_nurse = "                            if to_baby > 0.0 {\n\
                                if let Some(feeder) = state.players.get_mut(&conn_id) {\n\
                                    feeder.food = (feeder.food - from_m).max(0.0);\n\
                                }\n\
                                if let Some(tp) = state.players.get_mut(&t_conn) {\n\
                                    tp.food = (tp.food + to_baby).min(tp.food_max);\n\
                                }";
    let new_nurse = "                            if to_baby > 0.0 {\n\
                                // BREASTFEED-EDGES: Haxe food_store -= half without floor\n\
                                if let Some(feeder) = state.players.get_mut(&conn_id) {\n\
                                    feeder.food -= from_m;\n\
                                }\n\
                                if let Some(tp) = state.players.get_mut(&t_conn) {\n\
                                    let cap = get_max_child_feeding(tp.food_max);\n\
                                    tp.food = (tp.food + to_baby).min(cap);\n\
                                }";
    if !t.contains("BREASTFEED-EDGES: Haxe food_store -= half without floor")
        && replace_once(&mut t, old_nurse, new_nurse)
    {
        changed = true;
    }

    // --- Continuous breastfeed ---
    let old_cont = "            let (to_baby, from_m) = breastfeed_tick(dt, FOOD_USE_PER_SEC, b_food, b_max);\n\
            if to_baby <= 0.0 {\n\
                continue;\n\
            }\n\
            if let Some(m) = state.players.get_mut(&mother_conn) {\n\
                m.food = (m.food - from_m).max(0.0);\n\
            }\n\
            if let Some(b) = state.players.get_mut(&baby_conn) {\n\
                b.food = (b.food + to_baby).min(b.food_max);\n\
            }\n\
            // Heal baby hits slowly while nursing (Haxe hits -= dt * 0.2).\n\
            if let Some(s) = state.combat.stats.get_mut(&baby_p_id) {\n\
                if s.hits > 0.0 {\n\
                    s.hits = (s.hits - dt * 0.2).max(0.0);\n\
                }\n\
            }";
    let new_cont = "            // BREASTFEED-EDGES: factor/cap via breastfeed_tick; hits heal even if full.\n\
            // Haxe: TimeHelper breast-feed L950-973\n\
            let (to_baby, from_m) = breastfeed_tick(dt, FOOD_USE_PER_SEC, b_food, b_max);\n\
            if to_baby > 0.0 {\n\
                if let Some(m) = state.players.get_mut(&mother_conn) {\n\
                    // Haxe foodDecay += food/2 (may go negative; stops next tick)\n\
                    m.food -= from_m;\n\
                }\n\
                if let Some(b) = state.players.get_mut(&baby_conn) {\n\
                    let cap = get_max_child_feeding(b.food_max);\n\
                    b.food = (b.food + to_baby).min(cap);\n\
                }\n\
            }\n\
            // Heal baby hits slowly while nursing (Haxe hits -= dt * 0.2) even at cap.\n\
            if let Some(s) = state.combat.stats.get_mut(&baby_p_id) {\n\
                if s.hits > 0.0 {\n\
                    s.hits = nurse_hits_heal(s.hits, dt);\n\
                }\n\
            }";
    if !t.contains("BREASTFEED-EDGES: factor/cap via breastfeed_tick")
        && replace_once(&mut t, old_cont, new_cont)
    {
        changed = true;
    }

    // --- HOLD block ---
    if !t.contains("BREASTFEED-EDGES: strict < for pickup") {
        if let Some(start) = t.find("// HOLD <p_id> — pick up adjacent baby") {
            if let Some(rel_end) = t[start..].find("// PUTDOWN / DROPBABY") {
                let end = start + rel_end;
                let new_hold = HOLD_BLOCK;
                t.replace_range(start..end, new_hold);
                changed = true;
            }
        } else if let Some(start) = t.find("// HOLD <p_id> — pick up adjacent child") {
            let _ = start; // already new
        }
    }

    // Marker on continuous block
    if !t.contains("BREASTFEED-EDGES nurse_edges") {
        if replace_once(
            &mut t,
            "// Continuous breast-feeding (Haxe TimeHelper isHoldingChildInBreastFeedingAgeAndCanFeed).",
            "// Continuous breast-feeding (Haxe TimeHelper isHoldingChildInBreastFeedingAgeAndCanFeed).\n    // BREASTFEED-EDGES nurse_edges",
        ) {
            changed = true;
        }
    }

    // Live tests
    if !t.contains("breastfeed_edges_continuous_factor_and_hits") {
        let anchor =
            "    /// SAY NURSE / FEED while holding baby transfers held food to the baby.\n    #[test]\n    fn say_nurse_feeds_held_baby() {";
        if t.contains(anchor) {
            let insert = format!("{LIVE_TESTS}\n{anchor}");
            if replace_once(&mut t, anchor, &insert) {
                changed = true;
            }
        }
    }

    if changed {
        let out = restore_nl(t, crlf);
        if std::fs::write(&lib_path, out).is_err() {
            return false;
        }
    }

    patch_docs(workspace);

    std::fs::read_to_string(&lib_path)
        .map(|s| breastfeed_edges_wired(&s))
        .unwrap_or(false)
}

fn patch_docs(workspace: &Path) {
    let docs = workspace.join("docs/port");
    let todo = docs.join("TODO_PORT.md");
    let matrix = docs.join("FILE_MATRIX.md");
    let call = docs.join("CALL_INDEX.md");
    let queue = docs.join("QUEUE.md");

    if let Ok(mut t) = std::fs::read_to_string(&todo) {
        if !t.contains("BREASTFEED-EDGES nurse_edges") {
            t = t.replacen(
                "Last updated: **2026-07-26** (ALLY-STRENGTH ally_combat)",
                "Last updated: **2026-07-26** (BREASTFEED-EDGES nurse_edges)",
                1,
            );
            t = t.replacen(
                "- [ ] Breastfeeding edge cases  \n",
                "- [x] **BREASTFEED-EDGES nurse_edges** — factor 10 + `getMaxChildFeeding` + age 6 boundary + hits-at-cap + HOLD exhaustion/follow/happy + pickup age < 6  \n",
                1,
            );
            // recommended next
            if !t.contains("~~**BREASTFEED-EDGES**~~") {
                t = t.replacen(
                    "14. ~~**ALLY-STRENGTH ally_combat**~~ **DONE** source HIT allyFactor + anger + unarmed first-hit/exile + USE gate (default 0); residual PrestigeCostPerDamageForAlly  \n",
                    "14. ~~**ALLY-STRENGTH ally_combat**~~ **DONE** source HIT allyFactor + anger + unarmed first-hit/exile + USE gate (default 0); residual PrestigeCostPerDamageForAlly  \n\
15. ~~**BREASTFEED-EDGES nurse_edges**~~ **DONE** FoodRestoreFactor=10; getMaxChildFeeding; age6; hits heal at cap; HOLD exhaustion/follow/happy; residual multi-server twin heart  \n",
                    1,
                );
            }
            if !t.contains("| 2026-07-26 | **BREASTFEED-EDGES") {
                t = t.replacen(
                    "## Changelog (port docs)\n\n| Date | Change |\n|------|--------|\n",
                    "## Changelog (port docs)\n\n| Date | Change |\n|------|--------|\n\
| 2026-07-26 | **BREASTFEED-EDGES nurse_edges**: `feed.rs` FoodRestoreFactor=10; `get_max_child_feeding`=max(4,food_max); continuous age<=6 / pickup age<6; nurse hits heal at cap; mother food no floor; HOLD exhaustion+follow+happy PE; pickup age to 10 with +1yr; tests `feed::*` + `breastfeed_edges_*` |\n",
                    1,
                );
            }
            let _ = std::fs::write(&todo, t);
        }
    }

    if let Ok(mut t) = std::fs::read_to_string(&matrix) {
        if !t.contains("BREASTFEED-EDGES") {
            t = t.replacen(
                "Last reviewed: **2026-07-26** (ALLY-STRENGTH ally_combat)",
                "Last reviewed: **2026-07-26** (BREASTFEED-EDGES nurse_edges)",
                1,
            );
            t = t.replacen(
                "| GPI-BABY / **FERTILITY-TWINS** | baby/hold + fertile + twin wait | **DONE** (core) | pure `is_fertile`/`can_birth_full`/`TwinWaitQueue`; BIRTH/GESTATE/nurse/HOLD/mother-pick female gate; LOGIN→TWINJOIN; SAY TWINJOIN/?TWINWAIT/?TWINS; disconnect leave; residual twin death heart-link / multi-server sockets / ObjectData.male |\n",
                "| GPI-BABY / **FERTILITY-TWINS** | baby/hold + fertile + twin wait | **DONE** (core) | pure `is_fertile`/`can_birth_full`/`TwinWaitQueue`; BIRTH/GESTATE/nurse/HOLD/mother-pick female gate; LOGIN→TWINJOIN; SAY TWINJOIN/?TWINWAIT/?TWINS; disconnect leave; residual twin death heart-link / multi-server sockets / ObjectData.male |\n\
| **BREASTFEED-EDGES** / nurse_edges | continuous nurse + HOLD edges | **DONE** (core) | `feed.rs` factor 10 + getMaxChildFeeding; age6 nurse/pickup split; hits heal at cap; HOLD exhaustion/follow/happy; residual: content ObjectData.male for fertile gate |\n",
                1,
            );
            let _ = std::fs::write(&matrix, t);
        }
    }

    if let Ok(mut t) = std::fs::read_to_string(&call) {
        if !t.contains("get_max_child_feeding") {
            let entry = "\n### BREASTFEED-EDGES / nurse_edges\n\
| Symbol | Module | Notes |\n\
|--------|--------|-------|\n\
| `can_breastfeed` / `breastfeed_tick` / `pickup_feed_amounts` | `ol-sim/src/feed.rs` | continuous + HOLD restore |\n\
| `get_max_child_feeding` | same | Haxe `getMaxChildFeeding` max(4, food_max) |\n\
| `can_nurse_age` / `can_pickup_breastfeed_age` | same | age<=6 continuous; age<6 pickup |\n\
| `can_pickup_player_ages` | same | HOLD age <10 + carrier >= target+1 |\n\
| `nurse_hits_heal` / `should_set_follow_on_hold` | same | TimeHelper hits; doBaby follow |\n\
| `PICKUP_EXHAUSTION_GAIN` / `FOOD_RESTORE_FACTOR_WHILE_FEEDING` | same | 0.2 / 10 |\n\
| HOLD / continuous nurse wire | `lib.rs` | exhaustion, follow, happy PE, no mother food floor |\n\
| Tests | `feed::*` / `breastfeed_edges_*` | pure + live |\n";
            t.push_str(entry);
            let _ = std::fs::write(&call, t);
        }
    }

    if let Ok(mut t) = std::fs::read_to_string(&queue) {
        if t.contains("| `BREASTFEED-EDGES` | workflow (new) | nurse_edges |") {
            t = t.replacen(
                "| `BREASTFEED-EDGES` | workflow (new) | nurse_edges |\n",
                "",
                1,
            );
            t = t.replacen(
                "| 44 | `BREASTFEED-EDGES` | nurse_edges | **running** |\n",
                "| 44 | ~~`BREASTFEED-EDGES`~~ | nurse_edges | **DONE** factor10 + HOLD edges |\n",
                1,
            );
            let _ = std::fs::write(&queue, t);
        }
    }
}

const HOLD_BLOCK: &str = r#"// HOLD <p_id> — pick up adjacent child (Haxe doBaby / BREASTFEED-EDGES).
        // Age: target < MaxAgeForAllowingClothAndPrickupFromOthers (10) and
        // carrier.age >= target.age + 1; free hands (can_hold_baby).
        if upper.starts_with("HOLD ") || upper == "HOLD" {
            let baby_p_id: i32 = text
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let mother_p_id = p.p_id;
            let mother_age = p.age;
            let (mx, my) = (p.x, p.y);
            let ok = if baby_p_id != 0
                && state
                    .players
                    .get(&conn_id)
                    .map(|pl| pl.can_hold_baby())
                    .unwrap_or(false)
            {
                // Haxe doBaby: not deleted, free of heldBy, age gates, adjacent.
                state.players.values().any(|pl| {
                    pl.p_id == baby_p_id
                        && !pl.deleted
                        && pl.held_by == 0
                        && can_pickup_player_ages(mother_age, pl.age)
                        && (pl.x - mx).abs().max((pl.y - my).abs()) <= 1
                })
            } else {
                false
            };
            if ok {
                // Apply links after the immutable borrow ends.
                if let Some(pl) = state.players.get_mut(&conn_id) {
                    pl.start_holding(baby_p_id);
                    // Haxe: exhaustion += PickupExhaustionGain
                    pl.exhaustion += PICKUP_EXHAUSTION_GAIN;
                }
                if let Some(baby) = state.players.values_mut().find(|pl| pl.p_id == baby_p_id) {
                    baby.held_by = mother_p_id;
                    baby.x = mx;
                    baby.y = my;
                }
                // Haxe doBaby: pickup feeding when fertile mother + age < MaxChildAge.
                // FERTILITY-TWINS: full isFertile (female + age)
                // BREASTFEED-EDGES: strict < for pickup (not <= continuous)
                let mother_fertile = state
                    .players
                    .get(&conn_id)
                    .map(|pl| player_is_fertile(state, pl))
                    .unwrap_or(false);
                let baby_age = state
                    .players
                    .values()
                    .find(|pl| pl.p_id == baby_p_id)
                    .map(|b| b.age)
                    .unwrap_or(99.0);
                if mother_fertile && can_pickup_breastfeed_age(baby_age) {
                    let (b_food, b_max) = state
                        .players
                        .values()
                        .find(|pl| pl.p_id == baby_p_id)
                        .map(|b| (b.food, b.food_max))
                        .unwrap_or((0.0, 20.0));
                    let (to_baby, from_m) = pickup_feed_amounts(b_food, b_max);
                    if to_baby > 0.0 {
                        if let Some(pl) = state.players.get_mut(&conn_id) {
                            pl.food -= from_m; // Haxe no floor
                        }
                        if let Some(baby) =
                            state.players.values_mut().find(|pl| pl.p_id == baby_p_id)
                        {
                            let cap = get_max_child_feeding(baby.food_max);
                            baby.food = (baby.food + to_baby).min(cap);
                        }
                        info!(
                            conn_id,
                            baby_p_id,
                            to_baby,
                            from_m,
                            "sim: HOLD pickup breastfeed"
                        );
                    }
                }
                // Haxe: setFollowPlayer when no follow or non-fertile follow + fertile picker
                {
                    let has_follow = state.social.following.contains_key(&baby_p_id);
                    let follow_fertile = state
                        .social
                        .following
                        .get(&baby_p_id)
                        .and_then(|&fid| {
                            state
                                .players
                                .values()
                                .find(|pl| pl.p_id == fid)
                                .map(|pl| player_is_fertile(state, pl))
                        })
                        .unwrap_or(false);
                    if should_set_follow_on_hold(has_follow, follow_fertile, mother_fertile) {
                        let _ = state.social.set_follow(baby_p_id, mother_p_id);
                    }
                }
                // Haxe: heldPlayer.doEmote(Emote.happy) when can breastfeed after hold
                let m_food = state
                    .players
                    .get(&conn_id)
                    .map(|pl| pl.food)
                    .unwrap_or(0.0);
                let m_age = state
                    .players
                    .get(&conn_id)
                    .map(|pl| pl.age)
                    .unwrap_or(0.0);
                if can_breastfeed(m_age, m_food, mother_fertile, baby_age, true) {
                    let near = nearby_conn_ids(state, mx, my, NEARBY_RANGE);
                    let pe = format_player_emot(baby_p_id, 0).into_bytes(); // Emote.happy = 0
                    send_nearby(outbound, &near, pe);
                }
                let line = format!("{} HOLD {baby_p_id} OK", mother_p_id);
                send_ps_reply(outbound, conn_id, &line);
                info!(conn_id, baby_p_id, "sim: HOLD baby");
            } else {
                let line = format!("{} HOLD FAIL", mother_p_id);
                send_ps_reply(outbound, conn_id, &line);
            }
            return;
        }
        "#;

const LIVE_TESTS: &str = r#"
    /// BREASTFEED-EDGES: continuous nurse factor 10 + hits heal at food cap.
    #[test]
    fn breastfeed_edges_continuous_factor_and_hits() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let _rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let mother = spawn_player(&mut state, 1, "mom@bf");
        state.players.get_mut(&1).unwrap().age = 20.0;
        state.players.get_mut(&1).unwrap().food = 10.0;
        state.players.get_mut(&1).unwrap().display_object_id = 19;
        let baby_id = spawn_child(&mut state, 1).expect("baby");
        let baby_conn = state
            .players
            .iter()
            .find(|(_, p)| p.p_id == baby_id)
            .map(|(&c, _)| c)
            .expect("baby conn");
        {
            let m = state.players.get_mut(&1).unwrap();
            m.start_holding(baby_id);
            m.food = 10.0;
        }
        state.players.get_mut(&baby_conn).unwrap().held_by = mother;
        state.players.get_mut(&baby_conn).unwrap().food = 5.0;
        state.players.get_mut(&baby_conn).unwrap().food_max = 20.0;
        state.players.get_mut(&baby_conn).unwrap().age = 1.0;
        state.combat.apply_hits(baby_id, 1.0, 0);
        let hits_before = state.combat.hits_of(baby_id);
        assert!(hits_before > 0.0);

        let food_before = state.players.get(&baby_conn).unwrap().food;
        let m_food_before = state.players.get(&1).unwrap().food;
        tick_vitals(&mut state, &counters, &hub, 1.0);
        let food_after = state.players.get(&baby_conn).unwrap().food;
        let m_food_after = state.players.get(&1).unwrap().food;
        let gained = food_after - food_before;
        assert!(
            gained > 0.5,
            "baby should gain ~1 food from factor-10 nurse, gained {gained}"
        );
        assert!(
            m_food_after < m_food_before - 0.2,
            "mother should lose half of transfer, {m_food_before} -> {m_food_after}"
        );
        let hits_after = state.combat.hits_of(baby_id);
        assert!(
            hits_after < hits_before,
            "hits heal while nursing: {hits_before} -> {hits_after}"
        );
    }

    /// BREASTFEED-EDGES: HOLD pickup restore + exhaustion + follow + age < 6.
    #[test]
    fn breastfeed_edges_hold_pickup_exhaustion_follow() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        let mother = spawn_player(&mut state, 1, "mom@hold");
        state.players.get_mut(&1).unwrap().age = 20.0;
        state.players.get_mut(&1).unwrap().food = 10.0;
        state.players.get_mut(&1).unwrap().display_object_id = 19;
        let baby_id = spawn_child(&mut state, 1).expect("baby");
        let baby_conn = state
            .players
            .iter()
            .find(|(_, p)| p.p_id == baby_id)
            .map(|(&c, _)| c)
            .expect("baby conn");
        {
            let b = state.players.get_mut(&baby_conn).unwrap();
            b.age = 1.0;
            b.food = 0.0;
            b.x = 0;
            b.y = 0;
            b.held_by = 0;
        }
        state.players.get_mut(&1).unwrap().x = 0;
        state.players.get_mut(&1).unwrap().y = 0;
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("HOLD {baby_id}"),
            },
        );
        let m = state.players.get(&1).unwrap();
        assert_eq!(m.holding_player_id, baby_id, "mother holds baby");
        assert!(
            (m.exhaustion - PICKUP_EXHAUSTION_GAIN).abs() < 1e-5,
            "exhaustion gain, got {}",
            m.exhaustion
        );
        let b = state.players.get(&baby_conn).unwrap();
        assert_eq!(b.held_by, mother);
        assert!(
            b.food >= PICKUP_FEEDING_FOOD_RESTORE - 0.01,
            "pickup feed restore, got {}",
            b.food
        );
        assert_eq!(
            state.social.following.get(&baby_id),
            Some(&mother),
            "baby follows mother"
        );
    }

    /// BREASTFEED-EDGES: age == 6 continuous OK; pickup age == 6 no restore.
    #[test]
    fn breastfeed_edges_age_six_boundary() {
        assert!(can_nurse_age(6.0));
        assert!(!can_pickup_breastfeed_age(6.0));
        assert!(can_pickup_player_ages(20.0, 5.0));
        assert!(!can_pickup_player_ages(20.0, 10.0));
        let (to, _) = breastfeed_tick(1.0, FOOD_USE_PER_SEC, 0.0, 20.0);
        assert!((to - FOOD_RESTORE_FACTOR_WHILE_FEEDING * FOOD_USE_PER_SEC).abs() < 1e-5);
        assert!((get_max_child_feeding(2.0) - 4.0).abs() < 1e-5);
    }
"#;
