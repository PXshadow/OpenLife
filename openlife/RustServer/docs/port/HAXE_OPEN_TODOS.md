# Open TODOs in legacy Haxe code

These are **comments already in the Haxe tree** (`TODO` / `FIXME`), not the Rust port backlog.  
When porting a chunk: either implement the *intended* fix, or port *as-is* and note the choice.

**Counts (approx, 2026-07-26)**

| Area | Matches |
|------|---------|
| `server/GlobalPlayerInstance.hx` | ~81 |
| `server/TimeHelper.hx` | ~62 |
| `server/TransitionHelper.hx` | ~23 |
| `server/Connection.hx` | ~15 |
| `server/WorldMap.hx` | ~11 |
| `server/MoveHelper.hx` | ~11 |
| `server/Lineage.hx` | ~9 |
| other `server/*` | ~20 |
| `auto/AiBase.hx` | ~138 |
| `auto/AiHelper.hx` | ~32 |
| `auto/Pathfinder*` | ~4 |
| `settings/ServerSettings.hx` | ~69 |
| **Total** | **~470** |

Regenerate: search `TODO|FIXME` under `openlife/server`, `auto`, `settings`.

---

## Product-level (root `TODO.MD`)

- AI hire: dynamic prices; command for needed coins  
- Natural springs / tar spots respawn exception  
- Wells / oil decay exception  
- Boat graphics for cars  
- Many gameplay systems described as BETA  

---

## `Connection.hx` (selected)

| Theme | Note |
|-------|------|
| Twins | `// TODO twins` on login — **Rust FERTILITY-TWINS** implements protocol wait queue (product vs Haxe TODO) |
| Full server | limit AIs; score priority; last life length; IP spam block |
| Hold obj | `MakeSureHoldObjId...` race with threads |
| Leader | vanilla client breaks if LEADER out of range |
| DIE score | don’t lower score if `/DIE` |
| VALLEY_SPACING | unclear send |
| ServerAi | call path / access violations |

**Port impact:** spawn queue policy, leader packet range clamp, disconnect scoring.

---

## `GlobalPlayerInstance.hx` (selected themes)

| Theme | Examples |
|-------|----------|
| Diseases / redpoints | yellowFever, dehydration, spicyFood stubs |
| Prestige save | grandkids/parents/siblings prestige not saved |
| Spawn | eve not too far; jungle bananas; deadly animals; noob/noble weights |
| Transitions | Needle/Thread / Bone Needle bugs; tool use Ball of Thread |
| Clothing | store clothes in clothes (backpack while wearing) |
| Death | baby bones in arms; inherit if ally close; coins to kids / grave |
| Combat | stop move on hit; block non-ally move; bloody weapon; anger timing |
| Pickup | allow cloth pickup age; knockout |
| Commands | `!SWITCH` needs client; `!CREATE` |
| Temperature | closest heat object placement |
| Birth design | Arcurus suggestions (curses, nobles, prince) — design debt |

**Port impact:** prioritize food/death/combat/spawn chunks; track Haxe bugs as intentional or fixed.

---

## `TimeHelper.hx` (selected)

| Theme | Note |
|-------|------|
| Slow server | what if too slow (beyond skip tick) |
| Map scan | whole map each sec may not scale |
| Trust/leadership | trust calc, manual leader trust, good graves, protect nobles |
| Contained time | objects in containers / nested |
| Winter/spring | winter decay / spring regrow chances |
| Long-term | decay containers, multi-use, trash pit visible decay |
| Snow/walls | seasonal biome, wall align |
| Spring-only bug | comment: function only called for SPRING so never WINTER path in one place |

**Port impact:** TIME-WORLD / TIME-LONG chunks; fix known seasonal bug when porting.

---

## `TransitionHelper.hx`

See file for multi-use, container slot, horse, reverse-use TODOs (~23). Align with TH-* chunks.

---

## `MoveHelper.hx`

| Theme | Note |
|-------|------|
| Strong half penalty | TODO |
| Horse/car speed | sqrt mult |
| AI forced flag | display not moving |
| Road floor | quadDist /= 10; client desync |
| Exact client pos | reconciliation |

---

## `WorldMap.hx` / `Biome.hx`

- Deep river not walkable  
- Passable ocean biome color  
- Biome experience on eat (unused)  
- Save/load edge cases  

---

## `Lineage.hx` / `PlayerAccount` / `ScoreEntry` / `PlayerSoul`

- Delete/backup unused lineages  
- Archive important deleted  
- Alive/ownsObject only set on start  
- Account id 0 bugs  
- Family name with distance/prestige  
- AI lower score  
- ScoreEntry save to disk  
- Soul chat only from player talking to  

---

## `AiBase.hx` (largest — themes only)

| Theme | Intent |
|-------|--------|
| Mutex / threading | many acquire questions |
| Craft loops | drop/get loops near oven; pile of soil maxuse |
| Counting | home vs current pos double count; blocked objects |
| Profession depth | baker/smith/potter/farmer edge cases |
| Combat | perfect hit chance; steel from towns |
| Containers | drop into container; backpack for nobles |
| Food | popcorn mess; fill bowls without reason |
| Path | closest target with same result; steam/oil wells |
| Player interaction | “not supported yet” |

**Port impact:** do **not** try one-shot port; use AI-JOB-* profession chunks + shared craft engine.

---

## `AiHelper.hx` / Pathfinder

- Craft on table bread  
- Exception handling on closest-object  
- Food list second-best ignore  
- Ally agro / shoot first  
- Pathfinder diagonal / brute force one direction  
- Segfault risk multi-thread (Haxe) — N/A in Rust single writer  

---

## `ServerSettings.hx` (~69 TODOs)

Tunables and incomplete features annotated in-settings. When adding `server.toml` keys, grep the matching field in Haxe and resolve the TODO note.

---

## How to use during a port chunk

1. Open Haxe file for the chunk.  
2. Grep `TODO` within the line range.  
3. In the PR/chunk note, fill:

```markdown
### Haxe TODOs in range
- L1234: ... → port-as-is | implement-intent | defer (reason)
```

4. If implementing intent, add a unit test that locks the *new* behavior.  
5. If deferring, add a row under TODO_PORT “Intentional deltas” or leave in HAXE_OPEN_TODOS.
