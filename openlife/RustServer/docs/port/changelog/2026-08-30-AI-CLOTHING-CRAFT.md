# AI-CLOTHING-CRAFT — high/medium/low clothing bands

## Haxe

- `craftHighPriorityClothing` ~4492 (loincloth 200, reed skirt 128, sheep skin 593)
- `craftMediumPriorityClothing` ~4510 after `age > 10` (TAILOR + shoes/chest/head)
- `craftLowPriorityClothing` ~4599 (loom dresses/skirts; white/ginger hats; age>30 or assigned TAILOR)
- `fillUpQuiverHelper` GetItem(152) then GetOrCraftItem(151)
- `craftClothIfNeeded` empty slot or `RAG ` name
- doTimeStuffHelper ~684–690 after `craftingTasks`, before assigned jobs

## Rust

| Symbol | Role |
|--------|------|
| `plan_high_priority_clothing` | color-gated bottom/chest |
| `plan_medium_priority_clothing` | quiver then TAILOR list + extras |
| `plan_low_priority_clothing` | loom 2682 r=30 dresses/skirts; 2180/199 hats |
| `plan_clothing_craft_tick` | high; medium if age>10; low if age>30 or assigned TAILOR |
| `GetItemThenGetOrCraft` | GetItem 152 then GetOrCraft 151 |
| `plan_quiver_arrow_precursors` | `craftItemMax` 147/140/149 after GetOrCraft 148 miss |
| Empty Water Pouch 209 | medium after TAILOR, before shoes (`countCurrentObject` / max 1) |
| `PriorityRung::ClothingCraft` | player `apply_clothing_craft_tick` |
| npc think 2a3 | honor `craft_if_needed`; sequential bow seek; arrow precursor fallback |
| `fill_up_quiver_search_radius` | Haxe `fillUpQuiver` 60 vs 20 (`countProfession('TAILOR')` / `age < 20`) |
| player live wrap | `craft_ai.runtime.item.max_search_radius` save/restore + scan radius |
| npc 2a3 | age gate + `count_tailor_profession_from_rows` / `is_last_tailor` |
| `PlayerSnapshot.is_last_tailor` | `last_profession == TAILOR` |

## Residual

`has_tailor: true` hardcoded → **CLOTHING-HAS-TAILOR DONE** (`has_or_become_tailor`).
