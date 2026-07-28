//! `--probe-fix` implementation (structured inbound tags).

use std::sync::Arc;
use std::time::{Duration, Instant};

use ohol_headless::move_state::PathDelta;
use ohol_headless::parse::LoginOutcome;
use ohol_headless::session::{SessionConfig, SessionEvent, connect_and_login_logged};
use ohol_headless::wire_log::WireLog;
use ohol_headless::{note_map_changes, note_names, player_says_contains};

use crate::{env_or, flag_value, has_flag, DEFAULT_HOST, DEFAULT_PORT};

pub fn run(args: &[String]) -> anyhow::Result<bool> {
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
    let mut saw_mx_transform = false;
    let mut saw_mx_moving = false;
    let mut held_after_pickup: Option<i32> = None;
    let mut name_line = String::new();

    let boot_deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < boot_deadline {
        match session.poll_event() {
            Ok(SessionEvent::PlayerUpdate { pu, .. }) if Some(pu.player_id) == session.our_id => {
                held_after_pickup = Some(pu.held_id);
            }
            Ok(ev) => {
                let _ = note_names(&ev, &mut name_line, &mut saw_version_name);
                let _ = note_map_changes(&ev, &mut saw_mx_transform, &mut saw_mx_moving);
            }
            Err(_) => break,
        }
    }
    session.move_state.in_motion = false;
    session.move_state.awaiting_force_ack = false;

    let path = [PathDelta { x: 1, y: 0 }];
    let _ = session.send_move(&path)?;
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(3) {
        match session.poll_event() {
            Ok(SessionEvent::PlayerUpdate { pu, .. }) if Some(pu.player_id) == session.our_id => {
                held_after_pickup = Some(pu.held_id);
                if pu.done_moving_seq_num > 1 && !pu.force {
                    break;
                }
            }
            Ok(ev) => {
                let _ = note_map_changes(&ev, &mut saw_mx_transform, &mut saw_mx_moving);
            }
            Err(_) => continue,
        }
    }
    session.move_state.in_motion = false;

    let (fx, fy) = (session.move_state.x, session.move_state.y);
    let mut held = held_after_pickup.unwrap_or(0);

    if held == 0 {
        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (-1, 0), (0, -1), (1, 1)] {
            let ux = fx + dx;
            let uy = fy + dy;
            let _ = session.send_use(ux, uy, None, None)?;
            let t = Instant::now();
            while t.elapsed() < Duration::from_millis(800) {
                match session.poll_event() {
                    Ok(SessionEvent::PlayerUpdate { pu, .. }) if Some(pu.player_id) == session.our_id => {
                        held = pu.held_id;
                        held_after_pickup = Some(pu.held_id);
                        if pu.held_id != 0 {
                            println!("picked/held id={} at use ({ux},{uy})", pu.held_id);
                        }
                    }
                    Ok(ev) => {
                        let _ = note_map_changes(&ev, &mut saw_mx_transform, &mut saw_mx_moving);
                    }
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
        let drop_x = session.move_state.x;
        let drop_y = session.move_state.y;
        let _ = session.send_drop(drop_x, drop_y, -1)?;
        let t = Instant::now();
        while t.elapsed() < Duration::from_millis(800) {
            match session.poll_event() {
                Ok(SessionEvent::PlayerUpdate { pu, .. }) if Some(pu.player_id) == session.our_id => {
                    held = pu.held_id;
                }
                Ok(ev) => {
                    let _ = note_map_changes(&ev, &mut saw_mx_transform, &mut saw_mx_moving);
                }
                Err(_) => break,
            }
        }
        if held == 0 {
            let _ = session.send_use(drop_x, drop_y, None, None)?;
            let t = Instant::now();
            while t.elapsed() < Duration::from_millis(1200) {
                match session.poll_event() {
                    Ok(SessionEvent::PlayerUpdate { pu, .. }) if Some(pu.player_id) == session.our_id => {
                        if pu.held_id != 0 {
                            pickup_ok = true;
                            held = pu.held_id;
                            println!("pickup OK held={}", pu.held_id);
                        }
                    }
                    Ok(ev) => {
                        let _ = note_map_changes(&ev, &mut saw_mx_transform, &mut saw_mx_moving);
                    }
                    Err(_) => break,
                }
            }
        }
    } else {
        println!("WARN: never held an object — pickup path untested on this spawn");
    }

    let animal_wait = Instant::now() + Duration::from_secs(6);
    while Instant::now() < animal_wait && !saw_mx_moving {
        match session.poll_event() {
            Ok(ev) => {
                let _ = note_map_changes(&ev, &mut saw_mx_transform, &mut saw_mx_moving);
            }
            Err(_) => continue,
        }
    }

    let _ = session.send_say("!CLOSE")?;
    let mut close_ps = false;
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(2) {
        match session.poll_event() {
            Ok(ev) if player_says_contains(&ev, "CLOSE") => {
                close_ps = true;
                println!("CLOSE reply: {}", ev.tag_str());
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

    let health_ok = std::net::TcpStream::connect(format!("{}:{}", cfg.host, cfg.port))
        .map(|s| {
            drop(s);
            true
        })
        .unwrap_or(false);

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

    let pass = saw_version_name && health_ok && web_ok;
    if !pickup_ok {
        println!("note: pickup not verified (no holdable tile found near spawn)");
    }
    if !saw_mx_moving {
        println!("note: no animal-move MX in window (animals may be out of range)");
    }
    Ok(pass)
}
