//! Reverse craft graph: product → ingredient pairs for planning / self-play.
//!
//! Pure in-memory reverse transition map. No content I/O.

use std::collections::{HashMap, HashSet, VecDeque};

/// Reverse transition map: product object id → list of (actor, target) that produce it.
///
/// A product can appear as `new_actor` and/or `new_target` of a transition.
#[derive(Debug, Default, Clone)]
pub struct ReverseCraftGraph {
    /// product_id → candidate ingredient pairs (actor, target).
    products: HashMap<i32, Vec<(i32, i32)>>,
}

impl ReverseCraftGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that transition `(actor, target) → (new_actor, new_target)`.
    ///
    /// Inserts reverse edges for each non-zero product id among
    /// `new_actor` / `new_target` (skips product 0).
    pub fn insert(&mut self, actor: i32, target: i32, new_actor: i32, new_target: i32) {
        // Skip non-positive products (0 empty, negative OHOL category/wildcard ids).
        for product in [new_actor, new_target] {
            if product <= 0 {
                continue;
            }
            // Avoid self-mapping noise when product is also an input.
            let entry = self.products.entry(product).or_default();
            let pair = (actor, target);
            if !entry.contains(&pair) {
                entry.push(pair);
            }
        }
    }

    /// Ingredient pairs that can produce `product`, if any.
    pub fn ingredients_for(&self, product: i32) -> Option<&[(i32, i32)]> {
        self.products.get(&product).map(|v| v.as_slice())
    }

    /// Number of distinct product ids with at least one reverse edge.
    pub fn product_count(&self) -> usize {
        self.products.len()
    }

    /// Total reverse edges across all products.
    pub fn edge_count(&self) -> usize {
        self.products.values().map(|v| v.len()).sum()
    }

    /// `SAY ?CRAFTSTATS` body without leading p_id: `CRAFTSTATS products=N edges=M`.
    pub fn format_craft_stats_query(&self) -> String {
        format!(
            "CRAFTSTATS products={} edges={}",
            self.product_count(),
            self.edge_count()
        )
    }

    /// `SAY PLAN <object_id>` body without leading p_id.
    ///
    /// Formats the reverse-craft ingredient path from [`Self::find_path_to_product`]:
    /// - `PLAN {want} HAVE` when `want` is already in `have_set`
    /// - `PLAN {want} a+b c+d …` craft steps leaf→root (`actor+target` pairs)
    /// - `PLAN {want} FAIL` when unreachable within `max_depth`
    pub fn format_plan_query(
        &self,
        want: i32,
        have_set: &HashSet<i32>,
        max_depth: usize,
    ) -> String {
        match self.find_path_to_product(want, have_set, max_depth) {
            Some(path) if path.is_empty() => format!("PLAN {want} HAVE"),
            Some(path) => {
                let steps: Vec<String> = path
                    .iter()
                    .map(|&(a, t)| format!("{a}+{t}"))
                    .collect();
                format!("PLAN {want} {}", steps.join(" "))
            }
            None => format!("PLAN {want} FAIL"),
        }
    }

    /// Seed reverse edges from transition records `(actor, target, new_actor, new_target)`.
    ///
    /// Stops after `max_transitions` inserts (boot speed cap). Returns how many
    /// transitions were applied.
    pub fn seed_from_pairs(
        &mut self,
        pairs: impl IntoIterator<Item = (i32, i32, i32, i32)>,
        max_transitions: usize,
    ) -> usize {
        let mut n = 0;
        for (actor, target, new_actor, new_target) in pairs {
            if n >= max_transitions {
                break;
            }
            self.insert(actor, target, new_actor, new_target);
            n += 1;
        }
        n
    }

    /// BFS reverse from `want` over the reverse graph until all ingredients are in
    /// `have_set` or depth exceeds `max_depth`.
    ///
    /// Returns one path of `(actor, target)` pairs from leaves toward `want`
    /// (order: first steps to craft intermediates, last step produces `want`),
    /// or `None` if unreachable within depth.
    pub fn find_path_to_product(
        &self,
        want: i32,
        have_set: &HashSet<i32>,
        max_depth: usize,
    ) -> Option<Vec<(i32, i32)>> {
        if have_set.contains(&want) {
            return Some(Vec::new());
        }
        // State: product we still need, path of transitions chosen so far (root→want).
        let mut queue: VecDeque<(i32, Vec<(i32, i32)>)> = VecDeque::new();
        queue.push_back((want, Vec::new()));
        let mut visited: HashSet<i32> = HashSet::new();
        visited.insert(want);

        while let Some((need, path)) = queue.pop_front() {
            if path.len() > max_depth {
                continue;
            }
            let Some(options) = self.ingredients_for(need) else {
                continue;
            };
            for &(actor, target) in options {
                let mut next_path = path.clone();
                // Prepend so final order is leaf→root (craft order).
                next_path.insert(0, (actor, target));

                // Non-positive ids are empty / OHOL category wildcards — not concrete seeks.
                let actor_ok = actor <= 0 || have_set.contains(&actor);
                let target_ok = target <= 0 || have_set.contains(&target);
                if actor_ok && target_ok {
                    return Some(next_path);
                }

                if next_path.len() > max_depth {
                    continue;
                }

                // Expand missing ingredients as new needs (one branch at a time).
                // Prefer expanding the first missing ingredient for a simple path.
                let missing: Vec<i32> = [actor, target]
                    .into_iter()
                    .filter(|&id| id > 0 && !have_set.contains(&id))
                    .collect();
                if missing.is_empty() {
                    return Some(next_path);
                }
                // Continue BFS from the first missing ingredient; path already
                // includes the step that would use it once available.
                for m in missing {
                    if visited.contains(&m) {
                        continue;
                    }
                    // Only mark visited for this BFS frontier to avoid cycles.
                    visited.insert(m);
                    queue.push_back((m, next_path.clone()));
                }
            }
        }
        None
    }

    /// Pick a non-zero ingredient object id to seek while planning toward `want`.
    ///
    /// Uses [`Self::find_path_to_product`] when a path exists (first missing leaf
    /// ingredient). Falls back to the first reverse-edge actor/target not already
    /// in `have`. Returns `None` if `want` is already owned or no reverse data.
    pub fn seek_ingredient_for(&self, want: i32, have: &HashSet<i32>) -> Option<i32> {
        if have.contains(&want) {
            return None;
        }
        if let Some(path) = self.find_path_to_product(want, have, 6) {
            if path.is_empty() {
                return None;
            }
            // First craft step (leaf→root order): seek first missing positive input.
            if let Some(&(actor, target)) = path.first() {
                if actor > 0 && !have.contains(&actor) {
                    return Some(actor);
                }
                if target > 0 && !have.contains(&target) {
                    return Some(target);
                }
            }
        }
        // Fallback: any reverse ingredient for the product (self-play map scan).
        if let Some(pairs) = self.ingredients_for(want) {
            for &(actor, target) in pairs {
                if actor > 0 && !have.contains(&actor) {
                    return Some(actor);
                }
                if target > 0 && !have.contains(&target) {
                    return Some(target);
                }
            }
        }
        None
    }

    /// Product ids that list `ingredient` as actor or target in a reverse edge.
    ///
    /// Used by `SAY NEXTCRAFT` (held as ingredient → products it can help make).
    /// Sorted ascending; empty when `ingredient <= 0` or no reverse data.
    pub fn products_using(&self, ingredient: i32) -> Vec<i32> {
        if ingredient <= 0 {
            return Vec::new();
        }
        let mut out: Vec<i32> = self
            .products
            .iter()
            .filter_map(|(&product, pairs)| {
                if pairs
                    .iter()
                    .any(|&(a, t)| a == ingredient || t == ingredient)
                {
                    Some(product)
                } else {
                    None
                }
            })
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Complementary ingredient id to pair with `held` when crafting `want`.
    ///
    /// Scans reverse edges for `want` (and craft-path steps) for pairs containing
    /// `held`; returns the other positive side if not already in `have`.
    pub fn partner_for_held(&self, want: i32, held: i32, have: &HashSet<i32>) -> Option<i32> {
        if held <= 0 || want <= 0 {
            return None;
        }
        // Prefer path step that already uses held.
        if let Some(path) = self.find_path_to_product(want, have, 6) {
            for &(actor, target) in &path {
                if actor == held && target > 0 && !have.contains(&target) {
                    return Some(target);
                }
                if target == held && actor > 0 && !have.contains(&actor) {
                    return Some(actor);
                }
            }
        }
        // Direct reverse edges for want.
        if let Some(pairs) = self.ingredients_for(want) {
            for &(actor, target) in pairs {
                if actor == held && target > 0 {
                    return Some(target);
                }
                if target == held && actor > 0 {
                    return Some(actor);
                }
            }
        }
        // Any product made from held that is itself an ingredient for want (or is want).
        for product in self.products_using(held) {
            let useful = product == want
                || self
                    .ingredients_for(want)
                    .map(|pairs| pairs.iter().any(|&(a, t)| a == product || t == product))
                    .unwrap_or(false);
            if !useful {
                continue;
            }
            if let Some(pairs) = self.ingredients_for(product) {
                for &(actor, target) in pairs {
                    if actor == held && target > 0 && target != held {
                        return Some(target);
                    }
                    if target == held && actor > 0 && actor != held {
                        return Some(actor);
                    }
                }
            }
        }
        None
    }

    /// True when `held` appears as actor/target on a reverse edge for `want`
    /// (or a craft-path step). Self-play keeps such held items instead of dropping.
    pub fn held_is_craft_ingredient(&self, want: i32, held: i32) -> bool {
        if held == 0 || want == 0 {
            return false;
        }
        if held == want {
            return true;
        }
        let empty = HashSet::new();
        if let Some(path) = self.find_path_to_product(want, &empty, 6) {
            for &(actor, target) in &path {
                if actor == held || target == held {
                    return true;
                }
            }
        }
        if let Some(pairs) = self.ingredients_for(want) {
            if pairs.iter().any(|&(a, t)| a == held || t == held) {
                return true;
            }
        }
        // Held crafts a product that is want or an ingredient of want.
        for product in self.products_using(held) {
            if product == want {
                return true;
            }
            if let Some(pairs) = self.ingredients_for(want) {
                if pairs.iter().any(|&(a, t)| a == product || t == product) {
                    return true;
                }
            }
        }
        false
    }

    /// `SAY NEXTCRAFT` / `SAY NEXTCRAFT <id>` body without leading p_id.
    ///
    /// Lists products craftable using `item` as an ingredient (reverse graph scan):
    /// - `NEXTCRAFT empty` when item is 0
    /// - `NEXTCRAFT {id} none` when no products use it
    /// - `NEXTCRAFT {id} p1 p2 …` (up to 12 product ids)
    pub fn format_nextcraft_query(&self, item: i32) -> String {
        if item == 0 {
            return "NEXTCRAFT empty".into();
        }
        let products = self.products_using(item);
        if products.is_empty() {
            format!("NEXTCRAFT {item} none")
        } else {
            let list: Vec<String> = products
                .iter()
                .take(12)
                .map(|p| p.to_string())
                .collect();
            format!("NEXTCRAFT {item} {}", list.join(" "))
        }
    }

    /// `SAY RECIPE` / `SAY RECIPE <id>` body without leading p_id.
    ///
    /// Lists [`Self::ingredients_for`] pairs treating `product` as the craft result:
    /// - `RECIPE empty` when product is 0
    /// - `RECIPE {id} none` when no reverse edges
    /// - `RECIPE {id} a+b c+d …` (up to 12 actor+target pairs)
    pub fn format_recipe_query(&self, product: i32) -> String {
        if product == 0 {
            return "RECIPE empty".into();
        }
        match self.ingredients_for(product) {
            Some(pairs) if !pairs.is_empty() => {
                let list: Vec<String> = pairs
                    .iter()
                    .take(12)
                    .map(|&(a, t)| format!("{a}+{t}"))
                    .collect();
                format!("RECIPE {product} {}", list.join(" "))
            }
            _ => format!("RECIPE {product} none"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tiny chain: A+B→C, C+D→E
    fn sample_graph() -> ReverseCraftGraph {
        let mut g = ReverseCraftGraph::new();
        // A=1, B=2, C=3, D=4, E=5
        g.insert(1, 2, 3, 0); // A+B → C (held)
        g.insert(3, 4, 5, 0); // C+D → E (held)
        g
    }

    #[test]
    fn ingredients_for_product() {
        let g = sample_graph();
        assert_eq!(g.ingredients_for(3), Some(&[(1, 2)][..]));
        assert_eq!(g.ingredients_for(5), Some(&[(3, 4)][..]));
        assert!(g.ingredients_for(99).is_none());
    }

    #[test]
    fn format_craft_stats_query_products_edges() {
        let g = sample_graph();
        let s = g.format_craft_stats_query();
        assert!(s.starts_with("CRAFTSTATS "), "got {s}");
        assert!(s.contains("products=2"), "got {s}");
        assert!(s.contains("edges=2"), "got {s}");
        let empty = ReverseCraftGraph::new().format_craft_stats_query();
        assert_eq!(empty, "CRAFTSTATS products=0 edges=0");
    }

    #[test]
    fn format_plan_query_path_have_fail() {
        let g = sample_graph();
        let have_ab: HashSet<i32> = [1, 2].into_iter().collect();
        assert_eq!(g.format_plan_query(3, &have_ab, 4), "PLAN 3 1+2");
        let have_e: HashSet<i32> = [5].into_iter().collect();
        assert_eq!(g.format_plan_query(5, &have_e, 4), "PLAN 5 HAVE");
        let empty = HashSet::new();
        assert_eq!(g.format_plan_query(99, &empty, 4), "PLAN 99 FAIL");
        let have_abd: HashSet<i32> = [1, 2, 4].into_iter().collect();
        let plan = g.format_plan_query(5, &have_abd, 4);
        assert!(plan.starts_with("PLAN 5 "), "got {plan}");
        assert!(plan.contains("1+2"), "got {plan}");
        assert!(plan.contains("3+4"), "got {plan}");
    }

    #[test]
    fn path_when_have_direct_ingredients() {
        let g = sample_graph();
        let have: HashSet<i32> = [1, 2].into_iter().collect();
        let path = g.find_path_to_product(3, &have, 4).expect("path to C");
        assert_eq!(path, vec![(1, 2)]);
    }

    #[test]
    fn path_two_step_chain() {
        let g = sample_graph();
        // Have A,B,D — need to craft C then E
        let have: HashSet<i32> = [1, 2, 4].into_iter().collect();
        let path = g.find_path_to_product(5, &have, 4).expect("path to E");
        // Must include both transitions; C is intermediate from A+B.
        assert!(path.contains(&(1, 2)), "must craft C from A+B: {path:?}");
        assert!(path.contains(&(3, 4)), "must craft E from C+D: {path:?}");
        assert_eq!(path.len(), 2);
    }

    #[test]
    fn already_have_product_empty_path() {
        let g = sample_graph();
        let have: HashSet<i32> = [5].into_iter().collect();
        let path = g.find_path_to_product(5, &have, 2).unwrap();
        assert!(path.is_empty());
    }

    #[test]
    fn unreachable_returns_none() {
        let g = sample_graph();
        let have: HashSet<i32> = [1].into_iter().collect(); // missing B for C, missing all for E
        assert!(g.find_path_to_product(5, &have, 4).is_none());
    }

    #[test]
    fn insert_records_new_target_product() {
        let mut g = ReverseCraftGraph::new();
        g.insert(10, 11, 0, 12); // product on ground
        assert_eq!(g.ingredients_for(12), Some(&[(10, 11)][..]));
    }

    #[test]
    fn seed_from_pairs_respects_cap() {
        let mut g = ReverseCraftGraph::new();
        let pairs = (0..10).map(|i| (i, i + 1, i + 100, 0));
        let n = g.seed_from_pairs(pairs, 3);
        assert_eq!(n, 3);
        assert_eq!(g.product_count(), 3);
        assert!(g.edge_count() >= 3);
    }

    #[test]
    fn seek_ingredient_prefers_missing_leaf() {
        let g = sample_graph();
        // Have A=1 only; want E=5 → need B (or C path). First missing for C is B=2.
        let have: HashSet<i32> = [1].into_iter().collect();
        let seek = g.seek_ingredient_for(5, &have);
        // Path may expand missing ingredients; any of 2,3,4 is a useful intermediate.
        assert!(
            matches!(seek, Some(2) | Some(3) | Some(4)),
            "expected intermediate ingredient, got {seek:?}"
        );
        // Already have product → None
        let have_e: HashSet<i32> = [5].into_iter().collect();
        assert_eq!(g.seek_ingredient_for(5, &have_e), None);
        // Empty have + known product → first reverse actor/target
        let empty = HashSet::new();
        let s = g.seek_ingredient_for(3, &empty);
        assert!(matches!(s, Some(1) | Some(2)), "got {s:?}");
    }

    #[test]
    fn products_using_and_nextcraft_recipe() {
        let g = sample_graph();
        assert_eq!(g.products_using(1), vec![3]);
        assert_eq!(g.products_using(3), vec![5]);
        assert!(g.products_using(99).is_empty());
        assert_eq!(g.format_nextcraft_query(0), "NEXTCRAFT empty");
        assert_eq!(g.format_nextcraft_query(1), "NEXTCRAFT 1 3");
        assert_eq!(g.format_nextcraft_query(99), "NEXTCRAFT 99 none");
        assert_eq!(g.format_recipe_query(0), "RECIPE empty");
        assert_eq!(g.format_recipe_query(3), "RECIPE 3 1+2");
        assert_eq!(g.format_recipe_query(5), "RECIPE 5 3+4");
        assert_eq!(g.format_recipe_query(99), "RECIPE 99 none");
    }

    #[test]
    fn partner_and_held_craft_ingredient() {
        let g = sample_graph();
        let empty = HashSet::new();
        assert_eq!(g.partner_for_held(3, 1, &empty), Some(2));
        assert_eq!(g.partner_for_held(5, 3, &empty), Some(4));
        assert!(g.held_is_craft_ingredient(5, 1));
        assert!(g.held_is_craft_ingredient(5, 3));
        assert!(g.held_is_craft_ingredient(5, 5));
        assert!(!g.held_is_craft_ingredient(5, 99));
    }
}
