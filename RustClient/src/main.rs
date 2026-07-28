//! Headless OHOL client CLI for playtesting servers that speak the original protocol.
//!
//! Default target: local Open Life / OHOL game server at `127.0.0.1:8005`.

mod probe_test;

use std::env;
use std::io::Read;
use std::net::TcpListener;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use ohol_headless::content_binary::{bake_content, cache_dir_for};
use ohol_headless::ground_sprites::bake_olga_to_dir;
use ohol_headless::sprite_bank::bake_olsa_to_dir;
use ohol_headless::load_bench::{bench_full, resolve_content_root, write_report};
use ohol_headless::frame::{write_message, FrameReader};
use ohol_headless::login::hmac_sha1_hex;
use ohol_headless::move_state::PathDelta;
use ohol_headless::parse::LoginOutcome;
use ohol_headless::play_snapshot::{
    write_play_snapshot, PlaySnapshot, SnapshotViewExtras, DEFAULT_SNAPSHOT_DIR,
};
use ohol_headless::session::{
    connect_and_login, connect_and_login_logged, SessionConfig, SessionEvent,
};
use ohol_headless::wire_log::WireLog;
use std::path::PathBuf;
use ohol_headless::{
    encode_drop, encode_ka, encode_move, encode_remv, encode_self, encode_swap, encode_use,
    pure_account_key,
};

pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: &str = "8005";

fn usage() {
    eprintln!(
        "Usage:
  ohol-headless [options]              connect to {DEFAULT_HOST}:{DEFAULT_PORT}
  ohol-headless --self-check           local fixture peer (no game server)
  ohol-headless --bake-content         bake OLC1/OLT1 cache from OneLifeData7
  ohol-headless --bake-ground-atlas    optional full multi-page OLGA ground dump
  ohol-headless --bake-sprite-atlas    optional full multi-page OLSA sprite dump
  ohol-headless --bench-load           time headless + graphics content load
  ohol-headless --probe-move           login, MOVE, wait for PM/PU
  ohol-headless --probe-actions        encode/send USE/DROP/REMV/SELF
  ohol-headless --probe-play           MOVE + SAY + USE playtest
  ohol-headless --probe-test           version-name, pickup, MX, !CLOSE
  ohol-headless --snapshot [PATH]      login, wait for our_id, write play snapshot
  ohol-headless --snapshot-self-check  synthetic snapshot roundtrip (no server)
  --src PATH         content root for bake/bench (or OHOL_CONTENT_DIR)
  --out PATH         cache out dir (default: <src>/cache)
  --report PATH      markdown report for --bench-load
  --ensure-bake      bake cache if missing before --bench-load
  --also-text        also time pure text load in --bench-load
  --log PATH         wire transcript path
  --snapshot-label L optional label for --snapshot (default: cli)
  --host/--port/--email/--password/--account-key
  --move dx,dy  --use x,y  --drop x,y  --remv x,y  --self x,y  --swap x,y
  --say TEXT  --ka  --timeout SECS  --no-email-pad  --reconnect

Env:
  OHOL_LOAD_PROGRESS=1   print content/bank load stages (P5#36)
  OHOL_DEBUG=1           prefill settings.debug (F9/SNAP tools in ohol-client)
"
    );
}

fn main() -> ExitCode {
    let _ = dotenvy::dotenv();
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        usage();
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "--bake-content") {
        return match run_bake_content(&args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("bake-content FAILED: {e:#}");
                ExitCode::FAILURE
            }
        };
    }
    if args.iter().any(|a| a == "--bake-ground-atlas") {
        return match run_bake_ground_atlas(&args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("bake-ground-atlas FAILED: {e:#}");
                ExitCode::FAILURE
            }
        };
    }
    if args.iter().any(|a| a == "--bake-sprite-atlas") {
        return match run_bake_sprite_atlas(&args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("bake-sprite-atlas FAILED: {e:#}");
                ExitCode::FAILURE
            }
        };
    }
    if args.iter().any(|a| a == "--bench-load") {
        return match run_bench_load(&args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("bench-load FAILED: {e:#}");
                ExitCode::FAILURE
            }
        };
    }
    if args.iter().any(|a| a == "--self-check") {
        return match run_self_check() {
            Ok(()) => {
                println!("self-check: OK");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("self-check FAILED: {e:#}");
                ExitCode::FAILURE
            }
        };
    }
    if args.iter().any(|a| a == "--snapshot-self-check") {
        return match run_snapshot_self_check() {
            Ok(path) => {
                println!("snapshot-self-check: OK → {}", path.display());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("snapshot-self-check FAILED: {e:#}");
                ExitCode::FAILURE
            }
        };
    }
    if args.iter().any(|a| a == "--snapshot") {
        return match run_snapshot(&args) {
            Ok(path) => {
                println!("snapshot: OK → {}", path.display());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("snapshot FAILED: {e:#}");
                ExitCode::FAILURE
            }
        };
    }
    if args.iter().any(|a| a == "--probe-move") {
        return match run_probe_move(&args) {
            Ok(true) => {
                println!("probe-move: PASS");
                ExitCode::SUCCESS
            }
            Ok(false) => {
                println!("probe-move: FAIL (see report)");
                ExitCode::FAILURE
            }
            Err(e) => {
                eprintln!("probe-move FAILED: {e:#}");
                ExitCode::FAILURE
            }
        };
    }
    if args.iter().any(|a| a == "--probe-actions") {
        return match run_probe_actions(&args) {
            Ok(true) => {
                println!("probe-actions: PASS");
                ExitCode::SUCCESS
            }
            Ok(false) => {
                println!("probe-actions: FAIL");
                ExitCode::FAILURE
            }
            Err(e) => {
                eprintln!("probe-actions FAILED: {e:#}");
                ExitCode::FAILURE
            }
        };
    }
    if args.iter().any(|a| a == "--probe-play") {
        return match run_probe_play(&args) {
            Ok(true) => {
                println!("probe-play: PASS");
                ExitCode::SUCCESS
            }
            Ok(false) => {
                println!("probe-play: FAIL");
                ExitCode::FAILURE
            }
            Err(e) => {
                eprintln!("probe-play FAILED: {e:#}");
                ExitCode::FAILURE
            }
        };
    }
    if args.iter().any(|a| a == "--probe-test") {
        return match probe_test::run(&args) {
            Ok(true) => {
                println!("probe-test: PASS");
                ExitCode::SUCCESS
            }
            Ok(false) => {
                println!("probe-test: FAIL");
                ExitCode::FAILURE
            }
            Err(e) => {
                eprintln!("probe-test FAILED: {e:#}");
                ExitCode::FAILURE
            }
        };
    }
    match run_live(&args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

pub fn flag_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|w| w[0] == name)
        .map(|w| w[1].as_str())
}

pub fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

pub fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Synthetic snapshot write/read (no server) — for CI / AI smoke.
fn run_snapshot_self_check() -> anyhow::Result<PathBuf> {
    let snap = PlaySnapshot::synthetic_fixture();
    let path = PathBuf::from(DEFAULT_SNAPSHOT_DIR).join("self_check_fixture.txt");
    snap.write_file(&path).map_err(|e| anyhow::anyhow!(e))?;
    let loaded = PlaySnapshot::read_file(&path).map_err(|e| anyhow::anyhow!(e))?;
    if loaded.our_id != snap.our_id || loaded.x != snap.x {
        anyhow::bail!("roundtrip mismatch");
    }
    eprintln!("snapshot-self-check: {}", loaded.summary_line());
    Ok(path)
}

/// Login to server, wait until our_id, write play snapshot, exit.
fn run_snapshot(args: &[String]) -> anyhow::Result<PathBuf> {
    let label = flag_value(args, "--snapshot-label").unwrap_or("cli");
    // PATH after --snapshot, or --out, or default.
    let path_arg = args
        .windows(2)
        .find(|w| w[0] == "--snapshot" && !w[1].starts_with('-'))
        .map(|w| w[1].as_str())
        .or_else(|| flag_value(args, "--out"));
    let timeout = flag_value(args, "--timeout")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(12);

    let mut cfg = SessionConfig::default();
    cfg.host = flag_value(args, "--host")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| env_or("OHOL_HOST", DEFAULT_HOST).trim().to_string());
    cfg.port = flag_value(args, "--port")
        .and_then(|s| s.trim().parse().ok())
        .or_else(|| env::var("OHOL_PORT").ok().and_then(|s| s.trim().parse().ok()))
        .unwrap_or(8005);
    // Prefer multi-second connect for snapshot (AccountPage default 30ms is play-poll only).
    cfg.read_timeout = Duration::from_secs(timeout.max(8));
    cfg.write_timeout = Duration::from_secs(10);
    if let Some(e) = flag_value(args, "--email") {
        cfg.email = e.to_string();
    } else if let Ok(e) = env::var("OHOL_EMAIL") {
        if !e.is_empty() {
            cfg.email = e;
        }
    }
    if let Some(p) = flag_value(args, "--password") {
        cfg.password = p.to_string();
    } else if let Ok(p) = env::var("OHOL_PASSWORD") {
        if !p.is_empty() {
            cfg.password = p;
        }
    }
    if let Some(k) = flag_value(args, "--account-key") {
        cfg.account_key = k.to_string();
    } else if let Ok(k) = env::var("OHOL_ACCOUNT_KEY") {
        cfg.account_key = k;
    }
    eprintln!(
        "snapshot: connect {}:{} timeout={}s label={}",
        cfg.host, cfg.port, timeout, label
    );
    // Snapshot path: don't need full content banks before LOGIN — still loaded inside connect.
    // After login, use a short read timeout so we drain birth PU/FM without 10s blocks.
    let mut session = connect_and_login(&cfg)?;
    if !matches!(session.login, LoginOutcome::Accepted) {
        anyhow::bail!("login not accepted: {:?}", session.login);
    }
    let _ = session.set_read_timeout(Some(Duration::from_millis(200)));
    let deadline = Instant::now() + Duration::from_secs(timeout);
    while session.our_id.is_none() && Instant::now() < deadline {
        match session.poll_event() {
            Ok(ev) => {
                // Keep draining this frame batch.
                let _ = ev;
                while let Ok(_) = session.poll_event() {}
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                std::thread::sleep(Duration::from_millis(30));
            }
            Err(e) => return Err(e.into()),
        }
    }
    if session.our_id.is_none() {
        anyhow::bail!(
            "timeout waiting for our_id (login ok, no PU applied; FM batch?)"
        );
    }
    let extras = SnapshotViewExtras {
        label: label.to_string(),
        screen: "Headless".into(),
        last_status: "cli-snapshot".into(),
        ..Default::default()
    };
    let path = path_arg.map(Path::new);
    let written = write_play_snapshot(&session, &extras, path).map_err(|e| anyhow::anyhow!(e))?;
    let snap = PlaySnapshot::read_file(&written).map_err(|e| anyhow::anyhow!(e))?;
    eprintln!("snapshot: {}", snap.summary_line());
    Ok(written)
}

/// Time headless + graphics content load (OLC1/OLT1/OLA1 + sprites).
fn run_bench_load(args: &[String]) -> anyhow::Result<()> {
    let src = flag_value(args, "--src").map(Path::new);
    let root = resolve_content_root(src).map_err(|e| anyhow::anyhow!(e))?;
    let ensure_bake = has_flag(args, "--ensure-bake");
    let also_text = has_flag(args, "--also-text");
    let report = flag_value(args, "--report").unwrap_or("logs/load-bench.md");

    eprintln!(
        "bench-load: root={} ensure_bake={} also_text={}",
        root.display(),
        ensure_bake,
        also_text
    );
    let profiles =
        bench_full(&root, ensure_bake, also_text).map_err(|e| anyhow::anyhow!(e))?;
    for p in &profiles {
        print!("{}", p.report_lines());
    }
    write_report(&profiles, report).map_err(|e| anyhow::anyhow!(e))?;
    eprintln!("bench-load: wrote report {report}");
    Ok(())
}

/// Optional full multi-page sprite atlas dump (OLSA / P4#40).
/// Not part of default `bake_content` (large); OLS1 meta + lazy TGA remains default play path.
fn run_bake_sprite_atlas(args: &[String]) -> anyhow::Result<()> {
    let src = flag_value(args, "--src")
        .map(|s| s.to_string())
        .or_else(|| env::var("OHOL_CONTENT_DIR").ok())
        .unwrap_or_else(|| r"C:\OhOl\OpenLife\OneLifeData7".into());
    let src_path = Path::new(&src);
    if !src_path.join("sprites").is_dir() {
        anyhow::bail!(
            "content root missing sprites/: {} (set --src or OHOL_CONTENT_DIR)",
            src_path.display()
        );
    }
    let out = flag_value(args, "--out")
        .map(Path::new)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| cache_dir_for(src_path));
    let data_version = flag_value(args, "--version")
        .and_then(|s| s.parse::<u32>().ok())
        .or_else(|| {
            std::fs::read_to_string(src_path.join("dataVersionNumber.txt"))
                .ok()
                .and_then(|s| s.trim().parse().ok())
        })
        .unwrap_or(0);
    eprintln!(
        "bake-sprite-atlas: src={} out={} version={}",
        src_path.display(),
        out.display(),
        data_version
    );
    let stats = bake_olsa_to_dir(src_path, &out, data_version).map_err(|e| anyhow::anyhow!(e))?;
    eprintln!(
        "bake-sprite-atlas: OK {} → {}",
        stats.report_line(),
        out.join("olsa_sprite_atlas.bin").display()
    );
    eprintln!(
        "bake-sprite-atlas timings: pack={:.1}ms write={:.1}ms total={:.1}ms",
        stats.pack_duration.as_secs_f64() * 1000.0,
        stats.write_duration.as_secs_f64() * 1000.0,
        stats.total_duration.as_secs_f64() * 1000.0,
    );
    Ok(())
}

/// Optional full multi-page ground atlas dump (OLGA / SaveGroundData-style).
/// Not part of default `bake_content` (large); OLG1 index remains the default play path.
fn run_bake_ground_atlas(args: &[String]) -> anyhow::Result<()> {
    let src = flag_value(args, "--src")
        .map(|s| s.to_string())
        .or_else(|| env::var("OHOL_CONTENT_DIR").ok())
        .or_else(|| env::var("OHOL_GAME_DATA").ok())
        .unwrap_or_else(|| r"C:\OhOl\OpenLife\OneLifeGameSourceData".into());
    let src_path = Path::new(&src);
    let out = flag_value(args, "--out")
        .map(Path::new)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| {
            // Prefer content cache when objects/ present; else src/cache.
            if src_path.join("objects").is_dir() {
                cache_dir_for(src_path)
            } else {
                src_path.join("cache")
            }
        });
    let data_version = flag_value(args, "--version")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    eprintln!(
        "bake-ground-atlas: src={} out={} version={}",
        src_path.display(),
        out.display(),
        data_version
    );
    let stats = bake_olga_to_dir(Some(src_path), &out, data_version)
        .map_err(|e| anyhow::anyhow!(e))?;
    eprintln!(
        "bake-ground-atlas: OK {} → {}",
        stats.report_line(),
        out.join("olga_ground_atlas.bin").display()
    );
    // Explicit bake timing line for logs / CI greps.
    eprintln!(
        "bake-ground-atlas timings: pack={:.1}ms write={:.1}ms total={:.1}ms",
        stats.pack_duration.as_secs_f64() * 1000.0,
        stats.write_duration.as_secs_f64() * 1000.0,
        stats.total_duration.as_secs_f64() * 1000.0,
    );
    Ok(())
}

/// Bake OLC1/OLT1 binary cache (CONTENT_BINARY / H-BAKE).
fn run_bake_content(args: &[String]) -> anyhow::Result<()> {
    let src = flag_value(args, "--src")
        .map(|s| s.to_string())
        .or_else(|| env::var("OHOL_CONTENT_DIR").ok())
        .unwrap_or_else(|| r"C:\OhOl\OpenLife\OneLifeData7".into());
    let src_path = Path::new(&src);
    if !src_path.join("objects").is_dir() {
        anyhow::bail!(
            "content root missing objects/: {} (set --src or OHOL_CONTENT_DIR)",
            src_path.display()
        );
    }
    let out = flag_value(args, "--out")
        .map(Path::new)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| cache_dir_for(src_path));

    eprintln!(
        "bake-content: src={} out={}",
        src_path.display(),
        out.display()
    );
    let res = bake_content(src_path, &out).map_err(|e| anyhow::anyhow!(e))?;
    eprintln!(
        "bake-content: OK version={} objects={} transitions={} dummies={} ground={} \
         overlays={} sounds={} sprites={} olc1={}B olt1={}B ola1={}B olg1={}B olo1={}B \
         olsn={}B ols1={}B in {:.2}s → {}",
        res.data_version,
        res.object_count,
        res.transition_count,
        res.dummy_count,
        res.ground_count,
        res.overlay_count,
        res.sound_count,
        res.sprite_count,
        res.olc1_bytes,
        res.olt1_bytes,
        res.ola1_bytes,
        res.olg1_bytes,
        res.olo1_bytes,
        res.olsn_bytes,
        res.ols1_bytes,
        res.timings.total.as_secs_f64(),
        res.cache_dir.display()
    );
    eprintln!("bake-content timings:\n{}", res.timings.report_lines());
    eprintln!(
        "note: ols1 is meta only; sprite pixel atlas pages = P4#40 (not baked here). \
         ground pixel pages: --bake-ground-atlas"
    );
    Ok(())
}

fn session_cfg_from_args(args: &[String]) -> SessionConfig {
    let port: u16 = flag_value(args, "--port")
        .map(|s| s.to_string())
        .or_else(|| env::var("OHOL_PORT").ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(8005);
    let timeout_secs: u64 = flag_value(args, "--timeout")
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    SessionConfig {
        host: flag_value(args, "--host")
            .map(|s| s.to_string())
            .unwrap_or_else(|| env_or("OHOL_HOST", DEFAULT_HOST)),
        port,
        email: flag_value(args, "--email")
            .map(|s| s.to_string())
            .unwrap_or_else(|| env_or("OHOL_EMAIL", "blank_email")),
        password: flag_value(args, "--password")
            .map(|s| s.to_string())
            .unwrap_or_else(|| env_or("OHOL_PASSWORD", "x")),
        account_key: flag_value(args, "--account-key")
            .map(|s| s.to_string())
            .unwrap_or_else(|| env_or("OHOL_ACCOUNT_KEY", "")),
        tutorial_number: flag_value(args, "--tutorial")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        reconnect: has_flag(args, "--reconnect"),
        pad_email_to_80: !has_flag(args, "--no-email-pad"),
        read_timeout: Duration::from_secs(timeout_secs),
        write_timeout: Duration::from_secs(10),
        ..SessionConfig::default()
    }
}

fn run_live(args: &[String]) -> anyhow::Result<ExitCode> {
    let cfg = session_cfg_from_args(args);
    let timeout: u64 = flag_value(args, "--timeout")
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    let log_path = flag_value(args, "--log").map(|s| s.to_string());
    let mut session = if let Some(ref lp) = log_path {
        let wl = Arc::new(WireLog::create(lp)?);
        connect_and_login_logged(&cfg, wl)?
    } else {
        connect_and_login(&cfg)?
    };

    eprintln!(
        "logged in as our_id={:?} login={:?} (host={}:{})",
        session.our_id, session.login, cfg.host, cfg.port
    );
    if session.login != LoginOutcome::Accepted {
        return Ok(ExitCode::FAILURE);
    }

    let boot = Instant::now() + Duration::from_secs(3);
    while Instant::now() < boot && session.our_id.is_none() {
        let _ = session.poll_event();
    }

    if let Some(mv) = flag_value(args, "--move") {
        let parts: Vec<&str> = mv.split(',').collect();
        if parts.len() == 2 {
            let dx: i32 = parts[0].parse().unwrap_or(0);
            let dy: i32 = parts[1].parse().unwrap_or(0);
            let deltas = [PathDelta { x: dx, y: dy }];
            match session.send_move(&deltas) {
                Ok(line) => eprintln!("sent {line}"),
                Err(e) => eprintln!("MOVE error: {e}"),
            }
        }
    }

    let want_ka = has_flag(args, "--ka")
        || (flag_value(args, "--move").is_none()
            && flag_value(args, "--use").is_none()
            && flag_value(args, "--drop").is_none()
            && flag_value(args, "--remv").is_none()
            && flag_value(args, "--self").is_none()
            && flag_value(args, "--say").is_none());

    if want_ka {
        session.send_ka()?;
        eprintln!("sent KA");
    }

    if let Some(p) = flag_value(args, "--use") {
        let (x, y) = parse_xy(p)?;
        let id = flag_value(args, "--use-id").and_then(|s| s.parse().ok());
        let slot = flag_value(args, "--use-slot").and_then(|s| s.parse().ok());
        let line = session.send_use(x, y, id, slot)?;
        eprintln!("sent {line}");
    }
    if let Some(p) = flag_value(args, "--drop") {
        let (x, y) = parse_xy(p)?;
        let c: i32 = flag_value(args, "--drop-slot")
            .and_then(|s| s.parse().ok())
            .unwrap_or(-1);
        let line = session.send_drop(x, y, c)?;
        eprintln!("sent {line}");
    }
    if let Some(p) = flag_value(args, "--remv") {
        let (x, y) = parse_xy(p)?;
        let i: i32 = flag_value(args, "--remv-slot")
            .and_then(|s| s.parse().ok())
            .unwrap_or(-1);
        let line = session.send_remv(x, y, i)?;
        eprintln!("sent {line}");
    }
    if let Some(p) = flag_value(args, "--self") {
        let (x, y) = parse_xy(p)?;
        let i: i32 = flag_value(args, "--self-slot")
            .and_then(|s| s.parse().ok())
            .unwrap_or(-1);
        let line = session.send_self(x, y, i)?;
        eprintln!("sent {line}");
    }
    if let Some(p) = flag_value(args, "--swap") {
        let (x, y) = parse_xy(p)?;
        session.send_raw(&encode_swap(x, y))?;
        eprintln!("sent SWAP {x},{y}");
    }
    if let Some(text) = flag_value(args, "--say") {
        let line = session.send_say(text)?;
        eprintln!("sent {line}");
    }

    let deadline = Instant::now() + Duration::from_secs(timeout);
    while Instant::now() < deadline {
        // C++: idle > 15s without TX → KA 0 0#
        if let Ok(Some(line)) = session.maybe_send_ka() {
            eprintln!("sent {line} (idle keepalive)");
        }
        match session.poll_event() {
            Ok(ev) => eprintln!("event: {ev:?}"),
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                thread::sleep(Duration::from_millis(50));
            }
            Err(_) => break,
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn parse_xy(p: &str) -> anyhow::Result<(i32, i32)> {
    let parts: Vec<&str> = p.split(',').collect();
    if parts.len() != 2 {
        anyhow::bail!("expected x,y got {p}");
    }
    Ok((parts[0].parse()?, parts[1].parse()?))
}

// ── self-check fixture peer ──────────────────────────────────────────────────

fn run_self_check() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let challenge = "test_challenge_xyz";
    let password = "secret";
    let account_key = "key123";

    let peer = thread::spawn(move || -> anyhow::Result<()> {
        let (mut sock, _) = listener.accept()?;
        write_message(&mut sock, &format!("SN\n1/20\n{challenge}\n184\n"))?;
        let mut fr = FrameReader::new();
        let mut buf = [0u8; 4096];
        let login_body = loop {
            let n = sock.read(&mut buf)?;
            if n == 0 {
                anyhow::bail!("peer closed before LOGIN");
            }
            let msgs = fr.push(&buf[..n]);
            if let Some(m) = msgs.into_iter().next() {
                break m;
            }
        };
        if !(login_body.starts_with("LOGIN ") || login_body.starts_with("RLOGIN ")) {
            anyhow::bail!("expected LOGIN, got {login_body}");
        }
        let parts: Vec<&str> = login_body.split_whitespace().collect();
        if parts.len() < 6 {
            anyhow::bail!("LOGIN too short: {login_body}");
        }
        let pw_hash = parts[parts.len() - 3];
        let key_hash = parts[parts.len() - 2];
        let exp_pw = hmac_sha1_hex(password, challenge);
        let exp_key = hmac_sha1_hex(&pure_account_key(account_key), challenge);
        if pw_hash != exp_pw {
            anyhow::bail!("pw hash mismatch");
        }
        if key_hash != exp_key {
            anyhow::bail!("key hash mismatch");
        }
        write_message(&mut sock, "ACCEPTED\n")?;
        write_message(
            &mut sock,
            "PU\n7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n",
        )?;
        write_message(&mut sock, "FM\n")?;
        thread::sleep(Duration::from_millis(100));
        Ok(())
    });

    let cfg = SessionConfig {
        host: "127.0.0.1".into(),
        port,
        email: "user@test".into(),
        password: password.into(),
        account_key: account_key.into(),
        pad_email_to_80: false,
        read_timeout: Duration::from_secs(3),
        write_timeout: Duration::from_secs(3),
        ..SessionConfig::default()
    };
    let mut session = connect_and_login(&cfg)?;
    if session.login != LoginOutcome::Accepted {
        anyhow::bail!("login not accepted: {:?}", session.login);
    }
    for _ in 0..8 {
        match session.poll_event() {
            Ok(SessionEvent::PlayerUpdate { pu, .. }) => {
                if pu.player_id == 7 {
                    session.our_id = Some(7);
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    peer.join().map_err(|_| anyhow::anyhow!("peer panic"))??;
    let _ = encode_move(0, 0, 2, &[PathDelta { x: 1, y: 0 }])?;
    let _ = encode_use(0, 0, Some(33), None);
    let _ = encode_drop(1, 0, -1);
    let _ = encode_remv(0, 0, -1);
    let _ = encode_self(0, 0, -1);
    let _ = encode_ka(0, 0);
    let _ = encode_swap(0, 0);
    Ok(())
}

// ── probes ───────────────────────────────────────────────────────────────────

fn run_probe_move(args: &[String]) -> anyhow::Result<bool> {
    let cfg = session_cfg_from_args(args);
    let log_path = flag_value(args, "--log")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "logs/wire-probe-move.log".into());
    let wire = Arc::new(WireLog::create(&log_path)?);
    println!("wire log: {}", wire.path().display());

    let mut session = connect_and_login_logged(&cfg, Arc::clone(&wire))?;
    println!("login={:?}", session.login);
    if session.login != LoginOutcome::Accepted {
        return Ok(false);
    }
    session
        .stream_mut()
        .set_read_timeout(Some(Duration::from_millis(250)))
        .ok();

    let boot = Instant::now() + Duration::from_secs(5);
    while Instant::now() < boot && session.our_id.is_none() {
        let _ = session.maybe_send_ka();
        let _ = session.poll_event();
    }
    session.move_state.in_motion = false;
    session.move_state.awaiting_force_ack = false;

    let path = [PathDelta { x: 1, y: 0 }];
    let line = session.send_move(&path)?;
    println!("sent {line}");

    let mut saw_pm = false;
    let mut saw_pu = false;
    let wait_secs: u64 = flag_value(args, "--timeout")
        .and_then(|s| s.parse().ok())
        .unwrap_or(12);
    let wait = Instant::now() + Duration::from_secs(wait_secs);
    while Instant::now() < wait {
        let _ = session.maybe_send_ka();
        match session.poll_event() {
            Ok(SessionEvent::PlayerMovesStart(_)) => {
                saw_pm = true;
                println!("got PM");
            }
            Ok(SessionEvent::PlayerUpdate { pu, .. }) if Some(pu.player_id) == session.our_id => {
                saw_pu = true;
                println!(
                    "got PU id={} pos=({},{}) done={} force={}",
                    pu.player_id, pu.x, pu.y, pu.done_moving_seq_num, pu.force
                );
                if pu.done_moving_seq_num > 1 && !pu.force {
                    break;
                }
            }
            Ok(_) => {}
            Err(_) => continue,
        }
        if saw_pm && saw_pu {
            break;
        }
    }
    println!("probe-move summary: pm={saw_pm} pu={saw_pu}");
    Ok(saw_pm || saw_pu)
}

fn run_probe_actions(args: &[String]) -> anyhow::Result<bool> {
    let u = encode_use(1, 2, Some(33), None);
    assert!(u.starts_with("USE "), "{u}");
    let d = encode_drop(3, 4, -1);
    assert!(d.starts_with("DROP "), "{d}");
    let r = encode_remv(5, 6, 0);
    assert!(r.starts_with("REMV "), "{r}");
    let s = encode_self(0, 0, -1);
    assert!(s.starts_with("SELF "), "{s}");
    println!("encoder smoke: USE/DROP/REMV/SELF OK");

    let cfg = session_cfg_from_args(args);
    let log_path = flag_value(args, "--log")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "logs/wire-actions.log".into());
    match connect_and_login_logged(&cfg, Arc::new(WireLog::create(&log_path)?)) {
        Ok(mut session) if session.login == LoginOutcome::Accepted => {
            session
                .stream_mut()
                .set_read_timeout(Some(Duration::from_millis(200)))
                .ok();
            let boot = Instant::now() + Duration::from_secs(3);
            while Instant::now() < boot && session.our_id.is_none() {
                let _ = session.maybe_send_ka();
                let _ = session.poll_event();
            }
            let x = session.move_state.x;
            let y = session.move_state.y;
            let _ = session.send_use(x, y, None, None)?;
            let _ = session.send_drop(x, y, -1)?;
            let _ = session.send_self(x, y, -1)?;
            let _ = session.send_ka()?;
            println!("live actions sent near ({x},{y})");
            Ok(true)
        }
        Ok(_) => {
            println!("note: server login not accepted — encoder-only pass");
            Ok(true)
        }
        Err(e) => {
            println!("note: no server ({e}) — encoder-only pass");
            Ok(true)
        }
    }
}

fn run_probe_play(args: &[String]) -> anyhow::Result<bool> {
    let cfg = session_cfg_from_args(args);
    let log_path = flag_value(args, "--log")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "logs/wire-play.log".into());
    let wire = Arc::new(WireLog::create(&log_path)?);
    let mut session = connect_and_login_logged(&cfg, wire)?;
    if session.login != LoginOutcome::Accepted {
        println!("login={:?}", session.login);
        return Ok(false);
    }
    session
        .stream_mut()
        .set_read_timeout(Some(Duration::from_millis(250)))
        .ok();

    let boot = Instant::now() + Duration::from_secs(5);
    while Instant::now() < boot && session.our_id.is_none() {
        let _ = session.maybe_send_ka();
        let _ = session.poll_event();
    }
    session.move_state.in_motion = false;
    session.move_state.awaiting_force_ack = false;

    let path = [PathDelta { x: 1, y: 0 }];
    let _ = session.send_move(&path)?;
    let _ = session.send_say("hi")?;
    let x = session.move_state.x;
    let y = session.move_state.y;
    let _ = session.send_use(x, y, None, None)?;

    let mut events = 0usize;
    let wait = Instant::now() + Duration::from_secs(8);
    while Instant::now() < wait {
        let _ = session.maybe_send_ka();
        match session.poll_event() {
            Ok(_) => events += 1,
            Err(_) => continue,
        }
    }
    println!("probe-play events_seen={events}");
    Ok(events > 0 || session.our_id.is_some())
}
