# GPI-TOO-CLOSE / too_close_say_ps

**Date:** 2026-07-29  
**Mode:** implement  
**Status:** **DONE** (USE animal + killHelper player-target bow min-range public say/PS)

## Scope

### USE (animal)

Haxe `TransitionHelper.use` L757–765:

```haxe
// can only shoot at target with bow if not too close
if (deadlyDistance > 1.9 && this.target.isAnimal()
    && player.isCloseUseExact(this.target.tx, this.target.ty, 1.5)) {
    player.say('Too close...');   // public → sayHelper uppercases
    player.message = 'too close'; // debug refuse reason (not wire)
    return false;
}
```

### Kill (player target)

Haxe `GlobalPlayerInstance.killHelper` L4420–4428:

```haxe
// can only shoot at target with bow if not too close
if (deadlyDistance > 1.9 && isCloseToPlayerUseExact(targetPlayer, 1.5)) {
    this.connection.send(PLAYER_UPDATE, [this.toData()]);
    this.say('Too close...');
    return false;
}
```

No animal check on kill path — player targets only.

`say('Too close...')` (no `toSelf`) → `GlobalPlayerInstance.sayHelper` uppercases → `PLAYER_SAYS` + FRAME to speaker and nearby.

## Rust

| Piece | Location |
|-------|----------|
| Pure kill refuse | `refuse_ranged_kill_too_close` — `ol-sim/move_live_gates.rs` |
| Pure USE refuse | `refuse_ranged_use_too_close` (= kill core + animal gate) |
| Constants | `RANGED_DEADLY_DISTANCE_THRESHOLD` (1.9), `RANGED_MIN_USE_DISTANCE` (1.5), `TOO_CLOSE_SAY` (`TOO CLOSE...`), `TOO_CLOSE_MESSAGE` (`too close`) |
| Pending flag | `note_too_close_say` / `take_too_close_say` / `take_too_close_message` / `clear_too_close_pending` |
| USE refuse | `use_transition::apply_use_at` notes flag when refuse |
| HIT / SAY KILL | `lib.rs` SAY `HIT` + `KILL` arms → note + PU/FM + `maybe_too_close_say_feedback` |
| Live drain | `maybe_too_close_say_feedback` → `send_chat_ps` public PS + FM (age-scaled chat range) |

## Tests

- `refuse_ranged_use_too_close_bow_animal`
- `refuse_ranged_kill_too_close_bow_player`
- `too_close_say_note_take` (say + debug message)
- `use_refuses_ranged_too_close_to_animal` (notes flag + message)
- `use_refuses_ranged_too_close_emits_ps_say` (live `NetIntent::Use` → PS `TOO CLOSE...` + FRAME)
- `hit_refuses_ranged_too_close_emits_ps_say` (live SAY HIT bow → PS + no wound; far HIT connects)

## Residual

- ~~Adult chat fan-out 24 vs CloseForSay 20~~ → **PO-MAX-DISTANCE DONE** (`ADULT_CHAT_RANGE`=20)
- Haxe kill TODOs (stop movement if hit / block movement if not ally) remain combat residuals outside this chunk

## Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- too_close -- --test-threads=1
cargo test -p ol-sim --lib -- hit_refuses_ranged_too_close -- --test-threads=1
```
