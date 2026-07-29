//! Clothing transition matrix — equip, container-in-clothes, USE-on-worn.
//!
//! Chunk: **TH-CLOTHING-MATRIX** / `clothing_transitions`
//!
//! Haxe anchors:
//! - `ObjectData.getClothingSlot` / `isClothing`
//! - `GlobalPlayerInstance.doSelf` / `doSwitchCloths` / `tryTranstionOnClothing`
//! - `GlobalPlayerInstance.doPlaceObjInClothing` / `specialRemoveHelper`
//! - `TransitionHelper.drop` (clothingIndex ≥ 0) + `resetNumberOfUses` clothing rule
//! - `TransitionHelper.DoContainerStuffOnObj` when target is worn clothing

use crate::player::multi_use::{
    change_number_of_uses_on_actor, change_number_of_uses_on_target, reverse_target_exceeds_max,
    TargetUsesOutcome,
};
use crate::player::{ClothingSlot, Player, CLOTHING_SLOT_COUNT};
use ol_content::{ContentDb, Transition};
use ol_world::NestedHelper;

// ---------------------------------------------------------------------------
// Slot matrix (Haxe clothingObjects[6])
// ---------------------------------------------------------------------------

/// Haxe clothing index labels: 0=hat 1=tunic 2=frontShoe 3=backShoe 4=bottom 5=backpack.
pub const CLOTHING_INDEX_LABELS: [&str; CLOTHING_SLOT_COUNT] =
    ["hat", "tunic", "frontShoe", "backShoe", "bottom", "backpack"];

/// Haxe `ObjectData.getClothingSlot` — map `clothing` field first char → index, or `None`.
///
/// | char | slot |
/// |------|------|
/// | h    | 0 hat |
/// | t    | 1 tunic |
/// | s    | 2 shoes (front; dual-shoe fill may pick 3) |
/// | b    | 4 bottom |
/// | p    | 5 backpack |
/// | n / empty | not clothing |
#[inline]
pub fn get_clothing_slot_index(clothing: &str) -> Option<usize> {
    let c = clothing
        .trim()
        .chars()
        .next()
        .map(|ch| ch.to_ascii_lowercase())
        .unwrap_or('n');
    match c {
        'h' => Some(0),
        't' => Some(1),
        's' => Some(2),
        'b' => Some(4),
        'p' => Some(5),
        _ => None,
    }
}

/// True when content `clothing` marks a wearable (Haxe `isClothing`).
#[inline]
pub fn is_clothing_string(clothing: &str) -> bool {
    let t = clothing.trim();
    !t.is_empty() && !t.starts_with('n') && !t.starts_with('N')
}

/// Haxe: `resetNumberOfUses = !isClothing || numUses < 2`
///
/// Clothing multi-use (quivers etc.) must **not** reset uses when id changes.
#[inline]
pub fn allow_reset_uses_on_target(is_clothing: bool, num_uses: i32) -> bool {
    !is_clothing || num_uses < 2
}

/// Map clothing index → flat [`ClothingSlot`] for hat/chest/shoes (0..2 only).
#[inline]
pub fn index_to_live_slot(index: usize) -> Option<ClothingSlot> {
    match index {
        0 => Some(ClothingSlot::Hat),
        1 => Some(ClothingSlot::Chest),
        2 | 3 => Some(ClothingSlot::Shoes), // 3 = back shoe → shoes family
        _ => None,
    }
}

/// Preferred slot from content object (clothing field first, then name heuristics).
pub fn clothing_slot_from_def(name: &str, description: &str, clothing: &str) -> Option<usize> {
    if let Some(i) = get_clothing_slot_index(clothing) {
        return Some(i);
    }
    // Fallback: name / description heuristics (legacy WEAR path).
    let n = name.to_ascii_lowercase();
    if n.contains("hat") || n.contains("crown") || n.contains("mask") {
        return Some(0);
    }
    if n.contains("shoe") || n.contains("boot") {
        return Some(2);
    }
    if n.contains("backpack") || n.contains("quiver") || n.contains("pack") {
        return Some(5);
    }
    if n.contains("skirt") || n.contains("pants") || n.contains("bottom") || n.contains("trouser")
    {
        return Some(4);
    }
    if n.contains("chest")
        || n.contains("shirt")
        || n.contains("tunic")
        || n.contains("apron")
        || n.contains("coat")
    {
        return Some(1);
    }
    let d = description.to_ascii_lowercase();
    if d.contains("clothing=") {
        // Unknown clothing= code → chest default (legacy)
        return Some(1);
    }
    None
}

// ---------------------------------------------------------------------------
// doSwitchCloths slot resolution
// ---------------------------------------------------------------------------

/// Snapshot of the six clothing slot object ids (0 = empty).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClothingSlotIds {
    pub ids: [i32; CLOTHING_SLOT_COUNT],
}

impl ClothingSlotIds {
    pub fn from_player(p: &Player) -> Self {
        let mut ids = [0i32; CLOTHING_SLOT_COUNT];
        for i in 0..CLOTHING_SLOT_COUNT {
            ids[i] = p
                .clothing_helpers
                .get(i)
                .and_then(|h| h.as_ref())
                .map(|h| h.id)
                .filter(|&id| id > 0)
                .unwrap_or(0);
        }
        // Flat fields win for 0..2 when helpers empty.
        if ids[0] == 0 {
            ids[0] = p.hat;
        }
        if ids[1] == 0 {
            ids[1] = p.chest;
        }
        if ids[2] == 0 {
            ids[2] = p.shoes;
        }
        Self { ids }
    }

    pub fn get(self, i: usize) -> i32 {
        self.ids.get(i).copied().unwrap_or(0)
    }
}

/// Haxe `doSwitchCloths` slot pick (dual shoe + object-preferred slot).
///
/// - Shoes (`objClothingSlot == 2`): fill empty foot (prefer 3 if 2 occupied and 3 empty).
/// - Non-shoe with known slot: use object slot unless client forced shoe index 2/3.
///
/// Returns `None` when equip is impossible (bad slot / type mismatch).
pub fn resolve_switch_slot(
    held_id: i32,
    obj_clothing_slot: Option<usize>,
    requested: i32,
    worn: ClothingSlotIds,
) -> Option<usize> {
    let mut clothing_slot = requested;
    let obj = obj_clothing_slot.map(|u| u as i32);

    // Dual shoe fill (Haxe L3536–3538). Note: Haxe condition uses `||` which is always true
    // when clothingSlot is any int (`!=2 || !=3`); port behavior as written: always enter shoe branch.
    if obj == Some(2) {
        if worn.get(2) != 0 && worn.get(3) == 0 {
            clothing_slot = 3;
        } else {
            clothing_slot = 2;
        }
    } else if let Some(ocs) = obj {
        // Non-shoe: use object slot unless client requested a shoe index.
        if clothing_slot != 2 && clothing_slot != 3 {
            clothing_slot = ocs;
        }
    }

    if clothing_slot < 0 {
        return None;
    }
    let slot = clothing_slot as usize;
    if slot >= CLOTHING_SLOT_COUNT {
        return None;
    }

    // Haxe: tmpClothingSlot = clothingSlot==3 ? 2 : clothingSlot; reject type mismatch.
    if held_id != 0 {
        let Some(ocs) = obj_clothing_slot else {
            // Holding non-clothing — switch fails (place-into-container may still work).
            return None;
        };
        let tmp = if slot == 3 { 2 } else { slot };
        if tmp != ocs {
            return None;
        }
    }

    Some(slot)
}

/// Max age for clothing others (Haxe `MaxAgeForAllowingClothAndPrickupFromOthers`).
/// Default matches common ServerSettings (often ~5–10); use 10 as product default.
pub const MAX_AGE_CLOTH_OTHERS: f32 = 10.0;

/// Validate other-player clothing age gate (self always ok).
#[inline]
pub fn other_player_accepts_cloth(target_age: f32, max_age: f32) -> bool {
    target_age <= max_age
}

/// Crown / mask flavor lines after equip (Haxe doSwitchCloths).
pub fn crown_say_line(clothing_parent_id: i32) -> Option<&'static str> {
    match clothing_parent_id {
        695 => Some("I am almighty Wolf King!"),
        694 => Some("I am King of the Forests!"),
        693 => Some("I am King of the Carrots!"),
        3213 => Some("I am burning fire!"),
        3214 => Some("I am freezing water!"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// tryTranstionOnClothing (pure multi-use on worn target)
// ---------------------------------------------------------------------------

/// Inputs for Haxe `tryTranstionOnClothing`.
#[derive(Debug, Clone)]
pub struct ClothingTransitionIn {
    pub held_parent_id: i32,
    pub held_uses: i32,
    pub held_num_uses: i32,
    pub clothing_parent_id: i32,
    pub clothing_uses: i32,
    pub clothing_num_uses: i32,
    pub clothing_slot_old: Option<usize>,
    /// New target clothing slot from content (`getClothingSlot` on newTarget).
    pub new_target_clothing_slot: Option<usize>,
    pub new_target_parent_id: i32,
    pub reverse_use_target: bool,
    pub reverse_use_actor: bool,
    pub no_use_target: bool,
    pub no_use_actor: bool,
    pub new_actor_id: i32,
    pub new_target_id: i32,
    pub new_actor_num_uses: i32,
    pub new_target_num_uses: i32,
}

/// Result of applying a transition onto worn clothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClothingTransitionOut {
    pub clothing_id: i32,
    pub clothing_uses: i32,
    pub held_id: i32,
    pub held_uses: i32,
    /// Jinbali easter egg (pile of sheep skins 3919).
    pub praise_jinbali: bool,
}

/// Haxe `tryTranstionOnClothing` core (no ContentDb — pure).
///
/// Slot must match unless newTarget is pile-of-sheep-skins 3919.
/// Reverse-use refuse when clothing already at max uses.
pub fn try_transition_on_clothing_pure(
    inp: &ClothingTransitionIn,
) -> Option<ClothingTransitionOut> {
    if inp.clothing_parent_id == 0 {
        return None;
    }
    let praise = inp.new_target_parent_id == 3919;
    if inp.clothing_slot_old != inp.new_target_clothing_slot && !praise {
        return None;
    }
    if inp.clothing_num_uses > 1
        && inp.reverse_use_target
        && inp.clothing_uses >= inp.clothing_num_uses
    {
        return None;
    }
    if reverse_target_exceeds_max(inp.clothing_uses, inp.new_target_num_uses)
        && inp.reverse_use_target
    {
        // Pure path: caller may swap to max-use table; we refuse here.
        return None;
    }

    // Haxe: resetNumberOfUses = clothing.numUses < 2 (clothing always multi-use suppress when ≥2)
    let allow_reset = inp.clothing_num_uses < 2;
    let cloth_out = change_number_of_uses_on_target(
        inp.clothing_parent_id,
        inp.new_target_id,
        inp.clothing_uses,
        inp.clothing_num_uses,
        inp.new_target_num_uses,
        inp.reverse_use_target,
        inp.no_use_target,
        true,
        allow_reset,
    );
    let (clothing_id, clothing_uses) = match cloth_out {
        TargetUsesOutcome::Cleared => (0, 0),
        TargetUsesOutcome::Simple => (inp.new_target_id, 0),
        TargetUsesOutcome::Uses(u) => (inp.new_target_id, u),
    };

    let held_effective = if inp.held_uses > 0 {
        inp.held_uses
    } else if inp.held_num_uses >= 2 {
        inp.held_num_uses
    } else {
        0
    };
    let actor_out = change_number_of_uses_on_actor(
        inp.held_parent_id,
        inp.new_actor_id,
        held_effective,
        inp.new_actor_num_uses,
        inp.reverse_use_actor,
        inp.no_use_actor,
    );

    Some(ClothingTransitionOut {
        clothing_id,
        clothing_uses,
        held_id: actor_out.held_id,
        held_uses: actor_out.held_uses,
        praise_jinbali: praise,
    })
}

/// Look up transition held → worn clothing and apply pure rules.
pub fn try_transition_on_clothing_with_content(
    content: &ContentDb,
    held_id: i32,
    held_uses: i32,
    clothing_id: i32,
    clothing_uses: i32,
    clothing_is_last_use: bool,
) -> Option<ClothingTransitionOut> {
    if held_id == 0 || clothing_id == 0 {
        return None;
    }
    let held_base = content.resolve_base_id(held_id);
    let cloth_base = content.resolve_base_id(clothing_id);
    let tr: Transition = if clothing_is_last_use {
        content
            .find_transition_last_use(held_base, cloth_base)
            .or_else(|| content.find_transition(held_base, cloth_base))?
            .clone()
    } else {
        content
            .find_transition_prefer(
                held_base,
                cloth_base,
                clothing_uses > 0
                    && content
                        .get(cloth_base)
                        .map(|d| d.num_uses >= 2 && clothing_uses < d.num_uses)
                        .unwrap_or(false),
            )?
            .clone()
    };

    let cloth_def = content.get(cloth_base);
    let new_tgt = content.get(tr.new_target_id);
    let new_act = content.get(tr.new_actor_id);
    let clothing_num = cloth_def.map(|d| d.num_uses).unwrap_or(0);
    let clothing_uses = if clothing_uses > 0 {
        clothing_uses
    } else if clothing_num >= 2 {
        clothing_num
    } else {
        0
    };
    let held_num = content.get(held_base).map(|d| d.num_uses).unwrap_or(0);

    let clothing_slot_old = cloth_def.and_then(|d| get_clothing_slot_index(&d.clothing));
    let new_target_clothing_slot = new_tgt.and_then(|d| get_clothing_slot_index(&d.clothing));
    let new_target_parent = content.resolve_base_id(tr.new_target_id);

    let inp = ClothingTransitionIn {
        held_parent_id: held_base,
        held_uses,
        held_num_uses: held_num,
        clothing_parent_id: cloth_base,
        clothing_uses,
        clothing_num_uses: clothing_num,
        clothing_slot_old,
        new_target_clothing_slot,
        new_target_parent_id: new_target_parent,
        reverse_use_target: tr.reverse_use_target,
        reverse_use_actor: tr.reverse_use_actor,
        no_use_target: tr.no_use_target,
        no_use_actor: tr.no_use_actor,
        new_actor_id: tr.new_actor_id,
        new_target_id: tr.new_target_id,
        new_actor_num_uses: new_act.map(|d| d.num_uses).unwrap_or(0),
        new_target_num_uses: new_tgt.map(|d| d.num_uses).unwrap_or(0),
    };
    try_transition_on_clothing_pure(&inp)
}

// ---------------------------------------------------------------------------
// Container-in-clothing (doPlaceObjInClothing / SREMV)
// ---------------------------------------------------------------------------

/// Whether held can enter worn clothing as container (Haxe DoContainerStuffOnObj put path).
///
/// `held_contain_size` / `clothing_slot_size`: Haxe containSize/slotSize gate
/// (`containSize > slotSize` refuse). Defaults that always fit: 0.0 / 1.0.
/// // Haxe: TransitionHelper.DoContainerStuffOnObj containSize > slotSize
pub fn can_put_into_clothing(
    clothing_id: i32,
    clothing_num_slots: i32,
    clothing_contained_len: usize,
    held_id: i32,
    held_containable: bool,
    is_drop: bool,
) -> bool {
    can_put_into_clothing_sized(
        clothing_id,
        clothing_num_slots,
        clothing_contained_len,
        held_id,
        held_containable,
        is_drop,
        0.0,
        1.0,
    )
}

/// Like [`can_put_into_clothing`] with explicit Haxe containSize/slotSize.
pub fn can_put_into_clothing_sized(
    clothing_id: i32,
    clothing_num_slots: i32,
    clothing_contained_len: usize,
    held_id: i32,
    held_containable: bool,
    is_drop: bool,
    held_contain_size: f32,
    clothing_slot_size: f32,
) -> bool {
    if clothing_id == 0 || clothing_num_slots <= 0 {
        return false;
    }
    if held_id == 0 {
        // Empty hand: take path handled separately.
        return false;
    }
    if !held_containable {
        return false;
    }
    // Haxe: if (objToStoreObjData.containSize > containerObjData.slotSize) return false;
    if !crate::death_polish::contain_fits_slot(held_contain_size, clothing_slot_size) {
        return false;
    }
    if is_drop {
        // Drop always swaps/inserts (even when full — pops last), after size gate.
        return true;
    }
    clothing_contained_len < clothing_num_slots as usize
}

/// Haxe `DoContainerStuffOnObj` empty-hand default: `if (index < 0) index = 0` (first slot).
///
/// Contrast [`sremv_resolved_index`]: SREMV protocol `-1` = top of stack (last / pop).
#[inline]
pub fn empty_hand_container_take_index(index: i32) -> i32 {
    if index < 0 {
        0
    } else {
        index
    }
}

/// Haxe `ObjectHelper.removeContainedObject`: `index < 0` → pop last (SREMV top-of-stack).
#[inline]
pub fn sremv_resolved_index(index: i32, contained_len: usize) -> Option<usize> {
    if contained_len == 0 {
        return None;
    }
    if index < 0 {
        Some(contained_len - 1)
    } else {
        let i = index as usize;
        if i < contained_len {
            Some(i)
        } else {
            None
        }
    }
}

/// Haxe permanent-contained refuse: cannot pickup permanent objects from container.
#[inline]
pub fn refuse_take_permanent_contained(is_permanent: bool) -> bool {
    is_permanent
}

/// Pure put: append held into clothing nest (non-drop). Returns new clothing helper + cleared held.
pub fn put_into_clothing_nest(
    clothing: NestedHelper,
    held: NestedHelper,
    num_slots: usize,
    is_drop: bool,
) -> Option<(NestedHelper, Option<NestedHelper>)> {
    if clothing.is_empty() || held.is_empty() {
        return None;
    }
    let mut cloth = clothing;
    if is_drop {
        // Haxe drop: insert at 0, previous top becomes held.
        let prev = if cloth.contained.is_empty() {
            None
        } else {
            Some(cloth.contained.remove(cloth.contained.len() - 1))
        };
        cloth.contained.insert(0, held);
        // Soft cap: if over slots, drop oldest tail (defensive).
        if num_slots > 0 && cloth.contained.len() > num_slots {
            cloth.contained.truncate(num_slots);
        }
        Some((cloth, prev))
    } else {
        if num_slots > 0 && cloth.contained.len() >= num_slots {
            return None;
        }
        cloth.contained.push(held);
        Some((cloth, None))
    }
}

/// Pure take from clothing nest.
///
/// - `index < 0` → **last** (Haxe `removeContainedObject` pop / SREMV top).
/// - For empty-hand USE/place container path, callers must pass
///   [`empty_hand_container_take_index`] first (Haxe defaults to **0** / first).
pub fn take_from_clothing_nest(
    clothing: NestedHelper,
    index: i32,
) -> Option<(NestedHelper, NestedHelper)> {
    if clothing.is_empty() || clothing.contained.is_empty() {
        return None;
    }
    let mut cloth = clothing;
    let idx = sremv_resolved_index(index, cloth.contained.len())?;
    let taken = cloth.contained.remove(idx);
    Some((cloth, taken))
}

/// Take with permanent refuse (Haxe DoContainerStuffOnObj empty-hand permanent check).
///
/// `is_permanent` is looked up on the candidate contained object id.
pub fn take_from_clothing_nest_checked(
    clothing: NestedHelper,
    index: i32,
    is_permanent: impl Fn(i32) -> bool,
) -> Option<(NestedHelper, NestedHelper)> {
    if clothing.is_empty() || clothing.contained.is_empty() {
        return None;
    }
    let idx = sremv_resolved_index(index, clothing.contained.len())?;
    let cand_id = clothing.contained[idx].id;
    if refuse_take_permanent_contained(is_permanent(cand_id)) {
        return None;
    }
    take_from_clothing_nest(clothing, index)
}

// ---------------------------------------------------------------------------
// doSelf drink (before clothing) — Bowl of Water / Full Water Pouch
// ---------------------------------------------------------------------------

/// Bowl of Water (Haxe drink heldId 382).
pub const WATER_BOWL_ID: i32 = 382;
/// Full Water Pouch (Haxe drink heldId 210).
pub const WATER_POUCH_ID: i32 = 210;
/// Clay Bowl after drink (235).
pub const EMPTY_BOWL_ID: i32 = 235;
/// Empty Water Pouch after drink (209).
pub const EMPTY_POUCH_ID: i32 = 209;
/// Haxe `ServerSettings.TemperatureReductionPerDrinking` default.
pub const TEMP_REDUCTION_PER_DRINK: f32 = 0.5;
/// Haxe `ServerSettings.MaxStoredWater` default.
pub const MAX_STORED_WATER: f32 = 1.0;

/// Pure inputs for Haxe `doSelf` → `drink()`.
#[derive(Debug, Clone, Copy)]
pub struct DrinkWaterIn {
    pub held_parent_id: i32,
    pub heat: f32,
    pub stored_water: f32,
    pub temp_reduction: f32,
    pub max_stored_water: f32,
}

/// Result of drinking water bowl/pouch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrinkWaterOut {
    pub empty_held_id: i32,
    pub heat: f32,
    pub stored_water: f32,
}

/// Haxe `GlobalPlayerInstance.drink` — water bowl/pouch only; pure heat/storage.
// Haxe: GlobalPlayerInstance.drink
pub fn try_drink_water_pure(inp: &DrinkWaterIn) -> Option<DrinkWaterOut> {
    let empty = match inp.held_parent_id {
        WATER_BOWL_ID => EMPTY_BOWL_ID,
        WATER_POUCH_ID => EMPTY_POUCH_ID,
        _ => return None,
    };
    let original = if inp.temp_reduction > 0.0 {
        inp.temp_reduction
    } else {
        TEMP_REDUCTION_PER_DRINK
    };
    let max_w = if inp.max_stored_water > 0.0 {
        inp.max_stored_water
    } else {
        MAX_STORED_WATER
    };
    let mut heat = inp.heat;
    let mut water = original;
    let mut stored = inp.stored_water;

    if heat > 0.4 {
        let too_much = heat - 0.2;
        if too_much > water {
            heat -= water;
            stored = (stored + original / 2.0).min(max_w);
            return Some(DrinkWaterOut {
                empty_held_id: empty,
                heat,
                stored_water: stored,
            });
        } else {
            heat = 0.2;
            water -= too_much;
            stored = (stored + water / 2.0 + original / 2.0).min(max_w);
            return Some(DrinkWaterOut {
                empty_held_id: empty,
                heat,
                stored_water: stored,
            });
        }
    }

    if stored >= max_w {
        return None;
    }
    stored = (stored + water / 2.0 + original / 2.0).min(max_w);
    Some(DrinkWaterOut {
        empty_held_id: empty,
        heat,
        stored_water: stored,
    })
}

/// Live drink on player (held water → empty vessel + heat/storage).
// Haxe: GlobalPlayerInstance.drink (doSelf first branch)
pub fn apply_drink_self(player: &mut Player, content: &ContentDb) -> bool {
    let held_base = content.resolve_base_id(player.held_id);
    let inp = DrinkWaterIn {
        held_parent_id: held_base,
        heat: player.heat,
        stored_water: player.stored_water,
        temp_reduction: TEMP_REDUCTION_PER_DRINK,
        max_stored_water: MAX_STORED_WATER,
    };
    let Some(out) = try_drink_water_pure(&inp) else {
        return false;
    };
    player.heat = out.heat;
    player.stored_water = out.stored_water;
    player.set_held(out.empty_held_id, 0);
    true
}

// ---------------------------------------------------------------------------
// Player mutators (live nest + flat ids)
// ---------------------------------------------------------------------------

/// Haxe `doSwitchCloths` — equip held into resolved index, swap previous to hands.
pub fn apply_switch_cloths(
    player: &mut Player,
    content: &ContentDb,
    requested_slot: i32,
) -> Result<(i32, i32, Option<&'static str>), &'static str> {
    if player.held_id < 0 {
        return Err("HOLDING_PLAYER");
    }
    // Clear light wound held alias like Haxe (hiddenWound == held → null hands).
    if player.is_holding_hidden_wound() {
        player.clear_held();
    }
    let held_id = player.held_id;
    if held_id == 0 {
        return Err("EMPTY");
    }
    let def = content.get(content.resolve_base_id(held_id));
    let obj_slot = def.and_then(|d| clothing_slot_from_def(&d.name, &d.description, &d.clothing));
    let worn = ClothingSlotIds::from_player(player);
    let slot = resolve_switch_slot(held_id, obj_slot, requested_slot, worn).ok_or("SLOT")?;
    let (new_id, prev_id) = switch_clothing_index_full(player, slot)?;
    let say = crown_say_line(content.resolve_base_id(new_id));
    Ok((new_id, prev_id, say))
}

/// Haxe `doSwitchCloths(playerFrom, playerTo, clothingSlot)` when From ≠ To (UBABY cloth).
///
/// Age gate: target must be ≤ `max_age` ([`MAX_AGE_CLOTH_OTHERS`] default).
// Haxe: GlobalPlayerInstance.doSwitchCloths other-player branch
pub fn apply_switch_cloths_on_other(
    player_from: &mut Player,
    player_to: &mut Player,
    content: &ContentDb,
    requested_slot: i32,
    max_age: f32,
) -> Result<(i32, i32, Option<&'static str>), &'static str> {
    if player_from.held_id < 0 {
        return Err("HOLDING_PLAYER");
    }
    if player_from.is_holding_hidden_wound() {
        player_from.clear_held();
    }
    if player_from.held_id == 0 {
        return Err("EMPTY");
    }
    if !other_player_accepts_cloth(player_to.age, max_age) {
        return Err("TOO_OLD");
    }
    let held_id = player_from.held_id;
    let def = content.get(content.resolve_base_id(held_id));
    let obj_slot = def.and_then(|d| clothing_slot_from_def(&d.name, &d.description, &d.clothing));
    let worn = ClothingSlotIds::from_player(player_to);
    let slot = resolve_switch_slot(held_id, obj_slot, requested_slot, worn).ok_or("SLOT")?;

    // Move held from `from` onto `to` clothing index (swap previous onto from hands).
    let held = player_from
        .held_helper
        .take()
        .unwrap_or_else(|| NestedHelper::with_uses(player_from.held_id, player_from.held_uses));
    let _held_id = held.id;
    player_from.clear_held();

    // Temporarily set held on target, switch, then put previous into from hands.
    player_to.set_held_helper(held);
    let (new_id, prev_id) = match switch_clothing_index_full(player_to, slot) {
        Ok(v) => v,
        Err(e) => {
            // Restore held to from on failure.
            if let Some(h) = player_to.held_helper.take() {
                player_from.set_held_helper(h);
            } else if player_to.held_id != 0 {
                player_from.set_held(player_to.held_id, player_to.held_uses);
                player_to.clear_held();
            }
            return Err(e);
        }
    };
    // After switch, previous clothing is in player_to hands — give to player_from.
    if player_to.held_id != 0 || player_to.held_helper.is_some() {
        let prev = player_to
            .held_helper
            .take()
            .unwrap_or_else(|| NestedHelper::with_uses(player_to.held_id, player_to.held_uses));
        player_to.clear_held();
        if !prev.is_empty() {
            player_from.set_held_helper(prev);
        }
    }
    let say = crown_say_line(content.resolve_base_id(new_id));
    Ok((new_id, prev_id, say))
}

/// Equip/swap at absolute index 0..5 (includes bottom/backpack).
// Haxe: doSwitchCloths clothingObjects[slot] swap
pub fn switch_clothing_index_full(
    player: &mut Player,
    index: usize,
) -> Result<(i32, i32), &'static str> {
    if index >= CLOTHING_SLOT_COUNT {
        return Err("BAD");
    }
    if player.held_id == 0 {
        return Err("EMPTY");
    }
    if index <= 2 {
        let slot = match index {
            0 => ClothingSlot::Hat,
            1 => ClothingSlot::Chest,
            _ => ClothingSlot::Shoes,
        };
        return player.wear_held(slot);
    }
    // Slot 3 (back shoe) / 4 bottom / 5 backpack: independent helpers.
    let held = player
        .held_helper
        .take()
        .unwrap_or_else(|| NestedHelper::with_uses(player.held_id, player.held_uses));
    let held_id = held.id;
    let prev = player.clothing_helpers[index].take();
    let prev_id = prev.as_ref().map(|h| h.id).filter(|id| *id > 0).unwrap_or(0);
    player.clothing_helpers[index] = Some(held);
    match prev {
        Some(p) if !p.is_empty() => player.set_held_helper(p),
        _ => player.clear_held(),
    }
    Ok((held_id, prev_id))
}

/// Haxe `tryTranstionOnClothing` live apply on player clothing index.
pub fn apply_transition_on_clothing(
    player: &mut Player,
    content: &ContentDb,
    clothing_slot: i32,
) -> Result<ClothingTransitionOut, &'static str> {
    if clothing_slot < 0 || clothing_slot as usize >= CLOTHING_SLOT_COUNT {
        return Err("BAD");
    }
    let idx = clothing_slot as usize;
    let cloth = player.clothing_helpers[idx]
        .clone()
        .or_else(|| {
            let id = match idx {
                0 => player.hat,
                1 => player.chest,
                2 => player.shoes,
                _ => 0,
            };
            if id > 0 {
                Some(NestedHelper::id_only(id))
            } else {
                None
            }
        })
        .ok_or("EMPTY_CLOTH")?;
    if cloth.is_empty() {
        return Err("EMPTY_CLOTH");
    }
    let cloth_uses = cloth.uses_remaining;
    let cloth_num = content
        .get(content.resolve_base_id(cloth.id))
        .map(|d| d.num_uses)
        .unwrap_or(0);
    let is_last = cloth_num >= 2 && cloth_uses > 0 && cloth_uses < cloth_num;
    let out = try_transition_on_clothing_with_content(
        content,
        player.held_id,
        player.held_uses,
        cloth.id,
        cloth_uses,
        is_last,
    )
    .ok_or("NO_TRANS")?;

    // Write clothing result.
    let mut new_cloth = cloth;
    new_cloth.id = out.clothing_id;
    new_cloth.uses_remaining = out.clothing_uses;
    // Haxe: clothing.TransformToDummy() after id / uses change (tryTranstionOnClothing).
    // // Haxe: ObjectHelper.TransformToDummy
    if out.clothing_id != 0 {
        let base = content.resolve_base_id(out.clothing_id);
        if let Some(def) = content.get(base) {
            if def.num_uses >= 2 {
                crate::nested_body::apply_transform_to_dummy_on_helper(
                    &mut new_cloth,
                    def.num_uses,
                    0,
                    0,
                    !def.dummy_ids.is_empty() && out.clothing_id != base,
                    base,
                    &def.dummy_ids,
                );
            }
        }
    }
    if out.clothing_id == 0 {
        player.clothing_helpers[idx] = None;
        match idx {
            0 => player.hat = 0,
            1 => player.chest = 0,
            2 => player.shoes = 0,
            _ => {}
        }
    } else {
        player.set_clothing_index_helper(idx, Some(new_cloth));
    }
    // Write held result.
    if out.held_id == 0 {
        player.clear_held();
    } else {
        player.set_held(out.held_id, out.held_uses);
    }
    Ok(out)
}

/// Haxe `doPlaceObjInClothing` — put/swap held into worn clothing container.
pub fn apply_place_obj_in_clothing(
    player: &mut Player,
    content: &ContentDb,
    clothing_slot: i32,
    is_drop: bool,
) -> Result<(), &'static str> {
    if clothing_slot < 0 || clothing_slot as usize >= CLOTHING_SLOT_COUNT {
        return Err("BAD");
    }
    let idx = clothing_slot as usize;
    let cloth = player.clothing_helpers[idx]
        .clone()
        .or_else(|| {
            let id = match idx {
                0 => player.hat,
                1 => player.chest,
                2 => player.shoes,
                _ => 0,
            };
            if id > 0 {
                Some(NestedHelper::id_only(id))
            } else {
                None
            }
        })
        .ok_or("EMPTY_CLOTH")?;
    if cloth.is_empty() {
        return Err("EMPTY_CLOTH");
    }
    let num_slots = content
        .get(content.resolve_base_id(cloth.id))
        .map(|d| d.num_slots.max(0) as usize)
        .unwrap_or(0);
    if num_slots == 0 {
        return Err("NOT_CONTAINER");
    }

    // Empty hands: Haxe DoContainerStuffOnObj takes **first** (index 0), not last.
    // // Haxe: TransitionHelper.DoContainerStuffOnObj L609–617
    if player.held_id == 0 {
        if is_drop {
            return Err("EMPTY");
        }
        let take_i = empty_hand_container_take_index(-1);
        let content_ref = content;
        let (new_cloth, taken) = take_from_clothing_nest_checked(cloth, take_i, |id| {
            content_ref
                .get(content_ref.resolve_base_id(id))
                .map(|d| d.permanent)
                .unwrap_or(false)
        })
        .ok_or("EMPTY_SLOT")?;
        player.set_clothing_index_helper(idx, Some(new_cloth));
        player.set_held_helper(taken);
        return Ok(());
    }

    let held_base = content.resolve_base_id(player.held_id);
    let cloth_base = content.resolve_base_id(cloth.id);
    let held_def = content.get(held_base);
    let held_containable = held_def.map(|d| d.containable).unwrap_or(false);
    let held_contain_size = held_def.map(|d| d.contain_size).unwrap_or(0.0);
    let clothing_slot_size = content
        .get(cloth_base)
        .map(|d| d.slot_size)
        .unwrap_or(1.0);
    // Haxe DoContainerStuffOnObj: containSize > slotSize refuse (also on DROP swap).
    if !can_put_into_clothing_sized(
        cloth.id,
        num_slots as i32,
        cloth.contained.len(),
        player.held_id,
        held_containable,
        is_drop,
        held_contain_size,
        clothing_slot_size,
    ) {
        return Err("FULL_OR_BAD");
    }
    let held = player
        .held_helper
        .take()
        .unwrap_or_else(|| NestedHelper::with_uses(player.held_id, player.held_uses));
    let (new_cloth, prev) = put_into_clothing_nest(cloth, held, num_slots, is_drop).ok_or("PUT")?;
    player.set_clothing_index_helper(idx, Some(new_cloth));
    match prev {
        Some(p) => player.set_held_helper(p),
        None => player.clear_held(),
    }
    Ok(())
}

/// Haxe `specialRemoveHelper` — SREMV from clothing; empty nest → strip clothing into hands.
pub fn apply_sremv_from_clothing(
    player: &mut Player,
    clothing_slot: i32,
    index: Option<i32>,
) -> Result<i32, &'static str> {
    if clothing_slot < 0 || clothing_slot as usize >= CLOTHING_SLOT_COUNT {
        return Err("BAD");
    }
    if player.held_id < 0 {
        return Err("HOLDING_PLAYER");
    }
    if player.is_holding_hidden_wound() {
        player.clear_held();
    }
    let idx = clothing_slot as usize;
    let cloth = player.clothing_helpers[idx].clone().ok_or("EMPTY_CLOTH")?;
    if cloth.is_empty() {
        return Err("EMPTY_CLOTH");
    }
    if cloth.contained.is_empty() {
        // Haxe: empty nest → doSwitchCloths (strip/swap clothing).
        if player.held_id != 0 {
            return switch_clothing_index_full(player, idx).map(|(id, _)| id);
        }
        let taken = player.clothing_helpers[idx].take().unwrap();
        match idx {
            0 => player.hat = 0,
            1 => player.chest = 0,
            2 => player.shoes = 0,
            _ => {}
        }
        let id = taken.id;
        player.set_held_helper(taken);
        return Ok(id);
    }
    if player.held_id != 0 {
        return Err("HANDS");
    }
    // SREMV: -1 = top of stack (last). Permanent contained refuse when content known.
    let idx_i = index.unwrap_or(-1);
    let (new_cloth, taken) = take_from_clothing_nest(cloth, idx_i).ok_or("EMPTY_SLOT")?;
    let id = taken.id;
    player.set_clothing_index_helper(idx, Some(new_cloth));
    player.set_held_helper(taken);
    Ok(id)
}

/// Haxe `specialRemoveHelper` with permanent check when `content` available.
// Haxe: TransitionHelper.DoContainerStuffOnObj permanent refuse
pub fn apply_sremv_from_clothing_with_content(
    player: &mut Player,
    content: &ContentDb,
    clothing_slot: i32,
    index: Option<i32>,
) -> Result<i32, &'static str> {
    if clothing_slot < 0 || clothing_slot as usize >= CLOTHING_SLOT_COUNT {
        return Err("BAD");
    }
    if player.held_id < 0 {
        return Err("HOLDING_PLAYER");
    }
    if player.is_holding_hidden_wound() {
        player.clear_held();
    }
    let idx = clothing_slot as usize;
    let cloth = player.clothing_helpers[idx].clone().ok_or("EMPTY_CLOTH")?;
    if cloth.is_empty() {
        return Err("EMPTY_CLOTH");
    }
    if cloth.contained.is_empty() {
        return apply_sremv_from_clothing(player, clothing_slot, index);
    }
    if player.held_id != 0 {
        return Err("HANDS");
    }
    let idx_i = index.unwrap_or(-1);
    let (new_cloth, taken) = take_from_clothing_nest_checked(cloth, idx_i, |id| {
        content
            .get(content.resolve_base_id(id))
            .map(|d| d.permanent)
            .unwrap_or(false)
    })
    .ok_or("EMPTY_SLOT")?;
    let id = taken.id;
    player.set_clothing_index_helper(idx, Some(new_cloth));
    player.set_held_helper(taken);
    Ok(id)
}

/// Haxe `doSelf` path: drink → (eat residual) → clothing transition → switch → place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfClothingPath {
    Drink,
    Transition,
    Switch,
    Place,
}

/// Full `doSelf` clothing branch after optional drink (Haxe order).
// Haxe: GlobalPlayerInstance.doSelf L2718–2742
pub fn apply_self_clothing(
    player: &mut Player,
    content: &ContentDb,
    clothing_slot: i32,
) -> Result<(SelfClothingPath, Option<&'static str>), &'static str> {
    if player.held_id < 0 {
        return Err("HOLDING_PLAYER");
    }
    if player.is_holding_hidden_wound() {
        player.clear_held();
    }
    // drink() first (water bowl/pouch) — before clothing.
    if apply_drink_self(player, content) {
        return Ok((SelfClothingPath::Drink, None));
    }
    // tryTranstionOnClothing
    if clothing_slot >= 0 {
        if let Ok(out) = apply_transition_on_clothing(player, content, clothing_slot) {
            let say = if out.praise_jinbali {
                Some("Praise Jinbali!")
            } else {
                None
            };
            return Ok((SelfClothingPath::Transition, say));
        }
    }
    // doSwitchCloths
    if let Ok((_id, _prev, say)) = apply_switch_cloths(player, content, clothing_slot) {
        return Ok((SelfClothingPath::Switch, say));
    }
    // doPlaceObjInClothing
    apply_place_obj_in_clothing(player, content, clothing_slot, false)
        .map(|_| (SelfClothingPath::Place, None))
}

/// Format one clothing nest like Haxe `ObjectHelper.toString` (colon sub-nest).
///
/// - bare: `198`
/// - flat: `198,33,40`
/// - sub-nest: `198,33:100:101,40`
// Haxe: ObjectHelper.toString
pub fn format_clothing_helper_string(h: &NestedHelper) -> String {
    if h.is_empty() {
        return "0".to_string();
    }
    let mut s = h.id.to_string();
    for c in &h.contained {
        s.push(',');
        s.push_str(&c.id.to_string());
        for sub in &c.contained {
            s.push(':');
            s.push_str(&sub.id.to_string());
        }
    }
    s
}

/// Format Haxe `clothing_set` style string from six slot helpers.
// Haxe: GlobalPlayerInstance.setInClothingSet / clothing_set
pub fn format_clothing_set(player: &Player) -> String {
    let worn = ClothingSlotIds::from_player(player);
    let mut parts = Vec::with_capacity(6);
    for i in 0..CLOTHING_SLOT_COUNT {
        let id = worn.get(i);
        if id == 0 {
            parts.push("0".to_string());
            continue;
        }
        if let Some(h) = player.clothing_helpers[i].as_ref() {
            parts.push(format_clothing_helper_string(h));
        } else {
            parts.push(id.to_string());
        }
    }
    parts.join(";")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ol_content::{ObjectDef, Transition};

    fn def_cloth(id: i32, clothing: &str, num_slots: i32, num_uses: i32, containable: bool) -> ObjectDef {
        ObjectDef {
            id,
            description: format!("cloth{id}"),
            name: format!("Cloth{id}"),
            containable,
            permanent: false,
            blocks_walking: false,
            food_value: 0,
            heat_value: 0.0,
            map_chance: 0.0,
            biomes: Vec::new(),
            num_uses,
            num_slots,
            floor: false,
            dummy_ids: Vec::new(),
            use_chance: 0.0,
            speed_mult: 1.0,
            winter_decay_factor: 0.0,
            spring_regrow_factor: 0.0,
            decay_factor: 1.0,
            decays_to_obj: 0,
            r_value: 0.0,
            clothing: clothing.into(),
            counts_or_grows_as: 0,
            crafting_steps: 0,
        use_distance: 1,
        deadly_distance: 0.0,
        moves: 0,
        damage: 0.0,
        damage_protection_factor: 1.0,
        wound_factor: 0.5,
        male: false,
        contain_size: 0.0,
        slot_size: 1.0,
        }
    }

    #[test]
    fn get_clothing_slot_matrix() {
        assert_eq!(get_clothing_slot_index("h"), Some(0));
        assert_eq!(get_clothing_slot_index("t"), Some(1));
        assert_eq!(get_clothing_slot_index("s"), Some(2));
        assert_eq!(get_clothing_slot_index("b"), Some(4));
        assert_eq!(get_clothing_slot_index("p"), Some(5));
        assert_eq!(get_clothing_slot_index("n"), None);
        assert_eq!(get_clothing_slot_index(""), None);
        assert_eq!(get_clothing_slot_index("H "), Some(0));
        assert!(is_clothing_string("h"));
        assert!(!is_clothing_string("n"));
    }

    #[test]
    fn reset_uses_clothing_rule() {
        assert!(!allow_reset_uses_on_target(true, 5));
        assert!(allow_reset_uses_on_target(true, 1));
        assert!(allow_reset_uses_on_target(false, 5));
    }

    #[test]
    fn dual_shoe_prefers_empty_foot() {
        let mut worn = ClothingSlotIds { ids: [0; 6] };
        worn.ids[2] = 100; // front occupied
        assert_eq!(
            resolve_switch_slot(200, Some(2), -1, worn),
            Some(3),
            "fill back shoe when front full"
        );
        worn.ids[3] = 101;
        assert_eq!(
            resolve_switch_slot(200, Some(2), -1, worn),
            Some(2),
            "default front when both considered"
        );
        assert_eq!(resolve_switch_slot(50, Some(0), -1, worn), Some(0));
        assert_eq!(resolve_switch_slot(200, Some(2), 0, worn), Some(2));
        assert_eq!(resolve_switch_slot(300, Some(5), 2, worn), None);
    }

    #[test]
    fn crown_lines() {
        assert!(crown_say_line(695).unwrap().contains("Wolf"));
        assert!(crown_say_line(1).is_none());
    }

    #[test]
    fn transition_on_clothing_slot_must_match() {
        let mut inp = ClothingTransitionIn {
            held_parent_id: 10,
            held_uses: 0,
            held_num_uses: 0,
            clothing_parent_id: 100,
            clothing_uses: 3,
            clothing_num_uses: 5,
            clothing_slot_old: Some(5),
            new_target_clothing_slot: Some(5),
            new_target_parent_id: 101,
            reverse_use_target: false,
            reverse_use_actor: false,
            no_use_target: false,
            no_use_actor: false,
            new_actor_id: 0,
            new_target_id: 101,
            new_actor_num_uses: 0,
            new_target_num_uses: 5,
        };
        let out = try_transition_on_clothing_pure(&inp).unwrap();
        assert_eq!(out.clothing_id, 101);
        assert_eq!(out.held_id, 0);
        inp.new_target_clothing_slot = Some(0);
        assert!(try_transition_on_clothing_pure(&inp).is_none());
        inp.new_target_parent_id = 3919;
        inp.new_target_id = 3919;
        assert!(try_transition_on_clothing_pure(&inp).unwrap().praise_jinbali);
    }

    #[test]
    fn reverse_use_at_max_refuses() {
        let inp = ClothingTransitionIn {
            held_parent_id: 10,
            held_uses: 0,
            held_num_uses: 0,
            clothing_parent_id: 100,
            clothing_uses: 5,
            clothing_num_uses: 5,
            clothing_slot_old: Some(5),
            new_target_clothing_slot: Some(5),
            new_target_parent_id: 101,
            reverse_use_target: true,
            reverse_use_actor: false,
            no_use_target: false,
            no_use_actor: false,
            new_actor_id: 0,
            new_target_id: 101,
            new_actor_num_uses: 0,
            new_target_num_uses: 5,
        };
        assert!(try_transition_on_clothing_pure(&inp).is_none());
    }

    #[test]
    fn put_take_clothing_nest() {
        let bag = NestedHelper::id_only(198);
        let item = NestedHelper::id_only(33);
        let (bag2, prev) = put_into_clothing_nest(bag, item, 4, false).unwrap();
        assert!(prev.is_none());
        assert_eq!(bag2.contained.len(), 1);
        let (bag3, taken) = take_from_clothing_nest(bag2, -1).unwrap();
        assert_eq!(taken.id, 33);
        assert!(bag3.contained.is_empty());
    }

    /// CLOTHING-CONTAIN-SIZE: containSize > clothing.slotSize refuses put (USE + DROP).
    #[test]
    fn clothing_contain_size_gate() {
        // Fits: contain 1 into slot 1.
        assert!(can_put_into_clothing_sized(
            198, 4, 0, 33, true, false, 1.0, 1.0
        ));
        // Too big for USE put.
        assert!(!can_put_into_clothing_sized(
            198, 4, 0, 33, true, false, 2.0, 1.0
        ));
        // DROP also refuses oversized (Haxe size gate before swap).
        assert!(!can_put_into_clothing_sized(
            198, 4, 0, 33, true, true, 2.0, 1.0
        ));
        // Default can_put_into_clothing still allows (0 <= 1).
        assert!(can_put_into_clothing(198, 4, 0, 33, true, false));
    }

    #[test]
    fn apply_place_obj_in_clothing_size_refuse() {
        let mut db = ContentDb::default();
        // Backpack clothing with slotSize 1.
        let mut bag = def_cloth(198, "b", 4, 0, false);
        bag.slot_size = 1.0;
        db.objects.insert(198, bag);
        // Large containable item.
        let mut big = def_cloth(900, "n", 0, 0, true);
        big.containable = true;
        big.contain_size = 2.0;
        db.objects.insert(900, big);

        let mut p = Player::new(1, 1, "size@t");
        p.set_clothing_index_helper(4, Some(NestedHelper::id_only(198)));
        p.set_held_helper(NestedHelper::id_only(900));
        assert_eq!(
            apply_place_obj_in_clothing(&mut p, &db, 4, false),
            Err("FULL_OR_BAD")
        );
        assert_eq!(p.held_id, 900, "held kept on size refuse");
        // Fit when contain_size lowered.
        db.objects.get_mut(&900).unwrap().contain_size = 1.0;
        assert!(apply_place_obj_in_clothing(&mut p, &db, 4, false).is_ok());
        assert_eq!(p.held_id, 0);
    }

    #[test]
    fn drop_into_clothing_swaps_top() {
        let bag = NestedHelper::from_wire(198, &[10, 11]);
        let held = NestedHelper::id_only(99);
        let (bag2, prev) = put_into_clothing_nest(bag, held, 4, true).unwrap();
        assert_eq!(prev.unwrap().id, 11);
        assert_eq!(bag2.contained[0].id, 99);
    }

    #[test]
    fn apply_switch_and_place_live() {
        let mut db = ContentDb::default();
        db.objects.insert(693, def_cloth(693, "h", 0, 0, false));
        db.objects.insert(198, def_cloth(198, "p", 4, 0, false));
        db.objects.insert(33, def_cloth(33, "n", 0, 0, true));

        let mut p = Player::new(1, 1, "c@t");
        p.held_id = 693;
        p.held_helper = Some(NestedHelper::id_only(693));
        let (id, prev, say) = apply_switch_cloths(&mut p, &db, -1).unwrap();
        assert_eq!(id, 693);
        assert_eq!(prev, 0);
        assert!(say.unwrap().contains("Carrots"));
        assert_eq!(p.hat, 693);
        assert_eq!(p.held_id, 0);

        p.held_id = 198;
        p.held_helper = Some(NestedHelper::id_only(198));
        apply_switch_cloths(&mut p, &db, 5).unwrap();
        assert!(p.clothing_helpers[5].is_some());
        p.held_id = 33;
        p.held_helper = Some(NestedHelper::id_only(33));
        apply_place_obj_in_clothing(&mut p, &db, 5, false).unwrap();
        assert_eq!(p.held_id, 0);
        assert_eq!(
            p.clothing_helpers[5].as_ref().unwrap().contained[0].id,
            33
        );

        let taken = apply_sremv_from_clothing(&mut p, 5, Some(-1)).unwrap();
        assert_eq!(taken, 33);
        assert_eq!(p.held_id, 33);
    }

    #[test]
    fn apply_self_clothing_order_transition_first() {
        let mut db = ContentDb::default();
        db.objects.insert(400, def_cloth(400, "p", 0, 5, false));
        db.objects.insert(401, def_cloth(401, "p", 0, 5, false));
        db.objects.insert(50, def_cloth(50, "n", 0, 0, false));
        db.transitions.insert(
            (50, 400),
            Transition {
                actor_id: 50,
                target_id: 400,
                new_actor_id: 0,
                new_target_id: 401,
                last_use_actor: false,
                last_use_target: false,
                auto_decay_seconds: 0.0,
                reverse_use_actor: false,
                reverse_use_target: true,
                no_use_actor: false,
                no_use_target: false,
                move_dist: 0,
                desired_move_dist: 0,
                actor_min_use_fraction: 0.0,
                target_min_use_fraction: 0.0,
                switch_number_of_uses: false,
                target_number_of_uses: -1,
            is_pickup_or_drop: false,
            },
        );
        let mut p = Player::new(1, 1, "q@t");
        p.clothing_helpers[5] = Some(NestedHelper::with_uses(400, 1));
        p.held_id = 50;
        p.held_helper = Some(NestedHelper::id_only(50));
        let (path, _) = apply_self_clothing(&mut p, &db, 5).unwrap();
        assert_eq!(path, SelfClothingPath::Transition);
        assert_eq!(p.clothing_helpers[5].as_ref().unwrap().id, 401);
        assert_eq!(p.held_id, 0);
    }

    #[test]
    fn format_clothing_set_six_slots() {
        let mut p = Player::new(1, 1, "f@t");
        p.hat = 1;
        p.chest = 2;
        p.shoes = 3;
        p.clothing_helpers[5] = Some(NestedHelper::from_wire(198, &[9]));
        let s = format_clothing_set(&p);
        assert_eq!(s, "1;2;3;0;0;198,9");
    }

    #[test]
    fn format_clothing_helper_colon_subnest() {
        let mut bag = NestedHelper::id_only(198);
        let mut basket = NestedHelper::id_only(33);
        basket.contained.push(NestedHelper::id_only(100));
        basket.contained.push(NestedHelper::id_only(101));
        bag.contained.push(basket);
        bag.contained.push(NestedHelper::id_only(40));
        assert_eq!(format_clothing_helper_string(&bag), "198,33:100:101,40");
    }

    #[test]
    fn empty_hand_place_takes_first_not_last() {
        // Haxe DoContainerStuffOnObj: empty hand index < 0 → 0 (first).
        assert_eq!(empty_hand_container_take_index(-1), 0);
        let bag = NestedHelper::from_wire(198, &[10, 11, 12]);
        let take_i = empty_hand_container_take_index(-1);
        let (_bag2, taken) = take_from_clothing_nest(bag, take_i).unwrap();
        assert_eq!(taken.id, 10, "empty-hand place takes first contained");
    }

    #[test]
    fn sremv_minus_one_takes_last() {
        let bag = NestedHelper::from_wire(198, &[10, 11, 12]);
        let (_b, taken) = take_from_clothing_nest(bag, -1).unwrap();
        assert_eq!(taken.id, 12);
    }

    #[test]
    fn permanent_contained_refuses_take() {
        let bag = NestedHelper::from_wire(198, &[50]);
        assert!(take_from_clothing_nest_checked(bag.clone(), 0, |_| true).is_none());
        let (b2, t) = take_from_clothing_nest_checked(bag, 0, |_| false).unwrap();
        assert_eq!(t.id, 50);
        assert!(b2.contained.is_empty());
    }

    #[test]
    fn drink_water_bowl_before_clothing() {
        let mut db = ContentDb::default();
        db.objects.insert(382, def_cloth(382, "n", 0, 0, false));
        db.objects.insert(235, def_cloth(235, "n", 0, 0, false));
        let mut p = Player::new(1, 1, "d@t");
        p.held_id = 382;
        p.held_helper = Some(NestedHelper::id_only(382));
        p.heat = 0.7;
        let (path, _) = apply_self_clothing(&mut p, &db, -1).unwrap();
        assert_eq!(path, SelfClothingPath::Drink);
        assert_eq!(p.held_id, 235);
        assert!(p.heat < 0.7);
        assert!(p.stored_water > 0.0);
    }

    #[test]
    fn drink_pure_refuses_when_full_and_cool() {
        let inp = DrinkWaterIn {
            held_parent_id: 382,
            heat: 0.3,
            stored_water: 1.0,
            temp_reduction: 0.5,
            max_stored_water: 1.0,
        };
        assert!(try_drink_water_pure(&inp).is_none());
    }

    #[test]
    fn ubaby_cloth_age_gate() {
        let mut db = ContentDb::default();
        db.objects.insert(693, def_cloth(693, "h", 0, 0, false));
        let mut adult = Player::new(1, 1, "a@t");
        adult.age = 20.0;
        adult.held_id = 693;
        adult.held_helper = Some(NestedHelper::id_only(693));
        let mut baby = Player::new(2, 2, "b@t");
        baby.age = 5.0;
        let (id, prev, say) =
            apply_switch_cloths_on_other(&mut adult, &mut baby, &db, -1, MAX_AGE_CLOTH_OTHERS)
                .unwrap();
        assert_eq!(id, 693);
        assert_eq!(prev, 0);
        assert!(say.is_some());
        assert_eq!(baby.hat, 693);
        assert_eq!(adult.held_id, 0);

        adult.held_id = 693;
        adult.held_helper = Some(NestedHelper::id_only(693));
        let mut old = Player::new(3, 3, "o@t");
        old.age = 40.0;
        assert_eq!(
            apply_switch_cloths_on_other(&mut adult, &mut old, &db, -1, MAX_AGE_CLOTH_OTHERS)
                .unwrap_err(),
            "TOO_OLD"
        );
    }

    #[test]
    fn other_player_accepts_cloth_gate() {
        assert!(other_player_accepts_cloth(5.0, 10.0));
        assert!(!other_player_accepts_cloth(11.0, 10.0));
    }
}
