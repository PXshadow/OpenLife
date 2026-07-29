# AI-FARM-STICKY / basic_farmer_live

## Chunk
- **matrix_id:** `AI-FARM-STICKY`
- **chunk:** `basic_farmer_live`
- **mode:** implement
- **status:** **DONE** (live sticky weight R/W)
- **Haxe:** `openlife/auto/AiBase.hx` — `doBasicFarming` `profession['BASICFARMER']=1` before mid `isSheepHerding(1)`; `=0` when idle after advanced; `doAdvancedFarming(maxProfession)` (2 default / 100 assigned)
- **Rust:** `ol-sim` `farmer_profession` + `profession_scan` + `make_stuff_live` + `Player.farm_profession` + `npc_ai` farm_rt

## Implemented

1. **`basic_farmer_weight_from_runtime`** — read Haxe `profession['BASICFARMER']` (default 1.0 when unset)
2. **`apply_basic_farmer_weight_side_effect`** — pure write for `DeferSheepHerding` (=1) / `ClearBasicFarmerWeight` (=0)
3. **`farm_action_to_live_intent(..., farm_rt)`** — applies sticky side-effect; mid expand: `isSheepHerding(1)` then `do_basic_farming_after_sheep(..., max_profession)`
4. **Live wire** — `apply_profession_ladder_tick` / `apply_profession_scan_tick` seed `ProfessionScanInput.basic_farmer_weight` from `Player.farm_profession` and write runtime back after tick
5. **`make_stuff_scan_tick` + ladder** pass `farm_rt`; **npc_ai** sticky `NpcProfessionState.farm_rt` + weight from_runtime
6. **`do_basic_farming(max_profession)`** — carries outer max on `DeferSheepHerding`; assigned rung → 100, default/makeStuff → 2

## Tests
- `basic_farmer_weight_from_runtime_default_and_sticky`
- `farm_action_defer_sheep_writes_basic_farmer_weight_sticky`
- `farm_profession_input_reads_player_basic_farmer_weight`
- `farm_after_sheep_assigned_max_profession_pass_through`
- `do_basic_farming_mid_defers_sheep_herding` (max 2 / 100)

## Residual
- `doWatering(3)` before mid sheep (WaterBringer body)
- live `has_or_become_profession` peer-cap (scan still uses `has_profession` bool)
- npc sticky lives on `NpcProfessionState.farm_rt` (PlayerSnapshot has no farm weights)

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib farm_
cargo test -p ol-sim --lib basic_farmer_weight
```
