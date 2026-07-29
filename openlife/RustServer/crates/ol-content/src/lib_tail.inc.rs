/// Haxe `createAndaddCategoryTransitions` — expand actor/target category parents
/// into concrete member transitions (e.g. `@ Shallow Digger` 722 → sharp stone 34).
pub(crate) fn expand_category_transitions(db: &mut ContentDb) {
    if db.categories.is_empty() {
        return;
    }
    let base: Vec<Transition> = db
        .transitions
        .values()
        .cloned()
        .chain(db.transitions_last_use.values().cloned())
        .collect();
    let mut added = 0usize;
    for t in base {
        let actor_cat = db.categories.get(&t.actor_id).cloned();
        let target_cat = db.categories.get(&t.target_id).cloned();
        match (actor_cat, target_cat) {
            (Some(actors), None) => {
                for aid in actors {
                    let mut nt = t.clone();
                    if nt.new_actor_id == t.actor_id {
                        nt.new_actor_id = aid;
                    }
                    nt.actor_id = aid;
                    if insert_expanded(db, nt) {
                        added += 1;
                    }
                }
            }
            (None, Some(targets)) => {
                for tid in targets {
                    let mut nt = t.clone();
                    if nt.new_target_id == t.target_id {
                        nt.new_target_id = tid;
                    }
                    nt.target_id = tid;
                    if insert_expanded(db, nt) {
                        added += 1;
                    }
                }
            }
            (Some(actors), Some(targets)) => {
                for aid in &actors {
                    for tid in &targets {
                        let mut nt = t.clone();
                        if nt.new_actor_id == t.actor_id {
                            nt.new_actor_id = *aid;
                        }
                        if nt.new_target_id == t.target_id {
                            nt.new_target_id = *tid;
                        }
                        nt.actor_id = *aid;
                        nt.target_id = *tid;
                        if insert_expanded(db, nt) {
                            added += 1;
                        }
                    }
                }
            }
            (None, None) => {}
        }
    }
    db.transition_count = db.transitions.len();
    db.last_use_transition_count = db.transitions_last_use.len();
    info!(added, "content category transitions expanded");
}

fn target_remains(t: &Transition) -> bool {
    t.target_id >= 0 && t.target_id == t.new_target_id
}

/// Haxe `ServerSettings` secondTimeOutcome patches (goose pond, rabbits, …).
///
/// Only inserts when the object id exists in `db.objects` (or always for known
/// ids so unit tests / partial loads can opt-in by inserting outcomes manually).
pub fn apply_default_second_time_outcomes(db: &mut ContentDb) {
    // (object_id, outcome_id, seconds)
    // Haxe ServerSettings.PatchObjectData subset used by DoSecondTimeOutcome.
    const PATCHES: &[(i32, i32, f32)] = &[
        (141, 142, 30.0),           // Canada Goose Pond → swimming
        (142, 1261, 60.0 * 10.0),   // swimming → with Egg
        (1261, 142, 60.0 * 4.0),    // with Egg → swimming
        (511, 142, 60.0 * 60.0 * 24.0), // Pond → goose swimming
        (164, 173, 90.0),           // Rabbit Hole out,single → Family Hole out
        (173, 3566, 90.0),          // Family Hole out → Fleeing Rabbit
        (1438, 1435, 30.0 * 60.0),  // Shot Bison → Bison
        (1440, 1436, 30.0 * 60.0),  // Shot Bison with Calf → Bison
    ];
    for &(id, out, secs) in PATCHES {
        db.second_time_outcomes.entry(id).or_insert((out, secs));
    }
}

/// Haxe `ServerSettings.AnimalDecayFactor`.
const ANIMAL_DECAY_FACTOR: f32 = 0.05;
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
            d.deadly_distance = ANIMAL_DEADLY_DISTANCE_FACTOR;
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
        (560, 5.0, Some(0.8)),   // Knife damage + protection
        (750, 5.0, Some(0.8)),   // Bloody Knife
        (3047, 6.0, Some(0.8)),  // War Sword
        (3048, 6.0, Some(0.8)),  // Bloody War Sword
        (152, 9.0, None),        // Bow and Arrow
        (1624, 12.0, None),      // Bow and Arrow with Note
    ];
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
        (418, 3.0, None),   // Wolf
        (420, 5.0, None),   // Shot Wolf
        (764, 2.0, Some(0.98)), // Rattle Snake + woundFactor
        (1323, 3.0, None),  // Wild Boar
        (1328, 5.0, None),  // Wild Boar with Piglet
        (628, 5.0, None),   // Grizzly
        (631, 6.0, None),   // Hungry Grizzly
        (653, 6.0, None),   // Hungry Grizzly attacking
        (4762, 5.0, None),  // Sleepy Grizzly
        (632, 6.0, None),   // Shot Grizzly 1
        (635, 7.0, None),   // Shot Grizzly 2
        (637, 8.0, None),   // Shot Grizzly 3
        (1435, 2.0, None),  // Bison
        (1438, 5.0, None),  // Shot Bison
        (1436, 4.0, None),  // Bison with Calf
        (1440, 6.0, None),  // Shot Bison with Calf
        (2156, 1.0, None),  // Mosquito Swarm
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

/// Set `ObjectData.moves` from auto-decay / time-move transitions (`move_dist > 0`).
///
/// Haxe sets `animal.objectData.moves` during `doAnimalMovement`; stamping from
/// content lets `isAnimal()` work for USE bow min-range without a prior move tick.
// Haxe: TimeHelper.doAnimalMovement animal.objectData.moves = moveDist
pub fn apply_animal_moves_from_transitions(db: &mut ContentDb) {
    for tr in db.auto_decays.values() {
        if tr.move_dist <= 0 {
            continue;
        }
        if let Some(d) = db.objects.get_mut(&tr.target_id) {
            if d.moves < tr.move_dist {
                d.moves = tr.move_dist;
            }
        }
    }
    for tr in db.transitions.values() {
        if tr.move_dist <= 0 || tr.target_id <= 0 {
            continue;
        }
        if let Some(d) = db.objects.get_mut(&tr.target_id) {
            if d.moves < tr.move_dist {
                d.moves = tr.move_dist;
            }
        }
    }
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
// Haxe: TransitionImporter.changeToolTransitions
pub fn change_tool_transitions(db: &mut ContentDb) -> usize {
    let mut rewritten = 0usize;

    // Snapshot keys; tool rows themselves have target_id < 1 so they are not rewritten.
    let normal_keys: Vec<(i32, i32)> = db.transitions.keys().copied().collect();
    for key in normal_keys {
        if rewrite_one_tool_transition(db, key, false) {
            rewritten += 1;
        }
    }
    let last_keys: Vec<(i32, i32)> = db.transitions_last_use.keys().copied().collect();
    for key in last_keys {
        if rewrite_one_tool_transition(db, key, true) {
            rewritten += 1;
        }
    }
    if rewritten > 0 {
        info!(rewritten, "content changeToolTransitions rewrote new_actor_id");
    }
    rewritten
}

/// Apply Haxe changeToolTransitions filters + rewrite for one map entry.
fn rewrite_one_tool_transition(db: &mut ContentDb, key: (i32, i32), in_last_use: bool) -> bool {
    let (actor_id, target_id, new_actor_id) = {
        let t = if in_last_use {
            db.transitions_last_use.get(&key)
        } else {
            db.transitions.get(&key)
        };
        let Some(t) = t else {
            return false;
        };
        // Haxe: actorID != newActorID → skip (EMPTY+Cold Bowl)
        if t.actor_id != t.new_actor_id {
            return false;
        }
        // Haxe: targetID < 1 → skip
        if t.target_id < 1 {
            return false;
        }
        // Haxe: actor numUses > 1 → skip multi-use tools
        let num_uses = db.objects.get(&t.actor_id).map(|d| d.num_uses).unwrap_or(0);
        if num_uses > 1 {
            return false;
        }
        // Haxe TODO: Rubber Ball 2170 special skip
        if t.actor_id == 2170 {
            return false;
        }
        // Haxe: newActorID == 0 → skip
        if t.new_actor_id == 0 {
            return false;
        }
        (t.actor_id, t.target_id, t.new_actor_id)
    };
    let _ = (actor_id, target_id); // filters already applied; keep names for Haxe anchors

    // Haxe: GetTransition(newActorID, -1, lastUseActor=true) then non-LA
    let tool_new = db
        .find_transition_last_use(new_actor_id, -1)
        .or_else(|| db.find_transition(new_actor_id, -1))
        .map(|tr| tr.new_actor_id);

    let Some(tool_new_actor) = tool_new else {
        return false;
    };
    if tool_new_actor == new_actor_id {
        return false;
    }

    if in_last_use {
        if let Some(t) = db.transitions_last_use.get_mut(&key) {
            t.new_actor_id = tool_new_actor;
            return true;
        }
    } else if let Some(t) = db.transitions.get_mut(&key) {
        t.new_actor_id = tool_new_actor;
        return true;
    }
    false
}

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
    patch_decay(db, 3159, Some(779), Some(ANIMAL_DECAY_FACTOR), None, None);
    patch_decay(db, 779, Some(774), Some(ANIMAL_DECAY_FACTOR), None, None);
    patch_decay(db, 774, Some(4154), Some(ANIMAL_DECAY_FACTOR), None, None);

    // Domestic animals → dead variants
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
        patch_decay(db, id, Some(to), Some(ANIMAL_DECAY_FACTOR), None, None);
    }

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

/// Insert non-last-use transition; Haxe double-transition maxUse handling.
fn insert_normal_or_max_use(db: &mut ContentDb, t: Transition) -> bool {
    let key = (t.actor_id, t.target_id);
    let remains = target_remains(&t);
    if let Some(existing) = db.transitions.get(&key).cloned() {
        let exist_remains = target_remains(&existing);
        // Haxe: targetRemains true + false pair → non-remains goes to maxUse table
        if exist_remains && !remains {
            db.transitions_max_use.insert(key, t);
            return true;
        }
        if !exist_remains && remains {
            db.transitions_max_use.insert(key, existing);
            db.transitions.insert(key, t);
            return true;
        }
        // Same kind: keep first (category expansion / duplicates)
        return false;
    }
    db.transitions.insert(key, t);
    true
}

fn insert_expanded(db: &mut ContentDb, t: Transition) -> bool {
    let key = (t.actor_id, t.target_id);
    if t.last_use_actor || t.last_use_target {
        if db.transitions_last_use.contains_key(&key) {
            return false;
        }
        db.transitions_last_use.insert(key, t);
        true
    } else {
        insert_normal_or_max_use(db, t)
    }
}

fn load_transitions_into(db: &mut ContentDb, dir: &Path) -> Result<(), ContentError> {
    if !dir.is_dir() {
        warn!(path = %dir.display(), "transitions directory missing");
        return Ok(());
    }

    let mut paths = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("txt") {
            paths.push(path);
        }
    }

    let results: Vec<Result<Transition, ContentError>> = paths
        .par_iter()
        .map(|path| load_transition_file(path))
        .collect();

    let mut loaded = 0usize;
    let mut loaded_last_use = 0usize;
    let mut errors = 0u32;

    for res in results {
        match res {
            Ok(t) => {
                // Auto-decay / animal move: actor -1 (TIME).
                // Haxe: negative autoDecaySeconds = hours; must still index for map timers.
                if t.actor_id < 0 && t.auto_decay_seconds != 0.0 {
                    db.auto_decays.insert(t.target_id, t.clone());
                }
                // Also index pure animal-move transitions (autoDecaySeconds may be 0).
                if t.actor_id < 0 && t.move_dist > 0 {
                    db.auto_decays
                        .entry(t.target_id)
                        .or_insert_with(|| t.clone());
                }
                if t.last_use_actor || t.last_use_target {
                    db.transitions_last_use
                        .insert((t.actor_id, t.target_id), t);
                    loaded_last_use += 1;
                } else if insert_normal_or_max_use(db, t) {
                    loaded += 1;
                }
            }
            Err(e) => {
                errors += 1;
                debug!(error = %e, "skip transition");
            }
        }
    }

    db.transition_count = loaded;
    db.last_use_transition_count = loaded_last_use;
    info!(
        loaded,
        loaded_last_use,
        errors,
        path = %dir.display(),
        "content transitions loaded"
    );
    Ok(())
}

/// Parse `actor_target.txt` or `actor_target_LA.txt` / `_LT` / `_L`.
pub fn load_transition_file(path: &Path) -> Result<Transition, ContentError> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| ContentError::BadObject {
            path: path.display().to_string(),
            msg: "bad filename".into(),
        })?;
    let parts: Vec<&str> = stem.split('_').collect();
    if parts.len() < 2 {
        return Err(ContentError::BadObject {
            path: path.display().to_string(),
            msg: "filename needs actor_target".into(),
        });
    }
    let actor_id: i32 = parts[0].parse().map_err(|_| ContentError::BadObject {
        path: path.display().to_string(),
        msg: "bad actor id".into(),
    })?;
    let target_id: i32 = parts[1].parse().map_err(|_| ContentError::BadObject {
        path: path.display().to_string(),
        msg: "bad target id".into(),
    })?;
    let flag = parts.get(2).copied().unwrap_or("");
    let last_use_actor = flag == "LA";
    let last_use_target = flag == "LT" || flag == "L";

    let text = fs::read_to_string(path)?;
    let line = text.lines().next().unwrap_or("").trim();
    let data: Vec<&str> = line.split_whitespace().collect();
    if data.len() < 2 {
        return Err(ContentError::BadObject {
            path: path.display().to_string(),
            msg: "need at least newActor newTarget".into(),
        });
    }

    let parse_i = |i: usize, default: i32| -> i32 {
        data.get(i).and_then(|s| s.parse().ok()).unwrap_or(default)
    };
    let parse_f = |i: usize| -> f32 {
        data.get(i).and_then(|s| s.parse().ok()).unwrap_or(0.0)
    };
    let parse_b = |i: usize| -> bool { data.get(i).map(|s| *s == "1").unwrap_or(false) };

    Ok(Transition {
        actor_id,
        target_id,
        new_actor_id: parse_i(0, 0),
        new_target_id: parse_i(1, 0),
        last_use_actor,
        last_use_target,
        auto_decay_seconds: parse_f(2),
        reverse_use_actor: parse_b(5),
        reverse_use_target: parse_b(6),
        no_use_actor: parse_b(9),
        no_use_target: parse_b(10),
        move_dist: parse_i(7, 0),
        desired_move_dist: parse_i(8, 0),
        actor_min_use_fraction: parse_f(3),
        target_min_use_fraction: parse_f(4),
        switch_number_of_uses: false,
        target_number_of_uses: -1,
        is_pickup_or_drop: false,
    })
}

/// Parse object file + person race (0 when absent).
/// Also fills [`ObjectDef::male`] via key parse in [`load_object_file`].
// TWIN-PARTY-RESID: ObjectData.male + person race
pub fn load_object_file_full(path: &Path) -> Result<ParsedObject, ContentError> {
    let text = fs::read_to_string(path)?;
    let def = load_object_file(path)?;
    let person = parse_person_from_text(&text);
    Ok(ParsedObject { def, person })
}

/// Parse a single object description file (OHOL / Open Life line-oriented format).
///
/// First line may be bare `33` or `id=100`. Second line is description when present.
pub fn load_object_file(path: &Path) -> Result<ObjectDef, ContentError> {
    let text = fs::read_to_string(path)?;
    let mut lines = text.lines().peekable();

    let id_line = lines.next().ok_or_else(|| ContentError::BadObject {
        path: path.display().to_string(),
        msg: "empty file".into(),
    })?;
    let id_raw = id_line.trim();
    let id: i32 = id_raw
        .strip_prefix("id=")
        .unwrap_or(id_raw)
        .trim()
        .parse()
        .map_err(|_| ContentError::BadObject {
            path: path.display().to_string(),
            msg: format!("bad id line: {id_line}"),
        })?;

    let mut def = ObjectDef::empty(id);

    // Description is the next non-key=value line (bare name), if any.
    if let Some(peek) = lines.peek() {
        let t = peek.trim();
        if !t.is_empty() && !t.contains('=') {
            let desc = lines.next().unwrap().to_string();
            def.description = desc.clone();
            def.name = description_to_name(&desc);
        }
    }

    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Keys may be embedded in comma-joined groups: permanent=1,minPickupAge=3
        for part in line.split(',') {
            let part = part.trim();
            if let Some(rest) = part.strip_prefix("containable=") {
                def.containable = rest.starts_with('1') || rest.eq_ignore_ascii_case("true");
            } else if let Some(rest) = part.strip_prefix("permanent=") {
                def.permanent = rest.starts_with('1') || rest.eq_ignore_ascii_case("true");
            } else if let Some(rest) = part.strip_prefix("blocksWalking=") {
                def.blocks_walking = rest.starts_with('1') || rest.eq_ignore_ascii_case("true");
            } else if let Some(rest) = part.strip_prefix("foodValue=") {
                def.food_value = rest.parse().unwrap_or(0);
            } else if let Some(rest) = part.strip_prefix("heatValue=") {
                def.heat_value = rest.parse().unwrap_or(0.0);
            } else if let Some(rest) = part.strip_prefix("numUses=") {
                // Haxe: numUses = int(array[0]); useChance = array[1] if present.
                // Outer loop splits on ','; re-read full line when `numUses=N,chance`.
                let full = if line.contains("numUses=") {
                    line.split("numUses=").nth(1).unwrap_or(rest)
                } else {
                    rest
                };
                let full = full.split('#').next().unwrap_or(full);
                let mut it = full.split(',');
                if let Some(num) = it.next() {
                    def.num_uses = num.trim().parse().unwrap_or(0);
                }
                if let Some(chance) = it.next() {
                    def.use_chance = chance.trim().parse().unwrap_or(0.0);
                }
            } else if let Some(rest) = part.strip_prefix("numSlots=") {
                // numSlots=4#timeStretch=1.000000
                let num = rest.split(|c| c == ',' || c == '#').next().unwrap_or(rest);
                def.num_slots = num.parse().unwrap_or(0);
            } else if let Some(rest) = part.strip_prefix("containSize=") {
                // Haxe: containSize=N,vertSlotRot=… — first comma/hash-separated float.
                // // Haxe: ObjectData.containSize
                let num = rest.split(|c| c == ',' || c == '#').next().unwrap_or(rest);
                def.contain_size = num.parse().unwrap_or(0.0);
            } else if let Some(rest) = part.strip_prefix("slotsSize=") {
                // Haxe text key `slotsSize` → ObjectData.slotSize (default 1).
                // // Haxe: ObjectData.slotSize
                let num = rest.split(|c| c == ',' || c == '#').next().unwrap_or(rest);
                def.slot_size = num.parse().unwrap_or(1.0);
            } else if let Some(rest) = part.strip_prefix("slotSize=") {
                // Alternate key (some exporters); same as slotsSize.
                let num = rest.split(|c| c == ',' || c == '#').next().unwrap_or(rest);
                def.slot_size = num.parse().unwrap_or(1.0);
            } else if let Some(rest) = part.strip_prefix("male=") {
                // Haxe ObjectData.male — person sex (0/1 or true/false).
                // TWIN-PARTY-RESID / ObjectData.male
                def.male = rest.starts_with('1') || rest.eq_ignore_ascii_case("true");
            } else if let Some(rest) = part.strip_prefix("floor=") {
                // floor=1 — floor-only objects (roads, stone floors); not ground placeables.
                def.floor = rest.starts_with('1') || rest.eq_ignore_ascii_case("true");
            } else if let Some(rest) = part.strip_prefix("speedMult=") {
                def.speed_mult = rest
                    .split(|c| c == ',' || c == '#')
                    .next()
                    .unwrap_or(rest)
                    .parse()
                    .unwrap_or(1.0);
            } else if let Some(rest) = part.strip_prefix("rValue=") {
                // Haxe ObjectData.rValue — insulation / isWall gate.
                def.r_value = rest
                    .split(|c| c == ',' || c == '#')
                    .next()
                    .unwrap_or(rest)
                    .parse()
                    .unwrap_or(0.0);
            } else if let Some(rest) = part.strip_prefix("clothing=") {
                // clothing=n or clothing=h etc. (may share line with clothingOffset — split earlier).
                def.clothing = rest
                    .split(|c| c == ',' || c == '#')
                    .next()
                    .unwrap_or(rest)
                    .trim()
                    .to_string();
                if def.clothing.is_empty() {
                    def.clothing = "n".into();
                }
            } else if let Some(rest) = part.strip_prefix("useDistance=") {
                // Haxe ObjectData.useDistance (default 1).
                def.use_distance = rest
                    .split(|c| c == ',' || c == '#')
                    .next()
                    .unwrap_or(rest)
                    .parse()
                    .unwrap_or(1);
            } else if let Some(rest) = part.strip_prefix("deadlyDistance=") {
                // Haxe ObjectData.deadlyDistance (float; files often store int).
                def.deadly_distance = rest
                    .split(|c| c == ',' || c == '#')
                    .next()
                    .unwrap_or(rest)
                    .parse()
                    .unwrap_or(0.0);
            } else if let Some(rest) = part.strip_prefix("moves=") {
                // Haxe ObjectData.moves (rare in files; usually set from time-move).
                def.moves = rest
                    .split(|c| c == ',' || c == '#')
                    .next()
                    .unwrap_or(rest)
                    .parse()
                    .unwrap_or(0);
            } else if let Some(rest) = part.strip_prefix("mapChance=") {
                // mapChance=1.000000#biomes_0,3,4,5  (biomes may span later commas —
                // re-parse full line segment after mapChance= when '#' present)
                // Prefer full line when this part looks truncated.
                let full = if line.contains("mapChance=") {
                    line.split("mapChance=")
                        .nth(1)
                        .unwrap_or(rest)
                } else {
                    rest
                };
                let (chance_s, rest2) = if let Some(i) = full.find('#') {
                    (&full[..i], Some(&full[i + 1..]))
                } else {
                    (full.split(',').next().unwrap_or(full), None)
                };
                def.map_chance = chance_s.trim().parse().unwrap_or(0.0);
                if let Some(r) = rest2 {
                    let biomes_part = r
                        .strip_prefix("biomes_")
                        .or_else(|| r.strip_prefix("biomes="))
                        .unwrap_or(r);
                    // Stop at next known key if present on same line.
                    let biomes_part = biomes_part
                        .split("heatValue=")
                        .next()
                        .unwrap_or(biomes_part)
                        .trim_end_matches(',')
                        .trim();
                    def.biomes = biomes_part
                        .split(|c| c == ',' || c == ' ')
                        .filter_map(|s| {
                            let s = s.trim();
                            if s.is_empty() {
                                None
                            } else {
                                s.parse().ok()
                            }
                        })
                        .collect();
                }
            }
        }
    }

    Ok(def)
}

fn description_to_name(desc: &str) -> String {
    // OHOL: "Wild Gooseberry# just picked"
    let base = desc.split('#').next().unwrap_or(desc).trim();
    base.to_string()
}

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
        assert_eq!(def.use_distance, 1);
        assert_eq!(def.deadly_distance, 0.0);
        assert!((def.contain_size - 0.0).abs() < 1e-5);
        assert!((def.slot_size - 1.0).abs() < 1e-5);
        let _ = fs::remove_dir_all(&dir);
    }

    /// CLOTHING-CONTAIN-SIZE: containSize + slotsSize from object text.
    // Haxe: ObjectData.containSize / slotSize (text key slotsSize)
    #[test]
    fn parse_contain_size_and_slot_size() {
        let dir = std::env::temp_dir().join("ol_content_test_contain_size");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("333.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "id=333").unwrap();
        writeln!(f, "Basket# container").unwrap();
        writeln!(f, "containable=1").unwrap();
        writeln!(f, "containSize=2.000000,vertSlotRot=-0.250000").unwrap();
        writeln!(f, "numSlots=4#timeStretch=1.000000").unwrap();
        writeln!(f, "slotsSize=1").unwrap();
        let def = load_object_file(&path).unwrap();
        assert!((def.contain_size - 2.0).abs() < 1e-5);
        assert!((def.slot_size - 1.0).abs() < 1e-5);
        assert_eq!(def.num_slots, 4);
        // Fit gate: basket containSize 2 needs container slotSize >= 2
        let mut pocket = ObjectDef::empty(198);
        pocket.slot_size = 1.0;
        assert!(!def.contain_fits_in_container(&pocket));
        pocket.slot_size = 2.0;
        assert!(def.contain_fits_in_container(&pocket));
        let _ = fs::remove_dir_all(&dir);
    }

    /// CLOTHING-CONTAIN-SIZE: ServerSettings.PatchObjectData containSize force table.
    // Haxe: ServerSettings.PatchObjectData L633–758
    #[test]
    fn apply_default_contain_size_patches_ids_and_description() {
        let mut db = ContentDb::default();
        // Id force-patches (create stubs).
        for id in [0, 356, 302, 319, 2574] {
            db.objects.insert(id, ObjectDef::empty(id));
        }
        // Description rules
        let mut flat = ObjectDef::empty(9001);
        flat.description = "Something on Flat Rock".into();
        db.objects.insert(9001, flat);
        let mut mech = ObjectDef::empty(9002);
        mech.description = "Clockwork Mechanism".into();
        db.objects.insert(9002, mech);
        let mut cruc_wood = ObjectDef::empty(9003);
        cruc_wood.description = "Crucible in Wooden Tongs".into();
        db.objects.insert(9003, cruc_wood);
        let mut cruc = ObjectDef::empty(9004);
        cruc.description = "Steel Crucible".into();
        db.objects.insert(9004, cruc);
        let mut shears = ObjectDef::empty(9005);
        shears.description = "Steel Shears".into();
        shears.permanent = true;
        db.objects.insert(9005, shears);

        apply_default_contain_size_patches(&mut db);

        assert!((db.objects.get(&0).unwrap().contain_size - 1.0).abs() < 1e-5);
        assert!(db.objects.get(&0).unwrap().containable);
        assert!((db.objects.get(&356).unwrap().contain_size - 2.0).abs() < 1e-5);
        assert!(db.objects.get(&356).unwrap().containable);
        assert!((db.objects.get(&302).unwrap().contain_size - 1.0).abs() < 1e-5);
        assert!((db.objects.get(&319).unwrap().contain_size - 2.0).abs() < 1e-5);
        assert!((db.objects.get(&2574).unwrap().contain_size - 2.0).abs() < 1e-5);

        assert!((db.objects.get(&9001).unwrap().contain_size - 2.0).abs() < 1e-5);
        assert!(db.objects.get(&9001).unwrap().containable);
        assert!((db.objects.get(&9002).unwrap().contain_size - 2.0).abs() < 1e-5);
        // "Crucible in Wooden" must NOT force containSize 2
        assert!((db.objects.get(&9003).unwrap().contain_size - 0.0).abs() < 1e-5);
        assert!(!db.objects.get(&9003).unwrap().containable);
        assert!((db.objects.get(&9004).unwrap().contain_size - 2.0).abs() < 1e-5);
        assert!(db.objects.get(&9004).unwrap().containable);
        // Shears: permanent cleared, contain 1
        assert!(!db.objects.get(&9005).unwrap().permanent);
        assert!((db.objects.get(&9005).unwrap().contain_size - 1.0).abs() < 1e-5);
        assert!(db.objects.get(&9005).unwrap().containable);
    }

    /// TWIN-PARTY-RESID: ObjectData.male from person object files.
    // Haxe: ObjectData.male
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

    // Haxe: ServerSettings.PatchObjectData damage / woundFactor
    #[test]
    fn apply_combat_damage_patches_knife_snake_wound() {
        let mut db = ContentDb::default();
        db.objects.insert(560, ObjectDef::empty(560));
        db.objects.insert(764, ObjectDef::empty(764));
        db.objects.insert(798, ObjectDef::empty(798));
        apply_default_combat_damage_patches(&mut db);
        let knife = db.get(560).unwrap();
        assert!((knife.damage - 5.0).abs() < 1e-5);
        assert!((knife.damage_protection_factor - 0.8).abs() < 1e-5);
        let snake = db.get(764).unwrap();
        assert!((snake.damage - 2.0).abs() < 1e-5);
        assert!((snake.wound_factor - 0.98).abs() < 1e-5);
        let arrow_w = db.get(798).unwrap();
        assert!((arrow_w.damage - 0.06).abs() < 1e-5);
    }

    // Haxe: ObjectData useDistance/deadlyDistance + PatchObjectData weapons
    #[test]
    fn parse_use_and_deadly_distance_and_weapon_patches() {
        let dir = std::env::temp_dir().join("ol_content_test_range");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("152.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "id=152").unwrap();
        writeln!(f, "Bow and Arrow").unwrap();
        writeln!(f, "deadlyDistance=3").unwrap();
        writeln!(f, "useDistance=5").unwrap();
        let def = load_object_file(&path).unwrap();
        assert_eq!(def.use_distance, 5);
        assert!((def.deadly_distance - 3.0).abs() < 1e-5);
        assert_eq!(def.effective_use_distance(), 5);

        let mut db = ContentDb::default();
        db.objects.insert(152, def);
        apply_default_weapon_range_patches(&mut db);
        let bow = db.get(152).unwrap();
        assert_eq!(bow.use_distance, 5);
        assert!((bow.deadly_distance - 4.0).abs() < 1e-5);

        // animal moves from auto_decays
        let mut wolf = ObjectDef::empty(418);
        db.objects.insert(418, wolf.clone());
        db.auto_decays.insert(
            418,
            Transition {
                actor_id: -1,
                target_id: 418,
                new_actor_id: 0,
                new_target_id: 418,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 3.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 2,
                desired_move_dist: 4,
                actor_min_use_fraction: 0.0,
                target_min_use_fraction: 0.0,
                switch_number_of_uses: false,
                target_number_of_uses: -1,
                is_pickup_or_drop: false,
            },
        );
        apply_animal_moves_from_transitions(&mut db);
        wolf = db.get(418).unwrap().clone();
        assert_eq!(wolf.moves, 2);
        assert!(wolf.is_animal());

        // Animal deadlyDistance factor (files often 1 → patch 0.5)
        let mut wolf_file = ObjectDef::empty(418);
        wolf_file.deadly_distance = 1.0;
        db.objects.insert(418, wolf_file);
        apply_default_animal_deadly_distance_patches(&mut db);
        let w = db.get(418).unwrap();
        assert!((w.deadly_distance - ANIMAL_DEADLY_DISTANCE_FACTOR).abs() < 1e-5);
        // Mosquito gets deadly patch but is_animal stays false (id 2156 exclusion).
        let mut moz = ObjectDef::empty(2156);
        moz.moves = 3;
        moz.deadly_distance = 1.0;
        db.objects.insert(2156, moz);
        apply_default_animal_deadly_distance_patches(&mut db);
        let m = db.get(2156).unwrap();
        assert!((m.deadly_distance - 0.5).abs() < 1e-5);
        assert!(!m.is_animal());
        let _ = fs::remove_dir_all(&dir);
    }

    // Haxe: ObjectData.useDistance clamp + isAnimal mosquito exclusion
    #[test]
    fn effective_use_distance_and_is_animal() {
        let mut d = ObjectDef::empty(1);
        d.use_distance = 0;
        assert_eq!(d.effective_use_distance(), 1);
        d.use_distance = -3;
        assert_eq!(d.effective_use_distance(), 1);
        d.use_distance = 5;
        assert_eq!(d.effective_use_distance(), 5);

        d.moves = 2;
        assert!(d.is_animal());
        let mut moz = ObjectDef::empty(2156);
        moz.moves = 5;
        assert!(!moz.is_animal());
        let mut rock = ObjectDef::empty(33);
        rock.moves = 0;
        assert!(!rock.is_animal());
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

    fn bare_tr(a: i32, t: i32, na: i32, nt: i32) -> Transition {
        Transition {
            actor_id: a,
            target_id: t,
            new_actor_id: na,
            new_target_id: nt,
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
        }
    }

    /// Haxe changeToolTransitions: 235+well same-actor fill rewrites via 235+-1 → 382;
    /// EMPTY+Cold Bowl 0+1021 is skipped (actorID != newActorID).
    // Haxe: TransitionImporter.changeToolTransitions
    #[test]
    fn change_tool_transitions_water_bowl_not_cold_bowl() {
        let mut db = ContentDb::default();
        db.objects.insert(
            235,
            ObjectDef {
                num_uses: 0,
                ..ObjectDef::empty(235)
            },
        );
        db.objects.insert(382, ObjectDef::empty(382));
        db.objects.insert(662, ObjectDef::empty(662));
        db.objects.insert(1021, ObjectDef::empty(1021));

        // File: 235 + 662 = 235 + 664 (same actor — empty bowl fill)
        db.transitions
            .insert((235, 662), bare_tr(235, 662, 235, 664));
        // Tool: 235 + -1 = 382 + 0
        db.transitions
            .insert((235, -1), bare_tr(235, -1, 382, 0));
        // EMPTY + Cold Bowl — must skip
        db.transitions
            .insert((0, 1021), bare_tr(0, 1021, 1022, 0));

        let n = change_tool_transitions(&mut db);
        assert!(n >= 1, "expected water-bowl style rewrite, got {n}");
        assert_eq!(
            db.find_transition(235, 662).unwrap().new_actor_id,
            382,
            "235+well newActor should rewrite via 235+-1 → 382"
        );
        assert_eq!(
            db.find_transition(0, 1021).unwrap().new_actor_id,
            1022,
            "EMPTY+Cold Bowl must not be rewritten"
        );
    }

    #[test]
    fn parse_transition_file() {
        let dir = std::env::temp_dir().join("ol_content_test_tr");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("0_33.txt");
        let mut f = fs::File::create(&path).unwrap();
        // bare hand on object 33 → newActor 34 newTarget 32 ...
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

    /// Goldens from real OneLifeData7 transition files (Haxe TransitionImporter shape).
    /// Skips if neither local content junction nor OhOl data tree is present.
    #[test]
    fn category_expands_shallow_digger_to_sharp_stone() {
        // Requires full content tree (skip if absent).
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/OneLifeData7");
        if !root.is_dir() {
            return;
        }
        let db = load_content(&root).expect("load content");
        // Category 722 (@ Shallow Digger) contains 34 Sharp Stone.
        // Transition 722+36 → 722+39 expands to 34+36 → 34+39.
        assert!(
            db.find_transition(34, 36).is_some(),
            "sharp stone on seeding wild carrot must resolve via category 722"
        );
        let t = db.find_transition(34, 36).unwrap();
        assert_eq!(t.new_target_id, 39, "dug wild carrot");
        // Dummy ids allocated for multi-use objects like stone pile 661.
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
        // Full load: find_transition must match goldens (Haxe lookup path).
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
        // Timing fields populated on load.
        assert!(db.load_objects_ms > 0 || db.load_total_ms > 0);
        assert!(db.load_transitions_ms > 0 || db.transition_count == 0);
    }

    #[test]
    fn fixture_transition_matches_haxe_filename_parse() {
        // Mirrors Haxe: stem actor_target, line "newActor newTarget …"
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
