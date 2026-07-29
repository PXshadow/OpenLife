# C-SS-AI-IGNORE / ai_should_ignore (2026-07-29)

## Status: DONE (core + gap-close)

### Implemented

Live **ServerSettings.PatchTransitions `TransitionData.aiShouldIgnore`** table for AI craft:

| Piece | Module | Notes |
|-------|--------|-------|
| `ContentDb.ai_should_ignore` | `ol-content` | primary `(actor, target)` craft-AI ignore |
| `ContentDb.ai_should_ignore_last_use` | same | last-use-only (pond 141/142 LT); not graph-seeded |
| `apply_default_ai_should_ignore_patches` / `_ex` | `ai_should_ignore_patches.inc.rs` | explicit + bulk + oven/kiln + synthetics |
| `insert_synthetic_ai_ignore_product_bodies` | same | coals/calf/skewer-fire/butter-water product rows |
| `mark_by_new_actor_with_decays_to` | same | broken steel 858/862 ignore + `decays_to_obj` |
| `transition_ai_should_ignore` / `_ex` | same + ContentDb methods | primary vs last-use lookup |
| load wire | `load_content` + `finish_cache_boot` | after horse patches |
| `ReverseCraftGraph.ai_should_ignore` | `ol-sim/craft_graph.rs` | seed **primary only**; path/seek skip |
| topdown search | `craft_topdown.rs` | `graph.ai_should_ignore_edge` + meta |
| `craft_trans_meta_map_from_content` | same | primary ignore; last-use-only when no primary |
| `craft_item_helper_with_meta` | `craft_item.rs` | optional full `meta_by_edge` into helper |

### Gap-close (this run)

1. **Synthetic product bodies** — Haxe `new TransitionData` for water+fire coals, knife+calf, skewer+fire, butter/ketchup water, TIME+bear cave target 631; insert-if-absent (never overwrite content).
2. **Pond LA/LT** — (235\|209, 141\|142) moved to `ai_should_ignore_last_use`; primary pond fill stays craftable; reverse graph not suppressed.
3. **Broken-steel decaysToObj** — bulk newActor 858/862 also sets actor `ObjectDef.decays_to_obj` (Haxe ~2849–2864 side effect).
4. **meta_by_edge helper path** — `craft_item_helper_with_meta` + reexport; graph edge skip still covers default path without meta.

### Tests

- `ol-content` `ai_should_ignore_tests::*` — explicit, oven/kiln, bulk tools, synthetic bodies, pond last-use-only, broken-steel decays
- `ol-sim` craft_graph ignore / load / seek
- `ol-sim` craft_topdown graph/meta + `craft_trans_meta_map_pond_last_use_only`
- `ol-sim` `craft_item_helper_with_meta_skips_ignored_edge`

### Residual

1. `AIAllowBuildOven` / `AIAllowBuilKiln` live config knobs (defaults match Haxe false; `AiShouldIgnorePatchOpts` gates apply path)
2. ObjectDef `aiCraftMax` / `aiCraftMin` content load (AI-CRAFT-TOPDOWN residual)
3. Live expand / profession paths not always pass a built `meta_by_edge` map (API ready; graph ignore covers skip)

### Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-content --lib -- ai_should_ignore
cargo test -p ol-sim --lib -- craft_graph
cargo test -p ol-sim --lib -- craft_topdown
cargo test -p ol-sim --lib -- craft_item_helper_with_meta
```
