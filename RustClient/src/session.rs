//! TCP session: connect, SN→LOGIN, ACCEPTED/REJECTED, send MOVE/actions.

use std::io::{self, Read};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::actions::{ObjectAction, encode_ka, encode_say};
use crate::frame::{FrameReader, write_message};
use crate::login::{LoginParams, encode_login};
use crate::move_state::{MoveError, MoveState, PathDelta};
use crate::parse::{
    LoginOutcome, PlayerMoveStart, ServerHello, message_type, parse_login_outcome, parse_pm_message,
    parse_pu_line, parse_sn,
};
use crate::wire_log::WireLog;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub host: String,
    pub port: u16,
    pub email: String,
    pub password: String,
    pub account_key: String,
    pub tutorial_number: i32,
    pub reconnect: bool,
    pub client_tag: String,
    pub pad_email_to_80: bool,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8005,
            email: "blank_email".into(),
            password: "x".into(),
            account_key: String::new(),
            tutorial_number: 0,
            reconnect: false,
            client_tag: crate::login::DEFAULT_CLIENT_TAG.into(),
            pad_email_to_80: true,
            read_timeout: Duration::from_secs(10),
            write_timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Debug)]
pub enum SessionEvent {
    Hello(ServerHello),
    Login(LoginOutcome),
    PlayerUpdate {
        player_id: i32,
        done_moving_seq_num: i32,
        force: bool,
        x: i32,
        y: i32,
        force_ack_sent: Option<String>,
    },
    PlayerMovesStart(Vec<PlayerMoveStart>),
    /// Map chunk header (binary payload already consumed by frame reader).
    MapChunk(String),
    Frame,
    Other(String),
}

/// Connected headless client session.
pub struct ClientSession {
    stream: TcpStream,
    frames: FrameReader,
    /// Complete messages already read from the socket but not yet consumed.
    pending: std::collections::VecDeque<String>,
    pub move_state: MoveState,
    pub hello: ServerHello,
    pub login: LoginOutcome,
    /// Our player id once known from PU (optional).
    pub our_id: Option<i32>,
    /// Map chunk upper-left + size from last MC (used to guess our player near center).
    last_mc: Option<MapChunkMeta>,
    /// Queued object action (LivingLifePage `nextActionMessageToSend`) — sent only when
    /// not mid-MOVE / not awaiting FORCE (protocol: USE/DROP/REMV ignored in motion).
    pending_action: Option<ObjectAction>,
    /// Optional full TX/RX transcript.
    wire_log: Option<Arc<WireLog>>,
}

#[derive(Debug, Clone, Copy)]
struct MapChunkMeta {
    size_x: i32,
    size_y: i32,
    x: i32,
    y: i32,
}

impl MapChunkMeta {
    fn center(&self) -> (i32, i32) {
        (
            self.x + self.size_x / 2,
            self.y + self.size_y / 2,
        )
    }
}

impl ClientSession {
    pub fn connect(cfg: &SessionConfig) -> io::Result<Self> {
        Self::connect_with_log(cfg, None)
    }

    pub fn connect_with_log(cfg: &SessionConfig, wire_log: Option<Arc<WireLog>>) -> io::Result<Self> {
        let addr = format!("{}:{}", cfg.host, cfg.port);
        let mut addrs = addr.to_socket_addrs()?;
        let sock_addr = addrs.next().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("cannot resolve {addr}"))
        })?;
        if let Some(log) = &wire_log {
            log.note(&format!("connect {addr}"));
        }
        let stream = TcpStream::connect_timeout(&sock_addr, cfg.read_timeout)?;
        stream.set_read_timeout(Some(cfg.read_timeout))?;
        stream.set_write_timeout(Some(cfg.write_timeout))?;
        stream.set_nodelay(true)?;

        let mut session = Self {
            stream,
            frames: FrameReader::new(),
            pending: std::collections::VecDeque::new(),
            move_state: MoveState::default(),
            hello: ServerHello {
                current_players: 0,
                max_players: 0,
                challenge: String::new(),
                required_version: 0,
            },
            login: LoginOutcome::Rejected,
            our_id: None,
            last_mc: None,
            pending_action: None,
            wire_log,
        };

        // First message: SN (or SHUTDOWN / SERVER_FULL)
        let first = session.read_one_message()?;
        if let Ok(outcome) = parse_login_outcome(&first) {
            match outcome {
                LoginOutcome::Shutdown { .. } | LoginOutcome::ServerFull { .. } => {
                    session.login = outcome;
                    return Ok(session);
                }
                _ => {}
            }
        }
        let hello = parse_sn(&first).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("expected SN: {e}; got {:?}", first.chars().take(80).collect::<String>()),
            )
        })?;
        session.hello = hello.clone();

        let login_line = encode_login(&LoginParams {
            reconnect: cfg.reconnect,
            client_tag: &cfg.client_tag,
            email: &cfg.email,
            password: &cfg.password,
            account_key: &cfg.account_key,
            challenge: &hello.challenge,
            tutorial_number: cfg.tutorial_number,
            twin_code: None,
            twin_count: 0,
            pad_email_to_80: cfg.pad_email_to_80,
        });
        session.send_raw(&login_line)?;

        let second = session.read_one_message()?;
        // May be NO_LIFE_TOKENS then REJECTED — accept either as outcome.
        let outcome = if second.trim().starts_with("NO_LIFE_TOKENS") {
            LoginOutcome::NoLifeTokens
        } else {
            parse_login_outcome(&second).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("expected ACCEPTED/REJECTED: {e}; body={second:?}"),
                )
            })?
        };
        session.login = outcome;
        if let Some(log) = &session.wire_log {
            log.note(&format!("login_outcome={:?}", session.login));
        }
        Ok(session)
    }

    pub fn set_wire_log(&mut self, log: Arc<WireLog>) {
        self.wire_log = Some(log);
    }

    fn read_one_message(&mut self) -> io::Result<String> {
        if let Some(m) = self.pending.pop_front() {
            if let Some(log) = &self.wire_log {
                log.rx(&m);
            }
            return Ok(m);
        }
        let mut chunk = [0u8; 8192];
        loop {
            let n = self.stream.read(&mut chunk)?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "server closed connection",
                ));
            }
            let mut msgs = self.frames.push(&chunk[..n]);
            if msgs.is_empty() {
                continue;
            }
            let first = msgs.remove(0);
            self.pending.extend(msgs);
            if let Some(log) = &self.wire_log {
                log.rx(&first);
            }
            return Ok(first);
        }
    }

    /// Read and process one framed message; applies move-state updates for PU.
    pub fn poll_event(&mut self) -> io::Result<SessionEvent> {
        let body = self.read_one_message()?;
        self.dispatch_body(body)
    }

    fn dispatch_body(&mut self, body: String) -> io::Result<SessionEvent> {
        let ty = message_type(&body);
        match ty {
            "PU" => {
                // Skip header line "PU", parse each data line
                let mut last_evt = None;
                for line in body.lines().skip(1) {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    if let Some(pu) = parse_pu_line(line) {
                        self.maybe_bind_our_player(&pu);
                        let is_ours = self.our_id == Some(pu.player_id);
                        let mut force_ack_sent = None;
                        if is_ours {
                            if let Some(ack) = self.move_state.on_player_update(
                                pu.done_moving_seq_num,
                                pu.force,
                                pu.x,
                                pu.y,
                            ) {
                                self.send_raw(&ack)?;
                                self.move_state.acknowledge_force_sent();
                                force_ack_sent = Some(ack);
                            }
                            // After move/force resolves, send queued USE/DROP/REMV like
                            // LivingLifePage when !inMotion.
                            let _ = self.flush_pending_action();
                        }
                        last_evt = Some(SessionEvent::PlayerUpdate {
                            player_id: pu.player_id,
                            done_moving_seq_num: pu.done_moving_seq_num,
                            force: pu.force,
                            x: pu.x,
                            y: pu.y,
                            force_ack_sent,
                        });
                    }
                }
                Ok(last_evt.unwrap_or(SessionEvent::Other(body)))
            }
            "PM" => Ok(SessionEvent::PlayerMovesStart(parse_pm_message(&body))),
            "MC" => {
                self.last_mc = parse_mc_meta(&body);
                Ok(SessionEvent::MapChunk(body))
            }
            "FM" => Ok(SessionEvent::Frame),
            _ => Ok(SessionEvent::Other(body)),
        }
    }

    /// Bind local player once: prefer PU nearest map-chunk center (spawn view).
    fn maybe_bind_our_player(&mut self, pu: &crate::parse::PlayerUpdate) {
        if self.our_id.is_some() {
            return;
        }
        // If we have MC meta, only bind when this PU is near the chunk center
        // (first post-login map is centered on the connecting player).
        if let Some(mc) = self.last_mc {
            let (cx, cy) = mc.center();
            let dist = (pu.x - cx).abs().max((pu.y - cy).abs());
            // Allow a few tiles of slack (player may be mid-path).
            if dist > 8 {
                return;
            }
        } else {
            // No MC yet: do not guess among multiplayer PUs.
            return;
        }
        self.our_id = Some(pu.player_id);
        self.move_state.x = pu.x;
        self.move_state.y = pu.y;
        // Birth seq is 1 until first MOVE; if server reports done_moving use max(1, that).
        if pu.done_moving_seq_num > 0 {
            self.move_state.last_move_sequence_number = pu.done_moving_seq_num;
        }
        if let Some(log) = &self.wire_log {
            log.note(&format!(
                "bound our_id={} pos=({},{}) last_seq={}",
                pu.player_id, pu.x, pu.y, self.move_state.last_move_sequence_number
            ));
        }
    }

    /// Drain server messages for up to `max` frames or until read timeout.
    pub fn drain(&mut self, max: usize) -> Vec<SessionEvent> {
        let mut out = Vec::new();
        for _ in 0..max {
            match self.poll_event() {
                Ok(ev) => out.push(ev),
                Err(_) => break,
            }
        }
        out
    }

    pub fn send_raw(&mut self, message: &str) -> io::Result<()> {
        if let Some(log) = &self.wire_log {
            log.tx(message);
        }
        write_message(&mut self.stream, message)
    }

    pub fn send_ka(&mut self) -> io::Result<()> {
        self.send_raw(&encode_ka(0, 0))
    }

    /// Send `SAY 0 0 text#` (SAY is allowed during MOVE per protocol).
    pub fn send_say(&mut self, text: &str) -> io::Result<String> {
        let line = encode_say(0, 0, text);
        self.send_raw(&line)?;
        Ok(line)
    }

    pub fn send_move(&mut self, path: &[PathDelta]) -> Result<String, SessionError> {
        let line = self.move_state.send_move(path)?;
        self.send_raw(&line)?;
        Ok(line)
    }

    pub fn send_use(
        &mut self,
        x: i32,
        y: i32,
        object_id: Option<i32>,
        slot: Option<i32>,
    ) -> Result<String, SessionError> {
        self.send_object_action(ObjectAction::Use {
            x,
            y,
            object_id,
            slot,
        })
    }

    pub fn send_drop(&mut self, x: i32, y: i32, clothing_slot: i32) -> Result<String, SessionError> {
        self.send_object_action(ObjectAction::Drop {
            x,
            y,
            clothing_slot,
        })
    }

    pub fn send_remv(&mut self, x: i32, y: i32, slot: i32) -> Result<String, SessionError> {
        self.send_object_action(ObjectAction::Remv { x, y, slot })
    }

    pub fn send_self(&mut self, x: i32, y: i32, clothing_slot: i32) -> Result<String, SessionError> {
        self.send_object_action(ObjectAction::SelfAct {
            x,
            y,
            clothing_slot,
        })
    }

    /// Queue or immediately send an object action (official client queues until not moving).
    ///
    /// - If idle: encode + send now, return the wire line.
    /// - If mid-MOVE / force-wait: store as pending; flush via [`Self::flush_pending_action`]
    ///   after done_moving / FORCE ack. Returns the encoded line either way.
    pub fn send_object_action(&mut self, action: ObjectAction) -> Result<String, SessionError> {
        let line = action.encode();
        if action.blocked_while_moving() {
            if let Err(e) = self.move_state.can_send_object_action() {
                // Mirror nextActionMessageToSend: replace any previous pending action.
                self.pending_action = Some(action);
                if let Some(log) = &self.wire_log {
                    log.note(&format!("queued action (blocked {e}): {line}"));
                }
                return Ok(line);
            }
        }
        self.pending_action = None;
        self.send_raw(&line)?;
        Ok(line)
    }

    /// If a pending action exists and we are free to act, send it (after MOVE completes).
    pub fn flush_pending_action(&mut self) -> Result<Option<String>, SessionError> {
        if self.pending_action.is_none() {
            return Ok(None);
        }
        if self.move_state.can_send_object_action().is_err() {
            return Ok(None);
        }
        let action = self.pending_action.take().unwrap();
        let line = action.encode();
        self.send_raw(&line)?;
        Ok(Some(line))
    }

    pub fn pending_action(&self) -> Option<&ObjectAction> {
        self.pending_action.as_ref()
    }

    /// Immediately send encoded action without move-gate (escape hatch / tests).
    pub fn send_action_now(&mut self, action: &ObjectAction) -> Result<String, SessionError> {
        let line = action.encode();
        self.pending_action = None;
        self.send_raw(&line)?;
        Ok(line)
    }

    pub fn stream_mut(&mut self) -> &mut TcpStream {
        &mut self.stream
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Move(#[from] MoveError),
}

/// Connect, login, return session (login outcome may be Rejected).
pub fn connect_and_login(cfg: &SessionConfig) -> io::Result<ClientSession> {
    ClientSession::connect(cfg)
}

/// Connect + login with full wire transcript.
pub fn connect_and_login_logged(
    cfg: &SessionConfig,
    log: Arc<WireLog>,
) -> io::Result<ClientSession> {
    ClientSession::connect_with_log(cfg, Some(log))
}

fn parse_mc_meta(body: &str) -> Option<MapChunkMeta> {
    // MC\nsizeX sizeY x y\nbinary_raw binary_compressed
    let mut lines = body.lines().map(|l| l.trim()).filter(|l| !l.is_empty());
    if lines.next()? != "MC" {
        return None;
    }
    let dims = lines.next()?;
    let mut p = dims.split_whitespace();
    let size_x: i32 = p.next()?.parse().ok()?;
    let size_y: i32 = p.next()?.parse().ok()?;
    let x: i32 = p.next()?.parse().ok()?;
    let y: i32 = p.next()?.parse().ok()?;
    Some(MapChunkMeta {
        size_x,
        size_y,
        x,
        y,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::login::hmac_sha1_hex;
    use std::net::TcpListener;
    use std::thread;

    fn fixture_peer() -> (u16, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            // SN
            write_message(
                &mut sock,
                "SN\n1/20\ntest_challenge_xyz\n184\n",
            )
            .unwrap();
            // read LOGIN
            let mut fr = FrameReader::new();
            let mut buf = [0u8; 4096];
            let login_body = loop {
                let n = sock.read(&mut buf).unwrap();
                if n == 0 {
                    return;
                }
                let msgs = fr.push(&buf[..n]);
                if let Some(m) = msgs.into_iter().next() {
                    break m;
                }
            };
            assert!(
                login_body.starts_with("LOGIN ") || login_body.starts_with("RLOGIN "),
                "got {login_body}"
            );
            // Validate HMAC against known challenge
            let parts: Vec<&str> = login_body.split_whitespace().collect();
            // LOGIN tag email... pw key tutorial — with padding email may be multi-token? 
            // pad_email uses spaces inside email field — split_whitespace breaks that.
            // Fixture uses pad_email_to_80: false in test config.
            assert!(parts.len() >= 6);
            let pw_hash = parts[parts.len() - 3];
            let key_hash = parts[parts.len() - 2];
            let expected_pw = hmac_sha1_hex("pw", "test_challenge_xyz");
            let expected_key = hmac_sha1_hex("ABC", "test_challenge_xyz");
            assert_eq!(pw_hash, expected_pw);
            assert_eq!(key_hash, expected_key);
            write_message(&mut sock, "ACCEPTED\n").unwrap();
            // optional: wait for MOVE / KA
            let mut got = Vec::new();
            sock.set_read_timeout(Some(Duration::from_millis(500))).ok();
            loop {
                match sock.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        got.extend_from_slice(&buf[..n]);
                        if got.iter().filter(|&&b| b == b'#').count() >= 2 {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            let text = String::from_utf8_lossy(&got);
            assert!(text.contains("MOVE "), "expected MOVE in {text}");
            assert!(text.contains("KA 0 0#"), "expected KA in {text}");
        });
        (port, handle)
    }

    #[test]
    fn connect_login_move_against_fixture() {
        let (port, handle) = fixture_peer();
        let cfg = SessionConfig {
            host: "127.0.0.1".into(),
            port,
            email: "t@e.com".into(),
            password: "pw".into(),
            account_key: "ABC".into(),
            pad_email_to_80: false,
            read_timeout: Duration::from_secs(3),
            write_timeout: Duration::from_secs(3),
            ..SessionConfig::default()
        };
        let mut session = connect_and_login(&cfg).unwrap();
        assert_eq!(session.login, LoginOutcome::Accepted);
        assert_eq!(session.hello.challenge, "test_challenge_xyz");
        let mv = session
            .send_move(&[PathDelta { x: 1, y: 0 }])
            .unwrap();
        assert_eq!(mv, "MOVE 0 0 @2 1 0#");
        session.send_ka().unwrap();
        handle.join().unwrap();
    }
}
