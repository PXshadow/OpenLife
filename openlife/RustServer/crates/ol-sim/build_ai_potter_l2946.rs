//! AI-POTTER-L2946: residual `doPotteryOnFire` other potter crafts (Haxe L2946 TODO).
//!
//! Idempotent source wire for smith_profession + pottery_profession + port docs.

use std::path::Path;

pub fn already_wired(smith: &str) -> bool {
    smith.contains("AI-POTTER-L2946")
        && smith.contains("FIRED_NOZZLE_TONGS")
        && smith.contains("count_wet_nozzle")
}

pub fn patch_all(src_dir: &Path) -> bool {
    let mut ok = true;
    ok &= patch_smith(src_dir);
    ok &= patch_pottery(src_dir);
    patch_docs(src_dir);
    ok
}

fn patch_smith(src_dir: &Path) -> bool {
    let path = src_dir.join("smith_profession.rs");
    let Ok(mut t) = std::fs::read_to_string(&path) else {
        return false;
    };
    if already_wired(&t) {
        return true;
    }
    let orig = t.clone();

    if !t.contains("pub const WET_CLAY_NOZZLE") {
        let old = "/// Huge Charcoal Pile (counts as coal stock).\npub const HUGE_CHARCOAL_PILE: i32 = 4102;\n/// Default aiCraftMax fallbacks when content DB not loaded (OHOL defaults).\npub const DEFAULT_MAX_CLAY_BOWLS: i32 = 3;\npub const DEFAULT_MAX_CLAY_PLATES: i32 = 3;\npub const DEFAULT_MAX_CLAY_CROCKS: i32 = 2;";
        let new = "/// Huge Charcoal Pile (counts as coal stock).\npub const HUGE_CHARCOAL_PILE: i32 = 4102;\n/// Wet Clay Nozzle (AI-POTTER-L2946 other potter craft).\n// Haxe residual L2946 / content id 285\npub const WET_CLAY_NOZZLE: i32 = 285;\n/// Wet Nozzle in Wooden Tongs.\npub const WET_NOZZLE_TONGS: i32 = 295;\n/// Clay Nozzle (fired).\npub const CLAY_NOZZLE: i32 = 286;\n/// Fired Nozzle in Wooden Tongs (craftItem target).\npub const FIRED_NOZZLE_TONGS: i32 = 296;\n/// Default aiCraftMax fallbacks when content DB not loaded (OHOL defaults).\npub const DEFAULT_MAX_CLAY_BOWLS: i32 = 3;\npub const DEFAULT_MAX_CLAY_PLATES: i32 = 3;\npub const DEFAULT_MAX_CLAY_CROCKS: i32 = 2;\n/// Default max clay nozzles (bellows / forge air; content has no LimitObject).\n// Haxe: L2946 TODO — no aiCraftMax; keep small stock\npub const DEFAULT_MAX_CLAY_NOZZLES: i32 = 2;";
        if !t.contains(old) {
            println!("cargo:warning=AI-POTTER-L2946: smith const block missing");
            return false;
        }
        t = t.replacen(old, new, 1);
    }

    if !t.contains("pub count_wet_nozzle:") {
        let old = "    /// Wet crock 1216 + tongs 1218.\n    pub count_wet_crock: i32,\n    /// Big/huge charcoal piles 300+4102.\n    pub count_coal: i32,\n    /// Firing kiln 282 present (adobe+charcoal shortCraft gate).\n    pub firing_kiln: bool,\n}";
        let new = "    /// Wet crock 1216 + tongs 1218.\n    pub count_wet_crock: i32,\n    /// Wet Clay Nozzle 285 + Wet Nozzle tongs 295 (AI-POTTER-L2946).\n    pub count_wet_nozzle: i32,\n    /// Clay Nozzle 286 + Fired Nozzle tongs 296.\n    pub count_nozzle: i32,\n    /// aiCraftMax-style cap for nozzles (default 2).\n    pub max_nozzle: i32,\n    /// Big/huge charcoal piles 300+4102.\n    pub count_coal: i32,\n    /// Firing kiln 282 present (adobe+charcoal shortCraft gate).\n    pub firing_kiln: bool,\n}";
        if !t.contains(old) {
            println!("cargo:warning=AI-POTTER-L2946: PotteryOnFireCounts fields missing");
            return false;
        }
        t = t.replacen(old, new, 1);
    }

    if !t.contains("count_wet_nozzle: 0,") {
        let old = "            count_wet_crock: 0,\n            count_coal: 0,\n            firing_kiln: true,\n        }\n    }\n}";
        let new = "            count_wet_crock: 0,\n            count_wet_nozzle: 0,\n            count_nozzle: 0,\n            max_nozzle: DEFAULT_MAX_CLAY_NOZZLES,\n            count_coal: 0,\n            firing_kiln: true,\n        }\n    }\n}";
        if !t.contains(old) {
            println!("cargo:warning=AI-POTTER-L2946: PotteryOnFireCounts Default missing");
            return false;
        }
        t = t.replacen(old, new, 1);
    }

    if !t.contains("let mut wet_nozzle = 0;") {
        let old = "    let mut wet_crock = 0;\n    let mut bowl_home = 0;\n    let mut plate_home = 0;\n    let mut crock_home = 0;\n    let mut bowl_close = 0;\n    let mut crock_close = 0;\n    let mut coal = 0;\n    let mut kiln = false;";
        let new = "    let mut wet_crock = 0;\n    let mut wet_nozzle = 0;\n    let mut bowl_home = 0;\n    let mut plate_home = 0;\n    let mut crock_home = 0;\n    let mut nozzle_home = 0;\n    let mut bowl_close = 0;\n    let mut crock_close = 0;\n    let mut coal = 0;\n    let mut kiln = false;";
        if !t.contains(old) {
            println!("cargo:warning=AI-POTTER-L2946: fill init missing");
            return false;
        }
        t = t.replacen(old, new, 1);
    }

    if !t.contains("WET_CLAY_NOZZLE | WET_NOZZLE_TONGS") {
        let old = "                CLAY_CROCK | CROCK_WITH_SQUASH => crock_home += 1,\n                BIG_CHARCOAL_PILE | HUGE_CHARCOAL_PILE => coal += 1,\n                _ => {}";
        let new = "                CLAY_CROCK | CROCK_WITH_SQUASH => crock_home += 1,\n                WET_CLAY_NOZZLE | WET_NOZZLE_TONGS => wet_nozzle += 1,\n                CLAY_NOZZLE | FIRED_NOZZLE_TONGS => nozzle_home += 1,\n                BIG_CHARCOAL_PILE | HUGE_CHARCOAL_PILE => coal += 1,\n                _ => {}";
        if !t.contains(old) {
            println!("cargo:warning=AI-POTTER-L2946: fill match missing");
            return false;
        }
        t = t.replacen(old, new, 1);
    }

    if !t.contains("c.count_wet_nozzle = wet_nozzle;") {
        let old = "    c.count_wet_crock = wet_crock;\n    c.count_bowl = bowl_home;\n    c.count_plate = plate_home;\n    c.count_crock = crock_home;\n    c.count_close_bowl = bowl_close;\n    c.count_close_crock = crock_close;\n    c.count_coal = coal;\n    c.firing_kiln = kiln;\n    c\n}";
        let new = "    c.count_wet_crock = wet_crock;\n    c.count_wet_nozzle = wet_nozzle;\n    c.count_bowl = bowl_home;\n    c.count_plate = plate_home;\n    c.count_crock = crock_home;\n    c.count_nozzle = nozzle_home;\n    c.max_nozzle = if c.max_nozzle > 0 {\n        c.max_nozzle\n    } else {\n        DEFAULT_MAX_CLAY_NOZZLES\n    };\n    c.count_close_bowl = bowl_close;\n    c.count_close_crock = crock_close;\n    c.count_coal = coal;\n    c.firing_kiln = kiln;\n    c\n}";
        if !t.contains(old) {
            println!("cargo:warning=AI-POTTER-L2946: fill assign missing");
            return false;
        }
        t = t.replacen(old, new, 1);
    }

    if !t.contains("AI-POTTER-L2946 / Haxe L2946 other potter crafts") {
        let start = t.find(
            "/// Haxe `doPotteryOnFire` pure decision body (smith prepare fallthrough).",
        );
        let end = t.find("/// Resolve DeferPottery through pottery body, else return action unchanged.");
        match (start, end) {
            (Some(s), Some(e)) if s < e => {
                let replacement = r#"/// Haxe `doPotteryOnFire` pure decision body (smith prepare fallthrough).
///
/// Order: fired bowl tongs under max → plate tongs → crock tongs →
/// **other potter crafts (L2946)** → adobe+kiln.
/// Bowl gate ports Haxe FIX as-is (`countBowl < countCloseBowls`); wet-bowl
/// path restored under L2946 residual crafts.
// Haxe: AiBase.doPotteryOnFire ~2908–2953
// Haxe: L2946 `// TODO make other potter stuff` → AI-POTTER-L2946
pub fn do_pottery_on_fire(c: &PotteryOnFireCounts) -> SmithAction {
    // Wooden Tongs with Fired Bowl 283 — Haxe FIX bowl limit uses close count.
    // Port-as-is: countBowl < maxBowls && countBowl < countCloseBowls
    if c.count_bowl < c.max_bowls && c.count_bowl < c.count_close_bowl {
        return SmithAction::CraftItem {
            object_id: FIRED_BOWL_TONGS,
        };
    }
    // Fired Plate in Wooden Tongs 241 when wet plates present under max
    if c.count_wet_plate > 0 && c.count_plate < c.max_plates {
        return SmithAction::CraftItem {
            object_id: FIRED_PLATE_TONGS,
        };
    }
    // Wooden Tongs with Fired Crock 1219
    if c.count_wet_crock > 0
        && c.count_crock < c.max_crock
        && c.count_close_crock < c.max_crock
    {
        return SmithAction::CraftItem {
            object_id: FIRED_CROCK_TONGS,
        };
    }

    // ── AI-POTTER-L2946 / Haxe L2946 other potter crafts ───────────────────
    // (slot is before adobe fuel in Haxe; adobe moved below residual crafts)

    // Wet-bowl fire path (Haxe original commented gate before FIX):
    // countWetBowl > 0 && countBowl < maxBowls && craftItem(283)
    if c.count_wet_bowl > 0 && c.count_bowl < c.max_bowls {
        return SmithAction::CraftItem {
            object_id: FIRED_BOWL_TONGS,
        };
    }

    // Shape wet crock: Wet Clay Bowl 233 + Wet Clay Bowl 233 → Wet Clay Crock 1216
    // Content: transitions/233_233.txt. Enables crock firing when wet bowls exist.
    let crock_stock = c.count_crock + c.count_wet_crock;
    if crock_stock < c.max_crock && c.count_wet_bowl >= 2 {
        return SmithAction::ShortCraft {
            actor: WET_CLAY_BOWL,
            target: WET_CLAY_BOWL,
        };
    }

    // Fire clay nozzle: craftItem(296) Fired Nozzle in Wooden Tongs
    let max_n = if c.max_nozzle > 0 {
        c.max_nozzle
    } else {
        DEFAULT_MAX_CLAY_NOZZLES
    };
    if c.count_wet_nozzle > 0 && c.count_nozzle < max_n {
        return SmithAction::CraftItem {
            object_id: FIRED_NOZZLE_TONGS,
        };
    }

    // Adobe 127 + Firing Adobe Kiln 282 when coal < 3 (Haxe after L2946 TODO)
    if c.count_coal < 3 && c.firing_kiln {
        return SmithAction::ShortCraft {
            actor: ADOBE,
            target: FIRING_KILN,
        };
    }
    SmithAction::None
}

"#;
                t = format!("{}{}{}", &t[..s], replacement, &t[e..]);
            }
            _ => {
                println!("cargo:warning=AI-POTTER-L2946: do_pottery_on_fire span missing");
                return false;
            }
        }
    }

    if !t.contains("do_pottery_on_fire_l2946_other_crafts") {
        let marker = "    #[test]\n    fn elder_collect_age_gate_on_rung_open() {";
        let tests = r#"    #[test]
    fn do_pottery_on_fire_l2946_other_crafts() {
        // Wet-bowl fire when FIX close-bowl gate fails (count_bowl == close)
        let mut pot = PotteryOnFireCounts {
            count_bowl: 1,
            count_close_bowl: 1,
            max_bowls: 3,
            count_wet_bowl: 2,
            firing_kiln: true,
            ..Default::default()
        };
        assert_eq!(
            do_pottery_on_fire(&pot),
            SmithAction::CraftItem {
                object_id: FIRED_BOWL_TONGS
            }
        );

        // Wet crock shaping 233+233 when under crock max and ≥2 wet bowls
        pot.count_wet_bowl = 2;
        pot.count_bowl = 3;
        pot.max_bowls = 3;
        pot.count_crock = 0;
        pot.count_wet_crock = 0;
        pot.max_crock = 2;
        assert_eq!(
            do_pottery_on_fire(&pot),
            SmithAction::ShortCraft {
                actor: WET_CLAY_BOWL,
                target: WET_CLAY_BOWL
            }
        );

        // Nozzle fire when wet nozzles present under max
        pot.count_wet_bowl = 0;
        pot.count_wet_nozzle = 1;
        pot.count_nozzle = 0;
        pot.max_nozzle = 2;
        assert_eq!(
            do_pottery_on_fire(&pot),
            SmithAction::CraftItem {
                object_id: FIRED_NOZZLE_TONGS
            }
        );

        // After residual crafts: adobe when coal low
        pot.count_wet_nozzle = 0;
        pot.count_coal = 1;
        pot.firing_kiln = true;
        assert_eq!(
            do_pottery_on_fire(&pot),
            SmithAction::ShortCraft {
                actor: ADOBE,
                target: FIRING_KILN
            }
        );
    }

    #[test]
    fn elder_collect_age_gate_on_rung_open() {"#;
        if !t.contains(marker) {
            println!("cargo:warning=AI-POTTER-L2946: smith test marker missing");
        } else {
            t = t.replacen(marker, tests, 1);
        }
    }

    if t != orig {
        if std::fs::write(&path, t).is_err() {
            return false;
        }
    }
    true
}

fn patch_pottery(src_dir: &Path) -> bool {
    let path = src_dir.join("pottery_profession.rs");
    let Ok(mut t) = std::fs::read_to_string(&path) else {
        return false;
    };
    if t.contains("l2946_other_pottery_crafts_via_on_fire_action")
        && t.contains("count_wet_nozzle")
        && t.contains("FIRED_NOZZLE_TONGS")
        && t.contains("before bowl/plate stock-full gate")
    {
        return true;
    }
    let orig = t.clone();

    t = t.replace(
        "//! Residual: live profession_scan USE/DROP tick, nested basket contain graph,\n//! Haxe L2946 other potter crafts, baker/smith DeferPottery full live chain.",
        "//! Residual: EmptyBasketAtHome dropIsAUse extract polish; smith DeferPottery live expand.\n//! **AI-POTTER-L2946** other crafts in shared `do_pottery_on_fire` (wet-bowl fire,\n//! wet-crock 233+233 shape, clay nozzle 296).",
    );

    if !t.contains("FIRED_NOZZLE_TONGS") {
        let old = "use crate::smith_profession::{\n    do_pottery_on_fire, PotteryOnFireCounts, SmithAction, ADOBE, BIG_CHARCOAL_PILE, CLAY_BOWL,\n    CLAY_CROCK, CLAY_PLATE, CROCK_WITH_SQUASH, DEFAULT_MAX_CLAY_BOWLS, DEFAULT_MAX_CLAY_CROCKS,\n    DEFAULT_MAX_CLAY_PLATES, FIRED_BOWL_TONGS, FIRED_CROCK_TONGS, FIRED_PLATE_TONGS, FIRING_KILN,\n    HUGE_CHARCOAL_PILE, STONE, WET_BOWL_TONGS, WET_CLAY_BOWL, WET_CLAY_CROCK, WET_CLAY_PLATE,\n    WET_CROCK_TONGS, WET_PLATE_TONGS,\n};";
        let new = "use crate::smith_profession::{\n    do_pottery_on_fire, PotteryOnFireCounts, SmithAction, ADOBE, BIG_CHARCOAL_PILE, CLAY_BOWL,\n    CLAY_CROCK, CLAY_NOZZLE, CLAY_PLATE, CROCK_WITH_SQUASH, DEFAULT_MAX_CLAY_BOWLS,\n    DEFAULT_MAX_CLAY_CROCKS, DEFAULT_MAX_CLAY_NOZZLES, DEFAULT_MAX_CLAY_PLATES, FIRED_BOWL_TONGS,\n    FIRED_CROCK_TONGS, FIRED_NOZZLE_TONGS, FIRED_PLATE_TONGS, FIRING_KILN, HUGE_CHARCOAL_PILE,\n    STONE, WET_BOWL_TONGS, WET_CLAY_BOWL, WET_CLAY_CROCK, WET_CLAY_NOZZLE, WET_CLAY_PLATE,\n    WET_CROCK_TONGS, WET_NOZZLE_TONGS, WET_PLATE_TONGS,\n};";
        if !t.contains(old) {
            println!("cargo:warning=AI-POTTER-L2946: pottery import missing");
            return false;
        }
        t = t.replacen(old, new, 1);
    }

    if !t.contains("pub max_nozzle: i32,") {
        let old = "    pub max_bowls: i32,\n    pub max_plates: i32,\n    pub max_crock: i32,\n    /// Close bowls r=20 from player (doPotteryOnFire FIX gate).\n    pub count_close_bowl: i32,\n    pub count_close_crock: i32,";
        let new = "    pub max_bowls: i32,\n    pub max_plates: i32,\n    pub max_crock: i32,\n    /// Clay nozzle aiCraftMax-style cap (AI-POTTER-L2946).\n    pub max_nozzle: i32,\n    /// Close bowls r=20 from player (doPotteryOnFire FIX gate).\n    pub count_close_bowl: i32,\n    pub count_close_crock: i32,";
        if !t.contains(old) {
            println!("cargo:warning=AI-POTTER-L2946: PotteryCounts fields missing");
            return false;
        }
        t = t.replacen(old, new, 1);
    }

    if !t.contains("fn count_wet_nozzle(") {
        let old = "    pub fn count_crock(&self) -> i32 {\n        self.sum(&[CLAY_CROCK, CROCK_WITH_SQUASH])\n    }\n\n    pub fn count_charcoal_basket(&self) -> i32 {";
        let new = "    pub fn count_crock(&self) -> i32 {\n        self.sum(&[CLAY_CROCK, CROCK_WITH_SQUASH])\n    }\n\n    /// Wet crock for shaping/firing (excludes squash).\n    // Haxe: countCurrentObjects([1216, 1218]) in doPotteryOnFire\n    pub fn count_wet_crock_raw(&self) -> i32 {\n        self.sum(&[WET_CLAY_CROCK, WET_CROCK_TONGS])\n    }\n\n    /// Wet Clay Nozzle 285 + Wet Nozzle tongs 295.\n    pub fn count_wet_nozzle(&self) -> i32 {\n        self.sum(&[WET_CLAY_NOZZLE, WET_NOZZLE_TONGS])\n    }\n\n    /// Clay Nozzle 286 + Fired Nozzle tongs 296.\n    pub fn count_nozzle(&self) -> i32 {\n        self.sum(&[CLAY_NOZZLE, FIRED_NOZZLE_TONGS])\n    }\n\n    pub fn count_charcoal_basket(&self) -> i32 {";
        if !t.contains(old) {
            println!("cargo:warning=AI-POTTER-L2946: pottery helpers missing");
            return false;
        }
        t = t.replacen(old, new, 1);
    }

    if !t.contains("max_nozzle: DEFAULT_MAX_CLAY_NOZZLES,") {
        let old = "        max_crock,\n        has_home: true,\n        ..Default::default()\n    };";
        let new = "        max_crock,\n        max_nozzle: DEFAULT_MAX_CLAY_NOZZLES,\n        has_home: true,\n        ..Default::default()\n    };";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
        }
    }

    if !t.contains("count_wet_nozzle: c.count_wet_nozzle()") {
        let old = "        // On-fire wet crock excludes squash (Haxe countCurrentObjects([1216, 1218])).\n        count_wet_crock: c.sum(&[WET_CLAY_CROCK, WET_CROCK_TONGS]),\n        count_coal: c.count_coal(),\n        firing_kiln: c.firing_kiln || c.kiln_parent_id == Some(FIRING_ADOBE_KILN),\n    }\n}";
        let new = "        // On-fire wet crock excludes squash (Haxe countCurrentObjects([1216, 1218])).\n        count_wet_crock: c.sum(&[WET_CLAY_CROCK, WET_CROCK_TONGS]),\n        count_wet_nozzle: c.count_wet_nozzle(),\n        count_nozzle: c.count_nozzle(),\n        max_nozzle: if c.max_nozzle > 0 {\n            c.max_nozzle\n        } else {\n            DEFAULT_MAX_CLAY_NOZZLES\n        },\n        count_coal: c.count_coal(),\n        firing_kiln: c.firing_kiln || c.kiln_parent_id == Some(FIRING_ADOBE_KILN),\n    }\n}";
        if !t.contains(old) {
            println!("cargo:warning=AI-POTTER-L2946: pottery_on_fire_counts_from_pottery missing");
            return false;
        }
        t = t.replacen(old, new, 1);
    }

    // Crock shape BEFORE bowl/plate stock-full gate (else never reachable when stock full).
    if !t.contains("before bowl/plate stock-full gate") {
        let old = "    // Stock full → clear profession stage\n    if counts.count_bowl() >= max_b && counts.count_plate() >= max_p {\n        runtime.stage = 0.0;\n        return PotteryAction::None;\n    }";
        let new = "    // AI-POTTER-L2946: shape wet crock (233+233) before bowl/plate stock-full gate\n    // so crocks still form when bowls+plates are at max (Haxe L2946 residual).\n    let max_c = if counts.max_crock > 0 {\n        counts.max_crock\n    } else {\n        DEFAULT_MAX_CLAY_CROCKS\n    };\n    let crock_stock = counts.count_crock() + counts.count_wet_crock_raw();\n    if crock_stock < max_c && counts.count_wet_bowl() >= 2 {\n        runtime.stage = runtime.stage.max(3.0);\n        return PotteryAction::ShortCraft {\n            actor: WET_CLAY_BOWL,\n            target: WET_CLAY_BOWL,\n        };\n    }\n\n    // Stock full → clear profession stage\n    if counts.count_bowl() >= max_b && counts.count_plate() >= max_p {\n        runtime.stage = 0.0;\n        return PotteryAction::None;\n    }";
        if !t.contains(old) {
            println!("cargo:warning=AI-POTTER-L2946: stock-full gate missing for crock insert");
            return false;
        }
        t = t.replacen(old, new, 1);
    }

    if !t.contains("max_nozzle: DEFAULT_MAX_CLAY_NOZZLES")
        || !t
            .split("fn counts_basic")
            .nth(1)
            .map(|s| s.contains("max_nozzle: DEFAULT_MAX_CLAY_NOZZLES"))
            .unwrap_or(false)
    {
        let old = "            max_crock: DEFAULT_MAX_CLAY_CROCKS,\n            kiln_parent_id: Some(ADOBE_KILN),\n            ..Default::default()\n        }\n    }";
        let new = "            max_crock: DEFAULT_MAX_CLAY_CROCKS,\n            max_nozzle: DEFAULT_MAX_CLAY_NOZZLES,\n            kiln_parent_id: Some(ADOBE_KILN),\n            ..Default::default()\n        }\n    }";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
        }
    }

    if !t.contains("l2946_other_pottery_crafts_via_on_fire_action") {
        let needle = "            crate::smith_profession::short_craft_on_ground_apply(0, FIRED_BOWL_TONGS)\n        );\n    }\n}";
        let insert = r#"            crate::smith_profession::short_craft_on_ground_apply(0, FIRED_BOWL_TONGS)
        );
    }

    /// AI-POTTER-L2946: residual doPotteryOnFire other crafts via potter bridge.
    #[test]
    fn l2946_other_pottery_crafts_via_on_fire_action() {
        let mut c = counts_basic();
        c.firing_kiln = true;
        c.max_bowls = 3;
        c.count_close_bowl = 0;
        c.set(WET_CLAY_BOWL, 2);
        assert_eq!(
            do_pottery_on_fire_action(&c),
            PotteryAction::CraftItem {
                object_id: FIRED_BOWL_TONGS
            }
        );

        c.set(CLAY_BOWL, 3);
        c.count_close_bowl = 3;
        c.set(WET_CLAY_BOWL, 2);
        c.max_crock = 2;
        assert_eq!(
            do_pottery_on_fire_action(&c),
            PotteryAction::ShortCraft {
                actor: WET_CLAY_BOWL,
                target: WET_CLAY_BOWL
            }
        );

        c.set(WET_CLAY_BOWL, 0);
        c.set(WET_CLAY_NOZZLE, 1);
        c.max_nozzle = 2;
        assert_eq!(
            do_pottery_on_fire_action(&c),
            PotteryAction::CraftItem {
                object_id: FIRED_NOZZLE_TONGS
            }
        );
    }

    #[test]
    fn l2946_do_pottery_shapes_wet_crock_at_stage3() {
        // Bowls+plates at max would hit stock-full; crock shape runs first (L2946).
        let mut c = counts_basic();
        c.set(CLAY, 5);
        c.set(CLAY_BOWL, 3);
        c.set(CLAY_PLATE, 3);
        c.set(WET_CLAY_BOWL, 2);
        c.max_bowls = 3;
        c.max_plates = 3;
        c.max_crock = 2;
        c.kiln_parent_id = Some(ADOBE_KILN);
        let mut rt = PotterProfessionRuntime {
            is_last_potter: true,
            stage: 2.0,
            ..Default::default()
        };
        let a = do_pottery(&c, &mut rt, 1, 0.0, 0.0, None);
        assert_eq!(
            a,
            PotteryAction::ShortCraft {
                actor: WET_CLAY_BOWL,
                target: WET_CLAY_BOWL
            }
        );
    }
}"#;
        if !t.contains(needle) {
            println!("cargo:warning=AI-POTTER-L2946: pottery test insert needle missing");
        } else {
            t = t.replacen(needle, insert, 1);
        }
    }

    if t != orig {
        if std::fs::write(&path, t).is_err() {
            return false;
        }
    }
    true
}

fn patch_docs(src_dir: &Path) {
    let Some(crate_root) = src_dir.parent() else {
        return;
    };
    let Some(ws) = crate_root.parent().and_then(|p| p.parent()) else {
        return;
    };
    let port = ws.join("docs/port");

    let fm = port.join("FILE_MATRIX.md");
    if let Ok(mut t) = std::fs::read_to_string(&fm) {
        if !t.contains("| **AI-POTTER-L2946**") {
            if let Some(idx) = t.find("| **AI-POTTER** / pottery_job |") {
                if let Some(eol) = t[idx..].find('\n') {
                    let end = idx + eol;
                    let row = "\n| **AI-POTTER-L2946** / pottery_crafts | doPotteryOnFire L2946 other potter crafts | **DONE** | shared `do_pottery_on_fire`: wet-bowl craftItem(283); wet crock shortCraft(233,233); nozzle craftItem(296); adobe after residual; crock shape before stock-full; tests l2946_* |";
                    t.insert_str(end, row);
                }
            }
        }
        t = t.replace(
            "Residual: Haxe L2946 other crafts; smith DeferPottery live expand path |",
            "Residual: EmptyBasketAtHome extract; smith DeferPottery live expand (**AI-POTTER-L2946** DONE) |",
        );
        t = t.replace(
            "Residual: Haxe L2946 other crafts; EmptyBasketAtHome dropIsAUse extract polish |",
            "Residual: EmptyBasketAtHome dropIsAUse extract polish (**AI-POTTER-L2946** crafts DONE) |",
        );
        let _ = std::fs::write(&fm, t);
    }

    let tp = port.join("TODO_PORT.md");
    if let Ok(mut t) = std::fs::read_to_string(&tp) {
        if !t.contains("AI-POTTER-L2946 pottery_crafts DONE") {
            t = t.replace(
                "- [~] **AI-POTTER pottery_job PARTIAL** — pure SM + scan USE/DROP + **AI-POTTER-NEST** basket nest (any-slot contain, kiln-home, deposit staging maxDist 0, full-basket-near-deposit pickup, dual clay r=80). Residual: Haxe L2946 other crafts; EmptyBasketAtHome dropIsAUse extract polish; smith DeferPottery live expand  ",
                "- [x] **AI-POTTER-L2946 pottery_crafts DONE** — `do_pottery_on_fire` residual: wet-bowl fire; wet crock shortCraft(233,233); clay nozzle craftItem(296); adobe after L2946; crock shape before stock-full; tests smith+pottery l2946_*\n- [~] **AI-POTTER pottery_job PARTIAL** — pure SM + scan USE/DROP + **AI-POTTER-NEST** + **AI-POTTER-L2946**. Residual: EmptyBasketAtHome dropIsAUse extract polish; smith DeferPottery live expand  ",
            );
            t = t.replace(
                "Residual: L2946 crafts / empty-hands USE extract  ",
                "Residual: empty-hands USE extract (**AI-POTTER-L2946** DONE)  ",
            );
            let header = "| Date | Change |\n|------|--------|\n";
            if let Some(idx) = t.find(header) {
                let insert_at = idx + header.len();
                let line = "| 2026-07-29 | **AI-POTTER-L2946 pottery_crafts DONE**: Haxe L2946 other potter crafts — wet-bowl fire, wet crock 233+233, nozzle 296; crock shape before stock-full; tests |\n";
                if !t.contains("AI-POTTER-L2946 pottery_crafts DONE**:") {
                    t.insert_str(insert_at, line);
                }
            }
        }
        let _ = std::fs::write(&tp, t);
    }

    let ci = port.join("CALL_INDEX.md");
    if let Ok(mut t) = std::fs::read_to_string(&ci) {
        if !t.contains("AI-POTTER-L2946") {
            let needle = "| `do_pottery_on_fire_action` / `pottery_on_fire_counts_from_pottery` / `smith_pottery_action_to_pottery` | same | on-fire via shared smith body |";
            let rep = "| `do_pottery_on_fire_action` / `pottery_on_fire_counts_from_pottery` / `smith_pottery_action_to_pottery` | same | on-fire via shared smith body |\n| `do_pottery_on_fire` L2946 residual | `smith_profession.rs` | **AI-POTTER-L2946**: wet-bowl fire; shortCraft(233,233); craftItem(296) nozzle; adobe after |\n| `WET_CLAY_NOZZLE` / `FIRED_NOZZLE_TONGS` / `DEFAULT_MAX_CLAY_NOZZLES` | same | nozzle ids + cap |\n| `count_wet_nozzle` / `count_nozzle` / `count_wet_crock_raw` | `pottery_profession.rs` | L2946 count helpers |";
            if t.contains(needle) {
                t = t.replacen(needle, rep, 1);
                let _ = std::fs::write(&ci, t);
            }
        }
    }
}
