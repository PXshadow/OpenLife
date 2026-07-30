//! Think plan types + apply via [`PlayerWriteInterface`].

use ol_ai_api::PlayerWriteInterface;

/// Lightweight body sensors for one think tick (no SimState).
#[derive(Debug, Clone, Copy)]
pub struct ThinkSensors {
    pub conn_id: u64,
    pub x: i32,
    pub y: i32,
    pub food_store: f32,
    pub food_store_max: f32,
    pub held_id: i32,
    pub moving: bool,
}

/// High-level action MainAI wants (server emits intents).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkPlan {
    /// Nothing to do this tick.
    Idle,
    /// Walk toward food (multi-step path left to server pathfinder).
    SeekFood {
        tx: i32,
        ty: i32,
        food_id: i32,
    },
    /// Adjacent food tile — USE (or DROP-held first is server residual).
    UseFoodTile {
        tx: i32,
        ty: i32,
        food_id: i32,
    },
}

/// Emit write intents for a plan. Returns `true` if any intent was enqueued.
pub fn apply_plan<W: PlayerWriteInterface>(
    write: &mut W,
    sensors: &ThinkSensors,
    plan: ThinkPlan,
) -> bool {
    match plan {
        ThinkPlan::Idle => false,
        ThinkPlan::SeekFood { tx, ty, .. } => {
            // Single-step delta toward target; server can expand to multi-step path.
            let dx = (tx - sensors.x).signum();
            let dy = (ty - sensors.y).signum();
            if dx == 0 && dy == 0 {
                return false;
            }
            write.move_path(sensors.conn_id, sensors.x, sensors.y, &[(dx, dy)], None)
        }
        ThinkPlan::UseFoodTile { tx, ty, food_id } => {
            write.use_at(sensors.conn_id, tx, ty, Some(food_id), None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_ai_api::CommandSink;
    use ol_net::NetIntent;

    struct VecSink(Vec<NetIntent>);
    impl CommandSink for VecSink {
        fn push(&mut self, intent: NetIntent) -> bool {
            self.0.push(intent);
            true
        }
    }

    #[test]
    fn apply_seek_emits_move() {
        let mut w = VecSink(Vec::new());
        let s = ThinkSensors {
            conn_id: 3,
            x: 0,
            y: 0,
            food_store: 1.0,
            food_store_max: 20.0,
            held_id: 0,
            moving: false,
        };
        assert!(apply_plan(
            &mut w,
            &s,
            ThinkPlan::SeekFood {
                tx: 5,
                ty: 0,
                food_id: 31
            }
        ));
        assert!(matches!(w.0[0], NetIntent::Move { .. }));
    }
}
