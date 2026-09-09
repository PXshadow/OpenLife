//! PrestigeClass + name table (Haxe Lineage.PrestigeClass).

/// Haxe `PrestigeClass` int tags (gaps 4–5 reserved as Noble aliases in Haxe name table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PrestigeClass {
    NotSet = 0,
    Serf = 1,
    #[default]
    Commoner = 2,
    Noble = 3,
    King = 6,
    Emperor = 7,
}

/// Haxe `Lineage.PrestigeClasses` title-case labels indexed by prestige-class int.
///
/// Indices 4 and 5 are Noble aliases (Haxe name table only; enum gaps).
/// // Haxe: Lineage.PrestigeClasses
pub const PRESTIGE_CLASS_NAMES: [&str; 8] = [
    "Not Set",  // 0 NotSet
    "Serf",     // 1
    "Commoner", // 2
    "Noble",    // 3
    "Noble",    // 4 alias
    "Noble",    // 5 alias
    "King",     // 6
    "Emperor",  // 7
];

/// Prestige below this → Serf.
pub const PRESTIGE_SERF_MAX: f32 = 10.0;
/// Prestige below this (and ≥ serf max) → Commoner.
pub const PRESTIGE_COMMONER_MAX: f32 = 50.0;
/// Prestige below this (and ≥ commoner max) → Noble.
pub const PRESTIGE_NOBLE_MAX: f32 = 100.0;
/// Prestige below this (and ≥ noble max) → King; else Emperor.
pub const PRESTIGE_KING_MAX: f32 = 200.0;

/// Haxe `calculateClassBoni` same-class bonus.
pub const CLASS_BONI_SAME: f32 = 2.0;
/// Haxe `calculateClassBoni` Noble↔Serf mismatch mali.
pub const CLASS_BONI_NOBLE_SERF: f32 = -3.0;

/// Haxe design note L6850: first N lives count as "noob" for noble birth weight.
///
/// Count is **completed lives before this birth** (`AccountRecord.lives` is
/// incremented in `on_spawn` **after** birth class is chosen).
// Haxe: GlobalPlayerInstance TODO L1276 / design L6850 "first 5 lifes"
pub const NOOB_NOBLE_MAX_LIVES: u32 = 5;

/// Haxe design note L6850: 50% chance of noble birth while still a noob.
// Haxe: "(new players have a 50% change of noble birth in their first 5 lifes)"
pub const NOOB_NOBLE_BIRTH_CHANCE: f32 = 0.5;

impl PrestigeClass {
    /// Assign class from a prestige float (sim threshold mapping).
    pub fn from_prestige(prestige: f32) -> Self {
        if !prestige.is_finite() || prestige < PRESTIGE_SERF_MAX {
            Self::Serf
        } else if prestige < PRESTIGE_COMMONER_MAX {
            Self::Commoner
        } else if prestige < PRESTIGE_NOBLE_MAX {
            Self::Noble
        } else if prestige < PRESTIGE_KING_MAX {
            Self::King
        } else {
            Self::Emperor
        }
    }

    /// Haxe wire / display name (lowercase, matches `PlayerSoul.getPrestigeClassName`).
    pub fn wire_name(self) -> &'static str {
        match self {
            Self::NotSet => "commoner",
            Self::Serf => "serf",
            Self::Commoner => "commoner",
            Self::Noble => "noble",
            Self::King => "king",
            Self::Emperor => "emperor",
        }
    }

    /// Haxe `Lineage.PrestigeClasses` title-case label for this enum value.
    pub fn class_name(self) -> &'static str {
        prestige_class_name_at_index(self.as_i32())
    }

    /// Haxe `Lineage.isNobleOrMore`.
    pub fn is_noble_or_more(self) -> bool {
        (self as u8) >= (Self::Noble as u8)
    }

    /// Haxe int discriminant.
    pub fn as_i32(self) -> i32 {
        self as u8 as i32
    }

    /// Parse Haxe int tag; unknown values map to `NotSet`.
    /// Noble aliases 4–5 normalize to [`PrestigeClass::Noble`].
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => Self::NotSet,
            1 => Self::Serf,
            2 => Self::Commoner,
            3 | 4 | 5 => Self::Noble, // Haxe name table aliases 4–5 as Noble
            6 => Self::King,
            7 => Self::Emperor,
            _ => Self::NotSet,
        }
    }
}

/// Haxe `Lineage.PrestigeClasses[index]` lookup (0..=7). Out of range → `"Not Set"`.
// Haxe: Lineage.get_className / PrestigeClasses
pub fn prestige_class_name_at_index(index: i32) -> &'static str {
    if index >= 0 && (index as usize) < PRESTIGE_CLASS_NAMES.len() {
        PRESTIGE_CLASS_NAMES[index as usize]
    } else {
        PRESTIGE_CLASS_NAMES[0]
    }
}


/// Compact class+prestige token for lineage / bootstrap wire lines.
pub fn prestige_class_wire_token(prestige: f32) -> String {
    let class = PrestigeClass::from_prestige(prestige);
    format!("class={} prestige={}", class.wire_name(), prestige)
}

