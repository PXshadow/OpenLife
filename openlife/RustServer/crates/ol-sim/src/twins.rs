//! Twin systems: multi-server peer registry + twin-code birth waiting queue.
//!
//! ## Multi-server peers ([`TwinRegistry`])
//! Configured peers are listed for diagnostics (`SAY ?TWINS`). Peer list is
//! seeded from `server.toml` `twin_peers` at boot and re-synced on LiveSettings
//! hot-reload. Pongs are applied via [`TwinRegistry::record_pong`] (live Raw
//! `TWINPONG` path or future inter-server sockets). Stale pongs age out via
//! [`TwinRegistry::clear_stale_pongs`] on the sim tick.
//!
//! **Residual:** no TCP/UDP sockets to peer endpoints, no LOGIN handoff / shared
//! wait queue across servers. Multi-server work is **parked**.
//!
//! ## Twin-code waiting queue ([`TwinWaitQueue`])
//! Protocol: `LOGIN … twin_code_hash twin_count` (OHOL protocol.txt).
//! Friends share a code + party size (2–4). Connections wait until the party
//! fills, then the sim births them together (same mother or twin-Eve cluster).
//!
//! Haxe OpenLife left this as `// TODO twins` on login — Rust implements the
//! protocol-documented waiting queue (product goal; intentional vs Haxe TODO).
//!
//! ## Same-server residual ([`crate::twin_heart`])
//! Murder of a twin party member wounds siblings (broken heart). See TWIN-PARTY-RESID.

use std::collections::HashMap;

// ── Multi-server peer registry ───────────────────────────────────────────────

/// Default sim-seconds after which a peer pong is considered stale.
///
/// // Haxe: no multi-server twin health — Rust product default for ?TWINS aging
pub const DEFAULT_PEER_STALE_SECS: f32 = 60.0;

/// One configured twin peer endpoint.
///
/// `last_pong` is sim-time of the last successful pong when networking or a
/// test/inject path records one.
#[derive(Debug, Clone, PartialEq)]
pub struct TwinPeer {
    pub host: String,
    pub port: u16,
    /// Last pong sim-time seconds, if any (and not yet aged out).
    pub last_pong: Option<f32>,
}

impl TwinPeer {
    /// New peer with no pong recorded.
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            last_pong: None,
        }
    }

    /// `host:port` display form.
    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// True when `last_pong` is present and younger than `stale_after` at `sim_time`.
    #[inline]
    pub fn is_fresh(&self, sim_time: f32, stale_after: f32) -> bool {
        match self.last_pong {
            Some(t) if stale_after.is_finite() && stale_after >= 0.0 => {
                (sim_time - t) <= stale_after
            }
            _ => false,
        }
    }
}

/// Pure in-memory twin peer list (sockets optional / residual).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TwinRegistry {
    peers: Vec<TwinPeer>,
}

impl TwinRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build from `(host, port)` pairs; `last_pong` starts as `None`.
    /// Duplicate host:port pairs are dropped (first wins).
    pub fn from_endpoints<H, I>(endpoints: I) -> Self
    where
        H: Into<String>,
        I: IntoIterator<Item = (H, u16)>,
    {
        let mut reg = Self::new();
        for (h, p) in endpoints {
            reg.add(TwinPeer::new(h, p));
        }
        reg
    }

    pub fn peers(&self) -> &[TwinPeer] {
        &self.peers
    }

    pub fn len(&self) -> usize {
        self.peers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }

    /// Whether a peer with this host:port is registered.
    pub fn contains(&self, host: &str, port: u16) -> bool {
        self.peers.iter().any(|p| p.host == host && p.port == port)
    }

    /// Append a peer. Duplicate host:port is a no-op (preserves existing `last_pong`).
    pub fn add(&mut self, peer: TwinPeer) {
        if self.contains(&peer.host, peer.port) {
            return;
        }
        self.peers.push(peer);
    }

    /// Replace the peer list from config endpoints, preserving `last_pong` for
    /// host:port pairs that remain. Dedups desired endpoints (first wins).
    ///
    /// Returns `true` when the set of endpoints (host:port, order-insensitive
    /// membership + order of first occurrence) differs from before.
    // TWIN-MULTI-SERVER: hot-reload / LiveSettings twin_peers re-sync
    pub fn sync_endpoints<H, I>(&mut self, endpoints: I) -> bool
    where
        H: Into<String>,
        I: IntoIterator<Item = (H, u16)>,
    {
        let mut desired: Vec<(String, u16)> = Vec::new();
        for (h, p) in endpoints {
            let host = h.into();
            if desired.iter().any(|(eh, ep)| eh == &host && *ep == p) {
                continue;
            }
            desired.push((host, p));
        }

        let old_keys: Vec<(String, u16)> = self
            .peers
            .iter()
            .map(|p| (p.host.clone(), p.port))
            .collect();
        let changed = old_keys.len() != desired.len()
            || old_keys
                .iter()
                .zip(desired.iter())
                .any(|((oh, op), (nh, np))| oh != nh || op != np);

        let mut next = Vec::with_capacity(desired.len());
        for (host, port) in desired {
            let last_pong = self
                .peers
                .iter()
                .find(|p| p.host == host && p.port == port)
                .and_then(|p| p.last_pong);
            next.push(TwinPeer {
                host,
                port,
                last_pong,
            });
        }
        self.peers = next;
        changed
    }

    /// Record a pong at sim-time `at` for the first matching host:port.
    /// Returns false if no peer matched.
    pub fn record_pong(&mut self, host: &str, port: u16, at: f32) -> bool {
        if let Some(p) = self
            .peers
            .iter_mut()
            .find(|p| p.host == host && p.port == port)
        {
            p.last_pong = Some(at);
            true
        } else {
            false
        }
    }

    /// Clear `last_pong` when older than `stale_after` sim-seconds at `sim_time`.
    ///
    /// Returns how many peers were aged out. Call from the sim tick so `?TWINS`
    /// shows `@-` after timeout without sockets.
    // TWIN-MULTI-SERVER peer health aging
    pub fn clear_stale_pongs(&mut self, sim_time: f32, stale_after: f32) -> usize {
        if !(stale_after.is_finite() && stale_after >= 0.0) {
            return 0;
        }
        let mut n = 0;
        for p in &mut self.peers {
            if let Some(t) = p.last_pong {
                if (sim_time - t) > stale_after {
                    p.last_pong = None;
                    n += 1;
                }
            }
        }
        n
    }

    /// Endpoints that need a health probe (no pong or already stale).
    ///
    /// Future inter-server sockets iterate this list; pure registry only.
    pub fn ping_targets(&self, sim_time: f32, stale_after: f32) -> Vec<(String, u16)> {
        self.peers
            .iter()
            .filter(|p| !p.is_fresh(sim_time, stale_after))
            .map(|p| (p.host.clone(), p.port))
            .collect()
    }

    /// `SAY ?TWINS` peer section (no leading p_id).
    ///
    /// Empty: `TWINS none`. Otherwise `TWINS host:port@- …` (`@-` = no pong / stale).
    pub fn format_query(&self) -> String {
        if self.peers.is_empty() {
            return "TWINS none".into();
        }
        let list = self
            .peers
            .iter()
            .map(|p| match p.last_pong {
                Some(t) => format!("{}@{t:.2}", p.endpoint()),
                None => format!("{}@-", p.endpoint()),
            })
            .collect::<Vec<_>>()
            .join(" ");
        format!("TWINS {list}")
    }
}

/// Parse `TWINPONG host port` payload. Returns `(host, port)` when valid.
// TWIN-MULTI-SERVER: inject path until inter-server sockets exist
pub fn parse_twin_pong_payload(payload: &str) -> Option<(String, u16)> {
    let mut it = payload.split_whitespace();
    let host = it.next()?.trim();
    if host.is_empty() {
        return None;
    }
    let port: u16 = it.next()?.parse().ok()?;
    if port == 0 {
        return None;
    }
    Some((host.to_string(), port))
}

// ── Twin-code birth waiting queue ───────────────────────────────────────────

/// Min party size (twins).
pub const TWIN_COUNT_MIN: i32 = 2;
/// Max party size (quadruplets per OHOL plan).
pub const TWIN_COUNT_MAX: i32 = 4;

/// One connection waiting on a twin code.
#[derive(Debug, Clone, PartialEq)]
pub struct TwinWaiter {
    pub conn_id: u64,
    pub email: String,
    /// Sim-time when this waiter joined.
    pub joined_at: f32,
}

/// Outcome of joining a twin-code party.
#[derive(Debug, Clone, PartialEq)]
pub enum TwinJoinOutcome {
    /// Party not full yet.
    Waiting { have: usize, need: i32 },
    /// This join filled the party; members ready to birth.
    Ready(ReadyTwinParty),
    /// Invalid twin_count (not in 2..=4).
    InvalidCount,
    /// Empty code hash.
    EmptyCode,
    /// Another waiter already used a different twin_count for this code.
    CountMismatch { expected: i32 },
    /// This conn_id was already waiting (moved / updated).
    AlreadyWaiting { have: usize, need: i32 },
}

/// A full twin party ready for simultaneous birth.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadyTwinParty {
    pub code_hash: String,
    pub twin_count: i32,
    pub members: Vec<TwinWaiter>,
}

/// Pure twin-code waiting queue (no network sockets).
///
/// // Haxe: Connection.loginHelper TODO twins — protocol twin_code_hash twin_count
#[derive(Debug, Default, Clone)]
pub struct TwinWaitQueue {
    /// code_hash → party in progress.
    parties: HashMap<String, PendingParty>,
    /// conn_id → code_hash (for leave / status).
    by_conn: HashMap<u64, String>,
}

#[derive(Debug, Clone)]
struct PendingParty {
    twin_count: i32,
    waiters: Vec<TwinWaiter>,
}

impl TwinWaitQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn waiting_count(&self) -> usize {
        self.by_conn.len()
    }

    pub fn party_count(&self) -> usize {
        self.parties.len()
    }

    pub fn is_empty(&self) -> bool {
        self.parties.is_empty()
    }

    /// Whether `conn_id` is currently waiting.
    pub fn is_waiting(&self, conn_id: u64) -> bool {
        self.by_conn.contains_key(&conn_id)
    }

    /// Status for one connection: `have/need` if waiting.
    pub fn status_for(&self, conn_id: u64) -> Option<(usize, i32, String)> {
        let code = self.by_conn.get(&conn_id)?;
        let p = self.parties.get(code)?;
        Some((p.waiters.len(), p.twin_count, code.clone()))
    }

    /// Join or create a party for `code_hash` with expected `twin_count`.
    ///
    /// When the party reaches `twin_count` members, returns
    /// [`TwinJoinOutcome::Ready`] and removes the party from the queue.
    // Haxe / protocol: LOGIN twin_code_hash twin_count waiting-to-be-born
    pub fn join(
        &mut self,
        code_hash: &str,
        twin_count: i32,
        conn_id: u64,
        email: impl Into<String>,
        sim_time: f32,
    ) -> TwinJoinOutcome {
        let code = code_hash.trim();
        if code.is_empty() {
            return TwinJoinOutcome::EmptyCode;
        }
        if twin_count < TWIN_COUNT_MIN || twin_count > TWIN_COUNT_MAX {
            return TwinJoinOutcome::InvalidCount;
        }

        // Already waiting elsewhere: leave first, then re-join.
        if let Some(prev) = self.by_conn.get(&conn_id).cloned() {
            if prev == code {
                if let Some(p) = self.parties.get(code) {
                    return TwinJoinOutcome::AlreadyWaiting {
                        have: p.waiters.len(),
                        need: p.twin_count,
                    };
                }
            }
            let _ = self.leave(conn_id);
        }

        let email = email.into();
        let entry = self.parties.entry(code.to_string()).or_insert_with(|| {
            PendingParty {
                twin_count,
                waiters: Vec::new(),
            }
        });

        if entry.twin_count != twin_count {
            return TwinJoinOutcome::CountMismatch {
                expected: entry.twin_count,
            };
        }

        // Dedup same conn (shouldn't happen after leave).
        entry.waiters.retain(|w| w.conn_id != conn_id);
        entry.waiters.push(TwinWaiter {
            conn_id,
            email,
            joined_at: sim_time,
        });
        self.by_conn.insert(conn_id, code.to_string());

        let have = entry.waiters.len();
        let need = entry.twin_count;
        if have as i32 >= need {
            // Party full → detach and return ready.
            let party = self.parties.remove(code).expect("party just joined");
            for w in &party.waiters {
                self.by_conn.remove(&w.conn_id);
            }
            TwinJoinOutcome::Ready(ReadyTwinParty {
                code_hash: code.to_string(),
                twin_count: party.twin_count,
                members: party.waiters,
            })
        } else {
            TwinJoinOutcome::Waiting { have, need }
        }
    }

    /// Remove a waiter (disconnect / cancel). Returns the code if any.
    pub fn leave(&mut self, conn_id: u64) -> Option<String> {
        let code = self.by_conn.remove(&conn_id)?;
        if let Some(p) = self.parties.get_mut(&code) {
            p.waiters.retain(|w| w.conn_id != conn_id);
            if p.waiters.is_empty() {
                self.parties.remove(&code);
            }
        }
        Some(code)
    }

    /// Evict waiters whose `joined_at` is older than `timeout` sim-seconds.
    /// Returns conn_ids removed (for PS `TWINWAIT FAIL timeout`).
    // TWIN-PARTY-RESID twin_wait_edges
    pub fn poll_timeouts(&mut self, sim_time: f32, timeout: f32) -> Vec<u64> {
        if !(timeout.is_finite() && timeout >= 0.0) {
            return Vec::new();
        }
        let mut expired: Vec<u64> = Vec::new();
        for (conn_id, code) in self.by_conn.iter() {
            if let Some(p) = self.parties.get(code) {
                if let Some(w) = p.waiters.iter().find(|w| w.conn_id == *conn_id) {
                    if (sim_time - w.joined_at) > timeout {
                        expired.push(*conn_id);
                    }
                }
            }
        }
        for cid in &expired {
            let _ = self.leave(*cid);
        }
        expired
    }

    /// Compact query line for waiting parties (append to `?TWINS` / `?TWINWAIT`).
    ///
    /// `TWINWAIT none` or `TWINWAIT code=ab have=1/2 code=cd have=2/3`.
    pub fn format_wait_query(&self) -> String {
        if self.parties.is_empty() {
            return "TWINWAIT none".into();
        }
        let mut parts: Vec<String> = self
            .parties
            .iter()
            .map(|(code, p)| {
                let short = if code.len() > 8 { &code[..8] } else { code };
                format!("code={short} have={}/{}", p.waiters.len(), p.twin_count)
            })
            .collect();
        parts.sort();
        format!("TWINWAIT {}", parts.join(" "))
    }

    /// Combined diagnostic: peer list + wait queue.
    pub fn format_twins_full(peers: &TwinRegistry, wait: &TwinWaitQueue) -> String {
        let peer_part = peers.format_query();
        let wait_part = wait.format_wait_query();
        format!("{peer_part} | {wait_part}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_peer_query() {
        let r = TwinRegistry::new();
        assert!(r.is_empty());
        assert_eq!(r.format_query(), "TWINS none");
    }

    #[test]
    fn list_and_pong() {
        let mut r = TwinRegistry::from_endpoints([("127.0.0.1", 8006u16), ("10.0.0.2", 8007)]);
        assert_eq!(r.len(), 2);
        let q = r.format_query();
        assert!(q.starts_with("TWINS "));
        assert!(q.contains("127.0.0.1:8006@-"));
        assert!(q.contains("10.0.0.2:8007@-"));
        assert!(r.record_pong("127.0.0.1", 8006, 12.5));
        assert!(!r.record_pong("nope", 1, 0.0));
        let q2 = r.format_query();
        assert!(q2.contains("127.0.0.1:8006@12.50"), "got {q2}");
        assert_eq!(r.peers()[0].last_pong, Some(12.5));
    }

    #[test]
    fn peer_endpoint() {
        let p = TwinPeer::new("localhost", 9000);
        assert_eq!(p.endpoint(), "localhost:9000");
        assert!(p.last_pong.is_none());
    }

    #[test]
    fn add_dedups_host_port() {
        let mut r = TwinRegistry::new();
        r.add(TwinPeer::new("a", 1));
        r.add(TwinPeer::new("a", 1));
        r.add(TwinPeer::new("a", 2));
        assert_eq!(r.len(), 2);
        assert!(r.record_pong("a", 1, 5.0));
        r.add(TwinPeer::new("a", 1)); // must not clobber pong
        assert_eq!(r.peers()[0].last_pong, Some(5.0));
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn from_endpoints_dedups() {
        let r = TwinRegistry::from_endpoints([
            ("h", 1u16),
            ("h", 1u16),
            ("h", 2u16),
        ]);
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn record_pong_unknown_leaves_peers_unchanged() {
        let mut r = TwinRegistry::from_endpoints([("x", 9u16)]);
        let before = r.clone();
        assert!(!r.record_pong("y", 9, 1.0));
        assert_eq!(r, before);
    }

    #[test]
    fn clear_stale_pongs_ages_out() {
        let mut r = TwinRegistry::from_endpoints([("h", 1u16), ("h", 2u16)]);
        assert!(r.record_pong("h", 1, 10.0));
        assert!(r.record_pong("h", 2, 50.0));
        // sim_time 80, stale 60 → peer@10 is stale (70s), peer@50 still fresh (30s)
        let n = r.clear_stale_pongs(80.0, 60.0);
        assert_eq!(n, 1);
        assert!(r.peers()[0].last_pong.is_none());
        assert_eq!(r.peers()[1].last_pong, Some(50.0));
        let q = r.format_query();
        assert!(q.contains("h:1@-"), "{q}");
        assert!(q.contains("h:2@50.00"), "{q}");
    }

    #[test]
    fn peer_is_fresh_and_ping_targets() {
        let mut r = TwinRegistry::from_endpoints([("a", 1u16), ("b", 2u16)]);
        assert!(r.record_pong("a", 1, 100.0));
        assert!(!r.peers()[0].is_fresh(200.0, 60.0)); // 100s old
        assert!(r.peers()[0].is_fresh(130.0, 60.0));
        let due = r.ping_targets(130.0, 60.0);
        assert_eq!(due, vec![("b".into(), 2u16)]); // only b has no pong
        let due2 = r.ping_targets(200.0, 60.0);
        assert_eq!(due2.len(), 2);
    }

    #[test]
    fn sync_endpoints_preserves_pong_and_removes() {
        let mut r = TwinRegistry::from_endpoints([("keep", 1u16), ("drop", 2u16)]);
        assert!(r.record_pong("keep", 1, 3.0));
        assert!(r.record_pong("drop", 2, 4.0));
        let changed = r.sync_endpoints([("keep", 1u16), ("new", 3u16)]);
        assert!(changed);
        assert_eq!(r.len(), 2);
        assert_eq!(r.peers()[0].last_pong, Some(3.0));
        assert!(r.contains("new", 3));
        assert!(!r.contains("drop", 2));
        // Idempotent same set in same order
        assert!(!r.sync_endpoints([("keep", 1u16), ("new", 3u16)]));
    }

    #[test]
    fn parse_twin_pong_payload_ok() {
        assert_eq!(
            parse_twin_pong_payload("127.0.0.1 8006"),
            Some(("127.0.0.1".into(), 8006))
        );
        assert!(parse_twin_pong_payload("").is_none());
        assert!(parse_twin_pong_payload("host").is_none());
        assert!(parse_twin_pong_payload("host 0").is_none());
        assert!(parse_twin_pong_payload("host xyz").is_none());
    }

    #[test]
    fn twin_wait_fills_to_ready() {
        let mut q = TwinWaitQueue::new();
        let o1 = q.join("abc", 2, 1, "a@x", 0.0);
        assert_eq!(o1, TwinJoinOutcome::Waiting { have: 1, need: 2 });
        assert!(q.is_waiting(1));
        let o2 = q.join("abc", 2, 2, "b@x", 1.0);
        match o2 {
            TwinJoinOutcome::Ready(p) => {
                assert_eq!(p.twin_count, 2);
                assert_eq!(p.members.len(), 2);
                assert_eq!(p.members[0].conn_id, 1);
                assert_eq!(p.members[1].conn_id, 2);
            }
            other => panic!("expected Ready, got {other:?}"),
        }
        assert!(q.is_empty());
        assert!(!q.is_waiting(1));
    }

    #[test]
    fn twin_wait_invalid_and_mismatch() {
        let mut q = TwinWaitQueue::new();
        assert_eq!(
            q.join("", 2, 1, "a", 0.0),
            TwinJoinOutcome::EmptyCode
        );
        assert_eq!(
            q.join("x", 1, 1, "a", 0.0),
            TwinJoinOutcome::InvalidCount
        );
        assert_eq!(
            q.join("x", 5, 1, "a", 0.0),
            TwinJoinOutcome::InvalidCount
        );
        let _ = q.join("code", 3, 1, "a", 0.0);
        assert_eq!(
            q.join("code", 2, 2, "b", 0.0),
            TwinJoinOutcome::CountMismatch { expected: 3 }
        );
    }

    #[test]
    fn twin_wait_leave_and_status() {
        let mut q = TwinWaitQueue::new();
        let _ = q.join("zz", 3, 10, "a", 0.0);
        let _ = q.join("zz", 3, 11, "b", 0.0);
        assert_eq!(q.status_for(10), Some((2, 3, "zz".into())));
        assert_eq!(q.leave(10), Some("zz".into()));
        assert_eq!(q.status_for(11), Some((1, 3, "zz".into())));
        assert_eq!(q.format_wait_query(), "TWINWAIT code=zz have=1/3");
        let _ = q.leave(11);
        assert_eq!(q.format_wait_query(), "TWINWAIT none");
    }

    #[test]
    fn twin_wait_already_waiting_same_code() {
        let mut q = TwinWaitQueue::new();
        let _ = q.join("c", 2, 1, "a", 0.0);
        assert_eq!(
            q.join("c", 2, 1, "a", 1.0),
            TwinJoinOutcome::AlreadyWaiting { have: 1, need: 2 }
        );
    }

    #[test]
    fn twin_poll_timeouts_evicts() {
        let mut q = TwinWaitQueue::new();
        let _ = q.join("t", 2, 1, "a", 0.0);
        assert!(q.poll_timeouts(10.0, 300.0).is_empty());
        assert_eq!(q.poll_timeouts(301.0, 300.0), vec![1u64]);
        assert!(!q.is_waiting(1));
    }

    #[test]
    fn format_twins_full_combines() {
        let peers = TwinRegistry::from_endpoints([("h", 1u16)]);
        let mut wait = TwinWaitQueue::new();
        let _ = wait.join("ab", 2, 1, "e", 0.0);
        let s = TwinWaitQueue::format_twins_full(&peers, &wait);
        assert!(s.contains("TWINS h:1@-"), "{s}");
        assert!(s.contains("TWINWAIT"), "{s}");
    }
}
