//! Build-time wire for **PREFER-SHORT-WAIT** / prefer_short_busy.
//!
//! - BusyMoving → ShortCraftLiveIntent::Wait (hold tick)
//! - PreferShortCraft craft_actor → SeekOrCraft craft_if_needed
//! - ProfessionScanInput.is_moving + smart_drop_held_profession_ex
//!
//! Idempotent pure-Rust string patches + optional Python full apply.
//! Hooked from `build_craft_live_tick::patch_all_craft_live_tick`.

use std::path::{Path, PathBuf};
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

fn replace_once(hay: &mut String, old: &str, new: &str) -> bool {
    if let Some(i) = hay.find(old) {
        hay.replace_range(i..i + old.len(), new);
        true
    } else {
        false
    }
}

fn replace_all(hay: &mut String, old: &str, new: &str) -> usize {
    let mut n = 0;
    while replace_once(hay, old, new) {
        n += 1;
    }
    n
}

fn write_if_changed(path: &Path, original: &str, next: &str) -> bool {
    if original == next {
        return false;
    }
    if let Err(e) = std::fs::write(path, next) {
        eprintln!(
            "cargo:warning=PREFER-SHORT-WAIT write {}: {e}",
            path.display()
        );
        return false;
    }
    true
}

/// True when Wait live intent + BusyMoving mapping + is_moving field exist.
pub fn prefer_short_wait_wired(drop_held: &str, sci: &str, lib: &str, ps: &str) -> bool {
    sci.contains("Wait,")
        && sci.contains("smart_drop_held_profession_ex")
        && sci.contains("live_intent_is_wait")
        && drop_held.contains("ShortCraftLiveIntent::Wait")
        && drop_held.contains("craft_if_needed: craft_actor")
        && ps.contains("pub is_moving: bool")
        && (ps.contains("smart_drop_held_profession_ex")
            || ps.contains("inp.is_moving"))
        && lib.contains("live_intent_is_wait")
        && lib.contains("smart_drop_held_profession_ex")
}

pub fn stamp_path(src: &Path) -> PathBuf {
    src.join(".prefer_short_wait_patched")
}

pub fn patch_prefer_short_wait(src: &Path, workspace: &Path) -> bool {
    // Prefer full Python apply when available.
    let apply = src.join("_apply_prefer_short_wait.py");
    if apply.exists() {
        let _ = Command::new("python")
            .arg(&apply)
            .status()
            .or_else(|_| Command::new("python3").arg(&apply).status());
    }

    let drop_path = src.join("drop_held_ai.rs");
    let sci_path = src.join("short_craft_intent.rs");
    let lib_path = src.join("lib.rs");
    let ps_path = src.join("profession_scan.rs");
    let goc_path = src.join("get_or_craft.rs");
    let npc_path = workspace.join("crates/ol-server/src/npc_ai.rs");
    let sp_path = workspace.join("crates/ol-server/src/selfplay.rs");

    let _ = patch_drop_held_to_live(&drop_path);
    let _ = patch_drop_held_tests(&drop_path);
    let _ = patch_sci(&sci_path);
    let _ = patch_lib(&lib_path);
    let _ = patch_profession_scan(&ps_path);
    let _ = patch_get_or_craft(&goc_path);
    if npc_path.exists() {
        let _ = patch_npc(&npc_path);
    }
    if sp_path.exists() {
        let _ = patch_selfplay(&sp_path);
    }

    let drop_held = std::fs::read_to_string(&drop_path).unwrap_or_default();
    let sci = std::fs::read_to_string(&sci_path).unwrap_or_default();
    let lib = std::fs::read_to_string(&lib_path).unwrap_or_default();
    let ps = std::fs::read_to_string(&ps_path).unwrap_or_default();
    prefer_short_wait_wired(&drop_held, &sci, &lib, &ps)
}

fn patch_drop_held_to_live(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("ShortCraftLiveIntent::Wait")
        && raw.contains("craft_if_needed: craft_actor")
    {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);

    let old = r#"    /// Map wire-capable decisions to [`ShortCraftLiveIntent`].
    ///
    /// `PreferShortCraft` unresolved + `BusyMoving` stay staging-only
    /// (caller runs [`resolve_prefer_short_craft`] / waits). SelfClothing maps to
    /// live [`ShortCraftLiveIntent::SelfClothing`] for npc_ai Raw SELF enqueue.
    // Haxe: dropTarget → DROP; useTarget → USE; gotoObj → walk; self → SELF clothing
    pub fn to_live_intent(self) -> ShortCraftLiveIntent {
        match self {
            Self::UseAt {
                x,
                y,
                target_id,
                actor_id,
            }
            | Self::UseAsDrop {
                x,
                y,
                target_id,
                actor_id,
            } => ShortCraftLiveIntent::UseAt {
                x,
                y,
                target_id,
                actor_id,
            },
            Self::DropAt { x, y } => ShortCraftLiveIntent::DropAt { x, y },
            // Haxe: myPlayer.gotoObj(target) while dropOnStart — walk, not DROP
            Self::Goto { x, y } => ShortCraftLiveIntent::Goto { x, y },
            // Haxe: myPlayer.self(0, 0, 5) quiver store
            Self::SelfClothing { slot } => ShortCraftLiveIntent::SelfClothing { slot },
            Self::PreferShortCraft { actor, .. } => ShortCraftLiveIntent::SeekOrCraft {
                actor,
                craft_if_needed: false,
            },
            Self::BusyMoving | Self::None | Self::RefuseWound => ShortCraftLiveIntent::None,
        }
    }
}"#;

    let new = r#"    /// Map wire-capable decisions to [`ShortCraftLiveIntent`].
    ///
    /// Prefer [`resolve_prefer_short_craft`] / [`plan_drop_held_live`] first so
    /// PreferShortCraft becomes UseAt when target is in scan. Unresolved
    /// PreferShortCraft keeps `craft_actor` as SeekOrCraft craft_if_needed.
    /// BusyMoving → Wait (hold tick; Haxe isMoving return true).
    // Haxe: dropTarget → DROP; useTarget → USE; gotoObj → walk; self → SELF clothing
    // Haxe: isMoving return true (PREFER-SHORT-WAIT BusyMoving)
    pub fn to_live_intent(self) -> ShortCraftLiveIntent {
        match self {
            Self::UseAt {
                x,
                y,
                target_id,
                actor_id,
            }
            | Self::UseAsDrop {
                x,
                y,
                target_id,
                actor_id,
            } => ShortCraftLiveIntent::UseAt {
                x,
                y,
                target_id,
                actor_id,
            },
            Self::DropAt { x, y } => ShortCraftLiveIntent::DropAt { x, y },
            // Haxe: myPlayer.gotoObj(target) while dropOnStart — walk, not DROP
            Self::Goto { x, y } => ShortCraftLiveIntent::Goto { x, y },
            // Haxe: myPlayer.self(0, 0, 5) quiver store
            Self::SelfClothing { slot } => ShortCraftLiveIntent::SelfClothing { slot },
            // Haxe: shortCraft(actor, target, …, craftActor) when target not tile-resolved
            Self::PreferShortCraft {
                actor,
                craft_actor,
                ..
            } => ShortCraftLiveIntent::SeekOrCraft {
                actor,
                craft_if_needed: craft_actor,
            },
            // Haxe: if (myPlayer.isMoving()) return true — hold tick, no fallthrough
            Self::BusyMoving => ShortCraftLiveIntent::Wait,
            Self::None | Self::RefuseWound => ShortCraftLiveIntent::None,
        }
    }
}"#;

    if !replace_once(&mut t, old, new) {
        // Already partially patched?
        if t.contains("ShortCraftLiveIntent::Wait") {
            return true;
        }
        eprintln!("cargo:warning=PREFER-SHORT-WAIT drop_held to_live_intent pattern miss");
        return false;
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf))
}

fn patch_drop_held_tests(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("busy_moving_to_wait_live_intent") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let marker = "    // ── DROP-HELD-TABLE ─────────────────────────────────────────────────────";
    let tests = r#"
    // ── PREFER-SHORT-WAIT ───────────────────────────────────────────────────

    #[test]
    fn busy_moving_to_wait_live_intent() {
        // Haxe: dropOnStart && isMoving → return true (hold tick)
        assert_eq!(
            DropHeldDecision::BusyMoving.to_live_intent(),
            ShortCraftLiveIntent::Wait
        );
    }

    #[test]
    fn drop_on_start_while_moving_is_busy_wait() {
        // Oven-near held stages dropOnStart; while moving → BusyMoving → Wait
        let mut tiles = empty_grid(0, 0, 6);
        tiles.push(ScanTile::simple(HOT_ADOBE_OVEN, 0, 0)); // home/oven
        let mut inp = DropHeldInput::basic(CLAY_BOWL, 20, 0, 0, 0);
        inp.max_distance_to_home = 40.0;
        inp.is_moving = true;
        // Clay bowl is oven-near → dropClose=false → dropOnStart → BusyMoving
        let d = drop_held_object(inp, &tiles);
        assert_eq!(d, DropHeldDecision::BusyMoving, "got {d:?}");
        assert_eq!(
            plan_drop_held_live(inp, &tiles).to_live_intent(),
            ShortCraftLiveIntent::Wait
        );
        assert_eq!(
            smart_drop_held_to_live_intent(inp, &tiles),
            ShortCraftLiveIntent::Wait
        );
    }

    #[test]
    fn prefer_short_craft_uses_craft_actor_flag() {
        // Unresolved PreferShortCraft keeps craft_actor as SeekOrCraft craft_if_needed
        let d = DropHeldDecision::PreferShortCraft {
            actor: STONE_HOE,
            target: BASKET,
            max_search: 15,
            craft_actor: true,
            max_new_actor: i32::MAX,
        };
        assert_eq!(
            d.to_live_intent(),
            ShortCraftLiveIntent::SeekOrCraft {
                actor: STONE_HOE,
                craft_if_needed: true,
            }
        );
        let d2 = DropHeldDecision::PreferShortCraft {
            actor: RAW_MUTTON,
            target: HOT_ADOBE_OVEN,
            max_search: 10,
            craft_actor: false,
            max_new_actor: 4,
        };
        assert_eq!(
            d2.to_live_intent(),
            ShortCraftLiveIntent::SeekOrCraft {
                actor: RAW_MUTTON,
                craft_if_needed: false,
            }
        );
    }

    #[test]
    fn plan_resolves_prefer_short_before_live() {
        // plan_drop_held_live must not leave PreferShortCraft when target in scan
        let mut tiles = empty_grid(5, 5, 8);
        tiles.push(ScanTile::simple(HOT_COALS, 8, 5));
        let inp = DropHeldInput::basic(SKEWERED_RABBIT, 5, 5, 0, 0);
        let d = plan_drop_held_live(inp, &tiles);
        assert!(
            matches!(
                d,
                DropHeldDecision::UseAt {
                    target_id: HOT_COALS,
                    actor_id: SKEWERED_RABBIT,
                    ..
                }
            ),
            "got {d:?}"
        );
        assert!(matches!(
            d.to_live_intent(),
            ShortCraftLiveIntent::UseAt {
                target_id: HOT_COALS,
                actor_id: SKEWERED_RABBIT,
                ..
            }
        ));
    }

"#;
    if !t.contains(marker) {
        eprintln!("cargo:warning=PREFER-SHORT-WAIT tests marker miss");
        return false;
    }
    if !replace_once(&mut t, marker, &format!("{tests}{marker}")) {
        return false;
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf))
}

fn patch_sci(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    // short_craft_intent may already be fully patched by hand
    if raw.contains("Wait,")
        && raw.contains("smart_drop_held_profession_ex")
        && raw.contains("live_intent_is_wait")
    {
        return true;
    }
    false // hand-patched preferred; skip complex rebuild
}

fn patch_lib(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("live_intent_is_wait") && raw.contains("smart_drop_held_profession_ex") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let old = r#"// Haxe: profession DropHeld → smart dropHeldObject (DROP-HELD-LIVE bridge)
pub use short_craft_intent::{
    drop_held_live_intent_actionable, smart_drop_held_profession,
};"#;
    let new = r#"// Haxe: profession DropHeld → smart dropHeldObject (DROP-HELD-LIVE / PREFER-SHORT-WAIT)
pub use short_craft_intent::{
    drop_held_live_intent_actionable, live_intent_is_wait, smart_drop_held_profession,
    smart_drop_held_profession_ex,
};"#;
    if !replace_once(&mut t, old, new) {
        // try alternate already partial
        if t.contains("smart_drop_held_profession,") && !t.contains("live_intent_is_wait") {
            let _ = replace_once(
                &mut t,
                "drop_held_live_intent_actionable, smart_drop_held_profession,",
                "drop_held_live_intent_actionable, live_intent_is_wait, smart_drop_held_profession,\n    smart_drop_held_profession_ex,",
            );
        } else if !t.contains("live_intent_is_wait") {
            eprintln!("cargo:warning=PREFER-SHORT-WAIT lib reexport pattern miss");
            return false;
        }
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf))
}

fn patch_profession_scan(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("pub is_moving: bool") {
        let old = r#"    /// Baker assigned-job dispatch (maxPeople 100 vs 1).
    pub is_assigned_job: bool,
}

impl ProfessionScanInput {
    pub fn basic(px: i32, py: i32, held_id: i32) -> Self {
        Self {
            player_x: px,
            player_y: py,
            home_x: px,
            home_y: py,
            held_id,
            held_uses: 1,
            held_contained: 0,
            held_contains_clay: held_id == crate::pottery_profession::CLAY,
            food_store: 20.0,
            transition_hungry_cost: 0.0,
            has_carrot_seeds: true,
            has_bean_seeds: true,
            is_hungry: false,
            basic_farmer_weight: 1.0,
            hardened_row_biome: None,
            target_reachable: true,
            peer_count: 0.0,
            was_idle: 0.0,
            age: 20.0,
            profession_is_sticky: true,
            is_assigned_job: true,
        }
    }
}"#;
        let new = r#"    /// Baker assigned-job dispatch (maxPeople 100 vs 1).
    pub is_assigned_job: bool,
    /// Haxe `myPlayer.isMoving()` — dropHeld BusyMoving → Wait hold tick.
    // Haxe: dropHeldObject dropOnStart isMoving (PREFER-SHORT-WAIT)
    pub is_moving: bool,
}

impl ProfessionScanInput {
    pub fn basic(px: i32, py: i32, held_id: i32) -> Self {
        Self {
            player_x: px,
            player_y: py,
            home_x: px,
            home_y: py,
            held_id,
            held_uses: 1,
            held_contained: 0,
            held_contains_clay: held_id == crate::pottery_profession::CLAY,
            food_store: 20.0,
            transition_hungry_cost: 0.0,
            has_carrot_seeds: true,
            has_bean_seeds: true,
            is_hungry: false,
            basic_farmer_weight: 1.0,
            hardened_row_biome: None,
            target_reachable: true,
            peer_count: 0.0,
            was_idle: 0.0,
            age: 20.0,
            profession_is_sticky: true,
            is_assigned_job: true,
            is_moving: false,
        }
    }
}"#;
        if replace_once(&mut t, old, new) {
            changed = true;
        }
    }

    // smart_drop → _ex with is_moving (farm/smith/baker pattern)
    let old_sd = r#"                let intent = super::smart_drop_held_profession(
                    tiles,
                    inp.held_id,
                    inp.held_uses,
                    inp.player_x,
                    inp.player_y,
                    inp.home_x,
                    inp.home_y,
                    inp.food_store,
                    false,
                    40.0,
                    false,
                );"#;
    let new_sd = r#"                let intent = super::smart_drop_held_profession_ex(
                    tiles,
                    inp.held_id,
                    inp.held_uses,
                    inp.player_x,
                    inp.player_y,
                    inp.home_x,
                    inp.home_y,
                    inp.food_store,
                    false,
                    40.0,
                    false,
                    inp.is_moving,
                );"#;
    if replace_all(&mut t, old_sd, new_sd) > 0 {
        changed = true;
    }

    let old_pot = r#"            let intent = super::smart_drop_held_profession(
                tiles,
                inp.held_id,
                inp.held_uses,
                inp.player_x,
                inp.player_y,
                inp.home_x,
                inp.home_y,
                inp.food_store,
                allow_piles,
                max_dist,
                held_contains_clay,
            );"#;
    let new_pot = r#"            let intent = super::smart_drop_held_profession_ex(
                tiles,
                inp.held_id,
                inp.held_uses,
                inp.player_x,
                inp.player_y,
                inp.home_x,
                inp.home_y,
                inp.food_store,
                allow_piles,
                max_dist,
                held_contains_clay,
                inp.is_moving,
            );"#;
    if replace_once(&mut t, old_pot, new_pot) {
        changed = true;
    }

    // base_inp is_moving from player
    if !t.contains("p.moving || p.move_path.is_some()") {
        let old_base = r#"    let (held_contained, held_contains_clay) = state
        .players
        .get(&conn_id)
        .map(|p| {
            (
                held_contained_from_player(p),
                held_contains_clay_from_player(p),
            )
        })
        .unwrap_or((0, held_id == crate::pottery_profession::CLAY));

    let base_inp = ProfessionScanInput {
        player_x: px,
        player_y: py,
        home_x,
        home_y,
        held_id,
        held_uses,
        held_contained,
        held_contains_clay,
        food_store,
        transition_hungry_cost: 0.0, // residual: content-pair hungry cost
        has_carrot_seeds,
        has_bean_seeds,
        is_hungry: food_store < 5.0,
        basic_farmer_weight: 1.0,
        hardened_row_biome: None,
        // PATH-REACH: tiles already filtered via Player.ai_path_reach + blocked_by_ai.
        target_reachable: true,
        peer_count,
        was_idle,
        age,
        profession_is_sticky,
        is_assigned_job: sticky.has_assigned_job(),
    };"#;
        let new_base = r#"    let (held_contained, held_contains_clay, is_moving) = state
        .players
        .get(&conn_id)
        .map(|p| {
            (
                held_contained_from_player(p),
                held_contains_clay_from_player(p),
                p.moving || p.move_path.is_some(),
            )
        })
        .unwrap_or((0, held_id == crate::pottery_profession::CLAY, false));

    let base_inp = ProfessionScanInput {
        player_x: px,
        player_y: py,
        home_x,
        home_y,
        held_id,
        held_uses,
        held_contained,
        held_contains_clay,
        food_store,
        transition_hungry_cost: 0.0, // residual: content-pair hungry cost
        has_carrot_seeds,
        has_bean_seeds,
        is_hungry: food_store < 5.0,
        basic_farmer_weight: 1.0,
        hardened_row_biome: None,
        // PATH-REACH: tiles already filtered via Player.ai_path_reach + blocked_by_ai.
        target_reachable: true,
        peer_count,
        was_idle,
        age,
        profession_is_sticky,
        is_assigned_job: sticky.has_assigned_job(),
        // PREFER-SHORT-WAIT: dropHeld isMoving → Wait
        is_moving,
    };"#;
        if replace_once(&mut t, old_base, new_base) {
            changed = true;
        }
    }

    // apply_profession_scan_tick inp
    if t.contains("is_assigned_job,\n    };")
        && t.contains("// Residual: pair hungry cost needs transition table lookup")
        && !t.contains("// PREFER-SHORT-WAIT: dropHeld isMoving → Wait\n        is_moving,\n    };")
    {
        let old_inp2 = r#"    let inp = ProfessionScanInput {
        player_x: px,
        player_y: py,
        home_x,
        home_y,
        held_id,
        held_uses,
        held_contained,
        held_contains_clay,
        food_store,
        // Residual: pair hungry cost needs transition table lookup (CRAFT-LIVE-IO).
        transition_hungry_cost: 0.0,
        has_carrot_seeds,
        has_bean_seeds,
        is_hungry: food_store < 5.0,
        basic_farmer_weight: 1.0,
        hardened_row_biome: None,
        // PATH-REACH: tiles already filtered via Player.ai_path_reach + blocked_by_ai.
        target_reachable: true,
        peer_count,
        was_idle,
        age,
        profession_is_sticky,
        is_assigned_job,
    };"#;
        let new_inp2 = r#"    let is_moving = state
        .players
        .get(&conn_id)
        .map(|p| p.moving || p.move_path.is_some())
        .unwrap_or(false);
    let inp = ProfessionScanInput {
        player_x: px,
        player_y: py,
        home_x,
        home_y,
        held_id,
        held_uses,
        held_contained,
        held_contains_clay,
        food_store,
        // Residual: pair hungry cost needs transition table lookup (CRAFT-LIVE-IO).
        transition_hungry_cost: 0.0,
        has_carrot_seeds,
        has_bean_seeds,
        is_hungry: food_store < 5.0,
        basic_farmer_weight: 1.0,
        hardened_row_biome: None,
        // PATH-REACH: tiles already filtered via Player.ai_path_reach + blocked_by_ai.
        target_reachable: true,
        peer_count,
        was_idle,
        age,
        profession_is_sticky,
        is_assigned_job,
        // PREFER-SHORT-WAIT: dropHeld isMoving → Wait
        is_moving,
    };"#;
        if replace_once(&mut t, old_inp2, new_inp2) {
            changed = true;
        }
    }

    // ladder Wait terminal
    if !t.contains("ShortCraftLiveIntent::Wait") {
        let old_l = r#"        if live_intent_is_wire(r.intent) {
            return r;
        }
        // Keep first staging intent (SeekOrCraft / CraftItem / Defer*) if no wire yet.
        if matches!(staging.intent, ShortCraftLiveIntent::None) {
            staging = r;
        }"#;
        let new_l = r#"        if live_intent_is_wire(r.intent) {
            return r;
        }
        // Haxe: dropHeld isMoving return true — hold tick, do not fall through (PREFER-SHORT-WAIT)
        if matches!(r.intent, ShortCraftLiveIntent::Wait) {
            return r;
        }
        // Keep first staging intent (SeekOrCraft / CraftItem / Defer*) if no wire yet.
        if matches!(staging.intent, ShortCraftLiveIntent::None) {
            staging = r;
        }"#;
        if replace_once(&mut t, old_l, new_l) {
            changed = true;
        }
    }

    // Any ProfessionScanInput { ... is_assigned_job: ... } missing is_moving
    // Fix compile breaks: add is_moving: false to remaining struct literals missing it.
    // Heuristic: after is_assigned_job: X, add is_moving if next is };
    // Handled by field + basic() + two main constructors.

    if changed {
        write_if_changed(path, &raw, &restore_nl(&t, crlf))
    } else {
        t.contains("pub is_moving: bool")
    }
}

fn patch_get_or_craft(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("GetOrCraftResult::BusyMoving => ShortCraftLiveIntent::Wait") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let old = "        GetOrCraftResult::BusyMoving | GetOrCraftResult::None => ShortCraftLiveIntent::None,";
    let new = r#"        // Haxe: GetOrCraftItem isMoving return true → hold tick (PREFER-SHORT-WAIT)
        GetOrCraftResult::BusyMoving => ShortCraftLiveIntent::Wait,
        GetOrCraftResult::None => ShortCraftLiveIntent::None,"#;
    if !replace_once(&mut t, old, new) {
        return false;
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf))
}

fn patch_npc(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("is_moving: p.moving") {
        let old = r#"                        age: p.age,
                        profession_is_sticky: sticky.has_sticky_profession(),
                        is_assigned_job: sticky.has_assigned_job(),
                    };"#;
        let new = r#"                        age: p.age,
                        profession_is_sticky: sticky.has_sticky_profession(),
                        is_assigned_job: sticky.has_assigned_job(),
                        // PREFER-SHORT-WAIT (npc usually skips when p.moving; keep field)
                        is_moving: p.moving,
                    };"#;
        if replace_once(&mut t, old, new) {
            changed = true;
        }
    }

    if !t.contains("prof_wait_busy_moving") {
        // Insert Wait arm before first profession `_ => {}` after SelfClothing
        if let Some(idx) = t.find("ShortCraftLiveIntent::SelfClothing { slot } => {") {
            if let Some(rel) = t[idx..].find("\n                            _ => {}") {
                let abs = idx + rel + 1;
                let arm = r#"                            ShortCraftLiveIntent::Wait => {
                                // Haxe: isMoving / dropHeld return true — hold tick
                                kind = NpcActivityKind::Craft;
                                detail = format!("prof_wait_busy_moving rung={}", rung.as_label());
                                game_ms = 200;
                                acted = true;
                            }
"#;
                t.insert_str(abs, arm);
                changed = true;
            }
        }
    }

    if changed {
        write_if_changed(path, &raw, &restore_nl(&t, crlf))
    } else {
        t.contains("is_moving: p.moving") || t.contains("prof_wait_busy_moving")
    }
}

fn patch_selfplay(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("SMART-DROP-WAIT") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let old = r#"                        ShortCraftLiveIntent::Goto { x: tx, y: ty } => {
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
                        _ => {"#;
    let new = r#"                        ShortCraftLiveIntent::Goto { x: tx, y: ty } => {
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
                        _ => {"#;
    if !replace_once(&mut t, old, new) {
        return false;
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf))
}
