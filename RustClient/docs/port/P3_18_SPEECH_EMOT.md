# P3#18 — Speech → emote `getEmotionIndex` (DONE)

**Date:** 2026-07-27  
**Area:** L-EMOT  
**Slice chosen:** speech `getEmotionIndex` (not eyesIndex/mainEyesOffset, not extraB)

## C++ reference

- `emotion.cpp` `getEmotionIndex`: uppercase speech; first trigger where speech **starts with** trigger **and** ends immediately after (exact match).
- `LivingLifePage.cpp` ~27071–27090: typed `/…` commands are **not** SAY; exact emote trigger → `EMOT 0 0 N#`.

## Rust

| Piece | Location |
|-------|----------|
| `EmotionBank::get_emotion_index` | `src/emotion.rs` |
| `SpeechOutbound` + `classify_speech_outbound` | `src/emotion.rs` |
| `encode_emot` | `src/actions.rs` |
| `ClientSession::send_say` routes EMOT/SAY/local | `src/session.rs` |
| `send_say_raw` / `send_emot` | `src/session.rs` |
| Unit + integration tests | `emotion` tests + `tests/speech_emot_p3_18.rs` |

## Behavior

| Input | Wire |
|-------|------|
| `/happy` (exact trigger) | `EMOT 0 0 0#` |
| `HELLO` | `SAY 0 0 HELLO#` |
| `/fps` (slash, not emote) | *(none — local only)* |

## Residual (P3#19) — **DONE**

- ~~`eyesIndex` / `mainEyesOffset` face placement~~ **DONE** (`content::setup_eyes_and_mouth` + render eyeEmot)
- ~~`extraB` anim type polish~~ **DONE** (`ANIM_EXTRA_B` + PE toggle + `setExtraIndexB` dual-fade)
- ~~Mouth sprite skip when `mouthEmot`~~ **DONE** (`hide_mouth` in `draw_object_with_pack`)
- ~~Emot creation/decay sounds~~ **DONE** (`play_emot_creation/decay_for_targets`, lazy OLSN)

## Verify

```powershell
cd C:\OhOl\OpenLife\RustClient
cargo test --lib emotion::
cargo test --test speech_emot_p3_18
cargo test --lib
```
