//! Content loading from OneLifeData7-style text files.
//!
//! Phase B: load a subset of object definitions. Full transition graph later.

#![forbid(unsafe_code)]

use rayon::prelude::*;
use std::collections::HashMap;
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
        }
    }

    pub fn is_container(&self) -> bool {
        self.num_slots > 0
    }

    /// True when this object is floor-only (Haxe `floor=1`).
    pub fn is_floor(&self) -> bool {
        self.floor
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
}

/// Loaded game content tables (immutable after load; share via Arc).
#[derive(Debug, Default)]
pub struct ContentDb {
    pub objects: HashMap<i32, ObjectDef>,
    /// Primary non-last-use transitions keyed by (actor, target).
    pub transitions: HashMap<(i32, i32), Transition>,
    /// Last-use actor and/or target transitions (Haxe LA / LT / L filenames).
    pub transitions_last_use: HashMap<(i32, i32), Transition>,
    pub transition_count: usize,
    pub last_use_transition_count: usize,
    pub data_version: i32,
    /// biome_id → (object ids with mapChance, total chance) for natural gen.
    pub biome_spawn: HashMap<i32, BiomeSpawnTable>,
    /// target object id → auto-decay transition (actor typically −1).
    pub auto_decays: HashMap<i32, Transition>,
    /// Dummy object id → parent base id (Haxe `dummyParent`).
    pub dummy_parent: HashMap<i32, i32>,
    /// Category parent id → member object ids (non-pattern only for expansion).
    pub categories: HashMap<i32, Vec<i32>>,
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

impl ContentDb {
    pub fn get(&self, id: i32) -> Option<&ObjectDef> {
        self.objects.get(&id)
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
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

    let results: Vec<Result<ObjectDef, ContentError>> = paths
        .par_iter()
        .map(|path| load_object_file(path))
        .collect();

    let mut loaded = 0u32;
    let mut errors = 0u32;
    for res in results {
        match res {
            Ok(def) => {
                if def.map_chance > 0.0 && !def.biomes.is_empty() {
                    for &b in &def.biomes {
                        let table = db.biome_spawn.entry(b).or_default();
                        table.total_chance += def.map_chance;
                        table.entries.push((def.id, def.map_chance));
                    }
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

    // Haxe ObjectData: allocate dummy ids after max real object id (nextObjectNumber).
    assign_multi_use_dummies(&mut db, root);

    let objects_ms = t0.elapsed().as_millis() as u64;
    db.load_objects_ms = objects_ms;
    info!(
        loaded,
        errors,
        biomes_with_spawns = db.biome_spawn.len(),
        dummies = db.dummy_parent.len(),
        version = db.data_version,
        ms = objects_ms,
        root = %root.display(),
        "content objects loaded"
    );

    load_categories_into(&mut db, &root.join("categories"));

    let t1 = Instant::now();
    load_transitions_into(&mut db, &root.join("transitions"))?;
    expand_category_transitions(&mut db);
    db.load_transitions_ms = t1.elapsed().as_millis() as u64;
    db.load_total_ms = t0.elapsed().as_millis() as u64;

    info!(
        total_ms = db.load_total_ms,
        objects_ms = db.load_objects_ms,
        transitions_ms = db.load_transitions_ms,
        objects = db.object_count(),
        transitions = db.transition_count,
        categories = db.categories.len(),
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
fn load_categories_into(db: &mut ContentDb, dir: &Path) {
    if !dir.is_dir() {
        return;
    }
    let mut n = 0usize;
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let mut parent = 0i32;
        let mut pattern = false;
        let mut members = Vec::new();
        let mut in_objects = false;
        for line in text.lines() {
            let line = line.trim().trim_end_matches('\r');
            if line.is_empty() {
                continue;
            }
            if !in_objects {
                if let Some(rest) = line.strip_prefix("parentID=") {
                    parent = rest.parse().unwrap_or(0);
                } else if line == "pattern" || line.starts_with("pattern=") {
                    pattern = true;
                } else if line.starts_with("numObjects=") {
                    in_objects = true;
                }
                continue;
            }
            // member lines: "34" or "34 0.5"
            let id_s = line.split_whitespace().next().unwrap_or("");
            if let Ok(id) = id_s.parse::<i32>() {
                members.push(id);
            }
        }
        if parent != 0 && !pattern && !members.is_empty() {
            db.categories.insert(parent, members);
            n += 1;
        }
    }
    info!(categories = n, "content categories loaded (non-pattern)");
}

/// Haxe `createAndaddCategoryTransitions` — expand actor/target category parents
/// into concrete member transitions (e.g. `@ Shallow Digger` 722 → sharp stone 34).
fn expand_category_transitions(db: &mut ContentDb) {
    if db.categories.is_empty() {
        return;
    }
    let base: Vec<Transition> = db
        .transitions
        .values()
        .cloned()
        .chain(db.transitions_last_use.values().cloned())
        .collect();
    let mut added = 0usize;
    for t in base {
        let actor_cat = db.categories.get(&t.actor_id).cloned();
        let target_cat = db.categories.get(&t.target_id).cloned();
        match (actor_cat, target_cat) {
            (Some(actors), None) => {
                for aid in actors {
                    let mut nt = t.clone();
                    if nt.new_actor_id == t.actor_id {
                        nt.new_actor_id = aid;
                    }
                    nt.actor_id = aid;
                    if insert_expanded(db, nt) {
                        added += 1;
                    }
                }
            }
            (None, Some(targets)) => {
                for tid in targets {
                    let mut nt = t.clone();
                    if nt.new_target_id == t.target_id {
                        nt.new_target_id = tid;
                    }
                    nt.target_id = tid;
                    if insert_expanded(db, nt) {
                        added += 1;
                    }
                }
            }
            (Some(actors), Some(targets)) => {
                for aid in &actors {
                    for tid in &targets {
                        let mut nt = t.clone();
                        if nt.new_actor_id == t.actor_id {
                            nt.new_actor_id = *aid;
                        }
                        if nt.new_target_id == t.target_id {
                            nt.new_target_id = *tid;
                        }
                        nt.actor_id = *aid;
                        nt.target_id = *tid;
                        if insert_expanded(db, nt) {
                            added += 1;
                        }
                    }
                }
            }
            (None, None) => {}
        }
    }
    db.transition_count = db.transitions.len();
    db.last_use_transition_count = db.transitions_last_use.len();
    info!(added, "content category transitions expanded");
}

fn insert_expanded(db: &mut ContentDb, t: Transition) -> bool {
    let key = (t.actor_id, t.target_id);
    if t.last_use_actor || t.last_use_target {
        if db.transitions_last_use.contains_key(&key) {
            return false;
        }
        db.transitions_last_use.insert(key, t);
    } else {
        if db.transitions.contains_key(&key) {
            return false;
        }
        db.transitions.insert(key, t);
    }
    true
}

fn load_transitions_into(db: &mut ContentDb, dir: &Path) -> Result<(), ContentError> {
    if !dir.is_dir() {
        warn!(path = %dir.display(), "transitions directory missing");
        return Ok(());
    }

    let mut paths = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("txt") {
            paths.push(path);
        }
    }

    let results: Vec<Result<Transition, ContentError>> = paths
        .par_iter()
        .map(|path| load_transition_file(path))
        .collect();

    let mut loaded = 0usize;
    let mut loaded_last_use = 0usize;
    let mut errors = 0u32;

    for res in results {
        match res {
            Ok(t) => {
                // Auto-decay / animal move: actor -1 (TIME).
                if t.auto_decay_seconds > 0.0 && t.actor_id < 0 {
                    db.auto_decays.insert(t.target_id, t.clone());
                }
                // Also index pure animal-move transitions (autoDecaySeconds may be used).
                if t.actor_id < 0 && t.move_dist > 0 {
                    db.auto_decays
                        .entry(t.target_id)
                        .or_insert_with(|| t.clone());
                }
                if t.last_use_actor || t.last_use_target {
                    db.transitions_last_use
                        .insert((t.actor_id, t.target_id), t);
                    loaded_last_use += 1;
                } else {
                    db.transitions.insert((t.actor_id, t.target_id), t);
                    loaded += 1;
                }
            }
            Err(e) => {
                errors += 1;
                debug!(error = %e, "skip transition");
            }
        }
    }

    db.transition_count = loaded;
    db.last_use_transition_count = loaded_last_use;
    info!(
        loaded,
        loaded_last_use,
        errors,
        path = %dir.display(),
        "content transitions loaded"
    );
    Ok(())
}

/// Parse `actor_target.txt` or `actor_target_LA.txt` / `_LT` / `_L`.
pub fn load_transition_file(path: &Path) -> Result<Transition, ContentError> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| ContentError::BadObject {
            path: path.display().to_string(),
            msg: "bad filename".into(),
        })?;
    let parts: Vec<&str> = stem.split('_').collect();
    if parts.len() < 2 {
        return Err(ContentError::BadObject {
            path: path.display().to_string(),
            msg: "filename needs actor_target".into(),
        });
    }
    let actor_id: i32 = parts[0].parse().map_err(|_| ContentError::BadObject {
        path: path.display().to_string(),
        msg: "bad actor id".into(),
    })?;
    let target_id: i32 = parts[1].parse().map_err(|_| ContentError::BadObject {
        path: path.display().to_string(),
        msg: "bad target id".into(),
    })?;
    let flag = parts.get(2).copied().unwrap_or("");
    let last_use_actor = flag == "LA";
    let last_use_target = flag == "LT" || flag == "L";

    let text = fs::read_to_string(path)?;
    let line = text.lines().next().unwrap_or("").trim();
    let data: Vec<&str> = line.split_whitespace().collect();
    if data.len() < 2 {
        return Err(ContentError::BadObject {
            path: path.display().to_string(),
            msg: "need at least newActor newTarget".into(),
        });
    }

    let parse_i = |i: usize, default: i32| -> i32 {
        data.get(i).and_then(|s| s.parse().ok()).unwrap_or(default)
    };
    let parse_f = |i: usize| -> f32 {
        data.get(i).and_then(|s| s.parse().ok()).unwrap_or(0.0)
    };
    let parse_b = |i: usize| -> bool { data.get(i).map(|s| *s == "1").unwrap_or(false) };

    Ok(Transition {
        actor_id,
        target_id,
        new_actor_id: parse_i(0, 0),
        new_target_id: parse_i(1, 0),
        last_use_actor,
        last_use_target,
        auto_decay_seconds: parse_f(2),
        reverse_use_actor: parse_b(5),
        reverse_use_target: parse_b(6),
        no_use_actor: parse_b(9),
        no_use_target: parse_b(10),
        move_dist: parse_i(7, 0),
        desired_move_dist: parse_i(8, 0),
    })
}

/// Parse a single object description file (OHOL / Open Life line-oriented format).
///
/// First line may be bare `33` or `id=100`. Second line is description when present.
pub fn load_object_file(path: &Path) -> Result<ObjectDef, ContentError> {
    let text = fs::read_to_string(path)?;
    let mut lines = text.lines().peekable();

    let id_line = lines.next().ok_or_else(|| ContentError::BadObject {
        path: path.display().to_string(),
        msg: "empty file".into(),
    })?;
    let id_raw = id_line.trim();
    let id: i32 = id_raw
        .strip_prefix("id=")
        .unwrap_or(id_raw)
        .trim()
        .parse()
        .map_err(|_| ContentError::BadObject {
            path: path.display().to_string(),
            msg: format!("bad id line: {id_line}"),
        })?;

    let mut def = ObjectDef::empty(id);

    // Description is the next non-key=value line (bare name), if any.
    if let Some(peek) = lines.peek() {
        let t = peek.trim();
        if !t.is_empty() && !t.contains('=') {
            let desc = lines.next().unwrap().to_string();
            def.description = desc.clone();
            def.name = description_to_name(&desc);
        }
    }

    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Keys may be embedded in comma-joined groups: permanent=1,minPickupAge=3
        for part in line.split(',') {
            let part = part.trim();
            if let Some(rest) = part.strip_prefix("containable=") {
                def.containable = rest.starts_with('1') || rest.eq_ignore_ascii_case("true");
            } else if let Some(rest) = part.strip_prefix("permanent=") {
                def.permanent = rest.starts_with('1') || rest.eq_ignore_ascii_case("true");
            } else if let Some(rest) = part.strip_prefix("blocksWalking=") {
                def.blocks_walking = rest.starts_with('1') || rest.eq_ignore_ascii_case("true");
            } else if let Some(rest) = part.strip_prefix("foodValue=") {
                def.food_value = rest.parse().unwrap_or(0);
            } else if let Some(rest) = part.strip_prefix("heatValue=") {
                def.heat_value = rest.parse().unwrap_or(0.0);
            } else if let Some(rest) = part.strip_prefix("numUses=") {
                let num = rest.split(|c| c == ',' || c == '#').next().unwrap_or(rest);
                def.num_uses = num.parse().unwrap_or(0);
            } else if let Some(rest) = part.strip_prefix("numSlots=") {
                // numSlots=4#timeStretch=1.000000
                let num = rest.split(|c| c == ',' || c == '#').next().unwrap_or(rest);
                def.num_slots = num.parse().unwrap_or(0);
            } else if let Some(rest) = part.strip_prefix("floor=") {
                // floor=1 — floor-only objects (roads, stone floors); not ground placeables.
                def.floor = rest.starts_with('1') || rest.eq_ignore_ascii_case("true");
            } else if let Some(rest) = part.strip_prefix("mapChance=") {
                // mapChance=1.000000#biomes_0,3,4,5  (biomes may span later commas —
                // re-parse full line segment after mapChance= when '#' present)
                // Prefer full line when this part looks truncated.
                let full = if line.contains("mapChance=") {
                    line.split("mapChance=")
                        .nth(1)
                        .unwrap_or(rest)
                } else {
                    rest
                };
                let (chance_s, rest2) = if let Some(i) = full.find('#') {
                    (&full[..i], Some(&full[i + 1..]))
                } else {
                    (full.split(',').next().unwrap_or(full), None)
                };
                def.map_chance = chance_s.trim().parse().unwrap_or(0.0);
                if let Some(r) = rest2 {
                    let biomes_part = r
                        .strip_prefix("biomes_")
                        .or_else(|| r.strip_prefix("biomes="))
                        .unwrap_or(r);
                    // Stop at next known key if present on same line.
                    let biomes_part = biomes_part
                        .split("heatValue=")
                        .next()
                        .unwrap_or(biomes_part)
                        .trim_end_matches(',')
                        .trim();
                    def.biomes = biomes_part
                        .split(|c| c == ',' || c == ' ')
                        .filter_map(|s| {
                            let s = s.trim();
                            if s.is_empty() {
                                None
                            } else {
                                s.parse().ok()
                            }
                        })
                        .collect();
                }
            }
        }
    }

    Ok(def)
}

fn description_to_name(desc: &str) -> String {
    // OHOL: "Wild Gooseberry# just picked"
    let base = desc.split('#').next().unwrap_or(desc).trim();
    base.to_string()
}

/// Try default content locations relative to cwd / common sibling path.
pub fn resolve_content_path(configured: &Path) -> PathBuf {
    if configured.exists() {
        return configured.to_path_buf();
    }
    let candidates = [
        PathBuf::from("content/OneLifeData7"),
        PathBuf::from("../OpenLife/OneLifeData7"),
        PathBuf::from(r"C:\OhOl\OpenLife\OneLifeData7"),
    ];
    for c in candidates {
        if c.exists() {
            return c;
        }
    }
    configured.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_minimal_object() {
        let dir = std::env::temp_dir().join("ol_content_test_obj");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("33.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "33").unwrap();
        writeln!(f, "Gooseberry# wild").unwrap();
        writeln!(f, "foodValue=3").unwrap();
        writeln!(f, "containable=1").unwrap();
        let def = load_object_file(&path).unwrap();
        assert_eq!(def.id, 33);
        assert_eq!(def.name, "Gooseberry");
        assert_eq!(def.food_value, 3);
        assert!(def.containable);
        assert!(!def.floor);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_floor_flag() {
        let dir = std::env::temp_dir().join("ol_content_test_floor");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("1596.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "id=1596").unwrap();
        writeln!(f, "Stone Road# groundOnly").unwrap();
        writeln!(f, "floor=1").unwrap();
        writeln!(f, "permanent=0").unwrap();
        let def = load_object_file(&path).unwrap();
        assert_eq!(def.id, 1596);
        assert!(def.floor);
        assert!(def.is_floor());
        assert_eq!(def.name, "Stone Road");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_openlife_id_prefix_and_map_chance() {
        let dir = std::env::temp_dir().join("ol_content_test_obj_id");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("100.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "id=100").unwrap();
        writeln!(f, "White Pine Tree with Needles").unwrap();
        writeln!(f, "containable=0").unwrap();
        writeln!(f, "permanent=1,minPickupAge=3").unwrap();
        writeln!(f, "blocksWalking=1,leftBlockingRadius=0").unwrap();
        writeln!(f, "mapChance=1.000000#biomes_0,3").unwrap();
        writeln!(f, "numUses=5,1.000000").unwrap();
        writeln!(f, "numSlots=0#timeStretch=1.000000").unwrap();
        let def = load_object_file(&path).unwrap();
        assert_eq!(def.id, 100);
        assert_eq!(def.name, "White Pine Tree with Needles");
        assert!(def.permanent);
        assert!(def.blocks_walking);
        assert!((def.map_chance - 1.0).abs() < 1e-5);
        assert_eq!(def.biomes, vec![0, 3]);
        assert_eq!(def.num_uses, 5);
        assert_eq!(def.num_slots, 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn name_from_description() {
        assert_eq!(description_to_name("Stone Hoe# tool"), "Stone Hoe");
    }

    #[test]
    fn parse_transition_file() {
        let dir = std::env::temp_dir().join("ol_content_test_tr");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("0_33.txt");
        let mut f = fs::File::create(&path).unwrap();
        // bare hand on object 33 → newActor 34 newTarget 32 ...
        writeln!(f, "34 32 0 0.000000 0.000000 0 0 0 0 0 0").unwrap();
        let t = load_transition_file(&path).unwrap();
        assert_eq!(t.actor_id, 0);
        assert_eq!(t.target_id, 33);
        assert_eq!(t.new_actor_id, 34);
        assert_eq!(t.new_target_id, 32);
        assert!(!t.last_use_target);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_last_use_transition_filename() {
        let dir = std::env::temp_dir().join("ol_content_test_lt");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("0_109_LT.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "0 0 0 0 0 0 0 0 0 0 0").unwrap();
        let t = load_transition_file(&path).unwrap();
        assert!(t.last_use_target);
        assert!(!t.last_use_actor);
        let _ = fs::remove_dir_all(&dir);
    }

    /// Goldens from real OneLifeData7 transition files (Haxe TransitionImporter shape).
    /// Skips if neither local content junction nor OhOl data tree is present.
    #[test]
    fn category_expands_shallow_digger_to_sharp_stone() {
        // Requires full content tree (skip if absent).
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../content/OneLifeData7");
        if !root.is_dir() {
            return;
        }
        let db = load_content(&root).expect("load content");
        // Category 722 (@ Shallow Digger) contains 34 Sharp Stone.
        // Transition 722+36 → 722+39 expands to 34+36 → 34+39.
        assert!(
            db.find_transition(34, 36).is_some(),
            "sharp stone on seeding wild carrot must resolve via category 722"
        );
        let t = db.find_transition(34, 36).unwrap();
        assert_eq!(t.new_target_id, 39, "dug wild carrot");
        // Dummy ids allocated for multi-use objects like stone pile 661.
        let pile = db.get(661).expect("stone pile");
        assert!(pile.num_uses >= 2);
        assert_eq!(pile.dummy_ids.len(), (pile.num_uses - 1) as usize);
        assert_eq!(db.wire_id_for_uses(661, pile.num_uses), 661);
        assert_ne!(db.wire_id_for_uses(661, 1), 661);
        assert_eq!(
            db.resolve_base_id(db.wire_id_for_uses(661, 1)),
            661
        );
    }

    #[test]
    fn real_data_transition_goldens() {
        let roots = [
            PathBuf::from("content/OneLifeData7"),
            PathBuf::from(r"C:\OhOl\OpenLife\OneLifeData7"),
            PathBuf::from(r"C:\OhOl\OpenLifeReborn\content\OneLifeData7"),
        ];
        let root = roots.into_iter().find(|p| p.join("transitions").is_dir());
        let Some(root) = root else {
            eprintln!("skip real_data_transition_goldens — no OneLifeData7");
            return;
        };
        let cases = [
            ("0_63.txt", 0, 63, 64, 48),
            ("0_242.txt", 0, 242, 223, 242),
            ("0_36.txt", 0, 36, 395, 404),
        ];
        for (file, actor, target, new_a, new_t) in cases {
            let path = root.join("transitions").join(file);
            assert!(path.is_file(), "missing {path:?}");
            let tr = load_transition_file(&path).expect(file);
            assert_eq!(tr.actor_id, actor, "{file} actor");
            assert_eq!(tr.target_id, target, "{file} target");
            assert_eq!(tr.new_actor_id, new_a, "{file} new_actor");
            assert_eq!(tr.new_target_id, new_t, "{file} new_target");
            assert!(!tr.last_use_actor && !tr.last_use_target, "{file} not last-use");
        }
        // Full load: find_transition must match goldens (Haxe lookup path).
        let db = load_content(&root).expect("load_content");
        assert!(db.object_count() > 100);
        assert!(db.transition_count > 100);
        for &(_, a, t, na, nt) in &cases {
            let tr = db
                .find_transition(a, t)
                .unwrap_or_else(|| panic!("missing transition {a}+{t}"));
            assert_eq!(tr.new_actor_id, na);
            assert_eq!(tr.new_target_id, nt);
        }
        // Timing fields populated on load.
        assert!(db.load_objects_ms > 0 || db.load_total_ms > 0);
        assert!(db.load_transitions_ms > 0 || db.transition_count == 0);
    }

    #[test]
    fn fixture_transition_matches_haxe_filename_parse() {
        // Mirrors Haxe: stem actor_target, line "newActor newTarget …"
        let dir = std::env::temp_dir().join("ol_content_golden_0_63");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("0_63.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "64 48 0").unwrap();
        let t = load_transition_file(&path).unwrap();
        assert_eq!((t.actor_id, t.target_id, t.new_actor_id, t.new_target_id), (0, 63, 64, 48));
        let _ = fs::remove_dir_all(&dir);
    }
}
