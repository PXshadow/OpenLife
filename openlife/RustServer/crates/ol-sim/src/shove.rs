//! Push / pull adjacent player helpers (Haxe shove subset).
//!
//! Pure geometry + outcome math — sim applies positions and PE/prestige.

/// Chebyshev range for PUSH / PULL / KISS (strictly adjacent; not same tile).
pub const SHOVE_RANGE: i32 = 1;

/// PE emot index for `SAY KISS` (Haxe `Emote.love` = 13 — cute/partner proxy).
pub const CUTE_EMOT_INDEX: i32 = 13;

/// Tiny prestige granted to the kisser when target is an ally (either direction).
pub const KISS_ALLY_PRESTIGE: f32 = 0.01;

/// Prestige granted to speaker on `SAY THANK <p_id>` when target is adjacent (Chebyshev ≤ 1).
pub const THANK_PRESTIGE: f32 = 0.05;

/// Tiny prestige granted to speaker on `SAY BLESS <p_id>` when target is adjacent.
pub const BLESS_PRESTIGE: f32 = 0.02;

/// PE emot index for `SAY HUG` (Haxe `Emote.love` = 13; same as KISS cute).
pub const LOVE_EMOT_INDEX: i32 = CUTE_EMOT_INDEX;

/// PE emot index for `SAY SLAP` (Haxe `Emote.mad` = 1).
pub const MAD_EMOT_INDEX: i32 = 1;

/// Wound stacks applied by `SAY SLAP` when target is not an ally.
pub const SLAP_WOUND: u8 = 1;

/// Chebyshev adjacent (excluding same tile).
pub fn is_adjacent(ax: i32, ay: i32, bx: i32, by: i32) -> bool {
    let d = (ax - bx).abs().max((ay - by).abs());
    d >= 1 && d <= SHOVE_RANGE
}

/// Direction from actor to target as unit step (-1,0,1).
pub fn dir_toward(ax: i32, ay: i32, bx: i32, by: i32) -> (i32, i32) {
    ((bx - ax).signum(), (by - ay).signum())
}

/// Push target one step away from actor.
pub fn push_dest(ax: i32, ay: i32, tx: i32, ty: i32) -> (i32, i32) {
    let (dx, dy) = dir_toward(ax, ay, tx, ty);
    (tx + dx, ty + dy)
}

/// Pull target one step toward actor.
pub fn pull_dest(ax: i32, ay: i32, tx: i32, ty: i32) -> (i32, i32) {
    let (dx, dy) = dir_toward(tx, ty, ax, ay);
    (tx + dx, ty + dy)
}

/// Outcome of a push attempt (positions only; god/adjacency checked by caller).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushOutcome {
    /// Target steps to `(nx, ny)`; actor stays put.
    Shove { nx: i32, ny: i32 },
    /// Swap actor and target tiles when shove dest is blocked/occupied.
    Swap,
}

/// Decide push: shove away when `dest_free`, else swap.
///
/// `dest_free` means walkable, not mountain, and not occupied by a third player.
pub fn resolve_push(ax: i32, ay: i32, tx: i32, ty: i32, dest_free: bool) -> PushOutcome {
    if dest_free {
        let (nx, ny) = push_dest(ax, ay, tx, ty);
        // Degenerate (same tile as target) → swap.
        if nx == tx && ny == ty {
            PushOutcome::Swap
        } else {
            PushOutcome::Shove { nx, ny }
        }
    } else {
        PushOutcome::Swap
    }
}

/// Whether pull may place the target at `(dx, dy)`.
///
/// Dest must be free of third players; actor's own tile is allowed (share space).
/// `dest_walkable` covers objects / biomes.
pub fn can_pull_to(
    ax: i32,
    ay: i32,
    dest_x: i32,
    dest_y: i32,
    dest_walkable: bool,
    third_player_on_dest: bool,
) -> bool {
    if !dest_walkable || third_player_on_dest {
        return false;
    }
    // Dest must be strictly closer or on actor (one step toward).
    let before = (ax - dest_x).abs().max((ay - dest_y).abs());
    // After pull, dist from actor to dest should be less than SHOVE_RANGE+something;
    // for adjacent pull, dest is actor tile (dist 0).
    before <= SHOVE_RANGE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_pull_geometry() {
        assert!(is_adjacent(0, 0, 1, 0));
        assert!(is_adjacent(0, 0, 1, 1));
        assert!(!is_adjacent(0, 0, 0, 0));
        assert!(!is_adjacent(0, 0, 2, 0));
        assert_eq!(push_dest(0, 0, 1, 0), (2, 0));
        assert_eq!(push_dest(0, 0, 1, 1), (2, 2));
        assert_eq!(pull_dest(0, 0, 1, 0), (0, 0));
        assert_eq!(pull_dest(0, 0, 2, 0), (1, 0));
    }

    #[test]
    fn resolve_push_shove_or_swap() {
        assert_eq!(
            resolve_push(0, 0, 1, 0, true),
            PushOutcome::Shove { nx: 2, ny: 0 }
        );
        assert_eq!(resolve_push(0, 0, 1, 0, false), PushOutcome::Swap);
    }

    #[test]
    fn pull_allows_actor_tile() {
        // Adjacent pull lands on actor; third_player false → ok.
        assert!(can_pull_to(0, 0, 0, 0, true, false));
        assert!(!can_pull_to(0, 0, 0, 0, true, true));
        assert!(!can_pull_to(0, 0, 0, 0, false, false));
    }

    #[test]
    fn kiss_constants() {
        assert_eq!(CUTE_EMOT_INDEX, 13);
        assert!(KISS_ALLY_PRESTIGE > 0.0 && KISS_ALLY_PRESTIGE < 0.1);
    }

    #[test]
    fn thank_bless_hug_slap_constants() {
        assert!((THANK_PRESTIGE - 0.05).abs() < 1e-6);
        assert!((BLESS_PRESTIGE - 0.02).abs() < 1e-6);
        assert_eq!(LOVE_EMOT_INDEX, 13);
        assert_eq!(MAD_EMOT_INDEX, 1);
        assert_eq!(SLAP_WOUND, 1);
    }
}
