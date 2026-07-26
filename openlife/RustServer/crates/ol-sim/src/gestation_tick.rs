//! Helpers to poll fertility due births (wired from tick_vitals).

use crate::fertility::FertilityState;

/// Mothers whose gestation completed this poll.
pub fn due_mothers(fertility: &mut FertilityState, sim_time: f32) -> Vec<i32> {
    fertility.poll_due(sim_time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fertility::{FertilityState, GESTATION_SECS};

    #[test]
    fn due_after_gestation() {
        let mut f = FertilityState::default();
        f.start_gestation(5, 0.0);
        assert!(due_mothers(&mut f, 1.0).is_empty());
        assert_eq!(due_mothers(&mut f, GESTATION_SECS), vec![5]);
    }
}
