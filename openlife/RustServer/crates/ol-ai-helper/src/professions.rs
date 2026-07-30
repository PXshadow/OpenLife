//! Lite profession actions: `SAY HARVEST` / `FISH` / `MINE` / `DIG` / `CHOP`.
//!
//! Biome-gated gather with a shared 5s cooldown (caller tracks
//! `last_prof_action_time` on the player body).
//!
//! // OL-AI-SPLIT: moved from ol-sim

use ol_content::ContentDb;

/// Shared cooldown (sim seconds) between successful profession actions.
pub const PROF_ACTION_COOLDOWN_SECS: f32 = 5.0;

/// Grassland / green biome id (Haxe `GREEN`).
pub const GRASSLAND_BIOME: u8 = 0;

/// Swamp biome id (Haxe `SWAMP`).
pub const SWAMP_BIOME: u8 = 1;

/// Yellow / savannah biome id (Haxe `YELLOW`).
pub const YELLOW_BIOME: u8 = 2;

/// Grey rock biome (counts as mountain for MINE adjacency).
pub const GREY_BIOME: u8 = 3;

/// Jungle biome id (Haxe `JUNGLE`).
pub const JUNGLE_BIOME: u8 = 6;

/// Ocean biome id.
pub const OCEAN_BIOME: u8 = 9;

/// Passable river biome id.
pub const PASSABLE_RIVER_BIOME: u8 = 13;

/// Border jungle biome id.
pub const BORDER_JUNGLE_BIOME: u8 = 15;

/// Impassable river biome id.
pub const RIVER_BIOME: u8 = 17;

/// Mountain wall biome id (Haxe `SNOWINGREY`).
pub const MOUNTAIN_BIOME: u8 = 21;

/// Harvest fallback when content has no berry / food object (Gooseberry).
pub const HARVEST_FALLBACK_ID: i32 = 33;

/// Fish object placeholder until content-wired.
pub const FISH_PLACEHOLDER_ID: i32 = 9101;

/// Stone object placeholder until content-wired.
pub const STONE_PLACEHOLDER_ID: i32 = 9102;

/// Dig (clay/soil) placeholder until content-wired.
pub const CLAY_PLACEHOLDER_ID: i32 = 9103;

/// Chop (wood) placeholder until content-wired.
pub const WOOD_PLACEHOLDER_ID: i32 = 9104;

/// Outcome of a lite profession action attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfActionResult {
    /// Object id placed in empty hands.
    Ok { object_id: i32 },
    /// Shared 5s cooldown not ready.
    Cooldown,
    /// Hands not empty.
    Hands,
    /// Standing biome wrong for this action.
    Biome,
    /// No adjacent mountain for MINE.
    Adjacent,
}

impl ProfActionResult {
    /// Suffix after `HARVEST` / `FISH` / â€¦ for private PS body.
    pub fn wire_suffix(self) -> String {
        match self {
            Self::Ok { object_id } => format!("OK id={object_id}"),
            Self::Cooldown => "FAIL COOLDOWN".into(),
            Self::Hands => "FAIL HANDS".into(),
            Self::Biome => "FAIL BIOME".into(),
            Self::Adjacent => "FAIL ADJACENT".into(),
        }
    }
}

/// True when enough sim time has passed since the last successful prof action.
pub fn prof_cooldown_ready(last_prof_action_time: f32, sim_time: f32) -> bool {
    if !last_prof_action_time.is_finite() || !sim_time.is_finite() {
        return true;
    }
    sim_time - last_prof_action_time >= PROF_ACTION_COOLDOWN_SECS
}

/// Standing biome is grassland (green).
pub fn is_grassland(biome: u8) -> bool {
    biome == GRASSLAND_BIOME
}

/// Standing biome is ocean or river (fishing waters).
pub fn is_fishing_biome(biome: u8) -> bool {
    matches!(biome, OCEAN_BIOME | RIVER_BIOME | PASSABLE_RIVER_BIOME)
}

/// Standing biome is swamp.
pub fn is_swamp(biome: u8) -> bool {
    biome == SWAMP_BIOME
}

/// Standing biome is jungle or yellow (chop wood).
pub fn is_chop_biome(biome: u8) -> bool {
    matches!(biome, JUNGLE_BIOME | YELLOW_BIOME | BORDER_JUNGLE_BIOME)
}

/// Biome counts as mountain for mining adjacency.
pub fn is_mountain_biome(biome: u8) -> bool {
    matches!(biome, MOUNTAIN_BIOME | GREY_BIOME)
}

/// True if any Chebyshev-1 neighbor (including own tile) is mountain/grey.
pub fn mountain_adjacent(
    center_x: i32,
    center_y: i32,
    mut get_biome: impl FnMut(i32, i32) -> u8,
) -> bool {
    for dy in -1..=1 {
        for dx in -1..=1 {
            if is_mountain_biome(get_biome(center_x + dx, center_y + dy)) {
                return true;
            }
        }
    }
    false
}

/// Lowest object id whose name contains any needle (case-insensitive).
pub fn first_id_name_contains(content: &ContentDb, needles: &[&str]) -> Option<i32> {
    let mut best: Option<i32> = None;
    for (&id, def) in &content.objects {
        if id <= 0 {
            continue;
        }
        let n = def.name.to_ascii_lowercase();
        if needles
            .iter()
            .any(|k| n.contains(&k.to_ascii_lowercase()))
        {
            best = Some(match best {
                Some(b) => b.min(id),
                None => id,
            });
        }
    }
    best
}

/// Sorted object ids with `food_value > 0` (stable pick order).
pub fn collect_food_ids(content: &ContentDb) -> Vec<i32> {
    let mut ids: Vec<i32> = content
        .objects
        .values()
        .filter(|d| d.food_value > 0 && d.id > 0)
        .map(|d| d.id)
        .collect();
    ids.sort_unstable();
    ids
}

/// Deterministic pick from food ids using `seed` (legacy helper).
pub fn pick_food_id(food_ids: &[i32], seed: u64) -> Option<i32> {
    if food_ids.is_empty() {
        return None;
    }
    let i = (seed as usize) % food_ids.len();
    Some(food_ids[i])
}

/// Food-like harvest id: first name containing `"berry"`, else lowest `food_value > 0`, else 33.
pub fn pick_harvest_id(content: &ContentDb) -> i32 {
    first_id_name_contains(content, &["berry"])
        .or_else(|| collect_food_ids(content).into_iter().next())
        .unwrap_or(HARVEST_FALLBACK_ID)
}

fn gate_common(
    held_id: i32,
    last_prof_action_time: f32,
    sim_time: f32,
) -> Result<(), ProfActionResult> {
    if !prof_cooldown_ready(last_prof_action_time, sim_time) {
        return Err(ProfActionResult::Cooldown);
    }
    if held_id != 0 {
        return Err(ProfActionResult::Hands);
    }
    Ok(())
}

/// `SAY HARVEST`: empty hands + grassland â†’ food-like id (first berry / food / 33).
pub fn try_harvest(
    held_id: i32,
    biome: u8,
    last_prof_action_time: f32,
    sim_time: f32,
    content: &ContentDb,
) -> ProfActionResult {
    if let Err(e) = gate_common(held_id, last_prof_action_time, sim_time) {
        return e;
    }
    if !is_grassland(biome) {
        return ProfActionResult::Biome;
    }
    ProfActionResult::Ok {
        object_id: pick_harvest_id(content),
    }
}

/// `SAY FISH`: empty hands + ocean/river â†’ fish placeholder.
pub fn try_fish(
    held_id: i32,
    biome: u8,
    last_prof_action_time: f32,
    sim_time: f32,
) -> ProfActionResult {
    if let Err(e) = gate_common(held_id, last_prof_action_time, sim_time) {
        return e;
    }
    if !is_fishing_biome(biome) {
        return ProfActionResult::Biome;
    }
    ProfActionResult::Ok {
        object_id: FISH_PLACEHOLDER_ID,
    }
}

/// `SAY MINE`: empty hands + mountain (standing or adj) â†’ stone placeholder.
pub fn try_mine(
    held_id: i32,
    last_prof_action_time: f32,
    sim_time: f32,
    mountain_near: bool,
) -> ProfActionResult {
    if let Err(e) = gate_common(held_id, last_prof_action_time, sim_time) {
        return e;
    }
    if !mountain_near {
        return ProfActionResult::Adjacent;
    }
    ProfActionResult::Ok {
        object_id: STONE_PLACEHOLDER_ID,
    }
}

/// `SAY DIG`: empty hands + swamp â†’ clay placeholder.
pub fn try_dig(
    held_id: i32,
    biome: u8,
    last_prof_action_time: f32,
    sim_time: f32,
) -> ProfActionResult {
    if let Err(e) = gate_common(held_id, last_prof_action_time, sim_time) {
        return e;
    }
    if !is_swamp(biome) {
        return ProfActionResult::Biome;
    }
    ProfActionResult::Ok {
        object_id: CLAY_PLACEHOLDER_ID,
    }
}

/// `SAY CHOP`: empty hands + jungle/yellow â†’ wood placeholder.
pub fn try_chop(
    held_id: i32,
    biome: u8,
    last_prof_action_time: f32,
    sim_time: f32,
) -> ProfActionResult {
    if let Err(e) = gate_common(held_id, last_prof_action_time, sim_time) {
        return e;
    }
    if !is_chop_biome(biome) {
        return ProfActionResult::Biome;
    }
    ProfActionResult::Ok {
        object_id: WOOD_PLACEHOLDER_ID,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::{ContentDb, ObjectDef};

    fn obj(id: i32, name: &str, food: i32) -> ObjectDef {
        ObjectDef {
            id,
            description: name.into(),
            name: name.into(),
            containable: true,
            permanent: false,
            blocks_walking: false,
            food_value: food,
            heat_value: 0.0,
            map_chance: 0.0,
            biomes: Vec::new(),
            num_uses: 0,
            num_slots: 0,
            floor: false,
        dummy_ids: Vec::new(),
        use_chance: 0.0,
        speed_mult: 1.0,
        winter_decay_factor: 0.0,
        spring_regrow_factor: 0.0,
        decay_factor: 1.0,
        decays_to_obj: 0,
        r_value: 0.0,
        clothing: "n".into(),
        counts_or_grows_as: 0,
        crafting_steps: 0,
        use_distance: 1,
        deadly_distance: 0.0,
        moves: 0,
        damage: 0.0,
        damage_protection_factor: 1.0,
        wound_factor: 0.5,
        male: false,
        contain_size: 0.0,
        slot_size: 1.0,
        }
    }

    fn food_db() -> ContentDb {
        let mut db = ContentDb::default();
        db.objects.insert(33, obj(33, "Gooseberry", 3));
        db.objects.insert(44, obj(44, "Carrot", 2));
        db.objects.insert(1, ObjectDef::empty(1));
        db
    }

    #[test]
    fn cooldown_gates() {
        assert!(prof_cooldown_ready(-100.0, 0.0));
        assert!(!prof_cooldown_ready(0.0, 4.9));
        assert!(prof_cooldown_ready(0.0, 5.0));
    }

    #[test]
    fn harvest_grassland_first_berry_or_33() {
        let db = food_db();
        // Gooseberry is first berry (and also food); always 33 when present.
        let r = try_harvest(0, GRASSLAND_BIOME, -10.0, 0.0, &db);
        assert_eq!(r, ProfActionResult::Ok { object_id: 33 });
        assert_eq!(r.wire_suffix(), "OK id=33");

        // No berry name but food present â†’ lowest food id.
        let mut food_only = ContentDb::default();
        food_only.objects.insert(44, obj(44, "Carrot", 2));
        food_only.objects.insert(50, obj(50, "Onion", 1));
        assert_eq!(
            try_harvest(0, GRASSLAND_BIOME, -10.0, 0.0, &food_only),
            ProfActionResult::Ok { object_id: 44 }
        );

        // Empty content â†’ fallback 33.
        assert_eq!(
            try_harvest(0, GRASSLAND_BIOME, -10.0, 0.0, &ContentDb::default()),
            ProfActionResult::Ok {
                object_id: HARVEST_FALLBACK_ID
            }
        );
    }

    #[test]
    fn harvest_fail_paths() {
        let db = food_db();
        assert_eq!(
            try_harvest(7, GRASSLAND_BIOME, -10.0, 0.0, &db),
            ProfActionResult::Hands
        );
        assert_eq!(
            try_harvest(0, OCEAN_BIOME, -10.0, 0.0, &db),
            ProfActionResult::Biome
        );
        assert_eq!(
            try_harvest(0, GRASSLAND_BIOME, 0.0, 1.0, &db),
            ProfActionResult::Cooldown
        );
    }

    #[test]
    fn fish_ocean_river() {
        assert_eq!(
            try_fish(0, OCEAN_BIOME, -10.0, 0.0),
            ProfActionResult::Ok {
                object_id: FISH_PLACEHOLDER_ID
            }
        );
        assert_eq!(
            try_fish(0, RIVER_BIOME, -10.0, 0.0),
            ProfActionResult::Ok {
                object_id: FISH_PLACEHOLDER_ID
            }
        );
        assert_eq!(
            try_fish(0, PASSABLE_RIVER_BIOME, -10.0, 0.0),
            ProfActionResult::Ok {
                object_id: FISH_PLACEHOLDER_ID
            }
        );
        assert_eq!(
            try_fish(0, GRASSLAND_BIOME, -10.0, 0.0),
            ProfActionResult::Biome
        );
        assert_eq!(try_fish(1, OCEAN_BIOME, -10.0, 0.0), ProfActionResult::Hands);
    }

    #[test]
    fn mine_adjacent_mountain() {
        assert_eq!(
            try_mine(0, -10.0, 0.0, true),
            ProfActionResult::Ok {
                object_id: STONE_PLACEHOLDER_ID
            }
        );
        assert_eq!(try_mine(0, -10.0, 0.0, false), ProfActionResult::Adjacent);
        assert_eq!(try_mine(9, -10.0, 0.0, true), ProfActionResult::Hands);
        assert_eq!(try_mine(0, 0.0, 2.0, true), ProfActionResult::Cooldown);
    }

    #[test]
    fn dig_swamp() {
        assert_eq!(
            try_dig(0, SWAMP_BIOME, -10.0, 0.0),
            ProfActionResult::Ok {
                object_id: CLAY_PLACEHOLDER_ID
            }
        );
        assert_eq!(
            try_dig(0, GRASSLAND_BIOME, -10.0, 0.0),
            ProfActionResult::Biome
        );
        assert_eq!(try_dig(1, SWAMP_BIOME, -10.0, 0.0), ProfActionResult::Hands);
    }

    #[test]
    fn chop_jungle_yellow() {
        assert_eq!(
            try_chop(0, JUNGLE_BIOME, -10.0, 0.0),
            ProfActionResult::Ok {
                object_id: WOOD_PLACEHOLDER_ID
            }
        );
        assert_eq!(
            try_chop(0, YELLOW_BIOME, -10.0, 0.0),
            ProfActionResult::Ok {
                object_id: WOOD_PLACEHOLDER_ID
            }
        );
        assert_eq!(
            try_chop(0, BORDER_JUNGLE_BIOME, -10.0, 0.0),
            ProfActionResult::Ok {
                object_id: WOOD_PLACEHOLDER_ID
            }
        );
        assert_eq!(
            try_chop(0, GRASSLAND_BIOME, -10.0, 0.0),
            ProfActionResult::Biome
        );
    }

    #[test]
    fn mountain_adjacent_scan() {
        assert!(mountain_adjacent(0, 0, |x, y| {
            if x == 1 && y == 0 {
                MOUNTAIN_BIOME
            } else {
                GRASSLAND_BIOME
            }
        }));
        assert!(!mountain_adjacent(0, 0, |_, _| GRASSLAND_BIOME));
        assert!(mountain_adjacent(5, 5, |x, y| {
            if x == 5 && y == 5 {
                GREY_BIOME
            } else {
                GRASSLAND_BIOME
            }
        }));
    }

    #[test]
    fn pick_harvest_prefers_berry_name() {
        let mut db = ContentDb::default();
        db.objects.insert(10, obj(10, "Carrot", 2));
        db.objects.insert(50, obj(50, "Wild Gooseberry", 3));
        db.objects.insert(40, obj(40, "Blueberry", 1));
        assert_eq!(pick_harvest_id(&db), 40);
    }
}
