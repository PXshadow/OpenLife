# CRAFT-LIVE-IO / useHeldObjOnTarget staging

## Chunk
- **matrix_id:** `CRAFT-LIVE-IO`
- **status:** **DONE** (staging live; Chebyshev vs `isClose` residual → `P0-IS-CLOSE`)
- **Haxe:** `AiBase.useHeldObjOnTarget` (~1391) + `isUsingItem` (~8896–9038)
- **Rust:** `ol-sim` `short_craft_intent` + `PlayerCraftAi.use_held`; `ol-server` npc sticky `pending_use`

## Implemented
1. `UseHeldStaging` / `UseHeldAdvance` — persist expected target parent, held actor parent, drop-in-container
2. `stage_use_held_on_target` — refuse if unreachable or `checkHungryWorkCost`
3. `advance_use_held_staging` — milkweed 50/51/52 keep; target gone/changed Cancel; wrong actor Cancel; actor 0+held DropHeld; contained without drop-in Cancel; moving Wait; far Goto; close UseNow
4. `apply_short_craft_live_intent` UseAt writes staging; far → `Staging(Goto)` not immediate USE
5. `apply_profession_ladder_tick` advances `use_held` **before** new profession
6. npc `NpcStickyMove.pending_use` + `use_actor_parent`; after path finish, USE staged target instead of replanning

## Residual
- Staging/NPC still use Chebyshev `max(|dx|,|dy|)<=1` for UseNow; Haxe `CalculateQuadDistanceToObject > 1` is squared-Euclidean (`isClose` d=1, diagonal fails). Next: **P0-IS-CLOSE**.
- Defer* baker farm tails (unchanged)

## Verify
```powershell
cargo test -p ol-sim --lib -- use_held_staging -- --test-threads=1
cargo check -p ol-server --offline
```
