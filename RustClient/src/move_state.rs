//! MOVE wire encoding and client-side move sequence / in-motion / FORCE state.
//!
//! Matches official client (`LivingLifePage.cpp`):
//! - `lastMoveSequenceNumber` starts at **1** on birth; first MOVE increments to **2**.
//! - Wire: `MOVE xs ys @seq_num xdelt0 ydelt0 ... xdeltN ydeltN#`
//!   exactly one space between tokens (protocol.txt).
//! - Path deltas are relative to `(xs, ys)` and must be within **±16**.
//! - Client sets `in_motion = true` when sending MOVE.
//! - Own-player `in_motion` clears when a PU reports `done_moving_seqNum == lastMoveSequenceNumber`.
//! - On `force=1` PU: snap position and send `FORCE x y#` ack.
//! - USE / DROP / REMV are not sent while a MOVE is in progress (client gate; server also ignores).

use crate::actions::encode_force;
use thiserror::Error;

/// Maximum absolute path-search radius / delta magnitude (protocol.txt).
pub const MAX_PATH_DELTA: i32 = 16;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MoveError {
    #[error("path is empty; need at least one destination delta")]
    EmptyPath,
    #[error("path delta ({0},{1}) exceeds ±{MAX_PATH_DELTA}")]
    DeltaOutOfRange(i32, i32),
    #[error("cannot send MOVE while already in motion (wait for done_moving PU)")]
    AlreadyInMotion,
    #[error("cannot send position-sensitive action until FORCE ack is sent")]
    AwaitingForceAck,
    #[error("cannot send USE/DROP/REMV while MOVE is in progress")]
    ActionWhileMoving,
}

/// Grid step relative to path start (xs, ys).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathDelta {
    pub x: i32,
    pub y: i32,
}

/// Client move bookkeeping for one local player.
#[derive(Debug, Clone)]
pub struct MoveState {
    /// Last move sequence number sent (or 1 at birth before any MOVE).
    pub last_move_sequence_number: i32,
    /// True after we send MOVE until matching done_moving PU.
    pub in_motion: bool,
    /// Current believed grid position (xs/ys for next MOVE, FORCE snap target).
    pub x: i32,
    pub y: i32,
    /// Server forced a position; block moves/actions until FORCE ack is sent.
    pub awaiting_force_ack: bool,
    pub force_x: i32,
    pub force_y: i32,
}

impl Default for MoveState {
    fn default() -> Self {
        Self {
            // LivingLifePage.cpp sets lastMoveSequenceNumber = 1 on player create.
            last_move_sequence_number: 1,
            in_motion: false,
            x: 0,
            y: 0,
            awaiting_force_ack: false,
            force_x: 0,
            force_y: 0,
        }
    }
}

impl MoveState {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            ..Self::default()
        }
    }

    /// Encode and apply a MOVE: increments seq, sets in_motion, validates deltas.
    pub fn send_move(&mut self, path_deltas: &[PathDelta]) -> Result<String, MoveError> {
        if self.awaiting_force_ack {
            return Err(MoveError::AwaitingForceAck);
        }
        if self.in_motion {
            return Err(MoveError::AlreadyInMotion);
        }
        let line = encode_move(self.x, self.y, self.last_move_sequence_number + 1, path_deltas)?;
        self.last_move_sequence_number += 1;
        self.in_motion = true;
        // Destination = start + last delta (official path end).
        if let Some(last) = path_deltas.last() {
            self.x += last.x;
            self.y += last.y;
        }
        Ok(line)
    }

    /// Whether USE/DROP/REMV may be issued (not mid-move, not waiting on FORCE).
    pub fn can_send_object_action(&self) -> Result<(), MoveError> {
        if self.awaiting_force_ack {
            return Err(MoveError::AwaitingForceAck);
        }
        if self.in_motion {
            return Err(MoveError::ActionWhileMoving);
        }
        Ok(())
    }

    /// Apply a PLAYER_UPDATE for the local player (done_moving_seqNum, force, x, y).
    pub fn on_player_update(
        &mut self,
        done_moving_seq_num: i32,
        force: bool,
        x: i32,
        y: i32,
    ) -> Option<String> {
        // force snap (protocol + client)
        if force {
            self.x = x;
            self.y = y;
            self.in_motion = false;
            self.awaiting_force_ack = true;
            self.force_x = x;
            self.force_y = y;
            return Some(encode_force(x, y));
        }

        if done_moving_seq_num > 0 {
            // Official client only clears in_motion when done_moving matches last sent seq.
            if done_moving_seq_num == self.last_move_sequence_number {
                self.in_motion = false;
                self.x = x;
                self.y = y;
            }
        }
        None
    }

    /// After sending FORCE ack, clear the sync gate.
    pub fn acknowledge_force_sent(&mut self) {
        self.awaiting_force_ack = false;
    }

    /// Build FORCE ack for current force coordinates (caller sends then calls acknowledge_force_sent).
    pub fn force_ack_message(&self) -> String {
        encode_force(self.force_x, self.force_y)
    }
}

/// Pure MOVE encoder (does not mutate state).
///
/// `seq_num` is the sequence to put on the wire (first move = 2).
/// `path_deltas` are relative to `(xs, ys)`; at least one step required.
pub fn encode_move(
    xs: i32,
    ys: i32,
    seq_num: i32,
    path_deltas: &[PathDelta],
) -> Result<String, MoveError> {
    if path_deltas.is_empty() {
        return Err(MoveError::EmptyPath);
    }
    for d in path_deltas {
        if d.x.abs() > MAX_PATH_DELTA || d.y.abs() > MAX_PATH_DELTA {
            return Err(MoveError::DeltaOutOfRange(d.x, d.y));
        }
    }

    // Exact spacing: "MOVE xs ys @seq" then " xdelt ydelt" pairs, then "#"
    let mut s = format!("MOVE {xs} {ys} @{seq_num}");
    for d in path_deltas {
        s.push_str(&format!(" {} {}", d.x, d.y));
    }
    s.push('#');
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_wire_single_step() {
        let line = encode_move(10, 20, 2, &[PathDelta { x: 1, y: 0 }]).unwrap();
        assert_eq!(line, "MOVE 10 20 @2 1 0#");
    }

    #[test]
    fn move_wire_multi_step_exact_spaces() {
        let line = encode_move(
            -5,
            3,
            2,
            &[
                PathDelta { x: 1, y: 0 },
                PathDelta { x: 2, y: 1 },
                PathDelta { x: 2, y: 2 },
            ],
        )
        .unwrap();
        assert_eq!(line, "MOVE -5 3 @2 1 0 2 1 2 2#");
        // no double spaces
        assert!(!line.contains("  "));
    }

    #[test]
    fn first_move_seq_is_2() {
        let mut st = MoveState::new(0, 0);
        assert_eq!(st.last_move_sequence_number, 1);
        let line = st
            .send_move(&[PathDelta { x: 1, y: 0 }])
            .unwrap();
        assert_eq!(line, "MOVE 0 0 @2 1 0#");
        assert_eq!(st.last_move_sequence_number, 2);
        assert!(st.in_motion);
        assert_eq!((st.x, st.y), (1, 0));
    }

    #[test]
    fn second_move_seq_is_3_after_done() {
        let mut st = MoveState::new(0, 0);
        st.send_move(&[PathDelta { x: 1, y: 0 }]).unwrap();
        st.on_player_update(2, false, 1, 0);
        assert!(!st.in_motion);
        let line = st
            .send_move(&[PathDelta { x: 0, y: 1 }])
            .unwrap();
        assert_eq!(line, "MOVE 1 0 @3 0 1#");
    }

    #[test]
    fn rejects_delta_beyond_16() {
        let err = encode_move(0, 0, 2, &[PathDelta { x: 17, y: 0 }]).unwrap_err();
        assert_eq!(err, MoveError::DeltaOutOfRange(17, 0));
    }

    #[test]
    fn blocks_action_while_moving() {
        let mut st = MoveState::new(0, 0);
        st.send_move(&[PathDelta { x: 1, y: 0 }]).unwrap();
        assert_eq!(st.can_send_object_action(), Err(MoveError::ActionWhileMoving));
    }

    #[test]
    fn force_snaps_and_returns_ack() {
        let mut st = MoveState::new(0, 0);
        st.send_move(&[PathDelta { x: 5, y: 0 }]).unwrap();
        let ack = st.on_player_update(0, true, 2, 3).unwrap();
        assert_eq!(ack, "FORCE 2 3#");
        assert_eq!((st.x, st.y), (2, 3));
        assert!(st.awaiting_force_ack);
        assert!(!st.in_motion);
        st.acknowledge_force_sent();
        assert!(!st.awaiting_force_ack);
    }

    #[test]
    fn done_moving_mismatch_keeps_in_motion() {
        let mut st = MoveState::new(0, 0);
        st.send_move(&[PathDelta { x: 1, y: 0 }]).unwrap();
        // Stale / other seq
        st.on_player_update(1, false, 0, 0);
        assert!(st.in_motion);
        st.on_player_update(2, false, 1, 0);
        assert!(!st.in_motion);
    }
}
