/// Remainder of Haxe `ServerSettings.PatchObjectData` not covered by earlier tables.
///
/// 1:1 from `openlife/settings/ServerSettings.hx` L601–1716 (uncommented).
/// Skips tables already applied by contain-size / decay / use-chance / combat /
/// clothing-prestige / second-time / mosquito / parseTags.
pub fn apply_haxe_patch_object_data_remainder(db: &mut ContentDb) {
    apply_haxe_object_description_loops(db);
    // Description Well/Mine/Ancient sets decayFactor -1; restore id-table values.
    apply_default_decay_object_patches(db);
    apply_haxe_object_id_remainder(db);
    rebuild_biome_spawn_tables(db);
}

fn apply_haxe_object_description_loops(db: &mut ContentDb) {
    let ids: Vec<i32> = db.objects.keys().copied().collect();
    for id in ids {
        let Some(desc) = db.objects.get(&id).map(|o| o.description.clone()) else {
            continue;
        };

        // Haxe L619–621 Wall / Door → allowFloorPlacement
        if desc.contains("Wall") || desc.contains("Door") {
            db.allow_floor_placement.insert(id);
        }
        // Haxe L624–626
        if desc.contains("groundOnly") {
            db.ground_only.insert(id);
        }
        // Haxe L629–630 +hungryWork → HungryWorkCost (5)
        if desc.contains("+hungryWork") {
            db.object_hungry_work.insert(id, HUNGRY_WORK_COST);
        }
        // Haxe L663–665 Steel → Broken Steel Tool no wood 862
        if desc.contains("Steel") {
            if let Some(obj) = db.objects.get_mut(&id) {
                obj.decays_to_obj = 862;
            }
        }
        // Haxe L675–684 Well / Pump (not Pumpkin) / Vein / Mine / Iron Pit /
        // Drilling / Rig / Cave / Ancient → never-decay (oil pump/rig/drilling).
        if haxe_never_decay_description(&desc) {
            if let Some(obj) = db.objects.get_mut(&id) {
                obj.decay_factor = -1.0;
            }
        }
        // Haxe L612–616 objectIdArrays[455] — parentId of every "Chisel" description
        if desc.contains("Chisel") {
            let parent = db.dummy_parent.get(&id).copied().unwrap_or(id);
            let arr = db.object_id_arrays.entry(STEEL_CHISEL_ARRAY_KEY).or_default();
            if !arr.contains(&parent) {
                arr.push(parent);
            }
        }
        // Haxe L689–691 +owned flags: Rust reads description tags (no ObjectDef field).
        // Haxe L1659–1663 Sports Car
        if desc.contains("Sports Car") {
            db.is_boat.insert(id);
            if let Some(obj) = db.objects.get_mut(&id) {
                obj.speed_mult = 3.5;
            }
        }
    }
}

fn apply_haxe_object_id_remainder(db: &mut ContentDb) {
    // Haxe L1088–1096 allowFloorPlacement ids (plus Wall/Door loop above)
    for id in [237, 247, 238, 281, 303, 305, 3371, 434, 3065] {
        db.allow_floor_placement.insert(id);
    }

    // Haxe L1099–1100 blocksRemove
    db.blocks_remove.insert(987);
    db.blocks_remove.insert(988);

    // Haxe L1116–1143 object hungryWork (commented HungryWorkToolCostFactor skipped)
    const HUNGRY: &[(i32, f32)] = &[
        (857, -2.0),
        (1845, 5.0),
        (1846, 5.0),
        (1847, 5.0),
        (1849, 5.0),
        (123, 2.0),
        (231, 10.0),
        (1020, 2.0),
        (138, 2.0),
        (3961, 5.0),
        (496, 4.0),
        (1011, 3.0),
        (213, 3.0),
        (1136, 3.0),
        (511, 2.0),
        (1261, 2.0),
        (141, 2.0),
        (142, 2.0),
        (143, 2.0),
        (662, 1.0),
        (663, 2.0),
    ];
    for &(id, cost) in HUNGRY {
        db.object_hungry_work.insert(id, cost);
    }

    // Haxe L1146–1156 +biomeBlock* farming-in-water forbid
    for id in [1136, 1101, 1138] {
        if let Some(d) = db.objects.get_mut(&id) {
            for tag in [" +biomeBlock9", " +biomeBlock13", " +biomeBlock17"] {
                if !d.description.contains(tag.trim_start()) {
                    d.description.push_str(tag);
                }
            }
        }
    }

    // countsOrGrowsAs not already in decay id-table (Haxe L1188–1450, L1504–1510)
    const GROWS: &[(i32, i32)] = &[
        (141, 1261),
        (142, 1261),
        (510, 1261),
        (509, 1261),
        (511, 1261),
        (512, 1261),
        (409, 125),
        (404, 1435), // overwritten below (Haxe also uses 404 as Wild Carrot)
        (1328, 1323),
        (762, 761),
        (763, 761),
        (2145, 2142),
        (279, 30),
        (164, 161),
        (165, 161), // Rabbit Hole peeking2
        (166, 161), // Rabbit Hole peeking1
        (173, 161),
        (3566, 161),
        (532, 531),
        (808, 805),
        (404, 36), // last write (Wild Carrot without Seed)
        (40, 36),
        (39, 36),
        (4252, 4251),
        (806, 804),
        (807, 804),
        (51, 50),
        (52, 50),
        (138, 136),
        (1438, 1435),
        (1436, 1435),
        (1440, 1435),
    ];
    for &(id, as_id) in GROWS {
        if let Some(d) = db.objects.get_mut(&id) {
            d.counts_or_grows_as = as_id;
        }
    }

    // mapChance (Haxe L1222–1310). Mosquito 2156 already applied.
    mul_map_chance(db, 3030, 3.0);
    mul_map_chance(db, 769, 2.0);
    copy_iron_vein_spawn_from_muddy(db);
    mul_map_chance(db, 2135, 0.25);
    mul_map_chance(db, 530, 0.5);
    mul_map_chance(db, 121, 3.0);
    mul_map_chance(db, 418, 1.2);
    mul_map_chance(db, 764, 5.0);
    mul_map_chance(db, 4251, 5.0);
    mul_map_chance(db, 50, 1.2);
    if let Some(src) = db.objects.get(&211).map(|o| o.map_chance) {
        if let Some(d) = db.objects.get_mut(&838) {
            d.map_chance = src / 5.0;
        }
    }

    // biomes (Haxe L1170, L1245–1290, L1310)
    push_biome(db, 4251, BIOME_TAG_GREY);
    for id in [
        575, 4213, 576, 614, 602, 542, 604, 1489, 1492, 1488, 1458, 1485, 1459, 770, 780, 3157,
        1256, 1278, 1255,
    ] {
        push_biome(db, id, BIOME_TAG_GREEN);
    }
    set_biomes(db, 1328, &[BIOME_TAG_SWAMP]);
    set_biomes(db, 3566, &[BIOME_TAG_YELLOW]);
    // 161 Rabbit Hole#hiding must natural-spawn on yellow prairie (mapChance=1 biomes_2).
    if let Some(d) = db.objects.get_mut(&161) {
        if d.map_chance <= 0.0 {
            d.map_chance = 1.0;
        }
        if !d.biomes.contains(&BIOME_TAG_YELLOW) {
            d.biomes.push(BIOME_TAG_YELLOW);
        }
    }
    for id in [631, 4762, 632, 635] {
        set_biomes(db, id, &[BIOME_TAG_GREY]);
    }
    push_biome(db, 418, BIOME_TAG_YELLOW);
    push_biome(db, 418, BIOME_TAG_SNOW);
    push_biome(db, 838, BIOME_TAG_GREEN);

    // speedMult (Haxe L1291–1318, L1463–1465, L1574–1577)
    if let Some(d) = db.objects.get_mut(&418) {
        d.speed_mult *= 1.5;
    }
    for id in [411, 345, 290, 314, 326] {
        if let Some(d) = db.objects.get_mut(&id) {
            d.speed_mult = SEMI_HEAVY_ITEM_SPEED;
        }
    }
    set_speed(db, 778, 1.40);
    set_speed(db, 3158, 1.50);
    set_speed(db, 484, 0.85);
    set_speed(db, 861, 0.85);
    set_speed(db, 2172, 0.9);
    set_speed(db, 750, 0.75);
    set_speed(db, 3048, 0.85);
    set_speed(db, 749, 0.6);
    set_speed(db, 2396, 2.5);
    set_speed(db, 4655, 1.0);

    // foodValue (Haxe L1356–1373)
    const FOOD: &[(i32, i32)] = &[
        (768, 4),
        (2143, 5),
        (31, 2),
        (253, 2),
        (2855, 3),
        (808, 2),
        (807, 4),
        (40, 4),
        (402, 4),
        (4252, 2),
        (2836, 4),
        (197, 25),
        (2190, 20),
        (1285, 15),
        (1292, 20),
    ];
    for &(id, fv) in FOOD {
        if let Some(d) = db.objects.get_mut(&id) {
            d.food_value = fv;
        }
    }

    // numUses / groundOnly / permanent / numSlots (Haxe L1382–1668)
    if let Some(d) = db.objects.get_mut(&624) {
        d.num_uses = 7;
    }
    db.ground_only.insert(623);
    if let Some(d) = db.objects.get_mut(&764) {
        d.permanent = false;
    }
    if let Some(d) = db.objects.get_mut(&766) {
        d.r_value = 1.0;
    }
    for id in [1618, 2100, 2098] {
        if let Some(d) = db.objects.get_mut(&id) {
            d.permanent = false;
        }
    }
    if let Some(d) = db.objects.get_mut(&1605) {
        d.num_slots = 0;
    }

    // winter / spring (Haxe L1393–1461)
    set_season(db, 805, 1.0, 1.0);
    set_season(db, 808, 2.0, 0.5);
    set_season(db, 36, 1.0, 1.0);
    set_season(db, 404, 1.0, 0.5);
    set_season(db, 40, 2.0, 0.5);
    set_season(db, 39, 2.0, 0.5);
    set_season(db, 4251, 1.0, 1.0);
    set_season(db, 4252, 2.0, 0.5);
    set_season(db, 804, 1.0, 1.0);
    set_season(db, 806, 2.0, 0.5);
    set_season(db, 807, 2.0, 0.5);
    set_season(db, 50, 0.0, 0.1);
    set_season(db, 51, 0.0, 0.1);
    set_season(db, 52, 0.0, 0.1);
    set_season(db, 57, 2.0, 0.0);
    if let Some(d) = db.objects.get_mut(&136) {
        d.spring_regrow_factor = 0.05;
    }
    set_season(db, 138, 2.0, 0.5);
    if let Some(d) = db.objects.get_mut(&30) {
        d.winter_decay_factor = 1.0;
        d.spring_regrow_factor = 1.0;
    }
    if let Some(d) = db.objects.get_mut(&279) {
        d.spring_regrow_factor = 6.0;
    }
    if let Some(d) = db.objects.get_mut(&31) {
        d.winter_decay_factor = 2.0;
    }
    if let Some(d) = db.objects.get_mut(&1135) {
        d.spring_regrow_factor = 0.2;
    }

    // isBloody / neverDrop (Haxe L1467–1473)
    for id in [750, 3048, 749] {
        db.is_bloody.insert(id);
        db.never_drop.insert(id);
    }

    // alternativeTimeOutcome (Haxe L1566–1572; PatchTransitions overwrites some)
    db.alternative_time_outcome.insert(797, 0);
    db.alternative_time_outcome.insert(1363, 0);
    db.alternative_time_outcome.insert(798, 1367);
    db.alternative_time_outcome.insert(1367, 1366);
    db.alternative_time_outcome.insert(1366, 0);

    // isBoat (Haxe L1574–1577)
    db.is_boat.insert(2396);
    db.is_boat.insert(4655);

    // blocksAnimal / groundOnly / rValue fences (Haxe L1579–1641)
    for id in [1851, 2757, 2758, 2759, 2760] {
        db.blocks_animal.insert(id);
    }
    for id in [1851, 4154, 550, 549, 551] {
        db.ground_only.insert(id);
    }
    set_r(db, 1851, 0.01);
    set_r(db, 558, 0.01);
    set_r(db, 2757, 0.95);
    set_r(db, 2758, 0.2);
    set_r(db, 2760, 0.2);
    for id in [772, 773, 774, 779, 3159, 4154, 550, 549, 551] {
        set_r(db, id, 0.01);
    }

    apply_haxe_object_extra_fields(db);
}

/// Haxe `ObjectData.reducesLongingFor` / `higherQaulityFood` / `prestigeClass` /
/// `blocksDomesticAnimal` (PatchObjectData L1320–1353, L1481, L1582/1605/1643).
fn apply_haxe_object_extra_fields(db: &mut ContentDb) {
    // Haxe L1320–1353 (last write wins on higherQaulityFood)
    const LONGING: &[(i32, i32)] = &[
        (253, 31),
        (272, 253),
        (1121, 4895),
        (402, 40),
        (273, 402),
        (2855, 808),
        (2860, 2855),
        (2861, 2836),
        (803, 570),
        (4081, 1463),
        (3593, 4081),
        (4082, 1481),
        (3596, 4082),
    ];
    for &(id, other) in LONGING {
        db.reduces_longing_for.insert(id, other);
    }
    const HQ: &[(i32, i32)] = &[
        (31, 253),
        (253, 272),
        (4895, 1121),
        (40, 402),
        (402, 273),
        (808, 2855),
        (2855, 2860),
        (2836, 2861),
        (570, 803),
        (1463, 4081),
        (4081, 3593),
        (1481, 4082),
        (4082, 3596),
    ];
    for &(id, other) in HQ {
        db.higher_quality_food.insert(id, other);
    }

    // Haxe L1481 War Sword → PrestigeClass.Noble (3)
    db.object_prestige_class
        .insert(3047, OBJECT_PRESTIGE_CLASS_NOBLE);

    // Haxe L1582 Fence Gate, L1605 Springy Wooden Door vertical, L1643 Fence Kit
    // (2762 commented out in Haxe)
    for id in [1851, 2759, 556] {
        db.blocks_domestic_animal.insert(id);
    }

    // Haxe L1666 Truck Chassis
    db.unreleased.insert(4647);
}

/// Haxe `ServerSettings.objectIdArrays` key for Steel Chisel 455.
pub const STEEL_CHISEL_ARRAY_KEY: i32 = 455;
/// Haxe `PrestigeClass.Noble` int tag.
pub const OBJECT_PRESTIGE_CLASS_NOBLE: u8 = 3;

fn mul_map_chance(db: &mut ContentDb, id: i32, factor: f32) {
    if let Some(d) = db.objects.get_mut(&id) {
        d.map_chance *= factor;
    }
}

fn copy_iron_vein_spawn_from_muddy(db: &mut ContentDb) {
    let Some(muddy) = db.objects.get(&942) else {
        return;
    };
    let mc = muddy.map_chance * 10.0;
    let biomes = muddy.biomes.clone();
    if let Some(vein) = db.objects.get_mut(&3961) {
        vein.map_chance = mc;
        vein.biomes = biomes;
    }
    if let Some(muddy) = db.objects.get_mut(&942) {
        muddy.map_chance = 0.0;
        muddy.biomes.clear();
    }
}

fn push_biome(db: &mut ContentDb, id: i32, biome: i32) {
    if let Some(d) = db.objects.get_mut(&id) {
        biome_push(d, biome);
    }
}

fn set_biomes(db: &mut ContentDb, id: i32, biomes: &[i32]) {
    if let Some(d) = db.objects.get_mut(&id) {
        d.biomes = biomes.to_vec();
    }
}

fn set_speed(db: &mut ContentDb, id: i32, speed: f32) {
    if let Some(d) = db.objects.get_mut(&id) {
        d.speed_mult = speed;
    }
}

fn set_season(db: &mut ContentDb, id: i32, winter: f32, spring: f32) {
    if let Some(d) = db.objects.get_mut(&id) {
        d.winter_decay_factor = winter;
        d.spring_regrow_factor = spring;
    }
}

fn set_r(db: &mut ContentDb, id: i32, r: f32) {
    if let Some(d) = db.objects.get_mut(&id) {
        d.r_value = r;
    }
}

/// Haxe `PatchObjectData` L675–684. Product TODO: wells / oil never decay.
/// Oil pumpjacks match `Pump` (not Pumpkin); oil rigs match `Drilling` / `Rig`.
fn haxe_never_decay_description(desc: &str) -> bool {
    desc.contains("Well")
        || (desc.contains("Pump") && !desc.contains("Pumpkin"))
        || desc.contains("Vein")
        || desc.contains("Mine")
        || desc.contains("Iron Pit")
        || desc.contains("Drilling")
        || desc.contains("Rig")
        || desc.contains("Cave")
        || desc.contains("Ancient")
}

#[cfg(test)]
mod object_data_remainder_tests {
    use super::*;

    fn stub(id: i32, desc: &str) -> ObjectDef {
        let mut o = ObjectDef::empty(id);
        o.description = desc.into();
        o
    }

    #[test]
    fn allow_floor_placement_ids_and_wall_door_description() {
        let mut db = ContentDb::default();
        db.objects.insert(237, stub(237, "Adobe Oven"));
        db.objects.insert(9001, stub(9001, "Stone Wall"));
        db.objects.insert(9002, stub(9002, "Pine Door"));
        db.objects.insert(9003, stub(9003, "Basket"));
        apply_haxe_patch_object_data_remainder(&mut db);
        assert!(db.allow_floor_placement.contains(&237));
        assert!(db.allow_floor_placement.contains(&9001));
        assert!(db.allow_floor_placement.contains(&9002));
        assert!(!db.allow_floor_placement.contains(&9003));
    }

    #[test]
    fn kiln_238_decays_to_4201() {
        let mut db = ContentDb::default();
        db.objects.insert(238, ObjectDef::empty(238));
        db.objects.insert(281, ObjectDef::empty(281));
        db.objects.insert(237, ObjectDef::empty(237));
        apply_default_decay_object_patches(&mut db);
        apply_haxe_patch_object_data_remainder(&mut db);
        assert_eq!(db.get(238).unwrap().decays_to_obj, 4201);
        assert_eq!(db.get(281).unwrap().decays_to_obj, 4201);
        assert_eq!(db.get(237).unwrap().decays_to_obj, 753);
    }

    #[test]
    fn rabbit_hole_161_ensured_on_yellow() {
        let mut db = ContentDb::default();
        let mut hole = ObjectDef::empty(161);
        hole.map_chance = 0.0;
        db.objects.insert(161, hole);
        apply_haxe_patch_object_data_remainder(&mut db);
        let d = db.get(161).unwrap();
        assert!(d.map_chance >= 1.0);
        assert!(d.biomes.contains(&BIOME_TAG_YELLOW));
        let table = db.biome_spawn.get(&BIOME_TAG_YELLOW).expect("yellow spawn table");
        assert!(
            table.entries.iter().any(|(id, _)| *id == 161),
            "161 must be in yellow biome_spawn"
        );
    }

    #[test]
    fn clothing_707_700_still_n_after_remainder() {
        let mut db = ContentDb::default();
        let mut seal = ObjectDef::empty(707);
        seal.clothing = "h".into();
        db.objects.insert(707, seal);
        let mut leaf = ObjectDef::empty(700);
        leaf.clothing = "h".into();
        db.objects.insert(700, leaf);
        apply_all_haxe_content_patches(&mut db);
        assert_eq!(db.get(707).unwrap().clothing, "n");
        assert_eq!(db.get(700).unwrap().clothing, "n");
    }

    #[test]
    fn longing_hq_prestige_domestic_and_chisel_array() {
        let mut db = ContentDb::default();
        db.objects.insert(253, stub(253, "Bowl of Gooseberries"));
        db.objects.insert(31, stub(31, "Gooseberry"));
        db.objects.insert(3047, stub(3047, "War Sword"));
        db.objects.insert(1851, stub(1851, "Fence Gate"));
        db.objects.insert(556, stub(556, "Fence Kit"));
        db.objects.insert(455, stub(455, "Steel Chisel"));
        db.objects.insert(466, stub(466, "Steel File with Chisel"));
        db.dummy_parent.insert(466, 466);
        apply_haxe_patch_object_data_remainder(&mut db);
        assert_eq!(db.reduces_longing_for_of(253), 31);
        assert_eq!(db.higher_quality_food_of(31), 253);
        assert_eq!(db.higher_quality_food_of(253), 272);
        assert_eq!(db.prestige_class_of(3047), OBJECT_PRESTIGE_CLASS_NOBLE);
        assert!(db.blocks_domestic_animal_of(1851));
        assert!(db.blocks_domestic_animal_of(556));
        assert!(!db.blocks_domestic_animal_of(2762));
        let chisels = db.object_id_array(STEEL_CHISEL_ARRAY_KEY);
        assert!(chisels.contains(&455), "{chisels:?}");
        assert!(chisels.contains(&466), "{chisels:?}");
        assert!(db.is_unreleased(4647));
    }

    #[test]
    fn wells_oil_description_never_decay() {
        let mut db = ContentDb::default();
        db.objects.insert(663, stub(663, "Deep Well"));
        db.objects.insert(2308, stub(2308, "Oil Pumpjack"));
        db.objects.insert(2304, stub(2304, "Dry Oil Drilling Rig"));
        db.objects.insert(2306, stub(2306, "Gushing Oil Rig"));
        db.objects.insert(9001, stub(9001, "Pumpkin"));
        db.objects.insert(9002, stub(9002, "Ripe Pumpkin"));
        db.objects.insert(9003, stub(9003, "Basket"));
        db.objects.insert(9004, stub(9004, "Iron Vein"));
        db.objects.insert(9005, stub(9005, "Bear Cave"));
        db.objects.insert(9006, stub(9006, "Ancient Stone"));
        db.objects.insert(9007, stub(9007, "Shallow Iron Pit"));
        db.objects.insert(9008, stub(9008, "Iron Mine"));
        db.objects.insert(2141, stub(2141, "Oil Palm"));
        apply_haxe_object_description_loops(&mut db);
        for id in [663, 2308, 2304, 2306, 9004, 9005, 9006, 9007, 9008] {
            assert_eq!(
                db.get(id).unwrap().decay_factor,
                -1.0,
                "never-decay id={id}"
            );
        }
        assert_eq!(db.get(9001).unwrap().decay_factor, 1.0);
        assert_eq!(db.get(9002).unwrap().decay_factor, 1.0);
        assert_eq!(db.get(9003).unwrap().decay_factor, 1.0);
        assert_eq!(db.get(2141).unwrap().decay_factor, 1.0);
        assert!(haxe_never_decay_description("Oil Pumpjack"));
        assert!(!haxe_never_decay_description("Pumpkin"));
        assert!(!haxe_never_decay_description("Oil Palm"));
    }

    #[test]
    fn shallow_well_662_id_table_overrides_never_decay() {
        let mut db = ContentDb::default();
        db.objects.insert(662, stub(662, "Shallow Well"));
        db.objects.insert(663, stub(663, "Deep Well"));
        apply_haxe_patch_object_data_remainder(&mut db);
        assert!((db.get(662).unwrap().decay_factor - 0.1).abs() < 1e-5);
        assert_eq!(db.get(662).unwrap().decays_to_obj, 3030);
        assert_eq!(db.get(663).unwrap().decay_factor, -1.0);
    }
}
