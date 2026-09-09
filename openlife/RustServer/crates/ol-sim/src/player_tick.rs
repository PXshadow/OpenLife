//! # Player tick vs world tick (Haxe `TimeHelper` split)
//!
//! Haxe `TimeHelper.DoTimeStuff` runs **two different concerns** in one loop.
//! Rust keeps them in separate modules so player systems are not mixed into
//! world map-slice logic (and vice versa).
//!
//! ## Haxe
//!
//! ```text
//! DoTimeStuff
//!   ├─ DoSeason                          → environment (shared)
//!   ├─ for each player: DoTimeStuffForPlayer   ← PLAYER TICK
//!   ├─ DoWorldMapTimeStuff               ← WORLD TICK
//!   ├─ RespawnObjects
//!   └─ DoWorldLongTermTimeStuff          ← WORLD LONG-TERM
//! ```
//!
//! ### Player tick (`DoTimeStuffForPlayer`) — lives under player modules
//!
//! | Step | Rust home |
//! |------|-----------|
//! | Display / close players | PU/PO helpers; **SendMoveEveryXTicks** `leader_range::maybe_refresh_close_players` |
//! | Leadership / follow confirm | `do_commands_wire`, `relations` |
//! | Held/clothing time transitions | use / nested body timers |
//! | Emotes | `fever_pe` / emotes |
//! | `updateAge` | age curves inside `tick_vitals` |
//! | `updateFoodAndDoHealing` | food pipes / heal inside `tick_vitals` |
//! | `updateTemperature` | **[`crate::temperature_handler`]** only |
//! | Eat is **not** a tick step — client/AI USE/SELF | **[`crate::food_eating`]** |
//!
//! Player file index: [`crate::player`] (`src/player/mod.rs`).
//! Entry today: [`crate::tick_vitals`] / `tick_vitals_with_metrics` in `lib.rs`
//! (orchestration still concentrated there; peel gradually without mixing
//! world decay into this file).
//!
//! ### World tick — must not own eat / body heat
//!
//! | Step | Rust home |
//! |------|-----------|
//! | Map-slice time transitions, water, tile temps sparse init | [`crate::world_time`] |
//! | Animals move/damage/pop | `animals`, `animal_move`, `animal_pop` |
//! | Nested container timers | `nested_timers` / world_time wire |
//! | Long-term decay / snow / growback | [`crate::long_term`] |
//!
//! ## Rule
//!
//! - **Temperature** changes for a living player → `temperature_handler`
//! - **Eating** held food → `food_eating`
//! - **World** object timers / animals / seasons on the map → `world_time` / `long_term`
//! - Do not add try_eat or body-heat integration into `world_time.rs`

/// Documentation anchor for the Haxe player slice.
pub const HAXE_DO_TIME_STUFF_FOR_PLAYER: &str = "server/TimeHelper.hx::DoTimeStuffForPlayer";

/// Documentation anchor for the Haxe world map slice.
pub const HAXE_DO_WORLD_MAP_TIME_STUFF: &str = "server/TimeHelper.hx::DoWorldMapTimeStuff";
