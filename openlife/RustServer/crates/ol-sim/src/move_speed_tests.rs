//! Unit tests for move-speed pure helpers (S-MOVE / S-MOVE-POLISH / MOVE-NEST-SPEED).
//!
//! Loaded via `#[path = "move_speed_tests.rs"]` from `move_speed.rs`.

use super::*;
use ol_content::{ContentDb, ObjectDef};
use ol_world::{NestedHelper, World};

fn content_with_speed(entries: &[(i32, f32)]) -> ContentDb {
    let mut db = ContentDb::default();
    for &(id, sm) in entries {
        let mut d = ObjectDef::empty(id);
        d.speed_mult = sm;
        db.objects.insert(id, d);
    }
    db
}

#[test]
fn floor_counts_as_road_threshold() {
    assert!(!floor_counts_as_road(1.0));
    assert!(!floor_counts_as_road(1.009));
    assert!(floor_counts_as_road(1.01));
    assert!(floor_counts_as_road(1.5));
}

#[test]
fn floor_road_biome_factor_clamps_min() {
    // No floor, off-road, slow biome → min clamp
    let f = floor_road_biome_factor(0, 1.0, 0.05, false, false);
    assert!(
        (f - MIN_BIOME_SPEED_FACTOR).abs() < 1e-5,
        "got {f}, want min {MIN_BIOME_SPEED_FACTOR}"
    );
    // Full path has road + floor mult 1.5, biome 1
    let r = floor_road_biome_factor(1596, 1.5, 1.0, true, false);
    assert!((r - 1.5).abs() < 1e-5, "got {r}");
}

#[test]
fn hitpoints_speed_factor_defaults() {
    let full =
        hitpoints_speed_factor(GROWN_UP_FOOD_STORE_MAX, GROWN_UP_FOOD_STORE_MAX, HITPOINTS_SPEED_FACTOR);
    assert!((full - 1.0).abs() < 1e-5);
    // Zero factor knob disables
    let off = hitpoints_speed_factor(10.0, GROWN_UP_FOOD_STORE_MAX, 0.0);
    assert!((off - 1.0).abs() < 1e-5);
}

#[test]
fn vitals_speed_product_shoes_boost() {
    let mut v = VitalsSpeedInput::default();
    v.has_both_shoes = true;
    let p = vitals_speed_product(&v);
    assert!(p >= SPEED_WITH_BOTH_SHOES - 1e-4, "got {p}");
}

#[test]
fn path_length_cardinal_and_diagonal() {
    assert!((path_length(&[(1, 0), (0, 1)]) - 2.0).abs() < 1e-5);
    let d = path_length(&[(1, 1)]);
    assert!((d - 2f32.sqrt()).abs() < 1e-5);
}

#[test]
fn held_nest_speed_product_empty() {
    let content = ContentDb::default();
    assert!((held_nest_speed_product(&content, None) - 1.0).abs() < 1e-6);
    let empty = NestedHelper::id_only(0);
    assert!((held_nest_speed_product(&content, Some(&empty)) - 1.0).abs() < 1e-6);
}

/// One contained id with speed_mult 0.8 → clamp product 0.8.
// Haxe: MoveHelper.calculateObjSpeedMult + held nest L173-174
#[test]
fn held_nest_speed_product_one_contained() {
    let content = content_with_speed(&[(100, 0.8)]);
    let held = NestedHelper::from_wire(50, &[100]);
    let p = held_nest_speed_product(&content, Some(&held));
    assert!((p - 0.8).abs() < 1e-5, "got {p}");
}

/// Contained + one sub-level both multiply; depth-2 under sub is ignored.
// Haxe: MoveHelper.calculateSpeed L173-179 (one nest under held only)
#[test]
fn held_nest_speed_product_one_sub_depth2_ignored() {
    let content = content_with_speed(&[(100, 0.9), (101, 0.8), (102, 0.7)]);
    let mut sub = NestedHelper::id_only(101);
    sub.contained.push(NestedHelper::id_only(102)); // depth-2 — not scanned
    let mut cargo = NestedHelper::id_only(100);
    cargo.contained.push(sub);
    let mut held = NestedHelper::id_only(50);
    held.contained.push(cargo);
    let p = held_nest_speed_product(&content, Some(&held));
    // 0.9 * 0.8 = 0.72; depth-2 0.7 must not apply
    assert!((p - 0.72).abs() < 1e-5, "got {p}");
}

/// Missing content id → speedMult default 1.0 → clamp upper 0.98.
// Haxe: calculateObjSpeedMult min(MinSpeedReduction, speedMult)
#[test]
fn held_nest_speed_product_missing_content_clamps_098() {
    let content = ContentDb::default();
    let held = NestedHelper::from_wire(50, &[9999]);
    let p = held_nest_speed_product(&content, Some(&held));
    assert!(
        (p - MIN_SPEED_REDUCTION_PER_CONTAINED).abs() < 1e-5,
        "got {p}, want {MIN_SPEED_REDUCTION_PER_CONTAINED}"
    );
}

/// Shoes √ only backpack; nest multiplies after (not under shoes √).
// Haxe: MoveHelper.calculateSpeed L166-179
#[test]
fn combine_backpack_and_held_nest_shoes_sqrt_backpack_only() {
    // backpack raw 0.81 → √ = 0.9 with shoes; nest 0.8 not softened
    let combined = combine_backpack_and_held_nest(0.81, 0.8, true);
    assert!((combined - 0.72).abs() < 1e-5, "got {combined}");
    // without shoes: 0.81 * 0.8
    let hard = combine_backpack_and_held_nest(0.81, 0.8, false);
    assert!((hard - 0.648).abs() < 1e-5, "got {hard}");
}

/// Clothing nest backpack preferred over flat ids (Haxe getPackpack).
// Haxe: GlobalPlayerInstance.getPackpack + MoveHelper L164-168
#[test]
fn resolve_backpack_prefers_clothing_nest() {
    let content = content_with_speed(&[(200, 0.8), (201, 0.7)]);
    // Flat says 201; clothing nest has 200 only
    let pack = NestedHelper::from_wire(198, &[200]);
    let p = resolve_backpack_speed_product(&content, &[201], Some(&pack));
    assert!((p - 0.8).abs() < 1e-5, "got {p} (should use nest 200, not flat 201)");
    // No clothing pack → flat
    let flat = resolve_backpack_speed_product(&content, &[201], None);
    assert!((flat - 0.7).abs() < 1e-5, "got {flat}");
}

/// Empty equipped pack nest → 1.0 even when flat has cargo (no double path).
#[test]
fn resolve_backpack_empty_nest_ignores_flat() {
    let content = content_with_speed(&[(201, 0.7)]);
    let pack = NestedHelper::id_only(198); // equipped, empty contained
    let p = resolve_backpack_speed_product(&content, &[201], Some(&pack));
    assert!((p - 1.0).abs() < 1e-5, "got {p}");
}

/// apply_held_floor_speed_ex: nest after backpack, then bad-biome double / floor√ / horse√.
// Haxe: MoveHelper.calculateSpeed L181-185
#[test]
fn apply_held_floor_speed_ex_nest_biome_floor_horse_order() {
    // product before adjust = backpack 0.9 * nest 0.8 = 0.72 (no shoes)
    // bad biome off-road: 0.72^2 = 0.5184
    let base = 10.0f32;
    let off = apply_held_floor_speed_ex(
        base,
        0,     // no floor
        1.0,   // floor_spd
        0.5,   // biome_spd < 0.9
        false, // full_path_has_road
        false, // boat
        1.0,   // held_speed_mult
        0.9,   // backpack_product
        false, // is_water
        false, // shoes
        false, // strong
        0.8,   // held_nest
    );
    // floor_biome uses min clamp path; contained after double mali:
    // adjust_contained: c=0.72, biome 0.5<0.9 && !on_floor → c*=c → 0.5184
    // no floor√, no horse√
    // held adjust may also change held mult; held=1.0 stays 1.0
    let pack = combine_backpack_and_held_nest(0.9, 0.8, false);
    let contained = adjust_contained_speed_mult(pack, false, false, 0.5);
    let floor_biome = floor_road_biome_factor(0, 1.0, 0.5, false, false);
    let expected = base * floor_biome * 1.0 * contained;
    assert!(
        (off - expected).abs() < 1e-4,
        "off-road bad biome got {off} expected {expected}"
    );

    // On floor: no double mali; if product < 0.99 → √ soften
    let on_floor = apply_held_floor_speed_ex(
        base, 1596, 1.5, 1.0, true, false, 1.0, 0.9, false, false, false, 0.8,
    );
    let pack2 = combine_backpack_and_held_nest(0.9, 0.8, false);
    let contained2 = adjust_contained_speed_mult(pack2, true, false, 1.0);
    let floor_biome2 = floor_road_biome_factor(1596, 1.5, 1.0, true, false);
    let expected2 = base * floor_biome2 * 1.0 * contained2;
    assert!(
        (on_floor - expected2).abs() < 1e-4,
        "on-floor got {on_floor} expected {expected2}"
    );

    // Horse held (≥1.1): contained < 0.99 gets extra √
    let horse = apply_held_floor_speed_ex(
        base, 0, 1.0, 1.0, true, false, 2.0, 0.9, false, false, false, 0.8,
    );
    let pack3 = combine_backpack_and_held_nest(0.9, 0.8, false);
    let contained3 = adjust_contained_speed_mult(pack3, false, true, 1.0);
    let floor_biome3 = floor_road_biome_factor(0, 1.0, 1.0, true, false);
    let held3 = adjust_held_speed_mult(2.0, true, 1.0, false);
    let expected3 = base * floor_biome3 * held3 * contained3;
    assert!(
        (horse - expected3).abs() < 1e-4,
        "horse got {horse} expected {expected3}"
    );
}

#[test]
fn ai_class_speed_factors() {
    assert!((ai_class_speed_factor(false, PrestigeClass::Serf) - 1.0).abs() < 1e-5);
    assert!((ai_class_speed_factor(true, PrestigeClass::Serf) - AI_SPEED_FACTOR_SERF).abs() < 1e-5);
    assert!(
        (ai_class_speed_factor(true, PrestigeClass::Commoner) - AI_SPEED_FACTOR_COMMONER).abs()
            < 1e-5
    );
    assert!((ai_class_speed_factor(true, PrestigeClass::Noble) - AI_SPEED_FACTOR_NOBLE).abs() < 1e-5);
}

#[test]
fn ai_class_speed_factor_ex_live() {
    // C-SS-MORE-BATCH5
    assert!(
        (ai_class_speed_factor_ex(true, PrestigeClass::Serf, 0.7, 0.85, 1.1) - 0.7).abs() < 1e-5
    );
    assert!(
        (ai_class_speed_factor_ex(true, PrestigeClass::Commoner, 0.7, 0.85, 1.1) - 0.85).abs()
            < 1e-5
    );
    assert!(
        (ai_class_speed_factor_ex(true, PrestigeClass::Noble, 0.7, 0.85, 1.1) - 1.1).abs() < 1e-5
    );
    assert!((close_enemy_speed_factor_ex(true, 0.5) - 0.5).abs() < 1e-5);
    assert!((close_enemy_speed_factor_ex(false, 0.5) - 1.0).abs() < 1e-5);
}

#[test]
fn shoes_soften_backpack_sqrt() {
    let raw = 0.81f32;
    let soft = shoes_soften_backpack_product(raw, true);
    assert!((soft - raw.sqrt()).abs() < 1e-5);
    let hard = shoes_soften_backpack_product(raw, false);
    assert!((hard - raw).abs() < 1e-5);
}

#[test]
fn scan_path_empty_world_smoke() {
    let world = World::new(8, 8, false);
    let content = ContentDb::default();
    let scan = scan_path_road_and_biome(&world, &content, 0, 0, &[(1, 0), (1, 0)], 0);
    assert!(!scan.steps.is_empty() || scan.trunc >= 0);
}
