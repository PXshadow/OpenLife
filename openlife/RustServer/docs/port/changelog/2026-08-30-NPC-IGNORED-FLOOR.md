# NPC-IGNORED-FLOOR — live AiIgnoredFloorIds + snapshot last farm

**Date:** 2026-08-30  
**Mode:** implement  
**Status:** **DONE** (two leftovers)

## Haxe

- `ServerSettings.AiIgnoredFloorIds` = [656, 888] (Bear Skin Rug)
- `AiHelper.IsIgnoredFloor`
- `countProfession('WATERBRINGER'|…)` uses `lastProfession` farm key

## Rust

| Symbol | Role |
|--------|------|
| `NpcConfig.ignored_floor_ids` | from `LiveSettings.ai_ignored_floor_ids` (empty → `ol_config::AI_IGNORED_FLOOR_IDS`) |
| `NpcConfig::from_live` / `Default` | live copy; compiled 656/888 fallback |
| npc `ProfessionScanInput.ignored_floor_ids` | `cfg.ignored_floor_ids.clone()` (not `Vec::new()`) |
| `PlayerSnapshot.last_farm` | `Option<FarmProfession>` from `farm_profession.last_profession` (`#[serde(skip)]`) |
| npc `NpcProfessionPeerRow.last_farm` | `profession_state.farm_rt.last_profession.or(snap.last_farm)` |

## Tests

- `npc_config_from_live_maps_knobs` — live `[656]` copies; empty live still `[656, 888]`
- `npc_config_from_live_ignored_floor_ids`
- `snapshot_last_farm_job_key` / `player_snapshot_includes_home_and_profession_sticky`

```
cargo test -p ol-server -- npc_config_from_live
cargo test -p ol-sim --lib -- snapshot_
cargo check -p ol-server
```

## Residual

CRAFT-LIVE-IO `useHeldObjOnTarget` multi-step staging.
