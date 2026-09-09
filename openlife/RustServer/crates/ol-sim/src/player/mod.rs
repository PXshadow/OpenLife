//! # Player — Haxe `GlobalPlayerInstance`
//!
//! **Open this module first** for player-related code.
//!
//! The living body is [`body`] (`Player` struct). Apply/tick still run in `ol-sim`
//! (sole world writer). Sibling files below stay at `src/` so existing
//! `crate::food_eating` paths keep working; they are listed here so you do not
//! have to hunt the crate root.
//!
//! | File | Owns |
//! |------|------|
//! | **`player/body.rs`** | `Player`, clothing slots, backpack, notes, title, snapshot |
//! | `player_tick.rs` | Player vs world tick **map** (docs); entry is `tick_vitals` in `lib.rs` |
//! | `players_persist.rs` | Disk record ↔ live `Player` |
//! | `food_eating.rs` | `try_eat_held` (GPI tryEat) |
//! | `food_fill.rs` / `yum.rs` / `food_store_max.rs` | Fill / yum re-exports; store-max formulas in **`ol-food-eating`** |
//! | `temperature_handler.rs` | Body heat API (`map_temp_player` + `heat_ideal`) |
//! | `angry_tick.rs` | Combat angry recover/drain |
//! | `age_curves.rs` / `age_stage.rs` | Age / stage |
//! | `clothing_cmds.rs` / `clothing_transitions.rs` | Wear / clothing USE |
//! | `feed.rs` / `feed_other_yum.rs` / `gestation_tick.rs` | Nurse / feed-other / birth due |
//! | `death_cause.rs` / `death_inherit.rs` / `death_polish.rs` | Death / grave / inherit |
//! | `weapon_wound.rs` | Wound **plans** re-export from **`ol-combat-rules`**; live HIT apply stays sim |
//! | `move_path.rs` / `move_live_gates.rs` | Timed MOVE on the player |
//! | `speech.rs` / `mute.rs` / `mumble.rs` / `do_commands_wire.rs` | SAY |
//! | `naming.rs` | I AM / YOU ARE |
//! | `player_soul.rs` | Chat memory FIFO (persist types; live view in `soul_live.rs`) |
//! | `afk.rs` | Idle / yawn |
//!
//! World map decay / animals / seasons are **not** here (`world_time.rs`, `long_term.rs`,
//! [`crate::tick_world_after_players`]).
//!
//! `crate::player::Player` is unchanged.

mod body;
pub use body::*;
