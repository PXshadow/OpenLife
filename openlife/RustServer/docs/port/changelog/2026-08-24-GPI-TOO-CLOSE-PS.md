# GPI-TOO-CLOSE-PS / too-close PS on ranged refuse

**Date:** 2026-08-24  
**Mode:** implement  
**Status:** **DONE** (stabilize USE live PS + finish killHelper HIT/SAY KILL)

## Audit gaps (before)

1. Live USE PS test flaky under load — pending say used process-global / stolen notes; refuse path relied only on static drain.
2. Changelog claimed HIT/SAY KILL wired, but `refuse_ranged_kill_too_close` was not called from live HIT/KILL arms.
3. Pure helpers (`refuse_ranged_*`, `note_too_close_say`) already existed.

## Changes

| Piece | Detail |
|-------|--------|
| `UseResult.ranged_too_close` | Authoritative USE refuse flag; live USE emits PS from result (not static race) |
| Thread-local pending | `LAST_TOO_CLOSE_*` in `thread_local!` Cell |
| `take_too_close_say_for` | Drain only matching conn |
| `maybe_too_close_say_feedback` / `emit_too_close_ps` | Named live drain helpers |
| HIT / SAY KILL | Haxe killHelper L4420–4428: PU + note + PS when bow deadly>1.9 and exact≤1.5 |

## Tests

```powershell
cargo test -p ol-sim --lib -- too_close -- --test-threads=1
```

- `use_refuses_ranged_too_close_to_animal` / `use_refuses_ranged_too_close_emits_ps_say`
- `hit_refuses_ranged_too_close_emits_ps_say` (HIT + SAY KILL arms)
- pure `refuse_ranged_*` / `too_close_say_note_take`

## Residual

- Native protocol `KILL x y [id]` (vs SAY KILL) still combat residual outside this chunk
- Haxe kill TODOs (stop movement / ally mali) unchanged
