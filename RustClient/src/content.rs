//! Object / transition load from OneLifeData7 text + OLC1/OLT1 binary cache.
//!
//! - **Text path** (dev): parse `objects/*.txt` + `transitions/*.txt`.
//! - **Binary cache** (fast start): [`crate::content_binary`] OLC1/OLT1 + manifest.
//!
//! C++: objectBank / transitionBank. Haxe: ObjectData / TransitionImporter / ObjectBake.
//! Shared layout: `docs/port/CONTENT_BINARY.md`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::category_bank::{expand_category_transitions, CategoryBank};

/// One sprite placement on an object (from object .txt `spriteID` / `pos` / `rot`).
///
/// C++: `objectBank` SpriteRecord fields; Haxe: `ObjectData.spriteArray`.
#[derive(Debug, Clone)]
pub struct ObjectSprite {
    pub sprite_id: i32,
    pub x: f32,
    pub y: f32,
    /// Rotation in turns (0..1 = full circle); C++ / Haxe `rot`.
    pub rot: f32,
    pub h_flip: bool,
    pub age_start: f32,
    pub age_end: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
    /// Parent sprite index, or `-1` if root. C++/Haxe `parent=`.
    pub parent: i32,
    /// Hide when object is held. C++ `invisHolding`.
    pub invis_holding: bool,
    /// Hide when clothing is worn. C++ `invisWorn == 1`.
    pub invis_worn: bool,
    /// Visible only when worn. C++ `spriteInvisibleWhenWorn == 2`.
    /// Skipped by [`get_object_center_offset`] (not considered for wideness).
    pub only_when_worn: bool,
    /// Draw behind container slots. C++ `behindSlots`.
    pub behind_slots: bool,
    /// Draw this sprite layer behind players (tall objects). C++ `spriteBehindPlayer`.
    pub behind_player: bool,
    /// C++ sparse `bodyIndex` — person torso layer.
    pub is_body: bool,
    /// C++ sparse `headIndex`.
    pub is_head: bool,
    /// C++ sparse `backFootIndex`.
    pub is_back_foot: bool,
    /// C++ sparse `frontFootIndex`.
    pub is_front_foot: bool,
    /// C++ `spriteIsEyes` — sprite tag contains `Eyes` (derived, not in object .txt).
    pub is_eyes: bool,
    /// C++ `spriteIsMouth` — sprite tag contains `Mouth` (derived, not in object .txt).
    pub is_mouth: bool,
    /// C++ `spriteUseVanish` — vanishes as multi-use depletes (`useVanishIndex`).
    pub use_vanish: bool,
    /// C++ `spriteUseAppear` — appears as multi-use depletes (`useAppearIndex`).
    pub use_appear: bool,
    /// C++ `spriteSkipDrawing` — per-use / per-dummy hide (from [`setup_sprite_use_vis`]).
    pub skip_drawing: bool,
}

impl Default for ObjectSprite {
    /// Match text-load defaults: always-visible age range, root parent, white mult.
    fn default() -> Self {
        Self {
            sprite_id: 0,
            x: 0.0,
            y: 0.0,
            rot: 0.0,
            h_flip: false,
            age_start: -1.0,
            age_end: -1.0,
            r: 1.0,
            g: 1.0,
            b: 1.0,
            parent: -1,
            invis_holding: false,
            invis_worn: false,
            only_when_worn: false,
            behind_slots: false,
            behind_player: false,
            is_body: false,
            is_head: false,
            is_front_foot: false,
            is_back_foot: false,
            is_eyes: false,
            is_mouth: false,
            use_vanish: false,
            use_appear: false,
            skip_drawing: false,
        }
    }
}

impl ObjectSprite {
    /// Visible for player age (years). `-1` range means always.
    pub fn visible_at_age(&self, age: f32) -> bool {
        let a0 = self.age_start;
        let a1 = self.age_end;
        if a0 < 0.0 && a1 < 0.0 {
            return true;
        }
        if a0 >= 0.0 && age < a0 {
            return false;
        }
        if a1 >= 0.0 && age > a1 {
            return false;
        }
        true
    }
}

/// C++ `setupSpriteUseVis` — per-sprite skip mask for multi-use visual stages.
///
/// `uses_remaining == num_uses` (full parent): hide all `use_appear` sprites.  
/// `uses_remaining == 0`: hide all `use_vanish` sprites.  
/// Intermediate `1..num_uses-1` (use dummies): progressive vanish/appear fractions
/// matching Jason's `objectBank.cpp` padding so the last dummy still shows one
/// vanishing sprite when `num_vanish < num_uses`.
///
/// Returns a `skip_drawing` bool per sprite index (true = do not draw).
pub fn setup_sprite_use_vis(
    sprites: &[ObjectSprite],
    num_uses: i32,
    uses_remaining: i32,
) -> Vec<bool> {
    let n = sprites.len();
    let mut skip = vec![false; n];
    if n == 0 || num_uses <= 0 {
        return skip;
    }

    if uses_remaining == num_uses {
        for (s, spr) in sprites.iter().enumerate() {
            if spr.use_appear {
                skip[s] = true;
            }
        }
        return skip;
    }
    if uses_remaining == 0 {
        for (s, spr) in sprites.iter().enumerate() {
            if spr.use_vanish {
                skip[s] = true;
            }
        }
        return skip;
    }

    // Intermediate use-dummy stages (C++ autoGenerateUsedObjects dummies).
    let vanishing: Vec<usize> = sprites
        .iter()
        .enumerate()
        .filter(|(_, spr)| spr.use_vanish)
        .map(|(i, _)| i)
        .collect();
    let appearing: Vec<usize> = sprites
        .iter()
        .enumerate()
        .filter(|(_, spr)| spr.use_appear)
        .map(|(i, _)| i)
        .collect();

    // Hide all appearing as basis, then unhide a prefix.
    for &ai in &appearing {
        skip[ai] = true;
    }

    let num_vanishing = vanishing.len() as i32;
    let num_appearing = appearing.len() as i32;
    let d = uses_remaining;

    if num_vanishing > 0 {
        let mut num_sprites_left = (d * num_vanishing) / num_uses;
        let num_in_last_dummy = num_vanishing / num_uses;
        let mut num_in_first_dummy = ((num_uses - 1) * num_vanishing) / num_uses;

        if num_in_last_dummy == 0 {
            // Pad so last dummy keeps ≥1 vanishing sprite.
            num_sprites_left += 1;
            num_in_first_dummy += 1;
        }
        if num_sprites_left > num_vanishing {
            num_sprites_left = num_vanishing;
        }
        if num_in_first_dummy > num_vanishing {
            num_in_first_dummy = num_vanishing;
        }

        // Avoid identical look between full parent and first dummy.
        if num_in_first_dummy == num_vanishing && num_sprites_left > 1 {
            num_sprites_left -= 1;
        }

        for v in num_sprites_left as usize..vanishing.len() {
            skip[vanishing[v]] = true;
        }
    }

    if num_appearing > 0 {
        // C++ uses lrint for appear count.
        let mut num_invis_left =
            ((d as f64) * (num_appearing as f64) / (num_uses as f64)).round() as i32;
        if num_invis_left > num_appearing {
            num_invis_left = num_appearing;
        }
        let unhide = (num_appearing - num_invis_left) as usize;
        for v in 0..unhide {
            skip[appearing[v]] = false;
        }
    }

    skip
}

/// Apply [`setup_sprite_use_vis`] onto `target.sprites[*].skip_drawing`.
///
/// `parent` supplies `num_uses` + vanish/appear flags; `target` receives skips
/// (may be the parent itself at full uses, or a use dummy).
pub fn apply_sprite_use_vis(
    target: &mut ClientObjectDef,
    parent_sprites: &[ObjectSprite],
    num_uses: i32,
    uses_remaining: i32,
) {
    let skip = setup_sprite_use_vis(parent_sprites, num_uses, uses_remaining);
    for (i, spr) in target.sprites.iter_mut().enumerate() {
        spr.skip_drawing = skip.get(i).copied().unwrap_or(false);
    }
}

/// Minimal object definition for client decisions + draw + shared OLC1 sim fields.
///
/// C++: `ObjectRecord` subset; Haxe: `ObjectData`. OLC1 format ≥ 3 stores
/// map/heat/speed/decay + blocking radii so server prefer-cache can stick.
#[derive(Debug, Clone)]
pub struct ClientObjectDef {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub permanent: bool,
    pub blocks_walking: bool,
    /// C++ `leftBlockingRadius` — wide objects block horizontal neighbors on path map.
    pub left_blocking_radius: i32,
    /// C++ `rightBlockingRadius`.
    pub right_blocking_radius: i32,
    /// C++ `sideAccess` — only approach from W/E (e.g. Ice Hole). Field `sideAccess=1`.
    pub side_access: bool,
    /// C++ `noBackAccess` — cannot approach from N. Description tag `+noBackAccess`.
    pub no_back_access: bool,
    pub containable: bool,
    pub food_value: i32,
    pub num_uses: i32,
    /// Haxe `ObjectData.useChance` — second CSV field on `numUses=N,chance` (0 = always consume).
    pub use_chance: f32,
    pub min_pickup_age: f32,
    pub person: i32,
    /// Floor layer object (`floor=1`). Drawn under map objects.
    pub floor: bool,
    /// Whole object drawn behind same-row players. C++ `drawBehindPlayer`.
    /// Forced true when wide (`left/rightBlockingRadius > 0`).
    pub draw_behind_player: bool,
    /// C++ `floorHugging` — walls that hug floor sprites; seeds `wall_layer`.
    pub floor_hugging: bool,
    /// C++ `wallLayer` — draw in wall sub-pass over same-row non-wall front objects.
    /// From `floorHugging`, `+wall` / `-wall` description tags (`setupWall`).
    pub wall_layer: bool,
    /// C++ `frontWall` — wall drawn after other walls (e.g. walls with signs).
    /// Description tag `+frontWall` when `wall_layer` is set.
    pub front_wall: bool,
    /// Haxe `mapChance` — natural world-gen weight (0 = never spawns naturally).
    pub map_chance: f32,
    /// Biome ids this natural object may spawn in (`mapChance=…#biomes_0,3`).
    pub biomes: Vec<i32>,
    /// C++/Haxe `heatValue` (object heat contribution; files store integer).
    pub heat_value: f32,
    /// Haxe `rValue` — wall/floor insulation; non-clothing + rValue>0 ⇒ wall.
    pub r_value: f32,
    /// Haxe `speedMult` — move multiplier while held/ridden (default 1).
    pub speed_mult: f32,
    /// Haxe `decayFactor` (default 1; ≤0 disables long-term decay).
    pub decay_factor: f32,
    /// Haxe `decaysToObj` (0 = unset; server patches trash pit for permanents).
    pub decays_to_obj: i32,
    /// Haxe `winterDecayFactor` — wild-food winter multi-use decay (0 = none).
    pub winter_decay_factor: f32,
    /// Haxe `springRegrowFactor` — spring multi-use regrow (0 = none).
    pub spring_regrow_factor: f32,
    /// Held item attachment offset in object units. C++ `heldOffset`.
    pub held_offset: (f32, f32),
    /// C++ `containOffsetX/Y` — applied to [`get_object_center_offset`] result.
    /// Parsed from description tags `+containOffsetX_N` / `+containOffsetY_N`.
    pub contain_offset: (i32, i32),
    /// True when held in hand (vs body). C++ `heldInHand` (heldInHand=1).
    pub held_in_hand: bool,
    /// Rideable vehicle. C++ `rideable` (heldInHand=2 in object files).
    pub rideable: bool,
    /// Clothing role: `n` none, `h` hat, `t` tunic, `s` shoe, `b` bottom, `p` backpack.
    pub clothing: char,
    /// Worn clothing attachment offset. C++ `clothingOffset`.
    pub clothing_offset: (f32, f32),
    /// Container slots. C++ `numSlots` / `slotPos`.
    pub num_slots: i32,
    pub slot_pos: Vec<(f32, f32)>,
    pub sprites: Vec<ObjectSprite>,
    /// Synthetic multi-use dummy ids for uses `1..num_uses-1` (Haxe `dummyObjects`).
    /// Index `uses - 1` → dummy id. Full `num_uses` uses base [`Self::id`].
    pub dummy_ids: Vec<i32>,
    /// If non-zero, this id is a multi-use dummy of that parent (Haxe `dummyParent`).
    pub dummy_parent: i32,
    /// C++ `variableDummyIDs` — one id per `$N` variant (`N` entries, 1-based labels).
    ///
    /// Generated by [`crate::content_binary::assign_variable_dummies`] (C++
    /// `autoGenerateVariableObjects` / `reAddObject`). Empty when no `$N` in
    /// description. Parent description is rewritten `$N` → `- ?` after assign.
    pub variable_dummy_ids: Vec<i32>,
    /// If non-zero, this id is a variable dummy of that parent (C++ `variableDummyParent`).
    pub variable_dummy_parent: i32,
    /// C++ `isVariableHidden` — `$N` appears after `#` comment (label not shown in UI).
    pub is_variable_hidden: bool,
    /// C++ `creationSound` — raw SoundUsage from `sounds=` CSV part 0.
    pub creation_sound: String,
    /// C++ `usingSound` — raw SoundUsage from `sounds=` CSV part 1 (also floor footstep).
    pub using_sound: String,
    /// C++ `eatingSound` — raw SoundUsage from `sounds=` CSV part 2.
    pub eating_sound: String,
    /// C++ `decaySound` — raw SoundUsage from `sounds=` CSV part 3.
    pub decay_sound: String,
    /// C++ `creationSoundInitialOnly` — only play creation when not a sprite-subset cycle.
    pub creation_sound_initial_only: bool,
    /// C++ `creationSoundForce` — play creation even when another sound already fired (PU held).
    pub creation_sound_force: bool,
    /// C++ `mainEyesOffset` — eyes sprite pos − head pos at age 30 (object units).
    ///
    /// Used to place PE `eyeEmot` on the face (not the head origin). Derived from
    /// sprite tags via [`Self::setup_eyes_and_mouth`]; default `(0,0)`.
    pub main_eyes_offset: (f32, f32),
    /// C++ `homeMarker` — permanent stake object (`homeMarker=1` / `eveHomeMarker`).
    ///
    /// // C++ LivingLifePage MX ~17238: our placement adds/removes homePosStack entry
    pub home_marker: bool,
    /// Haxe `ObjectData.useDistance` — USE/DROP range (default 1). Baked into OLC1 v7.
    // Haxe: ObjectData.useDistance
    pub use_distance: i32,
    /// Haxe `ObjectData.deadlyDistance` — combat / ranged min-range (tiles).
    // Haxe: ObjectData.deadlyDistance
    pub deadly_distance: f32,
    /// Haxe `ObjectData.moves` — animal walk class (`>0` ⇒ isAnimal; often from time-move).
    // Haxe: ObjectData.moves
    pub moves: i32,
}

impl Default for ClientObjectDef {
    fn default() -> Self {
        Self {
            id: 0,
            name: String::new(),
            description: String::new(),
            permanent: false,
            blocks_walking: false,
            left_blocking_radius: 0,
            right_blocking_radius: 0,
            side_access: false,
            no_back_access: false,
            containable: false,
            food_value: 0,
            num_uses: 0,
            use_chance: 0.0,
            min_pickup_age: 0.0,
            person: 0,
            floor: false,
            draw_behind_player: false,
            floor_hugging: false,
            wall_layer: false,
            front_wall: false,
            map_chance: 0.0,
            biomes: Vec::new(),
            heat_value: 0.0,
            r_value: 0.0,
            speed_mult: 1.0,
            decay_factor: 1.0,
            decays_to_obj: 0,
            winter_decay_factor: 0.0,
            spring_regrow_factor: 0.0,
            held_offset: (0.0, 0.0),
            contain_offset: (0, 0),
            held_in_hand: false,
            rideable: false,
            clothing: 'n',
            clothing_offset: (0.0, 0.0),
            num_slots: 0,
            slot_pos: Vec::new(),
            sprites: Vec::new(),
            dummy_ids: Vec::new(),
            dummy_parent: 0,
            variable_dummy_ids: Vec::new(),
            variable_dummy_parent: 0,
            is_variable_hidden: false,
            creation_sound: String::new(),
            using_sound: String::new(),
            eating_sound: String::new(),
            decay_sound: String::new(),
            creation_sound_initial_only: false,
            creation_sound_force: false,
            main_eyes_offset: (0.0, 0.0),
            home_marker: false,
            use_distance: 1,
            deadly_distance: 0.0,
            moves: 0,
        }
    }
}

/// Parse C++ variable-object marker `$N` from an object description.
///
/// Returns `(byte_index_of_dollar, N)` when `N >= 2`. C++ `autoGenerateVariableObjects`.
pub fn parse_variable_dollar_count(description: &str) -> Option<(usize, i32)> {
    let dollar = description.find('$')?;
    let after = &description[dollar + 1..];
    let num_len = after
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .count();
    if num_len == 0 {
        return None;
    }
    let n: i32 = after[..num_len].parse().ok()?;
    if n < 2 {
        return None;
    }
    Some((dollar, n))
}

/// C++ `getVarObjectLabel` — 1-based index → `"- A"`, `"- B"`, … `"- AA"`, …
pub fn var_object_label(in_number: i32) -> String {
    // C++: inNumber starts at 1; convert to 0-based then base-26 letters.
    let mut num_left = in_number - 1;
    let mut digits: Vec<char> = Vec::new();
    if num_left == 0 {
        digits.insert(0, 'A');
    }
    while num_left > 0 {
        let digit_number = num_left % 26;
        let digit = (b'A' + digit_number as u8) as char;
        digits.insert(0, digit);
        num_left -= digit_number;
        if num_left == 26 {
            digits.insert(0, 'A');
        }
        num_left /= 26;
        num_left -= 1;
    }
    format!("- {}", digits.iter().collect::<String>())
}

/// C++ `getVarObjectNumeral` — 1-based index with zero-pad width from `in_max`.
pub fn var_object_numeral(in_number: i32, in_max: i32) -> String {
    if in_max < 10 {
        format!("- {in_number}")
    } else if in_max < 100 {
        format!("- {in_number:02}")
    } else if in_max < 1000 {
        format!("- {in_number:03}")
    } else if in_max < 10000 {
        format!("- {in_number:04}")
    } else if in_max < 100000 {
        format!("- {in_number:05}")
    } else {
        format!("- {in_number}")
    }
}

/// C++ `+varNumeral` description tag — use numeric labels instead of A/B/C.
pub fn description_has_var_numeral(description: &str) -> bool {
    description.contains("+varNumeral")
}

/// C++ `isVariableHidden` — true when `$N` (or `- ?` after rewrite) sits after `#`.
pub fn variable_target_is_hidden(description: &str, target_byte_index: usize) -> bool {
    if let Some(comment) = description.find('#') {
        comment < target_byte_index
    } else {
        false
    }
}

/// Age used by C++ `setupEyesAndMouth` when sampling eyes for `mainEyesOffset`.
pub const MAIN_EYES_OFFSET_AGE: f32 = 30.0;

/// Rotate a 2D offset by `rot_turns` (full turns; C++ `rotate(v, -2π·rot)`).
///
/// PE eye placement applies head rot delta to `mainEyesOffset` before add.
pub fn rotate_offset_turns(ox: f32, oy: f32, rot_turns: f32) -> (f32, f32) {
    if rot_turns == 0.0 {
        return (ox, oy);
    }
    // C++ animationBank: rotate(offset, -2 * M_PI * animHeadRotDelta)
    let a = -rot_turns * std::f32::consts::TAU;
    let (c, s) = (a.cos(), a.sin());
    (ox * c - oy * s, ox * s + oy * c)
}

/// Object-space eyes draw point: head pose + rotated `mainEyesOffset`.
///
/// // C++: cPos = animHeadPos + rotate(flipX(mainEyesOffset), -2π·headRot)
/// Flip is applied later in screen space (caller); pass unflipped object coords.
pub fn eyes_anchor_from_head(
    head_x: f32,
    head_y: f32,
    head_rot_turns: f32,
    main_eyes_offset: (f32, f32),
) -> (f32, f32) {
    let (ox, oy) = rotate_offset_turns(main_eyes_offset.0, main_eyes_offset.1, head_rot_turns);
    (head_x + ox, head_y + oy)
}

/// C++ `HoldingPos` — attachment point returned from person draw for held objects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoldingPos {
    pub valid: bool,
    /// Object-space position of hand (or body when arms hidden/frozen).
    pub x: f32,
    pub y: f32,
    /// Sprite rotation in turns at attachment.
    pub rot: f32,
}

impl Default for HoldingPos {
    fn default() -> Self {
        Self {
            valid: false,
            x: 0.0,
            y: 0.0,
            rot: 0.0,
        }
    }
}

/// C++ `getArmHoldingParameters` — how a held object affects person limbs.
///
/// Returns `(hide_closest_arm, hide_all_limbs)`:
/// - `0` = attach to hand, no arm hide
/// - `-2` = freeze arms, attach to body (bulky held)
/// - `1` / `-1` = hide front/back arm (caller-set; not from this helper alone)
/// - rideable → hide all limbs (`hide_closest_arm` stays 0 for HoldingPos hand)
pub fn arm_holding_parameters(held: Option<&ClientObjectDef>) -> (i32, bool) {
    match held {
        None => (0, false),
        Some(o) if o.held_in_hand => (0, false),
        Some(o) if o.rideable => (0, true),
        Some(_) => (-2, false),
    }
}

/// Per-sprite geometry for [`get_object_center_offset`] (C++ `SpriteRecord` subset).
#[derive(Debug, Clone, Copy, Default)]
pub struct SpriteCenterInfo {
    pub visible_w: u32,
    pub visible_h: u32,
    pub center_x_offset: i32,
    pub center_y_offset: i32,
    /// C++ `multiplicativeBlend` — skipped when finding widest.
    pub multiplicative_blend: bool,
}

/// Parse signed int immediately after a description tag key (e.g. `+containOffsetX_`).
fn tag_i32_after(hay: &str, key: &str) -> i32 {
    let Some(pos) = hay.find(key) else {
        return 0;
    };
    let rest = &hay[pos + key.len()..];
    let end = rest
        .find(|c: char| c != '-' && !c.is_ascii_digit())
        .unwrap_or(rest.len());
    if end == 0 {
        return 0;
    }
    rest[..end].parse().unwrap_or(0)
}

/// C++ `setupContainOffset` — read `+containOffsetX_/Y_` from name/description.
pub fn parse_contain_offset_tags(name: &str, description: &str) -> (i32, i32) {
    // Tags usually live on the name line; description may duplicate after rewrite.
    let hay = if description.is_empty() {
        name.to_string()
    } else if name == description {
        name.to_string()
    } else {
        format!("{name} {description}")
    };
    (
        tag_i32_after(&hay, "+containOffsetX_"),
        tag_i32_after(&hay, "+containOffsetY_"),
    )
}

/// Apply description-derived fields that are not sole OLC1 flags.
///
/// C++: `setupContainOffset`, `setupNoBackAccess`, `setupWall` (objectBank).
pub fn apply_object_description_tags(def: &mut ClientObjectDef) {
    def.contain_offset = parse_contain_offset_tags(&def.name, &def.description);
    if def.description.contains("+noBackAccess") || def.name.contains("+noBackAccess") {
        def.no_back_access = true;
    }
    // OLC1 may omit homeMarker flag — recover from description tag on Home Marker.
    let hay = if def.description.is_empty() {
        def.name.as_str()
    } else {
        def.description.as_str()
    };
    if hay.contains("eveHomeMarker") {
        def.home_marker = true;
    }
    setup_wall(def);
}

/// C++ `setupWall` (objectBank.cpp) — wallLayer / frontWall from floorHugging + tags.
///
/// - Start `wall_layer = floor_hugging`
/// - If not wall yet and description has `+wall` → wall
/// - If wall and `-wall` → clear wall (and front)
/// - Else if wall and `+frontWall` → `front_wall`
fn setup_wall(def: &mut ClientObjectDef) {
    // C++ scans `description` only; name is often the same second-line text.
    let hay = if def.description.is_empty() {
        def.name.as_str()
    } else {
        def.description.as_str()
    };
    def.wall_layer = def.floor_hugging;
    def.front_wall = false;
    if !def.wall_layer && hay.contains("+wall") {
        def.wall_layer = true;
    }
    if def.wall_layer {
        if hay.contains("-wall") {
            def.wall_layer = false;
            def.front_wall = false;
        } else if hay.contains("+frontWall") {
            def.front_wall = true;
        }
    }
}

/// C++ `getObjectCenterOffset` (objectBank.cpp ~6496).
///
/// Center of the widest non-multiplicative sprite (lower Y wins ties), plus
/// rotated alpha-bbox center offsets and `containOffsetX/Y`. Used to recenter
/// non-person held objects (and container placement).
///
/// `sprite_info(id)` supplies meta from the sprite bank when available;
/// missing sprites are skipped.
pub fn get_object_center_offset(
    obj: &ClientObjectDef,
    mut sprite_info: impl FnMut(i32) -> Option<SpriteCenterInfo>,
) -> (f32, f32) {
    let mut best_i: Option<usize> = None;
    let mut best_w: i32 = 0;
    let mut best_y: f32 = 0.0;
    let mut best_info = SpriteCenterInfo::default();

    for (i, spr) in obj.sprites.iter().enumerate() {
        // C++ skips spriteInvisibleWhenWorn == 2 (only-when-worn).
        if spr.only_when_worn {
            continue;
        }
        let Some(info) = sprite_info(spr.sprite_id) else {
            continue;
        };
        if info.multiplicative_blend {
            continue;
        }
        let mut w = info.visible_w as i32;
        // C++: 90° / 270° (0.25 / 0.75 fractional turns) swaps visible W/H.
        let rot_abs = spr.rot.abs();
        let frac = rot_abs - rot_abs.floor();
        if (frac - 0.25).abs() < 1e-4 || (frac - 0.75).abs() < 1e-4 {
            w = info.visible_h as i32;
        }
        if best_i.is_none()
            || w > best_w
            || (w == best_w && spr.y < best_y)
        {
            best_i = Some(i);
            best_w = w;
            best_y = spr.y;
            best_info = info;
        }
    }

    let (cox, coy) = (
        obj.contain_offset.0 as f32,
        obj.contain_offset.1 as f32,
    );
    let Some(wi) = best_i else {
        return (cox, coy);
    };
    let spr = &obj.sprites[wi];
    // C++: rotate(centerOffset, +2π · spriteRot) — CCW, NOT the eyes-path −2π.
    let a = spr.rot * std::f32::consts::TAU;
    let (c, s) = (a.cos(), a.sin());
    let ox = best_info.center_x_offset as f32;
    let oy = best_info.center_y_offset as f32;
    let cx = ox * c - oy * s;
    let cy = ox * s + oy * c;
    (spr.x + cx + cox, spr.y + cy + coy)
}

/// Zero-info fallback when no sprite bank (visible size from placement only).
pub fn get_object_center_offset_simple(obj: &ClientObjectDef) -> (f32, f32) {
    get_object_center_offset(obj, |_| {
        Some(SpriteCenterInfo {
            visible_w: 1,
            visible_h: 1,
            center_x_offset: 0,
            center_y_offset: 0,
            multiplicative_blend: false,
        })
    })
}

/// C++ `computeHeldDrawPos` in **unflipped object-space** (caller flips X for screen).
///
/// `holding` from person draw; `held` is the held object def (for `heldOffset`).
/// Flip is only used to mirror holding-rot contribution like C++.
///
/// Non-person held objects subtract [`get_object_center_offset`] (P3#21).
/// Pass `center` when already computed with sprite-bank info; `None` uses
/// [`get_object_center_offset_simple`].
pub fn compute_held_draw_pos(
    holding: &HoldingPos,
    held: Option<&ClientObjectDef>,
    flip: bool,
) -> (f32, f32, f32) {
    compute_held_draw_pos_ex(holding, held, flip, None)
}

/// Like [`compute_held_draw_pos`] with optional precomputed object-center offset.
pub fn compute_held_draw_pos_ex(
    holding: &HoldingPos,
    held: Option<&ClientObjectDef>,
    flip: bool,
    center: Option<(f32, f32)>,
) -> (f32, f32, f32) {
    let (mut hx, mut hy) = if holding.valid {
        (holding.x, holding.y)
    } else {
        (0.0, 0.0)
    };
    let mut hrot = 0.0;
    if let Some(def) = held {
        let mut ox = def.held_offset.0;
        let mut oy = def.held_offset.1;
        // C++: if (!person) heldOffset = sub(heldOffset, getObjectCenterOffset)
        if def.person == 0 {
            let (cx, cy) = center.unwrap_or_else(|| get_object_center_offset_simple(def));
            ox -= cx;
            oy -= cy;
        }
        if holding.valid && holding.rot.abs() > 1e-8 && !def.rideable {
            // C++ rotates heldOffset by ±holding.rot before adding.
            let angle = if flip {
                holding.rot * std::f32::consts::TAU
            } else {
                -holding.rot * std::f32::consts::TAU
            };
            let (s, c) = angle.sin_cos();
            let rx = ox * c - oy * s;
            let ry = ox * s + oy * c;
            ox = rx;
            oy = ry;
            hrot = if flip { -holding.rot } else { holding.rot };
            while hrot > 1.0 {
                hrot -= 1.0;
            }
            while hrot < -1.0 {
                hrot += 1.0;
            }
        }
        hx += ox;
        hy += oy;
    }
    (hx, hy, hrot)
}

impl ClientObjectDef {
    /// True if any sprite is marked `behind_player` (C++ `anySpritesBehindPlayer`).
    pub fn any_sprites_behind_player(&self) -> bool {
        self.sprites.iter().any(|s| s.behind_player)
    }

    /// C++ `getBodyPartIndex` — top-most matching layer visible at age (default 0).
    fn body_part_index(&self, age: f32, pred: impl Fn(&ObjectSprite) -> bool) -> usize {
        if self.person == 0 {
            return 0;
        }
        for i in (0..self.sprites.len()).rev() {
            if pred(&self.sprites[i]) && self.sprites[i].visible_at_age(age) {
                return i;
            }
        }
        0
    }

    /// C++ `getBodyIndex`.
    pub fn body_index(&self, age: f32) -> usize {
        self.body_part_index(age, |s| s.is_body)
    }

    /// C++ `getHeadIndex`.
    pub fn head_index(&self, age: f32) -> usize {
        self.body_part_index(age, |s| s.is_head)
    }

    /// C++ `getBackFootIndex` / `getFrontFootIndex`.
    pub fn back_foot_index(&self, age: f32) -> usize {
        self.body_part_index(age, |s| s.is_back_foot)
    }

    pub fn front_foot_index(&self, age: f32) -> usize {
        self.body_part_index(age, |s| s.is_front_foot)
    }

    /// C++ `getEyesIndex` — top-most eyes layer visible at age, or `None` if missing.
    ///
    /// // C++ returns 0 when none; drawObjectAnim then maps 0 → -1 for emotes
    /// // ("never bottom layer"). We expose Option so PE can skip eyeEmot.
    pub fn eyes_index(&self, age: f32) -> Option<usize> {
        if self.person == 0 {
            return None;
        }
        for i in (0..self.sprites.len()).rev() {
            if self.sprites[i].is_eyes && self.sprites[i].visible_at_age(age) {
                // C++: eyesIndex == 0 → treat as non-existing for emote path
                if i == 0 {
                    return None;
                }
                return Some(i);
            }
        }
        None
    }

    /// C++ `getMouthIndex` — top-most mouth layer visible at age, or `None`.
    pub fn mouth_index(&self, age: f32) -> Option<usize> {
        if self.person == 0 {
            return None;
        }
        for i in (0..self.sprites.len()).rev() {
            if self.sprites[i].is_mouth && self.sprites[i].visible_at_age(age) {
                if i == 0 {
                    return None;
                }
                return Some(i);
            }
        }
        None
    }

    /// True when PE `eyeEmot` should draw (eyes layer exists for age).
    pub fn has_eyes_for_emot(&self, age: f32) -> bool {
        self.eyes_index(age).is_some() || {
            // Offset-only path: eyes tagged but only index 0 (rare) or offset set
            // from age-30 sample while current age uses different eyes.
            self.main_eyes_offset != (0.0, 0.0)
                && self.sprites.iter().any(|s| s.is_eyes)
        }
    }

    /// C++ `setupEyesAndMouth` — mark Eyes/Mouth from sprite tags; set `mainEyesOffset`.
    ///
    /// `sprite_tag(sprite_id)` returns the tag string from sprite bank / OLS1.
    /// Safe to call with empty tags (no-op flags). Idempotent.
    pub fn setup_eyes_and_mouth(&mut self, mut sprite_tag: impl FnMut(i32) -> Option<String>) {
        self.main_eyes_offset = (0.0, 0.0);
        for s in &mut self.sprites {
            s.is_eyes = false;
            s.is_mouth = false;
        }
        if self.person == 0 {
            return;
        }
        for s in &mut self.sprites {
            let Some(tag) = sprite_tag(s.sprite_id) else {
                continue;
            };
            if tag.contains("Eyes") {
                s.is_eyes = true;
            }
            if tag.contains("Mouth") {
                s.is_mouth = true;
            }
        }
        // C++: eyes visible at age 30 → mainEyesOffset = eyesPos − headPos
        let head_i = self.head_index(MAIN_EYES_OFFSET_AGE);
        let head_pos = self
            .sprites
            .get(head_i)
            .map(|s| (s.x, s.y))
            .unwrap_or((0.0, 0.0));
        for s in &self.sprites {
            if !s.is_eyes {
                continue;
            }
            // ageStart < 30 && ageEnd > 30 (C++ strict inequalities on range)
            let a0 = s.age_start;
            let a1 = s.age_end;
            let covers_30 = if a0 < 0.0 && a1 < 0.0 {
                true
            } else {
                (a0 < 0.0 || a0 < MAIN_EYES_OFFSET_AGE)
                    && (a1 < 0.0 || a1 > MAIN_EYES_OFFSET_AGE)
            };
            if covers_30 {
                self.main_eyes_offset = (s.x - head_pos.0, s.y - head_pos.1);
                break;
            }
        }
    }

    /// C++ `getHandIndices` — lowest two `invisHolding` layers by rest-pose Y.
    pub fn hand_indices(&self, age: f32) -> (Option<usize>, Option<usize>) {
        let mut hand_one: Option<usize> = None;
        let mut hand_two: Option<usize> = None;
        let mut y1 = f32::MAX;
        let mut y2 = f32::MAX;
        for (i, s) in self.sprites.iter().enumerate() {
            if !s.invis_holding || !s.visible_at_age(age) {
                continue;
            }
            let y = s.y;
            if y < y1 {
                hand_two = hand_one;
                y2 = y1;
                hand_one = Some(i);
                y1 = y;
            } else if y < y2 {
                hand_two = Some(i);
                y2 = y;
            }
        }
        (hand_one, hand_two)
    }

    /// C++ `getBackHandIndex` — left-er of two hands by rest-pose X.
    pub fn back_hand_index(&self, age: f32) -> Option<usize> {
        let (a, b) = self.hand_indices(age);
        match (a, b) {
            (Some(i), Some(j)) => {
                if self.sprites[i].x < self.sprites[j].x {
                    Some(i)
                } else {
                    Some(j)
                }
            }
            // Single hand: use it (C++ returns -1 for one-hand edge case; we keep it usable).
            (Some(i), None) => Some(i),
            _ => None,
        }
    }

    /// C++ `getFrontHandIndex` — right-er of two hands.
    pub fn front_hand_index(&self, age: f32) -> Option<usize> {
        let (a, b) = self.hand_indices(age);
        match (a, b) {
            (Some(i), Some(j)) => {
                if self.sprites[i].x > self.sprites[j].x {
                    Some(i)
                } else {
                    Some(j)
                }
            }
            (Some(i), None) => Some(i),
            _ => None,
        }
    }

    /// C++ `getLimbIndices` — walk parent chain from hand/foot until body.
    pub fn limb_indices_from(&self, tip: Option<usize>) -> Vec<usize> {
        let Some(mut next) = tip else {
            return Vec::new();
        };
        if next == 0 {
            let s = &self.sprites[0];
            if !(s.invis_holding || s.is_front_foot || s.is_back_foot) {
                return Vec::new();
            }
        }
        let mut out = Vec::new();
        while next < self.sprites.len() && !self.sprites[next].is_body {
            out.push(next);
            let p = self.sprites[next].parent;
            if p < 0 {
                break;
            }
            next = p as usize;
        }
        out
    }

    pub fn front_arm_indices(&self, age: f32) -> Vec<usize> {
        self.limb_indices_from(self.front_hand_index(age))
    }

    pub fn back_arm_indices(&self, age: f32) -> Vec<usize> {
        self.limb_indices_from(self.back_hand_index(age))
    }

    pub fn all_leg_indices(&self, age: f32) -> Vec<usize> {
        let mut out = Vec::new();
        if self.sprites.iter().any(|s| s.is_back_foot) {
            out.extend(self.limb_indices_from(Some(self.back_foot_index(age))));
        }
        if self.sprites.iter().any(|s| s.is_front_foot) {
            out.extend(self.limb_indices_from(Some(self.front_foot_index(age))));
        }
        // C++ also adds shadow roots below body — deferred.
        out
    }
}

/// Transition for client craft hints + OLT1 v2 server-parity fields.
///
/// C++: `TransRecord`; Haxe: `TransitionData`. OLT1 format 1 stores a subset;
/// format 2 adds reverse/no-use/move/min-use fractions.
/// P4#29: `switch_number_of_uses` (ServerSettings patches) + max-use table routing.
#[derive(Debug, Clone)]
pub struct ClientTransition {
    pub actor_id: i32,
    pub target_id: i32,
    pub new_actor_id: i32,
    pub new_target_id: i32,
    pub last_use_actor: bool,
    pub last_use_target: bool,
    pub auto_decay_seconds: f32,
    /// Field 5: reverse use on actor (Haxe `reverseUseActor`).
    pub reverse_use_actor: bool,
    /// Field 6: reverse use on target.
    pub reverse_use_target: bool,
    /// Field 9: no-use actor (skip use decrement).
    pub no_use_actor: bool,
    /// Field 10: no-use target.
    pub no_use_target: bool,
    /// Field 7: move type (0 none, 1–3 animal walk).
    pub move_dist: i32,
    /// Field 8: desired animal step radius.
    pub desired_move_dist: i32,
    /// Field 3: actor min-use fraction (1 = full).
    pub actor_min_use_fraction: f32,
    /// Field 4: target min-use fraction.
    pub target_min_use_fraction: f32,
    /// Haxe `switchNumberOfUses` — ServerSettings patches (dough/masa on table).
    /// Not in transition files; applied after load / serialized in OLT1 flag bit7.
    pub switch_number_of_uses: bool,
}

impl Default for ClientTransition {
    fn default() -> Self {
        Self {
            actor_id: 0,
            target_id: 0,
            new_actor_id: 0,
            new_target_id: 0,
            last_use_actor: false,
            last_use_target: false,
            auto_decay_seconds: 0.0,
            reverse_use_actor: false,
            reverse_use_target: false,
            no_use_actor: false,
            no_use_target: false,
            move_dist: 0,
            desired_move_dist: 0,
            actor_min_use_fraction: 0.0,
            target_min_use_fraction: 0.0,
            switch_number_of_uses: false,
        }
    }
}

/// In-memory content tables.
#[derive(Debug, Clone, Default)]
pub struct ClientContent {
    pub objects: HashMap<i32, ClientObjectDef>,
    /// Primary non-last-use transitions keyed by (actor, target).
    pub transitions: HashMap<(i32, i32), ClientTransition>,
    /// Last-use actor/target transitions (filename `_LA` / `_LT` / `_L`).
    /// Separate map so A_B and A_B_LA do not overwrite each other (server parity).
    pub transitions_last_use: HashMap<(i32, i32), ClientTransition>,
    /// Max-use target transitions (Haxe `maxUseTransitions` — well site full → complete).
    /// Populated when a non-last-use (actor,target) has both targetRemains and non-remains
    /// variants; non-remains goes here (server `insert_normal_or_max_use`).
    pub transitions_max_use: HashMap<(i32, i32), ClientTransition>,
    pub data_version: i32,
    pub root: Option<PathBuf>,
    /// Dummy object id → parent base id (Haxe `dummyParent` / server `dummy_parent`).
    pub dummy_parent: HashMap<i32, i32>,
    /// C++ `categoryBank` forward + reverse maps (C-CAT lite + pattern + pick).
    pub categories: CategoryBank,
    /// True when lite+pattern category transitions are already concrete in
    /// `transitions` / `transitions_last_use` (text expand or baked OLT1 flag).
    /// Written as OLT1 header [`crate::content_binary::OLT1_F_CATEGORY_EXPANDED`].
    pub transitions_category_expanded: bool,
}

impl ClientContent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, id: i32) -> Option<&ClientObjectDef> {
        self.objects.get(&id)
    }

    /// P3#19: derive `is_eyes` / `is_mouth` / `main_eyes_offset` for all persons.
    ///
    /// Call once after content + sprite meta are available (graphics path).
    /// `sprite_tag` should resolve sprite bank / OLS1 tags (`Eyes` / `Mouth`).
    pub fn setup_eyes_and_mouth(
        &mut self,
        mut sprite_tag: impl FnMut(i32) -> Option<String>,
    ) {
        for def in self.objects.values_mut() {
            if def.person != 0 {
                def.setup_eyes_and_mouth(&mut sprite_tag);
            }
        }
    }

    /// Resolve multi-use or variable dummy id to base object id (identity if not a dummy).
    ///
    /// C++ `getObjectParent` lite — checks `useDummyParent` then `variableDummyParent`.
    pub fn base_object_id(&self, id: i32) -> i32 {
        if let Some(&p) = self.dummy_parent.get(&id) {
            return p;
        }
        if let Some(def) = self.objects.get(&id) {
            if def.variable_dummy_parent != 0 {
                return def.variable_dummy_parent;
            }
            if def.dummy_parent != 0 {
                return def.dummy_parent;
            }
        }
        id
    }

    /// C++ `getObject(id)->blocksWalking` only — **not** `permanent`.
    ///
    /// `computePathToDest` unblocks solely when `!getObject()->blocksWalking`
    /// (permanent bushes/trees that you can walk through stay open).
    pub fn blocks_walking(&self, id: i32) -> bool {
        if id <= 0 {
            return false;
        }
        let base = self.base_object_id(id);
        self.objects
            .get(&base)
            .map(|d| d.blocks_walking)
            .unwrap_or(false)
    }

    /// C++ wide object: either blocking radius > 0.
    pub fn is_wide(&self, id: i32) -> bool {
        let base = self.base_object_id(id);
        self.objects
            .get(&base)
            .map(|d| d.left_blocking_radius > 0 || d.right_blocking_radius > 0)
            .unwrap_or(false)
    }

    pub fn food_value(&self, id: i32) -> i32 {
        let base = self.base_object_id(id);
        self.objects.get(&base).map(|d| d.food_value).unwrap_or(0)
    }

    /// C++ `getObject(id)->sideAccess`.
    pub fn side_access(&self, id: i32) -> bool {
        if id <= 0 {
            return false;
        }
        let base = self.base_object_id(id);
        self.objects
            .get(&base)
            .map(|d| d.side_access)
            .unwrap_or(false)
    }

    /// C++ `getObject(id)->noBackAccess` (description `+noBackAccess`).
    pub fn no_back_access(&self, id: i32) -> bool {
        if id <= 0 {
            return false;
        }
        let base = self.base_object_id(id);
        self.objects
            .get(&base)
            .map(|d| d.no_back_access)
            .unwrap_or(false)
    }

    /// Normal (non last-use) transition for actor on target. Resolves dummy ids.
    pub fn find_transition(&self, actor: i32, target: i32) -> Option<&ClientTransition> {
        let a = self.base_object_id(actor);
        let t = self.base_object_id(target);
        self.transitions.get(&(a, t))
    }

    /// Last-use variant (Haxe LA/LT when multi-use is exhausted).
    pub fn find_transition_last_use(&self, actor: i32, target: i32) -> Option<&ClientTransition> {
        let a = self.base_object_id(actor);
        let t = self.base_object_id(target);
        self.transitions_last_use.get(&(a, t))
    }

    /// Haxe `GetTransition(..., maxUseTarget=true)` — complete when reverse at max uses.
    #[inline]
    pub fn find_transition_max_use(&self, actor: i32, target: i32) -> Option<&ClientTransition> {
        let a = self.base_object_id(actor);
        let t = self.base_object_id(target);
        self.transitions_max_use.get(&(a, t))
    }

    /// Prefer last-use table when `prefer_last_use`, else normal; fall back either way.
    pub fn find_transition_prefer(
        &self,
        actor: i32,
        target: i32,
        prefer_last_use: bool,
    ) -> Option<&ClientTransition> {
        if prefer_last_use {
            self.find_transition_last_use(actor, target)
                .or_else(|| self.find_transition(actor, target))
        } else {
            self.find_transition(actor, target)
                .or_else(|| self.find_transition_last_use(actor, target))
        }
    }

    /// Max-use lookup + probSet materialize.
    pub fn find_ptrans_max_use(
        &self,
        actor: i32,
        target: i32,
        rand_new_actor: f32,
        rand_new_target: f32,
    ) -> Option<ClientTransition> {
        self.find_transition_max_use(actor, target)
            .map(|t| self.materialize_transition(t, rand_new_actor, rand_new_target))
    }

    /// Haxe dough/masa-on-table `switchNumberOfUses = true` patches (ServerSettings).
    ///
    /// Idempotent. Call after text or OLT1 load so queries match server.
    pub fn apply_default_switch_number_of_uses_patches(&mut self) {
        apply_default_switch_number_of_uses_patches(self);
    }

    /// C++ `getPTrans`-style materialize: resolve probSet parents on newActor/newTarget.
    ///
    /// `rand_new_actor` / `rand_new_target` in `[0, 1]` (server `transform_target` weight sum).
    /// Non-probSet ids pass through unchanged.
    pub fn materialize_transition(
        &self,
        tr: &ClientTransition,
        rand_new_actor: f32,
        rand_new_target: f32,
    ) -> ClientTransition {
        let mut out = tr.clone();
        out.new_actor_id = self
            .categories
            .pick_from_prob_set(out.new_actor_id, rand_new_actor);
        out.new_target_id = self
            .categories
            .pick_from_prob_set(out.new_target_id, rand_new_target);
        out
    }

    /// Lookup + materialize probSet outcomes (C++ `getPTrans` subset without chance/meta).
    pub fn find_ptrans(
        &self,
        actor: i32,
        target: i32,
        rand_new_actor: f32,
        rand_new_target: f32,
    ) -> Option<ClientTransition> {
        self.find_transition(actor, target)
            .map(|t| self.materialize_transition(t, rand_new_actor, rand_new_target))
    }

    /// Last-use lookup + probSet materialize.
    pub fn find_ptrans_last_use(
        &self,
        actor: i32,
        target: i32,
        rand_new_actor: f32,
        rand_new_target: f32,
    ) -> Option<ClientTransition> {
        self.find_transition_last_use(actor, target)
            .map(|t| self.materialize_transition(t, rand_new_actor, rand_new_target))
    }

    /// Load `categories/` and expand member + pattern transitions.
    ///
    /// // C++: `initCategoryBank*` + `autoGenerateCategoryTransitions` (lite + pattern)
    /// Idempotent with respect to already-expanded concrete keys.
    /// Prob-set outcomes stay abstract until [`Self::materialize_transition`].
    pub fn load_categories_and_expand(&mut self, categories_dir: impl AsRef<Path>) -> usize {
        let bank = CategoryBank::load_from_dir(categories_dir);
        self.apply_category_bank(bank)
    }

    /// Install a pre-built [`CategoryBank`] and run lite + pattern transition expansion.
    pub fn apply_category_bank(&mut self, bank: CategoryBank) -> usize {
        let added = expand_category_transitions(
            &mut self.transitions,
            &mut self.transitions_last_use,
            &mut self.transitions_max_use,
            &bank,
        );
        self.categories = bank;
        self.transitions_category_expanded = true;
        added
    }

    /// Install [`CategoryBank`] without expanding transitions (OLT1 already expanded).
    pub fn set_category_bank(&mut self, bank: CategoryBank) {
        self.categories = bank;
    }

    /// If `root/categories` exists, load + expand (no-op when missing).
    /// Marks [`Self::transitions_category_expanded`] even when the dir is absent
    /// (nothing left to expand for this tree).
    pub fn maybe_load_categories_from_root(&mut self, root: impl AsRef<Path>) -> usize {
        let cat_dir = root.as_ref().join("categories");
        if cat_dir.is_dir() {
            self.load_categories_and_expand(cat_dir)
        } else {
            self.transitions_category_expanded = true;
            0
        }
    }

    /// Load `categories/` into the bank only (no transition expand).
    /// Used when OLT1 was baked with category expansions already concrete.
    pub fn maybe_load_category_bank_from_root(&mut self, root: impl AsRef<Path>) {
        let cat_dir = root.as_ref().join("categories");
        if cat_dir.is_dir() {
            self.categories = CategoryBank::load_from_dir(cat_dir);
        }
    }

    /// Load from a OneLifeData7-style root (has `objects/`, `transitions/`).
    /// Does **not** assign multi-use dummies; call
    /// [`crate::content_binary::assign_multi_use_dummies`] or use
    /// [`Self::load_prefer_cache`].
    ///
    /// Loads categories and expands member transitions when `categories/` is present.
    pub fn load_from_dir(root: impl AsRef<Path>) -> Result<Self, String> {
        let root = root.as_ref();
        let mut db = Self {
            root: Some(root.to_path_buf()),
            ..Self::default()
        };
        let ver_path = root.join("dataVersionNumber.txt");
        if ver_path.exists() {
            if let Ok(s) = fs::read_to_string(&ver_path) {
                db.data_version = s.trim().parse().unwrap_or(0);
            }
        }
        let obj_dir = root.join("objects");
        if obj_dir.is_dir() {
            load_objects_dir(&obj_dir, &mut db.objects)?;
        }
        let tr_dir = root.join("transitions");
        if tr_dir.is_dir() {
            load_transitions_dir(&tr_dir, &mut db)?;
        }
        // C-CAT + C-TRANS: lite member expand + pattern second pass (once at load).
        db.maybe_load_categories_from_root(root);
        // Haxe ServerSettings switchNumberOfUses patches (dough/masa-on-table).
        apply_default_switch_number_of_uses_patches(&mut db);
        Ok(db)
    }

    /// Cache-first load: `root/cache` OLC1/OLT1 when valid, else text + dummies.
    pub fn load_prefer_cache(root: impl AsRef<Path>) -> Result<Self, String> {
        crate::content_binary::load_prefer_cache(root)
    }

    /// Cache-first load with optional P5#36 progress callback.
    pub fn load_prefer_cache_with_progress(
        root: impl AsRef<Path>,
        on_progress: crate::load_progress::ProgressCb<'_>,
    ) -> Result<Self, String> {
        crate::content_binary::load_prefer_cache_with_progress(root, on_progress)
    }

    /// Load only from a baked `cache/` directory.
    pub fn load_from_cache(cache_dir: impl AsRef<Path>) -> Result<Self, String> {
        crate::content_binary::load_from_cache(cache_dir, None)
    }

    /// Try common Open Life content locations on this machine.
    /// Prefers binary cache when present under each root.
    pub fn load_default_locations() -> Result<Self, String> {
        Self::load_default_locations_with_progress(None)
    }

    /// Same as [`Self::load_default_locations`] with optional P5#36 progress callback.
    pub fn load_default_locations_with_progress(
        mut on_progress: crate::load_progress::ProgressCb<'_>,
    ) -> Result<Self, String> {
        let candidates = [
            std::env::var("OHOL_CONTENT_DIR").unwrap_or_default(),
            r"C:\OhOl\OpenLife\openlife\RustServer\content\OneLifeData7".into(),
            r"C:\OhOl\OpenLife\OneLifeData7".into(),
            r"C:\OhOl\OpenLife\openlife\RustServer\content".into(),
        ];
        for c in candidates {
            if c.is_empty() {
                continue;
            }
            let p = Path::new(&c);
            if p.join("objects").is_dir() || p.join("dataVersionNumber.txt").exists() {
                return Self::load_prefer_cache_with_progress(p, crate::load_progress::reborrow_cb(&mut on_progress));
            }
            // bare cache dir
            if p.join("olc1_objects.bin").exists() || p.join("cache").join("olc1_objects.bin").exists()
            {
                crate::load_progress::report_stage(
                    crate::load_progress::LoadStage::Content,
                    0.0,
                    Some("cache_dir"),
                    crate::load_progress::reborrow_cb(&mut on_progress),
                );
                let db = if p.join("olc1_objects.bin").exists() {
                    Self::load_from_cache(p)?
                } else {
                    Self::load_from_cache(p.join("cache"))?
                };
                crate::load_progress::report_stage(
                    crate::load_progress::LoadStage::Content,
                    1.0,
                    Some("cache_dir"),
                    crate::load_progress::reborrow_cb(&mut on_progress),
                );
                return Ok(db);
            }
        }
        Err("no OneLifeData7 content dir found (set OHOL_CONTENT_DIR)".into())
    }
}

fn load_objects_dir(dir: &Path, out: &mut HashMap<i32, ClientObjectDef>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for ent in entries.flatten() {
        let path = ent.path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if stem == "nextObjectNumber" {
            continue;
        }
        let id: i32 = match stem.parse() {
            Ok(i) if i > 0 => i,
            _ => continue,
        };
        let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        if let Some(def) = parse_object_txt(id, &text) {
            out.insert(id, def);
        }
    }
    Ok(())
}

/// C++ `sparseCommaLineToBoolArray` helper for object .txt index lists.
///
/// Values are sprite indices; `-1` (or non-parseable) is ignored.
fn sparse_index_line_set_flag(
    v: &str,
    sprites: &mut [ObjectSprite],
    mut set: impl FnMut(&mut ObjectSprite),
) {
    for part in v.split(',') {
        let t = part.trim();
        if t == "-1" || t.is_empty() {
            continue;
        }
        if let Ok(idx) = t.parse::<usize>() {
            if let Some(spr) = sprites.get_mut(idx) {
                set(spr);
            }
        }
    }
}

/// Split object `sounds=` CSV into creation/using/eating/decay SoundUsage strings.
///
/// C++ `objectBank`: four comma-separated [`SoundUsage`](crate::sound_bank::SoundUsage)
/// fields. Blank / missing parts become empty (caller treats as no sound).
pub fn parse_object_sounds_csv(v: &str) -> [String; 4] {
    let mut out = [
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ];
    for (i, part) in v.split(',').take(4).enumerate() {
        out[i] = part.trim().to_string();
    }
    out
}

/// True when a raw SoundUsage string is blank / unset (no playback).
pub fn sound_usage_is_blank(s: &str) -> bool {
    let t = s.trim();
    t.is_empty()
        || t == "0"
        || t == "-1"
        || t == "-1:0"
        || t == "-1:0.0"
        || t == "0:0"
        || t == "0:0.0"
}

/// Parse OHOL object `.txt` (id\nname\n… key=value lines + sprite blocks).
pub fn parse_object_txt(id: i32, text: &str) -> Option<ClientObjectDef> {
    let mut lines = text.lines();
    let first = lines.next()?.trim();
    // File may start with `id=N`, bare numeric id, or name.
    let name = if first.starts_with("id=") {
        lines.next().unwrap_or("").trim().to_string()
    } else if first.parse::<i32>().is_ok() {
        lines.next().unwrap_or("").trim().to_string()
    } else {
        first.to_string()
    };
    let mut def = ClientObjectDef {
        id,
        name: name.clone(),
        description: name,
        ..Default::default()
    };
    let mut current_sprite: Option<ObjectSprite> = None;
    for line in lines {
        let line = line.trim();
        if let Some((k, v)) = line.split_once('=') {
            let v = v.trim();
            match k {
                "permanent" => {
                    def.permanent = v.split(',').next().unwrap_or(v).trim() == "1";
                }
                "homeMarker" => {
                    def.home_marker = v.split(',').next().unwrap_or(v).trim() == "1";
                }
                "blocksWalking" => {
                    // C++ objectBank one-line:
                    // blocksWalking=%d,leftBlockingRadius=%d,rightBlockingRadius=%d,drawBehindPlayer=%d
                    // Also accept split multi-line keys below.
                    for part in line.split(',') {
                        if let Some((kk, vv)) = part.split_once('=') {
                            match kk.trim() {
                                "blocksWalking" => {
                                    def.blocks_walking = vv.trim() == "1";
                                }
                                "leftBlockingRadius" => {
                                    def.left_blocking_radius =
                                        vv.trim().parse().unwrap_or(0);
                                }
                                "rightBlockingRadius" => {
                                    def.right_blocking_radius =
                                        vv.trim().parse().unwrap_or(0);
                                }
                                "drawBehindPlayer" => {
                                    def.draw_behind_player = vv.trim() == "1";
                                }
                                _ => {}
                            }
                        }
                    }
                }
                // Standalone (multi-line / test fixtures) wide radii + behind flag
                "leftBlockingRadius" => {
                    def.left_blocking_radius = v
                        .split(',')
                        .next()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(0);
                }
                "rightBlockingRadius" => {
                    def.right_blocking_radius = v
                        .split(',')
                        .next()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(0);
                }
                "drawBehindPlayer" => {
                    def.draw_behind_player =
                        v.split(',').next().unwrap_or(v).trim() == "1";
                }
                // C++ floorHugging — seeds wallLayer in setupWall
                "floorHugging" => {
                    def.floor_hugging =
                        v.split(',').next().unwrap_or(v).trim() == "1";
                }
                // C++ sparse indices: spritesDrawnBehind=0,1,2 (after sprite blocks)
                "spritesDrawnBehind" => {
                    if let Some(s) = current_sprite.take() {
                        def.sprites.push(s);
                    }
                    for part in v.split(',') {
                        if let Ok(idx) = part.trim().parse::<usize>() {
                            if let Some(spr) = def.sprites.get_mut(idx) {
                                spr.behind_player = true;
                            }
                        }
                    }
                }
                // C++ sparse `useVanishIndex` / `useAppearIndex` (after numUses).
                // `-1` = none (same as spritesDrawnBehind empty sentinel).
                "useVanishIndex" => {
                    if let Some(s) = current_sprite.take() {
                        def.sprites.push(s);
                    }
                    sparse_index_line_set_flag(v, &mut def.sprites, |spr| {
                        spr.use_vanish = true;
                    });
                }
                "useAppearIndex" => {
                    if let Some(s) = current_sprite.take() {
                        def.sprites.push(s);
                    }
                    sparse_index_line_set_flag(v, &mut def.sprites, |spr| {
                        spr.use_appear = true;
                    });
                }
                "containable" => def.containable = v.trim() == "1",
                "sideAccess" => {
                    def.side_access = v.split(',').next().unwrap_or(v).trim() == "1";
                }
                "foodValue" => def.food_value = v.trim().parse().unwrap_or(0),
                "heatValue" => {
                    def.heat_value = v
                        .split(|c| c == ',' || c == '#')
                        .next()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(0.0);
                }
                "rValue" => {
                    def.r_value = v
                        .split(|c| c == ',' || c == '#')
                        .next()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(0.0);
                }
                "speedMult" => {
                    def.speed_mult = v
                        .split(|c| c == ',' || c == '#')
                        .next()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(1.0);
                }
                "decayFactor" => {
                    def.decay_factor = v
                        .split(|c| c == ',' || c == '#')
                        .next()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(1.0);
                }
                "decaysToObj" | "decaysTo" => {
                    def.decays_to_obj = v
                        .split(|c| c == ',' || c == '#')
                        .next()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(0);
                }
                "winterDecayFactor" => {
                    def.winter_decay_factor = v
                        .split(|c| c == ',' || c == '#')
                        .next()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(0.0);
                }
                "springRegrowFactor" => {
                    def.spring_regrow_factor = v
                        .split(|c| c == ',' || c == '#')
                        .next()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(0.0);
                }
                "mapChance" => {
                    // C++/Haxe: `mapChance=1.000000#biomes_0,3,4` (biomes after #)
                    let full = v;
                    let (chance_s, rest2) = if let Some(i) = full.find('#') {
                        (&full[..i], Some(&full[i + 1..]))
                    } else {
                        (full.split(',').next().unwrap_or(full), None)
                    };
                    def.map_chance = chance_s.trim().parse().unwrap_or(0.0);
                    if let Some(r) = rest2 {
                        let biomes_part = r
                            .strip_prefix("biomes_")
                            .or_else(|| r.strip_prefix("biomes="))
                            .unwrap_or(r);
                        let biomes_part = biomes_part
                            .split("heatValue=")
                            .next()
                            .unwrap_or(biomes_part)
                            .trim_end_matches(',')
                            .trim();
                        def.biomes = biomes_part
                            .split(|c| c == ',' || c == ' ')
                            .filter_map(|s| {
                                let s = s.trim();
                                if s.is_empty() {
                                    None
                                } else {
                                    s.parse().ok()
                                }
                            })
                            .collect();
                    }
                }
                "numUses" => {
                    // C++/Haxe: `numUses=N` or `numUses=N,useChance`
                    let mut parts = v.split(',');
                    def.num_uses = parts
                        .next()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(0);
                    if let Some(c) = parts.next() {
                        def.use_chance = c.trim().parse().unwrap_or(0.0);
                    }
                }
                "minPickupAge" => {
                    def.min_pickup_age = v
                        .split(',')
                        .next()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(0.0);
                }
                "person" => {
                    def.person = v
                        .split(',')
                        .next()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(0);
                }
                "floor" => {
                    def.floor = v.split(',').next().unwrap_or(v).trim() == "1";
                }
                "heldOffset" => {
                    let mut it = v.split(',');
                    def.held_offset.0 = it.next().and_then(|t| t.parse().ok()).unwrap_or(0.0);
                    def.held_offset.1 = it.next().and_then(|t| t.parse().ok()).unwrap_or(0.0);
                }
                // C++: heldInHand=1 hand-held; heldInHand=2 rideable
                "heldInHand" => {
                    let n: i32 = v
                        .split(',')
                        .next()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(0);
                    def.held_in_hand = n == 1;
                    def.rideable = n == 2;
                    if def.rideable {
                        def.held_in_hand = false;
                    }
                }
                "rideable" => {
                    // Some forks use a separate key; C++ packs into heldInHand=2.
                    if v.split(',').next().unwrap_or(v).trim() == "1" {
                        def.rideable = true;
                        def.held_in_hand = false;
                    }
                }
                // Sparse body-part indices (after sprite blocks), C++ objectBank
                "bodyIndex" | "headIndex" | "backFootIndex" | "frontFootIndex" => {
                    if let Some(s) = current_sprite.take() {
                        def.sprites.push(s);
                    }
                    let flag = k;
                    for part in v.split(',') {
                        if let Ok(idx) = part.trim().parse::<usize>() {
                            if let Some(spr) = def.sprites.get_mut(idx) {
                                match flag {
                                    "bodyIndex" => spr.is_body = true,
                                    "headIndex" => spr.is_head = true,
                                    "backFootIndex" => spr.is_back_foot = true,
                                    "frontFootIndex" => spr.is_front_foot = true,
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                "clothing" => {
                    def.clothing = v
                        .trim()
                        .chars()
                        .next()
                        .unwrap_or('n');
                }
                "clothingOffset" => {
                    let mut it = v.split(',');
                    def.clothing_offset.0 = it.next().and_then(|t| t.parse().ok()).unwrap_or(0.0);
                    def.clothing_offset.1 = it.next().and_then(|t| t.parse().ok()).unwrap_or(0.0);
                }
                "numSlots" => {
                    // C++: `numSlots=%d#timeStretch=%f`
                    def.num_slots = v
                        .split('#')
                        .next()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(0);
                }
                "slotPos" => {
                    // C++: `slotPos=%lf,%lf,vert=%d,parent=%d` (or simpler x,y)
                    let mut it = v.split(',');
                    let sx = it.next().and_then(|t| t.parse().ok()).unwrap_or(0.0);
                    let sy = it.next().and_then(|t| t.parse().ok()).unwrap_or(0.0);
                    def.slot_pos.push((sx, sy));
                }
                "description" => def.description = v.to_string(),
                // C++ objectBank: sounds=creation,using,eating,decay (SoundUsage CSV)
                "sounds" => {
                    let parts = parse_object_sounds_csv(v);
                    def.creation_sound = parts[0].clone();
                    def.using_sound = parts[1].clone();
                    def.eating_sound = parts[2].clone();
                    def.decay_sound = parts[3].clone();
                }
                "creationSoundInitialOnly" => {
                    def.creation_sound_initial_only = v.trim() == "1";
                }
                "creationSoundForce" => {
                    def.creation_sound_force = v.trim() == "1";
                }
                // Haxe: ObjectData.useDistance / deadlyDistance / moves (OLC1 v7 trailer)
                "useDistance" => {
                    def.use_distance = v
                        .split(|c| c == ',' || c == '#')
                        .next()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(1);
                }
                "deadlyDistance" => {
                    def.deadly_distance = v
                        .split(|c| c == ',' || c == '#')
                        .next()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(0.0);
                }
                "moves" => {
                    def.moves = v
                        .split(|c| c == ',' || c == '#')
                        .next()
                        .and_then(|s| s.trim().parse().ok())
                        .unwrap_or(0);
                }
                "spriteID" => {
                    if let Some(s) = current_sprite.take() {
                        def.sprites.push(s);
                    }
                    current_sprite = Some(ObjectSprite {
                        sprite_id: v.parse().unwrap_or(0),
                        age_start: -1.0,
                        age_end: -1.0,
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        parent: -1,
                        ..Default::default()
                    });
                }
                "pos" => {
                    if let Some(ref mut s) = current_sprite {
                        let mut it = v.split(',');
                        s.x = it.next().and_then(|t| t.parse().ok()).unwrap_or(0.0);
                        s.y = it.next().and_then(|t| t.parse().ok()).unwrap_or(0.0);
                    }
                }
                "rot" => {
                    if let Some(ref mut s) = current_sprite {
                        s.rot = v.parse().unwrap_or(0.0);
                    }
                }
                "hFlip" => {
                    if let Some(ref mut s) = current_sprite {
                        s.h_flip = v.trim() == "1";
                    }
                }
                "color" => {
                    if let Some(ref mut s) = current_sprite {
                        let mut it = v.split(',');
                        s.r = it.next().and_then(|t| t.parse().ok()).unwrap_or(1.0);
                        s.g = it.next().and_then(|t| t.parse().ok()).unwrap_or(1.0);
                        s.b = it.next().and_then(|t| t.parse().ok()).unwrap_or(1.0);
                    }
                }
                "ageRange" => {
                    if let Some(ref mut s) = current_sprite {
                        let mut it = v.split(',');
                        s.age_start = it.next().and_then(|t| t.parse().ok()).unwrap_or(-1.0);
                        s.age_end = it.next().and_then(|t| t.parse().ok()).unwrap_or(-1.0);
                    }
                }
                "parent" => {
                    if let Some(ref mut s) = current_sprite {
                        s.parent = v.trim().parse().unwrap_or(-1);
                    }
                }
                "invisHolding" | "invisWorn" | "behindSlots" => {
                    // Combined line: invisHolding=0,invisWorn=0,behindSlots=0
                    // C++ invisWorn: 0=always, 1=hide when worn, 2=only when worn.
                    if let Some(ref mut s) = current_sprite {
                        for part in line.split(',') {
                            if let Some((kk, vv)) = part.split_once('=') {
                                match kk.trim() {
                                    "invisHolding" => s.invis_holding = vv.trim() == "1",
                                    "invisWorn" => {
                                        let n: i32 = vv.trim().parse().unwrap_or(0);
                                        s.invis_worn = n == 1;
                                        s.only_when_worn = n == 2;
                                    }
                                    "behindSlots" => s.behind_slots = vv.trim() == "1",
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(s) = current_sprite {
        def.sprites.push(s);
    }
    // C++: wide objects force drawBehindPlayer so players walk in front of them.
    if def.left_blocking_radius > 0 || def.right_blocking_radius > 0 {
        def.draw_behind_player = true;
    }
    if def.description.is_empty() {
        def.description = def.name.clone();
    }
    apply_object_description_tags(&mut def);
    Some(def)
}

fn load_transitions_dir(dir: &Path, db: &mut ClientContent) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for ent in entries.flatten() {
        let path = ent.path();
        if path.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        // e.g. 33_0.txt, 33_0_LA.txt
        let base = name.split('_').collect::<Vec<_>>();
        if base.len() < 2 {
            continue;
        }
        let actor: i32 = base[0].parse().unwrap_or(0);
        let target: i32 = base[1]
            .trim_end_matches(|c: char| c.is_ascii_alphabetic())
            .parse()
            .unwrap_or(0);
        let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        if let Some(tr) = parse_transition_txt(actor, target, name, &text) {
            insert_transition_record(db, tr);
        }
    }
    Ok(())
}

/// Haxe `targetRemains`: target id unchanged by transition (`targetID == newTargetID`).
#[inline]
pub fn target_remains(tr: &ClientTransition) -> bool {
    tr.target_id >= 0 && tr.target_id == tr.new_target_id
}

/// Insert non-last-use transition with Haxe double-transition maxUse handling.
///
/// When two normal transitions share `(actor, target)` and one has
/// `targetRemains` while the other does not, the non-remains row goes to
/// [`ClientContent::transitions_max_use`] (well site / pond full → complete).
/// Same-kind duplicates keep first (category expand / file order).
///
/// // Haxe: `TransitionImporter.addTransition` maxUse pair
/// // Server: `insert_normal_or_max_use`
pub fn insert_normal_or_max_use(db: &mut ClientContent, t: ClientTransition) -> bool {
    let key = (t.actor_id, t.target_id);
    let remains = target_remains(&t);
    if let Some(existing) = db.transitions.get(&key).cloned() {
        let exist_remains = target_remains(&existing);
        if exist_remains && !remains {
            db.transitions_max_use.insert(key, t);
            return true;
        }
        if !exist_remains && remains {
            db.transitions_max_use.insert(key, existing);
            db.transitions.insert(key, t);
            return true;
        }
        // Same kind: keep first
        return false;
    }
    db.transitions.insert(key, t);
    true
}

/// Route last-use vs normal/max-use insert (text load + OLT1 load helpers).
pub fn insert_transition_record(db: &mut ClientContent, t: ClientTransition) -> bool {
    let key = (t.actor_id, t.target_id);
    if t.last_use_actor || t.last_use_target {
        if db.transitions_last_use.contains_key(&key) {
            return false;
        }
        db.transitions_last_use.insert(key, t);
        true
    } else {
        insert_normal_or_max_use(db, t)
    }
}

/// Haxe dough/masa-on-table `switchNumberOfUses = true` patches.
///
/// // Haxe: ServerSettings (keys match server `apply_default_switch_number_of_uses_patches`)
pub fn apply_default_switch_number_of_uses_patches(db: &mut ClientContent) {
    const KEYS: &[(i32, i32)] = &[(252, 3371), (235, 4086), (1300, 3371), (235, 4090)];
    for &key in KEYS {
        if let Some(t) = db.transitions.get_mut(&key) {
            t.switch_number_of_uses = true;
        }
        if let Some(t) = db.transitions_max_use.get_mut(&key) {
            t.switch_number_of_uses = true;
        }
        if let Some(t) = db.transitions_last_use.get_mut(&key) {
            t.switch_number_of_uses = true;
        }
    }
}

pub fn parse_transition_txt(
    actor: i32,
    target: i32,
    filename: &str,
    text: &str,
) -> Option<ClientTransition> {
    let mut tr = ClientTransition {
        actor_id: actor,
        target_id: target,
        ..Default::default()
    };
    // Filename flags: exact suffix after actor_target (Haxe / C++).
    // // Haxe: LA / LT / L last-use keying; // C++: transitionBank lastUseActor/Target
    let parts: Vec<&str> = filename.split('_').collect();
    if parts.len() >= 3 {
        let flag = parts[2];
        tr.last_use_actor = flag == "LA";
        tr.last_use_target = flag == "LT" || flag == "L";
    }
    // First line: newActor newTarget autoDecaySeconds actorMin targetMin
    //             reverseActor reverseTarget move desiredMove noUseActor noUseTarget
    if let Some(line) = text.lines().next() {
        let p: Vec<&str> = line.split_whitespace().collect();
        let parse_i = |i: usize, default: i32| -> i32 {
            p.get(i).and_then(|s| s.parse().ok()).unwrap_or(default)
        };
        let parse_f = |i: usize| -> f32 { p.get(i).and_then(|s| s.parse().ok()).unwrap_or(0.0) };
        let parse_b = |i: usize| -> bool { p.get(i).map(|s| *s == "1").unwrap_or(false) };
        if p.len() >= 2 {
            tr.new_actor_id = parse_i(0, 0);
            tr.new_target_id = parse_i(1, 0);
        }
        if p.len() >= 3 {
            tr.auto_decay_seconds = parse_f(2);
        }
        if p.len() >= 5 {
            tr.actor_min_use_fraction = parse_f(3);
            tr.target_min_use_fraction = parse_f(4);
        }
        if p.len() >= 7 {
            tr.reverse_use_actor = parse_b(5);
            tr.reverse_use_target = parse_b(6);
        }
        if p.len() >= 9 {
            tr.move_dist = parse_i(7, 0);
            tr.desired_move_dist = parse_i(8, 0);
        }
        if p.len() >= 11 {
            tr.no_use_actor = parse_b(9);
            tr.no_use_target = parse_b(10);
        }
    }
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == "autoDecaySeconds" {
                tr.auto_decay_seconds = v.trim().parse().unwrap_or(0.0);
            }
        }
    }
    Some(tr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_object_minimal() {
        let t = "id=33\nGooseberry\nfoodValue=3\npermanent=0\nblocksWalking=0\n";
        let d = parse_object_txt(33, t).unwrap();
        assert_eq!(d.food_value, 3);
        assert!(!d.blocks_walking);
        assert_eq!(d.name, "Gooseberry");
        // bare-id first line still works
        let t2 = "33\nStone\nfoodValue=0\n";
        let d2 = parse_object_txt(33, t2).unwrap();
        // first line is id number → used as name if not id= form
        assert!(!d2.name.is_empty());
    }

    #[test]
    fn parse_home_marker_object() {
        let t = "id=487\nHome Marker# eveHomeMarker\nhomeMarker=1\npermanent=1\nblocksWalking=0\n";
        let d = parse_object_txt(487, t).unwrap();
        assert!(d.home_marker);
        assert!(d.permanent);
        // Description-tag recovery when field omitted (OLC1 path).
        let d2 = parse_object_txt(
            487,
            "id=487\nHome Marker# eveHomeMarker\npermanent=1\nblocksWalking=0\n",
        )
        .unwrap();
        assert!(d2.home_marker, "eveHomeMarker tag sets home_marker");
    }

    #[test]
    fn permanent_does_not_imply_blocks_walking() {
        // C++ path map: permanent alone is walkable.
        let t = "id=50\nBush\npermanent=1\nblocksWalking=0\n";
        let d = parse_object_txt(50, t).unwrap();
        let mut c = ClientContent::default();
        c.objects.insert(50, d);
        assert!(!c.blocks_walking(50));
    }

    #[test]
    fn parse_side_access_and_no_back_access() {
        let ice = parse_object_txt(
            706,
            "id=706\nIce Hole\npermanent=1\nsideAccess=1\nblocksWalking=1\n",
        )
        .unwrap();
        assert!(ice.side_access);
        assert!(!ice.no_back_access);

        let shelf = parse_object_txt(
            3240,
            "id=3240\nWall Shelf# +causeAutoOrient +noBackAccess\npermanent=1\nsideAccess=0\nblocksWalking=1\n",
        )
        .unwrap();
        assert!(!shelf.side_access);
        assert!(shelf.no_back_access);
        assert!(shelf.description.contains("+noBackAccess") || shelf.name.contains("+noBackAccess"));
    }

    /// P3#23: C++ `setupWall` — floorHugging / +wall / -wall / +frontWall.
    #[test]
    fn setup_wall_layer_and_front_wall_tags() {
        let wall = parse_object_txt(
            100,
            "id=100\nStone Wall# +wall\npermanent=1\nblocksWalking=1\n",
        )
        .unwrap();
        assert!(wall.wall_layer);
        assert!(!wall.front_wall);

        let front = parse_object_txt(
            101,
            "id=101\nWall with Sign# +wall +frontWall\npermanent=1\nblocksWalking=1\n",
        )
        .unwrap();
        assert!(front.wall_layer);
        assert!(front.front_wall);

        let hug = parse_object_txt(
            102,
            "id=102\nFloor Hug Wall\npermanent=1\nfloorHugging=1\nblocksWalking=1\n",
        )
        .unwrap();
        assert!(hug.floor_hugging);
        assert!(hug.wall_layer);
        assert!(!hug.front_wall);

        let no_wall = parse_object_txt(
            103,
            "id=103\nNot A Wall# +wall -wall\npermanent=1\nfloorHugging=1\nblocksWalking=1\n",
        )
        .unwrap();
        assert!(!no_wall.wall_layer);
        assert!(!no_wall.front_wall);

        let bush = parse_object_txt(
            104,
            "id=104\nBush\npermanent=1\nblocksWalking=0\n",
        )
        .unwrap();
        assert!(!bush.wall_layer);
        assert!(!bush.front_wall);
    }

    // Haxe: ObjectData.useDistance / deadlyDistance / moves (object file fields)
    #[test]
    fn parse_use_deadly_distance_and_moves() {
        let bow = parse_object_txt(
            152,
            "id=152\nBow and Arrow\ndeadlyDistance=3\nuseDistance=5\n",
        )
        .unwrap();
        assert_eq!(bow.use_distance, 5);
        assert!((bow.deadly_distance - 3.0).abs() < 1e-5);
        assert_eq!(bow.moves, 0);

        let wolf = parse_object_txt(
            418,
            "id=418\nWolf\ndeadlyDistance=1\nuseDistance=1\nmoves=2\n",
        )
        .unwrap();
        assert_eq!(wolf.use_distance, 1);
        assert!((wolf.deadly_distance - 1.0).abs() < 1e-5);
        assert_eq!(wolf.moves, 2);
    }

    #[test]
    fn parse_wide_blocking_radii() {
        let t = "id=70\nTruck\nblocksWalking=1\nleftBlockingRadius=2\nrightBlockingRadius=3\n";
        let d = parse_object_txt(70, t).unwrap();
        assert_eq!(d.left_blocking_radius, 2);
        assert_eq!(d.right_blocking_radius, 3);
        // C++: wide → drawBehindPlayer forced true
        assert!(d.draw_behind_player);
    }

    #[test]
    fn parse_sim_fields_map_chance_heat_speed() {
        // Real OneLifeData7-style spawn + heat/speed lines
        let t = "\
id=30\nWild Gooseberry Bush\n\
blocksWalking=0,leftBlockingRadius=0,rightBlockingRadius=0,drawBehindPlayer=0\n\
mapChance=1.000000#biomes_0,3,4\n\
heatValue=2\n\
rValue=0.500000\n\
speedMult=0.750000\n\
decayFactor=0.1\n\
decaysToObj=618\n\
";
        let d = parse_object_txt(30, t).unwrap();
        assert!((d.map_chance - 1.0).abs() < 1e-5);
        assert_eq!(d.biomes, vec![0, 3, 4]);
        assert!((d.heat_value - 2.0).abs() < 1e-5);
        assert!((d.r_value - 0.5).abs() < 1e-5);
        assert!((d.speed_mult - 0.75).abs() < 1e-5);
        assert!((d.decay_factor - 0.1).abs() < 1e-5);
        assert_eq!(d.decays_to_obj, 618);
    }

    /// P3#19: Eyes/Mouth tags → mainEyesOffset + eyes_index for PE eyeEmot.
    #[test]
    fn setup_eyes_and_mouth_main_eyes_offset() {
        let mut def = ClientObjectDef {
            id: 19,
            person: 1,
            sprites: vec![
                ObjectSprite {
                    sprite_id: 1,
                    x: 0.0,
                    y: 0.0,
                    is_body: true,
                    ..Default::default()
                },
                ObjectSprite {
                    sprite_id: 10,
                    x: 2.0,
                    y: 22.0,
                    is_eyes: false, // set by setup
                    age_start: 10.0,
                    age_end: 40.0,
                    ..Default::default()
                },
                ObjectSprite {
                    sprite_id: 11,
                    x: 0.0,
                    y: 8.0,
                    is_mouth: false,
                    age_start: -1.0,
                    age_end: -1.0,
                    ..Default::default()
                },
                ObjectSprite {
                    sprite_id: 2,
                    x: 0.0,
                    y: 20.0,
                    is_head: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        def.setup_eyes_and_mouth(|sid| match sid {
            10 => Some("MaleEyes".into()),
            11 => Some("MouthOpen".into()),
            _ => Some("Body".into()),
        });
        assert!(def.sprites[1].is_eyes);
        assert!(def.sprites[2].is_mouth);
        assert!(!def.sprites[0].is_eyes);
        // mainEyesOffset = eyes(2,22) − head(0,20) = (2, 2)
        assert!((def.main_eyes_offset.0 - 2.0).abs() < 1e-5);
        assert!((def.main_eyes_offset.1 - 2.0).abs() < 1e-5);
        assert_eq!(def.eyes_index(20.0), Some(1));
        assert_eq!(def.mouth_index(20.0), Some(2));
        assert!(def.has_eyes_for_emot(20.0));
        // eyes not visible at age 5 → no eyes index (ageStart 10)
        assert_eq!(def.eyes_index(5.0), None);
        // offset still non-zero so emot path can use head+offset
        assert!(def.has_eyes_for_emot(5.0));

        let (ex, ey) = eyes_anchor_from_head(0.0, 20.0, 0.0, def.main_eyes_offset);
        assert!((ex - 2.0).abs() < 1e-5);
        assert!((ey - 22.0).abs() < 1e-5);
        // 0.25 turns = −90° (CW in standard math when angle is negative): (2,2) → (2, -2)?
        // a = -0.25 * 2π = -π/2; cos(-π/2)=0, sin=-1 → (ox*0 - oy*(-1), ox*(-1)+oy*0) = (oy, -ox) = (2, -2)
        let (rx, ry) = rotate_offset_turns(2.0, 2.0, 0.25);
        assert!((rx - 2.0).abs() < 1e-4, "got {rx}");
        assert!((ry - (-2.0)).abs() < 1e-4, "got {ry}");
    }

    #[test]
    fn parse_held_in_hand_and_body_indices() {
        let t = "\
id=19\nPerson\nperson=1\n\
heldInHand=0\n\
heldOffset=0,0\n\
spriteID=1\npos=0,0\nparent=-1\ninvisHolding=0,invisWorn=0,behindSlots=0\n\
spriteID=2\npos=-10,5\nparent=0\ninvisHolding=1,invisWorn=0,behindSlots=0\n\
spriteID=3\npos=10,5\nparent=0\ninvisHolding=1,invisWorn=0,behindSlots=0\n\
spriteID=4\npos=0,20\nparent=-1\ninvisHolding=0,invisWorn=0,behindSlots=0\n\
bodyIndex=0\n\
headIndex=3\n\
";
        let d = parse_object_txt(19, t).unwrap();
        assert!(!d.held_in_hand);
        assert!(!d.rideable);
        assert!(d.sprites[0].is_body);
        assert!(d.sprites[3].is_head);
        assert_eq!(d.body_index(20.0), 0);
        assert_eq!(d.head_index(20.0), 3);
        // hands = invisHolding layers; back = lower x
        assert_eq!(d.back_hand_index(20.0), Some(1));
        assert_eq!(d.front_hand_index(20.0), Some(2));
        let arms = d.back_arm_indices(20.0);
        assert!(arms.contains(&1));
        // stone-like held in hand
        let stone = parse_object_txt(33, "id=33\nStone\nheldInHand=1\nheldOffset=5,-5\n").unwrap();
        assert!(stone.held_in_hand);
        assert!(!stone.rideable);
        let (hide, all) = arm_holding_parameters(Some(&stone));
        assert_eq!(hide, 0);
        assert!(!all);
        let cart = parse_object_txt(99, "id=99\nCart\nheldInHand=2\n").unwrap();
        assert!(cart.rideable);
        assert!(!cart.held_in_hand);
        let (hide2, all2) = arm_holding_parameters(Some(&cart));
        assert_eq!(hide2, 0);
        assert!(all2);
        let bulky = parse_object_txt(50, "id=50\nBasket\nheldInHand=0\nheldOffset=0,10\n").unwrap();
        let (hide3, all3) = arm_holding_parameters(Some(&bulky));
        assert_eq!(hide3, -2);
        assert!(!all3);
        let mut hp = HoldingPos {
            valid: true,
            x: 10.0,
            y: 20.0,
            rot: 0.0,
        };
        let (hx, hy, _) = compute_held_draw_pos(&hp, Some(&stone), false);
        assert!((hx - 15.0).abs() < 1e-4);
        assert!((hy - 15.0).abs() < 1e-4);
        hp.valid = false;
        let (hx2, hy2, _) = compute_held_draw_pos(&hp, Some(&stone), false);
        assert!((hx2 - 5.0).abs() < 1e-4);
        assert!((hy2 - (-5.0)).abs() < 1e-4);
    }

    #[test]
    fn parse_draw_behind_combined_line_and_sprites() {
        // Real OneLifeData7: one blocksWalking line + sparse spritesDrawnBehind
        let t = "\
id=1012\nMarked Grave\n\
blocksWalking=1,leftBlockingRadius=0,rightBlockingRadius=0,drawBehindPlayer=1\n\
spriteID=1\npos=0,0\nparent=-1\n\
spriteID=2\npos=0,10\nparent=-1\n\
spriteID=3\npos=0,20\nparent=-1\n\
spritesDrawnBehind=0,2\n\
";
        let d = parse_object_txt(1012, t).unwrap();
        assert!(d.blocks_walking);
        assert!(d.draw_behind_player);
        assert_eq!(d.sprites.len(), 3);
        assert!(d.sprites[0].behind_player);
        assert!(!d.sprites[1].behind_player);
        assert!(d.sprites[2].behind_player);
        assert!(d.any_sprites_behind_player());
    }

    #[test]
    fn parse_object_parent_held_slots() {
        // C++ objectBank sprite parent= / heldOffset / numSlots / slotPos
        let t = "\
id=100\nBag\n\
heldOffset=17.0,-12.0\n\
clothing=h\n\
clothingOffset=1.0,2.0\n\
floor=0\n\
numSlots=2#timeStretch=1.000000\n\
slotPos=0.000000,10.000000,vert=0,parent=-1\n\
slotPos=5.000000,12.000000,vert=0,parent=-1\n\
spriteID=1\n\
pos=0.0,0.0\n\
rot=0.25\n\
parent=-1\n\
invisHolding=0,invisWorn=0,behindSlots=0\n\
spriteID=2\n\
pos=4.0,8.0\n\
ageRange=10.0,999.0\n\
parent=0\n\
invisHolding=1,invisWorn=0,behindSlots=1\n\
";
        let d = parse_object_txt(100, t).unwrap();
        assert_eq!(d.held_offset, (17.0, -12.0));
        assert_eq!(d.clothing, 'h');
        assert_eq!(d.clothing_offset, (1.0, 2.0));
        assert_eq!(d.num_slots, 2);
        assert_eq!(d.slot_pos.len(), 2);
        assert_eq!(d.sprites.len(), 2);
        assert_eq!(d.sprites[0].parent, -1);
        assert!((d.sprites[0].rot - 0.25).abs() < 1e-5);
        assert_eq!(d.sprites[1].parent, 0);
        assert!(d.sprites[1].invis_holding);
        assert!(d.sprites[1].behind_slots);
        assert!(!d.sprites[1].visible_at_age(5.0));
        assert!(d.sprites[1].visible_at_age(20.0));
    }

    #[test]
    fn parse_transition_line() {
        let t = "0 32\nautoDecaySeconds=0\n";
        let tr = parse_transition_txt(33, 0, "33_0", t).unwrap();
        assert_eq!(tr.new_actor_id, 0);
        assert_eq!(tr.new_target_id, 32);
    }

    /// P3#21: widest sprite + rotated center offsets (C++ getObjectCenterOffset).
    #[test]
    fn object_center_offset_widest_and_held_subtract() {
        let mut obj = ClientObjectDef {
            id: 50,
            person: 0,
            held_offset: (10.0, 20.0),
            sprites: vec![
                ObjectSprite {
                    sprite_id: 1,
                    x: 0.0,
                    y: 5.0,
                    rot: 0.0,
                    ..Default::default()
                },
                ObjectSprite {
                    sprite_id: 2,
                    x: 3.0,
                    y: -2.0, // lower Y wins ties — but also wider
                    rot: 0.0,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let info = |sid: i32| {
            Some(match sid {
                1 => SpriteCenterInfo {
                    visible_w: 10,
                    visible_h: 10,
                    center_x_offset: 0,
                    center_y_offset: 0,
                    multiplicative_blend: false,
                },
                2 => SpriteCenterInfo {
                    visible_w: 20,
                    visible_h: 8,
                    center_x_offset: 2,
                    center_y_offset: -1,
                    multiplicative_blend: false,
                },
                _ => return None,
            })
        };
        let (cx, cy) = get_object_center_offset(&obj, info);
        // widest is sprite 2 at (3,-2) + (2,-1)
        assert!((cx - 5.0).abs() < 1e-4, "cx={cx}");
        assert!((cy - (-3.0)).abs() < 1e-4, "cy={cy}");

        // Multiplicative skipped
        let (cx2, cy2) = get_object_center_offset(&obj, |sid| {
            Some(SpriteCenterInfo {
                visible_w: if sid == 2 { 99 } else { 10 },
                visible_h: 10,
                center_x_offset: 0,
                center_y_offset: 0,
                multiplicative_blend: sid == 2,
            })
        });
        assert!((cx2 - 0.0).abs() < 1e-4);
        assert!((cy2 - 5.0).abs() < 1e-4);

        // Held draw subtracts center for non-person
        let hold = HoldingPos {
            valid: true,
            x: 0.0,
            y: 0.0,
            rot: 0.0,
        };
        let (hx, hy, _) =
            compute_held_draw_pos_ex(&hold, Some(&obj), false, Some((cx, cy)));
        assert!((hx - (10.0 - 5.0)).abs() < 1e-4, "hx={hx}");
        assert!((hy - (20.0 - (-3.0))).abs() < 1e-4, "hy={hy}");

        // Person: no center subtract
        obj.person = 1;
        let (hx2, hy2, _) =
            compute_held_draw_pos_ex(&hold, Some(&obj), false, Some((cx, cy)));
        assert!((hx2 - 10.0).abs() < 1e-4);
        assert!((hy2 - 20.0).abs() < 1e-4);
    }

    /// P3#21: C++ rotate(center, +2π·rot) + containOffset + only-when-worn skip.
    #[test]
    fn object_center_offset_rot_contain_and_worn_only_skip() {
        // 0.25 turns CCW: (4, 0) → (0, 4)
        let mut obj = ClientObjectDef {
            id: 60,
            sprites: vec![ObjectSprite {
                sprite_id: 1,
                x: 1.0,
                y: 2.0,
                rot: 0.25,
                ..Default::default()
            }],
            contain_offset: (10, -5),
            ..Default::default()
        };
        let (cx, cy) = get_object_center_offset(&obj, |_| {
            Some(SpriteCenterInfo {
                visible_w: 8,
                visible_h: 20, // after 0.25 rot, width uses H → 20
                center_x_offset: 4,
                center_y_offset: 0,
                multiplicative_blend: false,
            })
        });
        assert!((cx - (1.0 + 0.0 + 10.0)).abs() < 1e-3, "cx={cx}");
        assert!((cy - (2.0 + 4.0 - 5.0)).abs() < 1e-3, "cy={cy}");

        // only_when_worn skipped → empty → pure containOffset
        obj.sprites[0].only_when_worn = true;
        let (cx2, cy2) = get_object_center_offset(&obj, |_| {
            Some(SpriteCenterInfo {
                visible_w: 99,
                visible_h: 99,
                center_x_offset: 0,
                center_y_offset: 0,
                multiplicative_blend: false,
            })
        });
        assert!((cx2 - 10.0).abs() < 1e-4);
        assert!((cy2 - (-5.0)).abs() < 1e-4);

        // Description tags
        let (tx, ty) = parse_contain_offset_tags(
            "Glass Bottle# +containOffsetBottomY_-20 +containOffsetY_-14",
            "",
        );
        assert_eq!(tx, 0);
        assert_eq!(ty, -14);
        let (tx2, ty2) =
            parse_contain_offset_tags("Medium Antenna# +containOffsetX_40", "unused");
        assert_eq!(tx2, 40);
        assert_eq!(ty2, 0);

        // Text parse: invisWorn=2 + containOffset
        let d = parse_object_txt(
            71,
            "id=71\nAntenna# +containOffsetX_40 +containOffsetY_3\n\
             spriteID=1\npos=0,0\nparent=-1\ninvisHolding=0,invisWorn=2,behindSlots=0\n",
        )
        .unwrap();
        assert_eq!(d.contain_offset, (40, 3));
        assert!(d.sprites[0].only_when_worn);
        assert!(!d.sprites[0].invis_worn);
    }

    #[test]
    fn arm_holding_parameters_hide_closest() {
        assert_eq!(arm_holding_parameters(None), (0, false));
        let hand = ClientObjectDef {
            held_in_hand: true,
            ..Default::default()
        };
        assert_eq!(arm_holding_parameters(Some(&hand)), (0, false));
        let ride = ClientObjectDef {
            rideable: true,
            ..Default::default()
        };
        assert_eq!(arm_holding_parameters(Some(&ride)), (0, true));
        let bulky = ClientObjectDef {
            held_in_hand: false,
            rideable: false,
            ..Default::default()
        };
        // -2: freeze arms, body HoldingPos (hideClosestArm body attach)
        assert_eq!(arm_holding_parameters(Some(&bulky)), (-2, false));
    }

    #[test]
    fn parse_num_uses_chance_and_full_transition() {
        let t = "id=10\nTool\nnumUses=5,0.25\n";
        let d = parse_object_txt(10, t).unwrap();
        assert_eq!(d.num_uses, 5);
        assert!((d.use_chance - 0.25).abs() < 1e-5);

        // Full first-line fields (server / Haxe TransitionData layout)
        let line = "11 21 0.0 1.0 0.5 1 0 2 3 0 1\n";
        let tr = parse_transition_txt(10, 20, "10_20_LA", line).unwrap();
        assert!(tr.last_use_actor);
        assert!(!tr.last_use_target);
        assert_eq!(tr.new_actor_id, 11);
        assert_eq!(tr.new_target_id, 21);
        assert!((tr.actor_min_use_fraction - 1.0).abs() < 1e-5);
        assert!((tr.target_min_use_fraction - 0.5).abs() < 1e-5);
        assert!(tr.reverse_use_actor);
        assert!(!tr.reverse_use_target);
        assert_eq!(tr.move_dist, 2);
        assert_eq!(tr.desired_move_dist, 3);
        assert!(!tr.no_use_actor);
        assert!(tr.no_use_target);
    }

    /// P4#26: C++ getVarObjectLabel / getVarObjectNumeral / `$N` parse.
    #[test]
    fn variable_dummy_label_helpers() {
        assert_eq!(var_object_label(1), "- A");
        assert_eq!(var_object_label(2), "- B");
        assert_eq!(var_object_label(26), "- Z");
        assert_eq!(var_object_numeral(1, 9), "- 1");
        assert_eq!(var_object_numeral(1, 30), "- 01");
        assert_eq!(var_object_numeral(30, 30), "- 30");
        let lock = "Lock and Key $10# removed";
        assert_eq!(
            parse_variable_dollar_count(lock),
            Some((lock.find('$').unwrap(), 10))
        );
        assert_eq!(parse_variable_dollar_count("No dollar here"), None);
        assert_eq!(parse_variable_dollar_count("Tiny $1"), None); // N < 2
        assert!(description_has_var_numeral("Car $30# +varNumeral"));
        // `$10` before `#` → not hidden
        let desc = "Lock and Key $10# removed";
        let (idx, _) = parse_variable_dollar_count(desc).unwrap();
        assert!(!variable_target_is_hidden(desc, idx));
        // `$10` after `#` → hidden
        let hidden = "Note # secret $10";
        let (idx, _) = parse_variable_dollar_count(hidden).unwrap();
        assert!(variable_target_is_hidden(hidden, idx));
    }

    /// P4#25: parse useVanishIndex/useAppearIndex sparse lists (C++ objectBank).
    #[test]
    fn parse_use_vanish_appear_indices() {
        // Gooseberry bush style: 7 sprites, vanish 1..6, no appear.
        let t = "id=30\nBush\n\
                 spriteID=10\npos=0,0\nparent=-1\n\
                 spriteID=11\npos=0,0\nparent=-1\n\
                 spriteID=12\npos=0,0\nparent=-1\n\
                 spriteID=13\npos=0,0\nparent=-1\n\
                 spriteID=14\npos=0,0\nparent=-1\n\
                 spriteID=15\npos=0,0\nparent=-1\n\
                 spriteID=16\npos=0,0\nparent=-1\n\
                 headIndex=-1\nbodyIndex=-1\nbackFootIndex=-1\nfrontFootIndex=-1\n\
                 numUses=6\nuseVanishIndex=1,2,3,4,5,6\nuseAppearIndex=-1\n";
        let d = parse_object_txt(30, t).unwrap();
        assert_eq!(d.num_uses, 6);
        assert_eq!(d.sprites.len(), 7);
        assert!(!d.sprites[0].use_vanish);
        assert!(!d.sprites[0].use_appear);
        for i in 1..7 {
            assert!(d.sprites[i].use_vanish, "sprite {i} should vanish");
            assert!(!d.sprites[i].use_appear);
        }
    }

    /// P4#25: C++ setupSpriteUseVis progressive vanish stages.
    #[test]
    fn setup_sprite_use_vis_vanish_stages() {
        // 1 base + 6 vanish sprites, numUses=6 (like Wild Gooseberry Bush).
        let mut sprites = Vec::new();
        for i in 0..7 {
            let mut s = ObjectSprite {
                sprite_id: 100 + i,
                ..Default::default()
            };
            if i >= 1 {
                s.use_vanish = true;
            }
            sprites.push(s);
        }
        let num_uses = 6;

        // Full: no vanish skipped.
        let full = setup_sprite_use_vis(&sprites, num_uses, num_uses);
        assert!(full.iter().all(|&s| !s));

        // Zero uses: all vanish skipped, base kept.
        let empty = setup_sprite_use_vis(&sprites, num_uses, 0);
        assert!(!empty[0]);
        for i in 1..7 {
            assert!(empty[i], "vanish sprite {i} hidden at 0 uses");
        }

        // Uses remaining 5 (first dummy): fewer vanish visible than full.
        let d5 = setup_sprite_use_vis(&sprites, num_uses, 5);
        let vis5: usize = (1..7).filter(|&i| !d5[i]).count();
        let vis_full: usize = (1..7).filter(|&i| !full[i]).count();
        assert!(vis5 < vis_full, "first dummy must show fewer berries than full");

        // Uses remaining 1 (last dummy): still ≥1 vanish visible (pad rule).
        let d1 = setup_sprite_use_vis(&sprites, num_uses, 1);
        let vis1: usize = (1..7).filter(|&i| !d1[i]).count();
        assert!(vis1 >= 1, "last dummy keeps at least one vanish sprite");
        assert!(vis1 < vis5, "last dummy shows fewer than first dummy");

        // Monotonic: more uses remaining → more or equal vanish sprites visible.
        let mut prev = 0usize;
        for uses in 1..=5 {
            let skip = setup_sprite_use_vis(&sprites, num_uses, uses);
            let vis = (1..7).filter(|&i| !skip[i]).count();
            assert!(
                vis >= prev,
                "uses={uses} vis={vis} should be ≥ prev={prev}"
            );
            prev = vis;
        }
    }

    /// P4#25: appear sprites hidden on full parent, unhidden as uses deplete.
    #[test]
    fn setup_sprite_use_vis_appear_stages() {
        let mut sprites = vec![
            ObjectSprite {
                sprite_id: 1,
                ..Default::default()
            },
            ObjectSprite {
                sprite_id: 2,
                use_appear: true,
                ..Default::default()
            },
            ObjectSprite {
                sprite_id: 3,
                use_appear: true,
                ..Default::default()
            },
        ];
        let _ = &mut sprites;
        let num_uses = 3;
        let full = setup_sprite_use_vis(&sprites, num_uses, 3);
        assert!(!full[0]);
        assert!(full[1] && full[2], "appear hidden when full");

        let mid = setup_sprite_use_vis(&sprites, num_uses, 1);
        // At least one appear should unhide as uses drop.
        let appear_vis = (!mid[1]) as i32 + (!mid[2]) as i32;
        assert!(appear_vis >= 1, "low uses unhides appear sprites");
    }

    #[test]
    fn load_real_content_if_present() {
        match ClientContent::load_default_locations() {
            Ok(db) => {
                assert!(db.objects.len() > 100, "objects={}", db.objects.len());
                // Gooseberry 33 often exists
                if let Some(g) = db.get(33) {
                    assert!(g.food_value > 0 || !g.name.is_empty());
                }
                // Category expand: 722 @ Shallow Digger → 34 Sharp Stone on 36
                if db.categories.get_category(722).is_some() {
                    assert!(
                        db.find_transition(34, 36).is_some(),
                        "sharp stone on seeding wild carrot via category 722"
                    );
                }
            }
            Err(_) => {
                // CI without data is fine
            }
        }
    }

    #[test]
    fn category_expand_fixture() {
        // Minimal in-memory bank: parent 722 + trans 722+36, member 34.
        let mut c = ClientContent::default();
        c.transitions.insert(
            (722, 36),
            ClientTransition {
                actor_id: 722,
                target_id: 36,
                new_actor_id: 722,
                new_target_id: 39,
                ..Default::default()
            },
        );
        let mut bank = CategoryBank::new();
        bank.insert_record(crate::category_bank::CategoryRecord {
            parent_id: 722,
            is_pattern: false,
            is_probability_set: false,
            object_ids: vec![34],
            object_weights: vec![0.0],
        });
        let added = c.apply_category_bank(bank);
        assert_eq!(added, 1);
        let t = c.find_transition(34, 36).unwrap();
        assert_eq!(t.new_target_id, 39);
        assert_eq!(t.new_actor_id, 34);
        assert_eq!(c.categories.get_category_for_object(34, 0), 722);
    }

    #[test]
    fn category_pattern_and_ptrans_fixture() {
        let mut c = ClientContent::default();
        c.transitions.insert(
            (394, 1802),
            ClientTransition {
                actor_id: 394,
                target_id: 1802,
                new_actor_id: 394,
                new_target_id: 1806,
                ..Default::default()
            },
        );
        // Prob-set decay outcome for materialize.
        c.transitions.insert(
            (-1, 1195),
            ClientTransition {
                actor_id: -1,
                target_id: 1195,
                new_actor_id: 0,
                new_target_id: 3221,
                ..Default::default()
            },
        );
        let mut bank = CategoryBank::new();
        bank.insert_record(crate::category_bank::CategoryRecord {
            parent_id: 394,
            is_pattern: false,
            is_probability_set: false,
            object_ids: vec![210],
            object_weights: vec![0.0],
        });
        bank.insert_record(crate::category_bank::CategoryRecord {
            parent_id: 1802,
            is_pattern: true,
            is_probability_set: false,
            object_ids: vec![1803, 1804],
            object_weights: vec![0.0, 0.0],
        });
        bank.insert_record(crate::category_bank::CategoryRecord {
            parent_id: 1806,
            is_pattern: true,
            is_probability_set: false,
            object_ids: vec![1809, 1808],
            object_weights: vec![0.0, 0.0],
        });
        bank.insert_record(crate::category_bank::CategoryRecord {
            parent_id: 3221,
            is_pattern: false,
            is_probability_set: true,
            object_ids: vec![1196, 3220],
            object_weights: vec![0.8, 0.2],
        });
        let added = c.apply_category_bank(bank);
        assert!(added >= 4, "lite+pattern added={added}");
        assert_eq!(
            c.find_transition(210, 1803).unwrap().new_target_id,
            1809
        );
        // Stored row still has abstract prob-set parent.
        assert_eq!(
            c.find_transition(-1, 1195).unwrap().new_target_id,
            3221
        );
        let m = c.find_ptrans(-1, 1195, 0.0, 0.0).unwrap();
        assert_eq!(m.new_target_id, 1196);
        let m = c.find_ptrans(-1, 1195, 0.0, 0.9).unwrap();
        assert_eq!(m.new_target_id, 3220);
    }

    /// P4#29: Haxe maxUse pair — targetRemains true stays normal; non-remains → max_use.
    #[test]
    fn max_use_pair_insert_remains_first() {
        let mut db = ClientContent::default();
        // 33 + 1096 = 0 + 1096  (targetRemains)
        let remains = ClientTransition {
            actor_id: 33,
            target_id: 1096,
            new_actor_id: 0,
            new_target_id: 1096,
            ..Default::default()
        };
        // 33 + 1096 = 0 + 3963  (complete)
        let complete = ClientTransition {
            actor_id: 33,
            target_id: 1096,
            new_actor_id: 0,
            new_target_id: 3963,
            ..Default::default()
        };
        assert!(insert_normal_or_max_use(&mut db, remains));
        assert!(insert_normal_or_max_use(&mut db, complete));
        assert_eq!(
            db.find_transition(33, 1096).unwrap().new_target_id,
            1096
        );
        assert_eq!(
            db.find_transition_max_use(33, 1096).unwrap().new_target_id,
            3963
        );
        let m = db.find_ptrans_max_use(33, 1096, 0.0, 0.0).unwrap();
        assert_eq!(m.new_target_id, 3963);
    }

    /// P4#29: reverse insert order — non-remains first then remains swaps into max_use.
    #[test]
    fn max_use_pair_insert_complete_first() {
        let mut db = ClientContent::default();
        let complete = ClientTransition {
            actor_id: 33,
            target_id: 1096,
            new_actor_id: 0,
            new_target_id: 3963,
            ..Default::default()
        };
        let remains = ClientTransition {
            actor_id: 33,
            target_id: 1096,
            new_actor_id: 0,
            new_target_id: 1096,
            ..Default::default()
        };
        assert!(insert_normal_or_max_use(&mut db, complete));
        assert!(insert_normal_or_max_use(&mut db, remains));
        assert_eq!(
            db.find_transition(33, 1096).unwrap().new_target_id,
            1096,
            "remains ends in primary table"
        );
        assert_eq!(
            db.find_transition_max_use(33, 1096).unwrap().new_target_id,
            3963
        );
    }

    /// P4#29: dough/masa switchNumberOfUses ServerSettings keys.
    #[test]
    fn switch_number_of_uses_patches() {
        let mut db = ClientContent::default();
        for &key in &[(252, 3371), (235, 4086), (1300, 3371), (235, 4090)] {
            db.transitions.insert(
                key,
                ClientTransition {
                    actor_id: key.0,
                    target_id: key.1,
                    new_actor_id: key.0,
                    new_target_id: key.1,
                    ..Default::default()
                },
            );
        }
        // Unrelated key must stay false.
        db.transitions.insert(
            (1, 2),
            ClientTransition {
                actor_id: 1,
                target_id: 2,
                ..Default::default()
            },
        );
        apply_default_switch_number_of_uses_patches(&mut db);
        assert!(db.transitions.get(&(252, 3371)).unwrap().switch_number_of_uses);
        assert!(db.transitions.get(&(235, 4086)).unwrap().switch_number_of_uses);
        assert!(db.transitions.get(&(1300, 3371)).unwrap().switch_number_of_uses);
        assert!(db.transitions.get(&(235, 4090)).unwrap().switch_number_of_uses);
        assert!(!db.transitions.get(&(1, 2)).unwrap().switch_number_of_uses);
        // Idempotent
        apply_default_switch_number_of_uses_patches(&mut db);
        assert!(db.transitions.get(&(252, 3371)).unwrap().switch_number_of_uses);
    }

    #[test]
    fn target_remains_helper() {
        let r = ClientTransition {
            target_id: 10,
            new_target_id: 10,
            ..Default::default()
        };
        assert!(target_remains(&r));
        let n = ClientTransition {
            target_id: 10,
            new_target_id: 11,
            ..Default::default()
        };
        assert!(!target_remains(&n));
        let ground = ClientTransition {
            target_id: -1,
            new_target_id: -1,
            ..Default::default()
        };
        assert!(!target_remains(&ground), "negative target not remains");
    }
}
