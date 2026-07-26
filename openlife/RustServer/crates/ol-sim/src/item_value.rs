//! Pure item value helpers: recipe cost estimator, tool quality tiers,
//! and object rarity inferred from display names.
//!
//! No world / player state — pure functions only. Callers pass names and
//! optional reverse-craft ingredient pairs. Used by chat queries
//! `SAY ?RARITY` / `SAY ?QUALITY` (and `format_cost_query` for craft depth).

use crate::craft_graph::ReverseCraftGraph;
use std::collections::{HashSet, VecDeque};

// ---------------------------------------------------------------------------
// Tool quality tiers
// ---------------------------------------------------------------------------

/// Ordered tool / item quality tier (higher = better).
///
/// Derived from object **display name** keywords (case-insensitive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ToolQuality {
    /// Broken / cracked / ruined / worn-out.
    Broken = 0,
    /// Stone / flint / wood / bone crude tools.
    Crude = 1,
    /// Default / no strong material signal.
    Common = 2,
    /// Iron / copper / bronze mid-tier.
    Fair = 3,
    /// Steel / sharp / fine / tempered.
    Fine = 4,
    /// Masterwork / legendary / adamant / mythic.
    Master = 5,
}

impl ToolQuality {
    /// Wire / chat token (title case single word).
    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Broken => "Broken",
            Self::Crude => "Crude",
            Self::Common => "Common",
            Self::Fair => "Fair",
            Self::Fine => "Fine",
            Self::Master => "Master",
        }
    }

    /// Numeric score (matches discriminant).
    pub fn score(self) -> u8 {
        self as u8
    }

    /// Parse wire name (case-insensitive); unknown → `None`.
    pub fn from_wire(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "broken" => Some(Self::Broken),
            "crude" => Some(Self::Crude),
            "common" => Some(Self::Common),
            "fair" => Some(Self::Fair),
            "fine" => Some(Self::Fine),
            "master" => Some(Self::Master),
            _ => None,
        }
    }
}

/// Keyword tables for quality (checked highest-first so "steel" beats "iron").
const QUALITY_MASTER: &[&str] = &[
    "masterwork",
    "masterpiece",
    "legendary",
    "mythic",
    "adamant",
    "mithril",
    "perfect",
    "divine",
];
const QUALITY_FINE: &[&str] = &[
    "steel",
    "sharp",
    "fine",
    "tempered",
    "honed",
    "polished",
    "refined",
    "hardened",
];
const QUALITY_FAIR: &[&str] = &[
    "iron",
    "copper",
    "bronze",
    "brass",
    "metal",
    "forged",
];
const QUALITY_CRUDE: &[&str] = &[
    "flint",
    "stone",
    "wooden",
    "wood",
    "bone",
    "crude",
    "rough",
    "primitive",
];
const QUALITY_BROKEN: &[&str] = &[
    "broken",
    "cracked",
    "ruined",
    "shattered",
    "worn",
    "rusty",
    "rusted",
    "damaged",
];

/// Infer tool quality tier from an object display name.
///
/// Empty / whitespace names → [`ToolQuality::Common`].
/// Keyword priority: Broken → Master → Fine → Fair → Crude → Common.
/// Damage keywords win even when a material is also present
/// (e.g. `"Broken Iron Sword"` → Broken).
pub fn tool_quality_from_name(name: &str) -> ToolQuality {
    let n = name.trim().to_ascii_lowercase();
    if n.is_empty() {
        return ToolQuality::Common;
    }
    if contains_any(&n, QUALITY_BROKEN) {
        return ToolQuality::Broken;
    }
    if contains_any(&n, QUALITY_MASTER) {
        return ToolQuality::Master;
    }
    if contains_any(&n, QUALITY_FINE) {
        return ToolQuality::Fine;
    }
    if contains_any(&n, QUALITY_FAIR) {
        return ToolQuality::Fair;
    }
    if contains_any(&n, QUALITY_CRUDE) {
        return ToolQuality::Crude;
    }
    ToolQuality::Common
}

/// True if name looks like a tool / weapon / implement (for optional filters).
pub fn name_looks_like_tool(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    const HINTS: &[&str] = &[
        "axe", "knife", "sword", "pick", "shovel", "hoe", "hammer", "chisel",
        "saw", "blade", "tool", "adze", "mallet", "scythe", "sickle", "spear",
        "bow", "arrow", "club", "mace", "lance", "file", "awl", "needle",
        "tongs", "shear", "shears", "pliers",
    ];
    HINTS.iter().any(|h| n.contains(h))
}

// ---------------------------------------------------------------------------
// Object rarity from name
// ---------------------------------------------------------------------------

/// Ordered object rarity (higher = rarer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ObjectRarity {
    Common = 0,
    Uncommon = 1,
    Rare = 2,
    Epic = 3,
    Legendary = 4,
}

impl ObjectRarity {
    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Common => "Common",
            Self::Uncommon => "Uncommon",
            Self::Rare => "Rare",
            Self::Epic => "Epic",
            Self::Legendary => "Legendary",
        }
    }

    pub fn score(self) -> u8 {
        self as u8
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "common" => Some(Self::Common),
            "uncommon" => Some(Self::Uncommon),
            "rare" => Some(Self::Rare),
            "epic" => Some(Self::Epic),
            "legendary" => Some(Self::Legendary),
            _ => None,
        }
    }
}

const RARITY_LEGENDARY: &[&str] = &[
    "legendary",
    "mythic",
    "divine",
    "artifact",
    "relic",
    "phoenix",
    "dragon",
];
const RARITY_EPIC: &[&str] = &[
    "epic",
    "diamond",
    "platinum",
    "mithril",
    "adamant",
    "obsidian",
    "crystal",
    "enchanted",
];
const RARITY_RARE: &[&str] = &[
    "rare",
    "gold",
    "golden",
    "silver",
    "jewel",
    "gem",
    "ruby",
    "emerald",
    "sapphire",
    "pearl",
    "ivory",
];
const RARITY_UNCOMMON: &[&str] = &[
    "uncommon",
    "steel",
    "iron",
    "bronze",
    "copper",
    "glass",
    "porcelain",
    "silk",
    "fine",
    "polished",
    "tempered",
];
/// Common-material / wildcraft signals (only used when no higher hit).
const RARITY_COMMON_HINTS: &[&str] = &[
    "berry",
    "grass",
    "dirt",
    "mud",
    "stick",
    "twig",
    "leaf",
    "straw",
    "clay",
    "sand",
    "water",
    "snow",
    "ice",
    "stone",
    "rock",
    "wood",
    "wooden",
    "branch",
    "bark",
    "seed",
    "sprout",
    "weed",
];

/// Infer object rarity from display name keywords.
///
/// Empty name → [`ObjectRarity::Common`]. Highest matching band wins.
pub fn object_rarity_from_name(name: &str) -> ObjectRarity {
    let n = name.trim().to_ascii_lowercase();
    if n.is_empty() {
        return ObjectRarity::Common;
    }
    if contains_any(&n, RARITY_LEGENDARY) {
        return ObjectRarity::Legendary;
    }
    if contains_any(&n, RARITY_EPIC) {
        return ObjectRarity::Epic;
    }
    if contains_any(&n, RARITY_RARE) {
        return ObjectRarity::Rare;
    }
    if contains_any(&n, RARITY_UNCOMMON) {
        return ObjectRarity::Uncommon;
    }
    // Explicit common materials stay Common; unknown names also Common.
    let _ = contains_any(&n, RARITY_COMMON_HINTS);
    ObjectRarity::Common
}

/// Combine name rarity with craft depth for a slightly richer signal.
///
/// Depth ≥ 6 can raise rarity by at most one band (capped at Legendary).
/// Depth ≥ 10 can raise by two bands. Name-only floor is never lowered.
pub fn object_rarity_with_depth(name: &str, craft_depth: u32) -> ObjectRarity {
    let base = object_rarity_from_name(name);
    let bump = if craft_depth >= 10 {
        2
    } else if craft_depth >= 6 {
        1
    } else {
        0
    };
    let score = (base.score() as u32).saturating_add(bump).min(4) as u8;
    match score {
        0 => ObjectRarity::Common,
        1 => ObjectRarity::Uncommon,
        2 => ObjectRarity::Rare,
        3 => ObjectRarity::Epic,
        _ => ObjectRarity::Legendary,
    }
}

// ---------------------------------------------------------------------------
// Recipe cost estimator
// ---------------------------------------------------------------------------

/// Estimated craft cost for a product id via reverse-graph BFS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecipeCost {
    /// Product object id.
    pub product: i32,
    /// Minimum craft steps found (0 if already "free"/no graph / depth 0).
    pub steps: u32,
    /// Unique positive ingredient ids encountered on the cheapest path.
    pub unique_ingredients: u32,
    /// BFS depth (same as steps for path length).
    pub depth: u32,
    /// Composite cost score: `1 + steps*3 + unique*2 + depth`.
    pub cost: u32,
    /// True when no reverse edges existed for the product.
    pub unknown: bool,
}

impl RecipeCost {
    /// Empty hands / invalid product.
    pub fn empty() -> Self {
        Self {
            product: 0,
            steps: 0,
            unique_ingredients: 0,
            depth: 0,
            cost: 0,
            unknown: true,
        }
    }

    /// Leaf / natural object with no reverse edges (base cost 1).
    pub fn leaf(product: i32) -> Self {
        Self {
            product,
            steps: 0,
            unique_ingredients: 0,
            depth: 0,
            cost: 1,
            unknown: true,
        }
    }

    /// Build composite score from steps / unique / depth.
    pub fn from_parts(product: i32, steps: u32, unique: u32, depth: u32) -> Self {
        let cost = 1u32
            .saturating_add(steps.saturating_mul(3))
            .saturating_add(unique.saturating_mul(2))
            .saturating_add(depth);
        Self {
            product,
            steps,
            unique_ingredients: unique,
            depth,
            cost,
            unknown: false,
        }
    }
}

/// Estimate recipe cost for `product` using the reverse craft graph.
///
/// BFS over reverse edges up to `max_depth`. Treats non-positive ids as free
/// wildcards. Returns [`RecipeCost::empty`] for `product <= 0`,
/// [`RecipeCost::leaf`] when the product has no reverse edges.
pub fn estimate_recipe_cost(
    graph: &ReverseCraftGraph,
    product: i32,
    max_depth: usize,
) -> RecipeCost {
    if product <= 0 {
        return RecipeCost::empty();
    }
    // Direct ingredients?
    let Some(pairs) = graph.ingredients_for(product) else {
        return RecipeCost::leaf(product);
    };
    if pairs.is_empty() {
        return RecipeCost::leaf(product);
    }

    // BFS: state = (need_product, path_len, unique_set size estimate via set)
    // We track path ingredient ids for the first successful "all leaves" path.
    let mut queue: VecDeque<(i32, u32, HashSet<i32>)> = VecDeque::new();
    let mut visited: HashSet<i32> = HashSet::new();
    queue.push_back((product, 0, HashSet::new()));
    visited.insert(product);

    let mut best: Option<RecipeCost> = None;

    while let Some((need, depth, mut ingredients)) = queue.pop_front() {
        if depth as usize > max_depth {
            continue;
        }
        let Some(options) = graph.ingredients_for(need) else {
            // Leaf ingredient: cost is what we already collected.
            if need != product {
                ingredients.insert(need);
            }
            let steps = depth;
            let unique = ingredients.len() as u32;
            let cand = RecipeCost::from_parts(product, steps, unique, depth);
            best = Some(match best {
                Some(b) if b.cost <= cand.cost => b,
                _ => cand,
            });
            continue;
        };

        if options.is_empty() {
            let steps = depth;
            let unique = ingredients.len() as u32;
            let cand = RecipeCost::from_parts(product, steps, unique, depth);
            best = Some(match best {
                Some(b) if b.cost <= cand.cost => b,
                _ => cand,
            });
            continue;
        }

        for &(actor, target) in options {
            let mut next_ing = ingredients.clone();
            let mut missing: Vec<i32> = Vec::new();
            for id in [actor, target] {
                if id <= 0 {
                    continue;
                }
                next_ing.insert(id);
                // Expand only if this id itself has reverse edges (crafted).
                if graph.ingredients_for(id).is_some() && !visited.contains(&id) {
                    missing.push(id);
                }
            }
            let next_depth = depth + 1;
            if missing.is_empty() {
                let unique = next_ing.len() as u32;
                let cand = RecipeCost::from_parts(product, next_depth, unique, next_depth);
                best = Some(match best {
                    Some(b) if b.cost <= cand.cost => b,
                    _ => cand,
                });
            } else {
                for m in missing {
                    if visited.contains(&m) {
                        continue;
                    }
                    visited.insert(m);
                    queue.push_back((m, next_depth, next_ing.clone()));
                }
            }
        }
    }

    best.unwrap_or_else(|| {
        // Had reverse edges but BFS found nothing within depth — still not leaf.
        let direct_unique = pairs
            .iter()
            .flat_map(|&(a, t)| [a, t])
            .filter(|&id| id > 0)
            .collect::<HashSet<_>>()
            .len() as u32;
        RecipeCost::from_parts(product, 1, direct_unique, 1)
    })
}

/// Estimate cost from a flat list of direct ingredient pairs only (no BFS).
///
/// Useful when callers only know one-shot recipes.
pub fn estimate_direct_recipe_cost(product: i32, pairs: &[(i32, i32)]) -> RecipeCost {
    if product <= 0 {
        return RecipeCost::empty();
    }
    if pairs.is_empty() {
        return RecipeCost::leaf(product);
    }
    let unique = pairs
        .iter()
        .flat_map(|&(a, t)| [a, t])
        .filter(|&id| id > 0)
        .collect::<HashSet<_>>()
        .len() as u32;
    RecipeCost::from_parts(product, 1, unique, 1)
}

// ---------------------------------------------------------------------------
// Query formatters (SAY bodies without leading p_id)
// ---------------------------------------------------------------------------

/// `SAY ?QUALITY` body for held object.
///
/// - empty hands → `QUALITY 0`
/// - else → `QUALITY {id} tier={Name} score={n} [tool=1|0]`
pub fn format_quality_query(held_id: i32, name: Option<&str>) -> String {
    if held_id == 0 {
        return "QUALITY 0".into();
    }
    let name = name.unwrap_or("").trim();
    let q = tool_quality_from_name(name);
    let tool_flag = if name_looks_like_tool(name) { 1 } else { 0 };
    format!(
        "QUALITY {held_id} tier={} score={} tool={tool_flag}",
        q.wire_name(),
        q.score()
    )
}

/// `SAY ?RARITY` body for held object.
///
/// - empty hands → `RARITY 0`
/// - else → `RARITY {id} tier={Name} score={n}`
pub fn format_rarity_query(held_id: i32, name: Option<&str>) -> String {
    if held_id == 0 {
        return "RARITY 0".into();
    }
    let name = name.unwrap_or("").trim();
    let r = object_rarity_from_name(name);
    format!(
        "RARITY {held_id} tier={} score={}",
        r.wire_name(),
        r.score()
    )
}

/// `SAY ?RARITY` with optional craft-depth bump.
pub fn format_rarity_query_with_depth(
    held_id: i32,
    name: Option<&str>,
    craft_depth: u32,
) -> String {
    if held_id == 0 {
        return "RARITY 0".into();
    }
    let name = name.unwrap_or("").trim();
    let r = object_rarity_with_depth(name, craft_depth);
    format!(
        "RARITY {held_id} tier={} score={} depth={craft_depth}",
        r.wire_name(),
        r.score()
    )
}

/// `SAY ?COST` / recipe cost body (product id + estimate).
///
/// - product 0 → `COST 0`
/// - unknown leaf → `COST {id} cost=1 steps=0 unique=0 depth=0 unknown=1`
/// - known → `COST {id} cost=N steps=S unique=U depth=D unknown=0`
pub fn format_cost_query(est: &RecipeCost) -> String {
    if est.product == 0 {
        return "COST 0".into();
    }
    format!(
        "COST {} cost={} steps={} unique={} depth={} unknown={}",
        est.product,
        est.cost,
        est.steps,
        est.unique_ingredients,
        est.depth,
        if est.unknown { 1 } else { 0 }
    )
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

fn contains_any(haystack_lower: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack_lower.contains(n))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_graph() -> ReverseCraftGraph {
        let mut g = ReverseCraftGraph::new();
        // 1+2 → 3, 3+4 → 5, 5+6 → 7
        g.insert(1, 2, 3, 0);
        g.insert(3, 4, 5, 0);
        g.insert(5, 6, 7, 0);
        g
    }

    // --- quality ---

    #[test]
    fn quality_broken_wins_over_material() {
        assert_eq!(
            tool_quality_from_name("Broken Iron Sword"),
            ToolQuality::Broken
        );
        assert_eq!(tool_quality_from_name("Rusty Axe"), ToolQuality::Broken);
        assert_eq!(tool_quality_from_name("Cracked Flint Knife"), ToolQuality::Broken);
    }

    #[test]
    fn quality_master_and_fine() {
        assert_eq!(
            tool_quality_from_name("Masterwork Steel Blade"),
            ToolQuality::Master
        );
        assert_eq!(
            tool_quality_from_name("Legendary Sword"),
            ToolQuality::Master
        );
        assert_eq!(
            tool_quality_from_name("Steel Axe"),
            ToolQuality::Fine
        );
        assert_eq!(
            tool_quality_from_name("Sharp Flint Knife"),
            ToolQuality::Fine
        );
        assert_eq!(
            tool_quality_from_name("Tempered Iron Sword"),
            ToolQuality::Fine
        );
    }

    #[test]
    fn quality_fair_crude_common() {
        assert_eq!(tool_quality_from_name("Iron Axe"), ToolQuality::Fair);
        assert_eq!(tool_quality_from_name("Copper Knife"), ToolQuality::Fair);
        assert_eq!(tool_quality_from_name("Flint Knife"), ToolQuality::Crude);
        assert_eq!(tool_quality_from_name("Stone Axe"), ToolQuality::Crude);
        assert_eq!(tool_quality_from_name("Wooden Bowl"), ToolQuality::Crude);
        assert_eq!(tool_quality_from_name("Basket"), ToolQuality::Common);
        assert_eq!(tool_quality_from_name(""), ToolQuality::Common);
        assert_eq!(tool_quality_from_name("   "), ToolQuality::Common);
    }

    #[test]
    fn quality_case_insensitive_and_wire() {
        assert_eq!(tool_quality_from_name("IRON SWORD"), ToolQuality::Fair);
        assert_eq!(ToolQuality::Fine.wire_name(), "Fine");
        assert_eq!(ToolQuality::Fine.score(), 4);
        assert_eq!(ToolQuality::from_wire("fine"), Some(ToolQuality::Fine));
        assert_eq!(ToolQuality::from_wire("nope"), None);
        assert!(ToolQuality::Broken < ToolQuality::Crude);
        assert!(ToolQuality::Fine < ToolQuality::Master);
    }

    #[test]
    fn name_looks_like_tool_hints() {
        assert!(name_looks_like_tool("Flint Knife"));
        assert!(name_looks_like_tool("Long Bow"));
        assert!(name_looks_like_tool("Iron Pickaxe"));
        assert!(!name_looks_like_tool("Gooseberry"));
        assert!(!name_looks_like_tool("Basket"));
    }

    // --- rarity ---

    #[test]
    fn rarity_bands_from_name() {
        assert_eq!(object_rarity_from_name("Stick"), ObjectRarity::Common);
        assert_eq!(object_rarity_from_name("Wild Gooseberry"), ObjectRarity::Common);
        assert_eq!(object_rarity_from_name("Iron Ore"), ObjectRarity::Uncommon);
        assert_eq!(object_rarity_from_name("Steel Blade"), ObjectRarity::Uncommon);
        assert_eq!(object_rarity_from_name("Gold Coin"), ObjectRarity::Rare);
        assert_eq!(object_rarity_from_name("Silver Ring"), ObjectRarity::Rare);
        assert_eq!(object_rarity_from_name("Diamond"), ObjectRarity::Epic);
        assert_eq!(object_rarity_from_name("Enchanted Crystal"), ObjectRarity::Epic);
        assert_eq!(
            object_rarity_from_name("Legendary Artifact"),
            ObjectRarity::Legendary
        );
        assert_eq!(object_rarity_from_name("Dragon Scale"), ObjectRarity::Legendary);
        assert_eq!(object_rarity_from_name(""), ObjectRarity::Common);
    }

    #[test]
    fn rarity_case_and_wire() {
        assert_eq!(object_rarity_from_name("GOLD NUGGET"), ObjectRarity::Rare);
        assert_eq!(ObjectRarity::Epic.wire_name(), "Epic");
        assert_eq!(ObjectRarity::Epic.score(), 3);
        assert_eq!(
            ObjectRarity::from_wire("legendary"),
            Some(ObjectRarity::Legendary)
        );
        assert_eq!(ObjectRarity::from_wire("x"), None);
        assert!(ObjectRarity::Common < ObjectRarity::Rare);
    }

    #[test]
    fn rarity_depth_bump() {
        // Common name + depth 6 → Uncommon
        assert_eq!(
            object_rarity_with_depth("Basket", 6),
            ObjectRarity::Uncommon
        );
        // Common + depth 10 → Rare
        assert_eq!(object_rarity_with_depth("Basket", 10), ObjectRarity::Rare);
        // Already Rare + depth 6 → Epic
        assert_eq!(
            object_rarity_with_depth("Gold Coin", 6),
            ObjectRarity::Epic
        );
        // Cap at Legendary
        assert_eq!(
            object_rarity_with_depth("Legendary Sword", 99),
            ObjectRarity::Legendary
        );
        // No bump under threshold
        assert_eq!(
            object_rarity_with_depth("Basket", 5),
            ObjectRarity::Common
        );
    }

    // --- recipe cost ---

    #[test]
    fn cost_empty_and_leaf() {
        let g = ReverseCraftGraph::new();
        assert_eq!(estimate_recipe_cost(&g, 0, 6), RecipeCost::empty());
        assert_eq!(estimate_recipe_cost(&g, -1, 6), RecipeCost::empty());
        let leaf = estimate_recipe_cost(&g, 42, 6);
        assert_eq!(leaf, RecipeCost::leaf(42));
        assert_eq!(leaf.cost, 1);
        assert!(leaf.unknown);
    }

    #[test]
    fn cost_direct_and_chain() {
        let g = sample_graph();
        let c3 = estimate_recipe_cost(&g, 3, 6);
        assert!(!c3.unknown);
        assert_eq!(c3.product, 3);
        assert!(c3.steps >= 1);
        assert!(c3.unique_ingredients >= 2); // 1 and 2
        assert!(c3.cost >= 1 + 3 + 4); // steps*3 + unique*2 at minimum-ish

        let c7 = estimate_recipe_cost(&g, 7, 8);
        assert!(!c7.unknown);
        assert!(c7.depth >= c3.depth);
        assert!(c7.cost >= c3.cost);
    }

    #[test]
    fn cost_direct_helper() {
        assert_eq!(
            estimate_direct_recipe_cost(0, &[(1, 2)]),
            RecipeCost::empty()
        );
        assert_eq!(
            estimate_direct_recipe_cost(9, &[]),
            RecipeCost::leaf(9)
        );
        let d = estimate_direct_recipe_cost(10, &[(1, 2), (1, 3)]);
        assert_eq!(d.steps, 1);
        assert_eq!(d.unique_ingredients, 3); // 1,2,3
        assert!(!d.unknown);
        assert_eq!(d.cost, 1 + 3 + 6 + 1); // 1 + steps*3 + unique*2 + depth
    }

    #[test]
    fn cost_from_parts_formula() {
        let r = RecipeCost::from_parts(5, 2, 3, 2);
        assert_eq!(r.cost, 1 + 6 + 6 + 2);
        assert!(!r.unknown);
    }

    // --- formatters ---

    #[test]
    fn format_quality_shapes() {
        assert_eq!(format_quality_query(0, None), "QUALITY 0");
        assert_eq!(format_quality_query(0, Some("Iron Axe")), "QUALITY 0");
        let s = format_quality_query(12, Some("Iron Axe"));
        assert_eq!(s, "QUALITY 12 tier=Fair score=3 tool=1");
        let s2 = format_quality_query(3, Some("Basket"));
        assert_eq!(s2, "QUALITY 3 tier=Common score=2 tool=0");
        let s3 = format_quality_query(9, None);
        assert_eq!(s3, "QUALITY 9 tier=Common score=2 tool=0");
        let s4 = format_quality_query(1, Some("Broken Steel Sword"));
        assert_eq!(s4, "QUALITY 1 tier=Broken score=0 tool=1");
    }

    #[test]
    fn format_rarity_shapes() {
        assert_eq!(format_rarity_query(0, None), "RARITY 0");
        assert_eq!(
            format_rarity_query(7, Some("Gold Coin")),
            "RARITY 7 tier=Rare score=2"
        );
        assert_eq!(
            format_rarity_query(1, Some("Stick")),
            "RARITY 1 tier=Common score=0"
        );
        assert_eq!(
            format_rarity_query(2, None),
            "RARITY 2 tier=Common score=0"
        );
        let d = format_rarity_query_with_depth(4, Some("Basket"), 6);
        assert_eq!(d, "RARITY 4 tier=Uncommon score=1 depth=6");
        assert_eq!(format_rarity_query_with_depth(0, Some("x"), 9), "RARITY 0");
    }

    #[test]
    fn format_cost_shapes() {
        assert_eq!(format_cost_query(&RecipeCost::empty()), "COST 0");
        assert_eq!(
            format_cost_query(&RecipeCost::leaf(42)),
            "COST 42 cost=1 steps=0 unique=0 depth=0 unknown=1"
        );
        let est = RecipeCost::from_parts(5, 2, 3, 2);
        assert_eq!(
            format_cost_query(&est),
            format!(
                "COST 5 cost={} steps=2 unique=3 depth=2 unknown=0",
                est.cost
            )
        );
    }

    #[test]
    fn quality_ordering_scores_unique() {
        let all = [
            ToolQuality::Broken,
            ToolQuality::Crude,
            ToolQuality::Common,
            ToolQuality::Fair,
            ToolQuality::Fine,
            ToolQuality::Master,
        ];
        for (i, q) in all.iter().enumerate() {
            assert_eq!(q.score() as usize, i);
            assert_eq!(ToolQuality::from_wire(q.wire_name()), Some(*q));
        }
    }

    #[test]
    fn rarity_ordering_scores_unique() {
        let all = [
            ObjectRarity::Common,
            ObjectRarity::Uncommon,
            ObjectRarity::Rare,
            ObjectRarity::Epic,
            ObjectRarity::Legendary,
        ];
        for (i, r) in all.iter().enumerate() {
            assert_eq!(r.score() as usize, i);
            assert_eq!(ObjectRarity::from_wire(r.wire_name()), Some(*r));
        }
    }

    #[test]
    fn cost_unknown_product_in_partial_graph() {
        let g = sample_graph();
        // 99 not in graph
        let c = estimate_recipe_cost(&g, 99, 4);
        assert_eq!(c, RecipeCost::leaf(99));
    }
}
