//! Haxe `Category` with `probSet=true` (TransformTarget random outcomes).
//!
//! Example: category 3221 "Perhaps a Pumpkin" → 1196 (0.8) / 3220 (0.2).

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::info;

/// Weighted member list for a probSet category parent id.
#[derive(Debug, Clone, Default)]
pub struct ProbSetCategory {
    pub ids: Vec<i32>,
    pub weights: Vec<f32>,
}

/// Load `categories/*.txt` into non-pattern expansion members + probSet tables.
///
/// Returns `(categories, prob_sets, count_non_pattern)`.
pub fn load_category_tables(
    dir: &Path,
) -> (
    HashMap<i32, Vec<i32>>,
    HashMap<i32, ProbSetCategory>,
    usize,
) {
    let mut categories = HashMap::new();
    let mut prob_sets = HashMap::new();
    let mut n = 0usize;
    if !dir.is_dir() {
        return (categories, prob_sets, n);
    }
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
        let mut prob_set = false;
        let mut members = Vec::new();
        let mut weights = Vec::new();
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
                } else if line == "probSet" || line.starts_with("probSet=") {
                    // Haxe Category: bare `probSet` header sets flag.
                    prob_set = true;
                } else if line.starts_with("numObjects=") {
                    in_objects = true;
                }
                continue;
            }
            // member lines: "34" or "34 0.5"
            let mut parts = line.split_whitespace();
            let Some(id_s) = parts.next() else {
                continue;
            };
            if let Ok(id) = id_s.parse::<i32>() {
                members.push(id);
                if prob_set {
                    let w = parts
                        .next()
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(1.0);
                    weights.push(w);
                }
            }
        }
        if parent == 0 || members.is_empty() {
            continue;
        }
        if prob_set {
            prob_sets.insert(
                parent,
                ProbSetCategory {
                    ids: members.clone(),
                    weights,
                },
            );
            // Haxe still stores probSet categories in categoriesById for expansion.
            if !pattern {
                categories.insert(parent, members);
                n += 1;
            }
        } else if !pattern {
            categories.insert(parent, members);
            n += 1;
        }
    }
    info!(
        categories = n,
        prob_sets = prob_sets.len(),
        "content categories loaded (non-pattern + probSet)"
    );
    (categories, prob_sets, n)
}
