//! Open Life Reborn server entrypoint.
//!
//! Shared `RwLock<World>`: sim writes, net reads for MAP_CHUNK.
//! World bootstrapped from PNG + natural spawn, persisted as versioned OLW1 binary.
//! OutboundHub: sim pushes MX/PU/FX after mutations.
//! Self-play agent + web `/viewer` for live observation.
//!
//! ## Process lifetime (why the server used to “vanish”)
//! Main used to exit as soon as `tokio::signal::ctrl_c()` completed — including
//! when registering the handler **failed** (detached / no console) or when the
//! minimized console window was closed. That is a clean exit, not a Rust memory
//! crash. Rust prevents memory unsafety; it does **not** prevent panics, OOM, or
//! the OS killing / signalling a process.

mod selfplay;
mod npc_ai;
mod npc_activity;
mod world_boot;

use ol_config::ServerConfig;
use ol_content::{load_content, resolve_content_path, ContentDb};
use ol_metrics::Counters;
use ol_net::{run_game_listener, NetConfig, OutboundHub};
use ol_sim::{
    build_reverse_craft_graph_capped, load_accounts, load_lineages, run_sim_loop_with_views,
    save_accounts, save_lineages, AccountBook, AnimalSnapshot, AnimalWorld, PrestigeSnapshot,
    SocialState, TreasurySnapshot, TwinRegistry, WeatherSnapshot,
};
use ol_web::{serve as serve_web, WebState};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Touch this file (or `SAY !shutdown` as god) to stop a detached daemon cleanly.
const STOP_FLAG_REL: &str = "SaveFiles/stop.flag";
const HEARTBEAT_REL: &str = "SaveFiles/server.heartbeat";
const LOG_REL: &str = "SaveFiles/ol-server.log";

fn install_panic_hook(log_path: PathBuf) {
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!(
            "PANIC at {}: {}\n",
            chrono_like_now(),
            info
        );
        eprintln!("{msg}");
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            let _ = f.write_all(msg.as_bytes());
            let _ = f.flush();
        }
    }));
}

/// Lightweight timestamp without extra deps.
fn chrono_like_now() -> String {
    // Unix secs is enough for crash forensics.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix={secs}")
}

/// Detach from the parent console on Windows so closing a minimized console
/// window does not kill the server (CTRL_CLOSE_EVENT).
#[cfg(windows)]
fn try_detach_console() {
    unsafe {
        #[link(name = "kernel32")]
        extern "system" {
            fn FreeConsole() -> i32;
            fn GetConsoleWindow() -> *mut core::ffi::c_void;
        }
        // Only free if we actually own a console.
        if !GetConsoleWindow().is_null() {
            let _ = FreeConsole();
        }
    }
}

#[cfg(not(windows))]
fn try_detach_console() {}

fn stop_flag_path() -> PathBuf {
    PathBuf::from(STOP_FLAG_REL)
}

fn write_heartbeat(pid: u32, note: &str) {
    let line = format!(
        "pid={pid} {note} at {}\n",
        chrono_like_now()
    );
    let path = Path::new(HEARTBEAT_REL);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, line);
}

/// Block until operator stop: `!shutdown` flag, `SaveFiles/stop.flag`, or interactive Ctrl+C.
///
/// **Important:** if `ctrl_c` registration fails (daemon / no console), we must **not**
/// exit — that was a previous bug that made the server die immediately when detached.
async fn wait_for_shutdown(exit_flag: Arc<AtomicBool>) {
    let pid = std::process::id();
    let stop_path = stop_flag_path();
    // Clear stale stop flag from a previous run.
    let _ = std::fs::remove_file(&stop_path);

    info!(
        pid,
        stop = %stop_path.display(),
        "server running — stop with SAY !shutdown (god), delete process, or create {}",
        STOP_FLAG_REL
    );
    write_heartbeat(pid, "started");

    // Interactive Ctrl+C is optional; failure must not shut the server down.
    let mut ctrl_c_armed = true;
    loop {
        if exit_flag.load(Ordering::SeqCst) {
            info!("shutdown_exit flag set (!shutdown)");
            break;
        }
        if stop_path.exists() {
            // Leave stop.flag sticky until start-server.ps1 clears it. Deleting here
            // races the watchdog: it can see "no flag + no process" and WMI-restart.
            info!(
                path = %stop_path.display(),
                "stop.flag present — orderly exit (flag left sticky for watchdog)"
            );
            break;
        }

        if ctrl_c_armed {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(2)) => {
                    write_heartbeat(pid, "alive");
                }
                r = tokio::signal::ctrl_c() => {
                    match r {
                        Ok(()) => {
                            info!("ctrl-c received — orderly exit");
                            break;
                        }
                        Err(e) => {
                            warn!(
                                error = %e,
                                "ctrl_c unavailable (detached/no console) — staying up in daemon mode"
                            );
                            ctrl_c_armed = false;
                        }
                    }
                }
            }
        } else {
            tokio::time::sleep(Duration::from_secs(2)).await;
            write_heartbeat(pid, "alive-daemon");
        }
    }
    write_heartbeat(pid, "stopping");
}

fn init_logging(log_path: &Path) {
    let _ = std::fs::create_dir_all(
        log_path
            .parent()
            .unwrap_or_else(|| Path::new("SaveFiles")),
    );
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let stdout_layer = fmt::layer().with_writer(std::io::stderr);

    // Always mirror logs to SaveFiles/ol-server.log so deaths are diagnosable.
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path);
    match file {
        Ok(f) => {
            let file_layer = fmt::layer().with_ansi(false).with_writer(Mutex::new(f));
            let _ = tracing_subscriber::registry()
                .with(filter)
                .with(stdout_layer)
                .with(file_layer)
                .try_init();
        }
        Err(e) => {
            let _ = tracing_subscriber::registry()
                .with(filter)
                .with(stdout_layer)
                .try_init();
            eprintln!("could not open log file {}: {e}", log_path.display());
        }
    }
}

#[tokio::main]
async fn main() {
    let log_path = PathBuf::from(LOG_REL);
    install_panic_hook(log_path.clone());
    init_logging(&log_path);

    // Detach from console by default so closing a minimized window does not kill us.
    // Set OLR_INTERACTIVE=1 to keep console + Ctrl+C as primary stop.
    let interactive = std::env::var("OLR_INTERACTIVE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !interactive {
        try_detach_console();
    }

    info!(
        version = VERSION,
        pid = std::process::id(),
        log = %log_path.display(),
        interactive,
        "Open Life Reborn server process starting"
    );

    let config_path = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "server.toml".into()),
    );

    if !config_path.exists() {
        if let Err(e) = ServerConfig::write_default(&config_path) {
            error!(error = %e, "could not write default config");
        } else {
            info!(path = %config_path.display(), "wrote default server.toml");
        }
    }

    let cfg = match ServerConfig::load_or_default(&config_path) {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "config load failed; using defaults");
            ServerConfig::default()
        }
    };

    let counters = Arc::new(Counters::new());
    counters.mark_start_now();
    let restart_t0 = std::time::Instant::now();
    let (intent_tx, intent_rx) = tokio::sync::mpsc::channel(4096);
    let outbound = Arc::new(OutboundHub::new());

    let content_path = resolve_content_path(&cfg.content_path);
    let mut boot_objects_ms = 0u64;
    let mut boot_transitions_ms = 0u64;
    let content = match load_content(&content_path) {
        Ok(db) => {
            boot_objects_ms = db.load_objects_ms;
            boot_transitions_ms = db.load_transitions_ms;
            info!(
                objects = db.object_count(),
                transitions = db.transition_count,
                last_use = db.last_use_transition_count,
                biomes_spawn = db.biome_spawn.len(),
                version = db.data_version,
                objects_ms = db.load_objects_ms,
                transitions_ms = db.load_transitions_ms,
                path = %content_path.display(),
                "content ready"
            );
            Arc::new(db)
        }
        Err(e) => {
            warn!(error = %e, path = %content_path.display(), "content not loaded — empty ContentDb");
            Arc::new(ContentDb::default())
        }
    };

    // World-first: load OLW1 (fast path) or generate from PNG + natural objects.
    let world_t0 = std::time::Instant::now();
    let world = world_boot::bootstrap_world(&cfg, &content);
    let boot_world_ms = world_t0.elapsed().as_millis() as u64;
    info!(
        w = world.width_tiles,
        h = world.height_tiles,
        chunks = world.resident_chunk_count(),
        helpers = world.helper_count(),
        ticket_verify = cfg.verify_ohol_ticket,
        boot_ms = boot_world_ms,
        "shared world ready (sim write, net read for MC)"
    );
    // Human LOGIN bootstrap + sim spawn: grassland near map center (self-play band).
    let preferred_spawn = {
        use ol_sim::find_playable_spawn;
        let (sx, sy) = find_playable_spawn(&world, (0, 0));
        info!(sx, sy, "preferred human spawn (near center / AI, non-mountain)");
        Arc::new(RwLock::new((sx, sy)))
    };
    let shared_world = Arc::new(RwLock::new(world));

    // Lineages: load OLN1 after content/world boot (no SQL).
    let lineage_path = cfg.lineage_save_path();
    let lin_t0 = std::time::Instant::now();
    let shared_social = Arc::new(RwLock::new({
        if lineage_path.exists() {
            match load_lineages(&lineage_path) {
                Ok(s) => {
                    info!(
                        path = %lineage_path.display(),
                        count = s.lineages.len(),
                        "loaded lineages from disk"
                    );
                    s
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        path = %lineage_path.display(),
                        "lineage load failed; starting empty"
                    );
                    SocialState::default()
                }
            }
        } else {
            info!(
                path = %lineage_path.display(),
                "no lineage save — empty SocialState"
            );
            SocialState::default()
        }
    }));
    let boot_lineages_ms = lin_t0.elapsed().as_millis() as u64;

    // Soft accounts: load OLA1 after lineages (no SQL).
    let accounts_path = cfg.accounts_save_path();
    let acc_t0 = std::time::Instant::now();
    let shared_accounts = Arc::new(RwLock::new({
        if accounts_path.exists() {
            match load_accounts(&accounts_path) {
                Ok(b) => {
                    info!(
                        path = %accounts_path.display(),
                        count = b.len(),
                        "loaded accounts from disk"
                    );
                    b
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        path = %accounts_path.display(),
                        "account load failed; starting empty"
                    );
                    AccountBook::default()
                }
            }
        } else {
            info!(
                path = %accounts_path.display(),
                "no account save — empty AccountBook"
            );
            AccountBook::default()
        }
    }));
    let boot_accounts_ms = acc_t0.elapsed().as_millis() as u64;
    let boot_total_ms = restart_t0.elapsed().as_millis() as u64;
    counters.record_boot(
        boot_objects_ms,
        boot_transitions_ms,
        boot_world_ms,
        boot_lineages_ms,
        boot_accounts_ms,
        boot_total_ms,
    );
    info!(
        objects_ms = boot_objects_ms,
        transitions_ms = boot_transitions_ms,
        world_ms = boot_world_ms,
        lineages_ms = boot_lineages_ms,
        accounts_ms = boot_accounts_ms,
        total_ms = boot_total_ms,
        "server restart timings (one-shot)"
    );

    let selfplay_log = Arc::new(RwLock::new(Vec::<String>::new()));
    let selfplay_pos = Arc::new(RwLock::new((0i32, 0i32)));
    let player_views = Arc::new(RwLock::new(std::collections::HashMap::new()));
    let ops_series_view: Arc<RwLock<Vec<ol_metrics::OpsSample>>> =
        Arc::new(RwLock::new(Vec::new()));
    let env_view: ol_sim::EnvView = Arc::new(RwLock::new(ol_sim::EnvSnapshot::default()));
    let weather_view = Arc::new(RwLock::new(WeatherSnapshot::default()));
    // Seed account web view from boot-loaded book so /api/accounts works before first tick.
    let account_view = Arc::new(RwLock::new(shared_accounts.read().unwrap().snapshot()));
    let prestige_view = Arc::new(RwLock::new(PrestigeSnapshot::default()));
    // Seed lineage view from boot-loaded social so /lineage works before first sim tick.
    let lineage_view = Arc::new(RwLock::new(shared_social.read().unwrap().snapshot()));
    let animal_view = Arc::new(RwLock::new(AnimalSnapshot::default()));
    // Live AnimalWorld Arc for self-play threat sensing (sim publishes each vitals tick).
    let animals_share = Arc::new(RwLock::new(AnimalWorld::default()));
    let treasury_view = Arc::new(RwLock::new(TreasurySnapshot::default()));
    // Operator SAY SAVE sets this; autosave task polls every second.
    let force_save = Arc::new(AtomicBool::new(false));
    // Operator SAY !shutdown sets this after countdown + AP hold; main exits orderly.
    let shutdown_exit_flag = Arc::new(AtomicBool::new(false));

    info!(
        version = VERSION,
        game = %cfg.game_addr(),
        web = %cfg.web_addr(),
        "Open Life Reborn server starting"
    );

    let mut handles = Vec::new();

    {
        let counters = Arc::clone(&counters);
        let content = Arc::clone(&content);
        let world = Arc::clone(&shared_world);
        let outbound = Arc::clone(&outbound);
        let views = Arc::clone(&player_views);
        let env = Arc::clone(&env_view);
        let social = Arc::clone(&shared_social);
        let weather = Arc::clone(&weather_view);
        let accounts = Arc::clone(&account_view);
        let prestige = Arc::clone(&prestige_view);
        let lineages = Arc::clone(&lineage_view);
        let animals = Arc::clone(&animal_view);
        let animals_live = Arc::clone(&animals_share);
        let treasury = Arc::clone(&treasury_view);
        let shared_acc = Arc::clone(&shared_accounts);
        let save_req = Arc::clone(&force_save);
        let shutdown_exit = Arc::clone(&shutdown_exit_flag);
        let shutdown_cd = cfg.shutdown_countdown_secs.max(1) as f32;
        let shutdown_ap = cfg.shutdown_apocalypse_secs.max(1) as f32;
        let hz = cfg.tick_hz;
        let sim_speed = cfg.sim_speed_factor();
        let required_version = if content.data_version > 0 {
            content.data_version
        } else {
            cfg.required_version
        };
        let client_version_strict = cfg.client_version_strict;
        // Twin peer list from config (stub registry only — no network I/O).
        let twins = TwinRegistry::from_endpoints(
            cfg.twin_peers
                .iter()
                .map(|p| (p.host.clone(), p.port)),
        );
        if !twins.is_empty() {
            info!(count = twins.len(), "twin peers seeded from config (stub)");
        }
        if (sim_speed - 1.0).abs() > f32::EPSILON {
            info!(sim_speed, "time dilation sim_speed from config");
        }
        if client_version_strict {
            info!(
                required_version,
                "client_version_strict ON — LOGIN version mismatch hard-rejected"
            );
        }
        let timed_movement = cfg.timed_movement;
        let move_jump = cfg.move_jump_max_chebyshev;
        let ops_every = cfg.ops_sample_every_ticks;
        let ops_flush = cfg.ops_flush_secs;
        let intent_budget = cfg.intent_drain();
        let broadcast_all = cfg.broadcast_all_updates;
        let death_log = Arc::new(ol_sim::DeathLog::new(
            cfg.save_directory.join("deaths.journal"),
            2048,
            30,
        ));
        let death_log_flush = Arc::clone(&death_log);
        handles.push(tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                death_log_flush.try_flush();
            }
        }));
        let ops_view = Arc::clone(&ops_series_view);
        let death_for_sim = Arc::clone(&death_log);
        handles.push(tokio::spawn(async move {
            run_sim_loop_with_views(
                intent_rx,
                counters,
                hz,
                sim_speed,
                required_version,
                client_version_strict,
                content,
                world,
                outbound,
                Some(views),
                Some(env),
                Some(social),
                Some(weather),
                Some(accounts),
                Some(prestige),
                Some(lineages),
                Some(animals),
                Some(animals_live),
                Some(treasury),
                Some(shared_acc),
                twins,
                Some(save_req),
                timed_movement,
                move_jump,
                Some(ops_view),
                ops_every,
                ops_flush,
                intent_budget,
                broadcast_all,
                Some(death_for_sim),
                Some(shutdown_exit),
                shutdown_cd,
                shutdown_ap,
            )
            .await;
        }));
    }

    // Ops journal: delta append (~5 min) + shared watermark for shutdown flush.
    let ops_last_flushed_ms = Arc::new(std::sync::atomic::AtomicU64::new(0));
    {
        let ops_view = Arc::clone(&ops_series_view);
        let journal_path = cfg.ops_journal_path.clone();
        let flush_secs = cfg.ops_flush_secs.max(30);
        let watermark = Arc::clone(&ops_last_flushed_ms);
        handles.push(tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(flush_secs));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let since = watermark.load(Ordering::Relaxed);
                let samples: Vec<ol_metrics::OpsSample> = ops_view
                    .read()
                    .unwrap()
                    .iter()
                    .copied()
                    .filter(|s| s.wall_unix_ms > since)
                    .collect();
                if samples.is_empty() {
                    continue;
                }
                if let Err(e) = ol_metrics::append_ops_journal(&journal_path, &samples) {
                    warn!(error = %e, path = %journal_path.display(), "ops journal flush failed");
                } else {
                    let max_ms = samples.iter().map(|s| s.wall_unix_ms).max().unwrap_or(since);
                    watermark.store(max_ms, Ordering::Relaxed);
                    info!(
                        n = samples.len(),
                        path = %journal_path.display(),
                        "ops metrics journal flushed (delta)"
                    );
                }
            }
        }));
    }

    // AI NPC scheduler + activity log (RAM ring, flush every 30s).
    let npc_activity = Arc::new(npc_activity::NpcActivityLog::new(
        cfg.save_directory.join("npc_activity.journal"),
        4096,
        30,
    ));
    let npc_stats_view: Arc<RwLock<serde_json::Value>> =
        Arc::new(RwLock::new(serde_json::json!({})));
    {
        let act = Arc::clone(&npc_activity);
        let stats = Arc::clone(&npc_stats_view);
        handles.push(tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                act.try_flush();
                if let Ok(mut g) = stats.write() {
                    *g = act.summary_json();
                }
            }
        }));
    }
    {
        let npc_cfg = npc_ai::NpcConfig {
            enabled: cfg.npc_enabled,
            min: cfg.npc_min,
            max: cfg.npc_max,
            think_period_ticks: cfg.ai_think_period_ticks,
            observe_radius: cfg.ai_observe_radius,
            craft_radius: cfg.ai_craft_radius,
        };
        let intent_tx = intent_tx.clone();
        let world = Arc::clone(&shared_world);
        let content = Arc::clone(&content);
        let views = Arc::clone(&player_views);
        let counters = Arc::clone(&counters);
        let activity = Arc::clone(&npc_activity);
        handles.push(tokio::spawn(async move {
            npc_ai::run_npc_scheduler(
                npc_cfg, intent_tx, world, content, views, counters, activity,
            )
            .await;
        }));
    }

    // Periodic world + lineage + accounts save every 60s; also honor operator force-save
    // (SAY SAVE) via 1s poll of the shared AtomicBool (skip immediate first tick).
    {
        let world = Arc::clone(&shared_world);
        let social = Arc::clone(&shared_social);
        let accounts = Arc::clone(&shared_accounts);
        let counters = Arc::clone(&counters);
        let force_save = Arc::clone(&force_save);
        let save_path = cfg.world_save_path();
        let lineage_save = cfg.lineage_save_path();
        let accounts_save = cfg.accounts_save_path();
        handles.push(tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            interval.tick().await; // consume immediate first tick
            let mut secs_since_periodic: u32 = 0;
            loop {
                interval.tick().await;
                secs_since_periodic = secs_since_periodic.saturating_add(1);
                let forced = force_save.swap(false, Ordering::Relaxed);
                if !forced && secs_since_periodic < 60 {
                    continue;
                }
                secs_since_periodic = 0;
                if forced {
                    info!("force save requested (SAY SAVE)");
                }
                let mut any_ok = false;
                // Clone under short lock so sim is not blocked during disk I/O.
                let snapshot = world.read().unwrap().clone();
                if let Err(e) = ol_world::save_world_file(&snapshot, &save_path) {
                    warn!(error = %e, "autosave failed");
                } else {
                    any_ok = true;
                    info!(path = %save_path.display(), "world autosaved");
                }
                let social_snap = social.read().unwrap().clone();
                if let Err(e) = save_lineages(&social_snap, &lineage_save) {
                    warn!(error = %e, "lineage autosave failed");
                } else {
                    any_ok = true;
                    info!(
                        path = %lineage_save.display(),
                        count = social_snap.lineages.len(),
                        "lineages autosaved"
                    );
                }
                let accounts_snap = accounts.read().unwrap().clone();
                if let Err(e) = save_accounts(&accounts_snap, &accounts_save) {
                    warn!(error = %e, "account autosave failed");
                } else {
                    any_ok = true;
                    info!(
                        path = %accounts_save.display(),
                        count = accounts_snap.len(),
                        "accounts autosaved"
                    );
                }
                if any_ok {
                    counters
                        .autosaves
                        .fetch_add(1, Ordering::Relaxed);
                }
            }
        }));
    }

    if cfg.enable_web {
        let bind = cfg.web_addr();
        let state = WebState {
            counters: Arc::clone(&counters),
            version: VERSION,
            world: Arc::clone(&shared_world),
            content: Arc::clone(&content),
            selfplay_log: Arc::clone(&selfplay_log),
            selfplay_pos: Arc::clone(&selfplay_pos),
            player_views: Arc::clone(&player_views),
            env_view: Arc::clone(&env_view),
            weather_view: Arc::clone(&weather_view),
            account_view: Arc::clone(&account_view),
            prestige_view: Arc::clone(&prestige_view),
            lineage_view: Arc::clone(&lineage_view),
            animal_view: Arc::clone(&animal_view),
            treasury_view: Arc::clone(&treasury_view),
            ops_series: Arc::clone(&ops_series_view),
            npc_stats: Arc::clone(&npc_stats_view),
        };
        handles.push(tokio::spawn(async move {
            if let Err(e) = serve_web(&bind, state).await {
                error!(error = %e, "web server stopped");
            }
        }));
    }

    // Shared reverse craft graph for self-play craft planning (Farmer/Smith).
    // Cap from server.toml `craft_graph_seed_cap` keeps boot fast on large content.
    let craft_cap = cfg.craft_graph_cap();
    let craft_graph = Arc::new(build_reverse_craft_graph_capped(&content, craft_cap));
    info!(
        products = craft_graph.product_count(),
        edges = craft_graph.edge_count(),
        cap = craft_cap,
        "selfplay: reverse craft graph ready"
    );

    // Self-play agents for development / viewer (config: selfplay_enabled, selfplay_agents 1–3).
    if cfg.selfplay_enabled {
        let n_agents = cfg.selfplay_agent_count();
        info!(n_agents, "spawning self-play agents");
        // Agent 1: Forager (always when enabled).
        {
            let intent_tx = intent_tx.clone();
            let world = Arc::clone(&shared_world);
            let content = Arc::clone(&content);
            let log = Arc::clone(&selfplay_log);
            let pos = Arc::clone(&selfplay_pos);
            let views = Arc::clone(&player_views);
            let craft_graph = Arc::clone(&craft_graph);
            let animals = Arc::clone(&animals_share);
            let counters = Arc::clone(&counters);
            handles.push(tokio::spawn(async move {
                selfplay::run_selfplay_agent(
                    selfplay::SelfplayAgent::forager(),
                    intent_tx,
                    world,
                    content,
                    log,
                    pos,
                    views,
                    craft_graph,
                    animals,
                    Some(counters),
                )
                .await;
            }));
        }
        // Agent 2: Farmer
        if n_agents >= 2 {
            let intent_tx = intent_tx.clone();
            let world = Arc::clone(&shared_world);
            let content = Arc::clone(&content);
            let log = Arc::clone(&selfplay_log);
            // Farmer keeps its own pos so the web viewer still tracks the primary forager.
            let pos = Arc::new(RwLock::new((0i32, 0i32)));
            let views = Arc::clone(&player_views);
            let craft_graph = Arc::clone(&craft_graph);
            let animals = Arc::clone(&animals_share);
            let counters = Arc::clone(&counters);
            handles.push(tokio::spawn(async move {
                selfplay::run_selfplay_agent(
                    selfplay::SelfplayAgent::farmer(),
                    intent_tx,
                    world,
                    content,
                    log,
                    pos,
                    views,
                    craft_graph,
                    animals,
                    Some(counters),
                )
                .await;
            }));
        }
        // Agent 3: Hunter
        if n_agents >= 3 {
            let intent_tx = intent_tx.clone();
            let world = Arc::clone(&shared_world);
            let content = Arc::clone(&content);
            let log = Arc::clone(&selfplay_log);
            let pos = Arc::new(RwLock::new((0i32, 0i32)));
            let views = Arc::clone(&player_views);
            let craft_graph = Arc::clone(&craft_graph);
            let animals = Arc::clone(&animals_share);
            let counters = Arc::clone(&counters);
            handles.push(tokio::spawn(async move {
                selfplay::run_selfplay_agent(
                    selfplay::SelfplayAgent::hunter(),
                    intent_tx,
                    world,
                    content,
                    log,
                    pos,
                    views,
                    craft_graph,
                    animals,
                    Some(counters),
                )
                .await;
            }));
        }
    } else {
        info!("selfplay disabled (selfplay_enabled=false)");
    }

    let required_version = if content.data_version > 0 {
        content.data_version
    } else {
        cfg.required_version
    };
    info!(required_version, "SN version (dataVersion or config)");

    if cfg.enable_game_net {
        let net_cfg = NetConfig {
            bind: cfg.game_addr(),
            max_players: cfg.max_players,
            required_version,
            challenge_len: cfg.challenge_len,
            shared_world: Arc::clone(&shared_world),
            outbound: Arc::clone(&outbound),
            verify_ohol_ticket: cfg.verify_ohol_ticket,
            ticket_verify_url: cfg.ticket_verify_url.clone(),
            preferred_spawn: Arc::clone(&preferred_spawn),
        };
        let counters = Arc::clone(&counters);
        handles.push(tokio::spawn(async move {
            if let Err(e) = run_game_listener(net_cfg, counters, intent_tx).await {
                error!(error = %e, "game listener stopped");
            }
        }));
    } else {
        drop(intent_tx);
    }

    info!(
        "listening — game {} web {}  viewer http://127.0.0.1:{}/viewer",
        cfg.game_addr(),
        cfg.web_addr(),
        cfg.web_port
    );

    // Durable wait: !shutdown / stop.flag / interactive Ctrl+C.
    // Never treat a failed ctrl_c registration as process death.
    wait_for_shutdown(Arc::clone(&shutdown_exit_flag)).await;
    info!("entering orderly shutdown sequence");

    // Final ops journal delta flush on shutdown (before aborting tasks).
    {
        let since = ops_last_flushed_ms.load(Ordering::Relaxed);
        let samples: Vec<ol_metrics::OpsSample> = ops_series_view
            .read()
            .unwrap()
            .iter()
            .copied()
            .filter(|s| s.wall_unix_ms > since)
            .collect();
        if !samples.is_empty() {
            if let Err(e) = ol_metrics::append_ops_journal(&cfg.ops_journal_path, &samples) {
                warn!(error = %e, "ops journal shutdown flush failed");
            } else {
                info!(
                    n = samples.len(),
                    path = %cfg.ops_journal_path.display(),
                    "ops metrics journal flushed on shutdown"
                );
            }
        }
    }

    // Final save on shutdown
    {
        let w = shared_world.read().unwrap();
        world_boot::save_world_if_present(&cfg, &*w);
    }
    {
        let social = shared_social.read().unwrap();
        if let Err(e) = save_lineages(&*social, cfg.lineage_save_path()) {
            warn!(error = %e, "lineage shutdown save failed");
        } else {
            info!(
                path = %cfg.lineage_save_path().display(),
                count = social.lineages.len(),
                "lineages saved on shutdown"
            );
        }
    }
    {
        let accounts = shared_accounts.read().unwrap();
        if let Err(e) = save_accounts(&*accounts, cfg.accounts_save_path()) {
            warn!(error = %e, "account shutdown save failed");
        } else {
            info!(
                path = %cfg.accounts_save_path().display(),
                count = accounts.len(),
                "accounts saved on shutdown"
            );
        }
    }

    for h in handles {
        h.abort();
    }
}
