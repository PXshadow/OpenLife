//! Category bank — C++ `categoryBank` + transition category expand.
//!
//! Loads `categories/*.txt` into forward + reverse maps and exposes
//! `getCategory` / `getReverseCategory` / `getNumCategoriesForObject` /
//! `getCategoryForObject`.
//!
//! Transition expansion (load-time, once):
//! 1. **Lite** — non-pattern, non-probability-set members (cartesian both-sides).
//! 2. **Pattern** — C++ second pass: same-length index pairing on actor/target/
//!    newActor/newTarget pattern parents; requires actor|target pattern.
//! Probability-set parents are **not** expanded at load; resolve outcomes at
//! lookup via [`CategoryBank::pick_from_prob_set`] (C++ `pickFromProbSet` /
//! server `transform_target`).
//!
//! **P4#27 editor mutators** (in-memory; optional [`format_category_txt`]):
//! add/remove/move members, pattern/probSet flags, member weights
//! (C++ `addCategoryToObject` / `removeObjectFromCategory` / `setMemberWeight` / …).
//! Disk `saveCategoryToDisk` is editor-only — not required to play.
//!
//! // C++: categoryBank.h / categoryBank.cpp
//! // C++: transitionBank.cpp `autoGenerateCategoryTransitions`
//! // Haxe: TransitionImporter.createAndaddCategoryTransitions

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::content::ClientTransition;

/// C++ `CategoryRecord`.
#[derive(Debug, Clone, Default)]
pub struct CategoryRecord {
    /// Object id of the parent (may be abstract `@…` gameplay object).
    pub parent_id: i32,
    /// Pattern category — index-paired expansion (C++ second pass).
    pub is_pattern: bool,
    /// Weighted probability set — runtime pick only (no load expand).
    pub is_probability_set: bool,
    /// Child object ids in this category.
    pub object_ids: Vec<i32>,
    /// Weights aligned with `object_ids` (prob sets; 0 for plain categories).
    pub object_weights: Vec<f32>,
}

/// C++ `ReverseCategoryRecord` — child → parent category ids.
#[derive(Debug, Clone, Default)]
pub struct ReverseCategoryRecord {
    pub child_id: i32,
    /// Parent category ids this child is a member of (order = file order).
    pub category_ids: Vec<i32>,
}

/// Sparse category tables (forward + reverse).
#[derive(Debug, Clone, Default)]
pub struct CategoryBank {
    /// parent_id → record (C++ `idMap`).
    pub by_parent: HashMap<i32, CategoryRecord>,
    /// child_id → reverse record (C++ `reverseMap`).
    pub reverse: HashMap<i32, ReverseCategoryRecord>,
}

impl CategoryBank {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.by_parent.is_empty()
    }

    pub fn len(&self) -> usize {
        self.by_parent.len()
    }

    /// Load all `*.txt` under `categories/` (C++ `initCategoryBank*`).
    pub fn load_from_dir(dir: impl AsRef<Path>) -> Self {
        let dir = dir.as_ref();
        let mut bank = Self::new();
        if !dir.is_dir() {
            return bank;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return bank;
        };
        for ent in entries.flatten() {
            let path = ent.path();
            if path.extension().and_then(|e| e.to_str()) != Some("txt") {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            if let Some(rec) = parse_category_txt(&text) {
                bank.insert_record(rec);
            }
        }
        bank
    }

    /// Insert one category and update reverse map (C++ `initCategoryBankFinish`).
    pub fn insert_record(&mut self, rec: CategoryRecord) {
        let parent = rec.parent_id;
        if parent <= 0 {
            return;
        }
        for &obj_id in &rec.object_ids {
            if obj_id <= 0 {
                continue;
            }
            let rr = self.reverse.entry(obj_id).or_insert_with(|| ReverseCategoryRecord {
                child_id: obj_id,
                category_ids: Vec::new(),
            });
            if !rr.category_ids.contains(&parent) {
                rr.category_ids.push(parent);
            }
        }
        self.by_parent.insert(parent, rec);
    }

    /// C++ `getCategory(inParentID)`.
    pub fn get_category(&self, parent_id: i32) -> Option<&CategoryRecord> {
        self.by_parent.get(&parent_id)
    }

    /// C++ `getReverseCategory(inChildID)`.
    pub fn get_reverse_category(&self, child_id: i32) -> Option<&ReverseCategoryRecord> {
        self.reverse.get(&child_id)
    }

    /// C++ `getNumCategoriesForObject`.
    pub fn get_num_categories_for_object(&self, object_id: i32) -> i32 {
        self.reverse
            .get(&object_id)
            .map(|rr| rr.category_ids.len() as i32)
            .unwrap_or(0)
    }

    /// C++ `getCategoryForObject` — parent id at index, or `-1`.
    pub fn get_category_for_object(&self, object_id: i32, category_index: i32) -> i32 {
        if category_index < 0 {
            return -1;
        }
        self.reverse
            .get(&object_id)
            .and_then(|rr| rr.category_ids.get(category_index as usize).copied())
            .unwrap_or(-1)
    }

    /// C++ `isProbabilitySet`.
    pub fn is_probability_set(&self, parent_id: i32) -> bool {
        self.by_parent
            .get(&parent_id)
            .map(|r| r.is_probability_set)
            .unwrap_or(false)
    }

    /// Pattern flag helper (C++ `CategoryRecord::isPattern`).
    pub fn is_pattern(&self, parent_id: i32) -> bool {
        self.by_parent
            .get(&parent_id)
            .map(|r| r.is_pattern)
            .unwrap_or(false)
    }

    /// Members eligible for **lite** expansion: non-pattern, non-prob-set only.
    pub fn expand_members(&self, parent_id: i32) -> Option<&[i32]> {
        let r = self.by_parent.get(&parent_id)?;
        if r.is_pattern || r.is_probability_set {
            return None;
        }
        if r.object_ids.is_empty() {
            return None;
        }
        Some(r.object_ids.as_slice())
    }

    /// Pattern members when `parent_id` is a pattern category (same length lists).
    pub fn pattern_members(&self, parent_id: i32) -> Option<&[i32]> {
        let r = self.by_parent.get(&parent_id)?;
        if !r.is_pattern || r.object_ids.is_empty() {
            return None;
        }
        Some(r.object_ids.as_slice())
    }

    /// C++ `pickFromProbSet` / server `transform_target`.
    ///
    /// When `parent_id` is a probSet category, pick a weighted member using
    /// `rand01` in `[0, 1]` multiplied by total weight (Haxe/server semantics).
    /// Non-probSet or empty → return `parent_id` unchanged.
    pub fn pick_from_prob_set(&self, parent_id: i32, rand01: f32) -> i32 {
        let Some(r) = self.by_parent.get(&parent_id) else {
            return parent_id;
        };
        if !r.is_probability_set || r.object_ids.is_empty() {
            return parent_id;
        }
        let total: f32 = if r.object_weights.len() == r.object_ids.len() {
            r.object_weights.iter().sum()
        } else {
            r.object_ids.len() as f32
        };
        if total <= 0.0 {
            return r.object_ids.first().copied().unwrap_or(parent_id);
        }
        let acc_roll = rand01.clamp(0.0, 1.0) * total;
        let mut acc = 0.0_f32;
        for (i, &id) in r.object_ids.iter().enumerate() {
            let w = r.object_weights.get(i).copied().unwrap_or(0.0);
            acc += w;
            if acc_roll <= acc {
                return id;
            }
        }
        r.object_ids.last().copied().unwrap_or(parent_id)
    }

    // ── P4#27 category editor mutators (C++ categoryBank.cpp ~463–960) ────────

    /// C++ `addCategory` — create empty category if missing.
    pub fn add_category(&mut self, parent_id: i32) {
        if parent_id <= 0 || self.by_parent.contains_key(&parent_id) {
            return;
        }
        self.by_parent.insert(
            parent_id,
            CategoryRecord {
                parent_id,
                is_pattern: false,
                is_probability_set: false,
                object_ids: Vec::new(),
                object_weights: Vec::new(),
            },
        );
    }

    /// C++ `deleteCategoryFromBank` — drop category and reverse links (in-memory).
    pub fn delete_category(&mut self, parent_id: i32) {
        let Some(rec) = self.by_parent.remove(&parent_id) else {
            return;
        };
        for &obj_id in &rec.object_ids {
            if let Some(rr) = self.reverse.get_mut(&obj_id) {
                rr.category_ids.retain(|&c| c != parent_id);
                if rr.category_ids.is_empty() {
                    self.reverse.remove(&obj_id);
                }
            }
        }
    }

    /// C++ `addCategoryToObject` — append child to parent (pattern allows dups).
    pub fn add_category_to_object(&mut self, object_id: i32, parent_id: i32) {
        if object_id <= 0 || parent_id <= 0 {
            return;
        }
        self.add_category(parent_id);
        let Some(r) = self.by_parent.get_mut(&parent_id) else {
            return;
        };
        if !r.is_pattern && r.object_ids.contains(&object_id) {
            return;
        }
        r.object_ids.push(object_id);
        r.object_weights.push(0.0);
        let rr = self
            .reverse
            .entry(object_id)
            .or_insert_with(|| ReverseCategoryRecord {
                child_id: object_id,
                category_ids: Vec::new(),
            });
        if !rr.category_ids.contains(&parent_id) {
            rr.category_ids.push(parent_id);
        }
        self.auto_adjust_weights(parent_id, None);
    }

    /// C++ `removeCategoryFromObject` — remove first occurrence of child in parent.
    pub fn remove_category_from_object(&mut self, object_id: i32, parent_id: i32) {
        let Some(r) = self.by_parent.get_mut(&parent_id) else {
            return;
        };
        if let Some(index) = r.object_ids.iter().position(|&id| id == object_id) {
            r.object_ids.remove(index);
            if index < r.object_weights.len() {
                r.object_weights.remove(index);
            }
        }
        if let Some(rr) = self.reverse.get_mut(&object_id) {
            rr.category_ids.retain(|&c| c != parent_id);
            if rr.category_ids.is_empty() {
                self.reverse.remove(&object_id);
            }
        }
        self.auto_adjust_weights(parent_id, None);
    }

    /// C++ `removeObjectFromCategory` — remove at `list_index` when id matches.
    pub fn remove_object_from_category(
        &mut self,
        parent_id: i32,
        object_id: i32,
        list_index: i32,
    ) {
        if list_index < 0 {
            return;
        }
        let idx = list_index as usize;
        let Some(r) = self.by_parent.get_mut(&parent_id) else {
            return;
        };
        if r.object_ids.get(idx).copied() != Some(object_id) {
            return;
        }
        r.object_ids.remove(idx);
        if idx < r.object_weights.len() {
            r.object_weights.remove(idx);
        }
        if let Some(rr) = self.reverse.get_mut(&object_id) {
            rr.category_ids.retain(|&c| c != parent_id);
            if rr.category_ids.is_empty() {
                self.reverse.remove(&object_id);
            }
        }
        self.auto_adjust_weights(parent_id, None);
    }

    /// C++ `removeObjectFromAllCategories`.
    pub fn remove_object_from_all_categories(&mut self, object_id: i32) {
        let Some(rr) = self.reverse.remove(&object_id) else {
            return;
        };
        for c_id in rr.category_ids {
            if let Some(r) = self.by_parent.get_mut(&c_id) {
                while let Some(index) = r.object_ids.iter().position(|&id| id == object_id) {
                    r.object_ids.remove(index);
                    if index < r.object_weights.len() {
                        r.object_weights.remove(index);
                    }
                }
            }
            self.auto_adjust_weights(c_id, None);
        }
    }

    /// C++ `setCategoryIsPattern` — clears probSet + zeros weights.
    pub fn set_category_is_pattern(&mut self, parent_id: i32, is_pattern: bool) {
        let Some(r) = self.by_parent.get_mut(&parent_id) else {
            return;
        };
        r.is_pattern = is_pattern;
        if is_pattern {
            r.is_probability_set = false;
            for w in &mut r.object_weights {
                *w = 0.0;
            }
        }
    }

    /// C++ `setCategoryIsProbabilitySet` — clears pattern; seeds weight if newly set.
    pub fn set_category_is_probability_set(&mut self, parent_id: i32, is_prob: bool) {
        let Some(r) = self.by_parent.get_mut(&parent_id) else {
            return;
        };
        let old = r.is_probability_set;
        r.is_probability_set = is_prob;
        if is_prob {
            r.is_pattern = false;
            if !old {
                // Fix all-zero weights.
                if let Some(w0) = r.object_weights.first_mut() {
                    *w0 = 1.0;
                } else if !r.object_ids.is_empty() {
                    r.object_weights = vec![0.0; r.object_ids.len()];
                    r.object_weights[0] = 1.0;
                }
            }
        } else {
            for w in &mut r.object_weights {
                *w = 0.0;
            }
        }
    }

    /// C++ `moveCategoryMemberUp` — swap with previous; requires id at `list_index`.
    pub fn move_category_member_up(
        &mut self,
        parent_id: i32,
        object_id: i32,
        list_index: i32,
    ) -> bool {
        if list_index <= 0 {
            return false;
        }
        let idx = list_index as usize;
        let Some(r) = self.by_parent.get_mut(&parent_id) else {
            return false;
        };
        if r.object_ids.get(idx).copied() != Some(object_id) {
            return false;
        }
        r.object_ids.swap(idx, idx - 1);
        if r.object_weights.len() == r.object_ids.len() {
            r.object_weights.swap(idx, idx - 1);
        }
        true
    }

    /// C++ `moveCategoryMemberDown`.
    pub fn move_category_member_down(
        &mut self,
        parent_id: i32,
        object_id: i32,
        list_index: i32,
    ) -> bool {
        if list_index < 0 {
            return false;
        }
        let idx = list_index as usize;
        let Some(r) = self.by_parent.get_mut(&parent_id) else {
            return false;
        };
        if idx + 1 >= r.object_ids.len() {
            return false;
        }
        if r.object_ids.get(idx).copied() != Some(object_id) {
            return false;
        }
        r.object_ids.swap(idx, idx + 1);
        if r.object_weights.len() == r.object_ids.len() {
            r.object_weights.swap(idx, idx + 1);
        }
        true
    }

    /// C++ `setMemberWeight` — set weight then rebalance so sum ≈ 1 (hold index fixed).
    pub fn set_member_weight(&mut self, parent_id: i32, object_id: i32, weight: f32) {
        let hold = {
            let Some(r) = self.by_parent.get_mut(&parent_id) else {
                return;
            };
            if !r.is_probability_set {
                return;
            }
            let Some(index) = r.object_ids.iter().position(|&id| id == object_id) else {
                return;
            };
            while r.object_weights.len() < r.object_ids.len() {
                r.object_weights.push(0.0);
            }
            r.object_weights[index] = weight.max(0.0);
            Some(index)
        };
        self.auto_adjust_weights(parent_id, hold);
    }

    /// C++ `makeWeightUniform` — equal weights summing to 1.
    pub fn make_weight_uniform(&mut self, parent_id: i32) {
        let Some(r) = self.by_parent.get_mut(&parent_id) else {
            return;
        };
        if !r.is_probability_set || r.object_ids.is_empty() {
            return;
        }
        let n = r.object_ids.len();
        let u = 1.0 / n as f32;
        r.object_weights = vec![u; n];
    }

    /// C++ `autoAdjustWeights` — keep probability-set weights summing to ≤1 / =1.
    ///
    /// When `hold_index` is set, that member's weight is not reduced/increased
    /// during rebalance (C++ `inHoldIndex`).
    pub fn auto_adjust_weights(&mut self, parent_id: i32, hold_index: Option<usize>) {
        let Some(r) = self.by_parent.get_mut(&parent_id) else {
            return;
        };
        if !r.is_probability_set {
            return;
        }
        while r.object_weights.len() < r.object_ids.len() {
            r.object_weights.push(0.0);
        }
        r.object_weights.truncate(r.object_ids.len());
        if r.object_weights.is_empty() {
            return;
        }
        let mut weight_sum: f32 = r.object_weights.iter().sum();
        let n = r.object_weights.len();
        // Reduce until sum ≤ 1
        let mut next = 0usize;
        while weight_sum > 1.0 + 1e-6 {
            let extra = weight_sum - 1.0;
            if Some(next) == hold_index {
                next += 1;
            }
            if next >= n {
                break;
            }
            let w = r.object_weights[next];
            if w > extra {
                r.object_weights[next] = w - extra;
                weight_sum -= extra;
            } else {
                weight_sum -= w;
                r.object_weights[next] = 0.0;
            }
            next += 1;
            if next >= n {
                break;
            }
        }
        // Top up until sum ≥ 1
        weight_sum = r.object_weights.iter().sum();
        next = 0;
        while weight_sum < 1.0 - 1e-6 {
            let extra = 1.0 - weight_sum;
            if Some(next) == hold_index {
                next += 1;
            }
            if next >= n {
                break;
            }
            let w = r.object_weights[next];
            if w + extra <= 1.0 + 1e-6 {
                r.object_weights[next] = w + extra;
                weight_sum += extra;
            }
            next += 1;
            if next >= n {
                break;
            }
        }
    }

    /// C++ reverse `moveCategoryUp` — swap parent earlier in child's reverse list.
    pub fn move_category_up(&mut self, object_id: i32, parent_id: i32) -> bool {
        let Some(rr) = self.reverse.get_mut(&object_id) else {
            return false;
        };
        let Some(index) = rr.category_ids.iter().position(|&c| c == parent_id) else {
            return false;
        };
        if index == 0 {
            return false;
        }
        rr.category_ids.swap(index, index - 1);
        true
    }

    /// C++ reverse `moveCategoryDown`.
    pub fn move_category_down(&mut self, object_id: i32, parent_id: i32) -> bool {
        let Some(rr) = self.reverse.get_mut(&object_id) else {
            return false;
        };
        let Some(index) = rr.category_ids.iter().position(|&c| c == parent_id) else {
            return false;
        };
        if index + 1 >= rr.category_ids.len() {
            return false;
        }
        rr.category_ids.swap(index, index + 1);
        true
    }
}

/// Serialize a category to disk text (C++ `saveCategoryToDisk` body).
///
/// Empty members → empty string (caller may delete the file).
pub fn format_category_txt(rec: &CategoryRecord) -> String {
    if rec.object_ids.is_empty() {
        return String::new();
    }
    let mut lines = Vec::new();
    lines.push(format!("parentID={}", rec.parent_id));
    if rec.is_pattern {
        lines.push("pattern".into());
    } else if rec.is_probability_set {
        lines.push("probSet".into());
    }
    lines.push(format!("numObjects={}", rec.object_ids.len()));
    for (i, &id) in rec.object_ids.iter().enumerate() {
        if rec.is_probability_set {
            let w = rec.object_weights.get(i).copied().unwrap_or(0.0);
            lines.push(format!("{id} {w}"));
        } else {
            lines.push(format!("{id}"));
        }
    }
    lines.join("\n") + "\n"
}

/// Parse one category `.txt` (C++ `initCategoryBankStep` line scanner).
///
/// ```text
/// parentID=722
/// numObjects=2
/// 502
/// 34
/// ```
/// Optional `pattern` or `probSet` line after `parentID`.
pub fn parse_category_txt(text: &str) -> Option<CategoryRecord> {
    let mut parent_id = 0i32;
    let mut is_pattern = false;
    let mut is_probability_set = false;
    let mut object_ids = Vec::new();
    let mut object_weights = Vec::new();
    let mut in_objects = false;
    let mut expect_n: Option<usize> = None;

    for line in text.lines() {
        let line = line.trim().trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if !in_objects {
            if let Some(rest) = line.strip_prefix("parentID=") {
                parent_id = rest.trim().parse().unwrap_or(0);
            } else if line == "pattern" || line.starts_with("pattern=") {
                is_pattern = true;
            } else if line == "probSet" || line.starts_with("probSet=") {
                is_probability_set = true;
            } else if let Some(rest) = line.strip_prefix("numObjects=") {
                expect_n = rest.trim().parse().ok();
                in_objects = true;
            }
            continue;
        }
        // Member lines: "34" or "34 0.5"
        let mut parts = line.split_whitespace();
        let Some(id_s) = parts.next() else {
            continue;
        };
        let Ok(id) = id_s.parse::<i32>() else {
            continue;
        };
        if id <= 0 {
            continue;
        }
        object_ids.push(id);
        if is_probability_set {
            let w = parts
                .next()
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.0);
            object_weights.push(w);
        } else {
            object_weights.push(0.0);
        }
        if let Some(n) = expect_n {
            if object_ids.len() >= n {
                break;
            }
        }
    }

    if parent_id <= 0 {
        return None;
    }
    // Empty member lists allowed (editor-created cats); load path still works.
    Some(CategoryRecord {
        parent_id,
        is_pattern,
        is_probability_set,
        object_ids,
        object_weights,
    })
}

/// Auto-generate concrete transitions for category parents — **lite** pass.
///
/// // C++: `transitionBank.cpp` `autoGenerateCategoryTransitions` (member pass)
/// // Haxe: `createAndaddCategoryTransitions` (server `expand_category_transitions`)
///
/// Walks existing transitions; when actor and/or target is a **non-pattern,
/// non-probability-set** category parent, emits one concrete transition per
/// member (cartesian when both sides are expandable categories).
/// Existing concrete keys are never overwritten (C++ override / Haxe keep-first).
///
/// Pattern expansion is a separate second pass; prob-set outcomes use
/// [`CategoryBank::pick_from_prob_set`] at materialization time.
///
/// Returns number of newly inserted transitions (normal + last-use + max-use).
pub fn expand_category_transitions_lite(
    transitions: &mut HashMap<(i32, i32), ClientTransition>,
    transitions_last_use: &mut HashMap<(i32, i32), ClientTransition>,
    transitions_max_use: &mut HashMap<(i32, i32), ClientTransition>,
    cats: &CategoryBank,
) -> usize {
    if cats.is_empty() {
        return 0;
    }

    // Snapshot base rows only (do not re-expand already expanded members).
    let base: Vec<ClientTransition> = transitions
        .values()
        .cloned()
        .chain(transitions_last_use.values().cloned())
        .chain(transitions_max_use.values().cloned())
        .collect();

    let mut added = 0usize;

    for t in base {
        // expand_members returns None for pattern / prob-set / non-category.
        let actor_mem = cats
            .expand_members(t.actor_id)
            .map(|s| s.to_vec());
        let target_mem = cats
            .expand_members(t.target_id)
            .map(|s| s.to_vec());

        match (actor_mem, target_mem) {
            (Some(actors), None) => {
                for aid in actors {
                    let nt = substitute_actor(&t, t.actor_id, aid);
                    if try_insert_expanded(
                        transitions,
                        transitions_last_use,
                        transitions_max_use,
                        nt,
                    ) {
                        added += 1;
                    }
                }
            }
            (None, Some(targets)) => {
                for tid in targets {
                    let nt = substitute_target(&t, t.target_id, tid);
                    if try_insert_expanded(
                        transitions,
                        transitions_last_use,
                        transitions_max_use,
                        nt,
                    ) {
                        added += 1;
                    }
                }
            }
            (Some(actors), Some(targets)) => {
                for &aid in &actors {
                    for &tid in &targets {
                        let mut nt = t.clone();
                        // C++ both-side plug: replace parent ids in outcomes.
                        if nt.new_actor_id == t.actor_id {
                            nt.new_actor_id = aid;
                        } else if nt.new_actor_id == t.target_id {
                            nt.new_actor_id = tid;
                        }
                        if nt.new_target_id == t.target_id {
                            nt.new_target_id = tid;
                        } else if nt.new_target_id == t.actor_id {
                            nt.new_target_id = aid;
                        }
                        nt.actor_id = aid;
                        nt.target_id = tid;
                        if try_insert_expanded(
                            transitions,
                            transitions_last_use,
                            transitions_max_use,
                            nt,
                        ) {
                            added += 1;
                        }
                    }
                }
            }
            (None, None) => {}
        }
    }

    added
}

/// C++ `autoGenerateCategoryTransitions` **pattern second pass**.
///
/// For each existing transition, if any of actor/target/newActor/newTarget is a
/// pattern category and all pattern slots share the same member count, and at
/// least actor or target is a pattern parent, emit one concrete transition per
/// index `p` (index-paired). Keep-first: never overwrite existing keys.
///
/// Covers pure pattern edges and Haxe **C+P** after lite expanded the plain
/// category actor side (e.g. 394+1802 → 210+1803 after both passes).
///
/// Call **after** [`expand_category_transitions_lite`].
pub fn expand_category_transitions_pattern(
    transitions: &mut HashMap<(i32, i32), ClientTransition>,
    transitions_last_use: &mut HashMap<(i32, i32), ClientTransition>,
    transitions_max_use: &mut HashMap<(i32, i32), ClientTransition>,
    cats: &CategoryBank,
) -> usize {
    if cats.is_empty() {
        return 0;
    }

    // Snapshot includes lite-expanded rows (C++ walks all records after pass 1).
    let base: Vec<ClientTransition> = transitions
        .values()
        .cloned()
        .chain(transitions_last_use.values().cloned())
        .chain(transitions_max_use.values().cloned())
        .collect();

    let mut added = 0usize;

    for t in base {
        // transIDs[4] = actor, target, newActor, newTarget
        let ids = [t.actor_id, t.target_id, t.new_actor_id, t.new_target_id];
        let mut pattern_slots: [Option<&[i32]>; 4] = [None, None, None, None];
        let mut pattern_size: i32 = -1;
        let mut num_patterns = 0usize;

        for n in 0..4 {
            if ids[n] <= 0 {
                continue;
            }
            let Some(mem) = cats.pattern_members(ids[n]) else {
                continue;
            };
            let len = mem.len() as i32;
            if pattern_size != -1 && len != pattern_size {
                // Size mismatch → ignore this slot (C++ nulls the cat).
                continue;
            }
            if pattern_size == -1 {
                pattern_size = len;
            }
            pattern_slots[n] = Some(mem);
            num_patterns += 1;
        }

        // Require actor|target pattern; skip outcome-only patterns.
        if num_patterns == 0 || pattern_size <= 0 {
            continue;
        }
        if pattern_slots[0].is_none() && pattern_slots[1].is_none() {
            continue;
        }

        let psize = pattern_size as usize;
        for p in 0..psize {
            let mut new_ids = ids;
            for n in 0..4 {
                if let Some(mem) = pattern_slots[n] {
                    if let Some(&oid) = mem.get(p) {
                        new_ids[n] = oid;
                    }
                }
            }
            let mut nt = t.clone();
            nt.actor_id = new_ids[0];
            nt.target_id = new_ids[1];
            nt.new_actor_id = new_ids[2];
            nt.new_target_id = new_ids[3];
            if try_insert_expanded(
                transitions,
                transitions_last_use,
                transitions_max_use,
                nt,
            ) {
                added += 1;
            }
        }
    }

    added
}

/// Lite + pattern expand (load-time). Prob-set parents left abstract.
///
/// Returns total newly inserted transitions.
pub fn expand_category_transitions(
    transitions: &mut HashMap<(i32, i32), ClientTransition>,
    transitions_last_use: &mut HashMap<(i32, i32), ClientTransition>,
    transitions_max_use: &mut HashMap<(i32, i32), ClientTransition>,
    cats: &CategoryBank,
) -> usize {
    let a = expand_category_transitions_lite(
        transitions,
        transitions_last_use,
        transitions_max_use,
        cats,
    );
    let b = expand_category_transitions_pattern(
        transitions,
        transitions_last_use,
        transitions_max_use,
        cats,
    );
    a + b
}

/// C++ plug object in as actor (with newActor/newTarget cross-case).
fn substitute_actor(tr: &ClientTransition, parent_id: i32, o_id: i32) -> ClientTransition {
    let mut nt = tr.clone();
    nt.actor_id = o_id;
    if nt.new_actor_id == parent_id {
        nt.new_actor_id = o_id;
    } else if nt.new_target_id == parent_id {
        // cross-case: actor left on ground in place of target
        nt.new_target_id = o_id;
    }
    nt
}

/// C++ plug object in as target.
fn substitute_target(tr: &ClientTransition, parent_id: i32, o_id: i32) -> ClientTransition {
    let mut nt = tr.clone();
    nt.target_id = o_id;
    if nt.new_target_id == parent_id {
        nt.new_target_id = o_id;
    } else if nt.new_actor_id == parent_id {
        // cross-case: target ends up in hand
        nt.new_actor_id = o_id;
    }
    nt
}

fn try_insert_expanded(
    transitions: &mut HashMap<(i32, i32), ClientTransition>,
    transitions_last_use: &mut HashMap<(i32, i32), ClientTransition>,
    transitions_max_use: &mut HashMap<(i32, i32), ClientTransition>,
    tr: ClientTransition,
) -> bool {
    let key = (tr.actor_id, tr.target_id);
    if tr.last_use_actor || tr.last_use_target {
        if transitions_last_use.contains_key(&key) {
            return false;
        }
        transitions_last_use.insert(key, tr);
        true
    } else {
        // Haxe/server maxUse pair: targetRemains true+false on same key.
        insert_normal_or_max_use_maps(transitions, transitions_max_use, tr)
    }
}

/// Same rules as [`crate::content::insert_normal_or_max_use`] on raw maps.
fn insert_normal_or_max_use_maps(
    transitions: &mut HashMap<(i32, i32), ClientTransition>,
    transitions_max_use: &mut HashMap<(i32, i32), ClientTransition>,
    t: ClientTransition,
) -> bool {
    let key = (t.actor_id, t.target_id);
    let remains = t.target_id >= 0 && t.target_id == t.new_target_id;
    if let Some(existing) = transitions.get(&key).cloned() {
        let exist_remains = existing.target_id >= 0 && existing.target_id == existing.new_target_id;
        if exist_remains && !remains {
            transitions_max_use.insert(key, t);
            return true;
        }
        if !exist_remains && remains {
            transitions_max_use.insert(key, existing);
            transitions.insert(key, t);
            return true;
        }
        return false;
    }
    transitions.insert(key, t);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ClientTransition;

    #[test]
    fn parse_plain_category() {
        let t = "parentID=1001\nnumObjects=2\n912\n1000\n";
        let r = parse_category_txt(t).unwrap();
        assert_eq!(r.parent_id, 1001);
        assert!(!r.is_pattern);
        assert!(!r.is_probability_set);
        assert_eq!(r.object_ids, vec![912, 1000]);
    }

    /// P4#27 editor mutators: add/remove/move/weight/flags + format round-trip.
    #[test]
    fn category_editor_mutators_add_remove_move_weight() {
        let mut bank = CategoryBank::new();
        bank.add_category(900);
        assert!(bank.get_category(900).is_some());
        assert!(bank.get_category(900).unwrap().object_ids.is_empty());

        bank.add_category_to_object(10, 900);
        bank.add_category_to_object(20, 900);
        bank.add_category_to_object(30, 900);
        assert_eq!(
            bank.get_category(900).unwrap().object_ids,
            vec![10, 20, 30]
        );
        assert_eq!(bank.get_num_categories_for_object(20), 1);
        assert_eq!(bank.get_category_for_object(20, 0), 900);

        // Duplicate ignored for non-pattern
        bank.add_category_to_object(20, 900);
        assert_eq!(bank.get_category(900).unwrap().object_ids.len(), 3);

        // Move 30 up from index 2 → [10, 30, 20]
        assert!(bank.move_category_member_up(900, 30, 2));
        assert_eq!(
            bank.get_category(900).unwrap().object_ids,
            vec![10, 30, 20]
        );
        assert!(bank.move_category_member_down(900, 10, 0));
        assert_eq!(
            bank.get_category(900).unwrap().object_ids,
            vec![30, 10, 20]
        );

        bank.remove_object_from_category(900, 10, 1);
        assert_eq!(bank.get_category(900).unwrap().object_ids, vec![30, 20]);
        assert_eq!(bank.get_num_categories_for_object(10), 0);

        bank.remove_category_from_object(20, 900);
        assert_eq!(bank.get_category(900).unwrap().object_ids, vec![30]);

        // Prob set + weights
        bank.set_category_is_probability_set(900, true);
        bank.add_category_to_object(40, 900);
        {
            let r = bank.get_category(900).unwrap();
            assert!(r.is_probability_set);
            assert!(!r.is_pattern);
        }
        bank.set_member_weight(900, 30, 0.7);
        bank.set_member_weight(900, 40, 0.3);
        {
            let r = bank.get_category(900).unwrap();
            let sum: f32 = r.object_weights.iter().sum();
            assert!((sum - 1.0).abs() < 1e-4, "sum={sum}");
            assert!((r.object_weights[0] - 0.7).abs() < 1e-3);
        }
        bank.make_weight_uniform(900);
        {
            let r = bank.get_category(900).unwrap();
            for w in &r.object_weights {
                assert!((w - 0.5).abs() < 1e-4);
            }
        }

        // Format / re-parse
        let txt = format_category_txt(bank.get_category(900).unwrap());
        assert!(txt.contains("probSet"));
        let r2 = parse_category_txt(&txt).unwrap();
        assert_eq!(r2.object_ids, vec![30, 40]);

        bank.delete_category(900);
        assert!(bank.get_category(900).is_none());
        assert_eq!(bank.get_num_categories_for_object(30), 0);
    }

    #[test]
    fn category_editor_pattern_allows_duplicate_members() {
        let mut bank = CategoryBank::new();
        bank.add_category(50);
        bank.set_category_is_pattern(50, true);
        bank.add_category_to_object(7, 50);
        bank.add_category_to_object(7, 50); // pattern allows dup
        assert_eq!(bank.get_category(50).unwrap().object_ids, vec![7, 7]);
        bank.remove_object_from_all_categories(7);
        assert!(bank.get_category(50).unwrap().object_ids.is_empty());
        assert!(bank.get_reverse_category(7).is_none());
    }

    #[test]
    fn category_editor_reverse_move_and_multi_parent() {
        let mut bank = CategoryBank::new();
        bank.add_category_to_object(1, 100);
        bank.add_category_to_object(1, 200);
        assert_eq!(bank.get_num_categories_for_object(1), 2);
        assert_eq!(bank.get_category_for_object(1, 0), 100);
        assert_eq!(bank.get_category_for_object(1, 1), 200);
        assert!(bank.move_category_up(1, 200));
        assert_eq!(bank.get_category_for_object(1, 0), 200);
        assert_eq!(bank.get_category_for_object(1, 1), 100);
        assert!(bank.move_category_down(1, 200));
        assert_eq!(bank.get_category_for_object(1, 0), 100);
    }

    #[test]
    fn parse_pattern_and_prob_set() {
        let pat = "parentID=1016\npattern\nnumObjects=2\n1025\n1026\n";
        let r = parse_category_txt(pat).unwrap();
        assert!(r.is_pattern);
        assert!(!r.is_probability_set);

        let ps = "parentID=3221\nprobSet\nnumObjects=2\n1196 0.800000\n3220 0.200000\n";
        let r = parse_category_txt(ps).unwrap();
        assert!(r.is_probability_set);
        assert!(!r.is_pattern);
        assert_eq!(r.object_ids, vec![1196, 3220]);
        assert!((r.object_weights[0] - 0.8).abs() < 1e-5);
        assert!((r.object_weights[1] - 0.2).abs() < 1e-5);
    }

    #[test]
    fn reverse_map_queries() {
        let mut bank = CategoryBank::new();
        bank.insert_record(CategoryRecord {
            parent_id: 722,
            is_pattern: false,
            is_probability_set: false,
            object_ids: vec![502, 34],
            object_weights: vec![0.0, 0.0],
        });
        bank.insert_record(CategoryRecord {
            parent_id: 1016,
            is_pattern: true,
            is_probability_set: false,
            object_ids: vec![1025, 1026],
            object_weights: vec![0.0, 0.0],
        });

        assert_eq!(bank.get_category(722).unwrap().object_ids, vec![502, 34]);
        assert_eq!(bank.get_num_categories_for_object(34), 1);
        assert_eq!(bank.get_category_for_object(34, 0), 722);
        assert_eq!(bank.get_category_for_object(34, 1), -1);
        assert_eq!(bank.get_category_for_object(999, 0), -1);
        assert!(bank.get_reverse_category(34).is_some());
        assert!(bank.expand_members(722).is_some());
        assert!(bank.expand_members(1016).is_none()); // pattern skipped
        assert!(bank.is_pattern(1016));
        assert!(!bank.is_probability_set(722));
    }

    #[test]
    fn expand_lite_actor_parent() {
        // Category 722 (@ Shallow Digger) → 34 Sharp Stone.
        // Transition 722+36 → 722+39 expands to 34+36 → 34+39.
        let mut bank = CategoryBank::new();
        bank.insert_record(CategoryRecord {
            parent_id: 722,
            is_pattern: false,
            is_probability_set: false,
            object_ids: vec![502, 34],
            object_weights: vec![0.0, 0.0],
        });

        let mut transitions = HashMap::new();
        let mut last = HashMap::new();
        let mut max_use = HashMap::new();
        transitions.insert(
            (722, 36),
            ClientTransition {
                actor_id: 722,
                target_id: 36,
                new_actor_id: 722,
                new_target_id: 39,
                ..Default::default()
            },
        );

        let added = expand_category_transitions_lite(&mut transitions, &mut last, &mut max_use, &bank);
        assert!(added >= 2, "expected expansions for members 502 and 34");
        let t = transitions.get(&(34, 36)).expect("34+36 expanded");
        assert_eq!(t.new_actor_id, 34);
        assert_eq!(t.new_target_id, 39);
        assert!(transitions.contains_key(&(502, 36)));
    }

    #[test]
    fn expand_skips_pattern_and_prob_set() {
        let mut bank = CategoryBank::new();
        bank.insert_record(CategoryRecord {
            parent_id: 10,
            is_pattern: true,
            is_probability_set: false,
            object_ids: vec![11],
            object_weights: vec![0.0],
        });
        bank.insert_record(CategoryRecord {
            parent_id: 20,
            is_pattern: false,
            is_probability_set: true,
            object_ids: vec![21],
            object_weights: vec![1.0],
        });

        let mut transitions = HashMap::new();
        let mut last = HashMap::new();
        let mut max_use = HashMap::new();
        transitions.insert(
            (10, 5),
            ClientTransition {
                actor_id: 10,
                target_id: 5,
                new_actor_id: 10,
                new_target_id: 6,
                ..Default::default()
            },
        );
        transitions.insert(
            (20, 5),
            ClientTransition {
                actor_id: 20,
                target_id: 5,
                new_actor_id: 20,
                new_target_id: 6,
                ..Default::default()
            },
        );

        let added = expand_category_transitions_lite(&mut transitions, &mut last, &mut max_use, &bank);
        assert_eq!(added, 0);
        assert!(!transitions.contains_key(&(11, 5)));
        assert!(!transitions.contains_key(&(21, 5)));
    }

    #[test]
    fn expand_does_not_overwrite_concrete() {
        let mut bank = CategoryBank::new();
        bank.insert_record(CategoryRecord {
            parent_id: 100,
            is_pattern: false,
            is_probability_set: false,
            object_ids: vec![1],
            object_weights: vec![0.0],
        });
        let mut transitions = HashMap::new();
        let mut last = HashMap::new();
        let mut max_use = HashMap::new();
        transitions.insert(
            (100, 2),
            ClientTransition {
                actor_id: 100,
                target_id: 2,
                new_actor_id: 100,
                new_target_id: 3,
                ..Default::default()
            },
        );
        // Concrete override already present.
        transitions.insert(
            (1, 2),
            ClientTransition {
                actor_id: 1,
                target_id: 2,
                new_actor_id: 1,
                new_target_id: 99,
                ..Default::default()
            },
        );
        let added = expand_category_transitions_lite(&mut transitions, &mut last, &mut max_use, &bank);
        assert_eq!(added, 0);
        assert_eq!(transitions.get(&(1, 2)).unwrap().new_target_id, 99);
    }

    #[test]
    fn load_real_categories_if_present() {
        let dir = Path::new(r"C:\OhOl\OpenLife\OneLifeData7\categories");
        if !dir.is_dir() {
            return;
        }
        let bank = CategoryBank::load_from_dir(dir);
        assert!(bank.len() > 50, "categories={}", bank.len());
        // 722 @ Shallow Digger
        if let Some(c) = bank.get_category(722) {
            assert!(c.object_ids.contains(&34), "sharp stone member");
            assert!(!c.is_pattern && !c.is_probability_set);
            // 34 may sit in multiple categories; 722 must appear somewhere.
            let n = bank.get_num_categories_for_object(34);
            assert!(n >= 1);
            let mut found_722 = false;
            for i in 0..n {
                if bank.get_category_for_object(34, i) == 722 {
                    found_722 = true;
                    break;
                }
            }
            assert!(found_722, "reverse map must list parent 722 for member 34");
        }
        // 3221 prob set
        if let Some(c) = bank.get_category(3221) {
            assert!(c.is_probability_set);
            assert!(bank.expand_members(3221).is_none());
            // Weights sum ~1: low roll → first member, high roll → second.
            assert_eq!(bank.pick_from_prob_set(3221, 0.0), 1196);
            assert_eq!(bank.pick_from_prob_set(3221, 0.5), 1196);
            assert_eq!(bank.pick_from_prob_set(3221, 0.9), 3220);
        }
        // Pattern fixtures 1790 / 1802 / 1806 (maple cuttings/saplings).
        if let Some(c) = bank.get_category(1790) {
            assert!(c.is_pattern);
            assert_eq!(c.object_ids.len(), 7);
            assert_eq!(c.object_ids[0], 1791);
        }
        if let Some(c) = bank.get_category(1802) {
            assert!(c.is_pattern);
            assert_eq!(c.object_ids.len(), 7);
        }
    }

    /// Golden: pattern parents 1790 / 1802 / 1806 — index-paired expand.
    ///
    /// 394 + 1802 → 394 + 1806 expands to 394 + 1803 → 394 + 1809 (index 0)
    /// and peer indices; lite also expands actor category 394 → {210,382}.
    #[test]
    fn expand_pattern_index_pair_1802_1806() {
        let mut bank = CategoryBank::new();
        bank.insert_record(CategoryRecord {
            parent_id: 394,
            is_pattern: false,
            is_probability_set: false,
            object_ids: vec![210, 382],
            object_weights: vec![0.0, 0.0],
        });
        bank.insert_record(CategoryRecord {
            parent_id: 1802,
            is_pattern: true,
            is_probability_set: false,
            object_ids: vec![1803, 1804, 1805, 1872, 2723, 3069, 4311],
            object_weights: vec![0.0; 7],
        });
        bank.insert_record(CategoryRecord {
            parent_id: 1806,
            is_pattern: true,
            is_probability_set: false,
            object_ids: vec![1809, 1808, 1807, 1873, 2724, 3070, 4312],
            object_weights: vec![0.0; 7],
        });

        let mut transitions = HashMap::new();
        let mut last = HashMap::new();
        let mut max_use = HashMap::new();
        transitions.insert(
            (394, 1802),
            ClientTransition {
                actor_id: 394,
                target_id: 1802,
                new_actor_id: 394,
                new_target_id: 1806,
                ..Default::default()
            },
        );

        let added = expand_category_transitions(&mut transitions, &mut last, &mut max_use, &bank);
        assert!(added > 0, "expected lite+pattern expansions");

        // Pure pattern on abstract category actor (C++ keeps parent actor).
        let t = transitions
            .get(&(394, 1803))
            .expect("pattern index 0: 394+1803");
        assert_eq!(t.new_actor_id, 394);
        assert_eq!(t.new_target_id, 1809);

        // Last pattern index
        let t = transitions
            .get(&(394, 4311))
            .expect("pattern last index: 394+4311");
        assert_eq!(t.new_target_id, 4312);

        // Haxe C+P: concrete actor from category 394 + pattern target.
        let t = transitions
            .get(&(210, 1803))
            .expect("C+P: 210+1803 after lite actor + pattern");
        assert_eq!(t.new_actor_id, 210);
        assert_eq!(t.new_target_id, 1809);
        assert!(transitions.contains_key(&(382, 1804)));
    }

    /// Pattern-only target (no actor pattern): -1 + 1802 with non-pattern newTarget.
    #[test]
    fn expand_pattern_target_only_keeps_outcome() {
        let mut bank = CategoryBank::new();
        bank.insert_record(CategoryRecord {
            parent_id: 1802,
            is_pattern: true,
            is_probability_set: false,
            object_ids: vec![1803, 1804],
            object_weights: vec![0.0, 0.0],
        });
        let mut transitions = HashMap::new();
        let mut last = HashMap::new();
        let mut max_use = HashMap::new();
        transitions.insert(
            (-1, 1802),
            ClientTransition {
                actor_id: -1,
                target_id: 1802,
                new_actor_id: 0,
                new_target_id: 1828, // non-pattern outcome (Dead Sapling)
                auto_decay_seconds: 1200.0,
                ..Default::default()
            },
        );
        let added = expand_category_transitions_pattern(&mut transitions, &mut last, &mut max_use, &bank);
        assert_eq!(added, 2);
        let t = transitions.get(&(-1, 1803)).unwrap();
        assert_eq!(t.new_target_id, 1828);
        assert!((t.auto_decay_seconds - 1200.0).abs() < 1e-5);
        assert!(transitions.contains_key(&(-1, 1804)));
    }

    /// Cutting pattern 1790 → wet cutting pattern sibling (index pair).
    #[test]
    fn expand_pattern_1790_fixture() {
        let mut bank = CategoryBank::new();
        bank.insert_record(CategoryRecord {
            parent_id: 1790,
            is_pattern: true,
            is_probability_set: false,
            object_ids: vec![1791, 1799, 1798],
            object_weights: vec![0.0; 3],
        });
        // Wet cutting pattern — same length.
        bank.insert_record(CategoryRecord {
            parent_id: 1792,
            is_pattern: true,
            is_probability_set: false,
            object_ids: vec![1793, 1800, 1801],
            object_weights: vec![0.0; 3],
        });
        let mut transitions = HashMap::new();
        let mut last = HashMap::new();
        let mut max_use = HashMap::new();
        // 394 + 1790 → 394 + 1792 (portable water + dry cutting)
        transitions.insert(
            (394, 1790),
            ClientTransition {
                actor_id: 394,
                target_id: 1790,
                new_actor_id: 394,
                new_target_id: 1792,
                ..Default::default()
            },
        );
        let added = expand_category_transitions_pattern(&mut transitions, &mut last, &mut max_use, &bank);
        assert_eq!(added, 3);
        assert_eq!(
            transitions.get(&(394, 1791)).unwrap().new_target_id,
            1793
        );
        assert_eq!(
            transitions.get(&(394, 1799)).unwrap().new_target_id,
            1800
        );
    }

    #[test]
    fn pattern_does_not_overwrite_concrete() {
        let mut bank = CategoryBank::new();
        bank.insert_record(CategoryRecord {
            parent_id: 50,
            is_pattern: true,
            is_probability_set: false,
            object_ids: vec![51, 52],
            object_weights: vec![0.0, 0.0],
        });
        let mut transitions = HashMap::new();
        let mut last = HashMap::new();
        let mut max_use = HashMap::new();
        transitions.insert(
            (0, 50),
            ClientTransition {
                actor_id: 0,
                target_id: 50,
                new_actor_id: 0,
                new_target_id: 99,
                ..Default::default()
            },
        );
        // Authored override for first pattern member.
        transitions.insert(
            (0, 51),
            ClientTransition {
                actor_id: 0,
                target_id: 51,
                new_actor_id: 0,
                new_target_id: 77,
                ..Default::default()
            },
        );
        let added = expand_category_transitions_pattern(&mut transitions, &mut last, &mut max_use, &bank);
        assert_eq!(added, 1); // only 0+52
        assert_eq!(transitions.get(&(0, 51)).unwrap().new_target_id, 77);
        assert_eq!(transitions.get(&(0, 52)).unwrap().new_target_id, 99);
    }

    #[test]
    fn pick_from_prob_set_weighted() {
        let mut bank = CategoryBank::new();
        bank.insert_record(CategoryRecord {
            parent_id: 3221,
            is_pattern: false,
            is_probability_set: true,
            object_ids: vec![1196, 3220],
            object_weights: vec![0.8, 0.2],
        });
        // Non-prob → identity
        assert_eq!(bank.pick_from_prob_set(999, 0.5), 999);
        assert_eq!(bank.pick_from_prob_set(3221, 0.0), 1196);
        assert_eq!(bank.pick_from_prob_set(3221, 0.79), 1196);
        assert_eq!(bank.pick_from_prob_set(3221, 0.81), 3220);
        assert_eq!(bank.pick_from_prob_set(3221, 1.0), 3220);
    }

    #[test]
    fn expand_skips_prob_set_at_load() {
        let mut bank = CategoryBank::new();
        bank.insert_record(CategoryRecord {
            parent_id: 3221,
            is_pattern: false,
            is_probability_set: true,
            object_ids: vec![1196, 3220],
            object_weights: vec![0.8, 0.2],
        });
        let mut transitions = HashMap::new();
        let mut last = HashMap::new();
        let mut max_use = HashMap::new();
        // Decay into prob-set parent — must stay abstract for runtime pick.
        transitions.insert(
            (-1, 1195),
            ClientTransition {
                actor_id: -1,
                target_id: 1195,
                new_actor_id: 0,
                new_target_id: 3221,
                ..Default::default()
            },
        );
        let added = expand_category_transitions(&mut transitions, &mut last, &mut max_use, &bank);
        assert_eq!(added, 0);
        assert_eq!(
            transitions.get(&(-1, 1195)).unwrap().new_target_id,
            3221
        );
    }
}
