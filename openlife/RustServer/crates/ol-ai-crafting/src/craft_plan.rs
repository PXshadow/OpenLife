//! Format reverse-craft plans for SAY PLAN.

use crate::craft_graph::ReverseCraftGraph; // same crate
// OL-AI-SPLIT: moved from ol-sim

/// `PLAN id a+b; c+d` or `PLAN id none`.
pub fn format_plan(graph: &ReverseCraftGraph, product: i32) -> String {
    match graph.find_path_to_product(product, &Default::default(), 6) {
        Some(steps) if !steps.is_empty() => {
            let parts: Vec<String> = steps
                .iter()
                .map(|(a, t)| format!("{a}+{t}"))
                .collect();
            format!("PLAN {product} {}", parts.join("; "))
        }
        _ => {
            // Fall back to direct ingredients only.
            match graph.ingredients_for(product) {
                Some(list) if !list.is_empty() => {
                    let parts: Vec<String> = list
                        .iter()
                        .take(8)
                        .map(|(a, t)| format!("{a}+{t}"))
                        .collect();
                    format!("PLAN {product} {}", parts.join("; "))
                }
                _ => format!("PLAN {product} none"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_chain() {
        let mut g = ReverseCraftGraph::default();
        g.insert(1, 2, 3, 0); // 1+2 → 3
        g.insert(3, 4, 5, 0); // 3+4 → 5
        let p = format_plan(&g, 5);
        assert!(p.starts_with("PLAN 5"));
        assert!(p.contains('+'));
        assert_eq!(format_plan(&g, 999), "PLAN 999 none");
    }
}
