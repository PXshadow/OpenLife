//! Game TCP network: accept clients, send SN, read `#`-framed messages,
//! write outbound packets from sim.
//!
//! Ticket verify (Haxe `VerifyIfOholAccount`) is **on by default**; set
//! `verify_ohol_ticket = false` in server.toml for local/dev.

#![forbid(unsafe_code)]

mod login_bootstrap;
mod outbound;
mod ticket;

pub use login_bootstrap::{build_login_bootstrap, player_id_for_conn};
pub use outbound::{OutboundHub, OutboundRx};
pub use ticket::verify_ohol_ticket;

use ol_metrics::Counters;
use ol_protocol::{
    extract_frames, format_server_message, format_sn, generate_challenge, parse_client_command,
    ClientCommand, ProtocolError,
};
use ol_world::World;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum NetError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol: {0}")]
    Protocol(#[from] ProtocolError),
}

#[derive(Clone)]
pub struct NetConfig {
    pub bind: String,
    pub max_players: u32,
    pub required_version: i32,
    pub challenge_len: usize,
    /// Live sim world (read lock for MAP_CHUNK at login).
    pub shared_world: Arc<RwLock<World>>,
    pub outbound: Arc<OutboundHub>,
    /// Haxe `VerifyIfOholAccount` — default **true**.
    pub verify_ohol_ticket: bool,
    /// Ticket server URL (no query string).
    pub ticket_verify_url: String,
    /// Preferred human spawn (shared with sim — map-center grassland near self-play).
    pub preferred_spawn: Arc<RwLock<(i32, i32)>>,
}

/// Intent produced by the net layer for the simulation.
#[derive(Debug, Clone)]
pub enum NetIntent {
    Login {
        conn_id: u64,
        reconnect: bool,
        email: String,
        client_tag: String,
    },
    KeepAlive {
        conn_id: u64,
        x: i32,
        y: i32,
    },
    Use {
        conn_id: u64,
        x: i32,
        y: i32,
        id: Option<i32>,
        index: Option<i32>,
    },
    Drop {
        conn_id: u64,
        x: i32,
        y: i32,
        c: Option<i32>,
    },
    Move {
        conn_id: u64,
        xs: i32,
        ys: i32,
        deltas: Vec<(i32, i32)>,
        /// Client `@seq` from wire; None if omitted (self-play, old clients).
        seq: Option<i32>,
    },
    Raw {
        conn_id: u64,
        tag: String,
        payload: String,
    },
    Disconnected {
        conn_id: u64,
    },
}

impl NetIntent {
    /// Connection id for latency accounting (human vs AI bands).
    pub fn conn_id(&self) -> u64 {
        match self {
            Self::Login { conn_id, .. }
            | Self::KeepAlive { conn_id, .. }
            | Self::Use { conn_id, .. }
            | Self::Drop { conn_id, .. }
            | Self::Move { conn_id, .. }
            | Self::Raw { conn_id, .. }
            | Self::Disconnected { conn_id } => *conn_id,
        }
    }
}

pub type IntentTx = tokio::sync::mpsc::Sender<NetIntent>;

pub async fn run_game_listener(
    cfg: NetConfig,
    counters: Arc<Counters>,
    intent_tx: IntentTx,
) -> Result<(), NetError> {
    let listener = TcpListener::bind(&cfg.bind).await?;
    info!(
        addr = %cfg.bind,
        ticket_verify = cfg.verify_ohol_ticket,
        "game TCP listening"
    );

    let mut next_conn_id: u64 = 1;

    loop {
        let (socket, peer) = listener.accept().await?;
        let conn_id = next_conn_id;
        next_conn_id = next_conn_id.wrapping_add(1);

        counters.connections.fetch_add(1, Ordering::Relaxed);
        info!(%peer, conn_id, "client connected");

        let counters = Arc::clone(&counters);
        let intent_tx = intent_tx.clone();
        let cfg = cfg.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, conn_id, &cfg, &counters, intent_tx).await {
                warn!(conn_id, error = %e, "connection ended with error");
            }
            cfg.outbound.unregister(conn_id);
            counters.connections.fetch_sub(1, Ordering::Relaxed);
        });
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    conn_id: u64,
    cfg: &NetConfig,
    counters: &Counters,
    intent_tx: IntentTx,
) -> Result<(), NetError> {
    // Disable Nagle so small LS/PS/PM packets are not delayed/coalesced.
    if let Err(e) = socket.set_nodelay(true) {
        warn!(conn_id, error = %e, "set_nodelay failed");
    }
    let OutboundRx {
        mut urgent,
        mut normal,
    } = cfg.outbound.register(conn_id);

    let challenge = {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let chars: Vec<u8> = (0..cfg.challenge_len)
            .map(|_| {
                const C: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
                C[rng.gen_range(0..C.len())]
            })
            .collect();
        String::from_utf8(chars).unwrap_or_else(|_| generate_challenge(cfg.challenge_len))
    };

    let players = counters.connections.load(Ordering::Relaxed).saturating_sub(1) as u32;
    let sn = format_sn(players, cfg.max_players, &challenge, cfg.required_version);
    socket.write_all(sn.as_bytes()).await?;
    socket.flush().await?;

    let mut buf = Vec::with_capacity(4096);
    let mut tmp = [0u8; 2048];
    let mut logged_in = false;

    /// Write one packet; empty = close marker. Returns true if socket should exit.
    async fn write_out(
        socket: &mut TcpStream,
        conn_id: u64,
        pkt: Vec<u8>,
    ) -> Result<bool, NetError> {
        if pkt.is_empty() {
            info!(conn_id, "net: close marker — shutting down connection");
            let _ = socket.shutdown().await;
            return Ok(true);
        }
        socket.write_all(&pkt).await?;
        socket.flush().await?;
        Ok(false)
    }

    loop {
        tokio::select! {
            // 1) Urgent (PS/LS/PM) always before bulk so SAY is not stuck behind AI PU/MX.
            // 2) Normal bulk next.
            // 3) Inbound last.
            biased;
            maybe_u = urgent.recv() => {
                match maybe_u {
                    Some(pkt) => {
                        if write_out(&mut socket, conn_id, pkt).await? {
                            break;
                        }
                        // Drain remaining urgent immediately.
                        loop {
                            match urgent.try_recv() {
                                Ok(p) => {
                                    if write_out(&mut socket, conn_id, p).await? {
                                        return Ok(());
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                    }
                    None => break, // hub unregistered / closed
                }
            }
            maybe_n = normal.recv() => {
                match maybe_n {
                    Some(pkt) => {
                        if write_out(&mut socket, conn_id, pkt).await? {
                            break;
                        }
                        // Prefer any urgent that arrived while we wrote; then more normal.
                        loop {
                            let mut did = false;
                            while let Ok(p) = urgent.try_recv() {
                                did = true;
                                if write_out(&mut socket, conn_id, p).await? {
                                    return Ok(());
                                }
                            }
                            match normal.try_recv() {
                                Ok(p) => {
                                    if write_out(&mut socket, conn_id, p).await? {
                                        return Ok(());
                                    }
                                }
                                Err(_) => {
                                    if !did {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    None => break,
                }
            }
            read = socket.read(&mut tmp) => {
                let n = read?;
                if n == 0 {
                    let _ = intent_tx.send(NetIntent::Disconnected { conn_id }).await;
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
                let (frames, rest) = extract_frames(&buf)?;
                buf = rest;

                for frame in frames {
                    match parse_client_command(&frame) {
                        Ok(cmd) => {
                            if matches!(
                                cmd,
                                ClientCommand::Login(_) | ClientCommand::RLogin(_)
                            ) && !logged_in
                            {
                                let (email, account_key_hash, client_tag, reconnect, twin_hash, twin_count) =
                                    match &cmd {
                                        ClientCommand::Login(l) => (
                                            l.email.clone(),
                                            l.account_key_hash.clone(),
                                            l.client_tag.clone(),
                                            false,
                                            l.twin_code_hash.clone(),
                                            l.twin_count,
                                        ),
                                        ClientCommand::RLogin(l) => (
                                            l.email.clone(),
                                            l.account_key_hash.clone(),
                                            l.client_tag.clone(),
                                            true,
                                            l.twin_code_hash.clone(),
                                            l.twin_count,
                                        ),
                                        _ => unreachable!(),
                                    };

                                if cfg.verify_ohol_ticket {
                                    let ok = verify_ohol_ticket(
                                        &cfg.ticket_verify_url,
                                        &email,
                                        &account_key_hash,
                                        &challenge,
                                    )
                                    .await;
                                    if !ok {
                                        let rejected =
                                            format_server_message("REJECTED", &[]);
                                        socket.write_all(rejected.as_bytes()).await?;
                                        warn!(
                                            conn_id,
                                            email = %email,
                                            "LOGIN rejected (ticket verify failed)"
                                        );
                                        return Ok(());
                                    }
                                } else {
                                    info!(
                                        conn_id,
                                        email = %email,
                                        "ticket verify off — accepting LOGIN"
                                    );
                                }

                                let packets = {
                                    let world = cfg.shared_world.read().unwrap();
                                    let (sx, sy) = cfg
                                        .preferred_spawn
                                        .read()
                                        .map(|g| *g)
                                        .unwrap_or((0, 0));
                                    build_login_bootstrap(
                                        conn_id,
                                        sx,
                                        sy,
                                        14.0,
                                        10.0,
                                        20.0,
                                        0,
                                        &*world,
                                    )
                                };
                                for pkt in packets {
                                    socket.write_all(&pkt).await?;
                                }
                                socket.flush().await?;
                                logged_in = true;
                                info!(conn_id, "LOGIN bootstrap from shared sim world");

                                let intent = NetIntent::Login {
                                    conn_id,
                                    reconnect,
                                    email,
                                    client_tag,
                                };
                                if intent_tx.send(intent).await.is_err() {
                                    return Ok(());
                                }
                                // FERTILITY-TWINS: protocol twin_code_hash twin_count → wait queue
                                if let Some(hash) = twin_hash {
                                    if !hash.is_empty() {
                                        let count = twin_count.unwrap_or(2).max(2);
                                        let twin_intent = NetIntent::Raw {
                                            conn_id,
                                            tag: "TWINJOIN".into(),
                                            payload: format!("{hash} {count}"),
                                        };
                                        if intent_tx.send(twin_intent).await.is_err() {
                                            return Ok(());
                                        }
                                    }
                                }
                                continue;
                            }

                            let intent = command_to_intent(conn_id, cmd);
                            if intent_tx.send(intent).await.is_err() {
                                return Ok(());
                            }
                        }
                        Err(e) => {
                            warn!(conn_id, frame = %frame, error = %e, "bad client frame");
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn command_to_intent(conn_id: u64, cmd: ClientCommand) -> NetIntent {
    match cmd {
        ClientCommand::Login(l) => NetIntent::Login {
            conn_id,
            reconnect: false,
            email: l.email,
            client_tag: l.client_tag,
        },
        ClientCommand::RLogin(l) => NetIntent::Login {
            conn_id,
            reconnect: true,
            email: l.email,
            client_tag: l.client_tag,
        },
        ClientCommand::Ka { x, y } => NetIntent::KeepAlive { conn_id, x, y },
        ClientCommand::Use { x, y, id, index } => NetIntent::Use {
            conn_id,
            x,
            y,
            id,
            index,
        },
        ClientCommand::Drop { x, y, c } => NetIntent::Drop { conn_id, x, y, c },
        ClientCommand::Move {
            xs,
            ys,
            deltas,
            seq,
        } => NetIntent::Move {
            conn_id,
            xs,
            ys,
            deltas,
            seq,
        },
        ClientCommand::Remv { x, y, i } => {
            let payload = match i {
                Some(idx) => format!("{x} {y} {idx}"),
                None => format!("{x} {y}"),
            };
            NetIntent::Raw {
                conn_id,
                tag: "REMV".into(),
                payload,
            }
        }
        ClientCommand::Say { text } => NetIntent::Raw {
            conn_id,
            tag: "SAY".into(),
            payload: text,
        },
        ClientCommand::Die { x, y } => NetIntent::Raw {
            conn_id,
            tag: "DIE".into(),
            payload: format!("{x} {y}"),
        },
        ClientCommand::Emot { x, y, e } => NetIntent::Raw {
            conn_id,
            tag: "EMOT".into(),
            payload: format!("{x} {y} {e}"),
        },
        ClientCommand::Jump { x, y } => NetIntent::Raw {
            conn_id,
            tag: "JUMP".into(),
            payload: format!("{x} {y}"),
        },
        ClientCommand::Kill { x, y, id } => NetIntent::Raw {
            conn_id,
            tag: "KILL".into(),
            payload: match id {
                Some(i) => format!("{x} {y} {i}"),
                None => format!("{x} {y}"),
            },
        },
        other => {
            let (tag, payload) = match other {
                ClientCommand::Raw { tag, payload } => (tag.as_str().to_string(), payload),
                ClientCommand::Ping { unique_id, .. } => ("PING".into(), unique_id),
                _ => ("?".into(), String::new()),
            };
            NetIntent::Raw {
                conn_id,
                tag,
                payload,
            }
        }
    }
}
