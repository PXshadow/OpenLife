# 2026-09-02 SEARCH-HOME-OVEN

**status:** **DONE**

Haxe `AiHelper.SearchNewHome` uses `WorldMap.ovens` with `bestDistance = 80²` and wrap-aware quad (`CalculateDistance`). Rust already scanned local r=80 when the index was empty, but stored unwrapped loop coords and scored without torus wrap, so wrap-adjacent ovens on a 512 map were beyond `80²`.

Live: `collect_home_search_ovens` `wrap_tile`s candidates; HOME! / think-tick / AI HOME! use `search_new_home_ex` with world size.

Tests: `collect_home_search_ovens_local_when_global_empty` / `search_new_home_wrap_picks_across_edge` / `say_do_commands_home_bang_wrap_local_oven`.

`cargo check -p ol-server --offline` ok.
