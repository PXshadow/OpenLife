# FOODSTATS-DISK / foodstats_txt (2026-07-28)

## Status: **DONE** (+ ObjectCounts pure / HTML pure gap-close)

### Haxe
- `WorldMap.writeFoodStatistics(path)` — recompute `%` then write text dump
- Called from `WorldMap.write` after index files: `FoodStats{tmpDataNumber}.txt`
- Line format: `{pct}% t: {totalPct}% pipes: {val} {name}[{id}] yum: … meh: … boni: … mali: …`
- `totalPct` uses `getEatenFoodPercentage` (higher-quality rollup)
- Write-only (not loaded back); web can also show percentages
- Optional sibling: `TraceCountObjectsToDisk` → `ObjectCounts{N}.txt`
- `WebServer.generateFoodStatistics` HTML Food/Eaten/Related table

### Rust
- `ol-sim/src/world_food_stats.rs`
  - `format_stats_line` / `format_stats_lines_with_names` / `format_stats_text*`
  - `write_food_statistics` / `write_food_statistics_ids_only`
  - `format_food_statistics_html` (Haxe `generateFoodStatistics` pure)
  - `haxe_food_stats_slot_filename` (parity; default fixed name)
  - `WorldFoodShare = Arc<RwLock<WorldFoodStats>>`
  - `DEFAULT_FOOD_STATS_FILE = "FoodStats.txt"`
- `ol-sim/src/long_term.rs` (FOODSTATS residual ObjectCounts)
  - `format_object_count_line` / `format_object_counts_text` / `write_object_counts`
  - `LongTermState::{format_object_counts_*, write_object_counts}`
  - `DEFAULT_OBJECT_COUNTS_FILE` / `haxe_object_counts_slot_filename`
- Live map already on `SimState.world_food` (WORLD-FOOD-FACTOR)
- Sim mirror: `mirror_world_food_share` periodic + disconnect
- `SimBootLive.world_food_share`
- ol-config: `ServerConfig::food_stats_save_path` → `save_directory/FoodStats.txt`
- ol-config: `ServerConfig::object_counts_save_path` → `save_directory/ObjectCounts.txt`
- ol-server: autosave (60s / SAY SAVE) + shutdown dump with content names

### Intentional delta
| Haxe | Rust | Why |
|------|------|-----|
| `FoodStats{N}.txt` rotated with save slots | fixed `FoodStats.txt` (latest) | Matches other Rust saves (world_v1.olw, players_v1.bin); diagnostic dump |
| `ObjectCounts{N}.txt` when TraceCountObjectsToDisk | pure helpers + fixed path; no autosave share yet | Counts live on `LongTermState`; share wire residual |

### Residuals
- Lineage last-day death reason window (session counters still approximate starving factor)
- ObjectCounts **autosave share** (pure write ready; needs LongTerm counts mirror like WorldFoodShare)
- ol-web HTML table hook (pure `format_food_statistics_html` ready)

### Tests
- `world_food_stats::format_stats_lines_nonempty`
- `world_food_stats::format_stats_line_haxe_shape_with_name`
- `world_food_stats::format_stats_line_total_pct_includes_hq_rollup`
- `world_food_stats::format_food_statistics_html_haxe_shape`
- `world_food_stats::haxe_food_stats_slot_filename_shape`
- `world_food_stats::write_food_statistics_roundtrip_disk`
- `world_food_stats::write_food_statistics_empty_ok`
- `long_term::format_object_count_line_haxe_shape`
- `long_term::format_object_counts_lines_sorted_current_keys_only`
- `long_term::write_object_counts_roundtrip_disk`
- Prior WORLD-FOOD-FACTOR pure + live eat/feed tests
