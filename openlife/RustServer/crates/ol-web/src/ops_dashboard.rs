//! Ops dashboard HTML: duration units, human labels, charts.

use ol_metrics::{CounterSnapshot, OpsSample};

/// Format a duration stored as microseconds.
///
/// Units finer than milliseconds (`µs`) are used only when the value is below 1 ms.
pub fn format_ops_duration_us(us: u64) -> String {
    if us < 1_000 {
        format!("{us} µs")
    } else if us < 1_000_000 {
        if us % 1_000 == 0 {
            format!("{} ms", us / 1_000)
        } else if us % 100 == 0 {
            format!("{:.1} ms", us as f64 / 1_000.0)
        } else {
            format!("{:.2} ms", us as f64 / 1_000.0)
        }
    } else if us % 1_000_000 == 0 {
        format!("{} s", us / 1_000_000)
    } else {
        format!("{:.2} s", us as f64 / 1_000_000.0)
    }
}

/// Format a duration stored as whole milliseconds.
pub fn format_ops_duration_ms(ms: u64) -> String {
    format_ops_duration_us(ms.saturating_mul(1_000))
}

/// UTC `YYYY-MM-DD HH:MM:SS UTC` from Unix milliseconds. `0` → em dash.
pub fn format_unix_ms_utc(ms: u64) -> String {
    if ms == 0 {
        return "—".into();
    }
    let unix = (ms / 1_000) as i64;
    let days = unix.div_euclid(86_400);
    let sod = unix.rem_euclid(86_400) as u64;
    let h = sod / 3_600;
    let m = (sod % 3_600) / 60;
    let s = sod % 60;
    let (y, mo, d) = civil_from_unix_days(days);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}:{s:02} UTC")
}

/// Howard Hinnant civil-from-days, `days` since 1970-01-01.
fn civil_from_unix_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

fn card(label: &str, tip: &str, field: &str, value: &str, extra_html: &str) -> String {
    let extra = extra_html;
    format!(
        "<div class=\"card\"><strong>{label}<span class=\"tip\">{tip}</span></strong>\
         <br/><span data-field=\"{field}\">{value}</span>{extra}</div>"
    )
}

/// Full `/ops` document. Script is appended without `format!` so JSON `{` is safe.
pub fn build_ops_dashboard_html(
    s: &CounterSnapshot,
    samples: &[OpsSample],
    version: &str,
) -> String {
    let last = samples.last();
    let tick_us = last
        .map(|x| u64::from(x.tick_work_us))
        .unwrap_or(s.tick_work_ema_us);
    let intent_us = last
        .map(|x| u64::from(x.intent_ema_us))
        .unwrap_or(s.intent_ema_us);
    let lock_us = last
        .map(|x| u64::from(x.lock_wait_ema_us))
        .unwrap_or(s.lock_wait_ema_us);

    let mut cards = String::new();
    cards.push_str(&card(
        "Started",
        "When this server process started (UTC).",
        "started",
        &format_unix_ms_utc(s.start_unix_ms),
        "",
    ));
    cards.push_str(&card(
        "Tick",
        "How many simulation ticks have run since start.",
        "tick",
        &s.ticks.to_string(),
        "",
    ));
    cards.push_str(&card(
        "Skip Ticks",
        "Extra simulation steps when the server was behind real time. Catch-up advances, not dropped frames.",
        "skipTicks",
        &s.skip_ticks.to_string(),
        "",
    ));
    cards.push_str(&card(
        "Tick Work",
        "Time spent running one simulation tick (smoothed). Lower is healthier.",
        "tickWork",
        &format_ops_duration_us(tick_us),
        "",
    ));
    cards.push_str(&card(
        "Intent Time",
        "Time to apply one player or NPC action (smoothed average).",
        "intent",
        &format_ops_duration_us(intent_us),
        "",
    ));
    cards.push_str(&card(
        "Tick Average / P90",
        "Average tick work, and the slowest 10% (90th percentile).",
        "tickAvgP90",
        &format!(
            "{} / {}",
            format_ops_duration_us(s.tick_work_avg_us),
            format_ops_duration_us(s.tick_work_p90_us)
        ),
        "",
    ));
    cards.push_str(&card(
        "Tick Outliers / Normal",
        "How many recent ticks were unusually slow versus typical.",
        "tickOutliers",
        &format!("{} / {}", s.tick_work_outliers, s.tick_work_normal),
        "",
    ));
    cards.push_str(&card(
        "Intent Average / P90",
        "Average action-apply time, and the slowest 10%.",
        "intentAvgP90",
        &format!(
            "{} / {}",
            format_ops_duration_us(s.intent_avg_us),
            format_ops_duration_us(s.intent_p90_us)
        ),
        "",
    ));
    cards.push_str(&card(
        "Intent Outliers / Normal",
        "How many recent action applies were unusually slow versus typical.",
        "intentOutliers",
        &format!("{} / {}", s.intent_outliers, s.intent_normal),
        "",
    ));
    cards.push_str(&card(
        "Human Reply Average / P90",
        "Time to apply actions from human clients, average and slowest 10%.",
        "humanIntent",
        &format!(
            "{} / {}",
            format_ops_duration_us(s.human_intent_avg_us),
            format_ops_duration_us(s.human_intent_p90_us)
        ),
        &format!(
            "<br/><span class=\"muted\" data-field=\"humanIntentN\">n = {}</span>",
            s.human_intent_count
        ),
    ));
    cards.push_str(&card(
        "AI Intent Average / P90",
        "Time to apply NPC / AI actions, average and slowest 10%.",
        "aiIntent",
        &format!(
            "{} / {}",
            format_ops_duration_us(s.ai_intent_avg_us),
            format_ops_duration_us(s.ai_intent_p90_us)
        ),
        &format!(
            "<br/><span class=\"muted\" data-field=\"aiIntentN\">n = {}</span>",
            s.ai_intent_count
        ),
    ));
    cards.push_str(&card(
        "Lock Wait",
        "Time spent waiting for the world lock before a tick can run.",
        "lockWait",
        &format_ops_duration_us(lock_us),
        "",
    ));
    cards.push_str(&card(
        "Boot Total",
        "How long the server took to start (objects, transitions, and world load).",
        "bootTotal",
        &format_ops_duration_ms(s.boot_total_ms),
        "",
    ));
    cards.push_str(&card(
        "Boot Objects / Transitions / World",
        "Start-up time split: object data, transitions, then the world map.",
        "bootParts",
        &format!(
            "{} / {} / {}",
            format_ops_duration_ms(s.boot_objects_ms),
            format_ops_duration_ms(s.boot_transitions_ms),
            format_ops_duration_ms(s.boot_world_ms)
        ),
        "",
    ));
    cards.push_str(&card(
        "Samples",
        "How many timing samples are currently kept for the graphs.",
        "samples",
        &samples.len().to_string(),
        "",
    ));
    cards.push_str(&card(
        "AI Thinks",
        "How many NPC think steps have run since start.",
        "aiThinks",
        &s.ai_thinks.to_string(),
        "",
    ));

    let metrics = crate::metrics_json_from_snapshot(s, version, serde_json::json!({}));
    let series = crate::ops_series_json(samples);
    let metrics_js = serde_json::to_string(&metrics).unwrap_or_else(|_| "{}".into());
    let series_js = serde_json::to_string(&series).unwrap_or_else(|_| "{\"samples\":[],\"count\":0}".into());

    let mut html = String::with_capacity(24_000 + cards.len() + metrics_js.len() + series_js.len());
    html.push_str(OPS_HEAD);
    html.push_str(&cards);
    html.push_str(OPS_CHARTS);
    html.push_str("<script>window.OLR_OPS_METRICS=");
    html.push_str(&metrics_js);
    html.push_str(";window.OLR_OPS=");
    html.push_str(&series_js);
    html.push_str(";</script>\n<script>");
    html.push_str(include_str!("ops_dashboard.js"));
    html.push_str("</script>\n</body></html>");
    html
}

const OPS_HEAD: &str = r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><title>Ops — Open Life Reborn</title>
<style>
body{font-family:system-ui,sans-serif;background:#0b0f14;color:#e7eef7;margin:2rem;line-height:1.5}
a{color:#6ec6ff}
.cards{display:flex;flex-wrap:wrap;gap:1rem}
.card{background:#1a222e;padding:1rem 1.25rem;border-radius:8px;min-width:11rem;position:relative}
.card strong{position:relative;cursor:help;border-bottom:1px dotted #6ec6ff}
.card .tip,.chart-head h3 .tip{
  display:none;position:absolute;left:0;top:calc(100% + .35rem);z-index:5;
  background:#0e1620;color:#e7eef7;border:1px solid #2a3a4e;border-radius:6px;
  padding:.55rem .7rem;width:16rem;font-weight:400;font-size:.85rem;line-height:1.35;
  box-shadow:0 8px 24px rgba(0,0,0,.4)
}
.card:hover .tip,.chart-head h3:hover .tip{display:block}
.muted{color:#8b9bb0;font-size:.9rem}
.chart{margin:1.5rem 0;background:#121a24;border:1px solid #1e2a3a;border-radius:8px;padding:1rem}
.chart-head{display:flex;flex-wrap:wrap;align-items:center;justify-content:space-between;gap:.75rem;margin-bottom:.5rem}
.chart-head h3{margin:0;font-size:1.05rem;position:relative;cursor:help}
.chart-head label,.time-display{color:#8b9bb0;font-size:.9rem}
.time-display select,.chart-head select{background:#0b0f14;color:#e7eef7;border:1px solid #2a3a4e;border-radius:4px;padding:.25rem .4rem}
.chart-wrap{position:relative}
.chart-wrap svg{width:100%;height:180px;display:block;background:#0b0f14;border-radius:4px}
.chart-wrap .line{stroke:#6ec6ff;stroke-width:2;fill:none}
.chart-wrap .grid{stroke:#1e2a3a;stroke-width:1}
.chart-wrap .axis{stroke:#3a4a5e;stroke-width:1}
.chart-wrap .axis-label{fill:#8b9bb0;font-size:11px;font-family:system-ui,sans-serif}
.chart-wrap .cross{stroke:#e7eef7;stroke-width:1;stroke-dasharray:3 3;opacity:.7}
.chart-wrap .dot{fill:#6ec6ff;stroke:#0b0f14;stroke-width:1}
.hover-tip{
  position:absolute;pointer-events:none;background:#0e1620;color:#e7eef7;
  border:1px solid #2a3a4e;border-radius:6px;padding:.45rem .6rem;font-size:.85rem;
  line-height:1.35;min-width:10rem;box-shadow:0 8px 24px rgba(0,0,0,.45)
}
</style></head><body>
<p><a href="/">← Home</a></p>
<h1>Ops Dashboard</h1>
<p class="muted">Skip Ticks are catch-up advances when the server lags, not dropped wakes. Samples stay in memory about 5 seconds and flush about every 5 minutes. Graph time range is saved in this browser.</p>
<div class="cards">
"##;

const OPS_CHARTS: &str = r##"</div>
<p class="muted">Latency: average plus the slowest about 10% (P90), and outlier versus normal counts. Boot timings are recorded once at server start. Hover a graph for the value and time under the pointer.</p>
<p class="time-display"><label>Time range
<select id="ops-time-range" title="How much history the graphs show">
<option value="all">All samples</option>
<option value="30s">Last 30 seconds</option>
<option value="1m">Last 1 minute</option>
<option value="5m" selected>Last 5 minutes</option>
<option value="15m">Last 15 minutes</option>
<option value="1h">Last 1 hour</option>
</select></label></p>
<div id="ops-charts"></div>
<p class="muted"><a href="/api/ops/series">JSON series</a> · <a href="/api/metrics">JSON metrics</a></p>
"##;

#[cfg(test)]
mod tests {
    use super::*;
    use ol_metrics::Counters;

    #[test]
    fn duration_us_only_below_one_ms() {
        assert_eq!(format_ops_duration_us(0), "0 µs");
        assert_eq!(format_ops_duration_us(1), "1 µs");
        assert_eq!(format_ops_duration_us(999), "999 µs");
        assert_eq!(format_ops_duration_us(1_000), "1 ms");
        assert_eq!(format_ops_duration_us(1_500), "1.5 ms");
        assert_eq!(format_ops_duration_us(10_000), "10 ms");
        assert_eq!(format_ops_duration_us(1_000_000), "1 s");
        assert_eq!(format_ops_duration_us(1_500_000), "1.50 s");
    }

    #[test]
    fn duration_ms_promotes_to_seconds() {
        assert_eq!(format_ops_duration_ms(0), "0 µs");
        assert_eq!(format_ops_duration_ms(1), "1 ms");
        assert_eq!(format_ops_duration_ms(500), "500 ms");
        assert_eq!(format_ops_duration_ms(1_200), "1.20 s");
    }

    #[test]
    fn unix_epoch_and_one_second() {
        assert_eq!(format_unix_ms_utc(0), "—");
        assert_eq!(format_unix_ms_utc(1_000), "1970-01-01 00:00:01 UTC");
    }

    #[test]
    fn dashboard_labels_have_no_underscores() {
        let s = Counters::new().snapshot();
        let html = build_ops_dashboard_html(&s, &[], "0.1.0");
        assert!(html.contains("Skip Ticks"));
        assert!(html.contains("Tick Work"));
        assert!(html.contains("Intent Time"));
        assert!(html.contains("Lock Wait"));
        assert!(html.contains("Boot Total"));
        assert!(html.contains("class=\"tip\""));
        assert!(!html.contains("<strong>skip_ticks"));
        assert!(!html.contains(">tick_work_us<"));
        assert!(!html.contains("http-equiv=\"refresh\""));
        assert!(html.contains("localStorage"));
        assert!(html.contains("hover-tip"));
        assert!(html.contains("ops-time-range"));
        assert!(html.contains("Time range"));
        assert!(!html.contains("data-scale"));
    }
}
