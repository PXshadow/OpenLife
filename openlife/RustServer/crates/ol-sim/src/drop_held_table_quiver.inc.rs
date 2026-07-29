// DROP-HELD-TABLE quiver free helpers (include after QuiverClothing impl)

/// Collect clothing parent ids from a 6-slot Haxe clothingObjects array (pad/trunc).
// Haxe: clothingObjects[0..5]
#[inline]
pub fn clothing_ids_snapshot(ids: &[i32]) -> [i32; 6] {
    let mut out = [0i32; 6];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = ids.get(i).copied().unwrap_or(0).max(0);
    }
    out
}

/// Pure quiver snapshot from clothing parent ids (uses default open capacity).
// Haxe: storeInQuiver clothing scan
#[inline]
pub fn quiver_from_clothing_ids(ids: &[i32]) -> QuiverClothing {
    QuiverClothing::from_ids(ids)
}

/// Pure quiver snapshot with multi-use from clothing uses array.
// Haxe: canAddToQuiver numberOfUses
#[inline]
pub fn quiver_from_clothing_snapshot(ids: &[i32], uses: &[i32]) -> QuiverClothing {
    QuiverClothing::from_clothing_snapshot(ids, uses)
}
