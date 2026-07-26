//! Live player body owned by the sim.

use crate::move_path::MovePath;
use serde::Serialize;
use std::collections::VecDeque;

/// Max object ids storable in [`Player::backpack`] (SAY STORE / TAKE / INV).
pub const BACKPACK_MAX: usize = 8;

/// Max personal journal notes on a player (`SAY NOTE` / `?NOTES` / `REMEMBER` / `?MEMORY`).
pub const NOTES_MAX: usize = 5;

/// Max characters stored per note (excess is truncated).
pub const NOTE_TEXT_MAX: usize = 80;

/// Max characters for personal title (`SAY TITLE`; excess is truncated).
pub const TITLE_TEXT_MAX: usize = 40;

/// Wearable body slots (object id; 0 = empty).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClothingSlot {
    Hat,
    Chest,
    Shoes,
}

impl ClothingSlot {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hat => "hat",
            Self::Chest => "chest",
            Self::Shoes => "shoes",
        }
    }

    /// Parse slot name (`hat` / `chest` / `shoes`), case-insensitive.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "hat" => Some(Self::Hat),
            "chest" => Some(Self::Chest),
            "shoes" | "shoe" => Some(Self::Shoes),
            _ => None,
        }
    }
}

/// Infer clothing slot from object name / description (case-insensitive).
///
/// - name contains `"hat"` → hat
/// - name contains `"shoe"` → shoes
/// - name contains `"chest"`, `"shirt"`, or `"tunic"` → chest
/// - description contains `clothing=` → treat as wearable; slot from name, else chest
pub fn clothing_slot_for_object(name: &str, description: &str) -> Option<ClothingSlot> {
    let n = name.to_ascii_lowercase();
    if n.contains("hat") {
        return Some(ClothingSlot::Hat);
    }
    if n.contains("shoe") {
        return Some(ClothingSlot::Shoes);
    }
    if n.contains("chest") || n.contains("shirt") || n.contains("tunic") {
        return Some(ClothingSlot::Chest);
    }
    // Content may embed clothing=N; without a full clothing table, mark wearable
    // and default to chest unless name already matched above.
    let d = description.to_ascii_lowercase();
    if d.contains("clothing=") || n.contains("clothing=") {
        return Some(ClothingSlot::Chest);
    }
    None
}

#[derive(Debug, Clone)]
pub struct Player {
    pub p_id: i32,
    pub conn_id: u64,
    pub email: String,
    pub x: i32,
    pub y: i32,
    /// Absolute world birth origin (Haxe `gx`/`gy`, vanilla `birthPos`).
    /// Client wire coords are relative: `world = client + birth`.
    pub birth_x: i32,
    pub birth_y: i32,
    pub held_id: i32,
    pub food: f32,
    pub food_max: f32,
    pub age: f32,
    pub deleted: bool,
    pub connected: bool,
    pub moving: bool,
    pub death_reason: Option<String>,
    /// Prefer last-use transition table on next USE (Haxe multi-use last).
    pub force_last_use: bool,
    /// Haxe `done_moving_seqNum` — >0 means stationary; set to path.seq on complete/cancel.
    pub done_moving_seq: i32,
    /// Active timed path (Haxe `newMoves`); `Some` ⇔ [`Self::moving`].
    pub move_path: Option<MovePath>,
    /// Center of last MAP_CHUNK sent (Haxe sendMapChunkIfNeeded).
    pub last_mc_x: i32,
    pub last_mc_y: i32,
    /// Whether any MC was sent for this life.
    pub has_mc: bool,
    pub tools: crate::tools::ToolSlots,
    pub yum: crate::yum::YumState,
    /// Display first/last name for NM packet.
    pub first_name: String,
    pub family_name: String,
    /// Person object id on the wire (`po_id` in PU) — skin/body. Default 19.
    pub display_object_id: i32,
    /// Accumulates sim time toward BW/DY emit while starving infant (age&lt;3, food&lt;5).
    pub vitals_emit_timer: f32,
    /// Accumulates sim time toward PE hunger emote while food&lt;3.
    pub hunger_emot_timer: f32,
    /// Accumulates sim time toward PE sleep/snore emote while [`Self::sleeping`].
    pub sleep_emot_timer: f32,
    /// Personal home tile (SAY HOME / GOHOME).
    pub home_x: i32,
    pub home_y: i32,
    /// Clothing slots: object ids, 0 = empty.
    pub hat: i32,
    pub chest: i32,
    pub shoes: i32,
    /// True after SAY SLEEP until SAY WAKE; blocks MOVE and halves food drain.
    pub sleeping: bool,
    /// True after SAY SIT until SAY STAND; blocks MOVE and mildly reduces food drain.
    pub sitting: bool,
    /// True after SAY SICK until SAY CURE; multiplies food drain and sets DY isSick.
    pub sick: bool,
    /// True after SAY RIDE / MOUNT until SAY DISMOUNT; move-speed is noted only (no MOVE change).
    pub riding: bool,
    /// Godmode flag (SAY GODMODE). Enables lite god edits (`SAY VOGSET`); no invuln yet.
    pub godmode: bool,
    /// When true (`SAY DEAF` toggle), ignore normal/shout/mumble chat PS; whispers still deliver.
    pub deaf: bool,
    /// Sim-time timestamps of recent SAY intents (sliding window rate limit).
    pub last_say_times: VecDeque<f32>,
    /// PE / EMOTE rate limit (separate from SAY; max 3 / 10s).
    pub emote_rate: crate::emote_limit::EmoteRateLimiter,
    /// Pending coin trade offer: `(target_p_id, amount)` from SAY TRADE; cleared on ACCEPT.
    pub trade_offer: Option<(i32, i32)>,
    /// Personal item storage (SAY STORE / TAKE / INV); max [`BACKPACK_MAX`].
    pub backpack: Vec<i32>,
    /// Personal journal lines (`SAY NOTE` / `?NOTES` / `REMEMBER` / `?MEMORY`); max [`NOTES_MAX`].
    pub notes: Vec<String>,
    /// Optional personal title (`SAY TITLE`); shown in `SAY ?NAME` when non-empty.
    pub title: String,
    /// Baby `p_id` currently held (0 = none). Haxe baby-carrying.
    pub holding_player_id: i32,
    /// Mother's `p_id` when this player is held as a baby (0 = none).
    pub held_by: i32,
    /// Sim-time of last successful lite profession action
    /// (`HARVEST` / `FISH` / `MINE` / `DIG` / `CHOP`).
    /// Initialized negative so the first action is not on cooldown.
    pub last_prof_action_time: f32,
}

impl Player {
    pub fn new(p_id: i32, conn_id: u64, email: &str) -> Self {
        // Placeholders; `spawn_player` assigns via `naming::pick_random_name`.
        Self {
            p_id,
            conn_id,
            email: email.to_string(),
            x: 0,
            y: 0,
            birth_x: 0,
            birth_y: 0,
            held_id: 0,
            food: 10.0,
            food_max: 20.0,
            age: 14.0,
            deleted: false,
            connected: true,
            moving: false,
            death_reason: None,
            force_last_use: false,
            done_moving_seq: 1,
            move_path: None,
            last_mc_x: 0,
            last_mc_y: 0,
            has_mc: false,
            tools: crate::tools::ToolSlots::default(),
            yum: crate::yum::YumState::default(),
            first_name: "NEWBORN".into(),
            family_name: "SNOW".into(),
            display_object_id: 19,
            vitals_emit_timer: 0.0,
            hunger_emot_timer: 0.0,
            sleep_emot_timer: 0.0,
            home_x: 0,
            home_y: 0,
            hat: 0,
            chest: 0,
            shoes: 0,
            sleeping: false,
            sitting: false,
            sick: false,
            riding: false,
            godmode: false,
            deaf: false,
            last_say_times: VecDeque::new(),
            emote_rate: crate::emote_limit::EmoteRateLimiter::default(),
            trade_offer: None,
            backpack: Vec::new(),
            notes: Vec::new(),
            title: String::new(),
            holding_player_id: 0,
            held_by: 0,
            last_prof_action_time: -crate::professions::PROF_ACTION_COOLDOWN_SECS,
        }
    }

    /// Whether this player can pick up a baby (age ≥ 14, free hands, not already holding).
    pub fn can_hold_baby(&self) -> bool {
        self.age >= 14.0
            && !self.deleted
            && self.holding_player_id == 0
            && self.held_id == 0
    }

    /// Begin holding baby `baby_p_id` (caller must also set the baby's `held_by`).
    pub fn start_holding(&mut self, baby_p_id: i32) {
        self.holding_player_id = baby_p_id;
    }

    /// Release held baby; returns baby `p_id` or 0 if none.
    pub fn release_holding(&mut self) -> i32 {
        let id = self.holding_player_id;
        self.holding_player_id = 0;
        id
    }

    pub fn display_name(&self) -> String {
        format!("{} {}", self.first_name, self.family_name)
    }

    /// Display used by `SAY ?NAME` / `SAY NAME`: `first last` or `first last | title`.
    pub fn name_for_query(&self) -> String {
        if self.title.is_empty() {
            self.display_name()
        } else {
            format!("{} | {}", self.display_name(), self.title)
        }
    }

    /// Set personal title (`SAY TITLE <text>`). Trimmed; empty fails with `"EMPTY"`.
    /// Returns the stored title (truncated to [`TITLE_TEXT_MAX`]).
    pub fn set_title(&mut self, text: &str) -> Result<&str, &'static str> {
        let mut t = text.trim().to_string();
        if t.is_empty() {
            return Err("EMPTY");
        }
        if t.chars().count() > TITLE_TEXT_MAX {
            t = t.chars().take(TITLE_TEXT_MAX).collect();
        }
        self.title = t;
        Ok(self.title.as_str())
    }

    /// Assign an object id to a clothing slot (`id == 0` clears).
    pub fn set_clothing(&mut self, slot: ClothingSlot, id: i32) {
        match slot {
            ClothingSlot::Hat => self.hat = id,
            ClothingSlot::Chest => self.chest = id,
            ClothingSlot::Shoes => self.shoes = id,
        }
    }

    /// Object id currently in slot (0 = empty).
    pub fn clothing(&self, slot: ClothingSlot) -> i32 {
        match slot {
            ClothingSlot::Hat => self.hat,
            ClothingSlot::Chest => self.chest,
            ClothingSlot::Shoes => self.shoes,
        }
    }

    /// Chat body for SAY CLOTHES (without leading p_id).
    pub fn clothes_report(&self) -> String {
        format!(
            "CLOTHES hat={} chest={} shoes={}",
            self.hat, self.chest, self.shoes
        )
    }

    /// Chat body for SAY INV (without leading p_id): `INV n/8 [id …]`.
    pub fn inv_report(&self) -> String {
        if self.backpack.is_empty() {
            format!("INV 0/{BACKPACK_MAX}")
        } else {
            let ids = self
                .backpack
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            format!("INV {}/{BACKPACK_MAX} {ids}", self.backpack.len())
        }
    }

    /// Chat body for `SAY ?NOTES` (without leading p_id): `NOTES n/5 [i:text; …]`.
    pub fn notes_report(&self) -> String {
        if self.notes.is_empty() {
            format!("NOTES 0/{NOTES_MAX}")
        } else {
            let parts = self
                .notes
                .iter()
                .enumerate()
                .map(|(i, t)| format!("{i}:{t}"))
                .collect::<Vec<_>>()
                .join("; ");
            format!("NOTES {}/{NOTES_MAX} {parts}", self.notes.len())
        }
    }

    /// Append a personal journal note (`SAY NOTE <text>` / `SAY REMEMBER <text>`).
    ///
    /// Text is trimmed; empty fails with `"EMPTY"`. Full journal fails with `"FULL"`.
    /// Returns the new note count (1..=[`NOTES_MAX`]).
    pub fn add_note(&mut self, text: &str) -> Result<usize, &'static str> {
        let mut t = text.trim().to_string();
        if t.is_empty() {
            return Err("EMPTY");
        }
        if self.notes.len() >= NOTES_MAX {
            return Err("FULL");
        }
        if t.chars().count() > NOTE_TEXT_MAX {
            t = t.chars().take(NOTE_TEXT_MAX).collect();
        }
        self.notes.push(t);
        Ok(self.notes.len())
    }

    /// Pop the most recent journal note (`SAY FORGET`).
    ///
    /// Returns the removed text, or `"EMPTY"` when the journal has no notes.
    pub fn pop_note(&mut self) -> Result<String, &'static str> {
        self.notes.pop().ok_or("EMPTY")
    }

    /// Move [`Self::held_id`] into backpack if non-zero and space remains.
    ///
    /// Returns the stored object id, or an error token (`EMPTY` / `FULL`).
    pub fn store_to_backpack(&mut self) -> Result<i32, &'static str> {
        if self.held_id == 0 {
            return Err("EMPTY");
        }
        if self.backpack.len() >= BACKPACK_MAX {
            return Err("FULL");
        }
        let id = self.held_id;
        self.backpack.push(id);
        self.held_id = 0;
        Ok(id)
    }

    /// Remove backpack index `i` into empty hands.
    ///
    /// Returns the taken object id, or an error token (`HANDS` / `BAD`).
    pub fn take_from_backpack(&mut self, i: usize) -> Result<i32, &'static str> {
        if self.held_id != 0 {
            return Err("HANDS");
        }
        if i >= self.backpack.len() {
            return Err("BAD");
        }
        let id = self.backpack.remove(i);
        self.held_id = id;
        Ok(id)
    }

    /// Unequip a clothing slot into empty hands (`SAY STRIP hat|chest|shoes`).
    ///
    /// Returns the stripped object id, or an error token (`HANDS` / `EMPTY`).
    pub fn strip_slot(&mut self, slot: ClothingSlot) -> Result<i32, &'static str> {
        if self.held_id != 0 {
            return Err("HANDS");
        }
        let id = self.clothing(slot);
        if id == 0 {
            return Err("EMPTY");
        }
        self.set_clothing(slot, 0);
        self.held_id = id;
        Ok(id)
    }

    /// Equip [`Self::held_id`] into `slot`, swapping any previous slot item into hands.
    ///
    /// Returns `(equipped_id, previous_slot_id)` where `previous_slot_id` is now
    /// held (0 if the slot was empty). Errors with `"EMPTY"` when hands are empty.
    pub fn wear_held(&mut self, slot: ClothingSlot) -> Result<(i32, i32), &'static str> {
        if self.held_id == 0 {
            return Err("EMPTY");
        }
        let id = self.held_id;
        let prev = self.clothing(slot);
        self.set_clothing(slot, id);
        self.held_id = prev;
        Ok((id, prev))
    }

    /// Drain backpack into a list for death / DROPALL scatter (clears [`Self::backpack`]).
    pub fn take_backpack_for_scatter(&mut self) -> Vec<i32> {
        std::mem::take(&mut self.backpack)
    }

    /// Drain clothing slots into a list for death scatter (clears hat/chest/shoes).
    pub fn take_clothing_for_scatter(&mut self) -> Vec<i32> {
        let mut out = Vec::new();
        for slot in [ClothingSlot::Hat, ClothingSlot::Chest, ClothingSlot::Shoes] {
            let id = self.clothing(slot);
            if id != 0 {
                out.push(id);
                self.set_clothing(slot, 0);
            }
        }
        out
    }

    /// Drain held + clothing + backpack for death scatter (player keeps nothing).
    pub fn take_death_loot_for_scatter(&mut self) -> Vec<i32> {
        let mut items = Vec::new();
        if self.held_id != 0 {
            items.push(self.held_id);
            self.held_id = 0;
        }
        items.extend(self.take_clothing_for_scatter());
        items.extend(self.take_backpack_for_scatter());
        items
    }

    /// Drain held + backpack for `SAY DROPALL` (clothing stays equipped).
    pub fn take_dropall_for_scatter(&mut self) -> Vec<i32> {
        let mut items = Vec::new();
        if self.held_id != 0 {
            items.push(self.held_id);
            self.held_id = 0;
        }
        items.extend(self.take_backpack_for_scatter());
        items
    }

    /// Set birth origin to absolute world tile (Eve spawn or mother tile for babies).
    pub fn set_birth_origin(&mut self, world_x: i32, world_y: i32) {
        self.birth_x = world_x;
        self.birth_y = world_y;
    }

    /// Client wire → absolute world (vanilla: `m.x += birthPos.x`).
    #[inline]
    pub fn client_to_world(&self, client_x: i32, client_y: i32) -> (i32, i32) {
        (client_x + self.birth_x, client_y + self.birth_y)
    }

    /// Absolute world → client wire for this viewer (Haxe `tx - gx`).
    #[inline]
    pub fn world_to_client(&self, world_x: i32, world_y: i32) -> (i32, i32) {
        (world_x - self.birth_x, world_y - self.birth_y)
    }

    /// Haxe-style: resend MC when player moved far enough from last chunk center.
    pub fn needs_map_chunk(&self, threshold: i32) -> bool {
        if !self.has_mc {
            return true;
        }
        (self.x - self.last_mc_x).abs() >= threshold || (self.y - self.last_mc_y).abs() >= threshold
    }

    pub fn snapshot(&self) -> PlayerSnapshot {
        PlayerSnapshot {
            conn_id: self.conn_id,
            p_id: self.p_id,
            x: self.x,
            y: self.y,
            held_id: self.held_id,
            food: self.food,
            food_max: self.food_max,
            age: self.age,
            email: self.email.clone(),
            deleted: self.deleted,
            moving: self.moving || self.move_path.is_some(),
            done_moving_seq: self.done_moving_seq,
        }
    }
}

/// Read-only view for web viewer / self-play UI (updated by sim after mutations).
#[derive(Debug, Clone, Serialize)]
pub struct PlayerSnapshot {
    pub conn_id: u64,
    pub p_id: i32,
    pub x: i32,
    pub y: i32,
    pub held_id: i32,
    pub food: f32,
    pub food_max: f32,
    pub age: f32,
    pub email: String,
    pub deleted: bool,
    /// True while a timed MovePath is active (or legacy moving flag).
    pub moving: bool,
    /// Seq of last completed/cancelled path (client `@seq` when provided).
    pub done_moving_seq: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trade_offer_defaults_none_and_can_set() {
        let mut p = Player::new(1, 1, "c@test");
        assert_eq!(p.trade_offer, None);
        p.trade_offer = Some((42, 7));
        assert_eq!(p.trade_offer, Some((42, 7)));
        p.trade_offer = None;
        assert_eq!(p.trade_offer, None);
    }

    #[test]
    fn notes_add_report_and_cap() {
        let mut p = Player::new(1, 1, "n@test");
        assert!(p.notes.is_empty());
        assert_eq!(p.notes_report(), format!("NOTES 0/{NOTES_MAX}"));
        assert_eq!(p.add_note("  hello  ").unwrap(), 1);
        assert_eq!(p.notes, vec!["hello".to_string()]);
        assert_eq!(p.notes_report(), format!("NOTES 1/{NOTES_MAX} 0:hello"));
        assert_eq!(p.add_note("").unwrap_err(), "EMPTY");
        assert_eq!(p.add_note("   ").unwrap_err(), "EMPTY");
        for i in 1..NOTES_MAX {
            assert_eq!(p.add_note(&format!("n{i}")).unwrap(), i + 1);
        }
        assert_eq!(p.notes.len(), NOTES_MAX);
        assert_eq!(p.add_note("overflow").unwrap_err(), "FULL");
        // Long text is truncated to NOTE_TEXT_MAX chars.
        p.notes.clear();
        let long: String = "x".repeat(NOTE_TEXT_MAX + 20);
        p.add_note(&long).unwrap();
        assert_eq!(p.notes[0].chars().count(), NOTE_TEXT_MAX);
    }

    #[test]
    fn notes_pop_forget_last() {
        let mut p = Player::new(1, 1, "f@test");
        assert_eq!(p.pop_note().unwrap_err(), "EMPTY");
        p.add_note("a").unwrap();
        p.add_note("b").unwrap();
        assert_eq!(p.pop_note().unwrap(), "b");
        assert_eq!(p.notes, vec!["a".to_string()]);
        assert_eq!(p.pop_note().unwrap(), "a");
        assert!(p.notes.is_empty());
        assert_eq!(p.pop_note().unwrap_err(), "EMPTY");
    }

    #[test]
    fn title_set_and_name_for_query() {
        let mut p = Player::new(1, 1, "t@test");
        p.first_name = "ADA".into();
        p.family_name = "SNOW".into();
        assert!(p.title.is_empty());
        assert_eq!(p.name_for_query(), "ADA SNOW");
        assert_eq!(p.set_title("").unwrap_err(), "EMPTY");
        assert_eq!(p.set_title("  Scout  ").unwrap(), "Scout");
        assert_eq!(p.title, "Scout");
        assert_eq!(p.name_for_query(), "ADA SNOW | Scout");
        assert_eq!(p.display_name(), "ADA SNOW"); // title not in display_name / NM
        let long: String = "y".repeat(TITLE_TEXT_MAX + 10);
        p.set_title(&long).unwrap();
        assert_eq!(p.title.chars().count(), TITLE_TEXT_MAX);
    }

    #[test]
    fn riding_defaults_false_and_can_toggle() {
        let mut p = Player::new(1, 1, "r@test");
        assert!(!p.riding);
        p.riding = true;
        assert!(p.riding);
        p.riding = false;
        assert!(!p.riding);
    }

    #[test]
    fn sitting_defaults_false_and_can_toggle() {
        let mut p = Player::new(1, 1, "s@test");
        assert!(!p.sitting);
        p.sitting = true;
        assert!(p.sitting);
        p.sitting = false;
        assert!(!p.sitting);
    }

    #[test]
    fn godmode_defaults_false_and_can_toggle() {
        let mut p = Player::new(1, 1, "g@test");
        assert!(!p.godmode);
        p.godmode = true;
        assert!(p.godmode);
        p.godmode = false;
        assert!(!p.godmode);
    }

    #[test]
    fn set_clothing_assigns_slots_zero_empty() {
        let mut p = Player::new(1, 1, "c@test");
        assert_eq!(p.hat, 0);
        assert_eq!(p.chest, 0);
        assert_eq!(p.shoes, 0);

        p.set_clothing(ClothingSlot::Hat, 100);
        p.set_clothing(ClothingSlot::Chest, 200);
        p.set_clothing(ClothingSlot::Shoes, 300);
        assert_eq!(p.clothing(ClothingSlot::Hat), 100);
        assert_eq!(p.clothing(ClothingSlot::Chest), 200);
        assert_eq!(p.clothing(ClothingSlot::Shoes), 300);
        assert_eq!(p.clothes_report(), "CLOTHES hat=100 chest=200 shoes=300");

        p.set_clothing(ClothingSlot::Hat, 0);
        assert_eq!(p.hat, 0);
        assert_eq!(p.clothes_report(), "CLOTHES hat=0 chest=200 shoes=300");
    }

    #[test]
    fn clothing_slot_from_name_hat_chest_shoes() {
        assert_eq!(
            clothing_slot_for_object("Wool Hat", ""),
            Some(ClothingSlot::Hat)
        );
        assert_eq!(
            clothing_slot_for_object("Leather Shoes", ""),
            Some(ClothingSlot::Shoes)
        );
        assert_eq!(
            clothing_slot_for_object("Linen Shirt", ""),
            Some(ClothingSlot::Chest)
        );
        assert_eq!(
            clothing_slot_for_object("Mystery", "clothing=1"),
            Some(ClothingSlot::Chest)
        );
        assert_eq!(clothing_slot_for_object("Gooseberry", ""), None);
    }

    #[test]
    fn baby_holding_defaults_and_methods() {
        let mut p = Player::new(1, 1, "m@test");
        assert_eq!(p.holding_player_id, 0);
        assert_eq!(p.held_by, 0);
        assert!(p.can_hold_baby());
        p.held_id = 1;
        assert!(!p.can_hold_baby());
        p.held_id = 0;
        p.start_holding(42);
        assert_eq!(p.holding_player_id, 42);
        assert!(!p.can_hold_baby());
        assert_eq!(p.release_holding(), 42);
        assert_eq!(p.holding_player_id, 0);
        assert_eq!(p.release_holding(), 0);
        p.age = 10.0;
        assert!(!p.can_hold_baby());
        p.age = 14.0;
        p.deleted = true;
        assert!(!p.can_hold_baby());
    }

    #[test]
    fn backpack_store_take_inv_and_max() {
        let mut p = Player::new(1, 1, "bp@test");
        assert!(p.backpack.is_empty());
        assert_eq!(p.inv_report(), format!("INV 0/{BACKPACK_MAX}"));
        assert_eq!(p.store_to_backpack(), Err("EMPTY"));

        p.held_id = 33;
        assert_eq!(p.store_to_backpack(), Ok(33));
        assert_eq!(p.held_id, 0);
        assert_eq!(p.backpack, vec![33]);
        assert_eq!(p.inv_report(), format!("INV 1/{BACKPACK_MAX} 33"));

        p.held_id = 55;
        assert_eq!(p.store_to_backpack(), Ok(55));
        assert_eq!(p.backpack, vec![33, 55]);
        assert_eq!(p.inv_report(), format!("INV 2/{BACKPACK_MAX} 33 55"));

        // Fill to max.
        for id in 1..=(BACKPACK_MAX as i32 - 2) {
            p.held_id = 100 + id;
            assert_eq!(p.store_to_backpack(), Ok(100 + id));
        }
        assert_eq!(p.backpack.len(), BACKPACK_MAX);
        p.held_id = 999;
        assert_eq!(p.store_to_backpack(), Err("FULL"));
        assert_eq!(p.held_id, 999);

        // Hands full blocks take.
        assert_eq!(p.take_from_backpack(0), Err("HANDS"));
        p.held_id = 0;
        assert_eq!(p.take_from_backpack(99), Err("BAD"));
        assert_eq!(p.take_from_backpack(0), Ok(33));
        assert_eq!(p.held_id, 33);
        assert_eq!(p.backpack[0], 55);
        assert_eq!(p.backpack.len(), BACKPACK_MAX - 1);
    }

    #[test]
    fn strip_slot_requires_empty_hands_and_filled_slot() {
        let mut p = Player::new(1, 1, "strip@test");
        assert_eq!(p.strip_slot(ClothingSlot::Hat), Err("EMPTY"));
        p.set_clothing(ClothingSlot::Hat, 100);
        p.held_id = 1;
        assert_eq!(p.strip_slot(ClothingSlot::Hat), Err("HANDS"));
        p.held_id = 0;
        assert_eq!(p.strip_slot(ClothingSlot::Hat), Ok(100));
        assert_eq!(p.held_id, 100);
        assert_eq!(p.hat, 0);
    }

    #[test]
    fn wear_held_equips_and_swaps_previous() {
        let mut p = Player::new(1, 1, "wear@test");
        assert_eq!(p.wear_held(ClothingSlot::Chest), Err("EMPTY"));
        p.held_id = 200;
        assert_eq!(p.wear_held(ClothingSlot::Chest), Ok((200, 0)));
        assert_eq!(p.chest, 200);
        assert_eq!(p.held_id, 0);
        p.held_id = 201;
        assert_eq!(p.wear_held(ClothingSlot::Chest), Ok((201, 200)));
        assert_eq!(p.chest, 201);
        assert_eq!(p.held_id, 200);
    }

    #[test]
    fn take_backpack_for_scatter_drains_vec() {
        let mut p = Player::new(1, 1, "sc@test");
        p.backpack = vec![10, 20, 30];
        let items = p.take_backpack_for_scatter();
        assert_eq!(items, vec![10, 20, 30]);
        assert!(p.backpack.is_empty());
    }

    #[test]
    fn take_death_loot_drains_held_clothing_backpack() {
        let mut p = Player::new(1, 1, "loot@test");
        p.held_id = 1;
        p.hat = 2;
        p.chest = 3;
        p.shoes = 4;
        p.backpack = vec![5, 6];
        let items = p.take_death_loot_for_scatter();
        assert_eq!(items, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(p.held_id, 0);
        assert_eq!(p.hat, 0);
        assert_eq!(p.chest, 0);
        assert_eq!(p.shoes, 0);
        assert!(p.backpack.is_empty());
    }

    #[test]
    fn take_dropall_keeps_clothing() {
        let mut p = Player::new(1, 1, "dropall@test");
        p.held_id = 9;
        p.hat = 7;
        p.backpack = vec![8];
        let items = p.take_dropall_for_scatter();
        assert_eq!(items, vec![9, 8]);
        assert_eq!(p.held_id, 0);
        assert_eq!(p.hat, 7);
        assert!(p.backpack.is_empty());
    }
}
