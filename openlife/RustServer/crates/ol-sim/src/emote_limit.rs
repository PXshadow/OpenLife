//! Emote (PE / EMOTE) rate limit separate from SAY chat limit.

use std::collections::VecDeque;

/// Max emotes accepted per window.
pub const EMOTE_RATE_MAX: usize = 3;

/// Sliding window length in sim seconds.
pub const EMOTE_RATE_WINDOW_SECS: f32 = 10.0;

/// Sliding-window timestamp queue for one player.
#[derive(Debug, Clone, Default)]
pub struct EmoteRateLimiter {
    pub times: VecDeque<f32>,
}

impl EmoteRateLimiter {
    /// Drop timestamps older than the window relative to `now`.
    pub fn prune(&mut self, now: f32) {
        while self
            .times
            .front()
            .is_some_and(|&t| now - t >= EMOTE_RATE_WINDOW_SECS)
        {
            self.times.pop_front();
        }
    }

    /// Returns true if an emote is allowed and records `now`.
    pub fn try_emote(&mut self, now: f32) -> bool {
        self.prune(now);
        if self.times.len() >= EMOTE_RATE_MAX {
            return false;
        }
        self.times.push_back(now);
        true
    }

    pub fn count(&self) -> usize {
        self.times.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_three_then_blocks() {
        let mut lim = EmoteRateLimiter::default();
        assert!(lim.try_emote(0.0));
        assert!(lim.try_emote(1.0));
        assert!(lim.try_emote(2.0));
        assert!(!lim.try_emote(3.0));
        // After window, allowed again.
        assert!(lim.try_emote(12.0));
    }
}
