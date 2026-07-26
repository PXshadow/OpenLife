//! Embedded HTTP: health, metrics, self-play world viewer (full map + zoom).

#![forbid(unsafe_code)]

mod map_api;

pub use map_api::{build_overview, build_window, overview_step, MapOverview, MapWindow};

use axum::body::Body;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Json, Response};
use axum::routing::get;
use axum::Router;
use std::path::{Path, PathBuf};
use ol_content::ContentDb;
use ol_metrics::{Counters, OpsSample};
use ol_sim::{
    AccountBookSnapshot, AccountView, AnimalSnapshot, AnimalView, EnvSnapshot, EnvView,
    LineageSnapshot, LineageView, PlayerSnapshot, PrestigeSnapshot, PrestigeView,
    TreasurySnapshot, TreasuryView, WeatherSnapshot, WeatherView,
};
use ol_world::World;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::info;

#[derive(Clone)]
pub struct WebState {
    pub counters: Arc<Counters>,
    pub version: &'static str,
    pub world: Arc<RwLock<World>>,
    pub content: Arc<ContentDb>,
    pub selfplay_log: Arc<RwLock<Vec<String>>>,
    pub selfplay_pos: Arc<RwLock<(i32, i32)>>,
    /// Live player snapshots from sim (conn_id → snapshot).
    pub player_views: Arc<RwLock<HashMap<u64, PlayerSnapshot>>>,
    pub env_view: EnvView,
    /// Optional weather snapshot (EnvView-style); defaults empty clear.
    pub weather_view: WeatherView,
    /// Soft accounts summary for `/api/accounts`.
    pub account_view: AccountView,
    /// Living percentile prestige for `/api/prestige`.
    pub prestige_view: PrestigeView,
    /// Lineage list for `/api/lineages` + `/lineage`.
    pub lineage_view: LineageView,
    /// Animal counts for `/api/animals`.
    pub animal_view: AnimalView,
    /// Village treasury coins for `/api/treasury`.
    pub treasury_view: TreasuryView,
    /// Ops series samples (sim-mirrored) for `/ops` + `/api/ops/series`.
    pub ops_series: Arc<RwLock<Vec<OpsSample>>>,
    /// NPC activity counters (craft/eat/stuck/deaths) updated by server.
    pub npc_stats: Arc<RwLock<serde_json::Value>>,
}

impl WebState {
    /// Fill optional view arcs with empty defaults (callers can replace before serve).
    pub fn with_default_views(
        counters: Arc<Counters>,
        version: &'static str,
        world: Arc<RwLock<World>>,
        content: Arc<ContentDb>,
        selfplay_log: Arc<RwLock<Vec<String>>>,
        selfplay_pos: Arc<RwLock<(i32, i32)>>,
        player_views: Arc<RwLock<HashMap<u64, PlayerSnapshot>>>,
        env_view: EnvView,
    ) -> Self {
        Self {
            counters,
            version,
            world,
            content,
            selfplay_log,
            selfplay_pos,
            player_views,
            env_view,
            weather_view: Arc::new(RwLock::new(WeatherSnapshot::default())),
            account_view: Arc::new(RwLock::new(AccountBookSnapshot::default())),
            prestige_view: Arc::new(RwLock::new(PrestigeSnapshot::default())),
            lineage_view: Arc::new(RwLock::new(LineageSnapshot::default())),
            animal_view: Arc::new(RwLock::new(AnimalSnapshot::default())),
            treasury_view: Arc::new(RwLock::new(TreasurySnapshot::default())),
            ops_series: Arc::new(RwLock::new(Vec::new())),
            npc_stats: Arc::new(RwLock::new(serde_json::json!({}))),
        }
    }
}

/// Basename-only allowlist for public static images (no path traversal).
const ALLOWED_IMAGES: &[&str] = &[
    "OHOL-From-Dayemeg.png",
    "OHOL-Hard-Winter.png",
    "OLR-world-map.png",
];

pub fn router(state: WebState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/intro", get(intro_page))
        .route("/health", get(health))
        .route("/api/metrics", get(metrics))
        .route("/ops", get(ops_page))
        .route("/api/ops/series", get(ops_series_api))
        .route("/players", get(players_page))
        .route("/stats/food", get(stats_food_page))
        .route("/stats/accounts", get(stats_accounts_page))
        .route("/viewer", get(viewer_page))
        .route("/lineage", get(lineage_page))
        .route("/character", get(character_page))
        .route("/lineage/character/{id}", get(character_page_path))
        .route("/api/npc/stats", get(npc_stats_api))
        .route("/static/images/{name}", get(safe_static_image))
        .route("/static/faces/{name}", get(safe_face_image))
        .route("/api/world/summary", get(world_summary))
        .route("/api/world/overview", get(world_overview))
        .route("/api/world/view", get(world_view))
        .route("/api/players", get(players_api))
        .route("/api/selfplay", get(selfplay_status))
        .route("/api/environment", get(environment_api))
        .route("/api/weather", get(weather_api))
        .route("/api/accounts", get(accounts_api))
        .route("/api/prestige", get(prestige_api))
        .route("/api/lineages", get(lineages_api))
        .route("/api/animals", get(animals_api))
        .route("/api/treasury", get(treasury_api))
        .with_state(state)
}

/// Resolve `web/static/images/<name>` only if `name` is on the allowlist (no `..`).
fn resolve_allowed_image(name: &str) -> Option<PathBuf> {
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return None;
    }
    if !ALLOWED_IMAGES.iter().any(|a| *a == name) {
        return None;
    }
    // Prefer cwd-relative web/static, then next to the binary's parent.
    let candidates = [
        PathBuf::from("web/static/images").join(name),
        PathBuf::from("static/images").join(name),
    ];
    for p in candidates {
        if p.is_file() {
            // Canonicalize and ensure under images dir (defense in depth).
            if let (Ok(canon), Ok(root)) = (
                p.canonicalize(),
                Path::new("web/static/images")
                    .canonicalize()
                    .or_else(|_| Path::new("static/images").canonicalize()),
            ) {
                if canon.starts_with(&root) {
                    return Some(canon);
                }
            } else if p.extension().and_then(|e| e.to_str()) == Some("png") {
                return Some(p);
            }
        }
    }
    None
}

async fn safe_static_image(AxumPath(name): AxumPath<String>) -> Response {
    let Some(path) = resolve_allowed_image(&name) else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };
    serve_png_file(&path).await
}

/// Safe face sprite: only `face_<digits>_<digits>.png` under content faces dirs.
fn resolve_allowed_face(name: &str) -> Option<PathBuf> {
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return None;
    }
    // face_1007_30.png
    let stem = name.strip_suffix(".png")?;
    let rest = stem.strip_prefix("face_")?;
    let mut parts = rest.split('_');
    let a = parts.next()?;
    let b = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    if !a.chars().all(|c| c.is_ascii_digit()) || !b.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let candidates = [
        PathBuf::from("content/OneLifeData7/faces").join(name),
        PathBuf::from(r"C:\OhOl\OpenLife\OneLifeData7\faces").join(name),
        PathBuf::from(r"C:\OhOl\OpenLifeReborn\content\OneLifeData7\faces").join(name),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

async fn safe_face_image(AxumPath(name): AxumPath<String>) -> Response {
    let Some(path) = resolve_allowed_face(&name) else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };
    serve_png_file(&path).await
}

async fn serve_png_file(path: &Path) -> Response {
    match tokio::fs::read(path).await {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "image/png")
            .header(header::CACHE_CONTROL, "public, max-age=86400")
            .body(Body::from(bytes))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        Err(_) => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

async fn index() -> Html<&'static str> {
    Html(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><title>Open Life Reborn</title>
<style>
body{font-family:system-ui,sans-serif;background:#0b0f14;color:#e7eef7;margin:2rem;line-height:1.5;max-width:52rem}
a{color:#6ec6ff} code{background:#1a222e;padding:.1rem .3rem;border-radius:4px}
.cards{display:flex;flex-wrap:wrap;gap:1rem;margin:1rem 0}
.card{background:#1a222e;padding:1rem 1.25rem;border-radius:8px;min-width:10rem}
.hero{max-width:100%;height:auto;border-radius:8px;margin:1rem 0}
.muted{color:#8b9bb0}
</style></head>
<body>
<h1>Open Life Reborn</h1>
<p>Free multiplayer civilisation / survival on the base of
<a href="https://onehouronelife.com/">One Hour One Life</a>. Play with any OHOL-compatible client
(custom server, port <code>8005</code> by default).</p>
<img class="hero" src="/static/images/OHOL-From-Dayemeg.png" alt="Open Life Reborn banner" width="720"/>
<p class="muted">Local art only (no remote file access). Safe static allowlist under <code>/static/images/</code>.</p>
<div class="cards">
<div class="card"><a href="/intro"><strong>Intro</strong></a><br/>rules &amp; features</div>
<div class="card"><a href="/ops"><strong>Ops</strong></a><br/>timings &amp; boot</div>
<div class="card"><a href="/viewer"><strong>Viewer</strong></a><br/>map + self-play</div>
<div class="card"><a href="/players"><strong>Players</strong></a><br/>living bodies</div>
<div class="card"><a href="/lineage"><strong>Lineage</strong></a><br/>OLN1 families</div>
<div class="card"><a href="/stats/food"><strong>Food</strong></a><br/>vitals stats</div>
<div class="card"><a href="/stats/accounts"><strong>Accounts</strong></a><br/>OLA1 scores</div>
</div>
<ul>
<li><a href="/health">/health</a></li>
<li><a href="/api/metrics">/api/metrics</a> (boot + latency avg/p90/outliers)</li>
<li><a href="/api/ops/series">/api/ops/series</a></li>
<li><a href="/api/selfplay">/api/selfplay</a></li>
</ul>
</body></html>"#,
    )
}

async fn intro_page() -> Html<&'static str> {
    // r## so HTML anchors like href="#Rules" do not end the raw string.
    Html(
        r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><title>Welcome — Open Life Reborn</title>
<style>
body{font-family:system-ui,sans-serif;background:#000;color:#fff;max-width:800px;margin:auto;padding:1rem;line-height:1.5}
a{color:#6ec6ff} img{max-width:100%;height:auto;border-radius:6px}
table{background:#222;color:#fff;width:100%;border-collapse:collapse}
td,th{padding:.4rem .6rem;border:1px solid #444}
.muted{color:#aaa;font-size:.9rem}
</style></head>
<body>
<p><a href="/">← home</a> · <a href="/viewer">viewer</a> · <a href="/ops">ops</a></p>
<h1>Welcome to Open Life Reborn</h1>
<p>Open Life Reborn is a free roleplay multiplayer civilisation building and survival game
on the base of <a href="https://onehouronelife.com/">One Hour One Life</a>.
You can play with any One Hour One Life client by entering this host as a custom server
(default game port <strong>8005</strong>).</p>
<p class="muted">Community project. Linked external software is at your own risk.</p>
<p>
<a href="#Rules">Rules</a> ·
<a href="#Features">Features</a> ·
<a href="#Statistics">Statistics</a> ·
<a href="/viewer">Live map</a>
</p>
<center>
<img src="/static/images/OHOL-From-Dayemeg.png" alt="Open Life Reborn banner" width="720"/>
<p class="muted">Banner art served from local allowlist only.</p>
</center>
<img src="/static/images/OLR-world-map.png" alt="World map" width="480"/>
<img src="/static/images/OHOL-Hard-Winter.png" alt="Hard winter" width="480"/>
<h2 id="Rules">By playing you accept these rules</h2>
<ol>
<li>Have fun and make sure fellow players have fun.</li>
<li>Dive into the world and try to roleplay in a medieval fantasy setting.</li>
<li>If you are born noble, be a noble.</li>
<li>Antagonist play: coordinate with moderators first.</li>
<li>What happens in game happens in game — but remember rule 1.</li>
</ol>
<h2 id="Features">Features</h2>
<ul>
<li>Round map with rivers, bridges, mountains, oceans</li>
<li>Seasons, temperature, prestige, combat, currency (subset in Rust server)</li>
<li>Server-side AI NPCs with craft valuation (tools + food priority)</li>
<li>Live web <a href="/viewer">viewer</a> and <a href="/ops">ops</a> dashboards</li>
</ul>
<h2 id="Statistics">Statistics</h2>
<p>Live server metrics: <a href="/api/metrics">/api/metrics</a> ·
players: <a href="/players">/players</a> ·
food: <a href="/stats/food">/stats/food</a> ·
accounts: <a href="/stats/accounts">/stats/accounts</a></p>
<p class="muted">This page does not expose arbitrary filesystem paths — only allowlisted images under <code>/static/images/</code>.</p>
</body></html>"##,
    )
}

async fn health(State(st): State<WebState>) -> impl IntoResponse {
    let h = ol_metrics::Health::from_counters(&st.counters, st.version);
    (
        StatusCode::OK,
        [("content-type", "application/json")],
        h.to_json_line(),
    )
}

async fn metrics(State(st): State<WebState>) -> Json<serde_json::Value> {
    let s = st.counters.snapshot();
    let (w, h, chunks, helpers) = {
        let world = st.world.read().unwrap();
        (
            world.width_tiles,
            world.height_tiles,
            world.resident_chunk_count(),
            world.helper_count(),
        )
    };
    Json(metrics_json_from_snapshot(
        &s,
        st.version,
        serde_json::json!({ "width": w, "height": h, "chunks": chunks, "helpers": helpers }),
    ))
}

async fn ops_series_api(State(st): State<WebState>) -> Json<serde_json::Value> {
    let samples = st.ops_series.read().unwrap().clone();
    Json(ops_series_json(&samples))
}

async fn ops_page(State(st): State<WebState>) -> Html<String> {
    let s = st.counters.snapshot();
    let samples = st.ops_series.read().unwrap().clone();
    let n = samples.len();
    let last = samples.last();
    let tick_us = last.map(|x| x.tick_work_us).unwrap_or(s.tick_work_ema_us as u32);
    let intent_us = last.map(|x| x.intent_ema_us).unwrap_or(s.intent_ema_us as u32);
    let lock_us = last.map(|x| x.lock_wait_ema_us).unwrap_or(s.lock_wait_ema_us as u32);
    // Simple SVG sparklines from last up to 60 samples.
    let chart = |field: &str, vals: &[u32]| -> String {
        if vals.is_empty() {
            return format!("<p class=\"muted\">no {field} samples yet</p>");
        }
        let max = vals.iter().copied().max().unwrap_or(1).max(1);
        let w = 480i32;
        let h = 80i32;
        let mut pts = String::new();
        for (i, v) in vals.iter().enumerate() {
            let x = if vals.len() > 1 {
                (i as f32 / (vals.len() - 1) as f32) * (w as f32)
            } else {
                0.0
            };
            let y = h as f32 - (*v as f32 / max as f32) * (h as f32 - 4.0) - 2.0;
            pts.push_str(&format!("{x:.1},{y:.1} "));
        }
        format!(
            "<div class=\"chart\"><h3>{field}</h3><svg width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">\
             <polyline fill=\"none\" stroke=\"#6ec6ff\" stroke-width=\"2\" points=\"{pts}\"/></svg>\
             <p class=\"muted\">max={max} n={}</p></div>",
            vals.len()
        )
    };
    let tick_vals: Vec<u32> = samples.iter().rev().take(60).map(|x| x.tick_work_us).rev().collect();
    let intent_vals: Vec<u32> = samples.iter().rev().take(60).map(|x| x.intent_ema_us).rev().collect();
    let skip_vals: Vec<u32> = samples
        .iter()
        .rev()
        .take(60)
        .map(|x| (x.skip_ticks.min(u32::MAX as u64)) as u32)
        .rev()
        .collect();
    let lock_vals: Vec<u32> = samples.iter().rev().take(60).map(|x| x.lock_wait_ema_us).rev().collect();
    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><title>Ops — Open Life Reborn</title>
<meta http-equiv="refresh" content="5"/>
<style>
body{{font-family:system-ui,sans-serif;background:#0b0f14;color:#e7eef7;margin:2rem;line-height:1.5}}
a{{color:#6ec6ff}} .cards{{display:flex;flex-wrap:wrap;gap:1rem}}
.card{{background:#1a222e;padding:1rem 1.25rem;border-radius:8px;min-width:9rem}}
.muted{{color:#8b9bb0;font-size:.9rem}} .chart{{margin:1.25rem 0}}
</style></head><body>
<p><a href="/">← home</a></p>
<h1>Ops dashboard</h1>
<p class="muted">skip_ticks = Haxe catch-up advances (not dropped wakes). Samples in RAM ≈5s; flush ~5min.</p>
<div class="cards">
<div class="card"><strong>Start</strong><br/>{start} ms unix</div>
<div class="card"><strong>Tick</strong><br/>{tick}</div>
<div class="card"><strong>skip_ticks</strong><br/>{skip}</div>
<div class="card"><strong>tick work EMA</strong><br/>{tick_us} µs</div>
<div class="card"><strong>intent EMA</strong><br/>{intent_us} µs</div>
<div class="card"><strong>tick avg / p90</strong><br/>{tavg} / {tp90} µs</div>
<div class="card"><strong>tick outliers / normal</strong><br/>{tout} / {tnorm}</div>
<div class="card"><strong>intent avg / p90</strong><br/>{iavg} / {ip90} µs</div>
<div class="card"><strong>intent outliers / normal</strong><br/>{iout} / {inorm}</div>
<div class="card"><strong>human reply avg / p90</strong><br/>{havg} / {hp90} µs<br/><span class="muted">n={hcnt}</span></div>
<div class="card"><strong>AI intent avg / p90</strong><br/>{aavg} / {ap90} µs<br/><span class="muted">n={acnt}</span></div>
<div class="card"><strong>lock wait EMA</strong><br/>{lock_us} µs</div>
<div class="card"><strong>boot total</strong><br/>{boot} ms</div>
<div class="card"><strong>boot objects/trans/world</strong><br/>{bo}/{bt}/{bw} ms</div>
<div class="card"><strong>samples</strong><br/>{n}</div>
<div class="card"><strong>AI thinks</strong><br/>{ai}</div>
</div>
<p class="muted">Latency: average + worst ~10% (p90) + outlier vs normal counts. Boot timings recorded once at server start.</p>
{c1}{c2}{c3}{c4}
<p class="muted"><a href="/api/ops/series">JSON series</a> · <a href="/api/metrics">/api/metrics</a></p>
</body></html>"#,
        start = s.start_unix_ms,
        tick = s.ticks,
        skip = s.skip_ticks,
        tick_us = tick_us,
        intent_us = intent_us,
        lock_us = lock_us,
        tavg = s.tick_work_avg_us,
        tp90 = s.tick_work_p90_us,
        tout = s.tick_work_outliers,
        tnorm = s.tick_work_normal,
        iavg = s.intent_avg_us,
        ip90 = s.intent_p90_us,
        iout = s.intent_outliers,
        inorm = s.intent_normal,
        havg = s.human_intent_avg_us,
        hp90 = s.human_intent_p90_us,
        hcnt = s.human_intent_count,
        aavg = s.ai_intent_avg_us,
        ap90 = s.ai_intent_p90_us,
        acnt = s.ai_intent_count,
        boot = s.boot_total_ms,
        bo = s.boot_objects_ms,
        bt = s.boot_transitions_ms,
        bw = s.boot_world_ms,
        n = n,
        ai = s.ai_thinks,
        c1 = chart("tick_work_us", &tick_vals),
        c2 = chart("intent_ema_us", &intent_vals),
        c3 = chart("skip_ticks (cumulative)", &skip_vals),
        c4 = chart("lock_wait_ema_us", &lock_vals),
    ))
}

async fn players_page(State(st): State<WebState>) -> Html<String> {
    let views = st.player_views.read().unwrap();
    let mut rows = String::new();
    let mut list: Vec<_> = views.values().cloned().collect();
    list.sort_by_key(|p| p.p_id);
    for p in list {
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>({},{})</td><td>{}</td><td>{:.1}/{:.0}</td><td>{:.1}</td><td>{}</td><td>{}</td></tr>",
            p.p_id,
            html_escape(&p.email),
            p.x,
            p.y,
            p.held_id,
            p.food,
            p.food_max,
            p.age,
            if p.moving { "yes" } else { "no" },
            if p.deleted { "dead" } else { "live" },
        ));
    }
    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><title>Players</title>
<meta http-equiv="refresh" content="3"/>
<style>
body{{font-family:system-ui,sans-serif;background:#0b0f14;color:#e7eef7;margin:2rem}}
a{{color:#6ec6ff}} table{{border-collapse:collapse;width:100%}}
th,td{{border:1px solid #2a3544;padding:.4rem .6rem;text-align:left}}
th{{background:#1a222e}}
</style></head><body>
<p><a href="/">← home</a></p>
<h1>Players</h1>
<table><thead><tr><th>p_id</th><th>email</th><th>pos</th><th>held</th><th>food</th><th>age</th><th>moving</th><th>status</th></tr></thead>
<tbody>{rows}</tbody></table>
</body></html>"#
    ))
}

async fn stats_food_page(State(st): State<WebState>) -> Html<String> {
    let views = st.player_views.read().unwrap();
    let live: Vec<_> = views.values().filter(|p| !p.deleted).collect();
    let n = live.len();
    let avg_food = if n == 0 {
        0.0
    } else {
        live.iter().map(|p| p.food as f64).sum::<f64>() / n as f64
    };
    let hungry = live.iter().filter(|p| p.food < 3.0).count();
    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><title>Food stats</title>
<style>body{{font-family:system-ui,sans-serif;background:#0b0f14;color:#e7eef7;margin:2rem}}a{{color:#6ec6ff}}</style>
</head><body>
<p><a href="/">← home</a></p>
<h1>Food / vitals</h1>
<ul>
<li>Living players: {n}</li>
<li>Average food: {avg_food:.2}</li>
<li>Hungry (food&lt;3): {hungry}</li>
</ul>
<p><a href="/api/players">JSON players</a></p>
</body></html>"#
    ))
}

async fn stats_accounts_page(State(st): State<WebState>) -> Html<String> {
    let snap = st
        .account_view
        .read()
        .map(|g| g.clone())
        .unwrap_or_default();
    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><title>Accounts</title>
<style>body{{font-family:system-ui,sans-serif;background:#0b0f14;color:#e7eef7;margin:2rem}}a{{color:#6ec6ff}}</style>
</head><body>
<p><a href="/">← home</a></p>
<h1>Accounts (OLA1)</h1>
<pre>{:?}</pre>
<p><a href="/api/accounts">JSON</a></p>
</body></html>"#,
        snap
    ))
}



async fn world_summary(State(st): State<WebState>) -> Json<serde_json::Value> {
    let world = st.world.read().unwrap();
    let mut non_empty = 0u32;
    for y in -16..16 {
        for x in -16..16 {
            if world.get_object(x, y) != 0 {
                non_empty += 1;
            }
        }
    }
    Json(serde_json::json!({
        "width": world.width_tiles,
        "height": world.height_tiles,
        "wrap": world.wrap,
        "format_version": world.format_version,
        "chunks": world.resident_chunk_count(),
        "helpers": world.helper_count(),
        "sample_nonempty_32x32": non_empty,
    }))
}

#[derive(Debug, Deserialize)]
struct OverviewQuery {
    #[serde(default = "default_max_side")]
    max_side: i32,
}

fn default_max_side() -> i32 {
    256
}

async fn world_overview(
    State(st): State<WebState>,
    Query(q): Query<OverviewQuery>,
) -> Json<MapOverview> {
    let world = st.world.read().unwrap();
    Json(build_overview(&*world, q.max_side.clamp(16, 512)))
}

#[derive(Debug, Deserialize)]
struct ViewQuery {
    #[serde(default)]
    x: i32,
    #[serde(default)]
    y: i32,
    #[serde(default = "default_w")]
    w: i32,
    #[serde(default = "default_h")]
    h: i32,
}

fn default_w() -> i32 {
    64
}
fn default_h() -> i32 {
    64
}

async fn world_view(
    State(st): State<WebState>,
    Query(q): Query<ViewQuery>,
) -> Json<serde_json::Value> {
    let world = st.world.read().unwrap();
    let win = build_window(&*world, &st.content, q.x, q.y, q.w, q.h);
    let (ax, ay) = *st.selfplay_pos.read().unwrap();
    Json(serde_json::json!({
        "origin_x": win.origin_x,
        "origin_y": win.origin_y,
        "w": win.w,
        "h": win.h,
        "biomes": win.biomes,
        "floors": win.floors,
        "objects": win.objects,
        "uses": win.uses,
        "names": win.names,
        "agent": { "x": ax, "y": ay },
    }))
}

async fn players_api(State(st): State<WebState>) -> Json<serde_json::Value> {
    let views = st.player_views.read().unwrap();
    let mut players: Vec<PlayerSnapshot> = views.values().cloned().collect();
    players.sort_by_key(|p| p.p_id);
    Json(serde_json::json!({ "players": players }))
}

async fn environment_api(State(st): State<WebState>) -> Json<EnvSnapshot> {
    let snap = st
        .env_view
        .read()
        .map(|g| g.clone())
        .unwrap_or_default();
    Json(snap)
}

async fn weather_api(State(st): State<WebState>) -> Json<WeatherSnapshot> {
    let snap = st
        .weather_view
        .read()
        .map(|g| g.clone())
        .unwrap_or_default();
    Json(snap)
}

async fn accounts_api(State(st): State<WebState>) -> Json<AccountBookSnapshot> {
    let snap = st
        .account_view
        .read()
        .map(|g| g.clone())
        .unwrap_or_default();
    Json(snap)
}

async fn prestige_api(State(st): State<WebState>) -> Json<PrestigeSnapshot> {
    let snap = st
        .prestige_view
        .read()
        .map(|g| g.clone())
        .unwrap_or_default();
    Json(snap)
}

async fn lineages_api(State(st): State<WebState>) -> Json<LineageSnapshot> {
    let snap = st
        .lineage_view
        .read()
        .map(|g| g.clone())
        .unwrap_or_default();
    Json(snap)
}

async fn animals_api(State(st): State<WebState>) -> Json<AnimalSnapshot> {
    let snap = st
        .animal_view
        .read()
        .map(|g| g.clone())
        .unwrap_or_default();
    Json(snap)
}

async fn treasury_api(State(st): State<WebState>) -> Json<TreasurySnapshot> {
    let snap = st
        .treasury_view
        .read()
        .map(|g| g.clone())
        .unwrap_or_default();
    Json(snap)
}

async fn selfplay_status(State(st): State<WebState>) -> Json<serde_json::Value> {
    let log = st.selfplay_log.read().unwrap().clone();
    let (x, y) = *st.selfplay_pos.read().unwrap();
    // Prefer live snapshot if present
    let snap = st
        .player_views
        .read()
        .unwrap()
        .values()
        .find(|p| p.email.contains("selfplay"))
        .cloned();
    Json(serde_json::json!({
        "x": snap.as_ref().map(|p| p.x).unwrap_or(x),
        "y": snap.as_ref().map(|p| p.y).unwrap_or(y),
        "held_id": snap.as_ref().map(|p| p.held_id).unwrap_or(0),
        "food": snap.as_ref().map(|p| p.food).unwrap_or(0.0),
        "food_max": snap.as_ref().map(|p| p.food_max).unwrap_or(20.0),
        "age": snap.as_ref().map(|p| p.age).unwrap_or(0.0),
        "log": log,
    }))
}

async fn viewer_page() -> Html<&'static str> {
    Html(include_str!("viewer.html"))
}

async fn npc_stats_api(State(st): State<WebState>) -> Json<serde_json::Value> {
    let v = st
        .npc_stats
        .read()
        .map(|g| g.clone())
        .unwrap_or_else(|_| serde_json::json!({}));
    Json(v)
}

/// Query `?id=` character page (OHOL lineage-style).
#[derive(Debug, Deserialize)]
struct CharacterQuery {
    id: Option<i32>,
}

async fn character_page(
    State(st): State<WebState>,
    Query(q): Query<CharacterQuery>,
) -> Html<String> {
    let id = q.id.unwrap_or(0);
    render_character_page(&st, id)
}

async fn character_page_path(
    State(st): State<WebState>,
    AxumPath(id): AxumPath<i32>,
) -> Html<String> {
    render_character_page(&st, id)
}

fn render_character_page(st: &WebState, id: i32) -> Html<String> {
    let snap = st
        .lineage_view
        .read()
        .map(|g| g.clone())
        .unwrap_or_default();
    let entry = snap.lineages.iter().find(|e| e.id == id).cloned();
    // Living player with this p_id for age/food if online.
    let live = st
        .player_views
        .read()
        .ok()
        .and_then(|g| g.values().find(|p| p.p_id == id && !p.deleted).cloned());

    let Some(e) = entry else {
        return Html(format!(
            r#"<!DOCTYPE html><html><head><meta charset="utf-8"/><title>Character {id}</title>
<style>body{{font-family:system-ui;background:#111;color:#eee;margin:2rem}}a{{color:#6ec6ff}}</style></head>
<body><p><a href="/lineage">← lineages</a></p>
<h1>Character #{id}</h1>
<p>No lineage record for this id in the live snapshot.</p>
<p class="muted">Try <a href="/lineage">/lineage</a> for the list.</p>
</body></html>"#
        ));
    };

    // Ancestor chain (mother side).
    let mut chain = Vec::new();
    let mut cur = Some(e.id);
    let mut guard = 0;
    while let Some(cid) = cur {
        if guard > 32 {
            break;
        }
        guard += 1;
        if let Some(n) = snap.lineages.iter().find(|x| x.id == cid) {
            chain.push(n.clone());
            cur = n.mother_id;
        } else {
            break;
        }
    }

    // Children.
    let children: Vec<_> = snap
        .lineages
        .iter()
        .filter(|c| c.mother_id == Some(e.id) || c.father_id == Some(e.id))
        .cloned()
        .collect();

    let face = format!("/static/faces/face_{}_30.png", (e.id.unsigned_abs() % 50) + 1000);
    // Prefer a real face file if allowlisted pattern works; fallback generic.
    let age_s = live
        .as_ref()
        .map(|p| format!("{:.1}", p.age))
        .unwrap_or_else(|| "—".into());
    let food_s = live
        .as_ref()
        .map(|p| format!("{:.1}/{:.0}", p.food, p.food_max))
        .unwrap_or_else(|| "offline".into());
    let pos_s = live
        .as_ref()
        .map(|p| format!("{},{}", p.x, p.y))
        .unwrap_or_else(|| "—".into());

    let mut chain_html = String::new();
    for (i, n) in chain.iter().enumerate() {
        let link = format!("/character?id={}", n.id);
        chain_html.push_str(&format!(
            "<li>gen {} — <a href=\"{link}\">#{} {}</a> (prestige {:.1})</li>",
            n.generation,
            n.id,
            html_escape(&n.name),
            n.prestige
        ));
        if i == 0 {
            chain_html.push_str(" <!-- self -->");
        }
    }
    let mut kids_html = String::new();
    if children.is_empty() {
        kids_html.push_str("<li class=\"muted\">No children in snapshot</li>");
    } else {
        for c in &children {
            kids_html.push_str(&format!(
                "<li><a href=\"/character?id={}\">#{} {}</a> gen {}</li>",
                c.id,
                c.id,
                html_escape(&c.name),
                c.generation
            ));
        }
    }
    let mother = e
        .mother_id
        .map(|m| format!("<a href=\"/character?id={m}\">#{m}</a>"))
        .unwrap_or_else(|| "Eve/Adam".into());
    let father = e
        .father_id
        .map(|f| format!("<a href=\"/character?id={f}\">#{f}</a>"))
        .unwrap_or_else(|| "—".into());

    Html(format!(
        r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/>
<title>Character #{id} — {name}</title>
<style>
body{{font-family:Georgia,serif;background:#1a1410;color:#f5e6d3;margin:0;padding:1.5rem;line-height:1.45}}
a{{color:#c9a227}} .card{{background:#2a2118;border:1px solid #4a3b2a;border-radius:8px;padding:1rem 1.25rem;max-width:40rem;margin:1rem 0}}
.face{{width:96px;height:96px;image-rendering:pixelated;background:#000;border:2px solid #4a3b2a}}
.muted{{color:#a89070;font-size:.9rem}} h1{{font-weight:normal;color:#f0d78c}}
ul{{padding-left:1.2rem}}
</style></head>
<body>
<p class="muted"><a href="/">home</a> · <a href="/lineage">lineages</a> · <a href="/viewer">viewer</a>
 · OHOL-style character page</p>
<div class="card" style="display:flex;gap:1.25rem;align-items:flex-start">
<img class="face" src="{face}" alt="face" onerror="this.style.display='none'"/>
<div>
<h1>{name}</h1>
<p><strong>id</strong> {id} · <strong>generation</strong> {gen}<br/>
<strong>mother</strong> {mother} · <strong>father</strong> {father}<br/>
<strong>prestige</strong> {prestige:.1} ({pclass})<br/>
<strong>age</strong> {age} · <strong>food</strong> {food} · <strong>pos</strong> {pos}
</p>
</div>
</div>
<div class="card">
<h2>Matrilineal chain</h2>
<ul>{chain}</ul>
</div>
<div class="card">
<h2>Children</h2>
<ul>{kids}</ul>
</div>
<p class="muted">Inspired by
<a href="http://lineage.onehouronelife.com/">OHOL lineage</a> character pages.
Face sprites from free OneLifeData7 <code>faces/</code> when present (safe basename only).</p>
</body></html>"##,
        id = e.id,
        name = html_escape(&e.name),
        gen = e.generation,
        mother = mother,
        father = father,
        prestige = e.prestige,
        pclass = html_escape(&e.prestige_class),
        age = age_s,
        food = food_s,
        pos = pos_s,
        face = face,
        chain = chain_html,
        kids = kids_html,
    ))
}

async fn lineage_page(State(st): State<WebState>) -> Html<String> {
    let snap = st
        .lineage_view
        .read()
        .map(|g| g.clone())
        .unwrap_or_default();

    let mut rows = String::new();
    if snap.lineages.is_empty() {
        rows.push_str(
            "<tr><td colspan=\"6\" class=\"muted\">No lineages in live snapshot yet. \
             On disk: <code>SaveFiles/lineages_v1.bin</code> (OLN1).</td></tr>",
        );
    } else {
        for e in &snap.lineages {
            let mother = e
                .mother_id
                .map(|m| m.to_string())
                .unwrap_or_else(|| "—".into());
            rows.push_str(&format!(
                "<tr><td><a href=\"/character?id={id}\">{id}</a></td><td>{name}</td><td>{mother}</td><td>{gen}</td><td>{prestige:.1}</td><td>{pclass}</td></tr>",
                id = e.id,
                name = html_escape(&e.name),
                mother = mother,
                gen = e.generation,
                prestige = e.prestige,
                pclass = html_escape(&e.prestige_class)
            ));
        }
    }

    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"/><title>Lineages — Open Life Reborn</title>
<style>
body{{font-family:system-ui,sans-serif;background:#0b0f14;color:#e7eef7;margin:2rem;line-height:1.5}}
a{{color:#6ec6ff}} code{{background:#1a222e;padding:.1rem .3rem;border-radius:4px}}
table{{border-collapse:collapse;width:100%;max-width:56rem;margin-top:1rem}}
th,td{{border:1px solid #1e2a3a;padding:.4rem .55rem;text-align:left}}
th{{background:#121a24;color:#6ec6ff}}
.muted{{color:#8aa0b5}}
.box{{background:#121a24;border:1px solid #1e2a3a;border-radius:8px;padding:1rem;max-width:56rem;margin:1rem 0}}
</style></head>
<body>
<p><a href="/">home</a> · <a href="/viewer">viewer</a> · <a href="/api/lineages">JSON</a> · <a href="/api/npc/stats">NPC stats</a></p>
<h1>Lineages</h1>
<p class="muted">Live snapshot (count={count}). Click an id for an OHOL-style <strong>character page</strong>. Format <code>{format}</code>.</p>
<div class="box">
<strong>OLN1 save file</strong>
<p class="muted">Binary lineage index (no SQL). Default path:</p>
<p><code>SaveFiles/lineages_v1.bin</code></p>
<p class="muted">Character pages: <code>/character?id=N</code> or <code>/lineage/character/N</code>.</p>
</div>
<table>
<thead><tr><th>id</th><th>name</th><th>mother</th><th>gen</th><th>prestige</th><th>class</th></tr></thead>
<tbody>
{rows}
</tbody>
</table>
</body></html>"#,
        count = snap.count,
        format = if snap.format.is_empty() {
            "OLN1"
        } else {
            &snap.format
        },
        rows = rows,
    ))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}


/// Pure metrics JSON body (testable without axum).
pub fn metrics_json_from_snapshot(
    s: &ol_metrics::CounterSnapshot,
    version: &str,
    world: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "ticks": s.ticks,
        "intents_applied": s.intents_applied,
        "skip_ticks": s.skip_ticks,
        "skip_ticks_meaning": "catch_up_advances",
        "connections": s.connections,
        "logins": s.logins,
        "deaths": s.deaths,
        "crafts": s.crafts,
        "autosaves": s.autosaves,
        "start_unix_ms": s.start_unix_ms,
        "tick_work_ema_us": s.tick_work_ema_us,
        "intent_ema_us": s.intent_ema_us,
        "lock_wait_ema_us": s.lock_wait_ema_us,
        "selfplay_unstick_total": s.selfplay_unstick_total,
        "ai_sim_time_ms": s.ai_sim_time_ms,
        "ai_cpu_us": s.ai_cpu_us,
        "ai_thinks": s.ai_thinks,
        "boot": {
            "objects_ms": s.boot_objects_ms,
            "transitions_ms": s.boot_transitions_ms,
            "world_ms": s.boot_world_ms,
            "lineages_ms": s.boot_lineages_ms,
            "accounts_ms": s.boot_accounts_ms,
            "total_ms": s.boot_total_ms,
        },
        "latency": {
            "tick_avg_us": s.tick_work_avg_us,
            "tick_p90_us": s.tick_work_p90_us,
            "tick_outliers": s.tick_work_outliers,
            "tick_normal": s.tick_work_normal,
            "intent_avg_us": s.intent_avg_us,
            "intent_p90_us": s.intent_p90_us,
            "intent_outliers": s.intent_outliers,
            "intent_normal": s.intent_normal,
            "human_intent_avg_us": s.human_intent_avg_us,
            "human_intent_p90_us": s.human_intent_p90_us,
            "human_intent_count": s.human_intent_count,
            "ai_intent_avg_us": s.ai_intent_avg_us,
            "ai_intent_p90_us": s.ai_intent_p90_us,
            "ai_intent_count": s.ai_intent_count,
        },
        "version": version,
        "world": world,
    })
}

pub fn ops_series_json(samples: &[ol_metrics::OpsSample]) -> serde_json::Value {
    let series: Vec<serde_json::Value> = samples
        .iter()
        .map(|s| {
            serde_json::json!({
                "wall_unix_ms": s.wall_unix_ms,
                "tick": s.tick,
                "skip_ticks": s.skip_ticks,
                "tick_work_us": s.tick_work_us,
                "intent_ema_us": s.intent_ema_us,
                "lock_wait_ema_us": s.lock_wait_ema_us,
                "intents": s.intents,
                "connections": s.connections,
                "tick_avg_us": s.tick_avg_us,
                "tick_p90_us": s.tick_p90_us,
                "tick_outliers": s.tick_outliers,
                "tick_normal": s.tick_normal,
                "intent_avg_us": s.intent_avg_us,
                "intent_p90_us": s.intent_p90_us,
                "intent_outliers": s.intent_outliers,
                "intent_normal": s.intent_normal,
                "boot_total_ms": s.boot_total_ms,
            })
        })
        .collect();
    serde_json::json!({ "samples": series, "count": series.len() })
}

#[cfg(test)]
mod metrics_api_tests {
    use super::*;
    use ol_metrics::Counters;
    use std::sync::atomic::Ordering;

    #[test]
    fn metrics_includes_skip_ticks_meaning() {
        let c = Counters::new();
        c.skip_ticks.store(3, Ordering::Relaxed);
        let v = metrics_json_from_snapshot(&c.snapshot(), "0.1.0", serde_json::json!({}));
        assert_eq!(v["skip_ticks_meaning"], "catch_up_advances");
        assert_eq!(v["skip_ticks"], 3);
    }

    #[test]
    fn ops_series_shape() {
        let samples = vec![ol_metrics::OpsSample {
            wall_unix_ms: 1,
            tick: 2,
            skip_ticks: 0,
            tick_work_us: 10,
            intent_ema_us: 5,
            lock_wait_ema_us: 0,
            intents: 1,
            connections: 0,
            tick_avg_us: 10,
            tick_p90_us: 15,
            tick_outliers: 1,
            tick_normal: 9,
            intent_avg_us: 5,
            intent_p90_us: 8,
            intent_outliers: 1,
            intent_normal: 9,
            boot_total_ms: 1200,
        }];
        let v = ops_series_json(&samples);
        assert_eq!(v["count"], 1);
        assert_eq!(v["samples"][0]["tick"], 2);
        assert_eq!(v["samples"][0]["tick_p90_us"], 15);
        assert_eq!(v["samples"][0]["boot_total_ms"], 1200);
    }
}

pub async fn serve(bind: &str, state: WebState) -> Result<(), std::io::Error> {
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(bind).await?;
    info!(%bind, "web listening (viewer /viewer — full map + zoom; /lineage)");
    axum::serve(listener, app).await
}
