//! Haxe `AiBase.isHunting` (chunk **AI-JOB-HUNT**).
//!
//! `hasOrBecomeProfession('HUNTER', max)` then, if home quad < 400:
//! Knife 560 + Rattle Snake 764; Firebrand 248 + Mosquito 2157/2156.
//! Assigned/last `isHunting(100)`; mid `age > 14 && isHunting()`.
//!
//! No world I/O: live apply maps [`HuntingAction`] to shortCraft USE/seek.

/// Knife 560.
// Haxe: AiBase.isHunting ~5971
pub const HUNT_KNIFE: i32 = 560;
/// Rattle Snake 764.
pub const RATTLE_SNAKE: i32 = 764;
/// Firebrand 248.
pub const FIREBRAND: i32 = 248;
/// Mosquito Swarm just bit 2157.
pub const MOSQUITO_SWARM_JUST_BIT: i32 = 2157;
/// Mosquito Swarm 2156.
pub const MOSQUITO_SWARM: i32 = 2156;

/// Home quad gate (`CalculateQuadDistanceToObject(home) < 400`).
// Haxe: AiBase.isHunting ~5969
pub const HUNTING_HOME_QUAD: i32 = 400;
/// Haxe `shortCraft(..., 20)` search.
pub const HUNTING_SHORTCRAFT_RADIUS: i32 = 20;
/// Mid `isHunting()` default maxPeople.
pub const HUNTING_DEFAULT_MAX: i32 = 1;
/// Assigned/last `isHunting(100)`.
pub const HUNTING_ASSIGNED_MAX: i32 = 100;
/// Mid call `myPlayer.age > 14`.
// Haxe: doTimeStuffHelper ~655
pub const HUNTING_MID_MIN_AGE: f32 = 14.0;

/// Canonical Haxe profession string.
pub const HUNTER_PROFESSION_KEY: &str = "HUNTER";

/// Sticky last + assigned + weight for HUNTER.
// Haxe: AiBase.profession['HUNTER'] + lastProfession / assignedProfession
#[derive(Debug, Clone, PartialEq)]
pub struct HunterProfessionRuntime {
    pub is_last_hunter: bool,
    pub is_assigned_hunter: bool,
    /// Haxe `this.profession['HUNTER']` weight (0 idle / 1 active).
    pub weight: f32,
}

impl Default for HunterProfessionRuntime {
    fn default() -> Self {
        Self {
            is_last_hunter: false,
            is_assigned_hunter: false,
            weight: 0.0,
        }
    }
}

impl HunterProfessionRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear_weight(&mut self) {
        self.weight = 0.0;
    }

    pub fn wipe_on_eat(&mut self, last_was_foodserver: bool) {
        self.weight = 0.0;
        if !last_was_foodserver {
            self.is_last_hunter = false;
        }
    }
}

/// Parse speech / assigned tokens for hunter.
// Haxe: assignedProfession / lastProfession == 'HUNTER'
pub fn parse_hunter_profession_speech(text: &str) -> bool {
    let t = text.trim();
    let prof = t.strip_suffix('!').unwrap_or(t).trim();
    prof.eq_ignore_ascii_case("HUNTER") || prof.eq_ignore_ascii_case("HUNT")
}

/// Assign from speech `HUNTER!`.
pub fn assign_hunter_from_speech(runtime: &mut HunterProfessionRuntime, text: &str) -> bool {
    if !parse_hunter_profession_speech(text) {
        return false;
    }
    runtime.is_assigned_hunter = true;
    runtime.is_last_hunter = true;
    runtime.weight = 1.0;
    true
}

/// Assigned or last HUNTER job dispatch.
// Haxe: assignedProfession == 'HUNTER' || lastProfession == 'HUNTER'
pub fn resolve_hunter_assigned_job(runtime: &HunterProfessionRuntime) -> bool {
    runtime.is_assigned_hunter || runtime.is_last_hunter
}

/// Haxe `hasOrBecomeProfession('HUNTER', max)`.
// Haxe: AiBase.hasOrBecomeProfession ~4466
pub fn has_or_become_hunter(
    runtime: &mut HunterProfessionRuntime,
    max: i32,
    peer_count_with_last: f32,
    was_idle: f32,
) -> bool {
    if max < 0 {
        return true;
    }
    if runtime.is_last_hunter {
        return true;
    }
    let cap = max as f32 + was_idle.max(0.0);
    if peer_count_with_last.max(0.0) >= cap {
        return false;
    }
    runtime.weight = 1.0;
    runtime.is_last_hunter = true;
    true
}

/// World sensors for pure isHunting.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HuntingSensors {
    pub player_x: i32,
    pub player_y: i32,
    pub home_x: i32,
    pub home_y: i32,
    pub age: f32,
    /// Closest rattlesnake Chebyshev (or `i32::MAX` if none in r=20).
    pub snake_dist: i32,
    pub snake_x: i32,
    pub snake_y: i32,
    /// Mosquito swarm just-bit 2157.
    pub mosquito_bit_dist: i32,
    pub mosquito_bit_x: i32,
    pub mosquito_bit_y: i32,
    /// Mosquito swarm 2156.
    pub mosquito_dist: i32,
    pub mosquito_x: i32,
    pub mosquito_y: i32,
}

impl Default for HuntingSensors {
    fn default() -> Self {
        Self {
            player_x: 0,
            player_y: 0,
            home_x: 0,
            home_y: 0,
            age: 20.0,
            snake_dist: i32::MAX,
            snake_x: 0,
            snake_y: 0,
            mosquito_bit_dist: i32::MAX,
            mosquito_bit_x: 0,
            mosquito_bit_y: 0,
            mosquito_dist: i32::MAX,
            mosquito_x: 0,
            mosquito_y: 0,
        }
    }
}

/// Pure decision output for isHunting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HuntingAction {
    None,
    /// Haxe `shortCraft(actor, target, 20)`.
    ShortCraft { actor: i32, target: i32 },
}

impl HuntingAction {
    pub fn is_some(self) -> bool {
        !matches!(self, Self::None)
    }
}

#[inline]
fn home_quad(s: &HuntingSensors) -> i32 {
    let dx = s.player_x - s.home_x;
    let dy = s.player_y - s.home_y;
    dx * dx + dy * dy
}

/// Pure `isHunting` body after the profession gate.
// Haxe: AiBase.isHunting ~5967–5981
pub fn is_hunting(
    sensors: &HuntingSensors,
    hunter: &mut HunterProfessionRuntime,
    max_people: i32,
    peer_count: f32,
    was_idle: f32,
) -> HuntingAction {
    if !has_or_become_hunter(hunter, max_people, peer_count, was_idle) {
        return HuntingAction::None;
    }
    if home_quad(sensors) >= HUNTING_HOME_QUAD {
        return HuntingAction::None;
    }
    if sensors.snake_dist <= HUNTING_SHORTCRAFT_RADIUS {
        return HuntingAction::ShortCraft {
            actor: HUNT_KNIFE,
            target: RATTLE_SNAKE,
        };
    }
    if sensors.mosquito_bit_dist <= HUNTING_SHORTCRAFT_RADIUS {
        return HuntingAction::ShortCraft {
            actor: FIREBRAND,
            target: MOSQUITO_SWARM_JUST_BIT,
        };
    }
    if sensors.mosquito_dist <= HUNTING_SHORTCRAFT_RADIUS {
        return HuntingAction::ShortCraft {
            actor: FIREBRAND,
            target: MOSQUITO_SWARM,
        };
    }
    HuntingAction::None
}

/// Max people for isHunting from rung / assigned flag.
// Haxe: isHunting() / isHunting(100)
pub fn hunting_max_for_dispatch(is_assigned: bool, rung_label: &str) -> i32 {
    if is_assigned || rung_label == "ASSIGNED_JOB" {
        HUNTING_ASSIGNED_MAX
    } else {
        HUNTING_DEFAULT_MAX
    }
}

#[inline]
pub fn hunting_job_rung_label(rung_label: &str) -> bool {
    matches!(
        rung_label,
        "ASSIGNED_JOB" | "MID_PRIORITY_TASKS"
    )
}

/// Thin ladder bridge.
pub fn try_decide_hunting_from_rung(
    rung_label: &str,
    is_assigned_job: bool,
    sensors: &HuntingSensors,
    hunter: &mut HunterProfessionRuntime,
    peer_count: f32,
    was_idle: f32,
) -> Option<HuntingAction> {
    if !hunting_job_rung_label(rung_label) {
        return None;
    }
    if rung_label == "MID_PRIORITY_TASKS" && sensors.age <= HUNTING_MID_MIN_AGE {
        return None;
    }
    let assigned = is_assigned_job
        || resolve_hunter_assigned_job(hunter)
        || rung_label == "ASSIGNED_JOB";
    let max = hunting_max_for_dispatch(assigned, rung_label);
    Some(is_hunting(sensors, hunter, max, peer_count, was_idle))
}

/// Fill sensors from closest prey tiles (Chebyshev, Haxe GetClosestObjectById).
pub fn hunting_sensors_from_closest(
    player_x: i32,
    player_y: i32,
    home_x: i32,
    home_y: i32,
    age: f32,
    snake: Option<(i32, i32, i32)>,
    mosquito_bit: Option<(i32, i32, i32)>,
    mosquito: Option<(i32, i32, i32)>,
) -> HuntingSensors {
    let mut s = HuntingSensors {
        player_x,
        player_y,
        home_x,
        home_y,
        age,
        ..Default::default()
    };
    if let Some((d, x, y)) = snake {
        s.snake_dist = d;
        s.snake_x = x;
        s.snake_y = y;
    }
    if let Some((d, x, y)) = mosquito_bit {
        s.mosquito_bit_dist = d;
        s.mosquito_bit_x = x;
        s.mosquito_bit_y = y;
    }
    if let Some((d, x, y)) = mosquito {
        s.mosquito_dist = d;
        s.mosquito_x = x;
        s.mosquito_y = y;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near_home() -> HuntingSensors {
        HuntingSensors {
            player_x: 2,
            player_y: 0,
            home_x: 0,
            home_y: 0,
            age: 20.0,
            snake_dist: 5,
            snake_x: 5,
            snake_y: 0,
            ..Default::default()
        }
    }

    #[test]
    fn far_from_home_skips_prey() {
        let mut rt = HunterProfessionRuntime::default();
        rt.is_last_hunter = true;
        let mut s = near_home();
        s.player_x = 30;
        s.home_x = 0;
        let a = is_hunting(&s, &mut rt, 1, 0.0, 0.0);
        assert_eq!(a, HuntingAction::None);
        assert!(rt.is_last_hunter);
    }

    #[test]
    fn peer_cap_blocks_new_hunter() {
        let mut rt = HunterProfessionRuntime::default();
        let s = near_home();
        assert_eq!(
            is_hunting(&s, &mut rt, 1, 1.0, 0.0),
            HuntingAction::None
        );
        assert!(!rt.is_last_hunter);
    }

    #[test]
    fn last_hunter_kills_snake_before_mosquito() {
        let mut rt = HunterProfessionRuntime::default();
        rt.is_last_hunter = true;
        let mut s = near_home();
        s.mosquito_bit_dist = 3;
        s.mosquito_dist = 2;
        let a = is_hunting(&s, &mut rt, 1, 99.0, 0.0);
        assert_eq!(
            a,
            HuntingAction::ShortCraft {
                actor: HUNT_KNIFE,
                target: RATTLE_SNAKE,
            }
        );
    }

    #[test]
    fn mosquito_bit_before_plain_swarm() {
        let mut rt = HunterProfessionRuntime::default();
        rt.is_last_hunter = true;
        let mut s = near_home();
        s.snake_dist = i32::MAX;
        s.mosquito_bit_dist = 4;
        s.mosquito_dist = 1;
        let a = is_hunting(&s, &mut rt, 1, 0.0, 0.0);
        assert_eq!(
            a,
            HuntingAction::ShortCraft {
                actor: FIREBRAND,
                target: MOSQUITO_SWARM_JUST_BIT,
            }
        );
    }

    #[test]
    fn mosquito_swarm_when_no_snake() {
        let mut rt = HunterProfessionRuntime::default();
        rt.is_last_hunter = true;
        let mut s = near_home();
        s.snake_dist = i32::MAX;
        s.mosquito_dist = 8;
        let a = is_hunting(&s, &mut rt, 1, 0.0, 0.0);
        assert_eq!(
            a,
            HuntingAction::ShortCraft {
                actor: FIREBRAND,
                target: MOSQUITO_SWARM,
            }
        );
    }

    #[test]
    fn become_hunter_then_hunt() {
        let mut rt = HunterProfessionRuntime::default();
        let s = near_home();
        let a = is_hunting(&s, &mut rt, 1, 0.0, 0.0);
        assert!(rt.is_last_hunter);
        assert_eq!(rt.weight, 1.0);
        assert_eq!(
            a,
            HuntingAction::ShortCraft {
                actor: HUNT_KNIFE,
                target: RATTLE_SNAKE,
            }
        );
    }

    #[test]
    fn mid_age_gate() {
        let mut rt = HunterProfessionRuntime::default();
        rt.is_last_hunter = true;
        let mut s = near_home();
        s.age = 14.0;
        assert!(try_decide_hunting_from_rung(
            "MID_PRIORITY_TASKS",
            false,
            &s,
            &mut rt,
            0.0,
            0.0
        )
        .is_none());
        s.age = 14.1;
        let a = try_decide_hunting_from_rung(
            "MID_PRIORITY_TASKS",
            false,
            &s,
            &mut rt,
            0.0,
            0.0,
        );
        assert!(matches!(a, Some(HuntingAction::ShortCraft { .. })));
    }

    #[test]
    fn assigned_uses_max_100() {
        assert_eq!(
            hunting_max_for_dispatch(true, "ASSIGNED_JOB"),
            HUNTING_ASSIGNED_MAX
        );
        assert_eq!(
            hunting_max_for_dispatch(false, "MID_PRIORITY_TASKS"),
            HUNTING_DEFAULT_MAX
        );
    }

    #[test]
    fn speech_assign_hunter() {
        let mut r = HunterProfessionRuntime::default();
        assert!(parse_hunter_profession_speech("HUNTER!"));
        assert!(parse_hunter_profession_speech("hunt"));
        assert!(!parse_hunter_profession_speech("GRAVEKEEPER!"));
        assert!(assign_hunter_from_speech(&mut r, "HUNTER!"));
        assert!(r.is_assigned_hunter);
        assert!(r.is_last_hunter);
        assert_eq!(r.weight, 1.0);
    }

    #[test]
    fn prey_beyond_radius_idle() {
        let mut rt = HunterProfessionRuntime::default();
        rt.is_last_hunter = true;
        let mut s = near_home();
        s.snake_dist = 21;
        s.mosquito_bit_dist = 21;
        s.mosquito_dist = 21;
        assert_eq!(is_hunting(&s, &mut rt, 1, 0.0, 0.0), HuntingAction::None);
    }
}
