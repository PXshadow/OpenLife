//! Haxe `TransitionHelper.doCommandHelper` chest / pouch coin store and take.
//!
//! Side-effect on USE (does not by itself apply a transition). Indoor HX unused.
// Haxe: TransitionHelper.doCommandHelper L260–306
// TH-CHEST-COINS

/// Open Wooden Chest.
pub const OPEN_WOODEN_CHEST: i32 = 986;
/// Closed Wooden Chest.
pub const CLOSED_WOODEN_CHEST: i32 = 987;
/// Locked Wooden Chest.
pub const LOCKED_WOODEN_CHEST: i32 = 988;
/// Unlocked Wooden Chest.
pub const UNLOCKED_WOODEN_CHEST: i32 = 989;
/// Empty Water Pouch.
pub const EMPTY_WATER_POUCH: i32 = 209;
/// Key (opens locked chest with matching extern).
pub const KEY_OBJ: i32 = 917;

/// Haxe `ServerSettings.MaxCoinsPerChest` compiled fallback (live: `GameplayKnobs.max_coins_per_chest`).
// SETTINGS-KNOB-TAIL
pub const MAX_COINS_PER_CHEST: i32 = 200;
/// Haxe `ServerSettings.MaxCoinsPerPouch` compiled fallback (live: `GameplayKnobs.max_coins_per_pouch`).
// SETTINGS-KNOB-TAIL
pub const MAX_COINS_PER_POUCH: i32 = 50;

/// Min age (exclusive) for chest/pouch coin actions.
pub const CHEST_COIN_MIN_AGE: f32 = 5.0;
/// Store into an open/unlocked chest only when wallet is above this.
pub const CHEST_STORE_MIN_COINS: f32 = 10.0;
/// Store into a pouch only when wallet is at least this.
pub const POUCH_STORE_MIN_COINS: f32 = 50.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChestCoinAction {
    StoreInChest { amount: i32 },
    StoreInPouch { amount: i32 },
    TakeFromChest { amount: f32 },
    TakeFromPouch { amount: f32 },
}

/// Haxe `isHeldEmpty` — empty hands.
#[inline]
pub fn is_held_empty(held_id: i32) -> bool {
    held_id <= 0
}

fn floor_coins(player_coins: f32) -> i32 {
    if !player_coins.is_finite() || player_coins <= 0.0 {
        0
    } else {
        player_coins.floor() as i32
    }
}

fn store_amount(player_coins: f32, cap: i32) -> i32 {
    let n = floor_coins(player_coins);
    if cap <= 0 {
        0
    } else {
        n.min(cap)
    }
}

/// Plan chest/pouch coin side-effect for one USE (Haxe L260–306 order).
pub fn plan_chest_coin_use(
    held_id: i32,
    target_id: i32,
    player_coins: f32,
    target_coins: f32,
    held_coins: f32,
    age: f32,
    max_chest: i32,
    max_pouch: i32,
) -> Option<ChestCoinAction> {
    if !age.is_finite() || age <= CHEST_COIN_MIN_AGE {
        return None;
    }
    let empty = is_held_empty(held_id);
    let player_coins = if player_coins.is_finite() {
        player_coins
    } else {
        0.0
    };
    let target_coins = if target_coins.is_finite() {
        target_coins
    } else {
        0.0
    };
    let held_coins = if held_coins.is_finite() {
        held_coins
    } else {
        0.0
    };

    // Store into open / unlocked chest (empty hand).
    let close_chest = empty && (target_id == OPEN_WOODEN_CHEST || target_id == UNLOCKED_WOODEN_CHEST);
    if close_chest && player_coins > CHEST_STORE_MIN_COINS && target_coins < 1.0 {
        let amount = store_amount(player_coins, max_chest);
        if amount > 0 {
            return Some(ChestCoinAction::StoreInChest { amount });
        }
    }

    // Store into empty water pouch while targeting an open chest.
    let is_open_chest = target_id == OPEN_WOODEN_CHEST || target_id == UNLOCKED_WOODEN_CHEST;
    if is_open_chest
        && player_coins >= POUCH_STORE_MIN_COINS
        && held_id == EMPTY_WATER_POUCH
        && held_coins < 1.0
    {
        let amount = store_amount(player_coins, max_pouch);
        if amount > 0 {
            return Some(ChestCoinAction::StoreInPouch { amount });
        }
    }

    // Take from closed chest (empty hand) or locked chest (key).
    let open_chest = (empty && target_id == CLOSED_WOODEN_CHEST)
        || (held_id == KEY_OBJ && target_id == LOCKED_WOODEN_CHEST);
    if open_chest && target_coins > 0.0 {
        return Some(ChestCoinAction::TakeFromChest {
            amount: target_coins,
        });
    }

    // Take from a pouch on the ground.
    if target_id == EMPTY_WATER_POUCH && target_coins > 0.0 {
        return Some(ChestCoinAction::TakeFromPouch {
            amount: target_coins,
        });
    }

    None
}

pub fn chest_coin_say(action: ChestCoinAction) -> String {
    match action {
        ChestCoinAction::StoreInChest { amount } => format!("Stored {amount} Coins!"),
        ChestCoinAction::StoreInPouch { amount } => {
            format!("Stored {amount} Coins in you Pouch!")
        }
        ChestCoinAction::TakeFromChest { amount } => {
            let n = if amount.is_finite() { amount } else { 0.0 };
            format!("Got {n} Coins!")
        }
        ChestCoinAction::TakeFromPouch { amount } => {
            let n = if amount.is_finite() { amount } else { 0.0 };
            format!("Got {n} Coins from Pouch!")
        }
    }
}

/// Wallet debit/credit as i32 (Haxe player.coins is Float; live wallet is i32).
pub fn wallet_delta(action: ChestCoinAction) -> i32 {
    match action {
        ChestCoinAction::StoreInChest { amount } | ChestCoinAction::StoreInPouch { amount } => {
            -amount.max(0)
        }
        ChestCoinAction::TakeFromChest { amount } | ChestCoinAction::TakeFromPouch { amount } => {
            if amount.is_finite() && amount > 0.0 {
                amount.floor() as i32
            } else {
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_open_chest_empty_hand() {
        let a = plan_chest_coin_use(0, OPEN_WOODEN_CHEST, 20.0, 0.0, 0.0, 14.0, 200, 50)
            .expect("store");
        assert_eq!(a, ChestCoinAction::StoreInChest { amount: 20 });
        assert_eq!(chest_coin_say(a), "Stored 20 Coins!");
        assert_eq!(wallet_delta(a), -20);
    }

    #[test]
    fn store_requires_more_than_ten_and_age() {
        assert!(plan_chest_coin_use(0, OPEN_WOODEN_CHEST, 10.0, 0.0, 0.0, 14.0, 200, 50).is_none());
        assert!(plan_chest_coin_use(0, OPEN_WOODEN_CHEST, 20.0, 0.0, 0.0, 5.0, 200, 50).is_none());
        assert!(plan_chest_coin_use(0, OPEN_WOODEN_CHEST, 20.0, 1.0, 0.0, 14.0, 200, 50).is_none());
    }

    #[test]
    fn store_caps_at_max_chest() {
        let a = plan_chest_coin_use(0, UNLOCKED_WOODEN_CHEST, 250.0, 0.0, 0.0, 14.0, 200, 50)
            .unwrap();
        assert_eq!(a, ChestCoinAction::StoreInChest { amount: 200 });
        let live = plan_chest_coin_use(0, UNLOCKED_WOODEN_CHEST, 250.0, 0.0, 0.0, 14.0, 50, 50)
            .unwrap();
        assert_eq!(live, ChestCoinAction::StoreInChest { amount: 50 });
    }

    #[test]
    fn store_pouch_on_open_chest() {
        let a = plan_chest_coin_use(
            EMPTY_WATER_POUCH,
            OPEN_WOODEN_CHEST,
            50.0,
            0.0,
            0.0,
            14.0,
            200,
            50,
        )
        .unwrap();
        assert_eq!(a, ChestCoinAction::StoreInPouch { amount: 50 });
        assert!(chest_coin_say(a).contains("Pouch"));
    }

    #[test]
    fn take_closed_and_locked() {
        let a = plan_chest_coin_use(0, CLOSED_WOODEN_CHEST, 0.0, 15.5, 0.0, 14.0, 200, 50).unwrap();
        assert_eq!(a, ChestCoinAction::TakeFromChest { amount: 15.5 });
        assert_eq!(wallet_delta(a), 15);
        let b = plan_chest_coin_use(KEY_OBJ, LOCKED_WOODEN_CHEST, 0.0, 3.0, 0.0, 14.0, 200, 50)
            .unwrap();
        assert_eq!(b, ChestCoinAction::TakeFromChest { amount: 3.0 });
    }

    #[test]
    fn take_pouch_on_ground() {
        let a = plan_chest_coin_use(0, EMPTY_WATER_POUCH, 1.0, 12.0, 0.0, 14.0, 200, 50).unwrap();
        assert_eq!(a, ChestCoinAction::TakeFromPouch { amount: 12.0 });
        assert!(chest_coin_say(a).contains("from Pouch"));
    }
}
