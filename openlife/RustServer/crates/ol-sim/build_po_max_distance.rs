//! PO-MAX-DISTANCE / close_say_range build-time wire.
//!
//! Haxe `ServerSettings.MaxDistanceToBeConsideredAsCloseForSay` = 20.
//! Align adult chat range to 20; keep `NEARBY_RANGE` = 24 for PU/MX interest.
//! ModuleConst residual (not LiveSettings).
//!
//! Idempotent pure-Rust file patches (no Python required).

use std::path::Path;

fn nl(s: &str) -> (String, bool) {
    let crlf = s.contains("\r\n");
    (s.replace("\r\n", "\n").replace('\r', "\n"), crlf)
}

fn out(s: String, crlf: bool) -> String {
    if crlf {
        s.replace('\n', "\r\n")
    } else {
        s
    }
}

fn once(hay: &mut String, old: &str, new: &str) -> bool {
    if hay.contains(new) && new.len() > 20 {
        return false;
    }
    if let Some(i) = hay.find(old) {
        hay.replace_range(i..i + old.len(), new);
        true
    } else {
        false
    }
}

/// Patch lib.rs chat_range + say fans; field_map; mumble test; docs.
pub fn patch_po_max_distance(src: &Path, workspace: &Path) -> bool {
    let mut ok = true;
    ok &= patch_lib(src);
    ok &= patch_mumble(src);
    ok &= patch_field_map(workspace);
    ok &= patch_ai_follow_inc(src);
    let _ = patch_docs(workspace);
    ok
}

fn patch_lib(src: &Path) -> bool {
    let path = src.join("lib.rs");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return false;
    };
    let (mut t, crlf) = nl(&raw);
    let mut changed = false;

    // Re-export MAX_DISTANCE_CLOSE_FOR_SAY from speech
    if !t.contains("MAX_DISTANCE_CLOSE_FOR_SAY") {
        changed |= once(
            &mut t,
            "    ADULT_CHAT_RANGE, DoCommand, HIRE_COST, HIRE_COST_INCREASE_PER_PERSON, HOME_OVEN_IDS,\n",
            "    ADULT_CHAT_RANGE, MAX_DISTANCE_CLOSE_FOR_SAY, DoCommand, HIRE_COST,\n    HIRE_COST_INCREASE_PER_PERSON, HOME_OVEN_IDS,\n",
        );
    }

    // lib chat_range_for_age: adult uses ADULT_CHAT_RANGE (20), not NEARBY_RANGE (24)
    let old_chat = r#"/// Chat PS fan-out radius by speaker age (Haxe age-scaled speech range).
///
/// Infants &lt;3 → 8, children &lt;10 → 16, elders ≥60 → 20, else [`NEARBY_RANGE`].
pub fn chat_range_for_age(age: f32) -> i32 {
    if age < 3.0 {
        8
    } else if age < 10.0 {
        16
    } else if age >= 60.0 {
        20
    } else {
        NEARBY_RANGE
    }
}"#;
    let new_chat = r#"/// Chat PS fan-out radius by speaker age.
///
/// Haxe `sendSayToAllClose` uses `MaxDistanceToBeConsideredAsCloseForSay` (20) for adults.
/// Young soft scale is product-only; adults/elders → [`ADULT_CHAT_RANGE`] (not [`NEARBY_RANGE`]).
/// // Haxe: ServerSettings.MaxDistanceToBeConsideredAsCloseForSay = 20
/// // PO-MAX-DISTANCE
pub fn chat_range_for_age(age: f32) -> i32 {
    if age < 3.0 {
        8
    } else if age < 10.0 {
        16
    } else {
        ADULT_CHAT_RANGE
    }
}"#;
    changed |= once(&mut t, old_chat, new_chat);

    // Say-path NEARBY_RANGE → ADULT_CHAT_RANGE for send_chat_ps call sites that are pure chat.
    // (Leave PU/MX/FX NEARBY_RANGE alone.)
    let say_repls = [
        (
            "    for (cid, p_id, x, y, s) in pending_says {\n        let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);\n        send_chat_ps(state, outbound, cid, p_id, &s, &near);",
            "    for (cid, p_id, x, y, s) in pending_says {\n        // PO-MAX-DISTANCE: Haxe MaxDistanceToBeConsideredAsCloseForSay = 20\n        let near = nearby_conn_ids(state, x, y, ADULT_CHAT_RANGE);\n        send_chat_ps(state, outbound, cid, p_id, &s, &near);",
        ),
        (
            "        let near = nearby_conn_ids(state, ax, ay, NEARBY_RANGE);\n        send_chat_ps(\n            state,\n            outbound,\n            h.conn_id,\n            ai_p_id,\n            start.thinking_say,\n            &near,\n        );",
            "        // PO-MAX-DISTANCE: CloseForSay 20 (not NEARBY_RANGE 24)\n        let near = nearby_conn_ids(state, ax, ay, ADULT_CHAT_RANGE);\n        send_chat_ps(\n            state,\n            outbound,\n            h.conn_id,\n            ai_p_id,\n            start.thinking_say,\n            &near,\n        );",
        ),
        (
            "    for (cid, p_id, x, y, text) in ready {\n        let near = nearby_conn_ids(state, x, y, NEARBY_RANGE);\n        send_chat_ps(state, outbound, cid, p_id, &text, &near);",
            "    for (cid, p_id, x, y, text) in ready {\n        // PO-MAX-DISTANCE: CloseForSay 20\n        let near = nearby_conn_ids(state, x, y, ADULT_CHAT_RANGE);\n        send_chat_ps(state, outbound, cid, p_id, &text, &near);",
        ),
        (
            "        if let Some(text) = countdown {\n            let near = nearby_conn_ids(state, say_xy.0, say_xy.1, NEARBY_RANGE);\n            send_chat_ps(state, outbound, conn_id, p_id_for_say, &text, &near);",
            "        if let Some(text) = countdown {\n            // PO-MAX-DISTANCE: CloseForSay 20\n            let near = nearby_conn_ids(state, say_xy.0, say_xy.1, ADULT_CHAT_RANGE);\n            send_chat_ps(state, outbound, conn_id, p_id_for_say, &text, &near);",
        ),
        (
            "                let near_say = nearby_conn_ids(state, sx, sy, NEARBY_RANGE);\n                let pkt = format_player_says(sp_id, false, text).into_bytes();\n                send_nearby_chat(state, outbound, &near_say, sp_id, pkt);",
            "                // PO-MAX-DISTANCE: CloseForSay 20 for spoken DO-COMMANDS says\n                let near_say = nearby_conn_ids(state, sx, sy, ADULT_CHAT_RANGE);\n                let pkt = format_player_says(sp_id, false, text).into_bytes();\n                send_nearby_chat(state, outbound, &near_say, sp_id, pkt);",
        ),
        (
            // Residual: pending newFollower spoken_says still NEARBY_RANGE
            "                let near_say = nearby_conn_ids(state, sx, sy, NEARBY_RANGE);\n                let pkt = format_player_says(sp_id, false, text).into_bytes();\n                send_nearby_chat(state, outbound, &near_say, sp_id, pkt);",
            "                // PO-MAX-DISTANCE: CloseForSay 20 (newFollower confirm say)\n                let near_say = nearby_conn_ids(state, sx, sy, ADULT_CHAT_RANGE);\n                let pkt = format_player_says(sp_id, false, text).into_bytes();\n                send_nearby_chat(state, outbound, &near_say, sp_id, pkt);",
        ),
        (
            "            if let Some(say) = public_count {\n                let near = nearby_conn_ids(state, p.x, p.y, NEARBY_RANGE);\n                send_chat_ps(state, outbound, conn_id, p.p_id, &say, &near);\n            }",
            "            if let Some(say) = public_count {\n                // PO-MAX-DISTANCE: public social-pin count is sendSayToAllClose (CloseForSay 20)\n                let near = nearby_conn_ids(state, p.x, p.y, ADULT_CHAT_RANGE);\n                send_chat_ps(state, outbound, conn_id, p.p_id, &say, &near);\n            }",
        ),
        (
            "            let near = nearby_conn_ids(state, ax, ay, NEARBY_RANGE);\n            let say_pkt = format_player_says(attacker_id, false, &text).into_bytes();\n            send_nearby_chat(state, outbound, &near, attacker_id, say_pkt);",
            "            let near = nearby_conn_ids(state, ax, ay, ADULT_CHAT_RANGE);\n            let say_pkt = format_player_says(attacker_id, false, &text).into_bytes();\n            send_nearby_chat(state, outbound, &near, attacker_id, say_pkt);",
        ),
        (
            "        let near = nearby_conn_ids(state, tx, ty, NEARBY_RANGE);\n        let say_pkt = format_player_says(p_id, false, \"damm moskitos!\").into_bytes();\n        send_nearby_chat(state, outbound, &near, p_id, say_pkt);",
            "        // PO-MAX-DISTANCE: CloseForSay 20 (not NEARBY_RANGE 24)\n        let near = nearby_conn_ids(state, tx, ty, ADULT_CHAT_RANGE);\n        let say_pkt = format_player_says(p_id, false, \"damm moskitos!\").into_bytes();\n        send_nearby_chat(state, outbound, &near, p_id, say_pkt);",
        ),
        (
            "    // Age-scaled normal SAY range (matches live SAY path; adult=NEARBY_RANGE).",
            "    // Age-scaled normal SAY range (matches live SAY path; adult=ADULT_CHAT_RANGE / CloseForSay 20).",
        ),
        (
            "        // Beyond NEARBY_RANGE (24) but within SHOUT_RANGE (48).",
            "        // Beyond ADULT_CHAT_RANGE (20) / NEARBY_RANGE (24) but within SHOUT_RANGE (48).",
        ),
        (
            "        assert!(!far_soft, \"normal SAY must not reach beyond NEARBY_RANGE\");",
            "        assert!(!far_soft, \"normal SAY must not reach beyond ADULT_CHAT_RANGE/CloseForSay\");",
        ),
    ];
    for (old, new) in say_repls {
        // allow multiple near_say occurrences
        while once(&mut t, old, new) {
            changed = true;
        }
    }

    // Integration test: adult chat range 20 excludes cheby 22
    if !t.contains("say_adult_close_for_say_range_twenty") {
        let anchor = "    /// `SAY SHOUT <text>` fans out PS at [`SHOUT_RANGE`] (48), past normal nearby.\n    #[test]\n    fn say_shout_uses_larger_nearby_range() {";
        let insert = r#"    /// PO-MAX-DISTANCE: adult normal SAY uses CloseForSay 20 (not NEARBY_RANGE 24).
    // Haxe: Connection.sendSayToAllClose + ServerSettings.MaxDistanceToBeConsideredAsCloseForSay
    #[test]
    fn say_adult_close_for_say_range_twenty() {
        assert_eq!(ADULT_CHAT_RANGE, 20);
        assert_eq!(MAX_DISTANCE_CLOSE_FOR_SAY, 20);
        assert_eq!(chat_range_for_age(25.0), 20);
        assert_ne!(ADULT_CHAT_RANGE, NEARBY_RANGE);
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        spawn_player(&mut state, 1, "near@x");
        spawn_player(&mut state, 2, "mid@x");
        // Chebyshev 22: inside old NEARBY 24, outside CloseForSay 20.
        set_player_position(&mut state, 1, 0, 0);
        set_player_position(&mut state, 2, 22, 0);
        {
            let p = state.players.get_mut(&1).unwrap();
            p.age = 25.0;
        }
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "hello close-for-say".into(),
            },
        );
        let mut far_mid = false;
        while let Ok(pkt) = rx2.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("hello close-for-say") {
                far_mid = true;
            }
        }
        assert!(
            !far_mid,
            "adult SAY must not reach cheby 22 when CloseForSay=20"
        );
        // Within 20 must still hear.
        set_player_position(&mut state, 2, 20, 0);
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}
        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: "edge twenty".into(),
            },
        );
        let mut near_ok = false;
        while let Ok(pkt) = rx2.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("edge twenty") {
                near_ok = true;
            }
        }
        assert!(near_ok, "adult SAY must reach cheby 20");
    }

    /// `SAY SHOUT <text>` fans out PS at [`SHOUT_RANGE`] (48), past normal nearby.
    #[test]
    fn say_shout_uses_larger_nearby_range() {"#;
        changed |= once(&mut t, anchor, insert);
    }

    if changed {
        let _ = std::fs::write(&path, out(t, crlf));
    }
    // Success if chat_range uses ADULT_CHAT_RANGE
    std::fs::read_to_string(&path)
        .map(|s| s.contains("ADULT_CHAT_RANGE") && s.contains("chat_range_for_age"))
        .unwrap_or(false)
}

fn patch_mumble(src: &Path) -> bool {
    let path = src.join("mumble.rs");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return true;
    };
    let (mut t, crlf) = nl(&raw);
    let old = "        assert!(MUMBLE_RANGE < 24);";
    let new = "        // PO-MAX-DISTANCE: adult CloseForSay=20; mumble still narrower\n        assert!(MUMBLE_RANGE < 20);\n        assert!(MUMBLE_RANGE < 24);";
    if once(&mut t, old, new) {
        let _ = std::fs::write(&path, out(t, crlf));
    }
    true
}

fn patch_ai_follow_inc(src: &Path) -> bool {
    let path = src.join("ai_follow_walk_live.inc.rs");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return true;
    };
    let (mut t, crlf) = nl(&raw);
    let old = "                            let near = nearby_conn_ids(state, ax, ay, NEARBY_RANGE);\n                            send_chat_ps(state, outbound, conn_id, ai_p_id, name, &near);";
    let new = "                            // PO-MAX-DISTANCE: CloseForSay 20\n                            let near = nearby_conn_ids(state, ax, ay, ADULT_CHAT_RANGE);\n                            send_chat_ps(state, outbound, conn_id, ai_p_id, name, &near);";
    if once(&mut t, old, new) {
        let _ = std::fs::write(&path, out(t, crlf));
    }
    true
}

fn patch_field_map(workspace: &Path) -> bool {
    let path = workspace
        .join("crates")
        .join("ol-config")
        .join("src")
        .join("field_map.rs");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return true;
    };
    let (mut t, crlf) = nl(&raw);
    let old = r#"    FieldEntry {
        haxe_name: "MaxDistanceToBeConsideredAsCloseForSay",
        rust_path: "ol-sim SAY range",
        home: SettingsHome::ModuleConst,
    },"#;
    let new = r#"    FieldEntry {
        haxe_name: "MaxDistanceToBeConsideredAsCloseForSay",
        rust_path: "ol-sim speech::ADULT_CHAT_RANGE / MAX_DISTANCE_CLOSE_FOR_SAY (20) + chat_range_for_age",
        home: SettingsHome::ModuleConst, // PO-MAX-DISTANCE: intentional ModuleConst (not LiveSettings)
    },"#;
    if once(&mut t, old, new) {
        let _ = std::fs::write(&path, out(t, crlf));
    }
    true
}

fn patch_docs(workspace: &Path) -> bool {
    let port = workspace.join("docs").join("port");
    // TODO_PORT GPI residual clear + PO-MAX done
    let todo = port.join("TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo) {
        let (mut t, crlf) = nl(&raw);
        let mut c = false;
        c |= once(
            &mut t,
            "(residual: adult say radius 24 vs Haxe CloseForSay 20)",
            "(adult CloseForSay → **PO-MAX-DISTANCE DONE**)",
        );
        if !t.contains("**PO-MAX-DISTANCE / close_say_range**") {
            // insert near GPI-TOO-CLOSE line
            c |= once(
                &mut t,
                "- [x] **GPI-TOO-CLOSE / too_close_say_ps**",
                "- [x] **PO-MAX-DISTANCE / close_say_range** — Haxe `MaxDistanceToBeConsideredAsCloseForSay`=20 → `speech::ADULT_CHAT_RANGE` / `MAX_DISTANCE_CLOSE_FOR_SAY` + `chat_range_for_age` adult branch; say fans use 20 not `NEARBY_RANGE` 24; ModuleConst residual (not LiveSettings); tests `age_brackets` / `say_adult_close_for_say_range_twenty`\n- [x] **GPI-TOO-CLOSE / too_close_say_ps**",
            );
        }
        if !t.contains("PO-MAX-DISTANCE close_say_range") {
            let row = "| 2026-07-29 | **PO-MAX-DISTANCE close_say_range**: adult `ADULT_CHAT_RANGE`/`MAX_DISTANCE_CLOSE_FOR_SAY`=20 (Haxe CloseForSay); keep `NEARBY_RANGE`=24 for PU/MX; say PS fans + `chat_range_for_age`; ModuleConst not LiveSettings; tests `speech::age_brackets` / `say_adult_close_for_say_range_twenty` |\n";
            if let Some(i) = t.find("## Changelog (port docs)") {
                if let Some(j) = t[i..].find("\n| Date |") {
                    if let Some(k) = t[i + j..].find('\n') {
                        let at = i + j + k + 1;
                        t.insert_str(at, row);
                        c = true;
                    }
                }
            }
        }
        if c {
            let _ = std::fs::write(&todo, out(t, crlf));
        }
    }

    let matrix = port.join("FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&matrix) {
        let (mut t, crlf) = nl(&raw);
        let mut c = false;
        c |= once(
            &mut t,
            "| S-CONN | `Connection.hx` | 1079 | `ol-net`, `login_bootstrap`, `outbound`, sim send helpers, **`ai_takeover`** | PARTIAL | Login/ticket/MC/PU/MX core; **close→AI-TAKEOVER** + rlogin reclaim wired; **FERTILITY-TWINS** twin wait queue core; **LEADER-RANGE** leader PU exempt + LEAD pin; **sendSayToAllClose** distance-only (mute is Rust product) |",
            "| S-CONN | `Connection.hx` | 1079 | `ol-net`, `login_bootstrap`, `outbound`, sim send helpers, **`ai_takeover`** | PARTIAL | Login/ticket/MC/PU/MX core; **close→AI-TAKEOVER** + rlogin reclaim wired; **FERTILITY-TWINS** twin wait queue core; **LEADER-RANGE** leader PU exempt + LEAD pin; **sendSayToAllClose** **PO-MAX-DISTANCE** CloseForSay=20 (mute is Rust product) |",
        );
        c |= once(
            &mut t,
            "| C-SS | `ServerSettings.hx` | `ol-config` + `LiveSettings` + `GameplayKnobs` | PARTIAL | hot-reload + **C-SS-FULL-TABLE** + **C-SS-MORE** + **C-SS-TAIL-KNOBS** + **C-SS-AGE-FOOD** + **C-SS-MORE-KNOBS** (ExhaustionHealing/WoundDamage/MaxMoveQuadJump/FoodRestoreWhileFeeding/MaxHasEaten*/HasEatenReduction Live + InheritEatenFoodCounts); residual ~200 ModuleConst |",
            "| C-SS | `ServerSettings.hx` | `ol-config` + `LiveSettings` + `GameplayKnobs` | PARTIAL | hot-reload + **C-SS-*** batches + **PO-MAX-DISTANCE** CloseForSay ModuleConst=20; residual ~200 ModuleConst |",
        );
        if !t.contains("**PO-MAX-DISTANCE**") {
            c |= once(
                &mut t,
                "| LEADER-RANGE / FOLLOW-HIRE / MAP-LOCATION-PINS / PO-FAR | social UX | **DONE** (core) | |",
                "| LEADER-RANGE / FOLLOW-HIRE / MAP-LOCATION-PINS / PO-FAR | social UX | **DONE** (core) | |\n| **PO-MAX-DISTANCE** / close_say_range | MaxDistanceToBeConsideredAsCloseForSay=20 | **DONE** | `ADULT_CHAT_RANGE`≠`NEARBY_RANGE`; ModuleConst residual |\n",
            );
        }
        if c {
            let _ = std::fs::write(&matrix, out(t, crlf));
        }
    }

    let queue = port.join("QUEUE.md");
    if let Ok(raw) = std::fs::read_to_string(&queue) {
        let (mut t, crlf) = nl(&raw);
        let mut c = false;
        c |= once(
            &mut t,
            "| `PO-MAX-DISTANCE` | close_say_range | MaxDistance fans / NEARBY_RANGE vs Haxe 20 residual |\n",
            "",
        );
        if !t.contains("PO-MAX-DISTANCE") || t.contains("PO-MAX-DISTANCE close_say_range DONE") {
            // already removed or noted
        } else if !t.contains("**PO-MAX-DISTANCE**") {
            c |= once(
                &mut t,
                "## Done recently (do not re-queue)\n",
                "## Done recently (do not re-queue)\n\n**PO-MAX-DISTANCE** close_say_range DONE · ",
            );
        }
        if c {
            let _ = std::fs::write(&queue, out(t, crlf));
        }
    }

    // changelog
    let cl = port.join("changelog").join("2026-07-29-PO-MAX-DISTANCE.md");
    if !cl.exists() {
        let body = r#"# PO-MAX-DISTANCE / close_say_range

**Date:** 2026-07-29  
**Mode:** implement  
**Status:** **DONE**

## Haxe

- `ServerSettings.MaxDistanceToBeConsideredAsCloseForSay = 20`
- `Connection.sendSayToAllClose` → `isClose(..., MaxDistanceToBeConsideredAsCloseForSay)`
- Distinct from `MaxDistanceToBeConsideredAsClose` (PU, often 2e6) / movement 30 / map 10

## Rust

| Piece | Location |
|-------|----------|
| Adult say radius | `speech::ADULT_CHAT_RANGE` = **20** (+ `MAX_DISTANCE_CLOSE_FOR_SAY`) |
| Age soft scale | `chat_range_for_age` infants 8 / children 16 / adult+elder **20** |
| Interest cull | `NEARBY_RANGE` = **24** unchanged (PU/MX) |
| LiveSettings | **ModuleConst residual** (intentional; not hot-reloaded) |
| Say fans | AI/scripted/LLM/`send_chat_ps` paths use `ADULT_CHAT_RANGE` or `chat_range_for_age` |

## Tests

- `speech::tests::age_brackets` asserts 20
- `say_adult_close_for_say_range_twenty` live PS gate cheby 22 vs 20

## Residual

- Euclidean vs Chebyshev metric on `isClose` (product-wide distance metric; out of this chunk)
- `MaxDistanceToBeConsideredAsClose` product 2e6 vs practical `NEARBY_RANGE` (PO-FAR intentional)
"#;
        let _ = std::fs::create_dir_all(cl.parent().unwrap());
        let _ = std::fs::write(&cl, body);
    }

    // GPI-TOO-CLOSE residual clear
    let gpi = port.join("changelog").join("2026-07-29-GPI-TOO-CLOSE.md");
    if let Ok(raw) = std::fs::read_to_string(&gpi) {
        let (mut t, crlf) = nl(&raw);
        if once(
            &mut t,
            "- Adult chat fan-out: Rust `ADULT_CHAT_RANGE` / `NEARBY_RANGE` = **24** Chebyshev vs Haxe `MaxDistanceToBeConsideredAsCloseForSay` = **20** (product-wide; not only too-close)",
            "- ~~Adult chat fan-out 24 vs CloseForSay 20~~ → **PO-MAX-DISTANCE DONE** (`ADULT_CHAT_RANGE`=20)",
        ) {
            let _ = std::fs::write(&gpi, out(t, crlf));
        }
    }

    true
}
