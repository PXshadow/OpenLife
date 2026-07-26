//! Hot / warm / cold chunk simulation tiers (Haxe vast-world load subset).
//!
//! Pure bookkeeping: classify chunks by distance from active players so a
//! future sim loop can skip cold chunks. No world I/O.

use std::collections::HashMap;

/// Simulation priority for a resident chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChunkTier {
    /// Player interest — full sim every tick.
    Hot,
    /// Near players — reduced rate.
    Warm,
    /// Far / unloaded interest — rare or frozen.
    Cold,
}

impl ChunkTier {
    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Hot => "hot",
            Self::Warm => "warm",
            Self::Cold => "cold",
        }
    }

    /// Suggested sim period in ticks (1 = every tick).
    pub fn sim_period_ticks(self) -> u32 {
        match self {
            Self::Hot => 1,
            Self::Warm => 4,
            Self::Cold => 32,
        }
    }
}

/// Chebyshev distance from (cx,cy) to nearest player chunk coord.
pub fn min_chunk_chebyshev(
    cx: i32,
    cy: i32,
    player_chunks: &[(i32, i32)],
) -> i32 {
    player_chunks
        .iter()
        .map(|(px, py)| (cx - px).abs().max((cy - py).abs()))
        .min()
        .unwrap_or(i32::MAX)
}

/// Classify one chunk: hot if dist≤1, warm if dist≤3, else cold.
pub fn classify_chunk(dist: i32) -> ChunkTier {
    if dist <= 1 {
        ChunkTier::Hot
    } else if dist <= 3 {
        ChunkTier::Warm
    } else {
        ChunkTier::Cold
    }
}

/// Build tier map for a set of chunk coords given player tile positions and chunk size.
pub fn build_tier_map(
    chunk_coords: &[(i32, i32)],
    player_tiles: &[(i32, i32)],
    chunk_size: i32,
) -> HashMap<(i32, i32), ChunkTier> {
    let player_chunks: Vec<(i32, i32)> = player_tiles
        .iter()
        .map(|(x, y)| (x.div_euclid(chunk_size), y.div_euclid(chunk_size)))
        .collect();
    let mut out = HashMap::new();
    for &(cx, cy) in chunk_coords {
        let d = min_chunk_chebyshev(cx, cy, &player_chunks);
        out.insert((cx, cy), classify_chunk(d));
    }
    out
}

/// Counts of each tier (for metrics / SAY ?CHUNKS).
pub fn tier_counts(map: &HashMap<(i32, i32), ChunkTier>) -> (u32, u32, u32) {
    let mut hot = 0u32;
    let mut warm = 0u32;
    let mut cold = 0u32;
    for t in map.values() {
        match t {
            ChunkTier::Hot => hot += 1,
            ChunkTier::Warm => warm += 1,
            ChunkTier::Cold => cold += 1,
        }
    }
    (hot, warm, cold)
}

/// Format `CHUNKS hot=N warm=N cold=N` query body.
pub fn format_chunks_query(hot: u32, warm: u32, cold: u32) -> String {
    format!("CHUNKS hot={hot} warm={warm} cold={cold}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_bands() {
        assert_eq!(classify_chunk(0), ChunkTier::Hot);
        assert_eq!(classify_chunk(1), ChunkTier::Hot);
        assert_eq!(classify_chunk(2), ChunkTier::Warm);
        assert_eq!(classify_chunk(3), ChunkTier::Warm);
        assert_eq!(classify_chunk(4), ChunkTier::Cold);
    }

    #[test]
    fn build_map_around_player() {
        let chunks = vec![(0, 0), (1, 0), (5, 5)];
        let players = vec![(10, 10)]; // chunk (0,0) if size 64
        let map = build_tier_map(&chunks, &players, 64);
        assert_eq!(map.get(&(0, 0)), Some(&ChunkTier::Hot));
        assert_eq!(map.get(&(1, 0)), Some(&ChunkTier::Hot));
        assert_eq!(map.get(&(5, 5)), Some(&ChunkTier::Cold));
        let (h, w, c) = tier_counts(&map);
        assert_eq!(h + w + c, 3);
        assert!(format_chunks_query(h, w, c).contains("hot="));
    }

    #[test]
    fn periods() {
        assert_eq!(ChunkTier::Hot.sim_period_ticks(), 1);
        assert!(ChunkTier::Cold.sim_period_ticks() > ChunkTier::Warm.sim_period_ticks());
    }
}
