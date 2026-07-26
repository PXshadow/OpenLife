//! OHOL-style game protocol: ASCII messages terminated by `#`.
//!
//! Keep this crate pure (no I/O, no world state). Unit-test against fixtures.

#![forbid(unsafe_code)]

mod tags;
mod wire_out;

pub use tags::{
    format_photo_signature, format_pong, format_vog_update, ClientTag, ServerTag,
    PHOTO_DENIED_SIGNATURE,
};
pub use wire_out::{
    format_baby_wiggle, format_curse_score_change, format_curse_token_change, format_dying,
    format_exile_wire, format_following_wire, format_food_change, format_grave_place,
    format_frame, format_heat_change, format_held_update, format_learned_tool_report,
    format_location_says, format_player_says,
    format_map_change, format_map_change_moving, format_name_message, format_player_emot,
    format_player_moves_start,
    format_tool_slots, format_weather_status,
};

use thiserror::Error;

/// Frame delimiter used by the vanilla One Hour One Life protocol.
pub const MESSAGE_TERMINATOR: u8 = b'#';

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProtocolError {
    #[error("message is empty")]
    Empty,
    #[error("missing tag")]
    MissingTag,
    #[error("invalid utf-8 in message")]
    InvalidUtf8,
    #[error("invalid field count for {tag}: expected {expected}, got {got}")]
    FieldCount {
        tag: &'static str,
        expected: usize,
        got: usize,
    },
    #[error("invalid integer field")]
    InvalidInt,
    #[error("unknown or unsupported client tag: {0}")]
    UnknownTag(String),
}

/// One decoded client or server message (tag + remaining payload text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub tag: String,
    pub payload: String,
}

impl Message {
    pub fn fields(&self) -> Vec<&str> {
        self.payload
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .collect()
    }
}

/// Split a complete message body (without trailing `#`) into tag + payload.
pub fn parse_message(body: &str) -> Result<Message, ProtocolError> {
    let body = body.trim();
    if body.is_empty() {
        return Err(ProtocolError::Empty);
    }
    let mut parts = body.splitn(2, char::is_whitespace);
    let tag = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or(ProtocolError::MissingTag)?;
    let payload = parts.next().unwrap_or("").trim_start().to_string();
    Ok(Message {
        tag: tag.to_string(),
        payload,
    })
}

/// Extract complete `#`-terminated frames from a byte buffer.
/// Returns decoded UTF-8 bodies (without `#`) and leftover unparsed bytes.
pub fn extract_frames(buffer: &[u8]) -> Result<(Vec<String>, Vec<u8>), ProtocolError> {
    let mut frames = Vec::new();
    let mut start = 0usize;
    for (i, b) in buffer.iter().enumerate() {
        if *b == MESSAGE_TERMINATOR {
            let slice = &buffer[start..i];
            let s = std::str::from_utf8(slice).map_err(|_| ProtocolError::InvalidUtf8)?;
            if !s.is_empty() {
                frames.push(s.to_string());
            }
            start = i + 1;
        }
    }
    Ok((frames, buffer[start..].to_vec()))
}

/// Format a **client→server** style one-liner (rare for server replies).
/// Prefer [`format_server_message`] for server→client (Haxe `Connection.send`).
pub fn format_message(tag: &str, payload: &str) -> String {
    if payload.is_empty() {
        format!("{tag}\n#")
    } else {
        format!("{tag}\n{payload}\n#")
    }
}

/// Server→client wire format matching Haxe Open Life:
/// `tag\n` + optional data lines joined by `\n` + trailing `\n#`
/// (see `Connection.send`: `'$tag\n${data.join("\n")}\n#'`).
pub fn format_server_message(tag: &str, data_lines: &[&str]) -> String {
    if data_lines.is_empty() {
        format!("{tag}\n#")
    } else {
        format!("{tag}\n{}\n#", data_lines.join("\n"))
    }
}

/// Multi-line server message (alias of [`format_server_message`]).
pub fn format_multiline(tag: &str, lines: &[&str]) -> String {
    format_server_message(tag, lines)
}

/// Build the SN / SERVER_INFO greeting after TCP accept.
/// Haxe: `send(SERVER_INFO, ["0/0", challenge, '$version'])` where SERVER_INFO = "SN".
pub fn format_sn(
    current_players: u32,
    max_players: u32,
    challenge: &str,
    required_version: i32,
) -> String {
    format_server_message(
        "SN",
        &[
            &format!("{current_players}/{max_players}"),
            challenge,
            &required_version.to_string(),
        ],
    )
}

/// Minimal PU line matching Haxe `PlayerInstance.toData` field order.
///
/// **`done_moving_seq` must be the player's real `done_moving_seqNum`** (not a
/// hardcoded 1). After MOVE `@N` completes, subsequent USE/DROP PUs must still
/// carry `N` or clients stay mid-action / ignore interactions.
#[allow(clippy::too_many_arguments)]
pub fn format_player_update_line(
    p_id: i32,
    po_id: i32,
    held_id: i32,
    x: i32,
    y: i32,
    age: f32,
    move_speed: f32,
    done_moving_seq: i32,
) -> String {
    format_player_update_line_eat(
        p_id,
        po_id,
        held_id,
        x,
        y,
        age,
        move_speed,
        0,
        0,
        done_moving_seq,
    )
}

/// PU line with explicit eat fields (Haxe `just_ate` / `last_ate_id`).
#[allow(clippy::too_many_arguments)]
pub fn format_player_update_line_eat(
    p_id: i32,
    po_id: i32,
    held_id: i32,
    x: i32,
    y: i32,
    age: f32,
    move_speed: f32,
    just_ate: i32,
    last_ate: i32,
    done_moving_seq: i32,
) -> String {
    format_player_update_line_full(
        p_id,
        po_id,
        held_id,
        x,
        y,
        age,
        move_speed,
        just_ate,
        last_ate,
        0, // force
        0, // action
        0,
        0, // action target
        0, // o_origin_valid
        0,
        0, // origin
        -1, // o_trans
        done_moving_seq.max(1),
    )
}

/// Full Haxe `PlayerInstance.toData` field set.
///
/// Order: p_id po_id facing action atx aty held origin_valid ox oy o_trans heat
/// seq force x y age age_r speed clothing just_ate last_ate responsible yum learned
///
/// - `action=1` + `atx/aty` = USE in progress (Haxe SetTransitionData)
/// - `force=1` = snap/unstick (Haxe forced)
/// - `seq` = done_moving_seqNum (>0 means stationary at pos)
#[allow(clippy::too_many_arguments)]
pub fn format_player_update_line_full(
    p_id: i32,
    po_id: i32,
    held_id: i32,
    x: i32,
    y: i32,
    age: f32,
    move_speed: f32,
    just_ate: i32,
    last_ate: i32,
    force: i32,
    action: i32,
    atx: i32,
    aty: i32,
    o_origin_valid: i32,
    ox: i32,
    oy: i32,
    o_trans: i32,
    seq: i32,
) -> String {
    format!(
        "{p_id} {po_id} 0 {action} {atx} {aty} {held_id} {o_origin_valid} {ox} {oy} {o_trans} 0.50 {seq} {force} {x} {y} {age:.2} 60.00 {move_speed:.2} 0;0;0;0;0;0 {just_ate} {last_ate} -1 0 0"
    )
}

/// Parsed client commands we care about in early phases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientCommand {
    Login(LoginMessage),
    RLogin(LoginMessage),
    Ka { x: i32, y: i32 },
    Use {
        x: i32,
        y: i32,
        id: Option<i32>,
        index: Option<i32>,
    },
    Drop {
        x: i32,
        y: i32,
        c: Option<i32>,
    },
    Remv {
        x: i32,
        y: i32,
        i: Option<i32>,
    },
    Say { text: String },
    Emot { x: i32, y: i32, e: i32 },
    Die { x: i32, y: i32 },
    Jump { x: i32, y: i32 },
    Kill {
        x: i32,
        y: i32,
        id: Option<i32>,
    },
    Ping {
        x: i32,
        y: i32,
        unique_id: String,
    },
    /// MOVE xs ys @seq_num xdelt0 ydelt0 ...
    Move {
        xs: i32,
        ys: i32,
        seq: Option<i32>,
        deltas: Vec<(i32, i32)>,
    },
    /// Recognized tag but not fully parsed yet; keep raw payload.
    Raw { tag: ClientTag, payload: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginMessage {
    pub client_tag: String,
    pub email: String,
    pub password_hash: String,
    pub account_key_hash: String,
    pub tutorial_number: i32,
    pub twin_code_hash: Option<String>,
    pub twin_count: Option<i32>,
}

fn parse_i32(s: &str) -> Result<i32, ProtocolError> {
    s.parse().map_err(|_| ProtocolError::InvalidInt)
}

/// Parse a full client message body (without `#`) into a structured command.
pub fn parse_client_command(body: &str) -> Result<ClientCommand, ProtocolError> {
    let msg = parse_message(body)?;
    let tag = ClientTag::parse(&msg.tag).ok_or_else(|| ProtocolError::UnknownTag(msg.tag.clone()))?;
    let f = msg.fields();

    match tag {
        ClientTag::Login | ClientTag::RLogin => {
            // LOGIN client_tag email password_hash account_key_hash tutorial [twin_hash twin_count]
            if f.len() < 5 {
                return Err(ProtocolError::FieldCount {
                    tag: tag.as_str(),
                    expected: 5,
                    got: f.len(),
                });
            }
            let login = LoginMessage {
                client_tag: f[0].to_string(),
                email: f[1].to_string(),
                password_hash: f[2].to_string(),
                account_key_hash: f[3].to_string(),
                tutorial_number: parse_i32(f[4])?,
                twin_code_hash: f.get(5).map(|s| (*s).to_string()),
                twin_count: f.get(6).map(|s| parse_i32(s)).transpose()?,
            };
            Ok(if tag == ClientTag::RLogin {
                ClientCommand::RLogin(login)
            } else {
                ClientCommand::Login(login)
            })
        }
        ClientTag::Ka => {
            if f.len() < 2 {
                return Err(ProtocolError::FieldCount {
                    tag: "KA",
                    expected: 2,
                    got: f.len(),
                });
            }
            Ok(ClientCommand::Ka {
                x: parse_i32(f[0])?,
                y: parse_i32(f[1])?,
            })
        }
        ClientTag::Use => {
            if f.len() < 2 {
                return Err(ProtocolError::FieldCount {
                    tag: "USE",
                    expected: 2,
                    got: f.len(),
                });
            }
            Ok(ClientCommand::Use {
                x: parse_i32(f[0])?,
                y: parse_i32(f[1])?,
                id: f.get(2).map(|s| parse_i32(s)).transpose()?,
                index: f.get(3).map(|s| parse_i32(s)).transpose()?,
            })
        }
        ClientTag::Drop => {
            if f.len() < 2 {
                return Err(ProtocolError::FieldCount {
                    tag: "DROP",
                    expected: 2,
                    got: f.len(),
                });
            }
            Ok(ClientCommand::Drop {
                x: parse_i32(f[0])?,
                y: parse_i32(f[1])?,
                c: f.get(2).map(|s| parse_i32(s)).transpose()?,
            })
        }
        ClientTag::Remv => {
            if f.len() < 2 {
                return Err(ProtocolError::FieldCount {
                    tag: "REMV",
                    expected: 2,
                    got: f.len(),
                });
            }
            Ok(ClientCommand::Remv {
                x: parse_i32(f[0])?,
                y: parse_i32(f[1])?,
                i: f.get(2).map(|s| parse_i32(s)).transpose()?,
            })
        }
        ClientTag::Say => Ok(ClientCommand::Say {
            text: msg.payload.clone(),
        }),
        ClientTag::Emot => {
            if f.len() < 3 {
                return Err(ProtocolError::FieldCount {
                    tag: "EMOT",
                    expected: 3,
                    got: f.len(),
                });
            }
            Ok(ClientCommand::Emot {
                x: parse_i32(f[0])?,
                y: parse_i32(f[1])?,
                e: parse_i32(f[2])?,
            })
        }
        ClientTag::Die => {
            if f.len() < 2 {
                return Err(ProtocolError::FieldCount {
                    tag: "DIE",
                    expected: 2,
                    got: f.len(),
                });
            }
            Ok(ClientCommand::Die {
                x: parse_i32(f[0])?,
                y: parse_i32(f[1])?,
            })
        }
        ClientTag::Jump => {
            if f.len() < 2 {
                return Err(ProtocolError::FieldCount {
                    tag: "JUMP",
                    expected: 2,
                    got: f.len(),
                });
            }
            Ok(ClientCommand::Jump {
                x: parse_i32(f[0])?,
                y: parse_i32(f[1])?,
            })
        }
        ClientTag::Kill => {
            if f.len() < 2 {
                return Err(ProtocolError::FieldCount {
                    tag: "KILL",
                    expected: 2,
                    got: f.len(),
                });
            }
            Ok(ClientCommand::Kill {
                x: parse_i32(f[0])?,
                y: parse_i32(f[1])?,
                id: f.get(2).map(|s| parse_i32(s)).transpose()?,
            })
        }
        ClientTag::Ping => {
            if f.len() < 3 {
                return Err(ProtocolError::FieldCount {
                    tag: "PING",
                    expected: 3,
                    got: f.len(),
                });
            }
            Ok(ClientCommand::Ping {
                x: parse_i32(f[0])?,
                y: parse_i32(f[1])?,
                unique_id: f[2].to_string(),
            })
        }
        ClientTag::Move => {
            // MOVE xs ys @seq_num xdelt0 ydelt0 ...  OR  MOVE xs ys xdelt0 ydelt0 ...
            if f.len() < 2 {
                return Err(ProtocolError::FieldCount {
                    tag: "MOVE",
                    expected: 2,
                    got: f.len(),
                });
            }
            let xs = parse_i32(f[0])?;
            let ys = parse_i32(f[1])?;
            let mut i = 2usize;
            let mut seq = None;
            if let Some(tok) = f.get(2) {
                if let Some(s) = tok.strip_prefix('@') {
                    seq = Some(parse_i32(s)?);
                    i = 3;
                }
            }
            let mut deltas = Vec::new();
            while i + 1 < f.len() {
                deltas.push((parse_i32(f[i])?, parse_i32(f[i + 1])?));
                i += 2;
            }
            Ok(ClientCommand::Move {
                xs,
                ys,
                seq,
                deltas,
            })
        }
        other => Ok(ClientCommand::Raw {
            tag: other,
            payload: msg.payload,
        }),
    }
}

/// Generate a random-looking challenge string (ASCII alnum).
pub fn generate_challenge(len: usize) -> String {
    // Deterministic-friendly without OS RNG in pure crate: use a simple LCG seeded by time isn't pure.
    // Callers with entropy should pass their own; this provides a fixed-length placeholder pattern.
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut s = String::with_capacity(len);
    let mut state: u64 = 0xC0FFEE ^ (len as u64).wrapping_mul(0x9E37);
    for _ in 0..len {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let idx = (state >> 33) as usize % CHARS.len();
        s.push(CHARS[idx] as char);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_login_style() {
        let m = parse_message("LOGIN client_official a@b.c hash key 0 0 0").unwrap();
        assert_eq!(m.tag, "LOGIN");
        assert!(m.payload.starts_with("client_official"));
    }

    #[test]
    fn parse_login_command() {
        let cmd = parse_client_command(
            "LOGIN client_official user@example.com abcd efgh 0",
        )
        .unwrap();
        match cmd {
            ClientCommand::Login(l) => {
                assert_eq!(l.client_tag, "client_official");
                assert_eq!(l.email, "user@example.com");
                assert_eq!(l.tutorial_number, 0);
            }
            _ => panic!("expected Login"),
        }
    }

    #[test]
    fn parse_rlogin() {
        let cmd =
            parse_client_command("RLOGIN client_hetuw a@b.c p k 0 twin 2").unwrap();
        match cmd {
            ClientCommand::RLogin(l) => {
                assert_eq!(l.twin_code_hash.as_deref(), Some("twin"));
                assert_eq!(l.twin_count, Some(2));
            }
            _ => panic!("expected RLogin"),
        }
    }

    #[test]
    fn parse_ka_use_drop() {
        assert_eq!(
            parse_client_command("KA 10 20").unwrap(),
            ClientCommand::Ka { x: 10, y: 20 }
        );
        assert_eq!(
            parse_client_command("USE 1 2 33 0").unwrap(),
            ClientCommand::Use {
                x: 1,
                y: 2,
                id: Some(33),
                index: Some(0)
            }
        );
        assert_eq!(
            parse_client_command("DROP 5 6 -1").unwrap(),
            ClientCommand::Drop {
                x: 5,
                y: 6,
                c: Some(-1)
            }
        );
    }

    #[test]
    fn extract_multiple_frames() {
        let (frames, rest) = extract_frames(b"KA 0 0#USE 1 2#partial").unwrap();
        assert_eq!(frames, vec!["KA 0 0".to_string(), "USE 1 2".to_string()]);
        assert_eq!(rest, b"partial");
    }

    #[test]
    fn format_sn_shape() {
        let sn = format_sn(0, 100, "ABC123", 437);
        assert!(sn.starts_with("SN\n"));
        assert!(sn.ends_with("#") || sn.ends_with("\n#") || sn.ends_with('#'));
        assert!(sn.contains("0/100"));
        assert!(sn.contains("ABC123"));
        assert!(sn.contains("437"));
    }

    #[test]
    fn format_accepted_matches_haxe() {
        // Haxe: send(ACCEPTED) → "ACCEPTED\n#"
        assert_eq!(format_server_message("ACCEPTED", &[]), "ACCEPTED\n#");
        assert_eq!(format_message("ACCEPTED", ""), "ACCEPTED\n#");
    }

    #[test]
    fn format_sn_matches_haxe_server_info() {
        // Haxe: send(SERVER_INFO, ["0/0", challenge, version]) with SERVER_INFO="SN"
        let sn = format_sn(0, 0, "ABC123", 437);
        assert_eq!(sn, "SN\n0/0\nABC123\n437\n#");
    }

    #[test]
    fn challenge_length() {
        assert_eq!(generate_challenge(48).len(), 48);
    }

    #[test]
    fn parse_move_with_seq_and_deltas() {
        let cmd = parse_client_command("MOVE 10 20 @3 1 0 0 1").unwrap();
        match cmd {
            ClientCommand::Move {
                xs,
                ys,
                seq,
                deltas,
            } => {
                assert_eq!((xs, ys), (10, 20));
                assert_eq!(seq, Some(3));
                assert_eq!(deltas, vec![(1, 0), (0, 1)]);
            }
            _ => panic!("expected Move"),
        }
    }

    #[test]
    fn parse_photo_and_vog_as_raw() {
        match parse_client_command("PHOTO 10 20 1").unwrap() {
            ClientCommand::Raw { tag, payload } => {
                assert_eq!(tag, ClientTag::Photo);
                assert_eq!(payload, "10 20 1");
            }
            _ => panic!("expected Raw PHOTO"),
        }
        match parse_client_command("VOGS 0 0").unwrap() {
            ClientCommand::Raw { tag, payload } => {
                assert_eq!(tag, ClientTag::Vogs);
                assert!(tag.is_vog());
                assert_eq!(payload, "0 0");
            }
            _ => panic!("expected Raw VOGS"),
        }
        match parse_client_command("VOGI 1 2 99").unwrap() {
            ClientCommand::Raw { tag, .. } => assert_eq!(tag, ClientTag::Vogi),
            _ => panic!("expected Raw VOGI"),
        }
    }
}
