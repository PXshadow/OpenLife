//! LOGIN / RLOGIN construction matching `LivingLifePage.cpp` + `protocol.txt`.
//!
//! ```text
//! LOGIN client_tag email password_hash account_key_hash tutorial_number [twin_code_hash twin_count]#
//! password_hash     = HMAC_SHA1(password, challenge) as lowercase hex
//! account_key_hash  = HMAC_SHA1(pure_account_key, challenge) as lowercase hex
//! pure_account_key  = account key uppercased with hyphens removed
//! ```

use hmac::{Hmac, Mac};
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;

/// Official client tag reserved for Jason's client; headless uses a distinct honest tag.
pub const DEFAULT_CLIENT_TAG: &str = "client_ohol_headless";

/// HMAC-SHA1(key, data) → lowercase hex, same as `hmac_sha1` in minorGems / protocol.txt.
pub fn hmac_sha1_hex(key: &str, data: &str) -> String {
    let mut mac =
        HmacSha1::new_from_slice(key.as_bytes()).expect("HMAC-SHA1 accepts any key length");
    mac.update(data.as_bytes());
    let result = mac.finalize().into_bytes();
    hex::encode(result)
}

/// Account key as used for the login hash: uppercase, hyphens stripped (`getPureAccountKey`).
pub fn pure_account_key(account_key: &str) -> String {
    account_key
        .chars()
        .filter(|c| *c != '-')
        .collect::<String>()
        .to_uppercase()
}

#[derive(Debug, Clone)]
pub struct LoginParams<'a> {
    /// `LOGIN` or `RLOGIN`.
    pub reconnect: bool,
    pub client_tag: &'a str,
    pub email: &'a str,
    pub password: &'a str,
    pub account_key: &'a str,
    /// Challenge string from `SN` message.
    pub challenge: &'a str,
    pub tutorial_number: i32,
    /// Optional twin code (raw string; hashed with SHA1 digest for wire).
    pub twin_code: Option<&'a str>,
    pub twin_count: i32,
    /// Match official client: left-pad email field to 80 characters when email ≤ 80.
    pub pad_email_to_80: bool,
}

impl Default for LoginParams<'_> {
    fn default() -> Self {
        Self {
            reconnect: false,
            client_tag: DEFAULT_CLIENT_TAG,
            email: "blank_email",
            password: "x",
            account_key: "",
            challenge: "",
            tutorial_number: 0,
            twin_code: None,
            twin_count: 0,
            pad_email_to_80: true,
        }
    }
}

/// Build the full LOGIN/RLOGIN line including trailing `#`.
pub fn encode_login(p: &LoginParams<'_>) -> String {
    let word = if p.reconnect { "RLOGIN" } else { "LOGIN" };
    let email = if p.email.is_empty() {
        "blank_email"
    } else {
        p.email
    };
    let email_field = if p.pad_email_to_80 && email.len() <= 80 {
        format!("{email:<80}")
    } else {
        email.to_string()
    };

    let pw_hash = hmac_sha1_hex(p.password, p.challenge);
    let pure_key = pure_account_key(p.account_key);
    let key_hash = hmac_sha1_hex(&pure_key, p.challenge);

    let twin_extra = if let Some(code) = p.twin_code {
        let hash = sha1_hex(code.as_bytes());
        format!(" {hash} {}", p.twin_count)
    } else {
        String::new()
    };

    format!(
        "{word} {} {} {} {} {}{}#",
        p.client_tag, email_field, pw_hash, key_hash, p.tutorial_number, twin_extra
    )
}

fn sha1_hex(data: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_sha1_rfc2104_case1() {
        // RFC 2202 test case 1: key = 20 bytes of 0x0b, data = "Hi There"
        // We use string keys matching OHOL (ASCII password / challenge).
        // Known vector: key="key", data="The quick brown fox jumps over the lazy dog"
        let h = hmac_sha1_hex("key", "The quick brown fox jumps over the lazy dog");
        assert_eq!(h, "de7c9b85b8b78aa6bc8a7a36f70a90701c9db4d9");
    }

    #[test]
    fn pure_account_key_strips_hyphens_and_uppercases() {
        assert_eq!(pure_account_key("ab-cd-ef-12"), "ABCDEF12");
        assert_eq!(pure_account_key("AbC"), "ABC");
    }

    #[test]
    fn encode_login_wire_shape() {
        let line = encode_login(&LoginParams {
            reconnect: false,
            client_tag: "client_ohol_headless",
            email: "test@example.com",
            password: "secret",
            account_key: "AB-CD-EF-GH",
            challenge: "challenge123",
            tutorial_number: 0,
            twin_code: None,
            twin_count: 0,
            pad_email_to_80: false,
        });
        assert!(line.starts_with("LOGIN client_ohol_headless test@example.com "));
        assert!(line.ends_with(" 0#"));
        let pw = hmac_sha1_hex("secret", "challenge123");
        let kh = hmac_sha1_hex("ABCDEFGH", "challenge123");
        assert!(line.contains(&pw));
        assert!(line.contains(&kh));
        // Exactly: LOGIN tag email pwhash keyhash tutorial#
        let body = line.trim_end_matches('#');
        let parts: Vec<&str> = body.split(' ').collect();
        assert_eq!(parts[0], "LOGIN");
        assert_eq!(parts[1], "client_ohol_headless");
        assert_eq!(parts[2], "test@example.com");
        assert_eq!(parts[3], pw);
        assert_eq!(parts[4], kh);
        assert_eq!(parts[5], "0");
        assert_eq!(parts.len(), 6);
    }

    #[test]
    fn encode_rlogin() {
        let line = encode_login(&LoginParams {
            reconnect: true,
            pad_email_to_80: false,
            challenge: "c",
            ..LoginParams::default()
        });
        assert!(line.starts_with("RLOGIN "));
    }

    #[test]
    fn email_padding_matches_official_80() {
        let line = encode_login(&LoginParams {
            email: "a@b.c",
            challenge: "x",
            pad_email_to_80: true,
            ..LoginParams::default()
        });
        // email field is 80 chars between tag and password hash
        let body = line.trim_end_matches('#');
        // LOGIN client_tag <email80> hash hash tutorial
        let after_tag = body
            .strip_prefix("LOGIN client_ohol_headless ")
            .expect("prefix");
        let email_field: String = after_tag.chars().take(80).collect();
        assert_eq!(email_field.len(), 80);
        assert!(email_field.starts_with("a@b.c"));
        assert!(email_field.ends_with(' '));
    }
}
