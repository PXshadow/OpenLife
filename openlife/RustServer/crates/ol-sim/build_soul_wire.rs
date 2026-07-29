//! AI-SOUL-WIRE build-time: expand player_soul pub use + docs markers.
//! Idempotent.

use std::path::Path;

pub fn soul_wire_exports_ok(lib: &str) -> bool {
    lib.contains("is_angry_or_terrified")
        && lib.contains("haxe_season_text")
        && lib.contains("sticky_profession_pair")
        && lib.contains("email_looks_ai")
}

pub fn patch_lib_soul_wire(lib_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = raw.replace("\r\n", "\n").replace('\r', "\n");
    if soul_wire_exports_ok(&t) {
        return true;
    }

    let old = r#"pub use player_soul::{
    get_combat_prestige_label, get_external_family_text, get_external_intro,
    get_external_status_text, get_family_text, get_home_context_text, get_prestige_class_name,
    get_profession_text, get_soul_text, get_status_text, get_temperature_context_text,
    get_temperature_label, home_direction, ChatEntry, InteractionData, InteractionType,
    PlayerSoul, SoulView, AI_CHAT_MEMORY_MAX_ENTRIES, AI_MEMORY_MAX_ENTRIES,
};"#;

    let new = r#"pub use player_soul::{
    email_looks_ai, get_combat_prestige_label, get_external_family_text, get_external_intro,
    get_external_status_text, get_family_text, get_home_context_text, get_prestige_class_name,
    get_profession_text, get_soul_text, get_status_text, get_temperature_context_text,
    get_temperature_label, haxe_season_text, home_direction, home_option, is_angry_or_terrified,
    parent_display_name, person_looks_female, season_display_name, sticky_profession_pair,
    ChatEntry, InteractionData, InteractionType, PlayerSoul, SoulView, AI_CHAT_MEMORY_MAX_ENTRIES,
    AI_MEMORY_MAX_ENTRIES,
};"#;

    if !t.contains(old) {
        // Already partially updated or different formatting
        if soul_wire_exports_ok(&t) {
            return true;
        }
        return false;
    }
    t = t.replacen(old, new, 1);

    // Comment on mod player_soul
    t = t.replace(
        "// Haxe: openlife.server.PlayerSoul AI memory + prompt context (S-SOUL)\nmod player_soul;",
        "// Haxe: openlife.server.PlayerSoul AI memory + prompt context (S-SOUL + AI-SOUL-WIRE)\nmod player_soul;",
    );

    let out = if crlf {
        t.replace('\n', "\r\n")
    } else {
        t
    };
    std::fs::write(lib_path, out).is_ok() && soul_wire_exports_ok(&std::fs::read_to_string(lib_path).unwrap_or_default())
}
