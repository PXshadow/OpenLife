# AI-SAY-HELPER-FAN — live fan_out_ai_say_scripted

## Haxe

- `Connection.sendSayToAllClose` → each nearby AI `AiBase.sayHelper` (~L4727–5002) **before** LLM fallback
- Distance `MaxDistanceToBeConsideredAsCloseForSayAi`
- Hearer path only — speaker `I AM` / `YOU ARE` is `NamingHelper.DoNaming`

## Rust

| Symbol | Role |
|--------|------|
| `plan_scripted_say_helper` | pure HOLA/NAME?/FOLLOW/STOP/DROP/MAKE/HOME!/GO HOME/… |
| `fan_out_ai_say_scripted` | live apply in `ai_say_helper_live.inc.rs` |
| SAY path | after PS fan-out; `live_collect_ai_speech_hearers` then plan |

Uses existing `try_ai_follow_path_to`, `tick_ordered_ai_drop` (deferred DROP), `search_new_home` + `get_close_fire`, STOP `waitingTime = 10` assign (can lower).

## Tests

```
cargo test -p ol-sim --lib -- say_helper fan_out_ai_say_scripted
```

## Residual

1. ~~Live `fan_out_ai_speech_llm`~~ **AI-LLM-FAN DONE**
2. ~~YOU ARE speaker happy emote~~ **AI-YOU-ARE-EMOTE DONE**
3. Gender-split name files / lastNames.txt
