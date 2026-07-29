//! MAP-LOCATION-PINS / social_pins — Haxe `Connection.sendMapLocation` social labels.
//!
//! Pins are **PLAYER_SAYS** map markers:
//! `{viewer_p_id}/0 {TEXT1} *{text2} {target_p_id} *map {rel_x} {rel_y}`
//!
//! Labels (Haxe):
//! - Login mother → `MOTHER *leader`
//! - Birth to human parent → `BABY *baby`
//! - Follow request / CountAndDisplayFollower → `FOLLOWER *follower`
//! - `!H` / `!HUMAN` → `HUMAN *follower`
//! - CountAndDisplayAlly → `ALLY *follower`
//! - CountAndDisplayFamily → `FAM *follower`
//! - Age-10 father re-follow → child `LEADER *leader` + father `FOLLOWER *follower`
//!
//! LEADER personal pins also live in `leader_range` / LEADER-RANGE.

use crate::ai_takeover::player_is_human;
use crate::leadership::format_map_location_says_body;
use crate::relations::{get_top_leader, is_ally, is_same_family};
use crate::social::{format_following_for_player, SocialState};
use crate::SimState;
use ol_net::OutboundHub;
use ol_protocol::{format_player_says, format_server_message};
use std::collections::HashSet;

// Haxe: Connection.sendMapLocation text1/text2 pairs
pub const MOTHER_TEXT1: &str = "MOTHER";
pub const MOTHER_TEXT2: &str = "leader";
pub const BABY_TEXT1: &str = "BABY";
pub const BABY_TEXT2: &str = "baby";
pub const FOLLOWER_TEXT1: &str = "FOLLOWER";
pub const FOLLOWER_TEXT2: &str = "follower";
pub const HUMAN_TEXT1: &str = "HUMAN";
pub const HUMAN_TEXT2: &str = "follower";
pub const ALLY_TEXT1: &str = "ALLY";
pub const ALLY_TEXT2: &str = "follower";
pub const FAM_TEXT1: &str = "FAM";
pub const FAM_TEXT2: &str = "follower";
pub const LEADER_TEXT1: &str = "LEADER";
pub const LEADER_TEXT2: &str = "leader";

/// Haxe `ServerSettings.MinAgeToEat` years — birth BABY pin gate.
pub const MIN_AGE_TO_EAT_YEARS: f32 = 3.0;

/// Haxe male father re-follow chance at trueAge 10 (`rand > 0.4` → 60% fire).
pub const FATHER_REFOLLOW_CHANCE_MALE: f32 = 0.4;
/// Haxe female father re-follow chance at trueAge 10 (`rand > 0.8` → 20% fire).
pub const FATHER_REFOLLOW_CHANCE_FEMALE: f32 = 0.8;

/// One living candidate for closest-pick / name balloon.
#[derive(Debug, Clone)]
pub struct SocialPinCandidate {
    pub p_id: i32,
    pub x: i32,
    pub y: i32,
    /// Haxe `p.name` (first / given name).
    pub name: String,
}

/// Result of Haxe `CountAndDisplayFollower` / Ally / Family pure pick.
#[derive(Debug, Clone, Default)]
pub struct CountAndDisplayResult {
    pub count: i32,
    /// Closest member (squared world distance); None if empty.
    pub best: Option<SocialPinCandidate>,
    /// Name balloons when `display` — Haxe `${p.id}/0 +${p.name}+`
    pub balloons: Vec<(i32, String)>,
}

/// Squared euclidean distance (Haxe `AiHelper.CalculateDistanceToPlayer`).
#[inline]
pub fn dist_sq(ax: i32, ay: i32, bx: i32, by: i32) -> i64 {
    let dx = (ax - bx) as i64;
    let dy = (ay - by) as i64;
    dx * dx + dy * dy
}

/// Haxe name balloon body (`+Name+`).
// Haxe: CountAndDisplay* PLAYER_SAYS `+${p.name}+`
pub fn format_name_balloon_body(name: &str) -> String {
    format!("+{name}+")
}

/// Prefer Haxe `p.name` style: first token of lineage name, else first_name.
// Haxe: GlobalPlayerInstance.name used in CountAndDisplay balloons
pub fn social_pin_name(first_name: &str, lineage_name: Option<&str>) -> String {
    if let Some(ln) = lineage_name {
        let tok = ln.split_whitespace().next().unwrap_or("").trim();
        if !tok.is_empty() {
            return tok.to_string();
        }
    }
    first_name.to_string()
}

/// Haxe `?ALLY` → `ALLIES N!` / `ALLY?` → `ALLIES N`.
pub fn format_allies_count_say(count: i32, excited: bool) -> String {
    if excited {
        format!("ALLIES {count}!")
    } else {
        format!("ALLIES {count}")
    }
}

/// Haxe `?FOLLOWER` / `FOLLOWER?` reply body.
pub fn format_follower_count_say(count: i32) -> String {
    format!("FOLLOWER {count}")
}

/// Haxe `?FAM` / `FAM?` reply body (`FAMILY N`, not `FAM N`).
pub fn format_family_count_say(count: i32) -> String {
    format!("FAMILY {count}")
}

pub fn format_only_me_human_say() -> &'static str {
    "There is only me in this world!"
}

pub fn format_attacked_humans_say() -> &'static str {
    "I attacked humans!"
}

/// Pure closest-pick over candidates (Haxe CountAndDisplay* loop).
// Haxe: GlobalPlayerInstance.CountAndDisplayFollower/Ally/Family
pub fn count_and_display_closest(
    viewer_x: i32,
    viewer_y: i32,
    candidates: &[SocialPinCandidate],
    display: bool,
) -> CountAndDisplayResult {
    let mut out = CountAndDisplayResult::default();
    let mut best_dist = -1i64;
    for c in candidates {
        out.count += 1;
        if display {
            out.balloons
                .push((c.p_id, format_name_balloon_body(&c.name)));
        }
        let d = dist_sq(viewer_x, viewer_y, c.x, c.y);
        if out.best.is_some() && d > best_dist {
            continue;
        }
        best_dist = d;
        out.best = Some(c.clone());
    }
    out
}

/// Full map-location PS body after `p_id/0 ` (Haxe sendMapLocation message tail).
pub fn format_social_map_pin_body(
    text1: &str,
    text2: &str,
    target_p_id: i32,
    rel_x: i32,
    rel_y: i32,
) -> String {
    format_map_location_says_body(text1, text2, target_p_id, rel_x, rel_y)
}

/// Send one Haxe `sendMapLocation` PS (+ optional FRAME).
// Haxe: Connection.sendMapLocation
pub fn send_map_location_pin(
    outbound: &OutboundHub,
    conn_id: u64,
    viewer_p_id: i32,
    text1: &str,
    text2: &str,
    target_p_id: i32,
    rel_x: i32,
    rel_y: i32,
    with_frame: bool,
) {
    let body = format_social_map_pin_body(text1, text2, target_p_id, rel_x, rel_y);
    let ps = format_player_says(viewer_p_id, false, &body);
    outbound.send(conn_id, ps.into_bytes());
    if with_frame {
        outbound.send(conn_id, format_server_message("FM", &[]).into_bytes());
    }
}

/// Name balloon as spoken by target (`target_p_id/0 +Name+`).
fn send_name_balloon(outbound: &OutboundHub, conn_id: u64, target_p_id: i32, name_body: &str) {
    let ps = format_player_says(target_p_id, false, name_body);
    outbound.send(conn_id, ps.into_bytes());
}

/// Emit balloons + closest pin + FRAME (Haxe CountAndDisplay* display path).
pub fn send_count_display_pins(
    outbound: &OutboundHub,
    conn_id: u64,
    viewer_p_id: i32,
    viewer_birth_x: i32,
    viewer_birth_y: i32,
    result: &CountAndDisplayResult,
    text1: &str,
    text2: &str,
) {
    for (tid, body) in &result.balloons {
        send_name_balloon(outbound, conn_id, *tid, body);
    }
    if let Some(best) = &result.best {
        // Haxe transformX/Y: world − birth origin (gx/gy)
        let rel_x = best.x - viewer_birth_x;
        let rel_y = best.y - viewer_birth_y;
        send_map_location_pin(
            outbound,
            conn_id,
            viewer_p_id,
            text1,
            text2,
            best.p_id,
            rel_x,
            rel_y,
            false,
        );
    }
    outbound.send(conn_id, format_server_message("FM", &[]).into_bytes());
}

fn deleted_p_ids(state: &SimState) -> HashSet<i32> {
    state
        .players
        .values()
        .filter(|p| p.deleted)
        .map(|p| p.p_id)
        .collect()
}

fn pin_name_for(state: &SimState, p: &crate::player::Player) -> String {
    let lineage = state.social.lineages.get(&p.p_id).map(|n| n.name.as_str());
    social_pin_name(&p.first_name, lineage)
}

fn collect_follower_candidates(
    state: &SimState,
    leader_p_id: i32,
    only_own_family: bool,
) -> Vec<SocialPinCandidate> {
    let mut v = Vec::new();
    for p in state.players.values() {
        if p.deleted || p.p_id == leader_p_id {
            continue;
        }
        if !state.social.is_follower_from(p.p_id, leader_p_id) {
            continue;
        }
        if only_own_family && !is_same_family(&state.social, p.p_id, leader_p_id) {
            continue;
        }
        v.push(SocialPinCandidate {
            p_id: p.p_id,
            x: p.x,
            y: p.y,
            name: pin_name_for(state, p),
        });
    }
    v
}

fn collect_ally_candidates(state: &SimState, viewer_p_id: i32) -> Vec<SocialPinCandidate> {
    let deleted = deleted_p_ids(state);
    let mut v = Vec::new();
    for p in state.players.values() {
        if p.deleted || p.p_id == viewer_p_id {
            continue;
        }
        // Haxe isAlly: same top leader with exile/deleted-aware getTopLeader
        // Haxe: GlobalPlayerInstance.isAlly / CountAndDisplayAlly
        if !is_ally(
            &state.social.following,
            &state.social,
            &deleted,
            viewer_p_id,
            p.p_id,
        ) {
            continue;
        }
        v.push(SocialPinCandidate {
            p_id: p.p_id,
            x: p.x,
            y: p.y,
            name: pin_name_for(state, p),
        });
    }
    v
}

fn collect_family_candidates(state: &SimState, viewer_p_id: i32) -> Vec<SocialPinCandidate> {
    let mut v = Vec::new();
    for p in state.players.values() {
        if p.deleted || p.p_id == viewer_p_id {
            continue;
        }
        if !is_same_family(&state.social, p.p_id, viewer_p_id) {
            continue;
        }
        v.push(SocialPinCandidate {
            p_id: p.p_id,
            x: p.x,
            y: p.y,
            name: pin_name_for(state, p),
        });
    }
    v
}

/// Speak count privately (Haxe `toSelf = true` forms: `?ALLY`, `?F`, `?FAM`).
fn speak_count_private(
    outbound: &OutboundHub,
    conn_id: u64,
    viewer_p_id: i32,
    say: &str,
) {
    let ps = format_player_says(viewer_p_id, false, say);
    outbound.send(conn_id, ps.into_bytes());
    outbound.send(conn_id, format_server_message("FM", &[]).into_bytes());
}

/// Haxe `CountAndDisplayFollower` + count say.
///
/// `to_self`: when true, count is private PS; when false, count text is returned
/// for the caller to broadcast publicly (Haxe `FOLLOWER?` leaves `toSelf` false).
// Haxe: GlobalPlayerInstance.CountAndDisplayFollower + ?FOLLOWER / ?F / FOLLOWER?
pub fn apply_count_and_display_follower(
    state: &SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    only_own_family: bool,
    to_self: bool,
) -> (i32, Option<String>) {
    let Some(viewer) = state.players.get(&conn_id) else {
        return (0, None);
    };
    let cands = collect_follower_candidates(state, viewer.p_id, only_own_family);
    let result = count_and_display_closest(viewer.x, viewer.y, &cands, true);
    send_count_display_pins(
        outbound,
        conn_id,
        viewer.p_id,
        viewer.birth_x,
        viewer.birth_y,
        &result,
        FOLLOWER_TEXT1,
        FOLLOWER_TEXT2,
    );
    let say = format_follower_count_say(result.count);
    if to_self {
        speak_count_private(outbound, conn_id, viewer.p_id, &say);
        (result.count, None)
    } else {
        (result.count, Some(say))
    }
}

/// Haxe `CountAndDisplayAlly` + count say (`ALLIES N!` when excited).
// Haxe: GlobalPlayerInstance.CountAndDisplayAlly + ?ALLY / ALLY?
pub fn apply_count_and_display_ally(
    state: &SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    excited: bool,
    to_self: bool,
) -> (i32, Option<String>) {
    let Some(viewer) = state.players.get(&conn_id) else {
        return (0, None);
    };
    let cands = collect_ally_candidates(state, viewer.p_id);
    let result = count_and_display_closest(viewer.x, viewer.y, &cands, true);
    send_count_display_pins(
        outbound,
        conn_id,
        viewer.p_id,
        viewer.birth_x,
        viewer.birth_y,
        &result,
        ALLY_TEXT1,
        ALLY_TEXT2,
    );
    let say = format_allies_count_say(result.count, excited);
    if to_self {
        speak_count_private(outbound, conn_id, viewer.p_id, &say);
        (result.count, None)
    } else {
        (result.count, Some(say))
    }
}

/// Haxe `CountAndDisplayFamily` + count say.
// Haxe: GlobalPlayerInstance.CountAndDisplayFamily + ?FAM / FAM?
pub fn apply_count_and_display_family(
    state: &SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    to_self: bool,
) -> (i32, Option<String>) {
    let Some(viewer) = state.players.get(&conn_id) else {
        return (0, None);
    };
    let cands = collect_family_candidates(state, viewer.p_id);
    let result = count_and_display_closest(viewer.x, viewer.y, &cands, true);
    send_count_display_pins(
        outbound,
        conn_id,
        viewer.p_id,
        viewer.birth_x,
        viewer.birth_y,
        &result,
        FAM_TEXT1,
        FAM_TEXT2,
    );
    let say = format_family_count_say(result.count);
    if to_self {
        speak_count_private(outbound, conn_id, viewer.p_id, &say);
        (result.count, None)
    } else {
        (result.count, Some(say))
    }
}

/// Haxe `!H` / `!HUMAN` — first living human (any distance) → HUMAN map pin.
///
/// Haxe `getClosePlayer(-1, …, onlyHuman=true)` returns the **first** match in
/// `AllPlayers` order (not closest). Rust uses **lowest `p_id`** for stability.
// Haxe: GlobalPlayerInstance doCommands !HUMAN L5710+
pub fn apply_human_map_pin(state: &SimState, outbound: &OutboundHub, conn_id: u64) {
    let Some(viewer) = state.players.get(&conn_id) else {
        return;
    };
    if !viewer.allow_show_human {
        let ps = format_player_says(viewer.p_id, false, format_attacked_humans_say());
        outbound.send(conn_id, ps.into_bytes());
        outbound.send(conn_id, format_server_message("FM", &[]).into_bytes());
        return;
    }
    let viewer_p_id = viewer.p_id;
    let viewer_birth = (viewer.birth_x, viewer.birth_y);
    let mut best_id: Option<i32> = None;
    let mut best_xy: (i32, i32) = (0, 0);
    for p in state.players.values() {
        if p.deleted || p.p_id == viewer_p_id {
            continue;
        }
        if !player_is_human(p.connected, p.ai_controlled, &p.email) {
            continue;
        }
        match best_id {
            Some(id) if id <= p.p_id => {}
            _ => {
                best_id = Some(p.p_id);
                best_xy = (p.x, p.y);
            }
        }
    }
    let Some(human_id) = best_id else {
        let ps = format_player_says(viewer_p_id, false, format_only_me_human_say());
        outbound.send(conn_id, ps.into_bytes());
        outbound.send(conn_id, format_server_message("FM", &[]).into_bytes());
        return;
    };
    let rel_x = best_xy.0 - viewer_birth.0;
    let rel_y = best_xy.1 - viewer_birth.1;
    send_map_location_pin(
        outbound,
        conn_id,
        viewer_p_id,
        HUMAN_TEXT1,
        HUMAN_TEXT2,
        human_id,
        rel_x,
        rel_y,
        true,
    );
}

/// Login: pin mother if lineage has mother_id (Haxe Connection after sendMapChunk).
// Haxe: Connection L281 sendMapLocation(mother, 'MOTHER', 'leader')
pub fn send_mother_map_pin_on_login(state: &SimState, outbound: &OutboundHub, conn_id: u64) {
    let Some(child) = state.players.get(&conn_id) else {
        return;
    };
    let Some(mother_id) = state
        .social
        .lineages
        .get(&child.p_id)
        .and_then(|n| n.mother_id)
    else {
        return;
    };
    let Some(mother) = state
        .players
        .values()
        .find(|p| p.p_id == mother_id && !p.deleted)
    else {
        return;
    };
    let (rel_x, rel_y) = child.world_to_client(mother.x, mother.y);
    send_map_location_pin(
        outbound,
        conn_id,
        child.p_id,
        MOTHER_TEXT1,
        MOTHER_TEXT2,
        mother.p_id,
        rel_x,
        rel_y,
        true,
    );
}

/// Birth: human parent gets BABY pin when child age &lt; MinAgeToEat.
// Haxe: GlobalPlayerInstance init L1013–1027
pub fn send_baby_map_pin_to_parent(
    state: &SimState,
    outbound: &OutboundHub,
    parent_conn: u64,
    baby_p_id: i32,
) {
    let Some(parent) = state.players.get(&parent_conn) else {
        return;
    };
    if !player_is_human(parent.connected, parent.ai_controlled, &parent.email) {
        return;
    }
    let Some(baby) = state
        .players
        .values()
        .find(|p| p.p_id == baby_p_id && !p.deleted)
    else {
        return;
    };
    // C-SS-MIN-AGE-AI: live MinAgeToEat (Haxe ServerSettings.MinAgeToEat)
    let min_age = if state.gameplay.min_age_to_eat.is_finite() && state.gameplay.min_age_to_eat >= 0.0 {
        state.gameplay.min_age_to_eat
    } else {
        MIN_AGE_TO_EAT_YEARS
    };
    if baby.age >= min_age {
        return;
    }
    let (rel_x, rel_y) = parent.world_to_client(baby.x, baby.y);
    send_map_location_pin(
        outbound,
        parent_conn,
        parent.p_id,
        BABY_TEXT1,
        BABY_TEXT2,
        baby.p_id,
        rel_x,
        rel_y,
        true,
    );
}

/// After birth: BABY pin to mother (conn) and living human father (if any).
// Haxe: GlobalPlayerInstance init mother + father sendMapLocation BABY
pub fn send_baby_map_pins_on_birth(
    state: &SimState,
    outbound: &OutboundHub,
    mother_conn: u64,
    baby_p_id: i32,
) {
    send_baby_map_pin_to_parent(state, outbound, mother_conn, baby_p_id);
    let father_id = state
        .social
        .lineages
        .get(&baby_p_id)
        .and_then(|n| n.father_id);
    let Some(fid) = father_id else {
        return;
    };
    let Some(father_conn) = state
        .players
        .values()
        .find(|p| p.p_id == fid && !p.deleted)
        .map(|p| p.conn_id)
    else {
        return;
    };
    if father_conn == mother_conn {
        return;
    }
    send_baby_map_pin_to_parent(state, outbound, father_conn, baby_p_id);
}

/// Clear `allow_show_human` when attacker hits a human (Haxe DoDamage path).
// Haxe: GlobalPlayerInstance L4488 allowShowHuman = false
pub fn note_attacked_human(state: &mut SimState, attacker_p_id: i32, target_p_id: i32) {
    let target_human = state
        .players
        .values()
        .find(|p| p.p_id == target_p_id)
        .map(|p| player_is_human(p.connected, p.ai_controlled, &p.email))
        .unwrap_or(false);
    if !target_human {
        return;
    }
    if let Some(a) = state.players.values_mut().find(|p| p.p_id == attacker_p_id) {
        a.allow_show_human = false;
    }
}

/// Haxe `?F` → `sendToMeAllFollowings(true)`: FW lines for every living player to conn.
// Haxe: Connection.sendToMeAllFollowings(sendInfo=true)
pub fn send_to_me_all_followings(state: &SimState, outbound: &OutboundHub, conn_id: u64) {
    let deleted: HashSet<i32> = deleted_p_ids(state);
    let mut lines: Vec<String> = Vec::new();
    for p in state.players.values() {
        if p.deleted {
            continue;
        }
        let top = get_top_leader(
            &state.social.following,
            &state.social,
            &deleted,
            p.p_id,
            None,
        )
        .unwrap_or(p.p_id);
        lines.push(format_following_for_player(&state.social, p.p_id, top));
    }
    if lines.is_empty() {
        return;
    }
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    outbound.send(
        conn_id,
        format_server_message("FW", &refs).into_bytes(),
    );
}

/// Parse social pin SAY commands handled here (not LEADER-RANGE).
///
/// Returns `Some(kind)` when the utterance is a social map-pin query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocialPinSay {
    /// `?ALLY` — balloons + pin + private `ALLIES N!`
    AllyExcited,
    /// `ALLY?` — balloons + pin + **public** `ALLIES N`
    AllyPlain,
    /// `?F` / `?FOLLOWER` — FW refresh (?F only) + balloons + pin + private count
    FollowerPrivate,
    /// `FOLLOWER?` — balloons + pin + **public** count
    FollowerPublic,
    /// `?FAM` — private FAMILY count
    FamilyPrivate,
    /// `FAM?` — public FAMILY count
    FamilyPublic,
    Human,
}

pub fn parse_social_pin_say(upper: &str) -> Option<SocialPinSay> {
    if upper == "!H" || upper == "!HUMAN" || upper.starts_with("!HUMAN ") {
        return Some(SocialPinSay::Human);
    }
    if upper.starts_with("?ALLY") {
        return Some(SocialPinSay::AllyExcited);
    }
    if upper.starts_with("ALLY?") {
        return Some(SocialPinSay::AllyPlain);
    }
    if upper == "?F" || upper.starts_with("?FOLLOWER") {
        return Some(SocialPinSay::FollowerPrivate);
    }
    if upper.starts_with("FOLLOWER?") {
        return Some(SocialPinSay::FollowerPublic);
    }
    if upper.starts_with("?FAM") && !upper.starts_with("?FAMILY") {
        return Some(SocialPinSay::FamilyPrivate);
    }
    if upper.starts_with("FAM?") {
        return Some(SocialPinSay::FamilyPublic);
    }
    None
}

/// Apply parsed social pin say (count+pin or HUMAN).
///
/// Returns `Some(public_count_text)` when Haxe leaves `toSelf=false` so the
/// caller should fan the count as normal chat (nearby PS).
// Haxe: GlobalPlayerInstance doCommands ?ALLY / ALLY? / ?F / FOLLOWER? / ?FAM / FAM? / !H
pub fn apply_social_pin_say(
    state: &SimState,
    outbound: &OutboundHub,
    conn_id: u64,
    kind: SocialPinSay,
    raw_upper: &str,
) -> Option<String> {
    match kind {
        SocialPinSay::AllyExcited => {
            let (_, pub_say) =
                apply_count_and_display_ally(state, outbound, conn_id, true, true);
            pub_say
        }
        SocialPinSay::AllyPlain => {
            let (_, pub_say) =
                apply_count_and_display_ally(state, outbound, conn_id, false, false);
            pub_say
        }
        SocialPinSay::FollowerPrivate => {
            // Haxe: if (text == '?F') player.connection.sendToMeAllFollowings(true);
            if raw_upper == "?F" {
                send_to_me_all_followings(state, outbound, conn_id);
            }
            let (_, pub_say) =
                apply_count_and_display_follower(state, outbound, conn_id, false, true);
            pub_say
        }
        SocialPinSay::FollowerPublic => {
            let (_, pub_say) =
                apply_count_and_display_follower(state, outbound, conn_id, false, false);
            pub_say
        }
        SocialPinSay::FamilyPrivate => {
            let (_, pub_say) =
                apply_count_and_display_family(state, outbound, conn_id, true);
            pub_say
        }
        SocialPinSay::FamilyPublic => {
            let (_, pub_say) =
                apply_count_and_display_family(state, outbound, conn_id, false);
            pub_say
        }
        SocialPinSay::Human => {
            apply_human_map_pin(state, outbound, conn_id);
            None
        }
    }
}

/// Pure: did `true_age` just cross into integer year 10?
// Haxe: TimeHelper Std.int(player.trueAge) == 10 (once per life when crossing)
pub fn crossed_true_age_year(prev_true_age: f32, new_true_age: f32, year: i32) -> bool {
    let y = year as f32;
    prev_true_age < y && new_true_age >= y
}

/// Pure chance gate for age-10 father re-follow.
///
/// Haxe: `rand > chance` with chance 0.4 male / 0.8 female → fire when rand exceeds threshold.
// Haxe: TimeHelper L780–805 father re-follow
pub fn father_refollow_chance_fires(is_male: bool, rand01: f32) -> bool {
    let chance = if is_male {
        FATHER_REFOLLOW_CHANCE_MALE
    } else {
        FATHER_REFOLLOW_CHANCE_FEMALE
    };
    rand01 > chance
}

/// Preconditions for age-10 father re-follow (before RNG).
///
/// Haxe: `followPlayer == mother && father != null && !father.isDeleted()`.
pub fn father_refollow_eligible(
    following: &std::collections::HashMap<i32, i32>,
    child_p_id: i32,
    mother_id: Option<i32>,
    father_id: Option<i32>,
    father_deleted: bool,
) -> bool {
    let Some(mid) = mother_id else {
        return false;
    };
    let Some(fid) = father_id else {
        return false;
    };
    if father_deleted || fid == 0 || mid == 0 {
        return false;
    }
    following.get(&child_p_id).copied() == Some(mid)
}

/// Live: on trueAge cross 10, maybe re-follow father and emit LEADER/FOLLOWER pins.
///
/// Returns true when re-follow applied.
// Haxe: TimeHelper L780–805
pub fn try_age10_father_refollow(
    state: &mut SimState,
    outbound: &OutboundHub,
    child_conn: u64,
    is_male: bool,
    rand01: f32,
) -> bool {
    let (child_p_id, mother_id, father_id) = {
        let Some(child) = state.players.get(&child_conn) else {
            return false;
        };
        if child.deleted {
            return false;
        }
        let node = state.social.lineages.get(&child.p_id);
        (
            child.p_id,
            node.and_then(|n| n.mother_id),
            node.and_then(|n| n.father_id),
        )
    };
    let father_deleted = father_id
        .map(|fid| {
            state
                .players
                .values()
                .find(|p| p.p_id == fid)
                .map(|p| p.deleted)
                .unwrap_or(true)
        })
        .unwrap_or(true);
    if !father_refollow_eligible(
        &state.social.following,
        child_p_id,
        mother_id,
        father_id,
        father_deleted,
    ) {
        return false;
    }
    if !father_refollow_chance_fires(is_male, rand01) {
        return false;
    }
    let Some(fid) = father_id else {
        return false;
    };
    if state.social.set_follow(child_p_id, fid).is_err() {
        return false;
    }
    // Emit LEADER pin to child (father) + FOLLOWER pin to father (child).
    // Haxe: player.connection.sendMapLocation(father, 'LEADER', 'leader');
    //       father.connection.sendMapLocation(player, 'FOLLOWER', 'follower');
    let (father_conn, father_xy, child_xy, child_p, father_p, child_birth, father_birth) = {
        let child = state.players.get(&child_conn);
        let father = state.players.values().find(|p| p.p_id == fid && !p.deleted);
        match (child, father) {
            (Some(c), Some(f)) => (
                f.conn_id,
                (f.x, f.y),
                (c.x, c.y),
                c.p_id,
                f.p_id,
                (c.birth_x, c.birth_y),
                (f.birth_x, f.birth_y),
            ),
            _ => return true, // follow applied even if pin skip
        }
    };
    let (rel_x, rel_y) = (
        father_xy.0 - child_birth.0,
        father_xy.1 - child_birth.1,
    );
    send_map_location_pin(
        outbound,
        child_conn,
        child_p,
        LEADER_TEXT1,
        LEADER_TEXT2,
        father_p,
        rel_x,
        rel_y,
        true,
    );
    let (rel_x2, rel_y2) = (
        child_xy.0 - father_birth.0,
        child_xy.1 - father_birth.1,
    );
    send_map_location_pin(
        outbound,
        father_conn,
        father_p,
        FOLLOWER_TEXT1,
        FOLLOWER_TEXT2,
        child_p,
        rel_x2,
        rel_y2,
        true,
    );
    true
}

/// Relative map coords for a follow-request FOLLOWER pin (birth-origin transform).
// Haxe: Connection.sendMapLocation transformX/Y = target.tx - viewer.gx
pub fn follow_request_pin_rel(
    host_birth_x: i32,
    host_birth_y: i32,
    follower_x: i32,
    follower_y: i32,
) -> (i32, i32) {
    (follower_x - host_birth_x, follower_y - host_birth_y)
}

/// Helper for tests / callers: deleted set from players.
#[allow(dead_code)]
pub fn deleted_set(state: &SimState) -> HashSet<i32> {
    deleted_p_ids(state)
}

/// Follower membership for pure tests via SocialState.
#[allow(dead_code)]
pub fn is_follower_of(social: &SocialState, follower: i32, leader: i32) -> bool {
    social.is_follower_from(follower, leader)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn pin_body_labels() {
        assert_eq!(
            format_social_map_pin_body(MOTHER_TEXT1, MOTHER_TEXT2, 9, 2, -3),
            "MOTHER *leader 9 *map 2 -3"
        );
        assert_eq!(
            format_social_map_pin_body(BABY_TEXT1, BABY_TEXT2, 3, 0, 1),
            "BABY *baby 3 *map 0 1"
        );
        assert_eq!(
            format_social_map_pin_body(HUMAN_TEXT1, HUMAN_TEXT2, 4, 5, 6),
            "HUMAN *follower 4 *map 5 6"
        );
        assert_eq!(
            format_social_map_pin_body(ALLY_TEXT1, ALLY_TEXT2, 7, 1, 1),
            "ALLY *follower 7 *map 1 1"
        );
        assert_eq!(
            format_social_map_pin_body(FAM_TEXT1, FAM_TEXT2, 8, -1, 0),
            "FAM *follower 8 *map -1 0"
        );
        assert_eq!(
            format_social_map_pin_body(FOLLOWER_TEXT1, FOLLOWER_TEXT2, 2, 3, 4),
            "FOLLOWER *follower 2 *map 3 4"
        );
    }

    #[test]
    fn count_picks_closest() {
        let cands = vec![
            SocialPinCandidate {
                p_id: 2,
                x: 10,
                y: 0,
                name: "FAR".into(),
            },
            SocialPinCandidate {
                p_id: 3,
                x: 1,
                y: 0,
                name: "NEAR".into(),
            },
        ];
        let r = count_and_display_closest(0, 0, &cands, true);
        assert_eq!(r.count, 2);
        assert_eq!(r.best.as_ref().unwrap().p_id, 3);
        assert_eq!(r.balloons.len(), 2);
        assert_eq!(r.balloons[0].1, "+FAR+");
        assert_eq!(format_allies_count_say(2, true), "ALLIES 2!");
        assert_eq!(format_allies_count_say(2, false), "ALLIES 2");
        assert_eq!(format_follower_count_say(1), "FOLLOWER 1");
        assert_eq!(format_family_count_say(0), "FAMILY 0");
    }

    #[test]
    fn parse_social_pin_say_forms() {
        assert_eq!(parse_social_pin_say("!H"), Some(SocialPinSay::Human));
        assert_eq!(parse_social_pin_say("!HUMAN"), Some(SocialPinSay::Human));
        assert_eq!(
            parse_social_pin_say("?ALLY"),
            Some(SocialPinSay::AllyExcited)
        );
        assert_eq!(parse_social_pin_say("ALLY?"), Some(SocialPinSay::AllyPlain));
        assert_eq!(
            parse_social_pin_say("?F"),
            Some(SocialPinSay::FollowerPrivate)
        );
        assert_eq!(
            parse_social_pin_say("?FOLLOWER"),
            Some(SocialPinSay::FollowerPrivate)
        );
        assert_eq!(
            parse_social_pin_say("FOLLOWER?"),
            Some(SocialPinSay::FollowerPublic)
        );
        assert_eq!(
            parse_social_pin_say("?FAM"),
            Some(SocialPinSay::FamilyPrivate)
        );
        assert_eq!(
            parse_social_pin_say("FAM?"),
            Some(SocialPinSay::FamilyPublic)
        );
        // ?FAMILY is the string-name list query — not FAM pins
        assert_eq!(parse_social_pin_say("?FAMILY"), None);
        assert_eq!(parse_social_pin_say("ALLY 3"), None);
    }

    #[test]
    fn social_pin_name_prefers_lineage_first_token() {
        assert_eq!(
            social_pin_name("ADA", Some("LINA SNOW")),
            "LINA"
        );
        assert_eq!(social_pin_name("ADA", None), "ADA");
        assert_eq!(social_pin_name("ADA", Some("")), "ADA");
    }

    #[test]
    fn follow_request_rel_uses_birth_origin() {
        assert_eq!(follow_request_pin_rel(100, 200, 105, 210), (5, 10));
    }

    #[test]
    fn father_refollow_pure_gates() {
        assert!(crossed_true_age_year(9.9, 10.01, 10));
        assert!(!crossed_true_age_year(10.0, 10.5, 10));
        assert!(!crossed_true_age_year(8.0, 9.5, 10));
        assert!(father_refollow_chance_fires(true, 0.41));
        assert!(!father_refollow_chance_fires(true, 0.4));
        assert!(father_refollow_chance_fires(false, 0.81));
        assert!(!father_refollow_chance_fires(false, 0.5));
        let mut following = HashMap::new();
        following.insert(3, 1); // child follows mother
        assert!(father_refollow_eligible(
            &following,
            3,
            Some(1),
            Some(2),
            false
        ));
        following.insert(3, 2); // already follows father
        assert!(!father_refollow_eligible(
            &following,
            3,
            Some(1),
            Some(2),
            false
        ));
        following.insert(3, 1);
        assert!(!father_refollow_eligible(
            &following,
            3,
            Some(1),
            Some(2),
            true
        ));
    }
}
