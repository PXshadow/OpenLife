//! SCORE-AGE-58 — Haxe TimeHelper age-58 prestige GM.
//!
//! Once per life when `true_age` crosses 58 (same fire-once helper as AGE-10):
//! `Your life nears the end. You earned ${floor(yum_multiplier)} prestige!`
//! then, if `DisplayScoreOn` (Live, Haxe default true), extra GMs for each
//! OLN15 `prestige_from` bucket whose floor is `> 5` (wealth uses raw `> 5`),
//! amounts × `DisplayScoreFactor`.
//!
//! Empty bucket strings are omitted (`sendGlobalMessage` length gate). AI bodies
//! get no GM (`Connection.sendGlobalMessage` isAi return).
//!
//! Haxe L808 uses `Std.int(player.age)`; this leftover fires on **true_age**.
// Haxe: TimeHelper.updateAge L808–839

use crate::score_entry::format_global_message_text;
use crate::SimState;
use ol_identity::PrestigeFromBreakdown;
use ol_net::OutboundHub;
use ol_protocol::{format_frame, format_server_message};

/// Haxe `Std.int(player.age) == 58` year; Rust fires on `true_age` cross.
// Haxe: TimeHelper L808
pub const SCORE_AGE_YEAR: i32 = 58;

/// Haxe `ServerSettings.DisplayScoreOn` compiled fallback (live: `GameplayKnobs.display_score_on`).
// Haxe: ServerSettings.DisplayScoreOn = true
// SETTINGS-KNOB-TAIL
pub const DISPLAY_SCORE_ON_DEFAULT: bool = true;

/// Bucket shown when `Math.floor(prestigeFrom*) > 5` (wealth: raw `> 5`).
// Haxe: TimeHelper L820–826
pub const SCORE_BUCKET_DISPLAY_MIN: i32 = 5;

/// Inputs for [`age58_score_messages`].
#[derive(Debug, Clone, Copy)]
pub struct Age58ScoreInput {
    /// Haxe `player.yum_multiplier` (lineage prestige).
    pub yum_multiplier: f32,
    /// OLN15 / Haxe `prestigeFrom*` buckets.
    pub prestige_from: PrestigeFromBreakdown,
    /// Haxe `DisplayScoreFactor` (Live).
    pub display_score_factor: f32,
    /// Haxe `DisplayScoreOn`.
    pub display_score_on: bool,
}

/// Plain (pre-wire) GM bodies for the age-58 block, header first.
///
/// Empty Haxe ternary strings are not included.
// Haxe: TimeHelper L808–839
pub fn age58_score_messages(input: Age58ScoreInput) -> Vec<String> {
    let mut out = Vec::with_capacity(8);
    out.push(format_life_nears_end_gm(input.yum_multiplier));
    if !input.display_score_on {
        return out;
    }
    let factor = finite_factor(input.display_score_factor);
    let pf = input.prestige_from;
    push_floored_in_total(&mut out, pf.children, factor, "children");
    push_floored_in_total(&mut out, pf.grandkids, factor, "grandkids");
    push_floored_in_total(&mut out, pf.followers, factor, "followers");
    push_floored_in_total(&mut out, pf.eating, factor, "YUMMY food");
    if pf.wealth.is_finite() && pf.wealth > SCORE_BUCKET_DISPLAY_MIN as f32 {
        let n = haxe_num_text(pf.wealth * factor);
        out.push(format!("You have gained {n} prestige from your wealth!"));
    }
    push_floored_gained(&mut out, pf.parents, factor, "parents");
    push_floored_gained(&mut out, pf.siblings, factor, "siblings");
    out
}

/// Haxe `Your life nears the end. You earned ${Math.floor(yum_multiplier)} prestige!`
// Haxe: TimeHelper L829
pub fn format_life_nears_end_gm(yum_multiplier: f32) -> String {
    let total = haxe_floor_i32(yum_multiplier);
    format!("Your life nears the end. You earned {total} prestige!")
}

/// Vitals entry: `true_age` just crossed 58. Skip AI / deleted (Haxe sendGlobalMessage).
// Haxe: TimeHelper L808–839 + Connection.sendGlobalMessage isAi
pub fn try_age58_score_gm_on_cross(
    state: &mut SimState,
    outbound: &OutboundHub,
    conn_id: u64,
) {
    let (p_id, skip) = match state.players.get(&conn_id) {
        Some(p) => (p.p_id, p.deleted || p.is_ai_body()),
        None => return,
    };
    if skip {
        return;
    }
    let yum = state.player_prestige(p_id);
    let prestige_from = state
        .social
        .lineages
        .get(&p_id)
        .map(|n| n.prestige_from)
        .unwrap_or_default();
    let msgs = age58_score_messages(Age58ScoreInput {
        yum_multiplier: yum,
        prestige_from,
        display_score_factor: state.gameplay.display_score_factor,
        display_score_on: state.gameplay.display_score_on,
    });
    send_player_gms(outbound, conn_id, &msgs);
}

fn send_player_gms(outbound: &OutboundHub, conn_id: u64, msgs: &[String]) {
    if msgs.is_empty() {
        return;
    }
    for text in msgs {
        if text.is_empty() {
            continue;
        }
        let gm_wire = format_global_message_text(text);
        if gm_wire.is_empty() {
            continue;
        }
        outbound.send(
            conn_id,
            format_server_message("GM", &[&gm_wire]).into_bytes(),
        );
    }
    outbound.send_urgent(conn_id, format_frame().into_bytes());
}

fn push_floored_in_total(out: &mut Vec<String>, raw: f32, factor: f32, source: &str) {
    let floor = haxe_floor_i32(raw);
    if floor <= SCORE_BUCKET_DISPLAY_MIN {
        return;
    }
    let n = haxe_num_text(floor as f32 * factor);
    out.push(format!(
        "You have gained in total {n} prestige from {source}!"
    ));
}

fn push_floored_gained(out: &mut Vec<String>, raw: f32, factor: f32, source: &str) {
    let floor = haxe_floor_i32(raw);
    if floor <= SCORE_BUCKET_DISPLAY_MIN {
        return;
    }
    let n = haxe_num_text(floor as f32 * factor);
    out.push(format!("You have gained {n} prestige from {source}!"));
}

fn finite_factor(factor: f32) -> f32 {
    if factor.is_finite() {
        factor
    } else {
        1.0
    }
}

/// Haxe `Math.floor` then int (prestige display).
fn haxe_floor_i32(v: f32) -> i32 {
    if !v.is_finite() {
        return 0;
    }
    v.floor().clamp(i32::MIN as f32, i32::MAX as f32) as i32
}

/// Haxe `${float}` — whole values drop `.0`.
fn haxe_num_text(v: f32) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    let r = v.round();
    if (v - r).abs() <= 1e-4 {
        format!("{}", r as i64)
    } else {
        format!("{}", v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buckets(
        children: f32,
        grandkids: f32,
        followers: f32,
        eating: f32,
        wealth: f32,
        parents: f32,
        siblings: f32,
    ) -> PrestigeFromBreakdown {
        PrestigeFromBreakdown {
            children,
            grandkids,
            eating,
            followers,
            wealth,
            parents,
            siblings,
        }
    }

    #[test]
    fn life_nears_end_floors_yum_multiplier() {
        assert_eq!(
            format_life_nears_end_gm(12.7),
            "Your life nears the end. You earned 12 prestige!"
        );
        assert_eq!(
            format_life_nears_end_gm(0.9),
            "Your life nears the end. You earned 0 prestige!"
        );
    }

    #[test]
    fn age58_gm_texts_lock_haxe_wording_and_omit_small_buckets() {
        let msgs = age58_score_messages(Age58ScoreInput {
            yum_multiplier: 12.7,
            prestige_from: buckets(10.9, 3.0, 6.0, 20.0, 8.5, 6.0, 0.0),
            display_score_factor: 1.0,
            display_score_on: true,
        });
        assert_eq!(
            msgs,
            vec![
                "Your life nears the end. You earned 12 prestige!".to_string(),
                "You have gained in total 10 prestige from children!".to_string(),
                "You have gained in total 6 prestige from followers!".to_string(),
                "You have gained in total 20 prestige from YUMMY food!".to_string(),
                "You have gained 8.5 prestige from your wealth!".to_string(),
                "You have gained 6 prestige from parents!".to_string(),
            ]
        );
    }

    #[test]
    fn display_score_off_sends_header_only() {
        let msgs = age58_score_messages(Age58ScoreInput {
            yum_multiplier: 9.0,
            prestige_from: buckets(20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0),
            display_score_factor: 1.0,
            display_score_on: false,
        });
        assert_eq!(
            msgs,
            vec!["Your life nears the end. You earned 9 prestige!".to_string()]
        );
    }

    #[test]
    fn floor_bucket_needs_strictly_more_than_five() {
        let msgs = age58_score_messages(Age58ScoreInput {
            yum_multiplier: 1.0,
            prestige_from: buckets(5.9, 0.0, 0.0, 0.0, 5.0, 0.0, 0.0),
            display_score_factor: 1.0,
            display_score_on: true,
        });
        assert_eq!(
            msgs,
            vec!["Your life nears the end. You earned 1 prestige!".to_string()]
        );
        let wealth_on = age58_score_messages(Age58ScoreInput {
            yum_multiplier: 1.0,
            prestige_from: buckets(0.0, 0.0, 0.0, 0.0, 5.1, 0.0, 0.0),
            display_score_factor: 1.0,
            display_score_on: true,
        });
        assert_eq!(wealth_on.len(), 2);
        assert_eq!(
            wealth_on[1],
            "You have gained 5.1 prestige from your wealth!"
        );
    }

    #[test]
    fn display_score_factor_multiplies_shown_amount() {
        let msgs = age58_score_messages(Age58ScoreInput {
            yum_multiplier: 4.0,
            prestige_from: buckets(7.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            display_score_factor: 1.5,
            display_score_on: true,
        });
        assert_eq!(
            msgs[1],
            "You have gained in total 10.5 prestige from children!"
        );
    }

    #[test]
    fn gm_wire_upper_underscores_locks_header() {
        assert_eq!(
            format_global_message_text(&format_life_nears_end_gm(12.0)),
            "YOUR_LIFE_NEARS_THE_END._YOU_EARNED_12_PRESTIGE!"
        );
        assert_eq!(
            format_global_message_text(
                "You have gained in total 20 prestige from YUMMY food!"
            ),
            "YOU_HAVE_GAINED_IN_TOTAL_20_PRESTIGE_FROM_YUMMY_FOOD!"
        );
    }
}
