//! Multi-server twin peer registry (**stub only** — no network I/O).
//!
//! Configured peers are listed for diagnostics (`SAY ?TWINS`). There is no
//! inter-server ping, handoff, or shared-world sync yet.

/// One configured twin peer endpoint.
///
/// `last_pong` is sim-time of the last successful pong when/if networking is
/// wired; the stub never updates it automatically.
#[derive(Debug, Clone, PartialEq)]
pub struct TwinPeer {
    pub host: String,
    pub port: u16,
    /// Last pong sim-time seconds, if any.
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
}

/// Pure in-memory twin peer list (no sockets, no threads).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TwinRegistry {
    peers: Vec<TwinPeer>,
}

impl TwinRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build from `(host, port)` pairs; `last_pong` starts as `None`.
    pub fn from_endpoints<H, I>(endpoints: I) -> Self
    where
        H: Into<String>,
        I: IntoIterator<Item = (H, u16)>,
    {
        Self {
            peers: endpoints
                .into_iter()
                .map(|(h, p)| TwinPeer::new(h, p))
                .collect(),
        }
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

    /// Append a peer (no dedup).
    pub fn add(&mut self, peer: TwinPeer) {
        self.peers.push(peer);
    }

    /// Record a pong at sim-time `at` for the first matching host:port.
    /// Returns false if no peer matched (stub never calls the network).
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

    /// `SAY ?TWINS` body without leading p_id.
    ///
    /// Empty: `TWINS none`. Otherwise `TWINS host:port@- …` (`@-` = no pong yet).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query() {
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
}
