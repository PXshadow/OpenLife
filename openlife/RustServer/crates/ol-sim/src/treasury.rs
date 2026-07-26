//! Shared treasury helpers for taxes/donations (Haxe village economy subset).
//!
//! Treasury balance lives on [`crate::economy::Economy::treasury`].

use crate::economy::Economy;
use serde::Serialize;
use std::sync::{Arc, RwLock};

/// Shared village treasury for web (`/api/treasury`).
pub type TreasuryView = Arc<RwLock<TreasurySnapshot>>;

/// JSON-friendly treasury snapshot for `/api/treasury`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct TreasurySnapshot {
    pub coins: i32,
}

impl TreasurySnapshot {
    pub fn from_economy(economy: &Economy) -> Self {
        Self {
            coins: economy.treasury,
        }
    }
}

/// Debit `amount` from `from` into `economy.treasury`. Returns false if insufficient.
pub fn donate(economy: &mut Economy, from: i32, amount: i32) -> bool {
    economy.donate_to_treasury(from, amount)
}

/// Leader tax: same as donate (caller enforces leadership).
pub fn tax(economy: &mut Economy, from: i32, amount: i32) -> bool {
    economy.tax_to_treasury(from, amount)
}

/// Pay from treasury to player.
pub fn pay_from_treasury(economy: &mut Economy, to: i32, amount: i32) -> bool {
    economy.pay_from_treasury(to, amount)
}

pub fn format_treasury_query(treasury: i32) -> String {
    format!("TREASURY {treasury}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn donate_and_pay() {
        let mut e = Economy::default();
        e.add_coins(1, 50);
        assert!(donate(&mut e, 1, 20));
        assert_eq!(e.treasury, 20);
        assert_eq!(e.wallets.get(&1).unwrap().coins, 30);
        assert!(pay_from_treasury(&mut e, 2, 10));
        assert_eq!(e.treasury, 10);
        assert_eq!(e.wallets.get(&2).unwrap().coins, 10);
        assert_eq!(format_treasury_query(e.treasury), "TREASURY 10");
        assert!(!donate(&mut e, 1, 999));
    }

    #[test]
    fn snapshot_coins() {
        let mut e = Economy::default();
        e.treasury = 42;
        assert_eq!(TreasurySnapshot::from_economy(&e).coins, 42);
    }
}
