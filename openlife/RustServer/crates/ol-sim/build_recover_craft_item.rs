//! Recover truncated `src/craft_item.rs` from Grok session updates.jsonl oldText.
//!
//! Emergency recovery after accidental overwrite of craft_item.rs header only.

use std::path::Path;
use std::process::Command;

pub fn craft_item_intact(src: &Path) -> bool {
    let ci = src.join("craft_item.rs");
    std::fs::read_to_string(ci)
        .map(|t| {
            t.contains("pub struct FailedCraftings")
                && t.contains("pub fn craft_item_helper")
                && t.len() > 20_000
        })
        .unwrap_or(false)
}

/// Run recovery when craft_item.rs is truncated. Safe if already intact.
pub fn recover_if_needed(src: &Path) -> bool {
    if craft_item_intact(src) {
        return true;
    }
    eprintln!("cargo:warning=C-SS-AI-IGNORE recovery: craft_item.rs truncated — restoring from session log");
    let py = src.join("_recover_craft_item.py");
    if py.exists() {
        let status = Command::new("python")
            .arg(&py)
            .current_dir(src)
            .status()
            .or_else(|_| Command::new("python3").arg(&py).current_dir(src).status());
        if let Ok(s) = status {
            if s.success() && craft_item_intact(src) {
                eprintln!("cargo:warning=C-SS-AI-IGNORE recovery: craft_item.rs restored via Python");
                return true;
            }
        }
    }
    if recover_from_session_log_rust(src) {
        eprintln!("cargo:warning=C-SS-AI-IGNORE recovery: craft_item.rs restored via Rust parser");
        return true;
    }
    eprintln!("cargo:warning=C-SS-AI-IGNORE recovery: FAILED to restore craft_item.rs");
    false
}

fn recover_from_session_log_rust(src: &Path) -> bool {
    let home = match std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        Ok(h) => h,
        Err(_) => return false,
    };
    let sessions = Path::new(&home).join(".grok").join("sessions");
    let mut candidates = Vec::new();
    let preferred = sessions
        .join("C%3A%5CUsers%5Cmarti")
        .join("019fac93-5454-77b3-b002-9f68fb9b61a6")
        .join("updates.jsonl");
    if preferred.is_file() {
        candidates.push(preferred);
    }
    let root = sessions.join("C%3A%5CUsers%5Cmarti");
    if root.is_dir() {
        if let Ok(rd) = std::fs::read_dir(&root) {
            let mut dirs: Vec<_> = rd.filter_map(|e| e.ok()).collect();
            dirs.sort_by_key(|e| {
                std::cmp::Reverse(
                    e.metadata()
                        .and_then(|m| m.modified())
                        .ok()
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                )
            });
            for e in dirs.into_iter().take(50) {
                let u = e.path().join("updates.jsonl");
                if u.is_file() && !candidates.contains(&u) {
                    candidates.push(u);
                }
            }
        }
    }
    for updates in candidates {
        if let Ok(text) = std::fs::read_to_string(&updates) {
            for line in text.lines() {
                if !line.contains("craft_item.rs") || !line.contains("oldText") {
                    continue;
                }
                if let Some(body) = extract_old_text(line) {
                    if body.contains("pub struct FailedCraftings")
                        && body.contains("pub fn craft_item_helper")
                        && body.len() > 20_000
                    {
                        let ci = src.join("craft_item.rs");
                        if std::fs::write(&ci, body.as_bytes()).is_ok() {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

fn extract_old_text(line: &str) -> Option<String> {
    let key = "\"oldText\":\"";
    let path_idx = line.find("craft_item.rs")?;
    let search_from = path_idx.saturating_sub(200);
    let rel = line[search_from..].find(key)?;
    let start = search_from + rel + key.len();
    let bytes = line.as_bytes();
    let mut out = String::new();
    let mut i = start;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '\\' {
            if i + 1 >= bytes.len() {
                break;
            }
            let n = bytes[i + 1] as char;
            match n {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                'u' => {
                    // \uXXXX
                    if i + 5 < bytes.len() {
                        let hex = &line[i + 2..i + 6];
                        if let Ok(cp) = u32::from_str_radix(hex, 16) {
                            if let Some(ch) = char::from_u32(cp) {
                                out.push(ch);
                            }
                        }
                        i += 6;
                        continue;
                    }
                }
                other => out.push(other),
            }
            i += 2;
            continue;
        }
        if c == '"' {
            break;
        }
        out.push(c);
        i += 1;
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}
