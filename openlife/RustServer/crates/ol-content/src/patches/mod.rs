//! Haxe **content patches** — not LiveSettings.
//!
//! Haxe parked `PatchObjectData` and `PatchTransitions` inside
//! `settings/ServerSettings.hx`. Those mutate object/transition tables at boot.
//! They are **not** `server.toml` knobs. Rust keeps them in this unit and applies
//! them once after text/binary load (`apply_all_haxe_content_patches`).
//!
//! | Haxe | This unit |
//! |------|-----------|
//! | `ServerSettings.PatchObjectData` | [`object_data.inc.rs`](object_data.inc.rs) |
//! | `ServerSettings.PatchTransitions` (aiShouldIgnore) | `ai_should_ignore_patches.inc.rs` |
//! | `PatchTransitions` (alt outcome / fortify) | `alt_outcome_patches.inc.rs` |
//! | `PatchTransitions` (hungryWorkCost) | `hungry_work_cost_patches.inc.rs` |
//! | `PatchTransitions` (horse cart / hitch) | [`transitions_horse.inc.rs`](transitions_horse.inc.rs) |
//! | `PatchObjectData` remainder | [`object_data_remainder.inc.rs`](object_data_remainder.inc.rs) |
//! | `PatchTransitions` remainder | [`transitions_remainder.inc.rs`](transitions_remainder.inc.rs) |
//!
//! Importer steps that are **not** settings patches stay in `lib_tail.inc.rs`:
//! category expand, `changeToolTransitions`, `apply_animal_moves_from_transitions`.

use crate::{ContentDb, ObjectDef, Transition};
use std::collections::HashMap;

include!("../ai_should_ignore_patches.inc.rs");
include!("../alt_outcome_patches.inc.rs");
include!("../hungry_work_cost_patches.inc.rs");
include!("object_data.inc.rs");
include!("transitions_horse.inc.rs");
include!("haxe_parity_helpers.inc.rs");
include!("object_data_remainder.inc.rs");
include!("transitions_remainder.inc.rs");

/// Apply every Haxe `PatchObjectData` / `PatchTransitions` table.
///
/// Call after category expand + `change_tool_transitions`, before stamping
/// animal `moves` from time-transitions.
pub fn apply_all_haxe_content_patches(db: &mut ContentDb) {
    // PatchObjectData
    apply_default_second_time_outcomes(db);
    apply_default_decay_object_patches(db);
    apply_default_contain_size_patches(db);
    apply_default_use_chance_patches(db);
    apply_default_switch_number_of_uses_patches(db);
    // PatchTransitions (same order as former lib.rs boot)
    apply_default_horse_transition_patches(db);
    apply_default_alternative_outcome_patches(db);
    apply_default_ai_should_ignore_patches(db);
    apply_default_hungry_work_cost_patches(db);
    apply_default_weapon_range_patches(db);
    apply_default_min_pickup_age_patches(db);
    apply_default_animal_deadly_distance_patches(db);
    apply_default_combat_damage_patches(db);
    apply_default_mosquito_map_chance_patches(db);
    apply_default_clothing_prestige_patches(db);
    // Haxe ServerSettings.parseTags / InitVanillaObjectIdMap (PatchObjectData loop)
    crate::vanilla_id::init_vanilla_object_id_map(db);
    apply_haxe_patch_object_data_remainder(db);
    apply_haxe_patch_transitions_remainder(db);
    // PatchObjectData + PatchTransitions 1:1 (IDs audited vs Haxe). Remaining
    // ServerSettings *knobs* (not table rewrites) live in LiveSettings / field_map.
    // SETTINGS-KNOB-TAIL done (DisplayScoreOn, MaxCoinsPerChest, birth chances,
    // tile-temp rates, SpwanAtLastDead). Wool/RabbitFur times stay patch constants.
}
