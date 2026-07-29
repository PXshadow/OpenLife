//! AI-FARM-STICKY / basic_farmer_live — pure Rust idempotent source patches.
//!
//! Live `Player.farm_profession` BASICFARMER weight read/write on profession scan tick.
//! Haxe: AiBase.doBasicFarming ~2400 / ~2415 profession['BASICFARMER'] = 1 / 0

use std::path::Path;
use std::process::Command;

pub fn already_wired(src: &Path) -> bool {
    let farmer = std::fs::read_to_string(src.join("farmer_profession.rs")).unwrap_or_default();
    let scan = std::fs::read_to_string(src.join("profession_scan.rs")).unwrap_or_default();
    let lib = std::fs::read_to_string(src.join("lib.rs")).unwrap_or_default();
    farmer.contains("fn basic_farmer_weight_from_runtime")
        && lib.contains("basic_farmer_weight_from_runtime")
        && scan.contains("AI-FARM-STICKY")
        && scan.contains("p.farm_profession = farm_rt")
        && scan.contains("farm_rt: &mut FarmProfessionRuntime")
}

fn replace_once(hay: &mut String, old: &str, new: &str) -> bool {
    if hay.contains(new.split('\n').next().unwrap_or(new)) && new.len() > 40 {
        // already applied if distinctive fragment present — still try exact
    }
    if let Some(i) = hay.find(old) {
        hay.replace_range(i..i + old.len(), new);
        true
    } else {
        false
    }
}

fn replace_all(hay: &mut String, old: &str, new: &str) -> usize {
    let n = hay.matches(old).count();
    if n > 0 {
        *hay = hay.replace(old, new);
    }
    n
}

pub fn patch_all(src: &Path, workspace: &Path) -> bool {
    // Prefer Python apply (full fidelity: tests + npc).
    let py = src.join("_apply_ai_farm_sticky_all.py");
    if py.exists() {
        let status = Command::new("python")
            .arg(&py)
            .current_dir(src)
            .status()
            .or_else(|_| Command::new("python3").arg(&py).current_dir(src).status());
        if let Ok(s) = status {
            if s.success() && already_wired(src) {
                let _ = patch_docs_light(workspace);
                return true;
            }
        }
    }
    // Fallback pure Rust core (farmer + lib + profession_scan live paths).
    let mut ok = true;
    ok &= patch_farmer(src);
    ok &= patch_lib(src);
    ok &= patch_profession_scan(src);
    ok &= patch_make_stuff_live(src);
    ok &= patch_tests_light(src);
    ok &= patch_npc(src);
    if ok {
        let _ = patch_docs_light(workspace);
    }
    already_wired(src) || ok
}

fn patch_farmer(src: &Path) -> bool {
    let path = src.join("farmer_profession.rs");
    let Ok(mut t) = std::fs::read_to_string(&path) else {
        return false;
    };
    if t.contains("fn basic_farmer_weight_from_runtime") {
        return true;
    }
    let old = r#"pub fn apply_basic_farmer_weight_side_effect(
    runtime: &mut FarmProfessionRuntime,
    action: FarmAction,
) {
    if let Some(w) = action.basic_farmer_weight_side_effect() {
        runtime.weights.insert(FarmProfession::BasicFarmer, w);
    }
}
"#;
    let new = r#"pub fn apply_basic_farmer_weight_side_effect(
    runtime: &mut FarmProfessionRuntime,
    action: FarmAction,
) {
    if let Some(w) = action.basic_farmer_weight_side_effect() {
        runtime.weights.insert(FarmProfession::BasicFarmer, w);
    }
}

/// Read Haxe `profession['BASICFARMER']` sticky weight (default 1.0 when unset).
// Haxe: profession map lookup in doPlantBushes / doBasicFarming
// AI-FARM-STICKY: live scan reads this into ProfessionScanInput.basic_farmer_weight
pub fn basic_farmer_weight_from_runtime(runtime: &FarmProfessionRuntime) -> f32 {
    runtime
        .weights
        .get(&FarmProfession::BasicFarmer)
        .copied()
        .unwrap_or(1.0)
}
"#;
    if !replace_once(&mut t, old, new) {
        eprintln!("build_ai_farm_sticky: farmer helper insert failed");
        return false;
    }
    // Test
    if !t.contains("fn basic_farmer_weight_from_runtime_default_and_sticky") {
        let marker = r#"        assert_eq!(rt.weights.get(&FarmProfession::BasicFarmer), Some(&1.0));
    }

    #[test]
    fn do_basic_farming_after_sheep_late_plants_sharpie_advanced()"#;
        let insert = r#"        assert_eq!(rt.weights.get(&FarmProfession::BasicFarmer), Some(&1.0));
    }

    #[test]
    fn basic_farmer_weight_from_runtime_default_and_sticky() {
        let rt = FarmProfessionRuntime::default();
        assert_eq!(basic_farmer_weight_from_runtime(&rt), 1.0);
        let mut rt = FarmProfessionRuntime::default();
        apply_basic_farmer_weight_side_effect(
            &mut rt,
            FarmAction::DeferSheepHerding {
                max_profession: 1,
            },
        );
        assert_eq!(basic_farmer_weight_from_runtime(&rt), 1.0);
        apply_basic_farmer_weight_side_effect(&mut rt, FarmAction::ClearBasicFarmerWeight);
        assert_eq!(basic_farmer_weight_from_runtime(&rt), 0.0);
        assert_eq!(rt.weights.get(&FarmProfession::BasicFarmer), Some(&0.0));
    }

    #[test]
    fn do_basic_farming_after_sheep_late_plants_sharpie_advanced()"#;
        let _ = replace_once(&mut t, marker, insert);
    }
    std::fs::write(&path, t).is_ok()
}

fn patch_lib(src: &Path) -> bool {
    let path = src.join("lib.rs");
    let Ok(mut t) = std::fs::read_to_string(&path) else {
        return false;
    };
    if t.contains("basic_farmer_weight_from_runtime") {
        return true;
    }
    let ok = replace_once(
        &mut t,
        "apply_basic_farmer_weight_side_effect, default_wet_from_bowl, do_advanced_farming,",
        "apply_basic_farmer_weight_side_effect, basic_farmer_weight_from_runtime, default_wet_from_bowl, do_advanced_farming,",
    );
    if !ok {
        eprintln!("build_ai_farm_sticky: lib export failed");
        return false;
    }
    std::fs::write(&path, t).is_ok()
}

fn patch_profession_scan(src: &Path) -> bool {
    let path = src.join("profession_scan.rs");
    let Ok(mut t) = std::fs::read_to_string(&path) else {
        return false;
    };
    if t.contains("p.farm_profession = farm_rt") && t.contains("basic_farmer_weight_from_runtime")
    {
        // may be partial; continue if farm_action still missing farm_rt
        if t.contains("farm_rt: &mut FarmProfessionRuntime,\n) -> ProfessionScanTickResult {\n    // Haxe: sticky")
            || t.contains("apply_basic_farmer_weight_side_effect(farm_rt, action)")
        {
            return true;
        }
    }

    // imports
    if !t.contains("basic_farmer_weight_from_runtime") {
        let _ = replace_once(
            &mut t,
            "use crate::farmer_profession::{\n    expand_advanced_farming_or_clear, fill_farm_counts_from_map_with_floor, make_sharpie_food,\n    short_craft_apply_resolved, try_decide_farm_from_rung, FarmAction, FarmCounts, FarmMapObj,\n    FarmProfession, FarmProfessionRuntime, FarmTaskState, ShortCraftApply, ShortCraftInput,\n    FARM_COUNT_RADIUS, FARM_HOME_RADIUS, FARM_SHORTCRAFT_RADIUS, SKEWER,\n};",
            "use crate::farmer_profession::{\n    apply_basic_farmer_weight_side_effect, basic_farmer_weight_from_runtime,\n    expand_advanced_farming_or_clear, fill_farm_counts_from_map_with_floor, make_sharpie_food,\n    short_craft_apply_resolved, try_decide_farm_from_rung, FarmAction, FarmCounts, FarmMapObj,\n    FarmProfession, FarmProfessionRuntime, FarmTaskState, ShortCraftApply, ShortCraftInput,\n    FARM_COUNT_RADIUS, FARM_HOME_RADIUS, FARM_SHORTCRAFT_RADIUS, SKEWER,\n};",
        );
    }

    // farm_profession_scan_tick
    if !t.contains("farm_rt: &mut FarmProfessionRuntime,\n) -> ProfessionScanTickResult {\n    let counts = farm_counts_from_scan") {
        let _ = replace_once(
            &mut t,
            "    task: &mut FarmTaskState,\n    has_profession: bool,\n) -> ProfessionScanTickResult {\n    let counts = farm_counts_from_scan(\n        tiles,\n        inp.home_x,\n        inp.home_y,\n        inp.held_id,\n        FARM_COUNT_RADIUS,\n        inp.is_hungry,\n        inp.basic_farmer_weight,\n        inp.hardened_row_biome,\n    );\n    let Some(action) =\n        try_decide_farm_from_rung(job, rung_label, &counts, task, has_profession)\n    else {\n        return ProfessionScanTickResult::none();\n    };\n    farm_action_to_live_intent(tiles, inp, action)\n}",
            "    task: &mut FarmTaskState,\n    has_profession: bool,\n    farm_rt: &mut FarmProfessionRuntime,\n) -> ProfessionScanTickResult {\n    let counts = farm_counts_from_scan(\n        tiles,\n        inp.home_x,\n        inp.home_y,\n        inp.held_id,\n        FARM_COUNT_RADIUS,\n        inp.is_hungry,\n        inp.basic_farmer_weight,\n        inp.hardened_row_biome,\n    );\n    let Some(action) =\n        try_decide_farm_from_rung(job, rung_label, &counts, task, has_profession)\n    else {\n        return ProfessionScanTickResult::none();\n    };\n    farm_action_to_live_intent(tiles, inp, action, farm_rt)\n}",
        );
    }

    // farm_action_to_live_intent signature + apply side effect + recursive
    if !t.contains("apply_basic_farmer_weight_side_effect(farm_rt, action)") {
        let _ = replace_once(
            &mut t,
            "/// Map a decided [`FarmAction`] through shortCraft apply + spatial ctx → intent.\npub fn farm_action_to_live_intent(\n    tiles: &[ScanTile],\n    inp: &ProfessionScanInput,\n    action: FarmAction,\n) -> ProfessionScanTickResult {\n    match action {",
            "/// Map a decided [`FarmAction`] through shortCraft apply + spatial ctx → intent.\n///\n/// AI-FARM-STICKY: applies Haxe `profession['BASICFARMER']` writes onto `farm_rt`.\n// Haxe: AiBase.doBasicFarming ~2400 / ~2415\npub fn farm_action_to_live_intent(\n    tiles: &[ScanTile],\n    inp: &ProfessionScanInput,\n    action: FarmAction,\n    farm_rt: &mut FarmProfessionRuntime,\n) -> ProfessionScanTickResult {\n    // AI-FARM-STICKY: live Player.farm_profession.weights[BASICFARMER]\n    apply_basic_farmer_weight_side_effect(farm_rt, action);\n    match action {",
        );
        // recursive calls
        replace_all(
            &mut t,
            "other => farm_action_to_live_intent(tiles, inp, other),\n            }\n        }\n        // Haxe: doAdvancedFarming(max) then profession['BASICFARMER']=0",
            "other => farm_action_to_live_intent(tiles, inp, other, farm_rt),\n            }\n        }\n        // Haxe: doAdvancedFarming(max) then profession['BASICFARMER']=0",
        );
        // Clear path in DeferAdvanced
        let _ = replace_once(
            &mut t,
            "            match next {\n                FarmAction::ClearBasicFarmerWeight | FarmAction::None | FarmAction::Abort => {\n                    ProfessionScanTickResult::none()\n                }\n                FarmAction::DeferAdvancedFarming { .. } | FarmAction::DeferSheepHerding { .. } => {\n                    ProfessionScanTickResult::none()\n                }\n                other => farm_action_to_live_intent(tiles, inp, other),\n            }",
            "            match next {\n                FarmAction::ClearBasicFarmerWeight | FarmAction::None | FarmAction::Abort => {\n                    apply_basic_farmer_weight_side_effect(farm_rt, next);\n                    ProfessionScanTickResult::none()\n                }\n                FarmAction::DeferAdvancedFarming { .. } | FarmAction::DeferSheepHerding { .. } => {\n                    ProfessionScanTickResult::none()\n                }\n                other => farm_action_to_live_intent(tiles, inp, other, farm_rt),\n            }",
        );
        // Comment update for sheep
        let _ = replace_once(
            &mut t,
            "            // Haxe: this.profession['BASICFARMER']=1 immediately before isSheepHerding\n            // (sticky write applied by apply_profession_scan when runtime present).",
            "            // Haxe: this.profession['BASICFARMER']=1 immediately before isSheepHerding\n            // (applied above via apply_basic_farmer_weight_side_effect).",
        );
    }

    // profession_scan_tick Farm arm + farm_rt param
    if !t.contains("farm_has_profession,\n            farm_rt,\n        ),") {
        let _ = replace_once(
            &mut t,
            "    farm_task: &mut FarmTaskState,\n    farm_has_profession: bool,\n    smith_rt: &mut SmithProfessionRuntime,",
            "    farm_task: &mut FarmTaskState,\n    farm_has_profession: bool,\n    farm_rt: &mut FarmProfessionRuntime,\n    smith_rt: &mut SmithProfessionRuntime,",
        );
        let _ = replace_once(
            &mut t,
            "        ProfessionScanKind::Farm => farm_profession_scan_tick(\n            tiles,\n            inp,\n            farm_job,\n            rung_label,\n            farm_task,\n            farm_has_profession,\n        ),",
            "        ProfessionScanKind::Farm => farm_profession_scan_tick(\n            tiles,\n            inp,\n            farm_job,\n            rung_label,\n            farm_task,\n            farm_has_profession,\n            farm_rt,\n        ),",
        );
    }

    // ladder: farm_rt after farm_task, fire_rt present
    if !t.contains("farm_rt: &mut FarmProfessionRuntime,\n    smith_rt: &mut SmithProfessionRuntime,") {
        let _ = replace_once(
            &mut t,
            "    farm_task: &mut FarmTaskState,\n    smith_rt: &mut SmithProfessionRuntime,\n    baker_rt: &mut BakerProfessionRuntime,\n    baker_task: &mut BakerTaskState,\n    shepherd_rt: &mut ShepherdProfessionRuntime,\n    pottery_rt: &mut PotterProfessionRuntime,\n    fire_rt: &mut crate::FireFoodProfessionRuntime,\n) -> ProfessionScanTickResult {",
            "    farm_task: &mut FarmTaskState,\n    farm_rt: &mut FarmProfessionRuntime,\n    smith_rt: &mut SmithProfessionRuntime,\n    baker_rt: &mut BakerProfessionRuntime,\n    baker_task: &mut BakerTaskState,\n    shepherd_rt: &mut ShepherdProfessionRuntime,\n    pottery_rt: &mut PotterProfessionRuntime,\n    fire_rt: &mut crate::FireFoodProfessionRuntime,\n) -> ProfessionScanTickResult {",
        );
        // inject weight refresh + farm_rt into profession_scan_tick call
        let _ = replace_once(
            &mut t,
            "        inp.is_assigned_job = step.is_assigned_job;\n        inp.profession_is_sticky = step.profession_is_sticky;\n        let r = profession_scan_tick(\n            step.kind,\n            tiles,\n            &inp,\n            step.rung_label,\n            step.farm_job,\n            farm_task,\n            step.farm_has_profession,\n            smith_rt,",
            "        inp.is_assigned_job = step.is_assigned_job;\n        inp.profession_is_sticky = step.profession_is_sticky;\n        // AI-FARM-STICKY: refresh weight from sticky runtime each step\n        inp.basic_farmer_weight = basic_farmer_weight_from_runtime(farm_rt);\n        let r = profession_scan_tick(\n            step.kind,\n            tiles,\n            &inp,\n            step.rung_label,\n            step.farm_job,\n            farm_task,\n            step.farm_has_profession,\n            farm_rt,\n            smith_rt,",
        );
        // make_stuff_scan_tick call with farm_rt
        let _ = replace_once(
            &mut t,
            "        let r = make_stuff_scan_tick(\n            tiles,\n            base_inp,\n            farm_task,\n            shepherd_rt,\n            baker_rt,\n            baker_task,\n            fire_rt,\n        );",
            "        let mut inp = *base_inp;\n        inp.basic_farmer_weight = basic_farmer_weight_from_runtime(farm_rt);\n        let r = make_stuff_scan_tick(\n            tiles,\n            &inp,\n            farm_task,\n            farm_rt,\n            shepherd_rt,\n            baker_rt,\n            baker_task,\n            fire_rt,\n        );",
        );
    }

    // apply_ladder writeback
    if !t.contains("p.farm_profession = farm_rt") {
        let _ = replace_once(
            &mut t,
            "    let mut farm_task = p.farm_task.clone();\n    let mut smith_rt = p.smith_profession.clone();\n    let mut baker_rt = p.baker_profession.clone();\n    let mut baker_task = p.baker_task.clone();\n    let mut shepherd_rt = p.shepherd_profession.clone();\n    let mut pottery_rt = p.pottery_profession.clone();\n    let mut fire_rt = p.fire_food_profession.clone();\n\n    let result = ladder_profession_scan_tick(\n        rung,\n        &tiles,\n        &base_inp,\n        &sticky,\n        &mut farm_task,\n        &mut smith_rt,",
            "    let mut farm_task = p.farm_task.clone();\n    let mut farm_rt = p.farm_profession.clone();\n    let mut smith_rt = p.smith_profession.clone();\n    let mut baker_rt = p.baker_profession.clone();\n    let mut baker_task = p.baker_task.clone();\n    let mut shepherd_rt = p.shepherd_profession.clone();\n    let mut pottery_rt = p.pottery_profession.clone();\n    let mut fire_rt = p.fire_food_profession.clone();\n\n    // AI-FARM-STICKY: seed ProfessionScanInput from sticky BASICFARMER weight\n    let mut base_inp = base_inp;\n    base_inp.basic_farmer_weight = basic_farmer_weight_from_runtime(&farm_rt);\n\n    let result = ladder_profession_scan_tick(\n        rung,\n        &tiles,\n        &base_inp,\n        &sticky,\n        &mut farm_task,\n        &mut farm_rt,\n        &mut smith_rt,",
        );
        let _ = replace_once(
            &mut t,
            "    if let Some(p) = state.players.get_mut(&conn_id) {\n        p.farm_task = farm_task;\n        p.smith_profession = smith_rt;\n        p.baker_profession = baker_rt;\n        p.baker_task = baker_task;\n        p.shepherd_profession = shepherd_rt;\n        p.pottery_profession = pottery_rt;\n        p.fire_food_profession = fire_rt;\n    }",
            "    if let Some(p) = state.players.get_mut(&conn_id) {\n        p.farm_task = farm_task;\n        p.farm_profession = farm_rt;\n        p.smith_profession = smith_rt;\n        p.baker_profession = baker_rt;\n        p.baker_task = baker_task;\n        p.shepherd_profession = shepherd_rt;\n        p.pottery_profession = pottery_rt;\n        p.fire_food_profession = fire_rt;\n    }",
        );
    }

    // apply_profession_scan_tick
    if !t.contains("// AI-FARM-STICKY: seed + write back BASICFARMER weight on Player.farm_profession") {
        let _ = replace_once(
            &mut t,
            "    let mut farm_task = p.farm_task.clone();\n    let mut smith_rt = p.smith_profession.clone();\n    let mut baker_rt = p.baker_profession.clone();\n    let mut baker_task = p.baker_task.clone();\n    let mut shepherd_rt = p.shepherd_profession.clone();\n    let mut pottery_rt = p.pottery_profession.clone();\n\n    let result = profession_scan_tick(\n        kind,\n        &tiles,\n        &inp,\n        rung_label,\n        farm_job,\n        &mut farm_task,\n        farm_has,\n        &mut smith_rt,",
            "    let mut farm_task = p.farm_task.clone();\n    let mut farm_rt = p.farm_profession.clone();\n    let mut smith_rt = p.smith_profession.clone();\n    let mut baker_rt = p.baker_profession.clone();\n    let mut baker_task = p.baker_task.clone();\n    let mut shepherd_rt = p.shepherd_profession.clone();\n    let mut pottery_rt = p.pottery_profession.clone();\n\n    // AI-FARM-STICKY: seed + write back BASICFARMER weight on Player.farm_profession\n    let mut inp = inp;\n    inp.basic_farmer_weight = basic_farmer_weight_from_runtime(&farm_rt);\n\n    let result = profession_scan_tick(\n        kind,\n        &tiles,\n        &inp,\n        rung_label,\n        farm_job,\n        &mut farm_task,\n        farm_has,\n        &mut farm_rt,\n        &mut smith_rt,",
        );
        let _ = replace_once(
            &mut t,
            "    if let Some(p) = state.players.get_mut(&conn_id) {\n        p.farm_task = farm_task;\n        p.smith_profession = smith_rt;\n        p.baker_profession = baker_rt;\n        p.baker_task = baker_task;\n        p.shepherd_profession = shepherd_rt;\n        p.pottery_profession = pottery_rt;\n    }\n\n    // PATH-REACH: failed USE → age>3 notReachable else hostile (Haxe ~9133–9134).\n    let intent = result.intent;\n    let apply_r = apply_short_craft_live_intent(state, outbound, conn_id, intent);\n    if matches!(apply_r, ShortCraftLiveApplyResult::Failed) {\n        if let ShortCraftLiveIntent::UseAt { x, y, .. }\n        | ShortCraftLiveIntent::UseOnEmptyGround { x, y, .. } = intent\n        {\n            if let Some(p) = state.players.get_mut(&conn_id) {\n                let age = p.age;\n                crate::mark_use_path_fail(&mut p.ai_path_reach, x, y, age);\n            }\n        }\n    }\n    apply_r\n}\n\n// ── Unit tests",
            "    if let Some(p) = state.players.get_mut(&conn_id) {\n        p.farm_task = farm_task;\n        p.farm_profession = farm_rt;\n        p.smith_profession = smith_rt;\n        p.baker_profession = baker_rt;\n        p.baker_task = baker_task;\n        p.shepherd_profession = shepherd_rt;\n        p.pottery_profession = pottery_rt;\n    }\n\n    // PATH-REACH: failed USE → age>3 notReachable else hostile (Haxe ~9133–9134).\n    let intent = result.intent;\n    let apply_r = apply_short_craft_live_intent(state, outbound, conn_id, intent);\n    if matches!(apply_r, ShortCraftLiveApplyResult::Failed) {\n        if let ShortCraftLiveIntent::UseAt { x, y, .. }\n        | ShortCraftLiveIntent::UseOnEmptyGround { x, y, .. } = intent\n        {\n            if let Some(p) = state.players.get_mut(&conn_id) {\n                let age = p.age;\n                crate::mark_use_path_fail(&mut p.ai_path_reach, x, y, age);\n            }\n        }\n    }\n    apply_r\n}\n\n// ── Unit tests",
        );
    }

    std::fs::write(&path, t).is_ok()
}

fn patch_make_stuff_live(src: &Path) -> bool {
    let path = src.join("make_stuff_live.inc.rs");
    let Ok(mut t) = std::fs::read_to_string(&path) else {
        return true; // optional
    };
    if t.contains("farm_rt: &mut FarmProfessionRuntime") {
        return true;
    }
    let _ = replace_once(
        &mut t,
        "    farm_task: &mut FarmTaskState,\n    shepherd_rt: &mut ShepherdProfessionRuntime,\n    baker_rt: &mut BakerProfessionRuntime,\n    baker_task: &mut BakerTaskState,\n    fire_rt: &mut crate::FireFoodProfessionRuntime,\n) -> ProfessionScanTickResult {",
        "    farm_task: &mut FarmTaskState,\n    farm_rt: &mut FarmProfessionRuntime,\n    shepherd_rt: &mut ShepherdProfessionRuntime,\n    baker_rt: &mut BakerProfessionRuntime,\n    baker_task: &mut BakerTaskState,\n    fire_rt: &mut crate::FireFoodProfessionRuntime,\n) -> ProfessionScanTickResult {",
    );
    replace_all(
        &mut t,
        "farm_action_to_live_intent(tiles, inp, sharpie)",
        "farm_action_to_live_intent(tiles, inp, sharpie, farm_rt)",
    );
    replace_all(
        &mut t,
        "farm_action_to_live_intent(tiles, inp, farm)",
        "farm_action_to_live_intent(tiles, inp, farm, farm_rt)",
    );
    std::fs::write(&path, t).is_ok()
}

fn patch_tests_light(src: &Path) -> bool {
    let path = src.join("profession_scan_tests.inc.rs");
    let Ok(mut t) = std::fs::read_to_string(&path) else {
        return false;
    };
    if !t.contains("FarmProfessionRuntime") {
        let _ = replace_once(
            &mut t,
            "use crate::farmer_profession::{\n    FarmAction, FarmProfession, FarmTaskState, BOWL_OF_SOIL, DRY_PLANTED_CARROTS, DYING_BUSH,\n};",
            "use crate::farmer_profession::{\n    basic_farmer_weight_from_runtime, FarmAction, FarmProfession, FarmProfessionRuntime,\n    FarmTaskState, BOWL_OF_SOIL, DRY_PLANTED_CARROTS, DYING_BUSH,\n};",
        );
    }
    // Append farm_rt to farm_action_to_live_intent if missing — simple line-based
    // Prefer not to break if already patched.
    if !t.contains("farm_action_defer_sheep_writes_basic_farmer_weight_sticky") {
        t.push_str(
            r#"
#[test]
fn farm_action_defer_sheep_writes_basic_farmer_weight_sticky() {
    // AI-FARM-STICKY: DeferSheepHerding → profession['BASICFARMER']=1 on farm_rt
    let tiles = vec![ScanTile::empty(0, 0, 0, 0)];
    let inp = ProfessionScanInput::basic(0, 0, 0);
    let mut farm_rt = FarmProfessionRuntime::default();
    assert!(farm_rt.weights.is_empty());
    let r = farm_action_to_live_intent(
        &tiles,
        &inp,
        FarmAction::DeferSheepHerding {
            max_profession: 1,
        },
        &mut farm_rt,
    );
    assert_eq!(
        farm_rt.weights.get(&FarmProfession::BasicFarmer),
        Some(&1.0)
    );
    let _ = r;
    let r2 = farm_action_to_live_intent(
        &tiles,
        &inp,
        FarmAction::ClearBasicFarmerWeight,
        &mut farm_rt,
    );
    assert!(!r2.had_action);
    assert_eq!(
        farm_rt.weights.get(&FarmProfession::BasicFarmer),
        Some(&0.0)
    );
    assert_eq!(basic_farmer_weight_from_runtime(&farm_rt), 0.0);
}

#[test]
fn farm_profession_input_reads_player_basic_farmer_weight() {
    use crate::Player;
    let mut p = Player::new(1, 1, "farm@t");
    assert_eq!(basic_farmer_weight_from_runtime(&p.farm_profession), 1.0);
    p.farm_profession
        .weights
        .insert(FarmProfession::BasicFarmer, 7.0);
    assert_eq!(basic_farmer_weight_from_runtime(&p.farm_profession), 7.0);
}
"#,
        );
    }
    // Best-effort: inject farm_rt into ladder/profession_scan calls via known patterns
    if !t.contains("&mut farm_rt,\n        &mut smith_rt") {
        replace_all(
            &mut t,
            "&mut farm_task,\n        &mut smith_rt,",
            "&mut farm_task,\n        &mut farm_rt,\n        &mut smith_rt,",
        );
        replace_all(
            &mut t,
            "&mut farm_task,\n        true,\n    );",
            "&mut farm_task,\n        true,\n        &mut farm_rt,\n    );",
        );
        replace_all(
            &mut t,
            "&mut farm_task,\n        true,\n    )",
            "&mut farm_task,\n        true,\n        &mut farm_rt,\n    )",
        );
        // farm_action_to_live_intent three-arg → four-arg (common patterns)
        // Multline ShortCraft forms are hard; inject local farm_rt + trailing arg for simple cases
        if !t.contains("let mut farm_rt = FarmProfessionRuntime::default();") {
            // inject at start of tests that call farm_action
            // leave to Python for full coverage; sticky tests above compile if other tests fixed
        }
    }
    std::fs::write(&path, t).is_ok()
}

fn patch_npc(src: &Path) -> bool {
    // ol-server is sibling of ol-sim under crates/
    let npc = src
        .parent() // ol-sim
        .and_then(|p| p.parent()) // crates
        .map(|p| p.join("ol-server").join("src").join("npc_ai.rs"));
    let Some(path) = npc else {
        return true;
    };
    let Ok(mut t) = std::fs::read_to_string(&path) else {
        return true;
    };
    if t.contains("&mut st.farm_rt") {
        return true;
    }
    if !t.contains("FarmProfessionRuntime") {
        let _ = replace_once(
            &mut t,
            "CraftProfession, DropHeldSensorExtras, FarmProfession, FarmTaskState, NearbyObj,",
            "basic_farmer_weight_from_runtime, CraftProfession, DropHeldSensorExtras, FarmProfession,\n    FarmProfessionRuntime, FarmTaskState, FireFoodProfessionRuntime, NearbyObj,",
        );
    }
    if !t.contains("farm_rt:") {
        let _ = replace_once(
            &mut t,
            "struct NpcProfessionState {\n    farm_task: FarmTaskState,\n    smith_rt: SmithProfessionRuntime,",
            "struct NpcProfessionState {\n    farm_task: FarmTaskState,\n    farm_rt: FarmProfessionRuntime,\n    smith_rt: SmithProfessionRuntime,",
        );
        if !t.contains("fire_rt:") {
            let _ = replace_once(
                &mut t,
                "    pottery_rt: PotterProfessionRuntime,\n    /// PATH-REACH:",
                "    pottery_rt: PotterProfessionRuntime,\n    fire_rt: FireFoodProfessionRuntime,\n    /// PATH-REACH:",
            );
        }
    }
    let _ = replace_once(
        &mut t,
        "                        basic_farmer_weight: 1.0,",
        "                        basic_farmer_weight: basic_farmer_weight_from_runtime(&st.farm_rt),",
    );
    let _ = replace_once(
        &mut t,
        "                        &mut st.farm_task,\n                        &mut st.smith_rt,\n                        &mut st.baker_rt,\n                        &mut st.baker_task,\n                        &mut st.shepherd_rt,\n                        &mut st.pottery_rt,\n                    );",
        "                        &mut st.farm_task,\n                        &mut st.farm_rt,\n                        &mut st.smith_rt,\n                        &mut st.baker_rt,\n                        &mut st.baker_task,\n                        &mut st.shepherd_rt,\n                        &mut st.pottery_rt,\n                        &mut st.fire_rt,\n                    );",
    );
    std::fs::write(&path, t).is_ok()
}

fn patch_docs_light(workspace: &Path) -> bool {
    let port = workspace.join("docs").join("port");
    // changelog
    let cl = port.join("changelog").join("2026-07-28-AI-FARM-STICKY.md");
    if !cl.exists() {
        let body = r#"# AI-FARM-STICKY / basic_farmer_live

## Chunk
- **matrix_id:** `AI-FARM-STICKY`
- **chunk:** `basic_farmer_live`
- **mode:** implement
- **Haxe:** `openlife/auto/AiBase.hx` — `doBasicFarming` `profession['BASICFARMER']=1` mid sheep / `=0` idle clear
- **Rust:** `ol-sim` farmer_profession + profession_scan + Player.farm_profession

## Implemented
1. `basic_farmer_weight_from_runtime` — read sticky weight (default 1.0)
2. `farm_action_to_live_intent(..., farm_rt)` — applies `apply_basic_farmer_weight_side_effect` for DeferSheepHerding(=1) / ClearBasicFarmerWeight(=0)
3. Live wire: `apply_profession_ladder_tick` / `apply_profession_scan_tick` seed `ProfessionScanInput.basic_farmer_weight` from `Player.farm_profession` and write back after tick
4. `make_stuff_scan_tick` + ladder pass `farm_rt`; NPC sticky `NpcProfessionState.farm_rt`

## Residual
- assigned doBasicFarming(100) advanced max pass-through
- doWatering(3) before mid sheep (WaterBringer)
- has_or_become_profession live peer-cap on farm scan (still uses has_profession bool)

## Verify
```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
cargo test -p ol-sim --lib -- basic_farmer_weight farm_action_defer_sheep
```
"#;
        let _ = std::fs::create_dir_all(cl.parent().unwrap());
        let _ = std::fs::write(&cl, body);
    }
    true
}
