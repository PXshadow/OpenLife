//! OHOL ticket-server account verification (Haxe `Connection.verifyAccount`).
//!
//! Mirrors: `https://onehouronelife.com/ticketServer/server.php?action=check_ticket_hash&...`
//! Response body must be exactly `VALID` when the account is accepted.

use tracing::{info, warn};

/// Verify email + account_key_hash against the ticket server.
/// Returns `true` only when the body is exactly `VALID` (Haxe parity).
pub async fn verify_ohol_ticket(
    base_url: &str,
    email: &str,
    account_key_hash: &str,
    challenge: &str,
) -> bool {
    let url = format!(
        "{base_url}?action=check_ticket_hash&email={}&hash_value={}&string_to_hash={}",
        urlencoding_lite(email),
        urlencoding_lite(account_key_hash),
        urlencoding_lite(challenge),
    );

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, "ticket client build failed");
            return false;
        }
    };

    match client.get(&url).send().await {
        Ok(resp) => match resp.text().await {
            Ok(body) => {
                let body = body.trim();
                let ok = body == "VALID";
                info!(
                    email,
                    valid = ok,
                    response = %body.chars().take(80).collect::<String>(),
                    "ticket verification"
                );
                ok
            }
            Err(e) => {
                warn!(error = %e, "ticket response body error");
                false
            }
        },
        Err(e) => {
            warn!(error = %e, "ticket HTTP request failed");
            false
        }
    }
}

/// Minimal URL encoding for query values (email / hashes are mostly safe).
fn urlencoding_lite(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'@' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push(char::from(b"0123456789ABCDEF"[(b >> 4) as usize]));
                out.push(char::from(b"0123456789ABCDEF"[(b & 0xf) as usize]));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_specials() {
        let e = urlencoding_lite("a b");
        assert!(e.contains("%20"));
    }
}
