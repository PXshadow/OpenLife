//! Open Life **player interfaces** — shared by human clients and AI.
//!
//! # Architecture (Phase A)
//!
//! | Interface | Purpose |
//! |-----------|---------|
//! | [`PlayerWriteInterface`] | **Mutations** — same command path as players (`NetIntent` → `apply_intent`) |
//! | [`PlayerReadInterface`] | **Fast reads** — world + body + best-food (AI/tools only; not TCP) |
//!
//! Sub-traits under read: [`WorldView`], [`PlayerView`], [`FoodSearch`].
//!
//! **Hard rule:** AI never mutates `World` / `Player` directly. It only:
//! 1. **Reads** via [`PlayerReadInterface`] (or the sub-traits)
//! 2. **Writes** via [`PlayerWriteInterface`] → [`ol_net::NetIntent`]
//!
//! `NetIntent` remains the command *payload* enum (wire/sim). Prefer calling it
//! a “command” in docs; rename is deferred to avoid a huge blast radius.

#![forbid(unsafe_code)]

mod food_search;
mod player_view;
mod read;
mod world_view;
mod write;

pub use food_search::{BestFoodHit, BestFoodQuery, FoodSearch, DEFAULT_FOOD_SEARCH_RADIUS};
pub use player_view::PlayerView;
pub use read::{PlayerReadHandles, PlayerReadInterface};
pub use world_view::WorldView;
pub use write::{CommandSink, PlayerWriteInterface};

// ── Compatibility aliases (old names from early ol-ai) ───────────────────────
/// Deprecated name for [`CommandSink`].
pub use write::CommandSink as IntentSink;
/// Deprecated name for [`PlayerWriteInterface`].
pub use write::PlayerWriteInterface as PlayerCommands;

/// Chebyshev distance on a plane (no wrap). Adapters fold wrap coords first.
#[inline]
pub fn chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).abs().max((ay - by).abs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_net::NetIntent;

    struct VecSink(Vec<NetIntent>);

    impl CommandSink for VecSink {
        fn push(&mut self, intent: NetIntent) -> bool {
            self.0.push(intent);
            true
        }
    }

    #[test]
    fn write_interface_emits_net_intent() {
        let mut sink = VecSink(Vec::new());
        sink.use_at(7, 1, 2, Some(100), None);
        sink.drop_at(7, 1, 2, None);
        sink.move_path(7, 0, 0, &[(1, 0), (0, 1)], Some(3));
        sink.say_raw(7, "SAY", "hello");
        assert_eq!(sink.0.len(), 4);
        match &sink.0[0] {
            NetIntent::Use {
                conn_id,
                x,
                y,
                id,
                index,
            } => {
                assert_eq!(*conn_id, 7);
                assert_eq!((*x, *y), (1, 2));
                assert_eq!(*id, Some(100));
                assert!(index.is_none());
            }
            _ => panic!("expected Use"),
        }
    }

    #[test]
    fn food_query_default_radius_30() {
        let q = BestFoodQuery::new(1);
        assert_eq!(q.max_dist, DEFAULT_FOOD_SEARCH_RADIUS);
        assert_eq!(q.max_dist, 30);
    }

    #[test]
    fn chebyshev_basic() {
        assert_eq!(chebyshev(0, 0, 3, 4), 4);
        assert_eq!(chebyshev(10, 10, 10, 10), 0);
    }

    #[test]
    fn player_commands_alias_works() {
        let mut sink = VecSink(Vec::new());
        // Old name still resolves (alias).
        PlayerCommands::use_at(&mut sink, 1, 0, 0, None, None);
        assert_eq!(sink.0.len(), 1);
    }
}
