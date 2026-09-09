# 2026-09-05 AI-HANDLE-DEATH

**status:** **DONE**

Haxe `AiBase.handleDeath` (doTimeStuffHelper before eat, after wound). Age ≥ `ServerSettings.MaxAge - 2` (60−2=58, not vitals 120) → wipe profession map, `lastProfession = GRAVEKEEPER`, then:

- busy if using/moving
- 5% `Good bye!` / 5% Jasoniah say
- home quad &lt; 400 → `isHandlingGraves`
- else `isMovingToHome(5)` (firePlace else home)
- else `time += 2` + `dropHeldObject(0)`

- `should_handle_death` sensor fill (live `GameplayKnobs.max_age`)
- `handle_death.rs` pure plan + wipe
- Live `apply_handle_death_tick` + npc before eat (`deadlyPlayer == null`)

Residual: `isRemovingFromContainer` still sensor-only (no SM).

Tests: `handle_death::*` / `apply_handle_death_tick_*` / `handle_death_age_gate_and_rung`

`cargo check -p ol-server --offline` ok.
