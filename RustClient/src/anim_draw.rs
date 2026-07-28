//! L-ANIM-DRAW — player/object animation pack select + dual-anim sample.
//!
//! C++: `LivingLifePage` `curAnim`/`lastAnim`/`lastAnimFade` +
//! `drawObjectAnimPacked` / `drawObjectAnim` (`animationBank.cpp`).
//! Haxe: single-record sample only — dual fade is Jason parity.
//!
//! Chunk **moving_eating_pack_select**: choose moving/eating/doing/ground/extra
//! packs from player state, cross-fade with `inAnimFade`, apply frozen-rot from
//! the moving track when target rock/rot is zero, and reseed timeline on type
//! switch (`forceZeroStart` / `randomStartPhase`).

use crate::anim_bank::{
    is_extra_anim_type, AnimBank, AnimSample, ObjectAnimation, SpriteAnimParam, ANIM_DOING,
    ANIM_EATING, ANIM_EXTRA, ANIM_EXTRA_B, ANIM_GROUND, ANIM_GROUND2, ANIM_HELD, ANIM_MOVING,
};

/// C++ `endAnimType` — sentinel for “no frozen arms”.
pub const ANIM_END: i32 = 6;

/// C++ fade step per frame at 60fps / `frameRateFactor=1`: `lastAnimFade -= 0.05`.
pub const ANIM_FADE_STEP: f32 = 0.05;

/// C++ `ObjectAnimPack` — packed parameters for one `drawObjectAnim` call.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectAnimPack {
    pub object_id: i32,
    /// Current (or *last* when cross-fading) anim type.
    pub anim_type: i32,
    pub frame_time: f32,
    /// Weight of `anim_type` (1 = fully current, 0 = fully target). C++ `inAnimFade`.
    pub anim_fade: f32,
    pub fade_target_type: i32,
    pub fade_target_frame_time: f32,
    /// Frozen rot timeline from last moving stint (C++ `frozenRotFrameCount/60`).
    pub frozen_rot_frame_time: f32,
    /// `ANIM_END` = no arm freeze; else override arm layers (usually `ANIM_MOVING`).
    pub frozen_arm_type: i32,
    pub frozen_arm_fade_target_type: i32,
    /// Extra index when type is `ANIM_EXTRA` (`setExtraIndex`).
    pub extra_index: i32,
    /// Extra index when type is `ANIM_EXTRA_B` (`setExtraIndexB`).
    pub extra_index_b: i32,
    /// Set true if frozen-rot was applied to any layer during sample.
    pub frozen_rot_used: bool,
}

impl ObjectAnimPack {
    /// Single-type pack (no cross-fade).
    pub fn single(object_id: i32, anim_type: i32, frame_time: f32) -> Self {
        Self {
            object_id,
            anim_type,
            frame_time,
            anim_fade: 1.0,
            fade_target_type: anim_type,
            fade_target_frame_time: frame_time,
            frozen_rot_frame_time: 0.0,
            frozen_arm_type: ANIM_END,
            frozen_arm_fade_target_type: ANIM_END,
            extra_index: -1,
            extra_index_b: -1,
            frozen_rot_used: false,
        }
    }
}

/// Per-entity anim state (C++ `LiveObject` curAnim / lastAnim / frame counts).
#[derive(Debug, Clone, PartialEq)]
pub struct AnimDrawState {
    pub cur_anim: i32,
    pub last_anim: i32,
    /// 1 → 0 while cross-fading last → cur. C++ `lastAnimFade`.
    pub last_anim_fade: f32,
    /// Frame counter (increments ≈ speed/BASE each step). Converted to sec /60 at pack.
    pub animation_frame_count: f32,
    pub last_animation_frame_count: f32,
    pub frozen_rot_frame_count: f32,
    pub frozen_rot_frame_count_used: bool,
    /// C++ `extraAnimIndex` for `ANIM_EXTRA` (`setExtraIndex`).
    pub extra_index: i32,
    /// C++ `extraAnimIndexB` for `ANIM_EXTRA_B` (`setExtraIndexB`).
    pub extra_index_b: i32,
    /// Queued types while a non-ground fade is still running (C++ `futureAnimStack`).
    pub future_stack: Vec<i32>,
    // Held-item track (C++ curHeldAnim / lastHeldAnim)
    pub cur_held_anim: i32,
    pub last_held_anim: i32,
    pub last_held_anim_fade: f32,
    pub held_animation_frame_count: f32,
    pub last_held_animation_frame_count: f32,
    pub held_frozen_rot_frame_count: f32,
    pub held_frozen_rot_frame_count_used: bool,
    pub future_held_stack: Vec<i32>,
}

impl Default for AnimDrawState {
    fn default() -> Self {
        Self {
            cur_anim: ANIM_GROUND,
            last_anim: ANIM_GROUND,
            last_anim_fade: 0.0,
            animation_frame_count: 0.0,
            last_animation_frame_count: 0.0,
            frozen_rot_frame_count: 0.0,
            frozen_rot_frame_count_used: false,
            extra_index: -1,
            extra_index_b: -1,
            future_stack: Vec::new(),
            cur_held_anim: ANIM_HELD,
            last_held_anim: ANIM_HELD,
            last_held_anim_fade: 0.0,
            held_animation_frame_count: 0.0,
            last_held_animation_frame_count: 0.0,
            held_frozen_rot_frame_count: 0.0,
            held_frozen_rot_frame_count_used: false,
            future_held_stack: Vec::new(),
        }
    }
}

impl AnimDrawState {
    /// C++ `addNewAnimPlayerOnly` — start cross-fade or stack while mid-fade.
    pub fn switch_to(&mut self, new_anim: i32, new_record: Option<&ObjectAnimation>) {
        if self.cur_anim == new_anim && self.future_stack.is_empty() {
            return;
        }
        // Mid-fade and not on ground: stack (C++ addNewAnimPlayerOnly).
        if self.last_anim_fade > 0.0
            && self.cur_anim != ANIM_GROUND
            && self.cur_anim != ANIM_GROUND2
        {
            if self.future_stack.last().copied() != Some(new_anim) {
                while matches!(
                    self.future_stack.last().copied(),
                    Some(ANIM_GROUND) | Some(ANIM_GROUND2)
                ) {
                    self.future_stack.pop();
                }
                self.future_stack.push(new_anim);
            }
            return;
        }
        self.switch_to_direct(new_anim, new_record);
    }

    /// C++ `addNewAnimDirect` without stack logic.
    pub fn switch_to_direct(&mut self, new_anim: i32, new_record: Option<&ObjectAnimation>) {
        self.last_anim = self.cur_anim;
        self.cur_anim = new_anim;
        self.last_anim_fade = 1.0;
        self.last_animation_frame_count = self.animation_frame_count;

        if self.last_anim == ANIM_MOVING {
            self.frozen_rot_frame_count = self.last_animation_frame_count;
            self.frozen_rot_frame_count_used = false;
        } else if self.cur_anim == ANIM_MOVING
            && self.last_anim != ANIM_MOVING
            && self.frozen_rot_frame_count_used
        {
            self.animation_frame_count = self.frozen_rot_frame_count;
        }

        reseed_timeline(
            &mut self.animation_frame_count,
            &mut self.last_animation_frame_count,
            new_record,
        );
    }

    /// Held-item anim switch (C++ held stack / `addNewHeldAnimDirect`).
    pub fn switch_held_to(&mut self, new_anim: i32, new_record: Option<&ObjectAnimation>) {
        if self.cur_held_anim == new_anim {
            return;
        }
        if self.last_held_anim_fade > 0.0 {
            if self.future_held_stack.last().copied() != Some(new_anim) {
                self.future_held_stack.push(new_anim);
            }
            return;
        }
        self.switch_held_to_direct(new_anim, new_record);
    }

    pub fn switch_held_to_direct(&mut self, new_anim: i32, new_record: Option<&ObjectAnimation>) {
        self.last_held_anim = self.cur_held_anim;
        self.cur_held_anim = new_anim;
        self.last_held_anim_fade = 1.0;
        self.last_held_animation_frame_count = self.held_animation_frame_count;

        if self.last_held_anim == ANIM_MOVING {
            self.held_frozen_rot_frame_count = self.last_held_animation_frame_count;
            self.held_frozen_rot_frame_count_used = false;
        } else if self.cur_held_anim == ANIM_MOVING
            && self.last_held_anim != ANIM_MOVING
            && self.held_frozen_rot_frame_count_used
        {
            self.held_animation_frame_count = self.held_frozen_rot_frame_count;
        }

        reseed_timeline(
            &mut self.held_animation_frame_count,
            &mut self.last_held_animation_frame_count,
            new_record,
        );
    }

    /// Advance frame counters + decay cross-fades.
    ///
    /// `anim_speed` ≈ C++ `lastSpeed / BASE_SPEED` per tick.
    /// `frame_rate_factor` multiplies the 0.05 fade step (usually 1.0).
    pub fn step(
        &mut self,
        anim_speed: f32,
        frame_rate_factor: f32,
        person_fade_needed: bool,
        held_fade_needed: bool,
    ) {
        let speed = anim_speed.max(0.0);
        self.animation_frame_count += speed;
        self.last_animation_frame_count += speed;
        if self.cur_anim == ANIM_MOVING {
            self.frozen_rot_frame_count += speed;
        }

        self.held_animation_frame_count += speed;
        self.last_held_animation_frame_count += speed;
        if self.cur_held_anim == ANIM_MOVING {
            self.held_frozen_rot_frame_count += speed;
        }

        // Person fade
        if self.last_anim_fade > 0.0 {
            if (self.last_anim_fade - 1.0).abs() < 1e-6 && !person_fade_needed {
                self.last_anim_fade = 0.0;
            } else {
                self.last_anim_fade -= ANIM_FADE_STEP * frame_rate_factor;
                if self.last_anim_fade < 0.0 {
                    self.last_anim_fade = 0.0;
                    if let Some(next) = self.future_stack.first().copied() {
                        self.future_stack.remove(0);
                        self.switch_to_direct(next, None);
                    }
                }
            }
        }

        // Held fade
        if self.last_held_anim_fade > 0.0 {
            if (self.last_held_anim_fade - 1.0).abs() < 1e-6 && !held_fade_needed {
                self.last_held_anim_fade = 0.0;
            } else {
                self.last_held_anim_fade -= ANIM_FADE_STEP * frame_rate_factor;
                if self.last_held_anim_fade < 0.0 {
                    self.last_held_anim_fade = 0.0;
                    if let Some(next) = self.future_held_stack.first().copied() {
                        self.future_held_stack.remove(0);
                        self.switch_held_to_direct(next, None);
                    }
                }
            }
        }
    }

    /// Convenience: step without fade-needed checks (always decay).
    pub fn step_simple(&mut self, anim_speed: f32, frame_rate_factor: f32) {
        self.step(anim_speed, frame_rate_factor, true, true);
    }

    /// Build person draw pack (C++ LivingLifePage draw path ~5349).
    pub fn person_pack(&self, object_id: i32, rideable_or_hide_arm: bool) -> ObjectAnimPack {
        let (anim_type, fade_target, anim_fade, frame_time, target_time) =
            if self.last_anim_fade > 0.0 {
                (
                    self.last_anim,
                    self.cur_anim,
                    self.last_anim_fade,
                    self.last_animation_frame_count / 60.0,
                    self.animation_frame_count / 60.0,
                )
            } else {
                (
                    self.cur_anim,
                    self.cur_anim,
                    1.0,
                    self.animation_frame_count / 60.0,
                    self.animation_frame_count / 60.0,
                )
            };

        let (frozen_arm, frozen_arm_target) =
            frozen_arm_types(rideable_or_hide_arm, anim_type, fade_target);

        ObjectAnimPack {
            object_id,
            anim_type,
            frame_time,
            anim_fade,
            fade_target_type: fade_target,
            fade_target_frame_time: target_time,
            frozen_rot_frame_time: self.frozen_rot_frame_count / 60.0,
            frozen_arm_type: frozen_arm,
            frozen_arm_fade_target_type: frozen_arm_target,
            extra_index: self.extra_index,
            extra_index_b: self.extra_index_b,
            frozen_rot_used: false,
        }
    }

    /// Build held-item draw pack.
    pub fn held_pack(&self, held_object_id: i32) -> ObjectAnimPack {
        let (anim_type, fade_target, anim_fade, frame_time, target_time) =
            if self.last_held_anim_fade > 0.0 {
                (
                    self.last_held_anim,
                    self.cur_held_anim,
                    self.last_held_anim_fade,
                    self.last_held_animation_frame_count / 60.0,
                    self.held_animation_frame_count / 60.0,
                )
            } else {
                (
                    self.cur_held_anim,
                    self.cur_held_anim,
                    1.0,
                    self.held_animation_frame_count / 60.0,
                    self.held_animation_frame_count / 60.0,
                )
            };

        ObjectAnimPack {
            object_id: held_object_id,
            anim_type,
            frame_time,
            anim_fade,
            fade_target_type: fade_target,
            fade_target_frame_time: target_time,
            frozen_rot_frame_time: self.held_frozen_rot_frame_count / 60.0,
            frozen_arm_type: ANIM_END,
            frozen_arm_fade_target_type: ANIM_END,
            extra_index: -1,
            extra_index_b: -1,
            frozen_rot_used: false,
        }
    }

    /// Sync desired person + held types from player flags (pack select).
    ///
    /// `emot_extra` is `Some((anim_type, extra_index))` when PE has a gesture
    /// (`ANIM_EXTRA` or `ANIM_EXTRA_B` + table index). Indices are written into
    /// the matching A/B slot by PE apply (C++ toggle); we only re-assert them.
    pub fn sync_from_player_state(
        &mut self,
        bank: &mut AnimBank,
        display_id: i32,
        held_id: i32,
        moving: bool,
        just_ate: bool,
        action: i32,
        emot_extra: Option<(i32, i32)>,
        holding_baby: bool,
        rideable_held: bool,
    ) {
        let desired = select_player_anim_type(moving, just_ate, action, emot_extra);
        if let Some((ex_type, ex_idx)) = emot_extra {
            if ex_type == ANIM_EXTRA_B {
                self.extra_index_b = ex_idx;
            } else {
                self.extra_index = ex_idx;
            }
        } else if !is_extra_anim_type(self.cur_anim) {
            // Keep A/B indices for next PE toggle (C++ does not clear them).
        }

        if desired != self.cur_anim {
            let ex_for = if desired == ANIM_EXTRA_B {
                self.extra_index_b.max(0)
            } else if desired == ANIM_EXTRA {
                self.extra_index.max(0)
            } else {
                0
            };
            let (force_zero, rand_start) = if is_extra_anim_type(desired) {
                bank.get_extra(display_id, ex_for)
                    .map(|r| (r.force_zero_start, r.rand_start_phase))
                    .unwrap_or((false, 0.0))
            } else {
                bank.get(display_id, desired)
                    .map(|r| (r.force_zero_start, r.rand_start_phase))
                    .unwrap_or((false, 0.0))
            };
            let stub = ObjectAnimation {
                object_id: display_id,
                anim_type: if is_extra_anim_type(desired) {
                    ANIM_EXTRA
                } else {
                    desired
                },
                force_zero_start: force_zero,
                rand_start_phase: rand_start,
                ..Default::default()
            };
            self.switch_to(desired, Some(&stub));
        }

        if held_id != 0 {
            let held_desired = select_held_anim_type(desired, holding_baby, rideable_held);
            if held_desired != self.cur_held_anim {
                let hid = held_id.abs();
                let (force_zero, rand_start) = bank
                    .get(hid, held_desired)
                    .map(|r| (r.force_zero_start, r.rand_start_phase))
                    .unwrap_or((false, 0.0));
                let stub = ObjectAnimation {
                    object_id: hid,
                    anim_type: held_desired,
                    force_zero_start: force_zero,
                    rand_start_phase: rand_start,
                    ..Default::default()
                };
                self.switch_held_to(held_desired, Some(&stub));
            }
        }
    }

    /// Mark frozen-rot used after sample so resume-to-moving works.
    pub fn note_frozen_rot_used(&mut self, person: bool, used: bool) {
        if used {
            if person {
                self.frozen_rot_frame_count_used = true;
            } else {
                self.held_frozen_rot_frame_count_used = true;
            }
        }
    }

    /// Snap cross-fade to done when poses already align (C++ first-step `isAnimFadeNeeded`).
    ///
    /// Call after `switch_to` / pack select so static offset switches do not wait a frame.
    pub fn maybe_skip_fades(
        &mut self,
        bank: &mut AnimBank,
        person_id: i32,
        held_id: i32,
    ) {
        if (self.last_anim_fade - 1.0).abs() < 1e-6
            && !is_anim_fade_needed(bank, person_id, self.last_anim, self.cur_anim)
        {
            self.last_anim_fade = 0.0;
        }
        if held_id > 0
            && (self.last_held_anim_fade - 1.0).abs() < 1e-6
            && !is_anim_fade_needed(
                bank,
                held_id,
                self.last_held_anim,
                self.cur_held_anim,
            )
        {
            self.last_held_anim_fade = 0.0;
        }
    }
}

/// Clothing dual-fade pack: same clocks as person, types remapped HELD/MOVING.
///
/// // C++: clothingAnimType tracks person moving vs held; mid-fade uses person fade weights
pub fn clothing_pack_from_person(person: &ObjectAnimPack, cloth_id: i32) -> ObjectAnimPack {
    ObjectAnimPack {
        object_id: cloth_id,
        anim_type: select_clothing_anim_type(person.anim_type),
        frame_time: person.frame_time,
        anim_fade: person.anim_fade,
        fade_target_type: select_clothing_anim_type(person.fade_target_type),
        fade_target_frame_time: person.fade_target_frame_time,
        frozen_rot_frame_time: person.frozen_rot_frame_time,
        frozen_arm_type: ANIM_END,
        frozen_arm_fade_target_type: ANIM_END,
        extra_index: -1,
        extra_index_b: -1,
        frozen_rot_used: false,
    }
}

/// Reseed frame counters when switching anim types.
///
/// C++: `forceZeroStart` → 0; `randomStartPhase` → rand 0..10000.
fn reseed_timeline(
    frame_count: &mut f32,
    last_frame_count: &mut f32,
    new_record: Option<&ObjectAnimation>,
) {
    let Some(r) = new_record else {
        return;
    };
    if r.force_zero_start {
        *frame_count = 0.0;
        *last_frame_count = 0.0;
    } else if r.rand_start_phase > 0.5 {
        // Deterministic stand-in for C++ randSource 0..10000 (stable for tests).
        let seed = (r.object_id.wrapping_mul(31).wrapping_add(r.anim_type) as u32)
            .wrapping_mul(1103515245)
            .wrapping_add(12345);
        let r0 = (seed >> 16) % 10001;
        *frame_count = r0 as f32;
        *last_frame_count = *frame_count;
    }
}

/// C++ arm freeze when riding / hideClosestArm==-2.
fn frozen_arm_types(
    rideable_or_hide_arm: bool,
    cur_type: i32,
    fade_target: i32,
) -> (i32, i32) {
    if !rideable_or_hide_arm {
        return (ANIM_END, ANIM_END);
    }
    let freeze = |t: i32| -> i32 {
        if t == ANIM_GROUND2 || t == ANIM_MOVING || is_extra_anim_type(t) {
            ANIM_MOVING
        } else {
            ANIM_END
        }
    };
    (freeze(cur_type), freeze(fade_target))
}

// ── Pack select from player state ────────────────────────────────────────────

/// Choose person `AnimType` from LiveObject-ish flags.
///
/// Priority: extra/emote → moving → eating → doing → ground.
///
/// `emot_extra` is `Some((ANIM_EXTRA|ANIM_EXTRA_B, index))` when PE has a gesture
/// (C++ toggles A/B on each PE apply so gestures can cross-fade).
///
/// // C++: addNewAnim(moving) on PM; eating on justAte; doing on actionAttempt
pub fn select_player_anim_type(
    moving: bool,
    just_ate: bool,
    action: i32,
    emot_extra: Option<(i32, i32)>,
) -> i32 {
    if let Some((ex_type, _)) = emot_extra {
        // Prefer explicit EXTRA/EXTRA_B; fall back to EXTRA if caller passed junk.
        return if is_extra_anim_type(ex_type) {
            ex_type
        } else {
            ANIM_EXTRA
        };
    }
    if moving {
        return ANIM_MOVING;
    }
    if just_ate {
        return ANIM_EATING;
    }
    if action != 0 {
        return ANIM_DOING;
    }
    ANIM_GROUND
}

/// Held-item type given the person's desired anim (C++ `addNewAnim` held branch).
///
/// Baby held → always `held`. Person ground/doing/eating → item stays `held`.
/// Person moving → item uses `moving`.
pub fn select_held_anim_type(
    person_desired: i32,
    holding_baby: bool,
    _rideable: bool,
) -> i32 {
    if holding_baby {
        return ANIM_HELD;
    }
    match person_desired {
        ANIM_GROUND | ANIM_GROUND2 | ANIM_DOING | ANIM_EATING => ANIM_HELD,
        other => other,
    }
}

/// Clothing worn by a person: moving when person moving, else held.
pub fn select_clothing_anim_type(person_anim_type: i32) -> i32 {
    if resolve_type(person_anim_type) == ANIM_MOVING {
        ANIM_MOVING
    } else {
        ANIM_HELD
    }
}

#[inline]
fn resolve_type(t: i32) -> i32 {
    if t == ANIM_GROUND2 {
        ANIM_GROUND
    } else {
        t
    }
}

// ── P3#22 action wiggle + baby-held handoff ──────────────────────────────────

/// Per-frame progress step for pending-action bounce (C++ `0.025 * frameRateFactor`).
pub const PENDING_ACTION_PROGRESS_INC: f32 = 0.025;

/// Per-frame baby-wiggle progress (C++ `0.04 * frameRateFactor`).
pub const BABY_WIGGLE_PROGRESS_INC: f32 = 0.04;

/// Drop-offset slide step in tile units (C++ `0.0625 * frameRateFactor`).
pub const DROP_OFFSET_STEP: f32 = 0.0625;

/// Held-pos handoff slide step (C++ `0.0625 * frameRateFactor`; long slides speed up).
pub const HELD_POS_SLIDE_STEP: f32 = 0.0625;

/// Held-rot handoff step (C++ `0.03125 * frameRateFactor`).
pub const HELD_ROT_SLIDE_STEP: f32 = 0.03125;

/// Max wiggle amplitude in object units (C++ `CELL_D * 0.5 * 0.90`).
pub const ACTION_WIGGLE_MAX_UNITS: f32 = 128.0 * 0.5 * 0.90;

/// Baby jump-out lateral amplitude in object units (C++ `8 * (cos…)`).
pub const BABY_WIGGLE_AMP_UNITS: f32 = 8.0;

/// Young-baby lying shift while drop settles (C++ `32` object units, rot 0.25).
pub const BABY_LIE_SHIFT_UNITS: f32 = 32.0;

/// Start progress when an action is first queued/flushed (C++ `0.025 * frameRateFactor`).
pub const PENDING_ACTION_START_PROGRESS: f32 = 0.025;

/// Advance `pendingActionAnimationProgress` one frame (C++ LivingLifePage step ~23138).
///
/// - Local (`is_ours`): while `pending_action || progress != 0`, wrap past 1 if still
///   pending, else snap to 0 after a full cycle.
/// - Remote: advance while progress != 0; snap to 0 after one cycle.
///
/// Returns the new progress value.
pub fn step_pending_action_progress(
    progress: f32,
    pending_action: bool,
    is_ours: bool,
    frame_rate_factor: f32,
) -> f32 {
    let inc = PENDING_ACTION_PROGRESS_INC * frame_rate_factor.max(0.0);
    if is_ours {
        if !pending_action && progress == 0.0 {
            return 0.0;
        }
        let mut p = progress + inc;
        if p > 1.0 {
            if pending_action {
                p -= 1.0;
            } else {
                p = 0.0;
            }
        }
        p
    } else if progress != 0.0 {
        let mut p = progress + inc;
        if p > 1.0 {
            p = 0.0;
        }
        p
    } else {
        0.0
    }
}

/// Person-position action wiggle toward the action target (C++ drawLiveObject ~5276–5346).
///
/// Offset is in **object units** (multiply by `scale` for screen). Returns (0,0) when
/// progress is 0, or when cur/last anim is eating.
///
/// `progress` ∈ (0,1]: cosine half-cycle from 0 → amp → 0 each full unit.
pub fn action_wiggle_offset_units(
    progress: f32,
    cur_x: f32,
    cur_y: f32,
    target_x: f32,
    target_y: f32,
    eating: bool,
) -> (f32, f32) {
    if progress == 0.0 || eating {
        return (0.0, 0.0);
    }
    let mut dx = target_x - cur_x;
    let mut dy = target_y - cur_y;
    let len = (dx * dx + dy * dy).sqrt();
    if len > 1e-6 {
        dx /= len;
        dy /= len;
    } else {
        // Standing on target: tiny downward bounce
        dx = 0.0;
        dy = -1.0;
    }
    let mut half = ACTION_WIGGLE_MAX_UNITS * 0.5;
    if dx == 0.0 && dy == -1.0 {
        // standing-on-target or pure-down: reduced amp (C++ halfWiggleMax *= 0.25 / 0.5)
        // Distinguish pure down (len was >0) vs zero dir: caller zero-dir uses 0.25.
        // We used dy=-1 for both; use original length.
        if len <= 1e-6 {
            half *= 0.25;
        } else {
            half *= 0.5;
        }
    }
    let offset = half - half * (2.0 * std::f32::consts::PI * progress).cos();
    (dx * offset, dy * offset)
}

/// Baby lateral wiggle while held (C++ ~5845–5862). Progress ∈ (0,1].
///
/// Cosine from π → 3π gives smooth start/finish; `holding_flip` mirrors X.
pub fn baby_wiggle_offset_x_units(progress: f32, holding_flip: bool) -> f32 {
    if progress <= 0.0 || progress > 1.0 {
        return 0.0;
    }
    let wiggle_dir = if holding_flip { -1.0 } else { 1.0 };
    let wave = (progress * 2.0 * std::f32::consts::PI + std::f32::consts::PI).cos() * 0.5 + 0.5;
    wiggle_dir * BABY_WIGGLE_AMP_UNITS * wave
}

/// Advance baby wiggle progress; returns `(active, progress)`. Completes at >1.
pub fn step_baby_wiggle(
    active: bool,
    progress: f32,
    frame_rate_factor: f32,
) -> (bool, f32) {
    if !active {
        return (false, 0.0);
    }
    let p = progress + BABY_WIGGLE_PROGRESS_INC * frame_rate_factor.max(0.0);
    if p > 1.0 {
        (false, 0.0)
    } else {
        (true, p)
    }
}

/// Slide `heldByDropOffset` toward (0,0) one step (C++ ~5211–5246).
///
/// Offset is in **tile** units. Returns `(ox, oy, landed)` where `landed` is true
/// when the offset snapped to zero this step (play put-down sound).
pub fn step_held_by_drop_offset(
    ox: f32,
    oy: f32,
    frame_rate_factor: f32,
) -> (f32, f32, bool) {
    if ox == 0.0 && oy == 0.0 {
        return (0.0, 0.0, false);
    }
    let step = DROP_OFFSET_STEP * frame_rate_factor.max(0.0);
    let len = (ox * ox + oy * oy).sqrt();
    // delta = (0,0) - offset → direction toward zero is -offset
    if len < step {
        return (0.0, 0.0, true);
    }
    let nx = ox / len;
    let ny = oy / len;
    (ox - nx * step, oy - ny * step, false)
}

/// Compute initial drop offset from last held raw pos → ground tile (C++ ~19269).
///
/// Both positions in **tile** units. Snaps to zero if distance > 3 tiles.
pub fn held_by_drop_offset_from_raw(last_held_x: f32, last_held_y: f32, ground_x: f32, ground_y: f32) -> (f32, f32) {
    let ox = last_held_x - ground_x;
    let oy = last_held_y - ground_y;
    if (ox * ox + oy * oy).sqrt() > 3.0 {
        (0.0, 0.0)
    } else {
        (ox, oy)
    }
}

/// One step of held-object handoff slide (C++ heldPosOverride ~5575–5633).
///
/// Positions in **tile** units; rot in turns (0..1). Returns
/// `(pos_x, pos_y, rot, almost_over, still_overriding, step_count)`.
pub fn step_held_pos_handoff(
    current_x: f32,
    current_y: f32,
    current_rot: f32,
    target_x: f32,
    target_y: f32,
    target_rot: f32,
    slide_step_count: i32,
    frame_rate_factor: f32,
    stationary: bool,
    override_active: bool,
    almost_over: bool,
) -> (f32, f32, f32, bool, bool, i32) {
    if !stationary || !override_active || almost_over {
        // Track target every frame when not sliding (C++ else branch).
        return (target_x, target_y, target_rot, false, false, 0);
    }
    if (current_x - target_x).abs() < 1e-6 && (current_y - target_y).abs() < 1e-6 {
        // Already there — mark almost-over so limbs can hide.
        return (target_x, target_y, target_rot, true, true, slide_step_count);
    }
    let fr = frame_rate_factor.max(0.0);
    let dx = target_x - current_x;
    let dy = target_y - current_y;
    let mut rot_delta = target_rot - current_rot;
    if rot_delta > 0.5 {
        rot_delta -= 1.0;
    } else if rot_delta < -0.5 {
        rot_delta += 1.0;
    }
    let slide_time = slide_step_count as f32 * fr;
    let long_mod = if slide_time > 30.0 {
        (slide_time / 30.0).powi(2)
    } else {
        1.0
    };
    let step = HELD_POS_SLIDE_STEP * fr * long_mod;
    let rot_step = HELD_ROT_SLIDE_STEP * fr;
    let len = (dx * dx + dy * dy).sqrt();
    let (nx, ny, almost) = if len < step {
        (target_x, target_y, true)
    } else {
        let ux = dx / len;
        let uy = dy / len;
        (current_x + ux * step, current_y + uy * step, false)
    };
    let nrot = if rot_delta.abs() < rot_step {
        target_rot
    } else {
        current_rot + rot_delta.signum() * rot_step
    };
    (nx, ny, nrot, almost, true, slide_step_count + 1)
}

// ── isAnimFadeNeeded ─────────────────────────────────────────────────────────

/// C++ `isAnimFadeNeeded` — skip cross-fade when poses already align at t=0.
pub fn is_anim_fade_needed(
    bank: &mut AnimBank,
    object_id: i32,
    cur_type: i32,
    target_type: i32,
) -> bool {
    if (cur_type != ANIM_GROUND2 && target_type == ANIM_GROUND2)
        || (cur_type == ANIM_GROUND2 && target_type != ANIM_GROUND2)
    {
        return true;
    }
    let Some(cur) = bank.get(object_id, cur_type).cloned() else {
        return false;
    };
    let Some(target) = bank.get(object_id, target_type).cloned() else {
        return false;
    };
    is_anim_fade_needed_records(&cur, &target)
}

pub fn is_anim_fade_needed_records(cur: &ObjectAnimation, target: &ObjectAnimation) -> bool {
    let n = cur.sprite_params.len().max(target.sprite_params.len());
    for i in 0..n {
        let c = cur.sprite_params.get(i).cloned().unwrap_or_default();
        let t = target.sprite_params.get(i).cloned().unwrap_or_default();
        if layer_needs_fade(&c, &t) {
            return true;
        }
    }
    let ns = cur.slot_params.len().max(target.slot_params.len());
    for i in 0..ns {
        let c = cur.slot_params.get(i).cloned().unwrap_or_default();
        let t = target.slot_params.get(i).cloned().unwrap_or_default();
        if slot_needs_fade(&c, &t) {
            return true;
        }
    }
    false
}

fn layer_needs_fade(c: &SpriteAnimParam, t: &SpriteAnimParam) -> bool {
    // Either side has a non-zero rest offset → poses differ at t=0.
    if c.offset_x != 0.0 || c.offset_y != 0.0 || t.offset_x != 0.0 || t.offset_y != 0.0 {
        return true;
    }
    if c.x_osc_per_sec > 0.0 || t.x_osc_per_sec > 0.0 {
        return true;
    }
    if c.x_amp != 0.0 && c.x_phase != 0.0 && c.x_phase != 0.5 {
        return true;
    }
    if c.y_osc_per_sec > 0.0 || t.y_osc_per_sec > 0.0 {
        return true;
    }
    if c.y_amp != 0.0 && c.y_phase != 0.0 && c.y_phase != 0.5 {
        return true;
    }
    if c.rock_osc_per_sec > 0.0 || t.rock_osc_per_sec > 0.0 {
        return true;
    }
    if c.rock_amp != 0.0 && c.rock_phase != 0.0 && c.rock_phase != 0.5 {
        return true;
    }
    if c.rot_per_sec != 0.0 || c.rot_phase != 0.0 {
        return true;
    }
    if c.fade_osc_per_sec > 0.0 || t.fade_osc_per_sec > 0.0 {
        return true;
    }
    if c.fade_max != 1.0 || c.fade_phase != 0.0 {
        return true;
    }
    if t.x_amp != 0.0 && t.x_phase != 0.0 && t.x_phase != 0.5 {
        return true;
    }
    if t.y_amp != 0.0 && t.y_phase != 0.0 && t.y_phase != 0.5 {
        return true;
    }
    if t.rock_amp != 0.0 && t.rock_phase != 0.0 && t.rock_phase != 0.5 {
        return true;
    }
    if t.rot_per_sec != 0.0 || t.rot_phase != 0.0 {
        return true;
    }
    if t.fade_max != 1.0 || t.fade_phase != 0.0 {
        return true;
    }
    false
}

fn slot_needs_fade(c: &SpriteAnimParam, t: &SpriteAnimParam) -> bool {
    if c.offset_x != 0.0 || c.offset_y != 0.0 {
        return true;
    }
    if c.x_osc_per_sec > 0.0 {
        return true;
    }
    if c.x_amp != 0.0 && c.x_phase != 0.0 && c.x_phase != 0.5 {
        return true;
    }
    if c.y_osc_per_sec > 0.0 {
        return true;
    }
    if c.y_amp != 0.0 && c.y_phase != 0.0 && c.y_phase != 0.5 {
        return true;
    }
    if t.x_amp != 0.0 && t.x_phase != 0.0 && t.x_phase != 0.5 {
        return true;
    }
    if t.y_amp != 0.0 && t.y_phase != 0.0 && t.y_phase != 0.5 {
        return true;
    }
    false
}

// ── Dual-anim sample ─────────────────────────────────────────────────────────

/// Sample one sprite layer through an `ObjectAnimPack` (dual-anim + frozen rot).
///
/// // C++: drawObjectAnim working pos/rot/fade blend (~2139–2500)
pub fn sample_sprite_pack(
    bank: &mut AnimBank,
    pack: &mut ObjectAnimPack,
    sprite_index: usize,
) -> AnimSample {
    sample_layer_pack(bank, pack, sprite_index, true)
}

/// Sample one container slot through an `ObjectAnimPack`.
pub fn sample_slot_pack(
    bank: &mut AnimBank,
    pack: &mut ObjectAnimPack,
    slot_index: usize,
) -> AnimSample {
    sample_layer_pack(bank, pack, slot_index, false)
}

fn sample_layer_pack(
    bank: &mut AnimBank,
    pack: &mut ObjectAnimPack,
    layer_index: usize,
    is_sprite: bool,
) -> AnimSample {
    // C++: setExtraIndex / setExtraIndexB — A for `extra`, B for `extraB`.
    let cur_extra = extra_index_for_type(pack, pack.anim_type);
    let tgt_extra = extra_index_for_type(pack, pack.fade_target_type);
    let cur_type = resolve_type(pack.anim_type);
    let tgt_type = resolve_type(pack.fade_target_type);

    let cur_rec = get_anim_clone(bank, pack.object_id, pack.anim_type, cur_extra);
    let tgt_rec = get_anim_clone(bank, pack.object_id, pack.fade_target_type, tgt_extra);
    let frozen_rec = get_anim_clone(bank, pack.object_id, ANIM_MOVING, -1);

    let cur_use = layer_param(cur_rec.as_ref(), layer_index, is_sprite);
    let tgt_use = layer_param(tgt_rec.as_ref(), layer_index, is_sprite);
    let froz_p = layer_param(frozen_rec.as_ref(), layer_index, is_sprite);

    let cur_phase = cur_rec.as_ref().map(|r| r.rand_start_phase).unwrap_or(0.0);
    let tgt_phase = tgt_rec.as_ref().map(|r| r.rand_start_phase).unwrap_or(0.0);

    let anim_fade = pack.anim_fade.clamp(0.0, 1.0);
    let target_w = 1.0 - anim_fade;

    // Frozen-arm full-layer override deferred (needs front/back arm indices from
    // objectBank). Pack fields `frozen_arm_type` are retained for limb-hide pass.

    let cur_ft = cur_use.frame_time(pack.frame_time);
    let tgt_ft = tgt_use.frame_time(pack.fade_target_frame_time);

    let cur_x = cur_use.offset_x
        + cur_use.x_amp * phase_sin(cur_use.x_osc_per_sec, cur_ft, cur_use.x_phase + cur_phase);
    let cur_y = cur_use.offset_y
        + cur_use.y_amp * phase_sin(cur_use.y_osc_per_sec, cur_ft, cur_use.y_phase + cur_phase);
    let cur_rock = cur_use.rock_amp
        * phase_sin(
            cur_use.rock_osc_per_sec,
            cur_ft,
            cur_use.rock_phase + cur_phase,
        );

    let mut x = anim_fade * cur_x;
    let mut y = anim_fade * cur_y;
    let mut rock = anim_fade * cur_rock;
    let mut rcx = anim_fade * cur_use.rot_center_x;
    let mut rcy = anim_fade * cur_use.rot_center_y;

    let mut fade = if (cur_use.fade_hardness - 1.0).abs() < 1e-6 {
        cur_use.sample_fade(cur_ft, cur_phase)
    } else {
        anim_fade * cur_use.sample_fade(cur_ft, cur_phase)
    };

    if anim_fade < 1.0 {
        let tgt_x = tgt_use.offset_x
            + tgt_use.x_amp * phase_sin(tgt_use.x_osc_per_sec, tgt_ft, tgt_use.x_phase + tgt_phase);
        let tgt_y = tgt_use.offset_y
            + tgt_use.y_amp * phase_sin(tgt_use.y_osc_per_sec, tgt_ft, tgt_use.y_phase + tgt_phase);
        let tgt_rock = tgt_use.rock_amp
            * phase_sin(
                tgt_use.rock_osc_per_sec,
                tgt_ft,
                tgt_use.rock_phase + tgt_phase,
            );
        x += target_w * tgt_x;
        y += target_w * tgt_y;
        rock += target_w * tgt_rock;
        rcx += target_w * tgt_use.rot_center_x;
        rcy += target_w * tgt_use.rot_center_y;

        let fade_b = tgt_use.sample_fade(tgt_ft, tgt_phase);
        if (tgt_use.fade_hardness - 1.0).abs() < 1e-6 {
            if target_w > 0.5 {
                fade = fade_b;
            }
        } else {
            fade += target_w * fade_b;
        }
    }

    // Rotation (with frozen-rot substitution)
    let mut total_rot = rot_offset_for_layer(
        &cur_use,
        cur_ft,
        cur_type,
        &froz_p,
        pack.frozen_rot_frame_time,
        &mut pack.frozen_rot_used,
    );

    if anim_fade < 1.0 {
        let from_moving = cur_type == ANIM_MOVING;
        let tgt_zero_rot = layer_rot_zero(&tgt_use);
        if from_moving && tgt_type != ANIM_MOVING && tgt_zero_rot && froz_p.rot_per_sec != 0.0 {
            total_rot = froz_p.rot_per_sec * pack.frozen_rot_frame_time + froz_p.rot_phase;
            pack.frozen_rot_used = true;
        }
    }

    let mut rot = wrap01(total_rot);

    if anim_fade < 1.0 {
        let mut tgt_total = rot_offset_for_layer(
            &tgt_use,
            tgt_ft,
            tgt_type,
            &froz_p,
            pack.frozen_rot_frame_time,
            &mut pack.frozen_rot_used,
        );
        if tgt_type != ANIM_MOVING && layer_rot_zero(&tgt_use) && froz_p.rot_per_sec != 0.0 {
            tgt_total = froz_p.rot_per_sec * pack.frozen_rot_frame_time + froz_p.rot_phase;
            pack.frozen_rot_used = true;
        }
        let tgt_rot = wrap01(tgt_total);
        // Shortest-path circular average (C++ offset-to-0.5 trick).
        let offset = 0.5 - rot;
        let c0 = offset + rot;
        let mut c1 = offset + tgt_rot;
        if c1 < 0.0 {
            c1 += 1.0;
        } else if c1 > 1.0 {
            c1 -= 1.0;
        }
        let ave = anim_fade * c0 + target_w * c1;
        rot = wrap01(ave - offset);
    }

    rot += rock;

    let _ = pack.frozen_arm_type; // retained for limb-hide pass
    let _ = pack.frozen_arm_fade_target_type;

    AnimSample {
        x,
        y,
        rot,
        fade,
        rot_center_x: rcx,
        rot_center_y: rcy,
    }
}

fn rot_offset_for_layer(
    layer: &SpriteAnimParam,
    frame_time: f32,
    anim_type: i32,
    frozen: &SpriteAnimParam,
    frozen_time: f32,
    frozen_used: &mut bool,
) -> f32 {
    if anim_type != ANIM_MOVING && layer_rot_zero(layer) && frozen.rot_per_sec != 0.0 {
        *frozen_used = true;
        return frozen.rot_per_sec * frozen_time + frozen.rot_phase;
    }
    layer.rot_phase + layer.rot_per_sec * frame_time
}

fn layer_rot_zero(p: &SpriteAnimParam) -> bool {
    p.rot_per_sec == 0.0
        && p.rot_phase == 0.0
        && p.rock_osc_per_sec == 0.0
        && p.rock_phase == 0.0
}

fn wrap01(v: f32) -> f32 {
    let mut r = v - v.floor();
    if r < 0.0 {
        r += 1.0;
    }
    r
}

fn phase_sin(osc_per_sec: f32, t: f32, phase: f32) -> f32 {
    if osc_per_sec.abs() < 1e-8 && phase.abs() < 1e-8 {
        return 0.0;
    }
    ((osc_per_sec * t + phase) * std::f32::consts::TAU).sin()
}

fn layer_param(rec: Option<&ObjectAnimation>, idx: usize, is_sprite: bool) -> SpriteAnimParam {
    rec.and_then(|r| {
        if is_sprite {
            r.sprite_params.get(idx).cloned()
        } else {
            r.slot_params.get(idx).cloned()
        }
    })
    .unwrap_or_default()
}

/// C++ `setExtraIndex` vs `setExtraIndexB` for dual-fade EXTRA↔EXTRA_B.
#[inline]
fn extra_index_for_type(pack: &ObjectAnimPack, anim_type: i32) -> i32 {
    if anim_type == ANIM_EXTRA_B {
        pack.extra_index_b
    } else {
        pack.extra_index
    }
}

fn get_anim_clone(
    bank: &mut AnimBank,
    object_id: i32,
    anim_type: i32,
    extra_index: i32,
) -> Option<ObjectAnimation> {
    if anim_type == ANIM_END {
        return None;
    }
    if is_extra_anim_type(anim_type) {
        bank.get_ex(object_id, ANIM_EXTRA, extra_index.max(0))
            .cloned()
    } else {
        bank.get(object_id, anim_type).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim_bank::SpriteAnimParam;

    #[test]
    fn pack_select_moving_eating_doing_ground() {
        assert_eq!(
            select_player_anim_type(true, true, 1, None),
            ANIM_MOVING,
            "moving wins over eating"
        );
        assert_eq!(select_player_anim_type(false, true, 1, None), ANIM_EATING);
        assert_eq!(select_player_anim_type(false, false, 1, None), ANIM_DOING);
        assert_eq!(select_player_anim_type(false, false, 0, None), ANIM_GROUND);
        assert_eq!(
            select_player_anim_type(true, false, 0, Some((ANIM_EXTRA, 3))),
            ANIM_EXTRA,
            "emote extra wins"
        );
        assert_eq!(
            select_player_anim_type(false, false, 0, Some((ANIM_EXTRA_B, 2))),
            ANIM_EXTRA_B,
            "extraB gesture wins"
        );
    }

    #[test]
    fn extra_b_pack_uses_index_b_for_sample() {
        let mut bank = AnimBank::new(".");
        bank.insert(ObjectAnimation {
            object_id: 50,
            anim_type: ANIM_EXTRA,
            extra_index: 0,
            sprite_params: vec![SpriteAnimParam {
                offset_x: 5.0,
                fade_max: 1.0,
                ..Default::default()
            }],
            ..Default::default()
        });
        bank.insert(ObjectAnimation {
            object_id: 50,
            anim_type: ANIM_EXTRA,
            extra_index: 1,
            sprite_params: vec![SpriteAnimParam {
                offset_x: 20.0,
                fade_max: 1.0,
                ..Default::default()
            }],
            ..Default::default()
        });
        // Slot B index 1
        let mut pack = ObjectAnimPack {
            object_id: 50,
            anim_type: ANIM_EXTRA_B,
            frame_time: 0.0,
            anim_fade: 1.0,
            fade_target_type: ANIM_EXTRA_B,
            fade_target_frame_time: 0.0,
            frozen_rot_frame_time: 0.0,
            frozen_arm_type: ANIM_END,
            frozen_arm_fade_target_type: ANIM_END,
            extra_index: 0,
            extra_index_b: 1,
            frozen_rot_used: false,
        };
        let s = sample_sprite_pack(&mut bank, &mut pack, 0);
        assert!(
            (s.x - 20.0).abs() < 1e-4,
            "EXTRA_B must sample extra_index_b=1 (got x={})",
            s.x
        );
    }

    #[test]
    fn held_pack_select() {
        assert_eq!(
            select_held_anim_type(ANIM_GROUND, false, false),
            ANIM_HELD
        );
        assert_eq!(
            select_held_anim_type(ANIM_MOVING, false, false),
            ANIM_MOVING
        );
        assert_eq!(
            select_held_anim_type(ANIM_MOVING, true, false),
            ANIM_HELD,
            "baby always held"
        );
        assert_eq!(
            select_held_anim_type(ANIM_EATING, false, false),
            ANIM_HELD
        );
        assert_eq!(select_clothing_anim_type(ANIM_MOVING), ANIM_MOVING);
        assert_eq!(select_clothing_anim_type(ANIM_GROUND), ANIM_HELD);
    }

    #[test]
    fn switch_starts_fade_and_frozen_rot() {
        let mut s = AnimDrawState::default();
        s.cur_anim = ANIM_MOVING;
        s.animation_frame_count = 120.0;
        s.switch_to_direct(ANIM_GROUND, None);
        assert_eq!(s.cur_anim, ANIM_GROUND);
        assert_eq!(s.last_anim, ANIM_MOVING);
        assert!((s.last_anim_fade - 1.0).abs() < 1e-6);
        assert!((s.frozen_rot_frame_count - 120.0).abs() < 1e-4);
        assert!(!s.frozen_rot_frame_count_used);

        s.frozen_rot_frame_count_used = true;
        s.switch_to_direct(ANIM_MOVING, None);
        assert!((s.animation_frame_count - 120.0).abs() < 1e-4);
    }

    #[test]
    fn force_zero_reseed() {
        let mut s = AnimDrawState::default();
        s.animation_frame_count = 50.0;
        let rec = ObjectAnimation {
            force_zero_start: true,
            ..Default::default()
        };
        s.switch_to_direct(ANIM_EATING, Some(&rec));
        assert!((s.animation_frame_count - 0.0).abs() < 1e-6);
    }

    #[test]
    fn dual_anim_blends_offsets() {
        let mut bank = AnimBank::new(".");
        bank.insert(ObjectAnimation {
            object_id: 99,
            anim_type: ANIM_MOVING,
            sprite_params: vec![SpriteAnimParam {
                offset_x: 10.0,
                fade_max: 1.0,
                ..Default::default()
            }],
            ..Default::default()
        });
        bank.insert(ObjectAnimation {
            object_id: 99,
            anim_type: ANIM_GROUND,
            sprite_params: vec![SpriteAnimParam {
                offset_x: 0.0,
                fade_max: 1.0,
                ..Default::default()
            }],
            ..Default::default()
        });

        let mut pack = ObjectAnimPack {
            object_id: 99,
            anim_type: ANIM_MOVING,
            frame_time: 0.0,
            anim_fade: 0.5,
            fade_target_type: ANIM_GROUND,
            fade_target_frame_time: 0.0,
            frozen_rot_frame_time: 0.0,
            frozen_arm_type: ANIM_END,
            frozen_arm_fade_target_type: ANIM_END,
            extra_index: -1,
            extra_index_b: -1,
            frozen_rot_used: false,
        };
        let s = sample_sprite_pack(&mut bank, &mut pack, 0);
        assert!(
            (s.x - 5.0).abs() < 1e-3,
            "0.5*10 + 0.5*0 = 5, got {}",
            s.x
        );
    }

    #[test]
    fn frozen_rot_when_target_zero() {
        let mut bank = AnimBank::new(".");
        bank.insert(ObjectAnimation {
            object_id: 50,
            anim_type: ANIM_GROUND,
            sprite_params: vec![SpriteAnimParam {
                fade_max: 1.0,
                ..Default::default()
            }],
            ..Default::default()
        });
        bank.insert(ObjectAnimation {
            object_id: 50,
            anim_type: ANIM_MOVING,
            sprite_params: vec![SpriteAnimParam {
                rot_per_sec: 1.0,
                rot_phase: 0.25,
                fade_max: 1.0,
                ..Default::default()
            }],
            ..Default::default()
        });

        let mut pack = ObjectAnimPack {
            object_id: 50,
            anim_type: ANIM_GROUND,
            frame_time: 0.0,
            anim_fade: 1.0,
            fade_target_type: ANIM_GROUND,
            fade_target_frame_time: 0.0,
            frozen_rot_frame_time: 2.0,
            frozen_arm_type: ANIM_END,
            frozen_arm_fade_target_type: ANIM_END,
            extra_index: -1,
            extra_index_b: -1,
            frozen_rot_used: false,
        };
        let s = sample_sprite_pack(&mut bank, &mut pack, 0);
        assert!(pack.frozen_rot_used);
        assert!(
            (s.rot - 0.25).abs() < 1e-3,
            "frozen rot wrap expected 0.25, got {}",
            s.rot
        );
    }

    #[test]
    fn person_pack_from_state() {
        let mut s = AnimDrawState::default();
        s.cur_anim = ANIM_MOVING;
        s.animation_frame_count = 60.0;
        let p = s.person_pack(19, false);
        assert_eq!(p.anim_type, ANIM_MOVING);
        assert!((p.frame_time - 1.0).abs() < 1e-4);
        assert!((p.anim_fade - 1.0).abs() < 1e-6);

        s.switch_to_direct(ANIM_GROUND, None);
        let p2 = s.person_pack(19, false);
        assert_eq!(p2.anim_type, ANIM_MOVING);
        assert_eq!(p2.fade_target_type, ANIM_GROUND);
        assert!(p2.anim_fade > 0.9);
    }

    #[test]
    fn fade_needed_detects_motion() {
        let cur = ObjectAnimation {
            sprite_params: vec![SpriteAnimParam {
                x_osc_per_sec: 1.0,
                x_amp: 5.0,
                ..Default::default()
            }],
            ..Default::default()
        };
        let tgt = ObjectAnimation {
            sprite_params: vec![SpriteAnimParam::default()],
            ..Default::default()
        };
        assert!(is_anim_fade_needed_records(&cur, &tgt));
        assert!(!is_anim_fade_needed_records(&tgt, &tgt));
    }

    #[test]
    fn step_decays_fade() {
        let mut s = AnimDrawState::default();
        s.last_anim_fade = 1.0;
        s.step_simple(1.0, 1.0);
        assert!((s.last_anim_fade - 0.95).abs() < 1e-4);
        for _ in 0..30 {
            s.step_simple(1.0, 1.0);
        }
        assert_eq!(s.last_anim_fade, 0.0);
    }

    #[test]
    fn clothing_pack_tracks_person_fade() {
        let person = ObjectAnimPack {
            object_id: 19,
            anim_type: ANIM_MOVING,
            frame_time: 1.0,
            anim_fade: 0.4,
            fade_target_type: ANIM_GROUND,
            fade_target_frame_time: 2.0,
            frozen_rot_frame_time: 0.5,
            frozen_arm_type: ANIM_END,
            frozen_arm_fade_target_type: ANIM_END,
            extra_index: -1,
            extra_index_b: -1,
            frozen_rot_used: false,
        };
        let c = clothing_pack_from_person(&person, 55);
        assert_eq!(c.object_id, 55);
        assert_eq!(c.anim_type, ANIM_MOVING);
        assert_eq!(c.fade_target_type, ANIM_HELD); // ground → clothing held
        assert!((c.anim_fade - 0.4).abs() < 1e-6);
        assert!((c.frame_time - 1.0).abs() < 1e-6);
    }

    #[test]
    fn switch_to_mid_fade_stacks_and_step_pops() {
        let mut s = AnimDrawState::default();
        s.cur_anim = ANIM_MOVING;
        s.last_anim_fade = 0.0;
        s.switch_to_direct(ANIM_EATING, None);
        assert!((s.last_anim_fade - 1.0).abs() < 1e-6);
        // Mid-fade non-ground: stack doing
        s.switch_to(ANIM_DOING, None);
        assert_eq!(s.future_stack, vec![ANIM_DOING]);
        assert_eq!(s.cur_anim, ANIM_EATING);
        // Decay fade past 0 → pop stack into switch_to_direct (fade restarts at 1)
        s.last_anim_fade = ANIM_FADE_STEP * 0.5; // almost done
        s.step_simple(1.0, 1.0);
        assert_eq!(s.cur_anim, ANIM_DOING);
        assert_eq!(s.last_anim, ANIM_EATING);
        assert!(s.future_stack.is_empty());
        assert!((s.last_anim_fade - 1.0).abs() < 1e-6);
    }

    #[test]
    fn sync_selects_moving_and_eating() {
        let mut bank = AnimBank::new(".");
        bank.insert(ObjectAnimation {
            object_id: 19,
            anim_type: ANIM_MOVING,
            sprite_params: vec![SpriteAnimParam::default()],
            ..Default::default()
        });
        bank.insert(ObjectAnimation {
            object_id: 19,
            anim_type: ANIM_EATING,
            force_zero_start: true,
            sprite_params: vec![SpriteAnimParam::default()],
            ..Default::default()
        });
        bank.insert(ObjectAnimation {
            object_id: 19,
            anim_type: ANIM_GROUND,
            sprite_params: vec![SpriteAnimParam::default()],
            ..Default::default()
        });

        let mut s = AnimDrawState::default();
        s.sync_from_player_state(&mut bank, 19, 0, true, false, 0, None, false, false);
        assert_eq!(s.cur_anim, ANIM_MOVING);
        // Clear mid-fade so next select applies directly (not future stack).
        s.last_anim_fade = 0.0;

        s.sync_from_player_state(&mut bank, 19, 0, false, true, 0, None, false, false);
        assert_eq!(s.cur_anim, ANIM_EATING);
        assert!((s.animation_frame_count - 0.0).abs() < 1e-6); // forceZero
    }

    // ── P3#22 action wiggle + baby-held handoff ──────────────────────────────

    #[test]
    fn action_wiggle_zero_when_idle_or_eating() {
        assert_eq!(
            action_wiggle_offset_units(0.0, 0.0, 0.0, 1.0, 0.0, false),
            (0.0, 0.0)
        );
        assert_eq!(
            action_wiggle_offset_units(0.5, 0.0, 0.0, 1.0, 0.0, true),
            (0.0, 0.0)
        );
    }

    #[test]
    fn action_wiggle_peaks_mid_cycle_toward_target() {
        // progress=0.5 → cos(π)=-1 → offset = half - half*(-1) = 2*half = max
        let (ox, oy) = action_wiggle_offset_units(0.5, 0.0, 0.0, 2.0, 0.0, false);
        assert!(ox > 0.0, "wiggle toward +X target, got {ox}");
        assert!(oy.abs() < 1e-4, "no Y component, got {oy}");
        // peak amplitude = ACTION_WIGGLE_MAX_UNITS * 0.5 * 2 = ACTION_WIGGLE_MAX_UNITS
        assert!((ox - ACTION_WIGGLE_MAX_UNITS).abs() < 1e-3, "peak {ox}");
        // start of cycle ≈ 0
        let (ox0, _) = action_wiggle_offset_units(0.001, 0.0, 0.0, 2.0, 0.0, false);
        assert!(ox0 < ox * 0.05, "near-zero at start {ox0}");
    }

    #[test]
    fn step_pending_action_progress_wraps_when_pending() {
        let mut p = PENDING_ACTION_START_PROGRESS;
        // Simulate until wrap: from 0.025, need (1-0.025)/0.025 = 39 steps to exceed 1
        for _ in 0..40 {
            p = step_pending_action_progress(p, true, true, 1.0);
        }
        assert!(p > 0.0 && p < 1.0, "wrapped smoothly while pending: {p}");
        // Finish cycle when no longer pending
        p = 0.99;
        p = step_pending_action_progress(p, false, true, 1.0);
        assert_eq!(p, 0.0, "snaps to 0 after cycle when not pending");
    }

    #[test]
    fn baby_wiggle_offset_mirrors_with_flip() {
        let a = baby_wiggle_offset_x_units(0.5, false);
        let b = baby_wiggle_offset_x_units(0.5, true);
        assert!((a + b).abs() < 1e-4, "flip mirrors: {a} vs {b}");
        assert!(a > 0.0);
        assert_eq!(baby_wiggle_offset_x_units(0.0, false), 0.0);
        assert_eq!(baby_wiggle_offset_x_units(1.1, false), 0.0);
    }

    #[test]
    fn step_baby_wiggle_completes() {
        let (active, p) = step_baby_wiggle(true, 0.0, 1.0);
        assert!(active);
        assert!((p - BABY_WIGGLE_PROGRESS_INC).abs() < 1e-6);
        let (active, p) = step_baby_wiggle(true, 0.99, 1.0);
        assert!(!active);
        assert_eq!(p, 0.0);
    }

    #[test]
    fn held_by_drop_offset_snaps_far_and_steps_near() {
        let (ox, oy) = held_by_drop_offset_from_raw(10.0, 10.0, 0.0, 0.0);
        assert_eq!((ox, oy), (0.0, 0.0), "far drop snaps");
        let (ox, oy) = held_by_drop_offset_from_raw(1.0, 0.0, 0.0, 0.0);
        assert!((ox - 1.0).abs() < 1e-6);
        let (nx, ny, landed) = step_held_by_drop_offset(ox, oy, 1.0);
        assert!(!landed);
        assert!(nx < ox && nx > 0.0);
        assert_eq!(ny, 0.0);
        // Finish
        let (nx, ny, landed) = step_held_by_drop_offset(0.01, 0.0, 1.0);
        assert!(landed);
        assert_eq!((nx, ny), (0.0, 0.0));
    }

    #[test]
    fn held_pos_handoff_slides_toward_target() {
        let (x, y, rot, almost, active, steps) = step_held_pos_handoff(
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0, 1.0, true, true, false,
        );
        assert!(active);
        assert!(!almost);
        assert!(x > 0.0 && x < 1.0);
        assert_eq!(y, 0.0);
        assert_eq!(rot, 0.0);
        assert_eq!(steps, 1);
        // Non-stationary clears override tracking to target
        let (x, y, _, almost, active, _) = step_held_pos_handoff(
            0.0, 0.0, 0.0, 1.0, 0.5, 0.0, 5, 1.0, false, true, false,
        );
        assert!(!active);
        assert!(!almost);
        assert!((x - 1.0).abs() < 1e-6 && (y - 0.5).abs() < 1e-6);
    }
}
