//! Helpers for matching structured [`SessionEvent`]s in CLI probes.
//!
//! After L-NET-PARSE, MX/NM/PS/LS/FX/HX arrive as typed variants (not only
//! `SessionEvent::Other` raw bodies).

use crate::parse::{MapChange, PlayerName};
use crate::session::SessionEvent;

/// Collect NM display lines and detect version-like names (GROKPLAY / V0.x).
pub fn note_names(ev: &SessionEvent, name_line: &mut String, saw_version: &mut bool) -> bool {
    let SessionEvent::Names(names) = ev else {
        // Legacy: raw Other still possible for unknown shapes
        if let SessionEvent::Other(s) = ev {
            if s.starts_with("NM") {
                let flat = s.replace('\n', " ");
                *name_line = flat.clone();
                check_version_name(&flat, saw_version);
                return true;
            }
        }
        return false;
    };
    for n in names {
        let flat = format_name(n);
        println!("NM: {flat}");
        *name_line = flat.clone();
        check_version_name(&flat, saw_version);
    }
    true
}

fn format_name(n: &PlayerName) -> String {
    if n.last_name.is_empty() {
        format!("NM {} {}", n.player_id, n.first_name)
    } else {
        format!("NM {} {} {}", n.player_id, n.first_name, n.last_name)
    }
}

fn check_version_name(flat: &str, saw_version: &mut bool) {
    if flat.contains("GROKPLAY") && flat.contains('V') {
        *saw_version = true;
    }
    if flat.contains(env!("CARGO_PKG_VERSION"))
        || flat.contains("V0.")
        || flat.contains("V1.")
    {
        *saw_version = true;
    }
}

/// Apply MX transform / moving flags from a session event.
pub fn note_map_changes(
    ev: &SessionEvent,
    saw_transform: &mut bool,
    saw_moving: &mut bool,
) -> bool {
    match ev {
        SessionEvent::MapChanges(changes) => {
            for m in changes {
                apply_mx(m, saw_transform, saw_moving, true);
            }
            true
        }
        SessionEvent::Other(s) if s.starts_with("MX") => {
            // Fallback raw parse
            for line in s.lines().skip(1) {
                if let Some(m) = crate::parse::parse_mx_line(line) {
                    apply_mx(&m, saw_transform, saw_moving, true);
                }
            }
            true
        }
        _ => false,
    }
}

fn apply_mx(m: &MapChange, saw_transform: &mut bool, saw_moving: &mut bool, log: bool) {
    if m.is_transform() {
        *saw_transform = true;
        if log {
            println!(
                "MX transform p_id={}: {} {} {} {}",
                m.player_id, m.x, m.y, m.object_id_raw, m.player_id
            );
        }
    }
    if m.is_moving() {
        *saw_moving = true;
        if log {
            println!(
                "MX moving: {} {} {} {} {} {} {} {}",
                m.x,
                m.y,
                m.floor_id,
                m.object_id_raw,
                m.player_id,
                m.old_x.unwrap_or(0),
                m.old_y.unwrap_or(0),
                m.speed.unwrap_or(0.0)
            );
        }
    }
}

/// True if event is a player-says carrying `needle` in text.
pub fn player_says_contains(ev: &SessionEvent, needle: &str) -> bool {
    match ev {
        SessionEvent::PlayerSays(list) => list.iter().any(|p| p.text.contains(needle)),
        SessionEvent::Other(s) if s.starts_with("PS") => s.contains(needle),
        _ => false,
    }
}

/// Tag for logging bootstrap frames.
pub fn bootstrap_label(ev: &SessionEvent) -> String {
    match ev {
        SessionEvent::Other(s) => s.lines().next().unwrap_or("?").to_string(),
        other => other.tag_str().to_string(),
    }
}
