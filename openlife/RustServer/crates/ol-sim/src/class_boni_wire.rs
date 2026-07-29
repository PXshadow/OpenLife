//! CLASS-BONI helpers re-export surface (optional; core lives in `prestige` + `birth_fitness`).
//!
//! Kept as a documentation anchor for the prestige_class_table chunk.
//! // Haxe: GlobalPlayerInstance.calculateClassBoni / Lineage.PrestigeClasses

pub use crate::prestige::{
    calculate_class_boni, prestige_class_name_at_index, PrestigeClass, CLASS_BONI_NOBLE_SERF,
    CLASS_BONI_SAME, PRESTIGE_CLASS_NAMES,
};
