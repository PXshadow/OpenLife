//! Fast read-only world access for AI think ticks.

/// Cheap map queries. Implementors should avoid heavy locks on the hot path
/// (prefer `RwLock` read guards, snapshots, or resident-chunk lookups).
pub trait WorldView {
    fn width_height(&self) -> (i32, i32);

    fn wrap(&self) -> bool;

    /// Ground object id at tile (`0` = empty).
    fn object_at(&self, x: i32, y: i32) -> i32;

    /// Biome id / index at tile (adapter-defined encoding).
    fn biome_at(&self, x: i32, y: i32) -> u8;

    /// Floor object id at tile (`0` = none).
    fn floor_at(&self, x: i32, y: i32) -> i32;

    /// Visit non-empty objects in an inclusive axis-aligned rect.
    /// Callback: `(x, y, object_id)`.
    fn for_each_object_in_rect(
        &self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        f: &mut dyn FnMut(i32, i32, i32),
    );

    /// Convenience: scan a Chebyshev disc around `(cx,cy)` with radius `r`.
    fn for_each_object_in_chebyshev(
        &self,
        cx: i32,
        cy: i32,
        r: i32,
        f: &mut dyn FnMut(i32, i32, i32),
    ) {
        let r = r.max(0);
        self.for_each_object_in_rect(cx - r, cy - r, cx + r, cy + r, &mut |x, y, id| {
            let dx = (x - cx).abs();
            let dy = (y - cy).abs();
            if dx.max(dy) <= r {
                f(x, y, id);
            }
        });
    }
}
