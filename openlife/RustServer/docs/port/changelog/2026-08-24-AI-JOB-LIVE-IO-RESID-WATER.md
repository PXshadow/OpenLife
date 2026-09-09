# AI-JOB-LIVE-IO-RESID / doWatering(3) mid basic farm

## Chunk
- **matrix_id:** `AI-JOB-LIVE-IO-RESID` (WaterBringer slice)
- **status:** **PARTIAL** — watering mid wired; cleanUp pileUp + live peer-cap on basic-farm path still open
- **Haxe:** `AiBase.doBasicFarming` ~2395 `doWatering(3)`; `doWatering` / `doWateringHelper` ~3548–3577
- **Rust:** `ol-ai-professions` `farmer_profession::{do_watering_helper, do_watering, WATERING_TARGET_DRY_IDS}`

## Implemented
1. `do_watering_helper` — walk dry targets; skip carrots when `CARROT` stock ≥ 20
2. `do_watering` — `has_or_become_profession(WaterBringer, max)` then helper
3. `do_basic_farming` — call helper after composting (Haxe order before late wheat/corn + sheep)
4. WaterBringer job body → shared helper

## Tests
- `do_watering_helper_prefers_dry_carrots_then_skips_when_stock_high`
- `do_watering_respects_waterbringer_peer_cap`
- `do_watering_helper_is_invoked_after_compost_in_basic_farm_order`

## Residual
- Live `farm_profession_scan_tick` peer-cap before basic mid (optional DeferWatering expand)
- Full `cleanUp` pileUp (stone/straw/corn/skewers/bowls)
- Filtered peer snapshots on all farm hasOrBecome live paths

## Verify
```powershell
cargo test -p ol-ai-professions --lib -- do_watering_
```
