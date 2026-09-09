//! Haxe rabbit hole / fleeing-rabbit cycle (`TimeHelper` + `ServerSettings`).
//!
//! ## Map objects (Haxe animals *are* tiles)
//!
//! Natural spawn: **161** Rabbit Hole#hiding,single (`mapChance=1` yellow).
//!
//! Vanilla peeking chain (time trans, not ServerSettings):
//! - 161 hiding → 166 peeking1 (5s) → 165 peeking2 (5s) → 164 out (5s)
//! - 167 family hiding → 193 peeking1 (5s) → 194 peeking2 (5s) → 173 family out (5s)
//!
//! Haxe `PatchObjectData`:
//! - 164 secondTime → 173 @ 90s
//! - 173 secondTime → 3566 @ 90s
//! - countsOrGrowsAs 164 / 173 / 3566 → 161
//! - 3566 biomes = YELLOW only
//!
//! Fleeing move (`-1_3566` move=3 desired=5): dest **3568** groundOnly, then `-1_3568` → 161.
//! Dest off YELLOW/GREEN stays **3566** (wrong-place die boost).
//!
//! Spatial (`TimeHelper.CanAnimalEndUpHere` / `CalculateNonBlockedTarget`):
//! - 3566 may only end on empty tile with **no floor**
//! - path may walk through objects whose description contains `"Rabbit"`
//! - winter hide / spring unhide of 3566 in snow-that-was-not-snow
//!
//! Snare / bait tiles (content transitions, same spatial family as holes):
//! 162–163, 168–172, 174–179.

/// Haxe `BiomeTag.GREEN` / `YELLOW`.
const GREEN: u8 = 0;
const YELLOW: u8 = 2;

/// 160 Snare (held tool; not a hole).
pub const SNARE: i32 = 160;
/// 161 Rabbit Hole#hiding,single.
pub const RABBIT_HOLE_HIDING: i32 = 161;
/// 162 Snared Rabbit Hole#hiding.
pub const SNARED_HOLE_HIDING: i32 = 162;
/// 163 Snared Rabbit Family Hole#hiding.
pub const SNARED_FAMILY_HOLE_HIDING: i32 = 163;
/// 164 Rabbit Hole#out,single.
pub const RABBIT_HOLE_OUT: i32 = 164;
/// 165 Rabbit Hole#peeking2,single.
pub const RABBIT_HOLE_PEEKING2: i32 = 165;
/// 166 Rabbit Hole#peeking1,single.
pub const RABBIT_HOLE_PEEKING1: i32 = 166;
/// 167 Rabbit Family Hole#hiding.
pub const FAMILY_HOLE_HIDING: i32 = 167;
/// 168 Snared Rabbit Hole#peeking2,single.
pub const SNARED_HOLE_PEEKING2: i32 = 168;
/// 169 Baited Snared Rabbit Hole#peeking1,single.
pub const BAITED_SNARED_HOLE_PEEKING1: i32 = 169;
/// 171 Baited Snared Rabbit Family Hole#peeking2.
pub const BAITED_SNARED_FAMILY_PEEKING2: i32 = 171;
/// 172 Baited Snared Rabbit Family Hole#peeking1.
pub const BAITED_SNARED_FAMILY_PEEKING1: i32 = 172;
/// 173 Rabbit Family Hole#out.
pub const RABBIT_FAMILY_HOLE: i32 = 173;
/// 174 Snared Rabbit#alive.
pub const SNARED_RABBIT_ALIVE: i32 = 174;
/// 175 Snared Family Rabbit#alive.
pub const SNARED_FAMILY_RABBIT_ALIVE: i32 = 175;
/// 176 Snared Rabbit#dead.
pub const SNARED_RABBIT_DEAD: i32 = 176;
/// 177 Snared Rabbit#dead,family.
pub const SNARED_RABBIT_DEAD_FAMILY: i32 = 177;
/// 178 Rabbit Hole with Used Snare#single.
pub const HOLE_USED_SNARE: i32 = 178;
/// 179 Rabbit Hole with Used Snare#family.
pub const FAMILY_HOLE_USED_SNARE: i32 = 179;
/// 193 Rabbit Family Hole#peeking1.
pub const FAMILY_HOLE_PEEKING1: i32 = 193;
/// 194 Rabbit Family Hole#peeking2.
pub const FAMILY_HOLE_PEEKING2: i32 = 194;
/// 195 Abandoned Rabbit Hole.
pub const ABANDONED_HOLE: i32 = 195;
/// 196 Rabbit Hole#growing.
pub const RABBIT_HOLE_GROWING: i32 = 196;
/// 3566 Fleeing Rabbit.
pub const FLEEING_RABBIT: i32 = 3566;
/// 3568 Fleeing Rabbit dest# groundOnly.
pub const FLEEING_RABBIT_DEST: i32 = 3568;

/// Every hole / peeking / snared-hole / dest tile that must not be stomped as "empty".
const RABBIT_HOLE_CHAIN: &[i32] = &[
    RABBIT_HOLE_HIDING,
    SNARED_HOLE_HIDING,
    SNARED_FAMILY_HOLE_HIDING,
    RABBIT_HOLE_OUT,
    RABBIT_HOLE_PEEKING2,
    RABBIT_HOLE_PEEKING1,
    FAMILY_HOLE_HIDING,
    SNARED_HOLE_PEEKING2,
    BAITED_SNARED_HOLE_PEEKING1,
    BAITED_SNARED_FAMILY_PEEKING2,
    BAITED_SNARED_FAMILY_PEEKING1,
    RABBIT_FAMILY_HOLE,
    SNARED_RABBIT_ALIVE,
    SNARED_FAMILY_RABBIT_ALIVE,
    SNARED_RABBIT_DEAD,
    SNARED_RABBIT_DEAD_FAMILY,
    HOLE_USED_SNARE,
    FAMILY_HOLE_USED_SNARE,
    FAMILY_HOLE_PEEKING1,
    FAMILY_HOLE_PEEKING2,
    ABANDONED_HOLE,
    RABBIT_HOLE_GROWING,
];

/// True for hiding/out/family/peeking/snared rabbit-hole tiles (not 3566/3568).
#[inline]
pub fn is_rabbit_hole_id(id: i32) -> bool {
    RABBIT_HOLE_CHAIN.contains(&id)
}

/// True for the live hunt cycle (holes + fleeing + dest).
#[inline]
pub fn is_rabbit_cycle_id(id: i32) -> bool {
    is_rabbit_hole_id(id) || id == FLEEING_RABBIT || id == FLEEING_RABBIT_DEST
}

/// Haxe `animal.parentId == 3566` empty+no-floor gate.
#[inline]
pub fn rabbit_requires_empty_no_floor(parent_id: i32) -> bool {
    parent_id == FLEEING_RABBIT
}

/// Haxe dest biome: YELLOW or GREEN may go underground (3568).
#[inline]
pub fn rabbit_dest_biome_allows_hole(biome: u8) -> bool {
    biome == YELLOW || biome == GREEN
}

/// After a fleeing-rabbit time-move, which id to place and whether it is wrong-place.
///
/// Haxe `doAnimalMovement` (parent is dest 3568 after the time trans is applied):
/// - dest biome not YELLOW/GREEN → stay 3566, `rabbitInWrongPlace = true`
/// - origin original biome YELLOW/GREEN → `rabbitInWrongPlace = false`
pub fn rabbit_move_arrival_id(new_target_id: i32, dest_biome: u8) -> (i32, bool) {
    rabbit_move_arrival(new_target_id, dest_biome, dest_biome)
}

/// Full Haxe arrival: dest biome + origin original biome.
pub fn rabbit_move_arrival(
    new_target_id: i32,
    dest_biome: u8,
    origin_original_biome: u8,
) -> (i32, bool) {
    if new_target_id != FLEEING_RABBIT_DEST {
        return (new_target_id, false);
    }
    let mut id = FLEEING_RABBIT_DEST;
    let mut wrong = false;
    // First Haxe if: still parent dest 3568.
    if !rabbit_dest_biome_allows_hole(dest_biome) {
        id = FLEEING_RABBIT;
        wrong = true;
    }
    // Second Haxe if: parentId is still dest only when first if did not rewrite id.
    if id == FLEEING_RABBIT_DEST && rabbit_dest_biome_allows_hole(origin_original_biome) {
        wrong = false;
    }
    (id, wrong)
}

/// Haxe `CanAnimalEndUpHere` rabbit extra: empty tile and no floor.
#[inline]
pub fn rabbit_tile_allows_end(object_id: i32, floor_id: i32) -> bool {
    object_id == 0 && floor_id == 0
}

/// Haxe: do not clear a hole/snare tile when a fleeing rabbit leaves it.
#[inline]
pub fn rabbit_origin_must_keep(object_id: i32) -> bool {
    is_rabbit_hole_id(object_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hole_chain_and_cycle_ids() {
        assert!(is_rabbit_hole_id(RABBIT_HOLE_HIDING));
        assert!(is_rabbit_hole_id(RABBIT_HOLE_OUT));
        assert!(is_rabbit_hole_id(RABBIT_FAMILY_HOLE));
        assert!(is_rabbit_hole_id(RABBIT_HOLE_PEEKING1));
        assert!(is_rabbit_hole_id(RABBIT_HOLE_PEEKING2));
        assert!(is_rabbit_hole_id(FAMILY_HOLE_HIDING));
        assert!(is_rabbit_hole_id(FAMILY_HOLE_PEEKING1));
        assert!(is_rabbit_hole_id(FAMILY_HOLE_PEEKING2));
        assert!(is_rabbit_hole_id(SNARED_HOLE_HIDING));
        assert!(is_rabbit_hole_id(SNARED_FAMILY_HOLE_HIDING));
        assert!(is_rabbit_hole_id(SNARED_RABBIT_ALIVE));
        assert!(is_rabbit_hole_id(HOLE_USED_SNARE));
        assert!(is_rabbit_hole_id(ABANDONED_HOLE));
        assert!(is_rabbit_hole_id(RABBIT_HOLE_GROWING));
        assert!(!is_rabbit_hole_id(FLEEING_RABBIT));
        assert!(!is_rabbit_hole_id(SNARE));
        assert!(is_rabbit_cycle_id(FLEEING_RABBIT));
        assert!(is_rabbit_cycle_id(FLEEING_RABBIT_DEST));
        assert!(rabbit_origin_must_keep(RABBIT_HOLE_HIDING));
        assert!(!rabbit_origin_must_keep(FLEEING_RABBIT));
    }

    #[test]
    fn dest_wrong_biome_stays_fleeing() {
        let (id, wrong) = rabbit_move_arrival(FLEEING_RABBIT_DEST, YELLOW, YELLOW);
        assert_eq!(id, FLEEING_RABBIT_DEST);
        assert!(!wrong);
        let (id, wrong) = rabbit_move_arrival(FLEEING_RABBIT_DEST, GREEN, GREEN);
        assert_eq!(id, FLEEING_RABBIT_DEST);
        assert!(!wrong);
        let (id, wrong) = rabbit_move_arrival(FLEEING_RABBIT_DEST, 4, 4); // snow dest + orig
        assert_eq!(id, FLEEING_RABBIT);
        assert!(wrong);
        let (id, wrong) = rabbit_move_arrival(FLEEING_RABBIT_DEST, 5, 5); // desert
        assert_eq!(id, FLEEING_RABBIT);
        assert!(wrong);
        // Dest snow, but origin original was yellow: Haxe second if skipped (id already 3566).
        let (id, wrong) = rabbit_move_arrival(FLEEING_RABBIT_DEST, 4, YELLOW);
        assert_eq!(id, FLEEING_RABBIT);
        assert!(wrong);
    }

    #[test]
    fn empty_no_floor_for_fleeing() {
        assert!(rabbit_tile_allows_end(0, 0));
        assert!(!rabbit_tile_allows_end(30, 0));
        assert!(!rabbit_tile_allows_end(0, 1596));
        assert!(rabbit_requires_empty_no_floor(FLEEING_RABBIT));
        assert!(!rabbit_requires_empty_no_floor(RABBIT_HOLE_HIDING));
    }
}
