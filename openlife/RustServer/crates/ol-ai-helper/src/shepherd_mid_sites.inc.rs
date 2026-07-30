// Haxe: AiBase makeStuff / doBasicFarming mid sheep sites (AI-SHEPHERD-MID + AI-MAKE-STUFF)
// Included from shepherd_profession.rs

/// Haxe `makeStuff` pure step order (AI-SHEPHERD-MID / AI-MAKE-STUFF).
// Haxe: AiBase.makeStuff ~4074â€“4094
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MakeStuffAction {
    None,
    /// Haxe `makeSharpieFood` head
    DeferSharpieFood,
    /// Haxe `doBaking(max)` â€” body in baker_profession (AI-MAKE-STUFF)
    DeferBaking { max_profession: i32 },
    /// doBasicFarming(max) â€” farm action (may be DeferSheepHerding mid)
    BasicFarming { max_profession: i32 },
    /// isSheepHerding(max) after basic farm fallthrough
    SheepHerding { max_profession: i32 },
    /// Haxe `makeFireFood(max)` â€” body in fire_food_profession (AI-MAKE-STUFF)
    DeferFireFood { max_profession: i32 },
}

impl MakeStuffAction {
    pub fn is_some(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// Haxe makeStuff maxProfession for sheep (isSheepHerding(2)).
pub const MAKE_STUFF_SHEEP_MAX_PEOPLE: i32 = 2;
/// Haxe makeStuff / doBasicFarming(2) / doBaking(2) / makeFireFood(2) max.
pub const MAKE_STUFF_FARM_MAX_PEOPLE: i32 = 2;
/// Haxe doBasicFarming mid isSheepHerding(1).
pub const BASIC_FARM_MID_SHEEP_MAX_PEOPLE: i32 = 1;
/// Haxe after-sheep doAdvancedFarming(max) default when basic used default max.
pub const BASIC_FARM_AFTER_SHEEP_ADVANCED_MAX: i32 = 2;

/// Inputs for pure makeStuff order (all head booleans evaluated by caller).
// Haxe: AiBase.makeStuff ~4074
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MakeStuffInputs {
    pub sharpie_has_work: bool,
    pub baking_has_work: bool,
    pub basic_farm_has_work: bool,
    pub sheep_has_work: bool,
    pub fire_has_work: bool,
}

/// Pure `makeStuff` decision from explicit head flags (full Haxe order).
///
/// Order: makeSharpieFood â†’ doBaking(2) â†’ doBasicFarming(2) â†’ isSheepHerding(2)
/// â†’ makeFireFood(2).
// Haxe: AiBase.makeStuff ~4074â€“4083
pub fn make_stuff_ordered(inp: MakeStuffInputs) -> MakeStuffAction {
    if inp.sharpie_has_work {
        return MakeStuffAction::DeferSharpieFood;
    }
    if inp.baking_has_work {
        return MakeStuffAction::DeferBaking {
            max_profession: MAKE_STUFF_FARM_MAX_PEOPLE,
        };
    }
    if inp.basic_farm_has_work {
        return MakeStuffAction::BasicFarming {
            max_profession: MAKE_STUFF_FARM_MAX_PEOPLE,
        };
    }
    if inp.sheep_has_work {
        return MakeStuffAction::SheepHerding {
            max_profession: MAKE_STUFF_SHEEP_MAX_PEOPLE,
        };
    }
    if inp.fire_has_work {
        return MakeStuffAction::DeferFireFood {
            max_profession: MAKE_STUFF_FARM_MAX_PEOPLE,
        };
    }
    MakeStuffAction::None
}

/// Pure `makeStuff` decision: first applicable step.
///
/// When `include_residual_defers` is false, only farm â†’ sheep are considered
/// (sharpie/bake/fire heads left to caller). When true, residual fire is
/// emitted only after farm+sheep fallthrough (sharpie/bake still need
/// [`make_stuff_ordered`] with explicit flags, or [`make_stuff_try`]).
// Haxe: AiBase.makeStuff ~4074
pub fn make_stuff(
    basic_farm_action: crate::farmer_profession::FarmAction,
    sheep_has_work: bool,
    include_residual_defers: bool,
) -> MakeStuffAction {
    make_stuff_ordered(MakeStuffInputs {
        sharpie_has_work: false,
        baking_has_work: false,
        basic_farm_has_work: basic_farm_action.is_some(),
        sheep_has_work,
        fire_has_work: include_residual_defers,
    })
}

/// Full pure makeStuff expand using farm counts + optional baking/fire flags.
///
/// Evaluates sharpie body, then baking flag, then `do_basic_farming`, then
/// sheep work flag (caller expands sheep via [`make_stuff_try_sheep`]).
/// Prefer [`make_stuff_try_bodies`] when bake/fire counts are available.
// Haxe: AiBase.makeStuff ~4074
pub fn make_stuff_try(
    farm_counts: &crate::farmer_profession::FarmCounts,
    farm_task: &mut FarmTaskState,
    has_basic_farmer: bool,
    baking_has_work: bool,
    sheep_has_work: bool,
    fire_has_work: bool,
) -> MakeStuffAction {
    // 1) makeSharpieFood (ungated by age in makeStuff â€” Haxe has no age gate here)
    if crate::farmer_profession::make_sharpie_food(farm_counts).is_some() {
        return MakeStuffAction::DeferSharpieFood;
    }
    // 2) doBaking(2) â€” flag path; prefer [`make_stuff_try_bodies`] for pure body
    if baking_has_work {
        return MakeStuffAction::DeferBaking {
            max_profession: MAKE_STUFF_FARM_MAX_PEOPLE,
        };
    }
    // 3) doBasicFarming(2)
    let farm = crate::farmer_profession::do_basic_farming(
        farm_counts,
        farm_task,
        has_basic_farmer,
        MAKE_STUFF_FARM_MAX_PEOPLE,
    );
    if farm.is_some() {
        return MakeStuffAction::BasicFarming {
            max_profession: MAKE_STUFF_FARM_MAX_PEOPLE,
        };
    }
    // 4) isSheepHerding(2)
    if sheep_has_work {
        return MakeStuffAction::SheepHerding {
            max_profession: MAKE_STUFF_SHEEP_MAX_PEOPLE,
        };
    }
    // 5) makeFireFood(2) â€” flag path; prefer [`make_stuff_try_bodies`]
    if fire_has_work {
        return MakeStuffAction::DeferFireFood {
            max_profession: MAKE_STUFF_FARM_MAX_PEOPLE,
        };
    }
    MakeStuffAction::None
}

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
///
/// Order: sharpie â†’ doBaking(2) â†’ doBasicFarming(2) â†’ isSheepHerding(2) â†’ makeFireFood(2).
// Haxe: AiBase.makeStuff ~4074â€“4083
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
    let farm = crate::farmer_profession::do_basic_farming(
        farm_counts,
        farm_task,
        has_basic_farmer,
        MAKE_STUFF_FARM_MAX_PEOPLE,
    );
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

/// Expand makeStuff sheep step via full is_sheep_herding(2, default maxAnimal).
// Haxe: AiBase.makeStuff isSheepHerding(2) ~4081
pub fn make_stuff_try_sheep(
    runtime: &mut ShepherdProfessionRuntime,
    counts: &ShepherdCounts,
    farm_task: &mut FarmTaskState,
    peer_count: f32,
    was_idle: f32,
) -> SheepHerdingResult {
    is_sheep_herding(
        runtime,
        counts,
        farm_task,
        MAKE_STUFF_SHEEP_MAX_PEOPLE,
        SHEPHERD_DEFAULT_MAX_ANIMAL,
        peer_count,
        was_idle,
    )
}

/// Expand doBasicFarming mid isSheepHerding(1).
// Haxe: AiBase.doBasicFarming isSheepHerding(1) ~2402
pub fn basic_farm_mid_try_sheep(
    runtime: &mut ShepherdProfessionRuntime,
    counts: &ShepherdCounts,
    farm_task: &mut FarmTaskState,
    peer_count: f32,
    was_idle: f32,
) -> SheepHerdingResult {
    is_sheep_herding(
        runtime,
        counts,
        farm_task,
        BASIC_FARM_MID_SHEEP_MAX_PEOPLE,
        SHEPHERD_DEFAULT_MAX_ANIMAL,
        peer_count,
        was_idle,
    )
}

/// Thin reverse-craft / inventory bias for Profession::Shepherd.
// Haxe: self-play SeekObject domestic sheep / lamb feed pipeline
pub fn pick_shepherd_goal(
    graph: &ol_ai_crafting::craft_graph::ReverseCraftGraph,
    have: &std::collections::HashSet<i32>,
) -> Goal {
    for &id in &[
        DOMESTIC_SHEEP,
        HUNGRY_DOMESTIC_LAMB,
        DOMESTIC_LAMB,
        SHORN_DOMESTIC_SHEEP,
        MILK_COW,
        BOWL_BERRIES_CARROT,
        BOWL_CORN_KERNELS,
    ] {
        if !have.contains(&id) {
            let products = graph.products_using(id);
            if let Some(&p) = products.first() {
                return Goal::SeekObject(p);
            }
            return Goal::SeekObject(id);
        }
    }
    Goal::SeekObject(crate::ai_goals::SHEPHERD_TARGET_ID)
}
