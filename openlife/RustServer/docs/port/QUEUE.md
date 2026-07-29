# Server port workflow queue

## Concurrency policy (active)

**Default max: 2 concurrent `haxe-port-chunk` workflows.**

3rd only if different lanes. Prefer not stacking two heavy `ol-sim` Acts.

**Out of scope:** PHOTO, VOG, multi-server twins.

## In flight (≤2)

| # | matrix_id | chunk | workflow | lane |
|---|-----------|-------|----------|------|
| 1 | `MOSQUITO-MAPCHANCE` | mosquito_mapchance_swamp | haxe-port-chunk (relaunch; prior cancelled@Resolve) | A |

## Resume queue

| # | matrix_id | About | lane |
|---|-----------|--------|------|
| 0 | **`OL-AI-SPLIT` P3** | Move pure profession/craft AI modules into `ol-ai` (main-thread arch; not auto-refill) | arch |
| 1 | `AI-CRAFT-MULTI-SPECIALS` | GetCraftAndDrop / WaterSourceIds residual | A |
| — | ~~TWIN-MULTI-SERVER~~ | **Parked** | — |
| — | ~~PHOTO / VOG~~ | **Out of scope** | — |

## Done recently

**OL-AI-SPLIT P1–P2** · **TH-ALT-OUTCOME** · **CURSED-GRAVE-TELEPORT** · **CLOTHING-CONTAIN-SIZE** · **AI-CRAFT-NPC-ENQUEUE** · multi-server **parked**
