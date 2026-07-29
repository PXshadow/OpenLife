//! AI-SOUL-WIRE — attach pure PlayerSoul to live Player / SimState.
//!
//! Haxe: `GlobalPlayerInstance.playerSoul` lazy field + `PlayerSoul.getSoulText` assembly.
//! Chunk: **AI-SOUL-WIRE** / `soul_on_player`.
//!
//! Nested under `player_soul` module; `impl SimState` is crate-visible.

use crate::animal_damage::is_holding_weapon;
use crate::map_temp_player::get_tile_temperature;
use crate::{Player, SimState};

use super::{
    get_external_intro, get_soul_text, home_option, is_angry_or_terrified,
    is_super_cold_for_person, is_super_hot_for_person, parent_display_name, person_looks_female,
    sticky_profession_pair, InteractionType,
};

impl SimState {
    /// Resolve player by `p_id` (any connection).
    pub fn player_by_p_id(&self, p_id: i32) -> Option<&Player> {
        self.players.values().find(|p| p.p_id == p_id)
    }

    /// Mutable player by `p_id`.
    pub fn player_by_p_id_mut(&mut self, p_id: i32) -> Option<&mut Player> {
        self.players.values_mut().find(|p| p.p_id == p_id)
    }

    /// Build live [`SoulView`] for AI prompts (Haxe `PlayerSoul` GPI fields).
    ///
    /// // Haxe: PlayerSoul.getSoulText / getExternalIntro field gather
    // AI-SOUL-WIRE
    pub fn soul_view_for(&self, p_id: i32) -> Option<super::SoulView> {
        let p = self.player_by_p_id(p_id)?;
        let (name_obj, desc_obj) = self
            .content
            .get(p.display_object_id)
            .map(|d| (d.name.as_str(), d.description.as_str()))
            .unwrap_or(("", ""));
        let is_female = person_looks_female(p.display_object_id, name_obj, desc_obj);

        let lineage = self.social.lineages.get(&p_id);
        let mother_display = lineage
            .and_then(|n| n.mother_id)
            .and_then(|mid| self.player_by_p_id(mid))
            .map(|m| parent_display_name(&m.first_name, &m.family_name))
            .or_else(|| {
                lineage.and_then(|n| n.mother_id).and_then(|mid| {
                    self.social.lineages.get(&mid).map(|ln| ln.name.clone())
                })
            });
        let father_display = lineage
            .and_then(|n| n.father_id)
            .and_then(|fid| self.player_by_p_id(fid))
            .map(|f| parent_display_name(&f.first_name, &f.family_name))
            .or_else(|| {
                lineage.and_then(|n| n.father_id).and_then(|fid| {
                    self.social.lineages.get(&fid).map(|ln| ln.name.clone())
                })
            });

        // Haxe: player.partner.name (first name only)
        let partner_name = if p.partner_p_id != 0 {
            self.player_by_p_id(p.partner_p_id)
                .map(|partner| partner.first_name.clone())
        } else {
            None
        };

        let held_object_name = if p.held_id != 0 {
            self.content
                .get(p.held_id)
                .map(|d| d.name.clone())
                .or_else(|| Some(format!("object {}", p.held_id)))
        } else {
            None
        };
        let held_name_for_weapon = held_object_name.as_deref().unwrap_or("");
        let holding_weapon = is_holding_weapon(p.held_id, held_name_for_weapon);

        let tile_temperature =
            get_tile_temperature(&self.world_map_time, p.x, p.y).unwrap_or(p.last_temperature);

        let farm_assigned = p
            .farm_profession
            .assigned_profession
            .map(|j| j.as_str());
        let farm_last = p.farm_profession.last_profession.map(|j| j.as_str());
        let (assigned, last) = sticky_profession_pair(
            p.smith_profession.is_assigned_smith,
            p.smith_profession.is_last_smith,
            p.baker_profession.is_assigned_baker,
            p.baker_profession.is_last_baker,
            farm_assigned,
            farm_last,
            p.assigned_profession.as_deref(),
            p.last_profession.as_deref(),
        );

        // Haxe TimeHelper.SeasonText (refreshed on DoSeason reseed).
        let season_text = if self.environment.season_text.is_empty() {
            crate::player_soul::haxe_season_text(
                self.environment.season.as_str(),
                self.environment.season_hardness,
            )
        } else {
            self.environment.season_text.clone()
        };

        let person_color = self.content.person_color(p.display_object_id);
        let is_wounded = self.combat.wound_of(p_id) > 0 || p.is_wounded_held(true);

        Some(super::SoulView {
            name: p.first_name.clone(),
            family_name: p.family_name.clone(),
            is_female,
            // Haxe trueAge for soul age line
            true_age: p.true_age,
            prestige: self.player_prestige(p_id),
            prestige_class: self.player_prestige_class(p_id),
            partner_name,
            father_display,
            mother_display,
            food_store: p.food,
            food_store_max: p.food_max,
            is_wounded,
            // Haxe isSuperHot/isSuperCold with person-color thresholds
            is_super_hot: is_super_hot_for_person(p.heat, person_color),
            is_super_cold: is_super_cold_for_person(p.heat, person_color),
            heat: p.heat,
            tile_temperature,
            home: home_option(p.home_x, p.home_y),
            tx: p.x,
            ty: p.y,
            assigned_profession: assigned,
            last_profession: last,
            held_object_name,
            is_angry_or_terrified: is_angry_or_terrified(p.angry_time),
            is_holding_weapon: holding_weapon,
            // AI-TAKEOVER: takeover / offline / permanent AI email (not email-only).
            // Haxe: Connection.isAi / GlobalPlayerInstance.isAi
            is_ai: crate::ai_takeover::player_is_ai(p.connected, p.ai_controlled, &p.email),
            season_text,
        })
    }

    /// Haxe `player.playerSoul.getSoulText` via live [`SoulView`].
    // Haxe: PlayerSoul.getSoulText
    pub fn player_soul_text(&self, p_id: i32) -> Option<String> {
        self.soul_view_for(p_id).map(|v| get_soul_text(&v))
    }

    /// Haxe `player.playerSoul.getExternalIntro`.
    // Haxe: PlayerSoul.getExternalIntro
    pub fn player_external_intro(&self, p_id: i32) -> Option<String> {
        self.soul_view_for(p_id).map(|v| get_external_intro(&v))
    }

    /// Haxe `playerSoul.getMemoryText` for a living player.
    pub fn player_soul_memory_text(&self, p_id: i32) -> Option<String> {
        self.player_by_p_id(p_id).map(|p| p.soul.get_memory_text())
    }

    /// Haxe `playerSoul.getChatMemoryText`.
    pub fn player_soul_chat_text(&self, p_id: i32) -> Option<String> {
        self.player_by_p_id(p_id)
            .map(|p| p.soul.get_chat_memory_text())
    }

    /// Haxe `playerSoul.addInteraction` using [`SimState::ai_memory_max_entries`].
    // Haxe: PlayerSoul.addInteraction / ServerSettings.AiMemoryMaxEntries
    pub fn add_player_soul_interaction(
        &mut self,
        owner_p_id: i32,
        other_p_id: i32,
        other_name: &str,
        other_family: &str,
        ty: InteractionType,
        value: f32,
    ) -> bool {
        let max = self.ai_memory_max_entries.max(1);
        let Some(p) = self.player_by_p_id_mut(owner_p_id) else {
            return false;
        };
        p.soul.add_interaction(
            other_p_id,
            other_name,
            other_family,
            ty,
            value,
            max,
        );
        true
    }

    /// Haxe `playerSoul.addChatEntry` using [`SimState::ai_chat_memory_max_entries`].
    // Haxe: PlayerSoul.addChatEntry / ServerSettings.AiChatMemoryMaxEntries
    pub fn add_player_soul_chat_entry(
        &mut self,
        owner_p_id: i32,
        from_p_id: i32,
        from_name: &str,
        from_family: &str,
        message: &str,
        reply: &str,
    ) -> bool {
        let max = self.ai_chat_memory_max_entries.max(1);
        let Some(p) = self.player_by_p_id_mut(owner_p_id) else {
            return false;
        };
        p.soul.add_chat_entry(
            from_p_id,
            from_name,
            from_family,
            message,
            reply,
            max,
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spawn_player;
    use ol_content::ContentDb;
    use std::sync::Arc;

    fn test_content() -> Arc<ContentDb> {
        Arc::new(ContentDb::default())
    }

    #[test]
    fn ai_soul_wire_on_player_and_soul_view() {
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "ai@soultest");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.first_name = "Ada".into();
            p.family_name = "Stone".into();
            p.age = 22.0;
            p.true_age = 22.0;
            p.food = 5.0;
            p.food_max = 20.0;
            p.heat = 0.3;
            p.home_x = 100;
            p.home_y = 0;
            p.x = 0;
            p.y = 0;
            p.angry_time = 0.0;
            p.display_object_id = 19;
            p.smith_profession.is_assigned_smith = true;
            p.smith_profession.is_last_smith = true;
        }
        assert!(state.add_player_soul_interaction(
            p_id,
            9,
            "Bob",
            "Snow",
            InteractionType::AttackDamage,
            3.0
        ));
        let mem = state.player_soul_memory_text(p_id).unwrap();
        assert!(mem.contains("Bob Snow, AttackDamage += 3"), "{mem}");
        assert!(state.add_player_soul_chat_entry(p_id, 9, "Bob", "Snow", "hi", "hello"));
        let chat = state.player_soul_chat_text(p_id).unwrap();
        assert!(chat.contains("hi"), "{chat}");
        let soul = state.player_soul_text(p_id).unwrap();
        assert!(soul.contains("You are Ada Stone"), "{soul}");
        assert!(soul.contains("female"), "{soul}");
        assert!(soul.contains("Your profession: SMITH"), "{soul}");
        assert!(
            soul.contains("Your home is 100 miles to the east"),
            "{soul}"
        );
        let intro = state.player_external_intro(p_id).unwrap();
        assert!(intro.contains("communicating with Ada Stone"), "{intro}");
        // AI email → external intro shows profession
        assert!(
            intro.contains("Her profession is SMITH") || intro.contains("His profession is SMITH"),
            "{intro}"
        );
    }

    #[test]
    fn soul_view_partner_line() {
        let mut state = SimState::with_default_empty(test_content());
        let a = spawn_player(&mut state, 1, "ai@a");
        let b = spawn_player(&mut state, 2, "ai@b");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.first_name = "Ada".into();
            p.family_name = "Stone".into();
            p.true_age = 20.0;
            p.partner_p_id = b;
        }
        {
            let p = state.players.get_mut(&2).unwrap();
            p.first_name = "Bob".into();
            p.family_name = "Snow".into();
            p.partner_p_id = a;
        }
        let soul = state.player_soul_text(a).unwrap();
        assert!(soul.contains("Your partner is Bob!"), "{soul}");
    }

    #[test]
    fn soul_view_farm_profession_and_season_hardness() {
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "ai@farm");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.first_name = "Fay".into();
            p.family_name = "Field".into();
            p.true_age = 18.0;
            assert!(crate::assign_farm_from_speech(
                &mut p.farm_profession,
                "FARMER!"
            ));
        }
        state.environment.season = crate::environment::Season::Winter;
        state.environment.season_hardness = 1.3_f32 * 1.3_f32;
        state.environment.season_text = "A hard  Winter".into();
        let soul = state.player_soul_text(p_id).unwrap();
        assert!(soul.contains("Your profession: BASICFARMER"), "{soul}");
        assert!(soul.contains("It is currently A hard  Winter."), "{soul}");
    }

    #[test]
    fn soul_view_super_hot_status_line() {
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "human@x");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.first_name = "Hot".into();
            p.family_name = "Sun".into();
            p.true_age = 30.0;
            p.heat = 0.85; // > 0.8 base threshold
            p.food = 20.0;
            p.food_max = 20.0;
        }
        let soul = state.player_soul_text(p_id).unwrap();
        assert!(soul.contains("You are very hot."), "{soul}");
    }

    #[test]
    fn add_interaction_fifo_respects_sim_state_cap() {
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "ai@cap");
        state.ai_memory_max_entries = 2;
        assert!(state.add_player_soul_interaction(p_id, 1, "A", "X", InteractionType::ServedFood, 1.0));
        assert!(state.add_player_soul_interaction(p_id, 2, "B", "X", InteractionType::ServedFood, 1.0));
        assert!(state.add_player_soul_interaction(p_id, 3, "C", "X", InteractionType::ServedFood, 1.0));
        let p = state.player_by_p_id(p_id).unwrap();
        assert_eq!(p.soul.memory_len(), 2);
        assert!(p.soul.interaction(1).is_none());
        assert!(p.soul.interaction(2).is_some());
        assert!(p.soul.interaction(3).is_some());
    }

    #[test]
    fn free_profession_strings_override_sticky() {
        let mut state = SimState::with_default_empty(test_content());
        let p_id = spawn_player(&mut state, 1, "ai@free");
        {
            let p = state.players.get_mut(&1).unwrap();
            p.first_name = "Sid".into();
            p.family_name = "Forge".into();
            p.true_age = 25.0;
            p.smith_profession.is_assigned_smith = true;
            p.assigned_profession = Some("SHEPHERD".into());
            p.last_profession = Some("SHEPHERD".into());
        }
        let soul = state.player_soul_text(p_id).unwrap();
        assert!(soul.contains("Your profession: SHEPHERD"), "{soul}");
        assert!(!soul.contains("SMITH"), "{soul}");
    }
}
