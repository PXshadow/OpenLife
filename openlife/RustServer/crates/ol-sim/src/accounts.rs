//! Soft account / soul identity (Haxe PlayerAccount / PlayerSoul subset).
//!
//! No SQL — in-memory map keyed by normalized email. Tracks life count,
//! total score across lives, and last display name for web / bootstrap.

use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Shared soft-account list for web (`/api/accounts`).
pub type AccountView = Arc<RwLock<AccountBookSnapshot>>;

/// One account row for JSON APIs.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AccountSummary {
    pub email: String,
    pub lives: u32,
    pub total_score: i32,
    pub total_kills: u32,
    pub total_deaths: u32,
    pub last_name: String,
    pub last_p_id: i32,
    pub lifetime_coins: i32,
}

/// Book snapshot for `/api/accounts`.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AccountBookSnapshot {
    pub accounts: Vec<AccountSummary>,
    pub count: usize,
}

/// One soft account row.
#[derive(Debug, Clone, Default)]
pub struct AccountRecord {
    pub email: String,
    pub lives: u32,
    pub total_score: i32,
    pub total_kills: u32,
    pub total_deaths: u32,
    pub last_name: String,
    pub last_p_id: i32,
    /// Cumulative coins earned (not current wallet).
    pub lifetime_coins: i32,
}

impl AccountRecord {
    pub fn new(email: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            ..Default::default()
        }
    }

    /// Chat / web summary line.
    pub fn summary_line(&self) -> String {
        format!(
            "ACCOUNT {} lives={} score={} kills={} deaths={} name=\"{}\"",
            self.email, self.lives, self.total_score, self.total_kills, self.total_deaths, self.last_name
        )
    }
}

/// Normalize email for map keys (trim + lowercase).
pub fn normalize_email(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

/// Session account book.
#[derive(Debug, Default, Clone)]
pub struct AccountBook {
    pub by_email: HashMap<String, AccountRecord>,
}

impl AccountBook {
    pub fn ensure(&mut self, email: &str) -> &mut AccountRecord {
        let key = normalize_email(email);
        self.by_email
            .entry(key.clone())
            .or_insert_with(|| AccountRecord::new(key))
    }

    pub fn get(&self, email: &str) -> Option<&AccountRecord> {
        self.by_email.get(&normalize_email(email))
    }

    /// Record a new life spawn / login.
    pub fn on_spawn(&mut self, email: &str, p_id: i32, display_name: &str) {
        let r = self.ensure(email);
        r.lives = r.lives.saturating_add(1);
        r.last_p_id = p_id;
        if !display_name.is_empty() {
            r.last_name = display_name.to_string();
        }
    }

    /// Fold end-of-life stats into the account.
    pub fn on_death(
        &mut self,
        email: &str,
        score: i32,
        kills: u32,
        deaths: u32,
        coins_earned: i32,
    ) {
        let r = self.ensure(email);
        r.total_score = r.total_score.saturating_add(score);
        r.total_kills = r.total_kills.saturating_add(kills);
        r.total_deaths = r.total_deaths.saturating_add(deaths.max(1));
        r.lifetime_coins = r.lifetime_coins.saturating_add(coins_earned);
    }

    /// `SAY ?ACCOUNT` body without leading p_id.
    pub fn format_query(&self, email: &str) -> String {
        match self.get(email) {
            Some(r) => r.summary_line(),
            None => format!("ACCOUNT {} lives=0 score=0", normalize_email(email)),
        }
    }

    pub fn len(&self) -> usize {
        self.by_email.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_email.is_empty()
    }

    /// Soft-account summary list for web (sorted by email).
    pub fn snapshot(&self) -> AccountBookSnapshot {
        let mut accounts: Vec<AccountSummary> = self
            .by_email
            .values()
            .map(|r| AccountSummary {
                email: r.email.clone(),
                lives: r.lives,
                total_score: r.total_score,
                total_kills: r.total_kills,
                total_deaths: r.total_deaths,
                last_name: r.last_name.clone(),
                last_p_id: r.last_p_id,
                lifetime_coins: r.lifetime_coins,
            })
            .collect();
        accounts.sort_by(|a, b| a.email.cmp(&b.email));
        let count = accounts.len();
        AccountBookSnapshot { accounts, count }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_and_spawn() {
        let mut b = AccountBook::default();
        b.on_spawn("  Ada@X.COM ", 7, "Ada Snow");
        let r = b.get("ada@x.com").unwrap();
        assert_eq!(r.lives, 1);
        assert_eq!(r.last_p_id, 7);
        assert_eq!(r.last_name, "Ada Snow");
        b.on_spawn("ada@x.com", 8, "Ada Snow");
        assert_eq!(b.get("ada@x.com").unwrap().lives, 2);
    }

    #[test]
    fn death_folds_stats() {
        let mut b = AccountBook::default();
        b.on_spawn("a@b.c", 1, "A");
        b.on_death("a@b.c", 15, 2, 1, 3);
        let r = b.get("a@b.c").unwrap();
        assert_eq!(r.total_score, 15);
        assert_eq!(r.total_kills, 2);
        assert_eq!(r.total_deaths, 1);
        assert_eq!(r.lifetime_coins, 3);
        assert!(b.format_query("a@b.c").contains("lives=1"));
    }

    #[test]
    fn missing_account_query() {
        let b = AccountBook::default();
        assert!(b.format_query("nobody@x").contains("lives=0"));
    }
}
