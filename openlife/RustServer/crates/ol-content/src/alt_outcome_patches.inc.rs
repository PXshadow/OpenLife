/// Haxe `ServerSettings.PatchTransitions` / `PatchObjectData` alternativeTransitionOutcome
/// and fortification tables (**TH-ALT-OUTCOME**).
///
/// Side-tables on [`ContentDb`] (not OLC1/OLT1 fields): same pattern as
/// `second_time_outcomes` / `ai_should_ignore`.
// Haxe: ServerSettings AlternativeOutcome* + alternativeTransitionOutcome.push
// Haxe: ObjectData.fortificationObjId / fortificationValue

/// Append one outcome id onto the object-keyed list (Haxe `ObjectData….push`).
fn push_obj_alt(db: &mut ContentDb, object_id: i32, outcome_id: i32) {
    db.alt_outcomes_object
        .entry(object_id)
        .or_default()
        .push(outcome_id);
}

/// Append one outcome id onto the transition-keyed list.
fn push_tr_alt(db: &mut ContentDb, actor: i32, target: i32, outcome_id: i32) {
    db.alt_outcomes_transition
        .entry((actor, target))
        .or_default()
        .push(outcome_id);
}

/// Haxe ServerSettings fortification + alternativeTransitionOutcome patches.
///
/// Safe for partial unit-test DBs (only fills side-tables; no missing-object require).
// Haxe: ServerSettings.PatchObjectData fortification + PatchTransitions alt outcomes
pub fn apply_default_alternative_outcome_patches(db: &mut ContentDb) {
    // ── Fortification materials (fortificationValue) ────────────────────────
    // Haxe: L817–820
    db.fortification_value.insert(33, 1.0); // Stone
    db.fortification_value.insert(67, 2.0); // Long Straight Shaft
    db.fortification_value.insert(127, 5.0); // Adobe
    db.fortification_value.insert(470, 5.0); // Boards

    // ── Fortification object id on walls/fences/doors ───────────────────────
    // Haxe: L807–836, L1001–1009
    const FORT_OBJ: &[(i32, i32)] = &[
        (895, 881),  // Ancient Stone Wall corner → Cut Stones
        (896, 881),  // Ancient Stone Wall horizontal
        (897, 881),  // Ancient Stone Wall vertical
        (885, 881),  // Stone Wall variants
        (886, 881),
        (887, 881),
        (237, 33),   // Adobe Oven → Stone (comment says Adobe; code uses 33)
        (238, 33),   // Adobe Kiln
        (2962, 67),  // Property Gate → Long Straight Shaft
        (550, 67),   // Fence horizontal
        (549, 67),   // Fence vertical
        (551, 67),   // Fence corner
        (2757, 470), // Springy Wooden Door horizontal → Boards
        (2759, 470), // Springy Wooden Door vertical
    ];
    for &(id, fort) in FORT_OBJ {
        db.fortification_obj_id.insert(id, fort);
    }

    // ── Transition-keyed alternative outcomes ───────────────────────────────
    // Haxe L2529–2564: PropertyGate-style push(0) so gate applies even when not fortified
    // (fail roll → no place because outcomeId>0 filter; proceed → hits−=5 then transform).
    // Steel Mining Pick 684 + Ancient Stone Wall H/C/V
    push_tr_alt(db, 684, 896, 0);
    push_tr_alt(db, 684, 895, 0);
    push_tr_alt(db, 684, 897, 0);
    // Steel Adze 462 + Springy Wooden Door H/V
    push_tr_alt(db, 462, 2757, 0);
    push_tr_alt(db, 462, 2759, 0);

    // Haxe: shovel 502 + Stump 338 → Kindling 72
    push_tr_alt(db, 502, 338, 72);
    // Haxe: shovel 502 + Empty Clay Pit 408 → Clay 126 (pushed twice)
    push_tr_alt(db, 502, 408, 126);
    push_tr_alt(db, 502, 408, 126);
    // Haxe: Steel Mining Pick 684 + Bear Cave 650 → Big Charcoal Pile 300
    push_tr_alt(db, 684, 650, 300);
    // Haxe: Shovel 502 + Big Hard Rock 32 → Stone 33
    push_tr_alt(db, 502, 32, 33);
    // Haxe: Steel Mining Pick 684 + Gold Vein 680
    push_tr_alt(db, 684, 680, 0);
    push_tr_alt(db, 684, 680, 33);
    push_tr_alt(db, 684, 680, 33);
    push_tr_alt(db, 684, 680, 291);
    push_tr_alt(db, 684, 680, 681);

    // ── Object-keyed alternative outcomes (trees / mining) ──────────────────
    // Chopped trees: Fire Wood 344 ×3 weight, Butt Log 345 ×1
    // Haxe: L2662–2684
    for tree in [342, 340, 3146] {
        push_obj_alt(db, tree, 344);
        push_obj_alt(db, tree, 344);
        push_obj_alt(db, tree, 344);
        push_obj_alt(db, tree, 345);
    }

    // Cut Stones 1853: Stone / empty / empty
    // Haxe: L2693–2695
    push_obj_alt(db, 1853, 33);
    push_obj_alt(db, 1853, 0);
    push_obj_alt(db, 1853, 0);

    // Iron Vein 3961: Stone / empty / empty
    // Haxe: L2701–2703
    push_obj_alt(db, 3961, 33);
    push_obj_alt(db, 3961, 0);
    push_obj_alt(db, 3961, 0);

    // Shallow Pit with Ore 3956
    // Haxe: L2705–2711
    for _ in 0..4 {
        push_obj_alt(db, 3956, 0);
    }
    push_obj_alt(db, 3956, 33);
    push_obj_alt(db, 3956, 33);
    push_obj_alt(db, 3956, 291);

    // Deep Pit with Ore 3958
    // Haxe: L2713–2719
    for _ in 0..4 {
        push_obj_alt(db, 3958, 0);
    }
    push_obj_alt(db, 3958, 33);
    push_obj_alt(db, 3958, 33);
    push_obj_alt(db, 3958, 291);

    // Mine with Ore 3959
    // Haxe: L2721–2733
    for _ in 0..7 {
        push_obj_alt(db, 3959, 0);
    }
    push_obj_alt(db, 3959, 33);
    push_obj_alt(db, 3959, 33);
    push_obj_alt(db, 3959, 33);
    push_obj_alt(db, 3959, 33);
    push_obj_alt(db, 3959, 33); // Haxe comment says Flat Rock once at 2732 but id 33
    push_obj_alt(db, 3959, 291);
    push_obj_alt(db, 3959, 503); // Dug Big Rock
}

#[cfg(test)]
mod alt_outcome_patch_tests {
    use super::*;

    #[test]
    fn alt_outcome_patches_trees_and_transitions() {
        let mut db = ContentDb::default();
        apply_default_alternative_outcome_patches(&mut db);

        let tree = db.alt_outcomes_object.get(&340).expect("chopped tree");
        assert_eq!(tree.iter().filter(|&&x| x == 344).count(), 3);
        assert_eq!(tree.iter().filter(|&&x| x == 345).count(), 1);

        let clay = db
            .alt_outcomes_transition
            .get(&(502, 408))
            .expect("clay pit");
        assert_eq!(clay, &vec![126, 126]);

        assert_eq!(db.fortification_obj_id.get(&550), Some(&67));
        assert!((db.fortification_value.get(&33).copied().unwrap_or(0.0) - 1.0).abs() < 1e-5);

        // Haxe L2529–2564 push(0) walls/doors — gate relies on non-empty outcomes
        assert_eq!(
            db.alt_outcomes_transition.get(&(684, 896)).map(|v| v.as_slice()),
            Some(&[0][..])
        );
        assert_eq!(
            db.alt_outcomes_transition.get(&(462, 2757)).map(|v| v.as_slice()),
            Some(&[0][..])
        );
        assert_eq!(db.alternative_outcomes_for(684, 896, 887), &[0]);

        // resolve helper: transition list wins over object
        db.alt_outcomes_object.insert(999, vec![1]);
        db.alt_outcomes_transition.insert((1, 2), vec![3, 0]);
        assert_eq!(db.alternative_outcomes_for(1, 2, 999), &[3, 0]);
        assert_eq!(db.alternative_outcomes_for(9, 9, 999), &[1]);
        assert!(db.alternative_outcomes_for(0, 0, 0).is_empty());
    }
}
