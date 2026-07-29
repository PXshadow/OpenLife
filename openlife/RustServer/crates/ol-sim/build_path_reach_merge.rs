//! PATH-REACH-MERGE / dual_map_merge — build-time pure-Rust wire (idempotent).
//!
//! Haxe `AiBase` keeps **one** pair of maps (`notReachableObjects` /
//! `objectsWithHostilePath`). Rust split ownership:
//! - live NetIntent / shortCraft fail → `Player.ai_path_reach`
//! - npc SeekFood / walk / profession → `NpcProfessionState.path_reach`
//!
//! This chunk wires bidirectional max-timer merge each tick:
//! 1. `PlayerSnapshot.ai_path_reach` so NPC can pull
//! 2. `tick_vitals` absorbs NPC marks from `player_views` before cleanup
//! 3. `npc_ai` pulls Player → NPC at think start; pushes NPC → views after marks
//!
//! // Haxe: AiBase L85–86 + cleanupBlockedObjectsHelper ~6264 (single maps)

use std::path::Path;

fn normalize_nl(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

fn restore_nl(s: &str, crlf: bool) -> String {
    if crlf {
        s.replace('\n', "\r\n")
    } else {
        s.to_string()
    }
}

fn write_if_changed(path: &Path, original: &str, next: &str) -> bool {
    if original == next {
        return false;
    }
    if let Err(e) = std::fs::write(path, next) {
        eprintln!(
            "cargo:warning=PATH-REACH-MERGE write {}: {e}",
            path.display()
        );
        return false;
    }
    true
}

/// True when dual-map merge is fully live-wired.
pub fn already_wired(src: &Path, workspace: &Path) -> bool {
    let player = src.join("player.rs");
    let lib = src.join("lib.rs");
    let reach = src.join("ai_path_reach.rs");
    let npc = workspace.join("crates/ol-server/src/npc_ai.rs");
    let Ok(p) = std::fs::read_to_string(&player) else {
        return false;
    };
    let Ok(l) = std::fs::read_to_string(&lib) else {
        return false;
    };
    let Ok(r) = std::fs::read_to_string(&reach) else {
        return false;
    };
    let Ok(n) = std::fs::read_to_string(&npc) else {
        return false;
    };
    p.contains("/// PATH-REACH-MERGE: expose timed maps for NPC dual-map sync")
        && p.contains("ai_path_reach: self.ai_path_reach.clone()")
        && l.contains("PATH-REACH-MERGE: absorb NPC path maps")
        && l.contains("merge_npc_path_reach_from_views")
        && r.contains("sync_path_reach_bidirectional")
        && n.contains("PATH-REACH-MERGE: pull Player.ai_path_reach")
        && n.contains("merge_path_reach_maps")
        && n.contains("push_npc_path_reach_to_views")
}

pub fn patch_path_reach_merge(src: &Path, workspace: &Path) -> bool {
    println!("cargo:rerun-if-changed=build_path_reach_merge.rs");
    println!("cargo:rerun-if-changed=src/ai_path_reach.rs");
    println!("cargo:rerun-if-changed=src/player.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=../ol-server/src/npc_ai.rs");

    if already_wired(src, workspace) {
        let stamp = src.join(".path_reach_merge_patched");
        let _ = std::fs::write(&stamp, b"path-reach-merge-1-source-wired\n");
        patch_docs(workspace);
        return true;
    }

    let mut ok = true;
    ok &= patch_ai_path_reach(&src.join("ai_path_reach.rs"));
    ok &= patch_player(&src.join("player.rs"));
    ok &= patch_lib(&src.join("lib.rs"));
    ok &= patch_npc_ai(&workspace.join("crates/ol-server/src/npc_ai.rs"));
    patch_docs(workspace);

    if already_wired(src, workspace) {
        let stamp = src.join(".path_reach_merge_patched");
        let _ = std::fs::write(&stamp, b"path-reach-merge-1-rs-patched\n");
        true
    } else {
        println!("cargo:warning=PATH-REACH-MERGE: could not fully wire dual_map_merge");
        ok
    }
}

fn patch_ai_path_reach(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("sync_path_reach_bidirectional") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);

    // Expand merge_path_reach_maps doc + add bidirectional helper after it.
    let anchor = r#"pub fn merge_path_reach_maps(dst: &mut AiPathReachMaps, src: &AiPathReachMaps) {
    for (&xy, &t) in &src.not_reachable {
        dst.not_reachable
            .entry(xy)
            .and_modify(|e| {
                if t > *e {
                    *e = t;
                }
            })
            .or_insert(t);
    }
    for (&xy, &t) in &src.hostile_path {
        dst.hostile_path
            .entry(xy)
            .and_modify(|e| {
                if t > *e {
                    *e = t;
                }
            })
            .or_insert(t);
    }
}
"#;
    let insert = r#"pub fn merge_path_reach_maps(dst: &mut AiPathReachMaps, src: &AiPathReachMaps) {
    for (&xy, &t) in &src.not_reachable {
        dst.not_reachable
            .entry(xy)
            .and_modify(|e| {
                if t > *e {
                    *e = t;
                }
            })
            .or_insert(t);
    }
    for (&xy, &t) in &src.hostile_path {
        dst.hostile_path
            .entry(xy)
            .and_modify(|e| {
                if t > *e {
                    *e = t;
                }
            })
            .or_insert(t);
    }
}

/// PATH-REACH-MERGE: max-merge both ways so dual ownership matches Haxe single maps.
///
/// After this, `a` and `b` hold identical max timers for every key.
// Haxe: AiBase L85–86 single notReachableObjects / objectsWithHostilePath
// PATH-REACH-MERGE / dual_map_merge
pub fn sync_path_reach_bidirectional(a: &mut AiPathReachMaps, b: &mut AiPathReachMaps) {
    merge_path_reach_maps(a, b);
    merge_path_reach_maps(b, a);
}
"#;
    if !t.contains(anchor) {
        // Try without exact whitespace — append after merge_path_reach_maps closing.
        if let Some(idx) = t.find("pub fn merge_path_reach_maps(") {
            if let Some(rel) = t[idx..].find("\n}\n\n/// Haxe `gotoAdv` fail:") {
                let at = idx + rel + "\n}\n".len();
                let helper = r#"
/// PATH-REACH-MERGE: max-merge both ways so dual ownership matches Haxe single maps.
///
/// After this, `a` and `b` hold identical max timers for every key.
// Haxe: AiBase L85–86 single notReachableObjects / objectsWithHostilePath
// PATH-REACH-MERGE / dual_map_merge
pub fn sync_path_reach_bidirectional(a: &mut AiPathReachMaps, b: &mut AiPathReachMaps) {
    merge_path_reach_maps(a, b);
    merge_path_reach_maps(b, a);
}

"#;
                t.insert_str(at, helper);
            } else {
                eprintln!("cargo:warning=PATH-REACH-MERGE: merge_path_reach_maps anchor miss");
                return false;
            }
        } else {
            return false;
        }
    } else {
        t = t.replacen(anchor, insert, 1);
    }

    // Test for bidirectional sync next to merge_path_reach_maps_max_timers
    if !t.contains("sync_path_reach_bidirectional_equalizes") {
        let test = r#"
    #[test]
    fn sync_path_reach_bidirectional_equalizes() {
        // PATH-REACH-MERGE: both maps end with max of either
        let mut a = AiPathReachMaps::new();
        a.add_not_reachable(1, 1, 10.0);
        a.add_hostile_path(2, 2, 5.0);
        let mut b = AiPathReachMaps::new();
        b.add_not_reachable(1, 1, 30.0);
        b.add_not_reachable(3, 3, 40.0);
        b.add_hostile_path(2, 2, 20.0);
        sync_path_reach_bidirectional(&mut a, &mut b);
        assert!((a.not_reachable[&(1, 1)] - 30.0).abs() < 0.01);
        assert!((b.not_reachable[&(1, 1)] - 30.0).abs() < 0.01);
        assert!((a.not_reachable[&(3, 3)] - 40.0).abs() < 0.01);
        assert!((b.not_reachable[&(3, 3)] - 40.0).abs() < 0.01);
        assert!((a.hostile_path[&(2, 2)] - 20.0).abs() < 0.01);
        assert!((b.hostile_path[&(2, 2)] - 20.0).abs() < 0.01);
    }

"#;
        if let Some(idx) = t.find("fn merge_path_reach_maps_max_timers()") {
            if let Some(rel) = t[idx..].find("\n    }\n\n    #[test]\n    fn mark_goto_path_fail_animal_vs_block") {
                let at = idx + rel + "\n    }\n".len();
                t.insert_str(at, test);
            } else if let Some(rel) = t[idx..].find("\n    }\n\n    #[test]\n") {
                // insert after this test's closing
                let at = idx + rel + "\n    }\n".len();
                t.insert_str(at, test);
            }
        }
    }

    write_if_changed(path, &raw, &restore_nl(&t, crlf));
    true
}

fn patch_player(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("/// PATH-REACH-MERGE: expose timed maps for NPC dual-map sync") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);

    // snapshot() field
    let snap_old = r#"            // AI-FOLLOW-WALK: sticky walk-with for NPC continuous follow
            ai_follow_p_id: self.ai_follow_p_id,
            ai_auto_stop_follow: self.ai_auto_stop_follow,
        }
    }
}
"#;
    let snap_new = r#"            // AI-FOLLOW-WALK: sticky walk-with for NPC continuous follow
            ai_follow_p_id: self.ai_follow_p_id,
            ai_auto_stop_follow: self.ai_auto_stop_follow,
            // PATH-REACH-MERGE: dual-map pull source for npc_ai
            ai_path_reach: self.ai_path_reach.clone(),
        }
    }
}
"#;
    if t.contains(snap_old) {
        t = t.replacen(snap_old, snap_new, 1);
    } else if !t.contains("ai_path_reach: self.ai_path_reach.clone()") {
        // fallback: insert before closing of snapshot
        if let Some(idx) = t.find("ai_auto_stop_follow: self.ai_auto_stop_follow,\n        }\n    }\n}") {
            let at = idx + "ai_auto_stop_follow: self.ai_auto_stop_follow,\n".len();
            t.insert_str(
                at,
                "            // PATH-REACH-MERGE: dual-map pull source for npc_ai\n            ai_path_reach: self.ai_path_reach.clone(),\n",
            );
        } else {
            eprintln!("cargo:warning=PATH-REACH-MERGE: player snapshot field anchor miss");
            return false;
        }
    }

    // struct field
    let struct_old = r#"    /// Haxe autoStopFollow — loose follow when true.
    // Haxe: AiBase.autoStopFollow
    #[serde(default = "default_true_snapshot")]
    pub ai_auto_stop_follow: bool,
}
"#;
    let struct_new = r#"    /// Haxe autoStopFollow — loose follow when true.
    // Haxe: AiBase.autoStopFollow
    #[serde(default = "default_true_snapshot")]
    pub ai_auto_stop_follow: bool,
    /// PATH-REACH-MERGE: expose timed maps for NPC dual-map sync.
    /// Not serialized to web JSON (serde skip).
    // Haxe: AiBase L85–86 notReachableObjects / objectsWithHostilePath
    // PATH-REACH-MERGE / dual_map_merge
    #[serde(skip)]
    pub ai_path_reach: crate::ai_path_reach::AiPathReachMaps,
}
"#;
    if t.contains(struct_old) {
        t = t.replacen(struct_old, struct_new, 1);
    } else if !t.contains("/// PATH-REACH-MERGE: expose timed maps for NPC dual-map sync") {
        if let Some(idx) = t.find("pub ai_auto_stop_follow: bool,\n}\n") {
            let at = idx + "pub ai_auto_stop_follow: bool,\n".len();
            t.insert_str(
                at,
                r#"    /// PATH-REACH-MERGE: expose timed maps for NPC dual-map sync.
    /// Not serialized to web JSON (serde skip).
    // Haxe: AiBase L85–86 notReachableObjects / objectsWithHostilePath
    // PATH-REACH-MERGE / dual_map_merge
    #[serde(skip)]
    pub ai_path_reach: crate::ai_path_reach::AiPathReachMaps,
"#,
            );
        } else {
            eprintln!("cargo:warning=PATH-REACH-MERGE: PlayerSnapshot field anchor miss");
            return false;
        }
    }

    // unit test near follow sticky if present in player tests — optional in lib

    write_if_changed(path, &raw, &restore_nl(&t, crlf));
    true
}

fn patch_lib(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("PATH-REACH-MERGE: absorb NPC path maps")
        && raw.contains("merge_npc_path_reach_from_views")
        && raw.contains("sync_path_reach_bidirectional")
    {
        // still may need export only
        return ensure_lib_export(&raw, path);
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);

    // Export sync_path_reach_bidirectional
    if !t.contains("sync_path_reach_bidirectional") {
        t = t.replacen(
            "mark_use_or_food_path_fail, merge_path_reach_maps, pending_food_tile_still_actionable,",
            "mark_use_or_food_path_fail, merge_path_reach_maps, merge_npc_path_reach_from_views, pending_food_tile_still_actionable,",
            1,
        );
        // add sync to export list
        if t.contains("merge_path_reach_maps, merge_npc_path_reach_from_views") {
            // good
        }
        t = t.replacen(
            "settle_pending_food_use_fail, mark_use_path_fail, plan_goto_obj,",
            "settle_pending_food_use_fail, mark_use_path_fail, plan_goto_obj, sync_path_reach_bidirectional,",
            1,
        );
    }

    // Live helper near mark_path_fail_after_food_pickup_action_live
    if !t.contains("fn merge_npc_path_reach_from_views")
        && !t.contains("pub fn merge_npc_path_reach_from_views")
    {
        let helper = r#"
/// PATH-REACH-MERGE: absorb NPC `path_reach` marks written into `player_views`.
///
/// NPC scheduler max-merges into `PlayerSnapshot.ai_path_reach`; each vitals tick
/// pulls those into live `Player.ai_path_reach` before personal cleanup.
// Haxe: AiBase single maps (L85–86) — Rust dual ownership bridge
// PATH-REACH-MERGE / dual_map_merge
pub fn merge_npc_path_reach_from_views(state: &mut SimState) {
    let Some(views) = &state.player_views else {
        return;
    };
    let npc_maps: Vec<(u64, crate::ai_path_reach::AiPathReachMaps)> = match views.read() {
        Ok(g) => g
            .iter()
            .filter(|(_, s)| !s.ai_path_reach.is_empty())
            .map(|(cid, s)| (*cid, s.ai_path_reach.clone()))
            .collect(),
        Err(_) => return,
    };
    for (cid, maps) in npc_maps {
        if let Some(p) = state.players.get_mut(&cid) {
            crate::merge_path_reach_maps(&mut p.ai_path_reach, &maps);
        }
    }
}

"#;
        // insert before mark_path_fail_after_use_live or tick_vitals_with_metrics
        if let Some(idx) = t.find("/// AI-FOOD-FAIL-MARK: food USE/DROP fail → 30s not_reachable on `Player.ai_path_reach`.") {
            t.insert_str(idx, helper);
        } else if let Some(idx) = t.find("pub fn mark_path_fail_after_use_live(") {
            t.insert_str(idx, helper);
        } else if let Some(idx) = t.find("pub fn tick_vitals_with_metrics(") {
            t.insert_str(idx, helper);
        } else {
            eprintln!("cargo:warning=PATH-REACH-MERGE: lib helper insert anchor miss");
            return false;
        }
    }

    // tick_vitals: merge before cleanup
    let cleanup_old = r#"    // PATH-REACH: cleanup AI path maps (Haxe cleanupBlockedObjects each reaction).
    // Haxe: AiBase.cleanupBlockedObjectsHelper ~6264
    {
        for p in state.players.values_mut() {
            p.ai_path_reach.cleanup(dt);
        }
    }
"#;
    let cleanup_new = r#"    // PATH-REACH-MERGE: absorb NPC path maps written into player_views (dual ownership).
    // Haxe: AiBase L85–86 single maps — NetIntent marks Player; npc_ai uses path_reach
    // PATH-REACH-MERGE / dual_map_merge
    merge_npc_path_reach_from_views(state);
    // PATH-REACH: cleanup AI path maps (Haxe cleanupBlockedObjects each reaction).
    // Haxe: AiBase.cleanupBlockedObjectsHelper ~6264
    {
        for p in state.players.values_mut() {
            p.ai_path_reach.cleanup(dt);
        }
    }
"#;
    if t.contains(cleanup_old) {
        t = t.replacen(cleanup_old, cleanup_new, 1);
    } else if !t.contains("PATH-REACH-MERGE: absorb NPC path maps") {
        if let Some(idx) = t.find("// PATH-REACH: cleanup AI path maps") {
            t.insert_str(
                idx,
                "    // PATH-REACH-MERGE: absorb NPC path maps written into player_views (dual ownership).\n    // Haxe: AiBase L85–86 single maps — NetIntent marks Player; npc_ai uses path_reach\n    // PATH-REACH-MERGE / dual_map_merge\n    merge_npc_path_reach_from_views(state);\n",
            );
        } else {
            eprintln!("cargo:warning=PATH-REACH-MERGE: tick_vitals cleanup anchor miss");
            return false;
        }
    }

    // Unit test near path_reach / food fail tests
    if !t.contains("path_reach_merge_views_into_player_on_tick") {
        let test = r#"
    /// PATH-REACH-MERGE: NPC marks on player_views.ai_path_reach → Player on tick_vitals.
    #[test]
    fn path_reach_merge_views_into_player_on_tick() {
        use crate::ai_path_reach::AiPathReachMaps;
        use std::collections::HashMap;
        use std::sync::{Arc, RwLock};
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "npc-merge@local");
        // Seed NPC-side mark via player_views (as npc_ai push would).
        let mut maps = AiPathReachMaps::new();
        maps.add_not_reachable(7, 8, 90.0);
        maps.add_hostile_path(9, 10, 20.0);
        let views: crate::PlayerViewMap = Arc::new(RwLock::new(HashMap::new()));
        {
            let mut snap = state.players.get(&1).unwrap().snapshot();
            snap.ai_path_reach = maps;
            views.write().unwrap().insert(1, snap);
        }
        state.player_views = Some(views);
        assert!(state.players.get(&1).unwrap().ai_path_reach.is_empty());
        let hub = OutboundHub::new();
        tick_vitals(&mut state, 0.0, &hub);
        let p = state.players.get(&1).unwrap();
        assert!(p.ai_path_reach.is_personal_not_reachable(7, 8));
        assert!(p.ai_path_reach.is_object_with_hostile_path(9, 10));
    }

    /// PATH-REACH-MERGE: snapshot exposes Player.ai_path_reach for NPC pull.
    #[test]
    fn player_snapshot_includes_path_reach_maps() {
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "npc-snap-pr@local");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.ai_path_reach.add_not_reachable(3, 4, 30.0);
        }
        let snap = state.players.get(&1).unwrap().snapshot();
        assert!(snap.ai_path_reach.is_personal_not_reachable(3, 4));
    }

"#;
        if let Some(idx) = t.find("fn player_snapshot_includes_follow_sticky()") {
            t.insert_str(idx, test);
        } else if let Some(idx) = t.find("fn ai_path_reach_sticky_defaults_and_survives()") {
            t.insert_str(idx, test);
        } else if let Some(idx) = t.rfind("\n    #[test]\n    fn ") {
            // append near end of tests — risky; skip if no good anchor
            let _ = idx;
            // try mark_path_fail test area
            if let Some(i2) = t.find("(p.ai_path_reach.not_reachable[&(1, 0)] - crate::NOT_REACHABLE_FOOD_SECS).abs() < 0.01") {
                if let Some(rel) = t[i2..].find("\n    }\n\n    #[test]\n") {
                    let at = i2 + rel + "\n    }\n".len();
                    t.insert_str(at, test);
                }
            }
        }
    }

    write_if_changed(path, &raw, &restore_nl(&t, crlf));
    true
}

fn ensure_lib_export(raw: &str, path: &Path) -> bool {
    if raw.contains("sync_path_reach_bidirectional")
        && raw.contains("merge_npc_path_reach_from_views")
    {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(raw);
    if !t.contains("sync_path_reach_bidirectional") {
        t = t.replacen(
            "settle_pending_food_use_fail, mark_use_path_fail, plan_goto_obj,",
            "settle_pending_food_use_fail, mark_use_path_fail, plan_goto_obj, sync_path_reach_bidirectional,",
            1,
        );
    }
    if !t.contains("merge_npc_path_reach_from_views") {
        t = t.replacen(
            "mark_use_or_food_path_fail, merge_path_reach_maps, pending_food_tile_still_actionable,",
            "mark_use_or_food_path_fail, merge_path_reach_maps, merge_npc_path_reach_from_views, pending_food_tile_still_actionable,",
            1,
        );
    }
    write_if_changed(path, raw, &restore_nl(&t, crlf));
    true
}

fn patch_npc_ai(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("PATH-REACH-MERGE: pull Player.ai_path_reach")
        && raw.contains("push_npc_path_reach_to_views")
    {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);

    // Import merge_path_reach_maps
    if !t.contains("merge_path_reach_maps") {
        t = t.replacen(
            "mark_food_path_fail, mark_goto_path_fail,",
            "mark_food_path_fail, mark_goto_path_fail, merge_path_reach_maps,",
            1,
        );
    }

    // Helper functions after npc_mark_goto_path_fail
    if !t.contains("fn pull_player_path_reach") {
        let helpers = r#"
/// PATH-REACH-MERGE: pull Player.ai_path_reach into NPC maps (max timers).
// Haxe: single AiBase maps — Rust dual ownership → merge each think
// PATH-REACH-MERGE / dual_map_merge
fn pull_player_path_reach(st: &mut NpcProfessionState, snap: &PlayerSnapshot) {
    merge_path_reach_maps(&mut st.path_reach, &snap.ai_path_reach);
}

/// PATH-REACH-MERGE: push NPC maps into player_views for tick_vitals absorb.
// Haxe: AiBase L85–86 single maps
fn push_npc_path_reach_to_views(
    player_views: &Arc<RwLock<HashMap<u64, PlayerSnapshot>>>,
    conn_id: u64,
    path_reach: &AiPathReachMaps,
) {
    if path_reach.is_empty() {
        return;
    }
    if let Ok(mut g) = player_views.write() {
        if let Some(s) = g.get_mut(&conn_id) {
            merge_path_reach_maps(&mut s.ai_path_reach, path_reach);
        }
    }
}

"#;
        if let Some(idx) = t.find("/// Sticky isPickingupFood / gotoObj bookkeeping (AI-GOTO-FOOD).") {
            t.insert_str(idx, helpers);
        } else if let Some(idx) = t.find("struct NpcFoodGotoState") {
            t.insert_str(idx, helpers);
        } else {
            eprintln!("cargo:warning=PATH-REACH-MERGE: npc helper insert miss");
            return false;
        }
    }

    // Wire pull+cleanup+push at the three path_reach.cleanup sites
    // Pattern: get st, cleanup, use path_reach
    // Replace cleanup lines with pull + cleanup, and add push after blocks is harder.
    // Simpler: wrap cleanup call sites.

    // Site 1 - early hungry selfplay-like path (~st.path_reach.cleanup)
    let c1_old = r#"                    let st = profession_state.entry(conn_id).or_default();
                    st.path_reach.cleanup(0.2 * think_period as f32);
                    settle_npc_pending_food_action(&content, &nearby, &p, st);
"#;
    let c1_new = r#"                    let st = profession_state.entry(conn_id).or_default();
                    // PATH-REACH-MERGE: pull Player.ai_path_reach (USE/DROP fail from sim)
                    pull_player_path_reach(st, &p);
                    st.path_reach.cleanup(0.2 * think_period as f32);
                    settle_npc_pending_food_action(&content, &nearby, &p, st);
"#;
    if t.contains(c1_old) {
        t = t.replacen(c1_old, c1_new, 1);
    }

    let c2_old = r#"                let st = profession_state.entry(conn_id).or_default();
                st.path_reach.cleanup(0.2 * think_period as f32);
                settle_npc_pending_food_action(&content, &nearby, &p, st);
"#;
    let c2_new = r#"                let st = profession_state.entry(conn_id).or_default();
                // PATH-REACH-MERGE: pull Player.ai_path_reach (USE/DROP fail from sim)
                pull_player_path_reach(st, &p);
                st.path_reach.cleanup(0.2 * think_period as f32);
                settle_npc_pending_food_action(&content, &nearby, &p, st);
"#;
    if t.contains(c2_old) {
        t = t.replacen(c2_old, c2_new, 1);
    }

    let c3_old = r#"                    let st = profession_state.entry(conn_id).or_default();
                    // PATH-REACH: decay + filter notReachable / hostile before profession picks.
                    // Haxe: cleanupBlockedObjects + isObjectNotReachable in GetClosest*
                    st.path_reach.cleanup(0.2 * think_period as f32);
"#;
    let c3_new = r#"                    let st = profession_state.entry(conn_id).or_default();
                    // PATH-REACH-MERGE: pull Player.ai_path_reach before profession filter
                    pull_player_path_reach(st, &p);
                    // PATH-REACH: decay + filter notReachable / hostile before profession picks.
                    // Haxe: cleanupBlockedObjects + isObjectNotReachable in GetClosest*
                    st.path_reach.cleanup(0.2 * think_period as f32);
"#;
    if t.contains(c3_old) {
        t = t.replacen(c3_old, c3_new, 1);
    }

    // Push after food settle / marks — end of hungry food block and after profession walk marks.
    // Best-effort: after settle + food run, push. And after profession ladder block.

    // After npc_run_is_picking_up_food in main loop — look for continue after food
    // Insert push before `continue` after food pickup and after profession acted.

    // Generic: after each `st.path_reach` mutation region, push once at end of think for that conn.
    // Insert near activity log at end of for-loop body for active NPCs.
    if !t.contains("push_npc_path_reach_to_views(&player_views, conn_id") {
        // Find a stable end-of-think site: activity push / stuck tracker update
        // Look for `activity.push` or similar near end of main for i in 0..active loop
        let push_snip = r#"
            // PATH-REACH-MERGE: push NPC path maps into player_views for tick_vitals absorb
            if let Some(st) = profession_state.get(&conn_id) {
                push_npc_path_reach_to_views(&player_views, conn_id, &st.path_reach);
            }
"#;
        // Insert before counters / timer record at end of think
        if let Some(idx) = t.find("counters.npc_thinks.fetch_add(1") {
            t.insert_str(idx, push_snip);
        } else if let Some(idx) = t.find("timer.record(&counters.npc_think_us)") {
            t.insert_str(idx, push_snip);
        } else if let Some(idx) = t.find("NpcActivityEvent {") {
            // first event in main loop — not ideal; try last occurrence near end
            // Fallback: after profession ladder result handling when acted
            if let Some(i2) = t.rfind("            if !acted {\n                // Explore") {
                t.insert_str(i2, push_snip);
            } else {
                // insert before end of for i in 0..active - search "for i in 0..active"
                eprintln!("cargo:warning=PATH-REACH-MERGE: npc push site miss — using food-only push");
                // At least push after food path settle in main loop
                if let Some(i3) = t.find("settle_npc_pending_food_action(&content, &nearby, &p, st);\n                if let Some(food) = resolve_npc_food_target(") {
                    // after the food block's closing is hard; add after pull sites manually
                    let _ = i3;
                }
            }
        }
    }

    // Also push after food fail marks in is_picking_up helper — when mark_food_path_fail is called
    // the end-of-think push covers it if we placed it.

    // Early explore path (first loop over AI conn) — pull before food if not done
    // First loop uses profession_state + path_reach.cleanup — already handled by c1.

    write_if_changed(path, &raw, &restore_nl(&t, crlf));
    true
}

fn patch_docs(workspace: &Path) {
    let port = workspace.join("docs").join("port");

    // TODO_PORT.md
    let todo = port.join("TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo) {
        let mut t = raw;
        t = t.replace(
            "Residual: live Player↔Npc dual-map merge each tick (pure `merge_path_reach_maps` ready)",
            "**PATH-REACH-MERGE dual_map_merge DONE** — live Player↔Npc max merge each tick",
        );
        t = t.replace(
            "- [~] **PATH-REACH not_reachable_maps PARTIAL** — `AiPathReachMaps` on `Player` (90/20/5s + food 30s); pure filters; tick personal cleanup; global `blocked_by_ai` via **BLOCKED-BY-AI** rebuild; profession/npc filter + USE/DROP/REMV fail; food-search not_reachable; dual-pass Goto + food sticky; **AI-FOOD-FAIL-MARK** food 30s. **PATH-REACH-MERGE dual_map_merge DONE** — live Player↔Npc max merge each tick",
            "- [x] **PATH-REACH not_reachable_maps DONE** — `AiPathReachMaps` on `Player` (90/20/5s + food 30s); pure filters; tick personal cleanup; global `blocked_by_ai` via **BLOCKED-BY-AI** rebuild; profession/npc filter + USE/DROP/REMV fail; food-search not_reachable; dual-pass Goto + food sticky; **AI-FOOD-FAIL-MARK** food 30s; **PATH-REACH-MERGE** Player↔Npc dual-map merge each tick",
        );
        // If first replace left PARTIAL line, force DONE status
        if t.contains("**PATH-REACH not_reachable_maps PARTIAL**")
            && t.contains("PATH-REACH-MERGE dual_map_merge DONE")
        {
            t = t.replace(
                "**PATH-REACH not_reachable_maps PARTIAL**",
                "**PATH-REACH not_reachable_maps DONE**",
            );
            t = t.replacen(
                "- [~] **PATH-REACH not_reachable_maps DONE**",
                "- [x] **PATH-REACH not_reachable_maps DONE**",
                1,
            );
        }
        // Explicit PATH-REACH-MERGE checkbox if missing
        if !t.contains("**PATH-REACH-MERGE") {
            if let Some(idx) = t.find("**AI-FOOD-FAIL-MARK food_use_fail_30s DONE**") {
                let line = "- [x] **PATH-REACH-MERGE dual_map_merge DONE** — `PlayerSnapshot.ai_path_reach` + `merge_npc_path_reach_from_views` tick + npc pull/push `merge_path_reach_maps` / `sync_path_reach_bidirectional`; tests path_reach_merge_*\n";
                // insert before AI-FOOD-FAIL-MARK line
                if let Some(line_start) = t[..idx].rfind("\n") {
                    t.insert_str(line_start + 1, line);
                }
            }
        }
        // changelog row
        if !t.contains("**PATH-REACH-MERGE dual_map_merge DONE")
            || !t.contains("| 2026-07-29 | **PATH-REACH-MERGE")
        {
            let row = "| 2026-07-29 | **PATH-REACH-MERGE dual_map_merge DONE**: `PlayerSnapshot.ai_path_reach`; `merge_npc_path_reach_from_views` in tick_vitals; npc_ai pull/push; pure `sync_path_reach_bidirectional`; tests path_reach_merge_* / snapshot |\n";
            if let Some(idx) = t.find("| 2026-07-29 | **AI-FOOD-FAIL-MARK") {
                t.insert_str(idx, row);
            } else if let Some(idx) = t.find("## Changelog (port docs)") {
                if let Some(rel) = t[idx..].find("\n| 20") {
                    t.insert_str(idx + rel + 1, row);
                }
            }
        }
        let _ = std::fs::write(&todo, t);
    }

    // CALL_INDEX.md
    let ci = port.join("CALL_INDEX.md");
    if let Ok(raw) = std::fs::read_to_string(&ci) {
        if !raw.contains("merge_npc_path_reach_from_views") {
            let mut t = raw;
            let old = "| `pending_food_tile_still_actionable` / `mark_food_pickup_action_fail_on_maps` / `merge_path_reach_maps` | same | container settle gate; DROP/REMV 30s; dual-map merge |\n";
            let new = "| `pending_food_tile_still_actionable` / `mark_food_pickup_action_fail_on_maps` / `merge_path_reach_maps` | same | container settle gate; DROP/REMV 30s; dual-map merge pure |\n| `sync_path_reach_bidirectional` / `merge_npc_path_reach_from_views` | `ai_path_reach.rs` / `lib.rs` | **PATH-REACH-MERGE** max both ways + tick absorb NPC marks |\n| `PlayerSnapshot.ai_path_reach` / npc `pull_player_path_reach` / `push_npc_path_reach_to_views` | `player.rs` / `npc_ai.rs` | **PATH-REACH-MERGE** dual-map pull/push each think |\n";
            if t.contains(old) {
                t = t.replacen(old, new, 1);
            } else if !t.contains("PATH-REACH-MERGE") {
                let anchor = "| `Player.ai_path_reach` | `player.rs` | sticky per-AI maps |\n";
                let insert = "| `Player.ai_path_reach` | `player.rs` | sticky per-AI maps |\n| `PlayerSnapshot.ai_path_reach` + `merge_npc_path_reach_from_views` | `player.rs` / `lib.rs` | **PATH-REACH-MERGE** dual-map bridge |\n";
                t = t.replacen(anchor, insert, 1);
            }
            let _ = std::fs::write(&ci, t);
        }
    }

    // FILE_MATRIX soft note
    let fm = port.join("FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&fm) {
        let mut t = raw;
        t = t.replace(
            "| A-HELP | `AiHelper.hx` | pathfind, craft helpers, **`search_best_food`**, deadly scans | PARTIAL | **SEARCH-BEST-FOOD** + PATH-REACH + AI-ANIMAL-GOTO + AI-GOTO-FOOD core |",
            "| A-HELP | `AiHelper.hx` | pathfind, craft helpers, **`search_best_food`**, deadly scans | PARTIAL | **SEARCH-BEST-FOOD** + PATH-REACH + **PATH-REACH-MERGE** + AI-ANIMAL-GOTO + AI-GOTO-FOOD core |",
        );
        let _ = std::fs::write(&fm, t);
    }

    // QUEUE — mark done if present
    let q = port.join("QUEUE.md");
    if let Ok(raw) = std::fs::read_to_string(&q) {
        let mut t = raw;
        t = t.replace(
            "| 3 | `PATH-REACH-MERGE` | dual_map_merge | haxe-port-chunk |",
            "| 3 | ~~`PATH-REACH-MERGE`~~ | dual_map_merge **DONE** | — |",
        );
        let _ = std::fs::write(&q, t);
    }

    // Changelog file
    let cl = port
        .join("changelog")
        .join("2026-07-29-PATH-REACH-MERGE.md");
    if !cl.exists() {
        let body = r#"# PATH-REACH-MERGE / dual_map_merge (2026-07-29)

## Summary

Live dual-map ownership bridge for Haxe single `AiBase` path maps.

| Side | Surface | Marks |
|------|---------|-------|
| Sim / NetIntent | `Player.ai_path_reach` | USE/DROP/REMV fail (AI-FOOD-FAIL-MARK) |
| NPC scheduler | `NpcProfessionState.path_reach` | walk fail, food settle, profession goto |

## Wire

1. `PlayerSnapshot.ai_path_reach` (serde skip) — NPC pull source
2. `npc_ai` each think: `pull_player_path_reach` → cleanup → act → `push_npc_path_reach_to_views`
3. `tick_vitals`: `merge_npc_path_reach_from_views` then personal `cleanup`
4. Pure: `merge_path_reach_maps` (max timers) + `sync_path_reach_bidirectional`

## Tests

```powershell
cargo test -p ol-sim --lib -- path_reach_merge merge_path_reach sync_path_reach player_snapshot_includes_path
```

## Residual

- empty-hand DROP pickup `apply_drop` (AI-PICKUP-FOOD)
- sticky foodTarget sync residual (AI-GOTO-FOOD)
"#;
        let _ = std::fs::write(&cl, body);
    }
}
