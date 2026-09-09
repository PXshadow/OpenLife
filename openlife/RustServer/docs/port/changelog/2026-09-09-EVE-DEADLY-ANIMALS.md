# 2026-09-09 EVE-DEADLY-ANIMALS

**status:** **DONE**

Haxe `GlobalPlayerInstance.spawnAsEve` L1184 `// TODO consider deadly animals`.

- `eve_location_fitness` adds the same divisor bump as a blocking grave when a deadly animal is close
- `is_eve_deadly_animal_id` matches wolf/boar (and patched combat animals); not rabbit/mosquito
- Live `find_eve_spawn_with_rng_graves` scans World tiles with `DEADLY_ANIMAL_SEARCH_DIST` 6

Tests: `fitness_deadly_animal_lowers_score` / `pick_best_prefers_clear_over_nearby_wolf` / `tile_has_close_wolf_or_boar` / `is_eve_deadly_animal_wolf_boar_not_rabbit`

`cargo check -p ol-server --offline` Finished.

Residual: map-object scan only (AnimalWorld movers not on the tile are missed); Chebyshev 6, not Haxe `moves²` GetCloseDeadlyAnimal filter. Next **OWNED-GATE-DELETE**.
