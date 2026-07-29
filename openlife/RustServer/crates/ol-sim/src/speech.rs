//! Age-dependent speech range and Haxe `doCommands` pure helpers.
//!
//! // Haxe: GlobalPlayerInstance.doCommands / processFollowCommand / processHireCommand
//! // Haxe: NamingHelper.GetName (third whitespace token)
//! // Haxe: Connection.sendSayToAllClose + ServerSettings.MaxDistanceToBeConsideredAsCloseForSay

/// Haxe `ServerSettings.MaxDistanceToBeConsideredAsCloseForSay` (default 20).
///
/// Used for normal-volume `PLAYER_SAYS` / PS fan-out (`sendSayToAllClose`).
/// **Not** the same as crate `NEARBY_RANGE` (24), which is the practical PU/MX interest cull.
/// Stays ModuleConst (not LiveSettings) — same home as other MaxDistance* fans.
// Haxe: ServerSettings.MaxDistanceToBeConsideredAsCloseForSay = 20
pub const ADULT_CHAT_RANGE: i32 = 20;

/// Alias for Haxe field name lookups (`MaxDistanceToBeConsideredAsCloseForSay`).
pub const MAX_DISTANCE_CLOSE_FOR_SAY: i32 = ADULT_CHAT_RANGE;

/// Whisper volume: private / Chebyshev range 1 (targeted WHISPER path).
pub const WHISPER_CHAT_RANGE: i32 = 1;

/// Mumble volume: soft nearby chat (matches [`crate::mumble::MUMBLE_RANGE`]).
pub const MUMBLE_CHAT_RANGE: i32 = 4;

/// Shout Chebyshev range (matches sim SHOUT_RANGE).
pub const SHOUT_CHAT_RANGE: i32 = 48;

/// Haxe `ServerSettings.HireCost` default.
pub const HIRE_COST: i32 = 10;

/// Haxe `ServerSettings.HireCostIncreasePerPerson` default.
pub const HIRE_COST_INCREASE_PER_PERSON: i32 = 10;

/// Haxe home-oven object ids (Clay Oven family used by HOME!).
// Haxe: AiHelper.SearchNewHome oven targets
pub const HOME_OVEN_IDS: &[i32] = &[237, 238, 752, 753];

/// Max squared distance when picking nearest home oven (Chebyshev-ish search cap).
pub const HOME_SEARCH_MAX_QUAD: i32 = 80 * 80;

/// Chat Chebyshev range based on age years (normal volume).
///
/// Haxe `sendSayToAllClose` always uses `MaxDistanceToBeConsideredAsCloseForSay` (20);
/// young-speaker soft scale is a Rust product enhancement only.
///
/// - infants (&lt;3): 8
/// - children (&lt;10): 16
/// - adults / elders: [`ADULT_CHAT_RANGE`] (20 — Haxe CloseForSay)
pub fn chat_range_for_age(age: f32) -> i32 {
    if !age.is_finite() || age < 0.0 {
        return ADULT_CHAT_RANGE;
    }
    if age < 3.0 {
        8
    } else if age < 10.0 {
        16
    } else {
        // Haxe: MaxDistanceToBeConsideredAsCloseForSay = 20
        ADULT_CHAT_RANGE
    }
}

/// Speech volume label for diagnostics / tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeechVolume {
    Whisper,
    Mumble,
    Normal,
    Shout,
}

impl SpeechVolume {
    /// Chebyshev fan-out for this volume (Normal uses adult default; prefer
    /// [`chat_range_for_age`] for live speakers).
    pub fn range(self) -> i32 {
        match self {
            Self::Whisper => WHISPER_CHAT_RANGE,
            Self::Mumble => MUMBLE_CHAT_RANGE,
            Self::Normal => ADULT_CHAT_RANGE,
            Self::Shout => SHOUT_CHAT_RANGE,
        }
    }
}

// ---------------------------------------------------------------------------
// DO-COMMANDS / say_commands — Haxe GlobalPlayerInstance.doCommands pure core
// ---------------------------------------------------------------------------

/// Parsed natural-language speech command (uppercase input expected).
///
/// // Haxe: GlobalPlayerInstance.doCommands
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DoCommand {
    /// `I EXILE NAME` / `I BANN NAME`
    Exile { name: String },
    /// `I REDEEM NAME`
    Redeem { name: String },
    /// `I FOLLOW MY…` — Haxe no-op (still broadcast chat)
    FollowMy,
    /// `I FOLLOW ME` / `I FOLLOW MYSELF` / `I FOLLOW NAME`
    Follow { name: String },
    /// `I HIRE NAME` / `I HIRE NAME …`
    Hire { name: String },
    /// `ORDER, text`
    Order { text: String },
    /// `I GIVE NAME <roman>`
    Give { name: String, coin_token: String },
    /// `NAME OWN THIS` / `NAME OWNES THIS` (or third-token name fallback)
    OwnThis { name: String },
    /// `HOME!` / `!HOME`
    HomeBang,
}

/// Whether Haxe `doCommands` also broadcasts the original speech nearby.
///
/// // Haxe: `if (doCommands(text)) sendSayToAllClose`
pub fn do_command_broadcasts_chat(cmd: &DoCommand) -> bool {
    match cmd {
        // processFollow / processHire / HOME! — suppress original SAY
        DoCommand::Follow { .. } | DoCommand::Hire { .. } | DoCommand::HomeBang => false,
        // Haxe redeem always returns false (uses separate `I REDDEM` say)
        // Haxe: GlobalPlayerInstance.redeem → return false
        DoCommand::Redeem { .. } => false,
        // I FOLLOW MY is a no-op that still chats
        DoCommand::FollowMy => true,
        // exile / order / give / own broadcast original phrase
        DoCommand::Exile { .. }
        | DoCommand::Order { .. }
        | DoCommand::Give { .. }
        | DoCommand::OwnThis { .. } => true,
    }
}

/// Haxe `NamingHelper.GetName` — third whitespace token (index 2), or empty.
// Haxe: NamingHelper.GetName(text)
pub fn extract_command_name(message: &str) -> &str {
    message.split_whitespace().nth(2).unwrap_or("")
}

/// Parse OHOL-style roman coin token (`I`=1, `V`=5, `X`=10, `L`=50, `C`=100, `D`=500, `M`=1000).
///
/// Non-roman chars are ignored (Haxe loop over each char).
// Haxe: GlobalPlayerInstance.doCommands I GIVE coinText
pub fn parse_roman_coin_amount(coin_text: &str) -> i32 {
    let mut amount = 0i32;
    for ch in coin_text.chars() {
        amount = amount.saturating_add(match ch {
            'I' | 'i' => 1,
            'V' | 'v' => 5,
            'X' | 'x' => 10,
            'L' | 'l' => 50,
            'C' | 'c' => 100,
            'D' | 'd' => 500,
            'M' | 'm' => 1000,
            _ => 0,
        });
    }
    amount
}

/// Parse name for OWN THIS forms: prefer token before OWN/OWNES when followed by THIS.
// Haxe: message.contains('OWNES THIS') || contains('OWN THIS') + GetName
pub fn parse_own_this_name(upper: &str) -> Option<&str> {
    let tokens: Vec<&str> = upper.split_whitespace().collect();
    for (i, t) in tokens.iter().enumerate() {
        if (*t == "OWNES" || *t == "OWN") && tokens.get(i + 1) == Some(&"THIS") {
            if i > 0 {
                let name = tokens[i - 1];
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }
    }
    // Fallback: third whitespace token (GetName)
    let n = extract_command_name(upper);
    if n.is_empty() {
        None
    } else {
        Some(n)
    }
}

/// Classify uppercase SAY body as a Haxe `doCommands` natural-language form.
// Haxe: GlobalPlayerInstance.doCommands
pub fn parse_do_command(upper: &str) -> Option<DoCommand> {
    let u = upper.trim();
    if u.is_empty() {
        return None;
    }
    if u.starts_with("I EXILE ") || u.starts_with("I BANN ") {
        let name = extract_command_name(u).to_string();
        if name.is_empty() {
            return None;
        }
        return Some(DoCommand::Exile { name });
    }
    if u.starts_with("I REDEEM ") {
        let name = extract_command_name(u).to_string();
        if name.is_empty() {
            return None;
        }
        return Some(DoCommand::Redeem { name });
    }
    // Dont do anything: I FOLLOW MY FATHER — still broadcast
    if u.starts_with("I FOLLOW MY") {
        return Some(DoCommand::FollowMy);
    }
    if u.starts_with("I FOLLOW ") {
        let name = extract_command_name(u).to_string();
        if name.is_empty() {
            return None;
        }
        return Some(DoCommand::Follow { name });
    }
    if u.starts_with("I HIRE") {
        // "I HIRE NAME" → third token; "I HIRE" alone → empty name (wire fails not_found)
        let name = extract_command_name(u).to_string();
        return Some(DoCommand::Hire { name });
    }
    if u.starts_with("ORDER, ") || u.starts_with("ORDER,") {
        let text = u
            .strip_prefix("ORDER,")
            .unwrap_or(u)
            .trim_start_matches(|c: char| c == ' ' || c == ',')
            .trim()
            .to_string();
        return Some(DoCommand::Order { text });
    }
    if u.starts_with("I GIVE ") {
        let tokens: Vec<&str> = u.split_whitespace().collect();
        // I GIVE NAME ROMAN — need ≥4 tokens
        if tokens.len() < 4 {
            return None;
        }
        let name = tokens[2].to_string();
        let coin_token = tokens[3].to_string();
        return Some(DoCommand::Give { name, coin_token });
    }
    if u.contains("OWNES THIS") || u.contains("OWN THIS") {
        let name = parse_own_this_name(u)?.to_string();
        return Some(DoCommand::OwnThis { name });
    }
    if u.starts_with("HOME!") || u.starts_with("!HOME") || u == "HOME!" || u == "!HOME" {
        return Some(DoCommand::HomeBang);
    }
    None
}

/// Haxe follow-self names for `I FOLLOW ME` / `MYSELF`.
pub fn is_follow_self_name(name: &str) -> bool {
    let n = name.trim();
    n.eq_ignore_ascii_case("ME") || n.eq_ignore_ascii_case("MYSELF")
}

/// Haxe hire age gate: 10 ≤ age ≤ 50.
// Haxe: processHireCommand age < 10 / age > 50
pub fn hire_age_ok(age: f32) -> Result<(), &'static str> {
    if !age.is_finite() {
        return Err("bad_age");
    }
    if age < 10.0 {
        return Err("too_young");
    }
    if age > 50.0 {
        return Err("too_old");
    }
    Ok(())
}

/// Haxe hire angryTime gate: angryTime ≥ 2.
// Haxe: if (player.angryTime < 2)
pub fn hire_angry_ok(angry_time: f32) -> bool {
    angry_time >= 2.0
}

/// Haxe hire class gate: hirer class ≥ target class (Serf=0, Commoner=1, Noble=2).
// Haxe: if (thisClass < playerClass)
pub fn hire_class_ok(hirer_class: i32, target_class: i32) -> bool {
    hirer_class >= target_class
}

/// Haxe processHireCommand coin cost.
///
/// base HireCost, ×3 Noble / ×2 Commoner, ×2 foreign non-friendly color,
/// /2 close relative, + hired_count * increase, + ceil(lostCombat/10), floor HireCost.
// Haxe: processHireCommand neededCoins
pub fn compute_hire_cost(
    base: i32,
    increase_per: i32,
    target_class: i32,
    friendly: bool,
    same_color: bool,
    close_rel: bool,
    hired_count: i32,
    lost_combat_prestige: f32,
) -> i32 {
    let mut needed = base.max(0);
    // PrestigeClass: Noble=2, Commoner=1, Serf=0
    if target_class >= 2 {
        needed = needed.saturating_mul(3);
    } else if target_class == 1 {
        needed = needed.saturating_mul(2);
    }
    if !friendly && !same_color {
        needed = needed.saturating_mul(2);
    }
    if close_rel {
        needed = (needed as f32 / 2.0).ceil() as i32;
    }
    needed = needed.saturating_add(hired_count.max(0).saturating_mul(increase_per.max(0)));
    let combat_impact = (lost_combat_prestige / 10.0).ceil() as i32;
    needed = needed.saturating_add(combat_impact.max(0));
    // Haxe: Math.max(ServerSettings.HireCost, neededCoins + combatPrestigeImppact)
    needed.max(base)
}

/// Closest living player matching first-name (case-insensitive), within max_cheby of speaker.
///
/// Returns p_id of best match, or None.
// Haxe: NamingHelper.GetPlayerByName
pub fn find_player_by_name(
    speaker_p_id: i32,
    speaker_x: i32,
    speaker_y: i32,
    name: &str,
    candidates: &[(i32, &str, i32, i32, bool)],
    max_cheby: i32,
) -> Option<i32> {
    let want = name.trim();
    if want.is_empty() {
        return None;
    }
    let mut best: Option<(i32, i32)> = None; // (quad, p_id)
    for &(p_id, first, x, y, deleted) in candidates {
        if deleted || p_id == speaker_p_id {
            continue;
        }
        if !first.eq_ignore_ascii_case(want) {
            continue;
        }
        let dx = (x - speaker_x).abs();
        let dy = (y - speaker_y).abs();
        if dx > max_cheby || dy > max_cheby {
            continue;
        }
        let quad = dx * dx + dy * dy;
        match best {
            None => best = Some((quad, p_id)),
            Some((bq, _)) if quad < bq => best = Some((quad, p_id)),
            Some((bq, bp)) if quad == bq && p_id < bp => best = Some((quad, p_id)),
            _ => {}
        }
    }
    best.map(|(_, id)| id)
}

/// Pick nearest oven with floor preferred; within max_quad Euclidean squared.
///
/// `ovens`: (x, y, has_floor).
// Haxe: AiHelper.SearchNewHome
pub fn pick_nearest_home_oven(
    sx: i32,
    sy: i32,
    ovens: &[(i32, i32, bool)],
    max_quad: i32,
) -> Option<(i32, i32)> {
    let mut best: Option<(i32, bool, i32, i32)> = None; // (quad, !has_floor, x, y) — floor preferred
    for &(x, y, has_floor) in ovens {
        let dx = x - sx;
        let dy = y - sy;
        let quad = dx * dx + dy * dy;
        if quad > max_quad {
            continue;
        }
        let key_no_floor = !has_floor;
        match best {
            None => best = Some((quad, key_no_floor, x, y)),
            Some((bq, bf, _, _)) if (key_no_floor, quad) < (bf, bq) => {
                best = Some((quad, key_no_floor, x, y));
            }
            _ => {}
        }
    }
    best.map(|(_, _, x, y)| (x, y))
}

/// Whether object id is a recognized home oven.
pub fn is_home_oven_id(id: i32) -> bool {
    HOME_OVEN_IDS.contains(&id)
}

/// Closest owned tile (x,y) to speaker from owning list.
pub fn closest_owned_tile(sx: i32, sy: i32, owning: &[(i32, i32)]) -> Option<(i32, i32)> {
    let mut best: Option<(i32, i32, i32)> = None; // quad, x, y
    for &(x, y) in owning {
        let dx = x - sx;
        let dy = y - sy;
        let quad = dx * dx + dy * dy;
        match best {
            None => best = Some((quad, x, y)),
            Some((bq, _, _)) if quad < bq => best = Some((quad, x, y)),
            _ => {}
        }
    }
    best.map(|(_, x, y)| (x, y))
}

// --- format helpers for private PS diagnostics / live wire ---

pub fn format_exile_say_result(speaker: i32, target_name: &str, ok: bool, reason: &str) -> String {
    if ok {
        format!("{speaker} EXILE OK {target_name}")
    } else {
        format!("{speaker} EXILE FAIL {target_name} {reason}")
    }
}

pub fn format_redeem_say_result(speaker: i32, target_name: &str, ok: bool, reason: &str) -> String {
    if ok {
        format!("{speaker} REDEEM OK {target_name}")
    } else {
        format!("{speaker} REDEEM FAIL {target_name} {reason}")
    }
}

pub fn format_follow_say_result(speaker: i32, target: &str, ok: bool, reason: &str) -> String {
    if ok {
        format!("{speaker} FOLLOW OK {target}")
    } else {
        format!("{speaker} FOLLOW FAIL {target} {reason}")
    }
}

pub fn format_hire_say_result(speaker: i32, target: &str, ok: bool, detail: &str) -> String {
    if ok {
        format!("{speaker} HIRE OK {target} {detail}")
    } else {
        format!("{speaker} HIRE FAIL {target} {detail}")
    }
}

pub fn format_give_say_result(
    speaker: i32,
    target: i32,
    amount: i32,
    ok: bool,
    reason: &str,
) -> String {
    if ok {
        format!("{speaker} GIVE OK {target} {amount}")
    } else {
        format!("{speaker} GIVE FAIL {target} {amount} {reason}")
    }
}

pub fn format_own_this_result(speaker: i32, target: i32, ok: bool, reason: &str) -> String {
    if ok {
        format!("{speaker} OWN OK {target}")
    } else {
        format!("{speaker} OWN FAIL {target} {reason}")
    }
}

pub fn format_home_bang_result(speaker: i32, x: i32, y: i32, ok: bool, reason: &str) -> String {
    if ok {
        format!("{speaker} HOME OK {x} {y}")
    } else {
        format!("{speaker} HOME FAIL {reason}")
    }
}

/// Haxe ORDER global message body: `ORDER:_text`
pub fn format_order_global(text: &str) -> String {
    format!("ORDER:_{text}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn age_brackets() {
        assert_eq!(chat_range_for_age(0.5), 8);
        assert_eq!(chat_range_for_age(2.9), 8);
        assert_eq!(chat_range_for_age(3.0), 16);
        assert_eq!(chat_range_for_age(9.9), 16);
        assert_eq!(chat_range_for_age(14.0), ADULT_CHAT_RANGE);
        assert_eq!(chat_range_for_age(59.9), ADULT_CHAT_RANGE);
        assert_eq!(chat_range_for_age(60.0), ADULT_CHAT_RANGE);
        assert_eq!(chat_range_for_age(100.0), ADULT_CHAT_RANGE);
        // PO-MAX-DISTANCE: Haxe MaxDistanceToBeConsideredAsCloseForSay = 20
        assert_eq!(ADULT_CHAT_RANGE, 20);
        assert_eq!(MAX_DISTANCE_CLOSE_FOR_SAY, 20);
    }

    #[test]
    fn nan_defaults_adult() {
        assert_eq!(chat_range_for_age(f32::NAN), ADULT_CHAT_RANGE);
    }

    #[test]
    fn volume_radii_ordered() {
        assert_eq!(SpeechVolume::Whisper.range(), 1);
        assert_eq!(SpeechVolume::Mumble.range(), 4);
        assert_eq!(SpeechVolume::Shout.range(), 48);
        assert_eq!(SpeechVolume::Normal.range(), ADULT_CHAT_RANGE);
        assert!(SpeechVolume::Whisper.range() < SpeechVolume::Mumble.range());
        assert!(SpeechVolume::Mumble.range() < SpeechVolume::Normal.range());
        assert!(SpeechVolume::Normal.range() < SpeechVolume::Shout.range());
    }

    #[test]
    fn extract_name_third_token() {
        assert_eq!(extract_command_name("I EXILE BOB"), "BOB");
        assert_eq!(extract_command_name("I FOLLOW ME"), "ME");
        assert_eq!(extract_command_name("I GIVE BOB XII"), "BOB");
        assert_eq!(extract_command_name("SHORT"), "");
    }

    #[test]
    fn roman_coins() {
        assert_eq!(parse_roman_coin_amount("I"), 1);
        assert_eq!(parse_roman_coin_amount("XII"), 12);
        assert_eq!(parse_roman_coin_amount("IV"), 6); // Haxe char sum, not subtractive
        assert_eq!(parse_roman_coin_amount("M"), 1000);
        assert_eq!(parse_roman_coin_amount(""), 0);
    }

    #[test]
    fn parse_do_commands_core() {
        assert_eq!(
            parse_do_command("I EXILE BOB"),
            Some(DoCommand::Exile {
                name: "BOB".into()
            })
        );
        assert_eq!(
            parse_do_command("I BANN CAROL"),
            Some(DoCommand::Exile {
                name: "CAROL".into()
            })
        );
        assert_eq!(
            parse_do_command("I REDEEM BOB"),
            Some(DoCommand::Redeem {
                name: "BOB".into()
            })
        );
        assert_eq!(parse_do_command("I FOLLOW MY FATHER"), Some(DoCommand::FollowMy));
        assert_eq!(
            parse_do_command("I FOLLOW ME"),
            Some(DoCommand::Follow { name: "ME".into() })
        );
        assert_eq!(
            parse_do_command("I HIRE WORKER"),
            Some(DoCommand::Hire {
                name: "WORKER".into()
            })
        );
        assert_eq!(
            parse_do_command("ORDER, dig clay"),
            Some(DoCommand::Order {
                text: "dig clay".into()
            })
        );
        assert_eq!(
            parse_do_command("I GIVE BOB XII"),
            Some(DoCommand::Give {
                name: "BOB".into(),
                coin_token: "XII".into()
            })
        );
        assert_eq!(
            parse_do_command("BOB OWN THIS"),
            Some(DoCommand::OwnThis {
                name: "BOB".into()
            })
        );
        assert_eq!(parse_do_command("HOME!"), Some(DoCommand::HomeBang));
        assert_eq!(parse_do_command("!HOME"), Some(DoCommand::HomeBang));
        assert!(parse_do_command("HELLO THERE").is_none());
    }

    #[test]
    fn broadcast_flags() {
        assert!(!do_command_broadcasts_chat(&DoCommand::Follow {
            name: "X".into()
        }));
        assert!(!do_command_broadcasts_chat(&DoCommand::Hire {
            name: "X".into()
        }));
        assert!(!do_command_broadcasts_chat(&DoCommand::HomeBang));
        assert!(!do_command_broadcasts_chat(&DoCommand::Redeem {
            name: "X".into()
        }));
        assert!(do_command_broadcasts_chat(&DoCommand::FollowMy));
        assert!(do_command_broadcasts_chat(&DoCommand::Exile {
            name: "X".into()
        }));
        assert!(do_command_broadcasts_chat(&DoCommand::Order {
            text: "go".into()
        }));
        assert!(do_command_broadcasts_chat(&DoCommand::Give {
            name: "X".into(),
            coin_token: "I".into()
        }));
        assert!(do_command_broadcasts_chat(&DoCommand::OwnThis {
            name: "X".into()
        }));
    }

    #[test]
    fn hire_cost_table() {
        // base 10, serf, friendly same color, no hirees, no combat
        assert_eq!(
            compute_hire_cost(10, 10, 0, true, true, false, 0, 0.0),
            10
        );
        // commoner ×2
        assert_eq!(
            compute_hire_cost(10, 10, 1, true, true, false, 0, 0.0),
            20
        );
        // noble ×3
        assert_eq!(
            compute_hire_cost(10, 10, 2, true, true, false, 0, 0.0),
            30
        );
        // foreign non-friendly ×2 on top of commoner
        assert_eq!(
            compute_hire_cost(10, 10, 1, false, false, false, 0, 0.0),
            40
        );
        // close relative halves
        assert_eq!(
            compute_hire_cost(10, 10, 1, true, true, true, 0, 0.0),
            10
        );
        // hired_count + combat floor
        assert_eq!(
            compute_hire_cost(10, 10, 0, true, true, false, 2, 25.0),
            33
        ); // 10 + 20 + ceil(2.5)=3
    }

    #[test]
    fn hire_age_and_class() {
        assert!(hire_age_ok(10.0).is_ok());
        assert!(hire_age_ok(50.0).is_ok());
        assert!(hire_age_ok(9.9).is_err());
        assert!(hire_age_ok(50.1).is_err());
        assert!(hire_class_ok(1, 1));
        assert!(hire_class_ok(2, 1));
        assert!(!hire_class_ok(0, 1));
        assert!(!hire_angry_ok(1.9));
        assert!(hire_angry_ok(2.0));
    }

    #[test]
    fn find_by_name_closest() {
        let cands = [
            (1, "ALICE", 0, 0, false),
            (2, "BOB", 3, 0, false),
            (3, "BOB", 1, 0, false),
            (4, "BOB", 0, 0, true),
        ];
        // closest non-deleted BOB is id 3 at dist 1
        assert_eq!(
            find_player_by_name(1, 0, 0, "BOB", &cands, 6),
            Some(3)
        );
        assert_eq!(find_player_by_name(1, 0, 0, "ZED", &cands, 6), None);
        // beyond max_cheby
        assert_eq!(find_player_by_name(1, 0, 0, "BOB", &cands, 0), None);
    }

    #[test]
    fn home_oven_pick() {
        let ovens = [(5, 0, false), (2, 0, true), (3, 0, false)];
        // prefer floor then closer
        assert_eq!(
            pick_nearest_home_oven(0, 0, &ovens, HOME_SEARCH_MAX_QUAD),
            Some((2, 0))
        );
        assert!(is_home_oven_id(237));
        assert!(!is_home_oven_id(1));
    }

    #[test]
    fn closest_owning() {
        let owning = [(10, 0), (3, 0), (5, 5)];
        assert_eq!(closest_owned_tile(0, 0, &owning), Some((3, 0)));
        assert_eq!(closest_owned_tile(0, 0, &[]), None);
    }
}
