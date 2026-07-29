//! Same-server twin party heart-link residual (TWIN-PARTY-RESID / twin_wait_edges).
//!
//! OHOL twins plan #10 (jasonrohrer forum): when one party member dies of **murder**,
//! remaining twins are wounded with a "broken heart" and die soon after.
//!
//! Multi-server peer sockets are **out of scope** (parked stub in [`crate::twins`]).
//!
//! // Haxe: Connection.loginHelper `// TODO twins` — product implements protocol residual
//! // OHOL: twins plan step 10 murder → broken heart

use std::collections::HashMap;

/// Wound stacks applied to surviving twins on murder of a sibling.
/// Caps at combat [`crate::combat::MAX_WOUND`] (5).
// OHOL: broken heart wound → die soon after
pub const BROKEN_HEART_WOUND_STACKS: u8 = 5;

/// Sim-seconds a connection may wait in [`crate::twins::TwinWaitQueue`] before eviction.
// Product residual (Haxe never had twins); 5 minutes
pub const TWIN_WAIT_TIMEOUT_SECS: f32 = 300.0;

/// Pure registry of living twin parties after simultaneous birth.
///
/// Maps each `p_id` to the full sibling set (including self). Murder of one
/// member returns the others for broken-heart wound application.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TwinHeartLinks {
    /// p_id → sorted unique sibling p_ids (includes self while living).
    by_player: HashMap<i32, Vec<i32>>,
}

impl TwinHeartLinks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.by_player.is_empty()
    }

    pub fn linked_count(&self) -> usize {
        self.by_player.len()
    }

    /// True when `p_id` is in an active twin party.
    pub fn is_linked(&self, p_id: i32) -> bool {
        self.by_player.contains_key(&p_id)
    }

    /// Sibling p_ids including `p_id`, if linked.
    pub fn party_of(&self, p_id: i32) -> Option<&[i32]> {
        self.by_player.get(&p_id).map(|v| v.as_slice())
    }

    /// Register a born party (2–4 p_ids). Dedups and ignores parties smaller than 2.
    // TWIN-PARTY-RESID: process_ready_twin_party after simultaneous birth
    pub fn register_party(&mut self, p_ids: &[i32]) {
        let mut ids: Vec<i32> = p_ids.iter().copied().filter(|&id| id != 0).collect();
        ids.sort_unstable();
        ids.dedup();
        if ids.len() < 2 {
            return;
        }
        for &id in &ids {
            self.by_player.insert(id, ids.clone());
        }
    }

    /// On death of a party member: remove them and return **other** still-linked
    /// sibling p_ids (for broken-heart application). Remaining siblings stay linked
    /// to each other when ≥2 left.
    pub fn on_member_death(&mut self, deceased_p_id: i32) -> Vec<i32> {
        let Some(party) = self.by_player.remove(&deceased_p_id) else {
            return Vec::new();
        };
        let others: Vec<i32> = party
            .into_iter()
            .filter(|&id| id != deceased_p_id)
            .collect();
        // Rebuild remaining party without deceased.
        let mut remain = others.clone();
        remain.sort_unstable();
        remain.dedup();
        if remain.len() < 2 {
            for id in &remain {
                self.by_player.remove(id);
            }
        } else {
            for &id in &remain {
                self.by_player.insert(id, remain.clone());
            }
        }
        others
    }

    /// Drop a player without returning siblings (disconnect / suicide cleanup).
    pub fn remove_player(&mut self, p_id: i32) {
        let _ = self.on_member_death(p_id);
    }
}

/// True when a death reason string is **murder** (illegal combat kill).
///
/// - `reason_killed` / `reason_killed_<objectId>` → true  
/// - `reason_killed_legal` / suicide / hunger / age / disconnect → false  
// OHOL: only murder triggers twin broken heart
pub fn is_murder_death_reason(reason: &str) -> bool {
    let t = reason.trim();
    if t == "reason_killed_legal" {
        return false;
    }
    if t == "reason_killed" {
        return true;
    }
    t.starts_with("reason_killed_")
}

/// Private PS body (no leading p_id) when a twin dies of murder.
// TWIN-PARTY-RESID PS polish
pub fn format_twin_heart_ps(deceased_p_id: i32) -> String {
    format!("TWINHEART broken twin={deceased_p_id}")
}

/// Private PS body when a waiter is timed out of the twin queue.
// TWIN-PARTY-RESID PS polish
pub fn format_twin_timeout_ps() -> String {
    "TWINWAIT FAIL timeout".into()
}

/// PS body for join wait with optional short code (polish).
// TWIN-PARTY-RESID PS polish
pub fn format_twin_wait_ps_code(have: usize, need: i32, code: &str) -> String {
    let short = if code.len() > 8 { &code[..8] } else { code };
    if short.is_empty() {
        format!("TWINWAIT have={have}/{need}")
    } else {
        format!("TWINWAIT have={have}/{need} code={short}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_murder_returns_siblings() {
        let mut h = TwinHeartLinks::new();
        h.register_party(&[10, 20, 30]);
        assert!(h.is_linked(10));
        assert_eq!(h.linked_count(), 3);
        let others = h.on_member_death(20);
        assert_eq!(others, vec![10, 30]);
        assert!(!h.is_linked(20));
        assert!(h.is_linked(10));
        // 10 and 30 still linked to each other
        assert_eq!(h.party_of(10), Some([10, 30].as_slice()));
        let last = h.on_member_death(10);
        assert_eq!(last, vec![30]);
        assert!(!h.is_linked(30), "singleton party dissolved");
    }

    #[test]
    fn small_party_ignored() {
        let mut h = TwinHeartLinks::new();
        h.register_party(&[1]);
        assert!(h.is_empty());
        h.register_party(&[1, 2]);
        assert_eq!(h.linked_count(), 2);
    }

    #[test]
    fn murder_reason_gate() {
        assert!(is_murder_death_reason("reason_killed"));
        assert!(is_murder_death_reason("reason_killed_99"));
        assert!(!is_murder_death_reason("reason_killed_legal"));
        assert!(!is_murder_death_reason("reason_suicide"));
        assert!(!is_murder_death_reason("reason_hunger"));
        assert!(!is_murder_death_reason("reason_age"));
    }

    #[test]
    fn ps_formats() {
        assert_eq!(
            format_twin_heart_ps(42),
            "TWINHEART broken twin=42"
        );
        assert_eq!(format_twin_timeout_ps(), "TWINWAIT FAIL timeout");
        assert_eq!(
            format_twin_wait_ps_code(1, 2, "abcdefghij"),
            "TWINWAIT have=1/2 code=abcdefgh"
        );
        assert_eq!(
            format_twin_wait_ps_code(1, 2, ""),
            "TWINWAIT have=1/2"
        );
    }
}
