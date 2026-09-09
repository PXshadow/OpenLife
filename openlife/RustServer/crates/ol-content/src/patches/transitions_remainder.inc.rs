/// Remainder of Haxe `ServerSettings.PatchTransitions` not covered by horse/alt/ignore/hungry.
///
/// 1:1 from `openlife/settings/ServerSettings.hx` L1732–3995 (uncommented).
/// Skips `trace` / `throw` / `InitWaterSourceIds` / `InitWateringTargets`.
pub fn apply_haxe_patch_transitions_remainder(db: &mut ContentDb) {
    // Haxe L1741–1742 lastUseObject / undoLastUseObject
    db.last_use_object.insert(30, 279);
    db.last_use_object.insert(279, 30);

    apply_haxe_time_auto_decay_table(db);
    apply_haxe_new_transition_bodies(db);
    apply_haxe_get_transition_mutations(db);
    apply_haxe_wound_time_outcomes(db);
    apply_haxe_actor_lt_minus1_and_special_times(db);
    apply_haxe_wool_rabbit_fur_time_loops(db);
    apply_haxe_well_tapout_ai_ignore(db);
    apply_haxe_property_gate_ai_and_alt(db);

    // Haxe L2689–2692 object hungryWork (PatchTransitions)
    db.object_hungry_work.insert(3146, HUNGRY_WORK_COST);
    db.object_hungry_work.insert(1853, HUNGRY_WORK_COST);

    apply_haxe_limit_transitions_stub(db);
}

/// Haxe `new TransitionData` + `addTransition` (including last-use extra args).
///
/// flags: 1 revA, 2 revT, 4 noA, 8 noT, 16 lastA, 32 lastT, 64 addLA, 128 addLT, 256 aiIgnore, 512 tool.
fn apply_haxe_new_transition_bodies(db: &mut ContentDb) {
    const NEW: &[(i32, i32, i32, i32, f32, i32, f32, i32, u32)] = &[
        (467, 556, 467, 550, 0.0, 0, 0.0, -1, 0),
        (210, 82, 209, 85, 0.0, 0, 0.0, -1, 256),
        (382, 82, 235, 85, 0.0, 0, 0.0, -1, 256),
        (560, 418, 750, 422, 0.0, 0, 0.0, -1, 0),
        (560, 1323, 750, 1332, 0.0, 0, 0.0, -1, 0),
        (560, 1328, 750, 1331, 0.0, 0, 0.0, -1, 0),
        (560, 1458, 560, 1900, 0.0, 0, 0.0, -1, 0),
        (560, 1459, 560, 1487, 0.0, 0, 0.0, -1, 256),
        (560, 1462, 560, 1487, 0.0, 0, 0.0, -1, 256),
        (560, 1900, 560, 587, 0.0, 0, 0.0, -1, 0),
        (560, 1487, 560, 587, 0.0, 0, 0.0, 2, 0),
        (3047, 418, 3048, 422, 0.0, 0, 0.0, -1, 0),
        (3047, 1323, 3048, 1332, 0.0, 0, 0.0, -1, 0),
        (3047, 1328, 3048, 1331, 0.0, 0, 0.0, -1, 0),
        (3047, 1458, 3048, 587, 0.0, 0, 0.0, -1, 0),
        (-1, 493, 0, 151, 2.0, 0, 0.0, -1, 0),
        (-1, 797, 0, 0, 60.0, 0, 0.0, -1, 0),
        (-1, 1380, 0, 0, 180.0, 0, 0.0, -1, 0),
        (-1, 1363, 0, 0, 60.0, 0, 0.0, -1, 0),
        (-1, 1381, 0, 0, 180.0, 0, 0.0, -1, 0),
        (-1, 1377, 0, 0, 40.0, 0, 0.0, -1, 0),
        (-1, 1384, 0, 0, 1200.0, 0, 0.0, -1, 0),
        (-1, 1364, 0, 0, 60.0, 0, 0.0, -1, 0),
        (-1, 1383, 0, 0, 180.0, 0, 0.0, -1, 0),
        (-1, 1366, 0, 0, 60.0, 0, 0.0, -1, 0),
        (-1, 1382, 0, 0, 180.0, 0, 0.0, -1, 0),
        (-1, 1367, 0, 0, -1.0, 0, 0.0, -1, 0),
        (-1, 798, 0, 1365, -2.0, 0, 0.0, -1, 0),
        (-1, 1365, 0, 0, -2.0, 0, 0.0, -1, 0),
        (-1, 421, 0, 422, -12.0, 0, 0.0, -1, 0),
        (-1, 565, 0, 566, -2.0, 0, 0.0, -1, 0),
        (-1, 422, 0, 566, -1.0, 0, 0.0, -1, 0),
        (-1, 423, 0, 566, -1.0, 0, 0.0, -1, 0),
        (-1, 1340, 0, 1343, -2.0, 0, 0.0, -1, 0),
        (-1, 1442, 0, 1444, -1.0, 0, 0.0, -1, 0),
        (-1, 1444, 0, 1446, -1.0, 0, 0.0, -1, 0),
        (-1, 1441, 0, 1443, -1.0, 0, 0.0, -1, 0),
        (-1, 1443, 0, 1445, -1.0, 0, 0.0, -1, 0),
        (-1, 1445, 0, 1437, -1.0, 0, 0.0, -1, 0),
        (-1, 2176, 0, 2177, -1.0, 0, 0.0, -1, 0),
        (-1, 2177, 0, 0, -2.0, 0, 0.0, -1, 0),
        (-1, 2179, 0, 0, -2.0, 0, 0.0, -1, 0),
        (-1, 1331, 0, 1335, -1.0, 0, 0.0, -1, 0),
        (-1, 1330, 0, 1332, -1.0, 0, 0.0, -1, 0),
        (-1, 1332, 0, 1343, -1.0, 0, 0.0, -1, 0),
        (-1, 562, 0, 566, -1.0, 0, 0.0, -1, 0),
        (-1, 650, 0, 630, -48.0, 0, 0.0, -1, 0),
        (684, 650, 684, 4102, 0.0, 0, 10.0, -1, 0),
        (684, 650, 858, 4102, 0.0, 0, 10.0, -1, 80),
        (-1, 30, 0, 30, -1.0, 0, 0.0, -1, 2),
        (-1, 279, 0, 30, -1.0, 0, 0.0, -1, 2),
        (-1, 2142, 0, 2142, -3.0, 0, 0.0, -1, 2),
        (-1, 2145, 0, 2142, -3.0, 0, 0.0, -1, 2),
        (-1, 227, 0, 0, -4.0, 0, 0.0, -1, 0),
        (-1, 1115, 0, 0, -4.0, 0, 0.0, -1, 0),
        (-1, 3180, 0, 291, -4.0, 0, 0.0, -1, 0),
        (-1, 1466, 0, 235, -1.0, 0, 0.0, -1, 0),
        (-1, 1201, 0, 236, -1.0, 0, 0.0, -1, 0),
        (-1, 1202, 0, 236, -1.0, 0, 0.0, -1, 0),
        (-1, 1468, 0, 236, -1.0, 0, 0.0, -1, 0),
        (-1, 4255, 0, 848, -12.0, 0, 0.0, -1, 0),
        (-1, 4265, 0, 848, -12.0, 0, 0.0, -1, 0),
        (-1, 577, 578, 576, 2.0, 1, 0.0, -1, 0),
        (-1, 4194, 1262, 1256, 10.0, 1, 0.0, -1, 0),
        (135, 850, 135, 34, 0.0, 0, 0.0, -1, 256),
        (135, 71, 135, 34, 0.0, 0, 0.0, -1, 256),
        (34, 850, 34, 92, 0.0, 0, 0.0, -1, 256),
        (850, 850, 850, 92, 0.0, 0, 0.0, -1, 768),
        (850, 235, 850, 126, 0.0, 0, 0.0, -1, 260),
        (850, 236, 850, 126, 0.0, 0, 0.0, -1, 4),
        (850, 292, 850, 124, 0.0, 0, 0.0, -1, 260),
        (34, 71, 34, 70, 0.0, 0, 0.0, -1, 256),
        (866, 82, 0, 83, 0.0, 0, 0.0, -1, 0),
        (865, 82, 0, 83, 0.0, 0, 0.0, -1, 0),
        (864, 82, 0, 83, 0.0, 0, 0.0, -1, 0),
        (869, 82, 0, 83, 0.0, 0, 0.0, -1, 0),
        (34, 32, 33, 32, 0.0, 0, 1.0, -1, 0),
        (253, 30, 253, 30, 0.0, 0, 0.0, -1, 1),
        (235, 30, 253, 30, 0.0, 0, 0.0, -1, 1),
        (253, 30, 253, 279, 0.0, 0, 0.0, -1, 129),
        (235, 30, 253, 279, 0.0, 0, 0.0, -1, 129),
        (253, 391, 253, 391, 0.0, 0, 0.0, -1, 1),
        (235, 391, 253, 391, 0.0, 0, 0.0, -1, 1),
        (253, 391, 253, 1135, 0.0, 0, 0.0, -1, 129),
        (235, 391, 253, 1135, 0.0, 0, 0.0, -1, 129),
        (1176, 1172, 1176, 1172, 0.0, 0, 0.0, -1, 1),
        (1176, 1172, 1176, 848, 0.0, 0, 0.0, -1, 129),
        (235, 1172, 1176, 1172, 0.0, 0, 0.0, -1, 257),
        (235, 1172, 1176, 848, 0.0, 0, 0.0, -1, 385),
        (1176, 213, 1176, 1161, 0.0, 0, 0.0, -1, 0),
        (1176, 213, 235, 1161, 0.0, 0, 0.0, -1, 80),
        (1180, 250, 1292, 250, 0.0, 0, 0.0, -1, 0),
        (2092, 191, 2091, 0, 0.0, 0, 0.0, -1, 0),
        (0, 2098, 2099, 2091, 0.0, 0, 0.0, -1, 0),
        (0, 3130, 2365, 945, 0.0, 0, 0.0, -1, 0),
        (0, 3129, 455, 3130, 0.0, 0, 0.0, -1, 0),
        (0, 2388, 2365, 3964, 0.0, 0, 0.0, -1, 0),
        (69, 549, 0, 4143, 0.0, 0, 0.0, -1, 0),
        (248, 2156, 67, 86, 0.0, 0, 5.0, -1, 0),
        (248, 2157, 0, 86, 0.0, 0, 3.0, -1, 0),
        (560, 1446, 560, 1340, 0.0, 0, 0.0, -1, 0),
        (569, 85, 570, 85, 0.0, 0, 0.0, -1, 0),
        (334, 852, 334, 72, 0.0, 0, 0.0, -1, 256),
        (71, 852, 71, 72, 0.0, 0, 0.0, -1, 256),
        (850, 357, 850, 1011, 0.0, 0, 0.0, -1, 516),
        (850, 87, 850, 1011, 0.0, 0, 0.0, -1, 516),
        (850, 88, 850, 1011, 0.0, 0, 0.0, -1, 516),
        (850, 89, 850, 1011, 0.0, 0, 0.0, -1, 516),
        (152, 427, 151, 420, 0.0, 0, 0.0, -1, 0),
        (394, 1099, 394, 1099, 0.0, 0, 0.0, -1, 0),
        (-1, 1284, 0, 291, -2.0, 0, 0.0, -1, 0),
        (227, 59, 0, 128, -2.0, 0, 0.0, -1, 0),
        (-1, 766, 0, 0, -720.0, 0, 0.0, -1, 0),
        (462, 846, 462, 67, 0.0, 0, 0.0, -1, 0),
        (0, 3425, 3425, 0, 0.0, 0, 0.0, -1, 0),
        (1603, 235, 1603, 0, 0.0, 0, 0.0, -1, 513),
        (1602, 316, 1602, 319, 0.0, 0, 0.0, -1, 512),
        (1602, 316, 236, 319, 0.0, 0, 0.0, -1, 592),
        (236, 322, 1602, 325, 0.0, 0, 0.0, -1, 513),
        (1602, 322, 1602, 325, 0.0, 0, 0.0, -1, 513),
        (1602, 236, 1602, 0, 0.0, 0, 0.0, -1, 513),
        (1467, 235, 560, 1465, 0.0, 0, 0.0, -1, 258),
        (579, 581, 579, 59, 0.0, 0, 0.0, -1, 0),
        (441, 107, 441, 77, 0.0, 0, 0.0, -1, 0),
        (77, 227, 0, 78, 0.0, 0, 0.0, -1, 0),
        (227, 86, 0, 78, 0.0, 0, 0.0, -1, 0),
        (227, 82, 0, 3029, 0.0, 0, 0.0, -1, 256),
        (298, 82, 292, 346, 0.0, 0, 0.0, -1, 256),
        (298, 0, 292, 300, 0.0, 0, 0.0, -1, 0),
        (517, 0, 852, 518, 0.0, 0, 0.0, -1, 0),
        (336, 0, 292, 1101, 0.0, 0, 0.0, -1, 0),
        (139, 82, 0, 83, 0.0, 0, 0.0, -1, 256),
        (852, 82, 0, 83, 0.0, 0, 0.0, -1, 256),
        (382, 1465, 235, 382, 0.0, 0, 0.0, -1, 256),
        (382, 2877, 235, 382, 0.0, 0, 0.0, -1, 256),
        (382, 2828, 235, 382, 0.0, 0, 0.0, -1, 0),
        (210, 2828, 209, 382, 0.0, 0, 0.0, -1, 0),
    ];
    for &(a, t, na, nt, auto, mv, hungry, uses, flags) in NEW {
        let mut tr = new_trans(a, t, na, nt);
        tr.auto_decay_seconds = auto;
        tr.move_dist = mv;
        tr.hungry_work_cost = hungry;
        tr.target_number_of_uses = uses;
        tr.reverse_use_actor = flags & 1 != 0;
        tr.reverse_use_target = flags & 2 != 0;
        tr.no_use_actor = flags & 4 != 0;
        tr.no_use_target = flags & 8 != 0;
        tr.last_use_actor = flags & 16 != 0;
        tr.last_use_target = flags & 32 != 0;
        let add_la = flags & 64 != 0;
        let add_lt = flags & 128 != 0;
        if flags & 256 != 0 {
            mark_ai_ignore(db, a, t);
        }
        if flags & 512 != 0 {
            db.trans_tool.insert((a, t));
        }
        add_transition_haxe(db, tr, add_la, add_lt);
    }
}

fn apply_haxe_time_auto_decay_table(db: &mut ContentDb) {
    // Haxe getTransition(-1, id).autoDecaySeconds (horse cart timers already applied)
    const TIME: &[(i32, f32)] = &[
        (631, 2.5),
        (761, 600.0),
        (282, 40.0),
        (885, -240.0),
        (886, -240.0),
        (887, -240.0),
        (884, -240.0),
        (750, 3.0),
        (3048, 2.0),
        (749, 6.0),
        (400, 600.0),
        (1385, 3.0),
        (1333, 3.0),
        (1334, 3.0),
        (653, 3.0),
        (654, 3.0),
        (655, 3.0),
        (637, 3.0),
        (1343, -4.0),
        (891, -24.0),
        (155, -24.0),
        (2180, -240.0),
        (712, -240.0),
        (2181, -240.0),
        (304, 40.0),
        (61, 300.0),
        (62, 300.0),
        (75, 20.0),
        (248, 90.0),
        (80, 15.0),
        (249, 25.0),
        (1281, 20.0),
        (861, -12.0),
        (846, -2.0),
        (330, 20.0),
        (252, 120.0),
        (389, -48.0),
        (866, -2.0),
        (865, -2.0),
        (869, -24.0),
        (864, -24.0),
        (2723, -24.0),
        (1872, -24.0),
        (1802, -24.0),
        (4311, -24.0),
        (3069, -24.0),
        (1805, -24.0),
        (1804, -24.0),
        (1803, -24.0),
        (1825, -24.0),
        (1823, -24.0),
        (1820, -24.0),
        (1818, -24.0),
        (1814, -1.0),
    ];
    for &(target, secs) in TIME {
        set_auto_decay_or_stored(db, -1, target, secs);
    }
    const TIME_MOVE: &[(i32, f32, i32)] = &[(427, 3.0, 5), (428, 3.0, 2)];
    for &(target, secs, mv) in TIME_MOVE {
        set_auto_decay_or_stored(db, -1, target, secs);
        if let Some(t) = trans_mut(db, -1, target) {
            t.move_dist = mv;
        }
        if let Some(t) = db.auto_decays.get_mut(&target) {
            t.move_dist = mv;
            t.auto_decay_seconds = secs;
        }
    }
}

fn apply_haxe_get_transition_mutations(db: &mut ContentDb) {
    // Bow and Arrow + 0 → Yew Bow (Haxe L1887–1889)
    if let Some(t) = trans_mut(db, 152, 0) {
        t.new_actor_id = 151;
    }

    // Canada Goose Pond swimming TIME → pond (Haxe L2779–2782)
    let pond_swim = trans_mut(db, -1, 142).map(|t| {
        t.new_target_id = 141;
        t.auto_decay_seconds = 20.0;
        t.clone()
    });
    if let Some(t) = pond_swim {
        db.auto_decays.insert(142, t.clone());
        add_transition_haxe(db, t, false, false);
    }

    // targetMinUseFraction = 0 (Haxe L2577–2604)
    for &(a, t, lut) in &[
        (253, 2742, false),
        (253, 2742, true),
        (253, 3978, false),
        (253, 3978, true),
        (1137, 1101, false),
        (1137, 1101, true),
        (235, 1101, false),
        (235, 1101, true),
    ] {
        if let Some(tr) = trans_mut_any(db, a, t, false, lut) {
            tr.target_min_use_fraction = 0.0;
        }
    }

    // Wolf / bear / bison / seal meat (Haxe L2614–2636)
    patch_then_readd(db, 0, 423, |t| {
        t.new_target_id = 565;
        t.target_number_of_uses = 2;
    });
    patch_then_readd(db, 0, 657, |t| {
        t.new_target_id = 1340;
    });
    patch_then_readd(db, 0, 709, |t| {
        t.new_target_id = 1340;
    });
    patch_then_readd(db, 0, 1444, |t| {
        t.new_target_id = 565;
    });

    // Rope + Newcomen Engine without Rope (Haxe L3075–3076)
    if let Some(t) = trans_mut(db, 59, 2245) {
        t.new_target_id = 2244;
    }

    // Steel Axe + Banana Plant isForbidden (Haxe L3103–3104)
    if let Some(t) = trans_mut(db, 334, 2142) {
        t.is_forbidden = true;
    }
    db.forbidden_transitions.insert((334, 2142));

    // TIME + Corn Tortilla Table justMade (Haxe L3454–3456)
    if let Some(t) = trans_mut(db, -1, 4091) {
        t.no_use_target = true;
    }

    // Fed Mouflon Lamb last-use → Domestic Sheep (Haxe L2987–2990)
    let mouflon = trans_mut_any(db, -1, 601, false, true).map(|t| {
        t.new_target_id = 575;
        t.clone()
    });
    if let Some(t) = mouflon {
        add_transition_haxe(db, t, false, false);
    }

    // Gold pick + Gold Vein coinCost (Haxe L3826–3828)
    if let Some(t) = trans_mut(db, 684, 680) {
        t.coin_cost = 20;
    }

    // TIME + Bear Cave awake already sets newTarget 631 in ai-ignore synth.
}

fn apply_haxe_wound_time_outcomes(db: &mut ContentDb) {
    // Haxe L1925–1978 PatchTransitions overwrites PatchObjectData alternatives
    db.alternative_time_outcome.insert(797, 1380);
    db.alternative_time_outcome.insert(1363, 1381);
    db.alternative_time_outcome.insert(1377, 1384);
    db.alternative_time_outcome.insert(1366, 1383); // Haxe writes Hog Cut onto 1366 first
    db.alternative_time_outcome.insert(1366, 1382); // last write Empty Arrow Wound

    let arrow = trans_mut(db, 0, 798).map(|t| {
        let old = t.new_target_id;
        t.new_target_id = 1367;
        (old, t.clone())
    });
    if let Some((old, t)) = arrow {
        db.alternative_time_outcome.insert(798, old);
        add_transition_haxe(db, t, false, false);
    }
    let extracted = trans_mut(db, 0, 1367).map(|t| {
        let old = t.new_target_id;
        t.new_target_id = 1366;
        (old, t.clone())
    });
    if let Some((old, t)) = extracted {
        db.alternative_time_outcome.insert(1367, old);
        add_transition_haxe(db, t, false, false);
    }
}

fn patch_then_readd(db: &mut ContentDb, actor: i32, target: i32, f: impl FnOnce(&mut Transition)) {
    let patched = trans_mut(db, actor, target).map(|t| {
        f(t);
        t.clone()
    });
    if let Some(t) = patched {
        add_transition_haxe(db, t, false, false);
    }
}

fn apply_haxe_actor_lt_minus1_and_special_times(db: &mut ContentDb) {
    // Haxe L2096–2124: actorID < -1 → 0; -168h → -24h; 9000s → 1200s
    let mut remap: Vec<Transition> = Vec::new();
    for t in db.transitions.values() {
        if t.actor_id < -1 {
            remap.push(t.clone());
        }
    }
    for t in db.transitions_last_use.values() {
        if t.actor_id < -1 {
            remap.push(t.clone());
        }
    }
    for t in db.transitions_max_use.values() {
        if t.actor_id < -1 {
            remap.push(t.clone());
        }
    }
    let old_keys: Vec<(i32, i32)> = db
        .transitions
        .keys()
        .copied()
        .filter(|(a, _)| *a < -1)
        .collect();
    for k in old_keys {
        db.transitions.remove(&k);
    }
    let old_lu: Vec<(i32, i32)> = db
        .transitions_last_use
        .keys()
        .copied()
        .filter(|(a, _)| *a < -1)
        .collect();
    for k in old_lu {
        db.transitions_last_use.remove(&k);
    }
    for mut t in remap {
        t.actor_id = 0;
        add_transition_haxe(db, t, false, false);
    }

    for_each_trans_mut(db, |t| {
        if (t.auto_decay_seconds - -168.0).abs() < 1e-3 {
            t.auto_decay_seconds = -24.0;
        }
        if (t.auto_decay_seconds - 9000.0).abs() < 1e-3 {
            t.auto_decay_seconds = 1200.0;
        }
    });
}

fn apply_haxe_wool_rabbit_fur_time_loops(db: &mut ContentDb) {
    // Haxe L2876–2898
    let ids: Vec<i32> = db.objects.keys().copied().collect();
    for id in ids {
        let Some(desc) = db.objects.get(&id).map(|o| o.description.clone()) else {
            continue;
        };
        let pid = parent_id(db, id);
        if desc.contains("Wool") {
            let updated = trans_mut(db, -1, pid).and_then(|t| {
                if t.auto_decay_seconds < 0.0 {
                    t.auto_decay_seconds = WOOL_CLOTH_DECAY_TIME;
                    Some(t.clone())
                } else {
                    None
                }
            });
            if let Some(t) = updated {
                db.auto_decays.insert(pid, t);
            } else if let Some(t) = db.auto_decays.get_mut(&pid) {
                if t.auto_decay_seconds < 0.0 {
                    t.auto_decay_seconds = WOOL_CLOTH_DECAY_TIME;
                }
            }
        }
        if desc.contains("Rabbit Fur") {
            let updated = trans_mut(db, -1, pid).and_then(|t| {
                if (t.auto_decay_seconds - -5.0).abs() < 1e-5 {
                    t.auto_decay_seconds = RABBIT_FUR_CLOTH_DECAY_TIME;
                    Some(t.clone())
                } else {
                    None
                }
            });
            if let Some(t) = updated {
                db.auto_decays.insert(pid, t);
            } else if let Some(t) = db.auto_decays.get_mut(&pid) {
                if (t.auto_decay_seconds - -5.0).abs() < 1e-5 {
                    t.auto_decay_seconds = RABBIT_FUR_CLOTH_DECAY_TIME;
                }
            }
        }
    }
}

fn apply_haxe_well_tapout_ai_ignore(db: &mut ContentDb) {
    // Haxe L3942–3984 GetTransitionByActor wells
    for actor in [662, 664, 663, 1097, 1861] {
        let mut keys = Vec::new();
        for t in db.transitions.values() {
            if t.actor_id == actor {
                keys.push((t.actor_id, t.target_id));
            }
        }
        for t in db.transitions_last_use.values() {
            if t.actor_id == actor {
                keys.push((t.actor_id, t.target_id));
            }
        }
        for t in db.transitions_max_use.values() {
            if t.actor_id == actor {
                keys.push((t.actor_id, t.target_id));
            }
        }
        for k in keys {
            db.ai_should_ignore.insert(k);
        }
    }
}

fn apply_haxe_property_gate_ai_and_alt(db: &mut ContentDb) {
    // Haxe L3836–3850: all aiShouldIgnore; hungry already patched; alt 0 for non-skewer
    let mut keys = Vec::new();
    for t in db
        .transitions
        .values()
        .chain(db.transitions_last_use.values())
        .chain(db.transitions_max_use.values())
    {
        if t.target_id == 2962 {
            keys.push((t.actor_id, t.target_id));
        }
    }
    for (a, t) in keys {
        db.ai_should_ignore.insert((a, t));
        if a == 0 || a == 139 || a == 852 {
            continue;
        }
        db.alt_outcomes_transition
            .entry((a, t))
            .or_default()
            .push(0);
    }
}

/// Haxe `LimitTransitionsIfTooMuchOfObject` / `LimitTransitionsIfTooFewOfObject`.
fn apply_haxe_limit_transitions_stub(db: &mut ContentDb) {
    apply_haxe_limit_transitions_if_too_much(db);
    apply_haxe_limit_transitions_if_too_few(db);
}

fn tag_ignore_max_on_edge(db: &mut ContentDb, actor: i32, target: i32, id: i32) {
    db.ignore_if_max_reached.insert((actor, target), id);
}

fn tag_ignore_min_on_edge(db: &mut ContentDb, actor: i32, target: i32, id: i32) {
    db.ignore_if_min_not_reached.insert((actor, target), id);
}

fn limit_transition_if_max_reached(db: &mut ContentDb, actor: i32, target: i32, id: i32, max: i32) {
    db.ai_craft_max.insert(id, max);
    tag_ignore_max_on_edge(db, actor, target, id);
}

fn pile_obj_id(db: &ContentDb, id: i32) -> i32 {
    if let Some(t) = db.transitions.get(&(id, id)) {
        if t.reverse_use_target && t.new_target_id > 0 {
            return t.new_target_id;
        }
    }
    -1
}

fn limit_object(db: &mut ContentDb, id: i32, limit_new_actor: i32, max: i32) {
    db.ai_craft_max.insert(id, max);
    let pile = pile_obj_id(db, id);
    let keys: Vec<(i32, i32)> = db
        .transitions
        .values()
        .chain(db.transitions_last_use.values())
        .filter(|t| t.new_actor_id == limit_new_actor && t.target_id != pile)
        .map(|t| (t.actor_id, t.target_id))
        .collect();
    for (a, t) in keys {
        tag_ignore_max_on_edge(db, a, t, id);
    }
}

fn limit_object_by_new_target(db: &mut ContentDb, id: i32, limit_new_target: i32, max: i32) {
    db.ai_craft_max.insert(id, max);
    let keys: Vec<(i32, i32)> = db
        .transitions
        .values()
        .chain(db.transitions_last_use.values())
        .filter(|t| t.new_target_id == limit_new_target)
        .map(|t| (t.actor_id, t.target_id))
        .collect();
    for (a, t) in keys {
        tag_ignore_max_on_edge(db, a, t, id);
    }
}

fn apply_haxe_limit_transitions_if_too_much(db: &mut ContentDb) {
    // Haxe L4058–4143
    limit_transition_if_max_reached(db, 559, 69, 500, 1);
    limit_transition_if_max_reached(db, 69, 559, 500, 1);
    limit_transition_if_max_reached(db, 443, 69, 441, 1);
    limit_transition_if_max_reached(db, 69, 443, 441, 1);
    limit_transition_if_max_reached(db, 124, 124, 292, 5);
    limit_transition_if_max_reached(db, 58, 124, 292, 5);
    limit_transition_if_max_reached(db, 58, 123, 292, 5);
    limit_transition_if_max_reached(db, 139, 227, 292, 5);
    limit_transition_if_max_reached(db, 852, 227, 292, 5);
    limit_object(db, 1121, 1122, 1);
    limit_object(db, 502, 500, 1);
    limit_object(db, 570, 570, 5);
    limit_object(db, 235, 283, 10);
    limit_object(db, 236, 241, 5);
    limit_object(db, 183, 180, 20);
    limit_object(db, 132, 132, 20);
    limit_object(db, 64, 64, 120);
    limit_object(db, 213, 1136, 20);
    limit_object(db, 1053, 1053, 1);
    limit_object(db, 1465, 1464, 1);
    limit_object(db, 2877, 2859, 1);
    limit_object_by_new_target(db, 2877, 2859, 1);
    limit_object(db, 1880, 1879, 1);
    limit_object_by_new_target(db, 1880, 1879, 1);
    limit_object(db, 1101, 336, 3);
    limit_object_by_new_target(db, 2835, 2829, 9);
    limit_object_by_new_target(db, 625, 623, 3);
    limit_object_by_new_target(db, 625, 625, 3);
    limit_object_by_new_target(db, 2828, 2825, 1);
    limit_object_by_new_target(db, 4232, 4225, 5);
    limit_object_by_new_target(db, 391, 216, 10);
    limit_object_by_new_target(db, 242, 228, 30);
    limit_object_by_new_target(db, 1246, 1246, 2);
    limit_transition_if_max_reached(db, 33, 245, 264, 3);
    limit_transition_if_max_reached(db, 1466, 236, 1471, 2);
    limit_transition_if_max_reached(db, 235, 1099, 382, 3);
    limit_transition_if_max_reached(db, 236, 2187, 2190, 1);
    // Haxe AiHelper.pies / AiBase.rawPies
    const PIES: [i32; 8] = [272, 803, 273, 274, 275, 276, 277, 278];
    const RAW_PIES: [i32; 8] = [265, 802, 268, 270, 266, 271, 269, 267];
    for i in 0..8 {
        limit_object_by_new_target(db, PIES[i], RAW_PIES[i], 2);
    }
}

fn limit_transition_if_min_not_reached(
    db: &mut ContentDb,
    actor: i32,
    target: i32,
    id: i32,
    min: i32,
) {
    db.ai_craft_min.insert(id, min);
    tag_ignore_min_on_edge(db, actor, target, id);
}

fn apply_haxe_limit_transitions_if_too_few(db: &mut ContentDb) {
    // Haxe L4191–4262
    limit_transition_if_min_not_reached(db, 334, 239, 239, 3);
    limit_transition_if_min_not_reached(db, 71, 239, 239, 3);
    limit_transition_if_min_not_reached(db, 135, 151, 151, 3);
    limit_transition_if_min_not_reached(db, 560, 151, 151, 3);
    limit_transition_if_min_not_reached(db, 152, 531, 531, 3);
    limit_transition_if_min_not_reached(db, 560, 575, 575, 3);
    limit_transition_if_min_not_reached(db, 560, 576, 576, 4);
    limit_transition_if_min_not_reached(db, 560, 1458, 1458, 4);
    limit_transition_if_min_not_reached(db, 3047, 1458, 1458, 4);
    limit_transition_if_min_not_reached(db, 1878, 1458, 1458, 2);
    limit_transition_if_min_not_reached(db, 152, 141, 141, 2);
    limit_transition_if_min_not_reached(db, 318, 302, 318, 8);
    limit_transition_if_min_not_reached(db, 318, 301, 318, 8);
    limit_transition_if_min_not_reached(db, 334, 63, 63, 20);
    limit_transition_if_min_not_reached(db, 334, 48, 48, 10);
    limit_transition_if_min_not_reached(db, 334, 153, 153, 10);
    limit_transition_if_min_not_reached(db, 334, 406, 406, 5);
    limit_transition_if_min_not_reached(db, 382, 2828, 2828, 3);
    limit_transition_if_min_not_reached(db, 210, 2828, 2828, 3);
    limit_transition_if_min_not_reached(db, 0, 51, 51, 2);
    limit_transition_if_min_not_reached(db, 0, 50, 50, 2);
    limit_transition_if_min_not_reached(db, 850, 236, 236, 10);
    limit_transition_if_min_not_reached(db, 1267, 338, 1256, 3);
    limit_transition_if_min_not_reached(db, 850, 1101, 1101, 7);
    limit_transition_if_min_not_reached(db, 857, 1101, 1101, 5);
    limit_transition_if_min_not_reached(db, 579, 581, 3919, 3);
}

#[cfg(test)]
mod transitions_remainder_tests {
    use super::*;

    fn bare_tr(a: i32, t: i32, na: i32, nt: i32) -> Transition {
        Transition {
            actor_id: a,
            target_id: t,
            new_actor_id: na,
            new_target_id: nt,
            ..Default::default()
        }
    }

    fn time_tr(target: i32, secs: f32) -> Transition {
        Transition {
            actor_id: -1,
            target_id: target,
            auto_decay_seconds: secs,
            ..Default::default()
        }
    }

    #[test]
    fn last_use_object_gooseberry_30_279() {
        let mut db = ContentDb::default();
        apply_haxe_patch_transitions_remainder(&mut db);
        assert_eq!(db.last_use_object.get(&30), Some(&279));
        assert_eq!(db.last_use_object.get(&279), Some(&30));
    }

    #[test]
    fn time_631_auto_decay_2_5_if_present() {
        let mut db = ContentDb::default();
        db.transitions.insert((-1, 631), time_tr(631, 3.0));
        db.auto_decays.insert(631, time_tr(631, 3.0));
        apply_haxe_patch_transitions_remainder(&mut db);
        let t = db.transitions.get(&(-1, 631)).unwrap();
        assert!((t.auto_decay_seconds - 2.5).abs() < 1e-5);
        assert!((db.auto_decays.get(&631).unwrap().auto_decay_seconds - 2.5).abs() < 1e-5);
    }

    #[test]
    fn wool_description_time_uses_wool_cloth_decay_time() {
        let mut db = ContentDb::default();
        let mut wool = ObjectDef::empty(9001);
        wool.description = "Wool Coat".into();
        db.objects.insert(9001, wool);
        db.transitions.insert((-1, 9001), time_tr(9001, -5.0));
        db.auto_decays.insert(9001, time_tr(9001, -5.0));
        apply_haxe_patch_transitions_remainder(&mut db);
        assert!(
            (db.transitions.get(&(-1, 9001)).unwrap().auto_decay_seconds - WOOL_CLOTH_DECAY_TIME)
                .abs()
                < 1e-5
        );
    }

    #[test]
    fn gold_pick_coin_cost_20() {
        let mut db = ContentDb::default();
        db.transitions
            .insert((684, 680), bare_tr(684, 680, 684, 681));
        apply_haxe_patch_transitions_remainder(&mut db);
        assert_eq!(db.transitions.get(&(684, 680)).unwrap().coin_cost, 20);
    }

    #[test]
    fn steel_axe_banana_is_forbidden() {
        let mut db = ContentDb::default();
        db.transitions
            .insert((334, 2142), bare_tr(334, 2142, 334, 0));
        apply_haxe_patch_transitions_remainder(&mut db);
        assert!(db.transitions.get(&(334, 2142)).unwrap().is_forbidden);
        assert!(db.forbidden_transitions.contains(&(334, 2142)));
    }

    #[test]
    fn stone_hoe_grave_is_tool() {
        let mut db = ContentDb::default();
        apply_haxe_patch_transitions_remainder(&mut db);
        assert!(db.trans_is_tool(850, 87));
        assert!(db.trans_is_tool(1603, 235));
        assert!(!db.trans_is_tool(560, 418));
    }

    #[test]
    fn limit_transitions_sets_ai_craft_max_and_min() {
        let mut db = ContentDb::default();
        apply_haxe_patch_transitions_remainder(&mut db);
        assert_eq!(db.ai_craft_max.get(&235), Some(&10));
        assert_eq!(db.ai_craft_max.get(&292), Some(&5));
        assert_eq!(db.ignore_if_max_reached.get(&(559, 69)), Some(&500));
        assert_eq!(db.ai_craft_min.get(&151), Some(&3));
        assert_eq!(db.ignore_if_min_not_reached.get(&(334, 239)), Some(&239));
    }
}
