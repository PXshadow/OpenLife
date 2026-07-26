//! Server→client message builders for post-mutation visibility (Haxe-shaped).

use crate::format_server_message;

/// MX — map cell change (Haxe `MAP_CHANGE`).
/// `x y new_floor_id new_id p_id`
///
/// **p_id semantics (protocol.txt):**
/// - positive `p_id` → object was **dropped** by that player (client may animate from them)
/// - `-(p_id)` (`p_id < -1`) → player-triggered **transform** (no fly-from-hand)
/// - `-1` / `0` → non-player change
pub fn format_map_change(
    x: i32,
    y: i32,
    floor_id: i32,
    object_id: i32,
    player_id: i32,
) -> String {
    format_server_message(
        "MX",
        &[&format!("{x} {y} {floor_id} {object_id} {player_id}")],
    )
}

/// MX with optional motion (Haxe `sendMapUpdateForMoving` / animal walks).
///
/// Wire: `x y new_floor_id new_id p_id old_x old_y speed`
/// Client interpolates the object from `(old_x,old_y)` to `(x,y)` at `speed`.
/// Use only for true object moves (animals). Instant places use [`format_map_change`].
pub fn format_map_change_moving(
    x: i32,
    y: i32,
    floor_id: i32,
    object_id: i32,
    player_id: i32,
    old_x: i32,
    old_y: i32,
    speed: f32,
) -> String {
    let speed = (speed * 100.0).round() / 100.0;
    format_server_message(
        "MX",
        &[&format!(
            "{x} {y} {floor_id} {object_id} {player_id} {old_x} {old_y} {speed:.2}"
        )],
    )
}

/// FX — food change line fields (Haxe `sendFoodUpdate`).
pub fn format_food_change(
    food_store: i32,
    food_capacity: i32,
    last_ate_id: i32,
    last_ate_fill_max: i32,
    move_speed: f32,
    responsible_id: i32,
    yum_bonus: i32,
    yum_multiplier: i32,
) -> String {
    format_server_message(
        "FX",
        &[&format!(
            "{food_store} {food_capacity} {last_ate_id} {last_ate_fill_max} {move_speed:.2} {responsible_id} {yum_bonus} {yum_multiplier}"
        )],
    )
}

/// BW — BABY_WIGGLE (Haxe `sendWiggle` / protocol list of baby p_ids).
/// Single-player line: `p_id`.
pub fn format_baby_wiggle(p_id: i32) -> String {
    format_server_message("BW", &[&p_id.to_string()])
}

/// LS — LOCATION_SAYS (Haxe `ClientTag.LOCATION_SAYS`).
///
/// Wire data line: `x y [text…]`.
/// Empty `text` → **exactly** `LS\n{x} {y}\n#` (no POS labels, no p_id, no extra fields).
///
/// **Official clients only apply LS after a following `FM` (FRAME)** once
/// login has set waitForFrameMessages (LivingLifePage.cpp). Always pair with FM.
pub fn format_location_says(x: i32, y: i32, text: &str) -> String {
    if text.is_empty() {
        // Bare coordinates only — used for 1 Hz pos debug to humans.
        format!("LS\n{x} {y}\n#")
    } else {
        format_server_message("LS", &[&format!("{x} {y} {text}")])
    }
}

/// PS — PLAYER_SAYS (protocol.txt + Haxe `id/curse text`).
///
/// Wire data line **must** be `p_id/isCurse text` (slash required).
/// Example: `PS\n1432/0 HELLO THERE\n#`
///
/// Official clients only play PS after a following `FM` once logged in.
pub fn format_player_says(p_id: i32, is_curse: bool, text: &str) -> String {
    let curse = if is_curse { 1 } else { 0 };
    let text = text.replace('#', " ").replace('\n', " ");
    format_server_message("PS", &[&format!("{p_id}/{curse} {text}")])
}

/// End-of-frame marker (protocol FRAME / FM). Required so official clients
/// flush buffered PM/PU/PS/LS after ACCEPTED.
pub fn format_frame() -> String {
    format_server_message("FM", &[])
}

/// DY — DYING (Haxe `SendDyingToAll` / protocol).
/// `p_id` or `p_id 1` when `sick` (isSick flag; client skips blood overlay).
pub fn format_dying(p_id: i32, sick: bool) -> String {
    if sick {
        format_server_message("DY", &[&format!("{p_id} 1")])
    } else {
        format_server_message("DY", &[&p_id.to_string()])
    }
}

/// LR — LEARNED_TOOL_REPORT (Haxe `sendLearnedTool` / protocol).
/// One data line: space-separated object ids (`tool_id tool_id ... tool_id`).
/// Empty `tool_ids` yields `LR\n#` (callers usually skip empty).
pub fn format_learned_tool_report(tool_ids: &[i32]) -> String {
    if tool_ids.is_empty() {
        return format_server_message("LR", &[]);
    }
    let mut sorted: Vec<i32> = tool_ids.to_vec();
    sorted.sort_unstable();
    let line = sorted
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    format_server_message("LR", &[&line])
}

/// TS — TOOL_SLOTS (Haxe `TOOL_SLOTS`): `used total`.
pub fn format_tool_slots(used: i32, total: i32) -> String {
    format_server_message("TS", &[&format!("{used} {total}")])
}

/// HX — HEAT_CHANGE (Haxe `send HEAT_CHANGE` / protocol).
///
/// Wire line: `heat food_time indoor_bonus`
/// Heat is warmth in \[0,1\] (0.5 ideal). Food drain time and indoor bonus are seconds.
pub fn format_heat_change(heat: f32, food_time: f32, indoor_bonus: f32) -> String {
    format_server_message(
        "HX",
        &[&format!("{heat:.2} {food_time:.2} {indoor_bonus:.2}")],
    )
}

/// NM — NAME (Haxe name / family display). `p_id first last` or full display line.
pub fn format_name_message(p_id: i32, first: &str, last: &str) -> String {
    format_server_message("NM", &[&format!("{p_id} {first} {last}")])
}

/// PE — PLAYER_EMOT (Haxe emote). `p_id emot_index`.
pub fn format_player_emot(p_id: i32, emot_index: i32) -> String {
    format_server_message("PE", &[&format!("{p_id} {emot_index}")])
}

/// PM — PLAYER_MOVES_START body fields (Haxe `generateRelativeMoveUpdateString`).
///
/// Wire shape: `p_id targetX targetY total_sec eta trunc dx0 dy0 …`
/// (`total_sec` / `eta` rounded to 2 decimals).
pub fn format_player_moves_start(
    p_id: i32,
    xs: i32,
    ys: i32,
    total_sec: f32,
    eta_sec: f32,
    trunc: i32,
    deltas: &[(i32, i32)],
) -> String {
    let total = (total_sec * 100.0).round() / 100.0;
    let eta = (eta_sec * 100.0).round() / 100.0;
    let mut body = format!("{p_id} {xs} {ys} {total:.2} {eta:.2} {trunc}");
    for &(dx, dy) in deltas {
        body.push_str(&format!(" {dx} {dy}"));
    }
    format_server_message("PM", &[&body])
}

/// PU — PLAYER_UPDATE single-line helper for held-only change notes.
/// Full PU lines are built by `format_player_update_line`; this is a minimal
/// held-change broadcast: `p_id x y held_id`.
pub fn format_held_update(p_id: i32, x: i32, y: i32, held_id: i32) -> String {
    format_server_message("PU", &[&format!("{p_id} {x} {y} {held_id}")])
}

/// WS — weather status line (server extension; not vanilla OHOL).
/// `kind drain_mult` e.g. `rain 1.02`.
pub fn format_weather_status(kind: &str, drain_mult: f32) -> String {
    format_server_message("WS", &[&format!("{kind} {drain_mult:.2}")])
}

/// GV — grave placed note (server extension / death marker).
/// `x y object_id p_id`.
pub fn format_grave_place(x: i32, y: i32, object_id: i32, p_id: i32) -> String {
    format_server_message(
        "GV",
        &[&format!("{x} {y} {object_id} {p_id}")],
    )
}

/// FW — FOLLOWING (Haxe follow badge).
/// Full wire packet: `follower_id leader_id color`.
pub fn format_following_wire(follower: i32, leader: i32, color: i32) -> String {
    format_server_message("FW", &[&format!("{follower} {leader} {color}")])
}

/// EX — EXILED pair (Haxe exile).
/// Full wire packet: `target_id exiler_id`.
pub fn format_exile_wire(target: i32, exiler: i32) -> String {
    format_server_message("EX", &[&format!("{target} {exiler}")])
}

/// CX — CURSE_TOKEN_CHANGE (protocol.txt / Haxe).
/// Wire: `CX\n{count}\n#`
pub fn format_curse_token_change(count: i32) -> String {
    format_server_message("CX", &[&count.to_string()])
}

/// CS — CURSE_SCORE_CHANGE (protocol.txt / Haxe).
/// Wire: `CS\n{excess}\n#`
pub fn format_curse_score_change(excess: i32) -> String {
    format_server_message("CS", &[&excess.to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ls_pos_debug_is_bare_xy_only() {
        // Empty text → coordinates only.
        assert_eq!(format_location_says(488, 488, ""), "LS\n488 488\n#");
        assert_eq!(format_location_says(-3, 12, ""), "LS\n-3 12\n#");
        // Spoken map label: single token (Haxe client uses split()[2] only).
        assert_eq!(
            format_location_says(0, 0, "488,489"),
            "LS\n0 0 488,489\n#"
        );
        let with_text = format_location_says(1, 2, "MOTHER");
        assert_eq!(with_text, "LS\n1 2 MOTHER\n#");
        assert!(!format_location_says(0, 0, "1,2").contains("POS"));
    }

    #[test]
    fn mx_shape() {
        let s = format_map_change(3, 4, 0, 33, 2);
        assert_eq!(s, "MX\n3 4 0 33 2\n#");
        // Transform responsible = -(player) so client does not fly-from-hand.
        let t = format_map_change(3, 4, 0, 0, -7);
        assert_eq!(t, "MX\n3 4 0 0 -7\n#");
        let m = format_map_change_moving(5, 6, 0, 418, -1, 4, 6, 1.0);
        assert_eq!(m, "MX\n5 6 0 418 -1 4 6 1.00\n#");
    }

    #[test]
    fn ps_protocol_requires_slash_is_curse() {
        // protocol.txt (PS): `p_id/isCurse text` — Haxe uses indexOf("/").
        assert_eq!(
            format_player_says(1432, false, "HELLO THERE"),
            "PS\n1432/0 HELLO THERE\n#"
        );
        assert_eq!(
            format_player_says(1501, true, "CURSE JOHN SMITH"),
            "PS\n1501/1 CURSE JOHN SMITH\n#"
        );
        assert_eq!(format_frame(), "FM\n#");
    }

    #[test]
    fn pm_golden_one_step_east() {
        let total = (1.0f32 / 3.75 * 100.0).round() / 100.0;
        let s = format_player_moves_start(7, 10, 20, total, total, 0, &[(1, 0)]);
        assert_eq!(s, "PM\n7 10 20 0.27 0.27 0 1 0\n#");
    }

    #[test]
    fn pm_golden_two_steps() {
        let total = (2.0f32 / 3.75 * 100.0).round() / 100.0;
        let s = format_player_moves_start(7, 10, 20, total, total, 0, &[(1, 0), (0, 1)]);
        assert_eq!(s, "PM\n7 10 20 0.53 0.53 0 1 0 0 1\n#");
    }

    #[test]
    fn pm_golden_diagonal() {
        let total = (std::f32::consts::SQRT_2 / 3.75 * 100.0).round() / 100.0;
        let s = format_player_moves_start(7, 10, 20, total, total, 0, &[(1, 1)]);
        assert_eq!(s, "PM\n7 10 20 0.38 0.38 0 1 1\n#");
    }

    #[test]
    fn pm_golden_trunc_one() {
        let total = (1.0f32 / 3.75 * 100.0).round() / 100.0;
        let s = format_player_moves_start(7, 10, 20, total, total, 1, &[(1, 0)]);
        assert_eq!(s, "PM\n7 10 20 0.27 0.27 1 1 0\n#");
    }

    #[test]
    fn fx_shape() {
        let s = format_food_change(10, 20, 0, 0, 3.75, -1, 0, 0);
        assert!(s.starts_with("FX\n"));
        assert!(s.contains("10 20"));
        assert!(s.ends_with("#"));
    }

    #[test]
    fn baby_wiggle_shape() {
        assert_eq!(format_baby_wiggle(42), "BW\n42\n#");
    }

    #[test]
    fn heat_change_shape() {
        assert_eq!(
            format_heat_change(0.5, 0.0, 0.0),
            "HX\n0.50 0.00 0.00\n#"
        );
    }

    #[test]
    fn dying_shape() {
        assert_eq!(format_dying(7, false), "DY\n7\n#");
        assert_eq!(format_dying(7, true), "DY\n7 1\n#");
    }

    #[test]
    fn learned_tool_report_shape() {
        // Protocol: LR\ntool_id tool_id ...\n#
        assert_eq!(format_learned_tool_report(&[334]), "LR\n334\n#");
        assert_eq!(format_learned_tool_report(&[334, 12]), "LR\n12 334\n#");
        assert_eq!(format_learned_tool_report(&[]), "LR\n#");
    }

    #[test]
    fn tool_slots_shape() {
        assert_eq!(format_tool_slots(2, 1000), "TS\n2 1000\n#");
    }

    #[test]
    fn name_emot_held_shapes() {
        assert_eq!(format_name_message(2, "Ada", "Snow"), "NM\n2 Ada Snow\n#");
        assert_eq!(format_player_emot(2, 1), "PE\n2 1\n#");
        assert_eq!(format_held_update(2, 10, 11, 33), "PU\n2 10 11 33\n#");
    }

    #[test]
    fn weather_and_grave_shapes() {
        assert_eq!(format_weather_status("rain", 1.02), "WS\nrain 1.02\n#");
        assert_eq!(format_grave_place(1, 2, 0, 7), "GV\n1 2 0 7\n#");
    }

    #[test]
    fn following_and_exile_wire_shapes() {
        assert_eq!(
            format_following_wire(2, 5, 1),
            "FW\n2 5 1\n#"
        );
        assert_eq!(format_exile_wire(7, 5), "EX\n7 5\n#");
    }

    #[test]
    fn curse_cx_cs_wire_shapes() {
        assert_eq!(format_curse_token_change(1), "CX\n1\n#");
        assert_eq!(format_curse_score_change(3), "CS\n3\n#");
        assert_eq!(format_curse_token_change(0), "CX\n0\n#");
    }
}
