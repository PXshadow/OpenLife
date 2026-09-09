# SETTINGS-LONG-TAIL / MaxDistanceToBeConsideredAsCloseForSayAi live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (CloseForSayAi promoted; long-tail continues)
- **Haxe:** `ServerSettings.MaxDistanceToBeConsideredAsCloseForSayAi = 20`; `AiBase.sayHelper` L4733–4740
- **Rust:** `LiveSettings.max_distance_say_ai` → `GameplayKnobs` → `ai_say_range` / `live_collect_ai_speech_hearers` on human SAY

## Implemented this fire
1. `max_distance_say_ai` on ServerConfig / LiveSettings / GameplayKnobs (default 20)
2. FIELD_MAP `MaxDistanceToBeConsideredAsCloseForSayAi` → Live
3. Free-form SAY collects AI hearers with the live Euclidean radius (not adult CloseForSay)
4. Tests: `live_ai_hear_uses_close_for_say_ai` + apply_live key + pure max shrink

## Residual
- Scripted/LLM `fan_out_ai_say_scripted` / `fan_out_ai_speech_llm` apply still missing from `lib.rs` (hearers collected, not yet applied)
- Next: `MaxDistanceToAutoExileAttacker` (15)
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables

## Verify
```powershell
cargo test -p ol-config --lib -- MaxDistanceToBeConsideredAsCloseForSayAi max_distance_say_ai
cargo test -p ol-sim --lib -- live_ai_hear_uses_close_for_say_ai apply_live_settings_gameplay_knobs collect_ai_speech_hearers
```
