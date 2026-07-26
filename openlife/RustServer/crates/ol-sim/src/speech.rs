//! Age-dependent speech range and volume radii (Haxe chat distance subset).

/// Default adult nearby chat range (matches [`crate::NEARBY_RANGE`] when wired).
pub const ADULT_CHAT_RANGE: i32 = 24;

/// Whisper volume: private / Chebyshev range 1 (targeted WHISPER path).
pub const WHISPER_CHAT_RANGE: i32 = 1;

/// Mumble volume: soft nearby chat (matches [`crate::mumble::MUMBLE_RANGE`]).
pub const MUMBLE_CHAT_RANGE: i32 = 4;

/// Shout Chebyshev range (matches sim SHOUT_RANGE).
pub const SHOUT_CHAT_RANGE: i32 = 48;

/// Chat Chebyshev range based on age years (normal volume).
///
/// - infants (&lt;3): 8
/// - children (&lt;10): 16
/// - adults: [`ADULT_CHAT_RANGE`]
/// - elders (≥60): 20 (slightly reduced)
pub fn chat_range_for_age(age: f32) -> i32 {
    if !age.is_finite() || age < 0.0 {
        return ADULT_CHAT_RANGE;
    }
    if age < 3.0 {
        8
    } else if age < 10.0 {
        16
    } else if age >= 60.0 {
        20
    } else {
        ADULT_CHAT_RANGE
    }
}

/// Speech volume label for diagnostics / tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeechVolume {
    Whisper,
    Mumble,
    Normal,
    Shout,
}

impl SpeechVolume {
    /// Chebyshev fan-out for this volume (Normal uses adult default; prefer
    /// [`chat_range_for_age`] for live speakers).
    pub fn range(self) -> i32 {
        match self {
            Self::Whisper => WHISPER_CHAT_RANGE,
            Self::Mumble => MUMBLE_CHAT_RANGE,
            Self::Normal => ADULT_CHAT_RANGE,
            Self::Shout => SHOUT_CHAT_RANGE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn age_brackets() {
        assert_eq!(chat_range_for_age(0.5), 8);
        assert_eq!(chat_range_for_age(2.9), 8);
        assert_eq!(chat_range_for_age(3.0), 16);
        assert_eq!(chat_range_for_age(9.9), 16);
        assert_eq!(chat_range_for_age(14.0), ADULT_CHAT_RANGE);
        assert_eq!(chat_range_for_age(59.9), ADULT_CHAT_RANGE);
        assert_eq!(chat_range_for_age(60.0), 20);
        assert_eq!(chat_range_for_age(100.0), 20);
    }

    #[test]
    fn nan_defaults_adult() {
        assert_eq!(chat_range_for_age(f32::NAN), ADULT_CHAT_RANGE);
    }

    #[test]
    fn volume_radii_ordered() {
        assert_eq!(SpeechVolume::Whisper.range(), 1);
        assert_eq!(SpeechVolume::Mumble.range(), 4);
        assert_eq!(SpeechVolume::Shout.range(), 48);
        assert!(SpeechVolume::Whisper.range() < SpeechVolume::Mumble.range());
        assert!(SpeechVolume::Mumble.range() < SpeechVolume::Normal.range());
        assert!(SpeechVolume::Normal.range() < SpeechVolume::Shout.range());
    }
}
