# 2026-09-02 NPC animal-path player ctx + hiddenWound / lostCombatPrestige

**status:** **DONE**

## Closed

- NPC Goto uses Haxe `isAnimalDeadlyForMe` (weapon + loved biome) on every `npc_try_walk_to` / takeover explore.
- `PlayerSnapshot.is_hidden_wound` — peer `countProfession` skips light hiddenWound alias (Haxe `isWounded`).
- View publish copies `lostCombatPrestige` onto snapshots for GetCloseDeadlyPlayer.

## Tests

- `npc_ai::hidden_wound_is_not_peer_wounded`
- `npc_ai::lost_combat_prestige_makes_deadly_player_candidate`
- `smith_profession::npc_smith_peer_wounded_excluded` hiddenWound ex
- `pathfind::loved_biome_animal_not_deadly_for_path_unless_weapon`
