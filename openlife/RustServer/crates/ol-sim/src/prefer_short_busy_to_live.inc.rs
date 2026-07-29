// PREFER-SHORT-WAIT: DropHeldDecision::to_live_intent body
// Haxe: dropTarget → DROP; useTarget → USE; gotoObj → walk; self → SELF clothing
// Haxe: isMoving return true (BusyMoving → Wait)

/// Map wire-capable decisions to [`ShortCraftLiveIntent`].
///
/// Prefer [`resolve_prefer_short_craft`] / [`plan_drop_held_live`] first so
/// PreferShortCraft becomes UseAt when target is in scan. Unresolved
/// PreferShortCraft keeps `craft_actor` as SeekOrCraft craft_if_needed.
/// BusyMoving → Wait (hold tick; Haxe isMoving return true).
pub fn drop_held_decision_to_live_intent(d: DropHeldDecision) -> ShortCraftLiveIntent {
    match d {
        DropHeldDecision::UseAt {
            x,
            y,
            target_id,
            actor_id,
        }
        | DropHeldDecision::UseAsDrop {
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
        DropHeldDecision::DropAt { x, y } => ShortCraftLiveIntent::DropAt { x, y },
        // Haxe: myPlayer.gotoObj(target) while dropOnStart — walk, not DROP
        DropHeldDecision::Goto { x, y } => ShortCraftLiveIntent::Goto { x, y },
        // Haxe: myPlayer.self(0, 0, 5) quiver store
        DropHeldDecision::SelfClothing { slot } => ShortCraftLiveIntent::SelfClothing { slot },
        // Haxe: shortCraft(actor, target, …, craftActor) when target not tile-resolved
        DropHeldDecision::PreferShortCraft {
            actor,
            craft_actor,
            ..
        } => ShortCraftLiveIntent::SeekOrCraft {
            actor,
            craft_if_needed: craft_actor,
        },
        // Haxe: if (myPlayer.isMoving()) return true — hold tick, no fallthrough
        DropHeldDecision::BusyMoving => ShortCraftLiveIntent::Wait,
        DropHeldDecision::None | DropHeldDecision::RefuseWound => ShortCraftLiveIntent::None,
    }
}
