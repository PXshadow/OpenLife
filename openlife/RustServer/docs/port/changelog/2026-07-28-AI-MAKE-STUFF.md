# AI-MAKE-STUFF / make_fire_bake

## Chunk
- **matrix_id:** `AI-MAKE-STUFF`
- **chunk:** `make_fire_bake`
- **mode:** implement
- **Haxe:** `openlife/auto/AiBase.hx` — `makeStuff` → `doBaking(2)` + `makeFireFood(2)`; `makeFireFood` body ~4315–4424; `doBaking`/`doBakingHelper` ~3120–3378
- **Rust:** `ol-sim` `fire_food_profession.rs` + `baker_profession::do_baking` + `shepherd_mid_sites.inc.rs` + `make_stuff_live.inc.rs` + `profession_scan`

## Implemented

1. **`fire_food_profession`** — pure FIREFOODMAKER sticky runtime, hasOrBecome, peer caps, speech `FIREFOOD!`/`FIREFOODMAKER!`
2. **`make_fire_food` pure body** — unskew cooked rabbit/goose; hot-coals cook ladder (mutton/goose/rabbit/pork/beans/kindling/stew); craft Fire when no fireplace; bear/flat-rock; popcorn pure; omelette (Haxe plate-count bug ported); second-fire/stock crafts; weight clear on fallthrough
3. **`make_stuff_try_bodies` / `make_stuff_bake_has_work` / `make_stuff_fire_has_work`** — pure makeStuff order with real bake+fire bodies
4. **`make_stuff_scan_tick` live expand** — `include!("make_stuff_live.inc.rs")` in `profession_scan.rs`; doBaking(2) then makeFireFood(2) via `bake_action_to_live_intent` / `fire_food_action_to_live_intent`
5. **`Player.fire_food_profession`** sticky across ticks; `apply_profession_ladder_tick` clones/writebacks `fire_rt`
6. **lib.rs** pub-use of fire_food + `make_stuff_try_bodies` / has_work / `MAKE_STUFF_FARM_MAX_PEOPLE` / `fire_food_action_to_live_intent`

## Residual
- Dedicated age-rotated / assigned FIREFOODMAKER job rung (`ProfessionScanKind::FireFood`)
- Late doTimeStuffHelper / hungry-path / isHandlingFire makeFireFood(1/2/3) outside makeStuff fallthrough
- `makePopcornIfNeeded` BowlFiller peer pick (pure defaults to self)
- Baker Defer* farm tails inside doBaking during makeStuff
- `ProfessionStickySnapshot` / `peer_count_for_kind` FIREFOOD peer filter
- `fill_fire_food_counts_from_map` hot_coals_is_fire_place always false; popcorn near-player radius

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
cargo test -p ol-sim --lib -- fire_food_profession make_stuff_try_bodies make_stuff_fire_has_work fire_food_profession_sticky ladder_profession_scan
```
