/// Build [`GetOrCraftWorldObj`] list from `(parent_id, x, y)` + optional slots table.
pub fn world_objs_from_ids(
    items: &[(i32, i32, i32)],
    num_slots_for: Option<&dyn Fn(i32) -> i32>,
) -> Vec<GetOrCraftWorldObj> {
    items
        .iter()
        .map(|&(id, x, y)| {
            let slots = num_slots_for.map(|f| f(id)).unwrap_or(0);
            GetOrCraftWorldObj::simple(id, x, y).with_slots(slots)
        })
        .collect()
}

/// Resolve SeekOrCraft/CraftItem against world, then apply wire USE/DROP when possible.
///
/// Non-wire residual stays as [`ShortCraftLiveApplyResult::Staging`].
// Haxe: GetOrCraftItem staging → use / drop this tick
pub fn apply_resolved_seek_or_craft(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
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
) -> ShortCraftLiveApplyResult {
    let resolved = resolve_seek_or_craft_live(
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
    );
    apply_short_craft_live_intent(state, outbound, conn_id, resolved)
}

// ── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    include!("get_or_craft_tests.inc.rs");
}
