# AI-CRAFT-TASKS-DRAIN — npc `craftingTasks` doTimeStuffHelper drain

## Haxe

`AiBase.doTimeStuffHelper` ~667–680 (before clothing / assigned job):

1. If `itemToCraftId > 0 && countDone < count` → `craftItem` (return on success)
2. If `craftingTasks.length > 0` → shift each, `craftItem`, push back on fail

`doBakingHelper` ~3244 / ~3376: `makeSeatsAndCleanUp` false continues to `cleanUp` when not hungry.

## Rust

| Symbol | Role |
|--------|------|
| `select_runtime_sticky_craft_for_tick` | NPC `CraftAiRuntime`: continue unfinished then `take_next_crafting_task` |
| `requeue_runtime_task_on_fail` | Haxe `craftingTasks.push` after failed queue craft |
| `npc_craft_expand_progress` | Idle `Wait` / leftover staging is not progress |
| npc think `2a2` | Drain before profession ladder (`!acted && !starving`); GetOrCraft + USE/DROP/MOVE |
| `expand_baker_cleanup_live` | Shared `clean_up_action` + pottery fallback |
| `DeferSeatsCleanup` | Tomato/hungry no-op → `cleanUp` when not hungry |
| Player path | Unchanged: `apply_sticky_craft_queue_tick` on `PriorityRung::CraftQueue` |

## Tests

```
cargo test -p ol-sim --lib select_runtime -- --test-threads=1
cargo test -p ol-sim --lib baker_defer -- --test-threads=1
cargo test -p ol-sim --lib craft_ai_sticky -- --test-threads=1
```

ol-server bin test `craft_queue_wait` is blocked by unrelated `take_completed_llm_results_from_share` import in `ai_provider.rs`.

## Residual

1. ~~DeferPottery staging intent~~ **AI-HUNGRY-EMOTE** SeekOrCraft kiln/plate
2. ~~hungry-work emote/FX sendFoodUpdate~~ **AI-HUNGRY-EMOTE DONE**
