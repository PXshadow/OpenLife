/// Haxe `ServerSettings.PatchTransitions` horse cart mount/dismount subset.
///
/// Marks cart pickups as `is_pickup_or_drop`, fixes tire-cart rubber preserve,
/// synthetic riding-horse put-down `770+0→0+1421`, hitch tire cart, escaped timers.
/// Safe for partial unit-test DBs (mutates only existing transitions / inserts synthetic).
// Haxe: ServerSettings.PatchTransitions (horse block ~2129–2236)
pub fn apply_default_horse_transition_patches(db: &mut ContentDb) {
    // Pickup/drop nest-swap flags (carts + grave baskets).
    const PICKUP_DROP_KEYS: &[(i32, i32)] = &[
        (0, 1422), // Escaped Horse-Drawn Cart just released
        (0, 780),  // Escaped Horse-Drawn Cart
        (0, 779),  // Hitched Horse-Drawn Cart
        (0, 3161), // Escaped Horse-Drawn Tire Cart just released
        (0, 3157), // Escaped Horse-Drawn Tire Cart
        (0, 3159), // Hitched Horse-Drawn Tire Cart
        (1618, -1), // Written Paper
        (292, 87),  // Basket + Fresh Grave
        (292, 88),  // Basket + Grave
        (292, 89),  // Basket + Old Grave
        (292, 357), // Basket + Bone Pile
        (356, -1),  // Basket of Bones put-down
    ];
    for &key in PICKUP_DROP_KEYS {
        if let Some(t) = db.transitions.get_mut(&key) {
            t.is_pickup_or_drop = true;
        }
    }

    // Synthetic: Riding Horse put-down on empty ground (770+0 = 0+1421).
    // Haxe also uses 770+-1 from content; this covers target id 0 lookups.
    let key_770_0 = (770, 0);
    if !db.transitions.contains_key(&key_770_0) {
        db.transitions.insert(
            key_770_0,
            Transition {
                actor_id: 770,
                target_id: 0,
                new_actor_id: 0,
                new_target_id: 1421,
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
                hungry_work_cost: 0.0,
                hungry_work_temperature: -1.0,
                coin_cost: 0,
                is_forbidden: false,
            },
        );
        db.transition_count = db.transition_count.saturating_add(1);
    }

    // Tire cart put-down: 3158+-1 → 0+3161 (preserve rubber, not 1422).
    if let Some(t) = db.transitions.get_mut(&(3158, -1)) {
        t.new_target_id = 3161;
    }
    // Tire cart pickups: empty + escaped tire → hold 3158 not 778.
    if let Some(t) = db.transitions.get_mut(&(0, 3161)) {
        t.new_actor_id = 3158;
        t.is_pickup_or_drop = true;
    }
    if let Some(t) = db.transitions.get_mut(&(0, 3157)) {
        t.new_actor_id = 3158;
        t.is_pickup_or_drop = true;
    }
    // Escaped tire just-released → escaped tire auto-decay.
    if let Some(t) = db.auto_decays.get_mut(&3161) {
        t.new_target_id = 3157;
        t.auto_decay_seconds = 20.0;
    }
    if let Some(t) = db.transitions.get_mut(&(-1, 3161)) {
        t.new_target_id = 3157;
        t.auto_decay_seconds = 20.0;
    }
    // Escaped tire cart move soften.
    if let Some(t) = db.auto_decays.get_mut(&3157) {
        t.move_dist = 2;
    }
    if let Some(t) = db.transitions.get_mut(&(-1, 3157)) {
        t.move_dist = 2;
    }
    // Hitch tire cart.
    if let Some(t) = db.transitions.get_mut(&(3158, 4154)) {
        t.new_target_id = 3159;
    }
    if let Some(t) = db.transitions.get_mut(&(3158, 550)) {
        t.new_target_id = 3159;
    }
    // Escaped cart / horse release timers + move.
    if let Some(t) = db.auto_decays.get_mut(&1422) {
        t.auto_decay_seconds = 15.0;
    }
    if let Some(t) = db.transitions.get_mut(&(-1, 1422)) {
        t.auto_decay_seconds = 15.0;
    }
    if let Some(t) = db.auto_decays.get_mut(&780) {
        t.move_dist = 2;
    }
    if let Some(t) = db.transitions.get_mut(&(-1, 780)) {
        t.move_dist = 2;
    }
    if let Some(t) = db.auto_decays.get_mut(&1421) {
        t.auto_decay_seconds = 20.0;
    }
    if let Some(t) = db.transitions.get_mut(&(-1, 1421)) {
        t.auto_decay_seconds = 20.0;
    }
    if let Some(t) = db.auto_decays.get_mut(&775) {
        t.move_dist = 3;
    }
    if let Some(t) = db.transitions.get_mut(&(-1, 775)) {
        t.move_dist = 3;
    }
}
