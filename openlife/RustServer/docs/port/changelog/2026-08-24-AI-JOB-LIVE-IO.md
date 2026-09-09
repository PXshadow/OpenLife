# AI-JOB-LIVE-IO / baker Defer* live expand

## Chunk
- **matrix_id:** `AI-JOB-LIVE-IO`
- **status:** **PARTIAL → baker Defer* DONE** (core expand); residuals listed below
- **Haxe:** `AiBase.doBakingHelper` ~3314–3376 (`doPlantCarrots`, `doHarvestWheat`, `doPlantWheat`, `doPlantBeans`, `fillBerryBowlIfNeeded`, `makeSeatsAndCleanUp`, `cleanUp`)
- **Rust:** `ol-sim` `profession_scan::expand_baker_defer_live` + `bake_action_to_live_intent`

## Implemented
1. `BakeAction::DeferPlantCarrots` → `do_plant_carrots(2,10)` → `farm_action_to_live_intent`
2. `DeferHarvestWheat` → `do_harvest_wheat(1,4)`
3. `DeferPlantWheat` → `do_plant_wheat(2,5)`
4. `DeferPlantBeans` → `do_plant` dry beans (2,4)
5. `DeferFarm` → `do_basic_farming(..., max=2)` (generic handoff)
6. `DeferBerryBowl` → re-eval `fill_berry_bowl_if_needed` / seek bush CraftItem
7. `DeferSeatsCleanup` → `make_seats_and_cleanup_ex(force, bowl_filler)`
8. `DeferCleanup` → wet-nozzle cleanup + pottery scan tick fallback

## Tests
- `baker_defer_farm_had_action_expands_or_stages`
- `baker_defer_plant_carrots_expands_craft_item`

## Residual (`AI-JOB-LIVE-IO-RESID`)
- Full Haxe `cleanUp` pileUp live wire (pure `cleanup_profession` started)
- `hasOrBecomeProfession` peer-cap live on farm scan (`has_profession` bool)
- ~~`doWatering(3)` before mid sheep~~ → **DONE** (`do_basic_farming_ex` + live ctx)
- Hungry-cost pair / live AI notReachable map

## Verify
```powershell
cargo test -p ol-sim --lib -- baker_defer_
```
