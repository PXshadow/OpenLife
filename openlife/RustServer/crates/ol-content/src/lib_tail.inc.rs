/// Importer helpers (category expand, changeToolTransitions, animal moves).
/// Haxe `PatchObjectData` / `PatchTransitions` live in [`crate::patches`].
///
/// Haxe `createAndaddCategoryTransitions` — expand actor/target category parents
/// into concrete member transitions (e.g. `@ Shallow Digger` 722 → sharp stone 34).
pub(crate) fn expand_category_transitions(db: &mut ContentDb) {
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

fn target_remains(t: &Transition) -> bool {
    t.target_id >= 0 && t.target_id == t.new_target_id
}

/// Insert a last-use row; Haxe double-transition (`targetRemains` true + false)
/// sends the non-remains row to `maxUseTransitions` (well site 33+1096 → 3963).
// Haxe: TransitionImporter.addTransition ~479–496
fn insert_last_use_or_max_use(db: &mut ContentDb, t: Transition) -> bool {
    let key = (t.actor_id, t.target_id);
    let remains = target_remains(&t);
    if let Some(existing) = db.transitions_last_use.get(&key).cloned() {
        let exist_remains = target_remains(&existing);
        if exist_remains && !remains {
            db.transitions_max_use.insert(key, t);
            return true;
        }
        if !exist_remains && remains {
            db.transitions_max_use.insert(key, existing);
            db.transitions_last_use.insert(key, t);
            return true;
        }
        return false;
    }
    db.transitions_last_use.insert(key, t);
    true
}

/// Haxe `addTransition`: reverse-use rows are also registered as last-use so
/// `isLastUse()` (uses ≤ 1) still *adds* to a pile instead of firing `*_LT.txt`.
// Haxe: TransitionImporter.addTransition ~443–448
fn haxe_clone_reverse_into_last_use(db: &mut ContentDb, t: &Transition) {
    if t.last_use_actor || t.last_use_target {
        return;
    }
    if t.reverse_use_actor && t.reverse_use_target {
        let mut c = t.clone();
        c.last_use_actor = true;
        c.last_use_target = true;
        insert_last_use_or_max_use(db, c);
    } else if t.reverse_use_actor {
        let mut c = t.clone();
        c.last_use_actor = true;
        insert_last_use_or_max_use(db, c);
    } else if t.reverse_use_target {
        let mut c = t.clone();
        c.last_use_target = true;
        insert_last_use_or_max_use(db, c);
    }
}

/// Repair last-use vs max-use after OLT1 cache / category expand.
///
/// Well site: `33_1096.txt` reverse remains (add stone) + `33_1096_LT.txt`
/// `0+3963` (complete). Haxe puts 3963 in **maxUse** and keeps add-stone as
/// last-use. A raw LT insert left 3963 in last-use, so uses=1 completed the
/// site and uses=max refused (no max-use row).
// Haxe: TransitionImporter.addTransition reverseUse clone + maxUseTransitions
pub fn apply_haxe_reverse_use_last_and_max(db: &mut ContentDb) {
    let keys: Vec<(i32, i32)> = db.transitions.keys().copied().collect();
    for key in keys {
        let Some(primary) = db.transitions.get(&key).cloned() else {
            continue;
        };
        if !primary.reverse_use_target {
            continue;
        }
        let primary_remains = target_remains(&primary);
        if primary_remains {
            if let Some(lu) = db.transitions_last_use.get(&key).cloned() {
                if !target_remains(&lu) {
                    db.transitions_last_use.remove(&key);
                    db.transitions_max_use.entry(key).or_insert(lu);
                }
            }
        }
        if !db.transitions_last_use.contains_key(&key) {
            let mut clone = primary;
            clone.last_use_target = true;
            db.transitions_last_use.insert(key, clone);
        }
    }
}

/// Set `ObjectData.moves` from auto-decay / time-move transitions (`move_dist > 0`).
///
/// Haxe sets `animal.objectData.moves` during `doAnimalMovement`; stamping from
/// content lets `isAnimal()` work for USE bow min-range without a prior move tick.
// Haxe: TimeHelper.doAnimalMovement animal.objectData.moves = moveDist
pub fn apply_animal_moves_from_transitions(db: &mut ContentDb) {
    for tr in db.auto_decays.values() {
        if tr.move_dist <= 0 {
            continue;
        }
        if let Some(d) = db.objects.get_mut(&tr.target_id) {
            if d.moves < tr.move_dist {
                d.moves = tr.move_dist;
            }
        }
    }
    for tr in db.transitions.values() {
        if tr.move_dist <= 0 || tr.target_id <= 0 {
            continue;
        }
        if let Some(d) = db.objects.get_mut(&tr.target_id) {
            if d.moves < tr.move_dist {
                d.moves = tr.move_dist;
            }
        }
    }
}
pub fn change_tool_transitions(db: &mut ContentDb) -> usize {
    let mut rewritten = 0usize;

    // Snapshot keys; tool rows themselves have target_id < 1 so they are not rewritten.
    let normal_keys: Vec<(i32, i32)> = db.transitions.keys().copied().collect();
    for key in normal_keys {
        if rewrite_one_tool_transition(db, key, false) {
            rewritten += 1;
        }
    }
    let last_keys: Vec<(i32, i32)> = db.transitions_last_use.keys().copied().collect();
    for key in last_keys {
        if rewrite_one_tool_transition(db, key, true) {
            rewritten += 1;
        }
    }
    if rewritten > 0 {
        info!(rewritten, "content changeToolTransitions rewrote new_actor_id");
    }
    rewritten
}

/// Apply Haxe changeToolTransitions filters + rewrite for one map entry.
fn rewrite_one_tool_transition(db: &mut ContentDb, key: (i32, i32), in_last_use: bool) -> bool {
    let (actor_id, target_id, new_actor_id) = {
        let t = if in_last_use {
            db.transitions_last_use.get(&key)
        } else {
            db.transitions.get(&key)
        };
        let Some(t) = t else {
            return false;
        };
        // Haxe: actorID != newActorID → skip (EMPTY+Cold Bowl)
        if t.actor_id != t.new_actor_id {
            return false;
        }
        // Haxe: targetID < 1 → skip
        if t.target_id < 1 {
            return false;
        }
        // Haxe: actor numUses > 1 → skip multi-use tools
        let num_uses = db.objects.get(&t.actor_id).map(|d| d.num_uses).unwrap_or(0);
        if num_uses > 1 {
            return false;
        }
        // Haxe TODO: Rubber Ball 2170 special skip
        if t.actor_id == 2170 {
            return false;
        }
        // Haxe: newActorID == 0 → skip
        if t.new_actor_id == 0 {
            return false;
        }
        (t.actor_id, t.target_id, t.new_actor_id)
    };
    let _ = (actor_id, target_id); // filters already applied; keep names for Haxe anchors

    // Haxe: GetTransition(newActorID, -1, lastUseActor=true) then non-LA
    let tool_new = db
        .find_transition_last_use(new_actor_id, -1)
        .or_else(|| db.find_transition(new_actor_id, -1))
        .map(|tr| tr.new_actor_id);

    let Some(tool_new_actor) = tool_new else {
        return false;
    };
    if tool_new_actor == new_actor_id {
        return false;
    }

    if in_last_use {
        if let Some(t) = db.transitions_last_use.get_mut(&key) {
            t.new_actor_id = tool_new_actor;
            return true;
        }
    } else if let Some(t) = db.transitions.get_mut(&key) {
        t.new_actor_id = tool_new_actor;
        return true;
    }
    false
}
/// Insert non-last-use transition; Haxe double-transition maxUse handling.
fn insert_normal_or_max_use(db: &mut ContentDb, t: Transition) -> bool {
    let key = (t.actor_id, t.target_id);
    let remains = target_remains(&t);
    if let Some(existing) = db.transitions.get(&key).cloned() {
        let exist_remains = target_remains(&existing);
        // Haxe: targetRemains true + false pair → non-remains goes to maxUse table
        if exist_remains && !remains {
            db.transitions_max_use.insert(key, t);
            return true;
        }
        if !exist_remains && remains {
            db.transitions_max_use.insert(key, existing);
            db.transitions.insert(key, t);
            return true;
        }
        // Same kind: keep first (category expansion / duplicates)
        return false;
    }
    db.transitions.insert(key, t);
    true
}

fn insert_expanded(db: &mut ContentDb, t: Transition) -> bool {
    if t.last_use_actor || t.last_use_target {
        insert_last_use_or_max_use(db, t)
    } else {
        insert_normal_or_max_use(db, t)
    }
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
                // Haxe: negative autoDecaySeconds = hours; must still index for map timers.
                if t.actor_id < 0 && t.auto_decay_seconds != 0.0 {
                    db.auto_decays.insert(t.target_id, t.clone());
                }
                // Also index pure animal-move transitions (autoDecaySeconds may be 0).
                if t.actor_id < 0 && t.move_dist > 0 {
                    db.auto_decays
                        .entry(t.target_id)
                        .or_insert_with(|| t.clone());
                }
                if t.last_use_actor || t.last_use_target {
                    if insert_last_use_or_max_use(db, t) {
                        loaded_last_use += 1;
                    }
                } else {
                    let for_clone = t.clone();
                    if insert_normal_or_max_use(db, t) {
                        haxe_clone_reverse_into_last_use(db, &for_clone);
                        loaded += 1;
                    }
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
        actor_min_use_fraction: parse_f(3),
        target_min_use_fraction: parse_f(4),
        switch_number_of_uses: false,
        target_number_of_uses: -1,
        is_pickup_or_drop: false,
        hungry_work_cost: 0.0,
        hungry_work_temperature: -1.0,
        coin_cost: 0,
        is_forbidden: false,
    })
}

/// Parse object file + person race (0 when absent).
/// Also fills [`ObjectDef::male`] via key parse in [`load_object_file`].
// TWIN-PARTY-RESID: ObjectData.male + person race
pub fn load_object_file_full(path: &Path) -> Result<ParsedObject, ContentError> {
    let text = fs::read_to_string(path)?;
    let def = load_object_file(path)?;
    let person = parse_person_from_text(&text);
    Ok(ParsedObject { def, person })
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
            } else if let Some(rest) = part.strip_prefix("minPickupAge=") {
                // Haxe: ObjectData.minPickupAge (same comma group as permanent=)
                def.min_pickup_age = rest.trim().parse().unwrap_or(0);
            } else if let Some(rest) = part.strip_prefix("blocksWalking=") {
                def.blocks_walking = rest.starts_with('1') || rest.eq_ignore_ascii_case("true");
            } else if let Some(rest) = part.strip_prefix("foodValue=") {
                def.food_value = rest.parse().unwrap_or(0);
            } else if let Some(rest) = part.strip_prefix("heatValue=") {
                def.heat_value = rest.parse().unwrap_or(0.0);
            } else if let Some(rest) = part.strip_prefix("numUses=") {
                // Haxe: numUses = int(array[0]); useChance = array[1] if present.
                // Outer loop splits on ','; re-read full line when `numUses=N,chance`.
                let full = if line.contains("numUses=") {
                    line.split("numUses=").nth(1).unwrap_or(rest)
                } else {
                    rest
                };
                let full = full.split('#').next().unwrap_or(full);
                let mut it = full.split(',');
                if let Some(num) = it.next() {
                    def.num_uses = num.trim().parse().unwrap_or(0);
                }
                if let Some(chance) = it.next() {
                    def.use_chance = chance.trim().parse().unwrap_or(0.0);
                }
            } else if let Some(rest) = part.strip_prefix("numSlots=") {
                // numSlots=4#timeStretch=1.000000
                let num = rest.split(|c| c == ',' || c == '#').next().unwrap_or(rest);
                def.num_slots = num.parse().unwrap_or(0);
            } else if let Some(rest) = part.strip_prefix("containSize=") {
                // Haxe: containSize=N,vertSlotRot=… — first comma/hash-separated float.
                // // Haxe: ObjectData.containSize
                let num = rest.split(|c| c == ',' || c == '#').next().unwrap_or(rest);
                def.contain_size = num.parse().unwrap_or(0.0);
            } else if let Some(rest) = part.strip_prefix("slotsSize=") {
                // Haxe text key `slotsSize` → ObjectData.slotSize (default 1).
                // // Haxe: ObjectData.slotSize
                let num = rest.split(|c| c == ',' || c == '#').next().unwrap_or(rest);
                def.slot_size = num.parse().unwrap_or(1.0);
            } else if let Some(rest) = part.strip_prefix("slotSize=") {
                // Alternate key (some exporters); same as slotsSize.
                let num = rest.split(|c| c == ',' || c == '#').next().unwrap_or(rest);
                def.slot_size = num.parse().unwrap_or(1.0);
            } else if let Some(rest) = part.strip_prefix("male=") {
                // Haxe ObjectData.male — person sex (0/1 or true/false).
                // TWIN-PARTY-RESID / ObjectData.male
                def.male = rest.starts_with('1') || rest.eq_ignore_ascii_case("true");
            } else if let Some(rest) = part.strip_prefix("floor=") {
                // floor=1 — floor-only objects (roads, stone floors); not ground placeables.
                def.floor = rest.starts_with('1') || rest.eq_ignore_ascii_case("true");
            } else if let Some(rest) = part.strip_prefix("speedMult=") {
                def.speed_mult = rest
                    .split(|c| c == ',' || c == '#')
                    .next()
                    .unwrap_or(rest)
                    .parse()
                    .unwrap_or(1.0);
            } else if let Some(rest) = part.strip_prefix("rValue=") {
                // Haxe ObjectData.rValue — insulation / isWall gate.
                def.r_value = rest
                    .split(|c| c == ',' || c == '#')
                    .next()
                    .unwrap_or(rest)
                    .parse()
                    .unwrap_or(0.0);
            } else if let Some(rest) = part.strip_prefix("clothing=") {
                // clothing=n or clothing=h etc. (may share line with clothingOffset — split earlier).
                def.clothing = rest
                    .split(|c| c == ',' || c == '#')
                    .next()
                    .unwrap_or(rest)
                    .trim()
                    .to_string();
                if def.clothing.is_empty() {
                    def.clothing = "n".into();
                }
            } else if let Some(rest) = part.strip_prefix("useDistance=") {
                // Haxe ObjectData.useDistance (default 1).
                def.use_distance = rest
                    .split(|c| c == ',' || c == '#')
                    .next()
                    .unwrap_or(rest)
                    .parse()
                    .unwrap_or(1);
            } else if let Some(rest) = part.strip_prefix("deadlyDistance=") {
                // Haxe ObjectData.deadlyDistance (float; files often store int).
                def.deadly_distance = rest
                    .split(|c| c == ',' || c == '#')
                    .next()
                    .unwrap_or(rest)
                    .parse()
                    .unwrap_or(0.0);
            } else if let Some(rest) = part.strip_prefix("moves=") {
                // Haxe ObjectData.moves (rare in files; usually set from time-move).
                def.moves = rest
                    .split(|c| c == ',' || c == '#')
                    .next()
                    .unwrap_or(rest)
                    .parse()
                    .unwrap_or(0);
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
        assert_eq!(def.use_distance, 1);
        assert_eq!(def.deadly_distance, 0.0);
        assert!((def.contain_size - 0.0).abs() < 1e-5);
        assert!((def.slot_size - 1.0).abs() < 1e-5);
        let _ = fs::remove_dir_all(&dir);
    }

    /// CLOTHING-CONTAIN-SIZE: containSize + slotsSize from object text.
    // Haxe: ObjectData.containSize / slotSize (text key slotsSize)
    #[test]
    fn parse_contain_size_and_slot_size() {
        let dir = std::env::temp_dir().join("ol_content_test_contain_size");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("333.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "id=333").unwrap();
        writeln!(f, "Basket# container").unwrap();
        writeln!(f, "containable=1").unwrap();
        writeln!(f, "containSize=2.000000,vertSlotRot=-0.250000").unwrap();
        writeln!(f, "numSlots=4#timeStretch=1.000000").unwrap();
        writeln!(f, "slotsSize=1").unwrap();
        let def = load_object_file(&path).unwrap();
        assert!((def.contain_size - 2.0).abs() < 1e-5);
        assert!((def.slot_size - 1.0).abs() < 1e-5);
        assert_eq!(def.num_slots, 4);
        // Fit gate: basket containSize 2 needs container slotSize >= 2
        let mut pocket = ObjectDef::empty(198);
        pocket.slot_size = 1.0;
        assert!(!def.contain_fits_in_container(&pocket));
        pocket.slot_size = 2.0;
        assert!(def.contain_fits_in_container(&pocket));
        let _ = fs::remove_dir_all(&dir);
    }

    /// CLOTHING-CONTAIN-SIZE: ServerSettings.PatchObjectData containSize force table.
    // Haxe: ServerSettings.PatchObjectData L633–758
    #[test]
    fn apply_default_contain_size_patches_ids_and_description() {
        let mut db = ContentDb::default();
        // Id force-patches (create stubs).
        for id in [0, 356, 302, 319, 2574] {
            db.objects.insert(id, ObjectDef::empty(id));
        }
        // Description rules
        let mut flat = ObjectDef::empty(9001);
        flat.description = "Something on Flat Rock".into();
        db.objects.insert(9001, flat);
        let mut mech = ObjectDef::empty(9002);
        mech.description = "Clockwork Mechanism".into();
        db.objects.insert(9002, mech);
        let mut cruc_wood = ObjectDef::empty(9003);
        cruc_wood.description = "Crucible in Wooden Tongs".into();
        db.objects.insert(9003, cruc_wood);
        let mut cruc = ObjectDef::empty(9004);
        cruc.description = "Steel Crucible".into();
        db.objects.insert(9004, cruc);
        let mut shears = ObjectDef::empty(9005);
        shears.description = "Steel Shears".into();
        shears.permanent = true;
        db.objects.insert(9005, shears);

        apply_default_contain_size_patches(&mut db);

        assert!((db.objects.get(&0).unwrap().contain_size - 1.0).abs() < 1e-5);
        assert!(db.objects.get(&0).unwrap().containable);
        assert!((db.objects.get(&356).unwrap().contain_size - 2.0).abs() < 1e-5);
        assert!(db.objects.get(&356).unwrap().containable);
        assert!((db.objects.get(&302).unwrap().contain_size - 1.0).abs() < 1e-5);
        assert!((db.objects.get(&319).unwrap().contain_size - 2.0).abs() < 1e-5);
        assert!((db.objects.get(&2574).unwrap().contain_size - 2.0).abs() < 1e-5);

        assert!((db.objects.get(&9001).unwrap().contain_size - 2.0).abs() < 1e-5);
        assert!(db.objects.get(&9001).unwrap().containable);
        assert!((db.objects.get(&9002).unwrap().contain_size - 2.0).abs() < 1e-5);
        // "Crucible in Wooden" must NOT force containSize 2
        assert!((db.objects.get(&9003).unwrap().contain_size - 0.0).abs() < 1e-5);
        assert!(!db.objects.get(&9003).unwrap().containable);
        assert!((db.objects.get(&9004).unwrap().contain_size - 2.0).abs() < 1e-5);
        assert!(db.objects.get(&9004).unwrap().containable);
        // Shears: permanent cleared, contain 1
        assert!(!db.objects.get(&9005).unwrap().permanent);
        assert!((db.objects.get(&9005).unwrap().contain_size - 1.0).abs() < 1e-5);
        assert!(db.objects.get(&9005).unwrap().containable);
    }

    /// TWIN-PARTY-RESID: ObjectData.male from person object files.
    // Haxe: ObjectData.male
    #[test]
    fn parse_male_flag() {
        let dir = std::env::temp_dir().join("ol_content_test_male");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("19.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "id=19").unwrap();
        writeln!(f, "Female001 D").unwrap();
        writeln!(f, "person=4,noSpawn=0").unwrap();
        writeln!(f, "male=0").unwrap();
        let parsed = load_object_file_full(&path).unwrap();
        assert!(!parsed.def.male, "female person male=0");
        assert_eq!(parsed.person, 4);
        let path_m = dir.join("30.txt");
        let mut f = fs::File::create(&path_m).unwrap();
        writeln!(f, "id=30").unwrap();
        writeln!(f, "Male001 D").unwrap();
        writeln!(f, "person=4,noSpawn=0").unwrap();
        writeln!(f, "male=1").unwrap();
        let parsed_m = load_object_file_full(&path_m).unwrap();
        assert!(parsed_m.def.male, "male person male=1");
        let _ = fs::remove_dir_all(&dir);
    }

    // Haxe: ServerSettings.PatchObjectData L1774–1776 minPickupAge
    // MIN-PICKUP-AGE
    #[test]
    fn apply_min_pickup_age_patches_bow_knife() {
        let mut db = ContentDb::default();
        db.objects.insert(151, ObjectDef::empty(151));
        db.objects.insert(560, ObjectDef::empty(560));
        apply_default_min_pickup_age_patches(&mut db);
        assert_eq!(db.get(151).unwrap().min_pickup_age, 5, "Yew Bow last write");
        assert_eq!(db.get(560).unwrap().min_pickup_age, 2, "Knife");
    }

    // Haxe: ServerSettings.PatchObjectData damage / woundFactor
    #[test]
    fn apply_combat_damage_patches_knife_snake_wound() {
        let mut db = ContentDb::default();
        db.objects.insert(560, ObjectDef::empty(560));
        db.objects.insert(764, ObjectDef::empty(764));
        db.objects.insert(798, ObjectDef::empty(798));
        apply_default_combat_damage_patches(&mut db);
        let knife = db.get(560).unwrap();
        assert!((knife.damage - 5.0).abs() < 1e-5);
        assert!((knife.damage_protection_factor - 0.8).abs() < 1e-5);
        let snake = db.get(764).unwrap();
        assert!((snake.damage - 2.0).abs() < 1e-5);
        assert!((snake.wound_factor - 0.98).abs() < 1e-5);
        let arrow_w = db.get(798).unwrap();
        assert!((arrow_w.damage - 0.06).abs() < 1e-5);
    }

    // Haxe: ObjectData useDistance/deadlyDistance + PatchObjectData weapons
    #[test]
    fn parse_use_and_deadly_distance_and_weapon_patches() {
        let dir = std::env::temp_dir().join("ol_content_test_range");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("152.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "id=152").unwrap();
        writeln!(f, "Bow and Arrow").unwrap();
        writeln!(f, "deadlyDistance=3").unwrap();
        writeln!(f, "useDistance=5").unwrap();
        let def = load_object_file(&path).unwrap();
        assert_eq!(def.use_distance, 5);
        assert!((def.deadly_distance - 3.0).abs() < 1e-5);
        assert_eq!(def.effective_use_distance(), 5);

        let mut db = ContentDb::default();
        db.objects.insert(152, def);
        apply_default_weapon_range_patches(&mut db);
        let bow = db.get(152).unwrap();
        assert_eq!(bow.use_distance, 5);
        assert!((bow.deadly_distance - 4.0).abs() < 1e-5);

        // animal moves from auto_decays
        let mut wolf = ObjectDef::empty(418);
        db.objects.insert(418, wolf.clone());
        db.auto_decays.insert(
            418,
            Transition {
                actor_id: -1,
                target_id: 418,
                new_actor_id: 0,
                new_target_id: 418,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 3.0,
                reverse_use_actor: false,
                reverse_use_target: false,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 2,
                desired_move_dist: 4,
                actor_min_use_fraction: 0.0,
                target_min_use_fraction: 0.0,
                switch_number_of_uses: false,
                target_number_of_uses: -1,
                is_pickup_or_drop: false,
                hungry_work_cost: 0.0,
                hungry_work_temperature: -1.0,
                coin_cost: 0,
                is_forbidden: false,
            },
        );
        apply_animal_moves_from_transitions(&mut db);
        wolf = db.get(418).unwrap().clone();
        assert_eq!(wolf.moves, 2);
        assert!(wolf.is_animal());

        // Animal deadlyDistance factor (files often 1 → patch 0.5)
        let mut wolf_file = ObjectDef::empty(418);
        wolf_file.deadly_distance = 1.0;
        db.objects.insert(418, wolf_file);
        apply_default_animal_deadly_distance_patches(&mut db);
        let w = db.get(418).unwrap();
        assert!((w.deadly_distance - ANIMAL_DEADLY_DISTANCE_FACTOR).abs() < 1e-5);
        // Mosquito gets deadly patch but is_animal stays false (id 2156 exclusion).
        let mut moz = ObjectDef::empty(2156);
        moz.moves = 3;
        moz.deadly_distance = 1.0;
        db.objects.insert(2156, moz);
        apply_default_animal_deadly_distance_patches(&mut db);
        let m = db.get(2156).unwrap();
        assert!((m.deadly_distance - 0.5).abs() < 1e-5);
        assert!(!m.is_animal());
        let _ = fs::remove_dir_all(&dir);
    }

    // Haxe: ServerSettings.PatchObjectData 2156 mapChance*=0.3 + SWAMP biomes
    // MOSQUITO-MAPCHANCE
    #[test]
    fn mosquito_map_chance_swamp_rebuilds_biome_spawn() {
        let mut db = ContentDb::default();
        let mut moz = ObjectDef::empty(MOSQUITO_SWARM_OBJECT_ID);
        moz.map_chance = 1.0;
        moz.biomes = vec![6]; // JUNGLE only before patch
        db.objects.insert(MOSQUITO_SWARM_OBJECT_ID, moz);
        // Seed a decoy entry so rebuild must replace, not append.
        db.biome_spawn.insert(
            6,
            BiomeSpawnTable {
                total_chance: 1.0,
                entries: vec![(MOSQUITO_SWARM_OBJECT_ID, 1.0)],
            },
        );
        apply_default_mosquito_map_chance_patches(&mut db);
        let m = db.get(MOSQUITO_SWARM_OBJECT_ID).unwrap();
        assert!((m.map_chance - MOSQUITO_MAP_CHANCE_FACTOR).abs() < 1e-5);
        assert!(m.biomes.contains(&BIOME_TAG_SWAMP));
        assert!(m.biomes.contains(&6));
        let jungle = db.biome_spawn.get(&6).expect("jungle spawn");
        assert!((jungle.total_chance - MOSQUITO_MAP_CHANCE_FACTOR).abs() < 1e-5);
        assert_eq!(jungle.entries, vec![(MOSQUITO_SWARM_OBJECT_ID, MOSQUITO_MAP_CHANCE_FACTOR)]);
        let swamp = db.biome_spawn.get(&BIOME_TAG_SWAMP).expect("swamp spawn");
        assert!((swamp.total_chance - MOSQUITO_MAP_CHANCE_FACTOR).abs() < 1e-5);
        assert_eq!(
            swamp.entries,
            vec![(MOSQUITO_SWARM_OBJECT_ID, MOSQUITO_MAP_CHANCE_FACTOR)]
        );
        // Idempotent: second apply must not re-scale mapChance or dup SWAMP.
        apply_default_mosquito_map_chance_patches(&mut db);
        let m2 = db.get(MOSQUITO_SWARM_OBJECT_ID).unwrap();
        assert_eq!(m2.biomes.iter().filter(|&&b| b == BIOME_TAG_SWAMP).count(), 1);
        assert!((m2.map_chance - MOSQUITO_MAP_CHANCE_FACTOR).abs() < 1e-5);
    }

    // Haxe: ObjectData.useDistance clamp + isAnimal mosquito exclusion
    #[test]
    fn effective_use_distance_and_is_animal() {
        let mut d = ObjectDef::empty(1);
        d.use_distance = 0;
        assert_eq!(d.effective_use_distance(), 1);
        d.use_distance = -3;
        assert_eq!(d.effective_use_distance(), 1);
        d.use_distance = 5;
        assert_eq!(d.effective_use_distance(), 5);

        d.moves = 2;
        assert!(d.is_animal());
        let mut moz = ObjectDef::empty(2156);
        moz.moves = 5;
        assert!(!moz.is_animal());
        let mut rock = ObjectDef::empty(33);
        rock.moves = 0;
        assert!(!rock.is_animal());
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
        assert_eq!(def.min_pickup_age, 3);
        assert!(def.blocks_walking);
        assert!((def.map_chance - 1.0).abs() < 1e-5);
        assert_eq!(def.biomes, vec![0, 3]);
        assert_eq!(def.num_uses, 5);
        assert!((def.use_chance - 1.0).abs() < 1e-5);
        assert_eq!(def.num_slots, 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn name_from_description() {
        assert_eq!(description_to_name("Stone Hoe# tool"), "Stone Hoe");
    }

    #[test]
    fn parse_rvalue_and_clothing() {
        let dir = std::env::temp_dir().join("ol_content_test_rvalue");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("885.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "id=885").unwrap();
        writeln!(f, "Stone Wall# +cornerStone").unwrap();
        writeln!(f, "permanent=1").unwrap();
        writeln!(f, "rValue=0.900000").unwrap();
        writeln!(f, "floor=0").unwrap();
        writeln!(f, "clothing=n").unwrap();
        let def = load_object_file(&path).unwrap();
        assert!((def.r_value - 0.9).abs() < 1e-5);
        assert_eq!(def.clothing, "n");
        assert!(def.is_wall());
        assert!(!def.is_clothing());
        assert!((def.insulation_for_protection() - 0.9).abs() < 1e-5);
        assert!((def.get_insulation() - 0.9).abs() < 1e-5);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn clothing_get_insulation_and_heat_protection() {
        // Haxe: ObjectData.parts h/t/b/p=0.4 s=0.2; getInsulation / getHeatProtection
        let mut hat = ObjectDef::empty(1);
        hat.clothing = "h".into();
        hat.r_value = 1.0;
        assert!((hat.get_insulation() - 0.4).abs() < 1e-5);
        assert!((hat.get_heat_protection() - 0.0).abs() < 1e-5);

        let mut tunic = ObjectDef::empty(2);
        tunic.clothing = "t".into();
        tunic.r_value = 0.5;
        assert!((tunic.get_insulation() - 0.2).abs() < 1e-5);
        assert!((tunic.get_heat_protection() - 0.2).abs() < 1e-5);

        let mut shoes = ObjectDef::empty(3);
        shoes.clothing = "s".into();
        shoes.r_value = 0.0;
        assert!((shoes.get_insulation() - 0.2).abs() < 1e-5);
        assert!((shoes.get_heat_protection() - 0.2).abs() < 1e-5);

        let mut pack = ObjectDef::empty(4);
        pack.clothing = "p".into();
        pack.r_value = 0.5;
        assert!((pack.get_insulation() - 0.2).abs() < 1e-5);
        assert_eq!(pack.get_heat_protection(), 0.0);

        let mut padded = ObjectDef::empty(5);
        padded.clothing = "  h  ".into();
        padded.r_value = 1.0;
        assert!((padded.get_insulation() - 0.4).abs() < 1e-5);
        assert_eq!(clothing_part_weight("hat"), None);
    }

    #[test]
    fn clothing_get_prestige_factor_slot_weight() {
        let mut hat = ObjectDef::empty(1);
        hat.clothing = "h".into();
        assert!((hat.get_prestige_factor() - 0.2).abs() < 1e-5);
        hat.prestige_factor = 1.5;
        assert!((hat.get_prestige_factor() - 0.6).abs() < 1e-5);
        let none = ObjectDef::empty(2);
        assert_eq!(none.get_prestige_factor(), 0.0);
        let mut shoes = ObjectDef::empty(3);
        shoes.clothing = "s".into();
        assert!((shoes.get_prestige_factor() - 0.1).abs() < 1e-5);
    }

    #[test]
    fn decay_patches_apply_to_existing_objects() {
        let mut db = ContentDb::default();
        db.objects.insert(885, ObjectDef::empty(885));
        db.objects.insert(1596, ObjectDef {
            floor: true,
            ..ObjectDef::empty(1596)
        });
        db.objects.insert(1458, ObjectDef::empty(1458));
        apply_default_decay_object_patches(&mut db);
        let wall = db.get(885).unwrap();
        assert_eq!(wall.decays_to_obj, 1853);
        assert!((wall.decay_factor - 0.2).abs() < 1e-5);
        let road = db.get(1596).unwrap();
        assert_eq!(road.decays_to_obj, 291);
        let cow = db.get(1458).unwrap();
        assert_eq!(cow.decays_to_obj, 1900);
        assert!((cow.decay_factor - 0.05).abs() < 1e-5);
        assert!(is_animal_decay_factor_id(1458));
        assert!(is_animal_decay_factor_id(418));
        assert!(!is_animal_decay_factor_id(885));
        apply_animal_decay_factor_patches(&mut db, 0.2);
        let cow = db.get(1458).unwrap();
        assert!((cow.decay_factor - 0.2).abs() < 1e-5);
        assert_eq!(cow.decays_to_obj, 1900);
    }

    fn bare_tr(a: i32, t: i32, na: i32, nt: i32) -> Transition {
        Transition {
            actor_id: a,
            target_id: t,
            new_actor_id: na,
            new_target_id: nt,
            last_use_actor: false,
            last_use_target: false,
            auto_decay_seconds: 0.0,
            reverse_use_actor: false,
            reverse_use_target: false,
            no_use_actor: false,
            no_use_target: false,
            move_dist: 0,
            desired_move_dist: 0,
            actor_min_use_fraction: 0.0,
            target_min_use_fraction: 0.0,
            switch_number_of_uses: false,
            target_number_of_uses: -1,
            is_pickup_or_drop: false,
            hungry_work_cost: 0.0,
            hungry_work_temperature: -1.0,
            coin_cost: 0,
            is_forbidden: false,
        }
    }

    /// Haxe changeToolTransitions: 235+well same-actor fill rewrites via 235+-1 → 382;
    /// EMPTY+Cold Bowl 0+1021 is skipped (actorID != newActorID).
    // Haxe: TransitionImporter.changeToolTransitions
    #[test]
    fn change_tool_transitions_water_bowl_not_cold_bowl() {
        let mut db = ContentDb::default();
        db.objects.insert(
            235,
            ObjectDef {
                num_uses: 0,
                ..ObjectDef::empty(235)
            },
        );
        db.objects.insert(382, ObjectDef::empty(382));
        db.objects.insert(662, ObjectDef::empty(662));
        db.objects.insert(1021, ObjectDef::empty(1021));

        // File: 235 + 662 = 235 + 664 (same actor — empty bowl fill)
        db.transitions
            .insert((235, 662), bare_tr(235, 662, 235, 664));
        // Tool: 235 + -1 = 382 + 0
        db.transitions
            .insert((235, -1), bare_tr(235, -1, 382, 0));
        // EMPTY + Cold Bowl — must skip
        db.transitions
            .insert((0, 1021), bare_tr(0, 1021, 1022, 0));

        let n = change_tool_transitions(&mut db);
        assert!(n >= 1, "expected water-bowl style rewrite, got {n}");
        assert_eq!(
            db.find_transition(235, 662).unwrap().new_actor_id,
            382,
            "235+well newActor should rewrite via 235+-1 → 382"
        );
        assert_eq!(
            db.find_transition(0, 1021).unwrap().new_actor_id,
            1022,
            "EMPTY+Cold Bowl must not be rewritten"
        );
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

    #[test]
    fn prob_set_category_loads_weights() {
        let dir = std::env::temp_dir().join("ol_content_probset");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("3221.txt");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, "parentID=3221").unwrap();
        writeln!(f, "probSet").unwrap();
        writeln!(f, "numObjects=2").unwrap();
        writeln!(f, "1196 0.800000").unwrap();
        writeln!(f, "3220 0.200000").unwrap();
        let (cats, probs, _) = load_category_tables(&dir);
        assert!(cats.contains_key(&3221));
        let ps = probs.get(&3221).expect("prob set");
        assert_eq!(ps.ids, vec![1196, 3220]);
        assert!((ps.weights[0] - 0.8).abs() < 1e-5);
        assert!((ps.weights[1] - 0.2).abs() < 1e-5);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn well_site_lt_is_max_use_not_last_use() {
        // Haxe: 33+1096 reverse remains = add stone; 33+1096_LT 0+3963 = maxUse.
        // A last-use insert of 3963 made uses=1 complete the site.
        let mut db = ContentDb::default();
        let add = Transition {
            actor_id: 33,
            target_id: 1096,
            new_actor_id: 0,
            new_target_id: 1096,
            reverse_use_target: true,
            ..Default::default()
        };
        db.transitions.insert((33, 1096), add.clone());
        let complete = Transition {
            actor_id: 33,
            target_id: 1096,
            new_actor_id: 0,
            new_target_id: 3963,
            last_use_target: true,
            ..Default::default()
        };
        db.transitions_last_use.insert((33, 1096), complete);
        apply_haxe_reverse_use_last_and_max(&mut db);
        assert_eq!(
            db.find_transition_max_use(33, 1096).map(|t| t.new_target_id),
            Some(3963)
        );
        let lu = db.find_transition_last_use(33, 1096).unwrap();
        assert_eq!(lu.new_target_id, 1096, "last-use must still add a stone");
        assert!(lu.reverse_use_target);
        let _ = add;
    }
}
