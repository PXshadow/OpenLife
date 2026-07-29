//! Helpers to poll fertility due births + ready twin parties (wired from tick_vitals).
//!
//! // Haxe: TimeHelper player-slice birth due; Connection twin waiting (TODO in Haxe)
//! // TWIN-PARTY-RESID: wait timeout eviction + PS polish

use crate::fertility::FertilityState;
use crate::twins::{ReadyTwinParty, TwinWaitQueue};

/// Default twin-wait timeout (keep in sync with [`crate::twin_heart::TWIN_WAIT_TIMEOUT_SECS`]).
// TWIN-PARTY-RESID — duplicated so this module compiles before mod twin_heart is declared
const TWIN_WAIT_TIMEOUT_SECS: f32 = 300.0;

/// Mothers whose gestation completed this poll.
pub fn due_mothers(fertility: &mut FertilityState, sim_time: f32) -> Vec<i32> {
    fertility.poll_due(sim_time)
}

/// Format a private PS line when a twin party becomes ready.
// TWIN-PARTY-RESID PS polish
pub fn format_twin_party_ready(party: &ReadyTwinParty) -> String {
    let ids: Vec<String> = party
        .members
        .iter()
        .map(|m| m.conn_id.to_string())
        .collect();
    let code = if party.code_hash.len() > 8 {
        &party.code_hash[..8]
    } else {
        &party.code_hash
    };
    format!(
        "TWINREADY code={code} count={} members={}",
        party.twin_count,
        ids.join(",")
    )
}

/// Snapshot of waiting status for one conn (for PS after join).
// TWIN-PARTY-RESID PS polish
pub fn format_twin_wait_ps(have: usize, need: i32) -> String {
    format!("TWINWAIT have={have}/{need}")
}

/// Evict waiters older than [`TWIN_WAIT_TIMEOUT_SECS`]. Returns timed-out conn_ids.
///
/// // Haxe: never had twins; product wait-queue edge (TWIN-PARTY-RESID)
pub fn poll_twin_timeouts(wait: &mut TwinWaitQueue, sim_time: f32) -> Vec<u64> {
    wait.poll_timeouts(sim_time, TWIN_WAIT_TIMEOUT_SECS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fertility::{FertilityState, GESTATION_SECS};
    use crate::twins::{TwinJoinOutcome, TwinWaitQueue};

    #[test]
    fn due_after_gestation() {
        let mut f = FertilityState::default();
        f.start_gestation(5, 0.0);
        assert!(due_mothers(&mut f, 1.0).is_empty());
        assert_eq!(due_mothers(&mut f, GESTATION_SECS), vec![5]);
    }

    #[test]
    fn twin_ready_format() {
        let mut q = TwinWaitQueue::new();
        let _ = q.join("abcdefghij", 2, 1, "a", 0.0);
        match q.join("abcdefghij", 2, 2, "b", 1.0) {
            TwinJoinOutcome::Ready(p) => {
                let s = format_twin_party_ready(&p);
                assert!(s.starts_with("TWINREADY "), "{s}");
                assert!(s.contains("count=2"), "{s}");
                assert!(s.contains("members=1,2"), "{s}");
                assert!(s.contains("code=abcdefgh"), "{s}");
            }
            o => panic!("expected Ready: {o:?}"),
        }
    }

    #[test]
    fn twin_wait_ps() {
        assert_eq!(format_twin_wait_ps(1, 3), "TWINWAIT have=1/3");
    }

    #[test]
    fn twin_timeout_poll() {
        let mut q = TwinWaitQueue::new();
        let _ = q.join("z", 2, 7, "e", 0.0);
        assert!(poll_twin_timeouts(&mut q, 10.0).is_empty());
        let gone = poll_twin_timeouts(&mut q, TWIN_WAIT_TIMEOUT_SECS + 0.1);
        assert_eq!(gone, vec![7u64]);
    }
}
