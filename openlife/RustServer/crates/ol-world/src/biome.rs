//! Biome ids and PNG color → biome mapping (Haxe `Biome.hx` / `WorldMap.generate`).

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

/// Haxe `Biome.SGREEN` / `WorldMap.getBiomeSpeed` (without floor overrides).
/// Biomes with speed **&lt; 0.1** block animals/players (`isBiomeBlocking`).
pub fn biome_speed(biome: BiomeId) -> f32 {
    match biome {
        GREEN => 1.0,
        SWAMP => 0.9,
        YELLOW => 1.0,
        GREY => 0.98,
        SNOW => 0.98,
        DESERT => 0.98,
        JUNGLE | BORDER_JUNGLE => 0.98,
        SNOWINGREY => 0.01, // mountain / deep snow-grey
        OCEAN => 0.01,
        RIVER => 0.01,
        PASSABLE_RIVER => 0.98,
        _ => 1.0,
    }
}

/// Haxe `WorldMap.isBiomeBlocking`.
///
/// Any floor with `floorId > 0` **except** Pine Floor `3290` cancels blocking on
/// snowingrey / ocean / passable river / river. Otherwise `getBiomeSpeed < 0.1`.
pub fn is_biome_blocking(biome: BiomeId, floor_id: i32) -> bool {
    // Haxe: if (floorId > 0 && floorId != 3290) allow snowingrey/ocean/rivers
    const PINE_FLOOR: i32 = 3290;
    if floor_id > 0 && floor_id != PINE_FLOOR {
        if matches!(biome, SNOWINGREY | OCEAN | PASSABLE_RIVER | RIVER) {
            return false;
        }
    }
    biome_speed(biome) < 0.1
}

/// Haxe ARGB hex colors (without alpha in match we use RGB).
pub fn biome_from_rgba(r: u8, g: u8, b: u8) -> BiomeId {
    // Compare as 0xRRGGBB
    let rgb = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
    match rgb {
        0xB5E61D => GREEN,         // FFB5E61D
        0x008080 => SWAMP,         // FF008080
        0xFECC36 => YELLOW,        // FFFECC36 savannah
        0x808080 => GREY,          // FF808080
        0xFFFFFF => SNOW,          // FFFFFFFF
        0xDBAC4D => DESERT,        // FFDBAC4D
        0xEFE4B0 => DESERT,        // FFefe4b0 sand
        0x007F0E => JUNGLE,        // FF007F0E
        0x007F00 => BORDER_JUNGLE, // FF007F00
        0x404040 => SNOWINGREY,    // FF404040 mountain snow
        0x004080 => OCEAN,         // FF004080
        0x0080FF => RIVER,         // FF0080FF
        0x00E8FF => PASSABLE_RIVER,// FF00E8FF
        _ => GREEN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_colors() {
        assert_eq!(biome_from_rgba(0xB5, 0xE6, 0x1D), GREEN);
        assert_eq!(biome_from_rgba(0x00, 0x40, 0x80), OCEAN);
        assert_eq!(biome_from_rgba(0xFF, 0xFF, 0xFF), SNOW);
    }
}
