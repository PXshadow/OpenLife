//! Build-time wire for **PRESTIGE-ALLY-COST** / ally_prestige_cost.
//!
//! Idempotent patches:
//! - pure: `PrestigeCostFactors` + `compute_hit_reputation_with_factors` in reputation.rs
//! - pure: `is_ally` already in relations.rs (source)
//! - LiveSettings: `PrestigeCostPerDamageForAlly` → GameplayKnobs
//! - live HIT: exile-aware `is_ally` after mid-hit exile + live ally cost factor
//! - tests: ally GM pure + peer-ally HIT prestige/GM
//!
//! // Haxe: GlobalPlayerInstance.kill PrestigeCostPerDamageForAlly + sendGlobalMessage

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

fn replace_once(hay: &mut String, old: &str, new: &str) -> bool {
    if let Some(i) = hay.find(old) {
        hay.replace_range(i..i + old.len(), new);
        true
    } else {
        false
    }
}

fn write_if_changed(path: &Path, original: &str, next: &str) -> bool {
    if original == next {
        return false;
    }
    if let Err(e) = std::fs::write(path, next) {
        eprintln!(
            "cargo:warning=PRESTIGE-ALLY-COST write {}: {e}",
            path.display()
        );
        return false;
    }
    true
}

/// True when pure + live ally prestige cost paths are present.
pub fn prestige_ally_cost_wired(
    reputation: &str,
    relations: &str,
    settings: &str,
    config_lib: &str,
    lib: &str,
) -> bool {
    reputation.contains("PrestigeCostFactors")
        && reputation.contains("compute_hit_reputation_with_factors")
        && reputation.contains("prestige_cost_category_ally_and_gm_text")
        && relations.contains("pub fn is_ally(")
        && settings.contains("prestige_cost_per_damage_for_ally")
        && config_lib.contains("prestige_cost_per_damage_for_ally")
        && lib.contains("PRESTIGE-ALLY-COST")
        && lib.contains("say_hit_peer_ally_prestige_cost_and_gm")
        && lib.contains("compute_hit_reputation_with_factors")
}

pub fn patch_prestige_ally_cost(ol_sim_src: &Path, workspace: &Path) -> bool {
    let rep = ol_sim_src.join("reputation.rs");
    let rel = ol_sim_src.join("relations.rs");
    let settings = ol_sim_src.join("settings_live.rs");
    let lib = ol_sim_src.join("lib.rs");
    let config = workspace.join("crates/ol-config/src/lib.rs");
    let field_map = workspace.join("crates/ol-config/src/field_map.rs");

    let _ = patch_reputation(&rep);
    let _ = patch_field_map(&field_map);
    let _ = patch_config_lib(&config);
    let _ = patch_settings_live(&settings);
    let _ = patch_lib(&lib);

    let rep_t = std::fs::read_to_string(&rep).unwrap_or_default();
    let rel_t = std::fs::read_to_string(&rel).unwrap_or_default();
    let set_t = std::fs::read_to_string(&settings).unwrap_or_default();
    let cfg_t = std::fs::read_to_string(&config).unwrap_or_default();
    let lib_t = std::fs::read_to_string(&lib).unwrap_or_default();
    prestige_ally_cost_wired(&rep_t, &rel_t, &set_t, &cfg_t, &lib_t)
}

fn patch_reputation(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("PrestigeCostFactors")
        && raw.contains("compute_hit_reputation_with_factors")
        && raw.contains("prestige_cost_category_ally_and_gm_text")
    {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;

    if !t.contains("pub struct PrestigeCostFactors") {
        ch |= replace_once(
            &mut t,
            "/// Haxe `ServerSettings.CombatReputationRestorePerYear` default.\n\
/// Calm restore rate: subtract `(rate * dt) / 60` from lostCombatPrestige per tick.\n\
// Haxe: ServerSettings.CombatReputationRestorePerYear\n\
pub const COMBAT_REPUTATION_RESTORE_PER_YEAR: f32 = 2.0;\n",
            "/// Haxe `ServerSettings.CombatReputationRestorePerYear` default.\n\
/// Calm restore rate: subtract `(rate * dt) / 60` from lostCombatPrestige per tick.\n\
// Haxe: ServerSettings.CombatReputationRestorePerYear\n\
pub const COMBAT_REPUTATION_RESTORE_PER_YEAR: f32 = 2.0;\n\
\n\
/// Live / test overrides for category prestige-cost multipliers (Haxe ServerSettings).\n\
// Haxe: PrestigeCostPerDamageFor* ServerSettings\n\
// PRESTIGE-ALLY-COST\n\
#[derive(Debug, Clone, Copy, PartialEq)]\n\
pub struct PrestigeCostFactors {\n\
    pub child: f32,\n\
    pub elderly: f32,\n\
    pub ally: f32,\n\
    pub close_relative: f32,\n\
    pub woman_unarmed: f32,\n\
}\n\
\n\
impl Default for PrestigeCostFactors {\n\
    fn default() -> Self {\n\
        Self {\n\
            child: PRESTIGE_COST_PER_DAMAGE_CHILD,\n\
            elderly: PRESTIGE_COST_PER_DAMAGE_ELDERLY,\n\
            ally: PRESTIGE_COST_PER_DAMAGE_ALLY,\n\
            close_relative: PRESTIGE_COST_PER_DAMAGE_CLOSE_RELATIVE,\n\
            woman_unarmed: PRESTIGE_COST_PER_DAMAGE_WOMAN_UNARMED,\n\
        }\n\
    }\n\
}\n\
\n\
impl PrestigeCostFactors {\n\
    /// Sanitize non-finite entries back to Haxe defaults.\n\
    pub fn sanitized(self) -> Self {\n\
        let pick = |v: f32, d: f32| if v.is_finite() && v >= 0.0 { v } else { d };\n\
        Self {\n\
            child: pick(self.child, PRESTIGE_COST_PER_DAMAGE_CHILD),\n\
            elderly: pick(self.elderly, PRESTIGE_COST_PER_DAMAGE_ELDERLY),\n\
            ally: pick(self.ally, PRESTIGE_COST_PER_DAMAGE_ALLY),\n\
            close_relative: pick(self.close_relative, PRESTIGE_COST_PER_DAMAGE_CLOSE_RELATIVE),\n\
            woman_unarmed: pick(self.woman_unarmed, PRESTIGE_COST_PER_DAMAGE_WOMAN_UNARMED),\n\
        }\n\
    }\n\
}\n",
        );
    }

    if !t.contains("pub fn compute_hit_reputation_with_factors") {
        ch |= replace_once(
            &mut t,
            "// Haxe: GlobalPlayerInstance.kill prestigeCost / lostCombatPrestige after DoDamage\n\
pub fn compute_hit_reputation(input: &HitReputationInput) -> HitReputationDelta {\n\
    let damage = if input.damage.is_finite() {\n\
        input.damage.max(0.0)\n\
    } else {\n\
        0.0\n\
    };\n",
            "// Haxe: GlobalPlayerInstance.kill prestigeCost / lostCombatPrestige after DoDamage\n\
pub fn compute_hit_reputation(input: &HitReputationInput) -> HitReputationDelta {\n\
    compute_hit_reputation_with_factors(input, &PrestigeCostFactors::default())\n\
}\n\
\n\
/// Same as [`compute_hit_reputation`] with live/test [`PrestigeCostFactors`].\n\
// Haxe: ServerSettings.PrestigeCostPerDamageFor*\n\
// PRESTIGE-ALLY-COST\n\
pub fn compute_hit_reputation_with_factors(\n\
    input: &HitReputationInput,\n\
    factors: &PrestigeCostFactors,\n\
) -> HitReputationDelta {\n\
    let factors = factors.sanitized();\n\
    let damage = if input.damage.is_finite() {\n\
        input.damage.max(0.0)\n\
    } else {\n\
        0.0\n\
    };\n",
        );

        ch |= replace_once(
            &mut t,
            "    if input.target_true_age < MIN_AGE_TO_EAT_YEARS {\n\
        prestige_cost = (cost_damage * PRESTIGE_COST_PER_DAMAGE_CHILD).ceil();\n\
        prestige_cost_category = PrestigeCostCategory::Child;\n\
    } else if input.target_true_age > ELDERLY_AGE_YEARS && !input.target_is_cursed {\n\
        prestige_cost = (cost_damage * PRESTIGE_COST_PER_DAMAGE_ELDERLY).ceil();\n\
        prestige_cost_category = PrestigeCostCategory::Elderly;\n\
    } else if input.target_is_ally && !input.target_is_cursed {\n\
        prestige_cost = (cost_damage * PRESTIGE_COST_PER_DAMAGE_ALLY).ceil();\n\
        prestige_cost_category = PrestigeCostCategory::Ally;\n\
    } else if input.target_is_close_relative && !input.target_is_cursed {\n\
        prestige_cost = (cost_damage * PRESTIGE_COST_PER_DAMAGE_CLOSE_RELATIVE).ceil();\n\
        prestige_cost_category = PrestigeCostCategory::CloseRelative;\n\
    } else if input.target_is_female && !input.target_is_cursed {\n\
        prestige_cost = (cost_damage * PRESTIGE_COST_PER_DAMAGE_WOMAN_UNARMED).ceil();\n\
        prestige_cost_category = PrestigeCostCategory::WomanUnarmed;\n\
    }\n",
            "    if input.target_true_age < MIN_AGE_TO_EAT_YEARS {\n\
        prestige_cost = (cost_damage * factors.child).ceil();\n\
        prestige_cost_category = PrestigeCostCategory::Child;\n\
    } else if input.target_true_age > ELDERLY_AGE_YEARS && !input.target_is_cursed {\n\
        prestige_cost = (cost_damage * factors.elderly).ceil();\n\
        prestige_cost_category = PrestigeCostCategory::Elderly;\n\
    } else if input.target_is_ally && !input.target_is_cursed {\n\
        // PRESTIGE-ALLY-COST: PrestigeCostPerDamageForAlly (live via factors.ally)\n\
        prestige_cost = (cost_damage * factors.ally).ceil();\n\
        prestige_cost_category = PrestigeCostCategory::Ally;\n\
    } else if input.target_is_close_relative && !input.target_is_cursed {\n\
        prestige_cost = (cost_damage * factors.close_relative).ceil();\n\
        prestige_cost_category = PrestigeCostCategory::CloseRelative;\n\
    } else if input.target_is_female && !input.target_is_cursed {\n\
        prestige_cost = (cost_damage * factors.woman_unarmed).ceil();\n\
        prestige_cost_category = PrestigeCostCategory::WomanUnarmed;\n\
    }\n",
        );
    }

    if !t.contains("prestige_cost_category_ally_and_gm_text") {
        ch |= replace_once(
            &mut t,
            "    fn prestige_cost_category_child_and_gm_text() {\n\
        let d = compute_hit_reputation(&HitReputationInput {\n\
            damage: 1.2,\n\
            target_true_age: 2.0,\n\
            ..Default::default()\n\
        });\n\
        assert_eq!(d.prestige_cost_category, PrestigeCostCategory::Child);\n\
        let msg = format_prestige_cost_global_message(d.prestige_cost, d.prestige_cost_category, \"Kid\")\n\
            .expect(\"msg\");\n\
        assert!(msg.contains(\"Lost 6 prestige\"), \"{msg}\");\n\
        assert!(msg.contains(\"a child Kid\"), \"{msg}\");\n\
    }\n",
            "    fn prestige_cost_category_child_and_gm_text() {\n\
        let d = compute_hit_reputation(&HitReputationInput {\n\
            damage: 1.2,\n\
            target_true_age: 2.0,\n\
            ..Default::default()\n\
        });\n\
        assert_eq!(d.prestige_cost_category, PrestigeCostCategory::Child);\n\
        let msg = format_prestige_cost_global_message(d.prestige_cost, d.prestige_cost_category, \"Kid\")\n\
            .expect(\"msg\");\n\
        assert!(msg.contains(\"Lost 6 prestige\"), \"{msg}\");\n\
        assert!(msg.contains(\"a child Kid\"), \"{msg}\");\n\
    }\n\
\n\
    /// PRESTIGE-ALLY-COST: ally category + GM text + live factor override.\n\
    // Haxe: PrestigeCostPerDamageForAlly + sendGlobalMessage ally\n\
    #[test]\n\
    fn prestige_cost_category_ally_and_gm_text() {\n\
        let d = compute_hit_reputation(&HitReputationInput {\n\
            damage: 2.0,\n\
            target_is_ally: true,\n\
            target_true_age: 20.0,\n\
            ..Default::default()\n\
        });\n\
        assert_eq!(d.prestige_cost_category, PrestigeCostCategory::Ally);\n\
        assert!((d.prestige_cost - 2.0).abs() < 1e-5);\n\
        let msg = format_prestige_cost_global_message(\n\
            d.prestige_cost,\n\
            d.prestige_cost_category,\n\
            \"Buddy\",\n\
        )\n\
        .expect(\"msg\");\n\
        assert!(msg.contains(\"Lost 2 prestige\"), \"{msg}\");\n\
        assert!(msg.contains(\"ally Buddy\"), \"{msg}\");\n\
\n\
        // Live factor 3× → ceil(2 * 3) = 6\n\
        let mut factors = PrestigeCostFactors::default();\n\
        factors.ally = 3.0;\n\
        let live = compute_hit_reputation_with_factors(\n\
            &HitReputationInput {\n\
                damage: 2.0,\n\
                target_is_ally: true,\n\
                target_true_age: 20.0,\n\
                ..Default::default()\n\
            },\n\
            &factors,\n\
        );\n\
        assert!((live.prestige_cost - 6.0).abs() < 1e-5);\n\
        assert_eq!(live.prestige_cost_category, PrestigeCostCategory::Ally);\n\
\n\
        // Cursed ally: no category cost\n\
        let cursed = compute_hit_reputation(&HitReputationInput {\n\
            damage: 2.0,\n\
            target_is_ally: true,\n\
            target_is_cursed: true,\n\
            target_true_age: 20.0,\n\
            ..Default::default()\n\
        });\n\
        assert_eq!(cursed.prestige_cost, 0.0);\n\
        assert_eq!(cursed.prestige_cost_category, PrestigeCostCategory::None);\n\
    }\n",
        );
    }

    write_if_changed(path, &raw, &restore_nl(&t, crlf)) || ch
}

fn patch_field_map(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("PrestigeCostPerDamageForAlly")
        && raw.contains("PRESTIGE_COST_PER_DAMAGE_FOR_ALLY")
    {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;

    if !t.contains("PRESTIGE_COST_PER_DAMAGE_FOR_ALLY") {
        ch |= replace_once(
            &mut t,
            "    pub const HIRE_COST_INCREASE_PER_PERSON: f32 = 10.0;\n\
}\n",
            "    pub const HIRE_COST_INCREASE_PER_PERSON: f32 = 10.0;\n\
    /// Haxe `PrestigeCostPerDamageForAlly`.\n\
    // Haxe: ServerSettings.PrestigeCostPerDamageForAlly = 1\n\
    // PRESTIGE-ALLY-COST\n\
    pub const PRESTIGE_COST_PER_DAMAGE_FOR_ALLY: f32 = 1.0;\n\
}\n",
        );
    }

    if !t.contains("haxe_name: \"PrestigeCostPerDamageForAlly\"") {
        ch |= replace_once(
            &mut t,
            "        haxe_name: \"HireCostIncreasePerPerson\",\n\
        rust_path: \"server.toml hire_cost_increase_per_person / LiveSettings / SimState.gameplay\",\n\
        home: SettingsHome::Live,\n\
    },\n\
    // --- intentional ModuleConst",
            "        haxe_name: \"HireCostIncreasePerPerson\",\n\
        rust_path: \"server.toml hire_cost_increase_per_person / LiveSettings / SimState.gameplay\",\n\
        home: SettingsHome::Live,\n\
    },\n\
    // PRESTIGE-ALLY-COST\n\
    FieldEntry {\n\
        haxe_name: \"PrestigeCostPerDamageForAlly\",\n\
        rust_path: \"server.toml prestige_cost_per_damage_for_ally / LiveSettings / SimState.gameplay\",\n\
        home: SettingsHome::Live,\n\
    },\n\
    // --- intentional ModuleConst",
        );
    }

    if !t.contains("\"PrestigeCostPerDamageForAlly\"") {
        ch |= replace_once(
            &mut t,
            "            \"HireCostIncreasePerPerson\",\n\
        ] {",
            "            \"HireCostIncreasePerPerson\",\n\
            \"PrestigeCostPerDamageForAlly\",\n\
        ] {",
        );
    }

    write_if_changed(path, &raw, &restore_nl(&t, crlf)) || ch
}

fn patch_config_lib(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("prestige_cost_per_damage_for_ally")
        && raw.contains("\"prestige_cost_per_damage_for_ally\"")
        && raw.contains("PRESTIGE_COST_PER_DAMAGE_FOR_ALLY")
    {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;

    // ServerConfig field (doc-comment style, end of struct)
    if !t.contains("pub prestige_cost_per_damage_for_ally: f32") {
        ch |= replace_once(
            &mut t,
            "    /// Haxe `ServerSettings.HireCostIncreasePerPerson`.\n\
    // Haxe: ServerSettings.HireCostIncreasePerPerson\n\
    pub hire_cost_increase_per_person: f32,\n\
}\n",
            "    /// Haxe `ServerSettings.HireCostIncreasePerPerson`.\n\
    // Haxe: ServerSettings.HireCostIncreasePerPerson\n\
    pub hire_cost_increase_per_person: f32,\n\
    /// Haxe `ServerSettings.PrestigeCostPerDamageForAlly`.\n\
    // Haxe: ServerSettings.PrestigeCostPerDamageForAlly\n\
    // PRESTIGE-ALLY-COST\n\
    pub prestige_cost_per_damage_for_ally: f32,\n\
}\n",
        );
    }

    // LiveSettings field
    if t.matches("pub prestige_cost_per_damage_for_ally: f32").count() < 2 {
        ch |= replace_once(
            &mut t,
            "    /// Haxe `HireCostIncreasePerPerson`.\n\
    pub hire_cost_increase_per_person: f32,\n\
}\n",
            "    /// Haxe `HireCostIncreasePerPerson`.\n\
    pub hire_cost_increase_per_person: f32,\n\
    /// Haxe `PrestigeCostPerDamageForAlly`.\n\
    // PRESTIGE-ALLY-COST\n\
    pub prestige_cost_per_damage_for_ally: f32,\n\
}\n",
        );
    }

    if !t.contains(
        "prestige_cost_per_damage_for_ally: gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_ALLY",
    ) {
        ch |= replace_once(
            &mut t,
            "            hire_cost_increase_per_person: gameplay_defaults::HIRE_COST_INCREASE_PER_PERSON,\n\
        }\n\
    }\n\
}\n\
\n\
/// Subset of config knobs",
            "            hire_cost_increase_per_person: gameplay_defaults::HIRE_COST_INCREASE_PER_PERSON,\n\
            prestige_cost_per_damage_for_ally: gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_ALLY,\n\
        }\n\
    }\n\
}\n\
\n\
/// Subset of config knobs",
        );
    }

    if !t.contains("self.prestige_cost_per_damage_for_ally") {
        ch |= replace_once(
            &mut t,
            "            hire_cost_increase_per_person: sanitize_nonneg_or(\n\
                self.hire_cost_increase_per_person,\n\
                gameplay_defaults::HIRE_COST_INCREASE_PER_PERSON,\n\
            ),\n\
        }\n\
    }\n",
            "            hire_cost_increase_per_person: sanitize_nonneg_or(\n\
                self.hire_cost_increase_per_person,\n\
                gameplay_defaults::HIRE_COST_INCREASE_PER_PERSON,\n\
            ),\n\
            prestige_cost_per_damage_for_ally: sanitize_nonneg_or(\n\
                self.prestige_cost_per_damage_for_ally,\n\
                gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_ALLY,\n\
            ),\n\
        }\n\
    }\n",
        );
    }

    if !t.contains("\"prestige_cost_per_damage_for_ally\",\n            (old.prestige_cost_per_damage_for_ally")
    {
        ch |= replace_once(
            &mut t,
            "        push(\n\
            \"hire_cost_increase_per_person\",\n\
            (old.hire_cost_increase_per_person - new.hire_cost_increase_per_person).abs()\n\
                > f32::EPSILON,\n\
        );\n\
        keys\n\
    }\n",
            "        push(\n\
            \"hire_cost_increase_per_person\",\n\
            (old.hire_cost_increase_per_person - new.hire_cost_increase_per_person).abs()\n\
                > f32::EPSILON,\n\
        );\n\
        push(\n\
            \"prestige_cost_per_damage_for_ally\",\n\
            (old.prestige_cost_per_damage_for_ally - new.prestige_cost_per_damage_for_ally).abs()\n\
                > f32::EPSILON,\n\
        );\n\
        keys\n\
    }\n",
        );
    }

    if !t.contains("\"prestige_cost_per_damage_for_ally\",\n        ]")
        && !t.contains("\"prestige_cost_per_damage_for_ally\",\n            \"")
    {
        ch |= replace_once(
            &mut t,
            "            \"hire_cost_increase_per_person\",\n\
        ]\n\
    }\n\
}\n",
            "            \"hire_cost_increase_per_person\",\n\
            \"prestige_cost_per_damage_for_ally\",\n\
        ]\n\
    }\n\
}\n",
        );
    }

    if !t.contains("prestige_cost_per_damage_for_ally: 2.5") {
        ch |= replace_once(
            &mut t,
            "            hire_cost_increase_per_person: 15.0,\n\
            // boot-only noise\n\
            game_port: 9999,\n",
            "            hire_cost_increase_per_person: 15.0,\n\
            prestige_cost_per_damage_for_ally: 2.5,\n\
            // boot-only noise\n\
            game_port: 9999,\n",
        );
    }

    if !t.contains("c.prestige_cost_per_damage_for_ally - 1.0") {
        ch |= replace_once(
            &mut t,
            "        assert!((c.hire_cost_increase_per_person - 10.0).abs() < f32::EPSILON);\n\
    }\n",
            "        assert!((c.hire_cost_increase_per_person - 10.0).abs() < f32::EPSILON);\n\
        assert!((c.prestige_cost_per_damage_for_ally - 1.0).abs() < f32::EPSILON);\n\
    }\n",
        );
    }

    // live_settings() extract — map field from self
    if !t.contains("prestige_cost_per_damage_for_ally: sanitize_nonneg_or(\n                self.prestige_cost_per_damage_for_ally")
        && t.contains("hire_cost_increase_per_person: sanitize_nonneg_or")
    {
        // already handled above
    }
    // Ensure live_settings assignment includes the field
    if !t.contains("prestige_cost_per_damage_for_ally: sanitize_nonneg_or") {
        // fallback: add after hire_cost_increase in live_settings body if missing
    }

    write_if_changed(path, &raw, &restore_nl(&t, crlf)) || ch
}

fn patch_settings_live(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("prestige_cost_per_damage_for_ally") {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;

    ch |= replace_once(
        &mut t,
        "    /// Haxe `HireCostIncreasePerPerson`.\n\
    // Haxe: ServerSettings.HireCostIncreasePerPerson\n\
    pub hire_cost_increase_per_person: f32,\n\
}\n",
        "    /// Haxe `HireCostIncreasePerPerson`.\n\
    // Haxe: ServerSettings.HireCostIncreasePerPerson\n\
    pub hire_cost_increase_per_person: f32,\n\
    /// Haxe `PrestigeCostPerDamageForAlly` (illegal ally hit category cost).\n\
    // Haxe: ServerSettings.PrestigeCostPerDamageForAlly\n\
    // PRESTIGE-ALLY-COST\n\
    pub prestige_cost_per_damage_for_ally: f32,\n\
}\n",
    );

    ch |= replace_once(
        &mut t,
        "            hire_cost: gameplay_defaults::HIRE_COST,\n\
            hire_cost_increase_per_person: gameplay_defaults::HIRE_COST_INCREASE_PER_PERSON,\n\
        }\n\
    }\n\
}\n\
\n\
impl GameplayKnobs {",
        "            hire_cost: gameplay_defaults::HIRE_COST,\n\
            hire_cost_increase_per_person: gameplay_defaults::HIRE_COST_INCREASE_PER_PERSON,\n\
            prestige_cost_per_damage_for_ally: gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_ALLY,\n\
        }\n\
    }\n\
}\n\
\n\
impl GameplayKnobs {",
    );

    ch |= replace_once(
        &mut t,
        "            hire_cost: live.hire_cost,\n\
            hire_cost_increase_per_person: live.hire_cost_increase_per_person,\n\
        }\n\
    }\n\
}\n",
        "            hire_cost: live.hire_cost,\n\
            hire_cost_increase_per_person: live.hire_cost_increase_per_person,\n\
            prestige_cost_per_damage_for_ally: live.prestige_cost_per_damage_for_ally,\n\
        }\n\
    }\n\
}\n",
    );

    ch |= replace_once(
        &mut t,
        "    push_gp(\n\
        \"hire_cost_increase_per_person\",\n\
        (old.hire_cost_increase_per_person - gp.hire_cost_increase_per_person).abs()\n\
            > f32::EPSILON,\n\
    );\n\
    state.gameplay = gp;\n",
        "    push_gp(\n\
        \"hire_cost_increase_per_person\",\n\
        (old.hire_cost_increase_per_person - gp.hire_cost_increase_per_person).abs()\n\
            > f32::EPSILON,\n\
    );\n\
    push_gp(\n\
        \"prestige_cost_per_damage_for_ally\",\n\
        (old.prestige_cost_per_damage_for_ally - gp.prestige_cost_per_damage_for_ally).abs()\n\
            > f32::EPSILON,\n\
    );\n\
    state.gameplay = gp;\n",
    );

    ch |= replace_once(
        &mut t,
        "            hire_cost: 25.0,\n\
            hire_cost_increase_per_person: 12.0,\n\
            ..Default::default()\n\
        }\n\
        .live_settings();\n",
        "            hire_cost: 25.0,\n\
            hire_cost_increase_per_person: 12.0,\n\
            prestige_cost_per_damage_for_ally: 2.0,\n\
            ..Default::default()\n\
        }\n\
        .live_settings();\n",
    );

    write_if_changed(path, &raw, &restore_nl(&t, crlf)) || ch
}

fn patch_lib(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if raw.contains("say_hit_peer_ally_prestige_cost_and_gm")
        && raw.contains("PRESTIGE-ALLY-COST")
        && raw.contains("compute_hit_reputation_with_factors")
        && raw.contains("PrestigeCostFactors")
    {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut ch = false;

    // relations pub use — add is_ally
    if !t.contains("is_ally, is_close_relative") && !t.contains("is_ally,") {
        ch |= replace_once(
            &mut t,
            "pub use relations::{\n\
    format_children_query, format_gen_query, format_relation_query, get_top_leader,\n\
    get_top_leader_or_self, is_close_relative, is_eve, is_leadership_ally, is_same_family,\n\
    living_children_of, relation_of, root_eve_id, top_leader, Relation,\n\
};",
            "pub use relations::{\n\
    format_children_query, format_gen_query, format_relation_query, get_top_leader,\n\
    get_top_leader_or_self, is_ally, is_close_relative, is_eve, is_leadership_ally, is_same_family,\n\
    living_children_of, relation_of, root_eve_id, top_leader, Relation,\n\
};",
        );
    }

    // reputation pub use
    if !t.contains("PrestigeCostFactors") {
        ch |= replace_once(
            &mut t,
            "pub use reputation::{\n\
    attack_was_legit, combat_reputation_restore_delta, compute_hit_reputation,\n\
    format_prestige_cost_global_message, format_reputation_query, is_dangerous_lost_combat,\n\
    label_from_lost_combat, label_from_reputation, lost_combat_from_reputation,\n\
    reputation_from_lost_combat, HitReputationDelta, HitReputationInput, PrestigeCostCategory,\n\
    ReputationBook, ReputationLabel, COMBAT_REPUTATION_RESTORE_PER_YEAR, DEVIL_MASK_CLOTHING_ID,\n\
    ELDERLY_AGE_YEARS, MIN_AGE_TO_EAT_YEARS, PRESTIGE_COST_PER_DAMAGE_ALLY,\n\
    PRESTIGE_COST_PER_DAMAGE_CHILD, PRESTIGE_COST_PER_DAMAGE_CLOSE_RELATIVE,\n\
    PRESTIGE_COST_PER_DAMAGE_ELDERLY, PRESTIGE_COST_PER_DAMAGE_WOMAN_UNARMED,\n\
};",
            "pub use reputation::{\n\
    attack_was_legit, combat_reputation_restore_delta, compute_hit_reputation,\n\
    compute_hit_reputation_with_factors, format_prestige_cost_global_message,\n\
    format_reputation_query, is_dangerous_lost_combat, label_from_lost_combat,\n\
    label_from_reputation, lost_combat_from_reputation, reputation_from_lost_combat,\n\
    HitReputationDelta, HitReputationInput, PrestigeCostCategory, PrestigeCostFactors,\n\
    ReputationBook, ReputationLabel, COMBAT_REPUTATION_RESTORE_PER_YEAR, DEVIL_MASK_CLOTHING_ID,\n\
    ELDERLY_AGE_YEARS, MIN_AGE_TO_EAT_YEARS, PRESTIGE_COST_PER_DAMAGE_ALLY,\n\
    PRESTIGE_COST_PER_DAMAGE_CHILD, PRESTIGE_COST_PER_DAMAGE_CLOSE_RELATIVE,\n\
    PRESTIGE_COST_PER_DAMAGE_ELDERLY, PRESTIGE_COST_PER_DAMAGE_WOMAN_UNARMED,\n\
};",
        );
    }

    // After exile recompute: use is_ally
    if !t.contains("// PRESTIGE-ALLY-COST: exile-aware isAlly for prestige + allyFactor") {
        ch |= replace_once(
            &mut t,
            "                // Recompute ally after possible exile (top leaders may diverge).\n\
                let target_is_ally =\n\
                    is_leadership_ally(&state.social.following, killer_id, target_id);\n",
            "                // Recompute ally after possible exile (top leaders may diverge).\n\
                // PRESTIGE-ALLY-COST: exile-aware isAlly for prestige + allyFactor\n\
                // Haxe: GlobalPlayerInstance.isAlly after this.exile(targetPlayer)\n\
                let deleted_ids: std::collections::HashSet<i32> = state\n\
                    .players\n\
                    .values()\n\
                    .filter(|p| p.deleted)\n\
                    .map(|p| p.p_id)\n\
                    .collect();\n\
                let target_is_ally = is_ally(\n\
                    &state.social.following,\n\
                    &state.social,\n\
                    &deleted_ids,\n\
                    killer_id,\n\
                    target_id,\n\
                );\n",
        );
    }

    // apply_connecting_hit_reputation
    if !t.contains("compute_hit_reputation_with_factors(&input") {
        ch |= replace_once(
            &mut t,
            "    let input = HitReputationInput {\n\
        damage,\n\
        target_lost_combat: target_lost,\n\
        target_holding_weapon,\n\
        attacker_prestige_class: attacker_class,\n\
        target_prestige_class: target_class,\n\
        target_true_age,\n\
        target_is_ally,\n\
        target_is_close_relative: close_rel,\n\
        target_is_female,\n\
        target_is_cursed,\n\
        attacker_has_red_mask: has_red_mask,\n\
    };\n\
    let delta = compute_hit_reputation(&input);\n",
            "    // PRESTIGE-ALLY-COST: recompute exile-aware isAlly (ignore stale pre-exile flag).\n\
    // Haxe: targetPlayer.isAlly(this) after exile mid-kill\n\
    let _ = target_is_ally; // call-site flag may predate mid-hit exile\n\
    let deleted_ids: std::collections::HashSet<i32> = state\n\
        .players\n\
        .values()\n\
        .filter(|p| p.deleted)\n\
        .map(|p| p.p_id)\n\
        .collect();\n\
    let target_is_ally = is_ally(\n\
        &state.social.following,\n\
        &state.social,\n\
        &deleted_ids,\n\
        killer_id,\n\
        target_id,\n\
    );\n\
    let input = HitReputationInput {\n\
        damage,\n\
        target_lost_combat: target_lost,\n\
        target_holding_weapon,\n\
        attacker_prestige_class: attacker_class,\n\
        target_prestige_class: target_class,\n\
        target_true_age,\n\
        target_is_ally,\n\
        target_is_close_relative: close_rel,\n\
        target_is_female,\n\
        target_is_cursed,\n\
        attacker_has_red_mask: has_red_mask,\n\
    };\n\
    // PRESTIGE-ALLY-COST: live PrestigeCostPerDamageForAlly\n\
    let mut factors = PrestigeCostFactors::default();\n\
    factors.ally = state.gameplay.prestige_cost_per_damage_for_ally;\n\
    let delta = compute_hit_reputation_with_factors(&input, &factors);\n",
        );
    }

    // Integration test
    if !t.contains("say_hit_peer_ally_prestige_cost_and_gm") {
        ch |= replace_once(
            &mut t,
            "    /// ALLY-STRENGTH: HIT non-ally with nearby friendlies of attacker boosts damage (cap 1.2).\n\
    // Haxe: DoDamage allyFactor = min(strength, 1.2)\n\
    #[test]\n\
    fn say_hit_ally_strength_boosts_and_angers_close_allies() {\n",
            "    /// PRESTIGE-ALLY-COST: peer allies under same leader — second unarmed HIT applies\n\
    /// PrestigeCostPerDamageForAlly + GM (still isAlly after peer exile).\n\
    // Haxe: kill L4540–4545 prestigeCost ally branch + sendGlobalMessage\n\
    #[test]\n\
    fn say_hit_peer_ally_prestige_cost_and_gm() {\n\
        let counters = Counters::new();\n\
        let hub = OutboundHub::new();\n\
        let mut rx1 = hub.register(1);\n\
        let mut rx2 = hub.register(2);\n\
        let mut rx3 = hub.register(3);\n\
        let mut state = SimState::with_default_empty(test_content());\n\
        let leader = spawn_player(&mut state, 1, \"peer@l\");\n\
        let a = spawn_player(&mut state, 2, \"peer@a\");\n\
        let b = spawn_player(&mut state, 3, \"peer@b\");\n\
        state.social.ensure_lineage(leader, \"L\");\n\
        state.social.ensure_lineage(a, \"A\");\n\
        state.social.ensure_lineage(b, \"B\");\n\
        // Both follow same leader → isAlly; peer exile does not break shared top.\n\
        state.social.set_follow(a, leader).unwrap();\n\
        state.social.set_follow(b, leader).unwrap();\n\
        for (cid, x) in [(1, 0), (2, 1), (3, 2)] {\n\
            state.players.get_mut(&cid).unwrap().x = x;\n\
            state.players.get_mut(&cid).unwrap().y = 0;\n\
            state.players.get_mut(&cid).unwrap().held_id = 0;\n\
            state.players.get_mut(&cid).unwrap().true_age = 25.0;\n\
            state.players.get_mut(&cid).unwrap().display_object_id = 352;\n\
        }\n\
        // Seed score prestige so cost is visible.\n\
        state.combat.stats_mut(a).prestige = 50.0;\n\
        while rx1.try_recv().is_ok() {}\n\
        while rx2.try_recv().is_ok() {}\n\
        while rx3.try_recv().is_ok() {}\n\
\n\
        // First HIT: ally warn, no wound.\n\
        apply_intent(\n\
            &mut state,\n\
            &counters,\n\
            &hub,\n\
            NetIntent::Raw {\n\
                conn_id: 2,\n\
                tag: \"SAY\".into(),\n\
                payload: format!(\"HIT {b}\"),\n\
            },\n\
        );\n\
        assert_eq!(state.combat.wound_of(b), 0, \"first peer-ally hit warns\");\n\
\n\
        state.sim_time += SAY_RATE_WINDOW_SECS + 1.0;\n\
        while rx2.try_recv().is_ok() {}\n\
        let prest_before = state.combat.stats.get(&a).map(|s| s.prestige).unwrap_or(0.0);\n\
        let lost_before = state.reputation.lost_combat(a);\n\
\n\
        apply_intent(\n\
            &mut state,\n\
            &counters,\n\
            &hub,\n\
            NetIntent::Raw {\n\
                conn_id: 2,\n\
                tag: \"SAY\".into(),\n\
                payload: format!(\"HIT {b}\"),\n\
            },\n\
        );\n\
        assert_eq!(state.combat.wound_of(b), 1, \"second hit wounds peer ally\");\n\
        let lost_after = state.reputation.lost_combat(a);\n\
        assert!(\n\
            lost_after > lost_before + 0.5,\n\
            \"ally prestige path should add guilt+cost: before={lost_before} after={lost_after}\"\n\
        );\n\
        let prest_after = state.combat.stats.get(&a).map(|s| s.prestige).unwrap_or(0.0);\n\
        assert!(\n\
            prest_after < prest_before,\n\
            \"score prestige must drop for ally cost: before={prest_before} after={prest_after}\"\n\
        );\n\
\n\
        let mut saw_gm = false;\n\
        while let Ok(pkt) = rx2.try_recv() {\n\
            let s = String::from_utf8_lossy(&pkt);\n\
            if s.contains(\"prestige\") && s.contains(\"ally\") {\n\
                saw_gm = true;\n\
            }\n\
            if s.contains(\"Lost\") && s.contains(\"ally\") {\n\
                saw_gm = true;\n\
            }\n\
        }\n\
        assert!(saw_gm, \"expected ally prestige-cost GM to attacker\");\n\
    }\n\
\n\
    /// ALLY-STRENGTH: HIT non-ally with nearby friendlies of attacker boosts damage (cap 1.2).\n\
    // Haxe: DoDamage allyFactor = min(strength, 1.2)\n\
    #[test]\n\
    fn say_hit_ally_strength_boosts_and_angers_close_allies() {\n",
        );
    }

    write_if_changed(path, &raw, &restore_nl(&t, crlf)) || ch
}
