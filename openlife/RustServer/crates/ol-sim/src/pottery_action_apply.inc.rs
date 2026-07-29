/// Map shortCraft pottery action for live USE/DROP (smith apply family).
// Haxe: shortCraft / shortCraftOnGround
pub fn pottery_action_short_craft_apply(
    action: PotteryAction,
    held_id: i32,
) -> crate::smith_profession::SmithApply {
    use crate::smith_profession::SmithApply;
    match action {
        PotteryAction::ShortCraft { actor, target } => {
            if actor == 0 {
                if held_id != 0 {
                    SmithApply::DropHeld
                } else {
                    SmithApply::UseOnTarget { actor: 0, target }
                }
            } else if held_id == actor {
                SmithApply::UseOnTarget { actor, target }
            } else {
                SmithApply::SeekOrCraftActor { actor }
            }
        }
        PotteryAction::ShortCraftOnGround { target } => {
            crate::smith_profession::short_craft_on_ground_apply(held_id, target)
        }
        PotteryAction::CraftItem { object_id } => SmithApply::CraftItem { object_id },
        PotteryAction::SeekOrCraft { object_id } => SmithApply::SeekOrCraftActor { actor: object_id },
        PotteryAction::DropHeld { .. } => SmithApply::DropHeld,
        PotteryAction::Abort => SmithApply::Abort,
        _ => SmithApply::None,
    }
}
