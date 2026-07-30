//! Haxe: `AiBase.doTimeStuffHelper` priority ladder (chunk **AI-PRIO** / `priority_ladder`).
//!
//! Pure ordered rungs matching the Haxe control-flow skeleton. Each rung is a decision
//! *slot* — profession bodies (`doBaking`, `doSmithing`, …) stay deferred to **AI-JOB-*** /
//! **AI-CRAFT** / **AI-FOOD** chunks. Sensors are boolean/state flags only; no world I/O.
//!
//! Haxe order (high → low), simplified into skeleton rungs:
//! 1. held-by / ordered drop
//! 2. **escape** (flee)
//! 3. drop-in-progress / close use / wait
//! 4. baby hungry → mother / child follow / wounded
//! 5. death / eating / feed child / clothes / ordered follow
//! 6. consider make food / use / container / pickup food
//! 7. temperature / combat / child care / kill animal / feed player / follow move
//! 8. home-social / critical craft / mid tasks / craft queue / clothing craft
//! 9. assigned job / critical misc / age-rotated job / low work / go home / idle
//!
//! TODO phrase “flee food feed craft job follow” maps to bands via [`PriorityBand`].
//!
//! Module path: `crate::ai_goals::priority_ladder` (declared from `ai_goals.rs`).

use super::{Goal, Profession, BAKER_TARGET_ID, FARMER_TARGET_ID, POTTER_TARGET_ID, SHEPHERD_TARGET_ID, HUNGRY_FOOD, SMITH_TARGET_ID};

/// Haxe `ServerSettings.MinAgeToEat` (years).
pub const MIN_AGE_TO_EAT: f32 = 3.0;
/// Haxe `ServerSettings.MaxChildAgeForBreastFeeding`.
pub const MAX_CHILD_AGE_BREASTFEED: f32 = 6.0;
/// Haxe hungry enter fraction of `food_store_max`.
pub const HUNGRY_ENTER_FRAC: f32 = 0.3;
/// Haxe leave-hungry fraction of `food_store_max`.
pub const HUNGRY_LEAVE_FRAC: f32 = 0.8;
/// Absolute floor for hungry-enter (Haxe `max = 3`).
pub const HUNGRY_ENTER_FLOOR: f32 = 3.0;
/// Absolute floor while holding Smithing Hammer 441 (Haxe `max = 1`).
pub const HUNGRY_ENTER_FLOOR_SMITH_HAMMER: f32 = 1.0;
/// Smithing Hammer object parent id (Haxe 441).
pub const SMITHING_HAMMER_ID: i32 = 441;

// ── Escape policy (Haxe `AiBase.escape`) ────────────────────────────────────

/// Haxe: ignore deadly player when `angryTime > 4`.
pub const ESCAPE_ANGRY_TIME_IGNORE: f32 = 4.0;
/// Haxe: only call escape when `didNotReachFood < 5`.
pub const ESCAPE_DID_NOT_REACH_FOOD_MAX: f32 = 5.0;
/// Haxe: `food_store < -1` skips escape (crit food overrides flee).
pub const ESCAPE_FOOD_CRIT_SKIP: f32 = -1.0;
/// Haxe: weapon + not wounded + `age > 8` → hunt instead of flee.
pub const ESCAPE_HUNT_MIN_AGE: f32 = 8.0;
/// Haxe: `distPlayer > 64` ignores player threat (~8 tiles).
pub const ESCAPE_PLAYER_DIST_MAX: f32 = 64.0;
/// Haxe `escapeDist = 3` tile step away from threat.
pub const ESCAPE_DIST: i32 = 3;
/// Haxe: `dist < 100` (quad) forces `doStuff` even under superbad temp.
pub const DO_STUFF_THREAT_QUAD_FORCE: f32 = 100.0;
/// Haxe heat band for superbad temperature: `heat < 0.1 || heat > 0.9`.
pub const SUPERBAD_HEAT_LOW: f32 = 0.1;
pub const SUPERBAD_HEAT_HIGH: f32 = 0.9;

/// Coarse band for logs (TODO: flee · food · feed · craft · job · follow).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PriorityBand {
    Blocked,
    Flee,
    Food,
    Feed,
    Craft,
    Job,
    Follow,
    Other,
}

/// Ordered decision rungs of `AiBase.doTimeStuffHelper` (AI-PRIO skeleton).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PriorityRung {
    HeldByOther,
    OrderedDrop,
    Escape,
    DroppingItem,
    CloseUse,
    Wait,
    BabyHungryMother,
    ChildWithMother,
    WoundedSeekHelp,
    HandleDeath,
    Eating,
    FeedChild,
    SwitchClothes,
    OrderedFollow,
    ConsiderMakeFood,
    UsingItem,
    RemoveFromContainer,
    PickupFood,
    Temperature,
    Combat,
    StayCloseChild,
    KillAnimal,
    FeedPlayerInNeed,
    FollowPlayer,
    HomeSocial,
    CriticalCraft,
    MidPriorityTasks,
    CraftQueue,
    ClothingCraft,
    AssignedJob,
    CriticalMisc,
    AgeRotatedJob,
    LowPriorityWork,
    GoHome,
    Idle,
}

impl PriorityRung {
    pub fn as_label(self) -> &'static str {
        match self {
            Self::HeldByOther => "HELD_BY_OTHER",
            Self::OrderedDrop => "ORDERED_DROP",
            Self::Escape => "ESCAPE",
            Self::DroppingItem => "DROPPING_ITEM",
            Self::CloseUse => "CLOSE_USE",
            Self::Wait => "WAIT",
            Self::BabyHungryMother => "BABY_HUNGRY_MOTHER",
            Self::ChildWithMother => "CHILD_WITH_MOTHER",
            Self::WoundedSeekHelp => "WOUNDED_SEEK_HELP",
            Self::HandleDeath => "HANDLE_DEATH",
            Self::Eating => "EATING",
            Self::FeedChild => "FEED_CHILD",
            Self::SwitchClothes => "SWITCH_CLOTHES",
            Self::OrderedFollow => "ORDERED_FOLLOW",
            Self::ConsiderMakeFood => "CONSIDER_MAKE_FOOD",
            Self::UsingItem => "USING_ITEM",
            Self::RemoveFromContainer => "REMOVE_FROM_CONTAINER",
            Self::PickupFood => "PICKUP_FOOD",
            Self::Temperature => "TEMPERATURE",
            Self::Combat => "COMBAT",
            Self::StayCloseChild => "STAY_CLOSE_CHILD",
            Self::KillAnimal => "KILL_ANIMAL",
            Self::FeedPlayerInNeed => "FEED_PLAYER_IN_NEED",
            Self::FollowPlayer => "FOLLOW_PLAYER",
            Self::HomeSocial => "HOME_SOCIAL",
            Self::CriticalCraft => "CRITICAL_CRAFT",
            Self::MidPriorityTasks => "MID_PRIORITY_TASKS",
            Self::CraftQueue => "CRAFT_QUEUE",
            Self::ClothingCraft => "CLOTHING_CRAFT",
            Self::AssignedJob => "ASSIGNED_JOB",
            Self::CriticalMisc => "CRITICAL_MISC",
            Self::AgeRotatedJob => "AGE_ROTATED_JOB",
            Self::LowPriorityWork => "LOW_PRIORITY_WORK",
            Self::GoHome => "GO_HOME",
            Self::Idle => "IDLE",
        }
    }

    pub fn band(self) -> PriorityBand {
        match self {
            Self::HeldByOther
            | Self::OrderedDrop
            | Self::DroppingItem
            | Self::CloseUse
            | Self::Wait
            | Self::UsingItem
            | Self::RemoveFromContainer
            | Self::SwitchClothes => PriorityBand::Blocked,
            Self::Escape => PriorityBand::Flee,
            Self::BabyHungryMother
            | Self::Eating
            | Self::ConsiderMakeFood
            | Self::PickupFood => PriorityBand::Food,
            Self::FeedChild | Self::FeedPlayerInNeed => PriorityBand::Feed,
            Self::CriticalCraft
            | Self::MidPriorityTasks
            | Self::CraftQueue
            | Self::ClothingCraft => PriorityBand::Craft,
            Self::AssignedJob
            | Self::AgeRotatedJob
            | Self::LowPriorityWork
            | Self::CriticalMisc => PriorityBand::Job,
            Self::ChildWithMother
            | Self::WoundedSeekHelp
            | Self::OrderedFollow
            | Self::FollowPlayer => PriorityBand::Follow,
            Self::HandleDeath
            | Self::Temperature
            | Self::Combat
            | Self::StayCloseChild
            | Self::KillAnimal
            | Self::HomeSocial
            | Self::GoHome
            | Self::Idle => PriorityBand::Other,
        }
    }
}

impl PriorityBand {
    pub fn as_label(self) -> &'static str {
        match self {
            Self::Blocked => "BLOCKED",
            Self::Flee => "FLEE",
            Self::Food => "FOOD",
            Self::Feed => "FEED",
            Self::Craft => "CRAFT",
            Self::Job => "JOB",
            Self::Follow => "FOLLOW",
            Self::Other => "OTHER",
        }
    }
}

/// Haxe age-rotated job cycle (berry / basic / baking / pottery / sheep).
///
/// Haxe: `jobByAge = Math.round(age/5); for i in 0...5 { (jobByAge+i)%5 → … }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgeRotatedJobKind {
    BerryFarming,
    BasicFarming,
    Baking,
    Pottery,
    SheepHerding,
}

impl AgeRotatedJobKind {
    pub fn as_label(self) -> &'static str {
        match self {
            Self::BerryFarming => "BERRY",
            Self::BasicFarming => "BASIC",
            Self::Baking => "BAKING",
            Self::Pottery => "POTTERY",
            Self::SheepHerding => "SHEEP",
        }
    }

    pub fn from_index(index: u8) -> Self {
        match index % 5 {
            0 => Self::BerryFarming,
            1 => Self::BasicFarming,
            2 => Self::Baking,
            3 => Self::Pottery,
            _ => Self::SheepHerding,
        }
    }
}

/// Input sensors for [`resolve_priority_rung`]. Defaults = idle path.
#[derive(Debug, Clone, PartialEq)]
pub struct PrioritySensors {
    pub held_by_other: bool,
    pub ordered_drop: bool,
    pub threat_near: bool,
    pub food_critically_low: bool,
    pub skip_escape_for_hunt: bool,
    pub dropping_item: bool,
    pub close_use: bool,
    pub waiting: bool,
    pub age: f32,
    pub is_hungry: bool,
    pub has_mother: bool,
    pub is_child_with_mother: bool,
    pub wounded_or_fever: bool,
    pub handling_death: bool,
    pub is_eating: bool,
    pub feeding_child: bool,
    pub need_switch_clothes: bool,
    pub ordered_follow: bool,
    pub considering_make_food: bool,
    pub using_item: bool,
    pub removing_container: bool,
    pub picking_food: bool,
    /// Haxe `superbadTemp = heat < 0.1 || heat > 0.9`.
    pub superbad_temp: bool,
    pub handling_temperature: bool,
    /// Haxe `dist > 100` (quad) — threat far enough to allow temp handling / suppress doStuff.
    pub threat_far_for_temp: bool,
    /// Haxe `doStuff` after superbadTemp policy (see [`compute_do_stuff`]).
    pub do_stuff: bool,
    pub combat_target: bool,
    pub child_to_guard: bool,
    pub killable_animal: bool,
    pub feed_player_need: bool,
    pub smith_blocks_feed: bool,
    /// Haxe `isMovingToPlayer` after feed band (follow target present).
    pub follow_player: bool,
    pub need_home_social: bool,
    pub critical_craft_pending: bool,
    pub mid_tasks_pending: bool,
    pub has_craft_queue: bool,
    pub clothing_craft_pending: bool,
    pub has_assigned_job: bool,
    pub critical_misc: bool,
    pub age_job_pending: bool,
    pub low_work_pending: bool,
    pub away_from_home: bool,
    pub holding_nonwound: bool,
}

impl Default for PrioritySensors {
    fn default() -> Self {
        Self {
            held_by_other: false,
            ordered_drop: false,
            threat_near: false,
            food_critically_low: false,
            skip_escape_for_hunt: false,
            dropping_item: false,
            close_use: false,
            waiting: false,
            age: 20.0,
            is_hungry: false,
            has_mother: false,
            is_child_with_mother: false,
            wounded_or_fever: false,
            handling_death: false,
            is_eating: false,
            feeding_child: false,
            need_switch_clothes: false,
            ordered_follow: false,
            considering_make_food: false,
            using_item: false,
            removing_container: false,
            picking_food: false,
            superbad_temp: false,
            handling_temperature: false,
            threat_far_for_temp: true,
            do_stuff: true,
            combat_target: false,
            child_to_guard: false,
            killable_animal: false,
            feed_player_need: false,
            smith_blocks_feed: false,
            follow_player: false,
            need_home_social: false,
            critical_craft_pending: false,
            mid_tasks_pending: false,
            has_craft_queue: false,
            clothing_craft_pending: false,
            has_assigned_job: false,
            critical_misc: false,
            age_job_pending: false,
            low_work_pending: false,
            away_from_home: false,
            holding_nonwound: false,
        }
    }
}

// ── Hungry / child pure helpers ─────────────────────────────────────────────

/// Haxe `checkIsHungryAndEat` hysteresis update (pure).
// Haxe: AiBase.checkIsHungryAndEat
pub fn update_is_hungry(was_hungry: bool, food: f32, food_max: f32, held_id: i32) -> bool {
    let food_max = food_max.max(1.0);
    if was_hungry {
        return food < food_max * HUNGRY_LEAVE_FRAC;
    }
    let floor = if held_id == SMITHING_HAMMER_ID {
        HUNGRY_ENTER_FLOOR_SMITH_HAMMER
    } else {
        HUNGRY_ENTER_FLOOR
    };
    food < floor.max(food_max * HUNGRY_ENTER_FRAC)
}

/// Side-effect bundle of Haxe `checkIsHungryAndEat` (pure — no world I/O).
///
/// Haxe always clears `isCaringForFire`; babies say `F` under max breastfeed age;
/// `searchFoodAndEat` is requested when hungry and no food target yet.
// Haxe: AiBase.checkIsHungryAndEat
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HungryEatEffects {
    pub is_hungry: bool,
    /// Haxe always: `this.isCaringForFire = false`.
    pub clear_caring_for_fire: bool,
    /// Haxe: hungry && age < MaxChildAgeForBreastFeeding → `say('F')`.
    pub baby_say_f: bool,
    /// Haxe: hungry && foodTarget == null → `searchFoodAndEat()`.
    pub need_search_food: bool,
}

/// Pure `checkIsHungryAndEat` decision + side-effect flags.
///
/// Smith profession wipe lives on `isConsideringMakingFood` (not this helper).
/// Live tick: when `is_hungry` (or food target) and rung is ConsiderMakeFood,
/// call `apply_consider_making_food_smith_wipe` on `Player.smith_profession`
/// (AI-JOB-SMITH-LIVE).
// Haxe: AiBase.checkIsHungryAndEat; isConsideringMakingFood SMITH wipe ~8482
pub fn check_is_hungry_and_eat_effects(
    was_hungry: bool,
    food: f32,
    food_max: f32,
    held_id: i32,
    age: f32,
    has_food_target: bool,
) -> HungryEatEffects {
    let is_hungry = update_is_hungry(was_hungry, food, food_max, held_id);
    HungryEatEffects {
        is_hungry,
        clear_caring_for_fire: true,
        baby_say_f: is_hungry && age < MAX_CHILD_AGE_BREASTFEED,
        need_search_food: is_hungry && !has_food_target,
    }
}

/// Haxe `isChildAndHasMother`.
// Haxe: AiBase.isChildAndHasMother
pub fn is_child_and_has_mother(age: f32, has_mother: bool) -> bool {
    is_child_and_has_mother_ex(age, has_mother, MIN_AGE_TO_EAT)
}

/// Same as [`is_child_and_has_mother`] with live `ServerSettings.MinAgeToEat`.
// Haxe: ServerSettings.MinAgeToEat — C-SS-MIN-AGE-AI
pub fn is_child_and_has_mother_ex(age: f32, has_mother: bool, min_age_to_eat: f32) -> bool {
    let min_age = if min_age_to_eat.is_finite() && min_age_to_eat >= 0.0 {
        min_age_to_eat
    } else {
        MIN_AGE_TO_EAT
    };
    age < min_age && has_mother
}

// ── Age-rotated jobs ────────────────────────────────────────────────────────

/// Age-rotated job index (Haxe `Math.round(age / 5) % 5`).
// Haxe: AiBase.doTimeStuffHelper jobByAge
pub fn age_job_index(age: f32) -> u8 {
    let j = (age / 5.0).round() as i32;
    ((j % 5) + 5) as u8 % 5
}

/// Primary age-rotated job for this age (before the Haxe for-loop fallthrough).
// Haxe: AiBase.doTimeStuffHelper jobByAge
pub fn age_rotated_job_kind(age: f32) -> AgeRotatedJobKind {
    AgeRotatedJobKind::from_index(age_job_index(age))
}

/// Full Haxe try-order: start at `round(age/5)%5`, then +1…+4 mod 5.
// Haxe: AiBase.doTimeStuffHelper for (i in 0...5) jobByAge
pub fn age_rotated_job_sequence(age: f32) -> [AgeRotatedJobKind; 5] {
    let start = age_job_index(age);
    [
        AgeRotatedJobKind::from_index(start),
        AgeRotatedJobKind::from_index(start.wrapping_add(1)),
        AgeRotatedJobKind::from_index(start.wrapping_add(2)),
        AgeRotatedJobKind::from_index(start.wrapping_add(3)),
        AgeRotatedJobKind::from_index(start.wrapping_add(4)),
    ]
}

// ── Superbad temp / doStuff ─────────────────────────────────────────────────

/// Haxe: `superbadTemp = heat < 0.1 || heat > 0.9`.
// Haxe: AiBase.doTimeStuffHelper superbadTemp
pub fn is_superbad_temp(heat: f32) -> bool {
    heat < SUPERBAD_HEAT_LOW || heat > SUPERBAD_HEAT_HIGH
}

/// Haxe: `doStuff = !superbadTemp; if (dist < 100) doStuff = true`.
// Haxe: AiBase.doTimeStuffHelper doStuff
pub fn compute_do_stuff(superbad_temp: bool, threat_quad_dist: f32) -> bool {
    if threat_quad_dist < DO_STUFF_THREAT_QUAD_FORCE {
        true
    } else {
        !superbad_temp
    }
}

/// Haxe: temperature handling only when `dist > 100`.
// Haxe: AiBase.doTimeStuffHelper isHandlingTemperature && dist > 100
pub fn threat_is_far_for_temp(threat_quad_dist: f32) -> bool {
    threat_quad_dist > DO_STUFF_THREAT_QUAD_FORCE
}

/// Effective doStuff: honor explicit sensor flag, but superbad+far forces off.
// Haxe: AiBase.doTimeStuffHelper doStuff / superbadTemp
pub fn effective_do_stuff(s: &PrioritySensors) -> bool {
    if s.superbad_temp && s.threat_far_for_temp {
        return false;
    }
    s.do_stuff
}

// ── Escape pure policy ──────────────────────────────────────────────────────

/// Inputs for pure escape decision (Haxe `AiBase.escape` gates before pathing).
// Haxe: AiBase.escape
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EscapeContext {
    pub has_animal: bool,
    pub has_deadly_player: bool,
    /// Haxe `deadlyPlayer.angryTime`; `> 4` clears player threat.
    pub player_angry_time: f32,
    pub food_store: f32,
    pub holding_weapon: bool,
    pub is_wounded: bool,
    pub age: f32,
    /// Quad distance to deadly animal (Haxe `CalculateQuadDistanceToObject`).
    pub dist_animal_quad: f32,
    /// Distance to deadly player (Haxe `CalculateDistanceToPlayer`, not quad).
    pub dist_player: f32,
    pub has_weapon_close: bool,
    pub did_not_reach_food: f32,
}

impl Default for EscapeContext {
    fn default() -> Self {
        Self {
            has_animal: false,
            has_deadly_player: false,
            player_angry_time: 0.0,
            food_store: 10.0,
            holding_weapon: false,
            is_wounded: false,
            age: 20.0,
            dist_animal_quad: f32::MAX,
            dist_player: f32::MAX,
            has_weapon_close: false,
            did_not_reach_food: 0.0,
        }
    }
}

/// Who to flee from after Haxe gate filters.
// Haxe: AiBase.escape escapePlayer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EscapeThreat {
    None,
    Animal,
    Player,
}

/// Haxe gate: only attempt escape when `didNotReachFood < 5`.
// Haxe: AiBase.doTimeStuff / doTimeStuffHelper didNotReachFood < 5
pub fn should_attempt_escape(did_not_reach_food: f32) -> bool {
    did_not_reach_food < ESCAPE_DID_NOT_REACH_FOOD_MAX
}

/// Haxe: weapon + not wounded + age > 8 → skip escape (hunt mode).
// Haxe: AiBase.escape holding weapon age>8
pub fn skip_escape_for_hunt(holding_weapon: bool, is_wounded: bool, age: f32) -> bool {
    holding_weapon && !is_wounded && age > ESCAPE_HUNT_MIN_AGE
}

/// Pure escape decision matching Haxe pre-path gates.
///
/// Returns [`EscapeThreat::None`] when escape should not run (crit food, hunt skip,
/// no threat, player too far, weapon close, angryTime filter).
// Haxe: AiBase.escape
pub fn resolve_escape_threat(ctx: &EscapeContext) -> EscapeThreat {
    if !should_attempt_escape(ctx.did_not_reach_food) {
        return EscapeThreat::None;
    }

    // Haxe: if (deadlyPlayer != null && deadlyPlayer.angryTime > 4) deadlyPlayer = null;
    let player_active =
        ctx.has_deadly_player && ctx.player_angry_time <= ESCAPE_ANGRY_TIME_IGNORE;

    if !ctx.has_animal && !player_active {
        return EscapeThreat::None;
    }
    // Haxe: if (myPlayer.food_store < -1) return false;
    if ctx.food_store < ESCAPE_FOOD_CRIT_SKIP {
        return EscapeThreat::None;
    }
    // Haxe: weapon + not wounded + age > 8 → hunt
    if skip_escape_for_hunt(ctx.holding_weapon, ctx.is_wounded, ctx.age) {
        return EscapeThreat::None;
    }

    let mut dist_player = if player_active {
        ctx.dist_player
    } else {
        f32::MAX
    };
    // Haxe: if (distPlayer > 64 && animal == null) return false;
    if dist_player > ESCAPE_PLAYER_DIST_MAX && !ctx.has_animal {
        return EscapeThreat::None;
    }
    // Haxe: if (distPlayer > 64) distPlayer = 99999999;
    if dist_player > ESCAPE_PLAYER_DIST_MAX {
        dist_player = f32::MAX;
    }

    // Haxe: if (hasWeaponClose()) return false;
    if ctx.has_weapon_close {
        return EscapeThreat::None;
    }

    let dist_animal = if ctx.has_animal {
        ctx.dist_animal_quad
    } else {
        f32::MAX
    };
    // Haxe: escapePlayer = deadlyPlayer != null && distAnimal > distPlayer;
    let escape_player = player_active && dist_player < f32::MAX && dist_animal > dist_player;
    if escape_player {
        EscapeThreat::Player
    } else if ctx.has_animal {
        EscapeThreat::Animal
    } else if player_active && dist_player <= ESCAPE_PLAYER_DIST_MAX {
        EscapeThreat::Player
    } else {
        EscapeThreat::None
    }
}

/// Nominal flee tile (no RNG / blocked-tile retries) — away from threat by `escape_dist`.
// Haxe: AiBase.escape newEscapetarget.tx/ty base (before rand jitter)
pub fn escape_target_xy(
    player_tx: i32,
    player_ty: i32,
    threat_tx: i32,
    threat_ty: i32,
    escape_dist: i32,
) -> (i32, i32) {
    let escape_in_lower_x = threat_tx > player_tx;
    let escape_in_lower_y = threat_ty > player_ty;
    let tx = if escape_in_lower_x {
        player_tx - escape_dist
    } else {
        player_tx + escape_dist
    };
    let ty = if escape_in_lower_y {
        player_ty - escape_dist
    } else {
        player_ty + escape_dist
    };
    (tx, ty)
}

/// Side effects when escape commits (Haxe cancels use/food/craft on hostile path).
// Haxe: AiBase.escape CancleUse / foodTarget / itemToCraft.trans*
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EscapeSideEffects {
    pub cancel_use: bool,
    pub clear_food_target: bool,
    pub clear_craft_trans: bool,
    /// Haxe: if foodTarget != null → `didNotReachFood++`.
    pub increment_did_not_reach_food: bool,
}

/// Haxe: if any of use/food/escape targets set, cancel use + clear food/craft.
// Haxe: AiBase.escape useTarget/foodTarget/escapeTarget cleanup
pub fn escape_side_effects(
    had_use_target: bool,
    had_food_target: bool,
    had_escape_target: bool,
) -> EscapeSideEffects {
    let any = had_use_target || had_food_target || had_escape_target;
    EscapeSideEffects {
        cancel_use: any,
        clear_food_target: any,
        clear_craft_trans: any,
        increment_did_not_reach_food: had_food_target,
    }
}

/// Map escape threat → ladder sensors for threat_near / skip / food_crit.
// Haxe: AiBase.escape → sensors for resolve_priority_rung
pub fn apply_escape_to_sensors(s: &mut PrioritySensors, threat: EscapeThreat, food_store: f32) {
    s.threat_near = threat != EscapeThreat::None;
    s.food_critically_low = food_store < ESCAPE_FOOD_CRIT_SKIP;
    s.skip_escape_for_hunt = false; // already applied inside resolve_escape_threat
    s.threat_far_for_temp = !s.threat_near;
}

// ── Ladder resolve ──────────────────────────────────────────────────────────

/// Walk the Haxe ladder; return the first matching rung (or Idle).
// Haxe: AiBase.doTimeStuffHelper
pub fn resolve_priority_rung(s: &PrioritySensors) -> PriorityRung {
    if s.held_by_other {
        return PriorityRung::HeldByOther;
    }
    if s.ordered_drop {
        return PriorityRung::OrderedDrop;
    }
    // Haxe: escape when threat and not crit food and not hunt-skip
    if s.threat_near && !s.food_critically_low && !s.skip_escape_for_hunt {
        return PriorityRung::Escape;
    }
    if s.dropping_item {
        return PriorityRung::DroppingItem;
    }
    if s.close_use {
        return PriorityRung::CloseUse;
    }
    if s.waiting && !s.threat_near {
        return PriorityRung::Wait;
    }
    // C-SS-MIN-AGE-AI: is_child_with_mother already embeds live MinAgeToEat from sensors fill
    if s.is_child_with_mother && s.is_hungry {
        return PriorityRung::BabyHungryMother;
    }
    if s.is_child_with_mother || is_child_and_has_mother(s.age, s.has_mother) {
        return PriorityRung::ChildWithMother;
    }
    if s.wounded_or_fever {
        return PriorityRung::WoundedSeekHelp;
    }
    if s.handling_death && !s.threat_near {
        return PriorityRung::HandleDeath;
    }
    if s.is_eating {
        return PriorityRung::Eating;
    }
    if s.feeding_child {
        return PriorityRung::FeedChild;
    }
    if s.need_switch_clothes {
        return PriorityRung::SwitchClothes;
    }
    if s.ordered_follow {
        return PriorityRung::OrderedFollow;
    }
    if s.considering_make_food && !s.threat_near {
        return PriorityRung::ConsiderMakeFood;
    }
    if s.using_item {
        return PriorityRung::UsingItem;
    }
    if s.removing_container {
        return PriorityRung::RemoveFromContainer;
    }
    if s.picking_food {
        return PriorityRung::PickupFood;
    }
    // Haxe: isHandlingTemperature && dist > 100
    if s.handling_temperature && s.threat_far_for_temp {
        return PriorityRung::Temperature;
    }

    // Haxe doStuff gates combat / child / kill / feed
    let do_stuff = effective_do_stuff(s);

    if do_stuff && s.combat_target {
        return PriorityRung::Combat;
    }
    if do_stuff && s.child_to_guard {
        return PriorityRung::StayCloseChild;
    }
    if do_stuff && s.killable_animal {
        return PriorityRung::KillAnimal;
    }
    if do_stuff && s.feed_player_need && !s.smith_blocks_feed {
        return PriorityRung::FeedPlayerInNeed;
    }
    // Haxe: isMovingToPlayer after feed, before home-social
    if s.follow_player {
        return PriorityRung::FollowPlayer;
    }
    if s.need_home_social {
        return PriorityRung::HomeSocial;
    }
    if s.critical_craft_pending {
        return PriorityRung::CriticalCraft;
    }
    if s.mid_tasks_pending {
        return PriorityRung::MidPriorityTasks;
    }
    if s.has_craft_queue {
        return PriorityRung::CraftQueue;
    }
    if s.clothing_craft_pending {
        return PriorityRung::ClothingCraft;
    }
    if s.has_assigned_job {
        return PriorityRung::AssignedJob;
    }
    if s.critical_misc {
        return PriorityRung::CriticalMisc;
    }
    if s.age_job_pending {
        return PriorityRung::AgeRotatedJob;
    }
    if s.low_work_pending {
        return PriorityRung::LowPriorityWork;
    }
    if s.away_from_home {
        return PriorityRung::GoHome;
    }
    let _ = s.holding_nonwound;
    PriorityRung::Idle
}

/// Map ladder rung → existing self-play [`Goal`] (action layer).
///
/// Band mapping until dedicated action goals land:
/// Feed → SeekFood; Follow/home → Explore; Craft/Job → profession SeekObject/etc.
// Haxe: AiBase.doTimeStuffHelper → action layer (coarse Goal)
pub fn goal_from_rung(
    rung: PriorityRung,
    profession: Profession,
    held_id: i32,
    nearby_food: bool,
    prey_adjacent: bool,
    on_grassland: bool,
) -> Goal {
    match rung {
        PriorityRung::Escape => Goal::Flee,
        PriorityRung::BabyHungryMother
        | PriorityRung::Eating
        | PriorityRung::ConsiderMakeFood
        | PriorityRung::PickupFood
        | PriorityRung::FeedChild
        | PriorityRung::FeedPlayerInNeed => Goal::SeekFood,
        PriorityRung::ChildWithMother
        | PriorityRung::WoundedSeekHelp
        | PriorityRung::OrderedFollow
        | PriorityRung::FollowPlayer
        | PriorityRung::StayCloseChild
        | PriorityRung::HomeSocial
        | PriorityRung::GoHome => Goal::Explore,
        PriorityRung::Combat | PriorityRung::KillAnimal => {
            if profession == Profession::Hunter || prey_adjacent {
                Goal::Hunt
            } else {
                Goal::Flee
            }
        }
        PriorityRung::CraftQueue
        | PriorityRung::CriticalCraft
        | PriorityRung::ClothingCraft
        | PriorityRung::MidPriorityTasks
        | PriorityRung::AssignedJob
        | PriorityRung::AgeRotatedJob
        | PriorityRung::LowPriorityWork
        | PriorityRung::CriticalMisc => match profession {
            // Thin default without map snapshot. Live tick / selfplay with a tile scan should
            // call `farm_goal_from_map_and_rung` (fill → try_decide → farm_action_to_goal)
            // or `farm_goal_from_counts_and_rung` with sticky `Player.farm_profession` /
            // `Player.farm_task` (AI-JOB-FARM-WIRE / AI-JOB-FARM-LIVE).
            // Haxe: AssignedJob BASICFARMER → doBasicFarming(100); AgeRotated berry/basic
            Profession::Farmer => Goal::SeekObject(FARMER_TARGET_ID),
            // Thin default without map snapshot. Live tick prefers
            // `smith_goal_from_map_and_rung` / `try_decide_smith_from_rung` +
            // `smith_action_to_goal` / `smith_action_apply` and sticky
            // `Player.smith_profession` (AI-JOB-SMITH-WIRE / LIVE).
            // Early sticky: rung label `EARLY_STICKY_SMITH` → EarlySticky slot.
            // Hungry: `apply_consider_making_food_smith_wipe` on ConsiderMakeFood.
            // Haxe: AssignedJob/last SMITH → doSmithing(100); open doSmithing()
            Profession::Smith => Goal::SeekObject(SMITH_TARGET_ID),
            // Thin default without map snapshot. Live tick prefers
            // `baker_goal_from_map_and_rung` / `try_decide_baker_from_rung` +
            // `bake_action_to_goal` and sticky `Player.baker_profession` /
            // `Player.baker_task` (AI-JOB-BAKER-WIRE).
            // Haxe: AssignedJob BAKER → doBaking(100); AgeRotated Baking → doBaking()
            Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID),
            // Haxe: AssignedJob POTTER → doPottery(100); AgeRotated pottery → doPottery()
            Profession::Potter => Goal::SeekObject(POTTER_TARGET_ID),
            // Haxe: AssignedJob SHEPHERD → isSheepHerding(100); AgeRotated sheep → isSheepHerding()
            Profession::Shepherd => Goal::SeekObject(SHEPHERD_TARGET_ID),
            Profession::Hunter if prey_adjacent => Goal::Hunt,
            Profession::Forager if on_grassland => Goal::Harvest,
            Profession::Forager if nearby_food => Goal::SeekFood,
            _ => Goal::Explore,
        },
        PriorityRung::Temperature
        | PriorityRung::HandleDeath
        | PriorityRung::HeldByOther
        | PriorityRung::Wait
        | PriorityRung::DroppingItem
        | PriorityRung::CloseUse
        | PriorityRung::OrderedDrop
        | PriorityRung::UsingItem
        | PriorityRung::RemoveFromContainer
        | PriorityRung::SwitchClothes => Goal::Idle,
        PriorityRung::Idle => {
            if held_id != 0 {
                return Goal::Idle;
            }
            match profession {
                Profession::Forager => {
                    if on_grassland {
                        Goal::Harvest
                    } else if nearby_food {
                        Goal::SeekFood
                    } else {
                        Goal::Explore
                    }
                }
                Profession::Farmer => Goal::SeekObject(FARMER_TARGET_ID),
                Profession::Smith => Goal::SeekObject(SMITH_TARGET_ID),
                Profession::Baker => Goal::SeekObject(BAKER_TARGET_ID),
                Profession::Potter => Goal::SeekObject(POTTER_TARGET_ID),
                Profession::Shepherd => Goal::SeekObject(SHEPHERD_TARGET_ID),
                Profession::Explorer => Goal::Explore,
                Profession::Hunter => {
                    if prey_adjacent {
                        Goal::Hunt
                    } else if nearby_food {
                        Goal::SeekFood
                    } else {
                        Goal::Explore
                    }
                }
            }
        }
    }
}

/// Optional live-state enrichments for [`sensors_from_ext`].
///
/// Defaults leave mid/low bands off so the simple self-play path stays stable.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveSensorExtras {
    pub heat: Option<f32>,
    /// Closest threat quad distance; `None` → treat as far (no threat).
    pub threat_quad_dist: Option<f32>,
    pub follow_player: bool,
    pub feed_player_need: bool,
    pub smith_blocks_feed: bool,
    pub combat_target: bool,
    pub child_to_guard: bool,
    pub killable_animal: bool,
    pub has_craft_queue: bool,
    pub clothing_craft_pending: bool,
    pub critical_craft_pending: bool,
    pub mid_tasks_pending: bool,
    pub has_assigned_job: bool,
    pub age_job_pending: bool,
    pub critical_misc: bool,
    pub low_work_pending: bool,
    pub away_from_home: bool,
    pub need_home_social: bool,
    pub holding_weapon: bool,
    pub is_wounded: bool,
    pub feeding_child: bool,
    pub is_eating: bool,
    pub handling_temperature: bool,
    pub handling_death: bool,
    pub ordered_follow: bool,
    pub ordered_drop: bool,
    pub held_by_other: bool,
    pub dropping_item: bool,
    pub close_use: bool,
    pub waiting: bool,
    pub using_item: bool,
    pub removing_container: bool,
    pub need_switch_clothes: bool,
    pub wounded_or_fever: bool,
}

impl Default for LiveSensorExtras {
    fn default() -> Self {
        Self {
            heat: None,
            threat_quad_dist: None,
            follow_player: false,
            feed_player_need: false,
            smith_blocks_feed: false,
            combat_target: false,
            child_to_guard: false,
            killable_animal: false,
            has_craft_queue: false,
            clothing_craft_pending: false,
            critical_craft_pending: false,
            mid_tasks_pending: false,
            has_assigned_job: false,
            age_job_pending: false,
            critical_misc: false,
            low_work_pending: false,
            away_from_home: false,
            need_home_social: false,
            holding_weapon: false,
            is_wounded: false,
            feeding_child: false,
            is_eating: false,
            handling_temperature: false,
            handling_death: false,
            ordered_follow: false,
            ordered_drop: false,
            held_by_other: false,
            dropping_item: false,
            close_use: false,
            waiting: false,
            using_item: false,
            removing_container: false,
            need_switch_clothes: false,
            wounded_or_fever: false,
        }
    }
}

/// Build sensors for the simplified self-play path.
// Haxe: AiBase.doTimeStuffHelper simplified sensors
pub fn sensors_from_simple(
    held_id: i32,
    food: f32,
    threat_near: bool,
    nearby_food: bool,
    age: f32,
    has_mother: bool,
    food_max: f32,
    was_hungry: bool,
) -> PrioritySensors {
    sensors_from_ext(
        held_id,
        food,
        threat_near,
        nearby_food,
        age,
        has_mother,
        food_max,
        was_hungry,
        &LiveSensorExtras::default(),
    )
}

/// Full sensor fill from simple core + optional live extras.
// Haxe: AiBase.doTimeStuffHelper sensor fill
pub fn sensors_from_ext(
    held_id: i32,
    food: f32,
    threat_near: bool,
    nearby_food: bool,
    age: f32,
    has_mother: bool,
    food_max: f32,
    was_hungry: bool,
    extra: &LiveSensorExtras,
) -> PrioritySensors {
    sensors_from_ext_ex(
        held_id,
        food,
        threat_near,
        nearby_food,
        age,
        has_mother,
        food_max,
        was_hungry,
        extra,
        MIN_AGE_TO_EAT,
    )
}

/// Same as [`sensors_from_ext`] with live `ServerSettings.MinAgeToEat`.
// Haxe: ServerSettings.MinAgeToEat — C-SS-MIN-AGE-AI
pub fn sensors_from_ext_ex(
    held_id: i32,
    food: f32,
    threat_near: bool,
    nearby_food: bool,
    age: f32,
    has_mother: bool,
    food_max: f32,
    was_hungry: bool,
    extra: &LiveSensorExtras,
    min_age_to_eat: f32,
) -> PrioritySensors {
    let min_age = if min_age_to_eat.is_finite() && min_age_to_eat >= 0.0 {
        min_age_to_eat
    } else {
        MIN_AGE_TO_EAT
    };
    let is_hungry = update_is_hungry(was_hungry, food, food_max, held_id);
    let child = is_child_and_has_mother_ex(age, has_mother, min_age);
    let superbad = extra.heat.map(is_superbad_temp).unwrap_or(false);
    let threat_qd = extra.threat_quad_dist.unwrap_or(if threat_near {
        50.0 // near enough to force doStuff
    } else {
        10_000.0
    });
    let far = threat_is_far_for_temp(threat_qd);
    let do_stuff = compute_do_stuff(superbad, threat_qd);
    let hunt_skip = skip_escape_for_hunt(extra.holding_weapon, extra.is_wounded, age);

    PrioritySensors {
        held_by_other: extra.held_by_other,
        ordered_drop: extra.ordered_drop,
        threat_near,
        food_critically_low: food < ESCAPE_FOOD_CRIT_SKIP,
        skip_escape_for_hunt: hunt_skip,
        dropping_item: extra.dropping_item,
        close_use: extra.close_use,
        waiting: extra.waiting,
        age,
        is_hungry,
        has_mother,
        is_child_with_mother: child,
        wounded_or_fever: extra.wounded_or_fever || extra.is_wounded,
        handling_death: extra.handling_death,
        is_eating: extra.is_eating,
        feeding_child: extra.feeding_child,
        need_switch_clothes: extra.need_switch_clothes,
        ordered_follow: extra.ordered_follow,
        considering_make_food: is_hungry && age >= min_age && !nearby_food && held_id == 0,
        using_item: extra.using_item,
        removing_container: extra.removing_container,
        picking_food: is_hungry && age >= min_age && held_id == 0 && nearby_food,
        superbad_temp: superbad,
        handling_temperature: extra.handling_temperature,
        threat_far_for_temp: far,
        do_stuff,
        combat_target: extra.combat_target,
        child_to_guard: extra.child_to_guard,
        killable_animal: extra.killable_animal,
        feed_player_need: extra.feed_player_need,
        smith_blocks_feed: extra.smith_blocks_feed,
        follow_player: extra.follow_player,
        need_home_social: extra.need_home_social,
        critical_craft_pending: extra.critical_craft_pending,
        mid_tasks_pending: extra.mid_tasks_pending,
        has_craft_queue: extra.has_craft_queue,
        clothing_craft_pending: extra.clothing_craft_pending,
        has_assigned_job: extra.has_assigned_job,
        critical_misc: extra.critical_misc,
        age_job_pending: extra.age_job_pending,
        low_work_pending: extra.low_work_pending,
        away_from_home: extra.away_from_home,
        holding_nonwound: held_id != 0,
    }
}

/// Full ladder resolve → Goal (AI-PRIO entry).
// Haxe: AiBase.doTimeStuffHelper → Goal
pub fn pick_goal_from_ladder(
    profession: Profession,
    held_id: i32,
    food: f32,
    nearby_food: bool,
    threat_near: bool,
    prey_adjacent: bool,
    on_grassland: bool,
    age: f32,
    has_mother: bool,
    food_max: f32,
    was_hungry: bool,
) -> (PriorityRung, Goal) {
    let sensors = sensors_from_simple(
        held_id,
        food,
        threat_near,
        nearby_food,
        age,
        has_mother,
        food_max,
        was_hungry,
    );
    pick_goal_with_sensors(
        &sensors,
        profession,
        held_id,
        nearby_food,
        prey_adjacent,
        on_grassland,
    )
}

/// Resolve rung + goal from already-filled sensors (live AI / tests).
// Haxe: AiBase.doTimeStuffHelper → Goal
pub fn pick_goal_with_sensors(
    sensors: &PrioritySensors,
    profession: Profession,
    held_id: i32,
    nearby_food: bool,
    prey_adjacent: bool,
    on_grassland: bool,
) -> (PriorityRung, Goal) {
    let rung = resolve_priority_rung(sensors);
    let goal = goal_from_rung(
        rung,
        profession,
        held_id,
        nearby_food,
        prey_adjacent,
        on_grassland,
    );
    (rung, goal)
}

/// Approx parity helper: simple hungry using fixed [`HUNGRY_FOOD`].
pub fn is_hungry_simple(food: f32) -> bool {
    food <= HUNGRY_FOOD
}

// ── Live world sensor fill (AI-PRIO-LIVE / sensors_world_fill) ───────────────

/// Haxe `GetCloseDeadlyPlayer` default search (AiHelper); doTimeStuffHelper uses 30.
// Haxe: AiHelper.GetCloseDeadlyPlayer searchDistance
pub const DEADLY_PLAYER_SEARCH_DIST: i32 = 8;
/// Haxe doTimeStuffHelper: `GetCloseDeadlyPlayer(myPlayer, 30)`.
// Haxe: AiBase.doTimeStuffHelper GetCloseDeadlyPlayer(_, 30)
pub const DEADLY_PLAYER_SEARCH_DIST_AI: i32 = 30;
/// Haxe `GetClosePlayerTarget` default searchDistance.
// Haxe: AiHelper.GetClosePlayerTarget
pub const PLAYER_TARGET_SEARCH_DIST: i32 = 20;
/// Exile near home is dangerous when `quadDist < 1600` (~40 tiles).
// Haxe: AiHelper.GetCloseDeadlyPlayerHelper quadDistanceToHome < 1600
pub const EXILE_HOME_QUAD_DANGER: f32 = 1600.0;
/// Devil Mask clothing parent id.
// Haxe: AiHelper.GetClosePlayerTarget clothing 3213
pub const DEVIL_MASK_ID: i32 = 3213;
/// Goblin Mask clothing parent id.
// Haxe: AiHelper.GetClosePlayerTarget clothing 3214
pub const GOBLIN_MASK_ID: i32 = 3214;
/// Blue-mask targets only when their home quad dist ≤ 400.
// Haxe: AiHelper.GetClosePlayerTargetHelper distanceToHome > 400 skip
pub const BLUE_MASK_HOME_QUAD_MAX: f32 = 400.0;
/// Angry-time threshold for weapon danger (same as escape ignore, but inverted).
// Haxe: GetCloseDeadlyPlayerHelper angryTime < 4
pub const DEADLY_PLAYER_ANGRY_ACTIVE: f32 = 4.0;

/// Squared Euclidean tile distance (Haxe `CalculateDistanceToPlayer` / quad helpers).
// Haxe: AiHelper.CalculateDistanceToPlayer / CalculateQuadDistanceToObject
#[inline]
pub fn player_quad_dist(ax: i32, ay: i32, bx: i32, by: i32) -> f32 {
    let dx = (ax - bx) as f32;
    let dy = (ay - by) as f32;
    dx * dx + dy * dy
}

/// Snapshot of another player for pure deadly-player scan (no world I/O).
// Haxe: AiHelper.GetCloseDeadlyPlayerHelper
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeadlyPlayerCandidate {
    pub p_id: i32,
    pub x: i32,
    pub y: i32,
    pub deleted: bool,
    pub age: f32,
    pub angry_time: f32,
    pub lost_combat_prestige: f32,
    pub is_cursed: bool,
    pub is_ai: bool,
    pub holding_weapon: bool,
    pub held_is_bloody: bool,
    /// Precomputed: exiled by any of observer's leaders (`isExiledByAnyLeaderFrom`).
    pub exiled_by_observer_leaders: bool,
    /// Precomputed: `isFriendly(observer)`.
    pub is_friendly: bool,
}

/// Closest dangerous player from [`get_close_deadly_player`].
// Haxe: AiHelper.GetCloseDeadlyPlayer → GlobalPlayerInstance
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CloseDeadlyPlayer {
    pub p_id: i32,
    pub x: i32,
    pub y: i32,
    /// Priority-adjusted distance (cursed / prestige may shrink it).
    pub dist: f32,
    pub angry_time: f32,
    pub lost_combat_prestige: f32,
}

/// Whether a candidate counts as dangerous for the observer (pre-distance).
// Haxe: AiHelper.GetCloseDeadlyPlayerHelper dangerous flags
pub fn is_deadly_player_candidate(
    c: &DeadlyPlayerCandidate,
    observer_angry_time: f32,
    observer_home_x: i32,
    observer_home_y: i32,
) -> bool {
    if c.deleted {
        return false;
    }
    let mut dangerous = false;

    // Haxe: exiled + close to my home
    if c.exiled_by_observer_leaders {
        let qd = player_quad_dist(c.x, c.y, observer_home_x, observer_home_y);
        if qd < EXILE_HOME_QUAD_DANGER {
            dangerous = true;
        }
    }
    if c.held_is_bloody {
        dangerous = true;
    }
    if c.lost_combat_prestige > 4.0 {
        dangerous = true;
    }
    // Haxe: weapon + (their angry < 4 || my angry < 4)
    if c.holding_weapon
        && (c.angry_time < DEADLY_PLAYER_ANGRY_ACTIVE
            || observer_angry_time < DEADLY_PLAYER_ANGRY_ACTIVE)
    {
        dangerous = true;
    }
    // Haxe: cursed human age > 5
    if c.is_cursed && !c.is_ai && c.age > 5.0 {
        dangerous = true;
    }
    if c.lost_combat_prestige > 5.0 {
        dangerous = true;
    }
    if !dangerous {
        return false;
    }
    // Haxe: if (p.isFriendly(player)) continue;
    if c.is_friendly {
        return false;
    }
    true
}

/// Haxe `AiHelper.GetCloseDeadlyPlayerHelper` — pure scan over candidates.
// Haxe: AiHelper.GetCloseDeadlyPlayer / GetCloseDeadlyPlayerHelper
pub fn get_close_deadly_player(
    observer_x: i32,
    observer_y: i32,
    observer_angry_time: f32,
    observer_home_x: i32,
    observer_home_y: i32,
    search_distance: i32,
    candidates: &[DeadlyPlayerCandidate],
) -> Option<CloseDeadlyPlayer> {
    let search = search_distance.max(0);
    let mut best_dist = (search * search) as f32;
    let mut best: Option<CloseDeadlyPlayer> = None;
    for c in candidates {
        if !is_deadly_player_candidate(
            c,
            observer_angry_time,
            observer_home_x,
            observer_home_y,
        ) {
            continue;
        }
        let mut dist = player_quad_dist(observer_x, observer_y, c.x, c.y);
        // Haxe: cursed → dist /= 2; prestige → dist /= (15+prestige)/10
        if c.is_cursed {
            dist /= 2.0;
        }
        if c.lost_combat_prestige > 1.0 {
            dist /= (15.0 + c.lost_combat_prestige) / 10.0;
        }
        if dist > best_dist {
            continue;
        }
        best_dist = dist;
        best = Some(CloseDeadlyPlayer {
            p_id: c.p_id,
            x: c.x,
            y: c.y,
            dist,
            angry_time: c.angry_time,
            lost_combat_prestige: c.lost_combat_prestige,
        });
    }
    best
}

/// Snapshot for pure combat-target pick (mask / darkNosaj path).
// Haxe: AiHelper.GetClosePlayerTargetHelper
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerTargetCandidate {
    pub p_id: i32,
    pub x: i32,
    pub y: i32,
    pub deleted: bool,
    pub age: f32,
    pub is_same_family: bool,
    pub is_ally: bool,
    /// Haxe `player.getTopLeader(p) == p`.
    pub is_top_leader: bool,
    pub lost_combat_prestige: f32,
    pub is_cursed: bool,
    /// Target's quad distance to *observer's* home (blue mask filter).
    pub target_home_quad: f32,
}

/// Closest combat target from [`get_close_player_target`].
// Haxe: AiHelper.GetClosePlayerTarget
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClosePlayerTarget {
    pub p_id: i32,
    pub x: i32,
    pub y: i32,
    pub dist: f32,
}

/// Haxe `GetClosePlayerTargetHelper` — mask/darkNosaj combat pick (pure).
// Haxe: AiHelper.GetClosePlayerTarget / GetClosePlayerTargetHelper
pub fn get_close_player_target(
    has_red_mask: bool,
    has_blue_mask: bool,
    dark_nosaj: f32,
    observer_age: f32,
    observer_x: i32,
    observer_y: i32,
    search_distance: i32,
    candidates: &[PlayerTargetCandidate],
) -> Option<ClosePlayerTarget> {
    // Haxe: no mask and darkNosaj <= 0 → null
    if !has_red_mask && !has_blue_mask && dark_nosaj <= 0.0 {
        return None;
    }
    let mut search = search_distance.max(0);
    // Haxe: age < 10 → searchDistance = 4
    if observer_age < 10.0 {
        search = 4;
    }
    let mut best_dist = (search * search) as f32;
    let mut best: Option<ClosePlayerTarget> = None;
    for c in candidates {
        if c.deleted {
            continue;
        }
        if c.age < 4.0 {
            continue;
        }
        if c.is_same_family {
            continue;
        }
        // Haxe: without red mask, skip allies
        if !has_red_mask && c.is_ally {
            continue;
        }
        if !c.is_top_leader {
            continue;
        }
        if has_blue_mask && c.target_home_quad > BLUE_MASK_HOME_QUAD_MAX {
            continue;
        }
        let mut dist = player_quad_dist(observer_x, observer_y, c.x, c.y);
        if c.lost_combat_prestige > 1.0 {
            dist /= (15.0 + c.lost_combat_prestige) / 10.0;
        }
        if c.is_cursed {
            dist /= 2.0;
        }
        if dist > best_dist {
            continue;
        }
        best_dist = dist;
        best = Some(ClosePlayerTarget {
            p_id: c.p_id,
            x: c.x,
            y: c.y,
            dist,
        });
    }
    best
}

/// Haxe `isMovingToPlayer` distance gate: need move when `quadDist >= maxTiles²`.
// Haxe: AiBase.isMovingToPlayer maxDistance *= maxDistance; quadDistance < max → stay
pub fn is_moving_to_player_needed(quad_dist: f32, max_distance_tiles: i32) -> bool {
    let max_q = (max_distance_tiles.max(0) * max_distance_tiles.max(0)) as f32;
    quad_dist >= max_q
}

/// Follow radius for hungry baby (Haxe `isMovingToPlayer(5)` when age < MinAgeToEat && hungry).
// Haxe: AiBase.doTimeStuffHelper baby hungry isMovingToPlayer(5)
pub fn baby_hungry_follow_tiles() -> i32 {
    5
}

/// Follow radius for child-with-mother (nice baby 2, else 4).
// Haxe: AiBase.doTimeStuffHelper isChildAndHasMother tiles = isNiceBaby ? 2 : 4
pub fn child_with_mother_follow_tiles(is_nice_baby: bool) -> i32 {
    if is_nice_baby {
        2
    } else {
        4
    }
}

/// Ordered-follow vs auto-stop follow max tiles (Haxe `autoStopFollow ? 10 : 5`).
// Haxe: AiBase.doTimeStuffHelper isMovingToPlayer(autoStopFollow ? 10 : 5)
pub fn ordered_follow_max_tiles(auto_stop_follow: bool) -> i32 {
    if auto_stop_follow {
        10
    } else {
        5
    }
}

/// Follow radius for wounded / yellow-fever seek help (Haxe `isMovingToPlayer(2)`).
// Haxe: AiBase.doTimeStuffHelper isWounded/hasYellowFever isMovingToPlayer(2)
// AI-FOLLOW-ACQUIRE / continuous_follow bands
pub fn wounded_follow_tiles() -> i32 {
    2
}

/// Inputs gathered from world/players for one AI sensor fill (no I/O inside fill).
// Haxe: AiBase.doTimeStuffHelper deadlyAnimal/deadlyPlayer/heat/mother fill
#[derive(Debug, Clone, PartialEq)]
pub struct LiveSensorInput {
    pub held_id: i32,
    pub food: f32,
    pub food_max: f32,
    pub was_hungry: bool,
    pub age: f32,
    /// Haxe `ServerSettings.MinAgeToEat` (years). Default 3; live via GameplayKnobs.
    // Haxe: ServerSettings.MinAgeToEat — C-SS-MIN-AGE-AI
    pub min_age_to_eat: f32,
    pub heat: f32,
    pub has_mother: bool,
    /// Active follow target (ordered follow or mother / auto-follow).
    pub follow_player: bool,
    pub ordered_follow: bool,
    /// Closest deadly animal: `(x, y, dist_quad)`.
    pub deadly_animal: Option<(i32, i32, f32)>,
    /// Closest deadly player: `(x, y, dist, angry_time)`.
    pub deadly_player: Option<(i32, i32, f32, f32)>,
    pub nearby_food: bool,
    pub holding_weapon: bool,
    pub is_wounded: bool,
    pub did_not_reach_food: f32,
    pub has_weapon_close: bool,
    pub has_assigned_job: bool,
    pub has_craft_queue: bool,
    pub killable_animal: bool,
    pub combat_target: bool,
    pub held_by_other: bool,
    pub feeding_child: bool,
    pub is_eating: bool,
    pub feed_player_need: bool,
    pub smith_blocks_feed: bool,
    pub critical_craft_pending: bool,
    pub clothing_craft_pending: bool,
    pub mid_tasks_pending: bool,
    pub age_job_pending: bool,
    pub critical_misc: bool,
    pub low_work_pending: bool,
    pub away_from_home: bool,
    pub need_home_social: bool,
    pub child_to_guard: bool,
    pub wounded_or_fever: bool,
    pub handling_temperature: bool,
    pub handling_death: bool,
    pub ordered_drop: bool,
    pub dropping_item: bool,
    pub close_use: bool,
    pub waiting: bool,
    pub using_item: bool,
    pub removing_container: bool,
    pub need_switch_clothes: bool,
}

impl Default for LiveSensorInput {
    fn default() -> Self {
        Self {
            held_id: 0,
            food: 15.0,
            food_max: 20.0,
            was_hungry: false,
            age: 20.0,
            min_age_to_eat: MIN_AGE_TO_EAT,
            heat: 0.5,
            has_mother: false,
            follow_player: false,
            ordered_follow: false,
            deadly_animal: None,
            deadly_player: None,
            nearby_food: false,
            holding_weapon: false,
            is_wounded: false,
            did_not_reach_food: 0.0,
            has_weapon_close: false,
            has_assigned_job: false,
            has_craft_queue: false,
            killable_animal: false,
            combat_target: false,
            held_by_other: false,
            feeding_child: false,
            is_eating: false,
            feed_player_need: false,
            smith_blocks_feed: false,
            critical_craft_pending: false,
            clothing_craft_pending: false,
            mid_tasks_pending: false,
            age_job_pending: false,
            critical_misc: false,
            low_work_pending: false,
            away_from_home: false,
            need_home_social: false,
            child_to_guard: false,
            wounded_or_fever: false,
            handling_temperature: false,
            handling_death: false,
            ordered_drop: false,
            dropping_item: false,
            close_use: false,
            waiting: false,
            using_item: false,
            removing_container: false,
            need_switch_clothes: false,
        }
    }
}

/// Output of [`fill_live_sensors`] — extras, escape, and ready [`PrioritySensors`].
// Haxe: AiBase.doTimeStuffHelper sensor state after GetCloseDeadly*
#[derive(Debug, Clone, PartialEq)]
pub struct LiveSensorBundle {
    pub extras: LiveSensorExtras,
    pub threat_near: bool,
    pub threat_quad_dist: f32,
    pub is_hungry: bool,
    pub escape_ctx: EscapeContext,
    pub escape_threat: EscapeThreat,
    pub sensors: PrioritySensors,
}

/// Build [`EscapeContext`] from live deadly animal/player sensors.
// Haxe: AiBase.escape(animal, deadlyPlayer) gates
pub fn escape_context_from_threats(
    deadly_animal: Option<(i32, i32, f32)>,
    deadly_player: Option<(i32, i32, f32, f32)>,
    food_store: f32,
    holding_weapon: bool,
    is_wounded: bool,
    age: f32,
    did_not_reach_food: f32,
    has_weapon_close: bool,
) -> EscapeContext {
    EscapeContext {
        has_animal: deadly_animal.is_some(),
        has_deadly_player: deadly_player.is_some(),
        player_angry_time: deadly_player.map(|p| p.3).unwrap_or(0.0),
        food_store,
        holding_weapon,
        is_wounded,
        age,
        dist_animal_quad: deadly_animal.map(|a| a.2).unwrap_or(f32::MAX),
        dist_player: deadly_player.map(|p| p.2).unwrap_or(f32::MAX),
        has_weapon_close,
        did_not_reach_food,
    }
}

/// Threat quad distance used for superbad / doStuff (Haxe dist from player then animal).
// Haxe: AiBase.doTimeStuffHelper dist = deadlyPlayer … else deadlyAnimal
pub fn threat_quad_from_deadly(
    deadly_player: Option<(i32, i32, f32, f32)>,
    deadly_animal: Option<(i32, i32, f32)>,
) -> f32 {
    if let Some((_, _, d, _)) = deadly_player {
        return d;
    }
    if let Some((_, _, d)) = deadly_animal {
        return d;
    }
    10_000.0
}

/// Fill [`LiveSensorExtras`] + escape + [`PrioritySensors`] from a world snapshot input.
///
/// Pure — callers gather animal/player/mother/heat from World / AnimalWorld / players.
// Haxe: AiBase.doTimeStuffHelper GetCloseDeadly* + checkIsHungryAndEat + sensors
pub fn fill_live_sensors(input: &LiveSensorInput) -> LiveSensorBundle {
    let threat_qd = threat_quad_from_deadly(input.deadly_player, input.deadly_animal);
    let escape_ctx = escape_context_from_threats(
        input.deadly_animal,
        input.deadly_player,
        input.food,
        input.holding_weapon,
        input.is_wounded,
        input.age,
        input.did_not_reach_food,
        input.has_weapon_close,
    );
    let escape_threat = resolve_escape_threat(&escape_ctx);
    // Haxe: threat_near when escape would run, or raw deadly present before hunt-skip
    let raw_threat = input.deadly_animal.is_some() || input.deadly_player.is_some();
    let threat_near = escape_threat != EscapeThreat::None
        || (raw_threat
            && !skip_escape_for_hunt(input.holding_weapon, input.is_wounded, input.age)
            && input.food >= ESCAPE_FOOD_CRIT_SKIP);

    let extras = LiveSensorExtras {
        heat: Some(input.heat),
        threat_quad_dist: Some(threat_qd),
        follow_player: input.follow_player,
        feed_player_need: input.feed_player_need,
        smith_blocks_feed: input.smith_blocks_feed,
        combat_target: input.combat_target,
        child_to_guard: input.child_to_guard,
        killable_animal: input.killable_animal || input.deadly_animal.is_some(),
        has_craft_queue: input.has_craft_queue,
        clothing_craft_pending: input.clothing_craft_pending,
        critical_craft_pending: input.critical_craft_pending,
        mid_tasks_pending: input.mid_tasks_pending,
        has_assigned_job: input.has_assigned_job,
        age_job_pending: input.age_job_pending,
        critical_misc: input.critical_misc,
        low_work_pending: input.low_work_pending,
        away_from_home: input.away_from_home,
        need_home_social: input.need_home_social,
        holding_weapon: input.holding_weapon,
        is_wounded: input.is_wounded,
        feeding_child: input.feeding_child,
        is_eating: input.is_eating,
        handling_temperature: input.handling_temperature,
        handling_death: input.handling_death,
        ordered_follow: input.ordered_follow,
        ordered_drop: input.ordered_drop,
        held_by_other: input.held_by_other,
        dropping_item: input.dropping_item,
        close_use: input.close_use,
        waiting: input.waiting,
        using_item: input.using_item,
        removing_container: input.removing_container,
        need_switch_clothes: input.need_switch_clothes,
        wounded_or_fever: input.wounded_or_fever || input.is_wounded,
    };

    let mut sensors = sensors_from_ext_ex(
        input.held_id,
        input.food,
        threat_near,
        input.nearby_food,
        input.age,
        input.has_mother,
        input.food_max,
        input.was_hungry,
        &extras,
        input.min_age_to_eat,
    );
    // Align threat_near with resolved escape (angryTime / hunt / crit food filters).
    apply_escape_to_sensors(&mut sensors, escape_threat, input.food);
    // Restore killable when animal present but hunt-skip cleared escape threat_near.
    if escape_threat == EscapeThreat::None
        && input.deadly_animal.is_some()
        && skip_escape_for_hunt(input.holding_weapon, input.is_wounded, input.age)
    {
        sensors.killable_animal = true;
        sensors.skip_escape_for_hunt = true;
    }

    let is_hungry = sensors.is_hungry;
    LiveSensorBundle {
        extras,
        threat_near: sensors.threat_near,
        threat_quad_dist: threat_qd,
        is_hungry,
        escape_ctx,
        escape_threat,
        sensors,
    }
}

/// Convenience: fill sensors then pick goal (live AI entry).
// Haxe: AiBase.doTimeStuffHelper → Goal
pub fn pick_goal_from_live_sensors(
    input: &LiveSensorInput,
    profession: Profession,
    prey_adjacent: bool,
    on_grassland: bool,
) -> (PriorityRung, Goal, LiveSensorBundle) {
    let bundle = fill_live_sensors(input);
    let (rung, goal) = pick_goal_with_sensors(
        &bundle.sensors,
        profession,
        input.held_id,
        input.nearby_food,
        prey_adjacent,
        on_grassland,
    );
    (rung, goal, bundle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rung_labels_stable() {
        assert_eq!(PriorityRung::Escape.as_label(), "ESCAPE");
        assert_eq!(PriorityRung::AssignedJob.as_label(), "ASSIGNED_JOB");
        assert_eq!(PriorityRung::Escape.band(), PriorityBand::Flee);
        assert_eq!(PriorityRung::PickupFood.band(), PriorityBand::Food);
        assert_eq!(PriorityRung::FeedChild.band(), PriorityBand::Feed);
        assert_eq!(PriorityRung::CraftQueue.band(), PriorityBand::Craft);
        assert_eq!(PriorityRung::AssignedJob.band(), PriorityBand::Job);
        assert_eq!(PriorityRung::OrderedFollow.band(), PriorityBand::Follow);
        assert_eq!(PriorityRung::FollowPlayer.band(), PriorityBand::Follow);
    }

    #[test]
    fn escape_outranks_food_and_jobs() {
        let mut s = PrioritySensors {
            threat_near: true,
            is_hungry: true,
            picking_food: true,
            has_assigned_job: true,
            has_craft_queue: true,
            age: 20.0,
            ..Default::default()
        };
        assert_eq!(resolve_priority_rung(&s), PriorityRung::Escape);
        s.food_critically_low = true;
        assert_eq!(resolve_priority_rung(&s), PriorityRung::PickupFood);
    }

    #[test]
    fn food_outranks_feed_craft_job_when_no_ordered_follow() {
        let s = PrioritySensors {
            is_hungry: true,
            picking_food: true,
            feed_player_need: true,
            has_craft_queue: true,
            has_assigned_job: true,
            age: 20.0,
            ..Default::default()
        };
        assert_eq!(resolve_priority_rung(&s), PriorityRung::PickupFood);
    }

    #[test]
    fn ordered_follow_before_pickup_food() {
        let s = PrioritySensors {
            is_hungry: true,
            picking_food: true,
            ordered_follow: true,
            age: 20.0,
            ..Default::default()
        };
        assert_eq!(resolve_priority_rung(&s), PriorityRung::OrderedFollow);
    }

    #[test]
    fn feed_outranks_job_when_do_stuff() {
        let s = PrioritySensors {
            feed_player_need: true,
            has_assigned_job: true,
            has_craft_queue: true,
            do_stuff: true,
            age: 20.0,
            ..Default::default()
        };
        assert_eq!(resolve_priority_rung(&s), PriorityRung::FeedPlayerInNeed);
        let s = PrioritySensors {
            feed_player_need: true,
            smith_blocks_feed: true,
            has_assigned_job: true,
            do_stuff: true,
            age: 20.0,
            ..Default::default()
        };
        assert_eq!(resolve_priority_rung(&s), PriorityRung::AssignedJob);
    }

    #[test]
    fn craft_before_assigned_job() {
        let s = PrioritySensors {
            has_craft_queue: true,
            has_assigned_job: true,
            age_job_pending: true,
            age: 20.0,
            ..Default::default()
        };
        assert_eq!(resolve_priority_rung(&s), PriorityRung::CraftQueue);
    }

    #[test]
    fn assigned_job_before_age_rotated() {
        let s = PrioritySensors {
            has_assigned_job: true,
            age_job_pending: true,
            low_work_pending: true,
            age: 20.0,
            ..Default::default()
        };
        assert_eq!(resolve_priority_rung(&s), PriorityRung::AssignedJob);
    }

    #[test]
    fn baby_hungry_before_child_idle_follow() {
        let s = PrioritySensors {
            age: 1.0,
            is_hungry: true,
            has_mother: true,
            is_child_with_mother: true,
            ..Default::default()
        };
        assert_eq!(resolve_priority_rung(&s), PriorityRung::BabyHungryMother);
    }

    #[test]
    fn child_with_mother_before_jobs() {
        let s = PrioritySensors {
            age: 2.0,
            has_mother: true,
            is_child_with_mother: true,
            has_assigned_job: true,
            ..Default::default()
        };
        assert_eq!(resolve_priority_rung(&s), PriorityRung::ChildWithMother);
    }

    #[test]
    fn hungry_hysteresis_matches_haxe_shape() {
        assert!(update_is_hungry(false, 5.0, 20.0, 0));
        assert!(!update_is_hungry(false, 7.0, 20.0, 0));
        assert!(update_is_hungry(false, 2.5, 5.0, 0));
        assert!(!update_is_hungry(false, 3.0, 5.0, 0));
        assert!(update_is_hungry(false, 0.5, 20.0, SMITHING_HAMMER_ID));
        assert!(update_is_hungry(false, 2.0, 20.0, SMITHING_HAMMER_ID));
        assert!(!update_is_hungry(false, 6.0, 20.0, SMITHING_HAMMER_ID));
        assert!(update_is_hungry(true, 15.0, 20.0, 0));
        assert!(!update_is_hungry(true, 16.0, 20.0, 0));
    }

    #[test]
    fn age_job_index_rotates_every_five_years() {
        assert_eq!(age_job_index(0.0), 0);
        assert_eq!(age_job_index(5.0), 1);
        assert_eq!(age_job_index(10.0), 2);
        assert_eq!(age_job_index(12.0), 2);
        assert_eq!(age_job_index(13.0), 3);
        assert_eq!(age_job_index(25.0), 0);
    }

    #[test]
    fn pick_goal_from_ladder_flee_and_food() {
        let (r, g) = pick_goal_from_ladder(
            Profession::Forager,
            0,
            15.0,
            false,
            true,
            false,
            false,
            20.0,
            false,
            20.0,
            false,
        );
        assert_eq!(r, PriorityRung::Escape);
        assert_eq!(g, Goal::Flee);

        let (r, g) = pick_goal_from_ladder(
            Profession::Farmer,
            0,
            2.0,
            true,
            false,
            false,
            false,
            20.0,
            false,
            20.0,
            false,
        );
        assert_eq!(r, PriorityRung::PickupFood);
        assert_eq!(g, Goal::SeekFood);
    }

    #[test]
    fn pick_goal_from_ladder_profession_idle() {
        let (r, g) = pick_goal_from_ladder(
            Profession::Farmer,
            0,
            15.0,
            false,
            false,
            false,
            false,
            20.0,
            false,
            20.0,
            false,
        );
        assert_eq!(r, PriorityRung::Idle);
        assert_eq!(g, Goal::SeekObject(FARMER_TARGET_ID));

        let (_, g) = pick_goal_from_ladder(
            Profession::Forager,
            0,
            15.0,
            false,
            false,
            false,
            true,
            20.0,
            false,
            20.0,
            false,
        );
        assert_eq!(g, Goal::Harvest);
    }

    #[test]
    fn held_by_blocks_all() {
        let s = PrioritySensors {
            held_by_other: true,
            threat_near: true,
            is_hungry: true,
            ..Default::default()
        };
        assert_eq!(resolve_priority_rung(&s), PriorityRung::HeldByOther);
    }

    #[test]
    fn goal_from_rung_bands() {
        assert_eq!(
            goal_from_rung(PriorityRung::Escape, Profession::Forager, 0, false, false, false),
            Goal::Flee
        );
        assert_eq!(
            goal_from_rung(PriorityRung::FeedChild, Profession::Forager, 0, false, false, false),
            Goal::SeekFood
        );
        assert_eq!(
            goal_from_rung(
                PriorityRung::OrderedFollow,
                Profession::Forager,
                0,
                false,
                false,
                false
            ),
            Goal::Explore
        );
        assert_eq!(
            goal_from_rung(PriorityRung::CraftQueue, Profession::Smith, 0, false, false, false),
            Goal::SeekObject(SMITH_TARGET_ID)
        );
        assert_eq!(
            goal_from_rung(
                PriorityRung::AssignedJob,
                Profession::Farmer,
                0,
                false,
                false,
                false
            ),
            Goal::SeekObject(FARMER_TARGET_ID)
        );
    }

    // ── New gap-closure tests ───────────────────────────────────────────────

    #[test]
    fn follow_player_selected_after_feed_band() {
        // Haxe: isMovingToPlayer after isFeedingPlayerInNeed, before home-social
        let s = PrioritySensors {
            follow_player: true,
            need_home_social: true,
            has_assigned_job: true,
            age: 20.0,
            ..Default::default()
        };
        assert_eq!(resolve_priority_rung(&s), PriorityRung::FollowPlayer);

        // Feed still outranks follow
        let s = PrioritySensors {
            follow_player: true,
            feed_player_need: true,
            do_stuff: true,
            age: 20.0,
            ..Default::default()
        };
        assert_eq!(resolve_priority_rung(&s), PriorityRung::FeedPlayerInNeed);
    }

    #[test]
    fn superbad_temp_suppresses_combat_feed_unless_threat_close() {
        // Far threat + superbad → no doStuff → job wins over combat/feed
        let s = PrioritySensors {
            superbad_temp: true,
            threat_far_for_temp: true,
            do_stuff: true, // explicit true overridden by superbad+far
            combat_target: true,
            feed_player_need: true,
            has_assigned_job: true,
            age: 20.0,
            ..Default::default()
        };
        assert!(!effective_do_stuff(&s));
        assert_eq!(resolve_priority_rung(&s), PriorityRung::AssignedJob);

        // Close threat (threat_far_for_temp=false) keeps doStuff
        let s = PrioritySensors {
            superbad_temp: true,
            threat_far_for_temp: false,
            do_stuff: true,
            combat_target: true,
            has_assigned_job: true,
            age: 20.0,
            ..Default::default()
        };
        assert!(effective_do_stuff(&s));
        assert_eq!(resolve_priority_rung(&s), PriorityRung::Combat);
    }

    #[test]
    fn compute_do_stuff_matches_haxe() {
        assert!(compute_do_stuff(false, 10_000.0));
        assert!(!compute_do_stuff(true, 10_000.0));
        assert!(compute_do_stuff(true, 50.0)); // dist < 100 forces true
        assert!(is_superbad_temp(0.05));
        assert!(is_superbad_temp(0.95));
        assert!(!is_superbad_temp(0.5));
        assert!(threat_is_far_for_temp(101.0));
        assert!(!threat_is_far_for_temp(100.0));
    }

    #[test]
    fn full_priority_sensors_band_order() {
        // FeedPlayerInNeed > CraftQueue > AssignedJob > AgeRotatedJob
        let base = PrioritySensors {
            do_stuff: true,
            age: 20.0,
            feed_player_need: true,
            has_craft_queue: true,
            has_assigned_job: true,
            age_job_pending: true,
            ..Default::default()
        };
        assert_eq!(resolve_priority_rung(&base), PriorityRung::FeedPlayerInNeed);

        let s = PrioritySensors {
            feed_player_need: false,
            has_craft_queue: true,
            has_assigned_job: true,
            age_job_pending: true,
            age: 20.0,
            ..Default::default()
        };
        assert_eq!(resolve_priority_rung(&s), PriorityRung::CraftQueue);

        let s = PrioritySensors {
            has_assigned_job: true,
            age_job_pending: true,
            age: 20.0,
            ..Default::default()
        };
        assert_eq!(resolve_priority_rung(&s), PriorityRung::AssignedJob);

        let s = PrioritySensors {
            age_job_pending: true,
            age: 20.0,
            ..Default::default()
        };
        assert_eq!(resolve_priority_rung(&s), PriorityRung::AgeRotatedJob);
    }

    #[test]
    fn sensors_from_simple_crit_food_skips_escape_for_pickup() {
        // food < -1 → food_critically_low; hungry + nearby food → PickupFood not Escape
        let s = sensors_from_simple(0, -1.5, true, true, 20.0, false, 20.0, false);
        assert!(s.food_critically_low);
        assert!(s.threat_near);
        assert!(s.picking_food);
        assert_eq!(resolve_priority_rung(&s), PriorityRung::PickupFood);
    }

    #[test]
    fn sensors_from_ext_fills_follow_combat_job_temp() {
        let extra = LiveSensorExtras {
            heat: Some(0.05),
            threat_quad_dist: Some(500.0),
            follow_player: true,
            has_assigned_job: true,
            age_job_pending: true,
            holding_weapon: true,
            is_wounded: false,
            ..Default::default()
        };
        let s = sensors_from_ext(0, 15.0, false, false, 20.0, false, 20.0, false, &extra);
        assert!(s.superbad_temp);
        assert!(!s.do_stuff); // far + superbad
        assert!(s.skip_escape_for_hunt); // weapon + age 20
        assert!(s.follow_player);
        assert_eq!(resolve_priority_rung(&s), PriorityRung::FollowPlayer);

        let (r, g) = pick_goal_with_sensors(&s, Profession::Explorer, 0, false, false, false);
        assert_eq!(r, PriorityRung::FollowPlayer);
        assert_eq!(g, Goal::Explore);
    }

    #[test]
    fn age_rotated_job_sequence_matches_haxe_cycle() {
        // age 0 → index 0: berry, basic, baking, pottery, sheep
        assert_eq!(
            age_rotated_job_sequence(0.0),
            [
                AgeRotatedJobKind::BerryFarming,
                AgeRotatedJobKind::BasicFarming,
                AgeRotatedJobKind::Baking,
                AgeRotatedJobKind::Pottery,
                AgeRotatedJobKind::SheepHerding,
            ]
        );
        // age 10 → index 2: baking, pottery, sheep, berry, basic
        assert_eq!(age_rotated_job_kind(10.0), AgeRotatedJobKind::Baking);
        assert_eq!(
            age_rotated_job_sequence(10.0),
            [
                AgeRotatedJobKind::Baking,
                AgeRotatedJobKind::Pottery,
                AgeRotatedJobKind::SheepHerding,
                AgeRotatedJobKind::BerryFarming,
                AgeRotatedJobKind::BasicFarming,
            ]
        );
        assert_eq!(AgeRotatedJobKind::BerryFarming.as_label(), "BERRY");
    }

    #[test]
    fn escape_policy_angry_dist_weapon_hunt() {
        // No threat
        assert_eq!(
            resolve_escape_threat(&EscapeContext::default()),
            EscapeThreat::None
        );

        // Animal threat → flee animal
        let mut ctx = EscapeContext {
            has_animal: true,
            dist_animal_quad: 9.0,
            ..Default::default()
        };
        assert_eq!(resolve_escape_threat(&ctx), EscapeThreat::Animal);

        // angryTime > 4 clears player
        ctx.has_animal = false;
        ctx.has_deadly_player = true;
        ctx.player_angry_time = 5.0;
        ctx.dist_player = 10.0;
        assert_eq!(resolve_escape_threat(&ctx), EscapeThreat::None);

        // Player within 64, calm → flee player
        ctx.player_angry_time = 2.0;
        assert_eq!(resolve_escape_threat(&ctx), EscapeThreat::Player);

        // Player dist > 64 alone → skip
        ctx.dist_player = 100.0;
        assert_eq!(resolve_escape_threat(&ctx), EscapeThreat::None);

        // Crit food skip
        ctx = EscapeContext {
            has_animal: true,
            food_store: -2.0,
            dist_animal_quad: 4.0,
            ..Default::default()
        };
        assert_eq!(resolve_escape_threat(&ctx), EscapeThreat::None);

        // Hunt skip: weapon + age > 8
        ctx = EscapeContext {
            has_animal: true,
            holding_weapon: true,
            age: 10.0,
            dist_animal_quad: 4.0,
            ..Default::default()
        };
        assert_eq!(resolve_escape_threat(&ctx), EscapeThreat::None);
        assert!(skip_escape_for_hunt(true, false, 10.0));
        assert!(!skip_escape_for_hunt(true, false, 7.0));
        assert!(!skip_escape_for_hunt(true, true, 10.0));

        // didNotReachFood >= 5 → no attempt
        assert!(!should_attempt_escape(5.0));
        assert!(should_attempt_escape(4.9));
        ctx = EscapeContext {
            has_animal: true,
            did_not_reach_food: 5.0,
            dist_animal_quad: 4.0,
            ..Default::default()
        };
        assert_eq!(resolve_escape_threat(&ctx), EscapeThreat::None);

        // Prefer closer player over animal
        ctx = EscapeContext {
            has_animal: true,
            has_deadly_player: true,
            player_angry_time: 1.0,
            dist_animal_quad: 50.0,
            dist_player: 10.0,
            ..Default::default()
        };
        assert_eq!(resolve_escape_threat(&ctx), EscapeThreat::Player);
    }

    #[test]
    fn escape_target_and_side_effects() {
        // Threat east → flee west
        assert_eq!(escape_target_xy(10, 10, 15, 10, ESCAPE_DIST), (7, 13)); // wait
        // threat_tx > player_tx → escape_in_lower_x → player - dist
        // threat_ty == player_ty → escape_in_lower_y = false (threat_ty > player_ty is false)
        // so ty = player + dist
        assert_eq!(escape_target_xy(10, 10, 15, 10, 3), (7, 13));
        // Threat SW → flee NE
        assert_eq!(escape_target_xy(10, 10, 5, 5, 3), (13, 13));

        let e = escape_side_effects(true, true, false);
        assert!(e.cancel_use);
        assert!(e.clear_food_target);
        assert!(e.clear_craft_trans);
        assert!(e.increment_did_not_reach_food);

        let e = escape_side_effects(false, false, false);
        assert!(!e.cancel_use);
        assert!(!e.increment_did_not_reach_food);
    }

    #[test]
    fn check_is_hungry_and_eat_side_effects() {
        let e = check_is_hungry_and_eat_effects(false, 2.0, 20.0, 0, 4.0, false);
        assert!(e.is_hungry);
        assert!(e.clear_caring_for_fire);
        assert!(e.baby_say_f); // age 4 < 6
        assert!(e.need_search_food);

        let e = check_is_hungry_and_eat_effects(false, 2.0, 20.0, 0, 10.0, true);
        assert!(e.is_hungry);
        assert!(!e.baby_say_f);
        assert!(!e.need_search_food); // has food target

        let e = check_is_hungry_and_eat_effects(false, 15.0, 20.0, 0, 4.0, false);
        assert!(!e.is_hungry);
        assert!(e.clear_caring_for_fire); // always cleared
        assert!(!e.baby_say_f);
        assert!(!e.need_search_food);
    }

    #[test]
    fn escape_skip_when_skip_escape_for_hunt_sensor() {
        let s = PrioritySensors {
            threat_near: true,
            skip_escape_for_hunt: true,
            killable_animal: true,
            do_stuff: true,
            age: 20.0,
            ..Default::default()
        };
        assert_eq!(resolve_priority_rung(&s), PriorityRung::KillAnimal);
    }

    #[test]
    fn apply_escape_to_sensors_sets_threat_near() {
        let mut s = PrioritySensors::default();
        apply_escape_to_sensors(&mut s, EscapeThreat::Animal, 10.0);
        assert!(s.threat_near);
        assert!(!s.food_critically_low);
        assert_eq!(resolve_priority_rung(&s), PriorityRung::Escape);

        apply_escape_to_sensors(&mut s, EscapeThreat::None, -2.0);
        assert!(!s.threat_near);
        assert!(s.food_critically_low);
    }

    #[test]
    fn deadly_player_exile_home_and_angry_clear() {
        // Exile near home → dangerous
        let exile = DeadlyPlayerCandidate {
            p_id: 2,
            x: 5,
            y: 0,
            deleted: false,
            age: 20.0,
            angry_time: 10.0,
            lost_combat_prestige: 0.0,
            is_cursed: false,
            is_ai: false,
            holding_weapon: false,
            held_is_bloody: false,
            exiled_by_observer_leaders: true,
            is_friendly: false,
        };
        assert!(is_deadly_player_candidate(&exile, 10.0, 0, 0));
        // Far from home (dx=50 → 2500 > 1600) → not dangerous from exile alone
        let far = DeadlyPlayerCandidate {
            x: 50,
            y: 0,
            ..exile
        };
        assert!(!is_deadly_player_candidate(&far, 10.0, 0, 0));

        // Weapon + angryTime < 4 on either side
        let armed = DeadlyPlayerCandidate {
            p_id: 3,
            x: 2,
            y: 0,
            deleted: false,
            age: 20.0,
            angry_time: 2.0,
            lost_combat_prestige: 0.0,
            is_cursed: false,
            is_ai: false,
            holding_weapon: true,
            held_is_bloody: false,
            exiled_by_observer_leaders: false,
            is_friendly: false,
        };
        assert!(is_deadly_player_candidate(&armed, 10.0, 0, 0));
        // Both calm (>4) with weapon only — still dangerous if weapon && angry < 4 fails both
        let calm_armed = DeadlyPlayerCandidate {
            angry_time: 5.0,
            ..armed
        };
        // observer angry 5, their angry 5 → weapon branch false; no other flags → not dangerous
        assert!(!is_deadly_player_candidate(&calm_armed, 5.0, 0, 0));

        // Friendly skips even if bloody
        let friend = DeadlyPlayerCandidate {
            held_is_bloody: true,
            is_friendly: true,
            ..armed
        };
        assert!(!is_deadly_player_candidate(&friend, 0.0, 0, 0));

        // Closest of two: prestige dist shrink can win
        let near = DeadlyPlayerCandidate {
            p_id: 10,
            x: 4,
            y: 0,
            deleted: false,
            age: 20.0,
            angry_time: 1.0,
            lost_combat_prestige: 0.0,
            is_cursed: false,
            is_ai: false,
            holding_weapon: false,
            held_is_bloody: true,
            exiled_by_observer_leaders: false,
            is_friendly: false,
        };
        let prestige = DeadlyPlayerCandidate {
            p_id: 11,
            x: 6,
            y: 0,
            lost_combat_prestige: 10.0,
            held_is_bloody: true,
            ..near
        };
        let hit = get_close_deadly_player(0, 0, 0.0, 0, 0, 30, &[near, prestige])
            .expect("deadly player");
        // prestige dist = 36 / ((15+10)/10) = 36/2.5 = 14.4; near dist = 16 → prestige wins
        assert_eq!(hit.p_id, 11);
        assert!(hit.dist < 15.0);
    }

    #[test]
    fn get_close_player_target_mask_gates() {
        let c = PlayerTargetCandidate {
            p_id: 7,
            x: 3,
            y: 0,
            deleted: false,
            age: 20.0,
            is_same_family: false,
            is_ally: false,
            is_top_leader: true,
            lost_combat_prestige: 0.0,
            is_cursed: false,
            target_home_quad: 100.0,
        };
        // No mask / darkNosaj → None
        assert!(get_close_player_target(false, false, 0.0, 20.0, 0, 0, 20, &[c]).is_none());
        // Red mask → hit
        let t = get_close_player_target(true, false, 0.0, 20.0, 0, 0, 20, &[c]).unwrap();
        assert_eq!(t.p_id, 7);
        // Same family skip
        let fam = PlayerTargetCandidate {
            is_same_family: true,
            ..c
        };
        assert!(get_close_player_target(true, false, 0.0, 20.0, 0, 0, 20, &[fam]).is_none());
    }

    #[test]
    fn is_child_and_has_mother_ex_live_boundary() {
        assert!(is_child_and_has_mother_ex(2.9, true, 3.0));
        assert!(!is_child_and_has_mother_ex(3.0, true, 3.0));
        // live MinAgeToEat=5 keeps age 4 as child
        assert!(is_child_and_has_mother_ex(4.0, true, 5.0));
        assert!(!is_child_and_has_mother_ex(4.0, true, 3.0));
        assert!(!is_child_and_has_mother_ex(4.0, false, 5.0));
    }

    #[test]
    fn sensors_from_ext_ex_live_min_age_food_gates() {
        let extra = LiveSensorExtras::default();
        // age 4, min 5 → child; cannot pick/make food
        let s = sensors_from_ext_ex(0, 1.0, false, true, 4.0, true, 20.0, true, &extra, 5.0);
        assert!(s.is_child_with_mother);
        assert!(!s.picking_food);
        assert!(!s.considering_make_food);
        // age 4, min 3 → adult food gates open when hungry
        let s2 = sensors_from_ext_ex(0, 1.0, false, true, 4.0, false, 20.0, true, &extra, 3.0);
        assert!(!s2.is_child_with_mother);
        assert!(s2.picking_food);
    }

    #[test]
    fn fill_live_sensors_wolf_threat_and_mother() {
        // Deadly animal in range → Escape
        let mut input = LiveSensorInput {
            food: 15.0,
            food_max: 20.0,
            age: 20.0,
            heat: 0.5,
            deadly_animal: Some((12, 10, 4.0)),
            ..Default::default()
        };
        let (rung, goal, bundle) =
            pick_goal_from_live_sensors(&input, Profession::Explorer, false, false);
        assert_eq!(rung, PriorityRung::Escape);
        assert_eq!(goal, Goal::Flee);
        assert!(bundle.threat_near);
        assert_eq!(bundle.escape_threat, EscapeThreat::Animal);
        assert!((bundle.threat_quad_dist - 4.0).abs() < 1e-4);

        // Child + mother → ChildWithMother (no threat)
        input.deadly_animal = None;
        input.age = 2.0;
        input.has_mother = true;
        input.follow_player = true;
        let (rung, _, bundle) =
            pick_goal_from_live_sensors(&input, Profession::Explorer, false, false);
        assert_eq!(rung, PriorityRung::ChildWithMother);
        assert!(bundle.sensors.is_child_with_mother);
        assert!(bundle.sensors.follow_player);

        // Superbad heat + far threat → doStuff false; no escape
        input = LiveSensorInput {
            heat: 0.05,
            age: 20.0,
            food: 15.0,
            food_max: 20.0,
            ..Default::default()
        };
        let bundle = fill_live_sensors(&input);
        assert!(bundle.sensors.superbad_temp);
        assert!(!bundle.sensors.do_stuff);
        assert!(!bundle.threat_near);

        // Hungry hysteresis via food_max
        input.food = 5.0; // < max(3, 20*0.3=6) → hungry
        input.was_hungry = false;
        let bundle = fill_live_sensors(&input);
        assert!(bundle.is_hungry);
        input.food = 15.0; // was hungry, leave at 0.8*20=16
        input.was_hungry = true;
        let bundle = fill_live_sensors(&input);
        assert!(bundle.is_hungry);
        input.food = 17.0;
        let bundle = fill_live_sensors(&input);
        assert!(!bundle.is_hungry);
    }

    #[test]
    fn is_moving_to_player_and_follow_tiles() {
        assert!(!is_moving_to_player_needed(3.0, 2)); // 3 < 4
        assert!(is_moving_to_player_needed(4.0, 2)); // 4 >= 4
        assert_eq!(baby_hungry_follow_tiles(), 5);
        assert_eq!(child_with_mother_follow_tiles(true), 2);
        assert_eq!(child_with_mother_follow_tiles(false), 4);
        assert_eq!(wounded_follow_tiles(), 2);
        assert_eq!(ordered_follow_max_tiles(true), 10);
        assert_eq!(ordered_follow_max_tiles(false), 5);
    }

    #[test]
    fn escape_context_from_live_player_angry() {
        let ctx = escape_context_from_threats(
            None,
            Some((5, 0, 10.0, 5.0)), // angry_time 5 → cleared in resolve
            10.0,
            false,
            false,
            20.0,
            0.0,
            false,
        );
        assert_eq!(resolve_escape_threat(&ctx), EscapeThreat::None);
        let ctx = escape_context_from_threats(
            None,
            Some((5, 0, 10.0, 2.0)),
            10.0,
            false,
            false,
            20.0,
            0.0,
            false,
        );
        assert_eq!(resolve_escape_threat(&ctx), EscapeThreat::Player);
    }
}
