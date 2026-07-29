# OBJECTCOUNTS-LIVE / object_counts_share (2026-07-29)

## Status: **DONE**

Closes FOODSTATS-DISK residual ObjectCounts autosave share, plus census fidelity gaps
(`updateObjectCounts` / `countObjects` nest / boot seed).

### Haxe
- `WorldMap.write` when `TraceCountObjectsToDisk` → `ObjectCounts{N}.txt`
- Line: `Count object: [id] {description}: {current} original: {original}`
- `WorldMap.countObjects(objects, objectHelpers?)` — ground with `countsOrGrowsAs`; nest
  uses `parentId` only for contained + one sub-contained level
- `WorldMap.updateObjectCounts` — full recompute of `currentObjectsCount` (with helpers)
- `TimeHelper` `(tick + 20) % TicksBetweenSaving == 0` → `updateObjectCounts`
- Load path seeds original/current via ground-only `countObjects`

### Rust
- `ol-sim/src/object_counts_share.rs` — `ObjectCountsSnapshot` / `ObjectCountsShare`
- Pure format/write in `long_term.rs` (`format_object_count_*` / `write_object_counts`)
- Pure census: `count_objects_from_world`, `count_parent_id`, `LongTermState::update_object_counts`,
  `ensure_counts_for_dump` (seed ground originals + nest current)
- `should_update_object_counts` / `OBJECT_COUNTS_RECOMPUTE_TICKS` (600)
- Boot: `ensure_counts_for_dump` + first share mirror (non-empty early SAY SAVE / autosave)
- Tick: `maybe_update_object_counts` after long-term pass
- Disconnect: final nest recompute before mirror
- ol-config `object_counts_save_path` → `save_directory/ObjectCounts.txt`
- ol-server autosave (60s / SAY SAVE) + shutdown dump with content descriptions

### Intentional delta
| Haxe | Rust | Why |
|------|------|-----|
| `ObjectCounts{N}.txt` rotated with save slots | fixed `ObjectCounts.txt` | Matches FoodStats fixed latest dump |
| `TraceCountObjectsToDisk` gate | always dump on autosave | Diagnostic always-on |
| No per-USE/DROP count bump | same: drift until periodic recompute | Matches Haxe (full recompute only) |

### Tests
- `object_counts_share::object_counts_snapshot_from_long_term`
- `object_counts_share::object_counts_share_roundtrip_lock`
- `object_counts_share::from_long_term_before_seed_empty_after_ensure_non_empty`
- `long_term::count_objects_from_world_ground_and_counts_or_grows`
- `long_term::count_objects_from_world_includes_contained_nest`
- `long_term::seed_then_snapshot_matches_ground_census`
- `long_term::update_object_counts_corrects_player_drift_and_adds_nest`
- `long_term::ensure_counts_for_dump_seeds_and_includes_nest`
- `long_term::should_update_object_counts_haxe_tick_offset`
- Prior `long_term::format_object_count*` / `write_object_counts_roundtrip_disk`

### Remaining gaps (low priority)
- Per-intent USE/DROP/place count bumps (Haxe does not either; recompute corrects)
- Deep nest beyond Haxe's two levels (contained + subContained only)
- Slot-rotated `ObjectCounts{N}.txt` filenames (intentional fixed name)
