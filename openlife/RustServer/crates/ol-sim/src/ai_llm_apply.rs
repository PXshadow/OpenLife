//! Live apply of LLM `ApplyAiResponsePlan` (**AI-LLM-APPLY** / `llm_actions`).
//!
//! Haxe: `AiHandler.parseAiResponse` side-effects after HTTP reply:
//! `doEmote` / `startFollowingPlayer` / `doDropCommand` / `doMakeCraftCommand(..., true)`.
//!
//! Pure resolve helpers live here; sticky mutation is [`apply_sticky_from_plan`];
//! sim tick wires PE + DROP via `tick_llm_speech_wire`.
//!
//! Continuous follow walk pure helpers: [`ai_follow_walk`] (**AI-FOLLOW-WALK**).
//! Scripted sayHelper is crate-root [`crate::ai_say_helper`] (**AI-SAY-HELPER**).

use crate::ai_handler::{set_waiting_time_min, ApplyAiResponsePlan, AI_EMOTE_SECONDS, LlmSpeechRuntime};
use crate::craft_ai_sticky::PlayerCraftAi;

// Haxe: AiBase.isMovingToPlayer continuous follow (AI-FOLLOW-WALK / continuous_follow)
#[path = "ai_follow_walk.rs"]
pub mod ai_follow_walk;
pub use ai_follow_walk::*;

// Haxe: AiBase.sayHelper scripted cmds (AI-SAY-HELPER / scripted_cmds)
#[path = "ai_say_helper.rs"]
pub mod ai_say_helper;
pub use ai_say_helper::*;

// ---------------------------------------------------------------------------
// makeItem resolve (Haxe findObjectByCommand + bare name/id for LLM)
// ---------------------------------------------------------------------------

/// Haxe `findObjectByCommand` special name → id table.
// Haxe: GlobalPlayerInstance.findObjectByCommand HORSEX/PIE/BAKE/SHOE/ETERNAL
pub const MAKE_ITEM_ALIASES: &[(&str, i32)] = &[
    ("HORSEX", 779),   // Hitched Horse-Drawn Cart
    ("PIE", 265),      // Raw Berry Pie
    ("BAKE", 272),     // Cooked Berry Pie
    ("SHOE", 203),     // Rabbit Fur Shoe
    ("SHOES", 203),
    ("ETERNAL", 1407), // Fire Tut_only burns forever
];

/// Strip trailing `!` and return (search, ends_with flag).
// Haxe: findObjectByCommand `end` flag
pub fn normalize_make_item_search(raw: &str) -> (String, bool) {
    let mut s = raw.trim().to_string();
    let end = s.contains('!');
    s = s.replace('!', "");
    s = s.trim().to_string();
    (s, end)
}

/// Resolve alias table (exact uppercase match).
// Haxe: findObjectByCommand HORSEX / PIE / …
pub fn resolve_make_item_alias(search_upper: &str) -> Option<i32> {
    let u = search_upper.trim().to_ascii_uppercase();
    MAKE_ITEM_ALIASES
        .iter()
        .find(|(k, _)| *k == u)
        .map(|(_, id)| *id)
}

/// Haxe `ObjectData.GetObjectByName` order: exact → prefix/suffix → contains.
// Haxe: ObjectData.GetObjectByName
pub fn get_object_by_name_like<'a, I>(entries: I, search: &str, search_from_end: bool) -> Option<i32>
where
    I: IntoIterator<Item = (i32, &'a str)>,
{
    let needle = search.trim().to_ascii_uppercase();
    if needle.is_empty() {
        return None;
    }
    let mut list: Vec<(i32, String)> = entries
        .into_iter()
        .filter(|(id, _)| *id > 0)
        .map(|(id, n)| (id, n.to_ascii_uppercase()))
        .collect();
    list.sort_by_key(|(id, _)| *id);

    for (id, name) in &list {
        if name == &needle {
            return Some(*id);
        }
    }
    for (id, name) in &list {
        if search_from_end {
            if name.ends_with(&needle) {
                return Some(*id);
            }
        } else if name.starts_with(&needle) {
            return Some(*id);
        }
    }
    for (id, name) in &list {
        if name.contains(&needle) {
            return Some(*id);
        }
    }
    None
}

/// Extract search token from LLM `makeItem` / scripted MAKE/CRAFT text.
///
/// Bare `knife` is accepted (intentional delta vs Haxe `findObjectByCommand` needing ≥2 tokens).
// Haxe: findObjectByCommand split + bare LLM makeItem fix
pub fn make_item_search_token(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.is_empty() {
        return None;
    }
    let parts: Vec<&str> = t.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    let first_u = parts[0].to_ascii_uppercase();
    if first_u == "MAKE" || first_u == "CRAFT" {
        if parts.len() < 2 {
            return None;
        }
        let rest = t
            .split_once(char::is_whitespace)
            .map(|(_, r)| r.trim())
            .unwrap_or("")
            .to_string();
        if rest.is_empty() {
            return None;
        }
        return Some(rest);
    }
    Some(t.to_string())
}

/// Pure resolve of makeItem token → object id.
// Haxe: doMakeCraftCommand → findObjectByCommand → GetObjectByName
pub fn resolve_make_item_id(
    raw: &str,
    name_lookup: impl Fn(&str, bool) -> Option<i32>,
) -> Option<i32> {
    let token = make_item_search_token(raw)?;
    let (search, from_end) = normalize_make_item_search(&token);
    if search.is_empty() {
        return None;
    }
    if let Ok(id) = search.parse::<i32>() {
        if id > 0 {
            return Some(id);
        }
        return None;
    }
    let upper = search.to_ascii_uppercase();
    if let Some(id) = resolve_make_item_alias(&upper) {
        return Some(id);
    }
    name_lookup(&search, from_end)
}

// ---------------------------------------------------------------------------
// Sticky AI command state helpers (Haxe AiBase.playerToFollow / orderedToDrop)
// ---------------------------------------------------------------------------

/// Haxe `startFollowingPlayer` sticky mutation plan (no pathfind).
// Haxe: AiBase.startFollowingPlayer
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StartFollowPlan {
    pub follow_p_id: i32,
    pub auto_stop_follow: bool,
    pub follow_started_sim_time: f32,
    /// Clear force-stop so AI can walk (Haxe Goto).
    pub clear_force_stop: bool,
}

/// Build follow plan for speaker. Uses **speaker** p_id (product intent).
///
/// Haxe `parseAiResponse` incorrectly calls `startFollowingPlayer(aiPlayer)` (self).
/// Rust follows the human speaker — intentional delta.
// Haxe: AiHandler L489 startFollowingPlayer(aiPlayer) — bug; follow speaker
pub fn plan_start_following_player(speaker_p_id: i32, now_sim: f32) -> StartFollowPlan {
    StartFollowPlan {
        follow_p_id: speaker_p_id,
        auto_stop_follow: false,
        follow_started_sim_time: now_sim,
        clear_force_stop: true,
    }
}

/// Haxe `doDropCommand` sticky plan.
// Haxe: AiBase.doDropCommand Goto(self) + orderedToDrop + waitingTime=1
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropCommandPlan {
    pub ordered_to_drop: bool,
    pub waiting_time_min: f32,
    pub stop_movement: bool,
}

/// Pure drop-command plan after LLM `drop: true`.
// Haxe: AiBase.doDropCommand
pub fn plan_do_drop_command() -> DropCommandPlan {
    DropCommandPlan {
        ordered_to_drop: true,
        waiting_time_min: 1.0,
        stop_movement: true,
    }
}

/// Whether live apply should do anything (any flag set).
// Haxe: parseAiResponse emote/actions block
pub fn apply_plan_has_work(plan: &ApplyAiResponsePlan) -> bool {
    plan.emote_id.is_some()
        || plan.follow_player
        || plan.drop
        || plan
            .make_item
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
}

/// Result of applying sticky follow/drop/make (bookkeeping for tests).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AppliedAiResponseSticky {
    pub emote_id: Option<i32>,
    pub emote_seconds: i32,
    pub followed_p_id: Option<i32>,
    pub ordered_to_drop: bool,
    pub make_product_id: Option<i32>,
    pub make_name: Option<String>,
}

/// Apply sticky side-effects onto craft AI + follow/drop slots (no world DROP / PE).
// Haxe: parseAiResponse + startFollowingPlayer + doDrop + doMakeCraftCommand silent
#[allow(clippy::too_many_arguments)]
pub fn apply_sticky_from_plan(
    plan: &ApplyAiResponsePlan,
    speaker_p_id: i32,
    now_sim: f32,
    craft_ai: &mut PlayerCraftAi,
    ai_follow_p_id: &mut i32,
    ai_auto_stop_follow: &mut bool,
    ai_follow_started_sim_time: &mut f32,
    ai_ordered_to_drop: &mut bool,
    waiting_time_min: &mut f32,
    name_lookup: impl Fn(&str, bool) -> Option<(i32, String)>,
) -> AppliedAiResponseSticky {
    let mut out = AppliedAiResponseSticky {
        emote_seconds: AI_EMOTE_SECONDS,
        ..Default::default()
    };
    if let Some(eid) = plan.emote_id {
        out.emote_id = Some(eid);
    }
    if plan.follow_player && speaker_p_id > 0 {
        let fp = plan_start_following_player(speaker_p_id, now_sim);
        *ai_follow_p_id = fp.follow_p_id;
        *ai_auto_stop_follow = fp.auto_stop_follow;
        *ai_follow_started_sim_time = fp.follow_started_sim_time;
        out.followed_p_id = Some(fp.follow_p_id);
    }
    if plan.drop {
        let _dp = plan_do_drop_command();
        *ai_ordered_to_drop = true;
        apply_drop_waiting_floor(waiting_time_min);
        out.ordered_to_drop = true;
    }
    if let Some(ref raw) = plan.make_item {
        if let Some(token) = make_item_search_token(raw) {
            let (search, from_end) = normalize_make_item_search(&token);
            let resolved = if let Ok(id) = search.parse::<i32>() {
                if id > 0 {
                    Some((id, None))
                } else {
                    None
                }
            } else if let Some(id) = resolve_make_item_alias(&search.to_ascii_uppercase()) {
                Some((id, None))
            } else {
                name_lookup(&search, from_end).map(|(id, n)| (id, Some(n)))
            };
            if let Some((id, name_opt)) = resolved {
                let say = craft_ai.do_make_craft_command(id, name_opt.clone(), true);
                debug_assert!(say.is_none(), "silent make must not say");
                out.make_product_id = Some(id);
                out.make_name = name_opt.or_else(|| craft_ai.item_to_craft_name.clone());
            }
        }
    }
    out
}

/// Bump `waiting_time_min` floor for drop command (numeric pure).
// Haxe: doDropCommand waitingTime = 1
pub fn apply_drop_waiting_floor(waiting_time_min: &mut f32) {
    let w = plan_do_drop_command().waiting_time_min;
    if w > *waiting_time_min {
        *waiting_time_min = w;
    }
}

/// Bump LlmSpeechRuntime waiting for drop command.
// Haxe: doDropCommand waitingTime = 1
pub fn set_drop_waiting_on_runtime(rt: &mut LlmSpeechRuntime) {
    set_waiting_time_min(rt, plan_do_drop_command().waiting_time_min);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_handler::{parse_ai_response, plan_apply_parsed_ai_response};

    #[test]
    fn make_item_token_make_and_bare() {
        assert_eq!(
            make_item_search_token("MAKE knife").as_deref(),
            Some("knife")
        );
        assert_eq!(make_item_search_token("CRAFT 71").as_deref(), Some("71"));
        assert_eq!(make_item_search_token("knife").as_deref(), Some("knife"));
        assert_eq!(make_item_search_token("  71  ").as_deref(), Some("71"));
        assert!(make_item_search_token("MAKE").is_none());
        assert!(make_item_search_token("").is_none());
    }

    #[test]
    fn resolve_make_item_id_numeric_and_alias() {
        let none = |_: &str, _: bool| None;
        assert_eq!(resolve_make_item_id("71", none), Some(71));
        assert_eq!(resolve_make_item_id("MAKE 71", none), Some(71));
        assert_eq!(resolve_make_item_id("PIE", none), Some(265));
        assert_eq!(resolve_make_item_id("MAKE HORSEX", none), Some(779));
        assert_eq!(resolve_make_item_id("SHOES!", none), Some(203));
        assert_eq!(resolve_make_item_id("nope", none), None);
        assert_eq!(
            resolve_make_item_id("knife", |s, _| {
                if s.eq_ignore_ascii_case("knife") {
                    Some(560)
                } else {
                    None
                }
            }),
            Some(560)
        );
    }

    #[test]
    fn get_object_by_name_like_order() {
        let entries = [(10, "Steel Axe"), (5, "Axe"), (20, "Stone Axe")];
        assert_eq!(get_object_by_name_like(entries, "Axe", false), Some(5));
        assert_eq!(get_object_by_name_like(entries, "Steel", false), Some(10));
        assert_eq!(
            get_object_by_name_like([(99, "Flat Rock with Rabbit Bait")], "Rabbit", false),
            Some(99)
        );
    }

    #[test]
    fn plan_follow_and_drop() {
        let f = plan_start_following_player(42, 12.5);
        assert_eq!(f.follow_p_id, 42);
        assert!(!f.auto_stop_follow);
        assert!((f.follow_started_sim_time - 12.5).abs() < 1e-5);
        assert!(f.clear_force_stop);

        let d = plan_do_drop_command();
        assert!(d.ordered_to_drop);
        assert!((d.waiting_time_min - 1.0).abs() < 1e-5);
        assert!(d.stop_movement);
    }

    #[test]
    fn apply_sticky_from_full_plan() {
        let parsed = parse_ai_response(
            r#"{"text":"ok","emote":"happy","followPlayer":true,"drop":true,"makeItem":71}"#,
        );
        let plan = plan_apply_parsed_ai_response(&parsed);
        assert!(apply_plan_has_work(&plan));

        let mut craft = PlayerCraftAi::new();
        let mut follow = 0;
        let mut auto_stop = true;
        let mut started = 0.0;
        let mut ordered = false;
        let mut wait = 0.0;

        let applied = apply_sticky_from_plan(
            &plan,
            99,
            50.0,
            &mut craft,
            &mut follow,
            &mut auto_stop,
            &mut started,
            &mut ordered,
            &mut wait,
            |_, _| None,
        );
        assert_eq!(applied.emote_id, Some(0));
        assert_eq!(applied.followed_p_id, Some(99));
        assert_eq!(follow, 99);
        assert!(!auto_stop);
        assert!(ordered);
        assert!(applied.ordered_to_drop);
        assert_eq!(applied.make_product_id, Some(71));
        assert_eq!(craft.item_to_craft_id, 71);
        assert!((wait - 1.0).abs() < 1e-5);
    }

    #[test]
    fn apply_plan_empty_no_work() {
        let plan = plan_apply_parsed_ai_response(&parse_ai_response(r#"{"text":"hi"}"#));
        assert!(!apply_plan_has_work(&plan));
    }
}
