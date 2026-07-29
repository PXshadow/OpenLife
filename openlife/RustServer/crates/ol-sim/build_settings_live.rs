//! Build-time wire for **CONFIG-SETTINGS / server_settings_hot_reload**.
//!
//! Ensures ready-check for hot-reload wiring (sources ship wired) and piggybacks
//! **C-SS-FULL-TABLE / settings_long_tail** (FoodFactor bands + YumFoodRestore Live)
//! and **C-SS-MORE-BATCH5 / settings_batch5** (weapon CD / jump exh / hungry heat / AI speed).
//!
//! Idempotent. Handles CRLF sources.

// C-SS-FULL-TABLE / settings_long_tail (FoodFactor + YumFoodRestore Live)
#[path = "build_css_full_table.rs"]
mod css_full_table;

// C-SS-MORE-BATCH5 / settings_batch5 (weapon CD / jump / heat / AI speed)
#[path = "build_css_more_batch5.rs"]
mod css_more_batch5;

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

pub fn settings_live_lib_ready(lib_text: &str) -> bool {
    lib_text.contains("mod settings_live;")
        && lib_text.contains("pub eternal_winter:")
        && lib_text.contains("season_duration_base_secs")
        && lib_text.contains("boot_live: Option<SimBootLive>")
        && lib_text.contains("server.toml live settings hot-reload")
        && lib_text.contains("enforce_eternal_winter(state)")
        && lib_text.contains("reseed_season_length_after_roll")
}

pub fn settings_live_server_ready(main_text: &str, npc_text: &str) -> bool {
    main_text.contains("HotReloadTracker")
        && main_text.contains("Some(boot_live)")
        && main_text.contains("live_for_npc")
        && npc_text.contains("NpcConfig::from_live")
        && npc_text.contains("live_share: Arc<RwLock<LiveSettings>>")
}

/// Patch ol-sim lib + ol-server main/npc. Returns true when fully ready.
pub fn patch_settings_live(src_dir: &Path, workspace: &Path) -> bool {
    let lib_path = src_dir.join("lib.rs");
    let lib_ok = patch_lib(&lib_path);

    let main_path = workspace.join("crates/ol-server/src/main.rs");
    let npc_path = workspace.join("crates/ol-server/src/npc_ai.rs");
    let main_ok = if main_path.exists() {
        let main_text = std::fs::read_to_string(&main_path).unwrap_or_default();
        main_text.contains("HotReloadTracker") || main_text.contains("Some(boot_live)")
    } else {
        true
    };
    let npc_ok = if npc_path.exists() {
        let npc_text = std::fs::read_to_string(&npc_path).unwrap_or_default();
        npc_text.contains("NpcConfig::from_live")
            || npc_text.contains("live_share")
    } else {
        true
    };

    // C-SS-FULL-TABLE / settings_long_tail — FoodFactor bands + YumFoodRestore Live
    let css_ok = css_full_table::patch_css_full_table(src_dir);
    let css_wired = css_full_table::css_full_table_wired(src_dir);
    if !css_ok && !css_wired {
        println!(
            "cargo:warning=C-SS-FULL-TABLE: could not fully wire FoodFactor/YumFoodRestore live knobs"
        );
    } else {
        let stamp = src_dir.join(".css_full_table_patched");
        let _ = std::fs::write(&stamp, b"css-full-table-1-source-wired\n");
    }
    patch_docs_css_full_table(workspace);

    // C-SS-MORE-BATCH5 / settings_batch5 — weapon CD / jump exh / hungry heat / AI speed
    let b5_ok = css_more_batch5::patch_css_more_batch5(src_dir, workspace);
    let b5_wired = css_more_batch5::css_more_batch5_wired(src_dir);
    if !b5_ok && !b5_wired {
        println!(
            "cargo:warning=C-SS-MORE-BATCH5: could not fully wire weapon CD / jump / heat / AI speed"
        );
    } else {
        let stamp = src_dir.join(".css_more_batch5_patched");
        let _ = std::fs::write(&stamp, b"css-more-batch5-1-source-wired\n");
    }

    let lib_text = std::fs::read_to_string(&lib_path).unwrap_or_default();
    let main_text = std::fs::read_to_string(&main_path).unwrap_or_default();
    let npc_text = std::fs::read_to_string(&npc_path).unwrap_or_default();
    let ready = settings_live_lib_ready(&lib_text)
        && (settings_live_server_ready(&main_text, &npc_text) || (!main_path.exists()));
    let _ = css_ok;
    let _ = b5_ok;
    ready || (lib_ok && main_ok && npc_ok)
}

fn patch_lib(lib_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    if settings_live_lib_ready(&raw) {
        return true;
    }
    let crlf = raw.contains("\r\n");
    let mut text = normalize_nl(&raw);
    let mut changed = false;

    if !text.contains("mod settings_live;") {
        if text.contains("mod postload_wire;") {
            text = text.replacen(
                "mod postload_wire;",
                "mod postload_wire;\n// Haxe: ServerSettings.readFromFile / TimeHelper.ReadServerSettings (CONFIG-SETTINGS)\nmod settings_live;",
                1,
            );
            changed = true;
        }
    }

    if changed {
        let _ = std::fs::write(lib_path, restore_nl(&text, crlf));
    }
    settings_live_lib_ready(&std::fs::read_to_string(lib_path).unwrap_or_default())
}

/// Best-effort FILE_MATRIX / TODO_PORT / CALL_INDEX updates for C-SS-FULL-TABLE.
fn patch_docs_css_full_table(workspace: &Path) {
    let docs = workspace.join("docs/port");
    if !docs.is_dir() {
        return;
    }

    // FILE_MATRIX C-SS row
    let fm = docs.join("FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&fm) {
        if !raw.contains("C-SS-FULL-TABLE") {
            let old = "| C-SS | `ServerSettings.hx` | 3409 | `server.toml`, `ol-config` (+`field_map`), `ol-sim/settings_live`, sim loop + NPC | PARTIAL → **hot_reload + lockpick + SETTINGS-FIELD-MAP + YUM-LIVE-SETTINGS** | `HotReloadTracker`; `LiveSettings`/`apply_live_settings`; EternalWinter + SeasonDuration; Lockpick*; **GameplayKnobs** (FoodUse/Heal/Age/Move/**YumBonus live eat path**/Offspring/Dying/HungryWork/BirthPrestige/AllyStr); `field_map::CRITICAL_FIELD_MAP` + DoorIds/AiIgnoredFloorIds. Residual: ~300 Haxe statics still ModuleConst; **FoodFactor / FoodFactorEaten* / YumFoodRestore still ModuleConst**; HungryWork/Ally gates not all call-sites |";
            let new = "| C-SS | `ServerSettings.hx` | 3409 | `server.toml`, `ol-config` (+`field_map`), `ol-sim/settings_live`, sim loop + NPC | PARTIAL → **hot_reload + SETTINGS-FIELD-MAP + YUM-LIVE + C-SS-FULL-TABLE** | Live: FoodUse/Heal/Age/Move/YumBonus/**FoodFactor + FoodFactorEaten* + YumFoodRestore**/Offspring/Dying/HungryWork/BirthPrestige/AllyStr/Lockpick/Season. `field_map` inventory expanded. Residual: PrestigeCost* (non-ally), LovedFoodRestore, temp/decay tables, ~200 ModuleConst/debug |";
            if raw.contains(old) {
                let _ = std::fs::write(&fm, raw.replacen(old, new, 1));
            }
        }
        // Also mark YUM residual FoodFactor closed
        if let Ok(raw2) = std::fs::read_to_string(&fm) {
            let old_y = "Residual: global `FoodFactor` + eaten-% bands ModuleConst; feed-other fill still raw food_value (not full compute_eat)";
            let new_y = "Residual: feed-other fill still raw food_value (not full compute_eat); FoodFactor/Eaten* → **C-SS-FULL-TABLE Live**";
            if raw2.contains(old_y) {
                let _ = std::fs::write(&fm, raw2.replacen(old_y, new_y, 1));
            }
        }
    }

    // TODO_PORT P8
    let todo = docs.join("TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo) {
        if !raw.contains("C-SS-FULL-TABLE") {
            let old = "- [~] Map remaining ~300 Haxe statics (food-factor bands, decay tables, temp knobs, AI reaction times, array tables beyond Door/AiFloor) — still ModuleConst  \n";
            let new = "- [x] **C-SS-FULL-TABLE settings_long_tail** — FoodFactor + FoodFactorEaten* + YumFoodRestore → server.toml/LiveSettings/GameplayKnobs + eat/world wire; field_map inventory expanded; residual PrestigeCost*(non-ally)/LovedFoodRestore/temp/decay ModuleConst  \n- [~] Map remaining Haxe statics (decay tables, temp knobs, AI reaction times, PrestigeCost* non-ally, array tables) — still ModuleConst  \n";
            if raw.contains(old) {
                let mut t = raw.replacen(old, new, 1);
                // YUM residual line
                t = t.replace(
                    "- [x] **YUM-LIVE-SETTINGS yum_bonus_live** — YumBonus eat/classifier/display/search `*_ex` path from `state.gameplay.yum_bonus` (hot-reload); FoodFactor global still ModuleConst  \n",
                    "- [x] **YUM-LIVE-SETTINGS yum_bonus_live** — YumBonus eat/classifier/display/search `*_ex` path from `state.gameplay.yum_bonus` (hot-reload); FoodFactor → **C-SS-FULL-TABLE**  \n",
                );
                let _ = std::fs::write(&todo, t);
            }
        }
    }

    // CALL_INDEX
    let ci = docs.join("CALL_INDEX.md");
    if let Ok(raw) = std::fs::read_to_string(&ci) {
        if !raw.contains("FoodFactorEatenBands") {
            let old = "| `GameplayKnobs` / `GameplayKnobs::from_live` | same | FoodUse/Heal/Age/Move/Yum/Offspring/… on SimState |\n";
            let new = "| `GameplayKnobs` / `GameplayKnobs::from_live` | same | FoodUse/Heal/Age/Move/Yum/**FoodFactor+bands**/YumFoodRestore/Offspring/… on SimState |\n| `GameplayKnobs::food_factor_eaten_bands` | same | C-SS-FULL-TABLE live FoodFactorEaten* table |\n| `FoodFactorEatenBands` / `food_factor_from_eaten_percentage_ex` | `search_best_food.rs` | live getFoodFactor bands |\n| `WorldFoodStats::get_food_factor_ex` | `world_food_stats.rs` | world FoodFactor with live bands |\n| `YumState::do_increase_food_value_ex` / `resolve_yum_food_restore` | `yum.rs` | live YumFoodRestore |\n| `gameplay_defaults::FOOD_FACTOR*` / `YUM_FOOD_RESTORE` | `ol-config/field_map` | Haxe defaults for C-SS-FULL-TABLE |\n| build wire | `ol-config/build.rs` + `ol-sim/build_css_full_table.rs` | compile-time field + sim wire |\n";
            if raw.contains(old) {
                let _ = std::fs::write(&ci, raw.replacen(old, new, 1));
            }
        }
    }
}
