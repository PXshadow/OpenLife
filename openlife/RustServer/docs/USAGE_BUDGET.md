# Usage budget log

## Policy

- **Source of truth:** Grok Build slash command **`/usage`** (weekly SuperGrok pool).
- **Hard planning cap:** 80% of allowance until refresh.
- **Pacing:** cumulative soft ceiling = `80% × (day_index / days_until_refresh)`.
- Do not implement past today's soft ceiling without explicit user approval.
- Session context (`/context`) is separate from subscription budget.
- **Automation:** after `/usage`, run `.\scripts\record-usage.ps1`; gate with `.\scripts\check-budget.ps1`.
  See `docs/USAGE_AUTOMATION.md`.

## Setup (fill once)

| Field | Value |
|-------|--------|
| Refresh date (next) | _TBD — set when known_ |
| Days in period (D) | _TBD_ |
| Cap | 80% |
| Daily slice | `80% / D` |

## Formula reminder

```
soft_ceiling(day t) = 80% * (t / D)

Example D=10:
  day 1 → 8%
  day 2 → 16%
  day 10 → 80%
```

## Log

| Date | Day t/D | Soft ceiling (cumul.) | `/usage` note | Est. used this session | Cumul. est. | Work done |
|------|---------|----------------------|---------------|------------------------|-------------|-----------|
| 2026-07-21 | — | — | bootstrap offline | 0% (repo scaffold by agent in prior tree; new repo) | 0% | Created clean OpenLifeReborn git project |
| 2026-07-21 | 1/7 | ~11.4% | live fetch `used_percent=2` GrokBuild | ~0–1% | **2%** | Pure modules: reputation, mute, version_gate (+ tests); ol-sim lib 481 green |
| 2026-07-22 | 1/7 | ~11.4% | live fetch `used_percent=10` GrokBuild | ~1% (from ~9%) | **10%** | Full design PR1–PR11: catch-up ticks, MovePath+PM (flag off), USE gates, self-play, OpsSeries+/ops, birth fitness 42, NPC scheduler; workspace lib green ~648 ol-sim |

_Add a row after each Grok Build work session. Always re-check `/usage` live._

## Session checklist

- [ ] Ran `/usage` at start
- [ ] Compared to soft ceiling for today
- [ ] Single scoped goal
- [ ] `cargo test` after changes
- [ ] Logged row above
- [ ] Ran `/usage` if session was long
| 2026-07-21 12:55 UTC | - | - | slash-usage used=10% | 10% | 10% | record-usage.ps1 |
| 2026-07-21 ~18:50 UTC | 7/7 | 80% | live used=16% | docs+test+selfplay | 16% | cargo test --workspace --lib 370 pass; selfplay12 boot_ms=13; PROGRESS+BACKLOG comprehensive (no Haxe parity claim) |
| 2026-07-21 ~19:05 UTC | — | — | **weekly reset** period_start=2026-07-21T19:05:47Z → reset 2026-07-28 | — | ~0% (new period) | SuperGrok pool rolled; do not use pre-reset 16–17% |
| 2026-07-21 ~19:13 UTC | 1/7 | ~11% soft | used_percent=null (billing snapshot) | admin_env export + full test + selfplay8 | ~low / unknown | cargo test --workspace **437 pass**; selfplay8 **boot_ms=27**; all agents moved; PROGRESS budget honesty |
| 2026-07-21 ~19:42 UTC | 1/7 | ~11% soft | start used=**1%** → end used=**2%** (live) | pure `afk` + `death_cause`; cargo test + selfplay9 | ~1% of allowance | workspace **517 pass** (sim 449); selfplay9 **boot_ms=377**; agents moved; PROGRESS feature inventory; **no Haxe complete claim** |
| 2026-07-21 ~19:37 UTC | 1/7 | ~11% soft | live used=**1%** (Api product null; GrokBuild 1.0); post-reset null→1 | docs PROGRESS+BACKLOG + lib tests + selfplay5 | ~1% | cargo test --workspace --lib **482 pass**; selfplay5 **boot_ms=19**; all agents moved |
| 2026-07-21 ~19:57 UTC | 1/7 | ~11.43% soft | live used=**3%** (GrokBuild 3.0) | SAY NOTE/?NOTES personal journal + tests + selfplay8s | ~3% | workspace lib **563 pass** (sim 498); selfplay8s **boot_ms=16**; all agents moved; PROGRESS budget live % |
| 2026-07-21 ~20:08 UTC | 1/7 | ~11.43% soft | live used=**3%** (GrokBuild 3.0) | pure `biome_colors` + `object_tags` + `wire_fields`; workspace test + selfplay10s | ~3% | workspace lib **595 pass** (sim **528**); selfplay10s **boot_ms=16**; all agents moved; PROGRESS updated |
| 2026-07-21 ~20:20 UTC | 1/7 | ~11.43% soft | live used=**4%** (GrokBuild 4.0) | SAY RECIPE/NEXTCRAFT + craft-plan self-play + workspace lib + selfplay8n | ~4% | workspace lib **613 pass** (sim **545**); selfplay8n **boot_ms=15**; farmer SeekObject(242); craft products=3052 edges=7555 |
| 2026-07-21 ~20:36 UTC | 1/7 | ~11.43% soft | start used=**4%** → end used=**5%** (GrokBuild 5.0) | pure OHOL helpers ×5 + SAY HEAT/DAY/AGECURVE/WRAP + workspace + selfplay12 | ~1% of allowance | workspace lib **664 pass** (sim **596**); pure module tests 41; selfplay12 **boot_ms=37** all agents moved |
