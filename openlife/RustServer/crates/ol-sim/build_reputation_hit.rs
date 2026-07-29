//! Build-time wire for **REPUTATION-HIT** / hit_reputation + **PRESTIGE-ALLY-COST**.
//!
//! Idempotent patches to `src/lib.rs`:
//! - export pure helpers
//! - `apply_connecting_hit_reputation` free fn
//! - HIT Wound + Kill + SAY KILL paths
//! - integration tests
//! - PRESTIGE-ALLY-COST: runs `src/_apply_prestige_ally_cost.py` then verifies markers

use std::path::Path;
use std::process::Command;

fn base_reputation_wired(lib: &str) -> bool {
    lib.contains("fn apply_connecting_hit_reputation")
        && lib.contains("compute_hit_reputation")
        && lib.contains("REPUTATION-HIT: Haxe kill lostCombatPrestige after DoDamage (wound)")
        && lib.contains("say_hit_wound_applies_reputation_float")
}

/// Fully wired when REPUTATION-HIT core + PRESTIGE-ALLY-COST peer test / live factors land.
pub fn reputation_hit_wired(lib: &str) -> bool {
    base_reputation_wired(lib)
        && lib.contains("say_hit_peer_ally_prestige_cost_and_gm")
        && lib.contains("PRESTIGE-ALLY-COST")
        && lib.contains("compute_hit_reputation_with_factors")
        && lib.contains("PrestigeCostFactors")
}

fn run_prestige_ally_python(src: &Path) {
    let py = src.join("_apply_prestige_ally_cost.py");
    if !py.exists() {
        eprintln!(
            "cargo:warning=PRESTIGE-ALLY-COST: missing {}",
            py.display()
        );
        return;
    }
    let status = Command::new("python")
        .arg(&py)
        .status()
        .or_else(|_| Command::new("python3").arg(&py).status());
    match status {
        Ok(s) if s.success() => {
            println!("cargo:warning=PRESTIGE-ALLY-COST: python apply ok");
        }
        Ok(s) => {
            eprintln!(
                "cargo:warning=PRESTIGE-ALLY-COST: python apply exit {:?}",
                s.code()
            );
        }
        Err(e) => {
            eprintln!("cargo:warning=PRESTIGE-ALLY-COST: python not runnable: {e}");
        }
    }
}

pub fn patch_reputation_hit(lib_path: &Path) -> bool {
    let Some(src) = lib_path.parent() else {
        return false;
    };
    // PRESTIGE-ALLY-COST first (idempotent python)
    run_prestige_ally_python(src);

    let Ok(mut text) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    if reputation_hit_wired(&text) {
        return true;
    }

    // If only prestige is missing, python should have fixed it — re-read once more
    if base_reputation_wired(&text) {
        run_prestige_ally_python(src);
        let Ok(t2) = std::fs::read_to_string(lib_path) else {
            return false;
        };
        if reputation_hit_wired(&t2) {
            return true;
        }
        eprintln!(
            "cargo:warning=PRESTIGE-ALLY-COST: base REPUTATION-HIT ok but peer ally markers still missing"
        );
        // fall through to legacy REPUTATION-HIT patch path? no — base is ok
        return false;
    }

    let orig = text.clone();

    // --- pub use ---
    let old_use = "pub use reputation::{\n    format_reputation_query, is_dangerous_lost_combat, label_from_lost_combat,\n    label_from_reputation, lost_combat_from_reputation, reputation_from_lost_combat,\n    ReputationBook, ReputationLabel,\n};";
    let new_use = "pub use reputation::{\n    attack_was_legit, compute_hit_reputation, format_reputation_query, is_dangerous_lost_combat,\n    label_from_lost_combat, label_from_reputation, lost_combat_from_reputation,\n    reputation_from_lost_combat, HitReputationDelta, HitReputationInput, ReputationBook,\n    ReputationLabel, DEVIL_MASK_CLOTHING_ID, ELDERLY_AGE_YEARS, MIN_AGE_TO_EAT_YEARS,\n    PRESTIGE_COST_PER_DAMAGE_ALLY, PRESTIGE_COST_PER_DAMAGE_CHILD,\n    PRESTIGE_COST_PER_DAMAGE_CLOSE_RELATIVE, PRESTIGE_COST_PER_DAMAGE_ELDERLY,\n    PRESTIGE_COST_PER_DAMAGE_WOMAN_UNARMED,\n};";
    if text.contains(old_use) {
        text = text.replacen(old_use, new_use, 1);
    } else if !text.contains("HitReputationInput") {
        eprintln!("cargo:warning=REPUTATION-HIT: pub use reputation block not matched");
    }

    text = text.replace(
        "/// Combat reputation floats (≠ prestige / PrestigeClass); updated on illegal/legal kill.",
        "/// Combat reputation floats (≠ prestige / PrestigeClass); updated on every connecting HIT (REPUTATION-HIT).",
    );

    // --- helper ---
    if !text.contains("fn apply_connecting_hit_reputation(") {
        let helper = r#"
/// True if any clothing slot / helper holds `id` (Haxe getClothingById).
// Haxe: GlobalPlayerInstance.getClothingById
fn player_wears_clothing_id(p: &Player, id: i32) -> bool {
    if id <= 0 {
        return false;
    }
    if p.hat == id || p.chest == id || p.shoes == id {
        return true;
    }
    p.clothing_helpers
        .iter()
        .any(|h| h.as_ref().map(|x| x.id == id).unwrap_or(false))
}

/// REPUTATION-HIT: apply Haxe kill post-DoDamage `lostCombatPrestige` on a connecting hit.
///
/// Updates [`SimState::reputation`] and mirrors into `combat.stats.lost_combat_prestige`
/// for AI deadly-player scans. Prestige/health speech residual (addHealthAndPrestige / GM).
// Haxe: GlobalPlayerInstance.kill attackWasLegit / lostCombatPrestige after DoDamage
fn apply_connecting_hit_reputation(
    state: &mut SimState,
    killer_id: i32,
    target_id: i32,
    damage: f32,
    target_holding_weapon: bool,
    target_is_ally: bool,
) {
    if !damage.is_finite() || damage <= 0.0 {
        return;
    }
    let target_lost = state.reputation.lost_combat(target_id);
    let attacker_class = state.player_prestige_class(killer_id).as_i32();
    let target_class = state.player_prestige_class(target_id).as_i32();
    let close_rel = is_close_relative(&state.social, killer_id, target_id);
    // Snapshot fields first (avoid borrow conflicts with content lookups).
    let (target_true_age, target_is_cursed, target_display, has_red_mask) = {
        let tp = state.players.values().find(|p| p.p_id == target_id);
        let true_age = tp.map(|p| p.true_age).unwrap_or(20.0);
        let cursed = tp.map(|p| p.is_cursed).unwrap_or(false);
        let display = tp.map(|p| person_object_id(p)).unwrap_or(DEFAULT_PERSON_OBJECT);
        let red = state
            .players
            .values()
            .find(|p| p.p_id == killer_id)
            .map(|p| player_wears_clothing_id(p, DEVIL_MASK_CLOTHING_ID))
            .unwrap_or(false);
        (true_age, cursed, display, red)
    };
    let (name, desc) = state
        .content
        .get(target_display)
        .map(|d| (d.name.as_str(), d.description.as_str()))
        .unwrap_or(("", ""));
    let target_is_female = person_looks_female(target_display, name, desc);
    let input = HitReputationInput {
        damage,
        target_lost_combat: target_lost,
        target_holding_weapon,
        attacker_prestige_class: attacker_class,
        target_prestige_class: target_class,
        target_true_age,
        target_is_ally,
        target_is_close_relative: close_rel,
        target_is_female,
        target_is_cursed,
        attacker_has_red_mask: has_red_mask,
    };
    let delta = compute_hit_reputation(&input);
    state.reputation.apply_hit_delta(killer_id, target_id, &delta);
    // Mirror Haxe GPI.lostCombatPrestige for combat stats / AI
    if delta.attacker_lost_delta != 0.0 {
        let s = state.combat.stats_mut(killer_id);
        s.lost_combat_prestige += delta.attacker_lost_delta;
    }
    if delta.target_lost_delta != 0.0 {
        let s = state.combat.stats_mut(target_id);
        s.lost_combat_prestige += delta.target_lost_delta;
    }
}

"#;
        let anchor = "/// How to apply a bloody-weapon transform (COMBAT-BLOODY).";
        if text.contains(anchor) {
            text = text.replacen(anchor, &format!("{helper}{anchor}"), 1);
        } else {
            eprintln!("cargo:warning=REPUTATION-HIT: helper anchor not found");
            return false;
        }
    }

    // --- HIT wound ---
    if !text.contains("REPUTATION-HIT: Haxe kill lostCombatPrestige after DoDamage (wound)") {
        let marker = "                            kp.last_attacked_player_id = target_id;\n                        }\n                        // COMBAT-BLOODY: DoDamage weapon → bloody newActor + cool-down.\n                        // Haxe: setHeldObject + SendUpdateToAllClosePlayers\n                        if let Some(cid) = apply_bloody_weapon_transform(\n                            state,\n                            killer_id,\n                            held_id,\n                            BloodyApplyMode::Strike { long_wounding: w <= 1 },\n                        ) {\n                            send_player_update_and_frame(state, outbound, cid);\n                        }\n                        let line = format!(\n                            \"{} HIT {} WOUND {} dmg={:.1}\",\n                            killer_id, target_id, w, dmg\n                        );";
        let repl = "                            kp.last_attacked_player_id = target_id;\n                        }\n                        // REPUTATION-HIT: Haxe kill lostCombatPrestige after DoDamage (wound).\n                        // Haxe: attackWasLegit / illegal guilt on every connecting hit\n                        apply_connecting_hit_reputation(\n                            state,\n                            killer_id,\n                            target_id,\n                            dmg,\n                            target_holding_weapon,\n                            target_is_ally,\n                        );\n                        // COMBAT-BLOODY: DoDamage weapon → bloody newActor + cool-down.\n                        // Haxe: setHeldObject + SendUpdateToAllClosePlayers\n                        if let Some(cid) = apply_bloody_weapon_transform(\n                            state,\n                            killer_id,\n                            held_id,\n                            BloodyApplyMode::Strike { long_wounding: w <= 1 },\n                        ) {\n                            send_player_update_and_frame(state, outbound, cid);\n                        }\n                        let line = format!(\n                            \"{} HIT {} WOUND {} dmg={:.1}\",\n                            killer_id, target_id, w, dmg\n                        );";
        if text.contains(marker) {
            text = text.replacen(marker, repl, 1);
        } else {
            eprintln!("cargo:warning=REPUTATION-HIT: HIT wound marker not found");
        }
    }

    // --- HIT kill ---
    if !text.contains("REPUTATION-HIT: same float path as wound") {
        let old_kill = "                    HitResult::Kill => {\n                        if legal {\n                            state\n                                .reputation\n                                .apply_legal_hit(killer_id, target_id, 0.2);\n                        } else {\n                            state\n                                .reputation\n                                .apply_illegal_hit(killer_id, 1.0, 1.0);\n                        }\n                        state.sync_lineage_prestige_from_combat(killer_id);";
        let new_kill = "                    HitResult::Kill => {\n                        // REPUTATION-HIT: same float path as wound (damage-scaled), not fixed 1.0.\n                        // Exile \"legal\" flag remains for death_reason / scoreboard only.\n                        apply_connecting_hit_reputation(\n                            state,\n                            killer_id,\n                            target_id,\n                            dmg,\n                            target_holding_weapon,\n                            target_is_ally,\n                        );\n                        state.sync_lineage_prestige_from_combat(killer_id);";
        if text.contains(old_kill) {
            text = text.replacen(old_kill, new_kill, 1);
        } else {
            eprintln!("cargo:warning=REPUTATION-HIT: HIT kill rep block not found");
        }
    }

    // --- SAY KILL ---
    if !text.contains("REPUTATION-HIT: one-shot KILL uses damage=1.0") {
        let old_say = "                if state.combat.resolve_kill(killer_id, target_id, legal) {\n                    state.combat.clear_wound(target_id);\n                    // Combat reputation (≠ prestige): illegal guilt / legal recover.\n                    if legal {\n                        state\n                            .reputation\n                            .apply_legal_hit(killer_id, target_id, 0.2);\n                    } else {\n                        state\n                            .reputation\n                            .apply_illegal_hit(killer_id, 1.0, 1.0);\n                    }\n                    // Keep lineage prestige/class in sync with combat prestige.";
        let new_say = "                if state.combat.resolve_kill(killer_id, target_id, legal) {\n                    state.combat.clear_wound(target_id);\n                    // REPUTATION-HIT: one-shot KILL uses damage=1.0 through full Haxe rules.\n                    let (t_armed, t_ally) = {\n                        let th = state\n                            .players\n                            .values()\n                            .find(|x| x.p_id == target_id)\n                            .map(|tp| tp.held_id)\n                            .unwrap_or(0);\n                        let tname = held_object_name(state, th);\n                        let armed = is_holding_weapon(th, &tname);\n                        let ally =\n                            is_leadership_ally(&state.social.following, killer_id, target_id);\n                        (armed, ally)\n                    };\n                    apply_connecting_hit_reputation(\n                        state,\n                        killer_id,\n                        target_id,\n                        1.0,\n                        t_armed,\n                        t_ally,\n                    );\n                    // Keep lineage prestige/class in sync with combat prestige.";
        if text.contains(old_say) {
            text = text.replacen(old_say, new_say, 1);
        } else {
            eprintln!("cargo:warning=REPUTATION-HIT: SAY KILL rep block not found");
        }
    }

    // --- tests ---
    if !text.contains("say_hit_wound_applies_reputation_float") {
        let tests = r#"
    /// REPUTATION-HIT: connecting HIT wound worsens attacker reputation by damage (illegal).
    // Haxe: GlobalPlayerInstance.kill lostCombatPrestige after DoDamage
    #[test]
    fn say_hit_wound_applies_reputation_float() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let mut rx1 = hub.register(1);
        let mut rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        let a = spawn_player(&mut state, 1, "rep@a");
        let b = spawn_player(&mut state, 2, "rep@b");
        state.social.ensure_lineage(a, "A");
        state.social.ensure_lineage(b, "B");
        state.players.get_mut(&1).unwrap().x = 0;
        state.players.get_mut(&1).unwrap().y = 0;
        state.players.get_mut(&2).unwrap().x = 1;
        state.players.get_mut(&2).unwrap().y = 0;
        // Adult male target, unarmed, zero lost combat → full illegal guilt.
        state.players.get_mut(&2).unwrap().true_age = 25.0;
        state.players.get_mut(&2).unwrap().display_object_id = 352; // male skin
        while rx1.try_recv().is_ok() {}
        while rx2.try_recv().is_ok() {}

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("HIT {b}"),
            },
        );
        assert_eq!(state.combat.wound_of(b), 1);
        let lost = state.reputation.lost_combat(a);
        assert!(
            lost > 0.0,
            "attacker lost_combat should rise after illegal hit, got {lost}"
        );
        let stats_lost = state
            .combat
            .stats
            .get(&a)
            .map(|s| s.lost_combat_prestige)
            .unwrap_or(0.0);
        assert!(
            (stats_lost - lost).abs() < 1e-3,
            "combat.stats.lost_combat_prestige should mirror book: stats={stats_lost} book={lost}"
        );
        assert_eq!(state.reputation.lost_combat(b), 0.0);
    }

    /// REPUTATION-HIT: attacking a high-lost target is legit → both recover half damage.
    #[test]
    fn say_hit_legit_recovers_both_reputation() {
        let counters = Counters::new();
        let hub = OutboundHub::new();
        let _rx1 = hub.register(1);
        let _rx2 = hub.register(2);
        let mut state = SimState::with_default_empty(test_content());
        let a = spawn_player(&mut state, 1, "legit@a");
        let b = spawn_player(&mut state, 2, "legit@b");
        state.social.ensure_lineage(a, "A");
        state.social.ensure_lineage(b, "B");
        state.players.get_mut(&1).unwrap().x = 0;
        state.players.get_mut(&1).unwrap().y = 0;
        state.players.get_mut(&2).unwrap().x = 1;
        state.players.get_mut(&2).unwrap().y = 0;
        state.players.get_mut(&2).unwrap().true_age = 25.0;
        state.players.get_mut(&2).unwrap().display_object_id = 352;
        state.reputation.set_from_lost_combat(b, 50.0);
        state.combat.stats_mut(b).lost_combat_prestige = 50.0;
        state.reputation.set_from_lost_combat(a, 10.0);
        state.combat.stats_mut(a).lost_combat_prestige = 10.0;
        let a_before = state.reputation.lost_combat(a);
        let b_before = state.reputation.lost_combat(b);

        apply_intent(
            &mut state,
            &counters,
            &hub,
            NetIntent::Raw {
                conn_id: 1,
                tag: "SAY".into(),
                payload: format!("HIT {b}"),
            },
        );
        assert_eq!(state.combat.wound_of(b), 1);
        let a_after = state.reputation.lost_combat(a);
        let b_after = state.reputation.lost_combat(b);
        assert!(
            a_after < a_before,
            "legit hit recovers attacker lost: before={a_before} after={a_after}"
        );
        assert!(
            b_after < b_before,
            "legit hit recovers target lost: before={b_before} after={b_after}"
        );
    }

"#;
        let ally_marker = "    /// ALLY-STRENGTH: HIT ally unarmed first hit warns (no wound); second hit exiles + damages.\n    // Haxe: GlobalPlayerInstance.kill unarmed ally gate\n    #[test]\n    fn say_hit_ally_first_warn_second_exiles() {";
        if text.contains(ally_marker) {
            text = text.replacen(ally_marker, &format!("{tests}\n{ally_marker}"), 1);
        } else {
            eprintln!("cargo:warning=REPUTATION-HIT: ally test marker not found");
        }
    }

    // Fix existing KILL reputation test for female default skin
    let old_kill_test = "        // Illegal kill worsens reputation; ?REP reports it.\n        apply_intent(\n            &mut state,\n            &counters,\n            &hub,\n            NetIntent::Raw {\n                conn_id: 1,\n                tag: \"SAY\".into(),\n                payload: format!(\"KILL {b}\"),\n            },\n        );\n        assert_eq!(state.reputation.get(a), -1.0);";
    let new_kill_test = "        // Illegal kill worsens reputation; ?REP reports it.\n        // Male adult target so woman/child prestige-cost branches do not fire.\n        if let Some(tp) = state.players.values_mut().find(|p| p.p_id == b) {\n            tp.display_object_id = 352;\n            tp.true_age = 25.0;\n        }\n        apply_intent(\n            &mut state,\n            &counters,\n            &hub,\n            NetIntent::Raw {\n                conn_id: 1,\n                tag: \"SAY\".into(),\n                payload: format!(\"KILL {b}\"),\n            },\n        );\n        assert_eq!(state.reputation.get(a), -1.0);";
    if text.contains(old_kill_test) {
        text = text.replacen(old_kill_test, new_kill_test, 1);
    }

    if text != orig {
        if let Err(e) = std::fs::write(lib_path, &text) {
            eprintln!("cargo:warning=REPUTATION-HIT: write failed: {e}");
            return false;
        }
    }

    // After base wire, apply prestige again
    run_prestige_ally_python(src);
    let Ok(final_text) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    let ok = reputation_hit_wired(&final_text);
    if ok {
        println!("cargo:warning=REPUTATION-HIT+PRESTIGE-ALLY-COST: wired");
    } else if base_reputation_wired(&final_text) {
        eprintln!("cargo:warning=REPUTATION-HIT: base ok; PRESTIGE-ALLY-COST still partial");
        // Accept base so builds do not loop forever if python unavailable
        return true;
    } else {
        eprintln!("cargo:warning=REPUTATION-HIT: partial wire — re-check markers");
    }
    ok || base_reputation_wired(&final_text)
}
