//! Haxe `ObjectHelper.isWound` / `isArrowWound` (description + content).

use ol_content::ContentDb;

/// Haxe `ObjectHelper.isWound` — description contains Wound / Snake Bite / Hog Cut.
pub fn is_wound_description(description: &str) -> bool {
    let d = description;
    d.contains("Snake Bite") || d.contains("Hog Cut") || d.contains("Wound")
}

/// Haxe `ObjectHelper.isArrowWound` — description contains `"Arrow Wound"`.
#[inline]
pub fn is_arrow_wound_description(description: &str) -> bool {
    description.contains("Arrow Wound")
}

/// Content-backed wound check for a held object id (`0` = empty → not a wound).
pub fn is_wound_object(content: &ContentDb, object_id: i32) -> bool {
    if object_id == 0 {
        return false;
    }
    content
        .objects
        .get(&object_id)
        .map(|o| is_wound_description(&o.description) || is_wound_description(&o.name))
        .unwrap_or(false)
}

/// Content-backed arrow-wound check (`0` = empty → false).
pub fn is_arrow_wound_object(content: &ContentDb, object_id: i32) -> bool {
    if object_id == 0 {
        return false;
    }
    content
        .objects
        .get(&object_id)
        .map(|o| is_arrow_wound_description(&o.description) || is_arrow_wound_description(&o.name))
        .unwrap_or(false)
}
