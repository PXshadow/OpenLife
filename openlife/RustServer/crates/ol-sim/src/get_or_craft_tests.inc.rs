use super::*;
use std::collections::HashSet;

fn sample_graph() -> ReverseCraftGraph {
    let mut g = ReverseCraftGraph::new();
    // 1+2 → 3, 3+4 → 5
    g.insert(1, 2, 3, 0);
    g.insert(3, 4, 5, 0);
    g
}

#[test]
fn loose_object_found_pickup_intent() {
    let objs = vec![GetOrCraftWorldObj::simple(100, 5, 5)];
    let inp = GetOrCraftInput::get_item(100, 0, 0);
    let r = get_or_craft_item(&objs, &inp, None, None);
    assert_eq!(
        r,
        GetOrCraftResult::PickupLoose {
            x: 5,
            y: 5,
            object_id: 100
        }
    );
    assert_eq!(
        get_or_craft_result_to_live_intent(r, None),
        ShortCraftLiveIntent::DropAt { x: 5, y: 5 }
    );
}

#[test]
fn missing_craft_false_returns_none() {
    let objs = vec![GetOrCraftWorldObj::simple(1, 0, 0)];
    let inp = GetOrCraftInput::get_item(999, 0, 0);
    assert_eq!(get_or_craft_item(&objs, &inp, None, None), GetOrCraftResult::None);
    let g = sample_graph();
    let empty = HashSet::new();
    assert_eq!(
        get_or_craft_item(&objs, &inp.with_craft(false), Some(&g), Some(&empty)),
        GetOrCraftResult::None
    );
}

#[test]
fn pile_only_empty_hands_use_on_pile() {
    let objs = vec![GetOrCraftWorldObj::simple(11, 3, 4)];
    let inp = GetOrCraftInput::get_item(10, 0, 0).with_pile(11);
    let r = get_or_craft_item(&objs, &inp, None, None);
    assert_eq!(
        r,
        GetOrCraftResult::UseOnPile {
            x: 3,
            y: 4,
            pile_id: 11
        }
    );
    assert_eq!(
        get_or_craft_result_to_live_intent(r, None),
        ShortCraftLiveIntent::UseAt {
            x: 3,
            y: 4,
            target_id: 11,
            actor_id: 0,
        }
    );
}

#[test]
fn pile_with_held_needs_empty_hand() {
    let objs = vec![GetOrCraftWorldObj::simple(11, 3, 4)];
    let inp = GetOrCraftInput::get_item(10, 0, 0)
        .with_pile(11)
        .with_held(50);
    let r = get_or_craft_item(&objs, &inp, None, None);
    assert_eq!(
        r,
        GetOrCraftResult::NeedEmptyHand {
            x: 3,
            y: 4,
            object_id: 11,
            is_pile: true,
        }
    );
    assert_eq!(
        get_or_craft_result_to_live_intent(r, Some((8, 8))),
        ShortCraftLiveIntent::DropAt { x: 8, y: 8 }
    );
}

#[test]
fn container_num_slots_empty_hand_gate() {
    let objs = vec![GetOrCraftWorldObj::simple(292, 2, 2).with_slots(5)];
    let inp = GetOrCraftInput::get_item(292, 0, 0).with_held(1);
    let r = get_or_craft_item(&objs, &inp, None, None);
    assert!(matches!(
        r,
        GetOrCraftResult::NeedEmptyHand {
            is_pile: false,
            object_id: 292,
            ..
        }
    ));
    let empty_hands = GetOrCraftInput::get_item(292, 0, 0);
    assert_eq!(
        get_or_craft_item(&objs, &empty_hands, None, None),
        GetOrCraftResult::PickupLoose {
            x: 2,
            y: 2,
            object_id: 292
        }
    );
}

#[test]
fn target_relative_prefers_near_craft_target() {
    let objs = vec![
        GetOrCraftWorldObj::simple(33, 0, 0),
        GetOrCraftWorldObj::simple(33, 20, 20),
    ];
    let inp = GetOrCraftInput::get_item(33, 0, 0).with_target(21, 21);
    let r = get_or_craft_item(&objs, &inp, None, None);
    assert_eq!(
        r,
        GetOrCraftResult::PickupLoose {
            x: 20,
            y: 20,
            object_id: 33
        }
    );
}

#[test]
fn min_distance_skips_inside_forbidden_radius() {
    let objs = vec![
        GetOrCraftWorldObj::simple(7, 1, 0),
        GetOrCraftWorldObj::simple(7, 8, 0),
    ];
    let inp = GetOrCraftInput::get_item(7, 0, 0).with_min_distance(5);
    let r = get_or_craft_item(&objs, &inp, None, None);
    assert_eq!(
        r,
        GetOrCraftResult::PickupLoose {
            x: 8,
            y: 0,
            object_id: 7
        }
    );
}

#[test]
fn craft_true_missing_seek_ingredient_or_craft_item() {
    let objs: Vec<GetOrCraftWorldObj> = vec![];
    let g = sample_graph();
    let have: HashSet<i32> = [1].into_iter().collect();
    let inp = GetOrCraftInput::get_or_craft(5, 0, 0);
    let r = get_or_craft_item(&objs, &inp, Some(&g), Some(&have));
    match r {
        GetOrCraftResult::SeekIngredient {
            ingredient_id,
            for_product: 5,
        } => {
            assert!(
                matches!(ingredient_id, 2 | 3 | 4),
                "leaf intermediate, got {ingredient_id}"
            );
        }
        GetOrCraftResult::CraftItem { object_id: 5 } => {}
        other => panic!("expected craft staging, got {other:?}"),
    }
    assert_eq!(
        get_or_craft_item(&objs, &inp, None, None),
        GetOrCraftResult::CraftItem { object_id: 5 }
    );
}

#[test]
fn craft_if_needed_false_never_expands_graph() {
    let objs: Vec<GetOrCraftWorldObj> = vec![];
    let g = sample_graph();
    let empty = HashSet::new();
    let inp = GetOrCraftInput::get_item(5, 0, 0);
    assert_eq!(
        get_or_craft_item(&objs, &inp, Some(&g), Some(&empty)),
        GetOrCraftResult::None
    );
}

#[test]
fn loose_preferred_over_pile_when_both() {
    let objs = vec![
        GetOrCraftWorldObj::simple(10, 4, 0),
        GetOrCraftWorldObj::simple(11, 2, 0),
    ];
    let inp = GetOrCraftInput::get_item(10, 0, 0).with_pile(11);
    let r = get_or_craft_item(&objs, &inp, None, None);
    assert_eq!(
        r,
        GetOrCraftResult::PickupLoose {
            x: 4,
            y: 0,
            object_id: 10
        }
    );
}

#[test]
fn pile_close_r5_then_far_max_search() {
    let objs = vec![GetOrCraftWorldObj::simple(11, 20, 0)];
    let inp = GetOrCraftInput::get_item(10, 0, 0)
        .with_pile(11)
        .with_max_search(40);
    let r = get_or_craft_item(&objs, &inp, None, None);
    assert_eq!(
        r,
        GetOrCraftResult::UseOnPile {
            x: 20,
            y: 0,
            pile_id: 11
        }
    );
    let near = GetOrCraftInput::get_item(10, 0, 0)
        .with_pile(11)
        .with_max_search(10);
    assert_eq!(
        get_or_craft_item(&objs, &near, None, None),
        GetOrCraftResult::None
    );
}

#[test]
fn resolve_seek_or_craft_to_use_after_find() {
    let objs = vec![GetOrCraftWorldObj::simple(850, 6, 6)];
    let intent = ShortCraftLiveIntent::SeekOrCraft {
        actor: 850,
        craft_if_needed: true,
    };
    let resolved = resolve_seek_or_craft_live(
        intent,
        &objs,
        0,
        0,
        0,
        None,
        &|_| -1,
        None,
        None,
        None,
    );
    assert_eq!(resolved, ShortCraftLiveIntent::DropAt { x: 6, y: 6 });
    assert!(resolved.is_wire_action());
}

#[test]
fn resolve_seek_craft_false_no_expand() {
    let intent = ShortCraftLiveIntent::SeekOrCraft {
        actor: 5,
        craft_if_needed: false,
    };
    let g = sample_graph();
    let empty = HashSet::new();
    let resolved = resolve_seek_or_craft_live(
        intent,
        &[],
        0,
        0,
        0,
        None,
        &|_| -1,
        None,
        Some(&g),
        Some(&empty),
    );
    assert_eq!(resolved, ShortCraftLiveIntent::None);
}

#[test]
fn busy_moving_short_circuits() {
    let objs = vec![GetOrCraftWorldObj::simple(1, 0, 0)];
    let mut inp = GetOrCraftInput::get_item(1, 0, 0);
    inp.is_moving = true;
    assert_eq!(
        get_or_craft_item(&objs, &inp, None, None),
        GetOrCraftResult::BusyMoving
    );
}

#[test]
fn busy_moving_to_wait_live_intent() {
    // Haxe: GetOrCraftItem isMoving return true → hold tick (PREFER-SHORT-WAIT)
    assert_eq!(
        get_or_craft_result_to_live_intent(GetOrCraftResult::BusyMoving, None),
        ShortCraftLiveIntent::Wait
    );
    assert!(crate::short_craft_intent::live_intent_is_wait(
        ShortCraftLiveIntent::Wait
    ));
    assert!(crate::short_craft_intent::drop_held_live_intent_actionable(
        ShortCraftLiveIntent::Wait
    ));
    assert!(!ShortCraftLiveIntent::Wait.is_wire_action());
}

#[test]
fn get_item_helper_wraps_craft_false() {
    let objs = vec![GetOrCraftWorldObj::simple(44, 1, 1)];
    assert_eq!(
        get_item(&objs, 44, 0, 0, 40, None, -1, 0),
        GetOrCraftResult::PickupLoose {
            x: 1,
            y: 1,
            object_id: 44
        }
    );
    assert_eq!(
        get_item(&objs, 99, 0, 0, 40, None, -1, 0),
        GetOrCraftResult::None
    );
}

#[test]
fn resolve_craft_item_multi_step_pickup_actor() {
    // AI-CRAFT-MULTI: CraftItem expands to pickup when leaf pair present
    let g = sample_graph();
    let objs = vec![
        GetOrCraftWorldObj::simple(1, 5, 0),
        GetOrCraftWorldObj::simple(2, 6, 0),
    ];
    let intent = ShortCraftLiveIntent::CraftItem { object_id: 3 };
    let resolved = resolve_seek_or_craft_live(
        intent,
        &objs,
        0,
        0,
        0,
        None,
        &|_| -1,
        Some((0, 1)),
        Some(&g),
        None,
    );
    assert!(
        matches!(
            resolved,
            ShortCraftLiveIntent::DropAt { .. } | ShortCraftLiveIntent::UseAt { .. }
        ),
        "multi-step should produce wire staging, got {resolved:?}"
    );
}

#[test]
fn expand_craft_item_held_actor_uses_target() {
    let g = sample_graph();
    let objs = vec![GetOrCraftWorldObj::simple(2, 3, 3)];
    let live = expand_craft_item_live(
        3,
        &objs,
        0,
        0,
        1, // holding actor 1
        &|_| -1,
        None,
        &g,
        None,
        true,
        0.0,
    );
    assert_eq!(
        live,
        ShortCraftLiveIntent::UseAt {
            x: 3,
            y: 3,
            target_id: 2,
            actor_id: 1,
        }
    );
}

#[test]
fn expand_craft_item_live_ex_home_and_smith() {
    // AI-CRAFT-MULTI: resolve_seek_or_craft_live_ex wires home + smith gate
    let mut g = ReverseCraftGraph::new();
    g.insert(308, 303, 9000, 0);
    let objs = vec![
        GetOrCraftWorldObj::simple(308, 1, 1),
        GetOrCraftWorldObj::simple(303, 2, 2),
    ];
    let opts = CraftLiveExpandOpts::default().with_smith(false).with_home(0, 0);
    let intent = ShortCraftLiveIntent::CraftItem { object_id: 9000 };
    let resolved = resolve_seek_or_craft_live_ex(
        intent,
        &objs,
        0,
        0,
        308,
        None,
        &|_| -1,
        None,
        Some(&g),
        None,
        &opts,
        None,
    );
    // NeedSmithProfession → None intent
    assert_eq!(resolved, ShortCraftLiveIntent::None);

    let opts_ok = CraftLiveExpandOpts::default().with_smith(true);
    let resolved_ok = resolve_seek_or_craft_live_ex(
        ShortCraftLiveIntent::CraftItem { object_id: 9000 },
        &objs,
        0,
        0,
        308,
        None,
        &|_| -1,
        None,
        Some(&g),
        None,
        &opts_ok,
        None,
    );
    assert_eq!(
        resolved_ok,
        ShortCraftLiveIntent::UseAt {
            x: 2,
            y: 2,
            target_id: 303,
            actor_id: 308,
        }
    );
}

#[test]
fn sticky_runtime_cooldown_across_expand() {
    let g = sample_graph();
    let mut runtime = CraftAiRuntime::new();
    // Record fail manually then prove expand respects sticky cooldown.
    runtime.failed.record_fail(5, 50.0);
    let opts = CraftLiveExpandOpts::default().with_now(55.0);
    let live = expand_craft_item_live_sticky(
        5,
        &[],
        0,
        0,
        0,
        &|_| -1,
        None,
        &g,
        &opts,
        &mut runtime,
    );
    assert_eq!(live, ShortCraftLiveIntent::None); // Cooldown → None
}

/// GetOrCraft skips path-blocked tiles and picks next-closest (AI-CRAFT-LIVE-RESID).
// Haxe: GetClosestObject* isObjectNotReachable / isObjectWithHostilePath
#[test]
fn get_or_craft_skips_blocked_tile_picks_alt() {
    let objs = vec![
        GetOrCraftWorldObj::simple(10, 1, 0),
        GetOrCraftWorldObj::simple(10, 5, 0),
    ];
    let mut blocked = HashSet::new();
    blocked.insert((1, 0));
    let inp = GetOrCraftInput::get_item(10, 0, 0).with_max_search(40);
    let r = get_or_craft_item_ex(&objs, &inp, None, None, Some(&blocked));
    match r {
        GetOrCraftResult::PickupLoose { x, y, object_id } => {
            assert_eq!(object_id, 10);
            assert_eq!((x, y), (5, 0));
        }
        other => panic!("expected PickupLoose on free tile, got {other:?}"),
    }
}

#[test]
fn closest_obj_by_id_filtered_respects_blocked() {
    let objs = vec![
        GetOrCraftWorldObj::simple(7, 2, 0),
        GetOrCraftWorldObj::simple(7, 4, 0),
    ];
    let mut blocked = HashSet::new();
    blocked.insert((2, 0));
    let o = closest_obj_by_id_filtered(&objs, 7, 0, 0, 40, 0, Some(&blocked)).unwrap();
    assert_eq!((o.x, o.y), (4, 0));
}

// ── AI-CRAFT-NPC-ENQUEUE: multi-step GetOrCraft + scan filters ──────────────

/// SeekOrCraft skips path-blocked tile and picks free alt (npc enqueue path).
// Haxe: GetOrCraftItem + isObjectNotReachable
#[test]
fn npc_enqueue_seek_skips_blocked_tile() {
    let objs = vec![
        GetOrCraftWorldObj::simple(50, 1, 0),
        GetOrCraftWorldObj::simple(50, 8, 0),
    ];
    let mut blocked = HashSet::new();
    blocked.insert((1, 0));
    let intent = ShortCraftLiveIntent::SeekOrCraft {
        actor: 50,
        craft_if_needed: false,
    };
    let opts = CraftLiveExpandOpts::default();
    let resolved = npc_enqueue_get_or_craft(
        intent,
        &objs,
        0,
        0,
        0,
        false,
        None,
        None,
        &opts,
        None,
        Some(&blocked),
    );
    assert_eq!(resolved, ShortCraftLiveIntent::DropAt { x: 8, y: 0 });
}

/// CraftItem multi-step expands to USE when leaf pair present (npc enqueue).
// Haxe: craftItem → craftItemHelper → useTarget
#[test]
fn npc_enqueue_craft_item_multi_step_use() {
    let g = sample_graph();
    let objs = vec![
        GetOrCraftWorldObj::simple(1, 2, 0),
        GetOrCraftWorldObj::simple(2, 3, 0),
    ];
    let intent = ShortCraftLiveIntent::CraftItem { object_id: 3 };
    let opts = CraftLiveExpandOpts::default();
    let resolved = npc_enqueue_get_or_craft(
        intent,
        &objs,
        0,
        0,
        0,
        false,
        Some((0, 1)),
        Some(&g),
        &opts,
        None,
        None,
    );
    assert!(
        matches!(
            resolved,
            ShortCraftLiveIntent::DropAt { .. } | ShortCraftLiveIntent::UseAt { .. }
        ),
        "npc multi-step craft should produce wire staging, got {resolved:?}"
    );
}

/// SeekOrCraft with craft_if_needed expands multi-step when product missing.
// Haxe: GetOrCraftItem(obj, craft=true) → craftItem
#[test]
fn npc_enqueue_seek_or_craft_expands_multi_step_on_miss() {
    let g = sample_graph();
    // Have leaf ingredients for product 3, but not 3 itself.
    let objs = vec![
        GetOrCraftWorldObj::simple(1, 4, 0),
        GetOrCraftWorldObj::simple(2, 5, 0),
    ];
    let intent = ShortCraftLiveIntent::SeekOrCraft {
        actor: 3,
        craft_if_needed: true,
    };
    let opts = CraftLiveExpandOpts::default();
    let resolved = npc_enqueue_get_or_craft(
        intent,
        &objs,
        0,
        0,
        0,
        false,
        Some((0, 1)),
        Some(&g),
        &opts,
        None,
        None,
    );
    assert!(
        matches!(
            resolved,
            ShortCraftLiveIntent::DropAt { .. } | ShortCraftLiveIntent::UseAt { .. }
        ),
        "SeekOrCraft craft_if_needed should multi-step expand, got {resolved:?}"
    );
}

/// Path-blocked multi-step actor is skipped; free pair used (scan filters).
// Haxe: addObjectsForCrafting isObjectNotReachable
#[test]
fn npc_enqueue_multi_step_skips_blocked_actor() {
    let g = sample_graph();
    // Actor 1 blocked at (1,0); free actor at (6,0); target 2 at (7,0).
    let objs = vec![
        GetOrCraftWorldObj::simple(1, 1, 0),
        GetOrCraftWorldObj::simple(1, 6, 0),
        GetOrCraftWorldObj::simple(2, 7, 0),
    ];
    let mut blocked = HashSet::new();
    blocked.insert((1, 0));
    let intent = ShortCraftLiveIntent::CraftItem { object_id: 3 };
    let opts = CraftLiveExpandOpts::default();
    let resolved = npc_enqueue_get_or_craft(
        intent,
        &objs,
        0,
        0,
        0,
        false,
        Some((0, 1)),
        Some(&g),
        &opts,
        None,
        Some(&blocked),
    );
    match resolved {
        ShortCraftLiveIntent::DropAt { x, y } => {
            // Pickup free actor, not blocked one
            assert_ne!((x, y), (1, 0));
            assert!(x == 6 || x == 7, "expected free tile, got {x},{y}");
        }
        ShortCraftLiveIntent::UseAt { x, y, .. } => {
            assert_ne!((x, y), (1, 0));
        }
        other => panic!("expected wire staging, got {other:?}"),
    }
}

/// BusyMoving → Wait hold tick (PREFER-SHORT-WAIT via npc enqueue).
#[test]
fn npc_enqueue_busy_moving_waits() {
    let objs = vec![GetOrCraftWorldObj::simple(1, 0, 0)];
    let intent = ShortCraftLiveIntent::SeekOrCraft {
        actor: 1,
        craft_if_needed: false,
    };
    let opts = CraftLiveExpandOpts::default();
    let resolved = npc_enqueue_get_or_craft(
        intent,
        &objs,
        0,
        0,
        0,
        true, // is_moving
        None,
        None,
        &opts,
        None,
        None,
    );
    assert_eq!(resolved, ShortCraftLiveIntent::Wait);
}

/// Sticky runtime cooldown survives across npc_enqueue multi-step.
#[test]
fn npc_enqueue_sticky_runtime_cooldown() {
    let g = sample_graph();
    let mut runtime = CraftAiRuntime::new();
    runtime.failed.record_fail(5, 10.0);
    let opts = CraftLiveExpandOpts::default().with_now(12.0); // within 15s cooldown
    let resolved = npc_enqueue_get_or_craft(
        ShortCraftLiveIntent::CraftItem { object_id: 5 },
        &[],
        0,
        0,
        0,
        false,
        None,
        Some(&g),
        &opts,
        Some(&mut runtime),
        None,
    );
    assert_eq!(resolved, ShortCraftLiveIntent::None);
}

/// resolve_seek_or_craft_live_ex_scan threads blocked into shallow GetOrCraft.
#[test]
fn resolve_ex_scan_skips_blocked_product() {
    let objs = vec![
        GetOrCraftWorldObj::simple(9, 2, 0),
        GetOrCraftWorldObj::simple(9, 9, 0),
    ];
    let mut blocked = HashSet::new();
    blocked.insert((2, 0));
    let scan = CraftScanFilters::new().with_blocked(&blocked);
    let opts = CraftLiveExpandOpts::default();
    let resolved = resolve_seek_or_craft_live_ex_scan(
        ShortCraftLiveIntent::SeekOrCraft {
            actor: 9,
            craft_if_needed: false,
        },
        &objs,
        0,
        0,
        0,
        None,
        &|_| 0,
        None,
        None,
        None,
        &opts,
        None,
        scan,
        false,
    );
    assert_eq!(resolved, ShortCraftLiveIntent::DropAt { x: 9, y: 0 });
}

// ── AI-CRAFT-NPC-ENQUEUE gap-close: pile_id / full_piles / num_slots ─────────

/// Haxe ObjectData.getPileObjId: self+self → pile, empty+pile → original.
// Haxe: ObjectData.getPileObjId ~1531
#[test]
fn get_pile_obj_id_self_self_with_undo() {
    let mut map = std::collections::HashMap::new();
    // stone 33 + 33 → pile 32; empty + pile → stone
    map.insert((33, 33), (0, 32));
    map.insert((0, 32), (33, 32));
    assert_eq!(get_pile_obj_id_from_map(33, &map), 32);
    assert_eq!(get_pile_obj_id_from_map(99, &map), -1);
    // Undo yields different actor → not a pile form
    map.insert((50, 50), (0, 51));
    map.insert((0, 51), (9, 51));
    assert_eq!(get_pile_obj_id_from_map(50, &map), -1);
}

/// npc_enqueue_ex uses pile_id_for: pile-only world → UseOnPile / UseAt.
// Haxe: GetOrCraftItem hasPile searchDistance=5 + usePile
#[test]
fn npc_enqueue_ex_pile_id_uses_pile_form() {
    // Loose stone missing; pile 32 present
    let objs = vec![GetOrCraftWorldObj::simple(32, 3, 0)];
    let pile_for = |id: i32| if id == 33 { 32 } else { 0 };
    let opts = CraftLiveExpandOpts::default();
    let resolved = npc_enqueue_get_or_craft_ex(
        ShortCraftLiveIntent::SeekOrCraft {
            actor: 33,
            craft_if_needed: false,
        },
        &objs,
        0,
        0,
        0,
        false,
        None,
        None,
        &opts,
        None,
        &pile_for,
        None,
        None,
    );
    assert_eq!(
        resolved,
        ShortCraftLiveIntent::UseAt {
            x: 3,
            y: 0,
            target_id: 32,
            actor_id: 0,
        }
    );
}

/// ignoreFullPiles via full_pile_tiles: multi-step skips full multi-use actor tile.
// Haxe: ignoreFullPiles / numberOfUses >= numUses
#[test]
fn npc_enqueue_ex_full_pile_tiles_skipped_in_multi_step() {
    let g = sample_graph();
    // Actor 1 at (1,0) is "full pile" — should be skipped; free actor at (6,0)
    let objs = vec![
        GetOrCraftWorldObj::simple(1, 1, 0),
        GetOrCraftWorldObj::simple(1, 6, 0),
        GetOrCraftWorldObj::simple(2, 7, 0),
    ];
    let mut full = HashSet::new();
    full.insert((1, 0));
    let opts = CraftLiveExpandOpts::default();
    let resolved = npc_enqueue_get_or_craft_ex(
        ShortCraftLiveIntent::CraftItem { object_id: 3 },
        &objs,
        0,
        0,
        0,
        false,
        Some((0, 1)),
        Some(&g),
        &opts,
        None,
        &|_| 0,
        None,
        Some(&full),
    );
    match resolved {
        ShortCraftLiveIntent::DropAt { x, y } => {
            assert_ne!((x, y), (1, 0), "full pile tile must not be picked");
        }
        ShortCraftLiveIntent::UseAt { x, y, .. } => {
            assert_ne!((x, y), (1, 0), "full pile tile must not be picked");
        }
        other => panic!("expected wire staging, got {other:?}"),
    }
}

/// blocked set merges peer blockedByAI with path_reach (pure blocked_coords).
// Haxe: isObjectNotReachable ORs blockedByAI
#[test]
fn npc_enqueue_blocked_by_ai_merge_skips_peer_claim() {
    let objs = vec![
        GetOrCraftWorldObj::simple(50, 2, 0),
        GetOrCraftWorldObj::simple(50, 8, 0),
    ];
    // Peer AI claimed (2,0)
    let mut blocked = HashSet::new();
    blocked.insert((2, 0));
    let opts = CraftLiveExpandOpts::default();
    let resolved = npc_enqueue_get_or_craft(
        ShortCraftLiveIntent::SeekOrCraft {
            actor: 50,
            craft_if_needed: false,
        },
        &objs,
        0,
        0,
        0,
        false,
        None,
        None,
        &opts,
        None,
        Some(&blocked),
    );
    assert_eq!(resolved, ShortCraftLiveIntent::DropAt { x: 8, y: 0 });
}

/// ScanTile.num_slots flows into GetOrCraftWorldObj (container empty-hand gate).
// Haxe: objectData.numSlots > 0 → dropHeld before pickup
#[test]
fn get_or_craft_objs_from_scan_prefers_tile_num_slots() {
    use crate::{get_or_craft_objs_from_scan, ScanTile};
    let tiles = vec![
        ScanTile::simple(292, 1, 0).with_num_slots(5),
        ScanTile::simple(33, 2, 0), // no slots
    ];
    // Lookup would wrongly say basket has 0; tile.num_slots must win
    let objs = get_or_craft_objs_from_scan(&tiles, Some(&|_| 0));
    assert_eq!(objs.len(), 2);
    let basket = objs.iter().find(|o| o.parent_id == 292).unwrap();
    assert_eq!(basket.num_slots, 5);
    let stone = objs.iter().find(|o| o.parent_id == 33).unwrap();
    assert_eq!(stone.num_slots, 0);
}

/// full_pile_tiles_from_scan collects is_full_uses coords.
// Haxe: numberOfUses >= objectData.numUses
#[test]
fn full_pile_tiles_from_scan_collects_full_uses() {
    use crate::{full_pile_tiles_from_scan, ScanTile};
    let tiles = vec![
        ScanTile::simple(32, 1, 0).with_uses(3).with_num_uses(3), // full
        ScanTile::simple(32, 2, 0).with_uses(1).with_num_uses(3), // partial
        ScanTile::simple(33, 3, 0),                               // single-use
    ];
    let full = full_pile_tiles_from_scan(&tiles);
    assert!(full.contains(&(1, 0)));
    assert!(!full.contains(&(2, 0)));
    assert!(!full.contains(&(3, 0)));
}
