//! Minimal server→client message parsing needed for login + movement sync.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("not an SN message")]
    NotSn,
    #[error("malformed SN message: {0}")]
    MalformedSn(String),
    #[error("not a login outcome message")]
    NotLoginOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerHello {
    pub current_players: i32,
    pub max_players: i32,
    pub challenge: String,
    pub required_version: i32,
}

/// Parse `SN\ncurrent/max\nchallenge\nversion` body (without trailing `#`).
pub fn parse_sn(body: &str) -> Result<ServerHello, ParseError> {
    let body = body.trim_start_matches('\u{feff}').trim();
    let mut lines = body.lines();
    let first = lines.next().unwrap_or("").trim();
    if first != "SN" {
        return Err(ParseError::NotSn);
    }
    let players = lines
        .next()
        .ok_or_else(|| ParseError::MalformedSn("missing players line".into()))?
        .trim();
    let challenge = lines
        .next()
        .ok_or_else(|| ParseError::MalformedSn("missing challenge".into()))?
        .trim()
        .to_string();
    let version_line = lines
        .next()
        .ok_or_else(|| ParseError::MalformedSn("missing version".into()))?
        .trim();

    let (cur, max) = players
        .split_once('/')
        .ok_or_else(|| ParseError::MalformedSn("players not a/b".into()))?;
    let current_players: i32 = cur
        .trim()
        .parse()
        .map_err(|_| ParseError::MalformedSn("bad current_players".into()))?;
    let max_players: i32 = max
        .trim()
        .parse()
        .map_err(|_| ParseError::MalformedSn("bad max_players".into()))?;
    let required_version: i32 = version_line
        .parse()
        .map_err(|_| ParseError::MalformedSn("bad version".into()))?;

    if challenge.is_empty() {
        return Err(ParseError::MalformedSn("empty challenge".into()));
    }

    Ok(ServerHello {
        current_players,
        max_players,
        challenge,
        required_version,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginOutcome {
    Accepted,
    Rejected,
    /// `NO_LIFE_TOKENS` may precede REJECTED; treat as rejected for playtest.
    NoLifeTokens,
    Shutdown { current: i32, max: i32 },
    ServerFull { current: i32, max: i32 },
}

pub fn parse_login_outcome(body: &str) -> Result<LoginOutcome, ParseError> {
    let body = body.trim();
    let first = body.lines().next().unwrap_or("").trim();
    match first {
        "ACCEPTED" => Ok(LoginOutcome::Accepted),
        "REJECTED" => Ok(LoginOutcome::Rejected),
        "NO_LIFE_TOKENS" => Ok(LoginOutcome::NoLifeTokens),
        "SHUTDOWN" => {
            let (c, m) = parse_players_line(body);
            Ok(LoginOutcome::Shutdown {
                current: c,
                max: m,
            })
        }
        "SERVER_FULL" => {
            let (c, m) = parse_players_line(body);
            Ok(LoginOutcome::ServerFull {
                current: c,
                max: m,
            })
        }
        _ => Err(ParseError::NotLoginOutcome),
    }
}

fn parse_players_line(body: &str) -> (i32, i32) {
    for line in body.lines().skip(1) {
        if let Some((a, b)) = line.trim().split_once('/') {
            if let (Ok(c), Ok(m)) = (a.trim().parse(), b.trim().parse()) {
                return (c, m);
            }
        }
    }
    (0, 0)
}

/// Minimal subset of a PU line for the local player.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerUpdate {
    pub player_id: i32,
    pub done_moving_seq_num: i32,
    pub force: bool,
    pub x: i32,
    pub y: i32,
}

/// Parse first data line of a `PU` message body into a [`PlayerUpdate`] (best-effort).
///
/// Format (protocol.txt):  
/// `p_id po_id facing action action_target_x action_target_y o_id o_origin_valid
///  o_origin_x o_origin_y o_transition_source_id heat done_moving_seqNum force x y ...`
pub fn parse_pu_line(line: &str) -> Option<PlayerUpdate> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    // Need at least through y (index 16): 0..16 inclusive = 17 fields
    if parts.len() < 17 {
        return None;
    }
    let player_id: i32 = parts[0].parse().ok()?;
    let done_moving_seq_num: i32 = parts[12].parse().ok()?;
    let force: i32 = parts[13].parse().ok()?;
    // x,y may be "X" for deleted players
    if parts[14].eq_ignore_ascii_case("X") {
        return None;
    }
    let x: i32 = parts[14].parse().ok()?;
    let y: i32 = parts[15].parse().ok()?;
    Some(PlayerUpdate {
        player_id,
        done_moving_seq_num,
        force: force != 0,
        x,
        y,
    })
}

/// Extract type tag from a server message body (first line).
pub fn message_type(body: &str) -> &str {
    body.lines()
        .next()
        .unwrap_or("")
        .trim()
        .split_whitespace()
        .next()
        .unwrap_or("")
}

/// One line of a `PM` (PLAYER_MOVES_START) message.
///
/// protocol.txt: `p_id xs ys total_sec eta_sec trunc xdelt0 ydelt0 ...`
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerMoveStart {
    pub player_id: i32,
    pub xs: i32,
    pub ys: i32,
    pub total_sec: f32,
    pub eta_sec: f32,
    pub trunc: i32,
    pub deltas: Vec<(i32, i32)>,
}

pub fn parse_pm_line(line: &str) -> Option<PlayerMoveStart> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 6 {
        return None;
    }
    let player_id: i32 = parts[0].parse().ok()?;
    let xs: i32 = parts[1].parse().ok()?;
    let ys: i32 = parts[2].parse().ok()?;
    let total_sec: f32 = parts[3].parse().ok()?;
    let eta_sec: f32 = parts[4].parse().ok()?;
    let trunc: i32 = parts[5].parse().ok()?;
    let mut deltas = Vec::new();
    let mut i = 6;
    while i + 1 < parts.len() {
        let dx: i32 = parts[i].parse().ok()?;
        let dy: i32 = parts[i + 1].parse().ok()?;
        deltas.push((dx, dy));
        i += 2;
    }
    Some(PlayerMoveStart {
        player_id,
        xs,
        ys,
        total_sec,
        eta_sec,
        trunc,
        deltas,
    })
}

/// Parse all data lines of a PM message body.
pub fn parse_pm_message(body: &str) -> Vec<PlayerMoveStart> {
    body.lines()
        .skip(1)
        .filter_map(|l| {
            let t = l.trim();
            if t.is_empty() {
                None
            } else {
                parse_pm_line(t)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sn_ok() {
        let sn = "SN\n3/40\nabcChallenge99\n184\n";
        let h = parse_sn(sn).unwrap();
        assert_eq!(h.current_players, 3);
        assert_eq!(h.max_players, 40);
        assert_eq!(h.challenge, "abcChallenge99");
        assert_eq!(h.required_version, 184);
    }

    #[test]
    fn parse_accepted_rejected() {
        assert_eq!(
            parse_login_outcome("ACCEPTED\n").unwrap(),
            LoginOutcome::Accepted
        );
        assert_eq!(
            parse_login_outcome("REJECTED").unwrap(),
            LoginOutcome::Rejected
        );
    }

    #[test]
    fn parse_pm_line_ok() {
        let pm = parse_pm_line("5 488 488 0.27 0.27 0 1 0").unwrap();
        assert_eq!(pm.player_id, 5);
        assert_eq!((pm.xs, pm.ys), (488, 488));
        assert_eq!(pm.deltas, vec![(1, 0)]);
        assert_eq!(pm.trunc, 0);
    }

    #[test]
    fn parse_pu_done_moving_and_force() {
        // indices: 0 p_id, 1 po_id, 2 facing, 3 action, 4 atx, 5 aty, 6 o_id,
        // 7 o_origin_valid, 8 ox, 9 oy, 10 trans, 11 heat, 12 done, 13 force, 14 x, 15 y, ...
        let line = "42 100 1 0 0 0 0 0 0 0 -1 0.5 2 0 10 20 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1";
        let pu = parse_pu_line(line).unwrap();
        assert_eq!(pu.player_id, 42);
        assert_eq!(pu.done_moving_seq_num, 2);
        assert!(!pu.force);
        assert_eq!((pu.x, pu.y), (10, 20));

        let line_f = "42 100 1 0 0 0 0 0 0 0 -1 0.5 0 1 5 6 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1";
        let pu_f = parse_pu_line(line_f).unwrap();
        assert!(pu_f.force);
        assert_eq!((pu_f.x, pu_f.y), (5, 6));
    }
}
