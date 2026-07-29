# AI-SHEPHERD-MID / sheep_mid_sites

## Chunk
- **matrix_id:** `AI-SHEPHERD-MID`
- **chunk:** `sheep_mid_sites`
- **mode:** implement
- **Haxe:** `openlife/auto/AiBase.hx` — `doBasicFarming` mid `isSheepHerding(1)` + after tail; `makeStuff`; `doComposting` wet 625; `makeSharpieFood`
- **Rust:** `ol-sim` shepherd + farmer + profession_scan + ai_goals

## Implemented

1. **`FarmAction::DeferSheepHerding { max_profession }`** — mid site; sticky BASICFARMER=1 via `apply_basic_farmer_weight_side_effect`
2. **`do_basic_farming_after_sheep(counts, task, age, max_profession)`** — late plant wheat(15,30)/corn(8,12) → age&lt;20 `make_sharpie_food` → `DeferAdvancedFarming`
3. **`make_sharpie_food`** — pure Seeding Wild Carrot 36 / Burdock 804 + Sharp Stone 34 → dug products
4. **`do_advanced_farming` / `expand_advanced_farming_or_clear`** — rows + advanced plant step; else `ClearBasicFarmerWeight`
5. **`make_stuff_ordered` / `MakeStuffInputs` / `make_stuff_try`** — full Haxe order: sharpie → bake → farm → sheep → fire
6. **`make_stuff_scan_tick`** — live sequential makeStuff expand; hooked from `ladder_profession_scan_tick` on LowPriority/AgeRotated/MidPriority fallthrough
7. **`do_composting` wet-compost 625 recount** — stock_with_wet double-map + held (Haxe countCurrentObject + CountCloseObjects)
8. **`Profession::Shepherd` + `SHEPHERD_TARGET_ID` (575) + `pick_shepherd_goal`**
9. Tests: mid defer sticky, after_sheep sharpie/advanced/clear, make_sharpie, make_stuff order/try, wet625

## Residual
- makeFireFood / doBaking bodies inside makeStuff chain
- live Player.farm_profession BASICFARMER weight write on scan tick (pure helper only)
- assigned doBasicFarming(100) advanced max pass-through (default 2)
- doWatering(3) before mid sheep (deferred WaterBringer)
- nested milk buckets / skim-milk shortCrafts
- npc_ai multi-profession scan polish when farm mid sheep and shepherd scan co-exist

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
cargo test -p ol-sim --lib -- shepherd_profession farmer_profession::tests::do_basic_farming farmer_profession::tests::make_sharpie farmer_profession::tests::do_composting
```
