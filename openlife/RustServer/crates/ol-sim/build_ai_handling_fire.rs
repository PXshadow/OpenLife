//! AI-HANDLING-FIRE / is_handling_fire — pure Rust + Python wire (idempotent).
use std::path::Path;
use std::process::Command;

pub fn already_wired(src: &Path) -> bool {
    let lib = std::fs::read_to_string(src.join("lib.rs")).unwrap_or_default();
    let scan = std::fs::read_to_string(src.join("profession_scan.rs")).unwrap_or_default();
    let player = std::fs::read_to_string(src.join("player.rs")).unwrap_or_default();
    let hf = std::fs::read_to_string(src.join("handling_fire.rs")).unwrap_or_default();
    let live = std::fs::read_to_string(src.join("handling_fire_live.inc.rs")).unwrap_or_default();
    src.join("handling_fire.rs").exists()
        && !hf.contains("use crate::craft_item::LONG_STRAIGHT_SHAFT")
        && lib.contains("mod handling_fire")
        && lib.contains("is_handling_fire")
        && player.contains("fire_keeper_profession")
        && scan.contains("HandlingFire")
        && scan.contains("late_make_fire_food_scan_tick")
        && scan.contains("handling_fire_profession_scan_tick")
        // Residual wire markers (do not re-run python template once present)
        && live.contains("expand_handling_fire_do_baking")
        && live.contains("inp.is_winter")
        && scan.contains("CONSIDER_MAKE_FOOD")
        && scan.contains("late_make_fire_food_scan_tick(tiles")
}

pub fn patch_all(src: &Path, workspace: &Path) -> bool {
    // Always fix pure module import path (craft_item is under get_or_craft).
    let _ = fix_handling_fire_import(src);

    let py = src.join("_run_ai_handling_fire_wire.py");
    if py.exists() {
        let status = Command::new("python")
            .arg(&py)
            .current_dir(src)
            .status()
            .or_else(|_| Command::new("python3").arg(&py).current_dir(src).status());
        if let Ok(s) = status {
            let _ = fix_handling_fire_import(src);
            if s.success() && already_wired(src) {
                let _ = patch_docs(workspace);
                return true;
            }
        }
    }
    // Pure Rust fallback (lib + player + scan essentials)
    let mut ok = true;
    ok &= patch_lib(src);
    ok &= patch_player(src);
    ok &= patch_scan(src);
    let _ = fix_handling_fire_import(src);
    let _ = patch_docs(workspace);
    already_wired(src) || ok
}

fn fix_handling_fire_import(src: &Path) -> bool {
    let path = src.join("handling_fire.rs");
    let Ok(mut t) = std::fs::read_to_string(&path) else {
        return false;
    };
    if !t.contains("use crate::craft_item::LONG_STRAIGHT_SHAFT") {
        return true;
    }
    t = t.replace(
        "use crate::baker_profession::KINDLING;\nuse crate::craft_item::LONG_STRAIGHT_SHAFT;\nuse crate::fire_food_profession::{",
        "use crate::baker_profession::KINDLING;\nuse crate::fire_food_profession::{",
    );
    if !t.contains("pub const LONG_STRAIGHT_SHAFT") {
        t = t.replace(
            "// ── Object ids (OHOL / OpenLife; Haxe comments in AiBase.isHandlingFire) ─────\n\n/// Hot Adobe Oven 250",
            "// ── Object ids (OHOL / OpenLife; Haxe comments in AiBase.isHandlingFire) ─────\n\n/// Long Straight Shaft 67 (no-fire craft path).\n// Haxe: AiBase.isHandlingFire ~1104\npub const LONG_STRAIGHT_SHAFT: i32 = 67;\n/// Hot Adobe Oven 250",
        );
    }
    std::fs::write(path, t).is_ok()
}

fn replace_once(text: &mut String, old: &str, new: &str) -> bool {
    if text.contains(new) {
        return true;
    }
    if let Some(i) = text.find(old) {
        text.replace_range(i..i + old.len(), new);
        true
    } else {
        false
    }
}

fn write_ok(path: &Path, t: &str) -> bool {
    std::fs::write(path, t).is_ok()
}

fn patch_lib(src: &Path) -> bool {
    let path = src.join("lib.rs");
    let Ok(mut t) = std::fs::read_to_string(&path) else {
        return false;
    };
    let mut ok = true;
    ok &= replace_once(
        &mut t,
        "// Haxe: FIREFOODMAKER assigned/last makeFireFood(100) (AI-FIREFOOD-RUNG)\nmod fire_food_rung;\n// Haxe: AiBase.shortCraft",
        "// Haxe: FIREFOODMAKER assigned/last makeFireFood(100) (AI-FIREFOOD-RUNG)\nmod fire_food_rung;\n// Haxe: AiBase.isHandlingFire / FIREKEEPER (AI-HANDLING-FIRE)\nmod handling_fire;\n// Haxe: AiBase.shortCraft",
    );
    ok &= replace_once(
        &mut t,
        "pub use fire_food_rung::{fire_food_job_rung_label, try_decide_fire_food_from_rung};\n// Haxe: AiBase shepherd / isSheepHerding (AI-SHEPHERD)\n",
        "pub use fire_food_rung::{fire_food_job_rung_label, try_decide_fire_food_from_rung};\n// Haxe: AiBase.isHandlingFire / FIREKEEPER (AI-HANDLING-FIRE)\npub use handling_fire::{\n    assign_fire_keeper_from_speech, expand_handling_fire_action, fire_food_max_people_for_path,\n    get_close_fire, handling_fire_action_to_goal, handling_fire_job_rung_label,\n    handling_fire_max_for_dispatch, handling_fire_sensors_from_map, has_or_become_fire_keeper,\n    is_handling_fire, is_handling_fire_full, is_handling_fire_hot_coals_kindling,\n    is_large_fire_idle, is_self_best_fire_keeper, make_fire_food_late_or_hungry,\n    parse_fire_keeper_profession_speech, resolve_fire_keeper_assigned_job,\n    try_decide_handling_fire_from_rung, FireFoodDispatchPath, FireKeeperProfessionRuntime,\n    HandlingFireAction, HandlingFireMapObj, HandlingFireSensors, BASKET_OF_CHARCOAL,\n    BIG_CHARCOAL_PILE, BUTT_LOG, CHOPPED_TREE, FIREWOOD, FIRE_KEEPER_PROFESSION_KEY,\n    FIRE_KEEPER_URGENT_MAX, FLASH_FIRE, GET_CLOSE_FIRE_MAXDIST, HANDLING_FIRE_ASSIGNED_MAX,\n    HANDLING_FIRE_COUNT_RADIUS, HANDLING_FIRE_DEFAULT_MAX, HANDLING_FIRE_NEAR_RADIUS,\n    HANDLING_FIRE_TEMP_MAX, MAKE_FIRE_FOOD_CRITICAL_MAX, MAKE_FIRE_FOOD_HOT_COALS_PLACE_MAX,\n    MAKE_FIRE_FOOD_HUNGRY_MAX, MAKE_FIRE_FOOD_LATE_MAX, MAKE_FIRE_FOOD_NEAR_COALS_MAX,\n    SKEWER_FOR_FIRE, WEAK_SKEWER, LONG_STRAIGHT_SHAFT as HANDLING_LONG_STRAIGHT_SHAFT,\n};\n// Haxe: AiBase shepherd / isSheepHerding (AI-SHEPHERD)\n",
    );
    write_ok(&path, &t) && ok
}

fn patch_player(src: &Path) -> bool {
    let path = src.join("player.rs");
    let Ok(mut t) = std::fs::read_to_string(&path) else {
        return false;
    };
    let mut ok = true;
    ok &= replace_once(
        &mut t,
        "    pub fire_food_profession: crate::FireFoodProfessionRuntime,\n    /// Sticky baker task hysteresis",
        "    pub fire_food_profession: crate::FireFoodProfessionRuntime,\n    /// Sticky AI fire keeper (Haxe profession['FIREKEEPER']) (AI-HANDLING-FIRE).\n    // Haxe: AiBase.profession['FIREKEEPER']\n    pub fire_keeper_profession: crate::FireKeeperProfessionRuntime,\n    /// Sticky baker task hysteresis",
    );
    ok &= replace_once(
        &mut t,
        "            fire_food_profession: crate::FireFoodProfessionRuntime::default(),\n            baker_task:",
        "            fire_food_profession: crate::FireFoodProfessionRuntime::default(),\n            fire_keeper_profession: crate::FireKeeperProfessionRuntime::default(),\n            baker_task:",
    );
    if !t.contains("fire_keeper_profession_sticky") {
        ok &= replace_once(
            &mut t,
            "        assert!(!p.fire_food_profession.is_last_fire_food);\n    }\n\n    #[test]\n    fn smith_profession_sticky_defaults_and_survives() {",
            "        assert!(!p.fire_food_profession.is_last_fire_food);\n    }\n\n    #[test]\n    fn fire_keeper_profession_sticky_defaults_and_survives() {\n        let mut p = Player::new(1, 1, \"firekeep@test\");\n        assert!(!p.fire_keeper_profession.is_last_fire_keeper);\n        assert!(crate::assign_fire_keeper_from_speech(\n            &mut p.fire_keeper_profession,\n            \"FIREKEEPER!\"\n        ));\n        assert!(p.fire_keeper_profession.is_assigned_fire_keeper);\n        p.fire_keeper_profession.wipe_on_eat(false);\n        assert!(!p.fire_keeper_profession.is_last_fire_keeper);\n    }\n\n    #[test]\n    fn smith_profession_sticky_defaults_and_survives() {",
        );
    }
    write_ok(&path, &t) && ok
}

fn patch_scan(src: &Path) -> bool {
    let path = src.join("profession_scan.rs");
    let Ok(mut t) = std::fs::read_to_string(&path) else {
        return false;
    };
    let mut ok = true;

    ok &= replace_once(
        &mut t,
        "//! **AI-FIREFOOD-RUNG**: `ProfessionScanKind::FireFood` assigned/last makeFireFood(100).\n",
        "//! **AI-FIREFOOD-RUNG**: `ProfessionScanKind::FireFood` assigned/last makeFireFood(100).\n//! **AI-HANDLING-FIRE**: `ProfessionScanKind::HandlingFire` isHandlingFire + late makeFireFood(1).\n",
    );
    ok &= replace_once(
        &mut t,
        "    FireFood,\n}\n",
        "    FireFood,\n    /// Haxe `isHandlingFire` / FIREKEEPER (AI-HANDLING-FIRE).\n    HandlingFire,\n}\n",
    );
    ok &= replace_once(
        &mut t,
        "include!(\"make_stuff_live.inc.rs\");\n",
        "include!(\"make_stuff_live.inc.rs\");\n// AI-HANDLING-FIRE\ninclude!(\"handling_fire_live.inc.rs\");\n",
    );

    if !t.contains("fire_keeper_assigned") {
        ok &= replace_once(
            &mut t,
            "    pub fire_food_last: bool,\n    pub age: f32,\n}\n",
            "    pub fire_food_last: bool,\n    pub fire_keeper_assigned: bool,\n    pub fire_keeper_last: bool,\n    pub age: f32,\n}\n",
        );
        ok &= replace_once(
            &mut t,
            "        Self::from_runtimes_ex(farm, smith, baker, None, None, None, age)\n",
            "        Self::from_runtimes_ex(farm, smith, baker, None, None, None, None, age)\n",
        );
        ok &= replace_once(
            &mut t,
            "        fire_food: Option<&crate::FireFoodProfessionRuntime>,\n        age: f32,\n    ) -> Self {\n        let (shepherd_assigned, shepherd_last) = shepherd\n            .map(|s| (s.is_assigned_shepherd, s.is_last_shepherd))\n            .unwrap_or((false, false));\n        let (pottery_assigned, pottery_last) = pottery\n            .map(|p| (p.is_assigned_potter, p.is_last_potter))\n            .unwrap_or((false, false));\n        let (fire_food_assigned, fire_food_last) = fire_food\n            .map(|f| (f.is_assigned_fire_food, f.is_last_fire_food))\n            .unwrap_or((false, false));\n        Self {\n            farm_assigned: farm.assigned_profession,\n            farm_last: farm.last_profession,\n            smith_assigned: smith.is_assigned_smith,\n            smith_last: smith.is_last_smith,\n            baker_assigned: baker.is_assigned_baker,\n            baker_last: baker.is_last_baker,\n            pottery_assigned,\n            pottery_last,\n            shepherd_assigned,\n            shepherd_last,\n            fire_food_assigned,\n            fire_food_last,\n            age,\n        }\n    }\n",
            "        fire_food: Option<&crate::FireFoodProfessionRuntime>,\n        fire_keeper: Option<&crate::FireKeeperProfessionRuntime>,\n        age: f32,\n    ) -> Self {\n        let (shepherd_assigned, shepherd_last) = shepherd\n            .map(|s| (s.is_assigned_shepherd, s.is_last_shepherd))\n            .unwrap_or((false, false));\n        let (pottery_assigned, pottery_last) = pottery\n            .map(|p| (p.is_assigned_potter, p.is_last_potter))\n            .unwrap_or((false, false));\n        let (fire_food_assigned, fire_food_last) = fire_food\n            .map(|f| (f.is_assigned_fire_food, f.is_last_fire_food))\n            .unwrap_or((false, false));\n        let (fire_keeper_assigned, fire_keeper_last) = fire_keeper\n            .map(|f| (f.is_assigned_fire_keeper, f.is_last_fire_keeper))\n            .unwrap_or((false, false));\n        Self {\n            farm_assigned: farm.assigned_profession,\n            farm_last: farm.last_profession,\n            smith_assigned: smith.is_assigned_smith,\n            smith_last: smith.is_last_smith,\n            baker_assigned: baker.is_assigned_baker,\n            baker_last: baker.is_last_baker,\n            pottery_assigned,\n            pottery_last,\n            shepherd_assigned,\n            shepherd_last,\n            fire_food_assigned,\n            fire_food_last,\n            fire_keeper_assigned,\n            fire_keeper_last,\n            age,\n        }\n    }\n",
        );
        ok &= replace_once(
            &mut t,
            "            || self.fire_food_assigned\n    }\n",
            "            || self.fire_food_assigned\n            || self.fire_keeper_assigned\n    }\n",
        );
        ok &= replace_once(
            &mut t,
            "            || self.fire_food_last\n    }\n",
            "            || self.fire_food_last\n            || self.fire_keeper_last\n    }\n",
        );
    }

    if !t.contains("sticky.fire_keeper_assigned") {
        ok &= replace_once(
            &mut t,
            "    if sticky.fire_food_assigned {\n        out.push(ProfessionLadderStep {\n            kind: ProfessionScanKind::FireFood,\n            rung_label: \"ASSIGNED_JOB\",\n            farm_job: None,\n            farm_has_profession: false,\n            is_assigned_job: true,\n            profession_is_sticky: true,\n        });\n    }\n    // Sticky last without explicit assigned still works assigned-weight via last.\n",
            "    if sticky.fire_food_assigned {\n        out.push(ProfessionLadderStep {\n            kind: ProfessionScanKind::FireFood,\n            rung_label: \"ASSIGNED_JOB\",\n            farm_job: None,\n            farm_has_profession: false,\n            is_assigned_job: true,\n            profession_is_sticky: true,\n        });\n    }\n    if sticky.fire_keeper_assigned {\n        out.push(ProfessionLadderStep {\n            kind: ProfessionScanKind::HandlingFire,\n            rung_label: \"ASSIGNED_JOB\",\n            farm_job: None,\n            farm_has_profession: false,\n            is_assigned_job: true,\n            profession_is_sticky: true,\n        });\n    }\n    // Sticky last without explicit assigned still works assigned-weight via last.\n",
        );
        ok &= replace_once(
            &mut t,
            "        if sticky.fire_food_last {\n            out.push(ProfessionLadderStep {\n                kind: ProfessionScanKind::FireFood,\n                rung_label: \"ASSIGNED_JOB\",\n                farm_job: None,\n                farm_has_profession: false,\n                is_assigned_job: true,\n                profession_is_sticky: true,\n            });\n        }\n    }\n    out\n}\n",
            "        if sticky.fire_food_last {\n            out.push(ProfessionLadderStep {\n                kind: ProfessionScanKind::FireFood,\n                rung_label: \"ASSIGNED_JOB\",\n                farm_job: None,\n                farm_has_profession: false,\n                is_assigned_job: true,\n                profession_is_sticky: true,\n            });\n        }\n        if sticky.fire_keeper_last {\n            out.push(ProfessionLadderStep {\n                kind: ProfessionScanKind::HandlingFire,\n                rung_label: \"ASSIGNED_JOB\",\n                farm_job: None,\n                farm_has_profession: false,\n                is_assigned_job: true,\n                profession_is_sticky: true,\n            });\n        }\n    }\n    out\n}\n",
        );
    }

    ok &= replace_once(
        &mut t,
        "        PriorityRung::MidPriorityTasks | PriorityRung::CriticalMisc => {\n            let mut steps = plan_assigned_job_steps(sticky);\n            if steps.is_empty() {\n                steps = plan_age_rotated_steps(sticky.age);\n            }\n            steps\n        }\n",
        "        // AI-HANDLING-FIRE: mid isHandlingFire() before other mid work\n        PriorityRung::MidPriorityTasks | PriorityRung::CriticalMisc => {\n            let mut steps = vec![ProfessionLadderStep {\n                kind: ProfessionScanKind::HandlingFire,\n                rung_label: match rung {\n                    PriorityRung::CriticalMisc => \"CRITICAL_MISC\",\n                    _ => \"MID_PRIORITY_TASKS\",\n                },\n                farm_job: None,\n                farm_has_profession: false,\n                is_assigned_job: sticky.fire_keeper_assigned || sticky.fire_keeper_last,\n                profession_is_sticky: sticky.fire_keeper_assigned || sticky.fire_keeper_last,\n            }];\n            steps.extend(plan_assigned_job_steps(sticky));\n            if steps.len() == 1 {\n                steps.extend(plan_age_rotated_steps(sticky.age));\n            }\n            steps\n        }\n",
    );

    if !t.contains("fire_keeper_rt: &mut crate::FireKeeperProfessionRuntime") {
        ok &= replace_once(
            &mut t,
            "    fire_rt: &mut crate::FireFoodProfessionRuntime,\n) -> ProfessionScanTickResult {\n    let steps = plan_profession_ladder_steps(rung, sticky);\n",
            "    fire_rt: &mut crate::FireFoodProfessionRuntime,\n    fire_keeper_rt: &mut crate::FireKeeperProfessionRuntime,\n) -> ProfessionScanTickResult {\n    let steps = plan_profession_ladder_steps(rung, sticky);\n",
        );
        ok &= replace_once(
            &mut t,
            "            fire_rt,\n        );\n        if !r.had_action {\n            continue;\n        }\n        if live_intent_is_wire(r.intent) {\n            return r;\n        }\n        // Haxe: dropHeld isMoving return true — hold tick, do not fall through (PREFER-SHORT-WAIT)\n",
            "            fire_rt,\n            fire_keeper_rt,\n        );\n        if !r.had_action {\n            continue;\n        }\n        if live_intent_is_wire(r.intent) {\n            return r;\n        }\n        // Haxe: dropHeld isMoving return true — hold tick, do not fall through (PREFER-SHORT-WAIT)\n",
        );
        ok &= replace_once(
            &mut t,
            "    fire_rt: &mut crate::FireFoodProfessionRuntime,\n) -> ProfessionScanTickResult {\n    match kind {\n",
            "    fire_rt: &mut crate::FireFoodProfessionRuntime,\n    fire_keeper_rt: &mut crate::FireKeeperProfessionRuntime,\n) -> ProfessionScanTickResult {\n    match kind {\n",
        );
        ok &= replace_once(
            &mut t,
            "        ProfessionScanKind::FireFood => {\n            fire_food_profession_scan_tick(tiles, inp, rung_label, fire_rt)\n        }\n    }\n}\n",
            "        ProfessionScanKind::FireFood => {\n            fire_food_profession_scan_tick(tiles, inp, rung_label, fire_rt)\n        }\n        ProfessionScanKind::HandlingFire => handling_fire_profession_scan_tick(\n            tiles,\n            inp,\n            rung_label,\n            fire_keeper_rt,\n            fire_rt,\n            false,\n        ),\n    }\n}\n",
        );
    }

    if !t.contains("late_make_fire_food_scan_tick") {
        ok &= replace_once(
            &mut t,
            "        if r.had_action {\n            return r;\n        }\n    }\n    staging\n}\n\n/// Scan world + run ladder profession steps + apply USE/DROP for one AI player.\n",
            "        if r.had_action {\n            return r;\n        }\n        // AI-HANDLING-FIRE: late makeFireFood(1)\n        let late = late_make_fire_food_scan_tick(tiles, &inp, fire_rt);\n        if late.had_action {\n            return late;\n        }\n    }\n    staging\n}\n\n/// Scan world + run ladder profession steps + apply USE/DROP for one AI player.\n",
        );
    }

    t = t.replace(
        "Some(&p.fire_food_profession),\n        age,",
        "Some(&p.fire_food_profession),\n        Some(&p.fire_keeper_profession),\n        age,",
    );
    t = t.replace(
        "Some(&p.fire_food_profession),\n            age,",
        "Some(&p.fire_food_profession),\n            Some(&p.fire_keeper_profession),\n            age,",
    );
    t = t.replace(
        "None, None, Some(&fire), 25.0,",
        "None, None, Some(&fire), None, 25.0,",
    );

    if !t.contains("ProfessionScanKind::HandlingFire => crate::FIRE_FOOD_HOME_RADIUS") {
        t = t.replacen(
            "ProfessionScanKind::FireFood => crate::FIRE_FOOD_HOME_RADIUS,\n",
            "ProfessionScanKind::FireFood => crate::FIRE_FOOD_HOME_RADIUS,\n            ProfessionScanKind::HandlingFire => crate::FIRE_FOOD_HOME_RADIUS,\n",
            1,
        );
        t = t.replacen(
            "ProfessionScanKind::FireFood => crate::FIRE_FOOD_HOME_RADIUS,\n",
            "ProfessionScanKind::FireFood => crate::FIRE_FOOD_HOME_RADIUS,\n                    ProfessionScanKind::HandlingFire => crate::FIRE_FOOD_HOME_RADIUS,\n",
            1,
        );
    }

    if !t.contains("ProfessionScanKind::HandlingFire => {\n            p.fire_keeper_profession") {
        ok &= replace_once(
            &mut t,
            "        ProfessionScanKind::FireFood => {\n            p.fire_food_profession.is_assigned_fire_food\n                || p.fire_food_profession.is_last_fire_food\n        }\n    };\n    let profession_is_sticky = match kind {\n",
            "        ProfessionScanKind::FireFood => {\n            p.fire_food_profession.is_assigned_fire_food\n                || p.fire_food_profession.is_last_fire_food\n        }\n        ProfessionScanKind::HandlingFire => {\n            p.fire_keeper_profession.is_assigned_fire_keeper\n                || p.fire_keeper_profession.is_last_fire_keeper\n        }\n    };\n    let profession_is_sticky = match kind {\n",
        );
        ok &= replace_once(
            &mut t,
            "        ProfessionScanKind::FireFood => fire_food_sticky,\n    };\n",
            "        ProfessionScanKind::FireFood => fire_food_sticky,\n        ProfessionScanKind::HandlingFire => {\n            p.fire_keeper_profession.is_last_fire_keeper\n                || p.fire_keeper_profession.is_assigned_fire_keeper\n        }\n    };\n",
        );
    }

    if !t.contains("let mut fire_keeper_rt") {
        ok &= replace_once(
            &mut t,
            "    let mut fire_rt = p.fire_food_profession.clone();\n\n    // AI-FARM-STICKY:",
            "    let mut fire_rt = p.fire_food_profession.clone();\n    let mut fire_keeper_rt = p.fire_keeper_profession.clone();\n\n    // AI-FARM-STICKY:",
        );
        ok &= replace_once(
            &mut t,
            "        &mut pottery_rt,\n        &mut fire_rt,\n    );\n\n    if let Some(p) = state.players.get_mut(&conn_id) {\n        p.farm_task = farm_task;\n        p.farm_profession = farm_rt;\n        p.smith_profession = smith_rt;\n        p.baker_profession = baker_rt;\n        p.baker_task = baker_task;\n        p.shepherd_profession = shepherd_rt;\n        p.pottery_profession = pottery_rt;\n        p.fire_food_profession = fire_rt;\n    }\n",
            "        &mut pottery_rt,\n        &mut fire_rt,\n        &mut fire_keeper_rt,\n    );\n\n    if let Some(p) = state.players.get_mut(&conn_id) {\n        p.farm_task = farm_task;\n        p.farm_profession = farm_rt;\n        p.smith_profession = smith_rt;\n        p.baker_profession = baker_rt;\n        p.baker_task = baker_task;\n        p.shepherd_profession = shepherd_rt;\n        p.pottery_profession = pottery_rt;\n        p.fire_food_profession = fire_rt;\n        p.fire_keeper_profession = fire_keeper_rt;\n    }\n",
        );
    }

    let tests = src.join("profession_scan_tests.inc.rs");
    if let Ok(tt) = std::fs::read_to_string(&tests) {
        let tt2 = tt
            .replace(
                "None, None, Some(&fire), 25.0,",
                "None, None, Some(&fire), None, 25.0,",
            )
            .replace(
                "None, None, Some(&fire), 20.0,",
                "None, None, Some(&fire), None, 20.0,",
            );
        if tt2 != tt {
            let _ = std::fs::write(&tests, tt2);
        }
    }

    write_ok(&path, &t) && ok
}

fn patch_docs(workspace: &Path) -> bool {
    let fm = workspace.join("docs/port/FILE_MATRIX.md");
    if let Ok(mut t) = std::fs::read_to_string(&fm) {
        if !t.contains("AI-HANDLING-FIRE") {
            t = t.replacen(
                "| **AI-FIREFOOD-RUNG** / firefood_job |",
                "| **AI-HANDLING-FIRE** / is_handling_fire | isHandlingFire + late/hungry makeFireFood(1/2/3) | **DONE** → pure+wire | pure `handling_fire`; FIREKEEPER sticky; mid HandlingFire; late makeFireFood(1) |\n| **AI-FIREFOOD-RUNG** / firefood_job |",
                1,
            );
        }
        t = t.replace(
            "Residual: isHandlingFire/hungry makeFireFood(1/2/3); popcorn BowlFiller peer; bake Defer* farm tails. Assigned/last FIREFOOD → **AI-FIREFOOD-RUNG DONE**",
            "Residual: popcorn BowlFiller peer; bake Defer* farm tails. FIREFOOD → **AI-FIREFOOD-RUNG DONE**; isHandlingFire → **AI-HANDLING-FIRE DONE**",
        );
        let _ = std::fs::write(&fm, t);
    }
    let todo = workspace.join("docs/port/TODO_PORT.md");
    if let Ok(mut t) = std::fs::read_to_string(&todo) {
        t = t.replace(
            "Residual: late/hungry/isHandlingFire makeFireFood(1/2/3); popcorn BowlFiller peer; bake Defer* farm tails. Assigned/last FIREFOOD → **AI-FIREFOOD-RUNG**",
            "Residual: popcorn BowlFiller peer; bake Defer* farm tails. FIREFOOD → **AI-FIREFOOD-RUNG DONE**; late/hungry/isHandlingFire → **AI-HANDLING-FIRE DONE**",
        );
        if !t.contains("AI-HANDLING-FIRE is_handling_fire DONE") {
            t = t.replacen(
                "- [x] **AI-FIREFOOD-RUNG firefood_job DONE**",
                "- [x] **AI-HANDLING-FIRE is_handling_fire DONE** — pure `handling_fire` + nested makeFireFood(2/3) + late/hungry max=1; sticky FIREKEEPER; mid HandlingFire; late makeFireFood(1); tests  \n- [x] **AI-FIREFOOD-RUNG firefood_job DONE**",
                1,
            );
        }
        t = t.replacen(
            "Last updated: **2026-07-28**",
            "Last updated: **2026-07-28** (AI-HANDLING-FIRE is_handling_fire)",
            1,
        );
        let _ = std::fs::write(&todo, t);
    }
    let ci = workspace.join("docs/port/CALL_INDEX.md");
    if let Ok(mut t) = std::fs::read_to_string(&ci) {
        if !t.contains("is_handling_fire") {
            t = t.replacen(
                "| `ProfessionScanKind::FireFood` / `fire_food_profession_scan_tick` / `try_decide_fire_food_from_rung` | `profession_scan.rs` + `fire_food_rung.rs` | assigned/last FIREFOODMAKER makeFireFood(100) (AI-FIREFOOD-RUNG) |\n",
                "| `ProfessionScanKind::FireFood` / `fire_food_profession_scan_tick` / `try_decide_fire_food_from_rung` | `profession_scan.rs` + `fire_food_rung.rs` | assigned/last FIREFOODMAKER makeFireFood(100) (AI-FIREFOOD-RUNG) |\n| `is_handling_fire` / `HandlingFireAction` / `FireKeeperProfessionRuntime` / `FireFoodDispatchPath` | `handling_fire.rs` | isHandlingFire + late/hungry residual (AI-HANDLING-FIRE) |\n| `ProfessionScanKind::HandlingFire` / `handling_fire_profession_scan_tick` / `late_make_fire_food_scan_tick` | `profession_scan.rs` + `handling_fire_live.inc.rs` | mid early + FIREKEEPER + late makeFireFood(1) |\n",
                1,
            );
            let _ = std::fs::write(&ci, t);
        }
    }
    true
}
