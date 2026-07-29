// Haxe: AiHelper.CountCloseObjects + farm home radii (**AI-JOB-FARM-WIRE** / farm_spatial)
// Included into farmer_profession.rs (same module namespace).

// ── Radii (Haxe CountCloseObjects / shortCraft) ─────────────────────────────

/// Default home CountCloseObjects radius for farm (Haxe `distance = 30`).
pub const FARM_COUNT_RADIUS: i32 = FARM_HOME_RADIUS;
/// Corn seed / dried-corn family radius (Haxe `countCorn` r=20).
pub const CORN_SEED_COUNT_RADIUS: i32 = 20;
/// Watering / plant shortCraft default radius (Haxe shortCraft / doWateringOn r=30).
pub const FARM_SHORTCRAFT_RADIUS: i32 = 30;
/// Prepare-rows shallow/deep shortCraft radius (Haxe r=15 in older doBasicFarming).
pub const FARM_ROW_SHORTCRAFT_RADIUS: i32 = 15;

/// Wet Clay Bowl — Haxe forces `countPiles = false` (else Wet Clay Crock piles pollute).
// Haxe: AiHelper.CountCloseObjectsHelper objId == 233
pub const WET_CLAY_BOWL_ID: i32 = 233;
/// Big Charcoal Pile — Haxe remaps pile search to Huge Charcoal Pile 4102.
// Haxe: AiHelper.CountCloseObjectsHelper objId == 300
pub const BIG_CHARCOAL_PILE_ID: i32 = 300;
/// Huge Charcoal Pile (pile form for Big Charcoal 300).
pub const HUGE_CHARCOAL_PILE_ID: i32 = 4102;

/// Haxe `ServerSettings.AiIgnoredFloorIds` (Bear Skin Rug 656 / stone 888).
// Haxe: ServerSettings.AiIgnoredFloorIds
pub const AI_IGNORED_FLOOR_IDS: [i32; 2] = [656, 888];

/// Haxe count / shortCraft radii used by live `CountCloseObjects` / shortCraft fillers.
// Haxe: AiBase farm family CountCloseObjects home.tx/ty
pub fn farm_radius_table() -> &'static [(i32, &'static str)] {
    &[
        (FARM_COUNT_RADIUS, "home CountCloseObjects default"),
        (CORN_SEED_COUNT_RADIUS, "countCorn / dried corn family"),
        (FARM_SHORTCRAFT_RADIUS, "plant/water shortCraft"),
        (FARM_ROW_SHORTCRAFT_RADIUS, "row shortCraft (15)"),
    ]
}

// ── Map snapshot ────────────────────────────────────────────────────────────

/// Map object at a tile for mock [`fill_farm_counts_from_map`] / [`count_close_objects_at`].
///
/// `uses` is Haxe `numberOfUses` on pile tiles. For [`fill_farm_counts_from_map`] the full
/// uses land under `parent_id`. For [`count_close_objects_at`] matching `obj_id` always
/// contributes **1** per tile; matching **pile** parent contributes `uses` when piles on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FarmMapObj {
    pub parent_id: i32,
    pub x: i32,
    pub y: i32,
    /// Pile uses; `<= 1` → count as 1 when treated as a pile contribution.
    pub uses: i32,
    /// Floor under this tile (optional; ignore-floor uses origin floor param instead).
    pub floor_id: i32,
    /// Haxe `objData.foodValue > 0` — never skipped by IsIgnoredFloor.
    pub is_food: bool,
    /// Haxe `objData.isPermanent()` — never skipped by IsIgnoredFloor.
    pub is_permanent: bool,
}

impl FarmMapObj {
    pub fn simple(parent_id: i32, x: i32, y: i32) -> Self {
        Self {
            parent_id,
            x,
            y,
            uses: 1,
            floor_id: 0,
            is_food: false,
            is_permanent: false,
        }
    }

    pub fn pile(parent_id: i32, x: i32, y: i32, uses: i32) -> Self {
        Self {
            parent_id,
            x,
            y,
            uses: uses.max(1),
            floor_id: 0,
            is_food: false,
            is_permanent: false,
        }
    }

    /// Non-food non-permanent on a floor (for IsIgnoredFloor tests).
    pub fn with_floor(mut self, floor_id: i32) -> Self {
        self.floor_id = floor_id;
        self
    }

    pub fn food(mut self) -> Self {
        self.is_food = true;
        self
    }

    pub fn permanent(mut self) -> Self {
        self.is_permanent = true;
        self
    }

    fn pile_contrib(self) -> i32 {
        if self.uses <= 1 {
            1
        } else {
            self.uses
        }
    }

    /// Snapshot fill contribution (uses expanded under parent_id).
    fn fill_contrib(self) -> i32 {
        self.pile_contrib()
    }
}

/// Chebyshev distance (smith `chebyshev` / inclusive radius). Prefer
/// [`in_count_close_square`] for Haxe CountCloseObjects geometry.
// Haxe: not exact CountCloseObjects (see in_count_close_square)
#[inline]
pub fn farm_chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    crate::smith_profession::chebyshev(ax, ay, bx, by)
}

/// Haxe `CountCloseObjectsHelper` loop geometry: half-open square
/// `[tx - radius, tx + radius) × [ty - radius, ty + radius)`.
///
/// Differs from chebyshev `≤ radius` on the high edge and corners
/// (tile at `tx+radius` excluded; `tx-radius` included).
// Haxe: AiHelper.CountCloseObjectsHelper for (tty in ty-radius...ty+radius)
#[inline]
pub fn in_count_close_square(tx: i32, ty: i32, ox: i32, oy: i32, radius: i32) -> bool {
    ox >= tx - radius && ox < tx + radius && oy >= ty - radius && oy < ty + radius
}

// ── IsIgnoredFloor / pile specials ──────────────────────────────────────────

/// Haxe `AiHelper.IsIgnoredFloor` — skip non-food non-permanent on ignored floors.
// Haxe: AiHelper.IsIgnoredFloor ~42
pub fn is_ignored_floor(
    floor_id: i32,
    is_food: bool,
    is_permanent: bool,
    ignored_floor_ids: &[i32],
) -> bool {
    if floor_id < 1 {
        return false;
    }
    if is_food {
        return false;
    }
    if is_permanent {
        return false;
    }
    ignored_floor_ids.contains(&floor_id)
}

/// Apply Haxe CountCloseObjects pile specials for `obj_id`.
///
/// Returns `(count_piles, pile_obj_id)` where `pile_obj_id < 0` means no pile form.
// Haxe: AiHelper.CountCloseObjectsHelper ~634–635
pub fn count_close_pile_specials(obj_id: i32, count_piles: bool, pile_obj_id: i32) -> (bool, i32) {
    let mut count_piles = count_piles;
    let mut pile_obj_id = pile_obj_id;
    if obj_id == WET_CLAY_BOWL_ID {
        // Wet Clay Bowl 233 → otherwise Wet Clay Crock is counted too
        count_piles = false;
    }
    if obj_id == BIG_CHARCOAL_PILE_ID {
        // Big Charcoal Pile 300 → Huge Charcoal Pile 4102
        pile_obj_id = HUGE_CHARCOAL_PILE_ID;
    }
    (count_piles, pile_obj_id)
}

/// Look up pile parent id from a small content table (`obj_id → pile_id`).
/// Returns `-1` when no pile mapping (Haxe `getPileObjId` miss).
pub fn pile_obj_id_from_table(obj_id: i32, pile_table: &[(i32, i32)]) -> i32 {
    for &(id, pile) in pile_table {
        if id == obj_id {
            return pile;
        }
    }
    -1
}

// ── CountCloseObjects pure ──────────────────────────────────────────────────

/// Options for Haxe-faithful [`count_close_objects_ex`].
#[derive(Debug, Clone, Copy)]
pub struct CountCloseOpts<'a> {
    /// Haxe `countPiles` (overridden false for obj 233).
    pub count_piles: bool,
    /// Result of `getPileObjId()` before specials (`-1` = none). Charcoal 300 remaps to 4102.
    pub pile_obj_id: i32,
    /// Haxe quirk: floor check uses **origin** `(tx,ty)` floor, not each tile.
    // Haxe: getFloorId(tx, ty) inside the double loop (origin, not ttx/tty)
    pub origin_floor_id: i32,
    /// Haxe `ServerSettings.AiIgnoredFloorIds`.
    pub ignored_floor_ids: &'a [i32],
}

impl Default for CountCloseOpts<'static> {
    fn default() -> Self {
        Self {
            count_piles: true,
            pile_obj_id: -1,
            origin_floor_id: 0,
            ignored_floor_ids: &AI_IGNORED_FLOOR_IDS,
        }
    }
}

/// Pure Haxe `CountCloseObjects` for one parent id near `(tx, ty)`.
///
/// - Geometry: half-open square [`in_count_close_square`] (not chebyshev ≤ r)
/// - Matching `parent_id == obj_id` → +1 per tile
/// - Matching pile parent (after specials) → +`uses` when piles allowed
/// - Specials: 233 disables piles; 300 remaps pile → 4102
/// - Does not add held (use [`count_with_held`])
// Haxe: AiHelper.CountCloseObjectsHelper ~628
pub fn count_close_objects_at(
    tx: i32,
    ty: i32,
    obj_id: i32,
    radius: i32,
    objects: &[FarmMapObj],
) -> i32 {
    count_close_objects_ex(tx, ty, obj_id, radius, objects, CountCloseOpts::default())
}

/// Like [`count_close_objects_at`] with explicit pile id / floor options.
// Haxe: AiHelper.CountCloseObjects(player, tx, ty, objId, radius, countPiles)
pub fn count_close_objects_ex(
    tx: i32,
    ty: i32,
    obj_id: i32,
    radius: i32,
    objects: &[FarmMapObj],
    opts: CountCloseOpts<'_>,
) -> i32 {
    let (count_piles, pile_obj_id) =
        count_close_pile_specials(obj_id, opts.count_piles, opts.pile_obj_id);
    let mut n = 0;
    for o in objects {
        if !in_count_close_square(tx, ty, o.x, o.y, radius) {
            continue;
        }
        // Haxe: floorID = getFloorId(tx, ty) — origin, not tile
        if is_ignored_floor(
            opts.origin_floor_id,
            o.is_food,
            o.is_permanent,
            opts.ignored_floor_ids,
        ) {
            continue;
        }
        if o.parent_id == obj_id {
            n += 1;
        }
        if count_piles && pile_obj_id >= 0 && o.parent_id == pile_obj_id {
            n += o.pile_contrib();
        }
    }
    n
}

/// Count with pile table: `pile_table` maps counted id → pile parent id.
pub fn count_close_objects_with_piles(
    tx: i32,
    ty: i32,
    obj_id: i32,
    radius: i32,
    objects: &[FarmMapObj],
    pile_table: &[(i32, i32)],
) -> i32 {
    let pile = pile_obj_id_from_table(obj_id, pile_table);
    count_close_objects_ex(
        tx,
        ty,
        obj_id,
        radius,
        objects,
        CountCloseOpts {
            count_piles: true,
            pile_obj_id: pile,
            ..CountCloseOpts::default()
        },
    )
}

/// Count + held parent match (Haxe `countCurrentObject` / countCorn held += 1).
pub fn count_with_held(map_count: i32, held_id: i32, obj_id: i32) -> i32 {
    map_count + if held_id == obj_id { 1 } else { 0 }
}

/// Build [`FarmCounts`] from a nearby `(id, count)` snapshot (mock home radius).
///
/// Held is recorded on the struct; does **not** auto-add held into `by_id`
/// (mirrors Haxe CountCloseObjects vs countCurrentObject).
// Haxe: CountCloseObjects near home for farm SM
pub fn farm_counts_from_nearby(
    nearby: &[(i32, i32)],
    held_id: i32,
    is_hungry: bool,
    basic_farmer_weight: f32,
    hardened_row_biome: Option<u8>,
) -> FarmCounts {
    let mut c = FarmCounts {
        held_id,
        is_hungry,
        basic_farmer_weight,
        hardened_row_biome,
        ..Default::default()
    };
    for &(id, n) in nearby {
        if n <= 0 {
            continue;
        }
        c.set(id, c.get(id) + n);
    }
    c
}

/// Fill [`FarmCounts`] from a mock map snapshot (unit tests / thin live tick).
///
/// - All objects within Haxe half-open square `home_r` of home contribute by
///   `parent_id` (+ pile uses expanded)
/// - Default `home_r` = [`FARM_COUNT_RADIUS`] (30)
/// - Does not special-case corn r=20 (caller may re-query via [`count_close_objects_at`])
/// - No IsIgnoredFloor (origin floor unknown in bulk fill; use
///   [`fill_farm_counts_from_map_with_floor`] when known)
// Haxe: AiHelper.CountCloseObjects(player, home.tx, home.ty, id, 30) bulk snapshot
pub fn fill_farm_counts_from_map(
    home_x: i32,
    home_y: i32,
    held_id: i32,
    objects: &[FarmMapObj],
    home_r: i32,
) -> FarmCounts {
    fill_farm_counts_from_map_with_floor(home_x, home_y, held_id, objects, home_r, 0)
}

/// Like [`fill_farm_counts_from_map`] with origin floor for IsIgnoredFloor skip.
// Haxe: CountCloseObjects + IsIgnoredFloor(getFloorId(home))
pub fn fill_farm_counts_from_map_with_floor(
    home_x: i32,
    home_y: i32,
    held_id: i32,
    objects: &[FarmMapObj],
    home_r: i32,
    origin_floor_id: i32,
) -> FarmCounts {
    let mut counts = FarmCounts {
        held_id,
        ..Default::default()
    };
    for o in objects {
        if !in_count_close_square(home_x, home_y, o.x, o.y, home_r) {
            continue;
        }
        if is_ignored_floor(
            origin_floor_id,
            o.is_food,
            o.is_permanent,
            &AI_IGNORED_FLOOR_IDS,
        ) {
            continue;
        }
        let n = counts.get(o.parent_id) + o.fill_contrib();
        counts.set(o.parent_id, n);
    }
    counts
}

/// Like [`fill_farm_counts_from_map`] with player hunger / basic weight / biome.
pub fn fill_farm_counts_from_map_ex(
    home_x: i32,
    home_y: i32,
    held_id: i32,
    objects: &[FarmMapObj],
    home_r: i32,
    is_hungry: bool,
    basic_farmer_weight: f32,
    hardened_row_biome: Option<u8>,
) -> FarmCounts {
    let mut c = fill_farm_counts_from_map(home_x, home_y, held_id, objects, home_r);
    c.is_hungry = is_hungry;
    c.basic_farmer_weight = basic_farmer_weight;
    c.hardened_row_biome = hardened_row_biome;
    c
}

/// Haxe `countCorn` pure sum of dried/kernel family within [`CORN_SEED_COUNT_RADIUS`].
///
/// Held is added only for 1115 / 1120 / 1247 (port-as-is; 4106/4107 map-only).
// Haxe: AiBase.countCorn ~1920
pub fn count_corn_seeds_near(
    home_x: i32,
    home_y: i32,
    held_id: i32,
    objects: &[FarmMapObj],
) -> i32 {
    // Dried Ear 1115, Bowl Cob 1120, Bowl Kernels 1247, Dumped 4106, Pile 4107
    const CORN_MAP_IDS: [i32; 5] = [1115, 1120, 1247, 4106, 4107];
    // Haxe only adds held for 1115 / 1120 / 1247 — not 4106 / 4107
    const CORN_HELD_IDS: [i32; 3] = [1115, 1120, 1247];
    let mut n = 0;
    for id in CORN_MAP_IDS {
        n += count_close_objects_at(home_x, home_y, id, CORN_SEED_COUNT_RADIUS, objects);
    }
    for id in CORN_HELD_IDS {
        if held_id == id {
            n += 1;
        }
    }
    n
}

/// Soil-unit metric from a map snapshot (Haxe doPrepareSoil count at home r=30).
///
/// `2 * fertile_pile + fertile_soil + deep_rows` (pile uses expanded by fill).
// Haxe: AiBase.doPrepareSoil ~2003–2008
pub fn soil_units_from_map(home_x: i32, home_y: i32, objects: &[FarmMapObj]) -> i32 {
    let c = fill_farm_counts_from_map(home_x, home_y, 0, objects, FARM_COUNT_RADIUS);
    2 * c.get(FERTILE_SOIL_PILE) + c.get(FERTILE_SOIL) + c.get(DEEP_TILLED_ROW)
}

// ── Ladder bridge / goal map ────────────────────────────────────────────────

/// Map a [`FarmAction`] into a high-level [`Goal`] for self-play / thin tick.
// Haxe: shortCraft/craftItem seek target
pub fn farm_action_to_goal(action: FarmAction) -> Goal {
    match action {
        FarmAction::None | FarmAction::Abort | FarmAction::ClearBasicFarmerWeight => {
            Goal::SeekObject(FARMER_TARGET_ID)
        }
        FarmAction::ShortCraft { target, .. } => Goal::SeekObject(target),
        FarmAction::CraftItem { object_id } => Goal::SeekObject(object_id),
        // AI-SHEPHERD-MID: mid basic-farm sheep site → seek domestic sheep
        FarmAction::DeferSheepHerding { .. } => Goal::SeekObject(575),
        // After-sheep advanced farming fallthrough → farmer target
        FarmAction::DeferAdvancedFarming { .. } => Goal::SeekObject(FARMER_TARGET_ID),
    }
}

/// Job-band rungs that should run `decide_farm_job` when a farm profession is active.
// Haxe: AssignedJob BASICFARMER/… → doBasicFarming(100); AgeRotated berry/basic
pub fn farm_job_rung_label(rung_label: &str) -> bool {
    matches!(
        rung_label,
        "ASSIGNED_JOB"
            | "AGE_ROTATED_JOB"
            | "LOW_PRIORITY_WORK"
            | "MID_PRIORITY_TASKS"
            | "CRITICAL_MISC"
            | "CRAFT_QUEUE"
            | "CRITICAL_CRAFT"
    )
}

/// Haxe assigned maxProfession=100 vs default 1/2 for farm jobs.
// Haxe: doBasicFarming(100) assigned; doBasicFarming() / doCarrotFarming(1)
pub fn farm_max_people_for_dispatch(is_assigned_job: bool, default_max: i32) -> i32 {
    if is_assigned_job {
        100
    } else {
        default_max
    }
}

/// Map age-rotated ladder label or profession key → farm job.
pub fn farm_job_for_age_label(label: &str) -> Option<FarmProfession> {
    match label {
        "BERRY" | "BerryFarmer" | "BERRYFARMER" => Some(FarmProfession::BerryFarmer),
        "BASIC" | "BASICFARMER" => Some(FarmProfession::BasicFarmer),
        _ => FarmProfession::parse(label).or_else(|| parse_farm_profession_speech(label)),
    }
}

/// Thin ladder bridge: when rung is a farm job band and `job` is set, run pure `decide_farm_job`.
///
/// Prefer resolving `job` via [`resolve_farm_assigned_job`] or [`age_rotated_farm_profession`]
/// / [`farm_job_for_age_label`] before calling. Counts typically come from
/// [`fill_farm_counts_from_map`].
// Haxe: AiBase assignedProfession / jobByAge 0–1 farm slots
pub fn try_decide_farm_from_rung(
    job: Option<FarmProfession>,
    rung_label: &str,
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    has_profession: bool,
) -> Option<FarmAction> {
    let job = job?;
    if !farm_job_rung_label(rung_label) {
        return None;
    }
    // Haxe: assigned BASICFARMER → doBasicFarming(100); age/mid → doBasicFarming()=2
    // AI-FARM-STICKY: max carried on DeferSheepHerding → doAdvancedFarming(max)
    let max_profession = if matches!(job, FarmProfession::BasicFarmer)
        && rung_label == "ASSIGNED_JOB"
    {
        crate::BASIC_FARM_ASSIGNED_MAX_PROFESSION
    } else {
        crate::BASIC_FARM_DEFAULT_MAX_PROFESSION
    };
    Some(decide_farm_job(
        job,
        counts,
        task,
        has_profession,
        max_profession,
    ))
}

/// Compose fill → decide → goal for live tick / ladder consumers (pure).
///
/// Returns `None` when rung is not a farm band or `job` is unset (caller keeps thin
/// `SeekObject(FARMER_TARGET_ID)`). On decide success maps via [`farm_action_to_goal`].
// Haxe: AssignedJob/AgeRotated → doBasicFarming + shortCraft/craftItem seek
pub fn farm_goal_from_map_and_rung(
    job: Option<FarmProfession>,
    rung_label: &str,
    home_x: i32,
    home_y: i32,
    held_id: i32,
    objects: &[FarmMapObj],
    home_r: i32,
    task: &mut FarmTaskState,
    has_profession: bool,
    is_hungry: bool,
    basic_farmer_weight: f32,
    hardened_row_biome: Option<u8>,
) -> Option<Goal> {
    let counts = fill_farm_counts_from_map_ex(
        home_x,
        home_y,
        held_id,
        objects,
        home_r,
        is_hungry,
        basic_farmer_weight,
        hardened_row_biome,
    );
    let action = try_decide_farm_from_rung(job, rung_label, &counts, task, has_profession)?;
    Some(farm_action_to_goal(action))
}

/// Same as [`farm_goal_from_map_and_rung`] but from an already-built [`FarmCounts`].
pub fn farm_goal_from_counts_and_rung(
    job: Option<FarmProfession>,
    rung_label: &str,
    counts: &FarmCounts,
    task: &mut FarmTaskState,
    has_profession: bool,
) -> Option<Goal> {
    let action = try_decide_farm_from_rung(job, rung_label, counts, task, has_profession)?;
    Some(farm_action_to_goal(action))
}
