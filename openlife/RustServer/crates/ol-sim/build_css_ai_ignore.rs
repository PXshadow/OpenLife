//! C-SS-AI-IGNORE — wire ContentDb.ai_should_ignore into craft graph + topdown search.

use std::fs;
use std::path::Path;

pub fn already_wired(src: &Path) -> bool {
    let lib = src.join("lib.rs");
    let top = src.join("craft_topdown.rs");
    let lib_ok = fs::read_to_string(&lib)
        .map(|t| t.contains("load_ai_should_ignore_from"))
        .unwrap_or(false);
    let top_ok = fs::read_to_string(&top)
        .map(|t| t.contains("ai_should_ignore_edge"))
        .unwrap_or(false);
    lib_ok && top_ok
}

pub fn patch_all(src: &Path) -> bool {
    let mut changed = false;
    changed |= patch_lib(src);
    changed |= patch_craft_topdown(src);
    changed
}

fn patch_lib(src: &Path) -> bool {
    let path = src.join("lib.rs");
    let Ok(raw) = fs::read_to_string(&path) else {
        return false;
    };
    if raw.contains("load_ai_should_ignore_from") {
        return false;
    }
    let needle = "    let mut graph = ReverseCraftGraph::new();\n    let seeded = graph.seed_from_pairs(pairs, cap);\n    info!(\n        seeded,\n        products = graph.product_count(),\n        edges = graph.edge_count(),\n        cap,\n        \"sim: reverse craft graph built from content\"\n    );";
    let insert = "    let mut graph = ReverseCraftGraph::new();\n    let seeded = graph.seed_from_pairs(pairs, cap);\n    // C-SS-AI-IGNORE: ServerSettings.PatchTransitions aiShouldIgnore → craft skip edges\n    graph.load_ai_should_ignore_from(content.ai_should_ignore.iter().copied());\n    info!(\n        seeded,\n        products = graph.product_count(),\n        edges = graph.edge_count(),\n        ai_ignore = graph.ai_should_ignore_count(),\n        cap,\n        \"sim: reverse craft graph built from content\"\n    );";
    if !raw.contains(needle) {
        // Loose fallback
        if let Some(idx) = raw.find("let seeded = graph.seed_from_pairs(pairs, cap);") {
            let mut next = raw.clone();
            next.insert_str(
                idx + "let seeded = graph.seed_from_pairs(pairs, cap);".len(),
                "\n    // C-SS-AI-IGNORE\n    graph.load_ai_should_ignore_from(content.ai_should_ignore.iter().copied());",
            );
            if !next.contains("ai_ignore = graph.ai_should_ignore_count()") {
                next = next.replace(
                    "edges = graph.edge_count(),\n        cap,\n        \"sim: reverse craft graph built from content\"",
                    "edges = graph.edge_count(),\n        ai_ignore = graph.ai_should_ignore_count(),\n        cap,\n        \"sim: reverse craft graph built from content\"",
                );
            }
            let _ = fs::write(&path, next);
            return true;
        }
        return false;
    }
    let next = raw.replace(needle, insert);
    if next != raw {
        let _ = fs::write(&path, next);
        true
    } else {
        false
    }
}

fn patch_craft_topdown(src: &Path) -> bool {
    let path = src.join("craft_topdown.rs");
    let Ok(raw) = fs::read_to_string(&path) else {
        return false;
    };
    if raw.contains("ai_should_ignore_edge") {
        return false;
    }
    // Inject graph ignore check before meta skip in path and ingredients loops.
    let old = "        for &(actor, target) in &path {\n            let meta = opts.meta_for(actor, target);\n            if should_skip_craft_edge(actor, target, product_id, radius, opts, meta) {\n                continue;\n            }";
    let new = "        for &(actor, target) in &path {\n            // C-SS-AI-IGNORE: content PatchTransitions aiShouldIgnore\n            if graph.ai_should_ignore_edge(actor, target) {\n                continue;\n            }\n            let meta = opts.meta_for(actor, target);\n            if should_skip_craft_edge(actor, target, product_id, radius, opts, meta) {\n                continue;\n            }";
    let mut next = raw.replace(old, new);
    let old2 = "        for &(actor, target) in pairs {\n            let meta = opts.meta_for(actor, target);\n            if should_skip_craft_edge(actor, target, product_id, radius, opts, meta) {\n                continue;\n            }";
    let new2 = "        for &(actor, target) in pairs {\n            // C-SS-AI-IGNORE: content PatchTransitions aiShouldIgnore\n            if graph.ai_should_ignore_edge(actor, target) {\n                continue;\n            }\n            let meta = opts.meta_for(actor, target);\n            if should_skip_craft_edge(actor, target, product_id, radius, opts, meta) {\n                continue;\n            }";
    next = next.replace(old2, new2);
    // craft_trans_meta_map_from_content helper if missing
    if !next.contains("craft_trans_meta_map_from_content") {
        if let Some(idx) = next.find("pub fn with_ai_should_ignore(mut self, v: bool) -> Self {") {
            // insert after CraftTransMeta impl block ends — after with_ignore_if_min
            if let Some(end) = next[idx..].find("\n}\n\n/// Why `DoTransitionSearch`") {
                let at = idx + end + 1; // after closing brace of impl
                let helper = r#"

/// Build [`CraftTransMeta`] map from content transitions + ai_should_ignore side-table.
///
/// Keys are `(actor_id, target_id)`. Prefer primary transitions; last-use fills gaps.
/// `ai_should_ignore` from [`ol_content::ContentDb`] is applied on every edge.
// Haxe: TransitionData fields + ServerSettings.PatchTransitions aiShouldIgnore
pub fn craft_trans_meta_map_from_content(
    content: &ol_content::ContentDb,
) -> HashMap<(i32, i32), CraftTransMeta> {
    let mut map = HashMap::new();
    for t in content.transitions.values() {
        let mut m = CraftTransMeta::pair(
            t.actor_id,
            t.target_id,
            t.new_actor_id,
            t.new_target_id,
        )
        .with_auto_decay(t.auto_decay_seconds)
        .with_reverse_use_target(t.reverse_use_target)
        .with_target_min_use_fraction(t.target_min_use_fraction);
        if content.ai_should_ignore.contains(&(t.actor_id, t.target_id)) {
            m = m.with_ai_should_ignore(true);
        }
        map.insert((t.actor_id, t.target_id), m);
    }
    for t in content.transitions_last_use.values() {
        let key = (t.actor_id, t.target_id);
        if map.contains_key(&key) {
            continue;
        }
        let mut m = CraftTransMeta::pair(
            t.actor_id,
            t.target_id,
            t.new_actor_id,
            t.new_target_id,
        )
        .with_auto_decay(t.auto_decay_seconds)
        .with_reverse_use_target(t.reverse_use_target)
        .with_target_min_use_fraction(t.target_min_use_fraction);
        if content.ai_should_ignore.contains(&key) {
            m = m.with_ai_should_ignore(true);
        }
        map.insert(key, m);
    }
    // Orphan ignore keys (no transition body) still force skip via meta.
    for &(a, t) in &content.ai_should_ignore {
        map.entry((a, t))
            .and_modify(|m| m.ai_should_ignore = true)
            .or_insert_with(|| CraftTransMeta::pair(a, t, 0, 0).with_ai_should_ignore(true));
    }
    map
}
"#;
                next.insert_str(at + 1, helper); // after `}`
            }
        }
    }
    if next != raw {
        let _ = fs::write(&path, next);
        true
    } else {
        false
    }
}
