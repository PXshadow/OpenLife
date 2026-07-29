//! Dark Nosaj / Tarr Monument USE side-effects (Haxe TransitionHelper).
//!
//! Chunk **DARK-NOSAJ** / `dark_nosaj_use`:
//! - Tarr Monument **3112**: clear `darkNosaj` (+ yum / lost combat) or first Praise Jinbaili
//! - Dark Nosaj **2466** empty-hand: set `darkNosaj=1` (+ curse / yum / lost) or punish praised
//!
//! Haxe: `TransitionHelper.doCommandHelper` L144–185; fields on `GlobalPlayerInstance`.

use std::sync::Mutex;

/// Haxe object parent id: Dark Nosaj monument.
// Haxe: TransitionHelper L166–167 Dark Nosaj 2466
pub const DARK_NOSAJ_MONUMENT_ID: i32 = 2466;
/// Haxe object parent id: Tarr Monument (clears dark minion).
// Haxe: TransitionHelper L144–145 Tarr Monument 3112
pub const TARR_MONUMENT_ID: i32 = 3112;

/// CU level 0 clear word (Haxe `SendCurseToAll(player, 0, '_')`).
pub const CURSE_CLEAR_WORD: &str = "_";
/// CU level 1 dark-minion tag (Haxe `SendCurseToAll(player, 1, 'DARK_MINION')`).
pub const CURSE_DARK_MINION_WORD: &str = "DARK_MINION";

/// Pending public say + optional CU from monument USE (taken by USE intent handler).
static LAST_MONUMENT_FEEDBACK: Mutex<Option<MonumentFeedback>> = Mutex::new(None);

/// Wire feedback for one monument USE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonumentFeedback {
    pub conn_id: u64,
    /// Public `player.say` text (Haxe non-`toSelf`).
    pub say: &'static str,
    /// Optional CU: `(level, optional word)`.
    pub curse: Option<(i32, Option<&'static str>)>,
}

/// Record feedback for the USE intent path to broadcast.
pub fn note_monument_feedback(fb: MonumentFeedback) {
    if let Ok(mut g) = LAST_MONUMENT_FEEDBACK.lock() {
        *g = Some(fb);
    }
}

/// Take and clear pending monument feedback.
pub fn take_monument_feedback() -> Option<MonumentFeedback> {
    LAST_MONUMENT_FEEDBACK.lock().ok().and_then(|mut g| g.take())
}

/// Planned player mutations after monument USE (pure).
#[derive(Debug, Clone, PartialEq)]
pub struct MonumentUsePlan {
    pub say: &'static str,
    pub curse: Option<(i32, Option<&'static str>)>,
    pub dark_nosaj: f32,
    pub praised_jinbali: bool,
    /// Delta to Haxe `yum_multiplier` (prestige).
    pub yum_delta: f32,
    /// Delta to Haxe `lostCombatPrestige` (before floor-at-0 on clear path).
    pub lost_combat_delta: f32,
    /// Floor lost combat at 0 after apply (Tarr clear path).
    pub lost_combat_floor_zero: bool,
    /// Hits to add (Dark Nosaj punish praised path).
    pub hits_delta: f32,
}

/// Pure plan for Tarr (3112) / Dark Nosaj (2466) USE side-effects.
///
/// Returns `None` when the target is not a handled monument, or Dark Nosaj with non-empty hands.
// Haxe: TransitionHelper.doCommandHelper L144–185
pub fn plan_monument_use(
    target_parent_id: i32,
    held_parent_id: i32,
    dark_nosaj: f32,
    praised_jinbali: bool,
) -> Option<MonumentUsePlan> {
    if target_parent_id == TARR_MONUMENT_ID {
        return Some(plan_tarr(dark_nosaj, praised_jinbali));
    }
    if target_parent_id == DARK_NOSAJ_MONUMENT_ID && held_parent_id == 0 {
        return Some(plan_dark_nosaj(dark_nosaj, praised_jinbali));
    }
    None
}

fn plan_tarr(dark_nosaj: f32, praised_jinbali: bool) -> MonumentUsePlan {
    // Haxe: if player.darkNosaj > 0 → clear path
    if dark_nosaj.is_finite() && dark_nosaj > 0.0 {
        MonumentUsePlan {
            say: "Jasoniah is the one true god!",
            curse: Some((0, Some(CURSE_CLEAR_WORD))),
            dark_nosaj: 0.0,
            praised_jinbali,
            yum_delta: 90.0,
            lost_combat_delta: -90.0,
            lost_combat_floor_zero: true,
            hits_delta: 0.0,
        }
    } else {
        // Haxe: Praise Jinbaili — first time +5 yum + flag
        let first = !praised_jinbali;
        MonumentUsePlan {
            say: "Praise Jinbaili!",
            curse: None,
            dark_nosaj: if dark_nosaj.is_finite() {
                dark_nosaj
            } else {
                0.0
            },
            praised_jinbali: true,
            yum_delta: if first { 5.0 } else { 0.0 },
            lost_combat_delta: 0.0,
            lost_combat_floor_zero: false,
            hits_delta: 0.0,
        }
    }
}

fn plan_dark_nosaj(dark_nosaj: f32, praised_jinbali: bool) -> MonumentUsePlan {
    if praised_jinbali {
        // Haxe: reverse praise — −5 yum, +10 hits
        MonumentUsePlan {
            say: "AAAAAAAAAAAAAAAAAAAAaaaa!!!",
            curse: None,
            dark_nosaj: if dark_nosaj.is_finite() {
                dark_nosaj
            } else {
                0.0
            },
            praised_jinbali: false,
            yum_delta: -5.0,
            lost_combat_delta: 0.0,
            lost_combat_floor_zero: false,
            hits_delta: 10.0,
        }
    } else {
        // Haxe: All hail — CU DARK_MINION; if darkNosaj < 1 set to 1 + prestige hit
        let already = dark_nosaj.is_finite() && dark_nosaj >= 1.0;
        MonumentUsePlan {
            say: "All hail dark nosaj",
            curse: Some((1, Some(CURSE_DARK_MINION_WORD))),
            dark_nosaj: if already {
                dark_nosaj
            } else {
                1.0
            },
            praised_jinbali: false,
            yum_delta: if already { 0.0 } else { -100.0 },
            lost_combat_delta: if already { 0.0 } else { 100.0 },
            lost_combat_floor_zero: false,
            hits_delta: 0.0,
        }
    }
}

/// Apply lost-combat delta with optional floor-at-0 (pure).
pub fn apply_lost_combat_delta(before: f32, delta: f32, floor_zero: bool) -> f32 {
    let b = if before.is_finite() { before } else { 0.0 };
    let d = if delta.is_finite() { delta } else { 0.0 };
    let mut v = b + d;
    if floor_zero && v < 0.0 {
        v = 0.0;
    }
    v
}

/// Format CU wire with optional persistent name tag (Haxe `SendCurseToAll`).
// Haxe: Connection.SendCurseToAll → CURSED = "CU"
pub fn format_cursed_message_word(p_id: i32, level: i32, word: Option<&str>) -> String {
    match word {
        Some(w) if !w.is_empty() => format!("CU\n{p_id} {level} {w}\n#"),
        _ => format!("CU\n{p_id} {level}\n#"),
    }
}

/// Haxe DoDamage: `damage *= attacker.darkNosaj > 0 ? 1.2 : 1`.
// Haxe: GlobalPlayerInstance.DoDamage L4631
pub const DARK_NOSAJ_ATTACK_DAMAGE_MUL: f32 = 1.2;

/// Multiplier applied to outgoing attack damage when attacker is a dark minion.
// Haxe: GlobalPlayerInstance.DoDamage L4631
#[inline]
pub fn dark_nosaj_attack_damage_mul(dark_nosaj: f32) -> f32 {
    if dark_nosaj.is_finite() && dark_nosaj > 0.0 {
        DARK_NOSAJ_ATTACK_DAMAGE_MUL
    } else {
        1.0
    }
}

/// Haxe `addHealthAndPrestige` early-return when `darkNosaj > 0`.
// Haxe: GlobalPlayerInstance.addHealthAndPrestige L5997–5998
#[inline]
pub fn blocks_health_and_prestige(dark_nosaj: f32) -> bool {
    dark_nosaj.is_finite() && dark_nosaj > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_tarr_clears_dark_nosaj() {
        let p = plan_monument_use(TARR_MONUMENT_ID, 0, 1.0, false).unwrap();
        assert_eq!(p.say, "Jasoniah is the one true god!");
        assert_eq!(p.curse, Some((0, Some(CURSE_CLEAR_WORD))));
        assert_eq!(p.dark_nosaj, 0.0);
        assert_eq!(p.yum_delta, 90.0);
        assert_eq!(p.lost_combat_delta, -90.0);
        assert!(p.lost_combat_floor_zero);
        assert_eq!(
            apply_lost_combat_delta(50.0, -90.0, true),
            0.0
        );
        assert_eq!(apply_lost_combat_delta(120.0, -90.0, true), 30.0);
    }

    #[test]
    fn plan_tarr_praise_first_then_noop_yum() {
        let first = plan_monument_use(TARR_MONUMENT_ID, 5, 0.0, false).unwrap();
        assert_eq!(first.say, "Praise Jinbaili!");
        assert!(first.praised_jinbali);
        assert_eq!(first.yum_delta, 5.0);
        assert!(first.curse.is_none());

        let again = plan_monument_use(TARR_MONUMENT_ID, 0, 0.0, true).unwrap();
        assert_eq!(again.yum_delta, 0.0);
        assert!(again.praised_jinbali);
    }

    #[test]
    fn plan_dark_nosaj_set_requires_empty_hand() {
        assert!(plan_monument_use(DARK_NOSAJ_MONUMENT_ID, 1, 0.0, false).is_none());
        let p = plan_monument_use(DARK_NOSAJ_MONUMENT_ID, 0, 0.0, false).unwrap();
        assert_eq!(p.say, "All hail dark nosaj");
        assert_eq!(p.curse, Some((1, Some(CURSE_DARK_MINION_WORD))));
        assert_eq!(p.dark_nosaj, 1.0);
        assert_eq!(p.yum_delta, -100.0);
        assert_eq!(p.lost_combat_delta, 100.0);
    }

    #[test]
    fn plan_dark_nosaj_already_set_skips_prestige() {
        let p = plan_monument_use(DARK_NOSAJ_MONUMENT_ID, 0, 1.5, false).unwrap();
        assert_eq!(p.dark_nosaj, 1.5);
        assert_eq!(p.yum_delta, 0.0);
        assert_eq!(p.lost_combat_delta, 0.0);
        assert_eq!(p.curse, Some((1, Some(CURSE_DARK_MINION_WORD))));
    }

    #[test]
    fn plan_dark_nosaj_punishes_praised() {
        let p = plan_monument_use(DARK_NOSAJ_MONUMENT_ID, 0, 0.0, true).unwrap();
        assert_eq!(p.say, "AAAAAAAAAAAAAAAAAAAAaaaa!!!");
        assert!(!p.praised_jinbali);
        assert_eq!(p.yum_delta, -5.0);
        assert_eq!(p.hits_delta, 10.0);
        assert!(p.curse.is_none());
    }

    #[test]
    fn format_cu_with_and_without_word() {
        assert_eq!(
            format_cursed_message_word(7, 1, Some("DARK_MINION")),
            "CU\n7 1 DARK_MINION\n#"
        );
        assert_eq!(
            format_cursed_message_word(7, 0, Some("_")),
            "CU\n7 0 _\n#"
        );
        assert_eq!(format_cursed_message_word(7, 1, None), "CU\n7 1\n#");
    }

    #[test]
    fn note_take_feedback_roundtrip() {
        let _ = take_monument_feedback();
        note_monument_feedback(MonumentFeedback {
            conn_id: 9,
            say: "All hail dark nosaj",
            curse: Some((1, Some(CURSE_DARK_MINION_WORD))),
        });
        let fb = take_monument_feedback().unwrap();
        assert_eq!(fb.conn_id, 9);
        assert_eq!(fb.say, "All hail dark nosaj");
        assert!(take_monument_feedback().is_none());
    }

    /// Haxe DoDamage darkNosaj damage ×1.2.
    // Haxe: GlobalPlayerInstance.DoDamage L4631
    #[test]
    fn dark_nosaj_attack_damage_mul_gate() {
        assert!((dark_nosaj_attack_damage_mul(0.0) - 1.0).abs() < 1e-6);
        assert!((dark_nosaj_attack_damage_mul(-1.0) - 1.0).abs() < 1e-6);
        assert!((dark_nosaj_attack_damage_mul(0.1) - 1.2).abs() < 1e-6);
        assert!((dark_nosaj_attack_damage_mul(1.0) - 1.2).abs() < 1e-6);
        assert!(!dark_nosaj_attack_damage_mul(f32::NAN).is_nan());
        assert!((dark_nosaj_attack_damage_mul(f32::NAN) - 1.0).abs() < 1e-6);
    }

    /// Haxe addHealthAndPrestige early-return when dark.
    // Haxe: GlobalPlayerInstance.addHealthAndPrestige L5997–5998
    #[test]
    fn blocks_health_and_prestige_gate() {
        assert!(!blocks_health_and_prestige(0.0));
        assert!(!blocks_health_and_prestige(-0.5));
        assert!(blocks_health_and_prestige(0.01));
        assert!(blocks_health_and_prestige(1.0));
        assert!(!blocks_health_and_prestige(f32::NAN));
    }
}
