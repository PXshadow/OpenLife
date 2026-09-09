//! Haxe `AiBase.craftHighPriorityClothing` / `craftMediumPriorityClothing` /
//! `craftLowPriorityClothing` / `craftClothIfNeeded`.
//!
//! Pure planner: which clothing product to `craftItem` this tick. Execution is
//! existing GetOrCraft / sticky craft I/O.
// Haxe: AiBase ~4492–4629, craftClothIfNeeded ~4660

/// Haxe `Player.getColor()` (content `person` race).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonColor {
    Black,
    Brown,
    White,
    Ginger,
}

/// Map Haxe `ObjectData.person` race to [`PersonColor`].
/// Black=1 Brown=3 White=4 Ginger=6.
#[inline]
pub fn person_color_from_race(race: i32) -> PersonColor {
    match race {
        1 => PersonColor::Black,
        4 => PersonColor::White,
        6 => PersonColor::Ginger,
        _ => PersonColor::Brown,
    }
}

#[inline]
pub fn is_white_or_ginger(c: PersonColor) -> bool {
    matches!(c, PersonColor::White | PersonColor::Ginger)
}

/// Haxe clothingObjects indices.
pub const SLOT_HEAD: usize = 0;
pub const SLOT_CHEST: usize = 1;
pub const SLOT_SHOE: usize = 2;
pub const SLOT_BOTTOM: usize = 4;
pub const SLOT_BACK: usize = 5;

/// Known OHOL clothing slots for the Haxe priority lists (file `clothing=`).
#[inline]
pub fn default_cloth_slot(id: i32) -> Option<usize> {
    match id {
        128 | 200 | 2951 | 2878 => Some(SLOT_BOTTOM),
        593 | 585 | 564 | 712 | 711 | 202 | 201 | 2926 | 2879 => Some(SLOT_CHEST),
        844 | 2887 | 766 | 586 | 203 => Some(SLOT_SHOE),
        584 | 426 | 2920 | 3461 | 3431 | 2884 | 199 | 2180 => Some(SLOT_HEAD),
        198 | 874 => Some(SLOT_BACK),
        _ => None,
    }
}

/// Haxe `craftClothIfNeeded`: empty slot or `RAG ` name.
#[inline]
pub fn slot_needs_cloth(clothing_ids: &[i32; 6], rag: &[bool; 6], slot: usize) -> bool {
    if slot >= 6 {
        return false;
    }
    clothing_ids[slot] <= 0 || rag[slot]
}

/// Empty Arrow Quiver (Haxe 874).
pub const EMPTY_ARROW_QUIVER: i32 = 874;
/// Arrow Quiver (Haxe 3948).
pub const ARROW_QUIVER: i32 = 3948;
/// Arrow Quiver with Bow (Haxe 4151).
pub const ARROW_QUIVER_WITH_BOW: i32 = 4151;
/// Yew Bow.
pub const YEW_BOW: i32 = 151;
/// Bow and Arrow.
pub const BOW_AND_ARROW: i32 = 152;
/// Arrow.
pub const ARROW: i32 = 148;
/// Featherless Arrow (Haxe `craftItemMax(147, 2)`).
pub const FEATHERLESS_ARROW: i32 = 147;
/// Tied Skewer (Haxe `craftItemMax(140, 2)`).
pub const TIED_SKEWER: i32 = 140;
/// Headless Arrow (Haxe `craftItemMax(149, 2)`).
pub const HEADLESS_ARROW: i32 = 149;
/// Empty Water Pouch (Haxe `craftItemMax(209)`).
pub const EMPTY_WATER_POUCH: i32 = 209;
/// Backpack.
pub const BACKPACK: i32 = 198;
/// Rabbit Fur Hat.
pub const RABBIT_FUR_HAT: i32 = 199;
/// Rabbit Fur Hat with Feather.
pub const RABBIT_FUR_HAT_FEATHER: i32 = 2180;
/// Loom (Haxe CountCloseObjects 2682 r=30).
pub const LOOM: i32 = 2682;
/// Haxe `CountCloseObjects(..., 2682, 30)`.
pub const HOME_LOOM_RADIUS: i32 = 30;
/// Indigo Long Dress (`clothing=t`).
pub const INDIGO_LONG_DRESS: i32 = 2926;
/// Undyed Long Dress (`clothing=t`).
pub const UNDYED_LONG_DRESS: i32 = 2879;
/// Black Long Skirt (`clothing=b`).
pub const BLACK_LONG_SKIRT: i32 = 2951;
/// Undyed Long Skirt (`clothing=b`).
pub const UNDYED_LONG_SKIRT: i32 = 2878;
/// Haxe `self(0,0,5)` backpack clothing index.
pub const QUIVER_SELF_SLOT: i32 = 5;
/// Haxe `fillUpQuiver` wide `itemToCraft.maxSearchRadius`.
pub const QUIVER_SEARCH_RADIUS_WIDE: i32 = 60;
/// Haxe `fillUpQuiver` near radius (other TAILOR present, or `age < 20`).
pub const QUIVER_SEARCH_RADIUS_NEAR: i32 = 20;
/// Haxe `if (myPlayer.age < 20) maxSearchRadius = 20`.
pub const QUIVER_SEARCH_AGE_CAP: f32 = 20.0;

/// Near-home stock for extra winter / backpack (Haxe CountCloseObjects r=60).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HomeClothStock {
    pub backpack: i32,
    pub loincloth: i32,
    pub fur_hat: i32,
    pub fur_hat_feather: i32,
    pub fur_coat: i32,
    /// Empty Water Pouch 209 (Haxe `countCurrentObject` / `craftItemMax`).
    pub water_pouch: i32,
    /// Featherless Arrow 147.
    pub featherless_arrow: i32,
    /// Tied Skewer 140.
    pub tied_skewer: i32,
    /// Headless Arrow 149.
    pub headless_arrow: i32,
}

/// Planner output (craftItem / GetOrCraft / SELF onto quiver).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClothingCraftPlan {
    CraftItem(i32),
    GetOrCraft {
        object_id: i32,
        craft_if_needed: bool,
    },
    /// Haxe `GetItem(get_id)` then `GetOrCraftItem(fallback_id)` in one helper call.
    GetItemThenGetOrCraft {
        get_id: i32,
        fallback_id: i32,
    },
    SelfClothing {
        slot: i32,
    },
}

impl ClothingCraftPlan {
    pub fn as_craft_item(self) -> Option<i32> {
        match self {
            Self::CraftItem(id) => Some(id),
            _ => None,
        }
    }
}

/// Inputs for clothing craft planners.
#[derive(Debug, Clone, Copy)]
pub struct ClothingCraftInput<'a> {
    pub color: PersonColor,
    pub clothing_ids: &'a [i32; 6],
    pub rag: &'a [bool; 6],
    pub age: f32,
    /// Haxe `hasOrBecomeProfession('TAILOR', maxProf)`.
    pub has_tailor: bool,
    /// Haxe `age >= ObjectData(152).minPickupAge`.
    pub bow_old_enough: bool,
    pub held_id: i32,
    /// Haxe `quiver.canAddToQuiver()`.
    pub quiver_can_add: bool,
    pub home_stock: HomeClothStock,
    /// Haxe `myPlayer.isFemale()`.
    pub female: bool,
    /// Haxe `CountCloseObjects(..., Loom 2682, 30) > 0`.
    pub has_loom: bool,
    /// Haxe `assignedProfession == 'TAILOR' || lastProfession == 'TAILOR'`
    /// (`craftMedium/LowPriorityClothing(100)` assigned block).
    pub assigned_tailor: bool,
}

/// Canonical Haxe profession string.
pub const TAILOR_PROFESSION_KEY: &str = "TAILOR";
/// Assigned/last `craftMedium/LowPriorityClothing(100)`.
pub const TAILOR_ASSIGNED_MAX: i32 = 100;
/// Default `craftMedium/LowPriorityClothing()` maxProf.
pub const TAILOR_DEFAULT_MAX: i32 = 1;
/// Clothing craft `maxSearchRadius` (Haxe itemToCraft 40–60; quiver wraps separately).
pub const TAILOR_SCAN_RADIUS: i32 = 40;

pub fn parse_tailor_profession_speech(text: &str) -> bool {
    let t = text.trim();
    let prof = t.strip_suffix('!').unwrap_or(t).trim();
    prof.eq_ignore_ascii_case(TAILOR_PROFESSION_KEY)
}

/// Assigned/last TAILOR uses maxProf 100; open clothing uses 1.
pub fn tailor_max_for_dispatch(is_assigned: bool, rung_label: &str) -> i32 {
    if is_assigned || rung_label == "ASSIGNED_JOB" {
        TAILOR_ASSIGNED_MAX
    } else {
        TAILOR_DEFAULT_MAX
    }
}

/// Haxe assigned TAILOR block: high then medium(100) then low(100) (no age gates).
// Haxe: doTimeStuffHelper assignedProfession/lastProfession == 'TAILOR' ~749–752
pub fn plan_assigned_tailor_clothing(inp: &ClothingCraftInput<'_>) -> Option<ClothingCraftPlan> {
    if let Some(id) = plan_high_priority_clothing(inp) {
        return Some(ClothingCraftPlan::CraftItem(id));
    }
    if let Some(m) = plan_medium_priority_clothing(inp) {
        return Some(m);
    }
    plan_low_priority_clothing(inp)
}

/// Accrue home CountCloseObjects stock used by medium extras / quiver precursors.
pub fn add_home_cloth_id(s: &mut HomeClothStock, id: i32) {
    match id {
        198 => s.backpack += 1,
        200 => s.loincloth += 1,
        199 => s.fur_hat += 1,
        2180 => s.fur_hat_feather += 1,
        202 => s.fur_coat += 1,
        209 => s.water_pouch += 1,
        147 => s.featherless_arrow += 1,
        140 => s.tied_skewer += 1,
        149 => s.headless_arrow += 1,
        _ => {}
    }
}

/// Haxe `hasOrBecomeProfession('TAILOR', max)`.
///
/// Sticky last tailor always allowed. `max < 0` is high-priority (do job, do not
/// treat as a new assign). Else refuse when `countProfession >= max + wasIdle`.
// Haxe: AiBase.hasOrBecomeProfession ~4466; craftMedium/Low ~4520 / ~4600
pub fn has_or_become_tailor(
    is_last_tailor: bool,
    max: i32,
    peer_count: f32,
    was_idle: f32,
) -> bool {
    if max < 0 {
        return true;
    }
    if is_last_tailor {
        return true;
    }
    let cap = max as f32 + was_idle.max(0.0);
    if peer_count >= cap {
        return false;
    }
    true
}

/// Haxe `craftHighPriorityClothing` — no TAILOR gate.
// Haxe: AiBase ~4492
pub fn plan_high_priority_clothing(inp: &ClothingCraftInput<'_>) -> Option<i32> {
    let ids = inp.clothing_ids;
    let rag = inp.rag;
    let need = |id: i32| {
        default_cloth_slot(id).is_some_and(|s| slot_needs_cloth(ids, rag, s))
    };
    if is_white_or_ginger(inp.color) && need(200) {
        return Some(200);
    }
    if need(128) {
        return Some(128);
    }
    if inp.color == PersonColor::White && need(593) {
        return Some(593);
    }
    None
}

/// Medium clothing list after Empty Water Pouch 209 (Haxe order).
const MEDIUM_CLOTHS: &[(i32, fn(PersonColor, bool) -> bool)] = &[
    (844, |c, _| c == PersonColor::Black),
    (2887, |c, _| c == PersonColor::Black),
    (766, |c, _| c == PersonColor::Black),
    (586, |c, _| is_white_or_ginger(c)),
    (203, |c, _| is_white_or_ginger(c)),
    (585, |c, _| c == PersonColor::White),
    (564, |c, bow| c == PersonColor::White && bow),
    (712, |c, _| c == PersonColor::Ginger),
    (711, |c, _| c == PersonColor::Ginger),
    (202, |c, _| is_white_or_ginger(c)),
    (201, |c, _| is_white_or_ginger(c)),
    (584, |c, _| c == PersonColor::White),
    (426, |c, _| c == PersonColor::White),
    (2920, |_, _| true),
    (3461, |_, _| true),
    (3431, |_, _| true),
    (2884, |_, _| true),
];

fn wearing_any(ids: &[i32; 6], want: &[i32]) -> bool {
    ids.iter().any(|id| want.contains(id))
}

/// Haxe `fillUpQuiver` temporary `itemToCraft.maxSearchRadius`.
///
/// `countProfession('TAILOR') < 1 || lastProfession == 'TAILOR'` → 60, else 20;
/// `age < 20` always 20.
// Haxe: AiBase ~4632–4637
pub fn fill_up_quiver_search_radius(
    age: f32,
    tailor_profession_count: i32,
    is_last_tailor: bool,
) -> i32 {
    if age < QUIVER_SEARCH_AGE_CAP {
        return QUIVER_SEARCH_RADIUS_NEAR;
    }
    if tailor_profession_count < 1 || is_last_tailor {
        QUIVER_SEARCH_RADIUS_WIDE
    } else {
        QUIVER_SEARCH_RADIUS_NEAR
    }
}

/// True when the plan is from `fillUpQuiverHelper` (wrap maxSearchRadius).
pub fn is_fill_up_quiver_plan(plan: ClothingCraftPlan) -> bool {
    match plan {
        ClothingCraftPlan::CraftItem(id) => matches!(
            id,
            EMPTY_ARROW_QUIVER | FEATHERLESS_ARROW | TIED_SKEWER | HEADLESS_ARROW
        ),
        ClothingCraftPlan::GetItemThenGetOrCraft {
            get_id: BOW_AND_ARROW,
            fallback_id: YEW_BOW,
        } => true,
        ClothingCraftPlan::GetOrCraft {
            object_id: ARROW, ..
        } => true,
        ClothingCraftPlan::SelfClothing {
            slot: QUIVER_SELF_SLOT,
        } => true,
        _ => false,
    }
}

#[inline]
pub fn count_with_held(stock: i32, held_id: i32, obj_id: i32) -> i32 {
    stock + i32::from(held_id == obj_id)
}

/// Haxe `craftItemMax(147, 2)` / `craftItemMax(140, 2)` / `craftItemMax(149, 2)`
/// after `GetOrCraftItem(148)` fails.
// Haxe: fillUpQuiverHelper ~4690–4696
pub fn plan_quiver_arrow_precursors(
    held_id: i32,
    stock: &HomeClothStock,
) -> Option<ClothingCraftPlan> {
    const MAX: i32 = 2;
    if count_with_held(stock.featherless_arrow, held_id, FEATHERLESS_ARROW) < MAX {
        return Some(ClothingCraftPlan::CraftItem(FEATHERLESS_ARROW));
    }
    if count_with_held(stock.tied_skewer, held_id, TIED_SKEWER) < MAX {
        return Some(ClothingCraftPlan::CraftItem(TIED_SKEWER));
    }
    if count_with_held(stock.headless_arrow, held_id, HEADLESS_ARROW) < MAX {
        return Some(ClothingCraftPlan::CraftItem(HEADLESS_ARROW));
    }
    None
}

/// Haxe `fillUpQuiverHelper` (bow-age gate is in [`plan_medium_priority_clothing`]).
// Haxe: AiBase ~4644
pub fn plan_fill_up_quiver(inp: &ClothingCraftInput<'_>) -> Option<ClothingCraftPlan> {
    let ids = inp.clothing_ids;
    let rag = inp.rag;
    if default_cloth_slot(EMPTY_ARROW_QUIVER)
        .is_some_and(|s| slot_needs_cloth(ids, rag, s))
    {
        return Some(ClothingCraftPlan::CraftItem(EMPTY_ARROW_QUIVER));
    }
    let has_quiver_bow = wearing_any(ids, &[EMPTY_ARROW_QUIVER, ARROW_QUIVER]);
    if has_quiver_bow {
        if inp.held_id == YEW_BOW || inp.held_id == BOW_AND_ARROW {
            return Some(ClothingCraftPlan::SelfClothing {
                slot: QUIVER_SELF_SLOT,
            });
        }
        // Haxe: GetItem(152) then GetOrCraftItem(151)
        return Some(ClothingCraftPlan::GetItemThenGetOrCraft {
            get_id: BOW_AND_ARROW,
            fallback_id: YEW_BOW,
        });
    }
    let has_any_quiver = wearing_any(
        ids,
        &[EMPTY_ARROW_QUIVER, ARROW_QUIVER, ARROW_QUIVER_WITH_BOW],
    );
    if has_any_quiver && inp.quiver_can_add {
        if inp.held_id == ARROW {
            return Some(ClothingCraftPlan::SelfClothing {
                slot: QUIVER_SELF_SLOT,
            });
        }
        return Some(ClothingCraftPlan::GetOrCraft {
            object_id: ARROW,
            craft_if_needed: true,
        });
    }
    None
}

/// Extra winter + backpack after worn clothes (Haxe CountCloseObjects r=60).
// Haxe: AiBase ~4570–4592
fn plan_medium_home_stock(inp: &ClothingCraftInput<'_>) -> Option<ClothingCraftPlan> {
    if inp.age > 25.0 && inp.home_stock.backpack < 1 {
        return Some(ClothingCraftPlan::CraftItem(BACKPACK));
    }
    if is_white_or_ginger(inp.color) {
        if inp.home_stock.loincloth < 1 {
            return Some(ClothingCraftPlan::CraftItem(200));
        }
        if inp.home_stock.fur_hat < 1 {
            return Some(ClothingCraftPlan::CraftItem(RABBIT_FUR_HAT));
        }
        if inp.home_stock.fur_hat_feather < 1 {
            return Some(ClothingCraftPlan::CraftItem(RABBIT_FUR_HAT_FEATHER));
        }
        if inp.home_stock.fur_coat < 1 {
            return Some(ClothingCraftPlan::CraftItem(202));
        }
    }
    None
}

/// Haxe `craftMediumPriorityClothing`: quiver (bow age) then TAILOR clothes + extras.
// Haxe: AiBase ~4510
pub fn plan_medium_priority_clothing(inp: &ClothingCraftInput<'_>) -> Option<ClothingCraftPlan> {
    if inp.bow_old_enough {
        if let Some(q) = plan_fill_up_quiver(inp) {
            return Some(q);
        }
    }
    if !inp.has_tailor {
        return None;
    }
    if count_with_held(inp.home_stock.water_pouch, inp.held_id, EMPTY_WATER_POUCH) < 1 {
        return Some(ClothingCraftPlan::CraftItem(EMPTY_WATER_POUCH));
    }
    let ids = inp.clothing_ids;
    let rag = inp.rag;
    for &(id, gate) in MEDIUM_CLOTHS {
        if !gate(inp.color, inp.bow_old_enough) {
            continue;
        }
        if default_cloth_slot(id).is_some_and(|s| slot_needs_cloth(ids, rag, s)) {
            return Some(ClothingCraftPlan::CraftItem(id));
        }
    }
    plan_medium_home_stock(inp)
}

/// Loom dresses/skirts then rabbit-fur hats (Haxe `craftLowPriorityClothing`).
// Haxe: AiBase ~4599–4629
pub fn plan_low_priority_clothing(inp: &ClothingCraftInput<'_>) -> Option<ClothingCraftPlan> {
    if !inp.has_tailor {
        return None;
    }
    let ids = inp.clothing_ids;
    let rag = inp.rag;
    let need = |id: i32| {
        default_cloth_slot(id).is_some_and(|s| slot_needs_cloth(ids, rag, s))
    };
    if inp.has_loom && inp.female {
        for id in [
            INDIGO_LONG_DRESS,
            UNDYED_LONG_DRESS,
            BLACK_LONG_SKIRT,
            UNDYED_LONG_SKIRT,
        ] {
            if need(id) {
                return Some(ClothingCraftPlan::CraftItem(id));
            }
        }
    }
    if is_white_or_ginger(inp.color) {
        if inp.bow_old_enough && need(RABBIT_FUR_HAT_FEATHER) {
            return Some(ClothingCraftPlan::CraftItem(RABBIT_FUR_HAT_FEATHER));
        }
        if need(RABBIT_FUR_HAT) {
            return Some(ClothingCraftPlan::CraftItem(RABBIT_FUR_HAT));
        }
    }
    None
}

/// High band, then medium if `age > 10` or assigned/last TAILOR, then low if
/// `age > 30` or assigned/last TAILOR.
// Haxe: doTimeStuffHelper ~684–690, assigned TAILOR ~749–752, age>30 ~817
pub fn plan_clothing_craft_tick(inp: &ClothingCraftInput<'_>) -> Option<ClothingCraftPlan> {
    if let Some(id) = plan_high_priority_clothing(inp) {
        return Some(ClothingCraftPlan::CraftItem(id));
    }
    if inp.age > 10.0 || inp.assigned_tailor {
        if let Some(m) = plan_medium_priority_clothing(inp) {
            return Some(m);
        }
    }
    if inp.age > 30.0 || inp.assigned_tailor {
        return plan_low_priority_clothing(inp);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_slots() -> ([i32; 6], [bool; 6]) {
        ([0; 6], [false; 6])
    }

    #[test]
    fn high_priority_white_wants_loincloth() {
        let (ids, rag) = empty_slots();
        let inp = ClothingCraftInput {
            color: PersonColor::White,
            clothing_ids: &ids,
            rag: &rag,
            age: 20.0,
            has_tailor: false,
            bow_old_enough: true,
            held_id: 0,
            quiver_can_add: false,
            home_stock: HomeClothStock::default(),
            female: false,
            has_loom: false,
            assigned_tailor: false,
        };
        assert_eq!(plan_high_priority_clothing(&inp), Some(200));
        assert_eq!(
            plan_clothing_craft_tick(&inp),
            Some(ClothingCraftPlan::CraftItem(200))
        );
    }

    #[test]
    fn high_priority_brown_skips_loincloth_uses_reed_skirt() {
        let (ids, rag) = empty_slots();
        let inp = ClothingCraftInput {
            color: PersonColor::Brown,
            clothing_ids: &ids,
            rag: &rag,
            age: 20.0,
            has_tailor: false,
            bow_old_enough: true,
            held_id: 0,
            quiver_can_add: false,
            home_stock: HomeClothStock::default(),
            female: false,
            has_loom: false,
            assigned_tailor: false,
        };
        assert_eq!(plan_high_priority_clothing(&inp), Some(128));
    }

    #[test]
    fn rag_bottom_forces_recraft() {
        let mut ids = [0; 6];
        ids[SLOT_BOTTOM] = 128;
        let mut rag = [false; 6];
        rag[SLOT_BOTTOM] = true;
        let inp = ClothingCraftInput {
            color: PersonColor::Brown,
            clothing_ids: &ids,
            rag: &rag,
            age: 8.0,
            has_tailor: false,
            bow_old_enough: false,
            held_id: 0,
            quiver_can_add: false,
            home_stock: HomeClothStock::default(),
            female: false,
            has_loom: false,
            assigned_tailor: false,
        };
        assert_eq!(plan_high_priority_clothing(&inp), Some(128));
    }

    #[test]
    fn filled_bottom_no_high() {
        let mut ids = [0; 6];
        ids[SLOT_BOTTOM] = 128;
        let rag = [false; 6];
        let inp = ClothingCraftInput {
            color: PersonColor::Brown,
            clothing_ids: &ids,
            rag: &rag,
            age: 8.0,
            has_tailor: false,
            bow_old_enough: false,
            held_id: 0,
            quiver_can_add: false,
            home_stock: HomeClothStock::default(),
            female: false,
            has_loom: false,
            assigned_tailor: false,
        };
        assert_eq!(plan_high_priority_clothing(&inp), None);
        assert_eq!(plan_clothing_craft_tick(&inp), None);
    }

    #[test]
    fn medium_requires_tailor() {
        let (ids, rag) = empty_slots();
        let mut inp = ClothingCraftInput {
            color: PersonColor::Black,
            clothing_ids: &ids,
            rag: &rag,
            age: 20.0,
            has_tailor: false,
            bow_old_enough: false,
            held_id: 0,
            quiver_can_add: false,
            home_stock: HomeClothStock::default(),
            female: false,
            has_loom: false,
            assigned_tailor: false,
        };
        assert_eq!(plan_medium_priority_clothing(&inp), None);
        inp.has_tailor = true;
        assert_eq!(
            plan_medium_priority_clothing(&inp),
            Some(ClothingCraftPlan::CraftItem(EMPTY_WATER_POUCH))
        );
        inp.home_stock.water_pouch = 1;
        assert_eq!(
            plan_medium_priority_clothing(&inp),
            Some(ClothingCraftPlan::CraftItem(844))
        );
    }

    #[test]
    fn medium_white_shoes_after_bottom_filled() {
        let mut ids = [0; 6];
        ids[SLOT_BOTTOM] = 200;
        let rag = [false; 6];
        let inp = ClothingCraftInput {
            color: PersonColor::White,
            clothing_ids: &ids,
            rag: &rag,
            age: 20.0,
            has_tailor: true,
            bow_old_enough: false,
            held_id: 0,
            quiver_can_add: false,
            home_stock: HomeClothStock {
                loincloth: 1,
                fur_hat: 1,
                fur_hat_feather: 1,
                fur_coat: 1,
                backpack: 1,
                water_pouch: 1,
                ..HomeClothStock::default()
            },
            female: false,
            has_loom: false,
            assigned_tailor: false,
        };
        assert_eq!(plan_high_priority_clothing(&inp), Some(593)); // sheep skin chest
        let mut ids2 = ids;
        ids2[SLOT_CHEST] = 593;
        let inp2 = ClothingCraftInput {
            clothing_ids: &ids2,
            ..inp
        };
        assert_eq!(plan_high_priority_clothing(&inp2), None);
        assert_eq!(
            plan_medium_priority_clothing(&inp2),
            Some(ClothingCraftPlan::CraftItem(586))
        );
    }

    #[test]
    fn fill_up_quiver_crafts_empty_quiver_then_gets_bow() {
        let (ids, rag) = empty_slots();
        let mut inp = ClothingCraftInput {
            color: PersonColor::Brown,
            clothing_ids: &ids,
            rag: &rag,
            age: 20.0,
            has_tailor: false,
            bow_old_enough: true,
            held_id: 0,
            quiver_can_add: true,
            home_stock: HomeClothStock::default(),
            female: false,
            has_loom: false,
            assigned_tailor: false,
        };
        assert_eq!(
            plan_fill_up_quiver(&inp),
            Some(ClothingCraftPlan::CraftItem(EMPTY_ARROW_QUIVER))
        );
        let mut worn = ids;
        worn[SLOT_BACK] = EMPTY_ARROW_QUIVER;
        inp.clothing_ids = &worn;
        assert_eq!(
            plan_fill_up_quiver(&inp),
            Some(ClothingCraftPlan::GetItemThenGetOrCraft {
                get_id: BOW_AND_ARROW,
                fallback_id: YEW_BOW,
            })
        );
        inp.held_id = YEW_BOW;
        assert_eq!(
            plan_fill_up_quiver(&inp),
            Some(ClothingCraftPlan::SelfClothing {
                slot: QUIVER_SELF_SLOT,
            })
        );
    }

    #[test]
    fn extra_winter_crafts_when_home_stock_empty() {
        let mut ids = [0; 6];
        ids[SLOT_BOTTOM] = 200;
        ids[SLOT_CHEST] = 202;
        ids[SLOT_SHOE] = 586;
        ids[SLOT_HEAD] = 584;
        ids[SLOT_BACK] = EMPTY_ARROW_QUIVER;
        let rag = [false; 6];
        let inp = ClothingCraftInput {
            color: PersonColor::White,
            clothing_ids: &ids,
            rag: &rag,
            age: 26.0,
            has_tailor: true,
            bow_old_enough: false,
            held_id: 0,
            quiver_can_add: false,
            home_stock: HomeClothStock {
                water_pouch: 1,
                ..HomeClothStock::default()
            },
            female: false,
            has_loom: false,
            assigned_tailor: false,
        };
        assert_eq!(plan_high_priority_clothing(&inp), None);
        assert_eq!(
            plan_medium_priority_clothing(&inp),
            Some(ClothingCraftPlan::CraftItem(BACKPACK))
        );
        let inp2 = ClothingCraftInput {
            home_stock: HomeClothStock {
                backpack: 1,
                water_pouch: 1,
                ..HomeClothStock::default()
            },
            ..inp
        };
        assert_eq!(
            plan_medium_priority_clothing(&inp2),
            Some(ClothingCraftPlan::CraftItem(200))
        );
    }

    #[test]
    fn low_requires_tailor() {
        let (ids, rag) = empty_slots();
        let mut inp = ClothingCraftInput {
            color: PersonColor::Brown,
            clothing_ids: &ids,
            rag: &rag,
            age: 35.0,
            has_tailor: false,
            bow_old_enough: true,
            held_id: 0,
            quiver_can_add: false,
            home_stock: HomeClothStock::default(),
            female: true,
            has_loom: true,
            assigned_tailor: false,
        };
        assert_eq!(plan_low_priority_clothing(&inp), None);
        inp.has_tailor = true;
        assert_eq!(
            plan_low_priority_clothing(&inp),
            Some(ClothingCraftPlan::CraftItem(INDIGO_LONG_DRESS))
        );
    }

    #[test]
    fn low_female_loom_skirt_when_chest_filled() {
        let mut ids = [0; 6];
        ids[SLOT_CHEST] = INDIGO_LONG_DRESS;
        ids[SLOT_BOTTOM] = 0;
        let rag = [false; 6];
        let inp = ClothingCraftInput {
            color: PersonColor::Brown,
            clothing_ids: &ids,
            rag: &rag,
            age: 35.0,
            has_tailor: true,
            bow_old_enough: true,
            held_id: 0,
            quiver_can_add: false,
            home_stock: HomeClothStock::default(),
            female: true,
            has_loom: true,
            assigned_tailor: false,
        };
        assert_eq!(
            plan_low_priority_clothing(&inp),
            Some(ClothingCraftPlan::CraftItem(BLACK_LONG_SKIRT))
        );
    }

    #[test]
    fn low_skipped_under_30_unless_assigned() {
        let mut ids = [0; 6];
        ids[SLOT_BOTTOM] = 128;
        ids[SLOT_SHOE] = 844;
        ids[SLOT_HEAD] = 2920;
        let rag = [false; 6];
        let mut inp = ClothingCraftInput {
            color: PersonColor::Black,
            clothing_ids: &ids,
            rag: &rag,
            age: 20.0,
            has_tailor: true,
            bow_old_enough: false,
            held_id: 0,
            quiver_can_add: false,
            home_stock: HomeClothStock {
                backpack: 1,
                water_pouch: 1,
                ..HomeClothStock::default()
            },
            female: true,
            has_loom: true,
            assigned_tailor: false,
        };
        assert_eq!(plan_high_priority_clothing(&inp), None);
        assert_eq!(plan_medium_priority_clothing(&inp), None);
        assert_eq!(plan_clothing_craft_tick(&inp), None);
        inp.assigned_tailor = true;
        assert_eq!(
            plan_clothing_craft_tick(&inp),
            Some(ClothingCraftPlan::CraftItem(INDIGO_LONG_DRESS))
        );
        inp.assigned_tailor = false;
        inp.age = 31.0;
        assert_eq!(
            plan_clothing_craft_tick(&inp),
            Some(ClothingCraftPlan::CraftItem(INDIGO_LONG_DRESS))
        );
    }

    #[test]
    fn assigned_tailor_runs_medium_under_age_10() {
        // Haxe assigned TAILOR craftMediumPriorityClothing(100) — no age>10 gate
        let mut ids = [0; 6];
        ids[SLOT_BOTTOM] = 128;
        let rag = [false; 6];
        let mut inp = ClothingCraftInput {
            color: PersonColor::Black,
            clothing_ids: &ids,
            rag: &rag,
            age: 8.0,
            has_tailor: true,
            bow_old_enough: false,
            held_id: 0,
            quiver_can_add: false,
            home_stock: HomeClothStock::default(),
            female: false,
            has_loom: false,
            assigned_tailor: false,
        };
        assert_eq!(plan_high_priority_clothing(&inp), None);
        assert_eq!(plan_clothing_craft_tick(&inp), None);
        inp.assigned_tailor = true;
        assert_eq!(
            plan_clothing_craft_tick(&inp),
            Some(ClothingCraftPlan::CraftItem(EMPTY_WATER_POUCH))
        );
        assert_eq!(
            plan_assigned_tailor_clothing(&inp),
            Some(ClothingCraftPlan::CraftItem(EMPTY_WATER_POUCH))
        );
    }

    #[test]
    fn parse_tailor_speech_and_assigned_max() {
        assert!(parse_tailor_profession_speech("TAILOR!"));
        assert!(parse_tailor_profession_speech("tailor"));
        assert!(!parse_tailor_profession_speech("FOODSERVER!"));
        assert_eq!(tailor_max_for_dispatch(true, "ASSIGNED_JOB"), TAILOR_ASSIGNED_MAX);
        assert_eq!(tailor_max_for_dispatch(false, "LOW_PRIORITY_WORK"), TAILOR_DEFAULT_MAX);
        assert_eq!(tailor_max_for_dispatch(false, "ASSIGNED_JOB"), TAILOR_ASSIGNED_MAX);
    }

    #[test]
    fn low_white_feather_hat_when_head_empty() {
        let mut ids = [0; 6];
        ids[SLOT_BOTTOM] = 200;
        ids[SLOT_CHEST] = 202;
        ids[SLOT_SHOE] = 586;
        let rag = [false; 6];
        let inp = ClothingCraftInput {
            color: PersonColor::White,
            clothing_ids: &ids,
            rag: &rag,
            age: 35.0,
            has_tailor: true,
            bow_old_enough: true,
            held_id: 0,
            quiver_can_add: false,
            home_stock: HomeClothStock::default(),
            female: false,
            has_loom: false,
            assigned_tailor: false,
        };
        assert_eq!(
            plan_low_priority_clothing(&inp),
            Some(ClothingCraftPlan::CraftItem(RABBIT_FUR_HAT_FEATHER))
        );
    }

    #[test]
    fn quiver_arrow_precursors_stock_gate() {
        let stock = HomeClothStock::default();
        assert_eq!(
            plan_quiver_arrow_precursors(0, &stock),
            Some(ClothingCraftPlan::CraftItem(FEATHERLESS_ARROW))
        );
        let mut s2 = stock;
        s2.featherless_arrow = 2;
        assert_eq!(
            plan_quiver_arrow_precursors(0, &s2),
            Some(ClothingCraftPlan::CraftItem(TIED_SKEWER))
        );
        s2.tied_skewer = 2;
        assert_eq!(
            plan_quiver_arrow_precursors(0, &s2),
            Some(ClothingCraftPlan::CraftItem(HEADLESS_ARROW))
        );
        s2.headless_arrow = 2;
        assert_eq!(plan_quiver_arrow_precursors(0, &s2), None);
        assert_eq!(
            plan_quiver_arrow_precursors(HEADLESS_ARROW, &s2),
            None
        );
    }

    #[test]
    fn has_or_become_tailor_max_and_sticky() {
        // Haxe: lastProfession == TAILOR skips cap; count >= max+wasIdle refuses
        assert!(!has_or_become_tailor(false, 1, 1.0, 0.0));
        assert!(has_or_become_tailor(true, 1, 5.0, 0.0));
        assert!(has_or_become_tailor(false, 1, 0.0, 0.0));
        assert!(has_or_become_tailor(false, -1, 99.0, 0.0));
        assert!(has_or_become_tailor(false, 1, 1.0, 1.0));
        assert!(has_or_become_tailor(false, 100, 1.0, 0.0));
    }

    #[test]
    fn fill_up_quiver_search_radius_age_and_tailor() {
        assert_eq!(fill_up_quiver_search_radius(10.0, 0, false), 20);
        assert_eq!(fill_up_quiver_search_radius(19.9, 0, true), 20);
        assert_eq!(fill_up_quiver_search_radius(20.0, 0, false), 60);
        assert_eq!(fill_up_quiver_search_radius(25.0, 2, false), 20);
        assert_eq!(fill_up_quiver_search_radius(25.0, 2, true), 60);
        assert_eq!(fill_up_quiver_search_radius(25.0, 0, false), 60);
        assert!(is_fill_up_quiver_plan(ClothingCraftPlan::CraftItem(
            EMPTY_ARROW_QUIVER
        )));
        assert!(!is_fill_up_quiver_plan(ClothingCraftPlan::CraftItem(
            EMPTY_WATER_POUCH
        )));
    }

    #[test]
    fn person_color_from_race_table() {
        assert_eq!(person_color_from_race(1), PersonColor::Black);
        assert_eq!(person_color_from_race(4), PersonColor::White);
        assert_eq!(person_color_from_race(6), PersonColor::Ginger);
        assert_eq!(person_color_from_race(3), PersonColor::Brown);
    }
}
