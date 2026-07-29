//! Build-time wire for **AI-CRAFT-LIVE-MORE** / craft_multi_step.
//!
//! Pure GetCraftAndDropItemsCloseToObj + craftItemHelper adze/froe/goose/kindling
//! specials + fillBucket tank/bucket residual. Idempotent pure-Rust string patches.

use std::path::{Path, PathBuf};

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

fn write_if_changed(path: &Path, original: &str, next: &str) -> bool {
    if original == next {
        return false;
    }
    if let Err(e) = std::fs::write(path, next) {
        eprintln!(
            "cargo:warning=AI-CRAFT-LIVE-MORE write {}: {e}",
            path.display()
        );
        return false;
    }
    true
}

/// True when GetCraftAndDrop pure surface is already in craft_item.
pub fn already_wired(src: &Path) -> bool {
    let ci = std::fs::read_to_string(src.join("craft_item.rs")).unwrap_or_default();
    ci.contains("include!(\"craft_and_drop.inc.rs\")")
        && ci.contains("GotoDropAnchor")
        && ci.contains("adze_froe_butt_log_craft_and_drop")
        && ci.contains("fire_bow_kindling_craft_and_drop")
}

pub fn patch_all(src: &Path, workspace: &Path) -> bool {
    let mut any = false;
    any |= patch_craft_item(src);
    any |= patch_lib_exports(src);
    any |= patch_docs(workspace);
    any
}

fn patch_craft_item(src: &Path) -> bool {
    let path = src.join("craft_item.rs");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    // Module doc mention
    if !t.contains("AI-CRAFT-LIVE-MORE") {
        let old = "//! Multi-step **craftItem** / **craftItemHelper** world craft (AI-CRAFT-MULTI + **AI-CRAFT-TOPDOWN** + **AI-CRAFT-DUAL**).";
        let new = "//! Multi-step **craftItem** / **craftItemHelper** world craft (AI-CRAFT-MULTI + **AI-CRAFT-TOPDOWN** + **AI-CRAFT-DUAL** + **AI-CRAFT-LIVE-MORE**).";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }
    if !t.contains("GetCraftAndDropItemsCloseToObj") {
        let old = "//! - dual-center searchCurrentPosition + pile*1.5 / r=6 re-anchor ~7050–7242 (AI-CRAFT-DUAL)";
        let new = "//! - dual-center searchCurrentPosition + pile*1.5 / r=6 re-anchor ~7050–7242 (AI-CRAFT-DUAL)\n//! - GetCraftAndDropItemsCloseToObj adze/froe/goose/kindling + fillBucket residual (AI-CRAFT-LIVE-MORE)";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    // Include pure helpers after fire_bow_needs_kindling
    if !t.contains("include!(\"craft_and_drop.inc.rs\")") {
        let anchor = "    kindling.is_none() && tinder.is_none()\n}\n\n// ── Have-set builder";
        let insert = "    kindling.is_none() && tinder.is_none()\n}\n\n// Haxe: GetCraftAndDropItemsCloseToObj + craftItemHelper specials (AI-CRAFT-LIVE-MORE)\ninclude!(\"craft_and_drop.inc.rs\");\n\n// ── Have-set builder";
        if t.contains(anchor) {
            t = t.replacen(anchor, insert, 1);
            changed = true;
        } else {
            // Fallback: after fire_bow function closing
            let alt = "pub fn fire_bow_needs_kindling(";
            if let Some(idx) = t.find(alt) {
                if let Some(rel) = t[idx..].find("\n}\n\n") {
                    let at = idx + rel;
                    let end = at + "\n}\n\n".len();
                    t.insert_str(
                        end,
                        "// Haxe: GetCraftAndDropItemsCloseToObj + craftItemHelper specials (AI-CRAFT-LIVE-MORE)\ninclude!(\"craft_and_drop.inc.rs\");\n\n",
                    );
                    changed = true;
                }
            }
        }
    }

    // CraftItemDecision variants
    if !t.contains("GotoDropAnchor") {
        let old = r#"    /// Wait for time-transition target (actor id -1).
    WaitTime,
}"#;
        let new = r#"    /// Wait for time-transition target (actor id -1).
    WaitTime,
    /// GetCraftAndDrop: walk toward drop anchor while holding whichObj (quadDist > 5).
    // Haxe: GetCraftAndDropItemsCloseToObj gotoObj(target)
    GotoDropAnchor { target_x: i32, target_y: i32 },
    /// GetCraftAndDrop: drop held whichObj near anchor.
    // Haxe: dropHeldObject(5, target)
    DropNearAnchor { target_x: i32, target_y: i32 },
}"#;
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    // is_action includes new variants
    if t.contains("Self::ShortCraftOnGround { .. }")
        && !t.contains("Self::GotoDropAnchor")
    {
        let old = "| Self::ShortCraftOnGround { .. }\n        )";
        let new = "| Self::ShortCraftOnGround { .. }\n                | Self::GotoDropAnchor { .. }\n                | Self::DropNearAnchor { .. }\n        )";
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    // Wire specials in craft_item_helper after berry pie gate
    if !t.contains("adze_froe_butt_log_craft_and_drop") {
        let old = r#"    if berry_pie_crust_blocked(
        actor_id,
        target_id,
        objs,
        inp.player_x,
        inp.player_y,
        max_r,
    ) {
        return CraftItemDecision::Failed;
    }

    // Fire bow + shaft → need kindling/tinder near shaft first (residual of GetCraftAndDrop).
    // Haxe: craftItemHelper ~6890–6902
    if fire_bow_needs_kindling(
        objs,
        actor_id,
        target_id,
        target_x,
        target_y,
        inp.called_craft_item,
    ) {
        return CraftItemDecision::SeekIngredient {
            ingredient_id: KINDLING,
            for_product: product_id,
        };
    }"#;
        let new = r#"    if berry_pie_crust_blocked(
        actor_id,
        target_id,
        objs,
        inp.player_x,
        inp.player_y,
        max_r,
    ) {
        return CraftItemDecision::Failed;
    }

    // Bring targets to tool: Steel Adze/Froe + Butt Log (GetCraftAndDrop).
    // Haxe: craftItemHelper ~6761–6771
    if let Some(d) = adze_froe_butt_log_craft_and_drop(
        objs,
        actor_id,
        target_id,
        actor_x,
        actor_y,
        inp.held_id,
        inp.player_x,
        inp.player_y,
        inp.called_craft_item,
        product_id,
    ) {
        return d;
    }

    // Domestic Goose empty-hand: Steel Axe near stump (GetCraftAndDrop).
    // Haxe: craftItemHelper ~6773–6781
    if actor_id == 0 && target_id == DOMESTIC_GOOSE && !inp.called_craft_item {
        if closest_craft_obj(objs, STUMP, inp.player_x, inp.player_y, GOOSE_STUMP_SEARCH_R, None)
            .is_none()
        {
            return CraftItemDecision::Failed;
        }
        if let Some(d) = goose_axe_near_stump_craft_and_drop(
            objs,
            actor_id,
            target_id,
            inp.held_id,
            inp.player_x,
            inp.player_y,
            inp.called_craft_item,
            product_id,
        ) {
            return d;
        }
    }

    // Fire bow + shaft → GetCraftAndDrop kindling then tinder near shaft.
    // Haxe: craftItemHelper ~6890–6902
    if let Some(d) = fire_bow_kindling_craft_and_drop(
        objs,
        actor_id,
        target_id,
        target_x,
        target_y,
        inp.held_id,
        inp.player_x,
        inp.player_y,
        inp.called_craft_item,
        product_id,
    ) {
        return d;
    }"#;
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        } else {
            eprintln!(
                "cargo:warning=AI-CRAFT-LIVE-MORE: craft_item_helper specials anchor not found"
            );
        }
    }

    // Live intent mapping for new decisions
    if !t.contains("CraftItemDecision::GotoDropAnchor") {
        let old = r#"        CraftItemDecision::ShortCraftOnGround { object_id } => {
            ShortCraftLiveIntent::SeekGroundActor { target: object_id }
        }
    }
}"#;
        let new = r#"        CraftItemDecision::ShortCraftOnGround { object_id } => {
            ShortCraftLiveIntent::SeekGroundActor { target: object_id }
        }

        CraftItemDecision::GotoDropAnchor { target_x, target_y } => {
            ShortCraftLiveIntent::Goto {
                x: target_x,
                y: target_y,
            }
        }
        CraftItemDecision::DropNearAnchor { target_x, target_y } => {
            ShortCraftLiveIntent::DropAt {
                x: target_x,
                y: target_y,
            }
        }
    }
}"#;
        if t.contains(old) {
            t = t.replacen(old, new, 1);
            changed = true;
        }
    }

    // craft_item_with_runtime: set called_craft_item when GetCraftAndDrop specials fire
    if !t.contains("note_called_craft_item_from_decision") {
        // Add helper near note_craft_done
        if t.contains("pub fn clear_tick_guard(&mut self) {")
            && !t.contains("pub fn note_called_craft_item_from_decision")
        {
            let old = r#"    /// Clear per-tick recursion guard (Haxe `calledCraftItem = false` each doTime).
    pub fn clear_tick_guard(&mut self) {
        self.called_craft_item = false;
    }
}"#;
            let new = r#"    /// Clear per-tick recursion guard (Haxe `calledCraftItem = false` each doTime).
    pub fn clear_tick_guard(&mut self) {
        self.called_craft_item = false;
    }

    /// Mark recursion guard after GetCraftAndDrop specials (Haxe `calledCraftItem = true`).
    // Haxe: craftItemHelper calledCraftItem = true before GetCraftAndDrop
    pub fn note_called_craft_item_from_decision(&mut self, decision: CraftItemDecision) {
        if matches!(
            decision,
            CraftItemDecision::GotoDropAnchor { .. }
                | CraftItemDecision::DropNearAnchor { .. }
                | CraftItemDecision::PickupActor { .. }
                | CraftItemDecision::SeekIngredient { .. }
        ) {
            // Only set when decision came from craft-and-drop specials; callers may always call.
            self.called_craft_item = true;
        }
    }
}"#;
            if t.contains(old) {
                t = t.replacen(old, new, 1);
                changed = true;
            }
        }
    }

    // Integration tests at end of tests module (before final closing)
    if !t.contains("fn helper_adze_special_seeks_butt_log") {
        let marker = "fn smith_false_with_forge_returns_need_smith()";
        if let Some(idx) = t.rfind(marker) {
            // Find end of that test function
            if let Some(rel) = t[idx..].find("\n    }\n}") {
                let at = idx + rel + "\n    }\n".len();
                let tests = r#"
    #[test]
    fn helper_adze_special_seeks_butt_log() {
        let mut g = ReverseCraftGraph::new();
        // Adze + Butt Log → boards product 999
        g.insert(STEEL_ADZE, BUTT_LOG, 999, 0);
        let objs = vec![
            CraftWorldObj::simple(STEEL_ADZE, 0, 0),
            // no butt log near → GetCraftAndDrop → SeekIngredient(BUTT_LOG)
        ];
        let mut state = ItemToCraftState::new(999);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(999, 0, 0).with_held(STEEL_ADZE);
        let d = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None);
        assert_eq!(
            d,
            CraftItemDecision::SeekIngredient {
                ingredient_id: BUTT_LOG,
                for_product: 999
            }
        );
    }

    #[test]
    fn helper_goose_no_stump_fails() {
        let mut g = ReverseCraftGraph::new();
        g.insert(0, DOMESTIC_GOOSE, 1267, 0);
        let objs = vec![CraftWorldObj::simple(DOMESTIC_GOOSE, 1, 0)];
        let mut state = ItemToCraftState::new(1267);
        let mut failed = FailedCraftings::new();
        let inp = CraftItemInput::basic(1267, 0, 0); // empty hand
        let d = craft_item_helper(&objs, &inp, &mut state, &mut failed, &g, None);
        assert_eq!(d, CraftItemDecision::Failed);
    }

    #[test]
    fn craft_and_drop_decision_to_live_intent() {
        let g = craft_item_decision_to_live_intent(
            CraftItemDecision::GotoDropAnchor {
                target_x: 3,
                target_y: 4,
            },
            None,
        );
        assert_eq!(g, ShortCraftLiveIntent::Goto { x: 3, y: 4 });
        let d = craft_item_decision_to_live_intent(
            CraftItemDecision::DropNearAnchor {
                target_x: 1,
                target_y: 2,
            },
            None,
        );
        assert_eq!(d, ShortCraftLiveIntent::DropAt { x: 1, y: 2 });
    }
"#;
                t.insert_str(at, tests);
                changed = true;
            }
        }
    }

    if !changed {
        return false;
    }
    write_if_changed(&path, &raw, &restore_nl(&t, crlf))
}

fn patch_lib_exports(src: &Path) -> bool {
    let path = src.join("lib.rs");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    if t.contains("get_craft_and_drop_items_close_to_obj") {
        return false;
    }
    // Expand craft_item re-export block
    let old = "    fire_bow_needs_kindling, first_missing_ingredient, resolve_craft_item_live,";
    let new = "    fire_bow_needs_kindling, fire_bow_kindling_craft_and_drop,\n    first_missing_ingredient, get_craft_and_drop_items_close_to_obj,\n    adze_froe_butt_log_craft_and_drop, goose_axe_near_stump_craft_and_drop,\n    fill_bucket_if_needed_apply, craft_and_drop_to_decision, craft_quad_dist,\n    closest_craft_obj_from_anchor, resolve_craft_item_live,";
    if !t.contains(old) {
        eprintln!("cargo:warning=AI-CRAFT-LIVE-MORE: lib.rs re-export anchor missing");
        return false;
    }
    t = t.replacen(old, new, 1);

    // Types
    let old2 = "    CraftAiRuntime, CraftItemDecision, CraftItemInput,";
    let new2 = "    CraftAiRuntime, CraftAndDropApply, CraftItemDecision, CraftItemInput, FillBucketApply,";
    if t.contains(old2) && !t.contains("CraftAndDropApply") {
        t = t.replacen(old2, new2, 1);
    }

    // Constants
    let old3 = "    DEFAULT_WATER_SOURCE_IDS, FORGE_IDS, HARDENED_ROW, KINDLING,";
    let new3 = "    DEFAULT_WATER_SOURCE_IDS, DEFAULT_BUCKET_WATER_SOURCE_IDS, FORGE_IDS, HARDENED_ROW, KINDLING,\n    STEEL_ADZE, STEEL_FROE, BUTT_LOG, STEEL_AXE, STUMP, DOMESTIC_GOOSE,\n    EMPTY_BUCKET, CRAFT_DROP_GOTO_QUAD_DIST, ADZE_FROE_LOG_DIST,";
    if t.contains(old3) && !t.contains("DEFAULT_BUCKET_WATER_SOURCE_IDS") {
        t = t.replacen(old3, new3, 1);
    }

    write_if_changed(&path, &raw, &restore_nl(&t, crlf))
}

fn patch_docs(workspace: &Path) -> bool {
    let mut any = false;
    let todo = workspace.join("docs/port/TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if !t.contains("AI-CRAFT-LIVE-MORE") {
            // Update AI-CRAFT-MULTI residual line
            if let Some(idx) = t.find("**AI-CRAFT-MULTI craft_item_live PARTIAL**") {
                let line_end = t[idx..].find('\n').unwrap_or(200);
                let old_line = t[idx..idx + line_end].to_string();
                if old_line.contains("GetCraftAndDrop adze/bucket") {
                    let new_line = old_line
                        .replace(
                            "Residual: full GetCraftAndDrop adze/bucket; dynamic WaterSourceIds",
                            "GetCraftAndDrop adze/froe/goose/kindling + fillBucket → **AI-CRAFT-LIVE-MORE DONE**; residual: dynamic WaterSourceIds",
                        )
                        .replace(
                            "Residual: full GetCraftAndDrop adze/bucket",
                            "GetCraftAndDrop → **AI-CRAFT-LIVE-MORE DONE**; residual",
                        );
                    if new_line != old_line {
                        t.replace_range(idx..idx + line_end, &new_line);
                    }
                }
            }
            // Insert DONE checkbox after AI-CRAFT-MULTI or DUAL
            let line = "- [x] **AI-CRAFT-LIVE-MORE craft_multi_step DONE** — pure `get_craft_and_drop_items_close_to_obj` + adze/froe+butt log, goose+axe stump, fire-bow kindling/tinder GetCraftAndDrop; fillBucket tank/bucket residual; GotoDropAnchor/DropNearAnchor live map; tests craft_and_drop_* + helper. Residual: dynamic BucketWaterSourceIds from content; npc recursive craftItem enqueue depth\n";
            if let Some(idx) = t.find("- [x] **AI-CRAFT-DUAL dual_center_search DONE**") {
                let end = t[idx..].find('\n').map(|i| idx + i + 1).unwrap_or(idx);
                t.insert_str(end, line);
            } else if let Some(idx) = t.find("- [~] **AI-CRAFT-MULTI craft_item_live PARTIAL**") {
                t.insert_str(idx, line);
            }
            // Changelog table row
            if let Some(idx) = t.find("| 2026-07-29 | **AI-CRAFT-DUAL") {
                let row = "| 2026-07-29 | **AI-CRAFT-LIVE-MORE craft_multi_step**: pure GetCraftAndDropItemsCloseToObj + adze/froe/goose/kindling specials + fillBucket tank/bucket residual; GotoDropAnchor/DropNearAnchor; tests craft_and_drop_*; residual dynamic BucketWaterSourceIds / npc recursive craftItem |\n";
                t.insert_str(idx, row);
            }
            any |= write_if_changed(&todo, &raw, &restore_nl(&t, crlf));
        }
    }

    let matrix = workspace.join("docs/port/FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&matrix) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if !t.contains("AI-CRAFT-LIVE-MORE") {
            if let Some(idx) = t.find("| AI-PRIO / AI-JOB-* / CRAFT-LIVE / AI-LLM-* |") {
                let row = "| **AI-CRAFT-LIVE-MORE** / craft_multi_step | GetCraftAndDropItemsCloseToObj specials (adze/froe/goose/kindling/bucket) | **DONE** (pure+helper wire) | `craft_and_drop.inc.rs` + craft_item helper specials; GotoDropAnchor/DropNearAnchor; fillBucket residual; tests craft_and_drop_*. Residual: dynamic BucketWaterSourceIds |\n";
                t.insert_str(idx, row);
                any |= write_if_changed(&matrix, &raw, &restore_nl(&t, crlf));
            }
        }
    }

    let call = workspace.join("docs/port/CALL_INDEX.md");
    if let Ok(raw) = std::fs::read_to_string(&call) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if !t.contains("get_craft_and_drop_items_close_to_obj") {
            if let Some(idx) = t.find("## Rust: craftItem multi-step (AI-CRAFT-MULTI)") {
                let section = r#"
## Rust: GetCraftAndDrop live residual (AI-CRAFT-LIVE-MORE)

| Symbol | File | Role |
|--------|------|------|
| `get_craft_and_drop_items_close_to_obj` / `CraftAndDropApply` | `ol-sim/src/craft_and_drop.inc.rs` (via craft_item) | pure GetCraftAndDropItemsCloseToObj |
| `craft_and_drop_to_decision` / `GotoDropAnchor` / `DropNearAnchor` | same + CraftItemDecision | drop/goto/pickup/craft staging |
| `adze_froe_butt_log_craft_and_drop` | same | Steel Adze/Froe + Butt Log bring-log-to-tool |
| `goose_axe_near_stump_craft_and_drop` | same | Domestic Goose → Steel Axe near stump |
| `fire_bow_kindling_craft_and_drop` | same | Fire bow + shaft → kindling/tinder GetCraftAndDrop |
| `fill_bucket_if_needed_apply` / `FillBucketApply` | same | fillBucket tank/bucket shortCraft residual |
| `closest_craft_obj_from_anchor` / `craft_quad_dist` | same | anchor search + squared dist |

"#;
                t.insert_str(idx, section);
                any |= write_if_changed(&call, &raw, &restore_nl(&t, crlf));
            }
        }
    }

    // Changelog file
    let clog_dir = workspace.join("docs/port/changelog");
    let clog = clog_dir.join("2026-07-29-AI-CRAFT-LIVE-MORE.md");
    if !clog.exists() {
        let _ = std::fs::create_dir_all(&clog_dir);
        let body = r#"# AI-CRAFT-LIVE-MORE / craft_multi_step (2026-07-29)

## Status: DONE (core)

Pure **GetCraftAndDropItemsCloseToObj** + craftItemHelper specials residual from AI-CRAFT-MULTI.

### Implemented

| Piece | Notes |
|-------|-------|
| `get_craft_and_drop_items_close_to_obj` | count near → held goto/drop → pickup band → craft |
| Adze/Froe + Butt Log | bring log to tool (dist=6) |
| Goose + Steel Axe near stump | empty-hand 0+1256 |
| Fire bow kindling/tinder | full GetCraftAndDrop (not only Seek when none) |
| `fill_bucket_if_needed_apply` | tank 3168/3167 + empty bucket; bucket water sources |
| `GotoDropAnchor` / `DropNearAnchor` | live Goto / DropAt map |

### Tests

```powershell
cargo test -p ol-sim --lib -- craft_and_drop
cargo test -p ol-sim --lib -- helper_adze
cargo test -p ol-sim --lib -- craft_item
```

### Residual

1. Dynamic `ServerSettings.BucketWaterSourceIds` / `WaterSourceIds` from content transitions
2. npc recursive craftItem enqueue depth for CraftItem branch
3. hostile/unreachable filters on GetClosestObjectToTarget
"#;
        if std::fs::write(&clog, body).is_ok() {
            any = true;
        }
    }

    any
}
