//! **PlayerWriteInterface** — shared write path for humans and AI.
//!
//! All methods build [`ol_net::NetIntent`] and push to a [`CommandSink`].
//! The sim sole-writer applies intents; this trait never mutates the world.

use ol_net::NetIntent;

/// Where commands are enqueued (e.g. `mpsc::Sender::try_send` wrapper).
pub trait CommandSink {
    /// Returns `false` if the sink is full / closed (same idea as `try_send` fail).
    fn push(&mut self, intent: NetIntent) -> bool;
}

/// Shared **write** surface for human clients and AI.
///
/// Default methods all produce [`NetIntent`] variants that `apply_intent` already handles.
/// Prefer this name over the old `PlayerCommands` alias.
pub trait PlayerWriteInterface: CommandSink {
    /// USE at world tile (optional object id / container index).
    fn use_at(
        &mut self,
        conn_id: u64,
        x: i32,
        y: i32,
        id: Option<i32>,
        index: Option<i32>,
    ) -> bool {
        self.push(NetIntent::Use {
            conn_id,
            x,
            y,
            id,
            index,
        })
    }

    /// DROP at world tile; `clothing_slot` maps to wire `c` when set.
    fn drop_at(&mut self, conn_id: u64, x: i32, y: i32, clothing_slot: Option<i32>) -> bool {
        self.push(NetIntent::Drop {
            conn_id,
            x,
            y,
            c: clothing_slot,
        })
    }

    /// MOVE path from `(xs,ys)` with relative deltas (OHOL move body).
    fn move_path(
        &mut self,
        conn_id: u64,
        xs: i32,
        ys: i32,
        deltas: &[(i32, i32)],
        seq: Option<i32>,
    ) -> bool {
        self.push(NetIntent::Move {
            conn_id,
            xs,
            ys,
            deltas: deltas.to_vec(),
            seq,
        })
    }

    /// Raw protocol line (SAY, JUMP, SELF, …) — same as client `Raw` intent.
    fn say_raw(
        &mut self,
        conn_id: u64,
        tag: impl Into<String>,
        payload: impl Into<String>,
    ) -> bool {
        self.push(NetIntent::Raw {
            conn_id,
            tag: tag.into(),
            payload: payload.into(),
        })
    }

    /// SAY text (convenience over [`Self::say_raw`]).
    fn say(&mut self, conn_id: u64, text: impl Into<String>) -> bool {
        self.say_raw(conn_id, "SAY", text)
    }

    /// Keep-alive / position hint (rarely used by AI).
    fn keep_alive(&mut self, conn_id: u64, x: i32, y: i32) -> bool {
        self.push(NetIntent::KeepAlive { conn_id, x, y })
    }
}

// Blanket: any CommandSink gets the write helpers.
impl<T: CommandSink> PlayerWriteInterface for T {}
