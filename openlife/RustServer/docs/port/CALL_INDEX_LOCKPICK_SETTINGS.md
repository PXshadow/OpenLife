## Rust: LOCKPICK-SETTINGS / lockpick_live_knobs (extends TH-LOCK)

| Symbol | File | Role |
|--------|------|------|
| `LockpickSettings::from_parts` / `from_live` | `ol-sim/src/locks.rs` | build knobs from LiveSettings / raw (non-finite → defaults) |
| `lockpick_coins_to_wallet_i32` | same | pure Float→i32 floor writeback (Haxe `player.coins` Float residual) |
| `lockpick_settings_for_player` | same | female ×0.5 exhaustion / ×0.8 fail |
| `SimState.lockpick_settings` | `ol-sim/src/lib.rs` | live Haxe ServerSettings.Lockpick* |
| `apply_live_settings` lockpick_* | `ol-sim/src/settings_live.rs` | hot-reload → SimState |
| boot `apply_live_settings(tracker.last_live())` | `lib.rs` run_sim_loop | apply toml knobs before first USE |
| `apply_use_at` coins writeback | `use_transition.rs` | `lockpick_coins_to_wallet_i32` on lock gate |
| `server.toml` `lockpick_*` | `ol-config` ServerConfig/LiveSettings | LockpickSucessChance / Fail / Exhaustion / CoinCost |
| `ServerConfig::live_diff_keys` | `ol-config` | all four `lockpick_*` keys |
| Tests | `settings_live::apply_live_settings_lockpick_*` / `locks::fractional_coin_*` / `live_exhaustion_*` / `evaluate_gate_live_*` / `use_transition::lock_removal_1003_live_*` / ol-config `live_diff_keys_all_four_lockpick_only` | live apply + USE wire + sanitize |
