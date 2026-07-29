//! PlayerSoul — AI context / interaction memory (Haxe `openlife.server.PlayerSoul`).
//!
//! Pure memory FIFO + string builders for LLM roleplay prompts.
//! Not the account/grave "soul" token (`death_inherit::account_soul_token`).
//!
//! Chunk: **S-SOUL** (pure body). Wire: **AI-SOUL-WIRE** via `player_soul.rs` facade.

use crate::prestige::PrestigeClass;
use crate::reputation::label_from_lost_combat;
use std::collections::HashMap;

/// Haxe `ServerSettings.AiMemoryMaxEntries` — max interaction players remembered.
pub const AI_MEMORY_MAX_ENTRIES: usize = 20;
/// Haxe `ServerSettings.AiChatMemoryMaxEntries` — max chat history entries.
pub const AI_CHAT_MEMORY_MAX_ENTRIES: usize = 100;

// ---------------------------------------------------------------------------
// Interaction memory
// ---------------------------------------------------------------------------

/// Haxe `InteractionType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InteractionType {
    AttackDamage,
    ServedFood,
    ProvidedCloths,
    GivenCoins,
    /// Prepared for future use (no writers in Haxe either).
    ProvidedHealing,
    /// Prepared for future use (no writers in Haxe either).
    Trade,
}

/// Haxe `InteractionData` — accumulated values with one other player.
#[derive(Debug, Clone)]
pub struct InteractionData {
    pub player_id: i32,
    pub player_name: String,
    pub player_family_name: String,
    pub attack_damage: f32,
    pub served_food: f32,
    pub provided_cloths: f32,
    pub given_coins: f32,
    pub provided_healing: f32,
    pub trade_value: f32,
}

impl InteractionData {
    pub fn new(player_id: i32, player_name: &str, player_family_name: &str) -> Self {
        Self {
            player_id,
            player_name: player_name.to_string(),
            player_family_name: player_family_name.to_string(),
            attack_damage: 0.0,
            served_food: 0.0,
            provided_cloths: 0.0,
            given_coins: 0.0,
            provided_healing: 0.0,
            trade_value: 0.0,
        }
    }

    fn add(&mut self, ty: InteractionType, value: f32) {
        if !value.is_finite() {
            return;
        }
        match ty {
            InteractionType::AttackDamage => self.attack_damage += value,
            InteractionType::ServedFood => self.served_food += value,
            InteractionType::ProvidedCloths => self.provided_cloths += value,
            InteractionType::GivenCoins => self.given_coins += value,
            InteractionType::ProvidedHealing => self.provided_healing += value,
            InteractionType::Trade => self.trade_value += value,
        }
    }
}

/// Haxe `ChatEntry`.
#[derive(Debug, Clone)]
pub struct ChatEntry {
    pub player_id: i32,
    pub player_name: String,
    pub player_family_name: String,
    pub message: String,
    pub reply: String,
}

impl ChatEntry {
    pub fn new(
        player_id: i32,
        player_name: &str,
        player_family_name: &str,
        message: &str,
        reply: &str,
    ) -> Self {
        Self {
            player_id,
            player_name: player_name.to_string(),
            player_family_name: player_family_name.to_string(),
            message: message.to_string(),
            reply: reply.to_string(),
        }
    }

    /// Haxe `ChatEntry.toString`.
    // Haxe: PlayerSoul.ChatEntry.toString
    pub fn to_string_haxe(&self) -> String {
        format!(
            "Name {} {}: {} Your reply: {}",
            self.player_name, self.player_family_name, self.message, self.reply
        )
    }
}

/// Per-player AI memory book (Haxe `PlayerSoul` without owning a GPI pointer).
///
/// Thread safety is left to the caller (Rust sim is single-writer); Haxe used Mutex.
#[derive(Debug, Clone, Default)]
pub struct PlayerSoul {
    memory: HashMap<i32, InteractionData>,
    /// FIFO order of player ids for eviction.
    memory_order: Vec<i32>,
    chat_memory: Vec<ChatEntry>,
}

impl PlayerSoul {
    pub fn new() -> Self {
        Self::default()
    }

    /// Haxe `PlayerSoul.addInteraction`.
    // Haxe: PlayerSoul.addInteraction
    pub fn add_interaction(
        &mut self,
        other_player_id: i32,
        other_player_name: &str,
        other_player_family_name: &str,
        ty: InteractionType,
        value: f32,
        max_entries: usize,
    ) {
        if !self.memory.contains_key(&other_player_id) {
            let data =
                InteractionData::new(other_player_id, other_player_name, other_player_family_name);
            self.memory.insert(other_player_id, data);
            self.memory_order.push(other_player_id);
            let cap = max_entries.max(1);
            while self.memory_order.len() > cap {
                if let Some(oldest) = self.memory_order.first().copied() {
                    self.memory_order.remove(0);
                    self.memory.remove(&oldest);
                } else {
                    break;
                }
            }
        }
        if let Some(interaction) = self.memory.get_mut(&other_player_id) {
            // Keep latest display names if re-seen.
            interaction.player_name = other_player_name.to_string();
            interaction.player_family_name = other_player_family_name.to_string();
            interaction.add(ty, value);
        }
    }

    /// Convenience: Haxe default `AiMemoryMaxEntries` (20).
    pub fn add_interaction_default(
        &mut self,
        other_player_id: i32,
        other_player_name: &str,
        other_player_family_name: &str,
        ty: InteractionType,
        value: f32,
    ) {
        self.add_interaction(
            other_player_id,
            other_player_name,
            other_player_family_name,
            ty,
            value,
            AI_MEMORY_MAX_ENTRIES,
        );
    }

    /// Haxe `PlayerSoul.getMemoryText`.
    // Haxe: PlayerSoul.getMemoryText
    pub fn get_memory_text(&self) -> String {
        if self.memory_order.is_empty() {
            return String::new();
        }
        let mut result = String::from("Recent interactions with other players: ");
        for player_id in &self.memory_order {
            let Some(interaction) = self.memory.get(player_id) else {
                continue;
            };
            let player_name = format!(
                "{} {}",
                interaction.player_name, interaction.player_family_name
            );
            let mut parts: Vec<String> = Vec::new();
            if interaction.attack_damage > 0.0 {
                parts.push(format!(
                    "{}, AttackDamage += {}",
                    player_name, interaction.attack_damage
                ));
            }
            if interaction.served_food > 0.0 {
                parts.push(format!(
                    "{}, ServedFood += {}",
                    player_name, interaction.served_food
                ));
            }
            if interaction.provided_cloths > 0.0 {
                parts.push(format!(
                    "{}, ProvidedCloths += {}",
                    player_name, interaction.provided_cloths
                ));
            }
            if interaction.given_coins > 0.0 {
                parts.push(format!(
                    "{}, GivenCoins += {}",
                    player_name, interaction.given_coins
                ));
            }
            if interaction.provided_healing > 0.0 {
                parts.push(format!(
                    "{}, ProvidedHealing += {}",
                    player_name, interaction.provided_healing
                ));
            }
            if interaction.trade_value > 0.0 {
                parts.push(format!(
                    "{}, Trade += {}",
                    player_name, interaction.trade_value
                ));
            }
            if !parts.is_empty() {
                result.push_str(&parts.join("; "));
                result.push_str(" --- ");
            }
        }
        result
    }

    /// Haxe `PlayerSoul.addChatEntry` (after successful LLM reply).
    // Haxe: PlayerSoul.addChatEntry
    pub fn add_chat_entry(
        &mut self,
        player_id: i32,
        player_name: &str,
        player_family_name: &str,
        message: &str,
        reply: &str,
        max_entries: usize,
    ) {
        self.chat_memory.push(ChatEntry::new(
            player_id,
            player_name,
            player_family_name,
            message,
            reply,
        ));
        let cap = max_entries.max(1);
        while self.chat_memory.len() > cap {
            self.chat_memory.remove(0);
        }
    }

    /// Convenience: Haxe default `AiChatMemoryMaxEntries` (100).
    pub fn add_chat_entry_default(
        &mut self,
        player_id: i32,
        player_name: &str,
        player_family_name: &str,
        message: &str,
        reply: &str,
    ) {
        self.add_chat_entry(
            player_id,
            player_name,
            player_family_name,
            message,
            reply,
            AI_CHAT_MEMORY_MAX_ENTRIES,
        );
    }

    /// Haxe `PlayerSoul.getChatMemoryText`.
    ///
    /// Haxe TODO: "only from player talking to" — not filtered yet (port-as-is).
    // Haxe: PlayerSoul.getChatMemoryText
    pub fn get_chat_memory_text(&self) -> String {
        self.get_chat_memory_text_filtered(None)
    }

    /// Chat dump; optional `speaker_id` filter (product extension of Haxe TODO).
    pub fn get_chat_memory_text_filtered(&self, speaker_id: Option<i32>) -> String {
        let entries: Vec<&ChatEntry> = match speaker_id {
            Some(id) => self.chat_memory.iter().filter(|e| e.player_id == id).collect(),
            None => self.chat_memory.iter().collect(),
        };
        if entries.is_empty() {
            return String::new();
        }
        let mut result = String::from("Recent chat history: ");
        for entry in entries {
            result.push_str(&entry.to_string_haxe());
            result.push_str(" --- ");
        }
        result
    }

    pub fn memory_len(&self) -> usize {
        self.memory_order.len()
    }

    pub fn chat_len(&self) -> usize {
        self.chat_memory.len()
    }

    pub fn interaction(&self, player_id: i32) -> Option<&InteractionData> {
        self.memory.get(&player_id)
    }
}

// ---------------------------------------------------------------------------
// Pure labels (static in Haxe)
// ---------------------------------------------------------------------------

/// Haxe `PlayerSoul.getPrestigeClassName`.
// Haxe: PlayerSoul.getPrestigeClassName
pub fn get_prestige_class_name(prestige_class: PrestigeClass) -> &'static str {
    prestige_class.wire_name()
}

/// Haxe `PlayerSoul.getCombatPrestigeLabel`.
// Haxe: PlayerSoul.getCombatPrestigeLabel
pub fn get_combat_prestige_label(lost_combat_prestige: f32) -> &'static str {
    label_from_lost_combat(lost_combat_prestige).display()
}

/// Haxe `PlayerSoul.getTemperatureLabel` — absolute heat bands 0..1.
///
/// Distinct from [`crate::heat_ideal::label_for_heat`] (comfort-relative bands).
// Haxe: PlayerSoul.getTemperatureLabel
pub fn get_temperature_label(heat: f32) -> &'static str {
    let h = if heat.is_finite() { heat } else { 0.5 };
    if h < 0.1 {
        "freezing"
    } else if h < 0.25 {
        "cold"
    } else if h < 0.4 {
        "cool"
    } else if h < 0.6 {
        "mild"
    } else if h < 0.75 {
        "warm"
    } else if h < 0.9 {
        "hot"
    } else {
        "sweltering"
    }
}

// ---------------------------------------------------------------------------
// Snapshot for prompt builders (decouples from live Player / world)
// ---------------------------------------------------------------------------

/// Snapshot of fields needed by soul / external intro text builders.
#[derive(Debug, Clone)]
pub struct SoulView {
    pub name: String,
    pub family_name: String,
    pub is_female: bool,
    /// Haxe `trueAge` (years, float).
    pub true_age: f32,
    pub prestige: f32,
    pub prestige_class: PrestigeClass,
    /// Partner first name only (Haxe uses `partner.name`).
    pub partner_name: Option<String>,
    /// Full `"First Family"` or None.
    pub father_display: Option<String>,
    pub mother_display: Option<String>,
    pub food_store: f32,
    pub food_store_max: f32,
    pub is_wounded: bool,
    pub is_super_hot: bool,
    pub is_super_cold: bool,
    /// Body heat 0..1 (Haxe `player.heat`).
    pub heat: f32,
    /// Tile ambient at player (Haxe `WorldMap.getTileTemperature`).
    pub tile_temperature: f32,
    /// Home absolute tile; `None` or `(0,0)` = unset.
    pub home: Option<(i32, i32)>,
    pub tx: i32,
    pub ty: i32,
    pub assigned_profession: Option<String>,
    pub last_profession: Option<String>,
    pub held_object_name: Option<String>,
    pub is_angry_or_terrified: bool,
    pub is_holding_weapon: bool,
    pub is_ai: bool,
    /// Haxe `TimeHelper.SeasonText` (e.g. `"A hard  Winter"` or `"Spring"`).
    pub season_text: String,
}

impl Default for SoulView {
    fn default() -> Self {
        Self {
            name: "NEWBORN".into(),
            family_name: "SNOW".into(),
            is_female: false,
            true_age: 14.0,
            prestige: 0.0,
            prestige_class: PrestigeClass::NotSet,
            partner_name: None,
            father_display: None,
            mother_display: None,
            food_store: 10.0,
            food_store_max: 20.0,
            is_wounded: false,
            is_super_hot: false,
            is_super_cold: false,
            heat: 0.5,
            tile_temperature: 0.5,
            home: None,
            tx: 0,
            ty: 0,
            assigned_profession: None,
            last_profession: None,
            held_object_name: None,
            is_angry_or_terrified: false,
            is_holding_weapon: false,
            is_ai: false,
            season_text: "DONT KNOW".into(),
        }
    }
}

/// Haxe `PlayerSoul.getTemperatureContextText`.
// Haxe: PlayerSoul.getTemperatureContextText
pub fn get_temperature_context_text(heat: f32, tile_temperature: f32) -> String {
    let body = get_temperature_label(heat);
    let tile = get_temperature_label(tile_temperature);
    format!(
        "The temperature is {body}. The surrounding temperature is {tile}. "
    )
}

/// Cardinal / intercardinal from delta to home (Haxe home direction block).
///
/// Haxe condition: `useIntermediate = |dx| < 2*|dy| || |dy| < 2*|dx|`
/// (i.e. both axes "significant" enough for intercardinal).
// Haxe: PlayerSoul.getHomeContextText direction
pub fn home_direction(dx: i32, dy: i32) -> String {
    let adx = dx.abs();
    let ady = dy.abs();
    let use_intermediate = adx < 2 * ady || ady < 2 * adx;

    if dx > 0 && dy > 0 {
        // East, South
        if use_intermediate {
            "south east".into()
        } else if adx >= ady {
            "east".into()
        } else {
            "south".into()
        }
    } else if dx < 0 && dy > 0 {
        if use_intermediate {
            "south west".into()
        } else if adx >= ady {
            "west".into()
        } else {
            "south".into()
        }
    } else if dx > 0 && dy < 0 {
        if use_intermediate {
            "north east".into()
        } else if adx >= ady {
            "east".into()
        } else {
            "north".into()
        }
    } else if dx < 0 && dy < 0 {
        if use_intermediate {
            "north west".into()
        } else if adx >= ady {
            "west".into()
        } else {
            "north".into()
        }
    } else if dx > 0 {
        "east".into()
    } else if dx < 0 {
        "west".into()
    } else if dy > 0 {
        "south".into()
    } else if dy < 0 {
        "north".into()
    } else {
        String::new()
    }
}

/// Haxe `PlayerSoul.getHomeContextText` (1 tile ≈ 1 mile).
// Haxe: PlayerSoul.getHomeContextText
pub fn get_home_context_text(tx: i32, ty: i32, home: Option<(i32, i32)>) -> String {
    let Some((hx, hy)) = home else {
        return "No home. ".into();
    };
    if hx == 0 && hy == 0 {
        return "No home. ".into();
    }

    let dx = hx - tx;
    let dy = hy - ty;
    // Haxe: AiHelper.CalculateQuadDistanceToObject → sqrt → round miles
    let quad = (dx as f64) * (dx as f64) + (dy as f64) * (dy as f64);
    let miles = quad.sqrt().round() as i32;
    let direction = home_direction(dx, dy);

    if miles < 20 {
        return "You are at your home. ".into();
    }

    let miles_text = if miles == 1 { "mile" } else { "miles" };
    if !direction.is_empty() {
        format!("Your home is {miles} {miles_text} to the {direction}. ")
    } else {
        format!("Your home is {miles} {miles_text} away. ")
    }
}

/// Haxe `PlayerSoul.getFamilyText` (first-person; children skipped as in Haxe).
// Haxe: PlayerSoul.getFamilyText
pub fn get_family_text(father_display: Option<&str>, mother_display: Option<&str>) -> String {
    let mut text = String::new();
    if let Some(f) = father_display {
        text.push_str(&format!("Your father is {f}. "));
    }
    if let Some(m) = mother_display {
        text.push_str(&format!("Your mother is {m}. "));
    }
    text
}

/// Haxe `PlayerSoul.getExternalFamilyText`.
// Haxe: PlayerSoul.getExternalFamilyText
pub fn get_external_family_text(
    father_display: Option<&str>,
    mother_display: Option<&str>,
) -> String {
    let mut text = String::new();
    if let Some(f) = father_display {
        text.push_str(&format!("Their father is {f}. "));
    }
    if let Some(m) = mother_display {
        text.push_str(&format!("Their mother is {m}. "));
    }
    text
}

/// Haxe `PlayerSoul.getStatusText`.
// Haxe: PlayerSoul.getStatusText
pub fn get_status_text(
    food_store: f32,
    food_store_max: f32,
    is_wounded: bool,
    is_super_hot: bool,
    is_super_cold: bool,
) -> String {
    let mut text = String::new();
    let max = if food_store_max.is_finite() && food_store_max > 0.0 {
        food_store_max
    } else {
        1.0
    };
    let food = if food_store.is_finite() {
        food_store
    } else {
        0.0
    };
    let food_percent = (food / max) * 100.0;
    let pct_floor = food_percent.floor() as i32;
    if food_percent < 20.0 {
        text.push_str(&format!(
            "You are starving! Food level: {pct_floor}%. "
        ));
    } else if food_percent < 50.0 {
        text.push_str(&format!("You are hungry. Food level: {pct_floor}%. "));
    }
    if is_wounded {
        text.push_str("You are wounded. ");
    }
    if is_super_hot {
        text.push_str("You are very hot. ");
    }
    if is_super_cold {
        text.push_str("You are very cold. ");
    }
    text
}

/// Haxe `PlayerSoul.getExternalStatusText`.
// Haxe: PlayerSoul.getExternalStatusText
pub fn get_external_status_text(
    food_store: f32,
    food_store_max: f32,
    is_wounded: bool,
    is_super_hot: bool,
    is_super_cold: bool,
) -> String {
    let mut text = String::new();
    let max = if food_store_max.is_finite() && food_store_max > 0.0 {
        food_store_max
    } else {
        1.0
    };
    let food = if food_store.is_finite() {
        food_store
    } else {
        0.0
    };
    let food_percent = (food / max) * 100.0;
    if food_percent < 20.0 {
        text.push_str("They look starving! ");
    } else if food_percent < 50.0 {
        text.push_str("They look hungry. ");
    }
    if is_wounded {
        text.push_str("They are wounded. ");
    }
    if is_super_hot {
        text.push_str("They look very hot. ");
    }
    if is_super_cold {
        text.push_str("They look very cold. ");
    }
    text
}

/// Haxe `PlayerSoul.getProfessionText`.
// Haxe: PlayerSoul.getProfessionText
pub fn get_profession_text(
    assigned_profession: Option<&str>,
    last_profession: Option<&str>,
) -> String {
    match (assigned_profession, last_profession) {
        (None, None) => "NONE".into(),
        (None, Some(last)) => last.to_string(),
        (Some(assigned), None) => assigned.to_string(),
        (Some(assigned), Some(last)) => {
            if assigned == last {
                last.to_string()
            } else {
                format!("{assigned} doing {last}")
            }
        }
    }
}

/// Haxe `PlayerSoul.getSoulText` — first-person AI self-context.
// Haxe: PlayerSoul.getSoulText
pub fn get_soul_text(v: &SoulView) -> String {
    let sex = if v.is_female { "female" } else { "male" };
    let age_years = v.true_age.floor() as i32;
    let mut text = format!(
        "You are {} {}, a {} aged {} years. ",
        v.name, v.family_name, sex, age_years
    );
    if v.true_age < 6.0 {
        text.push_str("You are very young! Speak according to your age! ");
    }
    text.push_str(&format!("It is currently {}. ", v.season_text));

    let class_name = get_prestige_class_name(v.prestige_class);
    let prestige_round = v.prestige.round() as i32;
    text.push_str(&format!(
        "You are a {class_name} with prestige {prestige_round}. "
    ));

    if let Some(ref partner) = v.partner_name {
        text.push_str(&format!("Your partner is {partner}! "));
    }

    text.push_str(&get_family_text(
        v.father_display.as_deref(),
        v.mother_display.as_deref(),
    ));
    text.push_str(&get_status_text(
        v.food_store,
        v.food_store_max,
        v.is_wounded,
        v.is_super_hot,
        v.is_super_cold,
    ));
    text.push_str(&get_temperature_context_text(v.heat, v.tile_temperature));
    text.push_str(&get_home_context_text(v.tx, v.ty, v.home));

    let profession = get_profession_text(
        v.assigned_profession.as_deref(),
        v.last_profession.as_deref(),
    );
    if !profession.is_empty() {
        // Haxe always appends when length > 0; "NONE" is non-empty so included.
        text.push_str(&format!("Your profession: {profession}. "));
    }

    if let Some(ref held) = v.held_object_name {
        text.push_str(&format!("You are holding {held}. "));
    }
    if v.is_angry_or_terrified {
        // Haxe typo preserved: "acordingly"
        text.push_str("You are angry or terrified act acordingly! ");
    }
    if v.is_holding_weapon {
        text.push_str("You are holding a weapon. Consider this strongly!");
    }

    text
}

/// Haxe `PlayerSoul.getExternalIntro` — third-person intro for another AI.
// Haxe: PlayerSoul.getExternalIntro
pub fn get_external_intro(v: &SoulView) -> String {
    let sex = if v.is_female { "female" } else { "male" };
    let age_years = v.true_age.floor() as i32;
    let mut text = format!(
        "You are communicating with {} {}, a {} aged {} years. ",
        v.name, v.family_name, sex, age_years
    );

    let class_name = get_prestige_class_name(v.prestige_class);
    let prestige_round = v.prestige.round() as i32;
    text.push_str(&format!(
        "They are a {class_name} with prestige {prestige_round}. "
    ));

    text.push_str(&get_external_family_text(
        v.father_display.as_deref(),
        v.mother_display.as_deref(),
    ));
    text.push_str(&get_external_status_text(
        v.food_store,
        v.food_store_max,
        v.is_wounded,
        v.is_super_hot,
        v.is_super_cold,
    ));

    let profession = get_profession_text(
        v.assigned_profession.as_deref(),
        v.last_profession.as_deref(),
    );
    // Haxe: only when profession non-empty **and** target is AI
    if !profession.is_empty() && v.is_ai {
        if v.is_female {
            text.push_str(&format!("Her profession is {profession}. "));
        } else {
            text.push_str(&format!("His profession is {profession}. "));
        }
    }

    if let Some(ref held) = v.held_object_name {
        text.push_str(&format!("They are holding {held}. "));
    }
    if v.is_angry_or_terrified {
        text.push_str("They look angry or terrified. Consider this strongly!");
    }
    if v.is_holding_weapon {
        text.push_str("They are holding a weapon.");
    }

    text
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_interaction_accumulates_and_fifo() {
        let mut soul = PlayerSoul::new();
        soul.add_interaction(1, "A", "Alpha", InteractionType::AttackDamage, 3.0, 3);
        soul.add_interaction(1, "A", "Alpha", InteractionType::AttackDamage, 2.0, 3);
        soul.add_interaction(1, "A", "Alpha", InteractionType::ServedFood, 1.5, 3);
        let i = soul.interaction(1).unwrap();
        assert!((i.attack_damage - 5.0).abs() < 1e-4);
        assert!((i.served_food - 1.5).abs() < 1e-4);

        soul.add_interaction(2, "B", "Beta", InteractionType::GivenCoins, 1.0, 3);
        soul.add_interaction(3, "C", "Gamma", InteractionType::Trade, 4.0, 3);
        assert_eq!(soul.memory_len(), 3);
        // Next new id drops oldest (1)
        soul.add_interaction(4, "D", "Delta", InteractionType::ProvidedCloths, 1.0, 3);
        assert_eq!(soul.memory_len(), 3);
        assert!(soul.interaction(1).is_none());
        assert!(soul.interaction(2).is_some());
        assert!(soul.interaction(4).is_some());
    }

    #[test]
    fn get_memory_text_shape() {
        let mut soul = PlayerSoul::new();
        soul.add_interaction_default(7, "Bob", "Snow", InteractionType::AttackDamage, 2.0);
        soul.add_interaction_default(7, "Bob", "Snow", InteractionType::GivenCoins, 5.0);
        soul.add_interaction_default(8, "Eve", "Lake", InteractionType::ServedFood, 1.0);
        let t = soul.get_memory_text();
        assert!(t.starts_with("Recent interactions with other players: "));
        assert!(t.contains("Bob Snow, AttackDamage += 2"));
        assert!(t.contains("Bob Snow, GivenCoins += 5"));
        assert!(t.contains("Eve Lake, ServedFood += 1"));
        assert!(t.contains(" --- "));
    }

    #[test]
    fn chat_fifo_and_filter() {
        let mut soul = PlayerSoul::new();
        for i in 0..5 {
            soul.add_chat_entry(i, "P", "Fam", &format!("msg{i}"), &format!("r{i}"), 3);
        }
        assert_eq!(soul.chat_len(), 3);
        let all = soul.get_chat_memory_text();
        assert!(all.starts_with("Recent chat history: "));
        assert!(!all.contains("msg0"));
        assert!(!all.contains("msg1"));
        assert!(all.contains("msg2"));
        assert!(all.contains("Name P Fam: msg4 Your reply: r4"));

        let filtered = soul.get_chat_memory_text_filtered(Some(4));
        assert!(filtered.contains("msg4"));
        assert!(!filtered.contains("msg3"));
        assert_eq!(soul.get_chat_memory_text_filtered(Some(99)), "");
    }

    #[test]
    fn combat_prestige_labels_match_reputation() {
        assert_eq!(get_combat_prestige_label(51.0), "super bad");
        assert_eq!(get_combat_prestige_label(20.0), "bad");
        assert_eq!(get_combat_prestige_label(5.0), "fairly bad");
        assert_eq!(get_combat_prestige_label(0.0), "neutral");
        assert_eq!(get_combat_prestige_label(-4.0), "fairly good");
        assert_eq!(get_combat_prestige_label(-20.0), "fairly good");
        assert_eq!(get_combat_prestige_label(-50.0), "good");
        assert_eq!(get_combat_prestige_label(-51.0), "super good");
    }

    #[test]
    fn temperature_label_absolute_bands() {
        assert_eq!(get_temperature_label(0.0), "freezing");
        assert_eq!(get_temperature_label(0.09), "freezing");
        assert_eq!(get_temperature_label(0.1), "cold");
        assert_eq!(get_temperature_label(0.24), "cold");
        assert_eq!(get_temperature_label(0.25), "cool");
        assert_eq!(get_temperature_label(0.39), "cool");
        assert_eq!(get_temperature_label(0.4), "mild");
        assert_eq!(get_temperature_label(0.59), "mild");
        assert_eq!(get_temperature_label(0.6), "warm");
        assert_eq!(get_temperature_label(0.74), "warm");
        assert_eq!(get_temperature_label(0.75), "hot");
        assert_eq!(get_temperature_label(0.89), "hot");
        assert_eq!(get_temperature_label(0.9), "sweltering");
        assert_eq!(get_temperature_label(1.0), "sweltering");
        // Distinct from heat_ideal comfort labels at 0.5
        assert_eq!(get_temperature_label(0.5), "mild");
    }

    #[test]
    fn prestige_class_names() {
        assert_eq!(get_prestige_class_name(PrestigeClass::NotSet), "commoner");
        assert_eq!(get_prestige_class_name(PrestigeClass::Serf), "serf");
        assert_eq!(get_prestige_class_name(PrestigeClass::Commoner), "commoner");
        assert_eq!(get_prestige_class_name(PrestigeClass::Noble), "noble");
        assert_eq!(get_prestige_class_name(PrestigeClass::King), "king");
        assert_eq!(get_prestige_class_name(PrestigeClass::Emperor), "emperor");
    }

    #[test]
    fn home_context_null_at_home_and_direction() {
        assert_eq!(get_home_context_text(10, 10, None), "No home. ");
        assert_eq!(get_home_context_text(10, 10, Some((0, 0))), "No home. ");
        // 5 tiles east — miles < 20 → at home
        assert_eq!(
            get_home_context_text(0, 0, Some((5, 0))),
            "You are at your home. "
        );
        // 30 tiles pure east
        let far = get_home_context_text(0, 0, Some((30, 0)));
        assert_eq!(far, "Your home is 30 miles to the east. ");
        // Diagonal NE significant both axes
        let ne = get_home_context_text(0, 0, Some((30, -30)));
        assert!(ne.contains("north east"), "{ne}");
        assert!(ne.contains("42 miles") || ne.contains("42 mile"), "{ne}"); // round(sqrt(1800))≈42
    }

    #[test]
    fn status_food_thresholds() {
        // 10% → starving
        let s = get_status_text(2.0, 20.0, false, false, false);
        assert!(s.contains("starving"));
        assert!(s.contains("10%"));
        // 40% → hungry
        let h = get_status_text(8.0, 20.0, true, true, false);
        assert!(h.contains("hungry"));
        assert!(h.contains("You are wounded."));
        assert!(h.contains("You are very hot."));
        // 80% → no food line
        let full = get_status_text(16.0, 20.0, false, false, true);
        assert!(!full.contains("hungry"));
        assert!(!full.contains("starving"));
        assert!(full.contains("very cold"));

        let ext = get_external_status_text(2.0, 20.0, true, false, false);
        assert!(ext.contains("They look starving!"));
        assert!(ext.contains("They are wounded."));
    }

    #[test]
    fn profession_text_variants() {
        assert_eq!(get_profession_text(None, None), "NONE");
        assert_eq!(get_profession_text(None, Some("farmer")), "farmer");
        assert_eq!(get_profession_text(Some("smith"), Some("smith")), "smith");
        assert_eq!(
            get_profession_text(Some("smith"), Some("farmer")),
            "smith doing farmer"
        );
    }

    #[test]
    fn soul_text_and_external_intro_golden() {
        let v = SoulView {
            name: "Ada".into(),
            family_name: "Stone".into(),
            is_female: true,
            true_age: 22.7,
            prestige: 55.4,
            prestige_class: PrestigeClass::Noble,
            partner_name: Some("Bob".into()),
            father_display: Some("Carl Stone".into()),
            mother_display: Some("Dana Clay".into()),
            food_store: 5.0,
            food_store_max: 20.0,
            is_wounded: true,
            is_super_hot: false,
            is_super_cold: true,
            heat: 0.3,
            tile_temperature: 0.2,
            home: Some((100, 0)),
            tx: 0,
            ty: 0,
            assigned_profession: Some("baker".into()),
            last_profession: Some("farmer".into()),
            held_object_name: Some("Iron Axe".into()),
            is_angry_or_terrified: true,
            is_holding_weapon: true,
            is_ai: true,
            season_text: "A hard  Winter".into(),
        };

        let soul = get_soul_text(&v);
        assert!(soul.starts_with("You are Ada Stone, a female aged 22 years. "));
        assert!(soul.contains("It is currently A hard  Winter. "));
        assert!(soul.contains("You are a noble with prestige 55. "));
        assert!(soul.contains("Your partner is Bob! "));
        assert!(soul.contains("Your father is Carl Stone. "));
        assert!(soul.contains("Your mother is Dana Clay. "));
        assert!(soul.contains("You are hungry. Food level: 25%. "));
        assert!(soul.contains("You are wounded. "));
        assert!(soul.contains("You are very cold. "));
        assert!(soul.contains("The temperature is cool. "));
        assert!(soul.contains("The surrounding temperature is cold. "));
        assert!(soul.contains("Your home is 100 miles to the east. "));
        assert!(soul.contains("Your profession: baker doing farmer. "));
        assert!(soul.contains("You are holding Iron Axe. "));
        assert!(soul.contains("act acordingly!"));
        assert!(soul.contains("holding a weapon. Consider this strongly!"));

        let intro = get_external_intro(&v);
        assert!(intro.starts_with(
            "You are communicating with Ada Stone, a female aged 22 years. "
        ));
        assert!(intro.contains("They are a noble with prestige 55. "));
        assert!(intro.contains("Their father is Carl Stone. "));
        assert!(intro.contains("They look hungry. "));
        assert!(intro.contains("Her profession is baker doing farmer. "));
        assert!(intro.contains("They are holding Iron Axe. "));
        assert!(intro.contains("They look angry or terrified. Consider this strongly!"));
        assert!(intro.contains("They are holding a weapon."));

        // Human non-AI: profession branch suppressed in external intro
        let mut human = v.clone();
        human.is_ai = false;
        let intro_h = get_external_intro(&human);
        assert!(!intro_h.contains("profession is"));
    }

    #[test]
    fn young_age_prompt_line() {
        let mut v = SoulView::default();
        v.true_age = 3.2;
        v.name = "Kid".into();
        v.family_name = "Tiny".into();
        let t = get_soul_text(&v);
        assert!(t.contains("aged 3 years."));
        assert!(t.contains("You are very young! Speak according to your age! "));
    }

    #[test]
    fn temperature_context_text() {
        let t = get_temperature_context_text(0.05, 0.95);
        assert_eq!(
            t,
            "The temperature is freezing. The surrounding temperature is sweltering. "
        );
    }

    #[test]
    fn default_caps_match_haxe_settings() {
        assert_eq!(AI_MEMORY_MAX_ENTRIES, 20);
        assert_eq!(AI_CHAT_MEMORY_MAX_ENTRIES, 100);
    }
}
