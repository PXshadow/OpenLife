//! Per-connection outbound queues so the sim can push MX/PU/FX after mutations.
//!
//! Two lanes per connection:
//! - **urgent** — PS/LS and other interactive replies (must not wait behind AI PU/MX)
//! - **normal** — bulk map/player updates
//!
//! The connection task always drains urgent before normal so a SAY echo is not
//! stuck for a minute behind thousands of AI updates.

use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::mpsc;

/// Pair of receivers returned from [`OutboundHub::register`].
pub struct OutboundRx {
    pub urgent: mpsc::UnboundedReceiver<Vec<u8>>,
    pub normal: mpsc::UnboundedReceiver<Vec<u8>>,
}

impl OutboundRx {
    /// Drain **urgent first**, then normal (matches connection-task write order).
    /// Used by sim unit tests and any single-stream consumer of the dual lanes.
    pub fn try_recv(&mut self) -> Result<Vec<u8>, mpsc::error::TryRecvError> {
        match self.urgent.try_recv() {
            Ok(p) => Ok(p),
            Err(mpsc::error::TryRecvError::Empty) => self.normal.try_recv(),
            Err(e) => Err(e),
        }
    }
}

#[derive(Debug)]
struct ConnTx {
    urgent: mpsc::UnboundedSender<Vec<u8>>,
    normal: mpsc::UnboundedSender<Vec<u8>>,
}

/// Hub of conn_id → urgent + normal senders of raw wire packets.
#[derive(Debug, Default)]
pub struct OutboundHub {
    inner: Mutex<HashMap<u64, ConnTx>>,
}

impl OutboundHub {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, conn_id: u64) -> OutboundRx {
        let (urgent_tx, urgent_rx) = mpsc::unbounded_channel();
        let (normal_tx, normal_rx) = mpsc::unbounded_channel();
        self.inner.lock().unwrap().insert(
            conn_id,
            ConnTx {
                urgent: urgent_tx,
                normal: normal_tx,
            },
        );
        OutboundRx {
            urgent: urgent_rx,
            normal: normal_rx,
        }
    }

    pub fn unregister(&self, conn_id: u64) {
        self.inner.lock().unwrap().remove(&conn_id);
    }

    /// Signal the connection task to shut down the TCP socket after flushing
    /// any already-queued packets (Haxe `connection.close()`).
    ///
    /// Uses a zero-length marker packet on the **urgent** lane (never valid OHOL
    /// wire) then drops both senders so `recv()` ends.
    pub fn close(&self, conn_id: u64) {
        let mut guard = self.inner.lock().unwrap();
        if let Some(tx) = guard.remove(&conn_id) {
            let _ = tx.urgent.send(Vec::new()); // close marker
            // drop both senders
        }
    }

    /// Enqueue a packet. PS / LS always go **urgent** so chat and location says
    /// jump ahead of bulk PU/MX from AI. Everything else uses the normal lane.
    pub fn send(&self, conn_id: u64, packet: Vec<u8>) -> bool {
        if packet.is_empty() {
            return false; // reserved as close marker
        }
        let urgent = is_urgent_packet(&packet);
        let guard = self.inner.lock().unwrap();
        if let Some(tx) = guard.get(&conn_id) {
            if urgent {
                tx.urgent.send(packet).is_ok()
            } else {
                tx.normal.send(packet).is_ok()
            }
        } else {
            false
        }
    }

    /// Force urgent lane (interactive replies regardless of tag).
    pub fn send_urgent(&self, conn_id: u64, packet: Vec<u8>) -> bool {
        if packet.is_empty() {
            return false;
        }
        let guard = self.inner.lock().unwrap();
        if let Some(tx) = guard.get(&conn_id) {
            tx.urgent.send(packet).is_ok()
        } else {
            false
        }
    }

    pub fn broadcast(&self, packet: Vec<u8>) {
        if packet.is_empty() {
            return;
        }
        let urgent = is_urgent_packet(&packet);
        let guard = self.inner.lock().unwrap();
        for tx in guard.values() {
            if urgent {
                let _ = tx.urgent.send(packet.clone());
            } else {
                let _ = tx.normal.send(packet.clone());
            }
        }
    }

    pub fn connection_count(&self) -> usize {
        self.inner.lock().unwrap().len()
    }
}

/// Tags that must never sit behind AI bulk traffic.
fn is_urgent_packet(packet: &[u8]) -> bool {
    packet.starts_with(b"PS\n")
        || packet.starts_with(b"LS\n")
        || packet.starts_with(b"PM\n")
        || packet.starts_with(b"FX\n")
        || packet.starts_with(b"FM\n") // unstick client action wait
        || packet.starts_with(b"PE\n") // emote with following FM
        || packet.starts_with(b"PONG\n")
        || packet.starts_with(b"REJECTED\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ps_and_ls_are_urgent() {
        assert!(is_urgent_packet(b"PS\n1 hi\n#"));
        assert!(is_urgent_packet(b"LS\n0 0 1,2\n#"));
        assert!(!is_urgent_packet(b"PU\n1 19 0\n#"));
        assert!(!is_urgent_packet(b"MX\n1 2 0 0 0\n#"));
    }
}
