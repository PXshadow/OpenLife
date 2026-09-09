//! Haxe `AiBase.handleTemperature` (chunk **AI-HANDLE-TEMP**).
//!
//! Drink / GetOrCraft water, close biome, firePlace heat, sticky arrive/relax,
//! fail-cool / fail-warm (kindling + `isHandlingFire(2)`). Superbad remembered
//! place goto stays in [`crate::plan_temp_place_goto`].
//!
//! No world I/O: live apply maps [`HandleTemperatureAction`] to SELF / craft / Goto.

use crate::clothing_transitions::{WATER_BOWL_ID, WATER_POUCH_ID};
use crate::fire_food_profession::{FIRE, HOT_COALS, LARGE_FAST_FIRE};
pub use crate::fire_food_profession::LARGE_FAST_FIRE as HANDLE_TEMP_LARGE_FAST_FIRE;
use crate::map_temp_player::torus_quad_distance;
use ol_world::{DESERT, PASSABLE_RIVER};

/// Haxe `BiomeTag.SNOW` / `JUNGLE` (not re-exported from ol_world root).
const BIOME_SNOW: u8 = 4;
const BIOME_JUNGLE: u8 = 6;

/// Haxe `GetCloseBiome(..., searchDistance = 30)`.
// Haxe: AiHelper.GetCloseBiome
pub const GET_CLOSE_BIOME_DIST: i32 = 30;
/// Haxe `quadDistance < 2` → arrived.
// Haxe: AiBase.handleTemperature ~1708
pub const HANDLE_TEMP_ARRIVE_QUAD: i32 = 2;
/// Haxe `heat > 0.7` drink/craft water.
pub const HANDLE_TEMP_DRINK_HEAT: f32 = 0.7;
/// Sticky cooling while `heat > 0.6`.
pub const HANDLE_TEMP_STICKY_COOL: f32 = 0.6;
/// Sticky warming while `heat < 0.4`.
pub const HANDLE_TEMP_STICKY_WARM: f32 = 0.4;
/// Fail-cool: `heat > 0.5` and ambient `lastTemperature > 0.45`.
pub const HANDLE_TEMP_FAIL_COOL_HEAT: f32 = 0.5;
pub const HANDLE_TEMP_FAIL_COOL_AMBIENT: f32 = 0.45;
/// Fail-warm: `heat < 0.5` and ambient `lastTemperature < 0.55`.
pub const HANDLE_TEMP_FAIL_WARM_HEAT: f32 = 0.5;
pub const HANDLE_TEMP_FAIL_WARM_AMBIENT: f32 = 0.55;
/// Kindling / `isHandlingFire(2)` only when `age > 5`.
pub const HANDLE_TEMP_KINDLING_AGE: f32 = 5.0;
/// FirePlace `objectData.heatValue > 4`.
pub const HANDLE_TEMP_FIRE_HEAT_MIN: f32 = 4.0;
/// Haxe `this.time += 3` first arrival / relax.
pub const HANDLE_TEMP_RELAX_TIME: f32 = 3.0;
/// Kindling 72.
pub const HANDLE_TEMP_KINDLING: i32 = 72;
/// Haxe `myPlayer.say('lets drink')`.
pub const HANDLE_TEMP_SAY_DRINK: &str = "lets drink";

/// Cooling biomes: snow + passable river.
pub const COOL_BIOMES: [u8; 2] = [BIOME_SNOW, PASSABLE_RIVER];
/// Warming biomes: desert + jungle.
pub const WARM_BIOMES: [u8; 2] = [DESERT, BIOME_JUNGLE];

/// Planner output after needWarming / needCooling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HandleTemperatureAction {
    /// No need, or no goodPlace — Haxe returns false.
    Idle,
    /// Sticky `itemToCraftId == 83`.
    CraftLargeFastFire,
    /// Held bowl 382 / pouch 210 and heat > 0.7.
    DrinkSelf,
    /// `GetOrCraftItem(210)` then `GetOrCraftItem(382)`.
    GetOrCraftWater,
    /// First arrival or still helping — `time += 3`.
    Relax,
    /// `gotoObj(goodPlace)`.
    Goto { x: i32, y: i32 },
    /// Fail-warm: `shortCraftOnTarget(72, firePlace, false)`.
    KindlingOnFire { x: i32, y: i32 },
    /// Fail-warm: `isHandlingFire(2)`.
    HandlingFire,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandleTemperaturePlan {
    pub action: HandleTemperatureAction,
    pub is_handling: bool,
    pub just_arrived: bool,
    pub last_heat: f32,
    pub clear_cold_place: bool,
    pub clear_warm_place: bool,
    pub time_bump: f32,
    pub say: Option<&'static str>,
}

impl HandleTemperaturePlan {
    pub fn idle(last_heat: f32) -> Self {
        Self {
            action: HandleTemperatureAction::Idle,
            is_handling: false,
            just_arrived: false,
            last_heat,
            clear_cold_place: false,
            clear_warm_place: false,
            time_bump: 0.0,
            say: None,
        }
    }
}

/// Inputs gathered by live apply (no I/O in the planner).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandleTemperatureInput {
    pub heat: f32,
    /// Haxe `AiBase.lastTemperature` (previous body heat).
    pub last_heat: f32,
    /// Haxe `GlobalPlayerInstance.lastTemperature` (ambient).
    pub player_last_temperature: f32,
    pub is_handling: bool,
    pub just_arrived: bool,
    pub is_super_hot: bool,
    pub is_super_cold: bool,
    pub winter_female_with_kids: bool,
    pub held_id: i32,
    pub item_to_craft_id: i32,
    /// `(x, y, parent_id, heat_value)` when firePlace is set.
    pub fire_x: i32,
    pub fire_y: i32,
    pub fire_parent: i32,
    pub fire_heat_value: f32,
    pub has_fire_place: bool,
    pub close_cool: Option<(i32, i32)>,
    pub close_warm: Option<(i32, i32)>,
    pub cold_place: Option<(i32, i32)>,
    pub warm_place: Option<(i32, i32)>,
    pub px: i32,
    pub py: i32,
    pub age: f32,
    /// Live: GetOrCraft 210/382 already missed this tick — continue to biome.
    pub skip_water: bool,
}

impl Default for HandleTemperatureInput {
    fn default() -> Self {
        Self {
            heat: 0.5,
            last_heat: 0.5,
            player_last_temperature: 0.5,
            is_handling: false,
            just_arrived: false,
            is_super_hot: false,
            is_super_cold: false,
            winter_female_with_kids: false,
            held_id: 0,
            item_to_craft_id: -1,
            fire_x: 0,
            fire_y: 0,
            fire_parent: 0,
            fire_heat_value: 0.0,
            has_fire_place: false,
            close_cool: None,
            close_warm: None,
            cold_place: None,
            warm_place: None,
            px: 0,
            py: 0,
            age: 20.0,
            skip_water: false,
        }
    }
}

#[inline]
pub fn need_cooling(is_super_hot: bool, is_handling: bool, heat: f32) -> bool {
    is_super_hot || (is_handling && heat > HANDLE_TEMP_STICKY_COOL)
}

#[inline]
pub fn need_warming(
    is_super_cold: bool,
    is_handling: bool,
    heat: f32,
    winter_female_with_kids: bool,
) -> bool {
    is_super_cold || (is_handling && heat < HANDLE_TEMP_STICKY_WARM) || winter_female_with_kids
}

#[inline]
pub fn is_water_held(held_id: i32) -> bool {
    held_id == WATER_BOWL_ID || held_id == WATER_POUCH_ID
}

#[inline]
fn fire_place_xy(inp: &HandleTemperatureInput) -> Option<(i32, i32)> {
    if inp.has_fire_place {
        Some((inp.fire_x, inp.fire_y))
    } else {
        None
    }
}

fn quad_to(px: i32, py: i32, tx: i32, ty: i32) -> i32 {
    let dx = tx - px;
    let dy = ty - py;
    dx * dx + dy * dy
}

/// Haxe `AiHelper.GetCloseBiome` — closest matching biome tile; `None` if none found.
///
/// Haxe returns an empty ObjectHelper when nothing matches (`bestDistance < 0` never
/// fires). Intended `if (goodPlace == null)` fallback uses cold/warm place — we
/// return `None` when no biome tile is found.
// Haxe: AiHelper.GetCloseBiome ~499
pub fn get_close_biome(
    px: i32,
    py: i32,
    biomes: &[u8],
    tiles: &[(i32, i32, u8)],
    not_reachable: impl Fn(i32, i32) -> bool,
    map_w: i32,
    map_h: i32,
    wrap: bool,
) -> Option<(i32, i32)> {
    let mut best: Option<(i32, i32)> = None;
    let mut best_d = (GET_CLOSE_BIOME_DIST as f32) * 1000.0;
    for &(tx, ty, biome) in tiles {
        if !biomes.contains(&biome) {
            continue;
        }
        if not_reachable(tx, ty) {
            continue;
        }
        let d = torus_quad_distance(px, py, tx, ty, map_w, map_h, wrap);
        if d > best_d {
            continue;
        }
        best_d = d;
        best = Some((tx, ty));
    }
    best
}

/// Haxe `countLittleKids` — living kids with age ≤ MinAgeToEat.
// Haxe: GlobalPlayerInstance.countLittleKids ~1609
pub fn count_little_kids(
    parent_p_id: i32,
    min_age_to_eat: f32,
    kids: &[(i32, f32, bool, Option<i32>, Option<i32>)],
) -> i32 {
    let mut n = 0;
    for &(id, age, deleted, mother, father) in kids {
        if deleted || id == parent_p_id {
            continue;
        }
        if age > min_age_to_eat {
            continue;
        }
        if mother == Some(parent_p_id) || father == Some(parent_p_id) {
            n += 1;
        }
    }
    n
}

/// Full pure `handleTemperature` body.
// Haxe: AiBase.handleTemperature ~1645–1765
pub fn plan_handle_temperature(inp: HandleTemperatureInput) -> HandleTemperaturePlan {
    let last_heat = inp.heat;
    let tmp_last = inp.last_heat;
    let mut need_warm = need_warming(
        inp.is_super_cold,
        inp.is_handling,
        inp.heat,
        inp.winter_female_with_kids,
    );
    let need_cool = need_cooling(inp.is_super_hot, inp.is_handling, inp.heat);
    // Winter kids forces warming even after the first assignment (Haxe overwrites).
    if inp.winter_female_with_kids {
        need_warm = true;
    }

    if inp.item_to_craft_id == LARGE_FAST_FIRE {
        return HandleTemperaturePlan {
            action: HandleTemperatureAction::CraftLargeFastFire,
            is_handling: inp.is_handling,
            just_arrived: inp.just_arrived,
            last_heat,
            clear_cold_place: false,
            clear_warm_place: false,
            time_bump: 0.0,
            say: None,
        };
    }

    let mut good: Option<(i32, i32)> = None;
    if need_cool {
        if !inp.skip_water && inp.heat > HANDLE_TEMP_DRINK_HEAT {
            if is_water_held(inp.held_id) {
                return HandleTemperaturePlan {
                    action: HandleTemperatureAction::DrinkSelf,
                    is_handling: inp.is_handling,
                    just_arrived: inp.just_arrived,
                    last_heat,
                    clear_cold_place: false,
                    clear_warm_place: false,
                    time_bump: 0.0,
                    say: Some(HANDLE_TEMP_SAY_DRINK),
                };
            }
            return HandleTemperaturePlan {
                action: HandleTemperatureAction::GetOrCraftWater,
                is_handling: inp.is_handling,
                just_arrived: inp.just_arrived,
                last_heat,
                clear_cold_place: false,
                clear_warm_place: false,
                time_bump: 0.0,
                say: None,
            };
        }
        good = inp.close_cool.or(inp.cold_place);
    } else if need_warm {
        if inp.has_fire_place && inp.fire_heat_value > HANDLE_TEMP_FIRE_HEAT_MIN {
            good = fire_place_xy(&inp);
        }
    }

    if good.is_none() && need_warm {
        good = inp.close_warm.or(inp.warm_place);
    }

    let Some((gx, gy)) = good else {
        return HandleTemperaturePlan::idle(last_heat);
    };

    let quad = quad_to(inp.px, inp.py, gx, gy);
    if quad < HANDLE_TEMP_ARRIVE_QUAD {
        if !inp.just_arrived {
            return HandleTemperaturePlan {
                action: HandleTemperatureAction::Relax,
                is_handling: true,
                just_arrived: true,
                last_heat,
                clear_cold_place: false,
                clear_warm_place: false,
                time_bump: HANDLE_TEMP_RELAX_TIME,
                say: None,
            };
        }
        if inp.heat > HANDLE_TEMP_FAIL_COOL_HEAT
            && inp.player_last_temperature > HANDLE_TEMP_FAIL_COOL_AMBIENT
            && tmp_last <= inp.heat
        {
            return HandleTemperaturePlan {
                action: HandleTemperatureAction::Idle,
                is_handling: true,
                just_arrived: true,
                last_heat,
                clear_cold_place: true,
                clear_warm_place: false,
                time_bump: 0.0,
                say: None,
            };
        }
        if inp.heat < HANDLE_TEMP_FAIL_WARM_HEAT
            && inp.player_last_temperature < HANDLE_TEMP_FAIL_WARM_AMBIENT
            && tmp_last >= inp.heat
        {
            if inp.age > HANDLE_TEMP_KINDLING_AGE {
                if inp.has_fire_place && (inp.fire_parent == FIRE || inp.fire_parent == HOT_COALS)
                {
                    return HandleTemperaturePlan {
                        action: HandleTemperatureAction::KindlingOnFire {
                            x: inp.fire_x,
                            y: inp.fire_y,
                        },
                        is_handling: true,
                        just_arrived: true,
                        last_heat,
                        clear_cold_place: false,
                        clear_warm_place: false,
                        time_bump: 0.0,
                        say: None,
                    };
                }
                return HandleTemperaturePlan {
                    action: HandleTemperatureAction::HandlingFire,
                    is_handling: true,
                    just_arrived: true,
                    last_heat,
                    clear_cold_place: false,
                    clear_warm_place: false,
                    time_bump: 0.0,
                    say: None,
                };
            }
            return HandleTemperaturePlan {
                action: HandleTemperatureAction::Idle,
                is_handling: true,
                just_arrived: true,
                last_heat,
                clear_cold_place: false,
                clear_warm_place: true,
                time_bump: 0.0,
                say: None,
            };
        }
        return HandleTemperaturePlan {
            action: HandleTemperatureAction::Relax,
            is_handling: true,
            just_arrived: true,
            last_heat,
            clear_cold_place: false,
            clear_warm_place: false,
            time_bump: HANDLE_TEMP_RELAX_TIME,
            say: None,
        };
    }

    HandleTemperaturePlan {
        action: HandleTemperatureAction::Goto { x: gx, y: gy },
        is_handling: true,
        just_arrived: inp.just_arrived,
        last_heat,
        clear_cold_place: false,
        clear_warm_place: false,
        time_bump: 0.0,
        say: None,
    }
}

/// Commit fail-warm after kindling / isHandlingFire returned false this tick.
// Haxe: handleTemperature ~1743 warmPlace = null; return false
pub fn fail_warm_clear_place(plan: HandleTemperaturePlan) -> HandleTemperaturePlan {
    HandleTemperaturePlan {
        action: HandleTemperatureAction::Idle,
        is_handling: plan.is_handling,
        just_arrived: plan.just_arrived,
        last_heat: plan.last_heat,
        clear_cold_place: plan.clear_cold_place,
        clear_warm_place: true,
        time_bump: 0.0,
        say: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_when_comfortable() {
        let p = plan_handle_temperature(HandleTemperatureInput::default());
        assert_eq!(p.action, HandleTemperatureAction::Idle);
        assert!(!p.is_handling);
        assert!(!p.just_arrived);
    }

    #[test]
    fn craft_83_wins_before_drink() {
        let mut inp = HandleTemperatureInput::default();
        inp.is_super_hot = true;
        inp.heat = 0.95;
        inp.held_id = WATER_BOWL_ID;
        inp.item_to_craft_id = LARGE_FAST_FIRE;
        let p = plan_handle_temperature(inp);
        assert_eq!(p.action, HandleTemperatureAction::CraftLargeFastFire);
        assert!((p.last_heat - 0.95).abs() < 1e-6);
    }

    #[test]
    fn superhot_drinks_held_water() {
        let mut inp = HandleTemperatureInput::default();
        inp.is_super_hot = true;
        inp.heat = 0.85;
        inp.held_id = WATER_BOWL_ID;
        let p = plan_handle_temperature(inp);
        assert_eq!(p.action, HandleTemperatureAction::DrinkSelf);
        assert_eq!(p.say, Some(HANDLE_TEMP_SAY_DRINK));
    }

    #[test]
    fn superhot_get_or_craft_water_when_empty_hands() {
        let mut inp = HandleTemperatureInput::default();
        inp.is_super_hot = true;
        inp.heat = 0.85;
        let p = plan_handle_temperature(inp);
        assert_eq!(p.action, HandleTemperatureAction::GetOrCraftWater);
    }

    #[test]
    fn cooling_gotos_close_snow_then_cold_place() {
        let mut inp = HandleTemperatureInput::default();
        inp.is_super_hot = true;
        inp.heat = 0.65;
        inp.close_cool = Some((4, 0));
        inp.cold_place = Some((9, 9));
        let p = plan_handle_temperature(inp);
        assert_eq!(p.action, HandleTemperatureAction::Goto { x: 4, y: 0 });
        assert!(p.is_handling);

        inp.close_cool = None;
        let p2 = plan_handle_temperature(inp);
        assert_eq!(p2.action, HandleTemperatureAction::Goto { x: 9, y: 9 });
    }

    #[test]
    fn warming_prefers_hot_fire_then_biome() {
        let mut inp = HandleTemperatureInput::default();
        inp.is_super_cold = true;
        inp.heat = 0.1;
        inp.has_fire_place = true;
        inp.fire_x = 2;
        inp.fire_y = 3;
        inp.fire_parent = FIRE;
        inp.fire_heat_value = 5.0;
        inp.close_warm = Some((8, 0));
        let p = plan_handle_temperature(inp);
        assert_eq!(p.action, HandleTemperatureAction::Goto { x: 2, y: 3 });

        inp.fire_heat_value = 3.0;
        let p2 = plan_handle_temperature(inp);
        assert_eq!(p2.action, HandleTemperatureAction::Goto { x: 8, y: 0 });
    }

    #[test]
    fn first_arrival_relaxes_then_fail_cool_clears_cold() {
        let mut inp = HandleTemperatureInput::default();
        inp.is_super_hot = true;
        inp.heat = 0.65;
        inp.cold_place = Some((0, 0));
        inp.px = 0;
        inp.py = 0;
        let p = plan_handle_temperature(inp);
        assert_eq!(p.action, HandleTemperatureAction::Relax);
        assert!(p.just_arrived);
        assert_eq!(p.time_bump, HANDLE_TEMP_RELAX_TIME);

        inp.just_arrived = true;
        inp.heat = 0.7;
        inp.last_heat = 0.6;
        inp.player_last_temperature = 0.5;
        let p2 = plan_handle_temperature(inp);
        assert_eq!(p2.action, HandleTemperatureAction::Idle);
        assert!(p2.clear_cold_place);
        assert!(p2.is_handling);
    }

    #[test]
    fn fail_warm_kindling_then_handling_fire() {
        let mut inp = HandleTemperatureInput::default();
        inp.is_super_cold = true;
        inp.heat = 0.2;
        inp.last_heat = 0.3;
        inp.player_last_temperature = 0.4;
        inp.just_arrived = true;
        inp.has_fire_place = true;
        inp.fire_parent = FIRE;
        inp.fire_heat_value = 5.0;
        inp.fire_x = 0;
        inp.fire_y = 0;
        inp.age = 20.0;
        let p = plan_handle_temperature(inp);
        assert_eq!(
            p.action,
            HandleTemperatureAction::KindlingOnFire { x: 0, y: 0 }
        );

        inp.fire_parent = LARGE_FAST_FIRE;
        let p2 = plan_handle_temperature(inp);
        assert_eq!(p2.action, HandleTemperatureAction::HandlingFire);
        let cleared = fail_warm_clear_place(p2);
        assert!(cleared.clear_warm_place);
        assert_eq!(cleared.action, HandleTemperatureAction::Idle);
    }

    #[test]
    fn winter_female_kids_forces_warming() {
        let mut inp = HandleTemperatureInput::default();
        inp.heat = 0.5;
        inp.winter_female_with_kids = true;
        inp.warm_place = Some((3, 1));
        let p = plan_handle_temperature(inp);
        assert_eq!(p.action, HandleTemperatureAction::Goto { x: 3, y: 1 });
    }

    #[test]
    fn cooling_beats_winter_warming() {
        let mut inp = HandleTemperatureInput::default();
        inp.is_super_hot = true;
        inp.heat = 0.65;
        inp.winter_female_with_kids = true;
        inp.close_cool = Some((3, 0));
        inp.warm_place = Some((9, 0));
        let p = plan_handle_temperature(inp);
        assert_eq!(p.action, HandleTemperatureAction::Goto { x: 3, y: 0 });
    }

    #[test]
    fn get_close_biome_picks_nearest_and_skips_blocked() {
        let tiles = [(5, 0, BIOME_SNOW), (2, 0, BIOME_SNOW), (1, 0, DESERT)];
        let hit = get_close_biome(0, 0, &COOL_BIOMES, &tiles, |x, _| x == 2, 32, 32, false);
        assert_eq!(hit, Some((5, 0)));
        let miss = get_close_biome(0, 0, &COOL_BIOMES, &[(1, 0, DESERT)], |_, _| false, 32, 32, false);
        assert!(miss.is_none());
    }

    #[test]
    fn count_little_kids_age_gate() {
        let kids = [
            (2, 2.0, false, Some(1), None),
            (3, 4.0, false, Some(1), None),
            (4, 1.0, true, Some(1), None),
            (5, 1.0, false, None, Some(1)),
        ];
        assert_eq!(count_little_kids(1, 3.0, &kids), 2);
        assert_eq!(count_little_kids(9, 3.0, &kids), 0);
    }

    #[test]
    fn sticky_cool_without_superhot() {
        let mut inp = HandleTemperatureInput::default();
        inp.is_handling = true;
        inp.heat = 0.65;
        inp.cold_place = Some((6, 0));
        let p = plan_handle_temperature(inp);
        assert_eq!(p.action, HandleTemperatureAction::Goto { x: 6, y: 0 });
    }
}
