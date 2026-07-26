//! Pure debt ledger: who owes whom (no I/O, no economy mutation).
//!
//! Keyed by `(debtor, creditor) → amount`. `SAY LOAN` records after a coin move;
//! `SAY REPAY` reduces after a coin move. Callers own wallet changes.

use std::collections::HashMap;

/// Tracks outstanding loans between players.
///
/// Map key is `(debtor, creditor)` — how much `debtor` still owes `creditor`.
#[derive(Debug, Default, Clone)]
pub struct DebtBook {
    pub debts: HashMap<(i32, i32), i32>,
}

impl DebtBook {
    /// Amount `debtor` currently owes `creditor` (0 if none).
    pub fn owed(&self, debtor: i32, creditor: i32) -> i32 {
        self.debts
            .get(&(debtor, creditor))
            .copied()
            .unwrap_or(0)
            .max(0)
    }

    /// Total amount `debtor` still owes to all creditors.
    pub fn total_owed_by(&self, debtor: i32) -> i32 {
        self.debts
            .iter()
            .filter(|((d, _), _)| *d == debtor)
            .map(|(_, a)| (*a).max(0))
            .sum()
    }

    /// Total amount all debtors still owe `creditor`.
    pub fn total_owed_to(&self, creditor: i32) -> i32 {
        self.debts
            .iter()
            .filter(|((_, c), _)| *c == creditor)
            .map(|(_, a)| (*a).max(0))
            .sum()
    }

    /// Record a new loan: `debtor` owes `amount` more to `creditor`.
    ///
    /// Returns `Err` on invalid ids / non-positive amount.
    pub fn record_loan(
        &mut self,
        creditor: i32,
        debtor: i32,
        amount: i32,
    ) -> Result<(), &'static str> {
        if amount <= 0 {
            return Err("BAD_AMOUNT");
        }
        if creditor == 0 || debtor == 0 || creditor == debtor {
            return Err("BAD_IDS");
        }
        let e = self.debts.entry((debtor, creditor)).or_insert(0);
        *e = e.saturating_add(amount);
        Ok(())
    }

    /// Reduce debt of `debtor` toward `creditor` by up to `amount`.
    ///
    /// Returns the amount actually applied (may be less than requested if debt
    /// is smaller). Zero / bad ids → `Err`. Removes the map entry when cleared.
    pub fn repay(
        &mut self,
        debtor: i32,
        creditor: i32,
        amount: i32,
    ) -> Result<i32, &'static str> {
        if amount <= 0 {
            return Err("BAD_AMOUNT");
        }
        if creditor == 0 || debtor == 0 || creditor == debtor {
            return Err("BAD_IDS");
        }
        let key = (debtor, creditor);
        let owed = self.debts.get(&key).copied().unwrap_or(0);
        if owed <= 0 {
            return Err("NO_DEBT");
        }
        let applied = amount.min(owed);
        let remain = owed - applied;
        if remain <= 0 {
            self.debts.remove(&key);
        } else {
            self.debts.insert(key, remain);
        }
        Ok(applied)
    }

    /// Chat body for `SAY ?DEBT` without leading p_id.
    ///
    /// Format: `DEBT owe=TOTAL owed_to_me=TOTAL` plus compact pair list when non-empty.
    pub fn format_query(&self, p_id: i32) -> String {
        let owe = self.total_owed_by(p_id);
        let owed_to_me = self.total_owed_to(p_id);
        let mut pairs: Vec<String> = Vec::new();
        // Debts I owe: d->c
        for ((d, c), a) in &self.debts {
            if *d == p_id && *a > 0 {
                pairs.push(format!("to{c}:{a}"));
            }
        }
        for ((d, c), a) in &self.debts {
            if *c == p_id && *a > 0 {
                pairs.push(format!("from{d}:{a}"));
            }
        }
        pairs.sort();
        if pairs.is_empty() {
            format!("DEBT owe={owe} owed_to_me={owed_to_me}")
        } else {
            format!(
                "DEBT owe={owe} owed_to_me={owed_to_me} {}",
                pairs.join(" ")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loan_and_partial_repay() {
        let mut b = DebtBook::default();
        assert!(b.record_loan(1, 2, 10).is_ok());
        assert_eq!(b.owed(2, 1), 10);
        assert_eq!(b.total_owed_by(2), 10);
        assert_eq!(b.total_owed_to(1), 10);
        assert_eq!(b.repay(2, 1, 4).unwrap(), 4);
        assert_eq!(b.owed(2, 1), 6);
        assert_eq!(b.repay(2, 1, 100).unwrap(), 6);
        assert_eq!(b.owed(2, 1), 0);
        assert!(b.debts.is_empty());
    }

    #[test]
    fn rejects_bad_loan_and_no_debt() {
        let mut b = DebtBook::default();
        assert_eq!(b.record_loan(1, 1, 5), Err("BAD_IDS"));
        assert_eq!(b.record_loan(1, 2, 0), Err("BAD_AMOUNT"));
        assert_eq!(b.repay(2, 1, 1), Err("NO_DEBT"));
    }

    #[test]
    fn stacks_multiple_loans() {
        let mut b = DebtBook::default();
        b.record_loan(1, 2, 3).unwrap();
        b.record_loan(1, 2, 2).unwrap();
        assert_eq!(b.owed(2, 1), 5);
        b.record_loan(3, 2, 7).unwrap();
        assert_eq!(b.total_owed_by(2), 12);
    }

    #[test]
    fn format_query_lists_pairs() {
        let mut b = DebtBook::default();
        assert_eq!(b.format_query(9), "DEBT owe=0 owed_to_me=0");
        b.record_loan(1, 2, 4).unwrap();
        let q = b.format_query(2);
        assert!(q.contains("owe=4"));
        assert!(q.contains("to1:4"));
        let q1 = b.format_query(1);
        assert!(q1.contains("owed_to_me=4"));
        assert!(q1.contains("from2:4"));
    }
}
