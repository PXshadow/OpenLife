//! Player-facing commands — **same wire path as human clients**.
//!
//! All methods build [`ol_net::NetIntent`] and push to an [`IntentSink`].
//! AI never mutates the world here; the sim applies intents on the sole-writer path.

use ol_net::NetIntent;

/// Where AI / tests enqueue intents (e.g. `mpsc::Sender::try_send` wrapper).
pub trait IntentSink {
    /// Returns `false` if the sink is full / closed (same idea as `try_send` fail).
    fn push(&mut self, intent: NetIntent) -> bool;
}

/// Shared command surface for humans (net layer) and AI (this crate).
///
/// Default methods all produce [`NetIntent`] variants that `apply_intent` already handles.
pub trait PlayerCommands: IntentSink {
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
    fn say_raw(&mut self, conn_id: u64, tag: impl Into<String>, payload: impl Into<String>) -> bool {
        self.push(NetIntent::Raw {
            conn_id,
            tag: tag.into(),
            payload: payload.into(),
        })
    }

    /// Keep-alive / position hint (rarely used by AI).
    fn keep_alive(&mut self, conn_id: u64, x: i32, y: i32) -> bool {
        self.push(NetIntent::KeepAlive { conn_id, x, y })
    }
}

// Blanket: any IntentSink gets the command helpers.
impl<T: IntentSink> PlayerCommands for T {}
