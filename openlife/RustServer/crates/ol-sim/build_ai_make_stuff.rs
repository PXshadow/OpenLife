//! AI-MAKE-STUFF / make_fire_bake — pure Rust idempotent source patches.
//! Invoked from build.rs; also runs Python apply when present.

use std::path::Path;
use std::process::Command;

pub fn already_wired(src: &Path) -> bool {
    let lib = std::fs::read_to_string(src.join("lib.rs")).unwrap_or_default();
    let mid = std::fs::read_to_string(src.join("shepherd_mid_sites.inc.rs")).unwrap_or_default();
    let scan = std::fs::read_to_string(src.join("profession_scan.rs")).unwrap_or_default();
    let player = std::fs::read_to_string(src.join("player.rs")).unwrap_or_default();
    let fire = src.join("fire_food_profession.rs").exists();
    fire
        && lib.contains("mod fire_food_profession")
        && lib.contains("make_fire_food")
        && mid.contains("make_stuff_try_bodies")
        && scan.contains("fire_food_action_to_live_intent")
        && player.contains("fire_food_profession")
}

pub fn patch_all(src: &Path, workspace: &Path) -> bool {
    // Prefer Python apply script (full fidelity).
    let py = src.join("_apply_ai_make_stuff.py");
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
    // Fallback: minimal Rust patches
    let mut ok = true;
    ok &= patch_lib_min(src);
    ok &= patch_player_min(src);
    ok &= patch_mid_min(src);
    ok &= patch_scan_min(src);
    let _ = patch_docs_light(workspace);
    ok && already_wired(src) || ok
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

fn write_if_changed(path: &Path, text: &str) -> bool {
    let prev = std::fs::read_to_string(path).unwrap_or_default();
    if prev == text {
        return true;
    }
    std::fs::write(path, text).is_ok()
}

fn patch_lib_min(src: &Path) -> bool {
    let path = src.join("lib.rs");
    let Ok(mut t) = std::fs::read_to_string(&path) else {
        return false;
    };
    let mut ok = true;
    ok &= replace_once(
        &mut t,
        r#"// Haxe: AiBase shepherd / isSheepHerding profession family (AI-SHEPHERD)
mod shepherd_profession;
// Haxe: AiBase.shortCraft → useHeldObjOnTarget / DROP live intent (CRAFT-LIVE-IO)
mod short_craft_intent;"#,
        r#"// Haxe: AiBase shepherd / isSheepHerding profession family (AI-SHEPHERD)
mod shepherd_profession;
// Haxe: AiBase.makeFireFood / FIREFOODMAKER (AI-MAKE-STUFF)
mod fire_food_profession;
// Haxe: AiBase.shortCraft → useHeldObjOnTarget / DROP live intent (CRAFT-LIVE-IO)
mod short_craft_intent;"#,
    );
    ok &= replace_once(
        &mut t,
        r#"// Haxe: AiBase shepherd / isSheepHerding (AI-SHEPHERD)
pub use shepherd_profession::{"#,
        r#"// Haxe: AiBase.makeFireFood / FIREFOODMAKER (AI-MAKE-STUFF / make_fire_bake)
pub use fire_food_profession::{
    assign_fire_food_from_speech, count_fire_food_peers, count_fire_food_peers_filtered,
    count_done_goose, count_omelette_haxe_bug, count_raw_goose, count_raw_rabbit,
    fill_fire_food_counts_from_map, fire_food_action_to_goal, fire_food_counts_from_nearby,
    fire_food_max_people_for_dispatch, has_or_become_fire_food, has_or_become_fire_food_filtered,
    make_fire_food, make_popcorn_if_needed, needed_raw_fire_food, parse_fire_food_profession_speech,
    resolve_fire_food_assigned_job, FireFoodAction, FireFoodCounts, FireFoodMapObj,
    FireFoodPeerSnapshot, FireFoodProfessionRuntime, COOKED_GOOSE, COOKED_GOOSE_SKEWERED,
    COOKED_RABBIT, COOKED_RABBIT_SKEWERED, FIRE, FIRE_FOOD_ASSIGNED_MAX_PEOPLE,
    FIRE_FOOD_DEFAULT_MAX_PEOPLE, FIRE_FOOD_HOME_RADIUS, FIRE_FOOD_MAKE_STUFF_MAX_PEOPLE,
    FIRE_FOOD_PROFESSION_KEY, LARGE_FAST_FIRE, LARGE_SLOW_FIRE, OMELETTE, PLUCKED_GOOSE,
    POPCORN, POPPING_CORN, RAW_PORK, SKEWERED_GOOSE, SKEWERED_RABBIT, SKINNED_RABBIT,
};
// Re-export HOT_COALS under fire-food path name clash with drop_held — use fire_food_profession::HOT_COALS.
// Haxe: AiBase shepherd / isSheepHerding (AI-SHEPHERD)
pub use shepherd_profession::{"#,
    );
    ok &= replace_once(
        &mut t,
        r#"    make_stuff_try, make_stuff_try_sheep, basic_farm_mid_try_sheep, pick_shepherd_goal,"#,
        r#"    make_stuff_try, make_stuff_try_bodies, make_stuff_try_sheep, basic_farm_mid_try_sheep,
    pick_shepherd_goal, make_stuff_bake_has_work, make_stuff_fire_has_work,"#,
    );
    ok &= replace_once(
        &mut t,
        r#"    MAKE_STUFF_SHEEP_MAX_PEOPLE, BASIC_FARM_MID_SHEEP_MAX_PEOPLE,"#,
        r#"    MAKE_STUFF_SHEEP_MAX_PEOPLE, MAKE_STUFF_FARM_MAX_PEOPLE, BASIC_FARM_MID_SHEEP_MAX_PEOPLE,"#,
    );
    ok &= replace_once(
        &mut t,
        r#"    live_intent_is_wire, make_stuff_scan_tick, peer_count_for_kind, peer_roster_flags_for_player,"#,
        r#"    live_intent_is_wire, make_stuff_scan_tick, fire_food_action_to_live_intent,
    peer_count_for_kind, peer_roster_flags_for_player,"#,
    );
    write_if_changed(&path, &t) && ok
}

fn patch_player_min(src: &Path) -> bool {
    let path = src.join("player.rs");
    let Ok(mut t) = std::fs::read_to_string(&path) else {
        return false;
    };
    let mut ok = true;
    ok &= replace_once(
        &mut t,
        r#"    /// Sticky AI shepherd profession last / assigned / weight (Haxe `profession['SHEPHERD']`).
    // Haxe: AiBase.profession['SHEPHERD'] + lastProfession / assignedProfession
    pub shepherd_profession: crate::ShepherdProfessionRuntime,
    /// Sticky baker task hysteresis (`makeRawPies`, kindling, plant flags)."#,
        r#"    /// Sticky AI shepherd profession last / assigned / weight (Haxe `profession['SHEPHERD']`).
    // Haxe: AiBase.profession['SHEPHERD'] + lastProfession / assignedProfession
    pub shepherd_profession: crate::ShepherdProfessionRuntime,
    /// Sticky AI fire-food maker last / assigned / weight (Haxe `profession['FIREFOODMAKER']`).
    // Haxe: AiBase.profession['FIREFOODMAKER'] + lastProfession (AI-MAKE-STUFF)
    pub fire_food_profession: crate::FireFoodProfessionRuntime,
    /// Sticky baker task hysteresis (`makeRawPies`, kindling, plant flags)."#,
    );
    ok &= replace_once(
        &mut t,
        r#"            shepherd_profession: crate::ShepherdProfessionRuntime::default(),
            baker_task: crate::BakerTaskState::default(),"#,
        r#"            shepherd_profession: crate::ShepherdProfessionRuntime::default(),
            fire_food_profession: crate::FireFoodProfessionRuntime::default(),
            baker_task: crate::BakerTaskState::default(),"#,
    );
    write_if_changed(&path, &t) && ok
}

fn patch_mid_min(src: &Path) -> bool {
    let path = src.join("shepherd_mid_sites.inc.rs");
    let Ok(mut t) = std::fs::read_to_string(&path) else {
        return false;
    };
    if t.contains("make_stuff_try_bodies") {
        return true;
    }
    let mut ok = true;
    ok &= replace_once(
        &mut t,
        r#"    /// Residual: makeFireFood(max)
    DeferFireFood { max_profession: i32 },
}"#,
        r#"    /// Haxe `makeFireFood(max)` — body in fire_food_profession (AI-MAKE-STUFF)
    DeferFireFood { max_profession: i32 },
}"#,
    );
    // Append helpers before pick_shepherd_goal
    if !t.contains("make_stuff_try_bodies") {
        let insert = r#"
/// True when pure `doBaking(max=2)` would return work (not None/Abort).
// Haxe: AiBase.makeStuff doBaking(2) ~4079
pub fn make_stuff_bake_has_work(
    counts: &crate::baker_profession::BakeCounts,
    runtime: &mut crate::baker_profession::BakerProfessionRuntime,
    task: &mut crate::baker_profession::BakerTaskState,
    peer_count: f32,
    was_idle: f32,
    rng_pie_index: usize,
) -> bool {
    let a = crate::baker_profession::do_baking(
        counts,
        runtime,
        task,
        MAKE_STUFF_FARM_MAX_PEOPLE,
        peer_count,
        was_idle,
        rng_pie_index,
    );
    !matches!(
        a,
        crate::baker_profession::BakeAction::None | crate::baker_profession::BakeAction::Abort
    )
}

/// True when pure `makeFireFood(max=2)` would return work.
// Haxe: AiBase.makeStuff makeFireFood(2) ~4083
pub fn make_stuff_fire_has_work(
    counts: &crate::fire_food_profession::FireFoodCounts,
    runtime: &mut crate::fire_food_profession::FireFoodProfessionRuntime,
    peer_count: f32,
    was_idle: f32,
) -> bool {
    let a = crate::fire_food_profession::make_fire_food(
        counts,
        runtime,
        MAKE_STUFF_FARM_MAX_PEOPLE,
        peer_count,
        was_idle,
    );
    a.is_some()
}

/// Full pure makeStuff expand evaluating bake + fire bodies (AI-MAKE-STUFF).
// Haxe: AiBase.makeStuff ~4074–4083
pub fn make_stuff_try_bodies(
    farm_counts: &crate::farmer_profession::FarmCounts,
    farm_task: &mut FarmTaskState,
    has_basic_farmer: bool,
    bake_counts: &crate::baker_profession::BakeCounts,
    baker_rt: &mut crate::baker_profession::BakerProfessionRuntime,
    baker_task: &mut crate::baker_profession::BakerTaskState,
    bake_peer: f32,
    bake_idle: f32,
    rng_pie_index: usize,
    sheep_has_work: bool,
    fire_counts: &crate::fire_food_profession::FireFoodCounts,
    fire_rt: &mut crate::fire_food_profession::FireFoodProfessionRuntime,
    fire_peer: f32,
    fire_idle: f32,
) -> MakeStuffAction {
    if crate::farmer_profession::make_sharpie_food(farm_counts).is_some() {
        return MakeStuffAction::DeferSharpieFood;
    }
    if make_stuff_bake_has_work(
        bake_counts,
        baker_rt,
        baker_task,
        bake_peer,
        bake_idle,
        rng_pie_index,
    ) {
        return MakeStuffAction::DeferBaking {
            max_profession: MAKE_STUFF_FARM_MAX_PEOPLE,
        };
    }
    let farm = crate::farmer_profession::do_basic_farming(farm_counts, farm_task, has_basic_farmer);
    if farm.is_some() {
        return MakeStuffAction::BasicFarming {
            max_profession: MAKE_STUFF_FARM_MAX_PEOPLE,
        };
    }
    if sheep_has_work {
        return MakeStuffAction::SheepHerding {
            max_profession: MAKE_STUFF_SHEEP_MAX_PEOPLE,
        };
    }
    if make_stuff_fire_has_work(fire_counts, fire_rt, fire_peer, fire_idle) {
        return MakeStuffAction::DeferFireFood {
            max_profession: MAKE_STUFF_FARM_MAX_PEOPLE,
        };
    }
    MakeStuffAction::None
}

"#;
        ok &= replace_once(
            &mut t,
            r#"/// Thin reverse-craft / inventory bias for Profession::Shepherd.
// Haxe: self-play SeekObject domestic sheep / lamb feed pipeline
pub fn pick_shepherd_goal("#,
            &format!(
                "{insert}/// Thin reverse-craft / inventory bias for Profession::Shepherd.
// Haxe: self-play SeekObject domestic sheep / lamb feed pipeline
pub fn pick_shepherd_goal("
            ),
        );
    }
    write_if_changed(&path, &t) && ok
}

fn patch_scan_min(src: &Path) -> bool {
    let path = src.join("profession_scan.rs");
    let Ok(mut t) = std::fs::read_to_string(&path) else {
        return false;
    };
    if t.contains("fire_food_action_to_live_intent") && t.contains("fire_rt: &mut crate::FireFoodProfessionRuntime") {
        return true;
    }
    // Run python for full scan patch if possible
    let py = src.join("_apply_ai_make_stuff.py");
    if py.exists() {
        let _ = Command::new("python")
            .arg(&py)
            .current_dir(src)
            .status();
        let t2 = std::fs::read_to_string(&path).unwrap_or_default();
        return t2.contains("fire_food_action_to_live_intent");
    }
    false
}

fn patch_docs_light(workspace: &Path) -> bool {
    let port = workspace.join("docs").join("port");
    let cl = port.join("changelog").join("2026-07-28-AI-MAKE-STUFF.md");
    if !cl.exists() {
        let _ = std::fs::write(
            &cl,
            r#"# AI-MAKE-STUFF / make_fire_bake

## Chunk
- **matrix_id:** `AI-MAKE-STUFF`
- **chunk:** `make_fire_bake`
- **mode:** implement
- **Haxe:** `AiBase.makeStuff` / `doBaking(2)` / `makeFireFood(2)`
- **Rust:** `fire_food_profession` + `make_stuff_scan_tick` bake/fire bodies

## Residual
- FIREFOODMAKER age-rotated job rung; popcorn BowlFiller peer; bake Defer* farm tails
"#,
        );
    }
    true
}
