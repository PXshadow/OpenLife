//! OHOL map PNG biome colors ↔ id / name (pure table).
//!
//! Mirrors Haxe `Biome.hx` / `WorldMap.generate` RGB keys used by
//! `ol_world::biome_from_rgba`, with reverse lookup and chat/viewer helpers.
//! Not wired into world gen — pure classification for queries and tooling.

/// RGB triple (0–255 each).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Pack as `0xRRGGBB`.
    pub const fn to_u32(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Parse `0xRRGGBB` (alpha ignored if present in high byte).
    pub const fn from_u32(rgb: u32) -> Self {
        Self {
            r: ((rgb >> 16) & 0xff) as u8,
            g: ((rgb >> 8) & 0xff) as u8,
            b: (rgb & 0xff) as u8,
        }
    }

    /// CSS-style hex without `#`, e.g. `"B5E61D"`.
    pub fn to_hex(self) -> String {
        format!("{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

/// One known map-color → biome mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BiomeColorEntry {
    pub id: u8,
    pub name: &'static str,
    pub color: Rgb,
}

/// Canonical OHOL biome color table (primary map-PNG keys).
///
/// Order is stable for `format_biome_colors_query`. Desert has two PNG colors;
/// both map to id 5 — the first listed is the "primary" display color.
pub const BIOME_COLORS: &[BiomeColorEntry] = &[
    BiomeColorEntry {
        id: 0,
        name: "GREEN",
        color: Rgb::new(0xB5, 0xE6, 0x1D),
    },
    BiomeColorEntry {
        id: 1,
        name: "SWAMP",
        color: Rgb::new(0x00, 0x80, 0x80),
    },
    BiomeColorEntry {
        id: 2,
        name: "YELLOW",
        color: Rgb::new(0xFE, 0xCC, 0x36),
    },
    BiomeColorEntry {
        id: 3,
        name: "GREY",
        color: Rgb::new(0x80, 0x80, 0x80),
    },
    BiomeColorEntry {
        id: 4,
        name: "SNOW",
        color: Rgb::new(0xFF, 0xFF, 0xFF),
    },
    BiomeColorEntry {
        id: 5,
        name: "DESERT",
        color: Rgb::new(0xDB, 0xAC, 0x4D),
    },
    BiomeColorEntry {
        id: 5,
        name: "DESERT_SAND",
        color: Rgb::new(0xEF, 0xE4, 0xB0),
    },
    BiomeColorEntry {
        id: 6,
        name: "JUNGLE",
        color: Rgb::new(0x00, 0x7F, 0x0E),
    },
    BiomeColorEntry {
        id: 9,
        name: "OCEAN",
        color: Rgb::new(0x00, 0x40, 0x80),
    },
    BiomeColorEntry {
        id: 13,
        name: "PASSABLE_RIVER",
        color: Rgb::new(0x00, 0xE8, 0xFF),
    },
    BiomeColorEntry {
        id: 15,
        name: "BORDER_JUNGLE",
        color: Rgb::new(0x00, 0x7F, 0x00),
    },
    BiomeColorEntry {
        id: 17,
        name: "RIVER",
        color: Rgb::new(0x00, 0x80, 0xFF),
    },
    BiomeColorEntry {
        id: 21,
        name: "SNOWINGREY",
        color: Rgb::new(0x40, 0x40, 0x40),
    },
];

/// Map RGB → biome id (unknown → `0` green, matching world gen).
pub fn biome_id_from_rgb(r: u8, g: u8, b: u8) -> u8 {
    let rgb = Rgb::new(r, g, b).to_u32();
    for e in BIOME_COLORS {
        if e.color.to_u32() == rgb {
            return e.id;
        }
    }
    0
}

/// Primary display color for a biome id (`None` if unknown).
pub fn color_for_biome(id: u8) -> Option<Rgb> {
    BIOME_COLORS.iter().find(|e| e.id == id).map(|e| e.color)
}

/// First table name for `id` (`None` if unknown).
pub fn name_for_biome(id: u8) -> Option<&'static str> {
    BIOME_COLORS.iter().find(|e| e.id == id).map(|e| e.name)
}

/// Lookup by case-insensitive name token (`"ocean"`, `"SNOWINGREY"`, …).
pub fn biome_id_from_name(name: &str) -> Option<u8> {
    let n = name.trim();
    if n.is_empty() {
        return None;
    }
    BIOME_COLORS
        .iter()
        .find(|e| e.name.eq_ignore_ascii_case(n))
        .map(|e| e.id)
}

/// Chat / debug body: `BIOMECOLORS id:NAME:RRGGBB …` (unique ids only, first color).
pub fn format_biome_colors_query() -> String {
    let mut seen = [false; 256];
    let mut parts = Vec::new();
    for e in BIOME_COLORS {
        if seen[e.id as usize] {
            continue;
        }
        seen[e.id as usize] = true;
        parts.push(format!("{}:{}:{}", e.id, e.name, e.color.to_hex()));
    }
    format!("BIOMECOLORS {}", parts.join(" "))
}

/// `SAY ?HEX` body: map PNG color under feet for a biome id.
///
/// Format: `HEX {id} {RRGGBB}` when known, else `HEX {id} -`.
pub fn format_hex_query(biome: u8) -> String {
    match color_for_biome(biome) {
        Some(c) => format!("HEX {biome} {}", c.to_hex()),
        None => format!("HEX {biome} -"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_png_colors() {
        assert_eq!(biome_id_from_rgb(0xB5, 0xE6, 0x1D), 0);
        assert_eq!(biome_id_from_rgb(0x00, 0x40, 0x80), 9);
        assert_eq!(biome_id_from_rgb(0xFF, 0xFF, 0xFF), 4);
        assert_eq!(biome_id_from_rgb(0x40, 0x40, 0x40), 21);
        // secondary desert sand color
        assert_eq!(biome_id_from_rgb(0xEF, 0xE4, 0xB0), 5);
    }

    #[test]
    fn unknown_falls_back_to_green() {
        assert_eq!(biome_id_from_rgb(1, 2, 3), 0);
    }

    #[test]
    fn reverse_lookup() {
        assert_eq!(name_for_biome(9), Some("OCEAN"));
        assert_eq!(color_for_biome(0).unwrap().to_hex(), "B5E61D");
        assert_eq!(biome_id_from_name("ocean"), Some(9));
        assert_eq!(biome_id_from_name("SNOWINGREY"), Some(21));
        assert_eq!(biome_id_from_name(""), None);
        assert_eq!(biome_id_from_name("nope"), None);
    }

    #[test]
    fn rgb_pack_roundtrip() {
        let c = Rgb::new(0xDB, 0xAC, 0x4D);
        assert_eq!(c.to_u32(), 0x00DBAC4D);
        assert_eq!(Rgb::from_u32(c.to_u32()), c);
    }

    #[test]
    fn format_lists_unique_ids() {
        let q = format_biome_colors_query();
        assert!(q.starts_with("BIOMECOLORS "));
        assert!(q.contains("0:GREEN:B5E61D"));
        assert!(q.contains("9:OCEAN:004080"));
        // DESERT_SAND is not a separate id listing
        assert!(!q.contains("DESERT_SAND"));
        // id 5 appears once
        assert_eq!(q.matches("5:DESERT:").count(), 1);
    }

    #[test]
    fn format_hex_query_known_and_unknown() {
        assert_eq!(format_hex_query(0), "HEX 0 B5E61D");
        assert_eq!(format_hex_query(5), "HEX 5 DBAC4D");
        assert_eq!(format_hex_query(21), "HEX 21 404040");
        assert_eq!(format_hex_query(255), "HEX 255 -");
    }
}
