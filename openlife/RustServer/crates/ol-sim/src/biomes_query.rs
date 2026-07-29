//! Bad / impassable biome listing for SAY `?BIOMES` (Haxe BAD_BIOMES subset).

/// One entry in the bad-biome list advertised to clients / chat query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BadBiomeEntry {
    pub id: u8,
    pub name: &'static str,
}

/// Haxe BAD_BIOMES wall set used by login BB + move block (mountain primary).
///
/// Includes mountain (21) plus named hostile/water biomes for query display.
/// MOVE uses Haxe `isBiomeBlocking` (`biome_speed < 0.1` unless floor exception).
pub const BAD_BIOMES: &[BadBiomeEntry] = &[
    BadBiomeEntry {
        id: 21,
        name: "MOUNTAIN",
    },
    BadBiomeEntry {
        id: 9,
        name: "OCEAN",
    },
    BadBiomeEntry {
        id: 17,
        name: "RIVER",
    },
];

/// `SAY ?BIOMES` chat reply body (without leading player id).
///
/// Format: `BIOMES id:NAME id:NAME …`
pub fn format_biomes_query() -> String {
    let parts: Vec<String> = BAD_BIOMES
        .iter()
        .map(|b| format!("{}:{}", b.id, b.name))
        .collect();
    format!("BIOMES {}", parts.join(" "))
}

/// True when `id` is listed as a bad biome (query list).
pub fn is_listed_bad_biome(id: u8) -> bool {
    BAD_BIOMES.iter().any(|b| b.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mountain_listed() {
        assert!(is_listed_bad_biome(21));
        assert!(format_biomes_query().contains("21:MOUNTAIN"));
    }

    #[test]
    fn format_stable() {
        assert_eq!(format_biomes_query(), "BIOMES 21:MOUNTAIN 9:OCEAN 17:RIVER");
    }

    #[test]
    fn green_not_bad() {
        assert!(!is_listed_bad_biome(0));
    }
}
