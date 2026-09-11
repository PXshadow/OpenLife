//! `/ai-obs` page: NPC crafts, baby care, death ages, time spent (not CPU timings).

use ol_content::ContentDb;

pub fn build_ai_obs_html(stats: &serde_json::Value, version: &str, content: &ContentDb) -> String {
    let mut names = serde_json::Map::new();
    for key in ["crafted_objects", "food_eaten"] {
        if let Some(arr) = stats.get(key).and_then(|v| v.as_array()) {
            for row in arr {
                if let Some(id) = row.get("id").and_then(|x| x.as_i64()) {
                    let name = content
                        .get(id as i32)
                        .map(|d| d.name.clone())
                        .unwrap_or_else(|| format!("object {id}"));
                    names.insert(id.to_string(), serde_json::Value::String(name));
                }
            }
        }
    }
    if let Some(samples) = stats.get("samples").and_then(|v| v.as_array()) {
        for s in samples {
            if let Some(rows) = s.get("crafted").and_then(|v| v.as_array()) {
                for row in rows {
                    if let Some(id) = row.get(0).and_then(|x| x.as_i64()) {
                        if !names.contains_key(&id.to_string()) {
                            let name = content
                                .get(id as i32)
                                .map(|d| d.name.clone())
                                .unwrap_or_else(|| format!("object {id}"));
                            names.insert(id.to_string(), serde_json::Value::String(name));
                        }
                    }
                }
            }
        }
    }
    let mut stats_out = stats.clone();
    if let Some(obj) = stats_out.as_object_mut() {
        obj.insert("names".into(), serde_json::Value::Object(names));
    }
    let stats_js = serde_json::to_string(&stats_out).unwrap_or_else(|_| "{}".into());
    format!(
        r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><meta name="viewport" content="width=device-width, initial-scale=1"/><title>AI obs — Open Life Reborn</title>
<style>
body{{font-family:system-ui,sans-serif;background:#0b0f14;color:#e7eef7;margin:2rem;line-height:1.5}}
@media (max-width:640px){{body{{margin:1rem}} .card{{min-width:8rem}}}}
a{{color:#6ec6ff}}
.muted{{color:#8b9bb0;font-size:.9rem}}
.cards{{display:flex;flex-wrap:wrap;gap:1rem}}
.card{{background:#1a222e;padding:1rem 1.25rem;border-radius:8px;min-width:11rem}}
.chart{{margin:1.5rem 0;background:#121a24;border:1px solid #1e2a3a;border-radius:8px;padding:1rem}}
.chart-head{{display:flex;flex-wrap:wrap;align-items:center;justify-content:space-between;gap:.75rem;margin-bottom:.5rem}}
.chart-head h3{{margin:0;font-size:1.05rem}}
.time-display select,.chart-head select,.toolbar input,.toolbar select,.toolbar button{{background:#0b0f14;color:#e7eef7;border:1px solid #2a3a4e;border-radius:4px;padding:.25rem .4rem}}
.toolbar{{display:flex;flex-wrap:wrap;gap:.75rem;align-items:center;margin:1rem 0}}
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
th,td{{border-bottom:1px solid #1e2a3a;padding:.45rem .6rem;text-align:left;vertical-align:top}}
th{{color:#8b9bb0;font-weight:600;font-size:.85rem;letter-spacing:.02em}}
td.num{{text-align:right;font-variant-numeric:tabular-nums;white-space:nowrap}}
td.id{{color:#8b9bb0;font-size:.85rem;font-variant-numeric:tabular-nums}}
tbody tr:hover{{background:#151d28}}
.obj-name{{font-weight:600}}
h1 .ver{{color:#8b9bb0;font-size:.55em;font-weight:600;margin-left:.45rem}}
</style></head>
<body>
<p class="muted"><a href="/">home</a> · <a href="/ops">ops</a> · <a href="/object-counts">object counts</a> · AI obs · v{version}</p>
<h1>AI obs <span class="ver">v{version}</span></h1>
<p class="muted">NPC crafts, baby care, death ages, time spent, and NPC CPU. Sim tick timings stay on <a href="/ops">ops</a>. Graph time range is saved in this browser.</p>
<div class="cards">
<div class="card"><strong>Babies named</strong><br/><span data-field="named">0</span><br/><span class="muted" data-field="namedUnique">unique 0</span></div>
<div class="card"><strong>Babies picked up</strong><br/><span data-field="pickup">0</span><br/><span class="muted" data-field="pickupUnique">unique 0</span></div>
<div class="card"><strong>Babies dropped</strong><br/><span data-field="drop">0</span></div>
<div class="card"><strong>Deaths</strong><br/><span data-field="deaths">0</span></div>
<div class="card"><strong>Craft attempts</strong><br/><span data-field="craft">0</span></div>
<div class="card"><strong>Eat attempts</strong><br/><span data-field="eat">0</span></div>
<div class="card"><strong>Stuck events</strong><br/><span data-field="stuck">0</span></div>
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
<h2>Baby care</h2>
<p class="muted">Named vs picked up vs dropped, over the selected time range.</p>
<div id="baby-charts"></div>
<h2>Death ages</h2>
<div class="cards" id="age-cards"></div>
<div id="age-charts"></div>
<h2>Time spent</h2>
<p class="muted">Sim-time on each kind of action (walk to use, walk to baby, escape, attack, drop…). Graphs default to the ten largest totals.</p>
<div id="spend-charts"></div>
<table><thead><tr><th>What they did</th><th>Time</th></tr></thead><tbody id="spend-table"></tbody></table>
<h2>NPC timings</h2>
<p class="muted">CPU for NPC think / world scan. Not sim tick work.</p>
<div class="cards" id="ai-timing-cards"></div>
<div id="ai-timing-charts"></div>
<h2>Objects crafted</h2>
<p class="muted">What NPCs produced (craft USE / CraftItem / makeSharpieFood). Graphs default to the ten most-crafted types.</p>
<div id="craft-charts"></div>
<table><thead><tr><th>Object</th><th>Id</th><th>Count</th></tr></thead><tbody id="craft-table"></tbody></table>
<h2>Actions</h2>
<table><thead><tr><th>Action</th><th>Count</th></tr></thead><tbody id="action-table"></tbody></table>
<h2>Food eaten</h2>
<table><thead><tr><th>Food</th><th>Id</th><th>Count</th></tr></thead><tbody id="food-table"></tbody></table>
<p class="muted"><a href="/api/npc/stats">JSON stats</a> · <a href="/ops">ops timings</a></p>
<script>window.OLR_AI={stats_js};</script>
<script>{js}</script>
</body></html>
"##,
        version = version,
        stats_js = stats_js,
        js = include_str!("ai_obs.js"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::ContentDb;

    #[test]
    fn ai_obs_page_has_nav_and_graphs() {
        let html = build_ai_obs_html(&serde_json::json!({}), "0.2.2", &ContentDb::default());
        assert!(html.contains("AI obs"));
        assert!(html.contains("/ops"));
        assert!(html.contains("ops-time-range"));
        assert!(html.contains("Babies named"));
        assert!(html.contains("Babies picked up"));
        assert!(html.contains("Death ages"));
        assert!(html.contains("Time spent"));
        assert!(html.contains("Objects crafted"));
        assert!(html.contains("NPC timings"));
        assert!(html.contains("id=\"ai-timing-charts\""));
        assert!(html.contains("walk to use"));
        assert!(html.contains("id=\"baby-charts\""));
        assert!(html.contains("id=\"spend-charts\""));
        assert!(html.contains("id=\"craft-charts\""));
        assert!(html.contains("name=\"viewport\""));
        assert!(html.contains("window.OLR_AI="));
        assert!(!html.contains("tick_work_us"));
        assert!(html.contains("<th>Object</th>"));
        assert!(html.contains("<th>Food</th>"));
        assert!(html.contains("obj-name"));
        let named = build_ai_obs_html(
            &serde_json::json!({
                "crafted_objects": [{"id": 31, "count": 2}],
                "food_eaten": [{"id": 31, "count": 1}]
            }),
            "0.2.2",
            &ContentDb::default(),
        );
        assert!(named.contains("\"31\""));
        assert!(named.contains("object 31") || named.contains("Gooseberry"));
    }
}
