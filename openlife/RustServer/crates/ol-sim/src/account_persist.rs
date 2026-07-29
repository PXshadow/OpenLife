//! Versioned binary soft-account save/load (no SQL).
//!
//! Format `OLA1` (u32 version = ACCOUNT_FORMAT_VERSION):
//! ```text
//! magic[4] = b"OLA1"
//! version: u32 LE
//! count: u32 LE
//! records × count:
//!   lives: u32 LE
//!   total_score: i32 LE
//!   total_kills: u32 LE
//!   total_deaths: u32 LE
//!   last_p_id: i32 LE
//!   lifetime_coins: i32 LE
//!   email_len: u32 LE
//!   email: [u8; email_len]  (UTF-8, already normalized)
//!   name_len: u32 LE
//!   last_name: [u8; name_len]  (UTF-8)
//! ```
//!
//! Session-only fields (`display_yum`, `coins_inherited`, `graves`) are not on disk.

use crate::accounts::{normalize_email, AccountBook, AccountRecord};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::time::Instant;
use tracing::info;

pub const ACCOUNT_FORMAT_VERSION: u32 = 1;
const MAGIC: &[u8; 4] = b"OLA1";

/// Default on-disk name under the save directory.
pub const DEFAULT_ACCOUNT_FILE: &str = "accounts_v1.bin";

/// Write all soft accounts from `book` to `path` (atomic-ish via temp rename).
pub fn save_accounts(book: &AccountBook, path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    let t0 = Instant::now();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("bin.tmp");
    {
        let f = File::create(&tmp).map_err(|e| e.to_string())?;
        let mut w = BufWriter::with_capacity(64 * 1024, f);
        write_accounts(book, &mut w)?;
        w.flush().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    info!(
        path = %path.display(),
        count = book.len(),
        ms = t0.elapsed().as_millis() as u64,
        "accounts saved"
    );
    Ok(())
}

/// Load soft accounts from `path` into a fresh [`AccountBook`].
pub fn load_accounts(path: impl AsRef<Path>) -> Result<AccountBook, String> {
    let path = path.as_ref();
    let t0 = Instant::now();
    let f = File::open(path).map_err(|e| e.to_string())?;
    let mut r = BufReader::with_capacity(64 * 1024, f);
    let book = read_accounts(&mut r)?;
    info!(
        path = %path.display(),
        count = book.len(),
        ms = t0.elapsed().as_millis() as u64,
        "accounts loaded"
    );
    Ok(book)
}

fn write_accounts(book: &AccountBook, w: &mut impl Write) -> Result<(), String> {
    w.write_all(MAGIC).map_err(|e| e.to_string())?;
    w.write_u32::<LittleEndian>(ACCOUNT_FORMAT_VERSION)
        .map_err(|e| e.to_string())?;
    w.write_u32::<LittleEndian>(book.by_email.len() as u32)
        .map_err(|e| e.to_string())?;

    // Stable order for deterministic files / easier diffs in tests.
    let mut emails: Vec<&String> = book.by_email.keys().collect();
    emails.sort();
    for email in emails {
        let r = book.by_email.get(email).expect("email from keys");
        write_record(r, w)?;
    }
    Ok(())
}

fn write_record(r: &AccountRecord, w: &mut impl Write) -> Result<(), String> {
    w.write_u32::<LittleEndian>(r.lives)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(r.total_score)
        .map_err(|e| e.to_string())?;
    w.write_u32::<LittleEndian>(r.total_kills)
        .map_err(|e| e.to_string())?;
    w.write_u32::<LittleEndian>(r.total_deaths)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(r.last_p_id)
        .map_err(|e| e.to_string())?;
    w.write_i32::<LittleEndian>(r.lifetime_coins)
        .map_err(|e| e.to_string())?;
    let email_bytes = r.email.as_bytes();
    w.write_u32::<LittleEndian>(email_bytes.len() as u32)
        .map_err(|e| e.to_string())?;
    w.write_all(email_bytes).map_err(|e| e.to_string())?;
    let name_bytes = r.last_name.as_bytes();
    w.write_u32::<LittleEndian>(name_bytes.len() as u32)
        .map_err(|e| e.to_string())?;
    w.write_all(name_bytes).map_err(|e| e.to_string())?;
    Ok(())
}

fn read_accounts(r: &mut impl Read) -> Result<AccountBook, String> {
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if &magic != MAGIC {
        return Err(format!("bad account magic {:?}", magic));
    }
    let version = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())?;
    if version != ACCOUNT_FORMAT_VERSION {
        return Err(format!(
            "unsupported account version {version} (want {ACCOUNT_FORMAT_VERSION})"
        ));
    }
    let count = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    let mut by_email = HashMap::with_capacity(count);
    for _ in 0..count {
        let rec = read_record(r)?;
        let key = normalize_email(&rec.email);
        by_email.insert(key, rec);
    }
    Ok(AccountBook { by_email })
}

fn read_record(r: &mut impl Read) -> Result<AccountRecord, String> {
    let lives = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())?;
    let total_score = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let total_kills = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())?;
    let total_deaths = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())?;
    let last_p_id = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let lifetime_coins = r.read_i32::<LittleEndian>().map_err(|e| e.to_string())?;
    let email_len = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    if email_len > 4096 {
        return Err(format!("account email too long ({email_len})"));
    }
    let mut email_buf = vec![0u8; email_len];
    r.read_exact(&mut email_buf).map_err(|e| e.to_string())?;
    let email = String::from_utf8(email_buf).map_err(|e| e.to_string())?;
    let name_len = r.read_u32::<LittleEndian>().map_err(|e| e.to_string())? as usize;
    if name_len > 4096 {
        return Err(format!("account name too long ({name_len})"));
    }
    let mut name_buf = vec![0u8; name_len];
    r.read_exact(&mut name_buf).map_err(|e| e.to_string())?;
    let last_name = String::from_utf8(name_buf).map_err(|e| e.to_string())?;
    let email = normalize_email(&email);
    let is_ai = crate::accounts::account_email_looks_ai(&email);
    Ok(AccountRecord {
        email,
        lives,
        total_score,
        total_kills,
        total_deaths,
        last_name,
        last_p_id,
        lifetime_coins,
        // Haxe displayYum defaults true; not persisted in OLA1 yet.
        display_yum: true,
        // Haxe coinsInherited; not persisted in OLA1 yet.
        coins_inherited: 0.0,
        // Haxe femaleScore / maleScore — session; not OLA1.
        female_score: 0.0,
        male_score: 0.0,
        // Haxe isAi — OLA1 has no flag; email heuristic for permanent AI.
        is_ai,
        // Haxe familyPrestige map — session only (not OLA1).
        family_prestige: std::collections::HashMap::new(),
        // Haxe account.graves — session only; filled by InitObjectHelpersAfterRead.
        graves: Vec::new(),
        // Haxe ScoreEntry queue — OLA1 does not persist; see score_entry.rs.
        score_entries: Vec::new(),
    })
}

impl AccountBook {
    /// Persist soft accounts to disk.
    pub fn save_accounts_file(&self, path: impl AsRef<Path>) -> Result<(), String> {
        save_accounts(self, path)
    }

    /// Replace all rows from file.
    pub fn load_accounts_file(&mut self, path: impl AsRef<Path>) -> Result<(), String> {
        *self = load_accounts(path)?;
        Ok(())
    }

    /// Build an AccountBook from a save file.
    pub fn from_accounts_file(path: impl AsRef<Path>) -> Result<Self, String> {
        load_accounts(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Unique temp dir per call (parallel tests must not share fixed names).
    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = env::temp_dir().join(format!("{prefix}_{t}_{n}"));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn roundtrip_empty() {
        let dir = unique_temp_dir("ola_empty");
        let path = dir.join(DEFAULT_ACCOUNT_FILE);
        let book = AccountBook::default();
        save_accounts(&book, &path).unwrap();
        let loaded = load_accounts(&path).unwrap();
        assert!(loaded.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn roundtrip_one() {
        let dir = unique_temp_dir("ola_one");
        let path = dir.join(DEFAULT_ACCOUNT_FILE);
        let mut book = AccountBook::default();
        book.on_spawn("A@B.C", 3, "Ada");
        book.on_death("a@b.c", 10, 1, 1, 5);
        save_accounts(&book, &path).unwrap();
        let loaded = load_accounts(&path).unwrap();
        let r = loaded.get("a@b.c").unwrap();
        assert_eq!(r.lives, 1);
        assert_eq!(r.total_score, 10);
        assert_eq!(r.last_name, "Ada");
        assert_eq!(r.lifetime_coins, 5);
        assert!((r.coins_inherited - 0.0).abs() < 1e-6);
        assert!(r.graves.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
