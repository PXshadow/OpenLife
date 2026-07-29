//! Build-time wire for **DARK-NOSAJ** / `dark_nosaj_use`.
//!
//! Fully idempotent (safe to re-run on every cargo build):
//! - `Player.praised_jinbali` session field
//! - single `mod dark_nosaj` + re-exports in lib.rs
//! - single USE `apply_monument_use_side_effects` call site + intent feedback
//! - pure/live tests + port docs (no changelog / CALL_INDEX stacking)
//!
//! // Haxe: TransitionHelper.doCommandHelper L144–185 Tarr/Dark Nosaj

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

/// Idempotent replace: if `new` is already present, no-op success.
/// Critical: `new` often contains `old` as a suffix (prepending anchors), so we
/// must NOT re-apply when `old` remains as a substring of the already-injected block.
fn replace_once(hay: &mut String, old: &str, new: &str) -> bool {
    if hay.contains(new) {
        return true; // already applied
    }
    if let Some(i) = hay.find(old) {
        hay.replace_range(i..i + old.len(), new);
        true
    } else {
        false
    }
}

fn write_if_changed(path: &Path, original: &str, next: &str) -> bool {
    if original == next {
        return false;
    }
    if let Err(e) = std::fs::write(path, next) {
        eprintln!("cargo:warning=DARK-NOSAJ write {}: {e}", path.display());
        return false;
    }
    true
}

/// True when player/use/lib already have the monument USE surface (heal-only path).
pub fn dark_nosaj_wired(player: &str, use_tr: &str, lib: &str) -> bool {
    player.contains("praised_jinbali")
        && use_tr.contains("fn apply_monument_use_side_effects")
        && use_tr.contains("apply_monument_use_side_effects(state, conn_id, actor, target)")
        && use_tr.contains("DARK-NOSAJ")
        && lib.contains("mod dark_nosaj")
        && lib.contains("maybe_monument_feedback")
        && lib.contains("dark_nosaj_use_live_set_clear_wire")
}

/// Full patch (or self-heal) of DARK-NOSAJ surfaces. Always safe / idempotent.
pub fn patch_dark_nosaj(ol_sim_src: &Path, workspace: &Path) -> bool {
    let player = ol_sim_src.join("player.rs");
    let use_tr = ol_sim_src.join("use_transition.rs");
    let lib = ol_sim_src.join("lib.rs");
    let port = workspace.join("docs/port");

    let p = patch_player(&player);
    let u = patch_use(&use_tr);
    let l = patch_lib(&lib);
    let _ = patch_docs(&port);
    p && u && l
}

/// Heal stacked injections only (called when already wired so rebuilds cannot re-stack).
pub fn heal_dark_nosaj_stacking(ol_sim_src: &Path, workspace: &Path) -> bool {
    let use_tr = ol_sim_src.join("use_transition.rs");
    let lib = ol_sim_src.join("lib.rs");
    let port = workspace.join("docs/port");
    let u = heal_use_call_sites(&use_tr);
    let l = heal_lib_mod(&lib);
    let _ = heal_docs_duplicates(&port);
    u && l
}

fn patch_player(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    if t.contains("pub praised_jinbali: bool") {
        return true;
    }
    let mut any = false;
    any |= replace_once(
        &mut t,
        "    // Haxe: GlobalPlayerInstance.darkNosaj (not saved)\n    // WALLET-COINS\n    pub dark_nosaj: f32,\n    /// Haxe `lastAttackedPlayer` p_id (0 = none) — breaks isFriendly with target.\n",
        "    // Haxe: GlobalPlayerInstance.darkNosaj (not saved)\n    // WALLET-COINS\n    pub dark_nosaj: f32,\n    /// Haxe `GlobalPlayerInstance.praisedJinbali` — Tarr praise flag (session; not saved).\n    // Haxe: GlobalPlayerInstance.praisedJinbali\n    // DARK-NOSAJ\n    pub praised_jinbali: bool,\n    /// Haxe `lastAttackedPlayer` p_id (0 = none) — breaks isFriendly with target.\n",
    );
    any |= replace_once(
        &mut t,
        "            // WALLET-COINS: darkNosaj session-only (Haxe not saved)\n            dark_nosaj: 0.0,\n            last_attacked_player_id: 0,\n",
        "            // WALLET-COINS: darkNosaj session-only (Haxe not saved)\n            dark_nosaj: 0.0,\n            // DARK-NOSAJ: praisedJinbali session flag\n            praised_jinbali: false,\n            last_attacked_player_id: 0,\n",
    );
    if !any {
        eprintln!("cargo:warning=DARK-NOSAJ: player.rs anchors missing");
        return false;
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf));
    true
}

const USE_HELPER: &str = r#"
/// DARK-NOSAJ: Haxe TransitionHelper Tarr/Dark Nosaj monument side-effects on USE.
// Haxe: TransitionHelper.doCommandHelper L144–185
fn apply_monument_use_side_effects(
    state: &mut SimState,
    conn_id: u64,
    actor: i32,
    target: i32,
) {
    if target == 0 {
        return;
    }
    let target_parent = state.content.resolve_base_id(target);
    let held_parent = if actor == 0 {
        0
    } else {
        state.content.resolve_base_id(actor)
    };
    let (dark, praised, p_id, age, food, exh, true_age) = {
        let Some(p) = state.players.get(&conn_id) else {
            return;
        };
        (
            p.dark_nosaj,
            p.praised_jinbali,
            p.p_id,
            p.age,
            p.food,
            p.exhaustion,
            p.true_age,
        )
    };
    let Some(plan) =
        crate::dark_nosaj::plan_monument_use(target_parent, held_parent, dark, praised)
    else {
        return;
    };

    if let Some(p) = state.players.get_mut(&conn_id) {
        p.dark_nosaj = plan.dark_nosaj;
        p.praised_jinbali = plan.praised_jinbali;
    }

    if plan.yum_delta != 0.0 && plan.yum_delta.is_finite() {
        {
            let s = state.combat.stats_mut(p_id);
            s.prestige += plan.yum_delta;
        }
        if let Some(n) = state.social.lineages.get_mut(&p_id) {
            n.add_prestige(plan.yum_delta);
        }
    }

    if plan.lost_combat_delta != 0.0 || plan.lost_combat_floor_zero {
        let before = state.reputation.lost_combat(p_id);
        let after = crate::dark_nosaj::apply_lost_combat_delta(
            before,
            plan.lost_combat_delta,
            plan.lost_combat_floor_zero,
        );
        state.reputation.set_from_lost_combat(p_id, after);
        state.combat.stats_mut(p_id).lost_combat_prestige = after;
    }

    let mut hits = state.combat.hits_of(p_id);
    if plan.hits_delta != 0.0 && plan.hits_delta.is_finite() {
        hits = (hits + plan.hits_delta).max(0.0);
        state.combat.stats_mut(p_id).hits = hits;
    }
    if plan.yum_delta != 0.0 || plan.hits_delta != 0.0 {
        let health_f = state.player_health_food_store_max_factor(p_id, true_age);
        let knobs = state.gameplay.food_store_max_knobs();
        let new_max = crate::food_store_max_from_parts_ex(age, food, hits, exh, health_f, knobs);
        if let Some(p) = state.players.get_mut(&conn_id) {
            p.food_max = new_max;
            if p.food > new_max {
                p.food = new_max;
            }
        }
    }

    crate::dark_nosaj::note_monument_feedback(crate::dark_nosaj::MonumentFeedback {
        conn_id,
        say: plan.say,
        curse: plan.curse,
    });
}

"#;

const USE_TESTS: &str = r#"

    /// DARK-NOSAJ: empty-hand USE on 2466 sets dark_nosaj; Tarr 3112 clears.
    // Haxe: TransitionHelper L144–185
    #[test]
    fn dark_nosaj_monument_use_sets_and_clears() {
        use crate::dark_nosaj::{
            take_monument_feedback, DARK_NOSAJ_MONUMENT_ID, TARR_MONUMENT_ID,
            CURSE_CLEAR_WORD, CURSE_DARK_MINION_WORD,
        };
        let _ = take_monument_feedback();
        let mut state = state_with(ContentDb::default());
        let p_id = crate::spawn_player(&mut state, 1, "dn@test");
        apply_monument_use_side_effects(&mut state, 1, 0, DARK_NOSAJ_MONUMENT_ID);
        let pl = state.players.get(&1).unwrap();
        assert!((pl.dark_nosaj - 1.0).abs() < 1e-5, "dark_nosaj={}", pl.dark_nosaj);
        assert!(!pl.praised_jinbali);
        assert!((state.reputation.lost_combat(p_id) - 100.0).abs() < 1e-3);
        let prest = state.combat.stats.get(&p_id).map(|s| s.prestige).unwrap_or(0.0);
        assert!((prest + 100.0).abs() < 1e-2, "prestige={prest}");
        let fb = take_monument_feedback().expect("set feedback");
        assert_eq!(fb.say, "All hail dark nosaj");
        assert_eq!(fb.curse, Some((1, Some(CURSE_DARK_MINION_WORD))));

        apply_monument_use_side_effects(&mut state, 1, 0, TARR_MONUMENT_ID);
        let pl = state.players.get(&1).unwrap();
        assert_eq!(pl.dark_nosaj, 0.0);
        assert!((state.reputation.lost_combat(p_id) - 10.0).abs() < 1e-3);
        let fb = take_monument_feedback().expect("clear feedback");
        assert_eq!(fb.say, "Jasoniah is the one true god!");
        assert_eq!(fb.curse, Some((0, Some(CURSE_CLEAR_WORD))));
    }

    /// DARK-NOSAJ: praise path then dark nosaj punish (+hits).
    #[test]
    fn dark_nosaj_praise_then_punish() {
        use crate::dark_nosaj::{
            take_monument_feedback, DARK_NOSAJ_MONUMENT_ID, TARR_MONUMENT_ID,
        };
        let _ = take_monument_feedback();
        let mut state = state_with(ContentDb::default());
        let p_id = crate::spawn_player(&mut state, 1, "praise@test");
        apply_monument_use_side_effects(&mut state, 1, 0, TARR_MONUMENT_ID);
        assert!(state.players.get(&1).unwrap().praised_jinbali);
        let _ = take_monument_feedback();
        apply_monument_use_side_effects(&mut state, 1, 0, DARK_NOSAJ_MONUMENT_ID);
        let pl = state.players.get(&1).unwrap();
        assert!(!pl.praised_jinbali);
        assert!((state.combat.hits_of(p_id) - 10.0).abs() < 1e-3);
        let fb = take_monument_feedback().unwrap();
        assert_eq!(fb.say, "AAAAAAAAAAAAAAAAAAAAaaaa!!!");
        assert!(fb.curse.is_none());
    }
"#;

/// Collapse stacked `apply_monument_use_side_effects` call blocks to exactly one.
fn heal_use_call_sites(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    // Exact 3-line call block as it appears in source (4-space indent each line).
    let needle_one = "    // Haxe: TransitionHelper.doCommandHelper Tarr/Dark Nosaj monuments (side-effects).\n    // DARK-NOSAJ — runs even when no transition applies (matches Haxe early hook).\n    apply_monument_use_side_effects(state, conn_id, actor, target);\n\n";
    let double = format!("{needle_one}{needle_one}");
    let mut changed = false;
    while t.contains(&double) {
        if let Some(i) = t.find(&double) {
            t.replace_range(i..i + double.len(), needle_one);
            changed = true;
        } else {
            break;
        }
    }
    // Ensure the single call exists before horse mount (if helper fn exists).
    if t.contains("fn apply_monument_use_side_effects")
        && !t.contains("apply_monument_use_side_effects(state, conn_id, actor, target)")
    {
        if replace_once(
            &mut t,
            "    // Haxe: TransitionHelper.use → doHorseStuffPossible (eat while mounted).\n    if is_horse_mount_held(actor) && target != 0 {\n",
            &format!(
                "{needle_one}    // Haxe: TransitionHelper.use → doHorseStuffPossible (eat while mounted).\n    if is_horse_mount_held(actor) && target != 0 {{\n"
            ),
        ) {
            changed = true;
        }
    }
    if changed {
        write_if_changed(path, &raw, &restore_nl(&t, crlf));
    }
    true
}

fn patch_use(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut any = false;

    any |= replace_once(
        &mut t,
        "//! Chunk: **TH-MULTI** / **TH-MULTI-POLISH** / **TH-HORSE** / **HORSE-MOUNT-POLISH** / **TH-LOCK** / **EATEN-FOOD-PCT**\n",
        "//! Chunk: **TH-MULTI** / **TH-MULTI-POLISH** / **TH-HORSE** / **HORSE-MOUNT-POLISH** / **TH-LOCK** / **EATEN-FOOD-PCT** / **DARK-NOSAJ**\n",
    );

    if !t.contains("fn apply_monument_use_side_effects") {
        let old = "pub fn apply_use_at(\n    state: &mut SimState,\n    conn_id: u64,\n    tx: i32,\n    ty: i32,\n) -> Option<UseResult> {\n";
        let new = format!("{USE_HELPER}{old}");
        any |= replace_once(&mut t, old, &new);
    } else {
        any = true;
    }

    // Inject call site only when missing (never re-prepend; new contains old).
    if !t.contains("apply_monument_use_side_effects(state, conn_id, actor, target)") {
        any |= replace_once(
            &mut t,
            "    // Haxe: TransitionHelper.use → doHorseStuffPossible (eat while mounted).\n    if is_horse_mount_held(actor) && target != 0 {\n",
            "    // Haxe: TransitionHelper.doCommandHelper Tarr/Dark Nosaj monuments (side-effects).\n    // DARK-NOSAJ — runs even when no transition applies (matches Haxe early hook).\n    apply_monument_use_side_effects(state, conn_id, actor, target);\n\n    // Haxe: TransitionHelper.use → doHorseStuffPossible (eat while mounted).\n    if is_horse_mount_held(actor) && target != 0 {\n",
        );
    } else {
        // Self-heal stacked call sites from prior non-idempotent builds.
        let needle_one = "    // Haxe: TransitionHelper.doCommandHelper Tarr/Dark Nosaj monuments (side-effects).\n    // DARK-NOSAJ — runs even when no transition applies (matches Haxe early hook).\n    apply_monument_use_side_effects(state, conn_id, actor, target);\n\n";
        let double = format!("{needle_one}{needle_one}");
        while t.contains(&double) {
            if let Some(i) = t.find(&double) {
                t.replace_range(i..i + double.len(), needle_one);
            } else {
                break;
            }
        }
        any = true; // call site present (possibly healed)
    }

    if !t.contains("dark_nosaj_monument_use_sets_and_clears") {
        if let Some(i) = t.rfind("\n}") {
            t.insert_str(i, USE_TESTS);
            any = true;
        }
    }

    if !any && t.contains("apply_monument_use_side_effects") {
        return true;
    }
    if !any {
        eprintln!("cargo:warning=DARK-NOSAJ: use_transition.rs anchors missing");
        return false;
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf));
    true
}

const FEEDBACK_FN: &str = r#"
/// DARK-NOSAJ: public say + CU broadcast after Tarr/Dark Nosaj monument USE.
// Haxe: TransitionHelper player.say + Connection.SendCurseToAll
fn maybe_monument_feedback(state: &mut SimState, outbound: &OutboundHub, conn_id: u64) {
    let Some(fb) = crate::dark_nosaj::take_monument_feedback() else {
        return;
    };
    if fb.conn_id != conn_id {
        crate::dark_nosaj::note_monument_feedback(fb);
        return;
    }
    let Some(p) = state.players.get(&conn_id) else {
        return;
    };
    let p_id = p.p_id;
    let (x, y) = (p.x, p.y);
    let age = p.age;
    let say_text = fb.say.to_ascii_uppercase();
    let near = nearby_conn_ids(state, x, y, chat_range_for_age(age));
    send_chat_ps(state, outbound, conn_id, p_id, &say_text, &near);
    if let Some((level, word)) = fb.curse {
        let cu = crate::dark_nosaj::format_cursed_message_word(p_id, level, word);
        outbound.broadcast(cu.into_bytes());
    }
}

"#;

const LIVE_TEST: &str = r#"
    /// DARK-NOSAJ: live USE set/clear mutates Player.dark_nosaj + CU word wire.
    // Haxe: TransitionHelper L144–185 + Connection.SendCurseToAll
    #[test]
    fn dark_nosaj_use_live_set_clear_wire() {
        use crate::dark_nosaj::{DARK_NOSAJ_MONUMENT_ID, TARR_MONUMENT_ID};
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx = hub.register(1);
        let mut state = SimState::with_default_empty(test_content());
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Login {
                conn_id: 1,
                reconnect: false,
                email: "dark@nosaj".into(),
                client_tag: "t".into(),
            },
        );
        let (px, py, p_id) = {
            let p = state.players.get(&1).unwrap();
            (p.x, p.y, p.p_id)
        };
        {
            let mut w = state.world.write().unwrap();
            w.set_object(px, py, DARK_NOSAJ_MONUMENT_ID);
        }
        while rx.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Use {
                conn_id: 1,
                x: 0,
                y: 0,
                id: -1,
                index: -1,
            },
        );
        let pl = state.players.get(&1).unwrap();
        assert!(
            pl.dark_nosaj.is_finite() && pl.dark_nosaj >= 1.0,
            "dark_nosaj after set = {}",
            pl.dark_nosaj
        );
        let mut saw_cu_minion = false;
        let mut saw_hail = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains("DARK_MINION") {
                saw_cu_minion = true;
            }
            if s.to_ascii_uppercase().contains("ALL HAIL DARK NOSAJ") {
                saw_hail = true;
            }
        }
        assert!(saw_cu_minion, "expected CU … DARK_MINION");
        assert!(saw_hail, "expected public ALL HAIL say");

        {
            let mut w = state.world.write().unwrap();
            w.set_object(px, py, TARR_MONUMENT_ID);
        }
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Use {
                conn_id: 1,
                x: 0,
                y: 0,
                id: -1,
                index: -1,
            },
        );
        assert_eq!(state.players.get(&1).unwrap().dark_nosaj, 0.0);
        let mut saw_cu_clear = false;
        let mut saw_jasoniah = false;
        while let Ok(pkt) = rx.try_recv() {
            let s = String::from_utf8_lossy(&pkt);
            if s.contains(" 0 _") {
                saw_cu_clear = true;
            }
            if s.to_ascii_uppercase().contains("JASONIAH") {
                saw_jasoniah = true;
            }
        }
        assert!(saw_cu_clear, "expected CU clear with _ word");
        assert!(saw_jasoniah, "expected Jasoniah say");
        let _ = p_id;
    }

"#;

/// Collapse stacked `mod dark_nosaj` declarations to one.
fn heal_lib_mod(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let single =
        "// Haxe: TransitionHelper Dark Nosaj / Tarr monument USE (DARK-NOSAJ)\nmod dark_nosaj;\n";
    let double = format!("{single}{single}");
    let mut changed = false;
    while t.contains(&double) {
        if let Some(i) = t.find(&double) {
            t.replace_range(i..i + double.len(), single);
            changed = true;
        } else {
            break;
        }
    }
    // Strip bare duplicate `mod dark_nosaj;` after first (with optional preceding comment).
    let mut out = String::new();
    let mut seen = false;
    let comment = "// Haxe: TransitionHelper Dark Nosaj / Tarr monument USE (DARK-NOSAJ)";
    for line in t.lines() {
        if line.trim() == "mod dark_nosaj;" {
            if seen {
                // drop preceding DARK-NOSAJ comment line if present
                if out.ends_with(&(comment.to_string() + "\n")) {
                    out.truncate(out.len() - comment.len() - 1);
                }
                changed = true;
                continue;
            }
            seen = true;
        }
        out.push_str(line);
        out.push('\n');
    }
    if !t.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    if changed {
        write_if_changed(path, &raw, &restore_nl(&out, crlf));
    }
    true
}

fn patch_lib(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut any = false;

    // Self-heal stacked mods first.
    let single =
        "// Haxe: TransitionHelper Dark Nosaj / Tarr monument USE (DARK-NOSAJ)\nmod dark_nosaj;\n";
    let double = format!("{single}{single}");
    while t.contains(&double) {
        if let Some(i) = t.find(&double) {
            t.replace_range(i..i + double.len(), single);
            any = true;
        } else {
            break;
        }
    }

    if !t.contains("mod dark_nosaj") {
        any |= replace_once(
            &mut t,
            "mod curse;\n",
            "mod curse;\n// Haxe: TransitionHelper Dark Nosaj / Tarr monument USE (DARK-NOSAJ)\nmod dark_nosaj;\n",
        );
    } else {
        any = true;
    }

    if !t.contains("pub use dark_nosaj::") {
        any |= replace_once(
            &mut t,
            "pub use use_transition::{apply_use_at, place_after_use, place_after_use_ex, wire_held_id};\n",
            "pub use use_transition::{apply_use_at, place_after_use, place_after_use_ex, wire_held_id};\npub use dark_nosaj::{\n    apply_lost_combat_delta, format_cursed_message_word, plan_monument_use,\n    MonumentFeedback, MonumentUsePlan, CURSE_CLEAR_WORD, CURSE_DARK_MINION_WORD,\n    DARK_NOSAJ_MONUMENT_ID, TARR_MONUMENT_ID,\n};\n",
        );
    }

    if !t.contains("fn maybe_monument_feedback") {
        any |= replace_once(
            &mut t,
            "/// Haxe key/lock private say (`KEY DOES NOT FIT!`, lockpick feedback, etc.).\n// Haxe: TransitionHelper.doCommandHelper / LockPick player.say(..., true)\nfn maybe_lock_say_feedback(state: &mut SimState, outbound: &OutboundHub, conn_id: u64) {\n",
            &format!("{FEEDBACK_FN}/// Haxe key/lock private say (`KEY DOES NOT FIT!`, lockpick feedback, etc.).\n// Haxe: TransitionHelper.doCommandHelper / LockPick player.say(..., true)\nfn maybe_lock_say_feedback(state: &mut SimState, outbound: &OutboundHub, conn_id: u64) {{\n"),
        );
    }

    if !t.contains("maybe_monument_feedback(state, outbound, conn_id)") {
        any |= replace_once(
            &mut t,
            "                // Haxe TransitionHelper key/lock say(..., true)\n                maybe_lock_say_feedback(state, outbound, conn_id);\n                // Ranged too-close should not apply; drain any stale flag.\n                maybe_too_close_say_feedback(state, outbound, conn_id);\n            }\n            Some(r) => {\n",
            "                // Haxe TransitionHelper key/lock say(..., true)\n                maybe_lock_say_feedback(state, outbound, conn_id);\n                // DARK-NOSAJ: monument public say + CU\n                maybe_monument_feedback(state, outbound, conn_id);\n                // Ranged too-close should not apply; drain any stale flag.\n                maybe_too_close_say_feedback(state, outbound, conn_id);\n            }\n            Some(r) => {\n",
        );
        any |= replace_once(
            &mut t,
            "                let _ = crate::loved_food_wire::take_loved_food_extra();\n                maybe_lock_say_feedback(state, outbound, conn_id);\n                // Haxe TransitionHelper.use say('Too close...') when bow min-range refuse\n                maybe_too_close_say_feedback(state, outbound, conn_id);\n",
            "                let _ = crate::loved_food_wire::take_loved_food_extra();\n                maybe_lock_say_feedback(state, outbound, conn_id);\n                // DARK-NOSAJ: monument side-effects even when transition did not apply\n                maybe_monument_feedback(state, outbound, conn_id);\n                // Haxe TransitionHelper.use say('Too close...') when bow min-range refuse\n                maybe_too_close_say_feedback(state, outbound, conn_id);\n",
        );
    }

    if !t.contains("dark_nosaj_use_live_set_clear_wire") {
        for marker in [
            "\n    /// SAY DONATE / ?TREASURY move coins into Economy.treasury.\n",
            "\n    #[test]\n    fn say_donate_and_treasury_query() {\n",
        ] {
            if t.contains(marker) {
                any |= replace_once(&mut t, marker, &format!("{LIVE_TEST}{marker}"));
                break;
            }
        }
    }

    if !any && t.contains("mod dark_nosaj") && t.contains("maybe_monument_feedback") {
        return true;
    }
    if !any {
        eprintln!("cargo:warning=DARK-NOSAJ: lib.rs anchors missing");
        return false;
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf));
    true
}

fn heal_docs_duplicates(port: &Path) -> bool {
    // Collapse duplicated DARK-NOSAJ rows in CALL_INDEX and changelog in TODO_PORT.
    let call = port.join("CALL_INDEX.md");
    if let Ok(raw) = std::fs::read_to_string(&call) {
        let crlf = raw.contains("\r\n");
        let t = normalize_nl(&raw);
        let keep_once = [
            "| Rust `Player.praised_jinbali` |",
            "| Rust `plan_monument_use` / `format_cursed_message_word` |",
            "| Rust `apply_monument_use_side_effects` |",
            "| Rust `maybe_monument_feedback` |",
            "| Rust `dark_nosaj_attack_damage_mul` / `blocks_health_and_prestige` |",
        ];
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for line in t.lines() {
            let mut key: Option<&str> = None;
            for k in &keep_once {
                if line.starts_with(k) {
                    key = Some(*k);
                    break;
                }
            }
            if let Some(k) = key {
                if !seen.insert(k) {
                    continue;
                }
            }
            out.push(line.to_string());
        }
        let next = out.join("\n")
            + if t.ends_with('\n') { "\n" } else { "" };
        write_if_changed(&call, &raw, &restore_nl(&next, crlf));
    }

    let todo = port.join("TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo) {
        let crlf = raw.contains("\r\n");
        let t = normalize_nl(&raw);
        let marker = "| 2026-07-29 | **DARK-NOSAJ dark_nosaj_use**:";
        let mut out = Vec::new();
        let mut seen = false;
        for line in t.lines() {
            if line.starts_with(marker) {
                if seen {
                    continue;
                }
                seen = true;
            }
            out.push(line.to_string());
        }
        let next = out.join("\n")
            + if t.ends_with('\n') { "\n" } else { "" };
        write_if_changed(&todo, &raw, &restore_nl(&next, crlf));
    }
    true
}

fn patch_docs(port: &Path) -> bool {
    // Always heal duplicates first.
    let _ = heal_docs_duplicates(port);

    // FILE_MATRIX — insert once
    let matrix = port.join("FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&matrix) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if !t.contains("**DARK-NOSAJ**") {
            if replace_once(
                &mut t,
                "| **WALLET-COINS** / take_coins | takeCoins wallet on wound/damage | **DONE** | coins_stolen + economy gift; HIT lethal+equip; Player.dark_nosaj + CoinsOnWounding LiveSettings; residual Dark Nosaj monument USE / i32 wallet floor |\n",
                "| **WALLET-COINS** / take_coins | takeCoins wallet on wound/damage | **DONE** | coins_stolen + economy gift; HIT lethal+equip; Player.dark_nosaj + CoinsOnWounding LiveSettings; residual i32 wallet floor |\n| **DARK-NOSAJ** / dark_nosaj_use | Tarr 3112 + Dark Nosaj 2466 USE set/clear | **DONE** | pure `plan_monument_use`; Player.praised_jinbali; USE side-effects + CU word + public say |\n",
            ) {
                write_if_changed(&matrix, &raw, &restore_nl(&t, crlf));
            }
        }
    }

    let todo = port.join("TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        let mut any = false;
        if !t.contains("**DARK-NOSAJ dark_nosaj_use**") {
            any |= replace_once(
                &mut t,
                "- [x] **WALLET-COINS take_coins** — pure `coins_stolen_on_wound` / `take_coins_say_text`; `Economy::take_coins_on_wound`; live HIT `apply_take_coins_on_wound` lethal+equip; scoreboard + say; `Player.dark_nosaj` doubles factor + combat-restore gate; LiveSettings `coins_on_wounding_factor` (Haxe 0.5); residual Dark Nosaj monument USE (2466/3112) set/clear; i32 wallet floor vs Haxe Float coins  \n",
                "- [x] **WALLET-COINS take_coins** — pure `coins_stolen_on_wound` / `take_coins_say_text`; `Economy::take_coins_on_wound`; live HIT `apply_take_coins_on_wound` lethal+equip; scoreboard + say; `Player.dark_nosaj` doubles factor + combat-restore gate; LiveSettings `coins_on_wounding_factor` (Haxe 0.5); residual i32 wallet floor vs Haxe Float coins  \n- [x] **DARK-NOSAJ dark_nosaj_use** — pure `plan_monument_use` / `format_cursed_message_word`; `Player.praised_jinbali` + `dark_nosaj` set/clear; USE Tarr 3112 / Dark Nosaj 2466 side-effects (yum prestige, lost_combat, hits); public say + CU `_`/`DARK_MINION` broadcast; tests pure + use_transition + live wire  \n",
            );
        }
        // Changelog: only one DARK-NOSAJ history line (heal already collapses extras).
        if !t.contains("**DARK-NOSAJ dark_nosaj_use**:") {
            any |= replace_once(
                &mut t,
                "| 2026-07-29 | **WALLET-COINS take_coins** (gap-close): `Player.dark_nosaj` session field; HIT `apply_take_coins_on_wound` reads attacker dark_nosaj + live `GameplayKnobs.coins_on_wounding_factor`; LiveSettings/ServerConfig/FIELD_MAP CoinsOnWoundingFactor; combat-reputation restore uses dark_nosaj gate; tests live factor + dark_nosaj; residual monument USE set/clear (2466/3112), i32 wallet vs Float coins |\n",
                "| 2026-07-29 | **WALLET-COINS take_coins** (gap-close): `Player.dark_nosaj` session field; HIT `apply_take_coins_on_wound` reads attacker dark_nosaj + live `GameplayKnobs.coins_on_wounding_factor`; LiveSettings/ServerConfig/FIELD_MAP CoinsOnWoundingFactor; combat-reputation restore uses dark_nosaj gate; tests live factor + dark_nosaj; residual monument USE set/clear (2466/3112), i32 wallet vs Float coins |\n| 2026-07-29 | **DARK-NOSAJ dark_nosaj_use**: pure `plan_monument_use` (Tarr 3112 clear / Dark Nosaj 2466 set + praise punish); `Player.praised_jinbali`; USE side-effects + CU word + public say; tests pure + live |\n",
            );
        }
        if any {
            write_if_changed(&todo, &raw, &restore_nl(&t, crlf));
        }
    }

    let call = port.join("CALL_INDEX.md");
    if let Ok(raw) = std::fs::read_to_string(&call) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if !t.contains("plan_monument_use") {
            if replace_once(
                &mut t,
                "| Rust `Player.dark_nosaj` | `ol-sim/player.rs` | session field (not saved); ×2 factor / restore gate |\n",
                "| Rust `Player.dark_nosaj` | `ol-sim/player.rs` | session field (not saved); ×2 factor / restore gate |\n| Rust `Player.praised_jinbali` | `ol-sim/player.rs` | session; Tarr praise / Dark Nosaj punish (**DARK-NOSAJ**) |\n| Rust `plan_monument_use` / `format_cursed_message_word` | `ol-sim/dark_nosaj.rs` | pure Tarr 3112 + Dark Nosaj 2466 USE plan |\n| Rust `apply_monument_use_side_effects` | `ol-sim/use_transition.rs` | live USE side-effects (prestige/lost/hits) |\n| Rust `maybe_monument_feedback` | `ol-sim/lib.rs` | public say + CU broadcast with word |\n",
            ) {
                write_if_changed(&call, &raw, &restore_nl(&t, crlf));
            }
        }
    }

    let queue = port.join("QUEUE.md");
    if let Ok(raw) = std::fs::read_to_string(&queue) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        let mut any = false;
        any |= replace_once(
            &mut t,
            "| `DARK-NOSAJ` | dark_nosaj_use | Dark Nosaj monument USE set/clear residual |\n",
            "",
        );
        if !t.contains("**DARK-NOSAJ** DONE") {
            any |= replace_once(
                &mut t,
                "**WALLET-COINS** DONE ·",
                "**DARK-NOSAJ** DONE · **WALLET-COINS** DONE ·",
            );
        }
        if any {
            write_if_changed(&queue, &raw, &restore_nl(&t, crlf));
        }
    }

    true
}
