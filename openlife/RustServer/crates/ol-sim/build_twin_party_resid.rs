//! Build-time wire for **TWIN-PARTY-RESID / twin_wait_edges**.
//!
//! Same-server residual: TwinHeartLinks murder→broken-heart, ObjectData.male,
//! wait-queue timeout PS. Multi-server peers stay parked.
//!
//! Prefer Python apply (`src/_apply_twin_party_resid.py`); pure-Rust fallback
//! patches the critical lib.rs + content symbols so the crate still compiles.
//!
//! Also piggybacks **AI-FOLLOW-WALK** apply (`_apply_ai_follow_walk.py`) so continuous
//! follow wire lands even before build_ai_follow_walk is registered in build.rs.
//! Also piggybacks **MOVE-MIDPATH** (`build_move_midpath_piggy.inc.rs`) calculateNewPos recon.

use std::path::Path;
use std::process::Command;

/// True when core residual surfaces are present in lib.rs.
pub fn twin_party_resid_wired(lib: &str) -> bool {
    lib.contains("TwinHeartLinks")
        && lib.contains("mod twin_heart")
        && lib.contains("pub twin_heart: TwinHeartLinks")
        && lib.contains("apply_twin_heart_link_on_murder")
}

fn fix_twins_typo(src_dir: &Path) {
    let twins_path = src_dir.join("twins.rs");
    if let Ok(t) = std::fs::read_to_string(&twins_path) {
        if t.contains("Vec.with_capacity") {
            let t2 = t.replace("Vec.with_capacity", "Vec::with_capacity");
            let _ = std::fs::write(&twins_path, t2);
            println!("cargo:warning=TWIN-PARTY-RESID: fixed twins.rs Vec.with_capacity typo");
        }
    }
}

fn normalize_nl(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

fn restore_nl(s: String, crlf: bool) -> String {
    if crlf {
        s.replace('\n', "\r\n")
    } else {
        s
    }
}

fn pure_content_male_patch(workspace: &Path) {
    let content_lib = workspace.join("crates/ol-content/src/lib.rs");
    let content_tail = workspace.join("crates/ol-content/src/lib_tail.inc.rs");
    if let Ok(raw) = std::fs::read_to_string(&content_lib) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if !t.contains("pub male: bool,") {
            if t.contains("    pub wound_factor: f32,\n}") {
                t = t.replacen(
                    "    pub wound_factor: f32,\n}",
                    "    pub wound_factor: f32,\n\
    /// Haxe `ObjectData.male` — person sex (`true` = male). Default false.\n\
    /// // Haxe: ObjectData.male\n\
    pub male: bool,\n}",
                    1,
                );
                t = t.replacen(
                    "            wound_factor: 0.5,\n        }",
                    "            wound_factor: 0.5,\n            male: false,\n        }",
                    1,
                );
                let _ = std::fs::write(&content_lib, restore_nl(t, crlf));
                println!("cargo:warning=TWIN-PARTY-RESID: ObjectDef.male field added");
            }
        }
    }
    if let Ok(raw) = std::fs::read_to_string(&content_tail) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        let mut c = false;
        if !t.contains("strip_prefix(\"male=\")") {
            let old = "            } else if let Some(rest) = part.strip_prefix(\"floor=\") {\n\
                // floor=1 — floor-only objects (roads, stone floors); not ground placeables.\n\
                def.floor = rest.starts_with('1') || rest.eq_ignore_ascii_case(\"true\");\n";
            let new = "            } else if let Some(rest) = part.strip_prefix(\"male=\") {\n\
                // Haxe ObjectData.male — person sex (0/1 or true/false).\n\
                // TWIN-PARTY-RESID / ObjectData.male\n\
                def.male = rest.starts_with('1') || rest.eq_ignore_ascii_case(\"true\");\n\
            } else if let Some(rest) = part.strip_prefix(\"floor=\") {\n\
                // floor=1 — floor-only objects (roads, stone floors); not ground placeables.\n\
                def.floor = rest.starts_with('1') || rest.eq_ignore_ascii_case(\"true\");\n";
            if t.contains(old) {
                t = t.replacen(old, new, 1);
                c = true;
            }
        }
        if t.contains("Ok(ParsedObject { def, person: 0 })") {
            t = t.replacen(
                "pub fn load_object_file_full(path: &Path) -> Result<ParsedObject, ContentError> {\n\
    let def = load_object_file(path)?;\n\
    Ok(ParsedObject { def, person: 0 })\n\
}",
                "pub fn load_object_file_full(path: &Path) -> Result<ParsedObject, ContentError> {\n\
    let text = fs::read_to_string(path)?;\n\
    let def = load_object_file(path)?;\n\
    let person = parse_person_from_text(&text);\n\
    Ok(ParsedObject { def, person })\n\
}",
                1,
            );
            c = true;
        }
        if c {
            let _ = std::fs::write(&content_tail, restore_nl(t, crlf));
            println!("cargo:warning=TWIN-PARTY-RESID: male= parse + person full load");
        }
    }
}

/// Minimal pure-Rust wire so twin_party_live.inc.rs compiles.
fn pure_rust_lib_patch(src_dir: &Path) -> bool {
    let lib_path = src_dir.join("lib.rs");
    let Ok(raw) = std::fs::read_to_string(&lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("mod twin_heart;") {
        if t.contains("mod twins;") {
            t = t.replacen("mod twins;", "mod twins;\nmod twin_heart;", 1);
            changed = true;
        }
    }

    if !t.contains("pub use twin_heart::") {
        let insert = "pub use twin_heart::{\n\
    format_twin_heart_ps, format_twin_timeout_ps, format_twin_wait_ps_code,\n\
    is_murder_death_reason, TwinHeartLinks, BROKEN_HEART_WOUND_STACKS,\n\
    TWIN_WAIT_TIMEOUT_SECS,\n\
};\n";
        if t.contains("pub use twins::{") {
            t = t.replacen("pub use twins::{", &format!("{insert}pub use twins::{{"), 1);
            changed = true;
        }
    }

    if !t.contains("pub twin_heart: TwinHeartLinks") {
        if let Some(idx) = t.find("pub twin_wait: TwinWaitQueue,") {
            let after = &t[idx..];
            if let Some(rel) = after.find("pub twin_wait: TwinWaitQueue,\n") {
                let abs = idx + rel;
                let end = abs + "pub twin_wait: TwinWaitQueue,\n".len();
                if !t[end..].starts_with("    /// Same-server twin party heart-link") {
                    let insert = "pub twin_wait: TwinWaitQueue,\n\
    /// Same-server twin party heart-link after birth (murder → broken heart).\n\
    /// // OHOL twins plan #10; TWIN-PARTY-RESID\n\
    pub twin_heart: TwinHeartLinks,\n";
                    t = format!("{}{}{}", &t[..abs], insert, &t[end..]);
                    changed = true;
                }
            }
        }
    }

    if !t.contains("twin_heart: TwinHeartLinks::default()") {
        if t.contains("twin_wait: TwinWaitQueue::default(),") {
            t = t.replacen(
                "twin_wait: TwinWaitQueue::default(),\n",
                "twin_wait: TwinWaitQueue::default(),\n\
            twin_heart: TwinHeartLinks::default(),\n",
                1,
            );
            changed = true;
        }
    }

    let old_gest = "pub use gestation_tick::{due_mothers, format_twin_party_ready, format_twin_wait_ps};";
    let new_gest = "pub use gestation_tick::{due_mothers, format_twin_party_ready, format_twin_wait_ps, poll_twin_timeouts};";
    if t.contains(old_gest) {
        t = t.replacen(old_gest, new_gest, 1);
        changed = true;
    }

    if !t.contains("apply_twin_heart_link_on_murder") {
        let needle = "                    apply_death_inheritance(state, target_id);\n\
                    counters.deaths.fetch_add(1, Ordering::Relaxed);\n\
                    state.push_event(format_death_event_tag(target_id, &death_reason));\n\
                    state.afk.remove(target_id);\n\
                    let line = format!(\"{} KILLED {} legal={}\", killer_id, target_id, legal);\n";
        let repl = "                    apply_death_inheritance(state, target_id);\n\
                    // TWIN-PARTY-RESID: murder of twin → broken-heart siblings\n\
                    if is_murder_death_reason(&death_reason) {\n\
                        apply_twin_heart_link_on_murder(state, outbound, target_id);\n\
                    }\n\
                    counters.deaths.fetch_add(1, Ordering::Relaxed);\n\
                    state.push_event(format_death_event_tag(target_id, &death_reason));\n\
                    state.afk.remove(target_id);\n\
                    let line = format!(\"{} KILLED {} legal={}\", killer_id, target_id, legal);\n";
        if t.contains(needle) {
            t = t.replacen(needle, repl, 1);
            changed = true;
        }
        let needle2 = "                        apply_death_inheritance(state, target_id);\n\
                        counters.deaths.fetch_add(1, Ordering::Relaxed);\n\
                        state.push_event(format_death_event_tag(target_id, &death_reason));\n\
                        state.afk.remove(target_id);\n\
                        let line = format!(\n\
                            \"{} HIT {} KILL legal={} dmg={:.1}\",\n\
                            killer_id, target_id, legal, dmg\n\
                        );\n";
        let repl2 = "                        apply_death_inheritance(state, target_id);\n\
                        // TWIN-PARTY-RESID: murder of twin → broken-heart siblings\n\
                        if is_murder_death_reason(&death_reason) {\n\
                            apply_twin_heart_link_on_murder(state, outbound, target_id);\n\
                        }\n\
                        counters.deaths.fetch_add(1, Ordering::Relaxed);\n\
                        state.push_event(format_death_event_tag(target_id, &death_reason));\n\
                        state.afk.remove(target_id);\n\
                        let line = format!(\n\
                            \"{} HIT {} KILL legal={} dmg={:.1}\",\n\
                            killer_id, target_id, legal, dmg\n\
                        );\n";
        if t.contains(needle2) {
            t = t.replacen(needle2, repl2, 1);
            changed = true;
        }
    }

    if !t.contains("TWIN-PARTY-RESID: evict twin waiters") {
        let needle = "        // TWIN-MULTI-SERVER: age peer pongs so ?TWINS shows @- after timeout (no sockets yet).\n\
        let _ = state\n\
            .twins\n\
            .clear_stale_pongs(state.sim_time, DEFAULT_PEER_STALE_SECS);\n";
        let repl = "        // TWIN-MULTI-SERVER: age peer pongs so ?TWINS shows @- after timeout (no sockets yet).\n\
        let _ = state\n\
            .twins\n\
            .clear_stale_pongs(state.sim_time, DEFAULT_PEER_STALE_SECS);\n\
\n\
        // TWIN-PARTY-RESID: evict twin waiters past TWIN_WAIT_TIMEOUT_SECS.\n\
        let timed_out = poll_twin_timeouts(&mut state.twin_wait, state.sim_time);\n\
        for cid in timed_out {\n\
            let p_id = state.players.get(&cid).map(|p| p.p_id).unwrap_or(0);\n\
            let line = if p_id != 0 {\n\
                format!(\"{p_id} {}\", format_twin_timeout_ps())\n\
            } else {\n\
                format_twin_timeout_ps()\n\
            };\n\
            send_ps_reply(&outbound, cid, &line);\n\
        }\n";
        if t.contains(needle) {
            t = t.replacen(needle, repl, 1);
            changed = true;
        }
    }

    if !t.contains("TWIN-PARTY-RESID: ObjectData.male flag") {
        let new_pif = "/// Live female check for fertility / mother fitness (Haxe `isFemale`).\n\
///\n\
/// When content has a person race (`person` ≠ 0) or an explicit `male=1` flag,\n\
/// use Haxe `ObjectData.male` (`!male` ⇒ female). Otherwise fall back to the\n\
/// name/description heuristic (`person_looks_female`).\n\
// Haxe: GlobalPlayerInstance.isFemale / ObjectData.male\n\
// TWIN-PARTY-RESID: ObjectData.male flag\n\
pub fn player_is_female(state: &SimState, p: &Player) -> bool {\n\
    let po = person_object_id(p);\n\
    let Some(def) = state.content.get(po) else {\n\
        return person_looks_female(po, \"\", \"\");\n\
    };\n\
    let race = state.content.person_color(po);\n\
    let content_male = if race != 0 || def.male {\n\
        Some(def.male)\n\
    } else {\n\
        None\n\
    };\n\
    person_is_female(po, def.name.as_str(), def.description.as_str(), content_male)\n\
}\n";
        if let Some(start) = t.find("pub fn player_is_female(state: &SimState, p: &Player) -> bool {") {
            if let Some(rel_end) = t[start..].find("\n}\n") {
                let end = start + rel_end + 3;
                let doc_start = t[..start]
                    .rfind("/// Live female check")
                    .unwrap_or(start);
                t = format!("{}{}{}", &t[..doc_start], new_pif, &t[end..]);
                changed = true;
            }
        }
    }

    if changed {
        let out = restore_nl(t, crlf);
        if std::fs::write(&lib_path, out).is_ok() {
            println!("cargo:warning=TWIN-PARTY-RESID: pure-Rust lib.rs wire applied");
            return true;
        }
    }
    twin_party_resid_wired(&std::fs::read_to_string(&lib_path).unwrap_or_default())
}

/// Piggyback AI-FOLLOW-WALK pure+live wire (Python + minimal Rust includes).
fn piggyback_ai_follow_walk(src_dir: &Path, workspace: &Path) {
    let fw_py = src_dir.join("_apply_ai_follow_walk.py");
    if fw_py.exists() {
        let _ = Command::new("python")
            .arg(&fw_py)
            .current_dir(src_dir)
            .status()
            .or_else(|_| {
                Command::new("python3")
                    .arg(&fw_py)
                    .current_dir(src_dir)
                    .status()
            });
    }
    // Minimal pure-Rust follow wire if Python missing/failed.
    let lib_path = src_dir.join("lib.rs");
    if let Ok(raw) = std::fs::read_to_string(&lib_path) {
        if !raw.contains("tick_ai_follow_walk") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            let mut ch = false;
            if !t.contains("decide_follow_walk") {
                if let Some(i) = t.find("DropCommandPlan, StartFollowPlan, MAKE_ITEM_ALIASES,\n};\n") {
                    let old = "DropCommandPlan, StartFollowPlan, MAKE_ITEM_ALIASES,\n};\n";
                    let new = "DropCommandPlan, StartFollowPlan, MAKE_ITEM_ALIASES,\n    // AI-FOLLOW-WALK continuous isMovingToPlayer / ally Goto\n    ally_goto_speaker_xy, apply_follow_sticky_clear, decide_follow_walk,\n    follow_goal_xy, follow_max_tiles_for_sticky, follow_player_sensor, follow_seed,\n    follow_stand_half_range, follow_walk_holds_tick, ordered_follow_sensor,\n    plan_follow_sticky_clear, truncate_follow_path_steps, AiFollowSticky,\n    FollowStickyClearPlan, FollowTargetSnap, FollowWalkDecision,\n    AUTO_STOP_FOLLOW_CLEAR_AGE, FOLLOW_PATH_STEP_CAP, ORDERED_FOLLOW_MAX_SECS,\n};\n";
                    t = format!("{}{}{}", &t[..i], new, &t[i + old.len()..]);
                    ch = true;
                }
            }
            if !t.contains("ai_follow_walk_live.inc.rs") {
                if t.contains("include!(\"twin_party_live.inc.rs\");") {
                    t = t.replacen(
                        "include!(\"twin_party_live.inc.rs\");\n",
                        "include!(\"twin_party_live.inc.rs\");\n// AI-FOLLOW-WALK continuous follow + ally Goto pathfind\ninclude!(\"ai_follow_walk_live.inc.rs\");\n",
                        1,
                    );
                    ch = true;
                }
            }
            if !t.contains("tick_ai_follow_walk(state, outbound)") {
                t = t.replacen(
                    "    tick_llm_speech_wire(state, outbound);\n    // BLOCKED-BY-AI:",
                    "    tick_llm_speech_wire(state, outbound);\n    // AI-FOLLOW-WALK: continuous isMovingToPlayer walk toward ai_follow_p_id\n    tick_ai_follow_walk(state, outbound);\n    // BLOCKED-BY-AI:",
                    1,
                );
                ch = true;
            }
            if !t.contains("AI-FOLLOW-WALK: startFollowing") {
                if let Some(i) = t.find(
                    "if complete.goto_speaker {\n                    ai.force_stop_on_next_tile = false;\n                }\n            }\n            if let Some(eid) = emote_id {",
                ) {
                    let old = "if complete.goto_speaker {\n                    ai.force_stop_on_next_tile = false;\n                }\n            }\n            if let Some(eid) = emote_id {";
                    let new = "if complete.goto_speaker {\n                    ai.force_stop_on_next_tile = false;\n                }\n            }\n            // AI-FOLLOW-WALK: startFollowing / ally Goto pathfind to speaker+1\n            if apply_plan.follow_player || complete.goto_speaker {\n                let speaker_xy = state\n                    .players\n                    .values()\n                    .find(|p| p.p_id == res.speaker_p_id && !p.deleted)\n                    .map(|p| (p.x, p.y));\n                if let Some((sx, sy)) = speaker_xy {\n                    let (gx, gy) = ally_goto_speaker_xy(sx, sy);\n                    let _ = try_ai_follow_path_to(state, outbound, res.ai_conn_id, gx, gy);\n                }\n            }\n            if let Some(eid) = emote_id {";
                    t = format!("{}{}{}", &t[..i], new, &t[i + old.len()..]);
                    ch = true;
                }
            }
            if ch {
                let out = if crlf {
                    t.replace('\n', "\r\n")
                } else {
                    t
                };
                let _ = std::fs::write(&lib_path, out);
                println!("cargo:warning=AI-FOLLOW-WALK: pure-Rust lib wire applied (via twin_party piggyback)");
            }
        }
    }
    // player snapshot
    let player_path = src_dir.join("player.rs");
    if let Ok(raw) = std::fs::read_to_string(&player_path) {
        if !raw.contains("ai_follow_p_id: self.ai_follow_p_id") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            t = t.replacen(
                "            clothing: self.clothing_parent_ids(),\n            clothing_uses: self.clothing_uses_remaining(),\n        }\n    }\n}",
                "            clothing: self.clothing_parent_ids(),\n            clothing_uses: self.clothing_uses_remaining(),\n            // AI-FOLLOW-WALK: sticky walk-with for NPC continuous follow\n            ai_follow_p_id: self.ai_follow_p_id,\n            ai_auto_stop_follow: self.ai_auto_stop_follow,\n        }\n    }\n}",
                1,
            );
            t = t.replacen(
                "    /// Haxe clothing `numberOfUses` per slot (quiver multi-use capacity).\n    #[serde(default)]\n    pub clothing_uses: [i32; 6],\n}",
                "    /// Haxe clothing `numberOfUses` per slot (quiver multi-use capacity).\n    #[serde(default)]\n    pub clothing_uses: [i32; 6],\n    /// Sticky AI walk-with target p_id (Haxe playerToFollow); 0 = none (**AI-FOLLOW-WALK**).\n    // Haxe: AiBase.playerToFollow\n    #[serde(default)]\n    pub ai_follow_p_id: i32,\n    /// Haxe autoStopFollow — loose follow when true.\n    // Haxe: AiBase.autoStopFollow\n    #[serde(default = \"default_true_snapshot\")]\n    pub ai_auto_stop_follow: bool,\n}",
                1,
            );
            let out = if crlf {
                t.replace('\n', "\r\n")
            } else {
                t
            };
            let _ = std::fs::write(&player_path, out);
        }
    }
    // npc
    let npc_path = workspace.join("crates/ol-server/src/npc_ai.rs");
    if let Ok(raw) = std::fs::read_to_string(&npc_path) {
        if !raw.contains("AI-FOLLOW-WALK") {
            let crlf = raw.contains("\r\n");
            let mut t = normalize_nl(&raw);
            let old = "            // --- 2b. Profession ladder scan (NPC-CRAFT-LADDER) ---\n            // Haxe: AssignedJob / AgeRotatedJob → doBasicFarming/doSmithing/doBaking → USE/DROP\n            // Escape/food bands already handled above; only when not hungry-starving.\n            if !acted && !starving && !hungry {";
            let new = "            // --- 2a. Continuous follow walk (AI-FOLLOW-WALK) ---\n            // Haxe: AiBase.isMovingToPlayer after sticky playerToFollow / LLM follow\n            if !acted && !starving {\n                let follow_p = p.ai_follow_p_id;\n                if follow_p > 0 {\n                    let target = views.values().find(|o| o.p_id == follow_p && !o.deleted);\n                    if let Some(t) = target {\n                        let max_tiles = if p.ai_auto_stop_follow { 10 } else { 5 };\n                        let max_q = max_tiles * max_tiles;\n                        let dx = t.x - p.x;\n                        let dy = t.y - p.y;\n                        let qd = dx * dx + dy * dy;\n                        if qd >= max_q {\n                            let goal_x = t.x + 1;\n                            let goal_y = t.y;\n                            let dist = (goal_x - p.x).abs().max((goal_y - p.y).abs());\n                            if dist > 1 && !p.moving {\n                                if let Some((sdx, sdy)) = {\n                                    let w = world.read().unwrap();\n                                    next_step(&w, p.x, p.y, goal_x, goal_y, &|nx, ny| {\n                                        is_walkable(&w, &content, nx, ny)\n                                    })\n                                } {\n                                    if intent_tx\n                                        .try_send(NetIntent::Move {\n                                            conn_id,\n                                            xs: p.x,\n                                            ys: p.y,\n                                            deltas: vec![(sdx, sdy)],\n                                            seq: None,\n                                        })\n                                        .is_ok()\n                                    {\n                                        kind = NpcActivityKind::Think;\n                                        detail = format!(\n                                            \"follow_walk target={} @{},{}\",\n                                            follow_p, t.x, t.y\n                                        );\n                                        game_ms = 250;\n                                        acted = true;\n                                    }\n                                }\n                            } else if p.moving {\n                                kind = NpcActivityKind::Think;\n                                detail = format!(\"follow_busy_moving target={follow_p}\");\n                                game_ms = 200;\n                                acted = true;\n                            }\n                        }\n                    }\n                }\n            }\n\n            // --- 2b. Profession ladder scan (NPC-CRAFT-LADDER) ---\n            // Haxe: AssignedJob / AgeRotatedJob → doBasicFarming/doSmithing/doBaking → USE/DROP\n            // Escape/food bands already handled above; only when not hungry-starving.\n            // AI-FOLLOW-WALK: follow holds tick above when far from sticky target\n            if !acted && !starving && !hungry {";
            if t.contains(old) {
                t = t.replacen(old, new, 1);
                let out = if crlf {
                    t.replace('\n', "\r\n")
                } else {
                    t
                };
                let _ = std::fs::write(&npc_path, out);
                println!("cargo:warning=AI-FOLLOW-WALK: npc_ai follow_walk wired");
            }
        }
    }
    // MOVE-MIDPATH: always attempt calculateNewPos mid-path recon (idempotent)
    piggyback_move_midpath(src_dir, workspace);
}

/// Run Python apply if present; pure-Rust fallback for critical symbols.
pub fn patch_twin_party_resid(src_dir: &Path, workspace: &Path) -> bool {
    fix_twins_typo(src_dir);
    pure_content_male_patch(workspace);

    let mut twin_ok = false;
    let py = src_dir.join("_apply_twin_party_resid.py");
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
                let lib = std::fs::read_to_string(src_dir.join("lib.rs")).unwrap_or_default();
                if twin_party_resid_wired(&lib) || lib.contains("TwinHeartLinks") {
                    twin_ok = true;
                }
            }
        }
    }

    if !twin_ok {
        twin_ok = pure_rust_lib_patch(src_dir);
    }

    // AI-FOLLOW-WALK: always attempt (idempotent) after twin wire
    // (also runs MOVE-MIDPATH piggyback at end of piggyback_ai_follow_walk)
    piggyback_ai_follow_walk(src_dir, workspace);

    twin_ok
}

include!("build_move_midpath_piggy.inc.rs");
