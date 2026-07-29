# SOCIAL-WAR-PERSIST / war_posse_disk (2026-07-26)

## Status
**DONE (core)** — session WAR/POSSE maps survive restart via WPS1 disk.

## Haxe
- No server-side WAR/POSSE disk (protocol tags `WR`/`PJ` exist client-side).
- Rust product session maps: `WarState` / `PosseState` on `SimState`.

## Rust
| Piece | Path |
|-------|------|
| Pure WPS1 | `crates/ol-sim/src/war_posse_persist.rs` |
| Share type | `WarPosseSnapshot` / `WarPosseShare` |
| Boot package | `SimBootLive.war_posse_share` in `settings_live.rs` |
| Config path | `ServerConfig::war_posse_save_path` → `war_posse_v1.bin` |
| Server wire | `ol-server/src/main.rs` boot load + 60s autosave + shutdown |
| Sim wire | seed from share; `mirror_war_posse_share` on tick mirror + disconnect |

## Format (WPS1)
```
magic "WPS1" | version u32 | war pairs | posse killers→targets
```

## Tests
- `war_posse_persist::wps1_roundtrip_war_and_posse`
- `capture_apply_roundtrip`
- `bad_magic_errors`
- `empty_snapshot_writes_header_only`
- `prune_player_and_roundtrip`
- `war::prune_player_drops_pairs` / `prune_absent_keeps_living_pairs`
- `posse::prune_player_as_killer_and_target` / `prune_absent_keeps_living_edges`
- `death_prunes_war_and_posse_edges` (lib integration)

## Gap closes (this polish)
- Death prune via `prune_war_posse_for_player` in `apply_death_inheritance` (not on disconnect).
- `war_posse_dirty` + same-tick mirror flush after SAY WAR/POSSE/PEACE mutations.
- Docs: FILE_MATRIX / TODO_PORT / CALL_INDEX / DEPENDENCY_GRAPHS row for SOCIAL-WAR-PERSIST.

## Residual
- Keys are session `p_id` (same as live SAY WAR/POSSE). Full sticky identity needs Players.bin residual.
- No live SAY ALLIANCE speech path (codec roundtrips Alliance; product completeness).

## Cargo
```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
cargo test -p ol-sim --lib -- war_posse
cargo test -p ol-sim --lib -- death_prunes_war_and_posse
```
