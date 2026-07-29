/// Resolve SeekOrCraft / CraftItem staging against world objects + optional graph.
///
/// When the sought actor is already on the ground → pickup / pile USE path.
/// When craft_if_needed and missing → reverse-graph expand once.
/// **CraftItem** expands via multi-step [`craft_item_helper`] when `graph` is set
/// (**AI-CRAFT-MULTI**). Multi-step is depth-1: SeekOrCraft from multi-step uses
/// shallow GetOrCraft only (no nested craftItem expand) to avoid loops.
///
/// Uses default [`CraftLiveExpandOpts`] (no home, smith=true, now=0). Prefer
/// [`resolve_seek_or_craft_live_ex`] when home / SMITH / sticky runtime matter.
// Haxe: shortCraft → GetOrCraftItem(actor, craftActorIfNeeded, target)
// Haxe: craftItem(objId) multi-step
pub fn resolve_seek_or_craft_live(
    intent: ShortCraftLiveIntent,
    objs: &[GetOrCraftWorldObj],
    player_x: i32,
    player_y: i32,
    held_id: i32,
    target: Option<(i32, i32)>,
    pile_id_for: &dyn Fn(i32) -> i32,
    empty_drop: Option<(i32, i32)>,
    graph: Option<&ReverseCraftGraph>,
    have: Option<&HashSet<i32>>,
) -> ShortCraftLiveIntent {
    resolve_seek_or_craft_live_ex(
        intent,
        objs,
        player_x,
        player_y,
        held_id,
        target,
        pile_id_for,
        empty_drop,
        graph,
        have,
        &CraftLiveExpandOpts::default(),
        None,
    )
}

/// Resolve SeekOrCraft / CraftItem with home / smith / now / sticky runtime.
// Haxe: craftItem(home startLocation, SMITH, failedCraftings, itemToCraft)
pub fn resolve_seek_or_craft_live_ex(
    intent: ShortCraftLiveIntent,
    objs: &[GetOrCraftWorldObj],
    player_x: i32,
    player_y: i32,
    held_id: i32,
    target: Option<(i32, i32)>,
    pile_id_for: &dyn Fn(i32) -> i32,
    empty_drop: Option<(i32, i32)>,
    graph: Option<&ReverseCraftGraph>,
    have: Option<&HashSet<i32>>,
    opts: &CraftLiveExpandOpts,
    runtime: Option<&mut CraftAiRuntime>,
) -> ShortCraftLiveIntent {
    resolve_seek_or_craft_live_ex_scan(
        intent,
        objs,
        player_x,
        player_y,
        held_id,
        target,
        pile_id_for,
        empty_drop,
        graph,
        have,
        opts,
        runtime,
        CraftScanFilters::default(),
        false,
    )
}

/// Resolve with path-reach / hostile / blockedByAI scan filters (AI-CRAFT-LIVE-RESID +
/// AI-CRAFT-NPC-ENQUEUE). Used by npc_ai multi-step GetOrCraft enqueue.
// Haxe: GetOrCraftItem + craftItem + isObjectNotReachable / isObjectWithHostilePath
pub fn resolve_seek_or_craft_live_scan(
    intent: ShortCraftLiveIntent,
    objs: &[GetOrCraftWorldObj],
    player_x: i32,
    player_y: i32,
    held_id: i32,
    target: Option<(i32, i32)>,
    pile_id_for: &dyn Fn(i32) -> i32,
    empty_drop: Option<(i32, i32)>,
    graph: Option<&ReverseCraftGraph>,
    have: Option<&HashSet<i32>>,
    opts: &CraftLiveExpandOpts,
    runtime: Option<&mut CraftAiRuntime>,
    scan: CraftScanFilters<'_>,
) -> ShortCraftLiveIntent {
    resolve_seek_or_craft_live_ex_scan(
        intent,
        objs,
        player_x,
        player_y,
        held_id,
        target,
        pile_id_for,
        empty_drop,
        graph,
        have,
        opts,
        runtime,
        scan,
        false,
    )
}

/// Full multi-step GetOrCraft resolve for NPC/AI enqueue.
///
/// - `CraftItem` → multi-step craftItemHelper with `scan` filters
/// - `SeekOrCraft{craft_if_needed}` miss → same multi-step (Haxe GetOrCraftItem→craftItem)
/// - Shallow GetOrCraft skips tiles in `scan.blocked`
/// - `is_moving` → BusyMoving → Wait (PREFER-SHORT-WAIT)
// Haxe: AiBase.GetOrCraftItem ~6150 + craftItem ~6611; npc_ai multi-step enqueue
pub fn resolve_seek_or_craft_live_ex_scan(
    intent: ShortCraftLiveIntent,
    objs: &[GetOrCraftWorldObj],
    player_x: i32,
    player_y: i32,
    held_id: i32,
    target: Option<(i32, i32)>,
    pile_id_for: &dyn Fn(i32) -> i32,
    empty_drop: Option<(i32, i32)>,
    graph: Option<&ReverseCraftGraph>,
    have: Option<&HashSet<i32>>,
    opts: &CraftLiveExpandOpts,
    mut runtime: Option<&mut CraftAiRuntime>,
    scan: CraftScanFilters<'_>,
    is_moving: bool,
) -> ShortCraftLiveIntent {
    if is_moving {
        // Haxe: GetOrCraftItem isMoving return true → hold tick
        return ShortCraftLiveIntent::Wait;
    }

    // Multi-step craftItem expand (AI-CRAFT-MULTI) before shallow GetOrCraft.
    if let ShortCraftLiveIntent::CraftItem { object_id } = intent {
        if object_id > 0 {
            if let Some(g) = graph {
                let expanded = expand_craft_item_live_opts_scan(
                    object_id,
                    objs,
                    player_x,
                    player_y,
                    held_id,
                    pile_id_for,
                    empty_drop,
                    g,
                    opts,
                    runtime.as_deref_mut(),
                    scan,
                );
                match expanded {
                    // Leaf seek → shallow GetOrCraft only (no nested multi-step).
                    ShortCraftLiveIntent::SeekOrCraft {
                        actor,
                        craft_if_needed,
                    } => {
                        return resolve_seek_or_craft_shallow_scan(
                            actor,
                            craft_if_needed,
                            objs,
                            player_x,
                            player_y,
                            held_id,
                            target,
                            pile_id_for,
                            empty_drop,
                            graph,
                            have,
                            scan.blocked,
                            false, // depth-1: do not re-enter multi-step
                        );
                    }
                    // Still CraftItem → fall through to shallow product seek
                    ShortCraftLiveIntent::CraftItem { .. } => {}
                    // Wire / drop / use / none from multi-step
                    other => return other,
                }
            }
        }
    }

    let (actor, craft_if_needed) = match intent {
        ShortCraftLiveIntent::SeekOrCraft {
            actor,
            craft_if_needed,
        } => (actor, craft_if_needed),
        ShortCraftLiveIntent::CraftItem { object_id } => (object_id, true),
        ShortCraftLiveIntent::SeekGroundActor { target } => (target, false),
        other => return other,
    };

    // AI-CRAFT-NPC-ENQUEUE: GetOrCraft miss with craft_if_needed → multi-step craftItem
    // (Haxe GetOrCraftItem → craftItem(objId)), not just leaf seek staging.
    let shallow = resolve_seek_or_craft_shallow_scan(
        actor,
        craft_if_needed,
        objs,
        player_x,
        player_y,
        held_id,
        target,
        pile_id_for,
        empty_drop,
        graph,
        have,
        scan.blocked,
        false,
    );
    if !craft_if_needed {
        return shallow;
    }
    match shallow {
        // Full craftItem residual (no leaf from graph) → multi-step helper.
        ShortCraftLiveIntent::CraftItem { object_id } if object_id > 0 => {
            if let Some(g) = graph {
                let expanded = expand_craft_item_live_opts_scan(
                    object_id,
                    objs,
                    player_x,
                    player_y,
                    held_id,
                    pile_id_for,
                    empty_drop,
                    g,
                    opts,
                    runtime.as_deref_mut(),
                    scan,
                );
                return match expanded {
                    ShortCraftLiveIntent::SeekOrCraft {
                        actor: a,
                        craft_if_needed: c,
                    } => resolve_seek_or_craft_shallow_scan(
                        a,
                        false, // depth-1: locate leaf only
                        objs,
                        player_x,
                        player_y,
                        held_id,
                        target,
                        pile_id_for,
                        empty_drop,
                        graph,
                        have,
                        scan.blocked,
                        false,
                    ),
                    other => other,
                };
            }
            shallow
        }
        // craft_item_fallback leaf seek → resolve ingredient location this tick.
        // Haxe: craftItem first step often USE/DROP on missing leaf already in world.
        ShortCraftLiveIntent::SeekOrCraft {
            actor: leaf,
            craft_if_needed: _,
        } if leaf > 0 && leaf != actor => {
            // Prefer multi-step expand on original product (find pair, USE/DROP).
            if let Some(g) = graph {
                let expanded = expand_craft_item_live_opts_scan(
                    actor,
                    objs,
                    player_x,
                    player_y,
                    held_id,
                    pile_id_for,
                    empty_drop,
                    g,
                    opts,
                    runtime.as_deref_mut(),
                    scan,
                );
                match expanded {
                    ShortCraftLiveIntent::None
                    | ShortCraftLiveIntent::CraftItem { .. }
                    | ShortCraftLiveIntent::SeekOrCraft { .. } => {
                        // Fall back: pick up known leaf ingredient if present.
                        resolve_seek_or_craft_shallow_scan(
                            leaf,
                            false,
                            objs,
                            player_x,
                            player_y,
                            held_id,
                            target,
                            pile_id_for,
                            empty_drop,
                            graph,
                            have,
                            scan.blocked,
                            false,
                        )
                    }
                    other => other,
                }
            } else {
                resolve_seek_or_craft_shallow_scan(
                    leaf,
                    false,
                    objs,
                    player_x,
                    player_y,
                    held_id,
                    target,
                    pile_id_for,
                    empty_drop,
                    graph,
                    have,
                    scan.blocked,
                    false,
                )
            }
        }
        other => other,
    }
}

/// NPC pure helper: multi-step GetOrCraft enqueue with CraftScanFilters.
///
/// Converts profession `SeekOrCraft` / `CraftItem` staging into wire-ready
/// `UseAt` / `DropAt` / `Wait` / residual staging, honoring notReachable /
/// hostile / blockedByAI tiles and sticky multi-tick craft runtime.
///
/// Defaults: no pile form (`pile_id=0`), no ignoreFullPiles set. Prefer
/// [`npc_enqueue_get_or_craft_ex`] when live `getPileObjId` / full multi-use
/// tiles are available (AI-CRAFT-NPC-ENQUEUE residuals).
// Haxe: AiBase.GetOrCraftItem + craftItem → useTarget / dropTarget (npc action queue)
#[inline]
pub fn npc_enqueue_get_or_craft(
    intent: ShortCraftLiveIntent,
    objs: &[GetOrCraftWorldObj],
    player_x: i32,
    player_y: i32,
    held_id: i32,
    is_moving: bool,
    empty_drop: Option<(i32, i32)>,
    graph: Option<&ReverseCraftGraph>,
    opts: &CraftLiveExpandOpts,
    runtime: Option<&mut CraftAiRuntime>,
    blocked: Option<&HashSet<(i32, i32)>>,
) -> ShortCraftLiveIntent {
    npc_enqueue_get_or_craft_ex(
        intent,
        objs,
        player_x,
        player_y,
        held_id,
        is_moving,
        empty_drop,
        graph,
        opts,
        runtime,
        &|_| 0,
        blocked,
        None,
    )
}

/// NPC GetOrCraft enqueue with live pile_id + ignoreFullPiles scan filters.
///
/// - `pile_id_for`: Haxe `ObjectData.getPileObjId` (pile-first r=5 + pile USE)
/// - `blocked`: notReachable ∪ hostile ∪ blockedByAI tiles
/// - `full_pile_tiles`: when `Some`, enables `ignoreFullPiles` on multi-step
///   craft expand (skips full multi-use tiles in pair search)
// Haxe: GetOrCraftItem pileId + craftItemHelper filters; isObjectNotReachable
#[inline]
pub fn npc_enqueue_get_or_craft_ex(
    intent: ShortCraftLiveIntent,
    objs: &[GetOrCraftWorldObj],
    player_x: i32,
    player_y: i32,
    held_id: i32,
    is_moving: bool,
    empty_drop: Option<(i32, i32)>,
    graph: Option<&ReverseCraftGraph>,
    opts: &CraftLiveExpandOpts,
    runtime: Option<&mut CraftAiRuntime>,
    pile_id_for: &dyn Fn(i32) -> i32,
    blocked: Option<&HashSet<(i32, i32)>>,
    full_pile_tiles: Option<&HashSet<(i32, i32)>>,
) -> ShortCraftLiveIntent {
    let mut scan = match blocked {
        Some(b) => CraftScanFilters::new().with_blocked(b),
        None => CraftScanFilters::default(),
    };
    if let Some(fp) = full_pile_tiles {
        // Haxe: ignoreFullPiles = true for full multi-use skip on closest search
        scan = scan.with_full_piles(fp);
    }
    resolve_seek_or_craft_live_ex_scan(
        intent,
        objs,
        player_x,
        player_y,
        held_id,
        None,
        pile_id_for,
        empty_drop,
        graph,
        None,
        opts,
        runtime,
        scan,
        is_moving,
    )
}

/// Shallow GetOrCraft resolve (no multi-step craftItem expand).
fn resolve_seek_or_craft_shallow(
    actor: i32,
    craft_if_needed: bool,
    objs: &[GetOrCraftWorldObj],
    player_x: i32,
    player_y: i32,
    held_id: i32,
    target: Option<(i32, i32)>,
    pile_id_for: &dyn Fn(i32) -> i32,
    empty_drop: Option<(i32, i32)>,
    graph: Option<&ReverseCraftGraph>,
    have: Option<&HashSet<i32>>,
) -> ShortCraftLiveIntent {
    resolve_seek_or_craft_shallow_scan(
        actor,
        craft_if_needed,
        objs,
        player_x,
        player_y,
        held_id,
        target,
        pile_id_for,
        empty_drop,
        graph,
        have,
        None,
        false,
    )
}

/// Shallow GetOrCraft with optional path-blocked tiles.
// Haxe: GetClosestObject* isObjectNotReachable / hostile (AI-CRAFT-LIVE-RESID)
fn resolve_seek_or_craft_shallow_scan(
    actor: i32,
    craft_if_needed: bool,
    objs: &[GetOrCraftWorldObj],
    player_x: i32,
    player_y: i32,
    held_id: i32,
    target: Option<(i32, i32)>,
    pile_id_for: &dyn Fn(i32) -> i32,
    empty_drop: Option<(i32, i32)>,
    graph: Option<&ReverseCraftGraph>,
    have: Option<&HashSet<i32>>,
    blocked: Option<&HashSet<(i32, i32)>>,
    is_moving: bool,
) -> ShortCraftLiveIntent {
    if actor <= 0 {
        return ShortCraftLiveIntent::None;
    }
    let pile_id = pile_id_for(actor);
    let mut inp = GetOrCraftInput::get_or_craft(actor, player_x, player_y)
        .with_craft(craft_if_needed)
        .with_held(held_id)
        .with_pile(pile_id);
    inp.is_moving = is_moving;
    if let Some((tx, ty)) = target {
        inp = inp.with_target(tx, ty);
    }
    let result = get_or_craft_item_ex(objs, &inp, graph, have, blocked);
    get_or_craft_result_to_live_intent(result, empty_drop)
}
