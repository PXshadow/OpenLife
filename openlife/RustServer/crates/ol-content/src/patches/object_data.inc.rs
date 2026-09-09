/// Haxe `ServerSettings` secondTimeOutcome patches (goose pond, rabbits, …).
///
/// Only inserts when the object id exists in `db.objects` (or always for known
/// ids so unit tests / partial loads can opt-in by inserting outcomes manually).
pub fn apply_default_second_time_outcomes(db: &mut ContentDb) {
    // (object_id, outcome_id, seconds)
    // Haxe ServerSettings.PatchObjectData subset used by DoSecondTimeOutcome.
    const PATCHES: &[(i32, i32, f32)] = &[
        (141, 142, 30.0),               // Canada Goose Pond → swimming
        (142, 1261, 60.0 * 10.0),       // swimming → with Egg
        (1261, 142, 60.0 * 4.0),        // with Egg → swimming
        (511, 142, 60.0 * 60.0 * 24.0), // Pond → goose swimming
        (164, 173, 90.0),               // Rabbit Hole out,single → Family Hole out
        (173, 3566, 90.0),              // Family Hole out → Fleeing Rabbit
        (1438, 1435, 30.0 * 60.0),      // Shot Bison → Bison
        (1440, 1436, 30.0 * 60.0),      // Shot Bison with Calf → Bison
    ];
    for &(id, out, secs) in PATCHES {
        db.second_time_outcomes.entry(id).or_insert((out, secs));
    }
}

/// Haxe `ServerSettings.AnimalDecayFactor` default (live: `GameplayKnobs.animal_decay_factor`).
// SETTINGS-LONG-TAIL
pub const ANIMAL_DECAY_FACTOR: f32 = 0.05;

/// Object ids whose `decayFactor` is patched to `AnimalDecayFactor` (Haxe PatchObjectData).
pub const ANIMAL_DECAY_FACTOR_IDS: &[i32] = &[
    3159, 779, 774, // hitched horse carts / riding horse
    1458, 1488, 1454, 1489, 1459, 1462, 1485, // cows / calves
    575, 4213, 600, 576, 542, 604, // sheep / lambs
    418, 420, // wolves
];

/// True when Haxe `PatchObjectData` assigns `decayFactor = AnimalDecayFactor`.
#[inline]
pub fn is_animal_decay_factor_id(id: i32) -> bool {
    ANIMAL_DECAY_FACTOR_IDS.contains(&id)
}
/// Haxe `ServerSettings.ObjDecayFactorForPermanentObjs` (used when patch divides by it).
const OBJ_DECAY_FACTOR_FOR_PERMANENT: f32 = 0.2;

/// Haxe `ServerSettings.PatchObjectData` weapon `useDistance` / `deadlyDistance` (IS-CLOSE).
///
/// Safe if id missing. Binary cache may still carry object-file values; patches
/// keep bows at range 5 and deadly 4 so USE min-range works.
// Haxe: ServerSettings.PatchObjectData deadlyDistance weapons + object-file useDistance
pub fn apply_default_weapon_range_patches(db: &mut ContentDb) {
    // (id, use_distance, deadly_distance) — None = leave field unchanged.
    const PATCHES: &[(i32, Option<i32>, Option<f32>)] = &[
        (152, Some(5), Some(4.0)),  // Bow and Arrow
        (1624, Some(5), Some(4.0)), // Bow and Arrow with Note
        (749, Some(5), Some(4.0)),  // Bloody Yew Bow
        (560, None, Some(1.5)),     // Knife
        (3047, None, Some(1.5)),    // War Sword
        (750, None, Some(1.5)),     // Bloody Knife
        (3048, None, Some(1.5)),    // Bloody War Sword
    ];
    for &(id, use_d, deadly) in PATCHES {
        if let Some(d) = db.objects.get_mut(&id) {
            if let Some(u) = use_d {
                d.use_distance = u;
            }
            if let Some(dd) = deadly {
                d.deadly_distance = dd;
            }
        }
    }
}

/// Haxe `ServerSettings.PatchObjectData` weapon `minPickupAge`.
///
/// Haxe writes 151 twice (War Sword comment then Yew Bow) — last write wins (5).
/// Knife 560 → 2. Safe if id missing.
// Haxe: ServerSettings.PatchObjectData L1774–1776
// MIN-PICKUP-AGE
pub fn apply_default_min_pickup_age_patches(db: &mut ContentDb) {
    // Port-as-is: both writes hit id 151; do not "fix" War Sword 3047.
    const PATCHES: &[(i32, i32)] = &[
        (151, 10), // overwritten below (Haxe first write)
        (151, 5),  // Yew Bow (Haxe last write)
        (560, 2),  // Knife
    ];
    for &(id, age) in PATCHES {
        if let Some(d) = db.objects.get_mut(&id) {
            d.min_pickup_age = age;
        }
    }
}

/// Haxe `ServerSettings.AnimalDeadlyDistanceFactor` (default 0.5).
/// How close an animal must be to land a hit (`ObjectData.deadlyDistance`).
// Haxe: ServerSettings.AnimalDeadlyDistanceFactor
pub const ANIMAL_DEADLY_DISTANCE_FACTOR: f32 = 0.5;

/// Haxe `ServerSettings.PatchObjectData` animal `deadlyDistance = AnimalDeadlyDistanceFactor`.
///
/// Object files often store `deadlyDistance=1`; boot overwrites combat animals to 0.5.
/// Safe if id missing. Complements damage patches in `apply_default_combat_damage_patches`.
// Haxe: ServerSettings.PatchObjectData animal deadlyDistance
pub fn apply_default_animal_deadly_distance_patches(db: &mut ContentDb) {
    apply_animal_deadly_distance_patches_ex(db, ANIMAL_DEADLY_DISTANCE_FACTOR);
}

/// Live-knob variant: `factor` = Haxe AnimalDeadlyDistanceFactor.
// Haxe: ServerSettings.AnimalDeadlyDistanceFactor
// SETTINGS-LONG-TAIL
pub fn apply_animal_deadly_distance_patches_ex(db: &mut ContentDb, factor: f32) {
    let factor = if factor.is_finite() && factor >= 0.0 {
        factor
    } else {
        ANIMAL_DEADLY_DISTANCE_FACTOR
    };
    // Same animal ids as combat damage table (+ any deadly-only).
    const ANIMAL_IDS: &[i32] = &[
        418,  // Wolf
        420,  // Shot Wolf
        764,  // Rattle Snake
        1323, // Wild Boar
        1328, // Wild Boar with Piglet
        628,  // Grizzly
        631,  // Hungry Grizzly
        653,  // Hungry Grizzly attacking
        4762, // Sleepy Grizzly
        632,  // Shot Grizzly 1
        635,  // Shot Grizzly 2
        637,  // Shot Grizzly 3
        1435, // Bison
        1438, // Shot Bison
        1436, // Bison with Calf
        1440, // Shot Bison with Calf
        2156, // Mosquito Swarm
    ];
    for &id in ANIMAL_IDS {
        if let Some(d) = db.objects.get_mut(&id) {
            d.deadly_distance = factor;
        }
    }
}

/// Haxe `ServerSettings.PatchObjectData` combat `damage` / `woundFactor` / protection.
///
/// Subset used by DoDamage weapon+0 wound path + animal damage + bleed DPS tables.
/// Safe if id missing (binary cache / partial loads).
// Haxe: ServerSettings.PatchObjectData damage / woundFactor
pub fn apply_default_combat_damage_patches(db: &mut ContentDb) {
    // (id, damage, wound_factor override Option, damage_protection Option)
    // Weapons
    const WEAPON_DMG: &[(i32, f32, Option<f32>)] = &[
        (560, 5.0, Some(0.8)),  // Knife damage + protection
        (750, 5.0, Some(0.8)),  // Bloody Knife
        (3047, 6.0, Some(0.8)), // War Sword
        (3048, 6.0, Some(0.8)), // Bloody War Sword
        (152, 9.0, None),       // Bow and Arrow
        (1624, 12.0, None),     // Bow and Arrow with Note
    ];
    // Haxe PatchObjectData L1543: Riding Horse protection (do not zero damage).
    if let Some(d) = db.objects.get_mut(&770) {
        d.damage_protection_factor = 0.5;
    }
    for &(id, dmg, prot) in WEAPON_DMG {
        if let Some(d) = db.objects.get_mut(&id) {
            d.damage = dmg;
            if let Some(p) = prot {
                d.damage_protection_factor = p;
            }
        }
    }
    // Animals (deadlyDistance via apply_default_animal_deadly_distance_patches)
    const ANIMAL_DMG: &[(i32, f32, Option<f32>)] = &[
        (418, 3.0, None),       // Wolf
        (420, 5.0, None),       // Shot Wolf
        (764, 2.0, Some(0.98)), // Rattle Snake + woundFactor
        (1323, 3.0, None),      // Wild Boar
        (1328, 5.0, None),      // Wild Boar with Piglet
        (628, 5.0, None),       // Grizzly
        (631, 6.0, None),       // Hungry Grizzly
        (653, 6.0, None),       // Hungry Grizzly attacking
        (4762, 5.0, None),      // Sleepy Grizzly
        (632, 6.0, None),       // Shot Grizzly 1
        (635, 7.0, None),       // Shot Grizzly 2
        (637, 8.0, None),       // Shot Grizzly 3
        (1435, 2.0, None),      // Bison
        (1438, 5.0, None),      // Shot Bison
        (1436, 4.0, None),      // Bison with Calf
        (1440, 6.0, None),      // Shot Bison with Calf
        (2156, 1.0, None),      // Mosquito Swarm
    ];
    for &(id, dmg, wound_f) in ANIMAL_DMG {
        if let Some(d) = db.objects.get_mut(&id) {
            d.damage = dmg;
            if let Some(wf) = wound_f {
                d.wound_factor = wf;
            }
        }
    }
    // Wound bleed DPS (objectData.damage per sec) — residual EXHAUSTION-WOUND wire
    const WOUND_BLEED: &[(i32, f32)] = &[
        (3816, 0.1),  // Gushing Knife Wound
        (797, 0.05),  // Stable Knife Wound
        (1380, 0.03), // Clean Knife Wound
        (1625, 0.07), // Note Arrow Wound
        (798, 0.06),  // Arrow Wound
        (1365, 0.04), // Embedded Arrowhead Wound
        (1367, 0.06), // Extracted Arrowhead Wound
        (3817, 0.1),  // Gushing Empty Arrow Wound
        (1366, 0.03), // Empty Arrow Wound
        (1382, 0.03), // Clean Arrow Wound
        (1363, 0.05), // Bite Wound
        (1381, 0.03), // Clean Bite Wound
        (1377, 0.1),  // Snake Bite
        (1384, 0.05), // Clean Snake Bite
        (1364, 0.05), // Hog Cut
        (1383, 0.03), // Clean Hog Cut
    ];
    for &(id, dmg) in WOUND_BLEED {
        if let Some(d) = db.objects.get_mut(&id) {
            d.damage = dmg;
        }
    }
}

/// Haxe swamp biome id (`BiomeTag.SWAMP`).
pub const BIOME_TAG_SWAMP: i32 = 1;
/// Haxe Mosquito Swarm object id.
pub const MOSQUITO_SWARM_OBJECT_ID: i32 = 2156;
/// Haxe `mapChance *= 0.3` for Mosquito Swarm.
pub const MOSQUITO_MAP_CHANCE_FACTOR: f32 = 0.3;

/// Rebuild `biome_spawn` tables from current object `map_chance` / `biomes`.
///
/// Call after load-time mapChance / biome list patches so natural gen matches
/// ServerSettings.PatchObjectData.
// Haxe: ObjectData biomeTotalChance rebuild after mapChance patches
pub fn rebuild_biome_spawn_tables(db: &mut ContentDb) {
    db.biome_spawn.clear();
    for def in db.objects.values() {
        if def.map_chance > 0.0 && !def.biomes.is_empty() {
            for &b in &def.biomes {
                let table = db.biome_spawn.entry(b).or_default();
                table.total_chance += def.map_chance;
                table.entries.push((def.id, def.map_chance));
            }
        }
    }
}

/// Haxe `ServerSettings.PatchObjectData` mosquito mapChance / swamp biomes.
///
/// `2156.mapChance *= 0.3` and push `BiomeTag.SWAMP` so swarms spawn in swamp
/// as well as jungle. Rebuilds [`ContentDb::biome_spawn`] when 2156 is present.
// Haxe: ServerSettings.PatchObjectData ~1241–1242
// MOSQUITO-MAPCHANCE
pub fn apply_default_mosquito_map_chance_patches(db: &mut ContentDb) {
    let Some(d) = db.objects.get_mut(&MOSQUITO_SWARM_OBJECT_ID) else {
        return;
    };
    // SWAMP membership marks the patch as applied (boot may call finish twice).
    let already = d.biomes.contains(&BIOME_TAG_SWAMP);
    if !already {
        if d.map_chance > 0.0 {
            d.map_chance *= MOSQUITO_MAP_CHANCE_FACTOR;
        }
        d.biomes.push(BIOME_TAG_SWAMP);
    }
    rebuild_biome_spawn_tables(db);
}
/// Haxe `ServerSettings.PatchObjectData` useChance overrides (subset; safe if id missing).
pub fn apply_default_use_chance_patches(db: &mut ContentDb) {
    const PATCHES: &[(i32, f32)] = &[
        (4144, 0.8),
        (502, 0.05),
        (857, 0.02),
        (850, 0.1),
        (511, 0.5),
        (1261, 0.5),
        (141, 0.5),
        (142, 0.5),
        (143, 0.5),
        (662, 0.1),
        (944, 0.5),
        (3957, 1.0),
        (542, 0.1),
        (604, 0.1),
        (602, 0.2),
        (4213, 0.66),
        (600, 0.66),
        (1459, 0.2),
        (1462, 0.2),
        (1485, 0.2),
    ];
    for &(id, chance) in PATCHES {
        if let Some(d) = db.objects.get_mut(&id) {
            d.use_chance = chance;
        }
    }
}

/// Haxe Wolf / Leaf / Carrot crown ids (`extraPrestigeFactor` + `prestigeFactor`).
// Haxe: ServerSettings.PatchObjectData L1670–1681
pub const CROWN_BLANK_ID: i32 = 692;
pub const CARROT_CROWN_ID: i32 = 693;
pub const LEAF_CROWN_ID: i32 = 694;
pub const WOLF_CROWN_ID: i32 = 695;
/// Haxe crown `extraPrestigeFactor` (leader bonus).
pub const CROWN_EXTRA_PRESTIGE_FACTOR: f32 = 0.2;
/// Haxe colored-crown `prestigeFactor`.
pub const CROWN_PRESTIGE_FACTOR: f32 = 1.5;
/// Haxe Crown Blank `prestigeFactor`.
pub const CROWN_BLANK_PRESTIGE_FACTOR: f32 = 1.0;

/// Haxe `ServerSettings.SetClothingPrestige` — description modifiers on clothing.
// Haxe: ServerSettings.SetClothingPrestige L4279–4320
pub fn apply_set_clothing_prestige(obj: &mut ObjectDef) {
    let key = if obj.clothing.len() > 1 {
        obj.clothing.trim()
    } else {
        obj.clothing.as_str()
    };
    if key == "n" {
        return;
    }
    let desc = obj.description.as_str();
    if desc.starts_with("Red ")
        || desc.starts_with("Indigo ")
        || desc.starts_with("Green ")
        || desc.starts_with("Yellow ")
        || desc.starts_with("Black ")
    {
        obj.prestige_factor += 0.5;
    }
    if desc.contains(" Rose") {
        obj.prestige_factor += 0.5;
    }
    if desc.contains(" Feather") {
        obj.prestige_factor += 0.2;
    }
    if desc.contains("Rag ") {
        obj.prestige_factor /= 2.0;
    }
    if desc.contains("Old ") {
        obj.prestige_factor /= 2.0;
    }
    if desc.contains("Cloak") {
        obj.prestige_factor *= 2.0;
    }
    if desc.contains("Long Dress") {
        obj.prestige_factor *= 2.0;
    }
}

/// Haxe `PatchObjectData` clothing prestige: SetClothingPrestige then crown IDs.
// Haxe: ServerSettings.PatchObjectData L704 + L1670–1681
// PRESTIGE-CLOTH-FACTOR
pub fn apply_default_clothing_prestige_patches(db: &mut ContentDb) {
    for obj in db.objects.values_mut() {
        apply_set_clothing_prestige(obj);
    }
    for &id in &[CARROT_CROWN_ID, LEAF_CROWN_ID, WOLF_CROWN_ID] {
        if let Some(d) = db.objects.get_mut(&id) {
            d.extra_prestige_factor = CROWN_EXTRA_PRESTIGE_FACTOR;
            d.prestige_factor = CROWN_PRESTIGE_FACTOR;
        }
    }
    if let Some(d) = db.objects.get_mut(&CROWN_BLANK_ID) {
        d.prestige_factor = CROWN_BLANK_PRESTIGE_FACTOR;
    }
    // Haxe PatchObjectData L602 / L1683: not wearable.
    if let Some(d) = db.objects.get_mut(&707) {
        d.clothing = "n".into();
    }
    if let Some(d) = db.objects.get_mut(&700) {
        d.clothing = "n".into();
    }
}

/// Haxe dough/masa-on-table `switchNumberOfUses = true` patches.
pub fn apply_default_switch_number_of_uses_patches(db: &mut ContentDb) {
    const KEYS: &[(i32, i32)] = &[(252, 3371), (235, 4086), (1300, 3371), (235, 4090)];
    for &key in KEYS {
        if let Some(t) = db.transitions.get_mut(&key) {
            t.switch_number_of_uses = true;
        }
    }
}

/// Haxe `TransitionImporter.changeToolTransitions` — rewrite same-actor `newActorID`
/// via tool table `(newActor, -1)` last-use-actor first, then non-last-use.
///
/// Portable water / fill paths often keep `newActor == actor` (empty bowl) in files;
/// the real filled id lives on `newActor + -1` (e.g. Clay Bowl 235 → Bowl of Water 382).
///
/// **Skipped** (Haxe filters):
/// - `actorID != newActorID` — EMPTY+Cold Bowl `0+1021` (actor changes)
/// - `targetID < 1` — player / empty / TIME-style targets
/// - actor `numUses > 1` — multi-use tools (hoe piles)
/// - actor `2170` Rubber Ball (Haxe TODO special-case)
/// - `newActorID == 0` — clear-hand outcomes
///
/// Returns count of transitions whose `new_actor_id` changed.
pub fn apply_animal_decay_factor_patches(db: &mut ContentDb, factor: f32) {
    let factor = if factor.is_finite() && factor >= 0.0 {
        factor
    } else {
        ANIMAL_DECAY_FACTOR
    };
    patch_decay(db, 3159, Some(779), Some(factor), None, None);
    patch_decay(db, 779, Some(774), Some(factor), None, None);
    patch_decay(db, 774, Some(4154), Some(factor), None, None);
    for &(id, to) in &[
        (1458, 1900),
        (1488, 1900),
        (1454, 1900),
        (1489, 1900),
        (1459, 1487),
        (1462, 1487),
        (1485, 1487),
        (575, 595),
        (4213, 595),
        (600, 595),
        (576, 597),
        (542, 606),
        (604, 606),
        (418, 422),
        (420, 421),
    ] {
        patch_decay(db, id, Some(to), Some(factor), None, None);
    }
}

/// Haxe `ServerSettings.PatchObjectData` long-term decay product / factor / alias / rValue.
///
/// Only mutates objects that exist in `db.objects` (safe for partial unit-test DBs).
pub fn apply_default_decay_object_patches(db: &mut ContentDb) {
    // (id, decays_to, decay_factor_or_nan, counts_or_grows_as_or_0, r_value_or_nan)
    // decay_factor NaN = leave; r_value NaN = leave; counts 0 = leave.

    // Floors / roads
    patch_decay(db, 1596, Some(291), Some(0.1), None, None); // Stone Road → Flat Rock
    patch_decay(db, 884, Some(881), Some(0.1), None, None); // Stone Floor → Cut Stones
    patch_decay(db, 888, Some(884), Some(1.0), None, None); // Bear Skin Rug → Stone Floor
    patch_decay(db, 3290, None, Some(0.1), None, None); // Pine Floor
    patch_decay(db, 898, Some(1853), Some(0.02), None, None); // Ancient Stone Floor → Cut Stones

    // Stone walls → Cut Stones pile 1853
    for id in [885, 886, 887] {
        patch_decay(db, id, Some(1853), Some(0.2), None, None);
    }
    // Ancient stone walls
    for id in [895, 896, 897] {
        patch_decay(db, id, Some(1853), Some(0.02), None, None);
    }
    // Pine walls / doors → Pine Needles 96
    for id in [111, 112, 113, 115, 116, 117, 119, 3308, 3309, 3310] {
        patch_decay(db, id, Some(96), Some(2.0), None, None);
    }
    patch_decay(db, 119, None, None, None, Some(0.2)); // Open Pine Door H
    patch_decay(db, 117, None, None, None, Some(0.2)); // Open Pine Door V

    // Adobe walls → cracking variants
    patch_decay(db, 154, Some(889), None, None, None);
    patch_decay(db, 155, Some(891), None, None, None);
    patch_decay(db, 156, Some(890), None, None, None);

    // Plaster walls → adobe + slow decay + high rValue
    patch_decay(db, 1883, Some(154), Some(0.2), None, Some(0.98));
    patch_decay(db, 1884, Some(156), Some(0.2), None, Some(0.98));
    patch_decay(db, 1885, Some(155), Some(0.2), None, Some(0.98));

    // Wooden doors → boards; open doors low rValue
    patch_decay(db, 876, Some(470), None, None, None);
    patch_decay(db, 878, Some(470), None, None, Some(0.2));
    patch_decay(db, 877, Some(470), None, None, None);
    patch_decay(db, 879, Some(470), None, None, Some(0.2));

    // Wall shelves (containers-as-walls)
    patch_decay(db, 3240, Some(434), Some(0.2), None, Some(0.98));
    patch_decay(db, 3241, Some(1885), Some(0.2), None, Some(0.98));
    patch_decay(db, 3242, Some(3065), Some(0.2), None, Some(0.98));

    // Wooden chest decay chain (decayFactor /= permanent factor → net 5× base before permanent mult)
    let chest_boost = 1.0 / OBJ_DECAY_FACTOR_FOR_PERMANENT;
    for id in [986, 987, 4910, 2740, 434] {
        if let Some(d) = db.objects.get_mut(&id) {
            d.decay_factor = chest_boost;
        }
    }
    patch_decay(db, 986, Some(4910), None, None, None);
    patch_decay(db, 987, Some(4910), None, None, None);
    patch_decay(db, 4910, Some(2740), None, None, None);
    patch_decay(db, 2740, Some(434), None, None, None);
    patch_decay(db, 434, Some(470), None, None, None);
    patch_decay(db, 470, Some(847), None, None, None); // Boards → Broken Skewer
    patch_decay(db, 292, Some(860), None, None, None); // Basket → Broken Basket
    patch_decay(db, 204, Some(183), None, None, None); // Two Rabbit Furs → Fur
    patch_decay(db, 4063, Some(132), None, None, None); // Yew pile → branch
    patch_decay(db, 1121, Some(235), None, None, None); // Popcorn → Clay Bowl
    patch_decay(db, 625, Some(1101), None, None, None); // Wet Compost → Fertile Soil Pile
    patch_decay(db, 858, Some(862), None, None, None); // Broken Steel Tool → no wood
    patch_decay(db, 917, Some(862), None, None, None); // Key
    patch_decay(db, 1003, Some(862), None, None, None); // Lock Removal Key

    // Never-decay monuments / piles
    for id in [2709, 3112, 3961, 1598, 1837] {
        if let Some(d) = db.objects.get_mut(&id) {
            d.decay_factor = -1.0;
        }
    }

    // Well → Natural Spring
    patch_decay(db, 662, Some(3030), Some(0.1), None, None);
    // Forge → Adobe Kiln
    patch_decay(db, 303, Some(238), None, None, None);

    // Cart / horse decay chains
    patch_decay(db, 484, Some(483), None, None, None); // Hand Cart → Wheelbarrow
    patch_decay(db, 483, Some(471), None, None, None); // Wheelbarrow → Sledge
    patch_decay(db, 3157, Some(780), None, None, None);
    patch_decay(db, 780, Some(775), None, None, None);
    patch_decay(db, 775, Some(769), None, None, None);
    apply_animal_decay_factor_patches(db, ANIMAL_DECAY_FACTOR);

    // Iron vein aliases + strip/mine → Cut Stones
    patch_decay(db, 942, None, None, Some(3961), None); // Muddy Iron counts as vein
    for &(id, factor) in &[
        (3944, 0.1),
        (3957, 0.1),
        (3956, 0.1),
        (943, 0.1),
        (3958, 0.1),
        (944, 0.1),
        (3959, 0.1),
        (3960, 0.1),
        (945, 0.5),
        (3130, 0.1),
        (3129, 0.1),
        (3131, 0.1),
    ] {
        patch_decay(db, id, Some(881), Some(factor), Some(3961), None);
    }

    // Mango tree
    patch_decay(db, 1875, Some(1876), Some(0.1), None, None);
    patch_decay(db, 1876, None, Some(0.1), None, None);

    // Bear cave variants count as bear cave
    patch_decay(db, 650, None, None, Some(630), None);
    patch_decay(db, 647, None, None, Some(630), None);

    // Kiln / oven rubble (Haxe PatchObjectData L1057–1064)
    patch_decay(db, 238, Some(4201), None, None, None); // Adobe Kiln
    patch_decay(db, 281, Some(4201), None, None, None); // Wood-filled Adobe Kiln
    patch_decay(db, 237, Some(753), None, None, None); // Adobe Oven
    patch_decay(db, 247, Some(753), None, None, None); // Wood-filled Adobe Oven
    patch_decay(db, 4201, Some(753), Some(0.1), None, None); // Adobe Rubble (kiln)
    patch_decay(db, 753, None, Some(0.1), None, None); // Adobe Rubble

    // High tech (Haxe L1067–1084). 2365 is assigned three times; last write is 2243.
    patch_decay(db, 2385, Some(2383), None, None, None); // Diesel Drive Assembly
    patch_decay(db, 2240, Some(2243), None, None, None); // Newcomen Hammer
    patch_decay(db, 2243, Some(2245), None, None, None); // Multipurpose Newcomen Engine
    patch_decay(db, 2245, Some(2246), None, None, None); // Newcomen Engine without Rope
    patch_decay(db, 2280, Some(2243), None, None, None); // Newcomen Roller
    patch_decay(db, 2263, Some(2264), None, None, None); // Roller Mechanism
    patch_decay(db, 2270, Some(2243), None, None, None); // Newcomen Bore
    patch_decay(db, 2268, Some(2262), None, None, None); // Bore Mechanism
    patch_decay(db, 2359, Some(2243), None, None, None); // Newcomen Lathe
    patch_decay(db, 2356, Some(2262), None, None, None); // Lathe Mechanism
    patch_decay(db, 2365, Some(2243), None, None, None); // Diesel Engine (last write)

    // Fences / gates / hitch / springy doors (Haxe L1579–1646)
    patch_decay(db, 1851, Some(550), Some(0.2), None, None); // Fence Gate
    patch_decay(db, 2762, Some(878), None, None, None); // Springy Wooden Door not installed
    patch_decay(db, 2757, Some(876), None, None, None); // Springy Wooden Door H
    patch_decay(db, 2758, Some(876), None, None, None); // Springy Open Wooden Door H
    patch_decay(db, 2759, Some(879), None, None, None); // Springy Wooden Door V
    patch_decay(db, 2760, Some(879), None, None, None); // Springy Open Wooden Door V
    patch_decay(db, 4154, Some(556), Some(0.2), None, None); // Hitching Post
    patch_decay(db, 550, Some(556), Some(0.2), None, None); // Fence H
    patch_decay(db, 549, Some(556), Some(0.2), None, None); // Fence V
    patch_decay(db, 551, Some(556), Some(0.2), None, None); // Fence corner
    patch_decay(db, 3862, Some(434), None, None, None); // Dung Box

    // Seasonal stone / flint defaults when content has no decaysTo
    // 33 Stone, 34 Sharp Stone, 135 Flint Chip, 848 Hardened Row — leave content defaults;
    // snow path uses decays_to_obj when set.
}

/// Haxe `ServerSettings.PatchObjectData` containSize / containable force-patches.
///
/// Description rules run over every loaded object; id table overrides follow.
/// Safe if an id is missing from the db.
// Haxe: ServerSettings.PatchObjectData L633–758 containSize/containable
pub fn apply_default_contain_size_patches(db: &mut ContentDb) {
    // Description-based (smithing / glass / tools).
    // Haxe: "on Flat Rock" | "flat rock" | Mechanism | Blowpipe | Crucible | Shears
    let ids: Vec<i32> = db.objects.keys().copied().collect();
    for id in ids {
        let Some(obj) = db.objects.get_mut(&id) else {
            continue;
        };
        let desc = obj.description.as_str();
        // Allow for smithing — place on table sized containers.
        if desc.contains("on Flat Rock") || desc.contains("flat rock") {
            obj.contain_size = 2.0;
            obj.containable = true;
        }
        if desc.contains("Mechanism") {
            obj.contain_size = 2.0;
            obj.containable = true;
        }
        if desc.contains("Blowpipe") {
            obj.contain_size = 2.0;
            obj.containable = true;
        }
        // Crucible but not "in Wooden …"
        if desc.contains("Crucible") && !desc.contains("in Wooden") {
            obj.contain_size = 2.0;
            obj.containable = true;
        }
        if desc.contains("Shears") {
            obj.permanent = false;
            obj.contain_size = 1.0;
            obj.containable = true;
        }
    }

    // Explicit id force-patches (override description defaults).
    // Haxe: ObjectData.getObjectData(N).containSize / containable
    const ID_PATCHES: &[(i32, f32)] = &[
        (0, 1.0),    // Empty
        (356, 2.0),  // Basket of Bones
        (2188, 2.0), // Drum Sticks on Plate
        (2192, 1.0), // Turkey Leg Bone
        (2191, 1.0), // Turkey Drumstick
        (319, 2.0),  // Unforged Sealed Steel Crucible
        (321, 2.0),  // Hot Forged Steel Crucible
        (322, 2.0),  // Forged Steel Crucible
        (325, 2.0),  // Crucible with Steel
        (1528, 2.0), // Quenching Spring Steel
        (2574, 2.0), // Molten Glass
        (2578, 2.0), // Cool Glass
        (2573, 2.0), // Soda Lime Glass Batch
        (300, 2.0),  // Big Charcoal Pile
        (301, 2.0),  // Small Charcoal Pile
        (302, 1.0),  // Charcoal
    ];
    for &(id, size) in ID_PATCHES {
        patch_contain_size(db, id, size, true);
    }
}

/// Set contain_size + containable on one object if present.
// Haxe: ObjectData.getObjectData(id).containSize / containable
fn patch_contain_size(db: &mut ContentDb, id: i32, contain_size: f32, containable: bool) {
    let Some(d) = db.objects.get_mut(&id) else {
        return;
    };
    d.contain_size = contain_size;
    d.containable = containable;
}

fn patch_decay(
    db: &mut ContentDb,
    id: i32,
    decays_to: Option<i32>,
    decay_factor: Option<f32>,
    counts_or_grows_as: Option<i32>,
    r_value: Option<f32>,
) {
    let Some(d) = db.objects.get_mut(&id) else {
        return;
    };
    if let Some(to) = decays_to {
        d.decays_to_obj = to;
    }
    if let Some(f) = decay_factor {
        d.decay_factor = f;
    }
    if let Some(c) = counts_or_grows_as {
        d.counts_or_grows_as = c;
    }
    if let Some(r) = r_value {
        d.r_value = r;
    }
}

#[cfg(test)]
mod clothing_prestige_patch_tests {
    use super::*;
    use crate::ObjectDef;

    #[test]
    fn crown_ids_get_extra_and_prestige_factor() {
        let mut db = ContentDb::default();
        for id in [
            CROWN_BLANK_ID,
            CARROT_CROWN_ID,
            LEAF_CROWN_ID,
            WOLF_CROWN_ID,
        ] {
            let mut o = ObjectDef::empty(id);
            o.clothing = "h".into();
            o.description = format!("Crown {id}");
            db.objects.insert(id, o);
        }
        apply_default_clothing_prestige_patches(&mut db);
        let wolf = db.get(WOLF_CROWN_ID).unwrap();
        assert!((wolf.extra_prestige_factor - 0.2).abs() < 1e-5);
        assert!((wolf.prestige_factor - 1.5).abs() < 1e-5);
        assert!((wolf.get_prestige_factor() - 0.6).abs() < 1e-5);
        let blank = db.get(CROWN_BLANK_ID).unwrap();
        assert_eq!(blank.extra_prestige_factor, 0.0);
        assert!((blank.prestige_factor - 1.0).abs() < 1e-5);
        assert!((blank.get_prestige_factor() - 0.4).abs() < 1e-5);
    }

    #[test]
    fn set_clothing_prestige_red_hat_and_rag() {
        let mut red = ObjectDef::empty(10);
        red.clothing = "h".into();
        red.description = "Red Hat".into();
        apply_set_clothing_prestige(&mut red);
        assert!((red.prestige_factor - 1.0).abs() < 1e-5);
        assert!((red.get_prestige_factor() - 0.4).abs() < 1e-5);

        let mut rag = ObjectDef::empty(11);
        rag.clothing = "t".into();
        rag.description = "Rag Shirt".into();
        apply_set_clothing_prestige(&mut rag);
        assert!((rag.prestige_factor - 0.25).abs() < 1e-5);

        let mut skip = ObjectDef::empty(12);
        skip.description = "Red Stone".into();
        apply_set_clothing_prestige(&mut skip);
        assert!((skip.prestige_factor - 0.5).abs() < 1e-5);
    }

    #[test]
    fn fur_seal_and_leaf_crown_with_leaf_are_not_clothing() {
        let mut db = ContentDb::default();
        let mut seal = ObjectDef::empty(707);
        seal.clothing = "h".into();
        db.objects.insert(707, seal);
        let mut leaf = ObjectDef::empty(700);
        leaf.clothing = "h".into();
        db.objects.insert(700, leaf);
        apply_default_clothing_prestige_patches(&mut db);
        assert_eq!(db.get(707).unwrap().clothing, "n");
        assert_eq!(db.get(700).unwrap().clothing, "n");
    }
}
