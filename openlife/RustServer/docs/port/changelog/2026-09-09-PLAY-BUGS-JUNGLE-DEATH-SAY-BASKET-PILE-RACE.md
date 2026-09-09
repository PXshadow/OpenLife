# Play bugs: jungle BB, death PU, curse say, basket put-in, pile dummy MC, birth races

Date: 2026-09-09

## Haxe vs Rust

- Login `BB` used biome ids 2 and 6 (Yellow Prairie / Jungle) labeled RIVER/OCEAN. Haxe sends 21/17/9. Official clients path-block BB, so prairie→jungle was unwalkable.
- Hunger/age/combat/suicide deaths set `deleted` but never sent PU `X X reason_*` + FRAME (Haxe `PlayerInstance.toData` + `SendUpdateToAllClosePlayers`). Clients treated TCP silence as disconnect.
- Curse enter/clear PS was sent without FRAME (`send_ps_reply`).
- USE with a held item on a basket swapped held/ground. Haxe `doContainerStuff(false)` puts in; DROP swaps.
- MAP_CHUNK used parent object ids. Haxe `dummyId()` for partial `numberOfUses`. MX on interact already used dummy ids.
- Spawn defaulted `po_id` to 19 (white). Haxe constructor picks `personObjectData[rand]`, then Eve biome/pair or child color. Humans never tried `spawnAsChild`. AI rebirth was immediate; Haxe waits `10 + 2 * max(1,60-age) * TimeToAiRebirthPerYear * rand`.

## Rust

- `login_bootstrap` BB = `21 MOUNTAIN / 17 RIVER / 9 OCEAN`.
- `format_player_update_line_death` + `send_death_player_update` on vitals, DIE, combat, animal, USE food-death.
- Curse PS via `send_ps_reply` (`p_id/0 text` + FRAME).
- USE both-nonempty: put-in container, else refuse (no swap).
- Live MC uses `encode_object_for_map_wired` / `wire_id_for_uses`.
- Birth: random person object; Eve vs child for humans and AI; child age 0.01; EVE/ADAM names; new `p_id` after death. NPC wait `ai_rebirth_wait_secs`.
