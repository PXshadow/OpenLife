# 2026-09-02 FEVER-HUNGER-PE

**status:** **DONE** (death-path split; no death-timing change)

Haxe `UpdateEmotes` starving PE 31 fires while `food_store < 0` and still alive (Haxe death is `food_store_max < DeathWithFoodStoreMax`). Rust death is `food < 0`, so PE 31 cannot emit on a living player.

Living hunger face is the separate product path: PE 1 (`Emote.mad`) when `0 <= food < 3` every 8s (`HUNGER_EMOT_*`). Ladder still implements PE 31 for the Haxe gate (pure tests).

Tests: `living_low_food_is_not_starving_pe` / `tick_vitals_alive_low_food_is_not_starving_pe` / `tick_vitals_food_below_zero_deletes_without_starving_pe`.

`cargo check -p ol-server --offline` ok.
