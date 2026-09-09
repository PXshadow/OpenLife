# Server port workflow queue

**Next:** **IDLE-COMPLETE** (non-AI resume table empty). Skip further AiBase rungs until this table is empty. Skip SETTINGS-LONG-TAIL / unused ModuleConst / PHOTO / VOG / multi-server twins / SQL.

**Changelog (2026-09-09):** **SETTINGS-KNOB-TAIL** live. **STARTING-GX-DEATH** live. **NESTED-PERSIST-SER** done (OLW3 NestedHelper already roundtrips). **SCORE-AGE-58** live. **OWNED-GATE-DELETE** live. **EVE-DEADLY-ANIMALS** live. **WELLS-OIL-DECAY** live. **SPRINGS-TAR-RESPAWN** live. **AGE-10-FATHER-LIVE** live. **LOCATION-SAYS-MARKERS** live. **NAME-FULL-LINEAGE** live. **CONTENT-OBJ-SPAWN** / **CONTENT-OBJ-DECAY-TAIL** / **CONTENT-TRANS-INSERT** / **CONTENT-TRANS-TIME** / **CONTENT-OBJ-FLOOR-TAGS** live. **RECENT-EXILE-ALLY** live. **BABY-BONES-ARMS** live. **HIT-BLOCK-NONALLY-MOVE** live. **HIT-STOP-MOVE** live. **HALF-PENALTY-STRONG** live. **VANILLA-ID-MAP** live. **PRESTIGE-FAMILY-PERSIST** live (OLN15). **VALLEY-SPACING** dropped (Open Life does not send `VS`). **SPAWN-QUEUE-POLICY** live. **PULL-CARROT-ROW** live. **DIE-SCORE-SKIP** live.

## Concurrency policy (active)

**Worker + 60s watchdog.** The worker is one continued conversation (`resume_from`). The watchdog only **resumes if stuck** and SuperGrok used % is **below** `resume_below_percent` in `%USERPROFILE%\.grok\openlife-port-watchdog\watchdog.toml` (default **50**). That query script is **outside this git repo**. No full-repo rescan. No tool cap.

| Role | ID | Interval | Job |
|------|----|----------|-----|
| Watchdog | local `watchdog-loop.ps1` | **60s, no Grok tokens** | `DONE` only if stuck + usage OK |
| Worker | none (IDLE-COMPLETE) | — | — |

**Stuck** = In flight **WORKER** is not `RUNNING`, or heartbeat older than **45 minutes**, and Next is not IDLE-COMPLETE.

**Usage gate (not in this git repo):** `%USERPROFILE%\.grok\openlife-port-watchdog\watchdog-usage-gate.ps1`. Edit `watchdog.toml` there for the percent.

When in-scope work is empty, mark **IDLE-COMPLETE**.

**Out of scope:** PHOTO, VOG, multi-server twins, SQL, mutex/debug knobs, unused ModuleConst, further AiBase rungs until this table is empty.

---

## In flight

**WORKER:** none  
**State:** IDLE-COMPLETE  
**Heartbeat:** 2026-09-09T15:04:00Z

Token-free watchdog: `%USERPROFILE%\.grok\openlife-port-watchdog\watchdog-loop.ps1`. Grok scheduler is **off**.

---

## Resume queue

**Pick order:** **only** this non-AI list. Verify with grep; skip if already live. Do **not** pick AiBase profession/fill/craft rungs until this table is empty.

| # | matrix_id | About | Evidence still open |
|---|-----------|--------|---------------------|

---

## Done recently

**SETTINGS-KNOB-TAIL** 2026-09-09. **STARTING-GX-DEATH** 2026-09-09. **NESTED-PERSIST-SER** 2026-09-09 (already OLW3). **SCORE-AGE-58** 2026-09-09. **OWNED-GATE-DELETE** 2026-09-09. **EVE-DEADLY-ANIMALS** 2026-09-09. **WELLS-OIL-DECAY** 2026-09-09. **SPRINGS-TAR-RESPAWN** 2026-09-09. **AGE-10-FATHER-LIVE** 2026-09-09. **LOCATION-SAYS-MARKERS** 2026-09-09. **NAME-FULL-LINEAGE** 2026-09-09. **RECENT-EXILE-ALLY** 2026-09-09. **CONTENT-OBJ-SPAWN** / **CONTENT-OBJ-DECAY-TAIL** / **CONTENT-TRANS-INSERT** / **CONTENT-TRANS-TIME** / **CONTENT-OBJ-FLOOR-TAGS** 2026-09-09 (`object_data_remainder.inc.rs` / `transitions_remainder.inc.rs`; extra fields live: reducesLongingFor / higherQaulityFood / prestigeClass / blocksDomesticAnimal / objectIdArrays[455]. LimitTransitions / unreleased 4647 / trans.tool also live). **BABY-BONES-ARMS** 2026-09-09. **HIT-BLOCK-NONALLY-MOVE** 2026-09-09. **HIT-STOP-MOVE** 2026-09-09. **HALF-PENALTY-STRONG** 2026-09-09. **VANILLA-ID-MAP** 2026-09-09. **PRESTIGE-FAMILY-PERSIST** 2026-09-09. **VALLEY-SPACING** skipped (unsupported). **SPAWN-QUEUE-POLICY** 2026-09-09. **PULL-CARROT-ROW** 2026-09-09. **DIE-SCORE-SKIP** 2026-09-09. **FILL-BERRY-HELD** 2026-09-08.
