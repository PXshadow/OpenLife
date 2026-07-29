//! TCP session: connect, SN→LOGIN, ACCEPTED/REJECTED, send MOVE/actions.

use std::io::{self, Read};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use crate::actions::{ObjectAction, encode_jump, encode_ka};
use crate::emotion::{SpeechOutbound, classify_speech_outbound};
use crate::client_map::ClientMap;
use crate::content::ClientContent;
use crate::emotion::EmotionBank;
use crate::frame::{FrameReader, FramedMessage, write_message};
use crate::live_object::LiveWorld;
use crate::login::{LoginParams, encode_login};
use crate::map_global_offset::MapGlobalOffset;
use crate::move_state::{MoveError, MoveState, PathDelta};
use crate::parse::{
    Craving, CurseScoreChange, CurseTokenChange, DyingPlayer, FlightDest, FoodChange,
    GlobalMessage, HeatChange, InboundMessage, Lineage, LocationSays, LoginOutcome, MapChange,
    MapChunkHeader, MonumentCall, PlayerEmot, PlayerMoveStart, PlayerName, PlayerSays,
    PlayerUpdate, ServerHello, ValleySpacing, parse_inbound, parse_login_outcome, parse_sn,
};
use crate::tags::ServerTag;
use crate::wire_log::WireLog;
use std::collections::VecDeque;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// FM frame batching helpers
// C++: LivingLifePage.cpp getNextServerMessage / waitForFrameMessages
// ---------------------------------------------------------------------------

/// C++: `strstr(message, "FM") == message` — FRAME end-of-timestep marker.
fn framed_is_fm(framed: &FramedMessage) -> bool {
    match framed {
        FramedMessage::Text(s) => s.as_bytes().starts_with(b"FM"),
        FramedMessage::MapChunk { .. } => false,
    }
}

/// C++ pass-through while waiting for FM: MAP_CHUNK, PONG, FLIGHT_DEST, PHOTO_SIGNATURE.
/// These must not be held in `serverFrameMessages` (MC has binary; PONG is RTT; FD must
/// arrive before the destination MC is applied on the client path).
fn framed_is_frame_passthrough(framed: &FramedMessage) -> bool {
    match framed {
        FramedMessage::MapChunk { .. } => true,
        FramedMessage::Text(s) => match first_wire_tag(s) {
            Some(ServerTag::Mc)
            | Some(ServerTag::Pong)
            | Some(ServerTag::Fd)
            | Some(ServerTag::Ph) => true,
            _ => false,
        },
    }
}

fn first_wire_tag(body: &str) -> Option<ServerTag> {
    let line = body.lines().next()?.trim();
    let tok = line.split_whitespace().next().unwrap_or(line);
    ServerTag::parse(tok)
}

/// C++: `LivingLifePage.cpp` — idle KA when `game_getCurrentTime() - timeLastMessageSent > 15`.
pub const KA_IDLE_SECS: u64 = 15;

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

/// Session-level event after applying move-state side effects.
///
/// Multi-line PU messages emit **one event per player line** (via an internal
/// queue) so probes never drop other-player updates.
#[derive(Debug, Clone)]
pub enum SessionEvent {
    Hello(ServerHello),
    Login(LoginOutcome),
    /// One PU line after move-state side effects for our player.
    PlayerUpdate {
        pu: PlayerUpdate,
        force_ack_sent: Option<String>,
    },
    PlayerMovesStart(Vec<PlayerMoveStart>),
    /// Map chunk header (binary payload already consumed by frame reader).
    MapChunk(MapChunkHeader),
    MapChanges(Vec<MapChange>),
    FoodChange(FoodChange),
    HeatChange(HeatChange),
    PlayerSays(Vec<PlayerSays>),
    LocationSays(Vec<LocationSays>),
    PlayerEmot(Vec<PlayerEmot>),
    PlayerOutOfRange(Vec<i32>),
    BabyWiggle(Vec<i32>),
    Names(Vec<PlayerName>),
    Lineages(Vec<Lineage>),
    Dying(Vec<DyingPlayer>),
    Healed(Vec<i32>),
    Frame,
    ForcedShutdown,
    Apocalypse,
    ApocalypseDone,
    Pong(String),
    GlobalMessage(GlobalMessage),
    /// CU (CURSED) — curse level + optional name for speech tag.
    Cursed(Vec<crate::parse::CursedPlayer>),
    CurseTokens(CurseTokenChange),
    CurseScore(CurseScoreChange),
    ValleySpacing(ValleySpacing),
    FlightDest(FlightDest),
    Flip(Vec<crate::parse::FlipUpdate>),
    Craving(Craving),
    PosseJoin(Vec<i32>),
    MonumentCall(MonumentCall),
    Ghost(Vec<i32>),
    /// CM header only (inflate failed); successful CM never surfaces here.
    Compressed {
        raw_size: usize,
        compressed_size: usize,
    },
    /// Recognized tag kept as raw body, or fully unknown message.
    Other(String),
}

impl SessionEvent {
    /// Short wire tag for logging (best-effort).
    pub fn tag_str(&self) -> &str {
        match self {
            Self::Hello(_) => "SN",
            Self::Login(o) => match o {
                LoginOutcome::Accepted => "ACCEPTED",
                LoginOutcome::Rejected => "REJECTED",
                LoginOutcome::NoLifeTokens => "NO_LIFE_TOKENS",
                LoginOutcome::Shutdown { .. } => "SHUTDOWN",
                LoginOutcome::ServerFull { .. } => "SERVER_FULL",
            },
            Self::PlayerUpdate { .. } => "PU",
            Self::PlayerMovesStart(_) => "PM",
            Self::MapChunk(_) => "MC",
            Self::MapChanges(_) => "MX",
            Self::FoodChange(_) => "FX",
            Self::HeatChange(_) => "HX",
            Self::PlayerSays(_) => "PS",
            Self::LocationSays(_) => "LS",
            Self::PlayerEmot(_) => "PE",
            Self::PlayerOutOfRange(_) => "PO",
            Self::BabyWiggle(_) => "BW",
            Self::Names(_) => "NM",
            Self::Lineages(_) => "LN",
            Self::Dying(_) => "DY",
            Self::Healed(_) => "HE",
            Self::Frame => "FM",
            Self::ForcedShutdown => "SD",
            Self::Cursed(_) => "CU",
            Self::Apocalypse => "AP",
            Self::ApocalypseDone => "AD",
            Self::Pong(_) => "PONG",
            Self::GlobalMessage(_) => "MS",
            Self::CurseTokens(_) => "CX",
            Self::CurseScore(_) => "CS",
            Self::ValleySpacing(_) => "VS",
            Self::FlightDest(_) => "FD",
            Self::Flip(_) => "FL",
            Self::Craving(_) => "CR",
            Self::PosseJoin(_) => "PJ",
            Self::MonumentCall(_) => "MN",
            Self::Ghost(_) => "GH",
            Self::Compressed { .. } => "CM",
            Self::Other(s) => s.lines().next().unwrap_or("?"),
        }
    }
}

/// Connected headless client session.
pub struct ClientSession {
    stream: TcpStream,
    frames: FrameReader,
    /// Complete frames already read from the socket but not yet consumed.
    pending: VecDeque<FramedMessage>,
    /// Extra session events queued from multi-line messages (e.g. multi-PU).
    pending_events: VecDeque<SessionEvent>,
    /// C++: `waitForFrameMessages` — after ACCEPTED, hold gameplay tags until FM.
    wait_for_frame_messages: bool,
    /// C++: `serverFrameMessages` — non-pass-through frames buffered until FM.
    server_frame_messages: VecDeque<FramedMessage>,
    /// C++: `serverFrameReady` — FM seen and buffer non-empty; drain next.
    server_frame_ready: bool,
    /// Emit [`SessionEvent::Frame`] after the current batch fully drains (probe boundary).
    /// C++ discards FM and never surfaces it; we optionally expose a boundary *after* apply.
    frame_boundary_pending: bool,
    pub move_state: MoveState,
    /// C++ `mMapGlobalOffset` — local storage → wire for MOVE `sendX`/`sendY`.
    ///
    /// Default [`MapGlobalOffset::ZERO`] (storage frame == wire). Applied on every
    /// MOVE encode path; first MC may re-seed via [`MapGlobalOffset::from_first_mc_center`]
    /// (Rust policy still returns identity — see `map_global_offset` module).
    pub map_global_offset: MapGlobalOffset,
    pub hello: ServerHello,
    pub login: LoginOutcome,
    /// Our player id once known from PU (optional).
    pub our_id: Option<i32>,
    /// Map chunk upper-left + size from last MC (used to guess our player near center).
    last_mc: Option<MapChunkHeader>,
    /// Last FX applied (headless HUD/world state).
    pub food: Option<FoodChange>,
    /// C++ FX held on feeder's mid-walk until they stop moving (`responsible_id`).
    ///
    /// // C++ LivingLifePage FOOD_CHANGE ~21867–21881: push onto pendingReceivedMessages
    deferred_fx: Vec<(i32, FoodChange)>,
    /// Last HX applied.
    pub heat: Option<HeatChange>,
    /// Last curse token count (CX).
    pub curse_tokens: Option<i32>,
    /// Last excess curse points (CS) — L-HUD residual.
    pub excess_curse_points: Option<i32>,
    /// Lasting player entities (C++ LiveObject table) — **L-LIVEOBJ**.
    pub world: LiveWorld,
    /// Client map from MC/MX — **L-MAP**.
    pub map: ClientMap,
    /// C++ `mBadBiomeIndices` from login `BB` / BAD_BIOMES (path edge routing).
    pub bad_biomes: Vec<u8>,
    /// Optional content tables (objects/transitions) for blocking/food.
    pub content: ClientContent,
    /// PE emotion table (`contentSettings/emotionObjects.ini`) — **L-EMOT**.
    pub emotions: EmotionBank,
    /// OLSN sound index + lazy AIFF (**L-SOUND-TRIG** / C-SND).
    pub sounds: crate::sound_bank::SoundBank,
    /// Queued object action (LivingLifePage `nextActionMessageToSend`) — sent only when
    /// not mid-MOVE / not awaiting FORCE (protocol: USE/DROP/REMV ignored in motion).
    pending_action: Option<ObjectAction>,
    /// C++ `playerActionPending` — true after a USE/DROP/… is **sent** until the next
    /// non-force our-PU while already stationary confirms the action. Blocks clicks.
    pub player_action_pending: bool,
    /// Remaining MOVE chunks after the first ±16 window (C++ multi-hop after done_moving).
    /// // C++: LivingLifePage multi-hop after done_moving / pathFindingD=32
    pub(crate) multi_move_chunks: Vec<Vec<PathDelta>>,
    /// Ultimate click goal while multi-MOVE (or repath) is armed.
    pub(crate) multi_move_goal: Option<(i32, i32)>,
    /// C++ `lastFlipSent` (true = last FLIP was face-left).
    last_flip_sent: bool,
    /// C++ `lastFlipSendTime` — throttle FLIP spam (~2s).
    last_flip_send_time: Instant,
    /// C++: `timeLastMessageSent` — any client→server write (incl. LOGIN / KA / FORCE).
    last_tx: Instant,
    /// Wall time of last successful server→client read (any bytes).
    last_rx: Instant,
    /// Idle threshold before auto-KA (default 15s; tests may shorten).
    pub ka_idle: Duration,
    /// Optional full TX/RX transcript.
    wire_log: Option<Arc<WireLog>>,
}

impl ClientSession {
    pub fn connect(cfg: &SessionConfig) -> io::Result<Self> {
        Self::connect_with_log(cfg, None)
    }

    /// Connect using already-loaded content (P5#36 loading UI pre-boot).
    ///
    /// Skips a second prefer_cache pass when `ohol-client` already ran
    /// [`crate::load_progress::boot_load_prefer_cache`]. Sounds still load from
    /// the content root (index-only) unless the caller overwrites `session.sounds`.
    pub fn connect_with_content(
        cfg: &SessionConfig,
        content: ClientContent,
    ) -> io::Result<Self> {
        Self::connect_with_log_and_content(cfg, None, Some(content))
    }

    pub fn connect_with_log(cfg: &SessionConfig, wire_log: Option<Arc<WireLog>>) -> io::Result<Self> {
        Self::connect_with_log_and_content(cfg, wire_log, None)
    }

    pub fn connect_with_log_and_content(
        cfg: &SessionConfig,
        wire_log: Option<Arc<WireLog>>,
        preloaded_content: Option<ClientContent>,
    ) -> io::Result<Self> {
        let host = cfg.host.trim();
        let addr = format!("{}:{}", host, cfg.port);
        // Prefer explicit IPv4 parse when host is an IP literal (avoids dual-stack surprises).
        let sock_addr = if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            std::net::SocketAddr::new(ip, cfg.port)
        } else {
            let mut addrs = addr.to_socket_addrs()?;
            addrs.next().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, format!("cannot resolve {addr}"))
            })?
        };
        if let Some(log) = &wire_log {
            log.note(&format!("connect {addr} -> {sock_addr}"));
        }
        // Connect uses a dedicated timeout (min 3s) so short play-poll timeouts never
        // starve the TCP handshake (was 30ms from soft-FB AccountPage → os error 10060).
        let connect_timeout = cfg.read_timeout.max(Duration::from_secs(3));
        let stream = TcpStream::connect_timeout(&sock_addr, connect_timeout).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("connect {sock_addr} failed after {connect_timeout:?}: {e}"),
            )
        })?;
        stream.set_read_timeout(Some(cfg.read_timeout))?;
        stream.set_write_timeout(Some(cfg.write_timeout))?;
        stream.set_nodelay(true)?;

        let mut session = Self {
            stream,
            frames: FrameReader::new(),
            pending: VecDeque::new(),
            pending_events: VecDeque::new(),
            wait_for_frame_messages: false,
            server_frame_messages: VecDeque::new(),
            server_frame_ready: false,
            frame_boundary_pending: false,
            move_state: MoveState::default(),
            map_global_offset: MapGlobalOffset::ZERO,
            hello: ServerHello {
                current_players: 0,
                max_players: 0,
                challenge: String::new(),
                required_version: 0,
            },
            login: LoginOutcome::Rejected,
            our_id: None,
            last_mc: None,
            food: None,
            deferred_fx: Vec::new(),
            heat: None,
            curse_tokens: None,
            excess_curse_points: None,
            world: LiveWorld::new(),
            map: ClientMap::new(),
            bad_biomes: Vec::new(),
            content: match preloaded_content {
                Some(c) => c,
                None => {
                    // P5#36: optional headless progress lines when OHOL_LOAD_PROGRESS=1.
                    if crate::load_progress::load_progress_env_enabled() {
                        let mut log_cb = |s: &crate::load_progress::LoadingState| {
                            crate::load_progress::log_progress_line(s);
                        };
                        ClientContent::load_default_locations_with_progress(Some(&mut log_cb))
                            .unwrap_or_default()
                    } else {
                        ClientContent::load_default_locations().unwrap_or_default()
                    }
                }
            },
            emotions: EmotionBank::new(), // filled after content loads below
            sounds: crate::sound_bank::SoundBank::new("."),
            pending_action: None,
            player_action_pending: false,
            multi_move_chunks: Vec::new(),
            multi_move_goal: None,
            last_flip_sent: false,
            last_flip_send_time: Instant::now(),
            last_tx: Instant::now(),
            last_rx: Instant::now(),
            ka_idle: Duration::from_secs(KA_IDLE_SECS),
            wire_log,
        };
        // L-EMOT: load emotionWords/emotionObjects from content root (tiny ini).
        // L-SOUND-TRIG: OLSN index only (zero AIFF opens at boot).
        // P5#36: progress callback when OHOL_LOAD_PROGRESS=1.
        let log_progress = crate::load_progress::load_progress_env_enabled();
        if let Some(ref root) = session.content.root {
            session.emotions = EmotionBank::load_from_content_root(root);
            session.sounds = if log_progress {
                let mut log_cb = |s: &crate::load_progress::LoadingState| {
                    crate::load_progress::log_progress_line(s);
                };
                crate::sound_bank::SoundBank::load_prefer_cache_with_progress(
                    root,
                    Some(&mut log_cb),
                )
            } else {
                crate::sound_bank::SoundBank::load_prefer_cache(root)
            };
        } else {
            let fallback = std::path::Path::new(r"C:\OhOl\OpenLife\OneLifeData7");
            session.emotions = EmotionBank::load_from_content_root(fallback);
            session.sounds = if log_progress {
                let mut log_cb = |s: &crate::load_progress::LoadingState| {
                    crate::load_progress::log_progress_line(s);
                };
                crate::sound_bank::SoundBank::load_prefer_cache_with_progress(
                    fallback,
                    Some(&mut log_cb),
                )
            } else {
                crate::sound_bank::SoundBank::load_prefer_cache(fallback)
            };
        }

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
                format!(
                    "expected SN: {e}; got {:?}",
                    first.chars().take(80).collect::<String>()
                ),
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
        // C++: LivingLifePage ACCEPTED handler sets waitForFrameMessages = true.
        // Subsequent messages are FRAME batches until logout/reset.
        if matches!(session.login, LoginOutcome::Accepted) {
            session.wait_for_frame_messages = true;
        }
        if let Some(log) = &session.wire_log {
            log.note(&format!("login_outcome={:?}", session.login));
        }
        Ok(session)
    }

    pub fn set_wire_log(&mut self, log: Arc<WireLog>) {
        self.wire_log = Some(log);
    }

    /// Adjust TCP read timeout (e.g. short polls after login for snapshot tools).
    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.stream.set_read_timeout(timeout)
    }

    fn read_one_framed(&mut self) -> io::Result<FramedMessage> {
        if let Some(m) = self.pending.pop_front() {
            // Buffered from a prior socket read — still counts as recent RX traffic.
            if let Some(log) = &self.wire_log {
                log.rx(m.as_dispatch_text());
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
            // Any successful byte read from the server.
            self.last_rx = Instant::now();
            let mut msgs = self.frames.push_framed(&chunk[..n]);
            if msgs.is_empty() {
                continue;
            }
            let first = msgs.remove(0);
            self.pending.extend(msgs);
            if let Some(log) = &self.wire_log {
                log.rx(first.as_dispatch_text());
            }
            return Ok(first);
        }
    }

    /// Seconds since last successful server→client TCP read (0 if just received).
    pub fn secs_since_last_rx(&self) -> f32 {
        self.last_rx.elapsed().as_secs_f32()
    }

    /// Instant of last successful server→client read.
    pub fn last_rx_instant(&self) -> Instant {
        self.last_rx
    }

    fn read_one_message(&mut self) -> io::Result<String> {
        match self.read_one_framed()? {
            FramedMessage::Text(s) => Ok(s),
            FramedMessage::MapChunk { header, compressed } => {
                if let Some(h) = crate::parse::parse_mc_header(&header) {
                    let _ = self.map.apply_mc_binary(&h, &compressed);
                    self.note_first_mc_offset(&h);
                    self.last_mc = Some(h);
                }
                Ok(header)
            }
        }
    }

    /// Read and process one framed message; applies move-state / map / live world.
    ///
    /// C++: `LivingLifePage.cpp` `getNextServerMessage` — after ACCEPTED, non-pass-through
    /// messages are buffered until an FM end-of-frame marker, then drained in order so one
    /// client step applies a full server timestep together.
    pub fn poll_event(&mut self) -> io::Result<SessionEvent> {
        // 1) Prefer already-ready session events (multi-line PU split, etc.).
        if let Some(ev) = self.pending_events.pop_front() {
            return Ok(ev);
        }
        // Optional probe boundary: after batch + multi-PU expansions fully drained.
        if self.frame_boundary_pending
            && !self.server_frame_ready
            && self.server_frame_messages.is_empty()
        {
            self.frame_boundary_pending = false;
            return Ok(SessionEvent::Frame);
        }

        // 2) Pre-ACCEPTED (or reset): immediate apply like getNextServerMessageRaw.
        if !self.wait_for_frame_messages {
            let framed = self.read_one_framed()?;
            return self.dispatch_framed(framed);
        }

        // 3) C++ waitForFrameMessages branch.
        if !self.server_frame_ready {
            // Fill buffer until FM (non-empty) or a pass-through message.
            if let Some(passthrough) = self.fill_server_frame()? {
                return self.dispatch_framed(passthrough);
            }
            // fill_server_frame set server_frame_ready when buffer was non-empty at FM.
        }

        if self.server_frame_ready {
            let framed = self
                .server_frame_messages
                .pop_front()
                .expect("server_frame_ready implies non-empty buffer");
            if self.server_frame_messages.is_empty() {
                self.server_frame_ready = false;
                // Surface FM as a boundary *after* this last (or sole) batch message and
                // any pending_events it queues — not mid-batch.
                self.frame_boundary_pending = true;
            }
            return self.dispatch_framed(framed);
        }

        // C++ returns NULL while waiting for FM with nothing ready.
        Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "waiting for FM end-of-frame",
        ))
    }

    /// C++: fill `serverFrameMessages` until FM (buffer non-empty) or pass-through return.
    ///
    /// Returns `Ok(Some(msg))` for MC/PONG/FD/PH (deliver immediately, not queued).
    /// Returns `Ok(None)` when `server_frame_ready` became true (caller drains buffer).
    fn fill_server_frame(&mut self) -> io::Result<Option<FramedMessage>> {
        // C++: LivingLifePage.cpp getNextServerMessage — !serverFrameReady loop
        loop {
            let framed = self.read_one_framed()?;
            if framed_is_fm(&framed) {
                // End of frame: discard marker. Empty FM with empty buffer is ignored.
                if !self.server_frame_messages.is_empty() {
                    self.server_frame_ready = true;
                    // Stop so post-FM bytes stay in `pending` for the next frame.
                    return Ok(None);
                }
                continue;
            }
            if framed_is_frame_passthrough(&framed) {
                return Ok(Some(framed));
            }
            self.server_frame_messages.push_back(framed);
        }
    }

    fn dispatch_framed(&mut self, framed: FramedMessage) -> io::Result<SessionEvent> {
        match framed {
            FramedMessage::MapChunk { header, compressed } => {
                if let Some(h) = crate::parse::parse_mc_header(&header) {
                    let _ = self.map.apply_mc_binary(&h, &compressed);
                    self.note_first_mc_offset(&h);
                    self.last_mc = Some(h.clone());
                    Ok(SessionEvent::MapChunk(h))
                } else {
                    Ok(SessionEvent::Other(header))
                }
            }
            FramedMessage::Text(body) => self.dispatch_body(body),
        }
    }

    /// C++ first-MC `mMapGlobalOffset` seed (LivingLifePage ~16127).
    ///
    /// Only runs once (before `last_mc` is set). Rust policy keeps identity
    /// via [`MapGlobalOffset::from_first_mc_center`].
    fn note_first_mc_offset(&mut self, h: &MapChunkHeader) {
        if self.last_mc.is_some() {
            return;
        }
        let (cx, cy) = h.center();
        self.map_global_offset = MapGlobalOffset::from_first_mc_center(cx, cy);
    }

    /// Whether post-login FM batching is active (C++ `waitForFrameMessages`).
    pub fn wait_for_frame_messages(&self) -> bool {
        self.wait_for_frame_messages
    }

    /// Clear frame-wait state only (C++ fragment of logout / reset).
    ///
    /// Prefer [`Self::logout_reset`] for the full LivingLifePage clear path.
    pub fn clear_frame_batching(&mut self) {
        self.wait_for_frame_messages = false;
        self.server_frame_ready = false;
        self.frame_boundary_pending = false;
        self.server_frame_messages.clear();
    }

    /// C++: LivingLifePage page reset / death disconnect bookkeeping (not socket close).
    ///
    /// Clears FM batching (`waitForFrameMessages` / `serverFrameMessages`),
    /// `nextActionMessageToSend`, ready/pending event queues, LiveObject table, map,
    /// local move motion/FORCE gate, and HUD-ish FX/HX. Socket stays open for the
    /// caller to drop or reuse after RECONNECT flows.
    pub fn logout_reset(&mut self) {
        // C++: waitForFrameMessages = false; serverFrameMessages clear;
        // readyPendingReceivedMessages clear; nextActionMessageToSend = NULL;
        // clearLiveObjects(); playerActionPending = false; serverSocketBuffer deleteAll
        self.clear_frame_batching();
        self.pending.clear();
        self.pending_events.clear();
        self.pending_action = None;
        self.player_action_pending = false;
        self.multi_move_chunks.clear();
        self.multi_move_goal = None;
        self.frames = FrameReader::new();
        self.our_id = None;
        self.last_mc = None;
        self.food = None;
        self.deferred_fx.clear();
        self.heat = None;
        self.curse_tokens = None;
        self.excess_curse_points = None;
        self.world = LiveWorld::new();
        self.map = ClientMap::new();
        // Keep last_move_sequence semantics of a fresh birth (1) after full reset.
        self.move_state = MoveState::default();
        self.map_global_offset = MapGlobalOffset::ZERO;
        if let Some(log) = &self.wire_log {
            log.note("logout_reset");
        }
    }

    /// Drain events until a [`SessionEvent::Frame`] boundary or `max` events.
    ///
    /// Useful for probes that want one server timestep as an atomic batch. Pass-through
    /// MC/PONG/FD/PH that arrived mid-wait appear before the held batch + Frame.
    pub fn poll_frame_events(&mut self, max: usize) -> io::Result<Vec<SessionEvent>> {
        let mut out = Vec::with_capacity(max.min(16));
        for _ in 0..max {
            let ev = self.poll_event()?;
            let is_frame = matches!(ev, SessionEvent::Frame);
            out.push(ev);
            if is_frame {
                break;
            }
        }
        Ok(out)
    }

    /// Step all living players' dual-anim clocks (frame counts + cross-fade).
    ///
    /// Prefer letting [`crate::render::SceneRenderer::draw`] do this each frame
    /// (`dt > 0`). Expose for headless ticks without a soft-FB pass.
    ///
    /// Fires SoundAnimParam / footstep hooks via [`Self::sounds`].
    ///
    /// // C++: LivingLifePage per-frame animationFrameCount / lastAnimFade
    /// // L-ANIM-DRAW: `LiveWorld::step_anims` + pack select sync
    /// // L-SOUND-TRIG: `handleAnimSound`
    pub fn step_anims(
        &mut self,
        bank: &mut crate::anim_bank::AnimBank,
        anim_speed: f32,
        frame_rate_factor: f32,
    ) {
        // Fractional currentPos along own path (C++ per-frame path step).
        // frameRateFactor ≈ wall_dt * 60 when 60fps; invert for wall seconds.
        let wall_dt = if frame_rate_factor > 0.0 {
            (frame_rate_factor as f64) / 60.0
        } else {
            1.0 / 60.0
        };
        self.move_state.step_current_pos(wall_dt);

        // Headless / no SceneRenderer: approximate listener with fractional pos when moving.
        // (soft-FB path sets listener from camera in `SceneRenderer::draw`).
        if self.move_state.in_motion {
            self.sounds.set_listener(
                self.move_state.current_pos_x as f32,
                self.move_state.current_pos_y as f32,
            );
        } else if let Some(me) = self.world.our() {
            self.sounds.set_listener(me.x as f32, me.y as f32);
        }
        // P3#19: PE temporary TTL + decay sounds (also in SceneRenderer::draw)
        let decay = self.world.tick_emots(wall_dt as f32);
        crate::sound_bank::play_emot_decay_for_targets(
            &mut self.sounds,
            &self.content,
            &self.emotions,
            &decay,
        );
        self.world.step_anims_with_sounds(
            bank,
            &mut self.sounds,
            &self.content,
            &self.map,
            anim_speed,
            frame_rate_factor,
        );
        // P2#14: map object / floor ground-anim SoundAnimParam hooks (non-player).
        // frame_delta matches C++ per-draw mMapAnimationFrameCount++ with frf baked in.
        let map_frame_delta = if frame_rate_factor > 0.0 {
            frame_rate_factor
        } else {
            1.0
        };
        let _ = crate::sound_bank::step_map_ground_anims_with_sounds(
            &mut self.sounds,
            bank,
            &self.content,
            &mut self.map,
            map_frame_delta,
            self.our_id,
        );
    }

    /// Advance own fractional `currentPos` (wall-clock seconds).
    ///
    /// Call each frame for headless / GUI. Also invoked from [`Self::step_anims`].
    /// Syncs local [`LiveObject`] display pos + `moving` for Jason walk anim select.
    pub fn step_move_pos(&mut self, wall_dt: f64) {
        self.move_state.step_current_pos(wall_dt);
        self.sync_our_live_motion();
        self.maybe_send_flip();
    }

    /// Mirror [`MoveState`] into our [`LiveObject`] for draw/anim (C++ currentPos / onPath).
    pub fn sync_our_live_motion(&mut self) {
        let Some(oid) = self.our_id else {
            return;
        };
        let in_motion = self.move_state.in_motion;
        let (cx, cy) = (
            self.move_state.current_pos_x as f32,
            self.move_state.current_pos_y as f32,
        );
        let dir_x = if in_motion {
            self.move_state_facing_dx()
        } else {
            0.0
        };
        let Some(o) = self.world.get_mut(oid) else {
            return;
        };
        if in_motion {
            o.moving = true;
            o.set_display_pos(cx, cy);
            // Face move direction when |Δx| is substantial (Jason ~22789).
            if dir_x.abs() > 0.5 {
                o.set_holding_flip(dir_x < 0.0);
            }
        } else if !o.moving {
            // Idle: keep display on grid unless remote path still active.
            o.set_display_pos(o.x as f32, o.y as f32);
        }
    }

    /// Unit-ish X direction of current path segment (for facing).
    fn move_state_facing_dx(&self) -> f64 {
        let path = &self.move_state.path_to_dest;
        if path.len() < 2 {
            return 0.0;
        }
        let (cx, cy) = (
            self.move_state.current_pos_x,
            self.move_state.current_pos_y,
        );
        // Find nearest segment and its dx
        let mut best = 0.0f64;
        let mut best_d2 = f64::MAX;
        for w in path.windows(2) {
            let (x0, y0) = (w[0].0 as f64, w[0].1 as f64);
            let (x1, y1) = (w[1].0 as f64, w[1].1 as f64);
            let dx = x1 - x0;
            let dy = y1 - y0;
            let len2 = dx * dx + dy * dy;
            let u = if len2 > 1e-12 {
                ((cx - x0) * dx + (cy - y0) * dy) / len2
            } else {
                0.0
            }
            .clamp(0.0, 1.0);
            let px = x0 + dx * u;
            let py = y0 + dy * u;
            let d2 = (cx - px) * (cx - px) + (cy - py) * (cy - py);
            if d2 < best_d2 {
                best_d2 = d2;
                best = dx;
            }
        }
        best
    }

    /// Mark our LiveObject as moving when we send MOVE (before server PM arrives).
    fn mark_our_moving_from_send(&mut self) {
        if let Some(oid) = self.our_id {
            if let Some(o) = self.world.get_mut(oid) {
                o.moving = true;
                o.set_display_pos(
                    self.move_state.current_pos_x as f32,
                    self.move_state.current_pos_y as f32,
                );
                // Face first path step with |dx| > 0.5
                let deltas: Vec<(i32, i32)> = self
                    .move_state
                    .path_to_dest
                    .windows(2)
                    .map(|w| (w[1].0 - w[0].0, w[1].1 - w[0].1))
                    .collect();
                o.apply_facing_from_path_deltas(&deltas);
            }
        }
    }

    /// Send FLIP if our holdingFlip changed (throttled like C++ ~2s).
    ///
    /// Wire: `FLIP x y#` with a tile beside us in look direction (Jason ~23287).
    pub fn maybe_send_flip(&mut self) {
        let Some(oid) = self.our_id else {
            return;
        };
        let Some(o) = self.world.get(oid) else {
            return;
        };
        let flip = o.holding_flip();
        if flip == self.last_flip_sent {
            return;
        }
        if self.last_flip_send_time.elapsed() < Duration::from_secs(2) {
            return;
        }
        let (x, y) = (o.x, o.y);
        let offset = if flip { -1 } else { 1 };
        let msg = crate::actions::encode_flip(x + offset, y);
        if self.send_raw(&msg).is_ok() {
            self.last_flip_sent = flip;
            self.last_flip_send_time = Instant::now();
            self.last_tx = Instant::now();
        }
    }

    /// Pathfind on client map and send first MOVE segment (≤16 steps).
    ///
    /// Playable GUI path (L-HUD): `SceneRenderer::screen_to_tile` → this method.
    /// // C++: `LivingLifePage::pointerDown` empty-ground → `computePathToDest` → MOVE
    /// // Haxe: click tile → pathfind → MOVE (no full LivingLife HUD)
    ///
    /// Delegates to [`crate::click_tile::click_tile`] (cumulative deltas, repath,
    /// cancel pending USE/DROP, closest-reachable fallback).
    pub fn walk_to(&mut self, goal_x: i32, goal_y: i32) -> Result<(), MoveError> {
        crate::click_tile::click_tile(self, goal_x, goal_y).map(|_| ())
    }

    /// Screen-pixel ground click → tile pick → [`Self::walk_to`].
    ///
    /// Convenience for the soft-FB client: `screen_to_tile` + pathfind MOVE.
    /// Returns the world tile that was targeted.
    pub fn walk_to_screen(
        &mut self,
        scene: &crate::render::SceneRenderer,
        sx: f32,
        sy: f32,
        fb_w: u32,
        fb_h: u32,
    ) -> Result<(i32, i32), MoveError> {
        let (tx, ty) = scene.screen_to_tile(sx, sy, fb_w, fb_h);
        self.walk_to(tx, ty)?;
        Ok((tx, ty))
    }

    fn dispatch_body(&mut self, body: String) -> io::Result<SessionEvent> {
        // C++: LivingLifePage message switch; Haxe: Engine.message(ClientTag, …)
        let inbound = parse_inbound(&body);
        match inbound {
            InboundMessage::PlayerUpdates(list) => {
                if list.is_empty() {
                    return Ok(SessionEvent::Other(body));
                }
                let mut events = Vec::with_capacity(list.len());
                for pu in list {
                    // L-SOUND-TRIG: clothing / drop settle / held creation before apply
                    // (need previous LiveObject clothing + held).
                    self.play_pu_sounds(&pu);
                    // L-LIVEOBJ: lasting LiveObject table.
                    self.world.apply_pu(&pu);
                    // L-SOUND-TRIG: eatingSound when justAte (C++ ~18517)
                    if pu.just_ate && pu.last_ate_id > 0 {
                        if let Some(def) = self.content.get(pu.last_ate_id) {
                            let _ = crate::sound_bank::play_object_event_sound(
                                &mut self.sounds,
                                &def.eating_sound,
                            );
                        }
                    }
                    if !pu.deleted {
                        self.maybe_bind_our_player(&pu);
                    }
                    let is_ours = self.our_id == Some(pu.player_id);
                    let mut force_ack_sent = None;
                    if is_ours && !pu.deleted {
                        // Snapshot before move_state may clear in_motion on done_moving.
                        let was_in_motion = self.move_state.in_motion;
                        // C++ mid-frame order on our forced-pos PU:
                        // 1) snap + clear nextActionMessageToSend (do NOT flush it)
                        // 2) immediately send FORCE x y#
                        // 3) later (non-force) done_moving can release queued action
                        // Artificial force: destTruncated + pos mismatch (~18031–18048).
                        if let Some(ack) = self.move_state.on_player_update(
                            pu.done_moving_seq_num,
                            pu.force,
                            pu.x,
                            pu.y,
                        ) {
                            // FORCE path: cancel pending action (LivingLifePage ~19367–19378).
                            // Also drop multi-MOVE follow-up — C++ path is replaced by FORCE.
                            self.cancel_pending_action();
                            self.clear_multi_move();
                            self.player_action_pending = false;
                            if let Some(oid) = self.our_id {
                                if let Some(o) = self.world.get_mut(oid) {
                                    o.clear_pending_action_flag();
                                    o.pending_action_animation_progress = 0.0;
                                }
                            }
                            // Send FORCE before any subsequent messages in this frame batch
                            // are applied by the caller (we send inline during dispatch).
                            self.send_raw(&ack)?;
                            self.move_state.acknowledge_force_sent();
                            force_ack_sent = Some(ack);
                        } else {
                            // C++ ~19348–19357: post-action PU while !inMotion clears
                            // playerActionPending. done_moving arrives while was_in_motion
                            // so we do not clear before flush_pending_action below.
                            if !was_in_motion {
                                self.player_action_pending = false;
                                // P3#22: finish action-wiggle cycle (progress may still decay).
                                if let Some(oid) = self.our_id {
                                    if let Some(o) = self.world.get_mut(oid) {
                                        o.clear_pending_action_flag();
                                    }
                                }
                            }
                            // Playtest: multi-MOVE next hop BEFORE flushing queued action
                            // (long path continues after done_moving; action waits).
                            // // C++: done_moving → next path segment then nextActionMessageToSend
                            let _ = self.continue_multi_move()?;
                            // done_moving / non-force PU: flush nextAction when free.
                            // C++ also requires server pos match + anim delay; headless
                            // flushes as soon as !in_motion && !awaiting_force_ack.
                            // Non-matching done_moving leaves in_motion → flush is a no-op.
                            // After continue_multi_move arms another hop, in_motion blocks flush.
                            let _ = self.flush_pending_action()?;
                        }
                        // Jason: walk anim follows client onPath (move_state), not last PU alone.
                        // Re-assert moving + display after apply_pu may have cleared mid-path.
                        self.sync_our_live_motion();
                        if self.move_state.in_motion {
                            if let Some(oid) = self.our_id {
                                if let Some(o) = self.world.get_mut(oid) {
                                    o.moving = true;
                                }
                            }
                        }
                    }
                    // C++ ~19845: baby-held interrupt clears nextAction for ourID.
                    if !pu.deleted && pu.held_id < 0 {
                        let baby_id = -pu.held_id;
                        if self.our_id == Some(baby_id) {
                            self.cancel_pending_action();
                            self.clear_multi_move();
                            self.player_action_pending = false;
                            self.move_state.in_motion = false;
                            self.move_state.dest_truncated = false;
                            self.move_state.path_to_dest.clear();
                            self.move_state.path_speed = 0.0;
                        }
                    }
                    if is_ours && pu.deleted {
                        // Local death/disconnect — stop treating as in motion.
                        self.move_state.in_motion = false;
                        self.move_state.awaiting_force_ack = false;
                        self.move_state.dest_truncated = false;
                        // C++ death path also drops nextActionMessageToSend.
                        self.cancel_pending_action();
                        self.clear_multi_move();
                        self.player_action_pending = false;
                    }
                    // L-HUD: flush FX deferred while feeder was mid-walk.
                    if !pu.deleted {
                        let feeder_done = !self
                            .world
                            .get(pu.player_id)
                            .map(|o| o.moving)
                            .unwrap_or(false)
                            || pu.done_moving_seq_num > 0;
                        if feeder_done {
                            self.flush_deferred_fx_for(pu.player_id);
                        }
                    }
                    events.push(SessionEvent::PlayerUpdate {
                        pu,
                        force_ack_sent,
                    });
                }
                // Emit first now; queue the rest so multi-line PU is not collapsed.
                let mut iter = events.into_iter();
                let first = iter.next().unwrap();
                self.pending_events.extend(iter);
                Ok(first)
            }
            InboundMessage::PlayerMovesStart(v) => {
                self.world.apply_moves_start(&v);
                // C++ ~20139–20506: own truncated PM replaces path and cancels nextAction.
                // Also refine fractional currentPos speed from PM total_sec (L-MOVE).
                if let Some(our) = self.our_id {
                    for m in &v {
                        if m.player_id == our {
                            if m.trunc != 0 {
                                self.move_state
                                    .on_own_path_truncated(m.xs, m.ys, &m.deltas);
                                self.cancel_pending_action();
                                self.clear_multi_move();
                            }
                            self.move_state.on_own_pm_timing(
                                m.xs,
                                m.ys,
                                &m.deltas,
                                m.total_sec,
                            );
                        }
                    }
                }
                Ok(SessionEvent::PlayerMovesStart(v))
            }
            InboundMessage::MapChunk(h) => {
                // Text-only MC (no binary) — still record header.
                self.note_first_mc_offset(&h);
                self.last_mc = Some(h.clone());
                Ok(SessionEvent::MapChunk(h))
            }
            InboundMessage::MapChanges(v) => {
                // L-SOUND-TRIG: creation / decay on MX (C++ LivingLifePage ~17138+)
                self.play_mx_sounds(&v);
                // L-HUD: our homeMarker stake → homePosStack (C++ ~17238)
                self.apply_home_marker_mx(&v);
                self.map.apply_mx_many(&v);
                Ok(SessionEvent::MapChanges(v))
            }
            InboundMessage::FoodChange(f) => {
                // C++ ~21867: defer FX while feeder still has mid-walk pending.
                // Soft-FB proxy: responsible player currently `moving` (no readyPending).
                if f.responsible_id > 0 {
                    if let Some(o) = self.world.get(f.responsible_id) {
                        if o.moving {
                            self.deferred_fx.push((f.responsible_id, f.clone()));
                            // Do not update session.food yet — chrome waits for feeder settle.
                            return Ok(SessionEvent::FoodChange(f));
                        }
                    }
                }
                self.food = Some(f.clone());
                Ok(SessionEvent::FoodChange(f))
            }
            InboundMessage::HeatChange(h) => {
                self.heat = Some(h.clone());
                Ok(SessionEvent::HeatChange(h))
            }
            InboundMessage::PlayerSays(v) => {
                // L-SAY: store spoken bubble on LiveObject (keep event for probes).
                // C++ mCurseSound on successful PS isCurse (~20703) — SoundBank lazy path.
                for ps in &v {
                    if ps.is_curse {
                        if let Some(o) = self.world.get(ps.player_id) {
                            let (mx, my) = (o.x as f32, o.y as f32);
                            let _ = self.sounds.play_curse_sound_at(mx, my);
                        }
                    }
                }
                self.world.apply_says(&v);
                Ok(SessionEvent::PlayerSays(v))
            }
            InboundMessage::Cursed(v) => {
                // P3#16: curse name + immediate tag bubble.
                self.world.apply_cursed(&v);
                Ok(SessionEvent::Cursed(v))
            }
            InboundMessage::LocationSays(v) => {
                // L-SAY: world locationSpeech list (replace same cell).
                self.world.apply_location_says(&v);
                Ok(SessionEvent::LocationSays(v))
            }
            InboundMessage::PlayerEmot(v) => {
                // L-EMOT: resolve extraAnimIndex + TTL from emotion table
                // P3#19: creation sounds on PE apply (C++ newEmotPlaySound; skip ttl=-2)
                let dur = self.emotions.default_duration_sec;
                let sound_targets = self.world.apply_emots_with_bank(
                    &v,
                    Some(&self.emotions),
                    dur,
                );
                crate::sound_bank::play_emot_creation_for_targets(
                    &mut self.sounds,
                    &self.content,
                    &self.emotions,
                    &sound_targets,
                );
                Ok(SessionEvent::PlayerEmot(v))
            }
            InboundMessage::PlayerOutOfRange(v) => {
                self.world.apply_out_of_range(&v);
                Ok(SessionEvent::PlayerOutOfRange(v))
            }
            InboundMessage::BabyWiggle(v) => {
                // P3#22: BW → held baby bounce / ground flip (C++ ~21748).
                self.world.apply_baby_wiggle(&v);
                Ok(SessionEvent::BabyWiggle(v))
            }
            InboundMessage::Names(v) => {
                self.world.apply_names(&v);
                Ok(SessionEvent::Names(v))
            }
            InboundMessage::Lineages(v) => {
                self.world.apply_lineages(&v);
                Ok(SessionEvent::Lineages(v))
            }
            InboundMessage::Dying(v) => {
                self.world.apply_dying(&v);
                Ok(SessionEvent::Dying(v))
            }
            InboundMessage::Healed(v) => {
                self.world.apply_healed(&v);
                Ok(SessionEvent::Healed(v))
            }
            InboundMessage::Frame => Ok(SessionEvent::Frame),
            InboundMessage::ForcedShutdown => Ok(SessionEvent::ForcedShutdown),
            InboundMessage::ServerHello(h) => Ok(SessionEvent::Hello(h)),
            InboundMessage::Login(o) => {
                // C++: ACCEPTED enables waitForFrameMessages for subsequent traffic.
                if matches!(o, LoginOutcome::Accepted) {
                    self.wait_for_frame_messages = true;
                }
                Ok(SessionEvent::Login(o))
            }
            InboundMessage::Apocalypse => Ok(SessionEvent::Apocalypse),
            InboundMessage::ApocalypseDone => Ok(SessionEvent::ApocalypseDone),
            InboundMessage::Pong(id) => Ok(SessionEvent::Pong(id)),
            InboundMessage::GlobalMessage(m) => Ok(SessionEvent::GlobalMessage(m)),
            InboundMessage::CurseTokens(c) => {
                self.curse_tokens = Some(c.curse_token_count);
                Ok(SessionEvent::CurseTokens(c))
            }
            InboundMessage::CurseScore(c) => {
                self.excess_curse_points = Some(c.excess_curse_points);
                Ok(SessionEvent::CurseScore(c))
            }
            InboundMessage::ValleySpacing(v) => Ok(SessionEvent::ValleySpacing(v)),
            InboundMessage::FlightDest(f) => Ok(SessionEvent::FlightDest(f)),
            InboundMessage::Flip(v) => {
                // C++ FLIP: set holdingFlip when not mid-path (LivingLifePage ~14992).
                self.world.apply_flips(&v);
                Ok(SessionEvent::Flip(v))
            }
            InboundMessage::Craving(c) => Ok(SessionEvent::Craving(c)),
            InboundMessage::PosseJoin(v) => Ok(SessionEvent::PosseJoin(v)),
            InboundMessage::MonumentCall(m) => Ok(SessionEvent::MonumentCall(m)),
            InboundMessage::Ghost(v) => Ok(SessionEvent::Ghost(v)),
            InboundMessage::Compressed(h) => Ok(SessionEvent::Compressed {
                raw_size: h.binary_raw_size,
                compressed_size: h.binary_compressed_size,
            }),
            InboundMessage::Known { tag, .. } if tag == ServerTag::Bb => {
                // C++ BAD_BIOMES: replace mBadBiomeIndices for pathfind edge routing.
                self.apply_bad_biomes_message(&body);
                Ok(SessionEvent::Other(body))
            }
            InboundMessage::Known { .. } | InboundMessage::Unknown { .. } => {
                Ok(SessionEvent::Other(body))
            }
        }
    }

    /// Apply server `BB` body into [`Self::bad_biomes`] (C++ `mBadBiomeIndices`).
    pub fn apply_bad_biomes_message(&mut self, body: &str) {
        self.bad_biomes = crate::pathfind::parse_bad_biome_ids(body);
    }

    /// C++ rideable `ignoreBad`: holding a rideable vehicle.
    pub fn holding_rideable(&self) -> bool {
        let hid = self.our_held_id();
        hid > 0
            && self
                .content
                .get(hid)
                .map(|d| d.rideable)
                .unwrap_or(false)
    }

    /// Path options for click-to-move (BB list + rideable ignoreBad).
    ///
    /// Manual pointerDown: `auto_click = false` (edge-of-bad may enter a bad dest).
    pub fn path_find_opts(&self) -> crate::pathfind::PathFindOpts<'_> {
        self.path_find_opts_with(false)
    }

    /// Like [`Self::path_find_opts`] with C++ `isAutoClick` (hold / road auto-walk).
    ///
    /// When `auto_click` is true and the player stands on *good* terrain, bad-biome
    /// cells stay blocked so continuous repath cannot enter bad biomes from an edge.
    pub fn path_find_opts_with(&self, auto_click: bool) -> crate::pathfind::PathFindOpts<'_> {
        crate::pathfind::PathFindOpts {
            bad_biomes: &self.bad_biomes,
            ignore_bad: self.holding_rideable(),
            auto_click,
        }
    }

    /// Bind local player once: prefer PU nearest map-chunk center (spawn view).
    fn maybe_bind_our_player(&mut self, pu: &PlayerUpdate) {
        if self.our_id.is_some() || pu.deleted {
            return;
        }
        // If we have MC meta, only bind when this PU is near the chunk center
        // (first post-login map is centered on the connecting player).
        if let Some(ref mc) = self.last_mc {
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
        self.world.set_our_id(pu.player_id);
        self.move_state.x = pu.x;
        self.move_state.y = pu.y;
        self.move_state.current_pos_x = pu.x as f64;
        self.move_state.current_pos_y = pu.y as f64;
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
        // C++ sendToServerSocket updates timeLastMessageSent on every outbound line.
        self.last_tx = Instant::now();
        if let Some(log) = &self.wire_log {
            log.tx(message);
        }
        write_message(&mut self.stream, message)
    }

    /// Seconds since last client→server write (`timeLastMessageSent` parity).
    pub fn seconds_since_last_tx(&self) -> f64 {
        self.last_tx.elapsed().as_secs_f64()
    }

    /// True when idle longer than [`Self::ka_idle`] (default 15s).
    pub fn needs_ka(&self) -> bool {
        self.last_tx.elapsed() >= self.ka_idle
    }

    /// C++: if connected and idle > 15s, `sendToServerSocket("KA 0 0#")`.
    ///
    /// Returns `Ok(Some(line))` when a KA was written, `Ok(None)` when still within idle window.
    pub fn maybe_send_ka(&mut self) -> io::Result<Option<String>> {
        if !self.needs_ka() {
            return Ok(None);
        }
        let line = encode_ka(0, 0);
        self.send_raw(&line)?;
        Ok(Some(line))
    }

    pub fn send_ka(&mut self) -> io::Result<()> {
        self.send_raw(&encode_ka(0, 0))
    }

    /// Send typed speech: normal text → `SAY`; exact emotion trigger → `EMOT`.
    ///
    /// // C++ LivingLifePage say-field submit (~27071–27090):
    /// // `/happy` etc. → getEmotionIndex → `EMOT 0 0 N#` (not SAY).
    /// // Other `/` commands stay local (fps/die residual) — no wire.
    /// // Plain speech → `SAY 0 0 text#` (allowed mid-MOVE).
    ///
    /// Returns the wire line sent, or empty string when local-only / empty.
    pub fn send_say(&mut self, text: &str) -> io::Result<String> {
        match classify_speech_outbound(text, &self.emotions) {
            SpeechOutbound::Say(line) | SpeechOutbound::Emot { line, .. } => {
                self.send_raw(&line)?;
                Ok(line)
            }
            SpeechOutbound::LocalOnly => Ok(String::new()),
        }
    }

    /// Force a raw `SAY` without emotion routing (probes / server commands).
    pub fn send_say_raw(&mut self, text: &str) -> io::Result<String> {
        let line = crate::actions::encode_say(0, 0, text);
        self.send_raw(&line)?;
        Ok(line)
    }

    /// Force `EMOT 0 0 index#` (tests / scripted emotes).
    pub fn send_emot(&mut self, emot_index: i32) -> io::Result<String> {
        let line = crate::actions::encode_emot(0, 0, emot_index);
        self.send_raw(&line)?;
        Ok(line)
    }

    /// Queued object action (`nextActionMessageToSend`), if any.
    pub fn pending_action(&self) -> Option<&ObjectAction> {
        self.pending_action.as_ref()
    }

    /// Queue `nextActionMessageToSend` without sending (path-to-adjacent after MOVE).
    ///
    /// C++ always buffers until step flush / done_moving. Replaces any prior pending action.
    pub fn queue_pending_action(&mut self, action: ObjectAction) {
        self.pending_action = Some(action);
    }

    /// Send or queue an object action (`nextActionMessageToSend` parity).
    ///
    /// Always returns the encoded line. When mid-MOVE / awaiting FORCE the line is
    /// **not** sent yet (queued); otherwise it is written immediately and
    /// [`Self::player_action_pending`] is set (C++ `playerActionPending`).
    pub fn send_object_action(&mut self, action: ObjectAction) -> io::Result<String> {
        let line = action.encode();
        if self.move_state.in_motion || self.move_state.awaiting_force_ack {
            self.pending_action = Some(action);
            return Ok(line);
        }
        self.send_raw(&line)?;
        // Action on the wire — block further clicks until post-action PU.
        self.player_action_pending = true;
        // P3#22: start action-wiggle bounce (C++ pendingActionAnimationProgress).
        if let Some(oid) = self.our_id {
            if let Some(o) = self.world.get_mut(oid) {
                let (tx, ty) = action.target_xy();
                o.action_target_x = tx;
                o.action_target_y = ty;
                o.start_pending_action_anim();
            }
        }
        Ok(line)
    }

    /// Flush `nextActionMessageToSend` when free to act **and** adjacent to target.
    ///
    /// C++ (~23193–23280): after move ends (not on FORCE — FORCE **cancels** the queue);
    /// requires `isGridAdjacent` / same tile / `playerActionTargetNotAdjacent` before send.
    /// Headless omits the short pending-action anim delay; adjacency is enforced.
    /// Sets [`Self::player_action_pending`] when the action is written.
    pub fn flush_pending_action(&mut self) -> io::Result<Option<String>> {
        if self.move_state.in_motion || self.move_state.awaiting_force_ack {
            return Ok(None);
        }
        let Some(action) = self.pending_action.as_ref() else {
            return Ok(None);
        };
        let (tx, ty) = action.target_xy();
        let (px, py) = (self.move_state.x, self.move_state.y);
        if !crate::click_tile::can_execute_action_at(px, py, tx, ty) {
            // Still walking short of target or path truncated — keep queue.
            return Ok(None);
        }
        let action = self.pending_action.take().unwrap();
        // L-SOUND-TRIG: local using/creation preview on USE/DROP/SELF flush
        // (C++ also plays from MX/PU; this covers the send-side click).
        self.play_action_sound(&action);
        let line = action.encode();
        self.send_raw(&line)?;
        self.player_action_pending = true;
        // P3#22: start action wiggle bounce (C++ pendingActionAnimationProgress ~23220).
        if let Some(oid) = self.our_id {
            if let Some(o) = self.world.get_mut(oid) {
                let (tx, ty) = action.target_xy();
                o.action_target_x = tx;
                o.action_target_y = ty;
                o.start_pending_action_anim();
            }
        }
        Ok(Some(line))
    }

    /// Apply FX that were held while `responsible_id` was still mid-walk.
    ///
    /// // C++: pendingReceivedMessages drain after feeder path settles
    fn flush_deferred_fx_for(&mut self, player_id: i32) {
        if self.deferred_fx.is_empty() {
            return;
        }
        let mut kept = Vec::new();
        let mut last_apply: Option<FoodChange> = None;
        for (rid, fx) in self.deferred_fx.drain(..) {
            if rid == player_id {
                last_apply = Some(fx);
            } else {
                kept.push((rid, fx));
            }
        }
        self.deferred_fx = kept;
        if let Some(fx) = last_apply {
            self.food = Some(fx);
        }
    }

    /// C++ homeMarker MX: when **we** place/remove a stake, update `homePosStack`.
    fn apply_home_marker_mx(&mut self, changes: &[crate::parse::MapChange]) {
        let our = match self.our_id {
            Some(id) => id,
            None => return,
        };
        for ch in changes {
            // C++ uses responsiblePlayerID; MapChange.player_id is that field.
            let caused_by_us = ch.player_id == our || ch.player_id == -our;
            if !caused_by_us {
                continue;
            }
            let old = self.map.get_or_empty(ch.x, ch.y);
            let old_is = self
                .content
                .get(old.object_id)
                .map(|d| d.home_marker)
                .unwrap_or(false);
            let new_is = self
                .content
                .get(ch.object_id)
                .map(|d| d.home_marker)
                .unwrap_or(false);
            self.world
                .apply_home_marker_mx(ch.x, ch.y, old_is, new_is, true);
        }
    }

    /// MX creation / decay / floor / **contained fill** / **contained-slot using-on-fill**.
    ///
    /// Creation gated by [`crate::sound_bank::should_creation_sound_play`]
    /// (C++ ~12971 / MX ~16812–17364). **P2#13** residual paths.
    fn play_mx_sounds(&mut self, changes: &[crate::parse::MapChange]) {
        use crate::client_map::parse_object_raw_contained;
        use crate::sound_bank::{play_mx_change_sounds, MxSoundContext};
        for ch in changes {
            let old = self.map.get_or_empty(ch.x, ch.y);
            let new_contained: Vec<i32> = parse_object_raw_contained(&ch.object_id_raw)
                .into_iter()
                .map(|n| n.id)
                .collect();

            let (responsible_held, responsible_display) = if ch.player_id > 0 {
                self.world
                    .get(ch.player_id)
                    .map(|p| {
                        (
                            p.held_id,
                            if p.display_id > 0 { p.display_id } else { 0 },
                        )
                    })
                    .unwrap_or((0, 0))
            } else {
                (0, 0)
            };
            let causing_held = if ch.player_id < -1 {
                let pid = -ch.player_id;
                self.world.get(pid).map(|p| p.held_id).unwrap_or(0)
            } else {
                0
            };

            let ctx = MxSoundContext {
                old_object_id: old.object_id,
                old_floor_id: old.floor_id,
                old_contained: old.contained_ids(),
                new_object_id: ch.object_id,
                new_floor_id: ch.floor_id,
                new_contained,
                player_id: ch.player_id,
                is_moving: ch.is_moving(),
                map_x: ch.x,
                map_y: ch.y,
                responsible_held,
                responsible_display,
                causing_held,
            };
            let _ = play_mx_change_sounds(&mut self.sounds, &self.content, &ctx);
        }
    }

    /// PU-side clothing equip/remove, held creation, drop settle / baby put-down.
    ///
    /// // C++ LivingLifePage PU block ~18372–19100
    fn play_pu_sounds(&mut self, pu: &crate::parse::PlayerUpdate) {
        use crate::content::sound_usage_is_blank;
        use crate::live_object::ClothingSet;
        use crate::sound_bank::{
            maybe_register_off_screen_sound, play_clothing_change_sound,
            play_clothing_contained_fill_sound, play_drop_settle_sound, play_object_event_sound,
            should_creation_sound_play,
        };

        if pu.deleted {
            return;
        }
        let new_clothing = ClothingSet::parse(&pu.clothing_set);
        let Some(existing) = self.world.get(pu.player_id) else {
            // First sighting — no delta sounds.
            return;
        };
        let old_held = existing.held_id;
        let new_held = pu.held_id;
        let old_clothing = existing.clothing.clone();
        let display_id = if existing.display_id > 0 {
            existing.display_id
        } else {
            pu.display_id
        };
        let pos_x = existing.x as f32;
        let pos_y = existing.y as f32;
        let mut clothing_played = false;
        let mut creation_played = false;
        let mut other_played = false;

        // Clothing add/remove using sound (C++ ~18372)
        if old_clothing.slots != new_clothing.slots {
            clothing_played = play_clothing_change_sound(
                &mut self.sounds,
                &self.content,
                &old_clothing,
                &new_clothing,
                display_id,
            );
            // Clothing bag fill: more contained in a slot (C++ ~19400–19451).
            if !clothing_played {
                clothing_played = play_clothing_contained_fill_sound(
                    &mut self.sounds,
                    &self.content,
                    &old_clothing,
                    &new_clothing,
                    display_id,
                    pos_x,
                    pos_y,
                );
            }
        }

        // Held auto-decay: old held → new held with transition source -1 (C++ ~18841)
        if old_held > 0
            && new_held > 0
            && old_held != new_held
            && pu.held_transition_source_id == -1
            && !pu.held_origin_valid
        {
            if let Some(old_def) = self.content.get(old_held) {
                if play_object_event_sound(&mut self.sounds, &old_def.decay_sound) {
                    other_played = true;
                }
            }
        }

        // Pickup from map: held origin valid (C++ ~18673 — person using)
        if pu.held_origin_valid && new_held > 0 && old_held != new_held {
            if let Some(person) = self.content.get(display_id) {
                if play_object_event_sound(&mut self.sounds, &person.using_sound) {
                    other_played = true;
                }
            }
        }

        // Held creation (transition result in hands) — C++ ~18908+
        // Requires heldTransitionSourceID >= 0 (not pure container pull / auto-decay only).
        if new_held > 0
            && old_held != new_held
            && !clothing_played
            && !pu.held_origin_valid
            && pu.held_transition_source_id >= 0
        {
            if let Some(held_def) = self.content.get(new_held) {
                if !sound_usage_is_blank(&held_def.creation_sound) {
                    let mut test_ancestor = old_held;
                    if old_held <= 0 && pu.held_transition_source_id > 0 {
                        test_ancestor = pu.held_transition_source_id;
                    }
                    let force = held_def.creation_sound_force;
                    let gate = should_creation_sound_play(
                        &self.content,
                        test_ancestor.max(0),
                        new_held,
                    );
                    if (gate || force) && (!other_played || force) {
                        if play_object_event_sound(&mut self.sounds, &held_def.creation_sound) {
                            creation_played = true;
                            // C++ ~19018–19030: non-self + offScreenSound tag → edge marker.
                            maybe_register_off_screen_sound(
                                &mut self.sounds,
                                pu.player_id,
                                self.our_id,
                                &held_def.description,
                                pos_x,
                                pos_y,
                            );
                        }
                    }
                }
            }
        }

        // Drop settle / baby put-down when hands empty or baby released
        if !clothing_played && !creation_played && !other_played && !pu.just_ate {
            if old_held < 0 && new_held >= 0 {
                // Baby put-down: play baby's person using (C++ ~5230)
                let baby_id = -old_held;
                let baby_display = self
                    .world
                    .get(baby_id)
                    .map(|b| b.display_id)
                    .unwrap_or(0);
                if baby_display > 0 {
                    let _ = play_drop_settle_sound(
                        &mut self.sounds,
                        &self.content,
                        old_held,
                        new_held,
                        baby_display,
                    );
                }
            } else if old_held > 0 && new_held == 0 && !pu.held_origin_valid {
                // Hands empty after drop — held using then person using fallback.
                // MX creation covers map-side placement sound.
                let _ = play_drop_settle_sound(
                    &mut self.sounds,
                    &self.content,
                    old_held,
                    new_held,
                    display_id,
                );
            }
        }
    }

    /// Play object `using_sound` for the action target when content has it.
    fn play_action_sound(&mut self, action: &crate::actions::ObjectAction) {
        use crate::actions::ObjectAction;
        use crate::sound_bank::play_object_event_sound;
        let (tx, ty) = action.target_xy();
        match action {
            ObjectAction::Use { .. } | ObjectAction::Remv { .. } | ObjectAction::Swap { .. } => {
                let obj_id = self
                    .map
                    .get(tx, ty)
                    .map(|t| t.object_id)
                    .unwrap_or(0);
                if obj_id > 0 {
                    if let Some(def) = self.content.get(obj_id) {
                        let _ = play_object_event_sound(&mut self.sounds, &def.using_sound);
                    }
                }
            }
            ObjectAction::Drop { .. } => {
                // Drop onto tile: using sound of held (if any) is deferred to PU;
                // floor/container using is the common C++ path on MX.
                let held = self
                    .our_id
                    .and_then(|id| self.world.get(id))
                    .map(|o| o.held_id)
                    .unwrap_or(0);
                if held > 0 {
                    if let Some(def) = self.content.get(held) {
                        let _ = play_object_event_sound(&mut self.sounds, &def.using_sound);
                    }
                }
            }
            ObjectAction::SelfAct { .. } => {
                // Eating: held eatingSound when just about to self-feed.
                let held = self
                    .our_id
                    .and_then(|id| self.world.get(id))
                    .map(|o| o.held_id)
                    .unwrap_or(0);
                if held > 0 {
                    if let Some(def) = self.content.get(held) {
                        if !crate::content::sound_usage_is_blank(&def.eating_sound) {
                            let _ =
                                play_object_event_sound(&mut self.sounds, &def.eating_sound);
                        } else {
                            let _ =
                                play_object_event_sound(&mut self.sounds, &def.using_sound);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Drop queued action without sending (C++ FORCE / death / logout paths).
    ///
    /// Does **not** clear [`Self::player_action_pending`] alone — that gate is for
    /// actions already on the wire. Callers that must free the click gate (FORCE,
    /// baby-held, death) clear `player_action_pending` themselves.
    ///
    /// Does **not** clear multi-MOVE; FORCE / ground re-click call
    /// [`Self::clear_multi_move`] separately (action cancel ≠ path cancel).
    pub fn cancel_pending_action(&mut self) {
        self.pending_action = None;
    }

    /// `JUMP 0 0#` — baby jump-out / young wiggle (C++ pointerDown gates).
    ///
    /// P3#22: when held, arms a local `babyWiggle` bounce (C++ ~24967–24973).
    pub fn send_jump(&mut self) -> io::Result<String> {
        let held = self.we_are_held_by_adult();
        // C++ limits JUMP frequency by previous wiggle ending when held.
        if held {
            if let Some(oid) = self.our_id {
                if let Some(o) = self.world.get(oid) {
                    if o.baby_wiggle {
                        // Still bouncing — skip wire + restart.
                        return Ok(String::new());
                    }
                }
            }
        }
        let line = encode_jump(0, 0);
        self.send_raw(&line)?;
        if held {
            if let Some(oid) = self.our_id {
                if let Some(o) = self.world.get_mut(oid) {
                    o.start_baby_wiggle();
                }
            }
        }
        Ok(line)
    }

    /// Our player's age from last PU (no client age-rate clock yet).
    pub fn our_age(&self) -> Option<f32> {
        self.our_id
            .and_then(|id| self.world.get(id))
            .map(|o| o.age)
    }

    /// C++ `heldByAdultID != -1` for our live object.
    pub fn we_are_held_by_adult(&self) -> bool {
        self.our_id
            .and_then(|id| self.world.get(id))
            .map(|o| o.is_held_by_adult())
            .unwrap_or(false)
    }

    /// Held object id for our player (`holdingID`), or 0.
    pub fn our_held_id(&self) -> i32 {
        self.our_id
            .and_then(|id| self.world.get(id))
            .map(|o| o.held_id)
            .unwrap_or(0)
    }

    /// Drop multi-MOVE follow-up state (new ground click / repath cancels).
    pub fn clear_multi_move(&mut self) {
        self.multi_move_chunks.clear();
        self.multi_move_goal = None;
    }

    /// Arm remaining ±16 MOVE chunks toward `goal` after sending a hop ending at `end`.
    ///
    /// Returns true when more hops (or a later repath) are still needed.
    pub fn arm_multi_move(
        &mut self,
        rest: Vec<Vec<PathDelta>>,
        goal: (i32, i32),
        end: (i32, i32),
    ) -> bool {
        self.multi_move_chunks = rest;
        let more = !self.multi_move_chunks.is_empty() || end != goal;
        if more {
            self.multi_move_goal = Some(goal);
        } else {
            self.multi_move_goal = None;
        }
        more
    }

    /// True when multi-MOVE chunks or an ultimate goal remain.
    pub fn has_multi_move(&self) -> bool {
        !self.multi_move_chunks.is_empty() || self.multi_move_goal.is_some()
    }

    /// Send the next multi-MOVE hop when free (`!in_motion`).
    ///
    /// Prefers pre-split ±16 chunks; when empty, repaths toward `multi_move_goal`
    /// without canceling a queued `nextAction`. Call **before**
    /// [`Self::flush_pending_action`] on done_moving so a long path continues and
    /// the queued USE/DROP/REMV waits until the final hop.
    ///
    /// // C++: LivingLifePage multi-hop after done_moving / pathFindingD=32
    pub fn continue_multi_move(&mut self) -> io::Result<Option<String>> {
        crate::multi_move_ext::continue_multi_move_body(self)
    }

    /// Path-to-adjacent USE/DROP/REMV then queue or send ([`crate::click_tile::click_object`]).
    pub fn path_and_act(
        &mut self,
        action: ObjectAction,
    ) -> Result<crate::click_tile::ObjectClickResult, MoveError> {
        crate::click_tile::click_object(self, action)
    }

    /// Path-to-adjacent USE on map object tile.
    pub fn path_and_use(
        &mut self,
        tile_x: i32,
        tile_y: i32,
        object_id: Option<i32>,
        slot: Option<i32>,
    ) -> Result<crate::click_tile::ObjectClickResult, MoveError> {
        crate::click_tile::click_use(self, tile_x, tile_y, object_id, slot)
    }

    /// Empty tile → MOVE; object → path-to-adjacent USE (GUI LMB).
    pub fn walk_or_use_tile(
        &mut self,
        tile_x: i32,
        tile_y: i32,
    ) -> Result<crate::click_tile::WalkOrUseResult, MoveError> {
        crate::click_tile::walk_or_use_tile(self, tile_x, tile_y)
    }

    pub fn send_move(&mut self, path: &[PathDelta]) -> Result<String, SessionError> {
        let line = self
            .move_state
            .send_move_with_offset(path, self.map_global_offset)?;
        self.send_raw(&line)?;
        // Jason: client is onPath immediately so walk anim + currentPos start before PM.
        self.mark_our_moving_from_send();
        Ok(line)
    }

    pub fn send_use(
        &mut self,
        x: i32,
        y: i32,
        object_id: Option<i32>,
        slot: Option<i32>,
    ) -> io::Result<String> {
        self.send_object_action(ObjectAction::Use {
            x,
            y,
            object_id,
            slot,
        })
    }

    pub fn send_drop(&mut self, x: i32, y: i32, clothing_slot: i32) -> io::Result<String> {
        self.send_object_action(ObjectAction::Drop {
            x,
            y,
            clothing_slot,
        })
    }

    pub fn send_remv(&mut self, x: i32, y: i32, slot: i32) -> io::Result<String> {
        self.send_object_action(ObjectAction::Remv { x, y, slot })
    }

    pub fn send_self(&mut self, x: i32, y: i32, clothing_slot: i32) -> io::Result<String> {
        self.send_object_action(ObjectAction::SelfAct {
            x,
            y,
            clothing_slot,
        })
    }

    /// `SREMV x y c i#` — remove from worn clothing container.
    pub fn send_sremv(
        &mut self,
        x: i32,
        y: i32,
        clothing_slot: i32,
        slot: i32,
    ) -> io::Result<String> {
        self.send_object_action(ObjectAction::Sremv {
            x,
            y,
            clothing_slot,
            slot,
        })
    }

    /// Headless: DROP held into own clothing slot (`c` 0..5) at self.
    pub fn click_drop_clothing(
        &mut self,
        clothing_slot: i32,
    ) -> Result<crate::click_tile::ObjectClickResult, MoveError> {
        crate::click_tile::click_drop_clothing(self, clothing_slot)
    }

    /// Headless: self-click (eat / equip / SREMV) with optional clothing slot.
    pub fn click_self(
        &mut self,
        clothing_slot: i32,
        hit_slot: i32,
        mod_click: bool,
    ) -> Result<crate::click_tile::ObjectClickResult, MoveError> {
        crate::click_tile::click_self(self, clothing_slot, hit_slot, mod_click)
    }

    pub fn stream_mut(&mut self) -> &mut TcpStream {
        &mut self.stream
    }

    /// Last map chunk header if any (for tests / probes).
    pub fn last_map_chunk(&self) -> Option<&MapChunkHeader> {
        self.last_mc.as_ref()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::login::hmac_sha1_hex;
    use std::io::Write;
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    fn fixture_peer() -> (u16, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            write_message(&mut sock, "SN\n1/20\ntest_challenge_xyz\n184\n").unwrap();
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
            let parts: Vec<&str> = login_body.split_whitespace().collect();
            assert!(parts.len() >= 6);
            let pw_hash = parts[parts.len() - 3];
            let key_hash = parts[parts.len() - 2];
            let expected_pw = hmac_sha1_hex("secret", "test_challenge_xyz");
            // C++ getPureAccountKey: uppercase + strip hyphens before HMAC
            let expected_key = hmac_sha1_hex(
                &crate::login::pure_account_key("key123"),
                "test_challenge_xyz",
            );
            assert_eq!(pw_hash, expected_pw);
            assert_eq!(key_hash, expected_key);

            write_message(&mut sock, "ACCEPTED\n").unwrap();
            write_message(&mut sock, "MC\n32 30 0 0\n0 0\n").unwrap();
            // Multi-line PU: our player + other + delete — each must surface as its own event.
            write_message(
                &mut sock,
                "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n\
8 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 18 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n\
9 100 0 0 0 0 0 0 0 0 -1 0 0 0 X X reason_disconnected\n",
            )
            .unwrap();
            write_message(&mut sock, "FX\n10 20 0 0 3.75 -1 0 0\n").unwrap();
            write_message(&mut sock, "HX\n0.50 1.00 0.00\n").unwrap();
            write_message(&mut sock, "MX\n16 15 0 33 -1\n").unwrap();
            write_message(&mut sock, "PS\n7/0 HI *map 1 2 3\n").unwrap();
            write_message(&mut sock, "MS\nhello_world\n").unwrap();
            write_message(&mut sock, "AP\n").unwrap();
            write_message(&mut sock, "PONG\nping1\n").unwrap();
            // CM-wrapped PE (inflate via FrameReader) — held until FM like other gameplay.
            let inner = b"PE\n7 4 10\n";
            let comp = crate::frame::compress_cm_payload(inner).unwrap();
            let cm_hdr = format!("CM\n{} {}\n#", inner.len(), comp.len());
            sock.write_all(cm_hdr.as_bytes()).unwrap();
            sock.write_all(&comp).unwrap();
            write_message(&mut sock, "FM\n").unwrap();
            thread::sleep(Duration::from_millis(200));
        });
        (port, handle)
    }

    fn test_cfg(port: u16) -> SessionConfig {
        SessionConfig {
            host: "127.0.0.1".into(),
            port,
            email: "user@test".into(),
            password: "secret".into(),
            account_key: "key123".into(),
            pad_email_to_80: false,
            read_timeout: Duration::from_millis(400),
            write_timeout: Duration::from_secs(2),
            ..SessionConfig::default()
        }
    }

    /// Minimal peer: SN + LOGIN check + ACCEPTED, then caller-supplied bodies.
    fn login_then_peer(bodies: Vec<Vec<u8>>) -> (u16, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            write_message(&mut sock, "SN\n1/20\ntest_challenge_xyz\n184\n").unwrap();
            let mut fr = FrameReader::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = sock.read(&mut buf).unwrap();
                if n == 0 {
                    return;
                }
                let msgs = fr.push(&buf[..n]);
                if msgs.into_iter().any(|m| m.starts_with("LOGIN ") || m.starts_with("RLOGIN ")) {
                    break;
                }
            }
            write_message(&mut sock, "ACCEPTED\n").unwrap();
            for body in bodies {
                sock.write_all(&body).unwrap();
            }
            thread::sleep(Duration::from_millis(150));
        });
        (port, handle)
    }

    /// Peer that records every complete `#`-terminated text frame from the client after LOGIN.
    fn login_then_peer_capture(
        bodies: Vec<Vec<u8>>,
        captured: Arc<Mutex<Vec<String>>>,
    ) -> (u16, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            write_message(&mut sock, "SN\n1/20\ntest_challenge_xyz\n184\n").unwrap();
            let mut fr = FrameReader::new();
            let mut buf = [0u8; 4096];
            // Wait for LOGIN.
            loop {
                let n = match sock.read(&mut buf) {
                    Ok(0) => return,
                    Ok(n) => n,
                    Err(_) => return,
                };
                let msgs = fr.push(&buf[..n]);
                if msgs
                    .into_iter()
                    .any(|m| m.starts_with("LOGIN ") || m.starts_with("RLOGIN "))
                {
                    break;
                }
            }
            write_message(&mut sock, "ACCEPTED\n").unwrap();
            for body in bodies {
                sock.write_all(&body).unwrap();
            }
            // Collect further client TX (FORCE, KA, USE, …) until idle.
            sock.set_read_timeout(Some(Duration::from_millis(300))).ok();
            loop {
                let n = match sock.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => break,
                };
                let msgs = fr.push(&buf[..n]);
                if let Ok(mut c) = captured.lock() {
                    c.extend(msgs);
                }
            }
        });
        (port, handle)
    }

    fn framed_text(s: &str) -> Vec<u8> {
        crate::frame::encode_raw(s).into_bytes()
    }

    #[test]
    fn login_and_inbound_tags_fixture() {
        let (port, handle) = fixture_peer();
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        assert_eq!(session.login, LoginOutcome::Accepted);
        assert!(session.wait_for_frame_messages());
        assert_eq!(session.hello.challenge, "test_challenge_xyz");

        let mut saw_mc = false;
        let mut pu_ids = Vec::new();
        let mut saw_delete = false;
        let mut saw_fx = false;
        let mut saw_hx = false;
        let mut saw_mx = false;
        let mut saw_ps = false;
        let mut saw_ms = false;
        let mut saw_ap = false;
        let mut saw_pong = false;
        let mut saw_pe = false;
        let mut saw_fm = false;
        let mut tags_order: Vec<String> = Vec::new();
        for _ in 0..40 {
            match session.poll_event() {
                Ok(ev) => {
                    tags_order.push(ev.tag_str().to_string());
                    match ev {
                        SessionEvent::MapChunk(h) => {
                            saw_mc = true;
                            assert_eq!((h.size_x, h.size_y), (32, 30));
                        }
                        SessionEvent::PlayerUpdate { pu, .. } => {
                            pu_ids.push(pu.player_id);
                            if pu.player_id == 7 {
                                assert_eq!((pu.x, pu.y), (16, 15));
                                if session.our_id.is_none() {
                                    session.our_id = Some(7);
                                    session.world.set_our_id(7);
                                }
                                assert_eq!(session.our_id, Some(7));
                                assert!(session.world.get(7).is_some());
                            }
                            if pu.deleted {
                                saw_delete = true;
                                assert_eq!(pu.delete_reason.as_deref(), Some("reason_disconnected"));
                            }
                        }
                        SessionEvent::FoodChange(f) => {
                            saw_fx = true;
                            assert_eq!(f.food_store, 10);
                            assert_eq!(session.food.as_ref().map(|x| x.food_store), Some(10));
                        }
                        SessionEvent::HeatChange(h) => {
                            saw_hx = true;
                            assert!((h.heat - 0.5).abs() < 0.01);
                            assert!(session.heat.is_some());
                        }
                        SessionEvent::MapChanges(v) => {
                            saw_mx = true;
                            assert_eq!(v[0].object_id, 33);
                        }
                        SessionEvent::PlayerSays(v) => {
                            saw_ps = true;
                            assert!(v[0].text.contains("HI"));
                            assert!(v[0].map.is_some());
                        }
                        SessionEvent::GlobalMessage(m) => {
                            saw_ms = true;
                            assert_eq!(m.text, "hello world");
                        }
                        SessionEvent::Apocalypse => saw_ap = true,
                        SessionEvent::Pong(id) => {
                            saw_pong = true;
                            assert_eq!(id, "ping1");
                        }
                        SessionEvent::PlayerEmot(v) => {
                            saw_pe = true;
                            assert_eq!(v[0].player_id, 7);
                            assert_eq!(v[0].emot_index, 4);
                        }
                        // C++ discards FM; we emit Frame as a post-batch probe boundary.
                        SessionEvent::Frame => saw_fm = true,
                        _ => {}
                    }
                }
                Err(_) => break,
            }
        }
        assert!(
            saw_mc
                && pu_ids.contains(&7)
                && pu_ids.contains(&8)
                && pu_ids.contains(&9)
                && saw_delete
                && saw_fx
                && saw_hx
                && saw_mx
                && saw_ps
                && saw_ms
                && saw_ap
                && saw_pong
                && saw_pe
                && saw_fm,
            "mc={saw_mc} pu_ids={pu_ids:?} del={saw_delete} fx={saw_fx} hx={saw_hx} mx={saw_mx} \
             ps={saw_ps} ms={saw_ms} ap={saw_ap} pong={saw_pong} pe={saw_pe} fm={saw_fm} order={tags_order:?}"
        );
        // Pass-through MC/PONG may surface before held batch; FM boundary only after drain.
        let fm_pos = tags_order.iter().position(|t| t == "FM").expect("FM boundary");
        let first_pu = tags_order.iter().position(|t| t == "PU").expect("PU");
        assert!(
            first_pu < fm_pos,
            "PU must apply before Frame boundary: {tags_order:?}"
        );
        let _ = handle.join();
    }

    /// After ACCEPTED, PU without FM must not surface / apply.
    #[test]
    fn hold_pu_until_fm() {
        let pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let (port, handle) = login_then_peer(vec![framed_text(pu)]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        assert!(session.wait_for_frame_messages());
        // No FM → read eventually fails (timeout or peer close); world stays empty.
        let err = session.poll_event().expect_err("must not release without FM");
        assert!(
            matches!(
                err.kind(),
                io::ErrorKind::WouldBlock
                    | io::ErrorKind::TimedOut
                    | io::ErrorKind::UnexpectedEof
                    | io::ErrorKind::Interrupted
            ) || format!("{err}").to_lowercase().contains("timed")
                || format!("{err}").contains("closed"),
            "unexpected err kind {:?}: {err}",
            err.kind()
        );
        assert!(session.world.get(7).is_none(), "PU must not apply before FM");
        let _ = handle.join();
    }

    /// PU then MX then FM: drain order PU then MX then Frame.
    #[test]
    fn drain_order_pu_mx_fm() {
        let pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let bodies = vec![
            framed_text(pu),
            framed_text("MX\n16 15 0 33 -1\n"),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer(bodies);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        let mut order = Vec::new();
        for _ in 0..8 {
            match session.poll_event() {
                Ok(ev) => order.push(ev.tag_str().to_string()),
                Err(_) => break,
            }
        }
        assert_eq!(order, vec!["PU", "MX", "FM"], "order={order:?}");
        assert!(session.world.get(7).is_some());
        let _ = handle.join();
    }

    /// Mid-wait MC is immediate; buffered PS only after FM.
    #[test]
    fn passthrough_mc_while_ps_held() {
        let bodies = vec![
            framed_text("PS\n7/0 HI\n"),
            framed_text("MC\n32 30 0 0\n0 0\n"),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer(bodies);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        // First ready surface may be MC pass-through while PS is buffered.
        let first = session.poll_event().unwrap();
        assert!(
            matches!(first, SessionEvent::MapChunk(_)),
            "expected MC pass-through first, got {:?}",
            first.tag_str()
        );
        let mut saw_ps = false;
        let mut saw_fm = false;
        for _ in 0..8 {
            match session.poll_event() {
                Ok(SessionEvent::PlayerSays(_)) => saw_ps = true,
                Ok(SessionEvent::Frame) => saw_fm = true,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(saw_ps && saw_fm, "ps={saw_ps} fm={saw_fm}");
        let _ = handle.join();
    }

    /// PONG mid-batch returns immediately; non-pass-through still held until FM.
    #[test]
    fn passthrough_pong_mid_batch() {
        let pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let bodies = vec![
            framed_text(pu),
            framed_text("PONG\nping9\n"),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer(bodies);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        let first = session.poll_event().unwrap();
        assert!(
            matches!(first, SessionEvent::Pong(ref id) if id == "ping9"),
            "PONG must pass through before held PU: {:?}",
            first.tag_str()
        );
        assert!(session.world.get(7).is_none(), "PU still held after PONG");
        let mut saw_pu = false;
        let mut saw_fm = false;
        for _ in 0..8 {
            match session.poll_event() {
                Ok(SessionEvent::PlayerUpdate { .. }) => saw_pu = true,
                Ok(SessionEvent::Frame) => saw_fm = true,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(saw_pu && saw_fm, "pu={saw_pu} fm={saw_fm}");
        let _ = handle.join();
    }

    /// FD mid-batch same as PONG/MC pass-through.
    #[test]
    fn passthrough_fd_mid_batch() {
        let bodies = vec![
            framed_text("MX\n1 1 0 5 -1\n"),
            // protocol: player_id dest_x dest_y
            framed_text("FD\n7 10 20\n"),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer(bodies);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        let first = session.poll_event().unwrap();
        assert!(
            matches!(
                first,
                SessionEvent::FlightDest(ref f) if f.player_id == 7 && f.dest_x == 10 && f.dest_y == 20
            ),
            "FD pass-through first, got {:?}",
            first.tag_str()
        );
        let mut order = Vec::new();
        for _ in 0..8 {
            match session.poll_event() {
                Ok(ev) => order.push(ev.tag_str().to_string()),
                Err(_) => break,
            }
        }
        assert_eq!(order, vec!["MX", "FM"], "order={order:?}");
        let _ = handle.join();
    }

    /// Empty FM alone does not release a batch; following PU+FM does.
    #[test]
    fn empty_fm_alone_does_not_release() {
        let pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let bodies = vec![
            framed_text("FM\n"),
            framed_text(pu),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer(bodies);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        let mut order = Vec::new();
        for _ in 0..8 {
            match session.poll_event() {
                Ok(ev) => order.push(ev.tag_str().to_string()),
                Err(_) => break,
            }
        }
        assert_eq!(order, vec!["PU", "FM"], "order={order:?}");
        let _ = handle.join();
    }

    /// Two frames back-to-back do not merge across FM.
    #[test]
    fn two_frames_do_not_merge() {
        let pu = |id: i32| {
            format!(
                "PU\n\
{id} 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n"
            )
        };
        let bodies = vec![
            framed_text(&pu(1)),
            framed_text("FM\n"),
            framed_text(&pu(2)),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer(bodies);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        let mut order = Vec::new();
        let mut ids = Vec::new();
        for _ in 0..12 {
            match session.poll_event() {
                Ok(SessionEvent::PlayerUpdate { pu, .. }) => {
                    order.push("PU".into());
                    ids.push(pu.player_id);
                }
                Ok(SessionEvent::Frame) => order.push("FM".into()),
                Ok(ev) => order.push(ev.tag_str().to_string()),
                Err(_) => break,
            }
        }
        assert_eq!(order, vec!["PU", "FM", "PU", "FM"], "order={order:?}");
        assert_eq!(ids, vec![1, 2], "ids={ids:?}");
        let _ = handle.join();
    }

    /// Multi-line PU still one SessionEvent per line after batch release; Frame last.
    #[test]
    fn multi_pu_then_frame_boundary() {
        let pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n\
8 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 18 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let bodies = vec![framed_text(pu), framed_text("FM\n")];
        let (port, handle) = login_then_peer(bodies);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        let mut order = Vec::new();
        for _ in 0..8 {
            match session.poll_event() {
                Ok(ev) => order.push(ev.tag_str().to_string()),
                Err(_) => break,
            }
        }
        assert_eq!(order, vec!["PU", "PU", "FM"], "order={order:?}");
        let _ = handle.join();
    }

    #[test]
    fn framed_helpers_classify() {
        assert!(framed_is_fm(&FramedMessage::Text("FM\n".into())));
        assert!(framed_is_fm(&FramedMessage::Text("FM".into())));
        assert!(!framed_is_fm(&FramedMessage::Text("PU\n1".into())));
        assert!(framed_is_frame_passthrough(&FramedMessage::MapChunk {
            header: "MC\n1 1 0 0\n0 0\n".into(),
            compressed: vec![],
        }));
        assert!(framed_is_frame_passthrough(&FramedMessage::Text(
            "PONG\nx\n".into()
        )));
        assert!(framed_is_frame_passthrough(&FramedMessage::Text(
            "FD\n1 2\n".into()
        )));
        assert!(framed_is_frame_passthrough(&FramedMessage::Text(
            "PH\nsig\n".into()
        )));
        assert!(!framed_is_frame_passthrough(&FramedMessage::Text(
            "PU\n1\n".into()
        )));
    }

    // -----------------------------------------------------------------------
    // L-MOVE force_flush_logout_ka
    // -----------------------------------------------------------------------

    /// FORCE ack mid-frame: cancel pending USE (do not flush), snap, send FORCE.
    #[test]
    fn force_cancels_pending_does_not_flush() {
        // Frame 1: MC + bind PU; Frame 2: forced-pos PU for our player.
        // force=1 at (20, 21); done_moving unused when forced.
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        // force field index 13 = 1, x=20 y=21
        let force_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 0 1 20 21 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let captured = Arc::new(Mutex::new(Vec::new()));
        let bodies = vec![
            framed_text("MC\n32 30 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
            framed_text(force_pu),
            framed_text("MX\n20 21 0 99 -1\n"),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer_capture(bodies, Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();

        // Drain first frame → bind our player.
        for _ in 0..8 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert_eq!(session.our_id, Some(7));
        assert_eq!((session.move_state.x, session.move_state.y), (16, 15));

        // Simulate mid-move queued action (nextActionMessageToSend).
        session.move_state.in_motion = true;
        let _ = session
            .send_object_action(ObjectAction::Use {
                x: 17,
                y: 15,
                object_id: Some(33),
                slot: None,
            })
            .unwrap();
        assert!(session.pending_action().is_some());

        // Process force frame: FORCE must go out; pending USE cancelled.
        let mut force_ack = None;
        let mut saw_mx = false;
        for _ in 0..10 {
            match session.poll_event() {
                Ok(SessionEvent::PlayerUpdate {
                    force_ack_sent, ..
                }) => {
                    force_ack = force_ack_sent;
                }
                Ok(SessionEvent::MapChanges(_)) => saw_mx = true,
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert_eq!(force_ack.as_deref(), Some("FORCE 20 21#"));
        assert_eq!((session.move_state.x, session.move_state.y), (20, 21));
        assert!(!session.move_state.in_motion);
        assert!(!session.move_state.awaiting_force_ack);
        assert!(
            session.pending_action().is_none(),
            "FORCE must cancel nextAction, not flush it"
        );
        assert!(saw_mx, "MX after FORCE in same frame still applies");

        // Wire: FORCE sent; no USE (queue was cancelled, not flushed).
        let _ = handle.join();
        let tx = captured.lock().unwrap().clone();
        assert!(
            tx.iter().any(|m| m.starts_with("FORCE 20 21")),
            "FORCE on wire: {tx:?}"
        );
        assert!(
            !tx.iter().any(|m| m.starts_with("USE ")),
            "USE must not flush after FORCE: {tx:?}"
        );
    }

    /// Non-force done_moving flushes queued action.
    #[test]
    fn done_moving_flushes_pending_action() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        // done_moving_seq=2 matches after client MOVE seq 2; force=0
        let done_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 2 0 17 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let captured = Arc::new(Mutex::new(Vec::new()));
        let bodies = vec![
            framed_text("MC\n32 30 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
            framed_text(done_pu),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer_capture(bodies, Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..8 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert_eq!(session.our_id, Some(7));
        // Client thinks it sent MOVE @2 and is mid-move with a queued USE.
        session.move_state.last_move_sequence_number = 2;
        session.move_state.in_motion = true;
        session.move_state.x = 17;
        session.move_state.y = 15;
        let _ = session
            .send_object_action(ObjectAction::Use {
                x: 17,
                y: 15,
                object_id: None,
                slot: None,
            })
            .unwrap();
        assert!(session.pending_action().is_some());

        for _ in 0..8 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(!session.move_state.in_motion);
        assert!(session.pending_action().is_none());

        let _ = handle.join();
        let tx = captured.lock().unwrap().clone();
        assert!(
            tx.iter().any(|m| m.starts_with("USE 17 15")),
            "done_moving must flush pending USE: {tx:?}"
        );
    }

    /// Non-matching done_moving keeps in_motion and does not flush pending.
    #[test]
    fn non_matching_done_moving_keeps_pending() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        // done_moving_seq=1 != client last seq 2
        let stale_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let captured = Arc::new(Mutex::new(Vec::new()));
        let bodies = vec![
            framed_text("MC\n32 30 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
            framed_text(stale_pu),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer_capture(bodies, Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..8 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        session.move_state.last_move_sequence_number = 2;
        session.move_state.in_motion = true;
        session.move_state.x = 17;
        session.move_state.y = 15;
        let _ = session
            .send_object_action(ObjectAction::Use {
                x: 17,
                y: 15,
                object_id: None,
                slot: None,
            })
            .unwrap();
        assert!(session.pending_action().is_some());

        for _ in 0..8 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(
            session.move_state.in_motion,
            "stale done_moving must not clear in_motion"
        );
        assert!(
            session.pending_action().is_some(),
            "pending must stay queued until matching done_moving"
        );

        let _ = handle.join();
        let tx = captured.lock().unwrap().clone();
        assert!(
            !tx.iter().any(|m| m.starts_with("USE ")),
            "must not flush USE on non-matching done_moving: {tx:?}"
        );
    }

    /// Own truncated PM cancels pending action (does not flush).
    #[test]
    fn truncation_cancels_pending_action() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        // PM: our id 7, start 16 15, trunc=1, path to +2,+0
        let trunc_pm = "PM\n\
7 16 15 1.0 0.5 1 1 0 2 0\n";
        let captured = Arc::new(Mutex::new(Vec::new()));
        let bodies = vec![
            framed_text("MC\n32 30 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
            framed_text(trunc_pm),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer_capture(bodies, Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..8 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert_eq!(session.our_id, Some(7));
        session.move_state.in_motion = true;
        session.move_state.last_move_sequence_number = 2;
        session.move_state.x = 21;
        session.move_state.y = 15;
        let _ = session
            .send_object_action(ObjectAction::Use {
                x: 21,
                y: 15,
                object_id: Some(9),
                slot: None,
            })
            .unwrap();
        assert!(session.pending_action().is_some());

        for _ in 0..8 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(SessionEvent::PlayerMovesStart(_)) => {}
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(session.move_state.dest_truncated);
        assert_eq!((session.move_state.x, session.move_state.y), (18, 15));
        assert!(
            session.pending_action().is_none(),
            "truncation must cancel nextAction"
        );

        let _ = handle.join();
        let tx = captured.lock().unwrap().clone();
        assert!(
            !tx.iter().any(|m| m.starts_with("USE ")),
            "truncation must not flush USE: {tx:?}"
        );
    }

    /// destTruncated + PU pos mismatch → artificial FORCE + cancel pending.
    #[test]
    fn artificial_force_on_truncated_dest_mismatch() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        // force=0 but client has dest_truncated with xd,yd != PU x,y
        let mismatch_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 2 0 19 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let captured = Arc::new(Mutex::new(Vec::new()));
        let bodies = vec![
            framed_text("MC\n32 30 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
            framed_text(mismatch_pu),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer_capture(bodies, Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..8 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        // Simulate prior truncated path ending at (18,15).
        session.move_state.last_move_sequence_number = 2;
        session.move_state.in_motion = true;
        session.move_state.x = 18;
        session.move_state.y = 15;
        session.move_state.dest_truncated = true;
        let _ = session
            .send_object_action(ObjectAction::Drop {
                x: 18,
                y: 15,
                clothing_slot: -1,
            })
            .unwrap();
        assert!(session.pending_action().is_some());

        let mut force_ack = None;
        for _ in 0..8 {
            match session.poll_event() {
                Ok(SessionEvent::PlayerUpdate {
                    force_ack_sent, ..
                }) => {
                    force_ack = force_ack_sent;
                }
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert_eq!(force_ack.as_deref(), Some("FORCE 19 15#"));
        assert_eq!((session.move_state.x, session.move_state.y), (19, 15));
        assert!(!session.move_state.dest_truncated);
        assert!(session.pending_action().is_none());

        let _ = handle.join();
        let tx = captured.lock().unwrap().clone();
        assert!(
            tx.iter().any(|m| m.starts_with("FORCE 19 15")),
            "artificial FORCE on wire: {tx:?}"
        );
        assert!(
            !tx.iter().any(|m| m.starts_with("DROP ")),
            "pending DROP must be cancelled, not flushed: {tx:?}"
        );
    }

    /// FORCE with mismatched done_moving still snaps and acks (session path).
    #[test]
    fn force_mismatched_done_moving_still_acks() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        // force=1, done_moving=99 (not matching), pos 30 31
        let force_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 99 1 30 31 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let captured = Arc::new(Mutex::new(Vec::new()));
        let bodies = vec![
            framed_text("MC\n32 30 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
            framed_text(force_pu),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer_capture(bodies, Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..8 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        session.move_state.last_move_sequence_number = 2;
        session.move_state.in_motion = true;

        let mut force_ack = None;
        for _ in 0..8 {
            match session.poll_event() {
                Ok(SessionEvent::PlayerUpdate {
                    force_ack_sent, ..
                }) => {
                    force_ack = force_ack_sent;
                }
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert_eq!(force_ack.as_deref(), Some("FORCE 30 31#"));
        assert_eq!((session.move_state.x, session.move_state.y), (30, 31));
        assert!(!session.move_state.in_motion);

        // Next MOVE must originate from forced coords.
        session.move_state.awaiting_force_ack = false;
        let line = session
            .send_move(&[PathDelta { x: 1, y: 0 }])
            .unwrap();
        assert_eq!(line, "MOVE 30 31 @3 1 0#");

        let _ = handle.join();
        let tx = captured.lock().unwrap().clone();
        assert!(tx.iter().any(|m| m.starts_with("FORCE 30 31")));
        assert!(tx.iter().any(|m| m.starts_with("MOVE 30 31 @3")));
    }

    /// Baby-held interrupt cancels our pending action (C++ ~19845).
    #[test]
    fn baby_held_cancels_pending_action() {
        let bind_pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        // Adult 8 holds baby 7 (held_id = -7)
        let adult_hold = "PU\n\
8 100 1 0 0 0 -7 0 0 0 -1 0.5 0 0 20 20 30.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let captured = Arc::new(Mutex::new(Vec::new()));
        let bodies = vec![
            framed_text("MC\n32 30 0 0\n0 0\n"),
            framed_text(bind_pu),
            framed_text("FM\n"),
            framed_text(adult_hold),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer_capture(bodies, Arc::clone(&captured));
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..8 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert_eq!(session.our_id, Some(7));
        session.move_state.in_motion = true;
        let _ = session
            .send_object_action(ObjectAction::Use {
                x: 17,
                y: 15,
                object_id: None,
                slot: None,
            })
            .unwrap();
        assert!(session.pending_action().is_some());

        for _ in 0..8 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(session.pending_action().is_none());
        assert!(!session.move_state.in_motion);

        let _ = handle.join();
        let tx = captured.lock().unwrap().clone();
        assert!(!tx.iter().any(|m| m.starts_with("USE ")));
    }

    /// Multi-second fixture: frames keep applying after T>5s; MOVE + USE succeed.
    ///
    /// Proves poll/apply does not stall after a few seconds (goal stall fix).
    #[test]
    fn sustained_poll_move_interact_past_five_seconds() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let frames_sent = Arc::new(AtomicUsize::new(0));
        let frames_sent2 = Arc::clone(&frames_sent);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let peer = thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            write_message(&mut sock, "SN\n1/20\ntest_challenge_xyz\n184\n").unwrap();
            let mut fr = FrameReader::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = sock.read(&mut buf).unwrap_or(0);
                if n == 0 {
                    return;
                }
                let msgs = fr.push(&buf[..n]);
                if msgs
                    .into_iter()
                    .any(|m| m.starts_with("LOGIN ") || m.starts_with("RLOGIN "))
                {
                    break;
                }
            }
            write_message(&mut sock, "ACCEPTED\n").unwrap();
            // MC first: center (16,15) so maybe_bind_our_player accepts PU nearby.
            write_message(&mut sock, "MC\n32 30 0 0\n0 0\n").unwrap();
            write_message(
                &mut sock,
                "PU\n7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n",
            )
            .unwrap();
            write_message(&mut sock, "MX\n16 15 0 33 -1\n").unwrap();
            write_message(&mut sock, "FM\n").unwrap();
            frames_sent2.fetch_add(1, Ordering::SeqCst);
            // Stream frames for ~7s so client can poll past the stall window.
            let t0 = Instant::now();
            let mut n = 0u32;
            while t0.elapsed() < Duration::from_secs(7) {
                n += 1;
                // Separate `#` frames (Jason wire) — one tag per write_message.
                write_message(
                    &mut sock,
                    "PU\n7 100 1 0 0 0 0 0 0 0 -1 0.5 0 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n",
                )
                .unwrap();
                write_message(
                    &mut sock,
                    &format!("MX\n16 15 0 {} -1\n", 33 + (n % 3) as i32),
                )
                .unwrap();
                write_message(&mut sock, "FM\n").unwrap();
                frames_sent2.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(200));
            }
        });

        let mut cfg = test_cfg(port);
        cfg.read_timeout = Duration::from_millis(50);
        let mut session = ClientSession::connect(&cfg).unwrap();
        assert!(matches!(session.login, LoginOutcome::Accepted));

        let start = Instant::now();
        let mut events_after_5s = 0usize;
        let mut total_ok = 0usize;
        let mut saw_mx = false;
        let mut sent_move = false;
        let mut sent_use = false;

        while start.elapsed() < Duration::from_secs(6) {
            let _ = session.maybe_send_ka();
            // Early MOVE once bound (walk path).
            if session.our_id.is_some() && !sent_move {
                session.move_state.in_motion = false;
                session.move_state.awaiting_force_ack = false;
                if session
                    .send_move(&[PathDelta { x: 1, y: 0 }])
                    .is_ok()
                {
                    sent_move = true;
                }
            }
            // Interact: after MOVE, free the motion gate (fixture has no done_moving PM)
            // then send USE — proves object-action path on a live session past join.
            if sent_move && !sent_use {
                session.move_state.in_motion = false;
                session.move_state.awaiting_force_ack = false;
                session.player_action_pending = false;
                let x = session.move_state.x;
                let y = session.move_state.y;
                if session.send_use(x, y, Some(33), None).is_ok() {
                    sent_use = true;
                }
            }
            match session.poll_event() {
                Ok(ev) => {
                    total_ok += 1;
                    if start.elapsed() >= Duration::from_secs(5) {
                        events_after_5s += 1;
                    }
                    if matches!(ev, SessionEvent::MapChanges(_)) {
                        saw_mx = true;
                    }
                    // Step anim/move clocks like a real client frame.
                    session.step_move_pos(1.0 / 60.0);
                }
                Err(e) => {
                    let k = e.kind();
                    if k != std::io::ErrorKind::WouldBlock && k != std::io::ErrorKind::TimedOut {
                        // Peer closed at end is ok after 6s wall.
                        if start.elapsed() < Duration::from_secs(5) {
                            panic!("poll failed early: {e}");
                        }
                        break;
                    }
                }
            }
        }
        let _ = peer.join();
        assert!(session.our_id.is_some(), "must bind our player");
        assert!(sent_move, "must send MOVE");
        assert!(sent_use, "must send USE interact after freeing motion");
        assert!(
            saw_mx || total_ok > 10,
            "must apply world updates (mx={saw_mx} total={total_ok})"
        );
        assert!(
            events_after_5s > 0,
            "must still apply events after 5s (got {events_after_5s}); total={total_ok} frames_sent={}",
            frames_sent.load(Ordering::SeqCst)
        );
        assert!(
            frames_sent.load(Ordering::SeqCst) >= 10,
            "fixture must stream multiple frames"
        );
    }

    /// KA auto-send after idle window (C++ 15s; test uses short ka_idle).
    #[test]
    fn maybe_send_ka_after_idle() {
        let (port, handle) = login_then_peer(vec![]);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        assert!(!session.needs_ka());
        assert!(session.maybe_send_ka().unwrap().is_none());

        session.ka_idle = Duration::from_millis(20);
        thread::sleep(Duration::from_millis(40));
        assert!(session.needs_ka());
        let line = session.maybe_send_ka().unwrap();
        assert_eq!(line.as_deref(), Some("KA 0 0#"));
        assert!(!session.needs_ka(), "send_raw must refresh last_tx");
        assert!(session.maybe_send_ka().unwrap().is_none());
        let _ = handle.join();
    }

    /// logout_reset clears FM wait, pending action, world, map, our_id.
    #[test]
    fn logout_reset_clears_frame_and_world() {
        let pu = "PU\n\
7 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 16 15 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n";
        let bodies = vec![
            framed_text("MC\n32 30 0 0\n0 0\n"),
            framed_text(pu),
            framed_text("FX\n5 20 0 0 3.75 -1 0 0\n"),
            framed_text("FM\n"),
        ];
        let (port, handle) = login_then_peer(bodies);
        let mut session = ClientSession::connect(&test_cfg(port)).unwrap();
        for _ in 0..12 {
            match session.poll_event() {
                Ok(SessionEvent::Frame) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        assert!(session.wait_for_frame_messages());
        assert_eq!(session.our_id, Some(7));
        assert!(session.world.get(7).is_some());
        assert!(session.food.is_some());
        session.move_state.in_motion = true;
        session.pending_action = Some(ObjectAction::Drop {
            x: 1,
            y: 2,
            clothing_slot: -1,
        });

        session.logout_reset();

        assert!(!session.wait_for_frame_messages());
        assert!(session.our_id.is_none());
        assert!(session.world.get(7).is_none());
        assert!(session.food.is_none());
        assert!(session.heat.is_none());
        assert!(session.pending_action().is_none());
        assert!(!session.move_state.in_motion);
        assert!(!session.move_state.awaiting_force_ack);
        assert_eq!(session.move_state.last_move_sequence_number, 1);
        assert!(session.last_map_chunk().is_none());
        let _ = handle.join();
    }
}
