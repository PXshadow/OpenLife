//! Headless OHOL client CLI for playtesting servers that speak the original protocol.
//!
//! Default target: local Open Life / OHOL game server at `127.0.0.1:8005`.
//!
//! ```text
//! ohol-headless
//! ohol-headless --email a@b.c --password x --account-key KEY --move 1,0 --ka
//! ohol-headless --self-check
//! ```

use std::env;
use std::io::Read;
use std::net::TcpListener;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use ohol_headless::frame::{FrameReader, write_message};
use ohol_headless::login::hmac_sha1_hex;
use ohol_headless::move_state::PathDelta;
use ohol_headless::parse::LoginOutcome;
use ohol_headless::session::{
    SessionConfig, SessionEvent, connect_and_login, connect_and_login_logged,
};
use ohol_headless::wire_log::WireLog;
use ohol_headless::{
    encode_drop, encode_ka, encode_move, encode_remv, encode_self, encode_use,
};
use std::sync::Arc;

/// Stock OHOL / OpenLifeReborn game TCP port.
const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: &str = "8005";

fn usage() {
    eprintln!(
        "Usage:
  ohol-headless [options]              connect to {DEFAULT_HOST}:{DEFAULT_PORT} (default OHOL port)
  ohol-headless --self-check           local fixture peer (no game server)
  ohol-headless --probe-move           login, MOVE, wait for PM/PU (wire log + report)
  --log PATH         wire transcript path (default logs/wire-TIMESTAMP.log)

Credentials (priority: CLI flag > process env > .env file):
  OHOL_HOST, OHOL_PORT, OHOL_EMAIL, OHOL_PASSWORD, OHOL_ACCOUNT_KEY
  Copy .env.example â†’ .env (gitignored) for local secrets.

Options:
  --host HOST        default {DEFAULT_HOST} or OHOL_HOST
  --port PORT        default {DEFAULT_PORT} or OHOL_PORT
  --email EMAIL      default blank_email or OHOL_EMAIL
  --password PASS    default x or OHOL_PASSWORD
  --account-key KEY  default empty or OHOL_ACCOUNT_KEY
  --tutorial N       tutorial map number (default 0)
  --reconnect        send RLOGIN instead of LOGIN
  --no-email-pad     do not pad email field to 80 chars
  --move dx,dy       after ACCEPTED, send one MOVE path step
  --use x,y          send USE x y  (optional --use-id ID [--use-slot I])
  --use-id ID        object id on USE (official client sends when target destID>0)
  --use-slot I       container slot for USE (useOnContained)
  --drop x,y         send DROP x y -1  (or --drop-slot C for clothing 0..5)
  --drop-slot C      clothing slot for DROP (default -1)
  --remv x,y         send REMV x y I  (I from --remv-slot, default -1)
  --remv-slot I      container slot for REMV (default -1 top)
  --self x,y         send SELF x y I  (I from --self-slot, default -1)
  --self-slot I      clothing slot for SELF (default -1)
  --swap x,y         send SWAP x y#
  --sremv x,y        send SREMV (needs --sremv-c C --sremv-i I)
  --probe-actions    login + encode/send USE/DROP/REMV/SELF (wire log)
  --probe-play       login, MOVE, SAY, USE â€” full playtest log
  --probe-fix        version-name, pickup, MX -p_id, animals, !CLOSE
  --say TEXT         send SAY 0 0 TEXT#
  --ka               send KA 0 0
  --timeout SECS     socket timeout (default 10)
"
    );
}

fn main() -> ExitCode {
    // Load local `.env` if present (does not override already-set process env).
    // Missing file is fine â€” CLI flags / system env still work.
    let _ = dotenvy::dotenv();

    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        usage();
        return ExitCode::SUCCESS;
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

    if args.iter().any(|a| a == "--probe-move") {
        return match run_probe_move(&args) {
            Ok(ok) => {
                if ok {
                    println!("probe-move: PASS");
                    ExitCode::SUCCESS
                } else {
                    println!("probe-move: FAIL (see report)");
                    ExitCode::FAILURE
                }
            }
            Err(e) => {
                eprintln!("probe-move FAILED: {e:#}");
                ExitCode::FAILURE
            }
        };
    }

    if args.iter().any(|a| a == "--probe-actions") {
        return match run_probe_actions(&args) {
            Ok(ok) => {
                if ok {
                    println!("probe-actions: PASS");
                    ExitCode::SUCCESS
                } else {
                    println!("probe-actions: FAIL");
                    ExitCode::FAILURE
                }
            }
            Err(e) => {
                eprintln!("probe-actions FAILED: {e:#}");
                ExitCode::FAILURE
            }
        };
    }

    if args.iter().any(|a| a == "--probe-play") {
        return match run_probe_play(&args) {
            Ok(ok) => {
                if ok {
                    println!("probe-play: PASS");
                    ExitCode::SUCCESS
                } else {
                    println!("probe-play: FAIL");
                    ExitCode::FAILURE
                }
            }
            Err(e) => {
                eprintln!("probe-play FAILED: {e:#}");
                ExitCode::FAILURE
            }
        };
    }

    if args.iter().any(|a| a == "--probe-fix") {
        return match run_probe_fix(&args) {
            Ok(ok) => {
                if ok {
                    println!("probe-fix: PASS");
                    ExitCode::SUCCESS
                } else {
                    println!("probe-fix: FAIL");
                    ExitCode::FAILURE
                }
            }
            Err(e) => {
                eprintln!("probe-fix FAILED: {e:#}");
                ExitCode::FAILURE
            }
        };
    }

    // No args or only action flags â†’ local rust server defaults (127.0.0.1:8005).
    match run_live(&args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn flag_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|w| w[0] == name)
        .map(|w| w[1].as_str())
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn run_live(args: &[String]) -> anyhow::Result<ExitCode> {
    let host = flag_value(args, "--host")
        .map(|s| s.to_string())
        .unwrap_or_else(|| env_or("OHOL_HOST", DEFAULT_HOST));
    let port: u16 = flag_value(args, "--port")
        .map(|s| s.to_string())
        .unwrap_or_else(|| env_or("OHOL_PORT", DEFAULT_PORT))
        .parse()
        .map_err(|_| anyhow::anyhow!("bad --port / OHOL_PORT"))?;
    let email = flag_value(args, "--email")
        .map(|s| s.to_string())
        .unwrap_or_else(|| env_or("OHOL_EMAIL", "blank_email"));
    let password = flag_value(args, "--password")
        .map(|s| s.to_string())
        .unwrap_or_else(|| env_or("OHOL_PASSWORD", "x"));
    let account_key = flag_value(args, "--account-key")
        .map(|s| s.to_string())
        .unwrap_or_else(|| env_or("OHOL_ACCOUNT_KEY", ""));
    let tutorial: i32 = flag_value(args, "--tutorial")
        .unwrap_or("0")
        .parse()
        .unwrap_or(0);
    let timeout: u64 = flag_value(args, "--timeout")
        .unwrap_or("10")
        .parse()
        .unwrap_or(10);

    // Default playtest action when none specified: keepalive so a bare `cargo run` probes login.
    let want_move = flag_value(args, "--move");
    let want_ka = has_flag(args, "--ka")
        || (want_move.is_none()
            && flag_value(args, "--use").is_none()
            && flag_value(args, "--drop").is_none()
            && flag_value(args, "--remv").is_none()
            && flag_value(args, "--self").is_none());

    let cfg = SessionConfig {
        host,
        port,
        email: email.clone(),
        password,
        account_key: account_key.clone(),
        tutorial_number: tutorial,
        reconnect: has_flag(args, "--reconnect"),
        pad_email_to_80: !has_flag(args, "--no-email-pad"),
        read_timeout: Duration::from_secs(timeout),
        write_timeout: Duration::from_secs(timeout),
        ..SessionConfig::default()
    };

    // Never print password or full account key.
    let key_hint = if account_key.is_empty() {
        "(empty)".to_string()
    } else {
        let keep = account_key.chars().take(4).collect::<String>();
        format!("{keep}â€¦")
    };
    println!(
        "connecting to {}:{} as {} key={} ...",
        cfg.host, cfg.port, email, key_hint
    );
    let mut session = connect_and_login(&cfg)?;
    println!(
        "SN challenge={} players={}/{} version={}",
        session.hello.challenge,
        session.hello.current_players,
        session.hello.max_players,
        session.hello.required_version
    );
    println!("login outcome: {:?}", session.login);

    match session.login {
        LoginOutcome::Accepted => {}
        other => {
            println!("not playing further: {other:?}");
            // Still success for REJECTED (server reachable); network failures are Err.
            return Ok(ExitCode::SUCCESS);
        }
    }

    // After ACCEPTED the server often pushes MC/PU/FX/FM â€” drain a few frames so our
    // player position can sync before MOVE (best-effort, non-fatal on timeout).
    session.stream_mut().set_read_timeout(Some(Duration::from_millis(300))).ok();
    for _ in 0..32 {
        match session.poll_event() {
            Ok(ev) => println!("server: {ev:?}"),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(e) => {
                println!("post-login read stopped: {e}");
                break;
            }
        }
    }
    session
        .stream_mut()
        .set_read_timeout(Some(Duration::from_secs(timeout)))
        .ok();

    if let Some(mv) = want_move {
        let (dx, dy) = parse_pair(mv)?;
        let line = session.send_move(&[PathDelta { x: dx, y: dy }])?;
        println!("sent {line}");
    }

    if want_ka {
        session.send_ka()?;
        println!("sent {}", encode_ka(0, 0));
    }

    if let Some(p) = flag_value(args, "--use") {
        let (x, y) = parse_pair(p)?;
        let id = flag_value(args, "--use-id").and_then(|s| s.parse().ok());
        let slot = flag_value(args, "--use-slot").and_then(|s| s.parse().ok());
        let line = session.send_use(x, y, id, slot)?;
        println!("sent/queued {line}");
    }
    if let Some(p) = flag_value(args, "--drop") {
        let (x, y) = parse_pair(p)?;
        let c: i32 = flag_value(args, "--drop-slot")
            .and_then(|s| s.parse().ok())
            .unwrap_or(-1);
        let line = session.send_drop(x, y, c)?;
        println!("sent/queued {line}");
    }
    if let Some(p) = flag_value(args, "--remv") {
        let (x, y) = parse_pair(p)?;
        let i: i32 = flag_value(args, "--remv-slot")
            .and_then(|s| s.parse().ok())
            .unwrap_or(-1);
        let line = session.send_remv(x, y, i)?;
        println!("sent/queued {line}");
    }
    if let Some(p) = flag_value(args, "--self") {
        let (x, y) = parse_pair(p)?;
        let i: i32 = flag_value(args, "--self-slot")
            .and_then(|s| s.parse().ok())
            .unwrap_or(-1);
        let line = session.send_self(x, y, i)?;
        println!("sent/queued {line}");
    }
    if let Some(p) = flag_value(args, "--swap") {
        let (x, y) = parse_pair(p)?;
        let line = session.send_object_action(ohol_headless::ObjectAction::Swap { x, y })?;
        println!("sent/queued {line}");
    }
    if let Some(text) = flag_value(args, "--say") {
        let line = session.send_say(text)?;
        println!("sent {line}");
    }

    Ok(ExitCode::SUCCESS)
}

/// Regression probe: version name, ground pickup, MX transform p_id, animal MX move, !CLOSE.
fn run_probe_fix(args: &[String]) -> anyhow::Result<bool> {
    use std::sync::Arc;
    use std::time::Instant;

    let log_path = flag_value(args, "--log")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "logs/wire-probe-fix.log".into());
    let wire = Arc::new(WireLog::create(&log_path)?);
    println!("wire log: {}", wire.path().display());

    let cfg = SessionConfig {
        host: env_or("OHOL_HOST", DEFAULT_HOST),
        port: env_or("OHOL_PORT", DEFAULT_PORT).parse()?,
        email: env_or("OHOL_EMAIL", "blank_email"),
        password: env_or("OHOL_PASSWORD", "x"),
        account_key: env_or("OHOL_ACCOUNT_KEY", ""),
        pad_email_to_80: !has_flag(args, "--no-email-pad"),
        read_timeout: Duration::from_secs(15),
        write_timeout: Duration::from_secs(5),
        ..SessionConfig::default()
    };

    let mut session = connect_and_login_logged(&cfg, Arc::clone(&wire))?;
    println!("login={:?}", session.login);
    if session.login != LoginOutcome::Accepted {
        return Ok(false);
    }
    session
        .stream_mut()
        .set_read_timeout(Some(Duration::from_millis(250)))
        .ok();

    let mut saw_version_name = false;
    let mut saw_mx_transform = false; // MX line with negative p_id
    let mut saw_mx_moving = false; // MX with old_x old_y speed (8+ fields)
    let mut held_after_pickup: Option<i32> = None;
    let mut name_line = String::new();

    // Drain bootstrap: look for NM with version.
    let boot_deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < boot_deadline {
        match session.poll_event() {
            Ok(SessionEvent::Other(s)) if s.starts_with("NM") => {
                let flat = s.replace('\n', " ");
                println!("NM: {flat}");
                name_line = flat.clone();
                // Expect GROKPLAY V0.1.0 style for playtest account.
                if flat.contains("GROKPLAY") && flat.contains('V') {
                    saw_version_name = true;
                }
                if flat.contains(env!("CARGO_PKG_VERSION"))
                    || flat.contains("V0.")
                    || flat.contains("V1.")
                {
                    saw_version_name = true;
                }
            }
            Ok(SessionEvent::PlayerUpdate {
                player_id,
                held_id,
                ..
            }) if Some(player_id) == session.our_id => {
                held_after_pickup = Some(held_id);
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    session.move_state.in_motion = false;
    session.move_state.awaiting_force_ack = false;

    // DROP nothing if empty hands; first USE empty may fail â€” place via god isn't available.
    // Strategy: MOVE, then USE feet (may harvest), then if we hold something DROP then USE pickup.
    let path = [PathDelta { x: 1, y: 0 }];
    let _ = session.send_move(&path)?;
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(3) {
        match session.poll_event() {
            Ok(SessionEvent::PlayerUpdate {
                player_id,
                done_moving_seq_num,
                force,
                held_id,
                ..
            }) if Some(player_id) == session.our_id => {
                held_after_pickup = Some(held_id);
                if done_moving_seq_num > 1 && !force {
                    break;
                }
            }
            Ok(SessionEvent::Other(s)) if s.starts_with("MX") => {
                let line = s.lines().nth(1).unwrap_or("");
                let parts: Vec<_> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    if let Ok(pid) = parts[4].parse::<i32>() {
                        if pid < -1 {
                            saw_mx_transform = true;
                        }
                    }
                }
                if parts.len() >= 8 {
                    saw_mx_moving = true;
                    println!("MX moving: {line}");
                }
            }
            Ok(_) => {}
            Err(_) => continue,
        }
    }
    session.move_state.in_motion = false;

    // If empty-handed, try to USE adjacent tiles for a non-permanent object or transition.
    // Then DROP at feet and USE again to verify bare-hand pickup.
    let (fx, fy) = (session.move_state.x, session.move_state.y);
    let mut held = held_after_pickup.unwrap_or(0);

    // Give server a stick via SAY isn't available â€” try USE on several nearby cells.
    if held == 0 {
        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (-1, 0), (0, -1), (1, 1)] {
            let ux = fx + dx;
            let uy = fy + dy;
            let _ = session.send_use(ux, uy, None, None)?;
            let t = Instant::now();
            while t.elapsed() < Duration::from_millis(800) {
                match session.poll_event() {
                    Ok(SessionEvent::PlayerUpdate {
                        player_id,
                        held_id,
                        ..
                    }) if Some(player_id) == session.our_id => {
                        held = held_id;
                        held_after_pickup = Some(held_id);
                        if held_id != 0 {
                            println!("picked/held id={held_id} at use ({ux},{uy})");
                        }
                    }
                    Ok(SessionEvent::Other(s)) if s.starts_with("MX") => {
                        let line = s.lines().nth(1).unwrap_or("");
                        let parts: Vec<_> = line.split_whitespace().collect();
                        if parts.len() >= 5 {
                            if let Ok(pid) = parts[4].parse::<i32>() {
                                if pid < -1 {
                                    saw_mx_transform = true;
                                    println!("MX transform p_id={pid}: {line}");
                                }
                            }
                        }
                        if parts.len() >= 8 {
                            saw_mx_moving = true;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
            if held != 0 {
                break;
            }
        }
    }

    let mut pickup_ok = false;
    if held != 0 {
        // DROP at feet (offset empty prefer 0,0 relative if world empty under us).
        let dx = session.move_state.x;
        let dy = session.move_state.y;
        // Drop one step east of current (client coords).
        let drop_x = dx;
        let drop_y = dy;
        let _ = session.send_drop(drop_x, drop_y, -1)?;
        let t = Instant::now();
        while t.elapsed() < Duration::from_millis(800) {
            match session.poll_event() {
                Ok(SessionEvent::PlayerUpdate {
                    player_id,
                    held_id,
                    ..
                }) if Some(player_id) == session.our_id => {
                    held = held_id;
                }
                Ok(SessionEvent::Other(s)) if s.starts_with("MX") => {
                    let line = s.lines().nth(1).unwrap_or("");
                    // Drop uses positive p_id.
                    println!("MX after drop: {line}");
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        // Bare-hand USE to pick up again.
        if held == 0 {
            let _ = session.send_use(drop_x, drop_y, None, None)?;
            let t = Instant::now();
            while t.elapsed() < Duration::from_millis(1200) {
                match session.poll_event() {
                    Ok(SessionEvent::PlayerUpdate {
                        player_id,
                        held_id,
                        ..
                    }) if Some(player_id) == session.our_id => {
                        if held_id != 0 {
                            pickup_ok = true;
                            held = held_id;
                            println!("pickup OK held={held_id}");
                        }
                    }
                    Ok(SessionEvent::Other(s)) if s.starts_with("MX") => {
                        let line = s.lines().nth(1).unwrap_or("");
                        let parts: Vec<_> = line.split_whitespace().collect();
                        if parts.len() >= 5 {
                            if let Ok(pid) = parts[4].parse::<i32>() {
                                if pid < -1 {
                                    saw_mx_transform = true;
                                    println!("MX pickup transform: {line}");
                                }
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        }
    } else {
        println!("WARN: never held an object â€” pickup path untested on this spawn");
    }

    // Listen for animal moving MX for a few seconds.
    let animal_wait = Instant::now() + Duration::from_secs(6);
    while Instant::now() < animal_wait && !saw_mx_moving {
        match session.poll_event() {
            Ok(SessionEvent::Other(s)) if s.starts_with("MX") => {
                let line = s.lines().nth(1).unwrap_or("");
                let parts: Vec<_> = line.split_whitespace().collect();
                if parts.len() >= 8 {
                    saw_mx_moving = true;
                    println!("animal/moving MX: {line}");
                }
            }
            Ok(_) => {}
            Err(_) => continue,
        }
    }

    // !CLOSE should disconnect us but leave server healthy.
    let _ = session.send_say("!CLOSE")?;
    let mut close_ps = false;
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(2) {
        match session.poll_event() {
            Ok(SessionEvent::Other(s)) if s.starts_with("PS") => {
                if s.contains("CLOSE") {
                    close_ps = true;
                    println!("CLOSE reply: {}", s.lines().next().unwrap_or(""));
                }
            }
            Ok(_) => {}
            Err(e)
                if e.kind() == std::io::ErrorKind::ConnectionReset
                    || e.kind() == std::io::ErrorKind::UnexpectedEof
                    || e.kind() == std::io::ErrorKind::ConnectionAborted =>
            {
                println!("socket closed after !CLOSE (expected)");
                break;
            }
            Err(_) => continue,
        }
    }

    // Health check: server must still be up.
    let health_ok = std::net::TcpStream::connect(format!(
        "{}:{}",
        cfg.host, cfg.port
    ))
    .map(|s| {
        drop(s);
        true
    })
    .unwrap_or(false);

    // Web health if available.
    let web_ok = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "try { (Invoke-WebRequest http://127.0.0.1:8080/health -UseBasicParsing -TimeoutSec 2).StatusCode -eq 200 } catch { $false }",
        ])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("True"))
        .unwrap_or(false);

    println!("## probe-fix summary");
    println!("  name_line={name_line}");
    println!("  version_name={saw_version_name}");
    println!("  pickup_ok={pickup_ok} held={held}");
    println!("  mx_transform_neg_pid={saw_mx_transform}");
    println!("  mx_moving_animal={saw_mx_moving}");
    println!("  close_ps={close_ps} tcp_still_up={health_ok} web_ok={web_ok}");
    println!("wire log: {}", wire.path().display());

    // Soft requirements: name + transform MX + server alive after CLOSE.
    // Pickup may fail if spawn has no non-permanent objects; still report.
    // Animals may be far from player â€” soft.
    let pass = saw_version_name && health_ok && web_ok;
    if !pickup_ok {
        println!("note: pickup not verified (no holdable tile found near spawn)");
    }
    if !saw_mx_moving {
        println!("note: no animal-move MX in window (animals may be out of range)");
    }
    Ok(pass)
}

/// Full playtest: login â†’ move â†’ say â†’ use at feet; measure reply latency; log wire.
fn run_probe_play(args: &[String]) -> anyhow::Result<bool> {
    use std::sync::Arc;
    use std::time::Instant;

    let log_path = flag_value(args, "--log")
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            format!("logs/wire-play-{ts}.log")
        });
    let wire = Arc::new(WireLog::create(&log_path)?);
    println!("wire log: {}", wire.path().display());

    let cfg = SessionConfig {
        host: env_or("OHOL_HOST", DEFAULT_HOST),
        port: env_or("OHOL_PORT", DEFAULT_PORT).parse()?,
        email: env_or("OHOL_EMAIL", "blank_email"),
        password: env_or("OHOL_PASSWORD", "x"),
        account_key: env_or("OHOL_ACCOUNT_KEY", ""),
        pad_email_to_80: !has_flag(args, "--no-email-pad"),
        read_timeout: Duration::from_secs(15),
        write_timeout: Duration::from_secs(5),
        ..SessionConfig::default()
    };

    let mut session = connect_and_login_logged(&cfg, Arc::clone(&wire))?;
    println!("login={:?}", session.login);
    if session.login != LoginOutcome::Accepted {
        return Ok(false);
    }
    session
        .stream_mut()
        .set_read_timeout(Some(Duration::from_millis(300)))
        .ok();
    let _ = session.drain(80);
    session.move_state.in_motion = false;
    session.move_state.awaiting_force_ack = false;

    // MOVE
    let path = [PathDelta { x: 1, y: 0 }];
    let expected_seq = session.move_state.last_move_sequence_number + 1;
    let mv = session.send_move(&path)?;
    println!("sent {mv}");
    let t_move = Instant::now();
    let mut move_done = false;
    let mut saw_pm = false;
    session
        .stream_mut()
        .set_read_timeout(Some(Duration::from_millis(200)))
        .ok();
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        match session.poll_event() {
            Ok(SessionEvent::PlayerMovesStart(v)) => {
                for m in v {
                    if Some(m.player_id) == session.our_id {
                        saw_pm = true;
                        println!("PM ours {:?}", m.deltas);
                    }
                }
            }
            Ok(SessionEvent::PlayerUpdate {
                player_id,
                done_moving_seq_num,
                force,
                x,
                y,
                ..
            }) if Some(player_id) == session.our_id => {
                println!(
                    "PU ours pos=({x},{y}) done={done_moving_seq_num} force={force} after_ms={}",
                    t_move.elapsed().as_millis()
                );
                if done_moving_seq_num == expected_seq && !force {
                    move_done = true;
                    break;
                }
                if force {
                    session.move_state.in_motion = false;
                    session.move_state.awaiting_force_ack = false;
                }
            }
            Ok(SessionEvent::Other(s)) if s.starts_with("LS") || s.starts_with("PS") => {
                println!("chat/loc {}", s.lines().next().unwrap_or(""));
            }
            Ok(_) => {}
            Err(_) => continue,
        }
    }
    println!(
        "move: pm={saw_pm} done={move_done} ms={}",
        t_move.elapsed().as_millis()
    );
    session.move_state.in_motion = false;
    session.move_state.awaiting_force_ack = false;

    // SAY
    let t_say = Instant::now();
    let say_line = session.send_say("HELLO FROM HEADLESS")?;
    println!("sent {say_line}");
    let mut say_reply_ms = None;
    let mut ls_count = 0usize;
    let mut ls_times = Vec::new();
    let say_deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < say_deadline {
        match session.poll_event() {
            Ok(SessionEvent::Other(s)) => {
                let tag = s.lines().next().unwrap_or("");
                if tag.starts_with("PS") && say_reply_ms.is_none() {
                    say_reply_ms = Some(t_say.elapsed().as_millis());
                    println!("SAY reply PS after_ms={}", say_reply_ms.unwrap());
                    println!("  {tag}");
                }
                if tag.starts_with("LS") {
                    ls_count += 1;
                    ls_times.push(t_say.elapsed().as_millis());
                    println!("LS #{} at_ms={} {}", ls_count, t_say.elapsed().as_millis(), tag);
                }
            }
            Ok(SessionEvent::PlayerUpdate {
                player_id,
                done_moving_seq_num,
                ..
            }) if Some(player_id) == session.our_id => {
                println!("PU during say wait done={done_moving_seq_num}");
            }
            Ok(_) => {}
            Err(_) => continue,
        }
        if say_reply_ms.is_some() && ls_count >= 2 {
            break;
        }
    }

    // USE at feet
    let (x, y) = (session.move_state.x, session.move_state.y);
    let seq_before = session.move_state.last_move_sequence_number;
    let t_use = Instant::now();
    let use_line = session.send_use(x, y, None, None)?;
    println!("sent {use_line} (seq_before={seq_before})");
    let mut use_reply = false;
    let mut use_seq_ok = true;
    let use_deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < use_deadline {
        match session.poll_event() {
            Ok(SessionEvent::PlayerUpdate {
                player_id,
                done_moving_seq_num,
                force,
                x,
                y,
                ..
            }) if Some(player_id) == session.our_id => {
                use_reply = true;
                println!(
                    "USE reply PU pos=({x},{y}) done={done_moving_seq_num} force={force} after_ms={}",
                    t_use.elapsed().as_millis()
                );
                // After a completed MOVE @N, action PUs must not reset done to 1 incorrectly
                // in a way that re-sticks motion â€” force=0 with seq>=last is OK.
                if done_moving_seq_num > 0 && done_moving_seq_num < seq_before && !force {
                    use_seq_ok = false;
                    println!("WARN: done_moving_seq {done_moving_seq_num} < last {seq_before}");
                }
                break;
            }
            Ok(SessionEvent::Other(s)) if s.starts_with("MX") || s.starts_with("FM") => {
                use_reply = true;
                println!(
                    "USE reply {} after_ms={}",
                    s.lines().next().unwrap_or("?"),
                    t_use.elapsed().as_millis()
                );
            }
            Ok(_) => {}
            Err(_) => continue,
        }
    }

    let say_ok = say_reply_ms.map(|m| m < 2000).unwrap_or(false);
    let pass = move_done && saw_pm && use_reply && use_seq_ok;
    println!("## Summary");
    println!("  move_ok={}", move_done && saw_pm);
    println!("  say_reply_ms={say_reply_ms:?} say_fast={say_ok}");
    println!("  ls_count={ls_count} (want â‰¥1 within window)");
    println!("  use_reply={use_reply} use_seq_ok={use_seq_ok}");
    println!("  pass={pass}");
    println!("wire log: {}", wire.path().display());
    Ok(pass)
}

/// Login and exercise object-interaction wire forms (and optional live send).
fn run_probe_actions(args: &[String]) -> anyhow::Result<bool> {
    use ohol_headless::ObjectAction;
    use std::sync::Arc;

    // Pure encode checks first (no server) â€” must match C++ / protocol.txt.
    let samples = [
        ObjectAction::Use {
            x: 0,
            y: 0,
            object_id: None,
            slot: None,
        },
        ObjectAction::Use {
            x: 1,
            y: 2,
            object_id: Some(33),
            slot: None,
        },
        ObjectAction::Use {
            x: 1,
            y: 2,
            object_id: Some(33),
            slot: Some(0),
        },
        ObjectAction::Drop {
            x: 3,
            y: 4,
            clothing_slot: -1,
        },
        ObjectAction::Remv {
            x: 5,
            y: 6,
            slot: -1,
        },
        ObjectAction::Remv {
            x: 5,
            y: 6,
            slot: 0,
        },
        ObjectAction::SelfAct {
            x: 0,
            y: 0,
            clothing_slot: -1,
        },
        ObjectAction::Sremv {
            x: 0,
            y: 0,
            clothing_slot: 5,
            slot: -1,
        },
        ObjectAction::Swap { x: 7, y: 8 },
        ObjectAction::Baby {
            x: 1,
            y: 1,
            player_id: Some(99),
        },
        ObjectAction::Ubaby {
            x: 1,
            y: 1,
            clothing_slot: -1,
            player_id: Some(99),
        },
    ];
    let expected = [
        "USE 0 0#",
        "USE 1 2 33#",
        "USE 1 2 33 0#",
        "DROP 3 4 -1#",
        "REMV 5 6 -1#",
        "REMV 5 6 0#",
        "SELF 0 0 -1#",
        "SREMV 0 0 5 -1#",
        "SWAP 7 8#",
        "BABY 1 1 99#",
        "UBABY 1 1 -1 99#",
    ];
    println!("## Encode check (LivingLifePage / protocol.txt)");
    let mut ok = true;
    for (a, exp) in samples.iter().zip(expected.iter()) {
        let got = a.encode();
        let pass = got == *exp;
        println!("  {}  got={got} expect={exp}", if pass { "OK" } else { "FAIL" });
        ok &= pass;
    }

    // Live: connect, wait stationary, send USE at feet + DROP + SELF + KA with wire log.
    let log_path = flag_value(args, "--log")
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            format!("logs/wire-actions-{ts}.log")
        });
    let wire = Arc::new(WireLog::create(&log_path)?);
    println!("wire log: {}", wire.path().display());

    let host = flag_value(args, "--host")
        .map(|s| s.to_string())
        .unwrap_or_else(|| env_or("OHOL_HOST", DEFAULT_HOST));
    let port: u16 = flag_value(args, "--port")
        .map(|s| s.to_string())
        .unwrap_or_else(|| env_or("OHOL_PORT", DEFAULT_PORT))
        .parse()?;
    let cfg = SessionConfig {
        host,
        port,
        email: env_or("OHOL_EMAIL", "blank_email"),
        password: env_or("OHOL_PASSWORD", "x"),
        account_key: env_or("OHOL_ACCOUNT_KEY", ""),
        pad_email_to_80: !has_flag(args, "--no-email-pad"),
        read_timeout: Duration::from_secs(8),
        write_timeout: Duration::from_secs(5),
        ..SessionConfig::default()
    };

    let mut session = connect_and_login_logged(&cfg, Arc::clone(&wire))?;
    println!("login={:?}", session.login);
    if session.login != LoginOutcome::Accepted {
        println!("encode-only result: {ok} (no live ACCEPTED)");
        return Ok(ok);
    }

    session
        .stream_mut()
        .set_read_timeout(Some(Duration::from_millis(400)))
        .ok();
    let _ = session.drain(64);
    // Clear in-motion / force so actions can send.
    session.move_state.in_motion = false;
    session.move_state.awaiting_force_ack = false;
    let (x, y) = (session.move_state.x, session.move_state.y);
    println!("our_id={:?} pos=({x},{y})", session.our_id);

    // Official: USE on target with id when dest known; at feet without id for bare ground.
    let use_line = session.send_use(x, y, None, None)?;
    println!("sent {use_line}");
    let drop_line = session.send_drop(x, y, -1)?;
    println!("sent {drop_line}");
    let remv_line = session.send_remv(x, y, -1)?;
    println!("sent {remv_line}");
    let self_line = session.send_self(x, y, -1)?;
    println!("sent {self_line}");
    session.send_ka()?;
    println!("sent {}", encode_ka(0, 0));

    // Drain a bit for any MX/PU response (best-effort).
    session
        .stream_mut()
        .set_read_timeout(Some(Duration::from_millis(500)))
        .ok();
    let mut saw_server = 0;
    for _ in 0..40 {
        match session.poll_event() {
            Ok(ev) => {
                saw_server += 1;
                match &ev {
                    SessionEvent::PlayerUpdate { player_id, .. }
                        if Some(*player_id) == session.our_id =>
                    {
                        println!("rx {ev:?}");
                    }
                    SessionEvent::Other(s) if s.starts_with("MX") || s.starts_with("FX") => {
                        println!("rx {}", s.lines().next().unwrap_or("?"));
                    }
                    _ => {}
                }
            }
            Err(_) => break,
        }
    }
    println!("live frames after actions: {saw_server}");
    println!("wire log written: {}", wire.path().display());
    Ok(ok)
}

/// Login, send multi-step MOVE, watch for PM + PU(done_moving=seq) for our player.
/// Returns Ok(true) if expected responses observed.
fn run_probe_move(args: &[String]) -> anyhow::Result<bool> {
    use std::time::Instant;

    let host = flag_value(args, "--host")
        .map(|s| s.to_string())
        .unwrap_or_else(|| env_or("OHOL_HOST", DEFAULT_HOST));
    let port: u16 = flag_value(args, "--port")
        .map(|s| s.to_string())
        .unwrap_or_else(|| env_or("OHOL_PORT", DEFAULT_PORT))
        .parse()?;
    let email = flag_value(args, "--email")
        .map(|s| s.to_string())
        .unwrap_or_else(|| env_or("OHOL_EMAIL", "blank_email"));
    let password = flag_value(args, "--password")
        .map(|s| s.to_string())
        .unwrap_or_else(|| env_or("OHOL_PASSWORD", "x"));
    let account_key = flag_value(args, "--account-key")
        .map(|s| s.to_string())
        .unwrap_or_else(|| env_or("OHOL_ACCOUNT_KEY", ""));
    let wait_secs: u64 = flag_value(args, "--timeout")
        .unwrap_or("8")
        .parse()
        .unwrap_or(8);

    let log_path = flag_value(args, "--log")
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            format!("logs/wire-haxe-{ts}.log")
        });
    let wire = Arc::new(WireLog::create(&log_path)?);
    println!("wire log: {}", wire.path().display());

    let cfg = SessionConfig {
        host,
        port,
        email: email.clone(),
        password,
        account_key,
        pad_email_to_80: !has_flag(args, "--no-email-pad"),
        read_timeout: Duration::from_secs(wait_secs),
        write_timeout: Duration::from_secs(5),
        ..SessionConfig::default()
    };

    let mut report = String::new();
    macro_rules! log {
        ($($t:tt)*) => {{
            let line = format!($($t)*);
            println!("{line}");
            report.push_str(&line);
            report.push('\n');
            wire.note(&line);
        }};
    }

    log!("# Movement probe against {}:{}", cfg.host, cfg.port);
    log!("email={}", email);
    log!("wire_log={}", wire.path().display());
    log!("");

    let mut session = connect_and_login_logged(&cfg, Arc::clone(&wire))?;
    log!("login={:?}", session.login);
    if session.login != LoginOutcome::Accepted {
        log!("ABORT: not ACCEPTED");
        write_probe_report(&report)?;
        return Ok(false);
    }

    // Drain bootstrap with short timeouts so MC binary is consumed correctly.
    session
        .stream_mut()
        .set_read_timeout(Some(Duration::from_millis(500)))
        .ok();
    let bootstrap = session.drain(128);
    for ev in &bootstrap {
        match ev {
            SessionEvent::PlayerUpdate {
                player_id, x, y, done_moving_seq_num, force, ..
            } => log!(
                "bootstrap PU id={player_id} pos=({x},{y}) done={done_moving_seq_num} force={force}"
            ),
            SessionEvent::MapChunk(_) => log!("bootstrap MC (binary skipped)"),
            SessionEvent::Frame => log!("bootstrap FM"),
            SessionEvent::PlayerMovesStart(v) => log!("bootstrap PM count={}", v.len()),
            SessionEvent::Other(s) => {
                let tag = s.lines().next().unwrap_or("?");
                log!("bootstrap {tag}");
            }
            _ => log!("bootstrap {ev:?}"),
        }
    }

    let our_id = session.our_id;
    let start_x = session.move_state.x;
    let start_y = session.move_state.y;
    log!(
        "our_id={our_id:?} pos=({start_x},{start_y}) last_seq={}",
        session.move_state.last_move_sequence_number
    );

    if our_id.is_none() {
        log!("ABORT: could not identify our player from MC center + PU");
        write_probe_report(&report)?;
        return Ok(false);
    }

    // Wait until our player is stationary (done_moving > 0). Bootstrap often arrives mid-path.
    log!("waiting for stationary PU on our player before MOVEâ€¦");
    session
        .stream_mut()
        .set_read_timeout(Some(Duration::from_millis(300)))
        .ok();
    let wait_station_deadline = Instant::now() + Duration::from_secs(wait_secs.min(20));
    let mut stationary = session.move_state.last_move_sequence_number > 0
        && !session.move_state.in_motion;
    // If bound with done=0, last_seq may still be 1 default â€” treat done=0 as not stationary.
    while Instant::now() < wait_station_deadline {
        match session.poll_event() {
            Ok(SessionEvent::PlayerUpdate {
                player_id,
                done_moving_seq_num,
                force,
                x,
                y,
                held_id: _, force_ack_sent,
            }) if Some(player_id) == our_id => {
                log!(
                    "wait PU id={player_id} pos=({x},{y}) done={done_moving_seq_num} force={force} ack={force_ack_sent:?}"
                );
                if done_moving_seq_num > 0 && !force {
                    stationary = true;
                    break;
                }
                if done_moving_seq_num > 0 && force {
                    // force snap â€” after FORCE ack we are stationary at x,y
                    stationary = true;
                    break;
                }
            }
            Ok(SessionEvent::PlayerMovesStart(ref moves)) => {
                for m in moves {
                    if Some(m.player_id) == our_id {
                        log!(
                            "wait PM ours start=({},{}) deltas={:?}",
                            m.xs, m.ys, m.deltas
                        );
                    }
                }
            }
            Ok(_) => {}
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(e) => {
                log!("wait read error: {e}");
                break;
            }
        }
    }
    // Ensure move state allows a new MOVE
    session.move_state.in_motion = false;
    session.move_state.awaiting_force_ack = false;
    let start_x = session.move_state.x;
    let start_y = session.move_state.y;
    log!(
        "stationary={} pos=({},{}) last_seq={}",
        stationary,
        start_x,
        start_y,
        session.move_state.last_move_sequence_number
    );

    // Path: two steps east (relative to start), within Â±16.
    let path = [PathDelta { x: 1, y: 0 }, PathDelta { x: 2, y: 0 }];
    let expected_seq = session.move_state.last_move_sequence_number + 1;
    let dest_x = start_x + 2;
    let dest_y = start_y;
    let move_line = session.send_move(&path)?;
    log!("sent {move_line}");
    log!("expect seq={expected_seq} dest=({dest_x},{dest_y})");    // Listen for PM/PU for several seconds.
    session
        .stream_mut()
        .set_read_timeout(Some(Duration::from_millis(250)))
        .ok();
    let deadline = Instant::now() + Duration::from_secs(wait_secs);

    let mut saw_pm_ours = false;
    let mut saw_pm_any = false;
    let mut pm_lines: Vec<String> = Vec::new();
    let mut saw_pu_done = false;
    let mut saw_pu_force = false;
    let mut pu_ours: Vec<String> = Vec::new();
    let mut other_tags: Vec<String> = Vec::new();

    while Instant::now() < deadline {
        match session.poll_event() {
            Ok(SessionEvent::PlayerMovesStart(moves)) => {
                saw_pm_any = true;
                for m in moves {
                    let line = format!(
                        "PM id={} start=({},{}) total={:.2} eta={:.2} trunc={} deltas={:?}",
                        m.player_id, m.xs, m.ys, m.total_sec, m.eta_sec, m.trunc, m.deltas
                    );
                    log!("{line}");
                    pm_lines.push(line);
                    if our_id == Some(m.player_id) {
                        saw_pm_ours = true;
                    }
                }
            }
            Ok(SessionEvent::PlayerUpdate {
                player_id,
                done_moving_seq_num,
                force,
                x,
                y,
                held_id: _, force_ack_sent,
            }) => {
                let line = format!(
                    "PU id={player_id} pos=({x},{y}) done={done_moving_seq_num} force={force} ack={force_ack_sent:?}"
                );
                if our_id == Some(player_id) || our_id.is_none() {
                    log!("{line}");
                    pu_ours.push(line);
                    if force {
                        saw_pu_force = true;
                    }
                    if done_moving_seq_num == expected_seq
                        || (done_moving_seq_num > 0 && x == dest_x && y == dest_y)
                    {
                        saw_pu_done = true;
                    }
                } else {
                    // keep noise low
                    other_tags.push(format!("PU(other id={player_id})"));
                }
            }
            Ok(SessionEvent::MapChunk(_)) => log!("MC"),
            Ok(SessionEvent::Frame) => {}
            Ok(SessionEvent::Other(s)) => {
                let tag = s.lines().next().unwrap_or("?").to_string();
                if !matches!(tag.as_str(), "HX" | "FX" | "CX") {
                    log!("other {tag}");
                }
                other_tags.push(tag);
            }
            Ok(ev) => log!("ev {ev:?}"),
            Err(e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(e) => {
                log!("read error: {e}");
                break;
            }
        }
        if saw_pm_ours && saw_pu_done {
            break;
        }
    }

    log!("");
    log!("## Checklist (protocol.txt / official client)");
    log!(
        "- [ {} ] LOGIN ACCEPTED",
        if session.login == LoginOutcome::Accepted {
            "x"
        } else {
            " "
        }
    );
    log!(
        "- [ {} ] Knew our player id + start pos before MOVE",
        if our_id.is_some() { "x" } else { " " }
    );
    log!(
        "- [ {} ] Client sent MOVE with @seq (seq starts at 2 for first life move)",
        if move_line.contains(&format!("@{expected_seq}")) {
            "x"
        } else {
            " "
        }
    );
    log!(
        "- [ {} ] Server PM (PLAYER_MOVES_START) for **our** player after MOVE",
        if saw_pm_ours { "x" } else { " " }
    );
    log!(
        "- [ {} ] Any PM at all after MOVE (other players ok)",
        if saw_pm_any { "x" } else { " " }
    );
    log!(
        "- [ {} ] Server PU with done_moving_seqNum matching our seq (or dest reached)",
        if saw_pu_done { "x" } else { " " }
    );
    log!(
        "- [ {} ] PU force for us (only expected on truncated/jump path)",
        if saw_pu_force { "x" } else { " " }
    );
    log!(
        "final client pos=({},{}) in_motion={} last_seq={}",
        session.move_state.x,
        session.move_state.y,
        session.move_state.in_motion,
        session.move_state.last_move_sequence_number
    );

    // Expected original behavior summary
    let pass = saw_pm_ours && saw_pu_done;
    log!("");
    log!(
        "## Verdict: {}",
        if pass {
            "PASS â€” movement responses look correct"
        } else {
            "FAIL â€” missing expected PM and/or done_moving PU for our player"
        }
    );
    if !pass {
        log!("");
        log!("### Gaps");
        if !saw_pm_ours {
            log!("- No PM line for our player id after MOVE (vanilla: server emits PM when walk starts).");
        }
        if !saw_pu_done {
            log!("- No PU with done_moving_seqNum == {expected_seq} (or dest pos) within {wait_secs}s.");
        }
        if !saw_pm_any {
            log!("- No PM messages at all after MOVE (possible: move ignored, wrong seq, or not broadcast).");
        }
    }

    write_probe_report(&report)?;
    // Also materialize feedback for the server team when failing.
    if !pass {
        write_server_feedback(&report, saw_pm_ours, saw_pm_any, saw_pu_done, expected_seq)?;
    }
    Ok(pass)
}

fn write_probe_report(report: &str) -> anyhow::Result<()> {
    let path = std::path::Path::new("move-probe-report.md");
    std::fs::write(path, report)?;
    println!("wrote {}", path.display());
    Ok(())
}

fn write_server_feedback(
    probe: &str,
    saw_pm_ours: bool,
    saw_pm_any: bool,
    saw_pu_done: bool,
    expected_seq: i32,
) -> anyhow::Result<()> {
    let path = std::path::Path::new("SERVER_MOVE_FEEDBACK.md");
    let mut md = String::new();
    md.push_str("# Server movement feedback (headless client probe)\n\n");
    md.push_str("Date: auto-generated by `ohol-headless --probe-move`\n\n");
    md.push_str("## Summary\n\n");
    md.push_str(
        "Logged into the local OpenLifeReborn game port (`127.0.0.1:8005`) with a real account \
and sent an original-format `MOVE` path. The server **did not fully return** the movement \
responses expected by the official OHOL protocol / client.\n\n",
    );
    md.push_str("## Expected (protocol.txt + LivingLifePage)\n\n");
    md.push_str("After client sends:\n\n");
    md.push_str("```text\nMOVE xs ys @seq_num xdelt0 ydelt0 ...#\n```\n\n");
    md.push_str("with `seq_num` starting at **2** for the first move of a life:\n\n");
    md.push_str("1. **`PM` (PLAYER_MOVES_START)** for that player:\n");
    md.push_str("   `p_id xs ys total_sec eta_sec trunc xdelt0 ydelt0 ...`\n");
    md.push_str("2. While walking, other PUs may arrive with `done_moving_seqNum == 0`.\n");
    md.push_str(
        "3. On arrival, **`PU`** with `done_moving_seqNum == seq_num` (and force usually 0), \
stationary at destination.\n",
    );
    md.push_str(
        "4. On truncated/forced path: `force=1` PU + client replies `FORCE x y#` before further moves.\n\n",
    );
    md.push_str("## Observed\n\n");
    md.push_str(&format!(
        "| Check | Result |\n|-------|--------|\n\
         | PM for our player after MOVE | {} |\n\
         | Any PM after MOVE | {} |\n\
         | PU done_moving matching seq {expected_seq} (or dest) | {} |\n\n",
        if saw_pm_ours { "YES" } else { "**NO**" },
        if saw_pm_any { "YES" } else { "**NO**" },
        if saw_pu_done { "YES" } else { "**NO**" },
    ));
    md.push_str("## Client wire (faithful)\n\n");
    md.push_str("- Framing: ASCII messages terminated by `#`\n");
    md.push_str("- MOVE exact spacing: `MOVE xs ys @seq d0x d0y ...#`\n");
    md.push_str("- Login: SN challenge + HMAC-SHA1(password) + HMAC-SHA1(pure account key)\n");
    md.push_str("- MC binary after `#` is skipped by compressed size (frame sync)\n\n");
    md.push_str("## Probe log\n\n");
    md.push_str("```\n");
    md.push_str(probe);
    md.push_str("```\n\n");
    md.push_str("## Suggested server fixes\n\n");
    md.push_str(
        "1. On accepted MOVE intent: broadcast **PM** to interest set (or all if broadcast_all_updates).\n",
    );
    md.push_str(
        "2. When timed path finishes: emit **PU** with `done_moving_seqNum` equal to client `@seq`.\n",
    );
    md.push_str(
        "3. On jump/truncation: PU `force=1` at corrected tile; ignore further pos-sensitive cmds until `FORCE x y#`.\n",
    );
    md.push_str(
        "4. Confirm MOVE is not dropped due to seq, start-tile mismatch, or ticket/session mapping.\n",
    );
    md.push_str("\n## Repro\n\n");
    md.push_str("```bash\ncd C:\\OhOl\\OpenLife\\RustClient\n");
    md.push_str("cargo run -- --probe-move --timeout 8\n```\n");
    md.push_str("(Uses gitignored `.env` credentials; server must listen on :8005.)\n");
    std::fs::write(path, md)?;
    println!("wrote {}", path.display());
    Ok(())
}

fn parse_pair(s: &str) -> anyhow::Result<(i32, i32)> {
    let (a, b) = s
        .split_once(',')
        .ok_or_else(|| anyhow::anyhow!("expected x,y got {s}"))?;
    Ok((a.trim().parse()?, b.trim().parse()?))
}

/// Local fixture peer speaking SN â†’ verify LOGIN HMACs â†’ ACCEPTED, then accept MOVE/actions.
fn run_self_check() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let challenge = "selfcheck_challenge_001";
    let password = "selfcheck_pw";
    let account_key = "SC-AB-CD";
    let pure = ohol_headless::pure_account_key(account_key);
    let exp_pw = hmac_sha1_hex(password, challenge);
    let exp_key = hmac_sha1_hex(&pure, challenge);

    let server = thread::spawn(move || -> anyhow::Result<Vec<String>> {
        let (mut sock, _) = listener.accept()?;
        write_message(
            &mut sock,
            &format!("SN\n0/8\n{challenge}\n184\n"),
        )?;
        let mut fr = FrameReader::new();
        let mut buf = [0u8; 8192];
        let mut received = Vec::new();
        // LOGIN
        let login = read_frame(&mut sock, &mut fr, &mut buf)?;
        received.push(login.clone());
        // With pad_email false for self-check, split_whitespace works.
        let parts: Vec<&str> = login.split_whitespace().collect();
        if parts.len() < 6 {
            anyhow::bail!("LOGIN token count {}: {login}", parts.len());
        }
        // last three: pw key tutorial
        let pw_hash = parts[parts.len() - 3];
        let key_hash = parts[parts.len() - 2];
        if pw_hash != exp_pw {
            anyhow::bail!("password hmac mismatch: {pw_hash} != {exp_pw}");
        }
        if key_hash != exp_key {
            anyhow::bail!("account key hmac mismatch: {key_hash} != {exp_key}");
        }
        write_message(&mut sock, "ACCEPTED\n")?;

        // Read subsequent client messages (MOVE + USE + DROP + REMV + SELF + KA = 6 after LOGIN)
        sock.set_read_timeout(Some(Duration::from_millis(1500))).ok();
        loop {
            match sock.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    for m in fr.push(&buf[..n]) {
                        received.push(m);
                    }
                    // LOGIN + 6 client commands
                    if received.len() >= 7 {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    break;
                }
                Err(e) => return Err(e.into()),
            }
        }
        Ok(received)
    });

    let cfg = SessionConfig {
        host: "127.0.0.1".into(),
        port,
        email: "selfcheck@test.local".into(),
        password: password.into(),
        account_key: account_key.into(),
        pad_email_to_80: false,
        read_timeout: Duration::from_secs(5),
        write_timeout: Duration::from_secs(5),
        ..SessionConfig::default()
    };

    let mut session = connect_and_login(&cfg)?;
    if session.login != LoginOutcome::Accepted {
        anyhow::bail!("expected ACCEPTED, got {:?}", session.login);
    }

    let move_line = session.send_move(&[PathDelta { x: 1, y: 0 }, PathDelta { x: 1, y: 1 }])?;
    assert_eq!(
        move_line,
        encode_move(0, 0, 2, &[PathDelta { x: 1, y: 0 }, PathDelta { x: 1, y: 1 }]).unwrap()
    );

    // Simulate server done_moving so object actions can send
    session
        .move_state
        .on_player_update(2, false, 1, 1);
    assert!(!session.move_state.in_motion);

    let use_line = session.send_use(1, 1, None, None)?;
    let drop_line = session.send_drop(1, 1, -1)?;
    let remv_line = session.send_remv(1, 1, -1)?;
    let self_line = session.send_self(1, 1, -1)?;
    session.send_ka()?;

    println!("client sent MOVE: {move_line}");
    println!("client sent USE:  {use_line}");
    println!("client sent DROP: {drop_line}");
    println!("client sent REMV: {remv_line}");
    println!("client sent SELF: {self_line}");
    println!("client sent KA:   {}", encode_ka(0, 0));

    let received = server.join().map_err(|_| anyhow::anyhow!("server thread panic"))??;
    println!("fixture received {} frames", received.len());
    for (i, m) in received.iter().enumerate() {
        println!("  [{i}] {m}");
    }

    let joined = received.join("\n");
    for expected in [
        "MOVE 0 0 @2 1 0 1 1",
        "USE 1 1",
        "DROP 1 1 -1",
        "REMV 1 1 -1",
        "SELF 1 1 -1",
        "KA 0 0",
    ] {
        if !joined.contains(expected) {
            anyhow::bail!("fixture did not receive {expected:?} in {joined}");
        }
    }

    // Static wire sample dump for evidence
    println!(
        "wire samples:\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}",
        encode_move(10, 20, 2, &[PathDelta { x: -1, y: 0 }]).unwrap(),
        encode_use(0, 0, Some(5), None),
        encode_drop(0, 0, -1),
        encode_remv(0, 0, 0),
        encode_self(0, 0, -1),
        encode_ka(0, 0),
    );

    Ok(())
}

fn read_frame(
    sock: &mut impl Read,
    fr: &mut FrameReader,
    buf: &mut [u8],
) -> anyhow::Result<String> {
    loop {
        let n = sock.read(buf)?;
        if n == 0 {
            anyhow::bail!("eof waiting for frame");
        }
        let msgs = fr.push(&buf[..n]);
        if let Some(m) = msgs.into_iter().next() {
            return Ok(m);
        }
    }
}
