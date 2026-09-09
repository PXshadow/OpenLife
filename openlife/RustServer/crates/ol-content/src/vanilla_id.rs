//! Haxe `Server.mapIdToVanillaId` / `VanillaObjIdMap` / `InitVanillaObjectIdMap`.
//!
//! Vanilla (non-OpenLife) clients cannot display OpenLife-only object ids.
//! Mapping is **wire-only** — world storage keeps raw OpenLife ids.
//!
//! Haxe: `Server.mapIdToVanillaId`, `ServerSettings.InitVanillaObjectIdMap`,
//! `Server.lastOpenLifeID` (set before dummy objects).

use crate::ContentDb;
use std::collections::HashMap;

/// Parse `+VanillaId N` from an object description (Haxe `InitVanillaObjectIdMap`).
///
/// Haxe used `substring(pos + 12)` on an 11-char prefix (off-by-one). This
/// parser takes the integer after the full `"+VanillaId "` token.
// Haxe: ServerSettings.InitVanillaObjectIdMap
pub fn parse_vanilla_id_from_description(desc: &str) -> Option<i32> {
    const PREFIX: &str = "+VanillaId ";
    let pos = desc.find(PREFIX)?;
    let mut rest = desc[pos + PREFIX.len()..].trim_start();
    if let Some(sp) = rest.find(char::is_whitespace) {
        rest = &rest[..sp];
    }
    if rest.is_empty() {
        return None;
    }
    // Haxe Std.parseInt: leading optional '-' then digits.
    let mut chars = rest.chars();
    let mut digits = String::new();
    if let Some(c) = chars.next() {
        if c == '-' || c.is_ascii_digit() {
            digits.push(c);
        } else {
            return None;
        }
    }
    for c in chars {
        if c.is_ascii_digit() {
            digits.push(c);
        } else {
            break;
        }
    }
    if digits.is_empty() || digits == "-" {
        return None;
    }
    digits.parse().ok()
}

/// Haxe `Server.mapIdToVanillaId(id)`.
///
/// - `last_vanilla_id < 1` → identity (mapping off; product default −1)
/// - `id <= last_vanilla_id` → identity (already vanilla)
/// - `id > last_open_life_id` → `id - (last_open_life_id - last_vanilla_id)` (dummy space)
/// - else map lookup, or **0** if unmapped (OpenLife-only)
// Haxe: Server.mapIdToVanillaId
pub fn map_id_to_vanilla_id(
    id: i32,
    last_vanilla_id: i32,
    last_open_life_id: i32,
    map: &HashMap<i32, i32>,
) -> i32 {
    if last_vanilla_id < 1 {
        return id;
    }
    if id <= last_vanilla_id {
        return id;
    }
    let id_offset = last_open_life_id - last_vanilla_id;
    if id > last_open_life_id {
        return id - id_offset;
    }
    map.get(&id).copied().unwrap_or(0)
}

/// Rewrite integer tokens in a Haxe map-cell object string (`391,33:100`).
pub fn map_object_id_string(s: &str, map_id: impl Fn(i32) -> i32) -> String {
    let mut out = String::with_capacity(s.len());
    let mut num = String::new();
    let flush = |num: &mut String, out: &mut String, map_id: &dyn Fn(i32) -> i32| {
        if num.is_empty() {
            return;
        }
        if let Ok(v) = num.parse::<i32>() {
            out.push_str(&map_id(v).to_string());
        } else {
            out.push_str(num.as_str());
        }
        num.clear();
    };
    for c in s.chars() {
        if c.is_ascii_digit() || (c == '-' && num.is_empty()) {
            num.push(c);
        } else {
            flush(&mut num, &mut out, &map_id);
            out.push(c);
        }
    }
    flush(&mut num, &mut out, &map_id);
    out
}

/// Fill `VanillaObjIdMap` from `+VanillaId N` on each object description.
///
/// Key is Haxe `ObjectData.parentId` (dummy → parent; else id).
// Haxe: ServerSettings.InitVanillaObjectIdMap / parseTags
pub fn init_vanilla_object_id_map(db: &mut ContentDb) {
    db.vanilla_obj_id_map.clear();
    let mut pairs = Vec::new();
    for (id, obj) in &db.objects {
        if let Some(vanilla) = parse_vanilla_id_from_description(&obj.description) {
            let parent = db.dummy_parent.get(id).copied().unwrap_or(*id);
            pairs.push((parent, vanilla));
        }
    }
    for (parent, vanilla) in pairs {
        db.vanilla_obj_id_map.insert(parent, vanilla);
    }
}

/// Haxe `Server.lastOpenLifeID` after dummy allocation (min dummy id − 1).
///
/// Text load sets this from `nextObjectNumber` **before** dummies; cache load
/// recovers it from `dummy_parent` keys.
pub fn stamp_last_open_life_id_from_dummies(db: &mut ContentDb) {
    if let Some(min_d) = db.dummy_parent.keys().copied().min() {
        db.last_open_life_id = min_d - 1;
    } else if db.last_open_life_id == 0 {
        db.last_open_life_id = db.objects.keys().copied().max().unwrap_or(0);
    }
}

impl ContentDb {
    /// Haxe `Server.mapIdToVanillaId` using this db's map + `last_open_life_id`.
    #[inline]
    pub fn map_id_to_vanilla_id(&self, id: i32, last_vanilla_id: i32) -> i32 {
        map_id_to_vanilla_id(
            id,
            last_vanilla_id,
            self.last_open_life_id,
            &self.vanilla_obj_id_map,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ObjectDef;

    #[test]
    fn mapping_off_when_last_vanilla_below_one() {
        let mut map = HashMap::new();
        map.insert(150, 33);
        assert_eq!(map_id_to_vanilla_id(150, -1, 200, &map), 150);
        assert_eq!(map_id_to_vanilla_id(150, 0, 200, &map), 150);
        assert_eq!(map_id_to_vanilla_id(250, -1, 200, &map), 250);
    }

    #[test]
    fn ids_at_or_below_last_vanilla_pass_through() {
        let map = HashMap::new();
        assert_eq!(map_id_to_vanilla_id(0, 100, 200, &map), 0);
        assert_eq!(map_id_to_vanilla_id(50, 100, 200, &map), 50);
        assert_eq!(map_id_to_vanilla_id(100, 100, 200, &map), 100);
    }

    #[test]
    fn openlife_range_uses_map_or_zero() {
        let mut map = HashMap::new();
        map.insert(150, 33);
        assert_eq!(map_id_to_vanilla_id(150, 100, 200, &map), 33);
        assert_eq!(map_id_to_vanilla_id(160, 100, 200, &map), 0);
        assert_eq!(map_id_to_vanilla_id(101, 100, 200, &map), 0);
        assert_eq!(map_id_to_vanilla_id(200, 100, 200, &map), 0);
    }

    #[test]
    fn dummy_ids_above_last_open_life_offset() {
        let map = HashMap::new();
        // idOffset = 200 - 100 = 100; dummy 250 → 150
        assert_eq!(map_id_to_vanilla_id(201, 100, 200, &map), 101);
        assert_eq!(map_id_to_vanilla_id(250, 100, 200, &map), 150);
    }

    #[test]
    fn parse_vanilla_id_tag() {
        assert_eq!(
            parse_vanilla_id_from_description("Carrot +VanillaId 33"),
            Some(33)
        );
        assert_eq!(
            parse_vanilla_id_from_description("Foo# +VanillaId 12 extra"),
            Some(12)
        );
        assert_eq!(parse_vanilla_id_from_description("no tag"), None);
        assert_eq!(
            parse_vanilla_id_from_description("X +VanillaId 99foo"),
            Some(99)
        );
    }

    #[test]
    fn init_map_uses_parent_id() {
        let mut db = ContentDb::default();
        let mut obj = ObjectDef::empty(150);
        obj.description = "New Carrot +VanillaId 33".into();
        db.objects.insert(150, obj);
        init_vanilla_object_id_map(&mut db);
        assert_eq!(db.vanilla_obj_id_map.get(&150), Some(&33));
        db.last_open_life_id = 200;
        assert_eq!(db.map_id_to_vanilla_id(150, 100), 33);
        assert_eq!(db.map_id_to_vanilla_id(50, 100), 50);
    }

    #[test]
    fn map_object_id_string_nested() {
        let mapped = map_object_id_string("391,292:100:101", |id| {
            if id == 391 {
                40
            } else if id == 100 {
                7
            } else {
                id
            }
        });
        assert_eq!(mapped, "40,292:7:101");
    }

    #[test]
    fn stamp_last_open_life_from_dummy_parent() {
        let mut db = ContentDb::default();
        db.dummy_parent.insert(5000, 30);
        db.dummy_parent.insert(5001, 30);
        stamp_last_open_life_id_from_dummies(&mut db);
        assert_eq!(db.last_open_life_id, 4999);
    }
}
