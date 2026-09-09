# AI-HUNGRY-EMOTE — hungry-work PE + DeferPottery seek

## Haxe

`TransitionHelper.hx` ~1211–1253 (`hungryWorkCost > 0`):

- Allow: `player.doEmote(Emote.biomeRelief)` then `sendFoodUpdate()`
- Refuse exhaustion: `say('Too exhausted! $excess')` + `doEmote(Emote.homesick)`
- Refuse food: `say('Need ${missingFood} more food!')` + `doEmote(Emote.homesick)`

Smith `prepareSmithingTools` DeferPottery → `Goal::SeekObject(FIRING_KILN)`.
Baker `BakeAction::DeferPottery` → `Goal::SeekObject(CLAY_PLATE)`.

## Rust

| Symbol | Role |
|--------|------|
| `HUNGRY_WORK_RELIEF_EMOTE` / `HUNGRY_WORK_HOMESICK_EMOTE` | 19 / 28 |
| `note_hungry_work_emote` / `take_hungry_work_emote` | Mutex pending PE (conn, index) |
| `apply_use_at` hungry-work match | Allow → 19; RefuseExhaustion / RefuseFood → 28 (keep `note_lock_say`) |
| `maybe_hungry_work_emote_feedback` | USE after `apply_use_at`: PE nearby + FRAME |
| success FX | still `packets_after_use` → `food_change_for_player` (no duplicate sendFoodUpdate) |
| `smith_apply_to_live_intent(DeferPottery)` | `SeekOrCraft { actor: FIRING_KILN, craft_if_needed: false }` |
| baker empty `pottery_profession_scan_tick` | `SeekOrCraft { actor: CLAY_PLATE, craft_if_needed: false }` |

`ShortCraftLiveIntent::DeferPottery` enum variant remains unused staging.

## Tests

```
cargo test -p ol-sim --lib -- hungry_work -- --test-threads=1
cargo test -p ol-sim --lib -- smith_defer_pottery baker_defer -- --test-threads=1
```

## Residual

Clothing craft bands (`AI-CRAFT-MULTI`).
