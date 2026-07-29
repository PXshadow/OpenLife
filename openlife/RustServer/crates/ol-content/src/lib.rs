//! Content loading from OneLifeData7-style text files + OLC1/OLT1 binary cache.
//!
//! Binary layout shared with RustClient (`CONTENT_BINARY.md`). Text remains
//! authoring SoT; `load_prefer_cache` prefers `cache/olc1_objects.bin` +
//! `olt1_transitions.bin` when valid.

#![forbid(unsafe_code)]

mod binary_cache;
mod prob_set;

pub use binary_cache::{
    cache_dir_for, finish_cache_boot, load_from_cache, load_olc1, load_olt1, load_prefer_cache,
    OLC1_FORMAT_MAX, OLC1_MAGIC, OLT1_FORMAT_MAX, OLT1_MAGIC,
};
pub use prob_set::ProbSetCategory;
use prob_set::load_category_tables;

use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Debug, Error)]
pub enum ContentError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("bad object file {path}: {msg}")]
    BadObject { path: String, msg: String },
    #[error("content path not found: {0}")]
    MissingRoot(String),
    #[error("binary cache: {0}")]
    Binary(String),
}

/// Minimal object definition needed by the server (expand over time).
#[derive(Debug, Clone)]
pub struct ObjectDef {
    pub id: i32,
    pub description: String,
    pub name: String,
    pub containable: bool,
    pub permanent: bool,
    pub blocks_walking: bool,
    pub food_value: i32,
    pub heat_value: f32,
    /// Haxe `mapChance` — weight for natural world gen in listed biomes.
    pub map_chance: f32,
    /// Biome ids this natural object may spawn in.
    pub biomes: Vec<i32>,
    /// Max uses for multi-use objects (0 = single / unknown).
    pub num_uses: i32,
    /// Container slot count (Haxe numSlots). 0 = not a container.
    pub num_slots: i32,
    /// Haxe `floor=1` — object is a floor tile, not a ground object.
    /// DROP must not place these on the object layer (skip / use floor path later).
    pub floor: bool,
    /// Synthetic multi-use dummy ids for uses `1..num_uses-1` (Haxe `dummyObjects`).
    /// Index `uses - 1` → dummy id. Full `num_uses` uses the base [`Self::id`].
    pub dummy_ids: Vec<i32>,
    /// Haxe `ObjectData.useChance` — probabilistic use skip (tool durability).
    /// Second value on `numUses=N,chance` line; 0 = always consume a use.
    pub use_chance: f32,
    /// Haxe `ObjectData.speedMult` — move/water-drift multiplier (default 1).
    pub speed_mult: f32,
    /// Haxe `ObjectData.winterDecayFactor` — wild-food winter multi-use decay (0 = none).
    pub winter_decay_factor: f32,
    /// Haxe `ObjectData.springRegrowFactor` — spring multi-use regrow (0 = none).
    pub spring_regrow_factor: f32,
    /// Haxe `ObjectData.decayFactor` (default 1; ≤0 disables long-term decay).
    pub decay_factor: f32,
    /// Haxe `ObjectData.decaysToObj` (0 → trash pit 618 for permanent objects).
    pub decays_to_obj: i32,
    /// Haxe `ObjectData.rValue` — wall/floor insulation; non-clothing + rValue>0 ⇒ wall.
    pub r_value: f32,
    /// Haxe `ObjectData.clothing` (`"n"` = not clothing).
    pub clothing: String,
    /// Haxe `ObjectData.countsOrGrowsAs` (0 = count as own / parent id).
    pub counts_or_grows_as: i32,
    /// Haxe `ObjectData.carftingSteps` (craft depth; 0 natural; used in decay tech factor).
    pub crafting_steps: i32,
    /// Haxe `ObjectData.useDistance` — USE/DROP Chebyshev-style squared range (min 1).
    pub use_distance: i32,
    /// Haxe `ObjectData.deadlyDistance` — combat / ranged min-range (tiles, float).
    pub deadly_distance: f32,
    /// Haxe `ObjectData.moves` — animal walk class (`>0` ⇒ isAnimal; often from time-move).
    pub moves: i32,
    /// Haxe `ObjectData.damage` — weapon/animal hit damage (and wound bleed DPS).
    /// Default 0; ServerSettings.PatchObjectData sets combat values.
    pub damage: f32,
    /// Haxe `ObjectData.damageProtectionFactor` — held protection (1 = none).
    pub damage_protection_factor: f32,
    /// Haxe `ObjectData.woundFactor` — wound when `food_store_max < not_reduced * factor`.
    /// Default 0.5; Rattle Snake patched to 0.98.
    pub wound_factor: f32,
    /// Haxe `ObjectData.male` — person sex (`true` = male). Default false.
    /// // Haxe: ObjectData.male
    pub male: bool,
    /// Haxe `ObjectData.containSize` — how large this object is when stored in a container.
    /// Gate: `containSize > container.slotSize` refuses put. Default 0.
    /// // Haxe: ObjectData.containSize / ObjectHelper.canBePlacedIn
    pub contain_size: f32,
    /// Haxe `ObjectData.slotSize` — max containSize accepted by this container's slots.
    /// Text key is `slotsSize=`. Default 1.
    /// // Haxe: ObjectData.slotSize
    pub slot_size: f32,
}

impl ObjectDef {
    pub fn empty(id: i32) -> Self {
        Self {
            id,
            description: String::new(),
            name: String::new(),
            containable: false,
            permanent: false,
            blocks_walking: false,
            food_value: 0,
            heat_value: 0.0,
            map_chance: 0.0,
            biomes: Vec::new(),
            num_uses: 0,
            num_slots: 0,
            floor: false,
            dummy_ids: Vec::new(),
            use_chance: 0.0,
            speed_mult: 1.0,
            winter_decay_factor: 0.0,
            spring_regrow_factor: 0.0,
            decay_factor: 1.0,
            decays_to_obj: 0,
            r_value: 0.0,
            clothing: "n".into(),
            counts_or_grows_as: 0,
            crafting_steps: 0,
            use_distance: 1,
            deadly_distance: 0.0,
            moves: 0,
            damage: 0.0,
            damage_protection_factor: 1.0,
            wound_factor: 0.5,
            male: false,
            contain_size: 0.0,
            slot_size: 1.0,
        }
    }

    pub fn is_container(&self) -> bool {
        self.num_slots > 0
    }

    /// Haxe `ObjectHelper.canBePlacedIn` size gate only: `containSize <= container.slotSize`.
    /// // Haxe: ObjectHelper.canBePlacedIn containSize/slotSize
    #[inline]
    pub fn contain_fits_in_container(&self, container: &ObjectDef) -> bool {
        self.contain_size <= container.slot_size
    }

    /// Haxe `ObjectData.isAnimal` — `moves > 0` and not Mosquito Swarm 2156.
    // Haxe: ObjectData.isAnimal
    #[inline]
    pub fn is_animal(&self) -> bool {
        self.moves > 0 && self.id != 2156
    }

    /// Haxe clamp: `useDistance < 1 → 1`.
    // Haxe: TransitionHelper.checkIfNotMovingAndCloseEnough useDistance clamp
    #[inline]
    pub fn effective_use_distance(&self) -> i32 {
        if self.use_distance < 1 {
            1
        } else {
            self.use_distance
        }
    }

    /// True when this object is floor-only (Haxe `floor=1`).
    pub fn is_floor(&self) -> bool {
        self.floor
    }

    /// Haxe `ObjectData.isClothing` — clothing does not start with `n`.
    #[inline]
    pub fn is_clothing(&self) -> bool {
        !self.clothing.is_empty() && !self.clothing.starts_with('n')
    }

    /// Haxe `ObjectData.isWall` — not clothing and rValue > 0.
    #[inline]
    pub fn is_wall(&self) -> bool {
        !self.is_clothing() && self.r_value > 0.0
    }

    /// Haxe `ObjectData.getInsulation` for ground/wall/floor (non-clothing returns `rValue`).
    ///
    /// Clothing on the ground is rare for IsProtected; treat as 0 (Haxe: `isClothing ? 0`).
    #[inline]
    pub fn insulation_for_protection(&self) -> f32 {
        if self.is_clothing() {
            0.0
        } else {
            self.r_value
        }
    }
}

/// One OHOL transition: actor + target → new_actor + new_target.
#[derive(Debug, Clone)]
pub struct Transition {
    pub actor_id: i32,
    pub target_id: i32,
    pub new_actor_id: i32,
    pub new_target_id: i32,
    pub last_use_actor: bool,
    pub last_use_target: bool,
    pub auto_decay_seconds: f32,
    pub reverse_use_actor: bool,
    pub reverse_use_target: bool,
    pub no_use_actor: bool,
    pub no_use_target: bool,
    /// Field 7: move type (0 none, 1–3 animal walk class). Haxe `TransitionData.move`.
    pub move_dist: i32,
    /// Field 8: desired animal step radius (`desiredMoveDist`). Default 0 → use `move_dist`.
    pub desired_move_dist: i32,
    /// Field 3: Haxe `actorMinUseFraction` (1 = actor must be full).
    pub actor_min_use_fraction: f32,
    /// Field 4: Haxe `targetMinUseFraction` (1 = target must be full).
    pub target_min_use_fraction: f32,
    /// Haxe `switchNumberOfUses` — set by ServerSettings patches, not file data.
    pub switch_number_of_uses: bool,
    /// Haxe `targetNumberOfUses` — force uses after transform (`-1` = unset).
    pub target_number_of_uses: i32,
    /// Haxe `TransitionData.isPickupOrDrop` — horse cart / grave basket nest swap on USE.
    /// Not in transition files; set by ServerSettings.PatchTransitions.
    pub is_pickup_or_drop: bool,
}

/// Loaded game content tables (immutable after load; share via Arc).
#[derive(Debug, Default, Clone)]
pub struct ContentDb {
    pub objects: HashMap<i32, ObjectDef>,
    /// Primary non-last-use transitions keyed by (actor, target).
    pub transitions: HashMap<(i32, i32), Transition>,
    /// Last-use actor and/or target transitions (Haxe LA / LT / L filenames).
    pub transitions_last_use: HashMap<(i32, i32), Transition>,
    /// Max-use target transitions (Haxe `maxUseTransitions` — well site full → complete).
    pub transitions_max_use: HashMap<(i32, i32), Transition>,
    pub transition_count: usize,
    pub last_use_transition_count: usize,
    pub data_version: i32,
    /// biome_id → (object ids with mapChance, total chance) for natural gen.
    pub biome_spawn: HashMap<i32, BiomeSpawnTable>,
    /// target object id → auto-decay transition (actor typically −1).
    /// Includes hour-based decays (`auto_decay_seconds < 0`) and move transitions.
    pub auto_decays: HashMap<i32, Transition>,
    /// Haxe `ObjectData.secondTimeOutcome` / `secondTimeOutcomeTimeToChange`
    /// (ServerSettings patches: goose pond chain, rabbit holes, …).
    /// Map: object id → (new_id, seconds threshold per full map cycle).
    pub second_time_outcomes: HashMap<i32, (i32, f32)>,
    /// Dummy object id → parent base id (Haxe `dummyParent`).
    pub dummy_parent: HashMap<i32, i32>,
    /// Category parent id → member object ids (non-pattern only for expansion).
    pub categories: HashMap<i32, Vec<i32>>,
    /// Haxe `Category` with `probSet=true` (TransformTarget weighted random outcomes).
    pub prob_sets: HashMap<i32, ProbSetCategory>,
    /// Haxe `ObjectData.person` race color for person objects (Black=1 Brown=3 White=4 Ginger=6).
    /// Only non-zero races are stored (TH-MULTI-POLISH loved biome lookup).
    pub person_race: HashMap<i32, i32>,
    /// Haxe `TransitionData.aiShouldIgnore` — (actor, target) craft-AI ignore edges.
    ///
    /// Side-table (not a per-Transition file field): set by
    /// [`apply_default_ai_should_ignore_patches`] from ServerSettings.PatchTransitions.
    /// Primary (and dual primary+last-use) ignores for reverse-graph craft filters.
    // Haxe: TransitionData.aiShouldIgnore + ServerSettings.PatchTransitions
    pub ai_should_ignore: HashSet<(i32, i32)>,
    /// Haxe last-use-only `aiShouldIgnore` edges (e.g. pond water LA/LT).
    ///
    /// Not loaded into reverse craft graph (primary water-fill stays craftable);
    /// used by last-use transition lookup / meta builders.
    // Haxe: getTransition(a,t,false,true) aiShouldIgnore only (pond 141/142)
    pub ai_should_ignore_last_use: HashSet<(i32, i32)>,
    /// Haxe `ObjectData.alternativeTransitionOutcome` (ServerSettings patches).
    /// // Haxe: ObjectData.alternativeTransitionOutcome
    /// // TH-ALT-OUTCOME
    pub alt_outcomes_object: HashMap<i32, Vec<i32>>,
    /// Haxe `TransitionData.alternativeTransitionOutcome` (ServerSettings patches).
    /// // Haxe: TransitionData.alternativeTransitionOutcome
    /// // TH-ALT-OUTCOME
    pub alt_outcomes_transition: HashMap<(i32, i32), Vec<i32>>,
    /// Haxe `ObjectData.fortificationObjId`.
    /// // TH-ALT-OUTCOME
    pub fortification_obj_id: HashMap<i32, i32>,
    /// Haxe `ObjectData.fortificationValue`.
    /// // TH-ALT-OUTCOME
    pub fortification_value: HashMap<i32, f32>,
    /// Load timing (ms) — set by [`load_content`].
    pub load_objects_ms: u64,
    pub load_transitions_ms: u64,
    pub load_total_ms: u64,
}

/// Weighted natural spawn list for one biome (Haxe `biomeObjectData`).
#[derive(Debug, Clone, Default)]
pub struct BiomeSpawnTable {
    pub total_chance: f32,
    /// (object_id, map_chance)
    pub entries: Vec<(i32, f32)>,
}

/// Object file parse result: def + Haxe `person` race (0 = not a person).
#[derive(Debug, Clone)]
pub struct ParsedObject {
    pub def: ObjectDef,
    pub person: i32,
}

impl ContentDb {
    pub fn get(&self, id: i32) -> Option<&ObjectDef> {
        self.objects.get(&id)
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// Haxe `ObjectData.person` for a person object id (po_id). 0 if not a person.
    #[inline]
    pub fn person_color(&self, object_id: i32) -> i32 {
        self.person_race.get(&object_id).copied().unwrap_or(0)
    }

    /// Haxe L1260–1261: transition alt list if non-empty, else new-target object list
    /// (and dummy parent base). Also falls back to current target id tables.
    // Haxe: TransitionHelper alternativeTransitionOutcome resolve
    // TH-ALT-OUTCOME
    pub fn alternative_outcomes_for(
        &self,
        actor_id: i32,
        target_id: i32,
        new_target_id: i32,
    ) -> &[i32] {
        if let Some(v) = self.alt_outcomes_transition.get(&(actor_id, target_id)) {
            if !v.is_empty() {
                return v.as_slice();
            }
        }
        let base = self.resolve_base_id(new_target_id);
        if let Some(v) = self.alt_outcomes_object.get(&new_target_id) {
            if !v.is_empty() {
                return v.as_slice();
            }
        }
        if base != new_target_id {
            if let Some(v) = self.alt_outcomes_object.get(&base) {
                if !v.is_empty() {
                    return v.as_slice();
                }
            }
        }
        let tbase = self.resolve_base_id(target_id);
        if let Some(v) = self.alt_outcomes_object.get(&target_id) {
            if !v.is_empty() {
                return v.as_slice();
            }
        }
        self.alt_outcomes_object
            .get(&tbase)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Resolve dummy multi-use id → base object id (Haxe `dummyParent`).
    #[inline]
    pub fn resolve_base_id(&self, id: i32) -> i32 {
        self.dummy_parent.get(&id).copied().unwrap_or(id)
    }

    /// Wire object id for multi-use display (Haxe `ObjectHelper.dummyId`).
    ///
    /// - `uses >= num_uses` or no dummies → base id  
    /// - `uses` in `1..num_uses-1` → `dummy_ids[uses-1]`
    pub fn wire_id_for_uses(&self, base_id: i32, uses: i32) -> i32 {
        let base = self.resolve_base_id(base_id);
        let Some(def) = self.objects.get(&base) else {
            return base_id;
        };
        if def.num_uses < 2 || def.dummy_ids.is_empty() {
            return base;
        }
        if uses >= def.num_uses || uses <= 0 {
            return base;
        }
        def.dummy_ids
            .get((uses as usize).saturating_sub(1))
            .copied()
            .unwrap_or(base)
    }

    /// Lookup a normal (non last-use) transition for actor on target.
    /// Resolves multi-use dummy ids to parents first (Haxe GetTransition).
    pub fn find_transition(&self, actor: i32, target: i32) -> Option<&Transition> {
        let a = self.resolve_base_id(actor);
        let t = self.resolve_base_id(target);
        self.transitions.get(&(a, t))
    }

    /// Lookup last-use variant if present (Haxe last-use when multi-use object is exhausted).
    pub fn find_transition_last_use(&self, actor: i32, target: i32) -> Option<&Transition> {
        let a = self.resolve_base_id(actor);
        let t = self.resolve_base_id(target);
        self.transitions_last_use.get(&(a, t))
    }

    /// Prefer last-use table when `prefer_last_use`, else normal; fall back either way.
    pub fn find_transition_prefer(
        &self,
        actor: i32,
        target: i32,
        prefer_last_use: bool,
    ) -> Option<&Transition> {
        if prefer_last_use {
            self.find_transition_last_use(actor, target)
                .or_else(|| self.find_transition(actor, target))
        } else {
            self.find_transition(actor, target)
                .or_else(|| self.find_transition_last_use(actor, target))
        }
    }

    /// Haxe `GetTransition(..., maxUseTarget=true)` — complete when reverse at max uses.
    #[inline]
    pub fn find_transition_max_use(&self, actor: i32, target: i32) -> Option<&Transition> {
        let a = self.resolve_base_id(actor);
        let t = self.resolve_base_id(target);
        self.transitions_max_use.get(&(a, t))
    }

    /// Haxe `TransitionData.aiShouldIgnore` for craft-AI edge `(actor, target)`.
    ///
    /// Checks primary table only (reverse-graph / default craft path).
    // Haxe: TransitionData.aiShouldIgnore
    #[inline]
    pub fn transition_ai_should_ignore(&self, actor: i32, target: i32) -> bool {
        let a = self.resolve_base_id(actor);
        let t = self.resolve_base_id(target);
        self.ai_should_ignore.contains(&(a, t))
    }

    /// Haxe `aiShouldIgnore` with last-use vs primary distinction.
    ///
    /// When `last_use`, primary **or** last-use-only tables match (Haxe GetTransition
    /// with lastUse flags). Primary-only ignores always apply.
    // Haxe: TransitionData.aiShouldIgnore + lastUseActor/Target maps
    #[inline]
    pub fn transition_ai_should_ignore_ex(
        &self,
        actor: i32,
        target: i32,
        last_use: bool,
    ) -> bool {
        let a = self.resolve_base_id(actor);
        let t = self.resolve_base_id(target);
        if self.ai_should_ignore.contains(&(a, t)) {
            return true;
        }
        last_use && self.ai_should_ignore_last_use.contains(&(a, t))
    }
}

/// Scan object text for Haxe `person=N` race field.
pub fn parse_person_from_text(text: &str) -> i32 {
    for line in text.lines() {
        for part in line.split(',') {
            if let Some(rest) = part.trim().strip_prefix("person=") {
                return rest
                    .split(|c| c == ',' || c == '#')
                    .next()
                    .unwrap_or(rest)
                    .trim()
                    .parse()
                    .unwrap_or(0);
            }
        }
    }
    0
}

/// Load all `objects/*.txt` under a OneLifeData7 root (or skip if missing).
/// Object files are parsed in parallel for fast server restart.
pub fn load_content(root: impl AsRef<Path>) -> Result<ContentDb, ContentError> {
    let root = root.as_ref();
    if !root.exists() {
        return Err(ContentError::MissingRoot(root.display().to_string()));
    }

    let t0 = Instant::now();
    let mut db = ContentDb::default();

    let version_path = root.join("dataVersionNumber.txt");
    if version_path.exists() {
        if let Ok(s) = fs::read_to_string(&version_path) {
            db.data_version = s.trim().parse().unwrap_or(0);
        }
    }

    let objects_dir = root.join("objects");
    if !objects_dir.is_dir() {
        warn!(path = %objects_dir.display(), "objects directory missing");
        return Ok(db);
    }

    let mut paths = Vec::new();
    for entry in fs::read_dir(&objects_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if !stem.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        paths.push(path);
    }

    let results: Vec<Result<ParsedObject, ContentError>> = paths
        .par_iter()
        .map(|path| load_object_file_full(path))
        .collect();

    let mut loaded = 0u32;
    let mut errors = 0u32;
    for res in results {
        match res {
            Ok(ParsedObject { def, person }) => {
                if def.map_chance > 0.0 && !def.biomes.is_empty() {
                    for &b in &def.biomes {
                        let table = db.biome_spawn.entry(b).or_default();
                        table.total_chance += def.map_chance;
                        table.entries.push((def.id, def.map_chance));
                    }
                }
                if person != 0 {
                    db.person_race.insert(def.id, person);
                }
                db.objects.insert(def.id, def);
                loaded += 1;
            }
            Err(e) => {
                errors += 1;
                debug!(error = %e, "skip object");
            }
        }
    }

    // Fill person_race if load_object_file_full stub left person=0 (parallel re-scan).
    if db.person_race.is_empty() {
        let races: Vec<(i32, i32)> = paths
            .par_iter()
            .filter_map(|path| {
                let text = fs::read_to_string(path).ok()?;
                let person = parse_person_from_text(&text);
                if person == 0 {
                    return None;
                }
                let stem = path.file_stem()?.to_str()?;
                let id: i32 = stem.parse().ok()?;
                Some((id, person))
            })
            .collect();
        for (id, p) in races {
            db.person_race.insert(id, p);
        }
    }

    // Haxe ObjectData: allocate dummy ids after max real object id (nextObjectNumber).
    assign_multi_use_dummies(&mut db, root);

    let objects_ms = t0.elapsed().as_millis() as u64;
    db.load_objects_ms = objects_ms;
    info!(
        loaded,
        errors,
        biomes_with_spawns = db.biome_spawn.len(),
        dummies = db.dummy_parent.len(),
        persons = db.person_race.len(),
        version = db.data_version,
        ms = objects_ms,
        root = %root.display(),
        "content objects loaded"
    );

    load_categories_into(&mut db, &root.join("categories"));

    let t1 = Instant::now();
    load_transitions_into(&mut db, &root.join("transitions"))?;
    expand_category_transitions(&mut db);
    // Haxe TransitionImporter.changeToolTransitions (after category expand).
    change_tool_transitions(&mut db);
    // Haxe ServerSettings.PatchObjectData secondTimeOutcome chains.
    apply_default_second_time_outcomes(&mut db);
    // Haxe ServerSettings.PatchObjectData decaysToObj / decayFactor / countsOrGrowsAs / rValue.
    apply_default_decay_object_patches(&mut db);
    // CLOTHING-CONTAIN-SIZE: ServerSettings.PatchObjectData containSize / containable.
    apply_default_contain_size_patches(&mut db);
    // TH-MULTI-POLISH: ServerSettings useChance + switchNumberOfUses patches.
    apply_default_use_chance_patches(&mut db);
    apply_default_switch_number_of_uses_patches(&mut db);
    // TH-HORSE: ServerSettings.PatchTransitions horse cart pickup/drop + tire fixes.
    apply_default_horse_transition_patches(&mut db);
    // TH-ALT-OUTCOME: alternativeTransitionOutcome + fortification tables.
    apply_default_alternative_outcome_patches(&mut db);
    // C-SS-AI-IGNORE: ServerSettings.PatchTransitions aiShouldIgnore table.
    apply_default_ai_should_ignore_patches(&mut db);
    // IS-CLOSE / action_range: weapon useDistance + deadlyDistance + animal moves.
    apply_default_weapon_range_patches(&mut db);
    // Haxe PatchObjectData animal deadlyDistance = AnimalDeadlyDistanceFactor (0.5).
    apply_default_animal_deadly_distance_patches(&mut db);
    // WEAPON-WOUND-TRANS: damage / woundFactor / protection + wound bleed DPS.
    apply_default_combat_damage_patches(&mut db);
    apply_animal_moves_from_transitions(&mut db);
    db.load_transitions_ms = t1.elapsed().as_millis() as u64;
    db.load_total_ms = t0.elapsed().as_millis() as u64;

    info!(
        total_ms = db.load_total_ms,
        objects_ms = db.load_objects_ms,
        transitions_ms = db.load_transitions_ms,
        objects = db.object_count(),
        transitions = db.transition_count,
        ai_should_ignore = db.ai_should_ignore.len(),
        categories = db.categories.len(),
        prob_sets = db.prob_sets.len(),
        "content ready (timed)"
    );

    Ok(db)
}

/// Haxe: dummy objects for `numUses >= 2` get sequential free ids starting at
/// `nextObjectNumber` (or max_id+1).
fn assign_multi_use_dummies(db: &mut ContentDb, root: &Path) {
    let mut next = fs::read_to_string(root.join("objects").join("nextObjectNumber.txt"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let max_id = db.objects.keys().copied().max().unwrap_or(0);
    if next <= max_id {
        next = max_id + 1;
    }
    let mut multi: Vec<i32> = db
        .objects
        .iter()
        .filter(|(_, d)| d.num_uses >= 2)
        .map(|(id, _)| *id)
        .collect();
    multi.sort_unstable();
    for id in multi {
        let n = db.objects.get(&id).map(|d| d.num_uses).unwrap_or(0);
        if n < 2 {
            continue;
        }
        let mut dummies = Vec::with_capacity((n - 1) as usize);
        for _ in 0..(n - 1) {
            let did = next;
            next += 1;
            dummies.push(did);
            db.dummy_parent.insert(did, id);
        }
        if let Some(def) = db.objects.get_mut(&id) {
            def.dummy_ids = dummies;
        }
    }
}

/// Load `categories/*.txt` (Haxe Category). Pattern categories stored but not expanded.
/// Also loads `probSet` categories for [`crate::` TransformTarget] weighted outcomes.
pub(crate) fn load_categories_into(db: &mut ContentDb, dir: &Path) {
    let (cats, probs, _) = load_category_tables(dir);
    db.categories = cats;
    db.prob_sets = probs;
}

include!("ai_should_ignore_patches.inc.rs");
include!("alt_outcome_patches.inc.rs");
include!("lib_tail.inc.rs");
