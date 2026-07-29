//! AI-FOOD-FAIL-MARK / food_use_fail_30s — build-time pure-Rust wire (idempotent).
//!
//! Haxe `isPickingupFood` USE/DROP/REMV fail → `addNotReachableObject(food, 30)`.
//! - pure `settle_pending_food_use_fail`
//! - live `mark_path_fail_after_use_live` AI-only + NetIntent after eat-fail
//! - npc `pending_food_xy` settle next tick
//!
//! // Haxe: AiBase.isPickingupFood ~8694–8700

use std::path::Path;
use std::process::Command;

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
            "cargo:warning=AI-FOOD-FAIL-MARK write {}: {e}",
            path.display()
        );
        return false;
    }
    true
}

/// True when pure settle + live AI gate + NetIntent mark-after-eat are present.
pub fn already_wired(src: &Path) -> bool {
    let reach = src.join("ai_path_reach.rs");
    let lib = src.join("lib.rs");
    let Ok(r) = std::fs::read_to_string(&reach) else {
        return false;
    };
    let Ok(l) = std::fs::read_to_string(&lib) else {
        return false;
    };
    r.contains("settle_pending_food_use_fail")
        && l.contains("settle_pending_food_use_fail")
        && l.contains("Some(p) if p.is_ai_body()")
        && l.contains("after eat fallback fails, mark path")
}

fn run_python(workspace: &Path) {
    for name in [
        "docs/port/_apply_ai_food_fail_mark.py",
        "docs/port/_doc_ai_food_fail_only.py",
    ] {
        let py = workspace.join(name);
        if !py.exists() {
            continue;
        }
        let _ = Command::new("python")
            .arg(&py)
            .status()
            .or_else(|_| Command::new("python3").arg(&py).status());
    }
}

fn patch_docs(workspace: &Path) {
    let port = workspace.join("docs").join("port");
    // FILE_MATRIX residual soft update
    let fm = port.join("FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&fm) {
        let mut t = raw;
        t = t.replace(
            "Residual: DROP/REMV apply-fail detect; NPC st.path_reach sync from Player mark",
            "npc `pending_food_xy` settle; AI-only mark-after-eat; `settle_pending_food_use_fail`",
        );
        t = t.replace(
            "Residual: remove-from-container fail mark; NPC `path_reach` sync from Player apply-fail",
            "Residual: remove-from-container fail mark polish",
        );
        t = t.replace(
            "**DONE → pure+live wire** | pure `food_action_fail_effects`",
            "**DONE** (pure+live) | pure `food_action_fail_effects`",
        );
        let _ = std::fs::write(&fm, t);
    }
    let todo = port.join("TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo) {
        let mut t = raw;
        t = t.replace(
            "Residual: DROP/REMV apply-fail detect; NPC `st.path_reach` sync",
            "npc pending settle + AI-only mark-after-eat",
        );
        t = t.replace(
            "Residual: remove-from-container fail mark; NPC path_reach sync from Player apply-fail",
            "Residual: remove-from-container fail mark polish",
        );
        let _ = std::fs::write(&todo, t);
    }
    let ci = port.join("CALL_INDEX.md");
    if let Ok(raw) = std::fs::read_to_string(&ci) {
        if !raw.contains("settle_pending_food_use_fail") {
            let mut t = raw;
            let old = "| `mark_path_fail_after_use_live` | `lib.rs` | live Player.ai_path_reach after USE fail (NetIntent + shortCraft) |\n";
            let new = "| `mark_path_fail_after_use_live` | `lib.rs` | live Player.ai_path_reach after USE fail (AI-only; NetIntent after eat-fail + shortCraft) |\n| `settle_pending_food_use_fail` / npc `pending_food_xy` | `ai_path_reach.rs` / `npc_ai.rs` | async food USE/DROP/REMV next-tick 30s settle |\n";
            if t.contains(old) {
                t = t.replacen(old, new, 1);
            }
            t = t.replace(
                "| NetIntent::Use !applied | `lib.rs` | note sticky + `mark_path_fail_after_use_live` |\n",
                "| NetIntent::Use !applied | `lib.rs` | after eat-fail: note sticky + `mark_path_fail_after_use_live` |\n",
            );
            let _ = std::fs::write(&ci, t);
        }
    }
}

fn patch_ai_path_reach(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("settle_pending_food_use_fail") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);

    let insert = r#"
/// Settle async food USE/DROP/REMV after send (Rust net is async; Haxe is sync).
///
/// If hands still empty and tile still food → `mark_food_path_fail` 30s + clear sticky.
/// If held anything → treat as success (no mark).
// Haxe: AiBase.isPickingupFood done==false ~8694–8700 (sync use/remove/drop)
// AI-FOOD-FAIL-MARK / food_use_fail_30s
pub fn settle_pending_food_use_fail(
    maps: &mut AiPathReachMaps,
    sticky_food: &mut Option<StickyFoodTarget>,
    pending_xy: Option<(i32, i32)>,
    held_id: i32,
    tile_still_food: bool,
) -> bool {
    let Some((x, y)) = pending_xy else {
        return false;
    };
    if held_id != 0 {
        return false;
    }
    if !tile_still_food {
        return false;
    }
    apply_food_action_fail(maps, sticky_food, None, x, y);
    true
}

"#;
    let needle = "/// Haxe `gotoAdv` fail: animal-only block → hostile; else not-reachable.\n";
    if !t.contains(needle) {
        eprintln!("cargo:warning=AI-FOOD-FAIL-MARK: gotoAdv fail comment missing");
        return false;
    }
    t = t.replacen(needle, &format!("{insert}{needle}"), 1);

    let test = r#"
    #[test]
    fn settle_pending_food_use_fail_marks_30s() {
        let mut maps = AiPathReachMaps::new();
        let mut sticky = Some(StickyFoodTarget::new(5, 6, 31));
        assert!(settle_pending_food_use_fail(
            &mut maps,
            &mut sticky,
            Some((5, 6)),
            0,
            true,
        ));
        assert!((maps.not_reachable[&(5, 6)] - NOT_REACHABLE_FOOD_SECS).abs() < 0.01);
        assert!(sticky.is_none());

        let mut maps2 = AiPathReachMaps::new();
        let mut sticky2 = Some(StickyFoodTarget::new(1, 1, 31));
        assert!(!settle_pending_food_use_fail(
            &mut maps2,
            &mut sticky2,
            Some((1, 1)),
            34,
            true,
        ));
        assert!(maps2.not_reachable.is_empty());
        assert!(sticky2.is_some());

        let mut maps3 = AiPathReachMaps::new();
        let mut sticky3 = None;
        assert!(!settle_pending_food_use_fail(
            &mut maps3,
            &mut sticky3,
            Some((2, 2)),
            0,
            false,
        ));
        assert!(maps3.not_reachable.is_empty());
    }

"#;
    let tneedle = "    #[test]\n    fn mark_goto_path_fail_animal_vs_block() {\n";
    if t.contains(tneedle) {
        t = t.replacen(tneedle, &format!("{test}{tneedle}"), 1);
    }

    write_if_changed(path, &raw, &restore_nl(&t, crlf))
}

fn patch_lib(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;

    if !t.contains("settle_pending_food_use_fail") {
        let old = "mark_use_or_food_path_fail,\n    mark_use_path_fail,";
        let new = "mark_use_or_food_path_fail,\n    settle_pending_food_use_fail, mark_use_path_fail,";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            ch = true;
        }
    }

    if !t.contains("Some(p) if p.is_ai_body()") {
        let old = r#"pub fn mark_path_fail_after_use_live(
    state: &mut SimState,
    conn_id: u64,
    x: i32,
    y: i32,
) -> bool {
    let (age, held_id) = match state.players.get(&conn_id) {
        Some(p) => (p.age, p.held_id),
        None => return false,
    };"#;
        let new = r#"pub fn mark_path_fail_after_use_live(
    state: &mut SimState,
    conn_id: u64,
    x: i32,
    y: i32,
) -> bool {
    // Haxe AiBase maps are AI-only (isPickingupFood / isUsingObject).
    let (age, held_id) = match state.players.get(&conn_id) {
        Some(p) if p.is_ai_body() => (p.age, p.held_id),
        Some(_) => return false,
        None => return false,
    };"#;
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            ch = true;
        }
    }

    if !t.contains("after eat fallback fails, mark path") {
        let old_mark = r#"                // AI-FOOD-FAIL-MARK / PATH-REACH: food USE fail → 30s not_reachable
                // (sticky food claim or empty-hand edible); else age-gated USE fail.
                // Haxe: AiBase.isPickingupFood ~8698 / isUsingObject ~9133
                // Note food sticky from tile food_value before mark (NetIntent has no shortCraft note).
                {
                    use crate::ShortCraftLiveIntent;
                    note_ai_block_targets_from_live_intent(
                        state,
                        conn_id,
                        ShortCraftLiveIntent::UseAt {
                            x,
                            y,
                            target_id: r.target_before,
                            actor_id: r.actor_before,
                        },
                    );
                    mark_path_fail_after_use_live(state, conn_id, x, y);
                }
                maybe_lock_say_feedback(state, outbound, conn_id);"#;
        let new_mark = r#"                maybe_lock_say_feedback(state, outbound, conn_id);"#;
        if t.contains(old_mark) {
            t = t.replacen(old_mark, new_mark, 1);
            ch = true;
            let old_else = r#"                } else {
                    debug!(
                        conn_id,
                        x,
                        y,
                        actor = r.actor_before,
                        target = r.target_before,
                        "sim: USE no transition — unstick PU+FM (keep seq)"
                    );
                    state.publish_player_view(conn_id);
                    send_action_result_pu_and_frame(state, outbound, conn_id);
                }
            }
            None => {
                warn!(conn_id, "sim: USE without player");"#;
            let new_else = r#"                } else {
                    // AI-FOOD-FAIL-MARK / PATH-REACH: after eat fallback fails, mark path.
                    // Food 30s when sticky food claim or empty-hand edible; else age-gate.
                    // Haxe: AiBase.isPickingupFood ~8698 / isUsingObject ~9133
                    {
                        use crate::ShortCraftLiveIntent;
                        note_ai_block_targets_from_live_intent(
                            state,
                            conn_id,
                            ShortCraftLiveIntent::UseAt {
                                x,
                                y,
                                target_id: r.target_before,
                                actor_id: r.actor_before,
                            },
                        );
                        mark_path_fail_after_use_live(state, conn_id, x, y);
                    }
                    debug!(
                        conn_id,
                        x,
                        y,
                        actor = r.actor_before,
                        target = r.target_before,
                        "sim: USE no transition — unstick PU+FM (keep seq)"
                    );
                    state.publish_player_view(conn_id);
                    send_action_result_pu_and_frame(state, outbound, conn_id);
                }
            }
            None => {
                warn!(conn_id, "sim: USE without player");"#;
            if t.contains(old_else) {
                t = t.replacen(old_else, new_else, 1);
            } else {
                eprintln!("cargo:warning=AI-FOOD-FAIL-MARK: NetIntent else branch not found");
            }
        }
    }

    if !t.contains("mark_path_fail_after_use_live_food_30s") {
        let test = r#"
    // AI-FOOD-FAIL-MARK: live Player.ai_path_reach 30s on empty-hand edible USE fail
    #[test]
    fn mark_path_fail_after_use_live_food_30s() {
        let mut state = SimState::with_default_empty(test_content());
        let _ = spawn_player(&mut state, 42, "npc@ai.local");
        {
            let p = state.players.get_mut(&42).expect("p");
            p.x = 0;
            p.y = 0;
            p.held_id = 0;
            p.age = 20.0;
        }
        {
            let mut w = state.world.write().unwrap();
            w.set_object(1, 0, 33);
        }
        assert!(mark_path_fail_after_use_live(&mut state, 42, 1, 0));
        let p = state.players.get(&42).expect("p");
        assert!(
            (p.ai_path_reach.not_reachable[&(1, 0)] - crate::NOT_REACHABLE_FOOD_SECS).abs() < 0.01,
            "food USE fail should mark 30s not_reachable"
        );
        let _ = spawn_player(&mut state, 43, "human@test");
        {
            let p = state.players.get_mut(&43).expect("p");
            p.held_id = 0;
            p.age = 20.0;
        }
        assert!(!mark_path_fail_after_use_live(&mut state, 43, 1, 0));
    }

"#;
        let needle = "    #[test]\n    fn baby_wiggle_and_dying_formatters() {\n";
        if t.contains(needle) {
            t = t.replacen(needle, &format!("{test}{needle}"), 1);
            ch = true;
        }
    }

    if !ch && t == normalize_nl(&raw) {
        return already_wired_lib(&raw);
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf)) || already_wired_lib(&t)
}

fn already_wired_lib(t: &str) -> bool {
    t.contains("settle_pending_food_use_fail")
        && t.contains("Some(p) if p.is_ai_body()")
        && t.contains("after eat fallback fails, mark path")
}

fn patch_npc(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("pending_food_xy") && raw.contains("settle_npc_pending_food_action") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;

    if !t.contains("settle_pending_food_use_fail") {
        let old = "    mark_food_path_fail, mark_goto_path_fail, next_step,\n";
        let new =
            "    mark_food_path_fail, mark_goto_path_fail, settle_pending_food_use_fail, next_step,\n";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            ch = true;
        }
    }

    if !t.contains("pending_food_xy") {
        let old = r#"struct NpcFoodGotoState {
    sticky_food: Option<StickyFoodTarget>,
    last_goto: Option<LastGotoObj>,
    last_goto_dist: f32,
    did_not_reach_food: f32,
}

impl Default for NpcFoodGotoState {
    fn default() -> Self {
        Self {
            sticky_food: None,
            last_goto: None,
            last_goto_dist: -1.0,
            did_not_reach_food: 0.0,
        }
    }
}"#;
        let new = r#"struct NpcFoodGotoState {
    sticky_food: Option<StickyFoodTarget>,
    last_goto: Option<LastGotoObj>,
    last_goto_dist: f32,
    did_not_reach_food: f32,
    /// Async food USE/DROP/REMV tile awaiting apply result (AI-FOOD-FAIL-MARK).
    // Haxe: isPickingupFood use/remove/drop returns false sync → mark 30s
    pending_food_xy: Option<(i32, i32)>,
}

impl Default for NpcFoodGotoState {
    fn default() -> Self {
        Self {
            sticky_food: None,
            last_goto: None,
            last_goto_dist: -1.0,
            did_not_reach_food: 0.0,
            pending_food_xy: None,
        }
    }
}"#;
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            ch = true;
        }
    }

    if !t.contains("settle_npc_pending_food_action") {
        let helper = r#"
/// After prior food USE/DROP/REMV was sent, mark 30s if still empty-handed and tile food.
// Haxe: isPickingupFood done==false → addNotReachableObject(food, 30) (AI-FOOD-FAIL-MARK)
fn settle_npc_pending_food_action(
    content: &ContentDb,
    nearby: &[NearbyObj],
    p: &PlayerSnapshot,
    st: &mut NpcProfessionState,
) {
    let pending = st.food_goto.pending_food_xy.take();
    let Some((x, y)) = pending else {
        return;
    };
    let tile_still_food = nearby
        .iter()
        .any(|o| o.x == x && o.y == y && food_at(content, o.id) > 0);
    if settle_pending_food_use_fail(
        &mut st.path_reach,
        &mut st.food_goto.sticky_food,
        Some((x, y)),
        p.held_id,
        tile_still_food,
    ) {
        st.food_goto.last_goto = None;
        st.food_goto.last_goto_dist = -1.0;
    }
}

"#;
        let needle =
            "/// Resolve sticky foodTarget or adopt nearest edible (Haxe isPickingupFood + SearchBestFood).\n";
        if t.contains(needle) {
            t = t.replacen(needle, &format!("{helper}{needle}"), 1);
            ch = true;
        }
    }

    // settle calls (two hungry sites)
    let sites = [
        (
            r#"                    st.path_reach.cleanup(0.2 * think_period as f32);
                    if let Some(food) = resolve_npc_food_target(
                        &content,
                        &nearby,
                        p.x,
                        p.y,
                        &st.path_reach,
                        &mut st.food_goto,
                    ) {"#,
            r#"                    st.path_reach.cleanup(0.2 * think_period as f32);
                    settle_npc_pending_food_action(&content, &nearby, &p, st);
                    if let Some(food) = resolve_npc_food_target(
                        &content,
                        &nearby,
                        p.x,
                        p.y,
                        &st.path_reach,
                        &mut st.food_goto,
                    ) {"#,
        ),
        (
            r#"                st.path_reach.cleanup(0.2 * think_period as f32);
                if let Some(food) = resolve_npc_food_target(
                    &content,
                    &nearby,
                    p.x,
                    p.y,
                    &st.path_reach,
                    &mut st.food_goto,
                ) {"#,
            r#"                st.path_reach.cleanup(0.2 * think_period as f32);
                settle_npc_pending_food_action(&content, &nearby, &p, st);
                if let Some(food) = resolve_npc_food_target(
                    &content,
                    &nearby,
                    p.x,
                    p.y,
                    &st.path_reach,
                    &mut st.food_goto,
                ) {"#,
        ),
    ];
    for (old, new) in sites {
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            ch = true;
        }
    }

    // pending on USE/DROP/REMV success
    for marker in [
        "remv_food id={}",
        "use_food id={}",
        "drop_pickup_food id={}",
    ] {
        if let Some(idx) = t.find(marker) {
            let before = &t[..idx];
            if before
                .rfind("pending_food_xy = Some((x, y))")
                .map(|p| idx - p < 500)
                .unwrap_or(false)
            {
                continue;
            }
            let needle = "st.food_goto.last_goto_dist = -1.0;\n";
            if let Some(pos) = before.rfind(needle) {
                let insert_at = pos + needle.len();
                t.insert_str(
                    insert_at,
                    "                st.food_goto.pending_food_xy = Some((x, y));\n",
                );
                ch = true;
            }
        }
    }

    if !ch {
        return t.contains("pending_food_xy");
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf))
}

/// Build hook entry.
pub fn patch_all(src: &Path, workspace: &Path) -> bool {
    println!("cargo:rerun-if-changed=build_ai_food_fail_mark.rs");
    println!("cargo:rerun-if-changed=src/ai_path_reach.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=docs/port/_apply_ai_food_fail_mark.py");
    println!("cargo:rerun-if-changed=docs/port/_doc_ai_food_fail_only.py");

    run_python(workspace);
    patch_docs(workspace);

    if already_wired(src) {
        // still ensure npc pending if missing
        let npc = workspace
            .join("crates")
            .join("ol-server")
            .join("src")
            .join("npc_ai.rs");
        if npc.exists() {
            let _ = patch_npc(&npc);
        }
        return true;
    }

    let mut ok = true;
    ok &= patch_ai_path_reach(&src.join("ai_path_reach.rs"));
    ok &= patch_lib(&src.join("lib.rs"));

    let npc = workspace
        .join("crates")
        .join("ol-server")
        .join("src")
        .join("npc_ai.rs");
    if npc.exists() {
        let _ = patch_npc(&npc);
    }

    patch_docs(workspace);
    already_wired(src) || ok
}
