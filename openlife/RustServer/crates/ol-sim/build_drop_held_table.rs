//! Build-time wire for **DROP-HELD-TABLE** / table_prefer.
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

fn write_if_changed(path: &Path, original: &str, next: &str) -> bool {
    if original == next {
        return false;
    }
    if let Err(e) = std::fs::write(path, next) {
        eprintln!(
            "cargo:warning=DROP-HELD-TABLE write {}: {e}",
            path.display()
        );
        return false;
    }
    true
}

/// True when table/small-food prefer + quiver snapshot helpers exist.
pub fn drop_held_table_wired(drop_held: &str, player: &str, lib: &str) -> bool {
    let table = drop_held.contains("drop_held_table_helpers.inc.rs")
        || drop_held.contains("fn should_drop_on_table(");
    let prefer = drop_held.contains("best_empty_or_container_drop")
        && drop_held.contains("allows_drop_in_container(held)");
    let quiver = drop_held.contains("from_clothing_snapshot")
        && (drop_held.contains("drop_held_table_quiver.inc.rs")
            || drop_held.contains("fn quiver_from_clothing_snapshot("));
    table
        && prefer
        && quiver
        && player.contains("pub clothing: [i32; 6]")
        && player.contains("clothing_parent_ids")
        && lib.contains("should_drop_on_table")
        && lib.contains("quiver_from_clothing_snapshot")
}

pub fn stamp_path(src: &Path) -> PathBuf {
    src.join(".drop_held_table_patched")
}

pub fn patch_drop_held_table(src: &Path, workspace: &Path) -> bool {
    // Prefer full Python apply (includes expanded tests) when available.
    let apply = src.join("_apply_drop_held_table.py");
    if apply.exists() {
        let _ = Command::new("python")
            .arg(&apply)
            .status()
            .or_else(|_| Command::new("python3").arg(&apply).status());
    }

    let drop_path = src.join("drop_held_ai.rs");
    let player_path = src.join("player.rs");
    let lib_path = src.join("lib.rs");
    let npc_path = workspace.join("crates/ol-server/src/npc_ai.rs");

    let _ = patch_drop_held_ai(&drop_path);
    let _ = patch_player(&player_path);
    let _ = patch_lib(&lib_path);
    if npc_path.exists() {
        let _ = patch_npc_ai(&npc_path);
    }

    let drop_held = std::fs::read_to_string(&drop_path).unwrap_or_default();
    let player = std::fs::read_to_string(&player_path).unwrap_or_default();
    let lib = std::fs::read_to_string(&lib_path).unwrap_or_default();
    drop_held_table_wired(&drop_held, &player, &lib)
}

fn patch_drop_held_ai(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("allows_drop_in_container(held)")
        && raw.contains("from_clothing_snapshot")
        && (raw.contains("drop_held_table_helpers.inc.rs")
            || raw.contains("fn should_drop_on_table("))
    {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if replace_once(
        &mut t,
        "use crate::baker_profession::{\n    max_dough_in_bowl, should_drop_near_oven, BOWL_OF_DOUGH, CLAY_PLATE, DROP_NEAR_OVEN_IDS,\n    RAW_MUTTON,\n};",
        "use crate::baker_profession::{\n    max_dough_in_bowl, should_drop_near_oven, BOWL_OF_DOUGH, CLAY_PLATE, COOKED_MUTTON,\n    COOKED_PIES, DROP_NEAR_OVEN_IDS, RAW_MUTTON,\n};",
    ) {
        changed = true;
    }

    if !t.contains("drop_held_table_helpers.inc.rs") {
        if replace_once(
            &mut t,
            "// ── Quiver ──────────────────────────────────────────────────────────────────",
            "// DROP-HELD-TABLE helpers (ShouldDropOnTable / isSmallFoodToStore / factors)\ninclude!(\"drop_held_table_helpers.inc.rs\");\n\n// ── Quiver ──────────────────────────────────────────────────────────────────",
        ) {
            changed = true;
        }
    }

    if !t.contains("from_clothing_snapshot") {
        if replace_once(
            &mut t,
            "    /// Recompute `can_add` after manually tweaking uses/flags.\n    pub fn refresh_can_add(&mut self) {\n        let has_quiver = self.empty_quiver\n            || self.arrow_quiver\n            || self.empty_quiver_with_bow\n            || self.quiver_with_bow;\n        self.can_add = has_quiver && can_add_to_quiver(self.quiver_uses, self.quiver_num_uses);\n    }\n}\n\n/// Haxe `ObjectHelper.canAddToQuiver`.",
            "    /// Recompute `can_add` after manually tweaking uses/flags.\n    pub fn refresh_can_add(&mut self) {\n        let has_quiver = self.empty_quiver\n            || self.arrow_quiver\n            || self.empty_quiver_with_bow\n            || self.quiver_with_bow;\n        self.can_add = has_quiver && can_add_to_quiver(self.quiver_uses, self.quiver_num_uses);\n    }\n\n    /// Build from Haxe `clothingObjects` parent ids + multi-use (PlayerSnapshot clothing).\n    // Haxe: getClothingById + canAddToQuiver uses (DROP-HELD-TABLE quiver snapshot)\n    pub fn from_clothing_snapshot(ids: &[i32], uses: &[i32]) -> Self {\n        let mut quiver_uses = 0i32;\n        for (i, &id) in ids.iter().enumerate() {\n            if matches!(\n                id,\n                EMPTY_ARROW_QUIVER_ID\n                    | ARROW_QUIVER_ID\n                    | EMPTY_ARROW_QUIVER_WITH_BOW\n                    | ARROW_QUIVER_WITH_BOW\n            ) {\n                quiver_uses = uses.get(i).copied().unwrap_or(0).max(0);\n                break;\n            }\n        }\n        Self::from_ids_with_uses(ids, quiver_uses, 0)\n    }\n}\n\ninclude!(\"drop_held_table_quiver.inc.rs\");\n\n/// Haxe `ObjectHelper.canAddToQuiver`.",
        ) {
            changed = true;
        }
    }

    if !t.contains("fn best_empty_or_container_drop") {
        let anchor = "    best.map(|(_, t)| t)\n}\n\n/// Prefer shortCraft if target id exists in scan within `max_search`.";
        let insert = r#"    best.map(|(_, t)| t)
}

/// Best free container for table/small-food held using Haxe prefer factors.
// Haxe: ShouldDropOnTable / isSmallFoodToStore container scoring ~232–272
pub fn closest_preferred_container(
    tiles: &[ScanTile],
    held_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    min_d: i32,
) -> Option<ScanTile> {
    if !allows_drop_in_container(held_id) {
        return None;
    }
    let mut best: Option<(f32, i32, ScanTile)> = None;
    for t in tiles {
        let Some(factor) = container_prefer_factor(held_id, t) else {
            continue;
        };
        let cheb = scan_chebyshev(from_x, from_y, t.x, t.y);
        if cheb < min_d || cheb > max_r {
            continue;
        }
        let score = adjust_container_drop_score(quad_distance_xy(from_x, from_y, t.x, t.y), factor);
        match best {
            None => best = Some((score, cheb, *t)),
            Some((bs, bd, bo)) => {
                if score < bs
                    || (score == bs
                        && (cheb < bd
                            || (cheb == bd && (t.y < bo.y || (t.y == bo.y && t.x < bo.x)))))
                {
                    best = Some((score, cheb, *t));
                }
            }
        }
    }
    best.map(|(_, _, t)| t)
}

/// Joint empty-tile + preferred-container pick (Haxe empty search with factor prefer).
// Haxe: GetClosestObjectToPositionHelper objIdToSearch==0 + container factors
fn best_empty_or_container_drop(
    tiles: &[ScanTile],
    held_id: i32,
    from_x: i32,
    from_y: i32,
    max_r: i32,
    opts: ClosestEmptyOpts,
) -> Option<(ScanTile, bool)> {
    let min_d = opts.min_distance.max(0);
    let allow_cont = allows_drop_in_container(held_id);
    let mut best: Option<(f32, i32, ScanTile, bool)> = None;

    if allow_cont {
        for t in tiles {
            let Some(factor) = container_prefer_factor(held_id, t) else {
                continue;
            };
            let cheb = scan_chebyshev(from_x, from_y, t.x, t.y);
            if cheb < min_d || cheb > max_r {
                continue;
            }
            let score =
                adjust_container_drop_score(quad_distance_xy(from_x, from_y, t.x, t.y), factor);
            match best {
                None => best = Some((score, cheb, *t, true)),
                Some((bs, bd, bo, _)) => {
                    if score < bs
                        || (score == bs
                            && (cheb < bd
                                || (cheb == bd && (t.y < bo.y || (t.y == bo.y && t.x < bo.x)))))
                    {
                        best = Some((score, cheb, *t, true));
                    }
                }
            }
        }
    }

    if let Some((ex, ey)) = closest_empty_tile_ex(tiles, from_x, from_y, max_r, opts) {
        let cheb = scan_chebyshev(from_x, from_y, ex, ey);
        let score = adjust_container_drop_score(quad_distance_xy(from_x, from_y, ex, ey), 1.0);
        let empty = ScanTile::empty(ex, ey, 0, 0);
        match best {
            None => best = Some((score, cheb, empty, false)),
            Some((bs, bd, bo, _)) => {
                if score < bs
                    || (score == bs
                        && (cheb < bd
                            || (cheb == bd && (ey < bo.y || (ey == bo.y && ex < bo.x)))))
                {
                    best = Some((score, cheb, empty, false));
                }
            }
        }
    }

    best.map(|(_, _, t, is_c)| (t, is_c))
}

/// Prefer shortCraft if target id exists in scan within `max_search`."#;
        if replace_once(&mut t, anchor, insert) {
            changed = true;
        }
    }

    let old_ring = r#"        if new_drop.is_none() {
            let opts = ClosestEmptyOpts {
                held_id: held,
                home_x: inp.anchors.home_x,
                home_y: inp.anchors.home_y,
                min_distance: min_d_filter,
                respect_home_clearance: min_distance >= 0,
                respect_not_floored: true,
            };
            if let Some((ex, ey)) =
                closest_empty_tile_ex(tiles, target_x, target_y, search_distance, opts)
            {
                new_drop = Some(ScanTile::empty(ex, ey, 0, 0));
                drop_in_container = false;
            }
        }

        // Haxe: empty search may yield container (numSlots>0) → useIsDropInContainer
        if new_drop.is_none() {
            if let Some(c) =
                closest_free_container(tiles, target_x, target_y, search_distance, min_d_filter)
            {
                new_drop = Some(c);
                drop_in_container = true;
            }
        }"#;
    let new_ring = r#"        // Haxe: empty search + table/small-food container prefer (DROP-HELD-TABLE)
        // Only ShouldDropOnTable / isSmallFoodToStore may USE-drop into free containers.
        if new_drop.is_none() {
            let opts = ClosestEmptyOpts {
                held_id: held,
                home_x: inp.anchors.home_x,
                home_y: inp.anchors.home_y,
                min_distance: min_d_filter,
                respect_home_clearance: min_distance >= 0,
                respect_not_floored: true,
            };
            if allows_drop_in_container(held) {
                if let Some((tile, is_cont)) = best_empty_or_container_drop(
                    tiles,
                    held,
                    target_x,
                    target_y,
                    search_distance,
                    opts,
                ) {
                    new_drop = Some(tile);
                    drop_in_container = is_cont;
                }
            } else if let Some((ex, ey)) =
                closest_empty_tile_ex(tiles, target_x, target_y, search_distance, opts)
            {
                new_drop = Some(ScanTile::empty(ex, ey, 0, 0));
                drop_in_container = false;
            }
            // Non-table/small-food: never free-container fallback (Haxe L195 gate).
        }"#;
    if replace_once(&mut t, old_ring, new_ring) {
        changed = true;
    }

    let old_test = r#"    #[test]
    fn container_empty_search_yields_use_as_drop() {
        // Only free-slot basket near player — no empty ground tiles.
        let basket = ScanTile::simple(BASKET, 2, 0)
            .with_num_slots(4)
            .with_contained_count(0);
        let tiles = vec![basket];
        let mut inp = DropHeldInput::basic(STONE, 0, 0, 0, 0);
        inp.max_distance_to_home = 1.0; // drop close to player
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(
                d,
                DropHeldDecision::UseAsDrop {
                    x: 2,
                    y: 0,
                    target_id: BASKET,
                    actor_id: STONE,
                }
            ),
            "got {d:?}"
        );
    }"#;
    let new_test = r#"    #[test]
    fn table_food_container_empty_search_yields_use_as_drop() {
        // Haxe: only ShouldDropOnTable / isSmallFoodToStore may drop into free containers.
        let basket = ScanTile::simple(BASKET, 2, 0)
            .with_num_slots(4)
            .with_contained_count(0);
        let tiles = vec![basket];
        let mut inp = DropHeldInput::basic(COOKED_MUTTON, 0, 0, 0, 0);
        inp.max_distance_to_home = 1.0;
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(
                d,
                DropHeldDecision::UseAsDrop {
                    x: 2,
                    y: 0,
                    target_id: BASKET,
                    actor_id: COOKED_MUTTON,
                }
            ),
            "got {d:?}"
        );
    }

    #[test]
    fn non_table_food_skips_free_container() {
        let basket = ScanTile::simple(BASKET, 2, 0)
            .with_num_slots(4)
            .with_contained_count(0);
        let tiles = vec![basket];
        let mut inp = DropHeldInput::basic(STONE, 0, 0, 0, 0);
        inp.max_distance_to_home = 1.0;
        let d = drop_held_object(inp, &tiles);
        assert!(
            matches!(d, DropHeldDecision::DropAt { x: 0, y: 0 }),
            "stone should fall back to feet, got {d:?}"
        );
    }"#;
    if replace_once(&mut t, old_test, new_test) {
        changed = true;
    }

    if changed {
        write_if_changed(path, &raw, &restore_nl(&t, crlf));
    }

    let after = std::fs::read_to_string(path).unwrap_or_default();
    after.contains("allows_drop_in_container(held)") && after.contains("from_clothing_snapshot")
}

fn patch_player(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("pub clothing: [i32; 6]") && raw.contains("clothing_parent_ids") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    let old_fn = r#"    /// Snapshot for web / self-play UI.
    pub fn snapshot(&self) -> PlayerSnapshot {
        PlayerSnapshot {
            conn_id: self.conn_id,
            p_id: self.p_id,
            x: self.x,
            y: self.y,
            held_id: self.held_id,
            held_uses: self.held_uses,
            food: self.food,
            food_max: self.food_max,
            age: self.age,
            email: self.email.clone(),
            deleted: self.deleted,
            connected: self.connected,
            ai_controlled: self.ai_controlled,
            moving: self.moving || self.move_path.is_some(),
            done_moving_seq: self.done_moving_seq,
            heat: self.heat,
            held_by: self.held_by,
        }
    }
}"#;
    let new_fn = r#"    /// Flat clothing parent ids for all 6 Haxe clothingObjects slots.
    // Haxe: clothingObjects[i].parentId (DROP-HELD-TABLE quiver snapshot)
    pub fn clothing_parent_ids(&self) -> [i32; 6] {
        let mut ids = [0i32; 6];
        for i in 0..CLOTHING_SLOT_COUNT {
            ids[i] = self.clothing_helpers[i]
                .as_ref()
                .map(|h| h.id)
                .filter(|&id| id > 0)
                .unwrap_or(0);
        }
        if ids[0] == 0 {
            ids[0] = self.hat.max(0);
        }
        if ids[1] == 0 {
            ids[1] = self.chest.max(0);
        }
        if ids[2] == 0 {
            ids[2] = self.shoes.max(0);
        }
        ids
    }

    /// Multi-use remaining per clothing slot (quiver capacity).
    // Haxe: clothingObjects[i].numberOfUses
    pub fn clothing_uses_remaining(&self) -> [i32; 6] {
        let mut uses = [0i32; 6];
        for i in 0..CLOTHING_SLOT_COUNT {
            uses[i] = self.clothing_helpers[i]
                .as_ref()
                .map(|h| h.uses_remaining.max(0))
                .unwrap_or(0);
        }
        uses
    }

    /// Snapshot for web / self-play UI.
    pub fn snapshot(&self) -> PlayerSnapshot {
        PlayerSnapshot {
            conn_id: self.conn_id,
            p_id: self.p_id,
            x: self.x,
            y: self.y,
            held_id: self.held_id,
            held_uses: self.held_uses,
            food: self.food,
            food_max: self.food_max,
            age: self.age,
            email: self.email.clone(),
            deleted: self.deleted,
            connected: self.connected,
            ai_controlled: self.ai_controlled,
            moving: self.moving || self.move_path.is_some(),
            done_moving_seq: self.done_moving_seq,
            heat: self.heat,
            held_by: self.held_by,
            clothing: self.clothing_parent_ids(),
            clothing_uses: self.clothing_uses_remaining(),
        }
    }
}"#;
    if replace_once(&mut t, old_fn, new_fn) {
        changed = true;
    }

    let old_s = r#"    /// Mother `p_id` when held as baby (0 = none); AI mother/follow sensors.
    #[serde(default)]
    pub held_by: i32,
}"#;
    let new_s = r#"    /// Mother `p_id` when held as baby (0 = none); AI mother/follow sensors.
    #[serde(default)]
    pub held_by: i32,
    /// Haxe `clothingObjects` parent ids (6 slots) for quiver / clothing AI.
    // Haxe: clothingObjects (DROP-HELD-TABLE)
    #[serde(default)]
    pub clothing: [i32; 6],
    /// Haxe clothing `numberOfUses` per slot (quiver multi-use capacity).
    #[serde(default)]
    pub clothing_uses: [i32; 6],
}"#;
    if replace_once(&mut t, old_s, new_s) {
        changed = true;
    }

    if changed {
        write_if_changed(path, &raw, &restore_nl(&t, crlf));
    }
    std::fs::read_to_string(path)
        .map(|s| s.contains("pub clothing: [i32; 6]"))
        .unwrap_or(false)
}

fn patch_lib(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("should_drop_on_table") && raw.contains("quiver_from_clothing_snapshot") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let old = r#"pub use short_craft_intent::drop_held_ai::{
    consider_drop_held_decision, consider_drop_held_decision_ex, consider_drop_held_object,
    count_bread_family_near, count_near, count_near_with_piles, drop_held_input_from_sensors,
    drop_held_object, fill_anchors_from_scan, force_drop_at_feet, has_knife_near_scan,
    must_use_as_drop, pile_blocked, plan_drop_held_live, resolve_prefer_short_craft,
    self_clothing_raw_payload, should_drop_near_fire, should_drop_near_forge,
    should_drop_near_oven_held, should_drop_near_well, smart_drop_held_from_sensors,
    smart_drop_held_to_live_intent, store_in_quiver, use_up_dough, DropHeldAnchors,
    DropHeldDecision, DropHeldInput, DropHeldSensorExtras, QuiverClothing, UseUpDoughInput,
    // BASKET_OF_BONES already pub-used from horse_mount; CLAY from pottery_profession.
    ARROW, ARROW_QUIVER_WITH_BOW, BANANA_PEEL, BASKET_OF_SOIL, CLAY_BOWL, CLAY_PLATE_ID,
    DONT_USE_DROP_FOR_ITEMS, DONT_USE_PILE_IDS, DROP_CLOSE_ENOUGH_QUAD, DROP_HELD_MAX_SEARCH,
    DROP_NEAR_FIRE_IDS, DROP_NEAR_WELL_IDS, EMPTY_ARROW_QUIVER_WITH_BOW, HOT_ADOBE_OVEN, HOT_COALS,
    QUIVER_CLOTHING_SLOT, SHARP_STONE, YEW_BOW,
};"#;
    let new = r#"pub use short_craft_intent::drop_held_ai::{
    adjust_container_drop_score, allows_drop_in_container, clothing_ids_snapshot,
    closest_preferred_container, consider_drop_held_decision, consider_drop_held_decision_ex,
    consider_drop_held_object, container_prefer_factor, count_bread_family_near, count_near,
    count_near_with_piles, drop_held_input_from_sensors, drop_held_object, fill_anchors_from_scan,
    force_drop_at_feet, has_knife_near_scan, is_baked_pie, is_small_food_to_store,
    must_use_as_drop, pile_blocked, plan_drop_held_live, quiver_from_clothing_ids,
    quiver_from_clothing_snapshot, resolve_prefer_short_craft, self_clothing_raw_payload,
    should_drop_near_fire, should_drop_near_forge, should_drop_near_oven_held,
    should_drop_near_well, should_drop_on_table, smart_drop_held_from_sensors,
    smart_drop_held_to_live_intent, store_in_quiver, use_up_dough, DropHeldAnchors,
    DropHeldDecision, DropHeldInput, DropHeldSensorExtras, QuiverClothing, UseUpDoughInput,
    // BASKET_OF_BONES already pub-used from horse_mount; CLAY from pottery_profession.
    ARROW, ARROW_QUIVER_WITH_BOW, BAKED_PIE_IDS, BANANA_PEEL, BASKET_OF_SOIL, CLAY_BOWL,
    CLAY_PLATE_ID, DONT_USE_DROP_FOR_ITEMS, DONT_USE_PILE_IDS, DROP_CLOSE_ENOUGH_QUAD,
    DROP_HELD_MAX_SEARCH, DROP_NEAR_FIRE_IDS, DROP_NEAR_WELL_IDS, DROP_ON_TABLE_IDS,
    EMPTY_ARROW_QUIVER_WITH_BOW, HOT_ADOBE_OVEN, HOT_COALS, OMELETTE, QUIVER_CLOTHING_SLOT,
    SHARP_STONE, SMALL_COOKED_FOOD_IDS, TABLE, WOODEN_SLOT_BOX, YEW_BOW,
};"#;
    if !replace_once(&mut t, old, new) {
        return false;
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf));
    true
}

fn patch_npc_ai(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("quiver_from_clothing_snapshot") && raw.contains("drop_extras.quiver") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("quiver_from_clothing_snapshot") {
        if replace_once(
            &mut t,
            "scan_world_radius, self_clothing_raw_payload, smart_drop_held_from_sensors,",
            "quiver_from_clothing_snapshot, scan_world_radius, self_clothing_raw_payload,\n    smart_drop_held_from_sensors,",
        ) {
            changed = true;
        } else if replace_once(
            &mut t,
            "smart_drop_held_from_sensors,",
            "quiver_from_clothing_snapshot, smart_drop_held_from_sensors,",
        ) {
            changed = true;
        }
    }

    let old_call = r#"                let intent = smart_drop_held_from_sensors(
                    p.held_id,
                    p.held_uses.max(1),
                    p.x,
                    p.y,
                    p.x,
                    p.y,
                    p.food,
                    false,
                    false,
                    1.0, // drop close to player
                    &tiles,
                    DropHeldSensorExtras::default(),
                );"#;
    let new_call = r#"                // Haxe: storeInQuiver clothingObjects scan (DROP-HELD-TABLE snapshot)
                let mut drop_extras = DropHeldSensorExtras::default();
                drop_extras.quiver =
                    quiver_from_clothing_snapshot(&p.clothing, &p.clothing_uses);
                let intent = smart_drop_held_from_sensors(
                    p.held_id,
                    p.held_uses.max(1),
                    p.x,
                    p.y,
                    p.x,
                    p.y,
                    p.food,
                    false,
                    false,
                    1.0, // drop close to player
                    &tiles,
                    drop_extras,
                );"#;
    if replace_once(&mut t, old_call, new_call) {
        changed = true;
    }

    if changed {
        write_if_changed(path, &raw, &restore_nl(&t, crlf));
    }
    std::fs::read_to_string(path)
        .map(|s| s.contains("quiver_from_clothing_snapshot"))
        .unwrap_or(false)
}
