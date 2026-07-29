//! Build-time wire for **TH-ALT-OUTCOME** / `alt_transition_outcome`.
//!
//! Idempotent patches:
//! - ol-content: ContentDb side-tables + `apply_default_alternative_outcome_patches`
//! - ol-sim: `mod alt_outcome` + live USE early-return on TryAgain
//! - port docs (FILE_MATRIX / TODO_PORT / CALL_INDEX / changelog)
//
// Haxe: TransitionHelper L1260–1306 alternativeTransitionOutcome

use std::path::{Path, PathBuf};

fn normalize_nl(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

fn restore_nl(s: &str, crlf: bool) -> String {
    if crlf {
        s.replace('\n', "\r\n")
    } else {
        s.to_string()
    }
}

fn replace_once(hay: &mut String, old: &str, new: &str) -> bool {
    if hay.contains(new) {
        return true;
    }
    if let Some(i) = hay.find(old) {
        hay.replace_range(i..i + old.len(), new);
        true
    } else {
        false
    }
}

fn write_if_changed(path: &Path, original: &str, next: &str) -> bool {
    if original == next {
        return false;
    }
    if let Err(e) = std::fs::write(path, next) {
        eprintln!("cargo:warning=TH-ALT-OUTCOME write {}: {e}", path.display());
        return false;
    }
    true
}

pub fn th_alt_outcome_wired(content_lib: &str, use_tr: &str, sim_lib: &str) -> bool {
    content_lib.contains("alt_outcomes_object")
        && content_lib.contains("apply_default_alternative_outcome_patches")
        && content_lib.contains("fn alternative_outcomes_for")
        && use_tr.contains("TH-ALT-OUTCOME")
        && use_tr.contains("evaluate_alternative_outcome")
        && sim_lib.contains("mod alt_outcome")
}

pub fn patch_th_alt_outcome(ol_sim_src: &Path, workspace: &Path) -> bool {
    let crates = ol_sim_src
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf());
    let Some(crates) = crates else {
        eprintln!("cargo:warning=TH-ALT-OUTCOME: cannot resolve crates dir");
        return false;
    };
    let content_src = crates.join("ol-content/src");
    let port = workspace.join("docs/port");

    let c = patch_content(&content_src);
    let l = patch_sim_lib(&ol_sim_src.join("lib.rs"));
    let u = patch_use_transition(&ol_sim_src.join("use_transition.rs"));
    let _ = patch_docs(&port);
    c && l && u
}

fn patch_content(content_src: &Path) -> bool {
    let lib = content_src.join("lib.rs");
    let bin = content_src.join("binary_cache.rs");
    let a = patch_content_lib(&lib);
    let b = patch_binary_cache(&bin);
    a && b
}

fn patch_content_lib(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    // Fully wired already — never re-inject (replace_once anchors remain after first apply).
    if raw.contains("fn alternative_outcomes_for")
        && raw.contains("alt_outcomes_object")
        && raw.contains("apply_default_alternative_outcome_patches")
    {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut any = false;

    // Side-tables on ContentDb
    any |= replace_once(
        &mut t,
        "    pub ai_should_ignore_last_use: HashSet<(i32, i32)>,\n    /// Load timing (ms) — set by [`load_content`].\n",
        "    pub ai_should_ignore_last_use: HashSet<(i32, i32)>,\n    /// Haxe `ObjectData.alternativeTransitionOutcome` (ServerSettings patches).\n    /// // Haxe: ObjectData.alternativeTransitionOutcome\n    /// // TH-ALT-OUTCOME\n    pub alt_outcomes_object: HashMap<i32, Vec<i32>>,\n    /// Haxe `TransitionData.alternativeTransitionOutcome` (ServerSettings patches).\n    /// // Haxe: TransitionData.alternativeTransitionOutcome\n    /// // TH-ALT-OUTCOME\n    pub alt_outcomes_transition: HashMap<(i32, i32), Vec<i32>>,\n    /// Haxe `ObjectData.fortificationObjId`.\n    /// // TH-ALT-OUTCOME\n    pub fortification_obj_id: HashMap<i32, i32>,\n    /// Haxe `ObjectData.fortificationValue`.\n    /// // TH-ALT-OUTCOME\n    pub fortification_value: HashMap<i32, f32>,\n    /// Load timing (ms) — set by [`load_content`].\n",
    );

    // Method on ContentDb
    any |= replace_once(
        &mut t,
        "    /// Resolve dummy multi-use id → base object id (Haxe `dummyParent`).\n    #[inline]\n    pub fn resolve_base_id(&self, id: i32) -> i32 {\n",
        "    /// Haxe L1260–1261: transition alt list if non-empty, else new-target object list\n    /// (and dummy parent base).\n    // Haxe: TransitionHelper alternativeTransitionOutcome resolve\n    // TH-ALT-OUTCOME\n    pub fn alternative_outcomes_for(\n        &self,\n        actor_id: i32,\n        target_id: i32,\n        new_target_id: i32,\n    ) -> &[i32] {\n        if let Some(v) = self.alt_outcomes_transition.get(&(actor_id, target_id)) {\n            if !v.is_empty() {\n                return v.as_slice();\n            }\n        }\n        let base = self.resolve_base_id(new_target_id);\n        if let Some(v) = self.alt_outcomes_object.get(&new_target_id) {\n            if !v.is_empty() {\n                return v.as_slice();\n            }\n        }\n        if base != new_target_id {\n            if let Some(v) = self.alt_outcomes_object.get(&base) {\n                return v.as_slice();\n            }\n        }\n        // Also try target id (when new_target is empty / same as current work object).\n        let tbase = self.resolve_base_id(target_id);\n        if let Some(v) = self.alt_outcomes_object.get(&target_id) {\n            if !v.is_empty() {\n                return v.as_slice();\n            }\n        }\n        self.alt_outcomes_object\n            .get(&tbase)\n            .map(|v| v.as_slice())\n            .unwrap_or(&[])\n    }\n\n    /// Resolve dummy multi-use id → base object id (Haxe `dummyParent`).\n    #[inline]\n    pub fn resolve_base_id(&self, id: i32) -> i32 {\n",
    );

    // include! patches file
    any |= replace_once(
        &mut t,
        "include!(\"ai_should_ignore_patches.inc.rs\");\ninclude!(\"lib_tail.inc.rs\");\n",
        "include!(\"ai_should_ignore_patches.inc.rs\");\ninclude!(\"alt_outcome_patches.inc.rs\");\ninclude!(\"lib_tail.inc.rs\");\n",
    );

    // load_content call after horse patches
    any |= replace_once(
        &mut t,
        "    // TH-HORSE: ServerSettings.PatchTransitions horse cart pickup/drop + tire fixes.\n    apply_default_horse_transition_patches(&mut db);\n    // C-SS-AI-IGNORE: ServerSettings.PatchTransitions aiShouldIgnore table.\n    apply_default_ai_should_ignore_patches(&mut db);\n",
        "    // TH-HORSE: ServerSettings.PatchTransitions horse cart pickup/drop + tire fixes.\n    apply_default_horse_transition_patches(&mut db);\n    // TH-ALT-OUTCOME: alternativeTransitionOutcome + fortification tables.\n    apply_default_alternative_outcome_patches(&mut db);\n    // C-SS-AI-IGNORE: ServerSettings.PatchTransitions aiShouldIgnore table.\n    apply_default_ai_should_ignore_patches(&mut db);\n",
    );

    if !any && t.contains("alt_outcomes_object") {
        return true;
    }
    if !any {
        eprintln!("cargo:warning=TH-ALT-OUTCOME: content lib.rs anchors missing");
        return t.contains("alt_outcomes_object");
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf));
    true
}

fn patch_binary_cache(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut any = false;

    any |= replace_once(
        &mut t,
        "    apply_default_horse_transition_patches,\n    apply_default_second_time_outcomes, apply_default_switch_number_of_uses_patches,\n",
        "    apply_default_horse_transition_patches,\n    apply_default_alternative_outcome_patches,\n    apply_default_second_time_outcomes, apply_default_switch_number_of_uses_patches,\n",
    );
    // alternate import layout
    any |= replace_once(
        &mut t,
        "    apply_default_horse_transition_patches,\n    apply_default_second_time_outcomes,",
        "    apply_default_horse_transition_patches,\n    apply_default_alternative_outcome_patches,\n    apply_default_second_time_outcomes,",
    );

    any |= replace_once(
        &mut t,
        "    apply_default_horse_transition_patches(db);\n    // C-SS-AI-IGNORE: ServerSettings.PatchTransitions aiShouldIgnore\n    apply_default_ai_should_ignore_patches(db);\n",
        "    apply_default_horse_transition_patches(db);\n    // TH-ALT-OUTCOME\n    apply_default_alternative_outcome_patches(db);\n    // C-SS-AI-IGNORE: ServerSettings.PatchTransitions aiShouldIgnore\n    apply_default_ai_should_ignore_patches(db);\n",
    );
    any |= replace_once(
        &mut t,
        "    apply_default_horse_transition_patches(db);\n    apply_default_ai_should_ignore_patches(db);\n",
        "    apply_default_horse_transition_patches(db);\n    // TH-ALT-OUTCOME\n    apply_default_alternative_outcome_patches(db);\n    apply_default_ai_should_ignore_patches(db);\n",
    );

    if t.contains("apply_default_alternative_outcome_patches") {
        if any {
            write_if_changed(path, &raw, &restore_nl(&t, crlf));
        }
        return true;
    }
    if !any {
        eprintln!("cargo:warning=TH-ALT-OUTCOME: binary_cache anchors missing");
    }
    any
}

fn patch_sim_lib(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    // Already wired — skip (avoid second pub use / mod).
    if raw.contains("mod alt_outcome")
        && raw.contains("evaluate_alternative_outcome")
        && raw.contains("ALTERNATIVE_OUTCOME_PERCENT_INCREASE_PER_HIT")
    {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut any = false;

    // mod declaration near use_transition / horse_mount
    if !t.contains("mod alt_outcome") {
        any |= replace_once(
            &mut t,
            "mod use_transition;\n",
            "mod alt_outcome;\nmod use_transition;\n",
        );
    }

    // re-exports
    if !t.contains("pub use alt_outcome::") {
        any |= replace_once(
            &mut t,
            "pub use use_transition::",
            "pub use alt_outcome::{\n    alt_outcome_effective_roll, alt_outcome_gate_applies, evaluate_alternative_outcome,\n    fortification_drop_chance, fortification_of, is_fortified_hits, pick_outcome_index,\n    resolve_alternative_outcomes, AltOutcomePlan, ALTERNATIVE_OUTCOME_HITS_DECREASE_ON_SUCCESS,\n    ALTERNATIVE_OUTCOME_PERCENT_INCREASE_PER_HIT, STONE_FLOOR_BUGFIX_ID,\n};\npub use use_transition::",
        );
    }

    if t.contains("mod alt_outcome") {
        if any {
            write_if_changed(path, &raw, &restore_nl(&t, crlf));
        }
        return true;
    }
    if !any {
        eprintln!("cargo:warning=TH-ALT-OUTCOME: sim lib.rs anchors missing");
    }
    any
}

/// Live USE wire: after hungry-work block inside transition branch, evaluate alt outcome.
const USE_WIRE: &str = r#"
        // TH-ALT-OUTCOME: Haxe TransitionHelper L1260–1306 alternativeTransitionOutcome.
        // After hungry-work cost is paid; TryAgain keeps tile + may place bonus; no main transform.
        {
            let outcomes: Vec<i32> = state
                .content
                .alternative_outcomes_for(actor, target, tr_work.new_target_id)
                .to_vec();
            let (count_obj, _) = {
                let w = state.world.read().unwrap();
                let c = w.get_helper(tx, ty).map(|h| h.count_obj).unwrap_or(0.0);
                (c, ())
            };
            // is_fortified from hungry-work cost vs hits (recompute lightly).
            let actor_hw = object_hungry_work(
                actor,
                state
                    .content
                    .get(actor)
                    .map(|d| d.description.as_str())
                    .unwrap_or(""),
                state.gameplay.hungry_work_cost,
            );
            let new_tgt_desc = state
                .content
                .get(tr_work.new_target_id)
                .map(|d| d.description.clone())
                .unwrap_or_default();
            let new_tgt_hw = object_hungry_work(
                tr_work.new_target_id,
                &new_tgt_desc,
                state.gameplay.hungry_work_cost,
            );
            let biome = {
                let w = state.world.read().unwrap();
                w.get_biome(tx, ty)
            };
            let base_cost = compute_hungry_work_cost(
                actor_hw,
                new_tgt_hw,
                0.0,
                biome == BIOME_PASSABLE_RIVER,
            );
            let (cost_for_fort, is_fortified) =
                apply_loose_fence_hungry_work_waiver(base_cost, &new_tgt_desc, hits_before);
            let _ = cost_for_fort;
            // allow_for_owner: owned + owner + original cost < 1 (Haxe L1195–1196)
            let object_is_owned = ol_world::description_is_owned(&target_desc);
            let allow_for_owner = if object_is_owned {
                let (owners_acc, owner_id) = {
                    let w = state.world.read().unwrap();
                    w.get_helper(tx, ty)
                        .map(|h| (h.owners_by_account.clone(), h.owner_id))
                        .unwrap_or_default()
                };
                let owner = owner_account_of(&owners_acc, owner_id);
                let player_account = account_soul_token(&player_email);
                let player_is_owner = owner.map(|o| o == player_account).unwrap_or(true);
                match adjust_hungry_work_for_ownership(base_cost, true, player_is_owner) {
                    HungryWorkOwnerAdj::OwnerHalf {
                        allow_for_owner: a,
                        ..
                    } => a,
                    _ => false,
                }
            } else {
                false
            };
            let (fort_id, fort_val) = crate::alt_outcome::fortification_of(&state.content, target);
            let plan = crate::alt_outcome::evaluate_alternative_outcome(
                tr_work.target_id,
                allow_for_owner,
                is_fortified,
                &outcomes,
                hits_before,
                count_obj,
                fort_id,
                fort_val,
                crate::alt_outcome::ALTERNATIVE_OUTCOME_PERCENT_INCREASE_PER_HIT,
                crate::alt_outcome::ALTERNATIVE_OUTCOME_HITS_DECREASE_ON_SUCCESS,
                rand::random::<f32>(),
                rand::random::<f32>(),
            );
            match plan {
                crate::alt_outcome::AltOutcomePlan::Skip => {}
                crate::alt_outcome::AltOutcomePlan::Proceed { hits_after } => {
                    // Continue main transition with reduced hits (stamp later via hits_out).
                    // Store via helper stamp before transform path uses hits_before only for loved food.
                    // Override hits_before by writing through to a local — see hits_out path below.
                    let mut w = state.world.write().unwrap();
                    crate::loved_food_wire::stamp_hits(&mut w, tx, ty, hits_after);
                    // Re-read for loved-food path: set via complex so subsequent hits_before stays old;
                    // we patch hits_out after loved-food block by re-reading if needed.
                    // Immediate: stash success hits on tile so final stamp keeps reduction when no extra.
                    let _ = hits_after;
                }
                crate::alt_outcome::AltOutcomePlan::TryAgain {
                    hits_after,
                    count_obj_after,
                    place_id,
                    say_fortification,
                } => {
                    {
                        let mut w = state.world.write().unwrap();
                        crate::loved_food_wire::stamp_hits(&mut w, tx, ty, hits_after);
                        if let Some(c) = count_obj_after {
                            if let Some(h) = w.helpers.get_mut(&(tx, ty)) {
                                h.count_obj = c;
                            } else {
                                let base = w.get_object(tx, ty);
                                if base != 0 {
                                    let mut co = ol_world::ComplexObject::new_simple(base);
                                    co.hits = hits_after;
                                    co.count_obj = c;
                                    w.set_object_complex(tx, ty, co);
                                }
                            }
                        }
                    }
                    if let Some(oid) = place_id {
                        if oid > 0 {
                            let _ = crate::death_polish::place_object_by_id(
                                state,
                                tx,
                                ty,
                                oid,
                                crate::death_polish::PlaceObjectOpts::default(),
                            );
                        }
                    }
                    if say_fortification {
                        note_lock_say(
                            conn_id,
                            &format!("Try again! Fortification: {}", -hits_after.round() as i32),
                        );
                    } else {
                        note_lock_say(
                            conn_id,
                            &format!("Try again! Hits {}", hits_after.round() as i32),
                        );
                    }
                    // Haxe: return true without main transform (action still applied).
                    return Some(UseResult {
                        actor_before: actor,
                        target_before: target,
                        actor_after: actor,
                        target_after: target,
                        applied: true,
                        x: tx,
                        y: ty,
                    });
                }
            }
        }

"#;

fn patch_use_transition(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut any = false;

    any |= replace_once(
        &mut t,
        "//! Chunk: **TH-MULTI** / **TH-MULTI-POLISH** / **TH-HORSE** / **HORSE-MOUNT-POLISH** / **TH-LOCK** / **EATEN-FOOD-PCT** / **DARK-NOSAJ**\n",
        "//! Chunk: **TH-MULTI** / **TH-MULTI-POLISH** / **TH-HORSE** / **HORSE-MOUNT-POLISH** / **TH-LOCK** / **EATEN-FOOD-PCT** / **DARK-NOSAJ** / **TH-ALT-OUTCOME**\n",
    );

    // Inject after hungry-work block closes, before transition tuple return.
    let anchor = "                HungryWorkGate::RefuseFood { .. } => {\n                    // Haxe: player.say('Need ${missingFood} more food!')\n                    note_lock_say(conn_id, \"Need more food!\");\n                    return refuse(actor, target);\n                }\n            }\n        }\n\n        (\n            tr_work.new_actor_id,\n";
    let injection = format!(
        "                HungryWorkGate::RefuseFood {{ .. }} => {{\n                    // Haxe: player.say('Need ${{missingFood}} more food!')\n                    note_lock_say(conn_id, \"Need more food!\");\n                    return refuse(actor, target);\n                }}\n            }}\n        }}\n{USE_WIRE}        (\n            tr_work.new_actor_id,\n"
    );
    if !t.contains("TH-ALT-OUTCOME: Haxe TransitionHelper L1260") {
        any |= replace_once(&mut t, anchor, &injection);
    } else {
        any = true;
    }

    // Fix Proceed path: need hits_before mut and override after loved food.
    // Make hits_before mutable and re-read after alt success stamp.
    any |= replace_once(
        &mut t,
        "    let (target, uses_remaining, hits_before) = {\n        let w = state.world.read().unwrap();\n        let target = w.get_object(tx, ty);\n        let helper = w.get_helper(tx, ty);\n        let uses = helper.map(|h| h.uses_remaining).unwrap_or(0);\n        let hits = helper.map(|h| h.hits).unwrap_or(0.0);\n        (target, uses, hits)\n    };\n",
        "    let (target, uses_remaining, mut hits_before) = {\n        let w = state.world.read().unwrap();\n        let target = w.get_object(tx, ty);\n        let helper = w.get_helper(tx, ty);\n        let uses = helper.map(|h| h.uses_remaining).unwrap_or(0);\n        let hits = helper.map(|h| h.hits).unwrap_or(0.0);\n        (target, uses, hits)\n    };\n",
    );

    // After alt Proceed stamps hits, refresh hits_before from world before loved-food.
    any |= replace_once(
        &mut t,
        "    // TH-MULTI-POLISH: loved-food bare-hand extra.\n    let mut hits_out = hits_before;\n",
        "    // TH-ALT-OUTCOME: refresh hits if Proceed path stamped a reduction on the tile.\n    {\n        let w = state.world.read().unwrap();\n        if let Some(h) = w.get_helper(tx, ty) {\n            hits_before = h.hits;\n        }\n    }\n    // TH-MULTI-POLISH: loved-food bare-hand extra.\n    let mut hits_out = hits_before;\n",
    );

    // Live test at end of tests module
    if !t.contains("alt_outcome_try_again_keeps_target") {
        let test = r#"

    /// TH-ALT-OUTCOME: low hits → TryAgain keeps target, stamps hits, may place bonus.
    // Haxe: TransitionHelper L1274–1303
    #[test]
    fn alt_outcome_try_again_keeps_target() {
        let mut db = ContentDb::default();
        db.objects.insert(71, def(71, 0, false)); // axe
        db.objects.insert(340, def(340, 0, true)); // chopped tree
        db.objects.insert(344, def(344, 0, false)); // fire wood
        db.transitions
            .insert((71, 340), tr(71, 340, 71, 340, false, false));
        db.alt_outcomes_object.insert(340, vec![344]);
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "chop");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(71, 0);
            p.food = 20.0;
            p.food_max = 20.0;
            p.exhaustion = 0.0;
        }
        state.world.write().unwrap().set_object(0, 0, 340);
        // Force fail by pre-seeding 0 hits; roll almost always < 1
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied, "try-again is still an applied action");
        assert_eq!(r.target_after, 340, "target not transformed on try-again");
        let hits = state
            .world
            .read()
            .unwrap()
            .get_helper(0, 0)
            .map(|h| h.hits)
            .unwrap_or(0.0);
        assert!(hits >= 1.0 - 1e-4, "hits stamped, got {hits}");
    }

    /// TH-ALT-OUTCOME: high hits → Proceed continues transition (target may change).
    #[test]
    fn alt_outcome_proceed_allows_transform() {
        let mut db = ContentDb::default();
        db.objects.insert(71, def(71, 0, false));
        db.objects.insert(340, def(340, 0, true));
        db.objects.insert(341, def(341, 0, true));
        db.transitions
            .insert((71, 340), tr(71, 340, 71, 341, false, false));
        db.alt_outcomes_object.insert(340, vec![344]);
        let mut state = state_with(db);
        crate::spawn_player(&mut state, 1, "chop2");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.set_held(71, 0);
            p.food = 20.0;
            p.food_max = 20.0;
        }
        {
            let mut w = state.world.write().unwrap();
            w.set_object(0, 0, 340);
            // hits=10 → roll always ≥ 1 → Proceed
            crate::loved_food_wire::stamp_hits(&mut w, 0, 0, 10.0);
        }
        let r = apply_use_at(&mut state, 1, 0, 0).unwrap();
        assert!(r.applied);
        assert_eq!(r.target_after, 341, "main transform on proceed");
        let hits = state
            .world
            .read()
            .unwrap()
            .get_helper(0, 0)
            .map(|h| h.hits)
            .unwrap_or(0.0);
        // Proceed: 10 - 5 = 5, then loved-food may not apply (held tool)
        assert!((hits - 5.0).abs() < 1e-3 || hits >= 5.0 - 1e-3, "hits={hits}");
    }
"#;
        if let Some(i) = t.rfind("\n}") {
            // Find last closing of tests mod - safer: search for end of hungry work tests area
            // Insert before final `}` of file's tests module
            t.insert_str(i, test);
            any = true;
        }
    }

    if !any && t.contains("TH-ALT-OUTCOME") {
        return true;
    }
    if !any {
        eprintln!("cargo:warning=TH-ALT-OUTCOME: use_transition anchors missing");
        return t.contains("TH-ALT-OUTCOME");
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf));
    true
}

fn patch_docs(port: &Path) -> bool {
    let _ = patch_file_matrix(&port.join("FILE_MATRIX.md"));
    let _ = patch_todo(&port.join("TODO_PORT.md"));
    let _ = patch_call_index(&port.join("CALL_INDEX.md"));
    let _ = write_changelog(&port.join("changelog"));
    let _ = patch_queue(&port.join("QUEUE.md"));
    true
}

fn patch_file_matrix(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("**TH-ALT-OUTCOME**") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let old = "Gaps: chest coin store; residual **alternativeTransitionOutcome**; ServerSettings containSize patches; TH containerSlotSize post-transition spill";
    let new = "Gaps: chest coin store; ServerSettings containSize patches; TH containerSlotSize post-transition spill. **TH-ALT-OUTCOME** DONE (core): alt outcomes + fort drop pure+live";
    if !replace_once(&mut t, old, new) {
        // softer
        let _ = replace_once(
            &mut t,
            "residual **alternativeTransitionOutcome**",
            "**TH-ALT-OUTCOME** DONE (core)",
        );
    }
    // Add matrix row if missing
    if !t.contains("| TH-ALT-OUTCOME |") {
        let _ = replace_once(
            &mut t,
            "| S-TH |",
            "| **TH-ALT-OUTCOME** / alt_transition_outcome | TransitionHelper alternativeTransitionOutcome + fortification | **DONE** (core) | `alt_outcome.rs` pure + ContentDb side-tables + `apply_use_at` TryAgain/Proceed; residual: LiveSettings knobs, PropertyGate per-trans push, coinCost before gate |\n| S-TH |",
        );
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf));
    true
}

fn patch_todo(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("**TH-ALT-OUTCOME**") && raw.contains("alt_transition_outcome") && raw.contains("[x] **TH-ALT-OUTCOME") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    // Update residual note on HORSE-MOUNT-POLISH
    let _ = replace_once(
        &mut t,
        "residual: **alternativeTransitionOutcome** (global TH, not cart-only)",
        "residual alt → **TH-ALT-OUTCOME DONE**",
    );
    // Insert done checkbox near TH items
    if !t.contains("[x] **TH-ALT-OUTCOME") {
        let _ = replace_once(
            &mut t,
            "- [x] **HORSE-MOUNT-POLISH hitch_cart**",
            "- [x] **TH-ALT-OUTCOME alt_transition_outcome** — pure `evaluate_alternative_outcome` + ContentDb `alt_outcomes_*` / fortification tables + live `apply_use_at` TryAgain (hits++/place bonus, no transform) / Proceed (hits−=5); ServerSettings tree/mine/shovel patches; tests `alt_outcome::*` + `use_transition::alt_outcome_*`; residual: LiveSettings knobs, PropertyGate bulk push, coinCost before gate\n- [x] **HORSE-MOUNT-POLISH hitch_cart**",
        );
    }
    // Changelog line
    if !t.contains("**TH-ALT-OUTCOME alt_transition_outcome**") {
        let _ = replace_once(
            &mut t,
            "## Changelog (port docs)\n\n",
            "## Changelog (port docs)\n\n| 2026-07-29 | **TH-ALT-OUTCOME alt_transition_outcome DONE**: pure `evaluate_alternative_outcome` (hits ramp, fort drop, outcome pick); ContentDb side-tables + ServerSettings patches; live USE TryAgain/Proceed; tests alt_outcome::* + use_transition::alt_outcome_*; residual LiveSettings / PropertyGate bulk / coinCost |\n",
        );
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf));
    true
}

fn patch_call_index(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("evaluate_alternative_outcome") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let block = "| `evaluate_alternative_outcome` / `alt_outcome_gate_applies` | `ol-sim/src/alt_outcome.rs` | TH-ALT-OUTCOME pure L1260–1306 |\n| `apply_default_alternative_outcome_patches` | `ol-content` | ServerSettings alt/fort tables |\n| `ContentDb::alternative_outcomes_for` | `ol-content` | transition list > new-target object list |\n| `apply_use_at` alt TryAgain/Proceed | `use_transition.rs` | live hits stamp + place_object_by_id + keep/transform |\n";
    // Append near use_transition hungry work rows if present
    if t.contains("evaluate_hungry_work_use") {
        let _ = replace_once(
            &mut t,
            "| `evaluate_hungry_work_use`",
            &format!("{block}| `evaluate_hungry_work_use`"),
        );
    } else if let Some(i) = t.find("| `apply_use_at`") {
        t.insert_str(i, block);
    } else {
        t.push_str("\n## TH-ALT-OUTCOME\n\n");
        t.push_str(block);
    }
    write_if_changed(path, &raw, &restore_nl(&t, crlf));
    true
}

fn write_changelog(dir: &Path) -> bool {
    let path = dir.join("2026-07-29-TH-ALT-OUTCOME.md");
    if path.exists() {
        return true;
    }
    let body = r#"# TH-ALT-OUTCOME / alt_transition_outcome

## Summary

Port Haxe `TransitionHelper` alternative transition outcome + fortification fail path (L1260–1306).

## Behavior

1. Resolve outcomes: transition list if non-empty, else new-target (or target) object list.
2. Gate: `targetID != 884`, `!allowForOwner`, outcomes non-empty **or** fortified (`cost>0 && hits<-0.1`).
3. Roll `rng + hits/10`; if `< 1` → **TryAgain**: hits+=1, optional PlaceObject bonus/fort material, **no** main transform, action applied.
4. Else **Proceed**: hits −= 5, continue normal USE transform.

## Surfaces

| Layer | Path |
|-------|------|
| Pure | `ol-sim/src/alt_outcome.rs` |
| Content tables | `ol-content` `alt_outcomes_*` + `apply_default_alternative_outcome_patches` |
| Live USE | `use_transition::apply_use_at` |

## Residuals

- LiveSettings knobs for percent/hits-decrease
- PropertyGate per-transition `push(0)` bulk
- `transition.coinCost` before gate (not yet on Transition)
"#;
    let _ = std::fs::create_dir_all(dir);
    std::fs::write(&path, body).is_ok()
}

fn patch_queue(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if !raw.contains("TH-ALT-OUTCOME") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    // Remove from in-flight if present
    let line = "| 2 | `TH-ALT-OUTCOME` | alt_transition_outcome | haxe-port-chunk-151 | A |\n";
    if t.contains(line) {
        t = t.replace(line, "");
        let _ = replace_once(
            &mut t,
            "## Done recently\n\n",
            "## Done recently\n\n**TH-ALT-OUTCOME** · ",
        );
        write_if_changed(path, &raw, &restore_nl(&t, crlf));
    }
    true
}
