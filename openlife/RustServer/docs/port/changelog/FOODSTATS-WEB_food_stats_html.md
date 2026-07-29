# FOODSTATS-WEB / food_stats_html (2026-07-29)

## Goal

Exact Haxe `WebServer.generateFoodStatistics` on Rust web `/stats/food`.

## Haxe anchor

- `openlife/server/WebServer.hx` L402–424
- Columns: **Food** | **Eaten** (%) | **Related** (HQ rollup %)
- Sort by food id; name from `ObjectData.name`

## Implementation

### Pure (already present from FOODSTATS-DISK)

- `ol-sim/src/world_food_stats.rs` — `format_food_statistics_html`
- Live map + HQ edges via `WorldFoodStats` / `get_eaten_food_percentage`

### Web wire

- `ol-web/src/lib.rs`
  - `WebState.food_view: WorldFoodShare`
  - `stats_food_page` uses pure HTML helper + content names
  - 30s meta refresh (product UX; Haxe regenerates on request)
  - Home card: "eaten % / related"

### Server wire

- `ol-server/src/main.rs` — `food_view: Arc::clone(&shared_world_food)`
  - Same Arc as FoodStats.txt autosave mirror (sim `mirror_world_food_share`)

## Tests

- `world_food_stats::html_table_shape`
- `world_food_stats::html_with_foods_sorted`
- `cargo test -p ol-sim world_food_stats` (26 ok)
- `cargo test -p ol-web` (4 ok)
- `cargo check -p ol-server` ok

## Residual

- FoodStats{N} slot rotation (disk product choice; fixed `FoodStats.txt` remains)
- Optional JSON `/api/food` (not in Haxe web path)
