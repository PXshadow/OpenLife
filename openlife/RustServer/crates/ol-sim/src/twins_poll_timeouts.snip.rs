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
