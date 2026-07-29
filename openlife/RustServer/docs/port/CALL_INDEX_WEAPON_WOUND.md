# Call index slice — WEAPON-WOUND-TRANS / weapon_zero (+ WALLET-COINS)

Appended from `haxe-port-chunk` (matrix_id WEAPON-WOUND-TRANS). Merge into [CALL_INDEX.md](CALL_INDEX.md) when convenient.

## Haxe

| Symbol | File | Role |
|--------|------|------|
| `GlobalPlayerInstance.DoDamage` weapon+0 | `server/GlobalPlayerInstance.hx` | GetTransition(weapon,0,LA) → doWound equip / ground / newActor bloody |
| `TransitionImporter.GetTransition(w,0,lastUseActor)` | `data/transition/TransitionImporter.hx` | prefer LA then non-LA |
| `ObjectData.woundFactor` / `damage` | `data/object/ObjectData.hx` | default 0.5; combat damage |
| `ServerSettings.PatchObjectData` damage/woundFactor | `settings/ServerSettings.hx` | knife/sword/bow/animals/wounds/snake 0.98 |
| `ObjectHelper.isArrowWound` / `isWound` | `data/object/ObjectHelper.hx` | ground path when already arrow wound |
| `WorldMap.PlaceObject` wound ground | `server/WorldMap.hx` | ttc=2 allowReplace |
| `setHeldObject` light wound | `server/GlobalPlayerInstance.hx` | hiddenWound when autoDecay→0 |
| `takeCoins` | same | CoinsOnWoundingFactor + darkNosaj×2 cap1 on lethal + first wound equip |

## Rust

| Symbol | File | Role |
|--------|------|------|
| `resolve_weapon_zero_transition` / `plan_weapon_zero_wound` | `ol-sim/src/weapon_wound.rs` | pure LA prefer + equip/ground plan |
| `should_do_wound` / `effective_wound_factor` / `object_wound_factor` | same | food_max gate + snake shoes |
| `bloody_weapon_from_zero_transition` | same | content newActor over hard-coded table |
| `coins_stolen_on_wound` / `take_coins_say_text` | same | pure takeCoins amount + say |
| `Economy::take_coins_on_wound` | `economy.rs` | wallet gift path (no trade prestige) |
| `apply_take_coins_on_wound` | `ol-sim/src/lib.rs` | live lethal + equip; reads dark_nosaj + live factor |
| `Player.dark_nosaj` | `player.rs` | session field (Haxe not saved); ×2 takeCoins factor |
| `GameplayKnobs.coins_on_wounding_factor` | `settings_live.rs` | LiveSettings CoinsOnWoundingFactor (0.5) |
| `set_held_wound_ctx_for` / `player_set_held_object` | weapon_wound + nested_body | light-wound hiddenWound |
| `is_arrow_wound_*` | `ol-sim/src/death_polish.rs` | Arrow Wound gate |
| `apply_weapon_zero_wound_hit` | `ol-sim/src/lib.rs` | HIT Wound/Kill live wire |
| `BloodyApplyMode::StrikeContent` | same | content newActor + base TTC |
| `ObjectDef.damage` / `wound_factor` / `damage_protection_factor` | `ol-content` | combat fields |
| `apply_default_combat_damage_patches` | `ol-content` lib_tail | ServerSettings damage table |
| Tests | `weapon_wound::*`, `wallet_take_coins_*`, content combat patches | pure + live factor/dark_nosaj |
