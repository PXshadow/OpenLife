//! Nested held / clothing / wound / fever ObjectHelpers on the player body.
//!
//! Chunk **NESTED-CLOTHING-PERSIST** — Haxe `ObjectHelper.WriteToFile` /
//! `ReadFromFile` + `GlobalPlayerInstance.WritePlayers` / `ReadPlayers` body
//! slices + `setHeldObject` light-wound → `hiddenWound`.
//!
//! Map NestedHelper lives in `ol-world` (OLW3). This module applies the same
//! tree type to player-held cargo, six clothing slots, hidden wound, and fever.

use crate::player::{ClothingSlot, Player, CLOTHING_SLOT_COUNT};
use ol_world::{
    read_nested_helper, read_optional_nested_helper, write_nested_helper,
    write_optional_nested_helper, NestedHelper, NESTED_NULL_ID,
};
use std::io::{Read, Write};

/// Yellow fever wound object id (Haxe `fever.id == 2155`).
pub const YELLOW_FEVER_ID: i32 = 2155;

// ── setHeldObject light-wound pure helpers ───────────────────────────────────

/// Inputs for Haxe `setHeldObject` light-wound branch (without full TransitionImporter).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SetHeldWoundCtx {
    /// `obj.isWound()` from description / content.
    pub is_wound: bool,
    /// `GetTransition(-1, parentId).newTargetID` (or alternativeTimeOutcome when ≥0).
    /// `Some(0)` means light wound (heals to empty); `None` = no auto-decay transition.
    pub auto_decay_new_target: Option<i32>,
    /// Base `CalculateTimeToChangeForObj` seconds before health factor.
    pub base_time_to_change: f32,
    /// `CalculateHealthFactor(2, 0.5)` divisor (≥ small epsilon).
    pub health_factor: f32,
}

impl Default for SetHeldWoundCtx {
    fn default() -> Self {
        Self {
            is_wound: false,
            auto_decay_new_target: None,
            base_time_to_change: 0.0,
            health_factor: 1.0,
        }
    }
}

/// True when a wound should become `hiddenWound` (light wound).
/// // Haxe: GlobalPlayerInstance.setHeldObject L3801–3815
pub fn is_light_wound(is_wound: bool, has_hidden: bool, auto_decay_new_target: Option<i32>) -> bool {
    is_wound && !has_hidden && auto_decay_new_target == Some(0)
}

/// Scale time-to-change by health factor (Haxe divides by healthFactor).
// Haxe: setHeldObject timeToChange = CalculateTimeToChangeForObj / healthFactor
pub fn wound_time_to_change(base: f32, health_factor: f32) -> f32 {
    let hf = if health_factor.abs() < 1e-6 {
        1.0
    } else {
        health_factor
    };
    (base / hf).max(0.0)
}

/// Outcome of applying setHeldObject wound rules to a helper.
#[derive(Debug, Clone, PartialEq)]
pub struct SetHeldOutcome {
    pub held: NestedHelper,
    /// When set, caller assigns `player.hidden_wound = Some(...)`.
    pub set_hidden_wound: Option<NestedHelper>,
    pub became_hidden: bool,
}

/// Pure `setHeldObject` wound + timer arming (no player mut).
// Haxe: GlobalPlayerInstance.setHeldObject
pub fn apply_set_held_wound_rules(
    mut obj: NestedHelper,
    has_hidden_wound: bool,
    held_is_same_as_hidden: bool,
    ctx: SetHeldWoundCtx,
) -> SetHeldOutcome {
    if obj.is_empty() {
        return SetHeldOutcome {
            held: NestedHelper::empty(),
            set_hidden_wound: None,
            became_hidden: false,
        };
    }

    let mut set_hidden = None;
    let mut became_hidden = false;

    if is_light_wound(ctx.is_wound, has_hidden_wound, ctx.auto_decay_new_target) {
        obj.time_to_change = wound_time_to_change(ctx.base_time_to_change, ctx.health_factor);
        set_hidden = Some(obj.clone());
        became_hidden = true;
    } else if !held_is_same_as_hidden {
        // Haxe: else if (obj != hiddenWound) arm timeToChange
        if ctx.base_time_to_change > 0.0 {
            obj.time_to_change = ctx.base_time_to_change;
        }
    }

    SetHeldOutcome {
        held: obj,
        set_hidden_wound: set_hidden,
        became_hidden,
    }
}

/// Apply pure set-held rules onto a player (syncs held + optional hiddenWound).
// Haxe: GlobalPlayerInstance.setHeldObject
pub fn player_set_held_object(player: &mut Player, obj: NestedHelper, ctx: SetHeldWoundCtx) {
    let has_hidden = player.hidden_wound.is_some();
    let same_as_hidden = match &player.hidden_wound {
        Some(w) => w.id == obj.id && obj.id != 0,
        None => false,
    };
    let out = apply_set_held_wound_rules(obj, has_hidden, same_as_hidden, ctx);
    if let Some(hw) = out.set_hidden_wound {
        player.hidden_wound = Some(hw);
    }
    player.set_held_helper(out.held);
}

/// Haxe ReadPlayers post-pass: alias `hiddenWound` to `heldObject` when same id.
// Haxe: GlobalPlayerInstance.ReadPlayers L862
pub fn alias_hidden_wound_to_held(player: &mut Player) {
    let held_id = player.held_helper.as_ref().map(|h| h.id).unwrap_or(0);
    if held_id == 0 {
        return;
    }
    if player
        .hidden_wound
        .as_ref()
        .map(|w| w.id == held_id)
        .unwrap_or(false)
    {
        player.hidden_wound = player.held_helper.clone();
    }
}

/// True when fever object is yellow fever (id 2155).
// Haxe: GlobalPlayerInstance.hasYellowFever / fever.id == 2155
pub fn is_yellow_fever(fever: Option<&NestedHelper>) -> bool {
    fever.map(|f| f.id == YELLOW_FEVER_ID).unwrap_or(false)
}

/// Clamp NestedHelper creation time to sim (Haxe ReadFromFile L268).
pub fn clamp_helper_creation(h: &mut NestedHelper, sim_time: f32) {
    if h.creation_time > sim_time {
        h.creation_time = sim_time;
    }
    for c in &mut h.contained {
        clamp_helper_creation(c, sim_time);
    }
}

/// Clear body timer object when elapsed (fever / hiddenWound tick stub).
// Haxe: TimeHelper wound/fever elapsed → clear
pub fn clear_if_timer_elapsed(h: &mut Option<NestedHelper>, sim_time: f32) -> bool {
    let elapsed = match h.as_ref() {
        Some(obj) if obj.time_to_change > 0.0 => {
            let passed = (sim_time - obj.creation_time).max(0.0);
            passed >= obj.time_to_change
        }
        _ => false,
    };
    if elapsed {
        *h = None;
    }
    elapsed
}

/// Apply transform_to_dummy ids onto a NestedHelper using parent/dummy table.
// Haxe: ObjectHelper.TransformToDummy (after ReadFromFile)
pub fn apply_transform_to_dummy_on_helper(
    h: &mut NestedHelper,
    num_uses: i32,
    last_use_object: i32,
    undo_last_use_object: i32,
    is_dummy: bool,
    dummy_parent: i32,
    dummy_ids: &[i32],
) {
    let (id, uses) = ol_world::transform_to_dummy(
        h.id,
        h.uses_remaining,
        num_uses,
        last_use_object,
        undo_last_use_object,
        is_dummy,
        dummy_parent,
        dummy_ids,
    );
    h.id = id;
    h.uses_remaining = uses;
}

// ── WritePlayers / ReadPlayers body-object slice ─────────────────────────────

/// Serializable body-object bundle (held + wound + fever + 6 clothing).
/// // Haxe: WritePlayers heldObject / hiddenWound / fever / clothingObjects
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerBodyObjects {
    pub held: Option<NestedHelper>,
    pub hidden_wound: Option<NestedHelper>,
    pub fever: Option<NestedHelper>,
    pub yellowfever_count: f32,
    /// Always length [`CLOTHING_SLOT_COUNT`]; empty slots are `None` (id 0 on disk as empty helper).
    pub clothing: [Option<NestedHelper>; CLOTHING_SLOT_COUNT],
}

impl PlayerBodyObjects {
    pub fn from_player(p: &Player) -> Self {
        Self {
            held: p.held_helper.clone(),
            hidden_wound: p.hidden_wound.clone(),
            fever: p.fever.clone(),
            yellowfever_count: p.yellowfever_count,
            clothing: p.clothing_helpers.clone(),
        }
    }

    pub fn apply_to_player(&self, p: &mut Player) {
        match &self.held {
            Some(h) if !h.is_empty() => p.set_held_helper(h.clone()),
            _ => p.clear_held(),
        }
        p.hidden_wound = self.hidden_wound.clone();
        p.fever = self.fever.clone();
        p.yellowfever_count = self.yellowfever_count;
        for i in 0..CLOTHING_SLOT_COUNT {
            p.set_clothing_index_helper(i, self.clothing[i].clone());
        }
        alias_hidden_wound_to_held(p);
    }
}

/// Write body objects (Haxe WritePlayers subset for ObjectHelpers).
// Haxe: GlobalPlayerInstance.WritePlayers (held / hiddenWound / fever / clothing)
pub fn write_player_body_objects(w: &mut impl Write, body: &PlayerBodyObjects) -> Result<(), String> {
    // held: empty → write id-0 helper (not null); Haxe always writes heldObject
    write_nested_helper(
        w,
        body.held
            .as_ref()
            .unwrap_or(&NestedHelper::empty()),
    )?;
    write_optional_nested_helper(w, body.hidden_wound.as_ref())?;
    write_optional_nested_helper(w, body.fever.as_ref())?;
    w.write_all(&body.yellowfever_count.to_le_bytes())
        .map_err(|e| e.to_string())?;
    // clothing count + each slot (empty = NestedHelper id 0)
    let n = CLOTHING_SLOT_COUNT as u16;
    w.write_all(&n.to_le_bytes()).map_err(|e| e.to_string())?;
    for i in 0..CLOTHING_SLOT_COUNT {
        write_nested_helper(
            w,
            body.clothing[i]
                .as_ref()
                .unwrap_or(&NestedHelper::empty()),
        )?;
    }
    Ok(())
}

/// Read body objects (Haxe ReadPlayers subset).
// Haxe: GlobalPlayerInstance.ReadPlayers
pub fn read_player_body_objects(r: &mut impl Read) -> Result<PlayerBodyObjects, String> {
    let held_raw = read_nested_helper(r)?;
    let held = if held_raw.is_empty() {
        None
    } else {
        Some(held_raw)
    };
    let hidden_wound = read_optional_nested_helper(r)?;
    let fever = read_optional_nested_helper(r)?;
    let mut yf = [0u8; 4];
    r.read_exact(&mut yf).map_err(|e| e.to_string())?;
    let yellowfever_count = f32::from_le_bytes(yf);
    let mut nbuf = [0u8; 2];
    r.read_exact(&mut nbuf).map_err(|e| e.to_string())?;
    let n = u16::from_le_bytes(nbuf) as usize;
    let mut clothing: [Option<NestedHelper>; CLOTHING_SLOT_COUNT] =
        [None, None, None, None, None, None];
    for i in 0..n {
        let c = read_nested_helper(r)?;
        if i < CLOTHING_SLOT_COUNT {
            clothing[i] = if c.is_empty() { None } else { Some(c) };
        }
        // Extra slots beyond 6: discarded (forward-compat).
    }
    Ok(PlayerBodyObjects {
        held,
        hidden_wound,
        fever,
        yellowfever_count,
        clothing,
    })
}

/// Sync flat hat/chest/shoes ids from clothing_helpers[0..3].
pub fn sync_flat_clothing_ids(player: &mut Player) {
    player.hat = player.clothing_helpers[0]
        .as_ref()
        .map(|h| h.id)
        .filter(|&id| id > 0)
        .unwrap_or(0);
    player.chest = player.clothing_helpers[1]
        .as_ref()
        .map(|h| h.id)
        .filter(|&id| id > 0)
        .unwrap_or(0);
    player.shoes = player.clothing_helpers[2]
        .as_ref()
        .map(|h| h.id)
        .filter(|&id| id > 0)
        .unwrap_or(0);
}

/// Transfer nest when taking a map container item into hands.
pub fn player_take_container_into_hands(player: &mut Player, taken: NestedHelper) {
    player.set_held_helper(taken);
}

/// Equip held into clothing index (0..5), swap previous into hands.
// Haxe: doSwitchCloths clothingObjects[slot] swap
pub fn switch_clothing_index(player: &mut Player, index: usize) -> Result<(i32, i32), &'static str> {
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
    let held = player
        .held_helper
        .take()
        .unwrap_or_else(|| NestedHelper::with_uses(player.held_id, player.held_uses));
    let held_id = held.id;
    let prev = player.clothing_helpers[index].take();
    let prev_id = prev.as_ref().map(|h| h.id).unwrap_or(0);
    player.clothing_helpers[index] = Some(held);
    match prev {
        Some(p) if !p.is_empty() => player.set_held_helper(p),
        _ => player.clear_held(),
    }
    Ok((held_id, prev_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn light_wound_becomes_hidden() {
        assert!(is_light_wound(true, false, Some(0)));
        assert!(!is_light_wound(true, true, Some(0)));
        assert!(!is_light_wound(true, false, Some(99)));
        assert!(!is_light_wound(false, false, Some(0)));
    }

    #[test]
    fn set_held_light_wound_arms_hidden() {
        let mut p = Player::new(1, 1, "w@t");
        let mut wound = NestedHelper::id_only(200);
        wound.creation_time = 10.0;
        let ctx = SetHeldWoundCtx {
            is_wound: true,
            auto_decay_new_target: Some(0),
            base_time_to_change: 40.0,
            health_factor: 2.0,
        };
        player_set_held_object(&mut p, wound, ctx);
        assert_eq!(p.held_id, 200);
        assert!(p.hidden_wound.is_some());
        assert_eq!(p.hidden_wound.as_ref().unwrap().id, 200);
        assert!((p.hidden_wound.as_ref().unwrap().time_to_change - 20.0).abs() < 1e-5);
        assert!(p.is_holding_hidden_wound());
        assert!(!p.is_wounded_held(true)); // light → not "wounded" for grave/AI
    }

    #[test]
    fn set_held_heavy_wound_not_hidden() {
        let mut p = Player::new(1, 1, "w@t");
        let wound = NestedHelper::id_only(201);
        let ctx = SetHeldWoundCtx {
            is_wound: true,
            auto_decay_new_target: Some(50), // heals to something
            base_time_to_change: 30.0,
            health_factor: 1.0,
        };
        player_set_held_object(&mut p, wound, ctx);
        assert!(p.hidden_wound.is_none());
        assert_eq!(p.held_id, 201);
        assert!(p.is_wounded_held(true));
    }

    #[test]
    fn body_objects_round_trip_nested_clothing_and_held() {
        let mut p = Player::new(7, 7, "body@t");
        let mut bag = NestedHelper::from_wire(292, &[33, 40]);
        bag.uses_remaining = 2;
        bag.time_to_change = 5.0;
        bag.creation_time = 1.0;
        p.set_held_helper(bag);
        let mut pack = NestedHelper::from_wire(697, &[100, 101]);
        pack.hits = 1.5;
        p.set_clothing_helper(ClothingSlot::Chest, pack);
        p.clothing_helpers[5] = Some(NestedHelper::from_wire(198, &[10])); // backpack slot
        p.hidden_wound = Some(NestedHelper::with_uses(200, 1));
        p.fever = Some(NestedHelper::id_only(YELLOW_FEVER_ID));
        p.yellowfever_count = 0.25;

        let body = PlayerBodyObjects::from_player(&p);
        let mut buf = Vec::new();
        write_player_body_objects(&mut buf, &body).unwrap();
        let got = read_player_body_objects(&mut Cursor::new(buf)).unwrap();

        assert_eq!(got.held.as_ref().unwrap().id, 292);
        assert_eq!(
            got.held
                .as_ref()
                .unwrap()
                .contained
                .iter()
                .map(|c| c.id)
                .collect::<Vec<_>>(),
            vec![33, 40]
        );
        assert_eq!(got.clothing[1].as_ref().unwrap().id, 697);
        assert_eq!(got.clothing[1].as_ref().unwrap().contained[1].id, 101);
        assert_eq!(got.clothing[5].as_ref().unwrap().id, 198);
        assert_eq!(got.hidden_wound.as_ref().unwrap().id, 200);
        assert_eq!(got.fever.as_ref().unwrap().id, YELLOW_FEVER_ID);
        assert!((got.yellowfever_count - 0.25).abs() < 1e-5);

        let mut p2 = Player::new(8, 8, "load@t");
        got.apply_to_player(&mut p2);
        assert_eq!(p2.held_id, 292);
        assert_eq!(p2.held_uses, 2);
        assert_eq!(p2.chest, 697);
        assert_eq!(p2.clothing_helpers[5].as_ref().unwrap().contained[0].id, 10);
    }

    #[test]
    fn read_players_alias_hidden_to_held() {
        let mut p = Player::new(1, 1, "a@t");
        p.set_held_helper(NestedHelper::with_uses(55, 1));
        p.hidden_wound = Some(NestedHelper::with_uses(55, 1));
        // Distinct instances before alias
        alias_hidden_wound_to_held(&mut p);
        // After alias, hidden is a clone of held (same id/uses)
        assert_eq!(
            p.hidden_wound.as_ref().map(|h| (h.id, h.uses_remaining)),
            p.held_helper.as_ref().map(|h| (h.id, h.uses_remaining))
        );
    }

    #[test]
    fn wear_strip_preserves_clothing_nest() {
        let mut p = Player::new(1, 1, "c@t");
        p.set_held_helper(NestedHelper::from_wire(697, &[11, 12]));
        assert_eq!(p.wear_held(ClothingSlot::Chest).unwrap(), (697, 0));
        assert_eq!(p.chest, 697);
        assert_eq!(
            p.clothing_helper(ClothingSlot::Chest)
                .unwrap()
                .contained
                .iter()
                .map(|c| c.id)
                .collect::<Vec<_>>(),
            vec![11, 12]
        );
        assert_eq!(p.held_id, 0);
        assert_eq!(p.strip_slot(ClothingSlot::Chest).unwrap(), 697);
        assert_eq!(
            p.held_helper
                .as_ref()
                .unwrap()
                .contained
                .iter()
                .map(|c| c.id)
                .collect::<Vec<_>>(),
            vec![11, 12]
        );
    }

    #[test]
    fn transform_to_dummy_on_held_after_round_trip() {
        let mut h = NestedHelper::with_uses(50, 3);
        // parent 50, num_uses 5, dummies for uses 1..4
        apply_transform_to_dummy_on_helper(&mut h, 5, 0, 0, false, 0, &[1001, 1002, 1003, 1004]);
        assert_eq!(h.id, 1003); // uses 3 → dummy_ids[2]
        assert_eq!(h.uses_remaining, 3);

        let body = PlayerBodyObjects {
            held: Some(h.clone()),
            ..Default::default()
        };
        let mut buf = Vec::new();
        write_player_body_objects(&mut buf, &body).unwrap();
        let got = read_player_body_objects(&mut Cursor::new(buf)).unwrap();
        let mut loaded = got.held.unwrap();
        // Disk stores resolved id; re-apply is stable when already dummy
        apply_transform_to_dummy_on_helper(
            &mut loaded,
            5,
            0,
            0,
            true,
            50,
            &[1001, 1002, 1003, 1004],
        );
        assert_eq!(loaded.uses_remaining, 3);
    }

    #[test]
    fn fever_timer_clears_when_elapsed() {
        let mut fever = Some(NestedHelper {
            id: YELLOW_FEVER_ID,
            creation_time: 100.0,
            time_to_change: 10.0,
            ..Default::default()
        });
        assert!(!clear_if_timer_elapsed(&mut fever, 105.0));
        assert!(fever.is_some());
        assert!(clear_if_timer_elapsed(&mut fever, 111.0));
        assert!(fever.is_none());
    }

    #[test]
    fn optional_null_nested_round_trip() {
        let mut buf = Vec::new();
        write_optional_nested_helper(&mut buf, None).unwrap();
        write_optional_nested_helper(&mut buf, Some(&NestedHelper::id_only(9))).unwrap();
        let mut c = Cursor::new(buf);
        assert!(read_optional_nested_helper(&mut c).unwrap().is_none());
        assert_eq!(read_optional_nested_helper(&mut c).unwrap().unwrap().id, 9);
        assert_eq!(NESTED_NULL_ID, -100);
    }

    #[test]
    fn container_take_into_hands_preserves_nest() {
        let mut world = ol_world::World::new(16, 16, false);
        world.set_object(2, 2, 391);
        assert!(world.container_put(2, 2, 292, 4));
        assert!(world.container_put_nested(2, 2, 0, 77, 4));
        if let Some(mut h) = world.helpers.remove(&(2, 2)) {
            h.synthesize_slots_from_wire();
            world.set_object_complex(2, 2, h);
        }
        let taken = world.container_take_helper(2, 2, Some(0)).unwrap();
        let mut p = Player::new(1, 1, "take@t");
        player_take_container_into_hands(&mut p, taken);
        assert_eq!(p.held_id, 292);
        assert_eq!(p.held_helper.as_ref().unwrap().contained[0].id, 77);
    }

    #[test]
    fn is_yellow_fever_id() {
        assert!(is_yellow_fever(Some(&NestedHelper::id_only(YELLOW_FEVER_ID))));
        assert!(!is_yellow_fever(Some(&NestedHelper::id_only(1))));
        assert!(!is_yellow_fever(None));
    }
}
