//! Multi-MOVE continuation body (repath beyond 32-window / closest-fallback).
//!
//! Session fields `multi_move_chunks` / `multi_move_goal` are `pub(crate)`.
//!
//! **P2#11:** When a hop ends short of the ultimate click/stand goal (pathfind
//! window clamp or closest-reachable fallback), `done_moving` repaths toward
//! `multi_move_goal` and arms the next MOVE before flushing any queued object action.

use std::io;

use crate::session::ClientSession;

/// Improved multi-MOVE continuation: chunks first, then repath to ultimate goal.
///
/// Called from [`ClientSession::continue_multi_move`] on matching done_moving
/// while free (`!in_motion`).
pub(crate) fn continue_multi_move_body(session: &mut ClientSession) -> io::Result<Option<String>> {
    if session.move_state.in_motion || session.move_state.awaiting_force_ack {
        return Ok(None);
    }

    // Prefer pre-split remaining ±16 chunks (rebased to segment origin).
    if !session.multi_move_chunks.is_empty() {
        let chunk = session.multi_move_chunks.remove(0);
        if chunk.is_empty() {
            return continue_multi_move_body(session);
        }
        let offset = session.map_global_offset;
        let line = match session.move_state.send_move_with_offset(&chunk, offset) {
            Ok(l) => l,
            Err(_) => {
                // Chunk invalid — drop queue and try ultimate-goal repath.
                session.multi_move_chunks.clear();
                return continue_multi_move_body(session);
            }
        };
        session.send_raw(&line)?;
        // After last pre-split hop: clear goal only if we landed on it.
        if session.multi_move_chunks.is_empty() {
            if let Some(goal) = session.multi_move_goal {
                if (session.move_state.x, session.move_state.y) == goal {
                    session.multi_move_goal = None;
                }
            }
        }
        return Ok(Some(line));
    }

    let Some(goal) = session.multi_move_goal else {
        return Ok(None);
    };
    let pos = (session.move_state.x, session.move_state.y);
    if pos == goal {
        session.multi_move_goal = None;
        return Ok(None);
    }

    // P2#11: repath toward ultimate goal (next 32-window / past closest-fallback).
    // Does **not** cancel pending object action — flush waits until final hop.
    repath_toward_goal(session, goal)
}

/// Plan + send next hop toward `goal`; re-arm multi-MOVE when still short.
fn repath_toward_goal(
    session: &mut ClientSession,
    goal: (i32, i32),
) -> io::Result<Option<String>> {
    // After matching done_moving, dest is the hop end. Sync world so
    // path_start_tile's idle hint cannot lag behind move_state for repath origin.
    if let Some(id) = session.our_id {
        if let Some(o) = session.world.get_mut(id) {
            o.x = session.move_state.x;
            o.y = session.move_state.y;
        }
    }

    match crate::click_tile::plan_click_tile_chunks(session, goal.0, goal.1) {
        Ok((plan, first, rest)) => {
            if first.is_empty() {
                session.clear_multi_move();
                return Ok(None);
            }
            let start = crate::click_tile::path_start_tile(session);
            // Closest-fallback with no advance: stuck at current cell.
            if plan.end == start {
                session.clear_multi_move();
                return Ok(None);
            }
            session.move_state.x = start.0;
            session.move_state.y = start.1;
            let offset = session.map_global_offset;
            let line = match session.move_state.send_move_with_offset(&first, offset) {
                Ok(l) => l,
                Err(_) => {
                    session.clear_multi_move();
                    return Ok(None);
                }
            };
            session.send_raw(&line)?;
            let end = first
                .last()
                .map(|d| (start.0 + d.x, start.1 + d.y))
                .unwrap_or(start);
            // Keep the **same** ultimate goal (not plan.closest / window edge).
            session.arm_multi_move(rest, goal, end);
            Ok(Some(line))
        }
        Err(_) => {
            // Unreachable / SameTile / empty — stop multi-MOVE.
            session.clear_multi_move();
            Ok(None)
        }
    }
}
