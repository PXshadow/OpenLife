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
    Ok(AccountRecord {
        email: normalize_email(&email),
        lives,
        total_score,
        total_kills,
        total_deaths,
        last_name,
        last_p_id,
        lifetime_coins,
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

    fn sample_book() -> AccountBook {
        let mut b = AccountBook::default();
        b.on_spawn("Ada@X.COM", 7, "Ada Snow");
        b.on_death("ada@x.com", 15, 2, 1, 3);
        b.on_spawn("bob@y.z", 9, "Bob Bell");
        b.on_death("bob@y.z", 4, 0, 1, 1);
        b
    }

    #[test]
    fn roundtrip_preserves_records() {
        let dir = unique_temp_dir("ol_account_persist_test");
        let path = dir.join("accounts_v1.bin");

        let original = sample_book();
        save_accounts(&original, &path).unwrap();
        let loaded = load_accounts(&path).unwrap();

        assert_eq!(loaded.len(), 2);
        let ada = loaded.get("ada@x.com").unwrap();
        assert_eq!(ada.lives, 1);
        assert_eq!(ada.total_score, 15);
        assert_eq!(ada.total_kills, 2);
        assert_eq!(ada.total_deaths, 1);
        assert_eq!(ada.lifetime_coins, 3);
        assert_eq!(ada.last_name, "Ada Snow");
        assert_eq!(ada.last_p_id, 7);

        let bob = loaded.get("bob@y.z").unwrap();
        assert_eq!(bob.lives, 1);
        assert_eq!(bob.total_score, 4);
        assert_eq!(bob.last_name, "Bob Bell");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn book_helpers_roundtrip() {
        let dir = unique_temp_dir("ol_account_helpers_test");
        let path = dir.join("accounts_v1.bin");

        let b = sample_book();
        b.save_accounts_file(&path).unwrap();

        let mut other = AccountBook::default();
        other.load_accounts_file(&path).unwrap();
        assert_eq!(other.len(), 2);
        assert_eq!(other.get("ada@x.com").unwrap().total_score, 15);

        let from = AccountBook::from_accounts_file(&path).unwrap();
        assert_eq!(from.get("bob@y.z").unwrap().last_name, "Bob Bell");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_bad_magic() {
        let dir = unique_temp_dir("ol_account_bad_magic");
        let path = dir.join("bad.bin");
        std::fs::write(&path, b"XXXX").unwrap();
        assert!(load_accounts(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_roundtrip() {
        let dir = unique_temp_dir("ol_account_empty");
        let path = dir.join("empty.bin");
        let b = AccountBook::default();
        save_accounts(&b, &path).unwrap();
        let loaded = load_accounts(&path).unwrap();
        assert!(loaded.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Stress: 20 OLA1 accounts save → load preserves all fields.
    #[test]
    fn stress_roundtrip_20_accounts() {
        let dir = unique_temp_dir("ol_account_stress_20");
        let path = dir.join("accounts_v1.bin");

        let mut original = AccountBook::default();
        for i in 0..20 {
            let email = format!("user{i}@stress.test");
            let name = format!("User {i}");
            original.on_spawn(&email, 100 + i, &name);
            original.on_death(&email, i * 3, (i % 5) as u32, 1, i * 2);
        }
        assert_eq!(original.len(), 20);

        save_accounts(&original, &path).unwrap();
        // Second save overwrites cleanly.
        save_accounts(&original, &path).unwrap();
        let loaded = load_accounts(&path).unwrap();

        assert_eq!(loaded.len(), 20);
        for i in 0..20 {
            let email = format!("user{i}@stress.test");
            let a = original.get(&email).unwrap();
            let b = loaded.get(&email).expect("missing email after load");
            assert_eq!(b.email, a.email);
            assert_eq!(b.lives, a.lives);
            assert_eq!(b.total_score, a.total_score);
            assert_eq!(b.total_kills, a.total_kills);
            assert_eq!(b.total_deaths, a.total_deaths);
            assert_eq!(b.last_p_id, a.last_p_id);
            assert_eq!(b.lifetime_coins, a.lifetime_coins);
            assert_eq!(b.last_name, a.last_name);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
