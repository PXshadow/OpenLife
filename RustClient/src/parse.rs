//! Server→client message parsing.
//!
//! C++: `server/protocol.txt` + LivingLifePage message handlers.
//! Haxe: `ClientTag.hx` + `engine/Engine.hx` `message()` switch.
//!
//! Chunk **L-NET-PARSE / inbound_tags**: full tag surface + structured parsers
//! for live-critical messages (MC/MX/FX/HX/PS/PE/FM/PO/BW/…).

use crate::tags::ServerTag;
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

// ---------------------------------------------------------------------------
// Login / SN (existing)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// PU / PM
// ---------------------------------------------------------------------------

/// One PU (PLAYER_UPDATE) data line — full protocol field set.
///
/// C++: `LivingLifePage.cpp` PU handler / `LiveObject` fill.  
/// Haxe: `PlayerInstance` from split tokens.  
/// protocol.txt field order (indices):
/// ```text
/// 0 p_id  1 po_id  2 facing  3 action  4 action_target_x  5 action_target_y
/// 6 o_id  7 o_origin_valid  8 o_origin_x  9 o_origin_y  10 o_transition_source_id
/// 11 heat  12 done_moving_seqNum  13 force  14 x  15 y  16 age  17 age_r
/// 18 move_speed  19 clothing_set  20 just_ate  21 last_ate_id  22 responsible_id
/// 23 held_yum  24 held_learned
/// ```
/// Deleted players: `x y` = `X X` plus trailing `reason_*`.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerUpdate {
    pub player_id: i32,
    /// Display object id (`po_id`).
    pub display_id: i32,
    /// 0 = no change, 1 = face right, -1 = face left.
    pub facing: i32,
    pub action: i32,
    pub action_target_x: i32,
    pub action_target_y: i32,
    /// Held object id raw (CONTAINER OBJECT FORMAT possible).
    pub held_id_raw: String,
    /// Leading integer of held object; negative = held baby p_id.
    pub held_id: i32,
    pub held_origin_valid: bool,
    pub held_origin_x: i32,
    pub held_origin_y: i32,
    /// -1 if held object is not a transition result.
    pub held_transition_source_id: i32,
    pub heat: f32,
    pub done_moving_seq_num: i32,
    pub force: bool,
    pub x: i32,
    pub y: i32,
    pub age: f32,
    /// Years per second (C++ `ageRate` = `1/invAgeRate`). Not the raw wire field.
    pub age_rate: f32,
    pub move_speed: f32,
    /// `hat;tunic;front_shoe;back_shoe;bottom;backpack` (each slot may be container).
    pub clothing_set: String,
    pub just_ate: bool,
    pub last_ate_id: i32,
    pub responsible_id: i32,
    pub held_yum: bool,
    pub held_learned: bool,
    /// True when line uses `X X` delete form.
    pub deleted: bool,
    /// `reason_disconnected` / `reason_hunger` / `reason_killed_N` / …
    pub delete_reason: Option<String>,
}

/// Parse one data line of a `PU` message into a [`PlayerUpdate`].
pub fn parse_pu_line(line: &str) -> Option<PlayerUpdate> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    // Need at least through force + x y (indices 0..15) = 16 fields for delete form;
    // full live line is 25 fields.
    if parts.len() < 16 {
        return None;
    }
    let player_id: i32 = parts[0].parse().ok()?;
    let display_id: i32 = parse_leading_i32(parts[1]).unwrap_or(0);
    let facing: i32 = parts[2].parse().ok()?;
    let action: i32 = parts[3].parse().ok()?;
    let action_target_x: i32 = parts[4].parse().ok()?;
    let action_target_y: i32 = parts[5].parse().ok()?;
    let held_id_raw = parts[6].to_string();
    // o_id may be container format "391,33,40" — take leading integer; 0 if empty
    let held_id: i32 = parse_leading_i32(parts[6]).unwrap_or(0);
    let held_origin_valid = parts[7].parse::<i32>().ok()? != 0;
    let held_origin_x: i32 = parts[8].parse().ok()?;
    let held_origin_y: i32 = parts[9].parse().ok()?;
    let held_transition_source_id: i32 = parts[10].parse().ok()?;
    let heat: f32 = parts[11].parse().ok()?;
    let done_moving_seq_num: i32 = parts[12].parse().ok()?;
    let force: i32 = parts[13].parse().ok()?;

    // Deleted players: x y = X X + reason at tail
    // C++: LivingLifePage PU delete path (`reason_disconnected`, `reason_hunger`, …)
    if parts[14].eq_ignore_ascii_case("X") {
        // Typical: … force X X reason_hunger
        // After second X (index 15), remaining tokens form the reason string.
        let delete_reason = if parts.len() > 16 {
            Some(parts[16..].join(" "))
        } else {
            None
        };
        return Some(PlayerUpdate {
            player_id,
            display_id,
            facing,
            action,
            action_target_x,
            action_target_y,
            held_id_raw,
            held_id,
            held_origin_valid,
            held_origin_x,
            held_origin_y,
            held_transition_source_id,
            heat,
            done_moving_seq_num,
            force: force != 0,
            x: 0,
            y: 0,
            age: 0.0,
            age_rate: 0.0,
            move_speed: 0.0,
            clothing_set: String::new(),
            just_ate: false,
            last_ate_id: 0,
            responsible_id: -1,
            held_yum: false,
            held_learned: false,
            deleted: true,
            delete_reason,
        });
    }

    let x: i32 = parts[14].parse().ok()?;
    let y: i32 = parts[15].parse().ok()?;
    let age: f32 = parts.get(16).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    // C++ PU field is **invAgeRate** (seconds per year, default 60), then
    // `o.ageRate = 1.0 / invAgeRate` years/sec (LivingLifePage.cpp ~17764–17866).
    // Storing invAgeRate as age_rate made age leap ~60 years/sec → ageRange
    // layers past 999 vanished and only body+head stayed visible.
    let inv_age_rate: f32 = parts
        .get(17)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60.0);
    let age_rate: f32 = if inv_age_rate.abs() > 1e-12 {
        1.0 / inv_age_rate
    } else {
        0.0
    };
    let move_speed: f32 = parts.get(18).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let clothing_set = parts.get(19).unwrap_or(&"0;0;0;0;0;0").to_string();
    let just_ate = parts
        .get(20)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
        != 0;
    let last_ate_id: i32 = parts.get(21).and_then(|s| s.parse().ok()).unwrap_or(0);
    let responsible_id: i32 = parts.get(22).and_then(|s| s.parse().ok()).unwrap_or(-1);
    let held_yum = parts
        .get(23)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
        != 0;
    let held_learned = parts
        .get(24)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
        != 0;

    Some(PlayerUpdate {
        player_id,
        display_id,
        facing,
        action,
        action_target_x,
        action_target_y,
        held_id_raw,
        held_id,
        held_origin_valid,
        held_origin_x,
        held_origin_y,
        held_transition_source_id,
        heat,
        done_moving_seq_num,
        force: force != 0,
        x,
        y,
        age,
        age_rate,
        move_speed,
        clothing_set,
        just_ate,
        last_ate_id,
        responsible_id,
        held_yum,
        held_learned,
        deleted: false,
        delete_reason: None,
    })
}

/// Parse all data lines of a PU message body.
pub fn parse_pu_message(body: &str) -> Vec<PlayerUpdate> {
    data_lines(body).filter_map(parse_pu_line).collect()
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
    data_lines(body).filter_map(parse_pm_line).collect()
}

// ---------------------------------------------------------------------------
// Structured live-critical types (L-NET-PARSE)
// ---------------------------------------------------------------------------

/// MC header only (binary payload already stripped by [`crate::frame::FrameReader`]).
///
/// Wire:
/// ```text
/// MC
/// sizeX sizeY x y
/// binary_raw_size binary_compressed_size
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapChunkHeader {
    pub size_x: i32,
    pub size_y: i32,
    pub x: i32,
    pub y: i32,
    pub binary_raw_size: Option<usize>,
    pub binary_compressed_size: Option<usize>,
}

impl MapChunkHeader {
    pub fn center(&self) -> (i32, i32) {
        (self.x + self.size_x / 2, self.y + self.size_y / 2)
    }
}

/// Parse MC text header (body without `#` / without binary).
pub fn parse_mc_header(body: &str) -> Option<MapChunkHeader> {
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
    let (binary_raw_size, binary_compressed_size) = if let Some(sizes) = lines.next() {
        let mut s = sizes.split_whitespace();
        let raw = s.next().and_then(|t| t.parse().ok());
        let comp = s.next().and_then(|t| t.parse().ok());
        (raw, comp)
    } else {
        (None, None)
    };
    Some(MapChunkHeader {
        size_x,
        size_y,
        x,
        y,
        binary_raw_size,
        binary_compressed_size,
    })
}

/// One MX (MAP_CHANGE) data line.
///
/// `x y new_floor_id new_id p_id [old_x old_y speed]`
/// `new_id` kept as raw string (CONTAINER OBJECT FORMAT possible).
#[derive(Debug, Clone, PartialEq)]
pub struct MapChange {
    pub x: i32,
    pub y: i32,
    pub floor_id: i32,
    /// Object id field (may be container format `391,33:100,40`).
    pub object_id_raw: String,
    /// Leading integer of object id (0 if empty/unparseable).
    pub object_id: i32,
    pub player_id: i32,
    pub old_x: Option<i32>,
    pub old_y: Option<i32>,
    pub speed: Option<f32>,
}

impl MapChange {
    /// True when line has motion fields (animal walk / moving object).
    pub fn is_moving(&self) -> bool {
        self.old_x.is_some() && self.old_y.is_some() && self.speed.is_some()
    }

    /// protocol: `p_id < -1` means transform by player `-p_id` (not a drop).
    pub fn is_transform(&self) -> bool {
        self.player_id < -1
    }
}

pub fn parse_mx_line(line: &str) -> Option<MapChange> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }
    let x: i32 = parts[0].parse().ok()?;
    let y: i32 = parts[1].parse().ok()?;
    let floor_id: i32 = parts[2].parse().ok()?;
    let object_id_raw = parts[3].to_string();
    let object_id = parse_leading_i32(parts[3]).unwrap_or(0);
    let player_id: i32 = parts[4].parse().ok()?;
    let (old_x, old_y, speed) = if parts.len() >= 8 {
        (
            parts[5].parse().ok(),
            parts[6].parse().ok(),
            parts[7].parse().ok(),
        )
    } else {
        (None, None, None)
    };
    Some(MapChange {
        x,
        y,
        floor_id,
        object_id_raw,
        object_id,
        player_id,
        old_x,
        old_y,
        speed,
    })
}

pub fn parse_mx_message(body: &str) -> Vec<MapChange> {
    data_lines(body).filter_map(parse_mx_line).collect()
}

/// FX (FOOD_CHANGE).
///
/// `food_store food_capacity last_ate_id last_ate_fill_max move_speed responsible_id [yum_bonus yum_multiplier]`
#[derive(Debug, Clone, PartialEq)]
pub struct FoodChange {
    pub food_store: i32,
    pub food_capacity: i32,
    pub last_ate_id: i32,
    pub last_ate_fill_max: i32,
    pub move_speed: f32,
    pub responsible_id: i32,
    pub yum_bonus: i32,
    pub yum_multiplier: i32,
}

pub fn parse_fx_message(body: &str) -> Option<FoodChange> {
    let line = first_data_line(body)?;
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 6 {
        return None;
    }
    Some(FoodChange {
        food_store: parts[0].parse().ok()?,
        food_capacity: parts[1].parse().ok()?,
        last_ate_id: parts[2].parse().ok()?,
        last_ate_fill_max: parts[3].parse().ok()?,
        move_speed: parts[4].parse().ok()?,
        responsible_id: parts[5].parse().ok()?,
        yum_bonus: parts.get(6).and_then(|s| s.parse().ok()).unwrap_or(0),
        yum_multiplier: parts.get(7).and_then(|s| s.parse().ok()).unwrap_or(0),
    })
}

/// HX (HEAT_CHANGE): `heat food_time indoor_bonus`
#[derive(Debug, Clone, PartialEq)]
pub struct HeatChange {
    pub heat: f32,
    pub food_time: f32,
    pub indoor_bonus: f32,
}

pub fn parse_hx_message(body: &str) -> Option<HeatChange> {
    let line = first_data_line(body)?;
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    Some(HeatChange {
        heat: parts[0].parse().ok()?,
        food_time: parts[1].parse().ok()?,
        indoor_bonus: parts[2].parse().ok()?,
    })
}

/// Optional map / player pointer convention embedded in PS text.
///
/// protocol.txt (not strict wire grammar): `*map x y [age]`, `*baby id`, etc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaysMapPointer {
    pub x: i32,
    pub y: i32,
    /// Present for pure map spots: `*map x y map_age_seconds`.
    pub map_age_seconds: Option<i32>,
}

/// Known `*target_label` values from protocol.txt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaysTargetLabel {
    Baby,
    Leader,
    Follower,
    Expert,
    Owner,
    Visitor,
    Prop,
    /// Unlisted label kept as raw (without leading `*`).
    Other(String),
}

impl SaysTargetLabel {
    pub fn parse(s: &str) -> Self {
        match s.trim_start_matches('*') {
            "baby" => Self::Baby,
            "leader" => Self::Leader,
            "follower" => Self::Follower,
            "expert" => Self::Expert,
            "owner" => Self::Owner,
            "visitor" => Self::Visitor,
            "prop" => Self::Prop,
            other => Self::Other(other.to_string()),
        }
    }

    /// Soft-FB / overhead short label (C++ `translate("babyLabel")` stand-in).
    pub fn short_name(&self) -> &str {
        match self {
            Self::Baby => "BABY",
            Self::Leader => "LEAD",
            Self::Follower => "FOLLOW",
            Self::Expert => "EXPT",
            Self::Owner => "OWNER",
            Self::Visitor => "VISITOR",
            Self::Prop => "PROP",
            Self::Other(s) => s.as_str(),
        }
    }

    /// Distinct soft-FB marker color (RGBA) per label kind.
    pub fn marker_rgba(&self) -> [u8; 4] {
        match self {
            Self::Baby => [255, 160, 200, 255],
            Self::Leader => [255, 220, 60, 255],
            Self::Follower => [120, 200, 255, 255],
            Self::Expert => [180, 120, 255, 255],
            Self::Owner => [255, 140, 60, 255],
            Self::Visitor => [100, 255, 160, 255],
            Self::Prop => [200, 180, 100, 255],
            Self::Other(_) => [220, 220, 220, 255],
        }
    }
}

/// PS (PLAYER_SAYS): `p_id/isCurse text…` with optional `*map` / `*label` pointers.
///
/// C++: LivingLifePage PLAYER_SAYS handler scans for ` *map` / ` *baby`.  
/// Haxe: slash split for id/curse only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSays {
    pub player_id: i32,
    pub is_curse: bool,
    /// Full text after curse flag (includes pointer tokens when present).
    pub text: String,
    /// Text with `*map` / `*label` pointer suffixes stripped (spoken part).
    pub spoken: String,
    pub map: Option<SaysMapPointer>,
    pub target_label: Option<SaysTargetLabel>,
    pub target_player_id: Option<i32>,
}

pub fn parse_ps_line(line: &str) -> Option<PlayerSays> {
    let line = line.trim();
    let slash = line.find('/')?;
    let player_id: i32 = line[..slash].trim().parse().ok()?;
    let rest = &line[slash + 1..];
    // isCurse is single digit then space then text (Haxe: substr(index+1,1))
    let (curse_ch, text) = if let Some((c, t)) = rest.split_once(' ') {
        (c, t)
    } else {
        (rest, "")
    };
    let is_curse = curse_ch.starts_with('1');
    let (spoken, map, target_label, target_player_id) = parse_ps_pointers(text);
    Some(PlayerSays {
        player_id,
        is_curse,
        text: text.to_string(),
        spoken,
        map,
        target_label,
        target_player_id,
    })
}

/// Extract `*map` / `*label id` conventions from PS text.
fn parse_ps_pointers(
    text: &str,
) -> (
    String,
    Option<SaysMapPointer>,
    Option<SaysTargetLabel>,
    Option<i32>,
) {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.is_empty() {
        return (String::new(), None, None, None);
    }
    let mut map = None;
    let mut target_label = None;
    let mut target_player_id = None;
    let mut spoken_end = tokens.len();

    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i];
        if t == "*map" || t.starts_with("*map") {
            // `*map x y [age]`
            if t == "*map" && i + 2 < tokens.len() {
                if let (Ok(x), Ok(y)) = (tokens[i + 1].parse(), tokens[i + 2].parse()) {
                    let age = tokens.get(i + 3).and_then(|s| s.parse().ok());
                    map = Some(SaysMapPointer {
                        x,
                        y,
                        map_age_seconds: age,
                    });
                    spoken_end = spoken_end.min(i);
                    i += if age.is_some() { 4 } else { 3 };
                    continue;
                }
            }
        }
        if t.starts_with('*') && t != "*map" {
            // `*baby 123` / `*leader 5` …
            let label = SaysTargetLabel::parse(t);
            if i + 1 < tokens.len() {
                if let Ok(id) = tokens[i + 1].parse::<i32>() {
                    target_label = Some(label);
                    target_player_id = Some(id);
                    spoken_end = spoken_end.min(i);
                    i += 2;
                    continue;
                }
            }
            target_label = Some(label);
            spoken_end = spoken_end.min(i);
            i += 1;
            continue;
        }
        i += 1;
    }

    let spoken = if spoken_end < tokens.len() {
        tokens[..spoken_end].join(" ")
    } else {
        text.to_string()
    };
    (spoken, map, target_label, target_player_id)
}

pub fn parse_ps_message(body: &str) -> Vec<PlayerSays> {
    data_lines(body).filter_map(parse_ps_line).collect()
}

/// LS (LOCATION_SAYS): `x y text…`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationSays {
    pub x: i32,
    pub y: i32,
    pub text: String,
}

pub fn parse_ls_line(line: &str) -> Option<LocationSays> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() < 2 {
        return None;
    }
    let x: i32 = tokens[0].parse().ok()?;
    let y: i32 = tokens[1].parse().ok()?;
    let text = if tokens.len() > 2 {
        let after_x = line.trim_start();
        let after_x = after_x
            .strip_prefix(tokens[0])
            .unwrap_or(after_x)
            .trim_start();
        let after_y = after_x
            .strip_prefix(tokens[1])
            .unwrap_or(after_x)
            .trim_start();
        after_y.to_string()
    } else {
        String::new()
    };
    Some(LocationSays { x, y, text })
}

pub fn parse_ls_message(body: &str) -> Vec<LocationSays> {
    data_lines(body).filter_map(parse_ls_line).collect()
}

/// PE (PLAYER_EMOT): `p_id emot_index [ttl_sec]`
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerEmot {
    pub player_id: i32,
    pub emot_index: i32,
    /// None = client default duration; Some(-1) permanent; Some(-2) permanent no sound.
    pub ttl_sec: Option<f32>,
}

pub fn parse_pe_line(line: &str) -> Option<PlayerEmot> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let player_id: i32 = parts[0].parse().ok()?;
    let emot_index: i32 = parts[1].parse().ok()?;
    let ttl_sec = parts.get(2).and_then(|s| s.parse().ok());
    Some(PlayerEmot {
        player_id,
        emot_index,
        ttl_sec,
    })
}

pub fn parse_pe_message(body: &str) -> Vec<PlayerEmot> {
    data_lines(body).filter_map(parse_pe_line).collect()
}

/// NM (NAME): `p_id first_name [last_name…]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerName {
    pub player_id: i32,
    pub first_name: String,
    pub last_name: String,
}

pub fn parse_nm_line(line: &str) -> Option<PlayerName> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let player_id: i32 = parts[0].parse().ok()?;
    let first_name = parts[1].to_string();
    let last_name = if parts.len() > 2 {
        parts[2..].join(" ")
    } else {
        String::new()
    };
    Some(PlayerName {
        player_id,
        first_name,
        last_name,
    })
}

pub fn parse_nm_message(body: &str) -> Vec<PlayerName> {
    data_lines(body).filter_map(parse_nm_line).collect()
}

/// LN (LINEAGE): ancestor ids + eve=
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lineage {
    /// p_id, mother_id, … (does not include eve= token)
    pub chain: Vec<i32>,
    pub eve_id: i32,
}

pub fn parse_ln_line(line: &str) -> Option<Lineage> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let mut chain = Vec::new();
    let mut eve_id = 0;
    for p in &parts {
        if let Some(rest) = p.strip_prefix("eve=") {
            eve_id = rest.parse().ok()?;
        } else if let Ok(id) = p.parse::<i32>() {
            chain.push(id);
        }
    }
    if chain.is_empty() {
        return None;
    }
    // If no eve= tag, last chain id is traditionally Eve
    if eve_id == 0 {
        eve_id = *chain.last().unwrap_or(&0);
    }
    Some(Lineage { chain, eve_id })
}

pub fn parse_ln_message(body: &str) -> Vec<Lineage> {
    data_lines(body).filter_map(parse_ln_line).collect()
}

/// DY (DYING): `p_id [isSick]`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DyingPlayer {
    pub player_id: i32,
    pub is_sick: bool,
}

pub fn parse_dy_line(line: &str) -> Option<DyingPlayer> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let player_id: i32 = parts[0].parse().ok()?;
    let is_sick = parts.get(1).map(|s| *s == "1").unwrap_or(false);
    Some(DyingPlayer { player_id, is_sick })
}

pub fn parse_dy_message(body: &str) -> Vec<DyingPlayer> {
    data_lines(body).filter_map(parse_dy_line).collect()
}

/// CM header sizes (normally inflated by [`crate::frame::FrameReader`]; kept for
/// failed-inflate fallback and unit tests).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressedHeader {
    pub binary_raw_size: usize,
    pub binary_compressed_size: usize,
}

pub fn parse_cm_header(body: &str) -> Option<CompressedHeader> {
    let mut lines = body.lines().map(|l| l.trim()).filter(|l| !l.is_empty());
    if lines.next()? != "CM" {
        return None;
    }
    let sizes = lines.next()?;
    let mut p = sizes.split_whitespace();
    let binary_raw_size: usize = p.next()?.parse().ok()?;
    let binary_compressed_size: usize = p.next()?.parse().ok()?;
    Some(CompressedHeader {
        binary_raw_size,
        binary_compressed_size,
    })
}

/// List of integer player ids (PO / BW / HE / GH lines).
pub fn parse_id_list_message(body: &str) -> Vec<i32> {
    data_lines(body)
        .flat_map(|line| line.split_whitespace())
        .filter_map(|t| t.parse().ok())
        .collect()
}

// ---------------------------------------------------------------------------
// Secondary tags (structured; previously Known → Other)
// ---------------------------------------------------------------------------

/// MS (GLOBAL_MESSAGE): underscores encode spaces; `**` encodes newline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalMessage {
    pub raw: String,
    pub text: String,
}

pub fn parse_ms_message(body: &str) -> Option<GlobalMessage> {
    let raw = first_data_line(body)?.to_string();
    let text = raw.replace("**", "\n").replace('_', " ");
    Some(GlobalMessage { raw, text })
}

/// One CU (CURSED) data line: `p_id level [curse_name]`.
///
/// // C++ LivingLifePage ~21467: `sscanf "%d %d %29s"`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursedPlayer {
    pub player_id: i32,
    pub level: i32,
    /// Optional name; underscores become spaces on apply.
    pub name: Option<String>,
}

/// Parse all CU data lines.
pub fn parse_cu_message(body: &str) -> Vec<CursedPlayer> {
    data_lines(body)
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let player_id: i32 = it.next()?.parse().ok()?;
            let level: i32 = it.next()?.parse().ok()?;
            let name = it.next().map(|s| s.to_string());
            Some(CursedPlayer {
                player_id,
                level,
                name,
            })
        })
        .collect()
}

/// CX (CURSE_TOKEN_CHANGE): `curse_token_count`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurseTokenChange {
    pub curse_token_count: i32,
}

pub fn parse_cx_message(body: &str) -> Option<CurseTokenChange> {
    // May be single-line `CX\nN` or `CX N` first-line form
    let n = first_data_line(body)
        .and_then(|s| s.split_whitespace().next()?.parse().ok())
        .or_else(|| {
            body.lines()
                .next()?
                .split_whitespace()
                .nth(1)?
                .parse()
                .ok()
        })?;
    Some(CurseTokenChange {
        curse_token_count: n,
    })
}

/// CS (CURSE_SCORE_CHANGE): `excess_curse_points`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurseScoreChange {
    pub excess_curse_points: i32,
}

pub fn parse_cs_message(body: &str) -> Option<CurseScoreChange> {
    let n = first_data_line(body)
        .and_then(|s| s.split_whitespace().next()?.parse().ok())
        .or_else(|| {
            body.lines()
                .next()?
                .split_whitespace()
                .nth(1)?
                .parse()
                .ok()
        })?;
    Some(CurseScoreChange {
        excess_curse_points: n,
    })
}

/// VS (VALLEY_SPACING): `y_spacing y_offset`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValleySpacing {
    pub y_spacing: i32,
    pub y_offset: i32,
}

pub fn parse_vs_message(body: &str) -> Option<ValleySpacing> {
    let line = first_data_line(body)?;
    let mut p = line.split_whitespace();
    Some(ValleySpacing {
        y_spacing: p.next()?.parse().ok()?,
        y_offset: p.next()?.parse().ok()?,
    })
}

/// FD (FLIGHT_DEST): `p_id dest_x dest_y`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlightDest {
    pub player_id: i32,
    pub dest_x: i32,
    pub dest_y: i32,
}

pub fn parse_fd_message(body: &str) -> Option<FlightDest> {
    let line = first_data_line(body)?;
    let mut p = line.split_whitespace();
    Some(FlightDest {
        player_id: p.next()?.parse().ok()?,
        dest_x: p.next()?.parse().ok()?,
        dest_y: p.next()?.parse().ok()?,
    })
}

/// FL (FLIP): player ids that just flipped facing.
pub fn parse_fl_message(body: &str) -> Vec<i32> {
    parse_id_list_message(body)
}

/// CR (CRAVING): `food_id bonus` (protocol: craving food id + yum bonus).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Craving {
    pub food_id: i32,
    pub bonus: i32,
}

pub fn parse_cr_message(body: &str) -> Option<Craving> {
    let line = first_data_line(body)?;
    let mut p = line.split_whitespace();
    Some(Craving {
        food_id: p.next()?.parse().ok()?,
        bonus: p.next().and_then(|s| s.parse().ok()).unwrap_or(0),
    })
}

/// PJ (POSSE_JOIN): `killer_id` or multi-field posse lines — keep as id list.
pub fn parse_pj_message(body: &str) -> Vec<i32> {
    parse_id_list_message(body)
}

/// MN (MONUMENT_CALL): `x y o_id`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonumentCall {
    pub x: i32,
    pub y: i32,
    pub object_id: i32,
}

pub fn parse_mn_message(body: &str) -> Option<MonumentCall> {
    let line = first_data_line(body)?;
    let mut p = line.split_whitespace();
    Some(MonumentCall {
        x: p.next()?.parse().ok()?,
        y: p.next()?.parse().ok()?,
        object_id: p.next()?.parse().ok()?,
    })
}

/// GH (GHOST): player ids that became ghosts.
pub fn parse_gh_message(body: &str) -> Vec<i32> {
    parse_id_list_message(body)
}

// ---------------------------------------------------------------------------
// InboundMessage — unified parse table
// ---------------------------------------------------------------------------

/// Fully (or partially) parsed server→client message.
#[derive(Debug, Clone, PartialEq)]
pub enum InboundMessage {
    ServerHello(ServerHello),
    Login(LoginOutcome),
    PlayerUpdates(Vec<PlayerUpdate>),
    PlayerMovesStart(Vec<PlayerMoveStart>),
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
    /// CM header only (inflate failed or header seen without FrameReader inflate).
    Compressed(CompressedHeader),
    Apocalypse,
    ApocalypseDone,
    ForcedShutdown,
    Pong(String),
    GlobalMessage(GlobalMessage),
    Cursed(Vec<CursedPlayer>),
    CurseTokens(CurseTokenChange),
    CurseScore(CurseScoreChange),
    ValleySpacing(ValleySpacing),
    FlightDest(FlightDest),
    Flip(Vec<i32>),
    Craving(Craving),
    PosseJoin(Vec<i32>),
    MonumentCall(MonumentCall),
    Ghost(Vec<i32>),
    /// Recognized tag without a dedicated structured parser yet (payload kept).
    Known {
        tag: ServerTag,
        body: String,
    },
    /// Completely unknown first-line token.
    Unknown {
        tag: String,
        body: String,
    },
}

impl InboundMessage {
    pub fn tag(&self) -> Option<ServerTag> {
        Some(match self {
            Self::ServerHello(_) => ServerTag::Sn,
            Self::Login(o) => match o {
                LoginOutcome::Accepted => ServerTag::Accepted,
                LoginOutcome::Rejected => ServerTag::Rejected,
                LoginOutcome::NoLifeTokens => ServerTag::NoLifeTokens,
                LoginOutcome::Shutdown { .. } => ServerTag::Shutdown,
                LoginOutcome::ServerFull { .. } => ServerTag::ServerFull,
            },
            Self::PlayerUpdates(_) => ServerTag::Pu,
            Self::PlayerMovesStart(_) => ServerTag::Pm,
            Self::MapChunk(_) => ServerTag::Mc,
            Self::MapChanges(_) => ServerTag::Mx,
            Self::FoodChange(_) => ServerTag::Fx,
            Self::HeatChange(_) => ServerTag::Hx,
            Self::PlayerSays(_) => ServerTag::Ps,
            Self::LocationSays(_) => ServerTag::Ls,
            Self::PlayerEmot(_) => ServerTag::Pe,
            Self::PlayerOutOfRange(_) => ServerTag::Po,
            Self::BabyWiggle(_) => ServerTag::Bw,
            Self::Names(_) => ServerTag::Nm,
            Self::Lineages(_) => ServerTag::Ln,
            Self::Dying(_) => ServerTag::Dy,
            Self::Healed(_) => ServerTag::He,
            Self::Frame => ServerTag::Fm,
            Self::Compressed(_) => ServerTag::Cm,
            Self::Apocalypse => ServerTag::Ap,
            Self::ApocalypseDone => ServerTag::Ad,
            Self::ForcedShutdown => ServerTag::Sd,
            Self::Pong(_) => ServerTag::Pong,
            Self::GlobalMessage(_) => ServerTag::Ms,
            Self::Cursed(_) => ServerTag::Cu,
            Self::CurseTokens(_) => ServerTag::Cx,
            Self::CurseScore(_) => ServerTag::Cs,
            Self::ValleySpacing(_) => ServerTag::Vs,
            Self::FlightDest(_) => ServerTag::Fd,
            Self::Flip(_) => ServerTag::Fl,
            Self::Craving(_) => ServerTag::Cr,
            Self::PosseJoin(_) => ServerTag::Pj,
            Self::MonumentCall(_) => ServerTag::Mn,
            Self::Ghost(_) => ServerTag::Gh,
            Self::Known { tag, .. } => *tag,
            Self::Unknown { .. } => return None,
        })
    }
}

/// Parse one framed server message body into a structured [`InboundMessage`].
///
/// Never fails hard: unknown / malformed payloads become [`InboundMessage::Unknown`]
/// or empty structured lists so the session loop can keep draining.
///
/// Note: successful CM frames are inflated by [`crate::frame::FrameReader`] into
/// the inner message body before this is called.
pub fn parse_inbound(body: &str) -> InboundMessage {
    let tag_str = message_type(body);
    let Some(tag) = ServerTag::parse(tag_str) else {
        return InboundMessage::Unknown {
            tag: tag_str.to_string(),
            body: body.to_string(),
        };
    };

    match tag {
        ServerTag::Sn => match parse_sn(body) {
            Ok(h) => InboundMessage::ServerHello(h),
            Err(_) => InboundMessage::Known {
                tag,
                body: body.to_string(),
            },
        },
        ServerTag::Accepted
        | ServerTag::Rejected
        | ServerTag::NoLifeTokens
        | ServerTag::Shutdown
        | ServerTag::ServerFull => match parse_login_outcome(body) {
            Ok(o) => InboundMessage::Login(o),
            Err(_) => InboundMessage::Known {
                tag,
                body: body.to_string(),
            },
        },
        ServerTag::Pu => InboundMessage::PlayerUpdates(parse_pu_message(body)),
        ServerTag::Pm => InboundMessage::PlayerMovesStart(parse_pm_message(body)),
        ServerTag::Mc => match parse_mc_header(body) {
            Some(h) => InboundMessage::MapChunk(h),
            None => InboundMessage::Known {
                tag,
                body: body.to_string(),
            },
        },
        ServerTag::Mx => InboundMessage::MapChanges(parse_mx_message(body)),
        ServerTag::Fx => match parse_fx_message(body) {
            Some(f) => InboundMessage::FoodChange(f),
            None => InboundMessage::Known {
                tag,
                body: body.to_string(),
            },
        },
        ServerTag::Hx => match parse_hx_message(body) {
            Some(h) => InboundMessage::HeatChange(h),
            None => InboundMessage::Known {
                tag,
                body: body.to_string(),
            },
        },
        ServerTag::Ps => InboundMessage::PlayerSays(parse_ps_message(body)),
        ServerTag::Ls => InboundMessage::LocationSays(parse_ls_message(body)),
        ServerTag::Pe => InboundMessage::PlayerEmot(parse_pe_message(body)),
        ServerTag::Po => InboundMessage::PlayerOutOfRange(parse_id_list_message(body)),
        ServerTag::Bw => InboundMessage::BabyWiggle(parse_id_list_message(body)),
        ServerTag::Nm => InboundMessage::Names(parse_nm_message(body)),
        ServerTag::Ln => InboundMessage::Lineages(parse_ln_message(body)),
        ServerTag::Dy => InboundMessage::Dying(parse_dy_message(body)),
        ServerTag::He => InboundMessage::Healed(parse_id_list_message(body)),
        ServerTag::Fm => InboundMessage::Frame,
        ServerTag::Cm => match parse_cm_header(body) {
            Some(h) => InboundMessage::Compressed(h),
            None => InboundMessage::Known {
                tag,
                body: body.to_string(),
            },
        },
        ServerTag::Ap => InboundMessage::Apocalypse,
        ServerTag::Ad => InboundMessage::ApocalypseDone,
        ServerTag::Sd => InboundMessage::ForcedShutdown,
        ServerTag::Pong => {
            let id = first_data_line(body).unwrap_or("").to_string();
            InboundMessage::Pong(id)
        }
        ServerTag::Ms => match parse_ms_message(body) {
            Some(m) => InboundMessage::GlobalMessage(m),
            None => InboundMessage::Known {
                tag,
                body: body.to_string(),
            },
        },
        ServerTag::Cu => InboundMessage::Cursed(parse_cu_message(body)),
        ServerTag::Cx => match parse_cx_message(body) {
            Some(c) => InboundMessage::CurseTokens(c),
            None => InboundMessage::Known {
                tag,
                body: body.to_string(),
            },
        },
        ServerTag::Cs => match parse_cs_message(body) {
            Some(c) => InboundMessage::CurseScore(c),
            None => InboundMessage::Known {
                tag,
                body: body.to_string(),
            },
        },
        ServerTag::Vs => match parse_vs_message(body) {
            Some(v) => InboundMessage::ValleySpacing(v),
            None => InboundMessage::Known {
                tag,
                body: body.to_string(),
            },
        },
        ServerTag::Fd => match parse_fd_message(body) {
            Some(f) => InboundMessage::FlightDest(f),
            None => InboundMessage::Known {
                tag,
                body: body.to_string(),
            },
        },
        ServerTag::Fl => InboundMessage::Flip(parse_fl_message(body)),
        ServerTag::Cr => match parse_cr_message(body) {
            Some(c) => InboundMessage::Craving(c),
            None => InboundMessage::Known {
                tag,
                body: body.to_string(),
            },
        },
        ServerTag::Pj => InboundMessage::PosseJoin(parse_pj_message(body)),
        ServerTag::Mn => match parse_mn_message(body) {
            Some(m) => InboundMessage::MonumentCall(m),
            None => InboundMessage::Known {
                tag,
                body: body.to_string(),
            },
        },
        ServerTag::Gh => InboundMessage::Ghost(parse_gh_message(body)),
        other => InboundMessage::Known {
            tag: other,
            body: body.to_string(),
        },
    }
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn data_lines(body: &str) -> impl Iterator<Item = &str> {
    body.lines()
        .skip(1)
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
}

fn first_data_line(body: &str) -> Option<&str> {
    data_lines(body).next()
}

/// Parse leading signed integer from container-ish fields (`391,33:1` → 391, `-5` → -5).
pub fn parse_leading_i32(s: &str) -> Option<i32> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let end = s
        .char_indices()
        .find(|&(i, c)| {
            if i == 0 && (c == '-' || c == '+') {
                false
            } else {
                !c.is_ascii_digit()
            }
        })
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    if end == 0 || (end == 1 && (s.starts_with('-') || s.starts_with('+'))) {
        return None;
    }
    s[..end].parse().ok()
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
        let line = "42 100 1 0 0 0 0 0 0 0 -1 0.5 2 0 10 20 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1";
        let pu = parse_pu_line(line).unwrap();
        assert_eq!(pu.player_id, 42);
        assert_eq!(pu.display_id, 100);
        assert_eq!(pu.facing, 1);
        assert_eq!(pu.done_moving_seq_num, 2);
        assert!(!pu.force);
        assert_eq!((pu.x, pu.y), (10, 20));
        assert!((pu.age - 12.0).abs() < 0.001);
        assert!((pu.move_speed - 3.75).abs() < 0.001);
        assert_eq!(pu.clothing_set, "0;0;0;0;0;0");
        assert!(pu.held_learned);
        assert!(!pu.deleted);

        let line_f = "42 100 1 0 0 0 0 0 0 0 -1 0.5 0 1 5 6 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1";
        let pu_f = parse_pu_line(line_f).unwrap();
        assert!(pu_f.force);
        assert_eq!((pu_f.x, pu_f.y), (5, 6));
    }

    #[test]
    fn parse_pu_delete_reason() {
        let line = "9 100 0 0 0 0 0 0 0 0 -1 0.5 0 0 X X reason_hunger";
        let pu = parse_pu_line(line).unwrap();
        assert!(pu.deleted);
        assert_eq!(pu.player_id, 9);
        assert_eq!(pu.delete_reason.as_deref(), Some("reason_hunger"));

        let killed = "3 50 0 0 0 0 0 0 0 0 -1 0 0 0 X X reason_killed_560";
        let pu2 = parse_pu_line(killed).unwrap();
        assert_eq!(pu2.delete_reason.as_deref(), Some("reason_killed_560"));
    }

    #[test]
    fn parse_pu_multi_line_message() {
        let body = "PU\n\
1 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 10 20 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1\n\
2 100 1 0 0 0 0 0 0 0 -1 0.5 1 0 11 21 5.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 0\n\
3 100 0 0 0 0 0 0 0 0 -1 0 0 0 X X reason_disconnected\n";
        let list = parse_pu_message(body);
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].player_id, 1);
        assert_eq!(list[1].player_id, 2);
        assert!(list[2].deleted);
    }

    #[test]
    fn parse_mx_static_and_moving() {
        let m = parse_mx_line("3 4 0 33 2").unwrap();
        assert_eq!((m.x, m.y), (3, 4));
        assert_eq!(m.object_id, 33);
        assert_eq!(m.player_id, 2);
        assert!(!m.is_moving());

        let t = parse_mx_line("3 4 0 0 -7").unwrap();
        assert!(t.is_transform());

        let mv = parse_mx_line("5 6 0 418 -1 4 6 1.00").unwrap();
        assert!(mv.is_moving());
        assert_eq!(mv.old_x, Some(4));
        assert_eq!(mv.speed, Some(1.0));
    }

    #[test]
    fn parse_mx_container_id() {
        let m = parse_mx_line("1 2 0 391,33:100,40 -3").unwrap();
        assert_eq!(m.object_id, 391);
        assert_eq!(m.object_id_raw, "391,33:100,40");
        assert!(m.is_transform());
    }

    #[test]
    fn parse_fx_hx() {
        let fx = parse_fx_message("FX\n20 20 0 0 3.75 -1 2 1\n").unwrap();
        assert_eq!(fx.food_store, 20);
        assert_eq!(fx.yum_bonus, 2);
        assert!((fx.move_speed - 3.75).abs() < 0.001);

        let hx = parse_hx_message("HX\n0.50 0.00 0.00\n").unwrap();
        assert!((hx.heat - 0.5).abs() < 0.001);
    }

    #[test]
    fn parse_ps_ls_pe() {
        let ps = parse_ps_line("1432/0 HELLO THERE").unwrap();
        assert_eq!(ps.player_id, 1432);
        assert!(!ps.is_curse);
        assert_eq!(ps.text, "HELLO THERE");
        assert_eq!(ps.spoken, "HELLO THERE");
        assert!(ps.map.is_none());

        let curse = parse_ps_line("1501/1 CURSE JOHN").unwrap();
        assert!(curse.is_curse);

        // protocol: *map x y map_age_seconds
        let spot = parse_ps_line("38499/0 :SPECIAL SPOT *map 13 6 92").unwrap();
        assert_eq!(spot.spoken, ":SPECIAL SPOT");
        let m = spot.map.unwrap();
        assert_eq!((m.x, m.y), (13, 6));
        assert_eq!(m.map_age_seconds, Some(92));

        // *visitor + *map
        let vis = parse_ps_line(
            "38501/0 OUTSIDER NAMELESS PERSON IS MY NEW FOLLOWER *visitor 38500 *map 3 0",
        )
        .unwrap();
        assert_eq!(vis.target_label, Some(SaysTargetLabel::Visitor));
        assert_eq!(vis.target_player_id, Some(38500));
        assert_eq!(vis.map.as_ref().map(|m| (m.x, m.y)), Some((3, 0)));

        let ls = parse_ls_line("0 25 HELLO THERE").unwrap();
        assert_eq!((ls.x, ls.y), (0, 25));
        assert_eq!(ls.text, "HELLO THERE");

        let pe = parse_pe_line("1501 6 30").unwrap();
        assert_eq!(pe.emot_index, 6);
        assert_eq!(pe.ttl_sec, Some(30.0));
        let pe2 = parse_pe_line("1432 4").unwrap();
        assert_eq!(pe2.ttl_sec, None);
        let pe_perm = parse_pe_line("1 2 -1").unwrap();
        assert_eq!(pe_perm.ttl_sec, Some(-1.0));
    }

    #[test]
    fn parse_mc_header_ok() {
        let h = parse_mc_header("MC\n32 30 472 473\n6544 608\n").unwrap();
        assert_eq!((h.size_x, h.size_y), (32, 30));
        assert_eq!((h.x, h.y), (472, 473));
        assert_eq!(h.binary_compressed_size, Some(608));
        assert_eq!(h.center(), (472 + 16, 473 + 15));
    }

    #[test]
    fn parse_po_bw_ids() {
        assert_eq!(parse_id_list_message("PO\n3\n7\n9\n"), vec![3, 7, 9]);
        assert_eq!(parse_id_list_message("BW\n12\n"), vec![12]);
    }

    #[test]
    fn parse_nm_ln_dy() {
        let n = parse_nm_line("5 ALICE SMITH").unwrap();
        assert_eq!(n.first_name, "ALICE");
        assert_eq!(n.last_name, "SMITH");

        let ln = parse_ln_line("10 9 8 1 eve=1").unwrap();
        assert_eq!(ln.chain, vec![10, 9, 8, 1]);
        assert_eq!(ln.eve_id, 1);

        let d = parse_dy_line("42 1").unwrap();
        assert!(d.is_sick);
    }

    #[test]
    fn parse_inbound_table_smoke() {
        assert!(matches!(parse_inbound("FM\n"), InboundMessage::Frame));
        assert!(matches!(
            parse_inbound("MX\n1 2 0 33 -1\n"),
            InboundMessage::MapChanges(ref v) if v.len() == 1
        ));
        assert!(matches!(
            parse_inbound("FX\n10 20 0 0 3.0 -1 0 0\n"),
            InboundMessage::FoodChange(_)
        ));
        assert!(matches!(
            parse_inbound("PS\n1/0 hi\n"),
            InboundMessage::PlayerSays(ref v) if v[0].text == "hi"
        ));
        assert!(matches!(
            parse_inbound("ZZ\nfoo\n"),
            InboundMessage::Unknown { tag, .. } if tag == "ZZ"
        ));
        assert!(matches!(
            parse_inbound("MS\nhello_world\n"),
            InboundMessage::GlobalMessage(ref m) if m.text == "hello world"
        ));
        assert_eq!(parse_inbound("PONG\nabc\n").tag(), Some(ServerTag::Pong));
        assert!(matches!(
            parse_inbound("AP\n"),
            InboundMessage::Apocalypse
        ));
        assert!(matches!(
            parse_inbound("VS\n40 -20\n"),
            InboundMessage::ValleySpacing(ref v) if v.y_spacing == 40
        ));
        assert!(matches!(
            parse_inbound("FD\n5 10 20\n"),
            InboundMessage::FlightDest(ref f) if f.dest_x == 10
        ));
        assert!(matches!(
            parse_inbound("CX\n3\n"),
            InboundMessage::CurseTokens(ref c) if c.curse_token_count == 3
        ));
        assert!(matches!(
            parse_inbound("CU\n42 2 Evil_Bob\n"),
            InboundMessage::Cursed(ref v)
                if v.len() == 1 && v[0].player_id == 42 && v[0].level == 2
                    && v[0].name.as_deref() == Some("Evil_Bob")
        ));
    }

    #[test]
    fn parse_cu_level_only_and_multi() {
        let v = parse_cu_message("CU\n1 0\n2 3 Hex\n");
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].player_id, 1);
        assert_eq!(v[0].level, 0);
        assert!(v[0].name.is_none());
        assert_eq!(v[1].name.as_deref(), Some("Hex"));
    }

    #[test]
    fn held_id_container_prefix() {
        let line =
            "1 100 1 0 0 0 391,33 0 0 0 -1 0.5 1 0 10 20 12.0 60.0 3.75 0;0;0;0;0;0 0 0 -1 0 1";
        let pu = parse_pu_line(line).unwrap();
        assert_eq!(pu.held_id, 391);
        assert_eq!(pu.held_id_raw, "391,33");
    }

    #[test]
    fn parse_fx_six_fields_yum_default() {
        let fx = parse_fx_message("FX\n10 20 0 0 3.0 -1\n").unwrap();
        assert_eq!(fx.yum_bonus, 0);
        assert_eq!(fx.yum_multiplier, 0);
    }

    #[test]
    fn parse_secondary_ms_cr_mn() {
        // `_` → space, `**` → newline (order: ** first so underscores beside ** stay spaces)
        let ms = parse_ms_message("MS\nSERVER**RESTART_SOON\n").unwrap();
        assert_eq!(ms.text, "SERVER\nRESTART SOON");
        let cr = parse_cr_message("CR\n31 2\n").unwrap();
        assert_eq!((cr.food_id, cr.bonus), (31, 2));
        let mn = parse_mn_message("MN\n4 5 999\n").unwrap();
        assert_eq!(mn.object_id, 999);
    }
}
