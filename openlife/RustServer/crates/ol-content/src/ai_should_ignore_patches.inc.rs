/// Haxe `ServerSettings.PatchTransitions` — `TransitionData.aiShouldIgnore` table.
///
/// Haxe stores the flag on each `TransitionData`. Rust keeps side-tables
/// [`ContentDb::ai_should_ignore`] (primary / dual) and
/// [`ContentDb::ai_should_ignore_last_use`] (last-use-only, e.g. pond LT).
/// Reverse craft graph seeds primary only. Safe for partial unit-test DBs
/// (inserts keys + optional synthetic product bodies).
///
/// // Haxe: ServerSettings.PatchTransitions aiShouldIgnore blocks ~1793–3770
/// // Haxe: TransitionData.aiShouldIgnore (set manually in setting)
///
/// Options for [`apply_default_ai_should_ignore_patches_ex`].
// Haxe: ServerSettings.AIAllowBuildOven / AIAllowBuilKiln
#[derive(Debug, Clone, Copy)]
pub struct AiShouldIgnorePatchOpts {
    /// Haxe `AIAllowBuildOven` (default false → ignore Stone+Adobe oven base).
    pub ai_allow_build_oven: bool,
    /// Haxe `AIAllowBuilKiln` (default false → ignore Adobe+Oven kiln).
    pub ai_allow_build_kiln: bool,
}

impl Default for AiShouldIgnorePatchOpts {
    fn default() -> Self {
        Self {
            // Haxe: AIAllowBuildOven = false
            ai_allow_build_oven: false,
            // Haxe: AIAllowBuilKiln = false
            ai_allow_build_kiln: false,
        }
    }
}

/// Apply default ServerSettings aiShouldIgnore patches (Haxe oven/kiln flags off).
// Haxe: ServerSettings.PatchTransitions aiShouldIgnore
pub fn apply_default_ai_should_ignore_patches(db: &mut ContentDb) {
    apply_default_ai_should_ignore_patches_ex(db, AiShouldIgnorePatchOpts::default());
}

/// Apply aiShouldIgnore content patches with oven/kiln allow flags.
// Haxe: ServerSettings.PatchTransitions + AIAllowBuildOven / AIAllowBuilKiln
pub fn apply_default_ai_should_ignore_patches_ex(
    db: &mut ContentDb,
    opts: AiShouldIgnorePatchOpts,
) {
    // Haxe `new TransitionData` product bodies that also set aiShouldIgnore.
    // Insert when missing so live lookup / reverse graph see real products.
    // Haxe: PatchTransitions ~1824–1861, ~3716–3734, ~3772–3776
    insert_synthetic_ai_ignore_product_bodies(db);

    // Explicit (actor, target) pairs from getTransition / new TransitionData.
    // Commented-out Haxe lines intentionally omitted.
    // Pond 141/142 last-use-only pairs live in EXPLICIT_LAST_USE (not here).
    const EXPLICIT: &[(i32, i32)] = &[
        // Early PatchTransitions (~1793–1861)
        (71, 3028),   // Stone Hatchet + Tripod
        (334, 3028),  // Steel Axe + Tripod
        (560, 180),   // Knife + Dead Rabbit
        (127, 282),   // Adobe + Firing Adobe Kiln
        (236, 2836),  // Clay Plate + Tomato
        (258, 659),   // Bowl of Gooseberries and Carrot + Empty Bucket
        (210, 82),    // Full Water Pouch + Fire → coals
        (382, 82),    // Bowl of Water + Fire → coals
        (560, 1459),  // Knife + Domestic Calf
        (560, 1462),  // Knife + Hungry Domestic Calf
        // Sharpie / rope recovery (~2349–2388)
        (135, 850), // Flint Chip + Stone Hoe
        (135, 71),  // Flint Chip + Stone Hatchet
        (34, 850),  // Sharp Stone + Stone Hoe
        (850, 850), // Stone Hoe + Stone Hoe
        (850, 235), // Stone Hoe + Clay Bowl
        (850, 292), // Stone Hoe + Basket
        (34, 71),   // Sharp Stone + Stone Hatchet
        // Beans / green beans (~2465–2478)
        (235, 1172), // Clay Bowl + Dry Bean Plants
        (0, 1175),   // empty + Bowl of Green Beans
        // Kindling from skewer (~2656–2659)
        (334, 852),
        (71, 852),
        // Clay / charcoal / fire (~3026–3053)
        (1467, 235), // Butter Knife + Clay Bowl
        (227, 82),   // Straw + Fire
        (298, 82),   // Basket of Charcoal + Fire
        // Core AI ignore block (~3084–3193)
        (58, 235),   // Thread + Clay Bowl
        (135, 74),   // Flint Chip + Fire Bow Drill
        (560, 74),   // Knife + Fire Bow Drill
        (461, 3371), // Bow Saw + Table
        (71, 67),    // Stone Hatchet + Long Straight Shaft
        (334, 67),   // Steel Axe + Long Straight Shaft
        (334, 2142), // Steel Axe + Banana Plant
        (334, 2145), // Steel Axe + Empty Banana Plant
        (334, 1059), // Steel Axe + Red Rose Bush
        (334, 583),  // Steel Axe + Knitting Needles
        (71, 583),   // Stone Hatchet + Knitting Needles
        (560, 4213), // Knife + Fed Domestic Sheep
        (560, 541),  // Knife + Domestic Mouflon
        (560, 708),  // Knife + Clubbed Seal
        (560, 242),  // Knife + Ripe Wheat
        (560, 121),  // Knife + Tule Reeds
        (560, 2765), // Knife + Sugarcane
        (2365, 3966), // Diesel Engine + Empty Scrap Box
        (345, 82),   // Butt Log + Fire
        (345, 83),   // Butt Log + Large Fast Fire
        (345, 3029), // Butt Log + Flash Fire
        (502, 36),   // Shovel + Seeding Wild Carrot
        (502, 404),  // Shovel + Wild Carrot
        (502, 804),  // Shovel + Burdock
        (67, 3065),  // Long Straight Shaft + Wooden Slot Box
        (0, 2244),   // empty + Newcomen Engine without Shaft
        (33, 127),   // Stone + Adobe (oven base; gated below)
        (127, 237),  // Adobe + Adobe Oven (kiln; gated below)
        (252, 291),  // Bowl of Dough + Flat Rock
        (0, 1471),   // empty + Sliced Bread
        // Burning food (~3209–3231)
        (516, 82),
        (516, 83),
        (516, 346),
        (516, 3029),
        (185, 82),
        (185, 83),
        (185, 346),
        (185, 3029),
        // Stakes / bushes / soil (~3233–3294)
        (107, 279),
        (71, 107),
        (334, 107),
        (107, 392),
        (502, 389),
        (139, 1136),
        (852, 1136),
        (139, 1138),
        (852, 1138),
        (139, 1101),
        (852, 1101),
        (0, 4092),
        (292, 1101),
        (0, 253),
        (0, 247),
        (0, 281),
        (192, 1121),
        (334, 3308),
        (334, 3309),
        (334, 3310),
        (334, 1876),
        (334, 1922),
        (334, 1923),
        (334, 344),
        (71, 344),
        (152, 0),
        (0, 2268),
        (288, 238),
        (288, 299),
        (0, 303),
        (292, 305),
        (1620, 382),
        (239, 309),
        (0, 316),
        (0, 318),
        (33, 318),
        (0, 675),
        (441, 560),
        (0, 185),
        (0, 547),
        (0, 254),
        (0, 1247),
        (502, 32),
        (0, 2090),
        (0, 465),
        (467, 1851),
        (0, 1176),
        (0, 1172),
        (1160, 235),
        (252, 3371),
        (235, 4086),
        (0, 1354),
        (1222, 227),
        (502, 761),
        (467, 550),
        (467, 549),
        (462, 3242),
        (560, 1465),
        (0, 1152),
        (0, 4056),
        (0, 245),
        (0, 4379),
        (-1, 1462),
        (239, 321),
        // (235|209, 141|142) → EXPLICIT_LAST_USE (pond LT only)
        (0, 1012),
        (0, 1849),
        (0, 3948),
        (0, 1121), // primary + last-use both ignored in Haxe
        (0, 2264),
        (0, 2243),
        (471, 434),
        // Skewer fire / butter water (~3716–3769)
        (139, 82),
        (852, 82),
        (382, 1465),
        (382, 2877),
        (210, 1099),
        (210, 659),
        (382, 1099),
        (382, 659),
        (-1, 665),
        (-1, 4085),
        (-1, 648),
    ];

    // Haxe last-use-target-only ignores (GetTransition(a,t,false,true)).
    // Not seeded into reverse craft graph so primary pond fill stays craftable.
    // Haxe: ~3531–3547 Canada Goose Pond 141 / swimming 142
    const EXPLICIT_LAST_USE: &[(i32, i32)] = &[
        (235, 141), // Clay Bowl + Canada Goose Pond (LT)
        (209, 141), // Empty Water Pouch + Pond (LT)
        (235, 142), // Clay Bowl + Pond swimming (LT)
        (209, 142), // Empty Water Pouch + Pond swimming (LT)
    ];

    for &key in EXPLICIT {
        // Oven/kiln gated by AIAllow* (Haxe if false → ignore).
        if key == (33, 127) && opts.ai_allow_build_oven {
            continue;
        }
        if key == (127, 237) && opts.ai_allow_build_kiln {
            continue;
        }
        db.ai_should_ignore.insert(key);
    }
    for &key in EXPLICIT_LAST_USE {
        db.ai_should_ignore_last_use.insert(key);
    }

    // Bulk rules (Haxe TransitionImporter.GetTransitionBy* loops).
    // Broken Steel Tool 858 / 862 — all transitions producing as newActor
    // **and** set actor ObjectData.decaysToObj to the broken tool id.
    // Haxe: ~2849–2864
    mark_by_new_actor_with_decays_to(db, 858, &[]);
    mark_by_new_actor_with_decays_to(db, 862, &[]);

    // Scrap Bowl 3076 as target — ignore except Leaf 62 / Scrap Steel 930.
    // Haxe: ~2866–2874 and ~3644–3652 (duplicate)
    mark_by_target(db, 3076, &[62, 930]);

    // Steel Adze 462 as actor — ignore except 557/156/154/846.
    // Haxe: ~3598–3611
    mark_by_actor(db, 462, &[557, 156, 154, 846]);

    // Bowl of Plaster 677 as actor — ignore except adobe walls 155/156/154.
    // Haxe: ~3613–3624
    mark_by_actor(db, 677, &[155, 156, 154]);

    // Steel Mining Pick 684 as actor — ignore except mining targets.
    // Haxe: ~3626–3642
    mark_by_actor(db, 684, &[680, 3944, 3957, 881, 944]);

    // TIME→Simmering Water 730 (newTarget) with actor < 0 (TIME only).
    // Haxe: ~3655–3663
    mark_time_to_new_target(db, 730);

    // Deep Tilled Row 213 newTarget — only Skewer 139 / Weak Skewer 852.
    // Haxe: ~3665–3674
    mark_by_new_target_actors_only(db, 213, &[139, 852]);

    // Stakes 107 as newTarget — ignore except target 62 / 4066.
    // Haxe: ~3676–3688
    mark_by_new_target_skip_targets(db, 107, &[62, 4066]);

    // Stakes 107 as newActor — ignore except target 4066.
    // Haxe: ~3690–3698
    mark_by_new_actor_skip_targets(db, 107, &[4066]);

    // Clay Plate 236 as newActor — only when actor is Raw Pie Crust 264.
    // Haxe: ~3700–3707
    mark_by_new_actor_actors_only(db, 236, &[264]);

    // Burnt Goose 520 as newActor — all.
    // Haxe: ~3709–3714
    mark_by_new_actor(db, 520, &[]);
}

/// Insert Haxe `new TransitionData` product rows that also set `aiShouldIgnore`.
///
/// Only fills missing keys (does not overwrite content-file transitions).
// Haxe: ServerSettings.PatchTransitions new TransitionData + aiShouldIgnore
fn insert_synthetic_ai_ignore_product_bodies(db: &mut ContentDb) {
    // (actor, target, new_actor, new_target)
    const SYNTH: &[(i32, i32, i32, i32)] = &[
        // Full Water Pouch 210 + Fire 82 → Empty Pouch 209 + Hot Coals 85
        // Haxe: ~1824–1826
        (210, 82, 209, 85),
        // Bowl of Water 382 + Fire 82 → Clay Bowl 235 + Hot Coals 85
        // Haxe: ~1829–1831
        (382, 82, 235, 85),
        // Knife + Domestic Calf 1459 → Knife + Dead Domestic Calf 1487
        // Haxe: ~1855–1857
        (560, 1459, 560, 1487),
        // Knife + Hungry Domestic Calf 1462 → Knife + Dead Domestic Calf 1487
        // Haxe: ~1860–1862
        (560, 1462, 560, 1487),
        // Skewer 139 + Fire 82 → 0 + Large Fast Fire 83
        // Haxe: ~3716–3719
        (139, 82, 0, 83),
        // Weak Skewer 852 + Fire 82 → 0 + Large Fast Fire 83
        // Haxe: ~3721–3724
        (852, 82, 0, 83),
        // Bowl of Water + Bowl of Butter 1465 → Clay Bowl + Bowl of Water
        // Haxe: ~3726–3729
        (382, 1465, 235, 382),
        // Bowl of Water + Bowl of Ketchup 2877 → Clay Bowl + Bowl of Water
        // Haxe: ~3731–3734
        (382, 2877, 235, 382),
    ];
    for &(a, t, na, nt) in SYNTH {
        insert_primary_transition_if_absent(db, a, t, na, nt);
    }

    // TIME + Bear Cave awake 648 → Hungry Grizzly 631 (also aiShouldIgnore).
    // Haxe: ~3772–3776 — mutates existing or inserts synthetic.
    let key = (-1, 648);
    if let Some(tr) = db.transitions.get_mut(&key) {
        tr.new_target_id = 631;
    } else if let Some(tr) = db.auto_decays.get_mut(&648) {
        tr.new_target_id = 631;
    } else {
        insert_primary_transition_if_absent(db, -1, 648, 0, 631);
    }
}

fn insert_primary_transition_if_absent(
    db: &mut ContentDb,
    actor: i32,
    target: i32,
    new_actor: i32,
    new_target: i32,
) {
    let key = (actor, target);
    if db.transitions.contains_key(&key) {
        return;
    }
    db.transitions.insert(
        key,
        Transition {
            actor_id: actor,
            target_id: target,
            new_actor_id: new_actor,
            new_target_id: new_target,
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

#[inline]
#[cfg_attr(not(test), allow(dead_code))]
fn mark_edge(db: &mut ContentDb, actor: i32, target: i32) {
    db.ai_should_ignore.insert((actor, target));
}

fn for_each_transition(db: &ContentDb, mut f: impl FnMut(&Transition)) {
    for t in db.transitions.values() {
        f(t);
    }
    for t in db.transitions_last_use.values() {
        f(t);
    }
    for t in db.transitions_max_use.values() {
        f(t);
    }
    for t in db.auto_decays.values() {
        f(t);
    }
}

fn mark_by_new_actor(db: &mut ContentDb, new_actor: i32, skip_actors: &[i32]) {
    mark_by_new_actor_with_decays_to_opt(db, new_actor, skip_actors, None);
}

/// Like [`mark_by_new_actor`] plus Haxe side-effect `objData.decaysToObj = new_actor`
/// on each matching transition's actor object (Broken Steel Tool loops).
// Haxe: GetTransitionByNewActor(858/862) + objData.decaysToObj
fn mark_by_new_actor_with_decays_to(db: &mut ContentDb, new_actor: i32, skip_actors: &[i32]) {
    mark_by_new_actor_with_decays_to_opt(db, new_actor, skip_actors, Some(new_actor));
}

fn mark_by_new_actor_with_decays_to_opt(
    db: &mut ContentDb,
    new_actor: i32,
    skip_actors: &[i32],
    decays_to: Option<i32>,
) {
    let mut keys = Vec::new();
    let mut actor_ids = Vec::new();
    for_each_transition(db, |t| {
        if t.new_actor_id == new_actor && !skip_actors.contains(&t.actor_id) {
            keys.push((t.actor_id, t.target_id));
            if decays_to.is_some() && t.actor_id > 0 {
                actor_ids.push(t.actor_id);
            }
        }
    });
    for k in keys {
        db.ai_should_ignore.insert(k);
    }
    if let Some(to) = decays_to {
        actor_ids.sort_unstable();
        actor_ids.dedup();
        for aid in actor_ids {
            if let Some(obj) = db.objects.get_mut(&aid) {
                obj.decays_to_obj = to;
            }
        }
    }
}

fn mark_by_new_actor_skip_targets(db: &mut ContentDb, new_actor: i32, skip_targets: &[i32]) {
    let mut keys = Vec::new();
    for_each_transition(db, |t| {
        if t.new_actor_id == new_actor && !skip_targets.contains(&t.target_id) {
            keys.push((t.actor_id, t.target_id));
        }
    });
    for k in keys {
        db.ai_should_ignore.insert(k);
    }
}

fn mark_by_new_actor_actors_only(db: &mut ContentDb, new_actor: i32, only_actors: &[i32]) {
    let mut keys = Vec::new();
    for_each_transition(db, |t| {
        if t.new_actor_id == new_actor && only_actors.contains(&t.actor_id) {
            keys.push((t.actor_id, t.target_id));
        }
    });
    for k in keys {
        db.ai_should_ignore.insert(k);
    }
}

fn mark_by_target(db: &mut ContentDb, target: i32, skip_actors: &[i32]) {
    let mut keys = Vec::new();
    for_each_transition(db, |t| {
        if t.target_id == target && !skip_actors.contains(&t.actor_id) {
            keys.push((t.actor_id, t.target_id));
        }
    });
    for k in keys {
        db.ai_should_ignore.insert(k);
    }
}

fn mark_by_actor(db: &mut ContentDb, actor: i32, skip_targets: &[i32]) {
    let mut keys = Vec::new();
    for_each_transition(db, |t| {
        if t.actor_id == actor && !skip_targets.contains(&t.target_id) {
            keys.push((t.actor_id, t.target_id));
        }
    });
    for k in keys {
        db.ai_should_ignore.insert(k);
    }
}

fn mark_time_to_new_target(db: &mut ContentDb, new_target: i32) {
    let mut keys = Vec::new();
    for_each_transition(db, |t| {
        // Haxe: if (trans.actorID > -1) continue — only TIME (actor ≤ -1)
        if t.new_target_id == new_target && t.actor_id <= -1 {
            keys.push((t.actor_id, t.target_id));
        }
    });
    for k in keys {
        db.ai_should_ignore.insert(k);
    }
}

fn mark_by_new_target_actors_only(db: &mut ContentDb, new_target: i32, only_actors: &[i32]) {
    let mut keys = Vec::new();
    for_each_transition(db, |t| {
        if t.new_target_id == new_target && only_actors.contains(&t.actor_id) {
            keys.push((t.actor_id, t.target_id));
        }
    });
    for k in keys {
        db.ai_should_ignore.insert(k);
    }
}

fn mark_by_new_target_skip_targets(db: &mut ContentDb, new_target: i32, skip_targets: &[i32]) {
    let mut keys = Vec::new();
    for_each_transition(db, |t| {
        if t.new_target_id == new_target && !skip_targets.contains(&t.target_id) {
            keys.push((t.actor_id, t.target_id));
        }
    });
    for k in keys {
        db.ai_should_ignore.insert(k);
    }
}

/// True when craft AI should ignore transition `(actor, target)` (primary table).
// Haxe: TransitionData.aiShouldIgnore
#[inline]
pub fn transition_ai_should_ignore(db: &ContentDb, actor_id: i32, target_id: i32) -> bool {
    db.ai_should_ignore.contains(&(actor_id, target_id))
}

/// True when craft AI should ignore edge, with last-use-only table when `last_use`.
// Haxe: TransitionData.aiShouldIgnore + lastUse maps
#[inline]
pub fn transition_ai_should_ignore_ex(
    db: &ContentDb,
    actor_id: i32,
    target_id: i32,
    last_use: bool,
) -> bool {
    if db.ai_should_ignore.contains(&(actor_id, target_id)) {
        return true;
    }
    last_use && db.ai_should_ignore_last_use.contains(&(actor_id, target_id))
}

#[cfg(test)]
mod ai_should_ignore_tests {
    use super::*;

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

    #[test]
    fn explicit_pairs_marked() {
        let mut db = ContentDb::default();
        // Exist or not — side-table still records ignore keys.
        apply_default_ai_should_ignore_patches(&mut db);
        assert!(transition_ai_should_ignore(&db, 58, 235)); // Thread + Clay Bowl
        assert!(transition_ai_should_ignore(&db, 71, 3028)); // Hatchet + Tripod
        assert!(transition_ai_should_ignore(&db, 516, 82)); // Skewered Goose + Fire
        assert!(transition_ai_should_ignore(&db, 0, 253)); // empty + Bowl of Gooseberries
        assert!(transition_ai_should_ignore(&db, -1, 665)); // TIME + Dry Deep Well
    }

    /// Gap-close sample across Haxe sections 1793–3770 (water loops, TIME, synthetic).
    // Haxe: PatchTransitions aiShouldIgnore scattered blocks
    #[test]
    fn gap_close_section_samples() {
        let mut db = ContentDb::default();
        apply_default_ai_should_ignore_patches(&mut db);
        // Early synthetic coals-from-fire
        assert!(transition_ai_should_ignore(&db, 210, 82));
        assert!(transition_ai_should_ignore(&db, 382, 82));
        // Calf kill ignore
        assert!(transition_ai_should_ignore(&db, 560, 1459));
        assert!(transition_ai_should_ignore(&db, 560, 1462));
        // Dough → bowl mess
        assert!(transition_ai_should_ignore(&db, 252, 291));
        // Don't put water back in bucket (loop)
        assert!(transition_ai_should_ignore(&db, 210, 1099));
        assert!(transition_ai_should_ignore(&db, 210, 659));
        assert!(transition_ai_should_ignore(&db, 382, 1099));
        assert!(transition_ai_should_ignore(&db, 382, 659));
        // TIME well / bear cave
        assert!(transition_ai_should_ignore(&db, -1, 4085));
        assert!(transition_ai_should_ignore(&db, -1, 648));
        // Synthetic skewer+fire (new TransitionData)
        assert!(transition_ai_should_ignore(&db, 139, 82));
        assert!(transition_ai_should_ignore(&db, 852, 82));
        // Butter water mess
        assert!(transition_ai_should_ignore(&db, 382, 1465));
        // Explicit table floor (Haxe active ignore keys, not bulk)
        assert!(db.ai_should_ignore.len() >= 140);
    }

    #[test]
    fn oven_kiln_gated_by_opts() {
        let mut db = ContentDb::default();
        apply_default_ai_should_ignore_patches_ex(
            &mut db,
            AiShouldIgnorePatchOpts {
                ai_allow_build_oven: true,
                ai_allow_build_kiln: true,
            },
        );
        assert!(!transition_ai_should_ignore(&db, 33, 127));
        assert!(!transition_ai_should_ignore(&db, 127, 237));

        let mut db2 = ContentDb::default();
        apply_default_ai_should_ignore_patches(&mut db2);
        assert!(transition_ai_should_ignore(&db2, 33, 127));
        assert!(transition_ai_should_ignore(&db2, 127, 237));
    }

    #[test]
    fn bulk_new_actor_broken_steel() {
        let mut db = ContentDb::default();
        db.transitions
            .insert((441, 560), bare_tr(441, 560, 858, 0));
        db.transitions
            .insert((1, 2), bare_tr(1, 2, 100, 0)); // not broken tool
        apply_default_ai_should_ignore_patches(&mut db);
        assert!(transition_ai_should_ignore(&db, 441, 560));
        assert!(!transition_ai_should_ignore(&db, 1, 2));
    }

    #[test]
    fn bulk_scrap_bowl_skips_leaf() {
        let mut db = ContentDb::default();
        db.transitions
            .insert((62, 3076), bare_tr(62, 3076, 0, 3076));
        db.transitions
            .insert((560, 3076), bare_tr(560, 3076, 0, 3076));
        apply_default_ai_should_ignore_patches(&mut db);
        assert!(!transition_ai_should_ignore(&db, 62, 3076)); // Leaf allowed
        assert!(transition_ai_should_ignore(&db, 560, 3076));
    }

    #[test]
    fn bulk_adze_skips_allowed_targets() {
        let mut db = ContentDb::default();
        db.transitions.insert((462, 557), bare_tr(462, 557, 0, 0));
        db.transitions.insert((462, 999), bare_tr(462, 999, 0, 0));
        apply_default_ai_should_ignore_patches(&mut db);
        assert!(!transition_ai_should_ignore(&db, 462, 557));
        assert!(transition_ai_should_ignore(&db, 462, 999));
    }

    #[test]
    fn bulk_mining_pick_skips_vein() {
        // Haxe: Steel Mining Pick 684 — keep gold vein 680, ignore others
        let mut db = ContentDb::default();
        db.transitions.insert((684, 680), bare_tr(684, 680, 0, 0));
        db.transitions.insert((684, 1), bare_tr(684, 1, 0, 0));
        apply_default_ai_should_ignore_patches(&mut db);
        assert!(!transition_ai_should_ignore(&db, 684, 680));
        assert!(transition_ai_should_ignore(&db, 684, 1));
    }

    #[test]
    fn bulk_plaster_skips_adobe_wall() {
        // Haxe: Bowl of Plaster 677 — keep adobe walls 155/156/154
        let mut db = ContentDb::default();
        db.transitions.insert((677, 155), bare_tr(677, 155, 0, 0));
        db.transitions.insert((677, 999), bare_tr(677, 999, 0, 0));
        apply_default_ai_should_ignore_patches(&mut db);
        assert!(!transition_ai_should_ignore(&db, 677, 155));
        assert!(transition_ai_should_ignore(&db, 677, 999));
    }

    #[test]
    fn bulk_stakes_new_target_skips_leaf_pile() {
        // Haxe: newTarget Stakes 107 — skip target 62 / 4066
        let mut db = ContentDb::default();
        db.transitions.insert((1, 62), bare_tr(1, 62, 0, 107));
        db.transitions.insert((2, 99), bare_tr(2, 99, 0, 107));
        apply_default_ai_should_ignore_patches(&mut db);
        assert!(!transition_ai_should_ignore(&db, 1, 62));
        assert!(transition_ai_should_ignore(&db, 2, 99));
    }

    #[test]
    fn bulk_time_simmering_water_only_time_actor() {
        // Haxe: newTarget 730 only when actorID <= -1
        let mut db = ContentDb::default();
        db.transitions
            .insert((-1, 100), bare_tr(-1, 100, 0, 730));
        db.transitions.insert((5, 100), bare_tr(5, 100, 0, 730));
        apply_default_ai_should_ignore_patches(&mut db);
        assert!(transition_ai_should_ignore(&db, -1, 100));
        assert!(!transition_ai_should_ignore(&db, 5, 100));
    }

    #[test]
    fn bulk_deep_tilled_only_skewers() {
        let mut db = ContentDb::default();
        db.transitions
            .insert((139, 10), bare_tr(139, 10, 0, 213));
        db.transitions
            .insert((850, 10), bare_tr(850, 10, 0, 213));
        apply_default_ai_should_ignore_patches(&mut db);
        assert!(transition_ai_should_ignore(&db, 139, 10));
        assert!(!transition_ai_should_ignore(&db, 850, 10));
    }

    #[test]
    fn bulk_pie_crust_plate() {
        let mut db = ContentDb::default();
        db.transitions
            .insert((264, 235), bare_tr(264, 235, 236, 0));
        db.transitions
            .insert((1, 2), bare_tr(1, 2, 236, 0));
        apply_default_ai_should_ignore_patches(&mut db);
        assert!(transition_ai_should_ignore(&db, 264, 235));
        assert!(!transition_ai_should_ignore(&db, 1, 2));
    }

    #[test]
    fn bulk_burnt_goose_all_new_actor() {
        // Haxe: GetTransitionByNewActor(520) Burnt Goose — all ignore
        let mut db = ContentDb::default();
        db.transitions
            .insert((516, 82), bare_tr(516, 82, 520, 0));
        apply_default_ai_should_ignore_patches(&mut db);
        assert!(transition_ai_should_ignore(&db, 516, 82));
    }

    #[test]
    fn mark_edge_used_by_explicit() {
        // smoke: helper is private; public insert path covered by explicit_pairs
        let mut db = ContentDb::default();
        mark_edge(&mut db, 9, 9);
        assert!(transition_ai_should_ignore(&db, 9, 9));
    }

    /// Side-table survives empty ContentDb (partial unit-test / finish_cache_boot order).
    #[test]
    fn content_db_transition_ai_should_ignore_method() {
        let mut db = ContentDb::default();
        apply_default_ai_should_ignore_patches(&mut db);
        assert!(db.transition_ai_should_ignore(58, 235));
        assert!(!db.transition_ai_should_ignore(1, 2));
        assert!(!db.ai_should_ignore.is_empty());
    }

    /// Haxe `new TransitionData` product bodies (coals / calf / skewer-fire / butter).
    // Haxe: PatchTransitions ~1824–1861, ~3716–3734
    #[test]
    fn synthetic_product_bodies_inserted() {
        let mut db = ContentDb::default();
        apply_default_ai_should_ignore_patches(&mut db);
        let coals = db.find_transition(210, 82).expect("water pouch + fire");
        assert_eq!((coals.new_actor_id, coals.new_target_id), (209, 85));
        let bowl_coals = db.find_transition(382, 82).expect("bowl water + fire");
        assert_eq!((bowl_coals.new_actor_id, bowl_coals.new_target_id), (235, 85));
        let calf = db.find_transition(560, 1459).expect("knife + calf");
        assert_eq!(calf.new_target_id, 1487);
        let skewer = db.find_transition(139, 82).expect("skewer + fire");
        assert_eq!((skewer.new_actor_id, skewer.new_target_id), (0, 83));
        let butter = db.find_transition(382, 1465).expect("water + butter");
        assert_eq!((butter.new_actor_id, butter.new_target_id), (235, 382));
        // Still ignored for craft AI.
        assert!(transition_ai_should_ignore(&db, 210, 82));
        assert!(transition_ai_should_ignore(&db, 139, 82));
        assert!(transition_ai_should_ignore(&db, 382, 1465));
        // Does not overwrite an existing content row.
        let mut db2 = ContentDb::default();
        db2.transitions
            .insert((210, 82), bare_tr(210, 82, 999, 999));
        apply_default_ai_should_ignore_patches(&mut db2);
        let kept = db2.find_transition(210, 82).unwrap();
        assert_eq!((kept.new_actor_id, kept.new_target_id), (999, 999));
        assert!(transition_ai_should_ignore(&db2, 210, 82));
    }

    /// Pond water-fill is last-use-only in Haxe — primary stays craftable.
    // Haxe: ~3531–3547 getTransition(235|209, 141|142, false, true)
    #[test]
    fn pond_last_use_only_does_not_suppress_primary() {
        let mut db = ContentDb::default();
        apply_default_ai_should_ignore_patches(&mut db);
        // Primary craft path must NOT ignore pond fill.
        assert!(!transition_ai_should_ignore(&db, 235, 141));
        assert!(!transition_ai_should_ignore(&db, 209, 141));
        assert!(!db.transition_ai_should_ignore(235, 142));
        // Last-use lookup does ignore.
        assert!(transition_ai_should_ignore_ex(&db, 235, 141, true));
        assert!(transition_ai_should_ignore_ex(&db, 209, 142, true));
        assert!(db.transition_ai_should_ignore_ex(235, 141, true));
        assert!(!db.transition_ai_should_ignore_ex(235, 141, false));
        assert!(db.ai_should_ignore_last_use.contains(&(235, 141)));
        assert!(!db.ai_should_ignore.contains(&(235, 141)));
    }

    /// Broken-steel bulk also sets actor ObjectData.decaysToObj (Haxe side effect).
    // Haxe: ~2849–2864 objData.decaysToObj = 858/862
    #[test]
    fn bulk_broken_steel_sets_actor_decays_to() {
        let mut db = ContentDb::default();
        db.objects.insert(441, ObjectDef::empty(441));
        db.objects.insert(999, ObjectDef::empty(999));
        db.transitions
            .insert((441, 560), bare_tr(441, 560, 858, 0));
        db.transitions
            .insert((999, 1), bare_tr(999, 1, 862, 0));
        apply_default_ai_should_ignore_patches(&mut db);
        assert!(transition_ai_should_ignore(&db, 441, 560));
        assert!(transition_ai_should_ignore(&db, 999, 1));
        assert_eq!(db.objects.get(&441).unwrap().decays_to_obj, 858);
        assert_eq!(db.objects.get(&999).unwrap().decays_to_obj, 862);
    }
}
