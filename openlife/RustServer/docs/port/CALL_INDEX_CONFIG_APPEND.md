
## Haxe: settings (`CONFIG-SETTINGS`)

| Symbol | File | Role |
|--------|------|------|
| `ServerSettings.readFromFile` / `writeToFile` | `settings/ServerSettings.hx` | RTTI dump + override lines; boot + hot reload |
| `TimeHelper.ReadServerSettings` | `server/TimeHelper.hx` | gate reload every 200 ticks |
| `ServerSettings.EternalWinter` / `SeasonDuration` / `NumberOfAis` | same | season force / length years / AI count |

## Rust: config hot-reload (`CONFIG-SETTINGS` / server_settings_hot_reload)

| Symbol | File | Role |
|--------|------|------|
| `ServerConfig::live_settings` | `RustServer/crates/ol-config` | runtime-safe knob snapshot |
| `ServerConfig::season_length_secs` | same | Haxe SeasonDuration years × 60 |
| `HotReloadTracker::new` / `poll` / `force_reload` | same | mtime + due-tick re-read of `server.toml` |
| `LiveSettings` | same | live field set (speed/move/season/npc/…) |
| `apply_live_settings` / `enforce_eternal_winter` | `ol-sim/src/settings_live.rs` | apply onto `SimState` |
| `intent_budget_from_live` | same | intent drain from live knobs |
| `SimBootLive` | same | boot package for `run_sim_loop_with_views` |
| Tests | `ol-config` `hot_reload_*` / `settings_live::*` | tracker mtime + apply idempotence |
