//! Lasting player entities (C++ `LiveObject` subset).
//!
//! Chunk **L-LIVEOBJ**: apply PU (and related tags) into a map keyed by `player_id`.
//! Chunk **L-ANIM-DRAW**: `anim` pack state (cur/last type, fade, frame counts).
//!
//! C++: `LivingLifePage.h` `LiveObject` + PU handler in `LivingLifePage.cpp`.
//! Haxe: `PlayerInstance` / nearby player table.

use std::collections::HashMap;

use crate::anim_bank::{
    is_extra_anim_type, AnimBank, ANIM_EXTRA, ANIM_EXTRA_B, ANIM_GROUND, ANIM_HELD,
};
use crate::anim_draw::{
    action_wiggle_offset_units, baby_wiggle_offset_x_units, held_by_drop_offset_from_raw,
    is_anim_fade_needed, select_player_anim_type, step_baby_wiggle, step_held_by_drop_offset,
    step_held_pos_handoff, step_pending_action_progress, AnimDrawState, ObjectAnimPack,
    PENDING_ACTION_START_PROGRESS,
};
use crate::emotion::{EmotionBank, DEFAULT_EMOT_DURATION_SEC};
use crate::parse::{
    DyingPlayer, Lineage, LocationSays, PlayerEmot, PlayerMoveStart, PlayerName, PlayerSays,
    PlayerUpdate, SaysMapPointer, SaysTargetLabel,
};

/// Full-opacity speech duration: C++ `3 + strlen / 5` seconds before fade.
#[inline]
pub fn speech_hold_sec(text: &str) -> f32 {
    3.0 + text.len() as f32 / 5.0
}

/// C++ fade step: `speechFade -= 0.05 * frameRateFactor` after ETA.
pub const SPEECH_FADE_STEP: f32 = 0.05;

/// C++ `maxCurseTagDisplayGap` — reinsert / nervous-tic interval (seconds).
pub const MAX_CURSE_TAG_DISPLAY_GAP: f32 = 15.0;

/// Default TTL for person/map pointer markers when no `map_age_seconds` / spoken hold.
///
/// // C++ `addTempHomeLocation`: person arrows expire in 60s (expert +120).
pub const SAYS_POINTER_DEFAULT_TTL_SEC: f32 = 60.0;

/// Extra TTL for `*expert` (C++ +120s on person temp home).
pub const SAYS_POINTER_EXPERT_EXTRA_SEC: f32 = 120.0;

/// Pure map-spot marker color (cyan pin).
pub const MAP_SPOT_MARKER_RGBA: [u8; 4] = [80, 220, 255, 255];

/// TTL for a PS map/label pointer marker (P3#17).
///
/// Prefer wire `map_age_seconds` when present and > 0 (goal: expire with
/// map_age / bubble TTL). Else use spoken bubble hold when non-empty, else
/// 60s (C++ person temp-home). Expert adds +120s (C++ `addTempHomeLocation`).
///
/// Note: C++ treats `map_age` as "years ago" speech metadata and does not
/// auto-expire pure map homes; soft-FB still needs a finite TTL.
#[inline]
pub fn says_pointer_ttl_sec(ps: &PlayerSays) -> f32 {
    if let Some(ref m) = ps.map {
        if let Some(age) = m.map_age_seconds {
            if age > 0 {
                return age as f32;
            }
        }
    }
    let mut ttl = if !ps.spoken.is_empty() {
        speech_hold_sec(&ps.spoken)
    } else {
        SAYS_POINTER_DEFAULT_TTL_SEC
    };
    if matches!(ps.target_label, Some(SaysTargetLabel::Expert)) {
        ttl += SAYS_POINTER_EXPERT_EXTRA_SEC;
    }
    ttl
}

/// C++ `getHomeDir` — compass index 0=N … 7=NW toward `(to_x,to_y)` from player.
///
/// Returns `None` when distance is zero (undefined angle). Dist `< 5` still
/// yields a direction (C++ `tooClose` only affects label, not index).
#[inline]
pub fn home_dir_index(from_x: f32, from_y: f32, to_x: f32, to_y: f32) -> Option<usize> {
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    let dist_sq = dx * dx + dy * dy;
    if dist_sq < 1e-12 {
        return None;
    }
    // C++: a = angle(vector) - π/2 with north = 0; angle = atan2(y, x).
    let mut a = dy.atan2(dx) - std::f32::consts::FRAC_PI_2;
    if a < -std::f32::consts::PI / 8.0 {
        a += 2.0 * std::f32::consts::PI;
    }
    let index = (8.0 * a / (2.0 * std::f32::consts::PI)).round() as i32;
    Some(((index % 8 + 8) % 8) as usize)
}

/// C++ `HomePos` — permanent stake / temp map-pointer / ancient homeland.
///
/// // C++ LivingLifePage.cpp ~616–916 `homePosStack`
#[derive(Debug, Clone, PartialEq)]
pub struct HomePos {
    pub x: i32,
    pub y: i32,
    pub ancient: bool,
    pub temporary: bool,
    pub temp_person: bool,
    pub person_id: i32,
    /// C++ `tempPersonKey` (`map` / `baby` / `lead` / `expt` / …); `None` = plain map.
    pub temp_person_key: Option<String>,
    /// Wall-clock seconds when temp expires; `0.0` = no auto-expire (held map).
    pub temporary_expire_eta: f64,
}

impl HomePos {
    pub fn permanent(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            ancient: false,
            temporary: false,
            temp_person: false,
            person_id: -1,
            temp_person_key: None,
            temporary_expire_eta: 0.0,
        }
    }

    pub fn temporary_map(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            ancient: false,
            temporary: true,
            temp_person: false,
            person_id: -1,
            temp_person_key: None,
            temporary_expire_eta: 0.0,
        }
    }

    pub fn temporary_person(
        x: i32,
        y: i32,
        person_id: i32,
        key: &str,
        expire_eta: f64,
    ) -> Self {
        Self {
            x,
            y,
            ancient: false,
            temporary: true,
            temp_person: true,
            person_id,
            temp_person_key: Some(key.to_string()),
            temporary_expire_eta: expire_eta,
        }
    }

    pub fn ancient(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            ancient: true,
            temporary: false,
            temp_person: false,
            person_id: -1,
            temp_person_key: None,
            temporary_expire_eta: 0.0,
        }
    }
}

/// Priority for temp home keys (lower = higher priority). C++ `getLocationKeyPriority`.
pub fn home_location_key_priority(key: Option<&str>) -> i32 {
    match key {
        // NULL / map / explicit user-request keys
        None | Some("map") | Some("expt") | Some("owner") | Some("mother") | Some("lead") => 1,
        Some("property") => 2,
        Some("supp") => 3,
        Some("baby") => 4,
        Some("visitor") => 5,
        Some(_) => 6,
    }
}

/// C++ `homePosStack` — most recent non-ancient home at end.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HomePosStack {
    entries: Vec<HomePos>,
}

impl HomePosStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn entries(&self) -> &[HomePos] {
        &self.entries
    }

    /// Expire top temp entry when `now > temporary_expire_eta` (C++ `processHomePosStack`).
    pub fn process_expire(&mut self, now_secs: f64) {
        if let Some(r) = self.entries.last() {
            if r.temporary
                && r.temporary_expire_eta != 0.0
                && now_secs > r.temporary_expire_eta
            {
                self.entries.pop();
            }
        }
    }

    /// Top non-ancient record (C++ `getHomePosRecord` after process).
    pub fn top_non_ancient(&self) -> Option<&HomePos> {
        self.entries.iter().rev().find(|p| !p.ancient)
    }

    /// Ancient at stack front only.
    pub fn ancient_pos(&self) -> Option<&HomePos> {
        self.entries.first().filter(|p| p.ancient)
    }

    pub fn remove_at(&mut self, x: i32, y: i32) {
        if let Some(i) = self.entries.iter().position(|p| p.x == x && p.y == y) {
            self.entries.remove(i);
        }
    }

    pub fn remove_all_temp(&mut self) {
        self.entries.retain(|p| !p.temporary);
    }

    /// C++ `addHomeLocation` — permanent stake; clears temps, re-pushes unique pos.
    pub fn add_home(&mut self, x: i32, y: i32) {
        self.remove_all_temp();
        self.remove_at(x, y);
        self.entries.push(HomePos::permanent(x, y));
    }

    /// C++ `addAncientHomeLocation` — single ancient at front.
    pub fn add_ancient(&mut self, x: i32, y: i32) {
        self.remove_at(x, y);
        self.entries.retain(|p| !p.ancient);
        self.entries.insert(0, HomePos::ancient(x, y));
    }

    /// C++ `doesNewTempLocationTrumpPrevious` + `addTempHomeLocation`.
    pub fn add_temp(
        &mut self,
        x: i32,
        y: i32,
        person: bool,
        person_id: i32,
        person_key: Option<&str>,
        now_secs: f64,
    ) {
        // C++: if a temp exists, only replace when new priority ≤ current
        // (`NULL` map key has priority 1, same as explicit map).
        if let Some(cur) = self.entries.iter().find(|p| p.temporary) {
            let current_key = cur.temp_person_key.as_deref();
            if home_location_key_priority(person_key)
                > home_location_key_priority(current_key)
            {
                return;
            }
        }
        self.remove_all_temp();
        let mut p = if person {
            let mut eta = now_secs + 60.0;
            if person_key == Some("expt") {
                eta += 120.0;
            }
            HomePos::temporary_person(x, y, person_id, person_key.unwrap_or("map"), eta)
        } else {
            HomePos::temporary_map(x, y)
        };
        if let Some(k) = person_key {
            p.temp_person_key = Some(k.to_string());
        }
        self.entries.push(p);
    }

    /// Follow a person-linked temp home when they PU-move.
    pub fn update_person_location(&mut self, person_id: i32, x: i32, y: i32) {
        for p in &mut self.entries {
            if p.temp_person && p.person_id == person_id {
                p.x = x;
                p.y = y;
            }
        }
    }

    /// Resolved arrow target for HUD (C++ `getHomeLocation` non-ancient).
    pub fn active_home(&self) -> Option<&HomePos> {
        self.top_non_ancient()
    }

    /// Compass index + optional pencil label for HUD strip.
    pub fn home_dir_and_label(
        &self,
        from_x: f32,
        from_y: f32,
    ) -> (Option<usize>, Option<String>) {
        let Some(p) = self.active_home() else {
            return (None, None);
        };
        let dir = home_dir_index(from_x, from_y, p.x as f32, p.y as f32);
        let label = if p.temporary {
            if p.temp_person {
                p.temp_person_key
                    .as_ref()
                    .map(|k| k.to_ascii_uppercase())
                    .or_else(|| Some("MAP".to_string()))
            } else {
                Some("MAP".to_string())
            }
        } else {
            None
        };
        (dir, label)
    }
}

/// Active soft-FB map/label pointer from PS `*map` / `*label` (P3#17).
///
/// // C++: temp `HomePos` + overhead label; we draw world markers instead of
/// // home-slip arrows only.
#[derive(Debug, Clone, PartialEq)]
pub struct SaysPointerMarker {
    pub speaker_id: i32,
    pub map: Option<SaysMapPointer>,
    pub target_label: Option<SaysTargetLabel>,
    pub target_player_id: Option<i32>,
    /// Seconds remaining at full opacity before fade.
    pub ttl_remaining: f32,
    /// 1.0 full → 0.0 cleared (same step as speech fade).
    pub fade: f32,
}

impl SaysPointerMarker {
    pub fn from_ps(ps: &PlayerSays) -> Option<Self> {
        if ps.map.is_none() && ps.target_label.is_none() && ps.target_player_id.is_none() {
            return None;
        }
        Some(Self {
            speaker_id: ps.player_id,
            map: ps.map.clone(),
            target_label: ps.target_label.clone(),
            target_player_id: ps.target_player_id,
            ttl_remaining: says_pointer_ttl_sec(ps),
            fade: 1.0,
        })
    }

    /// Tick hold then fade. Returns `false` when fully expired.
    pub fn tick(&mut self, wall_dt: f32, frame_rate_factor: f32) -> bool {
        if wall_dt <= 0.0 && frame_rate_factor <= 0.0 {
            return true;
        }
        if self.ttl_remaining > 0.0 {
            self.ttl_remaining -= wall_dt;
            if self.ttl_remaining > 0.0 {
                return true;
            }
            self.ttl_remaining = 0.0;
        }
        let frf = if frame_rate_factor > 0.0 {
            frame_rate_factor
        } else {
            (wall_dt * 60.0).clamp(0.0, 4.0)
        };
        self.fade -= SPEECH_FADE_STEP * frf;
        self.fade > 0.0
    }

    /// World tile for the map spot, if any.
    pub fn map_tile(&self) -> Option<(i32, i32)> {
        self.map.as_ref().map(|m| (m.x, m.y))
    }

    /// Short label text for overhead marker (None for pure map spots).
    pub fn label_text(&self) -> Option<String> {
        self.target_label
            .as_ref()
            .map(|l| l.short_name().to_string())
    }

    /// Soft-FB RGBA for this marker (label color or map-spot cyan).
    pub fn color_rgba(&self) -> [u8; 4] {
        if let Some(ref lab) = self.target_label {
            lab.marker_rgba()
        } else {
            MAP_SPOT_MARKER_RGBA
        }
    }
}

/// Format C++ curse-tag bubble `X {name} X`.
#[inline]
pub fn format_curse_tag(curse_name: &str) -> String {
    format!("X {curse_name} X")
}

/// Soft-FB / world-space location speech (C++ `LocationSpeech`).
///
/// // C++: LivingLifePage `locationSpeech` vector; replace same cell.
#[derive(Debug, Clone, PartialEq)]
pub struct LocationSpeech {
    pub x: i32,
    pub y: i32,
    pub text: String,
    /// 1.0 full → 0.0 cleared (C++ `fade`).
    pub fade: f32,
    /// Seconds remaining at full opacity before fade (C++ `fadeETATime − now`).
    pub ttl_remaining: f32,
}

impl LocationSpeech {
    pub fn new(x: i32, y: i32, text: String) -> Self {
        let ttl = speech_hold_sec(&text);
        Self {
            x,
            y,
            text,
            fade: 1.0,
            ttl_remaining: ttl,
        }
    }

    /// Tick hold then fade. Returns `false` when fully expired (caller removes).
    pub fn tick(&mut self, wall_dt: f32, frame_rate_factor: f32) -> bool {
        if wall_dt <= 0.0 && frame_rate_factor <= 0.0 {
            return true;
        }
        if self.ttl_remaining > 0.0 {
            self.ttl_remaining -= wall_dt;
            if self.ttl_remaining > 0.0 {
                return true;
            }
            // Overshoot into fade phase this frame.
            self.ttl_remaining = 0.0;
        }
        let frf = if frame_rate_factor > 0.0 {
            frame_rate_factor
        } else {
            (wall_dt * 60.0).clamp(0.0, 4.0)
        };
        self.fade -= SPEECH_FADE_STEP * frf;
        self.fade > 0.0
    }
}

/// Protocol clothing slot count (0=hat … 5=backpack).
pub const CLOTHING_SLOT_COUNT: usize = 6;

/// Protocol names for slots 0..5 (`server/protocol.txt` DROP/SELF/SREMV `c`).
pub const CLOTHING_SLOT_NAMES: [&str; CLOTHING_SLOT_COUNT] = [
    "hat",
    "tunic",
    "frontShoe",
    "backShoe",
    "bottom",
    "backpack",
];

/// Map object-bank `clothing=` char → primary slot index.
///
/// // C++ / Haxe `getClothingSlot`: h=0 t=1 s=2 (front shoe) b=4 p=5; `n` → none.
/// Back shoe (3) is chosen only via empty-slot resolve when front is full.
pub fn clothing_char_to_slot(c: char) -> Option<i32> {
    match c.to_ascii_lowercase() {
        'h' => Some(0),
        't' => Some(1),
        's' => Some(2),
        'b' => Some(4),
        'p' => Some(5),
        _ => None,
    }
}

/// Clothing slots from PU `clothing_set` (`hat;tunic;front_shoe;back_shoe;bottom;backpack`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClothingSet {
    pub raw: String,
    pub slots: [String; 6],
}

impl ClothingSet {
    pub fn parse(raw: &str) -> Self {
        let mut slots = [
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ];
        for (i, part) in raw.split(';').take(6).enumerate() {
            slots[i] = part.to_string();
        }
        Self {
            raw: raw.to_string(),
            slots,
        }
    }

    pub fn hat(&self) -> &str {
        &self.slots[0]
    }
    pub fn tunic(&self) -> &str {
        &self.slots[1]
    }
    pub fn backpack(&self) -> &str {
        &self.slots[5]
    }

    /// Leading object id for clothing slot `i` (0=hat … 5=backpack), or 0.
    ///
    /// Slot strings may include container nesting; draw uses the outer id.
    pub fn slot_id(&self, i: usize) -> i32 {
        self.slots
            .get(i)
            .and_then(|s| crate::parse::parse_leading_i32(s))
            .unwrap_or(0)
            .max(0)
    }

    /// True when slot `i` is empty (id 0 / missing).
    pub fn is_empty_slot(&self, i: usize) -> bool {
        self.slot_id(i) == 0
    }

    /// Prefer empty front shoe (2), else empty back shoe (3), else front (swap).
    pub fn resolve_shoe_slot(&self) -> i32 {
        if self.is_empty_slot(2) {
            2
        } else if self.is_empty_slot(3) {
            3
        } else {
            2
        }
    }

    /// All non-empty clothing object ids in draw order
    /// (back shoe, bottom, tunic, backpack, front shoe, hat — C++ drawObjectAnim clothing passes).
    pub fn draw_ids(&self) -> Vec<i32> {
        // C++ order approximation: backShoe, bottom, tunic, backpack, frontShoe, hat
        const ORDER: [usize; 6] = [3, 4, 1, 5, 2, 0];
        ORDER
            .iter()
            .filter_map(|&i| {
                let id = self.slot_id(i);
                if id > 0 {
                    Some(id)
                } else {
                    None
                }
            })
            .collect()
    }
}

/// One living (or recently deleted) player tracked by the client.
///
/// Field names map to C++ `LiveObject` / protocol PU where practical.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveObject {
    pub id: i32,
    pub display_id: i32,
    pub facing: i32,
    pub action: i32,
    pub action_target_x: i32,
    pub action_target_y: i32,
    pub held_id: i32,
    pub held_id_raw: String,
    pub held_origin_valid: bool,
    pub held_origin_x: i32,
    pub held_origin_y: i32,
    pub held_transition_source_id: i32,
    pub heat: f32,
    pub done_moving_seq: i32,
    pub force: bool,
    pub x: i32,
    pub y: i32,
    /// Fractional display position (C++ `LiveObject.currentPos`). Updated mid-path
    /// for local player from [`crate::move_state::MoveState`]; equals `(x,y)` when idle.
    pub display_x: f32,
    pub display_y: f32,
    /// Age in years at [`last_age_set`] (C++ `LiveObject.age` base).
    pub age: f32,
    /// Years per second (C++ `ageRate` = `1/invAgeRate` from PU).
    pub age_rate: f32,
    /// Wall clock when `age`/`age_rate` were last set from PU (C++ `lastAgeSetTime`).
    pub last_age_set: std::time::Instant,
    pub move_speed: f32,
    pub clothing: ClothingSet,
    pub just_ate: bool,
    pub last_ate_id: i32,
    pub responsible_id: i32,
    pub held_yum: bool,
    pub held_learned: bool,
    /// Present while server still tracks the life; cleared on delete PU.
    pub on_screen: bool,
    pub out_of_range: bool,
    pub deleted: bool,
    pub delete_reason: Option<String>,
    pub dying: bool,
    pub sick: bool,
    pub name: Option<String>,
    pub lineage: Option<Lineage>,
    /// Temporary PE emotion table index (`currentEmot`); cleared when TTL expires.
    pub last_emot_index: Option<i32>,
    /// Seconds remaining on temporary emot (`emotClearETATime − now`); `None` if none.
    pub emot_ttl_remaining: Option<f32>,
    /// Permanent PE layers (`ttl < 0`); stack with temporary (`permanentEmots`).
    pub permanent_emots: Vec<i32>,
    /// Resolved `Emotion.extraAnimIndex` for `ANIM_EXTRA` / `ANIM_EXTRA_B`
    /// (`None` if facial-only).
    ///
    /// // C++: PE sets extraIndex from `currentEmot->extraAnimIndex`, not the PE row index
    pub emot_extra_index: Option<i32>,
    /// C++ `LiveObject.extraAnimType` — toggles `extra`↔`extraB` on each PE gesture
    /// so dual-fade can transition smoothly. Starts as `ANIM_EXTRA_B` (first PE → EXTRA).
    pub emot_extra_anim_type: i32,
    /// True after a PM that targets this player until a PU settles position.
    pub moving: bool,
    /// Pending path start from last PM (xs, ys relative server coords on PM wire).
    pub last_move: Option<PlayerMoveStart>,
    /// L-ANIM-DRAW pack state (cur/last anim, fade, frame counts).
    pub anim: AnimDrawState,
    /// C++ `LiveObject.heldByAdultID` — adult id when we are a held baby, else **-1**.
    ///
    /// Not on the wire for the baby; derived when an adult PU has `held_id = -baby_id`.
    pub held_by_adult_id: i32,
    /// C++ `heldByAdultPendingID` — adult id from not-yet-played messages; else **-1**.
    pub held_by_adult_pending_id: i32,
    // ── P3#22 action wiggle + baby-held handoff (C++ LiveObject) ────────────
    /// C++ `pendingActionAnimationProgress` — cosine bounce phase while acting.
    pub pending_action_animation_progress: f32,
    /// C++ `pendingAction` — local action still awaiting server PU.
    pub pending_action: bool,
    /// C++ `actionTargetTweakX/Y` for moving targets.
    pub action_target_tweak_x: i32,
    pub action_target_tweak_y: i32,
    /// C++ `heldByDropOffset` (tile units) — slide after put-down.
    pub held_by_drop_offset_x: f32,
    pub held_by_drop_offset_y: f32,
    /// C++ `lastHeldByRawPos` (tile units) + set flag for drop handoff.
    pub last_held_by_raw_pos_set: bool,
    pub last_held_by_raw_pos_x: f32,
    pub last_held_by_raw_pos_y: f32,
    /// C++ `babyWiggle` / `babyWiggleProgress` — jump-out of arms bounce.
    pub baby_wiggle: bool,
    pub baby_wiggle_progress: f32,
    /// C++ `heldPosOverride` — slide held object from map origin into hand.
    pub held_pos_override: bool,
    pub held_pos_override_almost_over: bool,
    /// C++ `heldObjectPos` (tile units) / `heldObjectRot` during handoff.
    pub held_object_pos_x: f32,
    pub held_object_pos_y: f32,
    pub held_object_rot: f32,
    pub held_pos_slide_step_count: i32,
    /// C++ `currentSpeech` — active bubble text (cleared after fade).
    pub current_speech: Option<String>,
    /// C++ `speechFade` (1.0 full → 0 cleared).
    pub speech_fade: f32,
    /// Seconds remaining at full opacity before fade (C++ `speechFadeETATime − now`).
    pub speech_ttl_remaining: Option<f32>,
    /// Last spoken text kept after bubble clear (headless / probes).
    pub last_say: Option<String>,
    /// C++ `speechIsSuccessfulCurse` from PS `isCurse` flag.
    pub speech_is_curse: bool,
    /// C++ `curseLevel` from CU (CURSED) message.
    pub curse_level: i32,
    /// C++ `curseName` — display name with underscores → spaces (None if uncursed).
    pub curse_name: Option<String>,
    /// C++ `speechIsCurseTag` — current bubble is an auto "X name X" tag.
    pub speech_is_curse_tag: bool,
    /// Seconds since last curse-tag display (C++ `curTime - lastCurseTagDisplayTime`).
    pub curse_tag_idle_sec: f32,
}

/// Soft-FB speech ink RGB from speaker curse / dying state.
///
/// // C++ LivingLifePage `drawChalkBackgroundString` ~4083–4096
/// - dying && !sick → white
/// - curseLevel > 0 → white; if `speechIsSuccessfulCurse` → purple 0.875
/// - else if successful curse → purple 0.5
/// - else black
#[inline]
pub fn speech_text_rgb(o: &LiveObject) -> [u8; 3] {
    if o.dying && !o.sick {
        [255, 255, 255]
    } else if o.curse_level > 0 {
        if o.speech_is_curse {
            // C++ setDrawColor(0.875, 0, 0.875)
            [223, 0, 223]
        } else {
            [255, 255, 255]
        }
    } else if o.speech_is_curse {
        // C++ setDrawColor(0.5, 0, 0.5)
        [128, 0, 128]
    } else {
        [0, 0, 0]
    }
}

impl LiveObject {
    /// Build from a full live PU line (not delete form).
    pub fn from_pu(pu: &PlayerUpdate) -> Self {
        Self {
            id: pu.player_id,
            display_id: pu.display_id,
            facing: pu.facing,
            action: pu.action,
            action_target_x: pu.action_target_x,
            action_target_y: pu.action_target_y,
            held_id: pu.held_id,
            held_id_raw: pu.held_id_raw.clone(),
            held_origin_valid: pu.held_origin_valid,
            held_origin_x: pu.held_origin_x,
            held_origin_y: pu.held_origin_y,
            held_transition_source_id: pu.held_transition_source_id,
            heat: pu.heat,
            done_moving_seq: pu.done_moving_seq_num,
            force: pu.force,
            x: pu.x,
            y: pu.y,
            display_x: pu.x as f32,
            display_y: pu.y as f32,
            age: pu.age,
            age_rate: pu.age_rate,
            last_age_set: std::time::Instant::now(),
            move_speed: pu.move_speed,
            clothing: ClothingSet::parse(&pu.clothing_set),
            just_ate: pu.just_ate,
            last_ate_id: pu.last_ate_id,
            responsible_id: pu.responsible_id,
            held_yum: pu.held_yum,
            held_learned: pu.held_learned,
            on_screen: !pu.deleted,
            out_of_range: false,
            deleted: pu.deleted,
            delete_reason: pu.delete_reason.clone(),
            dying: false,
            sick: false,
            name: None,
            lineage: None,
            last_emot_index: None,
            emot_ttl_remaining: None,
            permanent_emots: Vec::new(),
            emot_extra_index: None,
            // C++ LiveObject init: extraAnimType = extraB (first PE toggles to extra)
            emot_extra_anim_type: ANIM_EXTRA_B,
            moving: false,
            last_move: None,
            anim: AnimDrawState::default(),
            // P3#22 residual fields (wiggle/handoff) — defaults match C++ fresh LiveObject
            held_by_adult_id: -1,
            held_by_adult_pending_id: -1,
            pending_action_animation_progress: 0.0,
            pending_action: false,
            action_target_tweak_x: 0,
            action_target_tweak_y: 0,
            held_by_drop_offset_x: 0.0,
            held_by_drop_offset_y: 0.0,
            last_held_by_raw_pos_set: false,
            last_held_by_raw_pos_x: 0.0,
            last_held_by_raw_pos_y: 0.0,
            baby_wiggle: false,
            baby_wiggle_progress: 0.0,
            held_pos_override: false,
            held_pos_override_almost_over: false,
            held_object_pos_x: 0.0,
            held_object_pos_y: 0.0,
            held_object_rot: 0.0,
            held_pos_slide_step_count: 0,
            current_speech: None,
            speech_fade: 1.0,
            speech_ttl_remaining: None,
            last_say: None,
            speech_is_curse: false,
            curse_level: 0,
            curse_name: None,
            speech_is_curse_tag: false,
            // Start "aged" so first 15s tic can fire after CU without waiting forever
            // from birth; actual display resets idle to 0.
            curse_tag_idle_sec: MAX_CURSE_TAG_DISPLAY_GAP + 1.0,
        }
    }

    /// Merge a PU into this object (preserve name/lineage/emot/speech/anim if not on PU).
    pub fn apply_pu(&mut self, pu: &PlayerUpdate) {
        if pu.deleted {
            self.deleted = true;
            self.on_screen = false;
            self.moving = false;
            self.last_move = None;
            self.held_by_adult_id = -1;
            self.held_by_adult_pending_id = -1;
            self.baby_wiggle = false;
            self.pending_action = false;
            self.pending_action_animation_progress = 0.0;
            self.delete_reason = pu.delete_reason.clone();
            // Keep last known pose/ids for probes.
            return;
        }
        let old_held = self.held_id;
        let name = self.name.take();
        let lineage = self.lineage.take();
        let emot = self.last_emot_index;
        let emot_ttl = self.emot_ttl_remaining;
        let permanent_emots = std::mem::take(&mut self.permanent_emots);
        let emot_extra = self.emot_extra_index;
        let emot_extra_type = self.emot_extra_anim_type;
        let anim = self.anim.clone();
        // Preserve mid-path motion unless this PU ends the path (Jason: done_moving / force).
        let was_moving = self.moving;
        let was_last_move = self.last_move.clone();
        let was_display = (self.display_x, self.display_y);
        // heldByAdultID is client-derived (not a PU field) — keep across baby PUs.
        let held_by = self.held_by_adult_id;
        let held_by_pending = self.held_by_adult_pending_id;
        // P3#22 handoff / wiggle state survives PU (not on wire).
        let pending_action_animation_progress = self.pending_action_animation_progress;
        let pending_action = self.pending_action;
        let action_target_tweak_x = self.action_target_tweak_x;
        let action_target_tweak_y = self.action_target_tweak_y;
        let held_by_drop_offset_x = self.held_by_drop_offset_x;
        let held_by_drop_offset_y = self.held_by_drop_offset_y;
        let last_held_by_raw_pos_set = self.last_held_by_raw_pos_set;
        let last_held_by_raw_pos_x = self.last_held_by_raw_pos_x;
        let last_held_by_raw_pos_y = self.last_held_by_raw_pos_y;
        let baby_wiggle = self.baby_wiggle;
        let baby_wiggle_progress = self.baby_wiggle_progress;
        let held_pos_override = self.held_pos_override;
        let held_pos_override_almost_over = self.held_pos_override_almost_over;
        let held_object_pos_x = self.held_object_pos_x;
        let held_object_pos_y = self.held_object_pos_y;
        let held_object_rot = self.held_object_rot;
        let held_pos_slide_step_count = self.held_pos_slide_step_count;
        // L-SAY: speech + curse state survive PU (not on wire).
        let current_speech = self.current_speech.take();
        let speech_fade = self.speech_fade;
        let speech_ttl = self.speech_ttl_remaining;
        let last_say = self.last_say.take();
        let speech_is_curse = self.speech_is_curse;
        let curse_level = self.curse_level;
        let curse_name = self.curse_name.take();
        let speech_is_curse_tag = self.speech_is_curse_tag;
        let curse_tag_idle_sec = self.curse_tag_idle_sec;
        *self = Self::from_pu(pu);
        self.name = name;
        self.lineage = lineage;
        self.last_emot_index = emot;
        self.emot_ttl_remaining = emot_ttl;
        self.permanent_emots = permanent_emots;
        self.emot_extra_index = emot_extra;
        self.emot_extra_anim_type = emot_extra_type;
        self.anim = anim;
        self.held_by_adult_id = held_by;
        self.held_by_adult_pending_id = held_by_pending;
        self.pending_action_animation_progress = pending_action_animation_progress;
        self.pending_action = pending_action;
        self.action_target_tweak_x = action_target_tweak_x;
        self.action_target_tweak_y = action_target_tweak_y;
        self.held_by_drop_offset_x = held_by_drop_offset_x;
        self.held_by_drop_offset_y = held_by_drop_offset_y;
        self.last_held_by_raw_pos_set = last_held_by_raw_pos_set;
        self.last_held_by_raw_pos_x = last_held_by_raw_pos_x;
        self.last_held_by_raw_pos_y = last_held_by_raw_pos_y;
        self.baby_wiggle = baby_wiggle;
        self.baby_wiggle_progress = baby_wiggle_progress;
        self.held_pos_override = held_pos_override;
        self.held_pos_override_almost_over = held_pos_override_almost_over;
        self.held_object_pos_x = held_object_pos_x;
        self.held_object_pos_y = held_object_pos_y;
        self.held_object_rot = held_object_rot;
        self.held_pos_slide_step_count = held_pos_slide_step_count;
        self.current_speech = current_speech;
        self.speech_fade = speech_fade;
        self.speech_ttl_remaining = speech_ttl;
        self.last_say = last_say;
        self.speech_is_curse = speech_is_curse;
        self.curse_level = curse_level;
        self.curse_name = curse_name;
        self.speech_is_curse_tag = speech_is_curse_tag;
        self.curse_tag_idle_sec = curse_tag_idle_sec;
        self.out_of_range = false;
        // C++ / Jason: only clear on-path when done_moving or force; intermediate PUs
        // (held, clothing, justAte) must not snap anim back to ground mid-walk.
        if pu.done_moving_seq_num > 0 || pu.force {
            self.moving = false;
            self.last_move = None;
            self.display_x = pu.x as f32;
            self.display_y = pu.y as f32;
        } else if was_moving {
            self.moving = true;
            self.last_move = was_last_move;
            // Keep fractional display until path end (local step_move_pos refreshes it).
            self.display_x = was_display.0;
            self.display_y = was_display.1;
        } else {
            self.moving = false;
            self.last_move = None;
            self.display_x = pu.x as f32;
            self.display_y = pu.y as f32;
        }

        // P3#22: remote actionAttempt starts a short bounce (C++ ~18436–18441).
        if pu.action != 0 && !pu.just_ate && self.pending_action_animation_progress == 0.0 {
            self.pending_action_animation_progress = PENDING_ACTION_START_PROGRESS;
        }

        // P3#22: pick up object from map → heldPosOverride slide (C++ ~18704–18711).
        if old_held == 0 && pu.held_id > 0 && pu.held_origin_valid {
            self.held_pos_override = true;
            self.held_pos_override_almost_over = false;
            self.held_pos_slide_step_count = 0;
            self.held_object_pos_x = pu.held_origin_x as f32;
            self.held_object_pos_y = pu.held_origin_y as f32;
            self.held_object_rot = 0.0;
        } else if pu.held_id == 0 {
            self.held_pos_override = false;
            self.held_pos_override_almost_over = false;
            self.held_pos_slide_step_count = 0;
        }
    }

    /// Show (or re-show) the curse-tag bubble `X {name} X` (C++ ~22412 / ~22429).
    pub fn show_curse_tag(&mut self) {
        let Some(ref name) = self.curse_name else {
            return;
        };
        if name.is_empty() {
            return;
        }
        let text = format_curse_tag(name);
        self.current_speech = Some(text.clone());
        self.speech_fade = 1.0;
        self.speech_ttl_remaining = Some(speech_hold_sec(&text));
        self.speech_is_curse_tag = true;
        self.speech_is_curse = false;
        self.curse_tag_idle_sec = 0.0;
    }

    /// Apply CU line fields onto this object (C++ CURSED ~21451–21508).
    ///
    /// `name` underscores become spaces. Level ≤ 0 clears the tag name.
    pub fn apply_cursed(&mut self, level: i32, name: Option<&str>) {
        self.curse_level = level;
        if level > 0 {
            if let Some(n) = name {
                let display = n.replace('_', " ");
                if !display.is_empty() {
                    self.curse_name = Some(display);
                    // C++ displays tag immediately on CU.
                    self.show_curse_tag();
                    return;
                }
            }
            // Level > 0 but no name: keep prior name if any.
        } else {
            self.curse_name = None;
            if self.speech_is_curse_tag {
                self.current_speech = None;
                self.speech_is_curse_tag = false;
                self.speech_fade = 1.0;
                self.speech_ttl_remaining = None;
            }
        }
    }

    /// True when this player is currently held by an adult (C++ `heldByAdultID != -1`).
    #[inline]
    pub fn is_held_by_adult(&self) -> bool {
        self.held_by_adult_id != -1
    }

    /// Start local pending-action bounce (C++ flush nextAction ~23220).
    pub fn start_pending_action_anim(&mut self) {
        self.pending_action = true;
        if self.pending_action_animation_progress == 0.0 {
            self.pending_action_animation_progress = PENDING_ACTION_START_PROGRESS;
        }
    }

    /// Clear local pending flag after server PU (progress finishes current cycle).
    pub fn clear_pending_action_flag(&mut self) {
        self.pending_action = false;
    }

    /// Start baby jump-out wiggle (C++ JUMP held / BW held).
    pub fn start_baby_wiggle(&mut self) {
        self.baby_wiggle = true;
        self.baby_wiggle_progress = 0.0;
    }

    /// Baby put-down: arm drop-offset from last held raw pos (C++ ~19269–19288).
    ///
    /// `ground_x/y` are tile coords after drop. Clears `lastHeldByRawPosSet`.
    /// Cross-fades person pack held → ground (C++ ~19296–19304 handoff anim).
    pub fn begin_drop_from_arms(&mut self, ground_x: f32, ground_y: f32) {
        if self.last_held_by_raw_pos_set {
            let (ox, oy) = held_by_drop_offset_from_raw(
                self.last_held_by_raw_pos_x,
                self.last_held_by_raw_pos_y,
                ground_x,
                ground_y,
            );
            self.held_by_drop_offset_x = ox;
            self.held_by_drop_offset_y = oy;
            self.last_held_by_raw_pos_set = false;
        }
        // When raw pos unknown, leave any mid-slide drop offset intact.
        self.held_by_adult_id = -1;
        self.baby_wiggle = false;
        self.baby_wiggle_progress = 0.0;
        // Handoff anim: fade from held (as carried) into ground pose.
        // // C++: lastAnim = adult.curHeldAnim; curAnim = ground; lastAnimFade = 1
        self.anim.last_anim = ANIM_HELD;
        self.anim.cur_anim = ANIM_GROUND;
        self.anim.last_anim_fade = 1.0;
    }

    /// Same as [`Self::begin_drop_from_arms`] but copies adult held-track clocks.
    pub fn begin_drop_from_arms_with_adult(
        &mut self,
        ground_x: f32,
        ground_y: f32,
        adult_held_anim: i32,
        adult_held_frame: f32,
    ) {
        self.begin_drop_from_arms(ground_x, ground_y);
        if adult_held_anim != 0 {
            self.anim.last_anim = adult_held_anim;
        }
        self.anim.animation_frame_count = adult_held_frame;
        self.anim.last_animation_frame_count = adult_held_frame;
    }

    /// Record adult hold world pos for later drop handoff (C++ ~5824).
    pub fn note_held_by_raw_pos(&mut self, tile_x: f32, tile_y: f32) {
        self.last_held_by_raw_pos_set = true;
        self.last_held_by_raw_pos_x = tile_x;
        self.last_held_by_raw_pos_y = tile_y;
    }

    /// Action-wiggle offset in object units (C++ drawLiveObject actionOffset).
    pub fn action_wiggle_units(&self) -> (f32, f32) {
        let eating = self.just_ate
            || self.anim.cur_anim == crate::anim_bank::ANIM_EATING
            || self.anim.last_anim == crate::anim_bank::ANIM_EATING;
        let tx = (self.action_target_x + self.action_target_tweak_x) as f32;
        let ty = (self.action_target_y + self.action_target_tweak_y) as f32;
        action_wiggle_offset_units(
            self.pending_action_animation_progress,
            self.x as f32,
            self.y as f32,
            tx,
            ty,
            eating,
        )
    }

    /// Baby held lateral wiggle in object units (0 when not wiggling).
    pub fn baby_wiggle_x_units(&self, holding_flip: bool) -> f32 {
        if !self.baby_wiggle {
            return 0.0;
        }
        baby_wiggle_offset_x_units(self.baby_wiggle_progress, holding_flip)
    }

    /// Draw position in tile units including drop offset (C++ pos + heldByDropOffset).
    ///
    /// Uses fractional [`Self::display_x`]/[`Self::display_y`] (mid-path currentPos).
    pub fn draw_pos_tiles(&self) -> (f32, f32) {
        (
            self.display_x + self.held_by_drop_offset_x,
            self.display_y + self.held_by_drop_offset_y,
        )
    }

    /// Snap grid + display to a world tile (FORCE / birth / done_moving).
    pub fn set_grid_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
        self.display_x = x as f32;
        self.display_y = y as f32;
    }

    /// Set fractional display only (path interpolation; grid xd/yd stay destination).
    pub fn set_display_pos(&mut self, x: f32, y: f32) {
        self.display_x = x;
        self.display_y = y;
    }

    /// Step P3#22 bounce / drop / baby-wiggle counters for one frame.
    ///
    /// `is_ours` controls wrap vs snap for pending-action progress.
    pub fn step_wiggle_handoff(&mut self, is_ours: bool, frame_rate_factor: f32) {
        self.pending_action_animation_progress = step_pending_action_progress(
            self.pending_action_animation_progress,
            self.pending_action,
            is_ours,
            frame_rate_factor,
        );
        if self.pending_action_animation_progress == 0.0 && !self.pending_action {
            self.action_target_tweak_x = 0;
            self.action_target_tweak_y = 0;
        }
        let (ox, oy, _landed) = step_held_by_drop_offset(
            self.held_by_drop_offset_x,
            self.held_by_drop_offset_y,
            frame_rate_factor,
        );
        self.held_by_drop_offset_x = ox;
        self.held_by_drop_offset_y = oy;
        let (active, prog) =
            step_baby_wiggle(self.baby_wiggle, self.baby_wiggle_progress, frame_rate_factor);
        self.baby_wiggle = active;
        self.baby_wiggle_progress = prog;
    }

    /// Begin held-object handoff slide toward `target` hand pos (tile units).
    pub fn begin_held_pos_handoff(&mut self, origin_x: f32, origin_y: f32) {
        self.held_pos_override = true;
        self.held_pos_override_almost_over = false;
        self.held_pos_slide_step_count = 0;
        self.held_object_pos_x = origin_x;
        self.held_object_pos_y = origin_y;
        self.held_object_rot = 0.0;
    }

    /// Step held-pos override toward current hand target (tile units).
    ///
    /// Returns the draw position in tile units (may be mid-slide).
    pub fn step_held_pos_toward(
        &mut self,
        target_x: f32,
        target_y: f32,
        target_rot: f32,
        stationary: bool,
        frame_rate_factor: f32,
    ) -> (f32, f32, f32) {
        let (x, y, rot, almost, active, steps) = step_held_pos_handoff(
            self.held_object_pos_x,
            self.held_object_pos_y,
            self.held_object_rot,
            target_x,
            target_y,
            target_rot,
            self.held_pos_slide_step_count,
            frame_rate_factor,
            stationary,
            self.held_pos_override,
            self.held_pos_override_almost_over,
        );
        self.held_object_pos_x = x;
        self.held_object_pos_y = y;
        self.held_object_rot = rot;
        self.held_pos_override_almost_over = almost;
        self.held_pos_override = active;
        self.held_pos_slide_step_count = steps;
        (x, y, rot)
    }

    /// Select + switch packs from current flags (L-ANIM-DRAW pack select).
    ///
    /// Call after PU/PM apply or each frame before draw. Uses bank for forceZero
    /// / randomStart reseed metadata. Snaps fade when `isAnimFadeNeeded` is false.
    pub fn sync_anim_packs(&mut self, bank: &mut AnimBank) {
        let holding_baby = self.held_id < 0;
        // Rideable detection deferred (needs object `rideable` flag); pass false for now.
        let rideable = false;
        // Emote → EXTRA/EXTRA_B when emotion table resolved a gesture slot.
        // // C++: PE + age>=1 + currentEmot->extraAnimIndex > -1 → addNewAnim(extra/extraB)
        let emot_extra = self.resolved_emot_extra_pack();
        let display = if self.display_id > 0 {
            self.display_id
        } else {
            19
        };
        // P3#22: local pending action bounce selects DOING like PU actionAttempt.
        let action_flag = if self.action != 0 || self.pending_action_animation_progress != 0.0 {
            1
        } else {
            0
        };
        self.anim.sync_from_player_state(
            bank,
            display,
            self.held_id,
            self.moving,
            self.just_ate,
            action_flag,
            emot_extra,
            holding_baby,
            rideable,
        );
        // // C++: first animation step zeros lastAnimFade when poses already match
        self.anim
            .maybe_skip_fades(bank, display, self.held_id.max(0));
    }

    /// Step anim counters + fade; check `isAnimFadeNeeded` for skip.
    pub fn step_anim(&mut self, bank: &mut AnimBank, anim_speed: f32, frame_rate_factor: f32) {
        let display = if self.display_id > 0 {
            self.display_id
        } else {
            19
        };
        let person_needed = if (self.anim.last_anim_fade - 1.0).abs() < 1e-6 {
            is_anim_fade_needed(
                bank,
                display,
                self.anim.last_anim,
                self.anim.cur_anim,
            )
        } else {
            true
        };
        let held_needed = if self.held_id > 0 && (self.anim.last_held_anim_fade - 1.0).abs() < 1e-6
        {
            is_anim_fade_needed(
                bank,
                self.held_id,
                self.anim.last_held_anim,
                self.anim.cur_held_anim,
            )
        } else {
            true
        };
        self.anim
            .step(anim_speed, frame_rate_factor, person_needed, held_needed);
    }

    /// Person draw pack for soft-FB / GPU.
    pub fn person_anim_pack(&self, rideable_or_hide_arm: bool) -> ObjectAnimPack {
        let id = if self.display_id > 0 {
            self.display_id
        } else {
            19
        };
        self.anim.person_pack(id, rideable_or_hide_arm)
    }

    /// Held-item draw pack (object id must be > 0).
    pub fn held_anim_pack(&self) -> Option<ObjectAnimPack> {
        if self.held_id > 0 {
            Some(self.anim.held_pack(self.held_id))
        } else {
            None
        }
    }

    /// Desired type from flags without mutating state (tests / HUD / SceneRenderer).
    pub fn desired_anim_type(&self) -> i32 {
        let action_flag = if self.action != 0 || self.pending_action_animation_progress != 0.0 {
            1
        } else {
            0
        };
        select_player_anim_type(
            self.moving,
            self.just_ate,
            action_flag,
            self.resolved_emot_extra_pack(),
        )
    }

    /// C++ `computeCurrentAgeNoOverride`: base age + ageRate × elapsed since PU.
    ///
    /// // LivingLifePage.cpp ~1498–1505
    #[inline]
    pub fn current_age(&self) -> f32 {
        let elapsed = self.last_age_set.elapsed().as_secs_f32();
        self.age + self.age_rate * elapsed
    }

    /// Extra anim index for pack select (`None` if baby age or no gesture).
    ///
    /// // C++: block extra anim for age < 1 so crying is not overridden
    pub fn resolved_emot_extra(&self) -> Option<i32> {
        if self.current_age() < 1.0 {
            return None;
        }
        self.emot_extra_index
    }

    /// `(ANIM_EXTRA|ANIM_EXTRA_B, index)` for pack select, or `None`.
    ///
    /// // C++: extraAnimType toggles on PE; draw uses setExtraIndex / setExtraIndexB
    pub fn resolved_emot_extra_pack(&self) -> Option<(i32, i32)> {
        let idx = self.resolved_emot_extra()?;
        let t = if is_extra_anim_type(self.emot_extra_anim_type) {
            self.emot_extra_anim_type
        } else {
            ANIM_EXTRA
        };
        Some((t, idx))
    }

    /// Temporary + permanent PE indices for emotion object-layer draw.
    ///
    /// Order matches C++ `drawWithEmots`: current first, then permanent stack.
    pub fn emot_draw_indices(&self) -> Vec<i32> {
        let mut v = Vec::with_capacity(1 + self.permanent_emots.len());
        if let Some(i) = self.last_emot_index {
            v.push(i);
        }
        for &p in &self.permanent_emots {
            if !v.contains(&p) {
                v.push(p);
            }
        }
        v
    }

    /// Tick temporary emote TTL (wall seconds). Clears when expired.
    ///
    /// Returns the cleared temporary emot table index when decay sounds should
    /// play (C++ ~22469–22494).
    pub fn tick_emot(&mut self, wall_dt: f32) -> Option<i32> {
        if wall_dt <= 0.0 {
            return None;
        }
        if let Some(rem) = self.emot_ttl_remaining.as_mut() {
            *rem -= wall_dt;
            if *rem <= 0.0 {
                let cleared = self.last_emot_index;
                self.last_emot_index = None;
                self.emot_ttl_remaining = None;
                self.emot_extra_index = None;
                // Keep emot_extra_anim_type so the next PE toggles the other slot.
                return cleared;
            }
        }
        None
    }

    /// Apply one PE to this object (C++ PLAYER_EMOT handler subset).
    ///
    /// Returns `Some(emot_index)` when creation sounds should play for the
    /// emotion's object slots (C++ `newEmotPlaySound`; skips ttl == −2).
    pub fn apply_emot(
        &mut self,
        e: &PlayerEmot,
        bank: Option<&EmotionBank>,
        default_duration_sec: f32,
    ) -> Option<i32> {
        let ttl = e.ttl_sec;
        // Permanent layer: ttl < 0 (−1 new, −2 silent/old).
        if let Some(t) = ttl {
            if t < 0.0 {
                let mut play = true;
                if !self.permanent_emots.contains(&e.emot_index) {
                    self.permanent_emots.push(e.emot_index);
                }
                // C++: ttl == -2 → permanent but not new → skip sound
                if (t + 2.0).abs() < 1e-6 {
                    play = false;
                }
                return if play { Some(e.emot_index) } else { None };
            }
        }
        // Temporary currentEmot — play creation when index changes (C++ oldEmot != new)
        let old_emot = self.last_emot_index;
        self.last_emot_index = Some(e.emot_index);
        let dur = match ttl {
            Some(t) if t > 0.0 => t,
            _ => default_duration_sec,
        };
        self.emot_ttl_remaining = Some(dur);
        // Resolve extra anim from emotion table (not the PE row index).
        // // C++: toggle extra ↔ extraB so gesture packs can cross-fade
        let ex = bank.and_then(|b| b.extra_anim_for(e.emot_index));
        self.emot_extra_index = ex;
        if let Some(ex_idx) = ex {
            if self.current_age() >= 1.0 {
                if self.emot_extra_anim_type == ANIM_EXTRA_B {
                    // First / odd PE after init → EXTRA (slot A)
                    self.emot_extra_anim_type = ANIM_EXTRA;
                    self.anim.extra_index = ex_idx;
                } else {
                    // Even PE → EXTRA_B (slot B)
                    self.emot_extra_anim_type = ANIM_EXTRA_B;
                    self.anim.extra_index_b = ex_idx;
                }
            }
        }
        if old_emot != Some(e.emot_index) {
            Some(e.emot_index)
        } else {
            None
        }
    }

    /// Apply one PS line (C++ PLAYER_SAYS → `currentSpeech` + ETA).
    ///
    /// Bubble text is always stripped `spoken` (P3#17: pointer tokens never
    /// appear in the chalk bubble). Empty spoken → no bubble (pointer-only PS
    /// still updates [`LiveWorld::says_pointers`] via [`LiveWorld::apply_says`]).
    ///
    /// // C++ ~20658–20683: babbling wrap when cursed and gap > 15s
    /// // C++ ~20712–20735: strip ` *map` / ` *label` before display
    pub fn apply_says(&mut self, ps: &PlayerSays) {
        // P3#17: keep spoken bubble as stripped `spoken` only.
        let mut text = ps.spoken.clone();
        if text.is_empty() {
            return;
        }
        // C++ skips displaying +FAMILY+ when already talking; still no bubble spam.
        if text == "+FAMILY+" && self.current_speech.is_some() {
            return;
        }
        self.speech_is_curse = ps.is_curse;
        // C++: force curse name into babble when gap > 15s (skip famSpeech / curses).
        let fam_speech = text == "+FAMILY+";
        let mut is_tag = false;
        if !ps.is_curse
            && !fam_speech
            && self.curse_tag_idle_sec > MAX_CURSE_TAG_DISPLAY_GAP
            && self.curse_name.is_some()
        {
            let name = self.curse_name.as_ref().unwrap();
            text = format!("{} - {text}", format_curse_tag(name));
            is_tag = true;
            self.curse_tag_idle_sec = 0.0;
        }
        // last_say keeps the unprefixed spoken line for headless probes.
        let last = if is_tag {
            ps.spoken.clone()
        } else {
            text.clone()
        };
        self.current_speech = Some(text.clone());
        self.last_say = Some(last);
        self.speech_fade = 1.0;
        self.speech_ttl_remaining = Some(speech_hold_sec(&text));
        self.speech_is_curse_tag = is_tag;
    }

    /// Tick speech hold + fade + curse-tag reinsert / 15s tic (P3#16).
    ///
    /// // C++: after `speechFadeETATime`, `speechFade -= 0.05 * frameRateFactor`
    /// // C++ ~22402–22437: reinsert tag after non-tag speech; 15s nervous tic
    /// Keeps [`Self::last_say`] after bubble clear.
    pub fn tick_speech(&mut self, wall_dt: f32, frame_rate_factor: f32) {
        if wall_dt > 0.0 {
            self.curse_tag_idle_sec += wall_dt;
        }
        if self.current_speech.is_none() {
            // C++ nervous tic: show curse tag every 15s when idle.
            if self.curse_name.is_some() && self.curse_tag_idle_sec > MAX_CURSE_TAG_DISPLAY_GAP {
                self.show_curse_tag();
            }
            return;
        }
        if wall_dt <= 0.0 && frame_rate_factor <= 0.0 {
            return;
        }
        let mut rem = self.speech_ttl_remaining.unwrap_or(0.0);
        if rem > 0.0 {
            rem -= wall_dt;
            if rem > 0.0 {
                self.speech_ttl_remaining = Some(rem);
                return;
            }
            self.speech_ttl_remaining = Some(0.0);
        }
        let frf = if frame_rate_factor > 0.0 {
            frame_rate_factor
        } else {
            (wall_dt * 60.0).clamp(0.0, 4.0)
        };
        self.speech_fade -= SPEECH_FADE_STEP * frf;
        if self.speech_fade <= 0.0 {
            self.speech_is_curse = false;
            // Reinsert curse tag after non-tag speech (C++ ~22409–22419).
            let was_tag = self.speech_is_curse_tag;
            self.current_speech = None;
            self.speech_fade = 1.0;
            self.speech_ttl_remaining = None;
            self.speech_is_curse_tag = false;
            // last_say retained for headless
            if !was_tag && self.curse_name.is_some() {
                self.show_curse_tag();
            }
        }
    }
}

/// Table of all known players for this session.
#[derive(Debug, Clone, Default)]
pub struct LiveWorld {
    players: HashMap<i32, LiveObject>,
    /// Our player id once bound.
    pub our_id: Option<i32>,
    /// C++ `locationSpeech` — LS bubbles at world tiles.
    pub location_speech: Vec<LocationSpeech>,
    /// P3#17: PS `*map` / `*label` soft-FB markers (session map-pointer list).
    ///
    /// // C++: temp `homePosStack` entries; we keep a parallel list for world draw.
    pub says_pointers: Vec<SaysPointerMarker>,
    /// C++ `homePosStack` — permanent stakes + temp map/person homes (L-HUD residual).
    pub home_stack: HomePosStack,
}

impl LiveWorld {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.players.len()
    }

    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
    }

    pub fn get(&self, id: i32) -> Option<&LiveObject> {
        self.players.get(&id)
    }

    pub fn get_mut(&mut self, id: i32) -> Option<&mut LiveObject> {
        self.players.get_mut(&id)
    }

    pub fn our(&self) -> Option<&LiveObject> {
        self.our_id.and_then(|id| self.players.get(&id))
    }

    pub fn living_ids(&self) -> Vec<i32> {
        let mut ids: Vec<i32> = self
            .players
            .values()
            .filter(|o| !o.deleted)
            .map(|o| o.id)
            .collect();
        ids.sort_unstable();
        ids
    }

    /// Apply one PU line. Returns whether the object is newly inserted.
    ///
    /// Also maintains `held_by_adult_id` from adult `held_id < 0` (C++ ~19824).
    ///
    /// P3#22: baby drop sets `heldByDropOffset` from `lastHeldByRawPos` (C++ ~19242).
    pub fn apply_pu(&mut self, pu: &PlayerUpdate) -> bool {
        // Baby PU while still marked held + done_moving → drop handoff (C++ ~19242).
        if !pu.deleted && pu.done_moving_seq_num > 0 {
            if let Some(existing) = self.players.get_mut(&pu.player_id) {
                if existing.held_by_adult_id != -1 {
                    existing.begin_drop_from_arms(pu.x as f32, pu.y as f32);
                }
            }
        }
        if pu.deleted {
            if let Some(obj) = self.players.get_mut(&pu.player_id) {
                obj.apply_pu(pu);
            } else {
                // Delete for unknown id — still record a stub for reason queries.
                let mut obj = LiveObject::from_pu(pu);
                obj.deleted = true;
                obj.on_screen = false;
                self.players.insert(pu.player_id, obj);
            }
            // Adult deleted — clear babies who thought this adult held them.
            self.clear_held_by_adult(pu.player_id, None);
            return false;
        }
        let inserted = match self.players.get_mut(&pu.player_id) {
            Some(obj) => {
                obj.apply_pu(pu);
                false
            }
            None => {
                self.players.insert(pu.player_id, LiveObject::from_pu(pu));
                true
            }
        };
        // Sync held-by links from this adult's held_id (baby = -held_id).
        if pu.held_id < 0 {
            let baby_id = -pu.held_id;
            self.clear_held_by_adult(pu.player_id, Some(baby_id));
            if let Some(baby) = self.players.get_mut(&baby_id) {
                baby.held_by_adult_id = pu.player_id;
            }
        } else {
            self.clear_held_by_adult(pu.player_id, None);
        }
        // C++ `updatePersonHomeLocation` when temp home tracks this person.
        if !pu.deleted {
            self.home_stack
                .update_person_location(pu.player_id, pu.x, pu.y);
        }
        inserted
    }

    /// Clear `held_by_adult_id` for players held by `adult_id`, optionally keep one baby.
    ///
    /// When a baby is released, arm drop-offset slide from last held raw pos.
    fn clear_held_by_adult(&mut self, adult_id: i32, keep_baby: Option<i32>) {
        for o in self.players.values_mut() {
            if o.held_by_adult_id == adult_id {
                if keep_baby != Some(o.id) {
                    let gx = o.x as f32;
                    let gy = o.y as f32;
                    // Sets held_by_adult_id = -1; uses raw pos when known.
                    o.begin_drop_from_arms(gx, gy);
                }
            }
        }
    }

    /// BW (baby wiggle) server tag — held babies bounce; ground babies flip (C++ ~21748).
    pub fn apply_baby_wiggle(&mut self, ids: &[i32]) {
        for &id in ids {
            if let Some(o) = self.players.get_mut(&id) {
                if o.held_by_adult_id != -1 {
                    o.start_baby_wiggle();
                } else {
                    // Ground: flip + brief moving→ground (C++ addNewAnim moving/ground).
                    o.facing = if o.facing < 0 { 1 } else { -1 };
                    o.anim.switch_to(crate::anim_bank::ANIM_MOVING, None);
                    o.anim.switch_to(crate::anim_bank::ANIM_GROUND, None);
                }
            }
        }
    }

    /// Mark players out of range (PO). Does not delete.
    pub fn apply_out_of_range(&mut self, ids: &[i32]) {
        for &id in ids {
            if let Some(o) = self.players.get_mut(&id) {
                o.out_of_range = true;
                o.on_screen = false;
            }
        }
    }

    pub fn apply_names(&mut self, names: &[PlayerName]) {
        for n in names {
            let display = if n.last_name.is_empty() {
                n.first_name.clone()
            } else {
                format!("{} {}", n.first_name, n.last_name)
            };
            if let Some(o) = self.players.get_mut(&n.player_id) {
                o.name = Some(display);
            } else {
                // Name before first PU — create minimal stub.
                let mut stub = LiveObject::from_pu(&PlayerUpdate {
                    player_id: n.player_id,
                    display_id: 0,
                    facing: 0,
                    action: 0,
                    action_target_x: 0,
                    action_target_y: 0,
                    held_id_raw: "0".into(),
                    held_id: 0,
                    held_origin_valid: false,
                    held_origin_x: 0,
                    held_origin_y: 0,
                    held_transition_source_id: -1,
                    heat: 0.0,
                    done_moving_seq_num: 0,
                    force: false,
                    x: 0,
                    y: 0,
                    age: 0.0,
                    age_rate: 0.0,
                    move_speed: 0.0,
                    clothing_set: "0;0;0;0;0;0".into(),
                    just_ate: false,
                    last_ate_id: 0,
                    responsible_id: -1,
                    held_yum: false,
                    held_learned: false,
                    deleted: false,
                    delete_reason: None,
                });
                stub.name = Some(display);
                stub.on_screen = false;
                self.players.insert(n.player_id, stub);
            }
        }
    }

    pub fn apply_lineages(&mut self, lineages: &[Lineage]) {
        for ln in lineages {
            // LN chain: first id is the subject player.
            let Some(&pid) = ln.chain.first() else {
                continue;
            };
            if let Some(o) = self.players.get_mut(&pid) {
                o.lineage = Some(ln.clone());
            }
        }
    }

    /// Apply PE lines without an emotion bank (wire index only; no EXTRA resolve).
    pub fn apply_emots(&mut self, emots: &[PlayerEmot]) {
        let _ = self.apply_emots_with_bank(emots, None, DEFAULT_EMOT_DURATION_SEC);
    }

    /// Apply PE with optional emotion table for `extraAnimIndex` + default TTL.
    ///
    /// Returns `(player_id, emot_index, map_x, map_y)` for each PE that should
    /// play creation sounds (C++ `newEmotPlaySound`).
    ///
    /// // C++: LivingLifePage PLAYER_EMOT — permanent vs temporary + extra anim
    pub fn apply_emots_with_bank(
        &mut self,
        emots: &[PlayerEmot],
        bank: Option<&EmotionBank>,
        default_duration_sec: f32,
    ) -> Vec<(i32, i32, f32, f32)> {
        let dur = if default_duration_sec > 0.0 {
            default_duration_sec
        } else {
            DEFAULT_EMOT_DURATION_SEC
        };
        let mut sound_targets = Vec::new();
        for e in emots {
            if let Some(o) = self.players.get_mut(&e.player_id) {
                if let Some(emot_idx) = o.apply_emot(e, bank, dur) {
                    if !o.out_of_range {
                        sound_targets.push((o.id, emot_idx, o.x as f32, o.y as f32));
                    }
                }
            }
        }
        sound_targets
    }

    /// Tick temporary emote TTLs for all living players (wall seconds).
    ///
    /// Returns `(player_id, cleared_emot_index, map_x, map_y)` for decay sounds
    /// (C++ ~22469).
    pub fn tick_emots(&mut self, wall_dt: f32) -> Vec<(i32, i32, f32, f32)> {
        if wall_dt <= 0.0 {
            return Vec::new();
        }
        let mut cleared = Vec::new();
        for o in self.players.values_mut() {
            if !o.deleted {
                if let Some(emot_idx) = o.tick_emot(wall_dt) {
                    if !o.out_of_range {
                        cleared.push((o.id, emot_idx, o.x as f32, o.y as f32));
                    }
                }
            }
        }
        cleared
    }

    /// Apply PS lines onto matching players (creates no stubs — needs prior PU).
    ///
    /// Also stores map/label pointer markers ([`Self::says_pointers`]) when
    /// present — even if the speaker is unknown (pointer-only self-think PS).
    pub fn apply_says(&mut self, says: &[PlayerSays]) {
        for ps in says {
            self.apply_says_pointer(ps);
            if let Some(o) = self.players.get_mut(&ps.player_id) {
                o.apply_says(ps);
            }
        }
    }

    /// Store / replace PS `*map` / `*label` soft-FB marker for this speaker.
    ///
    /// // C++ ~20712–20900: strip pointers + `addTempHomeLocation` (ourID only).
    /// // Soft-FB: apply whenever parse found map/label (server only sends to
    /// // the intended client for private map think).
    pub fn apply_says_pointer(&mut self, ps: &PlayerSays) {
        let Some(marker) = SaysPointerMarker::from_ps(ps) else {
            return;
        };
        // One active pointer per speaker (C++ replaces all temp homes).
        self.says_pointers
            .retain(|m| m.speaker_id != ps.player_id);
        // C++ `addTempHomeLocation` into homePosStack (priority-aware).
        if let Some((mx, my)) = marker.map_tile() {
            let person = marker.target_player_id.is_some() || marker.target_label.is_some();
            let person_id = marker.target_player_id.unwrap_or(-1);
            let key = marker
                .label_text()
                .map(|s| s.to_ascii_lowercase())
                .or_else(|| {
                    if marker.map.is_some() {
                        Some("map".to_string())
                    } else {
                        None
                    }
                });
            // Wall clock optional — 0 means no ETA for pure map; person keys get ETA from now=0 + 60.
            let now = 0.0_f64;
            self.home_stack.add_temp(
                mx,
                my,
                person,
                person_id,
                key.as_deref(),
                now,
            );
        }
        self.says_pointers.push(marker);
    }

    /// C++ MX home-marker stake add/remove when **we** caused the change.
    pub fn apply_home_marker_mx(
        &mut self,
        x: i32,
        y: i32,
        old_is_home: bool,
        new_is_home: bool,
        caused_by_us: bool,
    ) {
        if !caused_by_us {
            return;
        }
        if new_is_home {
            self.home_stack.add_home(x, y);
        } else if old_is_home {
            self.home_stack.remove_at(x, y);
        }
    }

    /// Apply CU (CURSED) lines — set curse level/name + show tag (P3#16).
    pub fn apply_cursed(&mut self, cursed: &[crate::parse::CursedPlayer]) {
        for c in cursed {
            if let Some(o) = self.players.get_mut(&c.player_id) {
                o.apply_cursed(c.level, c.name.as_deref());
            }
        }
    }

    /// Apply LS lines; replaces any speech at the same `(x, y)` tile.
    ///
    /// // C++: push new then delete older entries with same pos
    pub fn apply_location_says(&mut self, says: &[LocationSays]) {
        for ls in says {
            self.location_speech
                .retain(|e| !(e.x == ls.x && e.y == ls.y));
            self.location_speech
                .push(LocationSpeech::new(ls.x, ls.y, ls.text.clone()));
        }
    }

    /// Tick player speech + location speech + map/label pointers (hold then fade).
    pub fn tick_speech(&mut self, wall_dt: f32, frame_rate_factor: f32) {
        if wall_dt <= 0.0 && frame_rate_factor <= 0.0 {
            return;
        }
        for o in self.players.values_mut() {
            if !o.deleted {
                o.tick_speech(wall_dt, frame_rate_factor);
            }
        }
        self.location_speech
            .retain_mut(|ls| ls.tick(wall_dt, frame_rate_factor));
        self.says_pointers
            .retain_mut(|m| m.tick(wall_dt, frame_rate_factor));
    }

    /// Active map/label pointer markers (for soft-FB draw / tests).
    pub fn says_pointers(&self) -> &[SaysPointerMarker] {
        &self.says_pointers
    }

    pub fn apply_dying(&mut self, dying: &[DyingPlayer]) {
        for d in dying {
            if let Some(o) = self.players.get_mut(&d.player_id) {
                o.dying = true;
                o.sick = d.is_sick;
            }
        }
    }

    pub fn apply_healed(&mut self, ids: &[i32]) {
        for &id in ids {
            if let Some(o) = self.players.get_mut(&id) {
                o.dying = false;
                o.sick = false;
            }
        }
    }

    /// Record PM starts; mark moving (position still from last PU until path ends).
    pub fn apply_moves_start(&mut self, moves: &[PlayerMoveStart]) {
        for m in moves {
            if let Some(o) = self.players.get_mut(&m.player_id) {
                o.moving = true;
                o.last_move = Some(m.clone());
                // PM xs/ys are path origin on wire (often absolute or birth-relative).
                // Display starts at path origin; local player is refined by MoveState.
                o.x = m.xs;
                o.y = m.ys;
                o.display_x = m.xs as f32;
                o.display_y = m.ys as f32;
            }
        }
    }

    /// Step all living players' anim packs (frame counters + fade).
    ///
    /// P3#22: also steps action wiggle / baby wiggle / drop offset.
    pub fn step_anims(&mut self, bank: &mut AnimBank, anim_speed: f32, frame_rate_factor: f32) {
        let our = self.our_id;
        let ids = self.living_ids();
        for id in ids {
            if let Some(o) = self.players.get_mut(&id) {
                o.sync_anim_packs(bank);
                o.step_anim(bank, anim_speed, frame_rate_factor);
                o.step_wiggle_handoff(our == Some(id), frame_rate_factor);
            }
        }
    }

    /// Step anims and fire SoundAnimParam / footstep hooks (C++ `handleAnimSound`).
    ///
    /// // L-SOUND-TRIG: person + held tracks; floor `usingSound` for footstep=1
    pub fn step_anims_with_sounds(
        &mut self,
        bank: &mut AnimBank,
        sounds: &mut crate::sound_bank::SoundBank,
        content: &crate::content::ClientContent,
        map: &crate::client_map::ClientMap,
        anim_speed: f32,
        frame_rate_factor: f32,
    ) {
        let our = self.our_id;
        let ids = self.living_ids();
        for id in ids {
            // Snapshot fields we need after step (avoid borrow across mut).
            let (
                display,
                age,
                held_id,
                x,
                y,
                cur_anim,
                cur_held,
                old_frame,
                old_held_frame,
                out_of_range,
            ) = {
                let Some(o) = self.players.get_mut(&id) else {
                    continue;
                };
                o.sync_anim_packs(bank);
                (
                    if o.display_id > 0 { o.display_id } else { 19 },
                    o.current_age(),
                    o.held_id,
                    o.x,
                    o.y,
                    o.anim.cur_anim,
                    o.anim.cur_held_anim,
                    o.anim.animation_frame_count,
                    o.anim.held_animation_frame_count,
                    o.out_of_range,
                )
            };
            if let Some(o) = self.players.get_mut(&id) {
                o.step_anim(bank, anim_speed, frame_rate_factor);
                o.step_wiggle_handoff(our == Some(id), frame_rate_factor);
            }
            let (new_frame, new_held_frame) = self
                .players
                .get(&id)
                .map(|o| {
                    (
                        o.anim.animation_frame_count,
                        o.anim.held_animation_frame_count,
                    )
                })
                .unwrap_or((old_frame, old_held_frame));

            // Person anim sounds (skip out-of-range like C++).
            // Pass live player id as source for offScreenSound (P2#13).
            if !out_of_range {
                crate::sound_bank::handle_anim_sound_ex(
                    sounds,
                    bank,
                    content,
                    map,
                    display,
                    age,
                    cur_anim,
                    old_frame,
                    new_frame,
                    x as f32,
                    y as f32,
                    1.0, // frf already baked into frame deltas
                    id,
                    self.our_id,
                );
            }
            // Held-item anim sounds (source = person holding them).
            if held_id > 0 && !out_of_range {
                crate::sound_bank::handle_anim_sound_ex(
                    sounds,
                    bank,
                    content,
                    map,
                    held_id,
                    0.0,
                    cur_held,
                    old_held_frame,
                    new_held_frame,
                    x as f32,
                    y as f32,
                    1.0,
                    id,
                    self.our_id,
                );
            }
        }
    }

    /// Bind our id if not set (first non-deleted PU near map center heuristic left to session).
    pub fn set_our_id(&mut self, id: i32) {
        self.our_id = Some(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim_bank::{
        SpriteAnimParam, ANIM_DOING, ANIM_EATING, ANIM_GROUND, ANIM_HELD, ANIM_MOVING,
    };
    use crate::parse::parse_pu_line;

    fn sample_pu_line(id: i32, x: i32, y: i32, held: i32) -> String {
        // 25-field live PU
        format!(
            "{id} 19 0 0 0 0 {held} 0 0 0 -1 0.50 1 0 {x} {y} 20.00 60.00 3.75 0;0;0;0;0;0 0 0 -1 0 0"
        )
    }

    #[test]
    fn apply_pu_inserts_and_updates() {
        let mut w = LiveWorld::new();
        let pu = parse_pu_line(&sample_pu_line(7, 10, 20, 33)).unwrap();
        assert!(w.apply_pu(&pu));
        assert_eq!(w.len(), 1);
        let o = w.get(7).unwrap();
        assert_eq!((o.x, o.y), (10, 20));
        assert_eq!(o.held_id, 33);
        assert!(!o.deleted);

        let pu2 = parse_pu_line(&sample_pu_line(7, 11, 20, 0)).unwrap();
        assert!(!w.apply_pu(&pu2));
        let o = w.get(7).unwrap();
        assert_eq!((o.x, o.y), (11, 20));
        assert_eq!(o.held_id, 0);
    }

    #[test]
    fn apply_pu_delete_marks_deleted() {
        let mut w = LiveWorld::new();
        let pu = parse_pu_line(&sample_pu_line(3, 0, 0, 0)).unwrap();
        w.apply_pu(&pu);
        // force X X reason_hunger
        let del = parse_pu_line(
            "3 19 0 0 0 0 0 0 0 0 -1 0.00 1 0 X X reason_hunger",
        )
        .unwrap();
        assert!(del.deleted);
        w.apply_pu(&del);
        let o = w.get(3).unwrap();
        assert!(o.deleted);
        assert!(!o.on_screen);
        assert_eq!(o.delete_reason.as_deref(), Some("reason_hunger"));
        assert!(w.living_ids().is_empty());
    }

    #[test]
    fn name_and_emot_and_dying() {
        let mut w = LiveWorld::new();
        let pu = parse_pu_line(&sample_pu_line(5, 1, 2, 0)).unwrap();
        w.apply_pu(&pu);
        w.apply_names(&[PlayerName {
            player_id: 5,
            first_name: "ADA".into(),
            last_name: "SNOW".into(),
        }]);
        w.apply_emots(&[PlayerEmot {
            player_id: 5,
            emot_index: 2,
            ttl_sec: None,
        }]);
        w.apply_dying(&[DyingPlayer {
            player_id: 5,
            is_sick: true,
        }]);
        let o = w.get(5).unwrap();
        assert_eq!(o.name.as_deref(), Some("ADA SNOW"));
        assert_eq!(o.last_emot_index, Some(2));
        assert!(o.emot_ttl_remaining.is_some());
        // Without bank, facial PE does not force ANIM_EXTRA.
        assert_eq!(o.emot_extra_index, None);
        assert!(o.dying && o.sick);
        w.apply_healed(&[5]);
        let o = w.get(5).unwrap();
        assert!(!o.dying && !o.sick);
    }

    #[test]
    fn pe_permanent_and_ttl_and_extra_from_bank() {
        use crate::emotion::EmotionBank;

        let bank = EmotionBank::from_ini_strings(
            "/happy\n/mad\n/wave\n",
            "0 1843 0 0 0 0\n1839 1842 0 0 0 0\n0 0 0 0 0 0 2\n",
        );
        let mut w = LiveWorld::new();
        let pu = parse_pu_line(&sample_pu_line(9, 0, 0, 0)).unwrap();
        w.apply_pu(&pu);

        // Temporary facial — no extra anim
        w.apply_emots_with_bank(
            &[PlayerEmot {
                player_id: 9,
                emot_index: 0,
                ttl_sec: Some(3.0),
            }],
            Some(&bank),
            10.0,
        );
        let o = w.get(9).unwrap();
        assert_eq!(o.last_emot_index, Some(0));
        assert_eq!(o.emot_ttl_remaining, Some(3.0));
        assert_eq!(o.emot_extra_index, None);
        assert_eq!(o.desired_anim_type(), crate::anim_bank::ANIM_GROUND);

        // Gesture with extra_anim_index=2 — first PE toggles to EXTRA (from init EXTRA_B)
        w.apply_emots_with_bank(
            &[PlayerEmot {
                player_id: 9,
                emot_index: 2,
                ttl_sec: Some(5.0),
            }],
            Some(&bank),
            10.0,
        );
        let o = w.get(9).unwrap();
        assert_eq!(o.emot_extra_index, Some(2));
        assert_eq!(o.emot_extra_anim_type, crate::anim_bank::ANIM_EXTRA);
        assert_eq!(o.desired_anim_type(), crate::anim_bank::ANIM_EXTRA);
        assert_eq!(o.anim.extra_index, 2);

        // Second gesture PE toggles to EXTRA_B
        w.apply_emots_with_bank(
            &[PlayerEmot {
                player_id: 9,
                emot_index: 2,
                ttl_sec: Some(5.0),
            }],
            Some(&bank),
            10.0,
        );
        let o = w.get(9).unwrap();
        assert_eq!(o.emot_extra_anim_type, crate::anim_bank::ANIM_EXTRA_B);
        assert_eq!(o.desired_anim_type(), crate::anim_bank::ANIM_EXTRA_B);
        assert_eq!(o.anim.extra_index_b, 2);

        // Permanent layer (no sound target when already stacked is fine)
        w.apply_emots_with_bank(
            &[PlayerEmot {
                player_id: 9,
                emot_index: 1,
                ttl_sec: Some(-1.0),
            }],
            Some(&bank),
            10.0,
        );
        let o = w.get(9).unwrap();
        assert_eq!(o.permanent_emots, vec![1]);
        // Temporary still present
        assert_eq!(o.last_emot_index, Some(2));
        let draw = o.emot_draw_indices();
        assert_eq!(draw, vec![2, 1]);

        // TTL expiry clears temporary + extra, keeps permanent
        let cleared = w.tick_emots(6.0);
        assert_eq!(cleared.len(), 1);
        assert_eq!(cleared[0].1, 2); // cleared emot index
        let o = w.get(9).unwrap();
        assert_eq!(o.last_emot_index, None);
        assert_eq!(o.emot_extra_index, None);
        assert_eq!(o.permanent_emots, vec![1]);
        assert_eq!(o.emot_draw_indices(), vec![1]);
        // Toggle slot preserved for next PE
        assert_eq!(o.emot_extra_anim_type, crate::anim_bank::ANIM_EXTRA_B);
    }

    #[test]
    fn clothing_set_slots() {
        let c = ClothingSet::parse("1;2;3;4;5;6");
        assert_eq!(c.hat(), "1");
        assert_eq!(c.backpack(), "6");
        assert!(!c.is_empty_slot(0));
        assert_eq!(ClothingSet::default().resolve_shoe_slot(), 2);
        let front_full = ClothingSet::parse("0;0;99;0;0;0");
        assert_eq!(front_full.resolve_shoe_slot(), 3);
        let both_full = ClothingSet::parse("0;0;1;2;0;0");
        assert_eq!(both_full.resolve_shoe_slot(), 2);
    }

    #[test]
    fn clothing_char_to_slot_protocol() {
        assert_eq!(clothing_char_to_slot('h'), Some(0));
        assert_eq!(clothing_char_to_slot('T'), Some(1));
        assert_eq!(clothing_char_to_slot('s'), Some(2));
        assert_eq!(clothing_char_to_slot('b'), Some(4));
        assert_eq!(clothing_char_to_slot('p'), Some(5));
        assert_eq!(clothing_char_to_slot('n'), None);
        assert_eq!(clothing_char_to_slot('x'), None);
        assert_eq!(CLOTHING_SLOT_NAMES[0], "hat");
        assert_eq!(CLOTHING_SLOT_NAMES[5], "backpack");
    }

    #[test]
    fn pm_marks_moving() {
        let mut w = LiveWorld::new();
        let pu = parse_pu_line(&sample_pu_line(9, 0, 0, 0)).unwrap();
        w.apply_pu(&pu);
        w.apply_moves_start(&[PlayerMoveStart {
            player_id: 9,
            xs: 0,
            ys: 0,
            total_sec: 1.0,
            eta_sec: 1.0,
            trunc: 1,
            deltas: vec![(1, 0)],
        }]);
        let o = w.get(9).unwrap();
        assert!(o.moving);
        assert!(o.last_move.is_some());
        // Settling PU (done_moving_seq > 0) clears motion.
        let pu2 = parse_pu_line(&sample_pu_line(9, 1, 0, 0)).unwrap();
        w.apply_pu(&pu2);
        assert!(!w.get(9).unwrap().moving);
    }

    /// Intermediate PU with done_moving=0 must not kill mid-path walk anim (Jason).
    #[test]
    fn mid_path_pu_preserves_moving_and_display() {
        let mut w = LiveWorld::new();
        w.apply_pu(&parse_pu_line(&sample_pu_line(3, 0, 0, 0)).unwrap());
        w.apply_moves_start(&[PlayerMoveStart {
            player_id: 3,
            xs: 0,
            ys: 0,
            total_sec: 2.0,
            eta_sec: 2.0,
            trunc: 0,
            deltas: vec![(2, 0)],
        }]);
        {
            let o = w.get_mut(3).unwrap();
            o.set_display_pos(0.4, 0.0);
            assert!(o.moving);
        }
        // done_moving=0 force=0, new held id — clothing/held update mid-walk.
        let mid = "3 19 0 0 0 0 99 0 0 0 -1 0.50 0 0 0 0 20.00 60.00 3.75 0;0;0;0;0;0 0 0 -1 0 0";
        w.apply_pu(&parse_pu_line(mid).unwrap());
        let o = w.get(3).unwrap();
        assert!(o.moving, "mid-path PU must keep moving");
        assert!((o.display_x - 0.4).abs() < 1e-4, "keep fractional display");
        assert_eq!(o.held_id, 99);
    }

    #[test]
    fn out_of_range_flag() {
        let mut w = LiveWorld::new();
        w.apply_pu(&parse_pu_line(&sample_pu_line(1, 0, 0, 0)).unwrap());
        w.apply_out_of_range(&[1]);
        assert!(w.get(1).unwrap().out_of_range);
        assert!(!w.get(1).unwrap().on_screen);
    }

    #[test]
    fn anim_pack_select_on_move_and_eat() {
        let mut bank = AnimBank::new(".");
        for ty in [ANIM_GROUND, ANIM_MOVING, ANIM_EATING] {
            bank.insert(crate::anim_bank::ObjectAnimation {
                object_id: 19,
                anim_type: ty,
                sprite_params: vec![SpriteAnimParam::default()],
                ..Default::default()
            });
        }
        let mut w = LiveWorld::new();
        w.apply_pu(&parse_pu_line(&sample_pu_line(1, 0, 0, 0)).unwrap());
        {
            let o = w.get_mut(1).unwrap();
            o.moving = true;
            o.sync_anim_packs(&mut bank);
            assert_eq!(o.anim.cur_anim, ANIM_MOVING);
            assert_eq!(o.desired_anim_type(), ANIM_MOVING);
            // Settle fade so eating select is applied (not stacked).
            o.anim.last_anim_fade = 0.0;
        }
        {
            let o = w.get_mut(1).unwrap();
            o.moving = false;
            o.just_ate = true;
            o.sync_anim_packs(&mut bank);
            assert_eq!(o.anim.cur_anim, ANIM_EATING);
            let pack = o.person_anim_pack(false);
            // mid-fade from moving → eating
            assert!(pack.anim_fade > 0.0);
            assert_eq!(pack.fade_target_type, ANIM_EATING);
        }
    }

    #[test]
    fn anim_state_survives_pu() {
        let mut w = LiveWorld::new();
        w.apply_pu(&parse_pu_line(&sample_pu_line(2, 0, 0, 0)).unwrap());
        w.get_mut(2).unwrap().anim.animation_frame_count = 42.0;
        w.apply_pu(&parse_pu_line(&sample_pu_line(2, 1, 0, 0)).unwrap());
        assert!((w.get(2).unwrap().anim.animation_frame_count - 42.0).abs() < 1e-4);
    }

    #[test]
    fn step_anims_advances_frame_and_decays_fade() {
        let mut bank = AnimBank::new(".");
        bank.insert(crate::anim_bank::ObjectAnimation {
            object_id: 19,
            anim_type: ANIM_MOVING,
            sprite_params: vec![SpriteAnimParam {
                offset_x: 10.0,
                fade_max: 1.0,
                ..Default::default()
            }],
            ..Default::default()
        });
        bank.insert(crate::anim_bank::ObjectAnimation {
            object_id: 19,
            anim_type: ANIM_GROUND,
            sprite_params: vec![SpriteAnimParam::default()],
            ..Default::default()
        });
        let mut w = LiveWorld::new();
        w.apply_pu(&parse_pu_line(&sample_pu_line(1, 0, 0, 0)).unwrap());
        {
            let o = w.get_mut(1).unwrap();
            o.moving = true;
            o.sync_anim_packs(&mut bank);
            // Offset-only switch needs fade; force mid-fade for decay check.
            o.anim.last_anim_fade = 1.0;
            o.anim.animation_frame_count = 0.0;
        }
        w.step_anims(&mut bank, 1.0, 1.0);
        let o = w.get(1).unwrap();
        assert!(
            (o.anim.animation_frame_count - 1.0).abs() < 1e-4,
            "frame advanced"
        );
        assert!(
            (o.anim.last_anim_fade - 0.95).abs() < 1e-4,
            "fade decayed by 0.05, got {}",
            o.anim.last_anim_fade
        );
        let pack = o.person_anim_pack(false);
        assert!(pack.anim_fade > 0.9);
        assert_eq!(pack.fade_target_type, ANIM_MOVING);
    }

    #[test]
    fn ps_speech_hold_fade_and_last_say() {
        let mut w = LiveWorld::new();
        w.apply_pu(&parse_pu_line(&sample_pu_line(3, 0, 0, 0)).unwrap());
        w.apply_says(&[PlayerSays {
            player_id: 3,
            is_curse: false,
            text: "HI".into(),
            spoken: "HI".into(),
            map: None,
            target_label: None,
            target_player_id: None,
        }]);
        {
            let o = w.get(3).unwrap();
            assert_eq!(o.current_speech.as_deref(), Some("HI"));
            assert_eq!(o.last_say.as_deref(), Some("HI"));
            assert!((o.speech_fade - 1.0).abs() < 1e-6);
            let hold = speech_hold_sec("HI");
            assert!((o.speech_ttl_remaining.unwrap() - hold).abs() < 1e-4);
        }
        // Speech survives PU.
        w.apply_pu(&parse_pu_line(&sample_pu_line(3, 1, 0, 0)).unwrap());
        assert_eq!(w.get(3).unwrap().current_speech.as_deref(), Some("HI"));

        // Burn hold time.
        let hold = speech_hold_sec("HI");
        w.tick_speech(hold + 0.01, 1.0);
        assert!(w.get(3).unwrap().current_speech.is_some());
        assert!(w.get(3).unwrap().speech_fade < 1.0);

        // Fade out (~20 frames at 0.05).
        for _ in 0..25 {
            w.tick_speech(0.0, 1.0);
        }
        let o = w.get(3).unwrap();
        assert!(o.current_speech.is_none());
        assert_eq!(o.last_say.as_deref(), Some("HI"), "last_say kept for headless");
        assert!((o.speech_fade - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ls_replaces_same_cell_and_ticks() {
        let mut w = LiveWorld::new();
        w.apply_location_says(&[LocationSays {
            x: 5,
            y: 7,
            text: "A".into(),
        }]);
        w.apply_location_says(&[LocationSays {
            x: 5,
            y: 7,
            text: "B".into(),
        }]);
        // Longer text → longer hold so it outlives B after B's full fade.
        w.apply_location_says(&[LocationSays {
            x: 1,
            y: 1,
            text: "LONG SPEECH HERE".into(),
        }]);
        assert_eq!(w.location_speech.len(), 2);
        let at = w
            .location_speech
            .iter()
            .find(|e| e.x == 5 && e.y == 7)
            .unwrap();
        assert_eq!(at.text, "B");

        let hold_b = speech_hold_sec("B");
        w.tick_speech(hold_b + 0.01, 1.0);
        for _ in 0..25 {
            w.tick_speech(0.0, 1.0);
        }
        // B expired; long speech still holding.
        assert_eq!(w.location_speech.len(), 1);
        assert_eq!(w.location_speech[0].text, "LONG SPEECH HERE");
    }

    #[test]
    fn speech_is_curse_from_ps() {
        let mut w = LiveWorld::new();
        w.apply_pu(&parse_pu_line(&sample_pu_line(1, 0, 0, 0)).unwrap());
        w.apply_says(&[PlayerSays {
            player_id: 1,
            is_curse: true,
            text: "CURSE".into(),
            spoken: "CURSE".into(),
            map: None,
            target_label: None,
            target_player_id: None,
        }]);
        assert!(w.get(1).unwrap().speech_is_curse);
        // Uncursed speaker + successful curse → half purple.
        assert_eq!(speech_text_rgb(w.get(1).unwrap()), [128, 0, 128]);
    }

    #[test]
    fn speech_text_rgb_cursed_speaker_and_dying() {
        let mut o = LiveObject::from_pu(&parse_pu_line(&sample_pu_line(1, 0, 0, 0)).unwrap());
        // Default: black
        assert_eq!(speech_text_rgb(&o), [0, 0, 0]);
        // Cursed speaker, non-curse bubble → white
        o.curse_level = 2;
        o.speech_is_curse = false;
        assert_eq!(speech_text_rgb(&o), [255, 255, 255]);
        // Cursed + successful curse → bright purple
        o.speech_is_curse = true;
        assert_eq!(speech_text_rgb(&o), [223, 0, 223]);
        // Dying (not sick) overrides to white
        o.dying = true;
        o.sick = false;
        assert_eq!(speech_text_rgb(&o), [255, 255, 255]);
    }

    /// P3#16: CU sets name, shows tag, reinsert after speech, 15s tic.
    #[test]
    fn curse_tag_reinsert_and_15s_tic() {
        let mut w = LiveWorld::new();
        w.apply_pu(&parse_pu_line(&sample_pu_line(1, 0, 0, 0)).unwrap());
        w.apply_cursed(&[crate::parse::CursedPlayer {
            player_id: 1,
            level: 2,
            name: Some("Evil_Bob".into()),
        }]);
        {
            let o = w.get(1).unwrap();
            assert_eq!(o.curse_level, 2);
            assert_eq!(o.curse_name.as_deref(), Some("Evil Bob"));
            assert_eq!(o.current_speech.as_deref(), Some("X Evil Bob X"));
            assert!(o.speech_is_curse_tag);
            assert!((o.curse_tag_idle_sec).abs() < 1e-5);
        }
        // Fade out the initial tag completely (was_tag → no reinsert).
        let hold = speech_hold_sec("X Evil Bob X");
        w.tick_speech(hold + 0.01, 1.0);
        for _ in 0..40 {
            w.tick_speech(0.0, 1.0);
        }
        assert!(
            w.get(1).unwrap().current_speech.is_none(),
            "tag should clear without reinsert"
        );

        // Normal speech, then reinsert tag after fade (C++ ~22409).
        w.get_mut(1).unwrap().curse_tag_idle_sec = 0.0; // no babble wrap
        w.apply_says(&[PlayerSays {
            player_id: 1,
            is_curse: false,
            text: "HI".into(),
            spoken: "HI".into(),
            map: None,
            target_label: None,
            target_player_id: None,
        }]);
        assert_eq!(w.get(1).unwrap().current_speech.as_deref(), Some("HI"));
        assert!(!w.get(1).unwrap().speech_is_curse_tag);
        let hold_hi = speech_hold_sec("HI");
        w.tick_speech(hold_hi + 0.01, 1.0);
        for _ in 0..40 {
            w.tick_speech(0.0, 1.0);
        }
        let o = w.get(1).unwrap();
        assert_eq!(
            o.current_speech.as_deref(),
            Some("X Evil Bob X"),
            "reinsert after non-tag speech"
        );
        assert!(o.speech_is_curse_tag);

        // Clear tag again; wait >15s idle → nervous tic.
        let hold_t = speech_hold_sec("X Evil Bob X");
        w.tick_speech(hold_t + 0.01, 1.0);
        for _ in 0..40 {
            w.tick_speech(0.0, 1.0);
        }
        assert!(w.get(1).unwrap().current_speech.is_none());
        w.tick_speech(MAX_CURSE_TAG_DISPLAY_GAP + 0.1, 1.0);
        assert_eq!(
            w.get(1).unwrap().current_speech.as_deref(),
            Some("X Evil Bob X"),
            "15s nervous tic"
        );
    }

    #[test]
    fn curse_babble_wrap_when_gap_exceeded() {
        let mut w = LiveWorld::new();
        w.apply_pu(&parse_pu_line(&sample_pu_line(2, 0, 0, 0)).unwrap());
        {
            let o = w.get_mut(2).unwrap();
            o.curse_name = Some("Hex".into());
            o.curse_level = 1;
            o.curse_tag_idle_sec = MAX_CURSE_TAG_DISPLAY_GAP + 1.0;
        }
        w.apply_says(&[PlayerSays {
            player_id: 2,
            is_curse: false,
            text: "HELLO".into(),
            spoken: "HELLO".into(),
            map: None,
            target_label: None,
            target_player_id: None,
        }]);
        let o = w.get(2).unwrap();
        assert_eq!(o.current_speech.as_deref(), Some("X Hex X - HELLO"));
        assert!(o.speech_is_curse_tag);
        assert_eq!(format_curse_tag("Hex"), "X Hex X");
    }

    #[test]
    fn curse_level_zero_clears_tag() {
        let mut w = LiveWorld::new();
        w.apply_pu(&parse_pu_line(&sample_pu_line(4, 0, 0, 0)).unwrap());
        w.apply_cursed(&[crate::parse::CursedPlayer {
            player_id: 4,
            level: 1,
            name: Some("Doomed".into()),
        }]);
        assert!(w.get(4).unwrap().speech_is_curse_tag);
        w.apply_cursed(&[crate::parse::CursedPlayer {
            player_id: 4,
            level: 0,
            name: None,
        }]);
        let o = w.get(4).unwrap();
        assert_eq!(o.curse_level, 0);
        assert!(o.curse_name.is_none());
        assert!(o.current_speech.is_none());
        assert!(!o.speech_is_curse_tag);
    }

    /// P3#17: pure map spot stores pointer, bubble is stripped spoken only.
    #[test]
    fn ps_map_pointer_apply_and_ttl() {
        use crate::parse::{parse_ps_line, SaysTargetLabel};

        let mut w = LiveWorld::new();
        w.apply_pu(&parse_pu_line(&sample_pu_line(38499, 0, 0, 0)).unwrap());
        let ps = parse_ps_line("38499/0 :SPECIAL SPOT *map 13 6 92").unwrap();
        assert_eq!(ps.spoken, ":SPECIAL SPOT");
        assert!(ps.map.is_some());
        w.apply_says(&[ps]);
        {
            let o = w.get(38499).unwrap();
            assert_eq!(o.current_speech.as_deref(), Some(":SPECIAL SPOT"));
            assert!(
                !o.current_speech.as_ref().unwrap().contains("*map"),
                "bubble must not contain pointer tokens"
            );
        }
        assert_eq!(w.says_pointers.len(), 1);
        let m = &w.says_pointers[0];
        assert_eq!(m.speaker_id, 38499);
        assert_eq!(m.map_tile(), Some((13, 6)));
        assert_eq!(m.map.as_ref().unwrap().map_age_seconds, Some(92));
        assert!(
            (m.ttl_remaining - 92.0).abs() < 1e-3,
            "ttl_remaining={}",
            m.ttl_remaining
        );
        assert!((m.fade - 1.0).abs() < 1e-6);

        // Hold burn then fade out.
        w.tick_speech(92.01, 1.0);
        assert_eq!(w.says_pointers.len(), 1);
        assert!(w.says_pointers[0].fade < 1.0);
        for _ in 0..40 {
            w.tick_speech(0.0, 1.0);
        }
        assert!(w.says_pointers.is_empty(), "pointer expired after fade");

        // Visitor + map: label + target id + map tile.
        w.apply_pu(&parse_pu_line(&sample_pu_line(38501, 1, 0, 0)).unwrap());
        w.apply_pu(&parse_pu_line(&sample_pu_line(38500, 3, 0, 0)).unwrap());
        let vis = parse_ps_line(
            "38501/0 OUTSIDER NAMELESS PERSON IS MY NEW FOLLOWER *visitor 38500 *map 3 0",
        )
        .unwrap();
        w.apply_says(&[vis]);
        assert_eq!(w.get(38501).unwrap().current_speech.as_deref(), Some(
            "OUTSIDER NAMELESS PERSON IS MY NEW FOLLOWER",
        ));
        assert_eq!(w.says_pointers.len(), 1);
        let m = &w.says_pointers[0];
        assert_eq!(m.target_label, Some(SaysTargetLabel::Visitor));
        assert_eq!(m.target_player_id, Some(38500));
        assert_eq!(m.map_tile(), Some((3, 0)));
        assert_eq!(m.label_text().as_deref(), Some("VISITOR"));
        // No map_age → TTL from spoken hold.
        let hold = speech_hold_sec("OUTSIDER NAMELESS PERSON IS MY NEW FOLLOWER");
        assert!((m.ttl_remaining - hold).abs() < 1e-3);

        // Replace same speaker's prior pointer.
        let again = parse_ps_line("38501/0 HI *map 9 9 5").unwrap();
        w.apply_says(&[again]);
        assert_eq!(w.says_pointers.len(), 1);
        assert_eq!(w.says_pointers[0].map_tile(), Some((9, 9)));
        assert!((w.says_pointers[0].ttl_remaining - 5.0).abs() < 1e-3);
    }

    #[test]
    fn ps_pointer_only_no_bubble_still_stores_marker() {
        use crate::parse::parse_ps_line;

        let mut w = LiveWorld::new();
        // Speaker unknown is ok for pointer list.
        let ps = parse_ps_line("99/0 *map 1 2 10").unwrap();
        assert!(ps.spoken.is_empty());
        w.apply_says(&[ps]);
        assert_eq!(w.says_pointers.len(), 1);
        assert_eq!(w.says_pointers[0].map_tile(), Some((1, 2)));
        assert!(w.get(99).is_none());
    }

    #[test]
    fn says_pointer_ttl_expert_bonus() {
        use crate::parse::{parse_ps_line, SaysTargetLabel};
        // No map_age → spoken hold + expert bonus.
        let ps = parse_ps_line("1/0 FIND *expert 2 *map 0 0").unwrap();
        assert_eq!(ps.target_label, Some(SaysTargetLabel::Expert));
        let ttl = says_pointer_ttl_sec(&ps);
        let expected = speech_hold_sec("FIND") + SAYS_POINTER_EXPERT_EXTRA_SEC;
        assert!((ttl - expected).abs() < 1e-3);

        // map_age present → age wins (expert bonus not applied on age path).
        let aged = parse_ps_line("1/0 FIND *expert 2 *map 0 0 40").unwrap();
        assert!((says_pointer_ttl_sec(&aged) - 40.0).abs() < 1e-3);
    }

    #[test]
    fn home_dir_index_cardinals() {
        // C++ getHomeDir: north=0, then CCW (NW=1, W=2, … E=6, NE=7).
        assert_eq!(home_dir_index(0.0, 0.0, 0.0, 10.0), Some(0)); // N
        assert_eq!(home_dir_index(0.0, 0.0, -10.0, 0.0), Some(2)); // W
        assert_eq!(home_dir_index(0.0, 0.0, 0.0, -10.0), Some(4)); // S
        assert_eq!(home_dir_index(0.0, 0.0, 10.0, 0.0), Some(6)); // E
        assert_eq!(home_dir_index(1.0, 1.0, 1.0, 1.0), None);
    }

    #[test]
    fn home_pos_stack_permanent_and_temp_priority() {
        let mut s = HomePosStack::new();
        s.add_home(10, 20);
        assert_eq!(s.len(), 1);
        let (dir, lab) = s.home_dir_and_label(10.0, 0.0);
        assert_eq!(dir, Some(0), "north of home");
        assert!(lab.is_none(), "permanent stake has no MAP label");

        // Temp map replaces temps but keeps permanent under stack end.
        s.add_temp(30, 20, false, -1, None, 0.0);
        assert_eq!(s.active_home().map(|p| (p.x, p.y)), Some((30, 20)));
        let (_, lab) = s.home_dir_and_label(0.0, 0.0);
        assert_eq!(lab.as_deref(), Some("MAP"));

        // Lower-priority visitor cannot trump map temp (priority 1).
        s.add_temp(99, 99, true, 5, Some("visitor"), 0.0);
        assert_eq!(
            s.active_home().map(|p| (p.x, p.y)),
            Some((30, 20)),
            "visitor does not trump map"
        );

        // Baby (4) also lower than map (1) — no replace.
        s.add_temp(1, 1, true, 7, Some("baby"), 0.0);
        assert_eq!(s.active_home().map(|p| (p.x, p.y)), Some((30, 20)));

        // Explicit map again OK.
        s.add_temp(40, 50, false, -1, Some("map"), 0.0);
        assert_eq!(s.active_home().map(|p| (p.x, p.y)), Some((40, 50)));

        s.add_home(0, 0);
        assert!(
            s.entries().iter().all(|p| !p.temporary),
            "add_home clears temps"
        );
        assert_eq!(s.active_home().map(|p| (p.x, p.y)), Some((0, 0)));
    }

    #[test]
    fn home_stack_from_ps_map_and_marker_mx() {
        let mut w = LiveWorld::new();
        w.set_our_id(1);
        w.apply_pu(&parse_pu_line(&sample_pu_line(1, 5, 5, 0)).unwrap());
        // PS map pointer → temp home
        let ps = crate::parse::PlayerSays {
            player_id: 1,
            is_curse: false,
            text: "GO *map 12 8".into(),
            spoken: "GO".into(),
            map: Some(crate::parse::SaysMapPointer {
                x: 12,
                y: 8,
                map_age_seconds: None,
            }),
            target_label: None,
            target_player_id: None,
        };
        w.apply_says_pointer(&ps);
        assert!(!w.home_stack.is_empty());
        assert_eq!(w.home_stack.active_home().map(|p| (p.x, p.y)), Some((12, 8)));

        // Our home marker place
        w.apply_home_marker_mx(3, 4, false, true, true);
        assert_eq!(
            w.home_stack.active_home().map(|p| (p.x, p.y, p.temporary)),
            Some((3, 4, false)),
            "permanent stake clears temp and becomes top"
        );
    }

    // ── P3#22 action wiggle / baby-held handoff ──────────────────────────────

    #[test]
    fn p3_22_pending_action_selects_doing_and_wiggle_offset() {
        let mut bank = AnimBank::new(".");
        for ty in [ANIM_GROUND, ANIM_DOING] {
            bank.insert(crate::anim_bank::ObjectAnimation {
                object_id: 19,
                anim_type: ty,
                sprite_params: vec![SpriteAnimParam::default()],
                ..Default::default()
            });
        }
        let mut w = LiveWorld::new();
        w.apply_pu(&parse_pu_line(&sample_pu_line(1, 0, 0, 0)).unwrap());
        {
            let o = w.get_mut(1).unwrap();
            o.action_target_x = 2;
            o.action_target_y = 0;
            o.start_pending_action_anim();
            assert!(o.pending_action);
            assert!(o.pending_action_animation_progress > 0.0);
            o.sync_anim_packs(&mut bank);
            assert_eq!(
                o.anim.cur_anim, ANIM_DOING,
                "pending action selects DOING pack"
            );
            // Mid-cycle peak toward +X
            o.pending_action_animation_progress = 0.5;
            let (ox, oy) = o.action_wiggle_units();
            assert!(ox > 0.0, "wiggle toward target +X got {ox}");
            assert!(oy.abs() < 1e-3);
        }
        // Step wraps while still pending
        {
            let o = w.get_mut(1).unwrap();
            o.pending_action_animation_progress = 0.99;
            o.step_wiggle_handoff(true, 1.0);
            assert!(
                o.pending_action_animation_progress < 0.1,
                "wrapped while pending"
            );
            o.clear_pending_action_flag();
            o.pending_action_animation_progress = 0.99;
            o.step_wiggle_handoff(true, 1.0);
            assert_eq!(
                o.pending_action_animation_progress, 0.0,
                "snaps when not pending"
            );
        }
    }

    #[test]
    fn p3_22_baby_wiggle_and_drop_handoff_anim() {
        let mut w = LiveWorld::new();
        // Baby first, then adult holds them (so held_by link can be written).
        w.apply_pu(&parse_pu_line(&sample_pu_line(7, 5, 5, 0)).unwrap());
        let mut adult = parse_pu_line(&sample_pu_line(10, 5, 5, 0)).unwrap();
        adult.held_id = -7;
        adult.held_id_raw = "-7".into();
        w.apply_pu(&adult);
        assert_eq!(w.get(7).unwrap().held_by_adult_id, 10);

        // BW while held → arm wiggle
        w.apply_baby_wiggle(&[7]);
        {
            let b = w.get(7).unwrap();
            assert!(b.baby_wiggle);
            assert_eq!(b.baby_wiggle_progress, 0.0);
            let ox = b.baby_wiggle_x_units(false);
            // progress 0 → offset 0
            assert_eq!(ox, 0.0);
        }
        {
            let b = w.get_mut(7).unwrap();
            b.baby_wiggle_progress = 0.5;
            let ox = b.baby_wiggle_x_units(false);
            assert!(ox > 0.0);
        }

        // Note hold pos then drop via adult clear held
        {
            let b = w.get_mut(7).unwrap();
            b.note_held_by_raw_pos(5.5, 5.3);
        }
        // Adult puts baby down
        let mut adult2 = parse_pu_line(&sample_pu_line(10, 5, 5, 0)).unwrap();
        adult2.held_id = 0;
        adult2.held_id_raw = "0".into();
        w.apply_pu(&adult2);
        {
            let b = w.get(7).unwrap();
            assert_eq!(b.held_by_adult_id, -1);
            // Drop offset from last held raw toward ground tile
            assert!(
                b.held_by_drop_offset_x != 0.0 || b.held_by_drop_offset_y != 0.0,
                "drop offset armed"
            );
            // Handoff anim: held → ground mid-fade
            assert_eq!(b.anim.last_anim, ANIM_HELD);
            assert_eq!(b.anim.cur_anim, ANIM_GROUND);
            assert!((b.anim.last_anim_fade - 1.0).abs() < 1e-6);
            let (dx, dy) = b.draw_pos_tiles();
            assert!((dx - (b.x as f32 + b.held_by_drop_offset_x)).abs() < 1e-6);
            assert!((dy - (b.y as f32 + b.held_by_drop_offset_y)).abs() < 1e-6);
        }
        // Step drop toward zero
        {
            let b = w.get_mut(7).unwrap();
            let before = b.held_by_drop_offset_x.abs() + b.held_by_drop_offset_y.abs();
            b.step_wiggle_handoff(false, 1.0);
            let after = b.held_by_drop_offset_x.abs() + b.held_by_drop_offset_y.abs();
            assert!(after < before, "drop slides toward origin");
        }
    }

    #[test]
    fn p3_22_held_pos_handoff_slides() {
        let mut w = LiveWorld::new();
        w.apply_pu(&parse_pu_line(&sample_pu_line(1, 0, 0, 0)).unwrap());
        {
            let o = w.get_mut(1).unwrap();
            o.begin_held_pos_handoff(3.0, 4.0);
            assert!(o.held_pos_override);
            let (x, y, _) = o.step_held_pos_toward(0.0, 0.0, 0.0, true, 1.0);
            assert!(x < 3.0 && x > 0.0, "slides toward hand");
            assert!(y < 4.0 && y > 0.0);
            assert!(o.held_pos_override);
        }
    }
}
