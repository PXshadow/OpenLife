# JUMP-BW-FULL / jump_bw_full (2026-07-29)

**Mode:** implement  
**Status:** **DONE** (core) — pure + live JUMP/AI/MOVE wired; residual dropPlayer transform polish

## Haxe scope

| Symbol | File | Behavior |
|--------|------|----------|
| `GlobalPlayerInstance.jump` | `server/GlobalPlayerInstance.hx` L5098–5120 | Not held → self PU + `sendWiggle` (BW) + FRAME; held → `heldByPlayer.dropPlayer` |
| `MoveHelper.JumpToNonBlocked` | `server/MoveHelper.hx` L473–519 | If standing blocked: try E/S/W/N; VOG + map + forced PU + waitForForce; abort MOVE |
| `MoveHelper.moveHelper` jump | L546–624 | held `jump()`; JumpToNonBlocked; rate `MaxJumpsPerTenSec`; `exhaustion`/`jumpedTiles`; say exhausted |
| Client `JUMP` | `Server.hx` L248–249 | ignore x/y; call `player.jump()` |
| AI `JUMP!` | `auto/AiBase.hx` L4837–4840 | say JUMP + `myPlayer.jump()` |
| `forceStopOnNextTile` | `MoveHelper.updateMovement` L335–338 | cancel after next tile (already DONE) |

## Rust surfaces

| Piece | Path | Role |
|-------|------|------|
| pure | `ol-sim/src/jump_bw.rs` | `plan_player_jump`, `plan_jump_to_non_blocked`, `JUMP_EXHAUSTED_SAY`, BW always-on |
| pure (existing) | `ol-sim/src/move_path.rs` | `jump_rate_limited`, `apply_jump_cost(_ex)`, `decay_jumped_tiles`, `jump_quad_with_floor`, forceStop advance |
| live | `ol-sim/src/lib.rs` | `apply_player_jump`, `try_jump_to_non_blocked`, JUMP tag, AI `plan.jump`, MOVE JumpToNonBlocked + exhausted say |
| wire | `format_baby_wiggle` / BW | protocol BABY_WIGGLE |

## Gaps closed

1. **mod jump_bw** declared + re-exported from `lib.rs`
2. **AI JUMP!** residual was only `done_moving_seq` bump → full `apply_player_jump` (PU+BW / drop)
3. **Client JUMP** teleported on x/y → ignore coords (Haxe); always BW on not-held (not age-gated only)
4. **JumpToNonBlocked** on MOVE start when standing blocked → abort with force
5. **Exhausted jump say** `"I am too exhausted!"` after accepting client snap with `is_exhausted`
6. Tests: pure plan order E/S/W/N; `jump_emits_pu_note` keeps tile; adult BW; held drop; JumpToNonBlocked live; exhausted PS

## Already DONE (not re-ported)

- jump rate limit / exhaustion cost / jumpedTiles decay / floor softener / waitForForce
- forceStopOnNextTile cancel with VOG
- MOVE jump too-far / blocked-start CancleMovement + VOG threshold 25
- baby MOVE drop out of arms (now shares `apply_player_jump`)

## Tests

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- jump_ -- --test-threads=1
cargo test -p ol-sim --lib -- force_stop_on_next_tile_cancels -- --test-threads=1
```

## Residual

- Full `dropPlayerHelper` transformX/Y + dual close PU force fields (current release_holding + position at carrier is close)
- TimeHelper-style *periodic* JumpToNonBlocked when player stands blocked mid-tick (MOVE-start path covers MOVE entry)
- Haxe softener TODOs kept: floorId>0 quadDist/=10; exhaustion humans-only
