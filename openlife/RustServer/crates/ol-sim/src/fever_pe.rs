//! Fever / PE ladder — canonical in **`ol-combat-rules`**.
pub use ol_combat_rules::*;

#[cfg(test)]
mod person_color_tests {
    use super::*;
    use crate::player_soul::{is_super_hot_for_person, PERSON_COLOR_BLACK, PERSON_COLOR_BROWN};

    #[test]
    fn black_person_color_needs_higher_heat_for_heatstroke() {
        assert!(is_super_hot_for_person(0.81, 0));
        assert!(!is_super_hot_for_person(0.8, 0));
        assert!(!is_super_hot_for_person(0.89, PERSON_COLOR_BLACK));
        assert!(is_super_hot_for_person(0.91, PERSON_COLOR_BLACK));
        assert!(!is_super_hot_for_person(0.85, PERSON_COLOR_BROWN));
        assert!(is_super_hot_for_person(0.86, PERSON_COLOR_BROWN));
        let mut inp = UpdateEmotesInput {
            is_wounded: false,
            angry_time: 6.0,
            holding_weapon: false,
            attacker_mutual_weapon: false,
            has_yellow_fever: true,
            is_super_hot: is_super_hot_for_person(0.85, PERSON_COLOR_BLACK),
            is_super_cold: false,
            food_store: 10.0,
            age: 20.0,
            min_age_to_eat: UPDATE_EMOTES_MIN_AGE_TO_EAT,
            combat_angry_before_attack: 5.0,
            secs_since_ambient_emote: 0.0,
        };
        assert_eq!(resolve_update_emotes(&inp).emotes, vec![EMOTE_YELLOW_FEVER]);
        inp.is_super_hot = is_super_hot_for_person(0.91, PERSON_COLOR_BLACK);
        assert_eq!(resolve_update_emotes(&inp).emotes, vec![EMOTE_HEAT_STROKE]);
    }
}
