//! Texture atlas packer (Haxe `BinPack.hx` free-rect split).
//!
//! Algorithm: keep free rectangles sorted by area; place into first fit;
//! split remainder into two free rects using the Haxe width/height residual heuristic.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn area(&self) -> i32 {
        self.width.saturating_mul(self.height)
    }

    pub fn max_x(&self) -> i32 {
        self.x + self.width
    }

    pub fn max_y(&self) -> i32 {
        self.y + self.height
    }
}

/// Simple bin packer: sort free spaces by area, split remaining L-shapes.
#[derive(Debug, Clone)]
pub struct BinPack {
    width: i32,
    height: i32,
    spaces: Vec<Rect>,
}

impl BinPack {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            spaces: vec![Rect {
                x: 0,
                y: 0,
                width,
                height,
            }],
        }
    }

    /// Packer with no free space (pre-filled atlas page from OLGA / dump load).
    ///
    /// Further [`pack`] calls return `None` so restored pixels are not overwritten.
    pub fn sealed(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            spaces: Vec::new(),
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Free-space count (tests / debug).
    pub fn free_count(&self) -> usize {
        self.spaces.len()
    }

    /// Approximate free volume remaining (Haxe `volumeLeft` inverted semantics —
    /// here we sum free rect areas; overlapping free rects are possible after split,
    /// so this is an upper bound style metric used only for diagnostics).
    pub fn free_area(&self) -> i32 {
        self.spaces.iter().map(|s| s.area()).sum()
    }

    /// Pack `w×h`; returns placed rect or None if no fit.
    pub fn pack(&mut self, w: i32, h: i32) -> Option<Rect> {
        if w <= 0 || h <= 0 {
            return None;
        }
        if w > self.width || h > self.height {
            return None;
        }
        self.spaces.sort_by_key(|s| s.width * s.height);
        let mut idx = None;
        for (i, space) in self.spaces.iter().enumerate() {
            if space.width >= w && space.height >= h {
                idx = Some(i);
                break;
            }
        }
        let i = idx?;
        let space = self.spaces.remove(i);
        let placed = Rect {
            x: space.x,
            y: space.y,
            width: w,
            height: h,
        };
        // Exact fit
        if space.width == w && space.height == h {
            return Some(placed);
        }
        if space.width == w {
            self.spaces.push(Rect {
                x: space.x,
                y: space.y + h,
                width: space.width,
                height: space.height - h,
            });
            return Some(placed);
        }
        if space.height == h {
            self.spaces.push(Rect {
                x: space.x + w,
                y: space.y,
                width: space.width - w,
                height: space.height,
            });
            return Some(placed);
        }
        // Strictly smaller: split into two free rects (Haxe heuristic)
        if space.width - w > space.height - h {
            self.spaces.push(Rect {
                x: space.x + w,
                y: space.y,
                width: space.width - w,
                height: space.height,
            });
            self.spaces.push(Rect {
                x: space.x,
                y: space.y + h,
                width: w,
                height: space.height - h,
            });
        } else {
            self.spaces.push(Rect {
                x: space.x,
                y: space.y + h,
                width: space.width,
                height: space.height - h,
            });
            self.spaces.push(Rect {
                x: space.x + w,
                y: space.y,
                width: space.width - w,
                height: h,
            });
        }
        Some(placed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_several_no_overlap() {
        let mut p = BinPack::new(64, 64);
        let a = p.pack(16, 16).unwrap();
        let b = p.pack(16, 16).unwrap();
        assert_ne!((a.x, a.y), (b.x, b.y));
        // axis-aligned non-overlap
        let overlap = a.x < b.max_x()
            && b.x < a.max_x()
            && a.y < b.max_y()
            && b.y < a.max_y();
        assert!(!overlap);
    }

    #[test]
    fn exact_fill() {
        let mut p = BinPack::new(32, 32);
        assert!(p.pack(32, 32).is_some());
        assert!(p.pack(1, 1).is_none());
    }

    #[test]
    fn reject_oversized() {
        let mut p = BinPack::new(16, 16);
        assert!(p.pack(17, 8).is_none());
        assert!(p.pack(8, 17).is_none());
    }

    #[test]
    fn pack_grid_like_haxe() {
        let mut p = BinPack::new(128, 128);
        let mut placed = Vec::new();
        for _ in 0..16 {
            placed.push(p.pack(16, 16).expect("16x16 should fit 16 times in 128"));
        }
        assert_eq!(placed.len(), 16);
        // no pair overlaps
        for i in 0..placed.len() {
            for j in (i + 1)..placed.len() {
                let a = &placed[i];
                let b = &placed[j];
                let overlap = a.x < b.max_x()
                    && b.x < a.max_x()
                    && a.y < b.max_y()
                    && b.y < a.max_y();
                assert!(!overlap, "overlap {a:?} {b:?}");
            }
        }
    }
}
