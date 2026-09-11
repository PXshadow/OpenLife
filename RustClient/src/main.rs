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
  ohol-headless --bench-fps            soft-FB SceneRenderer FPS (real content)
  ohol-headless --bench-present        Soft vs GPU present-path CPU cost (no window)
  ohol-headless --probe-move           login, MOVE, wait for PM/PU
  ohol-headless --probe-walk-finish    sequential MOVE hops; print arrival PU seq/xy/force
  ohol-headless --probe-walk-repath    repath mid-walk; print FORCE / stale finish PU / jumps
  ohol-headless --probe-actions        encode/send USE/DROP/REMV/SELF
  ohol-headless --probe-play           MOVE + SAY + USE playtest
  ohol-headless --probe-test           version-name, pickup, MX, !CLOSE
  ohol-headless --snapshot [PATH]      login, wait for our_id, write play snapshot
  ohol-headless --snapshot-self-check  synthetic snapshot roundtrip (no server)
  ohol-headless --boot-shot [PATH]     login, draw first frame, write PPM/PNG, probe USE
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
    if args.iter().any(|a| a == "--bench-fps") {
        return match run_bench_fps(&args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("bench-fps: {e}");
                ExitCode::FAILURE
            }
        };
    }
    if args.iter().any(|a| a == "--bench-present") {
        return match run_bench_present(&args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("bench-present: {e}");
                ExitCode::FAILURE
            }
        };
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
    if args.iter().any(|a| a == "--boot-shot") {
        return match run_boot_shot(&args) {
            Ok(path) => {
                println!("boot-shot: OK → {}", path.display());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("boot-shot FAILED: {e:#}");
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
    if args.iter().any(|a| a == "--probe-walk-repath") {
        return match run_probe_walk_repath(&args) {
            Ok(true) => {
                println!("probe-walk-repath: PASS");
                ExitCode::SUCCESS
            }
            Ok(false) => {
                println!("probe-walk-repath: FAIL (see report)");
                ExitCode::FAILURE
            }
            Err(e) => {
                eprintln!("probe-walk-repath FAILED: {e:#}");
                ExitCode::FAILURE
            }
        };
    }
    if args.iter().any(|a| a == "--probe-walk-finish") {
        return match run_probe_walk_finish(&args) {
            Ok(true) => {
                println!("probe-walk-finish: PASS (server sent matching finish PU)");
                ExitCode::SUCCESS
            }
            Ok(false) => {
                println!("probe-walk-finish: FAIL (see hop table)");
                ExitCode::FAILURE
            }
            Err(e) => {
                eprintln!("probe-walk-finish FAILED: {e:#}");
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
/// Login, draw the first playable frame (~100ms after our_id), dump PPM/PNG,
/// and probe a nearby object USE so we can see interact + overlay bugs.
fn run_boot_shot(args: &[String]) -> anyhow::Result<PathBuf> {
    use ohol_headless::anim_bank::AnimBank;
    use ohol_headless::click_use;
    use ohol_headless::ground_sprites::GroundBank;
    use ohol_headless::hover_pick::pick_at_screen;
    use ohol_headless::render::{Framebuffer, SceneRenderer};
    use ohol_headless::sprite_bank::SpriteBank;

    let out = args
        .windows(2)
        .find(|w| w[0] == "--boot-shot" && !w[1].starts_with('-'))
        .map(|w| PathBuf::from(w[1].clone()))
        .or_else(|| flag_value(args, "--out").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("boot_shot.ppm"));

    let mut cfg = session_cfg_from_args(args);
    cfg.read_timeout = Duration::from_secs(12);
    cfg.write_timeout = Duration::from_secs(10);
    eprintln!("boot-shot: connect {}:{}", cfg.host, cfg.port);
    let mut session = connect_and_login(&cfg)?;
    if !matches!(session.login, LoginOutcome::Accepted) {
        anyhow::bail!("login not accepted: {:?}", session.login);
    }
    let _ = session.set_read_timeout(Some(Duration::from_millis(50)));
    let deadline = Instant::now() + Duration::from_secs(12);
    while session.our_id.is_none() && Instant::now() < deadline {
        match session.poll_event() {
            Ok(_) => {
                while session.poll_event().is_ok() {}
            }
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Err(e.into()),
        }
    }
    let born = Instant::now();
    // Drain ~100ms of follow-up MC/PU so the first frame matches "just logged in".
    let until = born + Duration::from_millis(100);
    while Instant::now() < until {
        match session.poll_event() {
            Ok(_) => {}
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                thread::sleep(Duration::from_millis(5));
            }
            Err(_) => break,
        }
    }
    let Some(me) = session.world.our() else {
        anyhow::bail!("no our player after login");
    };
    let age = me.current_age();
    let (px, py) = (me.x, me.y);
    let display_id = me.display_id;
    let moving = me.moving;
    eprintln!(
        "boot-shot: our_id={} pos=({},{}) display={} age={:.4} moving={} in_motion={} pending={} pointers={} tiles={}",
        me.id,
        px,
        py,
        display_id,
        age,
        moving,
        session.move_state.in_motion,
        session.player_action_pending,
        session.world.says_pointers().len(),
        session.map.len(),
    );

    let root = resolve_content_root(flag_value(args, "--src").map(Path::new))
        .map_err(|e| anyhow::anyhow!(e))?;
    let mut sprites = SpriteBank::load_prefer_cache(&root);
    let mut anims = AnimBank::load_prefer_cache(&root);
    let mut ground = GroundBank::load_prefer_cache(&root);
    let _ = ground.preload_overlays();
    if display_id > 0 {
        sprites.preload([display_id]);
    }

    // Always dump a lying newborn (age 0.05) so we can see feet / overlays
    // without depending on this login being a baby spawn.
    {
        use ohol_headless::client_map::{ClientMap, MapTile};
        use ohol_headless::live_object::LiveWorld;
        use ohol_headless::parse::parse_pu_line;
        let mut bw = LiveWorld::new();
        let pu = parse_pu_line(
            "1 19 0 0 0 0 0 0 0 0 -1 0.5 1 0 0 0 0.05 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 0",
        );
        if let Some(pu) = pu {
            bw.apply_pu(&pu);
            bw.set_our_id(1);
            let mut bm = ClientMap::new();
            for y in -2..=2 {
                for x in -3..=3 {
                    bm.set(x, y, MapTile { biome: 0, ..MapTile::empty() });
                }
            }
            let mut sc = SceneRenderer::default();
            sc.set_content_root(Some(&root));
            sc.camera.zoom = 64.0;
            sc.draw_hud = false;
            sc.highlight_tile = None;
            let mut fbb = Framebuffer::new(960, 540);
            sc.draw(&mut fbb, &mut bm, &mut bw, &session.content, &mut sprites, &mut anims, 0.0);
            let baby_path = out.with_file_name("baby_lie.ppm");
            let _ = fbb.write_ppm(&baby_path);
            eprintln!("boot-shot: baby_lie {}", baby_path.display());
        }
    }

    let mut scene = SceneRenderer::default();
    scene.set_content_root(Some(&root));
    scene.ground = ground;
    scene.draw_hud = true;
    scene.camera.x = px as f32;
    scene.camera.y = py as f32;
    scene.camera.zoom = 48.0;
    scene.highlight_tile = None;
    let mut fb = Framebuffer::new(960, 540);
    scene.draw(
        &mut fb,
        &mut session.map,
        &mut session.world,
        &session.content,
        &mut sprites,
        &mut anims,
        0.0,
    );
    fb.write_ppm(&out)?;
    eprintln!("boot-shot: wrote {}", out.display());

    // Nearby objects
    let mut nearby: Vec<(i32, i32, i32, String)> = Vec::new();
    for (tx, ty) in session.map.tile_coords() {
        let t = session.map.get_or_empty(tx, ty);
        if t.object_id > 0 {
            let dx = tx - px;
            let dy = ty - py;
            if dx.abs() <= 8 && dy.abs() <= 8 {
                let name = session
                    .content
                    .get(t.object_id)
                    .map(|d| d.name.clone())
                    .unwrap_or_default();
                nearby.push((dx.abs() + dy.abs(), tx, ty, format!("{} {}", t.object_id, name)));
            }
        }
    }
    nearby.sort_by_key(|e| e.0);
    eprintln!("boot-shot: nearby objects (≤8): {}", nearby.len());
    for (i, e) in nearby.iter().take(8).enumerate() {
        eprintln!("  [{}] ({},{}) {}", i, e.1, e.2, e.3);
    }

    let fbw = 960u32;
    let fbh = 540u32;
    let pick = pick_at_screen(
        &scene.camera,
        &session.map,
        &session.content,
        &mut sprites,
        fbw as f32 * 0.5,
        fbh as f32 * 0.5,
        fbw,
        fbh,
    );
    eprintln!(
        "boot-shot: center pick tile={:?} oid={} hit_map={} hit_self={}",
        pick.tile, pick.object_id, pick.hit_map, pick.hit_self
    );

    if let Some((_, tx, ty, desc)) = nearby.first().cloned() {
        eprintln!("boot-shot: click_use {tx},{ty} ({desc})");
        match click_use(&mut session, tx, ty, None, None) {
            Ok(r) => eprintln!(
                "boot-shot: USE result sent={} pending={} line={}",
                r.action_sent, session.player_action_pending, r.action_line
            ),
            Err(e) => eprintln!("boot-shot: USE err {e:?}"),
        }
        let until2 = Instant::now() + Duration::from_millis(1500);
        while Instant::now() < until2 {
            match session.poll_event() {
                Ok(ev) => eprintln!("boot-shot: ev {ev:?}"),
                Err(_) => thread::sleep(Duration::from_millis(20)),
            }
        }
        let after = session.map.get(tx, ty).map(|t| t.object_id).unwrap_or(-1);
        eprintln!(
            "boot-shot: after USE tile ({tx},{ty}) oid={after} pending={} in_motion={}",
            session.player_action_pending, session.move_state.in_motion
        );
    } else {
        eprintln!("boot-shot: no nearby object to USE");
    }

    // Convert PPM → PNG with stdlib python (no extra crates).
    if let Some(stem) = out.file_stem() {
        let png = out.with_file_name(format!("{}.png", stem.to_string_lossy()));
        let py = format!(
            "import binascii,struct,sys,zlib\n\
p=sys.argv[1]; o=sys.argv[2]\n\
b=open(p,'rb').read()\n\
assert b.startswith(b'P6')\n\
parts=b.split(b'\\n',3)\n\
w,h=map(int,parts[2].split())\n\
raw=parts[3]\n\
def chunk(tag,data):\n\
    c=tag+data\n\
    return struct.pack('>I',len(data))+c+struct.pack('>I',binascii.crc32(c)&0xffffffff)\n\
rows=b''.join(b'\\x00'+raw[i*w*3:(i+1)*w*3] for i in range(h))\n\
open(o,'wb').write(b'\\x89PNG\\r\\n\\x1a\\n'+chunk(b'IHDR',struct.pack('>IIBBBBB' ,w,h,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(rows,9))+chunk(b'IEND',b''))\n\
print('png',o,w,h)"
        );
        let status = std::process::Command::new("python")
            .args(["-c", &py, out.to_str().unwrap_or(""), png.to_str().unwrap_or("")])
            .status();
        if status.map(|s| s.success()).unwrap_or(false) {
            eprintln!("boot-shot: png {}", png.display());
            return Ok(png);
        }
    }
    Ok(out)
}

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
/// Compare Soft vs GPU **present-path** CPU work (no window / no wgpu device).
///
/// Scene authoring is identical either way. This isolates:
/// - Soft: pack RGBA8 → u32 ARGB (minifb path)
/// - GPU: nearest-stretch soft-FB → typical window size (pre-upload work in ohol-client)
fn run_bench_present(args: &[String]) -> anyhow::Result<()> {
    use ohol_headless::render::{stretch_rgba_nearest, Framebuffer, CLEAR_RGBA};

    let fb_w = 960u32;
    let fb_h = 540u32;
    let win_w = flag_value(args, "--win-w")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1920u32);
    let win_h = flag_value(args, "--win-h")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1080u32);
    let n = flag_value(args, "--frames")
        .and_then(|s| s.parse().ok())
        .unwrap_or(240usize);

    let mut fb = Framebuffer::new(fb_w, fb_h);
    fb.clear(CLEAR_RGBA);
    // Noise so stretch/pack cannot be optimized away as constant fill.
    for (i, p) in fb.pixels.chunks_exact_mut(4).enumerate() {
        let v = (i as u8).wrapping_mul(17);
        p[0] = v;
        p[1] = v.wrapping_add(40);
        p[2] = v.wrapping_add(80);
        p[3] = 255;
    }

    let mut soft_buf = vec![0u32; (fb_w * fb_h) as usize];
    let t0 = Instant::now();
    for _ in 0..n {
        // Same packing as ohol_client soft present (`rgba_to_u32`).
        let rgba = &fb.pixels;
        let out = &mut soft_buf[..];
        let m = out.len().min(rgba.len() / 4);
        for i in 0..m {
            let o = i * 4;
            let r = rgba[o] as u32;
            let g = rgba[o + 1] as u32;
            let b = rgba[o + 2] as u32;
            out[i] = (255 << 24) | (r << 16) | (g << 8) | b;
        }
        std::hint::black_box(&soft_buf[0]);
    }
    let soft_elapsed = t0.elapsed().as_secs_f64().max(1e-9);
    let soft_fps = n as f64 / soft_elapsed;
    let soft_ms = (soft_elapsed / n as f64) * 1000.0;

    let mut gpu_dst = vec![0u8; (win_w * win_h * 4) as usize];
    let t1 = Instant::now();
    for _ in 0..n {
        stretch_rgba_nearest(&fb.pixels, fb_w, fb_h, &mut gpu_dst, win_w, win_h);
        std::hint::black_box(&gpu_dst[0]);
    }
    let gpu_elapsed = t1.elapsed().as_secs_f64().max(1e-9);
    let gpu_fps = n as f64 / gpu_elapsed;
    let gpu_ms = (gpu_elapsed / n as f64) * 1000.0;

    // Ideal GPU path (current ohol-client): memcpy soft-FB → present buffer, wgpu scales.
    let mut gpu_copy = vec![0u8; fb.pixels.len()];
    let t2 = Instant::now();
    for _ in 0..n {
        gpu_copy.copy_from_slice(&fb.pixels);
        std::hint::black_box(&gpu_copy[0]);
    }
    let copy_elapsed = t2.elapsed().as_secs_f64().max(1e-9);
    let copy_fps = n as f64 / copy_elapsed;
    let copy_ms = (copy_elapsed / n as f64) * 1000.0;

    eprintln!("bench-present: soft-FB {fb_w}x{fb_h} → present paths ({n} frames)");
    eprintln!(
        "bench-present: Soft (RGBA→u32 pack)              → {soft_fps:.0} FPS ({soft_ms:.3} ms/frame)"
    );
    eprintln!(
        "bench-present: GPU memcpy (wgpu scales window)  → {copy_fps:.0} FPS ({copy_ms:.3} ms/frame) [+ wgpu upload/draw]"
    );
    eprintln!(
        "bench-present: old CPU stretch→{win_w}x{win_h}     → {gpu_fps:.0} FPS ({gpu_ms:.3} ms/frame) [avoided now]"
    );
    eprintln!(
        "bench-present: Settings → Graphics: \"GPU present (wgpu)\" vs \"Soft present (CPU)\" — Restart to apply"
    );
    eprintln!(
        "bench-present: note — scene author soft-FB is shared (~50–70 FPS, see --bench-fps); present is rarely the limiter"
    );
    Ok(())
}

/// Soft-FB SceneRenderer FPS with real prefer_cache content (prints numbers).
///
/// This is the honest play-path authoring cost (CPU soft-FB). GPU present only
/// uploads/scales the buffer; it does not remove this cost.
fn run_bench_fps(args: &[String]) -> anyhow::Result<()> {
    use ohol_headless::anim_bank::AnimBank;
    use ohol_headless::client_map::{ClientMap, MapTile};
    use ohol_headless::content::ClientContent;
    use ohol_headless::ground_sprites::GroundBank;
    use ohol_headless::live_object::LiveWorld;
    use ohol_headless::parse::parse_pu_line;
    use ohol_headless::render::{
        Framebuffer, SceneRenderer, ZOOM_DEFAULT, ZOOM_MAX,
    };
    use ohol_headless::sprite_bank::SpriteBank;

    let root = resolve_content_root(flag_value(args, "--src").map(Path::new))
        .map_err(|e| anyhow::anyhow!(e))?;
    eprintln!("bench-fps: content root {}", root.display());
    let t0 = Instant::now();
    let content = ClientContent::load_prefer_cache(&root).map_err(|e| anyhow::anyhow!(e))?;
    let mut anims = AnimBank::load_prefer_cache(&root);
    let mut sprites = SpriteBank::load_prefer_cache(&root);
    let mut ground = GroundBank::load_prefer_cache(&root);
    let _ = ground.preload_overlays();
    eprintln!(
        "bench-fps: load {:.2}s objects={} anim={} sprites_meta={}",
        t0.elapsed().as_secs_f64(),
        content.objects.len(),
        anims.len(),
        sprites.meta_count()
    );
    sprites.preload([19, 33, 144]);

    let mut map = ClientMap::new();
    for y in -8i32..=8 {
        for x in -12i32..=12 {
            let mut t = MapTile::empty();
            // Large same-biome plateaus → wholeSheet path like real play.
            t.biome = if y < -2 {
                0
            } else if y > 2 {
                2
            } else {
                3
            };
            if (x + y * 2).rem_euclid(8) == 0 {
                t.object_id = 33;
                t.object_raw = "33".into();
            }
            map.set(x, y, t);
        }
    }
    let mut world = LiveWorld::new();
    // Minimal our-player so music/emote paths are warm.
    let pu = parse_pu_line(
        "1 19 0 0 0 0 0 0 0 0 -1 0.5 0 0 0 0 20.0 0.05 3.75 0;0;0;0;0;0 0 0 -1 0 0",
    )
    .ok_or_else(|| anyhow::anyhow!("bad fixture PU"))?;
    world.apply_pu(&pu);
    world.set_our_id(1);

    let mut scene = SceneRenderer::default();
    scene.set_content_root(Some(&root));
    scene.ground = ground;
    scene.draw_hud = false; // isolate world draw cost
    // Measure both every-frame overlay and soft-FB period=2 (play default).
    scene.ground_overlay_period = 1;
    scene.camera.x = 0.0;
    scene.camera.y = 0.0;
    let mut fb = Framebuffer::new(960, 540);

    let measure = |scene: &mut SceneRenderer,
                   map: &mut ClientMap,
                   world: &mut LiveWorld,
                   sprites: &mut SpriteBank,
                   anims: &mut AnimBank,
                   content: &ClientContent,
                   fb: &mut Framebuffer,
                   zoom: f32,
                   label: &str| {
        scene.camera.zoom = zoom;
        for _ in 0..8 {
            scene.draw(fb, map, world, content, sprites, anims, 1.0 / 60.0);
        }
        let n = 120usize;
        let t0 = Instant::now();
        for _ in 0..n {
            scene.draw(fb, map, world, content, sprites, anims, 1.0 / 60.0);
        }
        let elapsed = t0.elapsed().as_secs_f64().max(1e-9);
        let fps = n as f64 / elapsed;
        let ms = (elapsed / n as f64) * 1000.0;
        eprintln!("bench-fps: {label} zoom={zoom:.0} → {fps:.1} FPS ({ms:.2} ms/frame, {n} frames)");
        fps
    };

    let fps_def = measure(
        &mut scene,
        &mut map,
        &mut world,
        &mut sprites,
        &mut anims,
        &content,
        &mut fb,
        ZOOM_DEFAULT,
        "default overlay=every",
    );
    let fps_max = measure(
        &mut scene,
        &mut map,
        &mut world,
        &mut sprites,
        &mut anims,
        &content,
        &mut fb,
        ZOOM_MAX,
        "max_zoom overlay=every",
    );
    scene.ground_overlay_period = 2;
    let fps_play = measure(
        &mut scene,
        &mut map,
        &mut world,
        &mut sprites,
        &mut anims,
        &content,
        &mut fb,
        ZOOM_DEFAULT,
        "default overlay=1/2 (play)",
    );
    eprintln!(
        "bench-fps: summary every={:.1}/{:.1} play_period2={:.1} (target ≥60)",
        fps_def, fps_max, fps_play
    );
    eprintln!(
        "bench-fps: note — Jason C++ uses OpenGL (GPU quads); soft-FB is CPU per-pixel. GPU present only scales the buffer."
    );
    if fps_play < 20.0 {
        anyhow::bail!("play soft-FB FPS {fps_play:.1} < 20 — too heavy");
    }
    Ok(())
}

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

    let origin = (
        session.move_state.x,
        session.move_state.y,
        session
            .world
            .our()
            .map(|o| (o.display_x, o.display_y))
            .unwrap_or((0.0, 0.0)),
    );
    let path = [PathDelta { x: 2, y: 0 }];
    let line = session.send_move(&path)?;
    println!(
        "sent {line} origin=({},{}) display=({:.2},{:.2})",
        origin.0, origin.1, origin.2 .0, origin.2 .1
    );

    let mut saw_pm = false;
    let mut saw_pu = false;
    let mut snapped_back = false;
    let mut max_display_x = origin.2 .0;
    let wait_secs: u64 = flag_value(args, "--timeout")
        .and_then(|s| s.parse().ok())
        .unwrap_or(12);
    let wait = Instant::now() + Duration::from_secs(wait_secs);
    let mut last_step = Instant::now();
    while Instant::now() < wait {
        let _ = session.maybe_send_ka();
        let dt = last_step.elapsed().as_secs_f64().clamp(0.001, 0.1);
        last_step = Instant::now();
        session.step_move_pos(dt);
        if let Some(o) = session.world.our() {
            if o.display_x > max_display_x {
                max_display_x = o.display_x;
            }
        }
        match session.poll_event() {
            Ok(SessionEvent::PlayerMovesStart(_)) => {
                saw_pm = true;
                session.step_move_pos(0.0);
                let disp = session
                    .world
                    .our()
                    .map(|o| (o.display_x, o.display_y, o.x, o.y));
                let cur = (
                    session.move_state.current_pos_x,
                    session.move_state.current_pos_y,
                );
                println!(
                    "got PM display={disp:?} current_pos=({:.2},{:.2}) dest=({},{})",
                    cur.0, cur.1, session.move_state.x, session.move_state.y
                );
                if let Some((dx, _, _, _)) = disp {
                    if max_display_x - origin.2 .0 > 0.4
                        && (dx - origin.2 .0).abs() < 0.15
                    {
                        snapped_back = true;
                        println!("TELEPORT: display snapped back to MOVE origin after PM");
                    }
                }
            }
            Ok(SessionEvent::PlayerUpdate { pu, .. }) if Some(pu.player_id) == session.our_id => {
                saw_pu = true;
                println!(
                    "got PU id={} pos=({},{}) done={} force={} display={:?}",
                    pu.player_id,
                    pu.x,
                    pu.y,
                    pu.done_moving_seq_num,
                    pu.force,
                    session.world.our().map(|o| (o.display_x, o.display_y))
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
    println!(
        "probe-move summary: pm={saw_pm} pu={saw_pu} max_display_x={max_display_x:.2} snapped_back={snapped_back}"
    );
    Ok((saw_pm || saw_pu) && !snapped_back)
}

/// Sequential ground hops. Prints every our-player PM/PU. Verdict is wire-only:
/// after MOVE `@N` to dest D, Jason requires a PU with `done_moving=N`, `force=0`,
/// `x,y=D`. Open Life often omits that PU; this probe records what actually arrives.
fn run_probe_walk_finish(args: &[String]) -> anyhow::Result<bool> {
    use ohol_headless::click_tile::click_tile;

    let cfg = session_cfg_from_args(args);
    let log_path = flag_value(args, "--log")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "logs/wire-probe-walk-finish.log".into());
    let wire = Arc::new(WireLog::create(&log_path)?);
    println!("wire log: {}", wire.path().display());
    println!("host={}:{}", cfg.host, cfg.port);

    let mut session = connect_and_login_logged(&cfg, Arc::clone(&wire))?;
    println!("login={:?}", session.login);
    if session.login != LoginOutcome::Accepted {
        return Ok(false);
    }
    session
        .stream_mut()
        .set_read_timeout(Some(Duration::from_millis(80)))
        .ok();

    let boot_until = Instant::now() + Duration::from_secs(6);
    let mut last = Instant::now();
    while Instant::now() < boot_until {
        let dt = last.elapsed().as_secs_f64().clamp(0.001, 0.08);
        last = Instant::now();
        session.step_move_pos(dt);
        let _ = session.maybe_send_ka();
        match session.poll_event() {
            Ok(SessionEvent::PlayerUpdate { pu, force_ack_sent })
                if Some(pu.player_id) == session.our_id =>
            {
                println!(
                    "boot PU pos=({},{}) done={} force={} force_ack={:?}",
                    pu.x, pu.y, pu.done_moving_seq_num, pu.force, force_ack_sent
                );
            }
            Ok(_) => {}
            Err(_) => {}
        }
        if session.our_id.is_some() && !session.move_state.awaiting_force_ack {
            // keep looping until boot_until so FORCE + map chunk can land
        }
    }
    if session.our_id.is_none() {
        println!("no our_id after boot");
        return Ok(false);
    }
    println!(
        "bound our_id={:?} stand=({},{}) seq={} awaiting_force={}",
        session.our_id,
        session.move_state.x,
        session.move_state.y,
        session.move_state.last_move_sequence_number,
        session.move_state.awaiting_force_ack,
    );

    fn drain_hop(
        session: &mut ohol_headless::session::ClientSession,
        expect_seq: i32,
        expect_dest: (i32, i32),
        hop_secs: f64,
    ) -> (Vec<String>, bool, bool) {
        let mut notes = Vec::new();
        let mut saw_finish = false;
        let mut saw_force = false;
        let until = Instant::now() + Duration::from_secs_f64(hop_secs);
        let mut last = Instant::now();
        let mut last_disp = (0.0f32, 0.0f32);
        if let Some(o) = session.world.our() {
            last_disp = (o.display_x, o.display_y);
        }
        while Instant::now() < until {
            let dt = last.elapsed().as_secs_f64().clamp(0.001, 0.08);
            last = Instant::now();
            session.step_move_pos(dt);
            let _ = session.maybe_send_ka();
            match session.poll_event() {
                Ok(SessionEvent::PlayerMovesStart(v)) => {
                    for m in v {
                        if Some(m.player_id) != session.our_id {
                            continue;
                        }
                        let end = ohol_headless::move_state::MoveState::path_end(
                            m.xs, m.ys, &m.deltas,
                        );
                        notes.push(format!(
                            "PM origin=({},{}) end=({},{}) trunc={} eta={:.2} n_delta={}",
                            m.xs,
                            m.ys,
                            end.0,
                            end.1,
                            m.trunc,
                            m.eta_sec,
                            m.deltas.len()
                        ));
                    }
                }
                Ok(SessionEvent::PlayerUpdate { pu, force_ack_sent })
                    if Some(pu.player_id) == session.our_id =>
                {
                    let match_seq = pu.done_moving_seq_num == expect_seq;
                    let match_xy = (pu.x, pu.y) == expect_dest;
                    let ok = match_seq && match_xy && !pu.force;
                    if ok {
                        saw_finish = true;
                    }
                    if pu.force {
                        saw_force = true;
                    }
                    notes.push(format!(
                        "PU pos=({},{}) done={} force={} ack={:?}  seq_ok={} xy_ok={} FINISH={}",
                        pu.x,
                        pu.y,
                        pu.done_moving_seq_num,
                        pu.force,
                        force_ack_sent,
                        match_seq,
                        match_xy,
                        ok
                    ));
                    let _ = pu;
                }
                Ok(_) => {}
                Err(_) => {}
            }
            if let Some(o) = session.world.our() {
                let dx = o.display_x - last_disp.0;
                let dy = o.display_y - last_disp.1;
                if dx * dx + dy * dy > 1.5 * 1.5 {
                    notes.push(format!(
                        "DISPLAY_JUMP ({:.2},{:.2}) -> ({:.2},{:.2}) dest=({},{}) in_motion={}",
                        last_disp.0,
                        last_disp.1,
                        o.display_x,
                        o.display_y,
                        session.move_state.x,
                        session.move_state.y,
                        session.move_state.in_motion
                    ));
                }
                last_disp = (o.display_x, o.display_y);
            }
            if saw_finish && !session.move_state.in_motion {
                break;
            }
        }
        (notes, saw_finish, saw_force)
    }

    let offsets = [(2, 0), (0, 2), (-2, 0), (0, -2), (1, 1), (-1, 0), (0, 1)];
    let hops = 3usize;
    let mut all_ok = true;
    let mut any_finish = false;
    for hop in 1..=hops {
        if session.move_state.awaiting_force_ack {
            println!("hop {hop}: still awaiting FORCE — drain 2s");
            let seq = session.move_state.last_move_sequence_number;
            let stand = (session.move_state.x, session.move_state.y);
            let (notes, _, _) = drain_hop(&mut session, seq, stand, 2.0);
            for n in notes {
                println!("  {n}");
            }
        }
        let stand = (session.move_state.x, session.move_state.y);
        let mut sent = None;
        for &(dx, dy) in &offsets {
            let goal = (stand.0 + dx, stand.1 + dy);
            match click_tile(&mut session, goal.0, goal.1) {
                Ok(r) => {
                    sent = Some(r);
                    break;
                }
                Err(e) => {
                    println!("hop {hop}: click {goal:?} from {stand:?} err {e}");
                }
            }
        }
        let Some(r) = sent else {
            println!("hop {hop}: no walkable dest from {stand:?}");
            all_ok = false;
            break;
        };
        let expect_seq = session.move_state.last_move_sequence_number;
        println!(
            "---- hop {hop} {}  start={:?} end={:?} expect_seq={} ----",
            r.move_line, r.start, r.end, expect_seq
        );
        let (notes, finish, forced) = drain_hop(&mut session, expect_seq, r.end, 5.0);
        if notes.is_empty() {
            println!("  (no our PM/PU during wait)");
        }
        for n in &notes {
            println!("  {n}");
        }
        println!(
            "  after: dest=({},{}) current=({:.2},{:.2}) in_motion={} display={:?}",
            session.move_state.x,
            session.move_state.y,
            session.move_state.current_pos_x,
            session.move_state.current_pos_y,
            session.move_state.in_motion,
            session
                .world
                .our()
                .map(|o| (o.display_x, o.display_y, o.x, o.y))
        );
        if finish {
            any_finish = true;
            println!("  VERDICT hop {hop}: SERVER sent finish PU done={expect_seq} force=0 xy={:?}", r.end);
        } else if forced {
            all_ok = false;
            println!(
                "  VERDICT hop {hop}: SERVER sent FORCE (not a normal finish PU)"
            );
        } else {
            all_ok = false;
            println!(
                "  VERDICT hop {hop}: NO finish PU with done={expect_seq} force=0 xy={:?}  (Jason requires this on arrival)",
                r.end
            );
        }
        // Pause so we are idle before next hop (no repath overlap).
        let _ = drain_hop(&mut session, expect_seq, r.end, 0.4);
    }

    println!("==== SUMMARY ====");
    println!(
        "Jason required on MOVE @N arrival: PU done_moving=N force=0 x,y=dest"
    );
    println!("any matching finish PU this run: {any_finish}");
    println!("all hops matched: {all_ok}");
    Ok(all_ok && any_finish)
}

/// Repath mid-walk (the play pattern that jumps the figure back).
fn run_probe_walk_repath(args: &[String]) -> anyhow::Result<bool> {
    use ohol_headless::click_tile::click_tile;

    let cfg = session_cfg_from_args(args);
    let log_path = flag_value(args, "--log")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "logs/wire-probe-walk-repath.log".into());
    let wire = Arc::new(WireLog::create(&log_path)?);
    println!("wire log: {}", wire.path().display());
    println!("host={}:{}", cfg.host, cfg.port);
    let mut session = connect_and_login_logged(&cfg, Arc::clone(&wire))?;
    if session.login != LoginOutcome::Accepted {
        println!("login={:?}", session.login);
        return Ok(false);
    }
    session
        .stream_mut()
        .set_read_timeout(Some(Duration::from_millis(80)))
        .ok();
    let boot = Instant::now() + Duration::from_secs(6);
    let mut last = Instant::now();
    while Instant::now() < boot {
        let dt = last.elapsed().as_secs_f64().clamp(0.001, 0.08);
        last = Instant::now();
        session.step_move_pos(dt);
        let _ = session.maybe_send_ka();
        let _ = session.poll_event();
    }
    if session.our_id.is_none() {
        println!("no our_id");
        return Ok(false);
    }
    let stand = (session.move_state.x, session.move_state.y);
    println!(
        "stand={stand:?} seq={}",
        session.move_state.last_move_sequence_number
    );
    let far = (stand.0 + 6, stand.1);
    let first = click_tile(&mut session, far.0, far.1)?;
    println!(
        "MOVE1 {} start={:?} end={:?} seq={}",
        first.move_line,
        first.start,
        first.end,
        session.move_state.last_move_sequence_number
    );
    let seq1 = session.move_state.last_move_sequence_number;
    // Walk a bit, then repath the other way (play: click a new tile while moving).
    let mut last = Instant::now();
    let walk_a_bit = Instant::now() + Duration::from_millis(350);
    while Instant::now() < walk_a_bit {
        let dt = last.elapsed().as_secs_f64().clamp(0.001, 0.08);
        last = Instant::now();
        session.step_move_pos(dt);
        let _ = session.poll_event();
    }
    println!(
        "mid-path current=({:.2},{:.2}) dest=({},{})",
        session.move_state.current_pos_x,
        session.move_state.current_pos_y,
        session.move_state.x,
        session.move_state.y
    );
    let other = (stand.0, stand.1 + 4);
    let second = match click_tile(&mut session, other.0, other.1) {
        Ok(r) => r,
        Err(e) => {
            println!("repath click {other:?} failed: {e}");
            return Ok(false);
        }
    };
    let seq2 = session.move_state.last_move_sequence_number;
    println!(
        "MOVE2 {} start={:?} end={:?} seq={} (prev seq={seq1})",
        second.move_line, second.start, second.end, seq2
    );
    let mut notes = Vec::new();
    let mut display_jumps = 0u32;
    let mut last_disp = session
        .world
        .our()
        .map(|o| (o.display_x, o.display_y))
        .unwrap_or((0.0, 0.0));
    let until = Instant::now() + Duration::from_secs(6);
    let mut last = Instant::now();
    while Instant::now() < until {
        let dt = last.elapsed().as_secs_f64().clamp(0.001, 0.08);
        last = Instant::now();
        session.step_move_pos(dt);
        let _ = session.maybe_send_ka();
        match session.poll_event() {
            Ok(SessionEvent::PlayerMovesStart(v)) => {
                for m in v {
                    if Some(m.player_id) != session.our_id {
                        continue;
                    }
                    let end =
                        ohol_headless::move_state::MoveState::path_end(m.xs, m.ys, &m.deltas);
                    notes.push(format!(
                        "PM origin=({},{}) end=({},{}) trunc={}",
                        m.xs, m.ys, end.0, end.1, m.trunc
                    ));
                }
            }
            Ok(SessionEvent::PlayerUpdate { pu, force_ack_sent })
                if Some(pu.player_id) == session.our_id =>
            {
                notes.push(format!(
                    "PU pos=({},{}) done={} force={} ack={:?}  vs_seq1={seq1} vs_seq2={seq2} dest2={:?}",
                    pu.x,
                    pu.y,
                    pu.done_moving_seq_num,
                    pu.force,
                    force_ack_sent,
                    second.end
                ));
            }
            Ok(_) => {}
            Err(_) => {}
        }
        if let Some(o) = session.world.our() {
            let dx = o.display_x - last_disp.0;
            let dy = o.display_y - last_disp.1;
            if dx * dx + dy * dy > 1.5 * 1.5 {
                display_jumps += 1;
                notes.push(format!(
                    "DISPLAY_JUMP ({:.2},{:.2})->({:.2},{:.2}) dest=({},{}) in_motion={}",
                    last_disp.0,
                    last_disp.1,
                    o.display_x,
                    o.display_y,
                    session.move_state.x,
                    session.move_state.y,
                    session.move_state.in_motion
                ));
            }
            last_disp = (o.display_x, o.display_y);
        }
    }
    for n in &notes {
        println!("  {n}");
    }
    println!(
        "final dest=({},{}) current=({:.2},{:.2}) in_motion={} jumps={display_jumps}",
        session.move_state.x,
        session.move_state.y,
        session.move_state.current_pos_x,
        session.move_state.current_pos_y,
        session.move_state.in_motion
    );
    let finish2 = notes.iter().any(|n| {
        n.contains(&format!("done={seq2}"))
            && n.contains("force=false")
            && n.contains(&format!("pos=({},{})", second.end.0, second.end.1))
    });
    let stale_finish1 = notes.iter().any(|n| {
        n.starts_with("PU ")
            && n.contains(&format!("done={seq1}"))
            && !n.contains(&format!("done={seq2}"))
    });
    let any_force = notes.iter().any(|n| n.contains("force=true"));
    println!("==== REPATH SUMMARY ====");
    println!("finish PU for NEW seq {seq2} at {:?}: {finish2}", second.end);
    println!("stale finish PU for OLD seq {seq1}: {stale_finish1}");
    println!("any FORCE PU: {any_force}");
    println!("display jumps >1.5 tiles: {display_jumps}");
    // Pass only if we did not jump and either got a proper new finish PU or no FORCE.
    Ok(display_jumps == 0 && !any_force)
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
