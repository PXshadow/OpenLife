//! Helpers so logs / wire dumps never print raw credentials.
//!
//! Login still sends full email on the wire (protocol). We only redact **files and stderr**.

/// Mask an email for logs: `ab***@example.com` (empty → empty).
pub fn redact_email(email: &str) -> String {
    let e = email.trim();
    if e.is_empty() {
        return String::new();
    }
    if let Some((user, domain)) = e.split_once('@') {
        let prefix: String = user.chars().take(2).collect();
        let domain = domain.trim();
        if domain.is_empty() {
            return format!("{prefix}***");
        }
        return format!("{prefix}***@{domain}");
    }
    let prefix: String = e.chars().take(2).collect();
    format!("{prefix}***")
}

/// True if `s` looks like a LOGIN / RLOGIN client→server line (may contain email + hashes).
pub fn is_login_wire_line(s: &str) -> bool {
    let t = s.trim_start();
    t.starts_with("LOGIN ") || t.starts_with("RLOGIN ")
}

/// Redact LOGIN/RLOGIN body for transcripts: keep tags, hide email + HMAC hex.
///
/// Wire shape (approx): `LOGIN <tag> <email_field> <pw_hash> <key_hash> <tutorial>…#`
pub fn redact_login_wire(message: &str) -> String {
    let had_hash = message.ends_with('#');
    let raw = message.trim_end_matches('#').trim();
    let mut parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.len() < 5 {
        return if had_hash {
            format!("{raw}#")
        } else {
            raw.to_string()
        };
    }
    // parts[0]=LOGIN/RLOGIN, [1]=client_tag, [2]=email (space-padded still one token),
    // [3]=pw_hash, [4]=key_hash, [5]=tutorial…
    let email_red = redact_email(parts[2]);
    if parts.len() > 3 && looks_like_hex_hash(parts[3]) {
        parts[3] = "(pw_hmac)";
    }
    if parts.len() > 4 && looks_like_hex_hash(parts[4]) {
        parts[4] = "(key_hmac)";
    }
    let mut out = String::new();
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        if i == 2 {
            out.push_str(&email_red);
        } else {
            out.push_str(p);
        }
    }
    if had_hash {
        out.push('#');
    }
    out
}

fn looks_like_hex_hash(s: &str) -> bool {
    let t = s.trim();
    t.len() >= 32 && t.chars().all(|c| c.is_ascii_hexdigit())
}

/// Redact free-form note lines that may embed `email=…`.
pub fn redact_note(text: &str) -> String {
    // email=foo@bar → email=fo***@bar
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find("email=") {
        out.push_str(&rest[..i]);
        out.push_str("email=");
        rest = &rest[i + 6..];
        let end = rest
            .find(|c: char| c.is_whitespace() || c == ',' || c == ';')
            .unwrap_or(rest.len());
        let email = &rest[..end];
        out.push_str(&redact_email(email));
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_email_masks_local_part() {
        assert_eq!(
            redact_email("76561198032560680@steamgames.com"),
            "76***@steamgames.com"
        );
        assert_eq!(redact_email("ab@x.y"), "ab***@x.y");
        assert_eq!(redact_email(""), "");
    }

    #[test]
    fn redact_login_hides_hashes() {
        let line = "LOGIN client_ohol test@e.com deadbeefcafebabe0123456789abcdef deadbeefcafebabe0123456789abcdef 0#";
        let r = redact_login_wire(line);
        assert!(r.contains("te***@e.com"), "{r}");
        assert!(r.contains("(pw_hmac)"), "{r}");
        assert!(r.contains("(key_hmac)"), "{r}");
        assert!(!r.contains("deadbeef"), "{r}");
        assert!(r.ends_with('#'));
    }

    #[test]
    fn redact_note_email() {
        assert_eq!(
            redact_note("user email=a@b.com host=1"),
            "user email=a***@b.com host=1"
        );
    }
}
