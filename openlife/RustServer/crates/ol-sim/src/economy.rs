//! Coins / trade prestige (Open Life Reborn economy subset).

use std::collections::HashMap;

/// Haxe `ServerSettings.InheritCoinsFactor` — fraction of wallet credited as
/// account `coinsInherited` on death (for future-life inheritance weight).
pub const INHERIT_COINS_FACTOR: f32 = 0.8;

#[derive(Debug, Clone, Default)]
pub struct Wallet {
    pub coins: i32,
    pub trade_prestige: f32,
}

#[derive(Debug, Default, Clone)]
pub struct Economy {
    pub wallets: HashMap<i32, Wallet>,
    /// Shared village treasury (taxes / donations / unclaimed inheritance).
    pub treasury: i32,
}

impl Economy {
    pub fn wallet_mut(&mut self, p_id: i32) -> &mut Wallet {
        self.wallets.entry(p_id).or_default()
    }

    pub fn add_coins(&mut self, p_id: i32, amount: i32) {
        let w = self.wallet_mut(p_id);
        w.coins = w.coins.saturating_add(amount);
        if amount > 0 {
            w.trade_prestige += (amount as f32) * 0.01;
        }
    }

    /// Transfer coins; returns false if insufficient.
    ///
    /// Grants small trade prestige to both parties (PAY / TRADE path).
    pub fn transfer(&mut self, from: i32, to: i32, amount: i32) -> bool {
        if amount <= 0 {
            return false;
        }
        if self.wallet_mut(from).coins < amount {
            return false;
        }
        self.wallet_mut(from).coins -= amount;
        self.wallet_mut(to).coins = self.wallet_mut(to).coins.saturating_add(amount);
        self.wallet_mut(from).trade_prestige += 0.05;
        self.wallet_mut(to).trade_prestige += 0.02;
        true
    }

    /// Move coins with **no** trade-prestige change (GIFT / LOAN / REPAY path).
    ///
    /// Returns false if `amount` is non-positive or `from` has insufficient coins.
    pub fn gift(&mut self, from: i32, to: i32, amount: i32) -> bool {
        if amount <= 0 || from == to {
            return false;
        }
        if self.wallet_mut(from).coins < amount {
            return false;
        }
        self.wallet_mut(from).coins -= amount;
        self.wallet_mut(to).coins = self.wallet_mut(to).coins.saturating_add(amount);
        true
    }

    /// Debit `amount` from `from` into [`Self::treasury`]. Returns false if insufficient.
    pub fn donate_to_treasury(&mut self, from: i32, amount: i32) -> bool {
        if amount <= 0 {
            return false;
        }
        let w = self.wallet_mut(from);
        if w.coins < amount {
            return false;
        }
        w.coins -= amount;
        self.treasury = self.treasury.saturating_add(amount);
        true
    }

    /// Leader tax: same coin path as donate (caller enforces leadership).
    pub fn tax_to_treasury(&mut self, from: i32, amount: i32) -> bool {
        self.donate_to_treasury(from, amount)
    }

    /// Pay from treasury to a player wallet.
    pub fn pay_from_treasury(&mut self, to: i32, amount: i32) -> bool {
        if amount <= 0 || self.treasury < amount {
            return false;
        }
        self.treasury -= amount;
        self.add_coins(to, amount);
        true
    }

    /// Read wallet coins (0 if missing).
    pub fn coins_of(&self, p_id: i32) -> i32 {
        self.wallets.get(&p_id).map(|w| w.coins).unwrap_or(0)
    }

    /// Haxe `takeCoins`: move coins target → attacker with **no** trade prestige.
    ///
    /// Same coin path as [`Self::gift`]; amount must already be resolved via
    /// `coins_stolen_on_wound` (floor factor +1, darkNosaj ×2 cap 1).
    // Haxe: GlobalPlayerInstance.takeCoins L4835–4836
    // WALLET-COINS
    pub fn take_coins_on_wound(&mut self, attacker: i32, target: i32, amount: i32) -> bool {
        self.gift(target, attacker, amount)
    }

    /// Zero wallet and return previous coins (no destination).
    pub fn take_wallet(&mut self, deceased: i32) -> i32 {
        let coins = self.coins_of(deceased);
        if let Some(w) = self.wallets.get_mut(&deceased) {
            w.coins = 0;
        }
        coins
    }

    /// Deposit residual coins into treasury (unclaimed inheritance / no kids).
    pub fn deposit_treasury(&mut self, amount: i32) {
        if amount > 0 {
            self.treasury = self.treasury.saturating_add(amount);
        }
    }

    /// Legacy helper: zero deceased wallet → mother (if `Some`) else treasury.
    ///
    /// Prefer [`crate::apply_death_inheritance`] (Haxe InheritCoins + kids).
    /// Returns the amount transferred (0 if the wallet was empty).
    pub fn inherit_on_death(&mut self, deceased: i32, mother_online: Option<i32>) -> i32 {
        let coins = self.take_wallet(deceased);
        if coins <= 0 {
            return 0;
        }
        match mother_online {
            Some(mid) if mid != deceased => {
                self.add_coins(mid, coins);
            }
            _ => {
                self.deposit_treasury(coins);
            }
        }
        coins
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_and_reject() {
        let mut e = Economy::default();
        e.add_coins(1, 10);
        assert!(e.transfer(1, 2, 3));
        assert_eq!(e.wallets.get(&1).unwrap().coins, 7);
        assert_eq!(e.wallets.get(&2).unwrap().coins, 3);
        assert!(!e.transfer(1, 2, 100));
    }

    #[test]
    fn gift_moves_coins_without_trade_prestige() {
        let mut e = Economy::default();
        e.add_coins(1, 20);
        let tp_from_before = e.wallets.get(&1).unwrap().trade_prestige;
        assert!(e.gift(1, 2, 5));
        assert_eq!(e.wallets.get(&1).unwrap().coins, 15);
        assert_eq!(e.wallets.get(&2).unwrap().coins, 5);
        assert_eq!(
            e.wallets.get(&1).unwrap().trade_prestige,
            tp_from_before,
            "gift must not change giver trade_prestige"
        );
        assert_eq!(
            e.wallets.get(&2).unwrap().trade_prestige,
            0.0,
            "gift must not grant trade_prestige to recipient"
        );
        assert!(!e.gift(1, 2, 999));
        assert!(!e.gift(1, 1, 1));
        assert!(!e.gift(1, 2, 0));
    }

    #[test]
    fn donate_tax_and_treasury() {
        let mut e = Economy::default();
        e.add_coins(1, 50);
        assert!(e.donate_to_treasury(1, 20));
        assert_eq!(e.treasury, 20);
        assert_eq!(e.wallets.get(&1).unwrap().coins, 30);
        assert!(e.tax_to_treasury(1, 5));
        assert_eq!(e.treasury, 25);
        assert!(e.pay_from_treasury(2, 10));
        assert_eq!(e.treasury, 15);
        assert_eq!(e.wallets.get(&2).unwrap().coins, 10);
        assert!(!e.donate_to_treasury(1, 999));
    }

    /// WALLET-COINS: pure amount + wallet gift path (no trade prestige).
    // Haxe: GlobalPlayerInstance.takeCoins
    #[test]
    fn take_coins_on_wound_moves_half_plus_one() {
        let mut e = Economy::default();
        e.wallet_mut(10).coins = 10; // target
        e.wallet_mut(1).coins = 0; // attacker
        // Mirrors weapon_wound::coins_stolen_on_wound(10, 0.5, false) = 6
        let amount = 6;
        assert!(e.take_coins_on_wound(1, 10, amount));
        assert_eq!(e.coins_of(1), 6);
        assert_eq!(e.coins_of(10), 4);
        assert_eq!(e.wallets.get(&1).unwrap().trade_prestige, 0.0);
        assert_eq!(e.wallets.get(&10).unwrap().trade_prestige, 0.0);
        // Empty / insufficient target
        assert!(!e.take_coins_on_wound(1, 10, 99));
        assert!(!e.take_coins_on_wound(1, 10, 0));
    }

    #[test]
    fn inherit_to_mother_or_treasury() {
        let mut e = Economy::default();
        e.add_coins(10, 12);
        assert_eq!(e.inherit_on_death(10, Some(20)), 12);
        assert_eq!(e.wallets.get(&10).map(|w| w.coins), Some(0));
        assert_eq!(e.wallets.get(&20).map(|w| w.coins), Some(12));
        assert_eq!(e.treasury, 0);

        e.add_coins(11, 7);
        assert_eq!(e.inherit_on_death(11, None), 7);
        assert_eq!(e.wallets.get(&11).map(|w| w.coins), Some(0));
        assert_eq!(e.treasury, 7);
    }
}
