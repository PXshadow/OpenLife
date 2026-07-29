//! Scripted `AiBase.sayHelper` commands (**AI-SAY-HELPER** / `scripted_cmds`).
//!
//! Haxe: `openlife.auto.AiBase.sayHelper` (~L4727–5002) — HOLA / NAME? / FOLLOW /
//! STOP / DROP / MAKE / CRAFT / PROF / profession assign / ally gates, before LLM fallback.
//!
//! Pure match + plan only. Live fan-out is `fan_out_ai_say_scripted` in `lib.rs`.

use crate::ai_handler::{
    check_if_you_are_allied_speech, AlliedSpeechOutcome, LLM_NOT_ALLY_SAY, LLM_SPEECH_ANGRY_EMOTE_ID,
    LLM_SPEECH_COOLDOWN_SECS,
};

// ---------------------------------------------------------------------------
// Constants (Haxe AiBase.sayHelper)
// ---------------------------------------------------------------------------

/// Haxe `timePassedInSeconds > 4` for HOLA / NAME? / ARE YOU AI.
pub const SCRIPTED_CMD_COOLDOWN_SECS: f32 = LLM_SPEECH_COOLDOWN_SECS; // 4.0
/// Haxe `waitingTime += 2` after friendly HOLA / NAME.
pub const HOLA_WAITING_TIME_ADD: f32 = 2.0;
/// Haxe STOP/WAIT `waitingTime = 10` (assign, not floor).
pub const STOP_WAITING_TIME: f32 = 10.0;
/// Haxe doDropCommand `waitingTime = 1` (assign).
pub const DROP_WAITING_TIME: f32 = 1.0;
/// Haxe GO HOME near home `this.time += 5`.
pub const GO_HOME_NEAR_TIME_BUMP: f32 = 5.0;
/// Haxe GO HOME far `this.time += 6`.
pub const GO_HOME_FAR_TIME_BUMP: f32 = 6.0;
/// Haxe GO HOME `quadDistance < 3` (already-home gate).
pub const GO_HOME_NEAR_QUAD: f32 = 3.0;
/// Haxe `isMovingToHome` default `maxDistance = 3` (tiles; squared compare).
// Haxe: AiBase.isMovingToHome maxDistance = 3
pub const GO_HOME_PATH_MAX_TILES: i32 = 3;
/// Haxe isMovingToHome stand-off `dist = 2` half-range for rand offset.
// Haxe: AiBase.isMovingToHome L8162
pub const GO_HOME_STAND_HALF: i32 = 2;
/// Haxe `shouldDebugSay` path success line.
// Haxe: AiBase.sayHelper GO HOME L4895
pub const GO_HOME_SAY_GOING: &str = "GOING HOME!";
/// Haxe `shouldDebugSay` path fail line.
// Haxe: AiBase.sayHelper GO HOME L4896
pub const GO_HOME_SAY_CANNOT: &str = "I CANNOT GO HOME!";
/// Haxe `BiomeTag.SWAMP` for SearchNewHome skip (no floor).
// Haxe: AiHelper.SearchNewHome L2101
pub const HOME_SEARCH_SWAMP_BIOME: u8 = 1;
/// Local oven scan radius for HOME! (Haxe uses global oven list; cap near AI).
// Haxe: AiHelper.SearchNewHome global ovens; Rust local scan
pub const HOME_SEARCH_LOCAL_RADIUS: i32 = 80;
/// Haxe not-follower reject.
pub const NOT_FOLLOWER_SAY: &str = "I AM NOT YOUR FOLLOWER!";
/// Haxe professions static list (AiBase.professions).
// Haxe: AiBase.professions
pub const AI_PROFESSIONS: &[&str] = &[
    "SOILMAKER",
    "ROWMAKER",
    "BASICFARMER",
    "ADVANCEDFARMER",
    "SHEPHERD",
    "BAKER",
    "POTTER",
    "FIREKEEPER",
    "TAILOR",
    "FIREFOODMAKER",
    "LUMBERJACK",
    "WATERBRINGER",
    "FOODSERVER",
    "GRAVEKEEPER",
    "HUNTER",
    "SMITH",
    "CARROTFARMER",
    "COLLECTOR",
];
/// Haxe random ARE YOU AI replies (rand 0..8; 7 = silent).
// Haxe: AiBase.sayHelper ARE YOU AI rand 0..8
pub const ARE_YOU_AI_REPLIES: &[&str] = &[
    "Im not a stupid AI!",
    "Im an AI!",
    "No",
    "Sure",
    "yes i am!",
    "Yes, And you?",
    "Why should I?",
];

// ---------------------------------------------------------------------------
// Context + plan
// ---------------------------------------------------------------------------

/// Live facts needed to pure-plan one scripted sayHelper invocation.
// Haxe: AiBase.sayHelper locals + myPlayer / speaker
#[derive(Debug, Clone)]
pub struct ScriptedSayCtx {
    /// Normalized text after attention strip (may still have mixed case).
    pub text: String,
    pub now_sim: f32,
    /// Haxe `timeReactedLastCommand` (0 = never).
    pub last_react_sim_time: f32,
    pub ai_name: String,
    pub ai_family_name: String,
    pub speaker_name: String,
    pub speaker_p_id: i32,
    pub ai_angry: bool,
    pub speaker_angry: bool,
    pub speaker_holding_weapon: bool,
    /// Haxe `isFriendly` / ally for MOVE/DROP/STOP/MAKE.
    pub is_friendly: bool,
    /// Haxe `isFollowerFrom || isCloseRelative`.
    pub should_do_command: bool,
    pub is_nice_baby: bool,
    pub assigned_profession: Option<String>,
    pub last_profession: Option<String>,
    /// Squared distance to home object (for GO HOME).
    pub home_quad_dist: f32,
    /// Object name under home tile (NHOME!).
    pub home_name: String,
    /// 0..8 inclusive for ARE YOU AI (Haxe randomInt(8)).
    pub rand_ai: u8,
    /// Debug flags currently on AI (for PROF?/DEBUG echo).
    pub debug_say: bool,
    pub debug_profession: bool,
}

/// Side-effects of a matched scripted command (no world mutation here).
// Haxe: sayHelper command bodies
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScriptedSayPlan {
    /// When true, skip LLM fallback for this hearer (Haxe `return` after match).
    pub handled: bool,
    pub say: Option<String>,
    pub emote_id: Option<i32>,
    /// Haxe `timeReactedLastCommand = tick`.
    pub mark_reacted: bool,
    /// Floor on `waitingTime` (max with current) — `setWaitingTimeMin` style.
    pub waiting_time_min: Option<f32>,
    /// Direct assign `waitingTime = N` (STOP/DROP Haxe assign).
    // Haxe: waitingTime = 10 / waitingTime = 1
    pub waiting_time_set: Option<f32>,
    /// Add to waiting time (HOLA `waitingTime += 2`).
    pub waiting_time_add: f32,
    pub stop_goto_self: bool,
    /// Haxe FOLLOW/MOVE Goto(speaker+1, speaker).
    pub goto_speaker_offset: bool,
    pub start_follow: bool,
    pub clear_follow: bool,
    pub set_auto_stop_follow: Option<bool>,
    /// Follow target p_id when `start_follow` (speaker).
    pub follow_p_id: i32,
    pub follow_started_sim_time: f32,
    pub ordered_to_drop: bool,
    /// Immediate dropHeldObject path (LLM APPLY only; scripted DROP is deferred).
    // Haxe: orderedToDrop deferred to AI tick; do_drop_now is Rust LLM delta
    pub do_drop_now: bool,
    pub jump: bool,
    /// Raw MAKE/CRAFT text for `do_make_craft_command` (non-silent).
    pub make_craft_text: Option<String>,
    pub set_debug_say: Option<bool>,
    pub set_debug_profession: Option<bool>,
    /// `Some(None)` clears assigned profession (NONE!).
    pub set_assigned_profession: Option<Option<String>>,
    /// AI think-time bump (GO HOME `this.time +=`; mapped to waiting floor live).
    pub think_time_bump: f32,
    /// Haxe HOME! SearchNewHome + firePlace.
    pub search_new_home: bool,
    /// Haxe GO HOME far → `isMovingToHome` pathfind attempt.
    // Haxe: AiBase.sayHelper GO HOME L4894–4897
    pub move_to_home: bool,
}

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// Haxe `timePassedInSeconds > 4 || timeReactedLastCommand < 1`.
// Haxe: sayHelper HOLA/NAME/AI cooldown
#[inline]
pub fn scripted_cooldown_ok(last_react_sim_time: f32, now_sim: f32) -> bool {
    if last_react_sim_time < 1.0 {
        return true;
    }
    (now_sim - last_react_sim_time).max(0.0) > SCRIPTED_CMD_COOLDOWN_SECS
}

/// Haxe `checkIfShouldDoCommand` gate for scripted path (loud reject).
// Haxe: AiBase.checkIfShouldDoCommand
pub fn plan_should_do_command(should: bool) -> Option<ScriptedSayPlan> {
    if should {
        return None;
    }
    Some(ScriptedSayPlan {
        handled: true,
        say: Some(NOT_FOLLOWER_SAY.to_string()),
        emote_id: Some(LLM_SPEECH_ANGRY_EMOTE_ID),
        ..Default::default()
    })
}

/// Haxe `checkIfYouAreAllied` gate for scripted path (loud by default).
// Haxe: AiBase.checkIfYouAreAllied
pub fn plan_ally_gate(is_friendly: bool) -> Option<ScriptedSayPlan> {
    match check_if_you_are_allied_speech(is_friendly, false) {
        AlliedSpeechOutcome::Allowed => None,
        AlliedSpeechOutcome::DeniedSilent => Some(ScriptedSayPlan {
            handled: true,
            ..Default::default()
        }),
        AlliedSpeechOutcome::DeniedLoud { say, emote_id } => Some(ScriptedSayPlan {
            handled: true,
            say: Some(say.to_string()),
            emote_id: Some(emote_id),
            ..Default::default()
        }),
    }
}

/// Normalize profession alias tokens (FARMER/WHEAT/CARROT/COLLECT/NONE).
// Haxe: sayHelper endsWith "!" profession aliases
pub fn normalize_profession_token(raw: &str) -> Option<Option<String>> {
    let mut p = raw.trim().to_ascii_uppercase();
    if p.is_empty() {
        return None;
    }
    if p == "FARMER" || p == "WHEAT" {
        p = "BASICFARMER".into();
    } else if p == "CARROT" {
        p = "CARROTFARMER".into();
    } else if p == "COLLECT" {
        p = "COLLECTOR".into();
    }
    if p == "NONE" {
        return Some(None);
    }
    if AI_PROFESSIONS.iter().any(|k| *k == p) {
        return Some(Some(p));
    }
    // Unknown profession with ! — Haxe still returns after checkIfShouldDoCommand
    // without assigning; treat as handled no-op assign.
    Some(Some(p))
}

/// Haxe `createProfessionText`.
// Haxe: AiBase.createProfessionText
pub fn create_profession_text(assigned: Option<&str>, last: Option<&str>) -> String {
    let mut text = assigned.unwrap_or("").to_string();
    if text.is_empty() || assigned == last {
        text = last.unwrap_or("").to_string();
    } else if let Some(lp) = last {
        if !lp.is_empty() {
            text = format!("{text} doing {lp}");
        }
    }
    if text.is_empty() {
        "NONE".into()
    } else {
        text
    }
}

/// Whether `token` is a known assignable profession (after alias normalize).
// Haxe: professions.contains(prof)
pub fn profession_is_known(prof: &str) -> bool {
    AI_PROFESSIONS.iter().any(|k| *k == prof)
}

/// Haxe `isMovingToHome` move target: prefer `firePlace` over `home`.
// Haxe: AiBase.isMovingToHome L8157
#[inline]
pub fn go_home_move_target(
    home_x: i32,
    home_y: i32,
    fire_place: Option<(i32, i32)>,
) -> (i32, i32) {
    fire_place.unwrap_or((home_x, home_y))
}

/// Whether `isMovingToHome` should attempt `gotoAdv` (quad ≥ maxDistance²).
// Haxe: AiBase.isMovingToHome L8158–8160
#[inline]
pub fn should_path_to_home(quad_dist: f32, max_tiles: i32) -> bool {
    let mt = max_tiles.max(0);
    let max_q = (mt * mt) as f32;
    quad_dist >= max_q
}

/// Goal tile near home/fire with Haxe stand-off rand (`dist = 2`).
// Haxe: AiBase.isMovingToHome gotoAdv(moveTarget.tx + randX, …)
pub fn go_home_goal_xy(tx: i32, ty: i32, seed: u32) -> (i32, i32) {
    let half = GO_HOME_STAND_HALF.max(0);
    if half == 0 {
        return (tx, ty);
    }
    let span = (2 * half + 1) as u32;
    let rx = (seed % span) as i32 - half;
    let ry = ((seed / 7) % span) as i32 - half;
    (tx + rx, ty + ry)
}

/// Haxe `shouldDebugSay` GO HOME path result lines.
// Haxe: AiBase.sayHelper L4894–4896
#[inline]
pub fn go_home_debug_say(debug_say: bool, path_ok: bool) -> Option<&'static str> {
    if !debug_say {
        return None;
    }
    Some(if path_ok {
        GO_HOME_SAY_GOING
    } else {
        GO_HOME_SAY_CANNOT
    })
}

/// Haxe SearchNewHome swamp filter: skip oven with no floor on swamp biome.
// Haxe: AiHelper.SearchNewHome L2101 `floorId < 1 && originalBiomeId == SWAMP`
#[inline]
pub fn home_oven_biome_allowed(has_floor: bool, biome: u8) -> bool {
    has_floor || biome != HOME_SEARCH_SWAMP_BIOME
}

/// Squared distance helper for home path decisions.
#[inline]
pub fn home_quad_dist(ax: i32, ay: i32, bx: i32, by: i32) -> f32 {
    let dx = (ax - bx) as f32;
    let dy = (ay - by) as f32;
    dx * dx + dy * dy
}

// ---------------------------------------------------------------------------
// Main pure planner
// ---------------------------------------------------------------------------

/// Plan scripted sayHelper for one AI hearer. `handled=false` → LLM may run.
// Haxe: AiBase.sayHelper L4755–4970
pub fn plan_scripted_say_helper(ctx: &ScriptedSayCtx) -> ScriptedSayPlan {
    let text = ctx.text.as_str();
    let upper = text.to_ascii_uppercase();
    let cooldown = scripted_cooldown_ok(ctx.last_react_sim_time, ctx.now_sim);

    // HOLA / HELLO / HI
    // Haxe: text.contains("HOLA") || text == "HELLO" || text == "HI"
    if upper.contains("HOLA") || upper == "HELLO" || upper == "HI" {
        if !cooldown {
            return ScriptedSayPlan {
                handled: true,
                ..Default::default()
            };
        }
        if ctx.speaker_holding_weapon {
            return ScriptedSayPlan {
                handled: true,
                say: Some("PUT DOWN YOUR WEAPON FIRST!".into()),
                ..Default::default()
            };
        }
        if ctx.ai_angry {
            return ScriptedSayPlan {
                handled: true,
                say: Some("DONT MAKE ME ANGRY!".into()),
                ..Default::default()
            };
        }
        if ctx.speaker_angry {
            return ScriptedSayPlan {
                handled: true,
                say: Some("YOU LOOK ANGRY!".into()),
                ..Default::default()
            };
        }
        return ScriptedSayPlan {
            handled: true,
            say: Some(format!("HOLA {}", ctx.speaker_name)),
            mark_reacted: true,
            waiting_time_add: HOLA_WAITING_TIME_ADD,
            stop_goto_self: true,
            ..Default::default()
        };
    }

    // NAME?
    if upper.starts_with("NAME?") {
        if !cooldown {
            return ScriptedSayPlan {
                handled: true,
                ..Default::default()
            };
        }
        if ctx.ai_angry {
            return ScriptedSayPlan {
                handled: true,
                say: Some("GRRR!".into()),
                ..Default::default()
            };
        }
        if ctx.speaker_holding_weapon {
            return ScriptedSayPlan {
                handled: true,
                say: Some("PUT DOWN YOUR WEAPON FIRST!".into()),
                ..Default::default()
            };
        }
        if ctx.speaker_angry {
            return ScriptedSayPlan {
                handled: true,
                say: Some("I DONT TRUST YOU!".into()),
                ..Default::default()
            };
        }
        return ScriptedSayPlan {
            handled: true,
            say: Some(format!("{} {}", ctx.ai_name, ctx.ai_family_name)),
            mark_reacted: true,
            waiting_time_add: HOLA_WAITING_TIME_ADD,
            stop_goto_self: true,
            ..Default::default()
        };
    }

    // ARE YOU AI
    if upper.contains("ARE YOU AI")
        || upper.contains("ARE YOU AN AI")
        || upper == "AI?"
        || upper == "AI"
    {
        if !cooldown {
            return ScriptedSayPlan {
                handled: true,
                ..Default::default()
            };
        }
        let reply = ARE_YOU_AI_REPLIES
            .get(ctx.rand_ai as usize)
            .map(|s| (*s).to_string());
        return ScriptedSayPlan {
            handled: true,
            say: reply,
            mark_reacted: true,
            ..Default::default()
        };
    }

    // NICE?
    if upper.starts_with("NICE?") {
        return ScriptedSayPlan {
            handled: true,
            say: Some(if ctx.is_nice_baby {
                "YES!".into()
            } else {
                "GRR!".into()
            }),
            ..Default::default()
        };
    }

    // JUMP!
    if upper == "JUMP!" {
        return ScriptedSayPlan {
            handled: true,
            say: Some("JUMP".into()),
            jump: true,
            ..Default::default()
        };
    }

    // MOVE!
    if upper.starts_with("MOVE!") {
        if let Some(deny) = plan_ally_gate(ctx.is_friendly) {
            return deny;
        }
        return ScriptedSayPlan {
            handled: true,
            say: Some("YES CAPTAIN".into()),
            goto_speaker_offset: true,
            ..Default::default()
        };
    }

    // NHOME!
    if upper.starts_with("NHOME!") {
        return ScriptedSayPlan {
            handled: true,
            say: Some(ctx.home_name.clone()),
            ..Default::default()
        };
    }

    // FOLLOW ME! / FOLLOW / COME  (before STOP FOLLOW)
    if upper.starts_with("FOLLOW ME!")
        || upper.starts_with("FOLLOW")
        || upper.starts_with("COME")
    {
        if let Some(deny) = plan_should_do_command(ctx.should_do_command) {
            return deny;
        }
        return ScriptedSayPlan {
            handled: true,
            say: Some("IM COMMING".into()),
            start_follow: true,
            goto_speaker_offset: true,
            set_auto_stop_follow: Some(false),
            follow_p_id: ctx.speaker_p_id,
            follow_started_sim_time: ctx.now_sim,
            ..Default::default()
        };
    }

    // STOP FOLLOW
    if upper.contains("STOP FOLLOW") {
        return ScriptedSayPlan {
            handled: true,
            say: Some("STOPED".into()),
            clear_follow: true,
            set_auto_stop_follow: Some(true),
            ..Default::default()
        };
    }

    // STOP / WAIT (ally)
    if upper.starts_with("STOP") || upper.starts_with("WAIT") {
        if let Some(deny) = plan_ally_gate(ctx.is_friendly) {
            return deny;
        }
        return ScriptedSayPlan {
            handled: true,
            say: Some("STOPING".into()),
            clear_follow: true,
            set_auto_stop_follow: Some(true),
            stop_goto_self: true,
            ordered_to_drop: true,
            // Haxe: waitingTime = 10 (assign)
            waiting_time_set: Some(STOP_WAITING_TIME),
            ..Default::default()
        };
    }

    // DROP (ally)
    // Haxe: doDropCommand → Goto(self) + orderedToDrop + waitingTime=1; drop next AI tick
    if upper.starts_with("DROP") {
        if let Some(deny) = plan_ally_gate(ctx.is_friendly) {
            return deny;
        }
        return ScriptedSayPlan {
            handled: true,
            say: Some("DROPING".into()),
            ordered_to_drop: true,
            // Deferred: live tick processes `ai_ordered_to_drop` (not immediate feet).
            do_drop_now: false,
            stop_goto_self: true,
            // Haxe: waitingTime = 1 (assign)
            waiting_time_set: Some(DROP_WAITING_TIME),
            ..Default::default()
        };
    }

    // GO HOME
    // Haxe: sayHelper GO HOME — near say + time+=5; far isMovingToHome + debug + time+=6
    if upper.contains("GO HOME") {
        if let Some(deny) = plan_should_do_command(ctx.should_do_command) {
            return deny;
        }
        if ctx.home_quad_dist < GO_HOME_NEAR_QUAD {
            return ScriptedSayPlan {
                handled: true,
                say: Some("I AM HOME!".into()),
                think_time_bump: GO_HOME_NEAR_TIME_BUMP,
                ..Default::default()
            };
        }
        return ScriptedSayPlan {
            handled: true,
            think_time_bump: GO_HOME_FAR_TIME_BUMP,
            move_to_home: true,
            ..Default::default()
        };
    }

    // HOME!
    if upper.starts_with("HOME!") {
        if let Some(deny) = plan_should_do_command(ctx.should_do_command) {
            return deny;
        }
        return ScriptedSayPlan {
            handled: true,
            search_new_home: true,
            ..Default::default()
        };
    }

    // MAKE / CRAFT (ally)
    if upper.starts_with("MAKE") || upper.starts_with("CRAFT") {
        if let Some(deny) = plan_ally_gate(ctx.is_friendly) {
            return deny;
        }
        return ScriptedSayPlan {
            handled: true,
            make_craft_text: Some(text.to_string()),
            ..Default::default()
        };
    }

    // DEBUG
    if upper.starts_with("DEBUG!") || upper.starts_with("DEBUG ON") {
        return ScriptedSayPlan {
            handled: true,
            say: Some("DEBUG ON".into()),
            set_debug_say: Some(true),
            ..Default::default()
        };
    }
    if upper.starts_with("DEBUG OFF") {
        return ScriptedSayPlan {
            handled: true,
            say: Some("DEBUG OFF".into()),
            set_debug_say: Some(false),
            ..Default::default()
        };
    }

    // PROF ON/OFF / PROFESSION?
    if upper.starts_with("PROF ON") {
        return ScriptedSayPlan {
            handled: true,
            say: Some("PROF ON".into()),
            set_debug_profession: Some(true),
            ..Default::default()
        };
    }
    if upper.starts_with("PROF OFF") {
        return ScriptedSayPlan {
            handled: true,
            say: Some("PROF OFF".into()),
            set_debug_profession: Some(false),
            ..Default::default()
        };
    }
    if upper.starts_with("PROFESSION?") || upper.starts_with("PROF?") {
        let t = create_profession_text(
            ctx.assigned_profession.as_deref(),
            ctx.last_profession.as_deref(),
        );
        return ScriptedSayPlan {
            handled: true,
            say: Some(t),
            ..Default::default()
        };
    }

    // PROFESSION! assign (ends with !)
    if upper.ends_with('!')
        && !upper.starts_with("JUMP")
        && !upper.starts_with("MOVE")
        && !upper.starts_with("HOME")
        && !upper.starts_with("NHOME")
        && !upper.starts_with("DEBUG")
        && !upper.starts_with("PROF")
        && !upper.starts_with("NAME")
        && !upper.starts_with("NICE")
        && !upper.starts_with("FOLLOW")
        && !upper.starts_with("STOP")
        && !upper.starts_with("WAIT")
        && !upper.starts_with("DROP")
        && !upper.starts_with("MAKE")
        && !upper.starts_with("CRAFT")
        && !upper.contains("GO HOME")
    {
        if let Some(deny) = plan_should_do_command(ctx.should_do_command) {
            return deny;
        }
        let prof_raw = upper.trim_end_matches('!').trim();
        let prof_part = prof_raw.split('!').next().unwrap_or(prof_raw).trim();
        match normalize_profession_token(prof_part) {
            Some(None) => {
                return ScriptedSayPlan {
                    handled: true,
                    set_assigned_profession: Some(None),
                    ..Default::default()
                };
            }
            Some(Some(p)) if profession_is_known(&p) => {
                return ScriptedSayPlan {
                    handled: true,
                    say: Some(p.clone()),
                    set_assigned_profession: Some(Some(p)),
                    ..Default::default()
                };
            }
            Some(Some(_)) | None => {
                return ScriptedSayPlan {
                    handled: true,
                    ..Default::default()
                };
            }
        }
    }

    // F / YOU ARE — Haxe TODO feed; still returns without LLM
    if upper == "F" || upper.starts_with("YOU ARE") {
        return ScriptedSayPlan {
            handled: true,
            ..Default::default()
        };
    }

    // Unhandled → LLM fallback may proceed
    ScriptedSayPlan {
        handled: false,
        ..Default::default()
    }
}

/// Apply waiting mutations onto current `waitingTime`.
///
/// Priority: direct assign (`waitingTime = N`) wins; else `+=` then max-floor.
// Haxe: waitingTime += 2 / waitingTime = 10 / setWaitingTimeMin
pub fn apply_scripted_waiting(current: f32, plan: &ScriptedSayPlan) -> f32 {
    // Haxe STOP/DROP: `waitingTime = N` overwrites (can lower).
    if let Some(set) = plan.waiting_time_set {
        return set;
    }
    let mut w = current;
    if plan.waiting_time_add > 0.0 {
        w = current + plan.waiting_time_add;
    }
    if let Some(min) = plan.waiting_time_min {
        if min > w {
            w = min;
        }
    }
    w
}

/// True when live should write `waiting_time_min` even if result is lower.
// Haxe: waitingTime = 10 assign can lower prior value
#[inline]
pub fn scripted_waiting_forces_write(plan: &ScriptedSayPlan) -> bool {
    plan.waiting_time_set.is_some()
        || plan.waiting_time_add > 0.0
        || plan.waiting_time_min.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_ctx(text: &str) -> ScriptedSayCtx {
        ScriptedSayCtx {
            text: text.into(),
            now_sim: 100.0,
            last_react_sim_time: 0.0,
            ai_name: "ALICE".into(),
            ai_family_name: "SNOW".into(),
            speaker_name: "BOB".into(),
            speaker_p_id: 7,
            ai_angry: false,
            speaker_angry: false,
            speaker_holding_weapon: false,
            is_friendly: true,
            should_do_command: true,
            is_nice_baby: true,
            assigned_profession: None,
            last_profession: Some("BAKER".into()),
            home_quad_dist: 100.0,
            home_name: "Hot Adobe Oven".into(),
            rand_ai: 1,
            debug_say: false,
            debug_profession: false,
        }
    }

    #[test]
    fn hola_friendly() {
        let p = plan_scripted_say_helper(&base_ctx("HOLA"));
        assert!(p.handled);
        assert_eq!(p.say.as_deref(), Some("HOLA BOB"));
        assert!(p.mark_reacted);
        assert!(p.stop_goto_self);
        assert!((p.waiting_time_add - 2.0).abs() < 1e-5);
    }

    #[test]
    fn hola_weapon_gate() {
        let mut c = base_ctx("HELLO");
        c.speaker_holding_weapon = true;
        let p = plan_scripted_say_helper(&c);
        assert_eq!(p.say.as_deref(), Some("PUT DOWN YOUR WEAPON FIRST!"));
        assert!(!p.mark_reacted);
    }

    #[test]
    fn hola_cooldown_silent() {
        let mut c = base_ctx("HI");
        c.last_react_sim_time = 99.0;
        c.now_sim = 100.0; // 1s < 4
        let p = plan_scripted_say_helper(&c);
        assert!(p.handled);
        assert!(p.say.is_none());
    }

    #[test]
    fn name_query() {
        let p = plan_scripted_say_helper(&base_ctx("NAME?"));
        assert_eq!(p.say.as_deref(), Some("ALICE SNOW"));
        assert!(p.mark_reacted);
    }

    #[test]
    fn are_you_ai_rand() {
        let mut c = base_ctx("ARE YOU AI");
        c.rand_ai = 2;
        let p = plan_scripted_say_helper(&c);
        assert_eq!(p.say.as_deref(), Some("No"));
        assert!(p.mark_reacted);
        c.rand_ai = 7; // silent slot
        let p2 = plan_scripted_say_helper(&c);
        assert!(p2.handled);
        assert!(p2.say.is_none());
    }

    #[test]
    fn nice_and_jump() {
        let p = plan_scripted_say_helper(&base_ctx("NICE?"));
        assert_eq!(p.say.as_deref(), Some("YES!"));
        let mut c = base_ctx("NICE?");
        c.is_nice_baby = false;
        assert_eq!(plan_scripted_say_helper(&c).say.as_deref(), Some("GRR!"));
        let j = plan_scripted_say_helper(&base_ctx("JUMP!"));
        assert!(j.jump);
        assert_eq!(j.say.as_deref(), Some("JUMP"));
    }

    #[test]
    fn follow_needs_command_gate() {
        let mut c = base_ctx("FOLLOW ME!");
        c.should_do_command = false;
        let p = plan_scripted_say_helper(&c);
        assert_eq!(p.say.as_deref(), Some(NOT_FOLLOWER_SAY));
        assert_eq!(p.emote_id, Some(LLM_SPEECH_ANGRY_EMOTE_ID));

        c.should_do_command = true;
        let p2 = plan_scripted_say_helper(&c);
        assert_eq!(p2.say.as_deref(), Some("IM COMMING"));
        assert!(p2.start_follow);
        assert_eq!(p2.follow_p_id, 7);
    }

    #[test]
    fn stop_follow_and_stop() {
        let p = plan_scripted_say_helper(&base_ctx("STOP FOLLOW"));
        assert_eq!(p.say.as_deref(), Some("STOPED"));
        assert!(p.clear_follow);

        let s = plan_scripted_say_helper(&base_ctx("STOP"));
        assert_eq!(s.say.as_deref(), Some("STOPING"));
        assert!(s.ordered_to_drop);
        assert_eq!(s.waiting_time_set, Some(10.0));
        assert!(s.waiting_time_min.is_none());

        let mut c = base_ctx("WAIT");
        c.is_friendly = false;
        let d = plan_scripted_say_helper(&c);
        assert_eq!(d.say.as_deref(), Some(LLM_NOT_ALLY_SAY));
    }

    #[test]
    fn drop_and_make() {
        let d = plan_scripted_say_helper(&base_ctx("DROP"));
        // Haxe: orderedToDrop deferred — not immediate feet drop
        assert!(!d.do_drop_now);
        assert!(d.ordered_to_drop);
        assert_eq!(d.waiting_time_set, Some(1.0));
        assert_eq!(d.say.as_deref(), Some("DROPING"));

        let m = plan_scripted_say_helper(&base_ctx("MAKE knife"));
        assert_eq!(m.make_craft_text.as_deref(), Some("MAKE knife"));
        assert!(m.handled);

        let mut c = base_ctx("CRAFT 71");
        c.is_friendly = false;
        assert_eq!(
            plan_scripted_say_helper(&c).say.as_deref(),
            Some(LLM_NOT_ALLY_SAY)
        );
    }

    #[test]
    fn go_home_near_far() {
        let mut c = base_ctx("GO HOME");
        c.home_quad_dist = 1.0;
        let p = plan_scripted_say_helper(&c);
        assert_eq!(p.say.as_deref(), Some("I AM HOME!"));
        assert!((p.think_time_bump - 5.0).abs() < 1e-5);
        assert!(!p.move_to_home);

        c.home_quad_dist = 50.0;
        let p2 = plan_scripted_say_helper(&c);
        assert!(p2.say.is_none());
        assert!((p2.think_time_bump - 6.0).abs() < 1e-5);
        assert!(p2.move_to_home);
    }

    #[test]
    fn debug_prof_assign() {
        let p = plan_scripted_say_helper(&base_ctx("DEBUG ON"));
        assert_eq!(p.set_debug_say, Some(true));
        let p2 = plan_scripted_say_helper(&base_ctx("PROF?"));
        assert_eq!(p2.say.as_deref(), Some("BAKER"));
        let a = plan_scripted_say_helper(&base_ctx("SMITH!"));
        assert_eq!(a.set_assigned_profession, Some(Some("SMITH".into())));
        assert_eq!(a.say.as_deref(), Some("SMITH"));
        let f = plan_scripted_say_helper(&base_ctx("FARMER!"));
        assert_eq!(
            f.set_assigned_profession,
            Some(Some("BASICFARMER".into()))
        );
        let n = plan_scripted_say_helper(&base_ctx("NONE!"));
        assert_eq!(n.set_assigned_profession, Some(None));
    }

    #[test]
    fn unhandled_for_llm() {
        let p = plan_scripted_say_helper(&base_ctx("what is the weather?"));
        assert!(!p.handled);
    }

    #[test]
    fn you_are_swallows() {
        let p = plan_scripted_say_helper(&base_ctx("YOU ARE COOL"));
        assert!(p.handled);
        assert!(p.say.is_none());
    }

    #[test]
    fn profession_text_compose() {
        assert_eq!(create_profession_text(None, None), "NONE");
        assert_eq!(
            create_profession_text(Some("SMITH"), Some("BAKER")),
            "SMITH doing BAKER"
        );
        assert_eq!(
            create_profession_text(Some("BAKER"), Some("BAKER")),
            "BAKER"
        );
    }

    #[test]
    fn waiting_apply() {
        // HOLA-style add only
        let add = ScriptedSayPlan {
            waiting_time_add: 2.0,
            ..Default::default()
        };
        assert!((apply_scripted_waiting(0.0, &add) - 2.0).abs() < 1e-5);
        assert!((apply_scripted_waiting(12.0, &add) - 14.0).abs() < 1e-5);

        // Floor min only
        let floor = ScriptedSayPlan {
            waiting_time_min: Some(10.0),
            ..Default::default()
        };
        assert!((apply_scripted_waiting(0.0, &floor) - 10.0).abs() < 1e-5);
        assert!((apply_scripted_waiting(12.0, &floor) - 12.0).abs() < 1e-5);

        // STOP assign can lower
        let stop = ScriptedSayPlan {
            waiting_time_set: Some(10.0),
            ..Default::default()
        };
        assert!((apply_scripted_waiting(20.0, &stop) - 10.0).abs() < 1e-5);
        assert!(scripted_waiting_forces_write(&stop));
        assert!(!scripted_waiting_forces_write(&ScriptedSayPlan::default()));
    }

    #[test]
    fn go_home_path_helpers() {
        assert_eq!(go_home_move_target(1, 2, None), (1, 2));
        assert_eq!(go_home_move_target(1, 2, Some((9, 8))), (9, 8));
        // max tiles 3 → max_q 9; close when < 9
        assert!(!should_path_to_home(4.0, 3));
        assert!(should_path_to_home(9.0, 3));
        assert!(should_path_to_home(50.0, 3));
        assert_eq!(go_home_debug_say(false, true), None);
        assert_eq!(go_home_debug_say(true, true), Some(GO_HOME_SAY_GOING));
        assert_eq!(go_home_debug_say(true, false), Some(GO_HOME_SAY_CANNOT));
        let (gx, gy) = go_home_goal_xy(10, 20, 0);
        assert!((gx - 10).abs() <= GO_HOME_STAND_HALF);
        assert!((gy - 20).abs() <= GO_HOME_STAND_HALF);
    }

    #[test]
    fn home_swamp_filter() {
        assert!(!home_oven_biome_allowed(false, HOME_SEARCH_SWAMP_BIOME));
        assert!(home_oven_biome_allowed(true, HOME_SEARCH_SWAMP_BIOME));
        assert!(home_oven_biome_allowed(false, 0)); // grassland ok
    }

    #[test]
    fn cooldown_helper() {
        assert!(scripted_cooldown_ok(0.0, 1.0));
        assert!(!scripted_cooldown_ok(10.0, 12.0));
        assert!(scripted_cooldown_ok(10.0, 15.0));
    }
}
