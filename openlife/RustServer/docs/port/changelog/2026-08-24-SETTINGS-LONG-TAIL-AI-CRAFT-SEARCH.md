# SETTINGS-LONG-TAIL / AI craft search knobs live

## Chunk
- **matrix_id:** `SETTINGS-LONG-TAIL`
- **status:** **PARTIAL** (craft search knobs promoted; long-tail continues)
- **Haxe:** `AiTimeToWaitIfCraftingFailed=15`, `AiMaxSearchRadius=60`, `AiMaxSearchIncrement=30`, `AiIgnoreTimeTransitionsLongerThen=120`
- **Rust:** LiveSettings → `GameplayKnobs.apply_craft_ai_search_knobs` → `CraftLiveExpandOpts` on profession-scan craftItem + top-down skip

## Implemented this fire
1. Four knobs on ServerConfig / LiveSettings / GameplayKnobs
2. FIELD_MAP Live
3. `FailedCraftings.remaining_wait_sec_ex` / `is_cooling_down_ex`
4. `craft_item_helper_ex` uses live wait + max radius; top-down uses live increment + ignore-time
5. Live profession-scan craft expand copies knobs from `state.gameplay`
6. Tests: cooldown `_ex` + `time_transition_exceeds_ai_ignore_ex` + `apply_live_settings_gameplay_knobs`

## Residual
- **CombatAngryTimeMinimum** still needs TimeHelper angry recovery tick (HIT subtracts only)
- DoorIds / AiIgnoredFloorIds remain ModuleConst tables

## Verify
```powershell
cargo test -p ol-config --lib
cargo test -p ol-sim --lib -- cooldown_blocks_retry time_auto_decay_negative_hours apply_live_settings_gameplay_knobs
```
