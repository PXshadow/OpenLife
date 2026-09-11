//! `/object-counts` page: minute object-census graphs with the same time-range control as `/ops`.

use ol_sim::ObjectCountSample;
use std::collections::HashMap;

pub fn object_counts_series_json(
    samples: &[ObjectCountSample],
    live_originals: &HashMap<i32, i32>,
) -> serde_json::Value {
    let mut names = serde_json::Map::new();
    let mut originals = serde_json::Map::new();
    let live_has = !live_originals.is_empty();
    for (&id, &n) in live_originals {
        if id > 0 {
            originals.insert(id.to_string(), serde_json::json!(n));
        }
    }
    let series: Vec<serde_json::Value> = samples
        .iter()
        .map(|s| {
            for t in &s.top {
                if !t.name.is_empty() {
                    names.insert(t.id.to_string(), serde_json::Value::String(t.name.clone()));
                }
                if !live_has && t.original > 0 {
                    originals.insert(t.id.to_string(), serde_json::json!(t.original));
                }
            }
            let objects: Vec<serde_json::Value> = s
                .top
                .iter()
                .map(|t| serde_json::json!([t.id, t.current, t.original]))
                .collect();
            serde_json::json!({
                "wall_unix_ms": s.wall_unix_ms,
                "total": s.total,
                "unique": s.unique,
                "objects": objects,
            })
        })
        .collect();
    serde_json::json!({
        "samples": series,
        "count": series.len(),
        "names": names,
        "originals": originals,
    })
}

pub fn build_object_counts_html(
    samples: &[ObjectCountSample],
    version: &str,
    live_originals: &HashMap<i32, i32>,
) -> String {
    let last = samples.last();
    let total = last.map(|s| s.total).unwrap_or(0);
    let unique = last.map(|s| s.unique).unwrap_or(0);
    let series = object_counts_series_json(samples, live_originals);
    let series_js =
        serde_json::to_string(&series).unwrap_or_else(|_| "{\"samples\":[],\"count\":0,\"names\":{}}".into());
    format!(
        r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><title>Object counts — Open Life Reborn</title>
<style>
body{{font-family:system-ui,sans-serif;background:#0b0f14;color:#e7eef7;margin:2rem;line-height:1.5}}
a{{color:#6ec6ff}}
.muted{{color:#8b9bb0;font-size:.9rem}}
.cards{{display:flex;flex-wrap:wrap;gap:1rem}}
.card{{background:#1a222e;padding:1rem 1.25rem;border-radius:8px;min-width:11rem}}
.chart{{margin:1.5rem 0;background:#121a24;border:1px solid #1e2a3a;border-radius:8px;padding:1rem}}
.chart-head{{display:flex;flex-wrap:wrap;align-items:center;justify-content:space-between;gap:.75rem;margin-bottom:.5rem}}
.chart-head h3{{margin:0;font-size:1.05rem}}
.time-display select,.chart-head select,.toolbar input,.toolbar select,.toolbar button{{background:#0b0f14;color:#e7eef7;border:1px solid #2a3a4e;border-radius:4px;padding:.25rem .4rem}}
.toolbar{{display:flex;flex-wrap:wrap;gap:.75rem;align-items:center;margin:1rem 0}}
.toolbar input[type=search]{{min-width:16rem}}
.chart-wrap{{position:relative}}
.chart-wrap svg{{width:100%;height:180px;display:block;background:#0b0f14;border-radius:4px}}
.chart-wrap .line{{stroke:#6ec6ff;stroke-width:2;fill:none}}
.chart-wrap .grid{{stroke:#1e2a3a;stroke-width:1}}
.chart-wrap .axis{{stroke:#3a4a5e;stroke-width:1}}
.chart-wrap .axis-label{{fill:#8b9bb0;font-size:11px;font-family:system-ui,sans-serif}}
.chart-wrap .cross{{stroke:#e7eef7;stroke-width:1;stroke-dasharray:3 3;opacity:.7}}
.chart-wrap .dot{{fill:#6ec6ff;stroke:#0b0f14;stroke-width:1}}
.hover-tip{{position:absolute;pointer-events:none;background:#0e1620;color:#e7eef7;border:1px solid #2a3a4e;border-radius:6px;padding:.45rem .6rem;font-size:.85rem;line-height:1.35;min-width:10rem;box-shadow:0 8px 24px rgba(0,0,0,.45)}}
table{{border-collapse:collapse;width:100%;margin-top:1rem}}
th,td{{border-bottom:1px solid #1e2a3a;padding:.4rem .5rem;text-align:left}}
th{{color:#8b9bb0;font-weight:600}}
.tri-up{{color:#3dd68c}}
.tri-down{{color:#ff5c5c}}
.tri-flat{{color:#8b9bb0}}
</style></head>
<body>
<p class="muted"><a href="/">home</a> · <a href="/ops">ops</a> · <a href="/ai-obs">AI obs</a> · object counts · v{version}</p>
<h1>Object counts</h1>
<p class="muted">World census sampled once a minute (same maps as ObjectCounts.txt). Time range matches the ops dashboard and is saved in this browser. Graphs default to the 10 types with the largest 24-hour change.</p>
<div class="cards">
<div class="card"><strong>Total objects</strong><br/><span data-field="total">{total}</span></div>
<div class="card"><strong>Unique types</strong><br/><span data-field="unique">{unique}</span></div>
<div class="card"><strong>Samples</strong><br/><span data-field="samples">{n}</span></div>
</div>
<p class="time-display"><label>Time range
<select id="ops-time-range" title="How much history the graphs show">
<option value="all">All samples</option>
<option value="30s">Last 30 seconds</option>
<option value="1m">Last 1 minute</option>
<option value="5m" selected>Last 5 minutes</option>
<option value="15m">Last 15 minutes</option>
<option value="1h">Last 1 hour</option>
<option value="1d">Last day</option>
<option value="1w">Last week</option>
<option value="30d">Last month</option>
</select></label></p>
<div id="ops-charts"></div>
<h2>Objects</h2>
<div class="toolbar">
<label>Search <input id="obj-search" type="search" placeholder="name or id" title="Filter the object list"/></label>
<label>Sort
<select id="obj-sort" title="Sort the object list">
<option value="change" selected>Biggest 24h % change</option>
<option value="count">Current count</option>
</select></label>
<button type="button" id="obj-select-all" title="Show every matching object, or only the top 100">Select all</button>
<span class="muted" id="obj-visible"></span>
<span class="muted">Excluded by search: <span data-field="excluded">0</span></span>
</div>
<table><thead><tr><th></th><th>Id</th><th>Name</th><th>Current</th><th>Original</th><th>24h</th><th>Week</th><th>Month</th></tr></thead>
<tbody id="top-table"></tbody></table>
<script>window.OLR_OBJ={series_js};</script>
<script>{js}</script>
</body></html>
"##,
        version = version,
        total = total,
        unique = unique,
        n = samples.len(),
        series_js = series_js,
        js = include_str!("object_counts.js"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_sim::{ObjectCountSample, ObjectCountTop};

    #[test]
    fn object_counts_page_has_time_range_and_title() {
        let samples = vec![ObjectCountSample {
            wall_unix_ms: 1,
            total: 12,
            unique: 2,
            top: vec![ObjectCountTop {
                id: 33,
                current: 10,
                original: 8,
                name: "Gooseberry".into(),
            }],
        }];
        let html = build_object_counts_html(&samples, "0.2.0", &std::collections::HashMap::new());
        assert!(html.contains("Object counts"));
        assert!(html.contains("Time range"));
        assert!(html.contains("ops-time-range"));
        assert!(html.contains("Last day"));
        assert!(html.contains("Last week"));
        assert!(html.contains("Last month"));
        assert!(html.contains("obj-search"));
        assert!(html.contains("obj-select-all"));
        assert!(html.contains("Biggest 24h % change"));
        assert!(html.contains("/ops"));
        assert!(html.contains("/ai-obs"));
        assert!(html.contains("id=\"ops-charts\""));
        assert!(html.contains("No samples yet"));
        assert!(html.contains("top-table"));
        assert!(!html.contains("class=\"pt\""));
        let v = object_counts_series_json(&samples, &std::collections::HashMap::new());
        assert_eq!(v["count"], 1);
        assert_eq!(v["samples"][0]["total"], 12);
        assert_eq!(v["names"]["33"], "Gooseberry");
        assert_eq!(v["samples"][0]["objects"][0][0], 33);
        assert_eq!(v["samples"][0]["objects"][0][1], 10);
        assert_eq!(v["originals"]["33"], 8);
        let mut live = std::collections::HashMap::new();
        live.insert(33, 40);
        let v2 = object_counts_series_json(&samples, &live);
        assert_eq!(v2["originals"]["33"], 40);
        assert_eq!(v2["originals"].as_object().unwrap().len(), 1);
    }

    #[test]
    fn empty_object_counts_page_still_has_chart_host() {
        let html = build_object_counts_html(&[], "0.2.0", &std::collections::HashMap::new());
        assert!(html.contains("id=\"ops-charts\""));
        assert!(html.contains("data-field=\"total\">0<"));
        assert!(html.contains("window.OLR_OBJ="));
        assert!(html.contains("obj-search"));
    }
}
