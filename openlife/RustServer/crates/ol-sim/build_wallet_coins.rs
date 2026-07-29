//! Build-time wire for **WALLET-COINS** / take_coins.
//!
//! Idempotent: de-dup weapon_wound header; ensure pure/live takeCoins markers.
//! Primary source wire is already in tree; this module is a safety net.
//!
//! // Haxe: GlobalPlayerInstance.takeCoins

use std::path::Path;

fn normalize_nl(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

fn restore_nl(s: &str, crlf: bool) -> String {
    if crlf {
        s.replace('\n', "\r\n")
    } else {
        s.to_string()
    }
}

/// True when pure + live wallet takeCoins paths are present.
pub fn wallet_coins_wired(weapon_wound: &str, economy: &str, lib: &str) -> bool {
    weapon_wound.contains("take_coins_say_text")
        && weapon_wound.contains("coins_stolen_on_wound")
        && economy.contains("take_coins_on_wound")
        && lib.contains("fn apply_take_coins_on_wound")
        && lib.contains("WALLET-COINS: Haxe takeCoins on food_store_max")
        && lib.contains("if take_coins {")
}

pub fn patch_wallet_coins(ol_sim_src: &Path, _workspace: &Path) -> bool {
    let ww = ol_sim_src.join("weapon_wound.rs");
    let eco = ol_sim_src.join("economy.rs");
    let lib = ol_sim_src.join("lib.rs");
    let _ = fix_weapon_wound_header(&ww);

    let ww_t = std::fs::read_to_string(&ww).unwrap_or_default();
    let eco_t = std::fs::read_to_string(&eco).unwrap_or_default();
    let lib_t = std::fs::read_to_string(&lib).unwrap_or_default();
    wallet_coins_wired(&ww_t, &eco_t, &lib_t)
}

fn fix_weapon_wound_header(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let t = normalize_nl(&raw);
    let count = t.matches("Chunk **WALLET-COINS**").count();
    if count <= 1 {
        return true;
    }
    let good = "//! Haxe `GlobalPlayerInstance.DoDamage` **weapon+0** / **animal+0** wound path.\n\
//!\n\
//! Chunk **WEAPON-WOUND-TRANS** / `weapon_zero`:\n\
//! - `GetTransition(weapon, 0, lastUseActor=true)` then non-LA\n\
//! - `woundFactor` gate vs `food_store_max` / not-reduced max\n\
//! - equip `newTargetID` wound on held, or ground-place when arrow wound / !doWound\n\
//! - attacker held → content `newActorID` (bloody) + cool-down TTC\n\
//!\n\
//! Chunk **WEAPON-ANIMAL-ZERO** / `animal_wound_zero`:\n\
//! - Same `GetTransition(animal, 0)` + doWound equip/ground as weapon path\n\
//! - `attacker == null` → `fromObj.id = trans.newActorID` (attacking form residual)\n\
//! - cool-down TTC on animal via `-1` auto-decay × WeaponCoolDownFactor*\n\
//! - `takeCoins` skipped (no human attacker)\n\
//! - Animal retaliate bloody (Haxe commented out) — skipped\n\
//!\n\
//! Chunk **WALLET-COINS** / `take_coins`:\n\
//! - pure [`coins_stolen_on_wound`] + [`take_coins_say_text`]\n\
//! - live wallet gift path on lethal + first wound equip (human attacker)\n\
\n";
    // Drop leading //! doc until first non-doc line
    let mut body = t.as_str();
    while let Some(rest) = body.strip_prefix("//!") {
        if let Some(nl) = rest.find('\n') {
            body = &rest[nl + 1..];
        } else {
            body = "";
            break;
        }
    }
    // skip blank lines after docs
    while body.starts_with('\n') {
        body = &body[1..];
    }
    let next = format!("{good}{body}");
    let out = restore_nl(&next, crlf);
    if out == raw {
        return true;
    }
    if let Err(e) = std::fs::write(path, out) {
        eprintln!("cargo:warning=WALLET-COINS header fix {}: {e}", path.display());
        return false;
    }
    true
}
