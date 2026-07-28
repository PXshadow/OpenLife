//! C++ `mMapGlobalOffset` / `sendX` / `sendY` / `applyReceiveOffset`.
//!
//! ## Why this exists (LivingLifePage.h ~622–640)
//!
//! The official client may subtract a global offset from all received map
//! coordinates so local storage stays in a low integer range. That avoids
//! GPU float rounding when rendering huge world tiles as `float`. Before
//! any client→server line, it reverse-applies the offset:
//!
//! ```text
//! sendX(local) = local + mMapGlobalOffset.x   // when set
//! sendY(local) = local + mMapGlobalOffset.y
//! applyReceiveOffset: *x -= offset.x; *y -= offset.y
//! ```
//!
//! Offset is chosen from the **first MC center** only if that center is outside
//! ±16384 of origin; otherwise it stays **(0, 0)** (LivingLifePage.cpp ~16127–16153).
//!
//! ## Rust client status — DONE-NA (offset = 0)
//!
//! This headless / soft-FB client stores **wire coordinates as-is**:
//! - `session.map` keys = MC/MX coords from the server
//! - `move_state.x/y` / `LiveObject` = PU/PM coords from the server
//! - `encode_move` / USE·DROP·REMV use those same values
//!
//! No second local frame is introduced, so `send_x` / `send_y` are the identity.
//! That matches the wire frame whether the server uses absolute world tiles
//! (protocol.txt “absolute world position”) or birth-relative tiles
//! (OpenLifeReborn bootstrap at ~0,0): client MOVE always echoes the frame
//! the server already put on PU/MC.
//!
//! Non-zero offset would only be needed if we reintroduced C++-style local
//! maps for far-from-origin absolute worlds. Until then this type stays at
//! `(0,0)` after first MC (or permanently) — **DONE-NA**, not a missing MOVE bug.

/// C++ `mMapGlobalOffset` + `mMapGlobalOffsetSet`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapGlobalOffset {
    /// C++ `mMapGlobalOffsetSet` — once true, send/receive helpers apply.
    pub set: bool,
    pub x: i32,
    pub y: i32,
}

impl Default for MapGlobalOffset {
    fn default() -> Self {
        // Identity frame: wire coords == storage coords (see module docs).
        Self {
            set: true,
            x: 0,
            y: 0,
        }
    }
}

impl MapGlobalOffset {
    /// Explicit zero offset (same as `Default`).
    pub const ZERO: Self = Self {
        set: true,
        x: 0,
        y: 0,
    };

    /// C++ first-MC policy: use (0,0) unless center is outside ±16384.
    ///
    /// Rust keeps offset 0 even for large centers because storage is wire-frame
    /// i32 (no float tile path). This helper documents the C++ threshold only.
    pub fn from_first_mc_center(cx: i32, cy: i32) -> Self {
        const MAX_OK: i32 = 16384;
        if cx.abs() <= MAX_OK && cy.abs() <= MAX_OK {
            Self::ZERO
        } else {
            // C++ would set offset = (cx, cy). We intentionally stay at zero:
            // storage remains wire coords; non-zero would require applying
            // receive offsets on every PU/MC/MX. See module docs.
            let _ = (cx, cy);
            Self::ZERO
        }
    }

    /// C++ `sendX` — local storage → wire.
    #[inline]
    pub fn send_x(self, in_x: i32) -> i32 {
        if self.set {
            in_x + self.x
        } else {
            in_x
        }
    }

    /// C++ `sendY` — local storage → wire.
    #[inline]
    pub fn send_y(self, in_y: i32) -> i32 {
        if self.set {
            in_y + self.y
        } else {
            in_y
        }
    }

    /// C++ `applyReceiveOffset` — wire → local storage (in place).
    #[inline]
    pub fn apply_receive(&self, x: &mut i32, y: &mut i32) {
        if self.set {
            *x -= self.x;
            *y -= self.y;
        }
    }

    /// Whether this is the identity transform (offset 0 or not yet set).
    #[inline]
    pub fn is_identity(self) -> bool {
        !self.set || (self.x == 0 && self.y == 0)
    }
}

/// Encode MOVE using C++ sendX/sendY on the path start (deltas unchanged).
///
/// Path deltas are relative to start in the **same** frame as `xs,ys` after
/// send conversion; with offset 0 this equals [`crate::encode_move`].
pub fn encode_move_with_offset(
    offset: MapGlobalOffset,
    xs: i32,
    ys: i32,
    seq_num: i32,
    path_deltas: &[crate::move_state::PathDelta],
) -> Result<String, crate::move_state::MoveError> {
    crate::move_state::encode_move(
        offset.send_x(xs),
        offset.send_y(ys),
        seq_num,
        path_deltas,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::move_state::PathDelta;

    #[test]
    fn zero_offset_is_identity() {
        let o = MapGlobalOffset::ZERO;
        assert!(o.is_identity());
        assert_eq!(o.send_x(488), 488);
        assert_eq!(o.send_y(-12), -12);
        let mut x = 10;
        let mut y = 20;
        o.apply_receive(&mut x, &mut y);
        assert_eq!((x, y), (10, 20));
    }

    #[test]
    fn first_mc_near_origin_stays_zero() {
        assert_eq!(MapGlobalOffset::from_first_mc_center(488, 488), MapGlobalOffset::ZERO);
        assert_eq!(MapGlobalOffset::from_first_mc_center(0, 0), MapGlobalOffset::ZERO);
    }

    #[test]
    fn first_mc_far_still_zero_by_policy() {
        // C++ would pick non-zero; Rust DONE-NA policy keeps identity.
        assert_eq!(
            MapGlobalOffset::from_first_mc_center(50_000, -50_000),
            MapGlobalOffset::ZERO
        );
    }

    #[test]
    fn encode_move_with_zero_offset_matches_encode_move() {
        let deltas = [PathDelta { x: 1, y: 0 }, PathDelta { x: 2, y: 0 }];
        let a = encode_move_with_offset(MapGlobalOffset::ZERO, 488, 488, 2, &deltas).unwrap();
        let b = crate::move_state::encode_move(488, 488, 2, &deltas).unwrap();
        assert_eq!(a, b);
        assert_eq!(a, "MOVE 488 488 @2 1 0 2 0#");
    }

    #[test]
    fn non_zero_offset_converts_start_only() {
        let o = MapGlobalOffset {
            set: true,
            x: 100,
            y: 200,
        };
        assert!(!o.is_identity());
        assert_eq!(o.send_x(5), 105);
        assert_eq!(o.send_y(7), 207);
        let line = encode_move_with_offset(o, 5, 7, 2, &[PathDelta { x: 1, y: 0 }]).unwrap();
        assert_eq!(line, "MOVE 105 207 @2 1 0#");
        let mut x = 105;
        let mut y = 207;
        o.apply_receive(&mut x, &mut y);
        assert_eq!((x, y), (5, 7));
    }
}
