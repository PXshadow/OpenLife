//! Shared walk-blocking for the Open Life client and server.
//!
//! Matches Haxe `WorldMap.isBiomeBlocking` / `GlobalPlayerInstance.isBlocked`
//! and Rust `ol-sim::pathfind::is_walkable` (gate/door name exception).
//!
//! Keep this crate dependency-free so the graphical client can use the same
//! function the server uses to reject MOVE steps (no bounce-back).

#![forbid(unsafe_code)]

pub type BiomeId = u8;

pub const GREEN: BiomeId = 0;
pub const SWAMP: BiomeId = 1;
pub const YELLOW: BiomeId = 2;
pub const GREY: BiomeId = 3;
pub const SNOW: BiomeId = 4;
pub const DESERT: BiomeId = 5;
pub const JUNGLE: BiomeId = 6;
pub const BORDER_JUNGLE: BiomeId = 15;
pub const SNOWINGREY: BiomeId = 21;
pub const OCEAN: BiomeId = 9;
pub const PASSABLE_RIVER: BiomeId = 13;
pub const RIVER: BiomeId = 17;

/// Haxe Pine Floor — does **not** cancel biome blocking (not a bridge).
pub const PINE_FLOOR: i32 = 3290;

/// Haxe hard-coded boat ids (`Running Crude Car` 2396, `Delivery Truck` 4655).
pub const BOAT_IDS: &[i32] = &[2396, 4655];

/// Inputs for [`tile_is_blocked`] (one map cell + optional held boat).
#[derive(Debug, Clone, Copy)]
pub struct TileWalkQuery<'a> {
    pub biome: BiomeId,
    pub floor_id: i32,
    pub object_blocks_walking: bool,
    pub object_name: &'a str,
    /// Haxe `heldObject.objectData.isBoat`.
    pub holding_boat: bool,
}

/// Haxe `Biome.SGREEN` / `WorldMap.getBiomeSpeed` (without floor overrides).
pub fn biome_speed(biome: BiomeId) -> f32 {
    match biome {
        GREEN => 1.0,
        SWAMP => 0.9,
        YELLOW => 1.0,
        GREY => 0.98,
        SNOW => 0.98,
        DESERT => 0.98,
        JUNGLE | BORDER_JUNGLE => 0.98,
        SNOWINGREY => 0.01,
        OCEAN => 0.01,
        RIVER => 0.01,
        PASSABLE_RIVER => 0.98,
        _ => 1.0,
    }
}

/// Haxe `Biome.IsWater` — ocean / passable river / river.
#[inline]
pub fn is_water_biome(biome: BiomeId) -> bool {
    matches!(biome, OCEAN | PASSABLE_RIVER | RIVER)
}

/// Haxe `WorldMap.isBiomeBlocking`.
///
/// Any floor with `floorId > 0` **except** Pine Floor `3290` cancels blocking on
/// snowingrey / ocean / passable river / river (bridges, wooden/stone floors).
/// Otherwise `getBiomeSpeed < 0.1`.
pub fn is_biome_blocking(biome: BiomeId, floor_id: i32) -> bool {
    if floor_id > 0 && floor_id != PINE_FLOOR {
        if matches!(biome, SNOWINGREY | OCEAN | PASSABLE_RIVER | RIVER) {
            return false;
        }
    }
    biome_speed(biome) < 0.1
}

/// True when the object name is a gate/door (server pathfind walks through).
pub fn name_is_gate_or_door(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("gate") || n.contains("door")
}

/// Object contribution to blocking (Haxe `obj.blocksWalking` + gate/door exception).
pub fn object_blocks_player(blocks_walking: bool, name: &str) -> bool {
    if !blocks_walking {
        return false;
    }
    !name_is_gate_or_door(name)
}

/// Haxe `heldObject.objectData.isBoat` (+isBoat tag / Sports Car / hard-coded ids).
pub fn object_is_boat(object_id: i32, description: &str, name: &str) -> bool {
    if object_id <= 0 {
        return false;
    }
    if BOAT_IDS.contains(&object_id) {
        return true;
    }
    let hay = if !description.is_empty() {
        description
    } else {
        name
    };
    hay.contains("+isBoat") || hay.contains("Sports Car")
}

/// Full tile block: object, then boat-on-water exception, then biome/floor.
///
/// Haxe `GlobalPlayerInstance.isBlocked` plus the server gate/door walk exception.
pub fn tile_is_blocked(q: TileWalkQuery<'_>) -> bool {
    if object_blocks_player(q.object_blocks_walking, q.object_name) {
        return true;
    }
    if q.holding_boat && is_water_biome(q.biome) {
        return false;
    }
    is_biome_blocking(q.biome, q.floor_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocean_blocks_without_floor() {
        assert!(is_biome_blocking(OCEAN, 0));
        assert!(is_biome_blocking(RIVER, 0));
        assert!(is_biome_blocking(SNOWINGREY, 0));
        assert!(!is_biome_blocking(PASSABLE_RIVER, 0));
        assert!(!is_biome_blocking(GREEN, 0));
        assert!(!is_biome_blocking(YELLOW, 0));
        assert!(!is_biome_blocking(JUNGLE, 0));
        assert!(!is_biome_blocking(BORDER_JUNGLE, 0));
    }

    #[test]
    fn bridge_floor_cancels_water_block() {
        assert!(!is_biome_blocking(OCEAN, 485));
        assert!(!is_biome_blocking(OCEAN, 884));
        assert!(!is_biome_blocking(RIVER, 898));
        assert!(is_biome_blocking(OCEAN, PINE_FLOOR));
    }

    #[test]
    fn tile_is_blocked_object_and_boat() {
        assert!(tile_is_blocked(TileWalkQuery {
            biome: GREEN,
            floor_id: 0,
            object_blocks_walking: true,
            object_name: "Wall",
            holding_boat: false,
        }));
        assert!(!tile_is_blocked(TileWalkQuery {
            biome: GREEN,
            floor_id: 0,
            object_blocks_walking: true,
            object_name: "Wood Door",
            holding_boat: false,
        }));
        assert!(!tile_is_blocked(TileWalkQuery {
            biome: OCEAN,
            floor_id: 0,
            object_blocks_walking: false,
            object_name: "",
            holding_boat: true,
        }));
        assert!(tile_is_blocked(TileWalkQuery {
            biome: OCEAN,
            floor_id: 0,
            object_blocks_walking: false,
            object_name: "",
            holding_boat: false,
        }));
        assert!(!tile_is_blocked(TileWalkQuery {
            biome: OCEAN,
            floor_id: 485,
            object_blocks_walking: false,
            object_name: "",
            holding_boat: false,
        }));
    }

    #[test]
    fn boat_ids_and_tag() {
        assert!(object_is_boat(2396, "", ""));
        assert!(object_is_boat(50, "Raft +isBoat", "Raft"));
        assert!(!object_is_boat(33, "Sharp Stone#tool", "Sharp Stone"));
    }
}
