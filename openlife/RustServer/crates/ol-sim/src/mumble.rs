//! Soft speech range (mumble) between whisper and normal chat.

/// Chebyshev range for `SAY MUMBLE <text>`.
pub const MUMBLE_RANGE: i32 = 4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mumble_narrower_than_adult() {
        // PO-MAX-DISTANCE: adult CloseForSay=20; mumble still narrower
        assert!(MUMBLE_RANGE < 20);
        assert!(MUMBLE_RANGE < 24);
        assert!(MUMBLE_RANGE > 1);
    }
}
