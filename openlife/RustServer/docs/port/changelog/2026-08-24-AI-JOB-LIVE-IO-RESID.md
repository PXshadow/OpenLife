# AI-JOB-LIVE-IO-RESID / WaterBringer mid doWatering(3)

## Chunk
- **matrix_id:** `AI-JOB-LIVE-IO-RESID`
- **status:** **DONE core** — WaterBringer mid + farm peer-cap + cleanUpBowls; hungry-cost elsewhere
- **Haxe:** `AiBase.doBasicFarming` ~2395 `doWatering(3)` before wheat(6,12) / mid sheep
- **Rust:** `ol-ai-professions` `do_watering` / `do_basic_farming_ex` + live farm/baker/makeStuff expand

## Implemented
1. `BASIC_FARM_MID_WATER_MAX_PEOPLE = 3`
2. `do_basic_farming` → `do_basic_farming_ex(..., watering: None)` (helper-only mid water)
3. `do_basic_farming_ex(..., Some((rt, peer, idle)))` → `do_watering` peer-cap then helper
4. Live: `farm_profession_scan_tick` BasicFarmer path, baker `DeferFarm`, `make_stuff_live` pass WaterBringer peer ctx
5. Mid-sheep fixture uses wet planted caps so watering has no targets (Haxe waters dry first)

## Tests
- `do_basic_farming_mid_defers_sheep_herding`
- `do_basic_farming_ex_mid_watering_peer_cap_falls_through_to_sheep`
- `do_watering_respects_waterbringer_peer_cap`
- `do_watering_helper_*`

## Residual
- Hungry-cost pair / live AI notReachable map (**CRAFT-LIVE-IO**)

## Verify
```powershell
cargo test -p ol-ai-professions --lib -- do_basic_farming_mid do_watering do_basic_farming_ex -- --test-threads=1
cargo test -p ol-sim --lib -- baker_defer_farm farm_action_defer_sheep -- --test-threads=1
```
