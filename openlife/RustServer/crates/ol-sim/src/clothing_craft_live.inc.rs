// AI-JOB-TAILOR: live scan helpers included from profession_scan.rs

/// Assigned/last TAILOR: high then medium(100) then low(100) → craft intent.
// Haxe: assignedProfession == 'TAILOR' || lastProfession == 'TAILOR' ~749–752
pub fn tailor_profession_scan_tick(
    tiles: &[ScanTile],
    inp: &ProfessionScanInput,
    rung_label: &str,
) -> ProfessionScanTickResult {
    use crate::clothing_craft::{
        add_home_cloth_id, has_or_become_tailor, plan_assigned_tailor_clothing,
        tailor_max_for_dispatch, ClothingCraftInput, HomeClothStock, LOOM,
    };
    let assigned = inp.assigned_tailor
        || inp.last_is_tailor
        || inp.is_assigned_job
        || rung_label == "ASSIGNED_JOB";
    let max = tailor_max_for_dispatch(assigned, rung_label);
    let was_idle = if inp.profession_is_sticky || inp.last_is_tailor {
        0.0
    } else {
        inp.was_idle
    };
    let has_tailor = has_or_become_tailor(inp.last_is_tailor, max, inp.peer_count, was_idle);
    let mut rag = [false; 6];
    if let Some(ref content) = inp.content {
        for i in 0..6 {
            let id = inp.clothing[i];
            if id <= 0 {
                continue;
            }
            if let Some(d) = content.get(id) {
                let n = d.name.to_ascii_uppercase();
                let desc = d.description.to_ascii_uppercase();
                rag[i] = n.contains("RAG ") || desc.contains("RAG ");
            }
        }
    }
    let mut home_stock = HomeClothStock::default();
    let mut has_loom = false;
    for t in tiles {
        if t.parent_id == LOOM {
            has_loom = true;
        }
        add_home_cloth_id(&mut home_stock, t.parent_id);
    }
    let cinp = ClothingCraftInput {
        color: inp.person_color,
        clothing_ids: &inp.clothing,
        rag: &rag,
        age: inp.age,
        has_tailor,
        bow_old_enough: inp.age >= 3.0,
        held_id: inp.held_id,
        quiver_can_add: quiver_can_add_from_slots(&inp.clothing, &inp.clothing_uses),
        home_stock,
        female: inp.looks_female,
        has_loom,
        assigned_tailor: assigned,
    };
    let Some(plan) = plan_assigned_tailor_clothing(&cinp) else {
        return ProfessionScanTickResult::none();
    };
    let intent = clothing_plan_to_live_intent(plan, tiles);
    if matches!(intent, ShortCraftLiveIntent::None) {
        ProfessionScanTickResult::none()
    } else {
        ProfessionScanTickResult {
            intent,
            had_action: true,
        }
    }
}

fn clothing_plan_to_live_intent(
    plan: crate::ClothingCraftPlan,
    tiles: &[ScanTile],
) -> ShortCraftLiveIntent {
    use crate::ClothingCraftPlan;
    match plan {
        ClothingCraftPlan::SelfClothing { slot } => ShortCraftLiveIntent::SelfClothing { slot },
        ClothingCraftPlan::GetOrCraft {
            object_id,
            craft_if_needed,
        } => ShortCraftLiveIntent::SeekOrCraft {
            actor: object_id,
            craft_if_needed,
        },
        ClothingCraftPlan::GetItemThenGetOrCraft {
            get_id,
            fallback_id,
        } => {
            if tiles.iter().any(|t| t.parent_id == get_id) {
                ShortCraftLiveIntent::SeekOrCraft {
                    actor: get_id,
                    craft_if_needed: false,
                }
            } else {
                ShortCraftLiveIntent::SeekOrCraft {
                    actor: fallback_id,
                    craft_if_needed: true,
                }
            }
        }
        ClothingCraftPlan::CraftItem(object_id) => ShortCraftLiveIntent::CraftItem { object_id },
    }
}
