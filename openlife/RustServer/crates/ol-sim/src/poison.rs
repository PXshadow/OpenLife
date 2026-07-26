//! Food poison detection for FEED/eat (Haxe sick food subset).

/// Name looks like poison / spoiled food.
pub fn name_looks_like_poison(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("poison")
        || n.contains("spoiled")
        || n.contains("rotten")
        || n.contains("toxic")
        || n.contains("nightshade")
}

/// After feeding poison food, target should become sick.
pub fn should_sicken_on_feed(held_name: &str, held_is_food: bool) -> bool {
    held_is_food && name_looks_like_poison(held_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_poison() {
        assert!(name_looks_like_poison("Deadly Poison"));
        assert!(should_sicken_on_feed("Rotten Berry", true));
        assert!(!should_sicken_on_feed("Rotten Berry", false));
        assert!(!name_looks_like_poison("Berry"));
    }
}
