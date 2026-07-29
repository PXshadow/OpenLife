// Included at end of horse_mount::tests.
// Live USE wire tests for HORSE-MOUNT-POLISH hitch_cart.

    fn def_perm(id: i32, slots: i32, permanent: bool) -> ObjectDef {
        let mut d = def(id, 0, slots);
        d.permanent = permanent;
        d
    }

    fn state_with(db: ContentDb) -> crate::SimState {
        crate::SimState::with_default_empty(std::sync::Arc::new(db))
    }

    // Haxe: 778 + 4154 = 0 + 779 with cargo (isHorseDropTrans nest swap)
    #[test]
    fn live_hitch_cart_with_cargo_preserves_nest() {
        let mut db = ContentDb::default();
        db.objects.insert(778, def(778, 0, 4));
        db.objects.insert(779, def(779, 0, 4));
        db.objects.insert(4154, def_perm(4154, 0, true));
        db.objects.insert(33, def(33, 0, 0));
        db.transitions.insert((778, 4154), tr(778, 4154, 0, 779));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held_helper(NestedHelper::from_wire(778, &[33]));
            p.x = 0;
            p.y = 0;
        }
        state.world.write().unwrap().set_object(0, 0, 4154);
        let r = crate::apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        assert_eq!(r.actor_after, 0);
        assert_eq!(r.target_after, 779);
        assert_eq!(state.players.get(&1).unwrap().held_id, 0);
        let world = state.world.read().unwrap();
        let h = world.get_helper(0, 0).expect("hitched");
        assert_eq!(h.base_id, 779);
        assert_eq!(h.contained, vec![33]);
    }

    // Haxe: 0 + 779 = 778 + 4154 with cargo (is_pickup_or_drop)
    #[test]
    fn live_unhitch_cart_with_cargo_preserves_nest() {
        let mut db = ContentDb::default();
        db.objects.insert(778, def(778, 0, 4));
        db.objects.insert(779, def(779, 0, 4));
        db.objects.insert(4154, def_perm(4154, 0, true));
        db.objects.insert(33, def(33, 0, 0));
        db.objects.insert(40, def(40, 0, 0));
        let mut t = tr(0, 779, 778, 4154);
        t.is_pickup_or_drop = true;
        db.transitions.insert((0, 779), t);
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.clear_held();
            p.x = 0;
            p.y = 0;
        }
        let mut hitched = ComplexObject::new_simple(779);
        hitched.contained = vec![33, 40];
        hitched.slots = vec![NestedHelper::id_only(33), NestedHelper::id_only(40)];
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(0, 0, hitched);
        let r = crate::apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        assert_eq!(r.actor_after, 778);
        assert_eq!(r.target_after, 4154);
        let p = state.players.get(&1).unwrap();
        assert_eq!(p.held_id, 778);
        let hh = p.held_helper.as_ref().expect("held cart");
        assert_eq!(hh.contained.len(), 2);
        assert_eq!(hh.contained[0].id, 33);
        assert_eq!(hh.contained[1].id, 40);
    }

    #[test]
    fn live_hitch_empty_cart_to_post() {
        let mut db = ContentDb::default();
        db.objects.insert(778, def(778, 0, 4));
        db.objects.insert(779, def(779, 0, 4));
        db.objects.insert(4154, def_perm(4154, 0, true));
        db.transitions.insert((778, 4154), tr(778, 4154, 0, 779));
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(778, 0);
            p.x = 0;
            p.y = 0;
        }
        state.world.write().unwrap().set_object(0, 0, 4154);
        let r = crate::apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        assert_eq!(r.actor_after, 0);
        assert_eq!(r.target_after, 779);
    }

    #[test]
    fn live_hitch_tire_cart_with_cargo() {
        let mut db = ContentDb::default();
        db.objects.insert(3158, def(3158, 0, 4));
        db.objects.insert(3159, def(3159, 0, 4));
        db.objects.insert(4154, def_perm(4154, 0, true));
        db.objects.insert(33, def(33, 0, 0));
        db.transitions.insert((3158, 4154), tr(3158, 4154, 0, 779));
        ol_content::apply_default_horse_transition_patches(&mut db);
        assert_eq!(
            db.transitions.get(&(3158, 4154)).unwrap().new_target_id,
            3159
        );
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held_helper(NestedHelper::from_wire(3158, &[33]));
            p.x = 0;
            p.y = 0;
        }
        state.world.write().unwrap().set_object(0, 0, 4154);
        let r = crate::apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        assert_eq!(r.actor_after, 0);
        assert_eq!(r.target_after, 3159);
        let world = state.world.read().unwrap();
        let h = world.get_helper(0, 0).expect("tire hitched");
        assert_eq!(h.base_id, 3159);
        assert_eq!(h.contained, vec![33]);
    }

    // Haxe: 292 + 87 isPickupOrDrop nest swap (empty basket scoop)
    #[test]
    fn live_grave_basket_pickup_or_drop_nest_swap() {
        let mut db = ContentDb::default();
        db.objects.insert(292, def(292, 0, 4));
        db.objects.insert(87, def(87, 0, 4));
        db.objects.insert(356, def(356, 0, 4));
        db.objects.insert(88, def_perm(88, 0, true));
        db.objects.insert(33, def(33, 0, 0));
        let mut t = tr(292, 87, 356, 88);
        t.is_pickup_or_drop = true;
        db.transitions.insert((292, 87), t);
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(292, 0);
            p.x = 0;
            p.y = 0;
        }
        let mut grave = ComplexObject::new_simple(87);
        grave.contained = vec![33];
        grave.slots = vec![NestedHelper::id_only(33)];
        state
            .world
            .write()
            .unwrap()
            .set_object_complex(0, 0, grave);
        let r = crate::apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        assert_eq!(r.actor_after, 356);
        assert_eq!(r.target_after, 88);
        let hh = state
            .players
            .get(&1)
            .unwrap()
            .held_helper
            .as_ref()
            .expect("basket of bones nest");
        assert_eq!(hh.id, 356);
        assert_eq!(hh.contained.len(), 1);
        assert_eq!(hh.contained[0].id, 33);
    }

    // Haxe L1322–1326: Basket with cargo refuses changeHeld (needs basket_refuse wire)
    #[test]
    fn live_basket_with_cargo_refuses_change_held() {
        let mut db = ContentDb::default();
        db.objects.insert(292, def(292, 0, 4));
        db.objects.insert(87, def(87, 0, 4));
        db.objects.insert(356, def(356, 0, 4));
        db.objects.insert(88, def_perm(88, 0, true));
        db.objects.insert(33, def(33, 0, 0));
        let mut t = tr(292, 87, 356, 88);
        t.is_pickup_or_drop = true;
        db.transitions.insert((292, 87), t);
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "u");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held_helper(NestedHelper::from_wire(292, &[33]));
            p.x = 0;
            p.y = 0;
        }
        state.world.write().unwrap().set_object(0, 0, 87);
        let r = crate::apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(!r.applied, "basket with cargo must refuse changeHeld");
        assert_eq!(state.players.get(&1).unwrap().held_id, 292);
        assert_eq!(state.world.read().unwrap().get_object(0, 0), 87);
    }
