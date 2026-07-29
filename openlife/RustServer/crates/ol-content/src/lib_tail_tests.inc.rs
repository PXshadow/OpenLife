/// Try default content locations relative to cwd / common sibling path.
pub fn resolve_content_path(configured: &Path) -> PathBuf {
    if configured.exists() {
        return configured.to_path_buf();
    }
    let candidates = [
        PathBuf::from("content/OneLifeData7"),
        PathBuf::from("../OpenLife/OneLifeData7"),
        PathBuf::from(r"C:\OhOl\OpenLife\OneLifeData7"),
    ];
    for c in candidates {
        if c.exists() {
            return c;
        }
    }
    configured.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_minimal_object() {
        let dir = std::env::temp_dir().join("ol_content_test_obj");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("33.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "33").unwrap();
        writeln!(f, "Gooseberry# wild").unwrap();
        writeln!(f, "foodValue=3").unwrap();
        writeln!(f, "containable=1").unwrap();
        let def = load_object_file(&path).unwrap();
        assert_eq!(def.id, 33);
        assert_eq!(def.name, "Gooseberry");
        assert_eq!(def.food_value, 3);
        assert!(def.containable);
        assert!(!def.floor);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_floor_flag() {
        let dir = std::env::temp_dir().join("ol_content_test_floor");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("1596.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "id=1596").unwrap();
        writeln!(f, "Stone Road# groundOnly").unwrap();
        writeln!(f, "floor=1").unwrap();
        writeln!(f, "permanent=0").unwrap();
        let def = load_object_file(&path).unwrap();
        assert_eq!(def.id, 1596);
        assert!(def.floor);
        assert!(def.is_floor());
        assert_eq!(def.name, "Stone Road");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    /// TWIN-PARTY-RESID: ObjectData.male from person object files.
    #[test]
    fn parse_male_flag() {
        let dir = std::env::temp_dir().join("ol_content_test_male");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("19.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "id=19").unwrap();
        writeln!(f, "Female001 D").unwrap();
        writeln!(f, "person=4,noSpawn=0").unwrap();
        writeln!(f, "male=0").unwrap();
        let parsed = load_object_file_full(&path).unwrap();
        assert!(!parsed.def.male, "female person male=0");
        assert_eq!(parsed.person, 4);
        let path_m = dir.join("30.txt");
        let mut f = fs::File::create(&path_m).unwrap();
        writeln!(f, "id=30").unwrap();
        writeln!(f, "Male001 D").unwrap();
        writeln!(f, "person=4,noSpawn=0").unwrap();
        writeln!(f, "male=1").unwrap();
        let parsed_m = load_object_file_full(&path_m).unwrap();
        assert!(parsed_m.def.male, "male person male=1");
        let _ = fs::remove_dir_all(&dir);
    }

    fn parse_person_race() {
        let dir = std::env::temp_dir().join("ol_content_test_person");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("19.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "id=19").unwrap();
        writeln!(f, "Female001 D").unwrap();
        writeln!(f, "person=4,noSpawn=0").unwrap();
        let parsed = load_object_file_full(&path).unwrap();
        assert_eq!(parsed.def.id, 19);
        assert_eq!(parsed.person, 4); // White
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn use_chance_patches_apply() {
        let mut db = ContentDb::default();
        db.objects.insert(502, ObjectDef::empty(502));
        db.objects.insert(850, ObjectDef {
            use_chance: 0.2,
            ..ObjectDef::empty(850)
        });
        apply_default_use_chance_patches(&mut db);
        assert!((db.get(502).unwrap().use_chance - 0.05).abs() < 1e-5);
        assert!((db.get(850).unwrap().use_chance - 0.1).abs() < 1e-5);
    }

    #[test]
    fn switch_number_of_uses_patches() {
        let mut db = ContentDb::default();
        db.transitions.insert(
            (252, 3371),
            Transition {
                actor_id: 252,
                target_id: 3371,
                new_actor_id: 252,
                new_target_id: 3371,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,
                desired_move_dist: 0,
                actor_min_use_fraction: 0.0,
                target_min_use_fraction: 0.0,
                switch_number_of_uses: false,
                target_number_of_uses: -1,
            is_pickup_or_drop: false,
            },
        );
        apply_default_switch_number_of_uses_patches(&mut db);
        assert!(db.transitions.get(&(252, 3371)).unwrap().switch_number_of_uses);
    }

    #[test]
    fn parse_openlife_id_prefix_and_map_chance() {
        let dir = std::env::temp_dir().join("ol_content_test_obj_id");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("100.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "id=100").unwrap();
        writeln!(f, "White Pine Tree with Needles").unwrap();
        writeln!(f, "containable=0").unwrap();
        writeln!(f, "permanent=1,minPickupAge=3").unwrap();
        writeln!(f, "blocksWalking=1,leftBlockingRadius=0").unwrap();
        writeln!(f, "mapChance=1.000000#biomes_0,3").unwrap();
        writeln!(f, "numUses=5,1.000000").unwrap();
        writeln!(f, "numSlots=0#timeStretch=1.000000").unwrap();
        let def = load_object_file(&path).unwrap();
        assert_eq!(def.id, 100);
        assert_eq!(def.name, "White Pine Tree with Needles");
        assert!(def.permanent);
        assert!(def.blocks_walking);
        assert!((def.map_chance - 1.0).abs() < 1e-5);
        assert_eq!(def.biomes, vec![0, 3]);
        assert_eq!(def.num_uses, 5);
        assert!((def.use_chance - 1.0).abs() < 1e-5);
        assert_eq!(def.num_slots, 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn name_from_description() {
        assert_eq!(description_to_name("Stone Hoe# tool"), "Stone Hoe");
    }

    #[test]
    fn parse_rvalue_and_clothing() {
        let dir = std::env::temp_dir().join("ol_content_test_rvalue");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("885.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "id=885").unwrap();
        writeln!(f, "Stone Wall# +cornerStone").unwrap();
        writeln!(f, "permanent=1").unwrap();
        writeln!(f, "rValue=0.900000").unwrap();
        writeln!(f, "floor=0").unwrap();
        writeln!(f, "clothing=n").unwrap();
        let def = load_object_file(&path).unwrap();
        assert!((def.r_value - 0.9).abs() < 1e-5);
        assert_eq!(def.clothing, "n");
        assert!(def.is_wall());
        assert!(!def.is_clothing());
        assert!((def.insulation_for_protection() - 0.9).abs() < 1e-5);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn decay_patches_apply_to_existing_objects() {
        let mut db = ContentDb::default();
        db.objects.insert(885, ObjectDef::empty(885));
        db.objects.insert(1596, ObjectDef {
            floor: true,
            ..ObjectDef::empty(1596)
        });
        db.objects.insert(1458, ObjectDef::empty(1458));
        apply_default_decay_object_patches(&mut db);
        let wall = db.get(885).unwrap();
        assert_eq!(wall.decays_to_obj, 1853);
        assert!((wall.decay_factor - 0.2).abs() < 1e-5);
        let road = db.get(1596).unwrap();
        assert_eq!(road.decays_to_obj, 291);
        let cow = db.get(1458).unwrap();
        assert_eq!(cow.decays_to_obj, 1900);
        assert!((cow.decay_factor - 0.05).abs() < 1e-5);
    }

    #[test]
    fn parse_transition_file() {
        let dir = std::env::temp_dir().join("ol_content_test_tr");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("0_33.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "34 32 0 0.000000 0.000000 0 0 0 0 0 0").unwrap();
        let t = load_transition_file(&path).unwrap();
        assert_eq!(t.actor_id, 0);
        assert_eq!(t.target_id, 33);
        assert_eq!(t.new_actor_id, 34);
        assert_eq!(t.new_target_id, 32);
        assert!(!t.last_use_target);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_last_use_transition_filename() {
        let dir = std::env::temp_dir().join("ol_content_test_lt");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("0_109_LT.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "0 0 0 0 0 0 0 0 0 0 0").unwrap();
        let t = load_transition_file(&path).unwrap();
        assert!(t.last_use_target);
        assert!(!t.last_use_actor);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn category_expands_shallow_digger_to_sharp_stone() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/OneLifeData7");
        if !root.is_dir() {
            return;
        }
        let db = load_content(&root).expect("load content");
        assert!(
            db.find_transition(34, 36).is_some(),
            "sharp stone on seeding wild carrot must resolve via category 722"
        );
        let t = db.find_transition(34, 36).unwrap();
        assert_eq!(t.new_target_id, 39, "dug wild carrot");
        let pile = db.get(661).expect("stone pile");
        assert!(pile.num_uses >= 2);
        assert_eq!(pile.dummy_ids.len(), (pile.num_uses - 1) as usize);
        assert_eq!(db.wire_id_for_uses(661, pile.num_uses), 661);
        assert_ne!(db.wire_id_for_uses(661, 1), 661);
        assert_eq!(
            db.resolve_base_id(db.wire_id_for_uses(661, 1)),
            661
        );
    }

    #[test]
    fn real_data_transition_goldens() {
        let roots = [
            PathBuf::from("content/OneLifeData7"),
            PathBuf::from(r"C:\OhOl\OpenLife\OneLifeData7"),
            PathBuf::from(r"C:\OhOl\OpenLifeReborn\content\OneLifeData7"),
        ];
        let root = roots.into_iter().find(|p| p.join("transitions").is_dir());
        let Some(root) = root else {
            eprintln!("skip real_data_transition_goldens — no OneLifeData7");
            return;
        };
        let cases = [
            ("0_63.txt", 0, 63, 64, 48),
            ("0_242.txt", 0, 242, 223, 242),
            ("0_36.txt", 0, 36, 395, 404),
        ];
        for (file, actor, target, new_a, new_t) in cases {
            let path = root.join("transitions").join(file);
            assert!(path.is_file(), "missing {path:?}");
            let tr = load_transition_file(&path).expect(file);
            assert_eq!(tr.actor_id, actor, "{file} actor");
            assert_eq!(tr.target_id, target, "{file} target");
            assert_eq!(tr.new_actor_id, new_a, "{file} new_actor");
            assert_eq!(tr.new_target_id, new_t, "{file} new_target");
            assert!(!tr.last_use_actor && !tr.last_use_target, "{file} not last-use");
        }
        let db = load_content(&root).expect("load_content");
        assert!(db.object_count() > 100);
        assert!(db.transition_count > 100);
        for &(_, a, t, na, nt) in &cases {
            let tr = db
                .find_transition(a, t)
                .unwrap_or_else(|| panic!("missing transition {a}+{t}"));
            assert_eq!(tr.new_actor_id, na);
            assert_eq!(tr.new_target_id, nt);
        }
        assert!(db.load_objects_ms > 0 || db.load_total_ms > 0);
        assert!(db.load_transitions_ms > 0 || db.transition_count == 0);
    }

    #[test]
    fn fixture_transition_matches_haxe_filename_parse() {
        let dir = std::env::temp_dir().join("ol_content_golden_0_63");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("0_63.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "64 48 0").unwrap();
        let t = load_transition_file(&path).unwrap();
        assert_eq!((t.actor_id, t.target_id, t.new_actor_id, t.new_target_id), (0, 63, 64, 48));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn prob_set_category_loads_weights() {
        let dir = std::env::temp_dir().join("ol_content_probset");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("3221.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "parentID=3221").unwrap();
        writeln!(f, "probSet").unwrap();
        writeln!(f, "numObjects=2").unwrap();
        writeln!(f, "1196 0.800000").unwrap();
        writeln!(f, "3220 0.200000").unwrap();
        let (cats, probs, _) = load_category_tables(&dir);
        assert!(cats.contains_key(&3221));
        let ps = probs.get(&3221).expect("prob set");
        assert_eq!(ps.ids, vec![1196, 3220]);
        assert!((ps.weights[0] - 0.8).abs() < 1e-5);
        assert!((ps.weights[1] - 0.2).abs() < 1e-5);
        let _ = fs::remove_dir_all(&dir);
    }
}
