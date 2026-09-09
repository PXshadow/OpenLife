# 2026-09-02 closest-heat + per-kind peer_count + foodTarget sync

**status:** **DONE**

## Closed

- Haxe `GetClosestHeatObject` + held `heatValue/20` added to player ambient (half impact when already hot/cold).
- Ladder `peer_count_by_kind` — each profession step uses that job’s `countProfession`, not the first rung’s count.
- NPC `foodTarget` / `lastGotoObj` / `didNotReachFood` merge onto Player via views; publish preserves NPC marks (`preserve_view_path_reach_on_publish` now live).
- Remaining TODO_PORT `[~]` rows except ModuleConst marked DONE (code was already live or closed here).

## Tests

- `closest_heat_object_adds_fire_ambient` / `closest_heat_half_impact_when_already_hot`
- `npc_peer_count_for_kind_multi_prof_and_wounded` per-kind table
