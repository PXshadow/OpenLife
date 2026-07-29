//! C-SS-FULL-TABLE / settings_long_tail — promote FoodFactor bands + YumFoodRestore
//! into ServerConfig / LiveSettings before compile.
//!
//! Idempotent pure-Rust source patch (same pattern as ol-sim build wires).

use std::path::Path;

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lib = manifest.join("src/lib.rs");
    let _ = patch_lib(&lib);
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/field_map.rs");
    println!("cargo:rerun-if-changed=build.rs");
}

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

fn already_patched(t: &str) -> bool {
    t.contains("pub food_factor: f32,")
        && t.contains("food_factor_eaten_less_than_one_percent")
        && t.contains("yum_food_restore")
        && t.contains("\"food_factor\"")
        && t.contains("gameplay_defaults::FOOD_FACTOR")
}

fn patch_lib(path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return false;
    };
    if already_patched(&raw) {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    // ServerConfig fields (before closing brace of struct)
    let sc_anchor = "    // PRESTIGE-ALLY-COST\n    pub prestige_cost_per_damage_for_ally: f32,\n}\n\n/// One configured twin peer";
    let sc_insert = r#"    // PRESTIGE-ALLY-COST
    pub prestige_cost_per_damage_for_ally: f32,
    // --- C-SS-FULL-TABLE / settings_long_tail: FoodFactor + eat bands + YumFoodRestore ---
    /// Haxe `ServerSettings.FoodFactor` — global fill scale in compute_eat.
    // Haxe: ServerSettings.FoodFactor
    pub food_factor: f32,
    /// Haxe `FoodFactorEatenMoreThanEightPercent`.
    pub food_factor_eaten_more_than_eight_percent: f32,
    /// Haxe `FoodFactorEatenMoreThanTenPercent`.
    pub food_factor_eaten_more_than_ten_percent: f32,
    /// Haxe `FoodFactorEatenLessThanFivePercent`.
    pub food_factor_eaten_less_than_five_percent: f32,
    /// Haxe `FoodFactorEatenLessThanThreePercent`.
    pub food_factor_eaten_less_than_three_percent: f32,
    /// Haxe `FoodFactorEatenLessThanOnePercent`.
    pub food_factor_eaten_less_than_one_percent: f32,
    /// Haxe `YumFoodRestore`.
    // Haxe: ServerSettings.YumFoodRestore
    pub yum_food_restore: f32,
}

/// One configured twin peer"#;
    if t.contains(sc_anchor) && !t.contains("pub food_factor: f32,") {
        t = t.replacen(sc_anchor, sc_insert, 1);
        changed = true;
    }

    // Default impl
    let def_anchor = "            prestige_cost_per_damage_for_ally: gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_ALLY,\n        }\n    }\n}\n\n/// Subset of config knobs";
    let def_insert = r#"            prestige_cost_per_damage_for_ally: gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_ALLY,
            food_factor: gameplay_defaults::FOOD_FACTOR,
            food_factor_eaten_more_than_eight_percent: gameplay_defaults::FOOD_FACTOR_EATEN_MORE_THAN_EIGHT_PERCENT,
            food_factor_eaten_more_than_ten_percent: gameplay_defaults::FOOD_FACTOR_EATEN_MORE_THAN_TEN_PERCENT,
            food_factor_eaten_less_than_five_percent: gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_FIVE_PERCENT,
            food_factor_eaten_less_than_three_percent: gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_THREE_PERCENT,
            food_factor_eaten_less_than_one_percent: gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_ONE_PERCENT,
            yum_food_restore: gameplay_defaults::YUM_FOOD_RESTORE,
        }
    }
}

/// Subset of config knobs"#;
    if t.contains(def_anchor) && !t.contains("food_factor: gameplay_defaults::FOOD_FACTOR") {
        t = t.replacen(def_anchor, def_insert, 1);
        changed = true;
    }

    // LiveSettings fields
    let live_anchor = "    // PRESTIGE-ALLY-COST\n    pub prestige_cost_per_damage_for_ally: f32,\n}\n\nimpl ServerConfig {";
    let live_insert = r#"    // PRESTIGE-ALLY-COST
    pub prestige_cost_per_damage_for_ally: f32,
    // --- C-SS-FULL-TABLE food factor long-tail ---
    /// Haxe `FoodFactor`.
    pub food_factor: f32,
    /// Haxe `FoodFactorEatenMoreThanEightPercent`.
    pub food_factor_eaten_more_than_eight_percent: f32,
    /// Haxe `FoodFactorEatenMoreThanTenPercent`.
    pub food_factor_eaten_more_than_ten_percent: f32,
    /// Haxe `FoodFactorEatenLessThanFivePercent`.
    pub food_factor_eaten_less_than_five_percent: f32,
    /// Haxe `FoodFactorEatenLessThanThreePercent`.
    pub food_factor_eaten_less_than_three_percent: f32,
    /// Haxe `FoodFactorEatenLessThanOnePercent`.
    pub food_factor_eaten_less_than_one_percent: f32,
    /// Haxe `YumFoodRestore`.
    pub yum_food_restore: f32,
}

impl ServerConfig {"#;
    if t.contains(live_anchor) && t.matches("pub food_factor: f32,").count() < 2 {
        t = t.replacen(live_anchor, live_insert, 1);
        changed = true;
    }

    // live_settings() extract — after prestige_cost sanitize, before closing
    let extract_anchor = "            prestige_cost_per_damage_for_ally: sanitize_nonneg_or(\n                self.prestige_cost_per_damage_for_ally,\n                gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_ALLY,\n            ),\n        }\n    }\n\n    /// Human-readable list of live fields";
    let extract_insert = r#"            prestige_cost_per_damage_for_ally: sanitize_nonneg_or(
                self.prestige_cost_per_damage_for_ally,
                gameplay_defaults::PRESTIGE_COST_PER_DAMAGE_FOR_ALLY,
            ),
            food_factor: sanitize_nonneg_or(self.food_factor, gameplay_defaults::FOOD_FACTOR),
            food_factor_eaten_more_than_eight_percent: sanitize_nonneg_or(
                self.food_factor_eaten_more_than_eight_percent,
                gameplay_defaults::FOOD_FACTOR_EATEN_MORE_THAN_EIGHT_PERCENT,
            ),
            food_factor_eaten_more_than_ten_percent: sanitize_nonneg_or(
                self.food_factor_eaten_more_than_ten_percent,
                gameplay_defaults::FOOD_FACTOR_EATEN_MORE_THAN_TEN_PERCENT,
            ),
            food_factor_eaten_less_than_five_percent: sanitize_nonneg_or(
                self.food_factor_eaten_less_than_five_percent,
                gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_FIVE_PERCENT,
            ),
            food_factor_eaten_less_than_three_percent: sanitize_nonneg_or(
                self.food_factor_eaten_less_than_three_percent,
                gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_THREE_PERCENT,
            ),
            food_factor_eaten_less_than_one_percent: sanitize_nonneg_or(
                self.food_factor_eaten_less_than_one_percent,
                gameplay_defaults::FOOD_FACTOR_EATEN_LESS_THAN_ONE_PERCENT,
            ),
            yum_food_restore: sanitize_nonneg_or(
                self.yum_food_restore,
                gameplay_defaults::YUM_FOOD_RESTORE,
            ),
        }
    }

    /// Human-readable list of live fields"#;
    if t.contains(extract_anchor) && !t.contains("self.yum_food_restore") {
        t = t.replacen(extract_anchor, extract_insert, 1);
        changed = true;
    }

    // live_diff_keys
    let diff_anchor = r#"        push(
            "prestige_cost_per_damage_for_ally",
            (old.prestige_cost_per_damage_for_ally - new.prestige_cost_per_damage_for_ally).abs()
                > f32::EPSILON,
        );
        keys
    }"#;
    let diff_insert = r#"        push(
            "prestige_cost_per_damage_for_ally",
            (old.prestige_cost_per_damage_for_ally - new.prestige_cost_per_damage_for_ally).abs()
                > f32::EPSILON,
        );
        // C-SS-FULL-TABLE food factor long-tail
        push(
            "food_factor",
            (old.food_factor - new.food_factor).abs() > f32::EPSILON,
        );
        push(
            "food_factor_eaten_more_than_eight_percent",
            (old.food_factor_eaten_more_than_eight_percent
                - new.food_factor_eaten_more_than_eight_percent)
                .abs()
                > f32::EPSILON,
        );
        push(
            "food_factor_eaten_more_than_ten_percent",
            (old.food_factor_eaten_more_than_ten_percent
                - new.food_factor_eaten_more_than_ten_percent)
                .abs()
                > f32::EPSILON,
        );
        push(
            "food_factor_eaten_less_than_five_percent",
            (old.food_factor_eaten_less_than_five_percent
                - new.food_factor_eaten_less_than_five_percent)
                .abs()
                > f32::EPSILON,
        );
        push(
            "food_factor_eaten_less_than_three_percent",
            (old.food_factor_eaten_less_than_three_percent
                - new.food_factor_eaten_less_than_three_percent)
                .abs()
                > f32::EPSILON,
        );
        push(
            "food_factor_eaten_less_than_one_percent",
            (old.food_factor_eaten_less_than_one_percent
                - new.food_factor_eaten_less_than_one_percent)
                .abs()
                > f32::EPSILON,
        );
        push(
            "yum_food_restore",
            (old.yum_food_restore - new.yum_food_restore).abs() > f32::EPSILON,
        );
        keys
    }"#;
    if t.contains(diff_anchor) && !t.contains("\"food_factor_eaten_more_than_eight_percent\"") {
        t = t.replacen(diff_anchor, diff_insert, 1);
        changed = true;
    }

    // live_settings_key_names
    let keys_anchor = r#"            "hire_cost_increase_per_person",
            "prestige_cost_per_damage_for_ally",
        ]
    }
}"#;
    let keys_insert = r#"            "hire_cost_increase_per_person",
            "prestige_cost_per_damage_for_ally",
            "food_factor",
            "food_factor_eaten_more_than_eight_percent",
            "food_factor_eaten_more_than_ten_percent",
            "food_factor_eaten_less_than_five_percent",
            "food_factor_eaten_less_than_three_percent",
            "food_factor_eaten_less_than_one_percent",
            "yum_food_restore",
        ]
    }
}"#;
    if t.contains(keys_anchor) && !t.contains("\"yum_food_restore\",") {
        t = t.replacen(keys_anchor, keys_insert, 1);
        changed = true;
    }

    // force_reload cfg1
    let fr_anchor = "            prestige_cost_per_damage_for_ally: 2.5,\n            twin_peers: vec![TwinPeerConfig {";
    let fr_insert = r#"            prestige_cost_per_damage_for_ally: 2.5,
            food_factor: 0.7,
            food_factor_eaten_more_than_eight_percent: 0.4,
            food_factor_eaten_more_than_ten_percent: 0.3,
            food_factor_eaten_less_than_five_percent: 1.8,
            food_factor_eaten_less_than_three_percent: 2.2,
            food_factor_eaten_less_than_one_percent: 3.0,
            yum_food_restore: 0.4,
            twin_peers: vec![TwinPeerConfig {"#;
    if t.contains(fr_anchor) && !t.contains("food_factor: 0.7,") {
        t = t.replacen(fr_anchor, fr_insert, 1);
        changed = true;
    }

    // gameplay_defaults_match_haxe asserts
    let assert_anchor = "        assert!((c.prestige_cost_per_damage_for_ally - 1.0).abs() < f32::EPSILON);\n    }\n\n    #[test]\n    fn field_map_critical_live_count() {";
    let assert_insert = r#"        assert!((c.prestige_cost_per_damage_for_ally - 1.0).abs() < f32::EPSILON);
        // C-SS-FULL-TABLE
        assert!((c.food_factor - 1.0).abs() < f32::EPSILON);
        assert!((c.food_factor_eaten_more_than_eight_percent - 0.8).abs() < f32::EPSILON);
        assert!((c.food_factor_eaten_more_than_ten_percent - 0.5).abs() < f32::EPSILON);
        assert!((c.food_factor_eaten_less_than_five_percent - 1.5).abs() < f32::EPSILON);
        assert!((c.food_factor_eaten_less_than_three_percent - 2.0).abs() < f32::EPSILON);
        assert!((c.food_factor_eaten_less_than_one_percent - 2.5).abs() < f32::EPSILON);
        assert!((c.yum_food_restore - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn field_map_critical_live_count() {"#;
    if t.contains(assert_anchor) && !t.contains("c.yum_food_restore - 0.8") {
        t = t.replacen(assert_anchor, assert_insert, 1);
        changed = true;
    }

    // field_map_critical_live_count body
    let count_anchor = r#"        let live = live_critical_names();
        assert!(live.len() >= 15, "live critical: {live:?}");
        let residual = module_const_critical_names();
        assert!(residual.contains(&"GrownUpFoodStoreMax"));
    }
}"#;
    let count_insert = r#"        let live = live_critical_names();
        assert!(live.len() >= 22, "live critical: {live:?}");
        assert!(live.contains(&"FoodFactor"));
        assert!(live.contains(&"YumFoodRestore"));
        let residual = module_const_critical_names();
        assert!(residual.contains(&"GrownUpFoodStoreMax"));
        assert!(!residual.contains(&"FoodFactor"));
    }
}"#;
    if t.contains(count_anchor) {
        t = t.replacen(count_anchor, count_insert, 1);
        changed = true;
    }

    if changed {
        let out = restore_nl(&t, crlf);
        let _ = std::fs::write(path, out);
    }
    already_patched(&std::fs::read_to_string(path).unwrap_or_default()) || changed
}
