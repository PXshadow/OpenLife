# 2026-09-02 TH-CHEST-COINS

**status:** **DONE**

## Closed

- Haxe `TransitionHelper.doCommandHelper` chest/pouch coin store and take on USE.
- Empty-hand store into open/unlocked chest (986/989) when wallet > 10, age > 5, chest coins < 1; cap `MaxCoinsPerChest` 200.
- Pouch 209 store while targeting open chest when wallet ≥ 50; cap 50.
- Take from closed chest 987 (empty hand) or locked 988 (key 917); take from ground pouch 209.
- Wallet i32 + helper `coins` persist (`hits ≥ 1` on store). Say via lock-say slot.

## Tests

- `chest_coins::plan_chest_coin_use` store/take/cap/age
- `chest_store_coins_empty_hand_open_chest` / `chest_take_coins_empty_hand_closed_chest`
