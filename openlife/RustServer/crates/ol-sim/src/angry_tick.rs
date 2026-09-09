//! Angry recovery — canonical in **`ol-combat-rules`**.
//!
//! **ANGRY-TIME-MIN:** `tick_vitals` passes live `CombatAngryTimeMinimum` into
//! [`tick_angry_time`] (Haxe killMode / last-attacker-armed drain floor).
pub use ol_combat_rules::{
    biome_angry_recover_factor, tick_angry_time, COMBAT_ANGRY_TIME_MINIMUM,
};

#[cfg(test)]
mod live_tests {
    use super::*;
    use crate::{spawn_player, tick_vitals};
    use ol_content::ContentDb;
    use ol_net::OutboundHub;
    use ol_world::PASSABLE_RIVER;
    use std::sync::Arc;

    #[test]
    fn vitals_recovers_and_kill_mode_drains() {
        let hub = OutboundHub::new();
        let mut state = crate::SimState::with_default_empty(Arc::new(ContentDb::default()));
        spawn_player(&mut state, 1, "a@test");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.angry_time = 0.0;
            p.kill_mode = false;
        }
        tick_vitals(&mut state, 1.0, &hub);
        let recovered = state.players.get(&1).unwrap().angry_time;
        assert!(
            (recovered - 1.0).abs() < 0.05,
            "expected ~1 recovered, got {recovered}"
        );

        {
            let p = state.players.get_mut(&1).unwrap();
            p.angry_time = 1.0;
            p.kill_mode = true;
        }
        tick_vitals(&mut state, 2.0, &hub);
        let drained = state.players.get(&1).unwrap().angry_time;
        assert!(drained < 0.0, "killMode should drain angryTime, got {drained}");
        assert!(drained >= state.gameplay.combat_angry_time_minimum_live() - 0.01);
        assert!((state.gameplay.combat_angry_time_minimum_live() - COMBAT_ANGRY_TIME_MINIMUM).abs() < 1e-6);
    }

    #[test]
    fn kill_mode_clamps_to_live_combat_angry_time_minimum() {
        // ANGRY-TIME-MIN: live override floor, not compiled −60 only.
        let hub = OutboundHub::new();
        let mut state = crate::SimState::with_default_empty(Arc::new(ContentDb::default()));
        spawn_player(&mut state, 1, "clamp@test");
        state.gameplay.combat_angry_time_minimum = -1.0;
        {
            let p = state.players.get_mut(&1).unwrap();
            p.angry_time = 0.0;
            p.kill_mode = true;
        }
        tick_vitals(&mut state, 5.0, &hub);
        let t = state.players.get(&1).unwrap().angry_time;
        assert!(
            (t - (-1.0)).abs() < 1e-4,
            "killMode drain must clamp to live min −1, got {t}"
        );
    }

    #[test]
    fn vitals_river_biome_recovers_angry_faster() {
        let hub = OutboundHub::new();
        let mut state = crate::SimState::with_default_empty(Arc::new(ContentDb::default()));
        spawn_player(&mut state, 1, "river@test");
        let (x, y) = {
            let p = state.players.get(&1).unwrap();
            (p.x, p.y)
        };
        state
            .world
            .write()
            .unwrap()
            .set_biome(x, y, PASSABLE_RIVER);
        {
            let p = state.players.get_mut(&1).unwrap();
            p.angry_time = 0.0;
            p.kill_mode = false;
        }
        tick_vitals(&mut state, 1.0, &hub);
        let recovered = state.players.get(&1).unwrap().angry_time;
        let expected = biome_angry_recover_factor(PASSABLE_RIVER);
        assert!(
            (recovered - expected).abs() < 0.05,
            "passable river recover ×2, got {recovered} expected {expected}"
        );
    }
}
