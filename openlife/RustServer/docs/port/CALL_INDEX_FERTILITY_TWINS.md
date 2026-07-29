### FERTILITY-TWINS / twin_sockets

| Symbol | Module | Notes |
|--------|--------|-------|
| `is_fertile` / `age_fertile` / `can_birth_full` | `fertility.rs` | Haxe `isFertile` (deleted + female + 14–42) |
| `FertilityState::format_query_sex` | same | `?FERTILE` male/ready/gestating |
| `TwinWaitQueue` / `TwinJoinOutcome` / `ReadyTwinParty` | `twins.rs` | protocol twin_code_hash party 2–4 |
| `apply_twin_join` / `process_ready_twin_party` | `twin_party_live.inc.rs` | live birth party |
| `player_is_female` / `player_is_fertile` | `lib.rs` | content name / po 19 default |
| `due_mothers` / `format_twin_party_ready` | `gestation_tick.rs` | tick + PS helpers |
| Tests | `fertility::*` / `twins::*` / `birth_requires_female_is_fertile` / `twin_join_party_ready_births` / `twin_wait_leave_on_disconnect` | pure + live |
| Disconnected → `twin_wait.leave` | `lib.rs` | wait-queue cleanup |

**Wire path:** live in `lib.rs` + `ol-net` LOGIN twin fields (`build_fertility_twins.rs` historical patch helper).
