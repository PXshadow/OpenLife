//! MUTE-SAY mute_delivery build-time patch (included from build.rs).
//!
//! Ensures WHISPER respects MuteBook and adds integration test
//! `say_mute_blocks_whisper`. Idempotent. Handles CRLF sources.

use std::path::PathBuf;

fn normalize_nl(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

fn restore_nl(s: &str, crlf: bool) -> String {
    if crlf {
        s.replace('\n', "\r\n")
    } else {
        s.to_string()
    }
}

const WHISPER_OLD: &str = r#"        // WHISPER <p_id> <text> — private PS only to target if online (find conn by p_id).
        if upper.starts_with("WHISPER ") {
            let rest = text
                .split_once(char::is_whitespace)
                .map(|(_, r)| r.trim())
                .unwrap_or("");
            let mut parts = rest.splitn(2, char::is_whitespace);
            let target_id: i32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let whisper_text = parts.next().map(str::trim).unwrap_or("");
            if target_id != 0 && !whisper_text.is_empty() {
                if let Some((&target_conn, _)) = state.players.iter().find(|(_, pl)| {
                    pl.p_id == target_id && pl.connected && !pl.deleted
                }) {
                    // Protocol: PS p_id/0 text + FM (private whisper still uses same wire).
                    send_ps_reply(
                        outbound,
                        target_conn,
                        &format!("{} {}", p.p_id, whisper_text),
                    );
                    info!(
                        conn_id,
                        target_id,
                        target_conn,
                        text = %whisper_text,
                        "sim: WHISPER"
                    );
                }
            }
            return;
        }
"#;

const WHISPER_NEW: &str = r#"        // WHISPER <p_id> <text> — private PS only to target if online (find conn by p_id).
        // Mute filters whispers; DEAF does not (should_hear(deaf, is_whisper=true)).
        // Haxe: Connection.sendSayToAllClose has no mute; product MUTE is Rust-side.
        if upper.starts_with("WHISPER ") {
            let rest = text
                .split_once(char::is_whitespace)
                .map(|(_, r)| r.trim())
                .unwrap_or("");
            let mut parts = rest.splitn(2, char::is_whitespace);
            let target_id: i32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let whisper_text = parts.next().map(str::trim).unwrap_or("");
            if target_id != 0 && !whisper_text.is_empty() {
                let speaker_p_id = p.p_id;
                // Listener muted this speaker → drop privately (no PS to either side).
                if !state.mutes.should_deliver(target_id, speaker_p_id) {
                    info!(
                        conn_id,
                        target_id,
                        speaker_p_id,
                        "sim: WHISPER dropped (muted)"
                    );
                    return;
                }
                if let Some((&target_conn, _)) = state.players.iter().find(|(_, pl)| {
                    pl.p_id == target_id && pl.connected && !pl.deleted
                }) {
                    // Protocol: PS p_id/0 text + FM (private whisper still uses same wire).
                    send_ps_reply(
                        outbound,
                        target_conn,
                        &format!("{} {}", speaker_p_id, whisper_text),
                    );
                    info!(
                        conn_id,
                        target_id,
                        target_conn,
                        text = %whisper_text,
                        "sim: WHISPER"
                    );
                }
            }
            return;
        }
"#;

const NEARBY_DOC_OLD: &str = r#"/// Normal SAY / SHOUT / MUMBLE PS fan-out: skip muted listeners and DEAF players.
///
/// Whispers use a private path and are not filtered by DEAF (see WHISPER handler).
/// Packet must already be full wire bytes for one PS (prefer [`format_player_says`]).
"#;

const NEARBY_DOC_NEW: &str = r#"/// Normal SAY / SHOUT / MUMBLE PS fan-out: skip muted listeners and DEAF players.
///
/// Whispers use a private path: muted listeners are skipped; DEAF does not block
/// whispers (`should_hear(deaf, true)`). Prefer live path [`send_chat_ps`].
/// Packet must already be full wire bytes for one PS (prefer [`format_player_says`]).
"#;

const TEST_ANCHOR: &str = r#"    /// SAY DEAF toggles Player.deaf; blocks normal chat; WHISPER still delivers.
    #[test]
    fn say_deaf_blocks_chat_allows_whisper() {
"#;

const TEST_INSERT: &str = r#"    /// MUTE also blocks WHISPER (unlike DEAF).
    #[test]
    fn say_mute_blocks_whisper() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        let a = spawn_player(&mut state, 1, "speaker@x");
        let b = spawn_player(&mut state, 2, "listener@x");
        set_player_position(&mut state, 1, 0, 0);
        set_player_position(&mut state, 2, 1, 0);
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 2,
                tag: "SAY".into(),
                payload: format!("MUTE {a}"),
            },
        );
        while rx2.try_recv().is_ok() {}
        assert!(!state.mutes.should_deliver(b, a));

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("WHISPER {b} secret muted"),
            },
        );
        let mut b_heard = false;
        while let Ok(pkt) = rx2.try_recv() {
            if String::from_utf8_lossy(&pkt).contains("secret muted") {
                b_heard = true;
            }
        }
        assert!(!b_heard, "muted listener must not receive WHISPER");
        assert!(rx1.try_recv().is_err(), "whisperer must not get echo when dropped");
        let _ = a;
        let _ = b;
    }

    /// SAY DEAF toggles Player.deaf; blocks normal chat; WHISPER still delivers.
    #[test]
    fn say_deaf_blocks_chat_allows_whisper() {
"#;

/// Wire WHISPER mute filter + mute whisper test. Returns true if wired after call.
pub fn patch_lib_mute_say(lib_path: &PathBuf) -> bool {
    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);
    let orig = text.clone();

    if text.contains(WHISPER_OLD) {
        text = text.replacen(WHISPER_OLD, WHISPER_NEW, 1);
    }

    if text.contains(NEARBY_DOC_OLD) {
        text = text.replacen(NEARBY_DOC_OLD, NEARBY_DOC_NEW, 1);
    }

    if !text.contains("fn say_mute_blocks_whisper()") && text.contains(TEST_ANCHOR) {
        text = text.replacen(TEST_ANCHOR, TEST_INSERT, 1);
    }

    if text == orig {
        return mute_say_wired_text(&text);
    }
    let out = restore_nl(&text, crlf);
    if std::fs::write(lib_path, out).is_ok() {
        mute_say_wired(lib_path)
    } else {
        false
    }
}

fn mute_say_wired_text(t: &str) -> bool {
    t.contains("WHISPER dropped (muted)")
        && t.contains("fn say_mute_blocks_whisper()")
        && t.contains("state.mutes.should_deliver(listener.p_id, speaker_p_id)")
}

/// True if mute delivery is fully wired in lib.rs.
pub fn mute_say_wired(lib_path: &PathBuf) -> bool {
    std::fs::read_to_string(lib_path)
        .map(|t| mute_say_wired_text(&normalize_nl(&t)))
        .unwrap_or(false)
}
