//! AI-CRAFT-STICKY / craft_runtime: wire `Player.craft_ai` + `craft_ai_sticky` (idempotent).
//!
//! // Haxe: AiBase.itemToCraft + failedCraftings + itemToCraftId + craftingTasks

use std::path::{Path, PathBuf};

fn normalize_nl(s: &str) -> String {
    s.replace("\r\n", "\n")
}

fn restore_nl(s: &str, crlf: bool) -> String {
    if crlf {
        s.replace('\n', "\r\n")
    } else {
        s.to_string()
    }
}

/// True when Player sticky craft_ai + lib re-exports exist.
pub fn already_wired(src: &Path) -> bool {
    let player = src.join("player.rs");
    let lib = src.join("lib.rs");
    let sticky = src.join("craft_ai_sticky.rs");
    let p_ok = std::fs::read_to_string(&player)
        .map(|t| t.contains("pub craft_ai:") && t.contains("PlayerCraftAi"))
        .unwrap_or(false);
    let l_ok = std::fs::read_to_string(&lib)
        .map(|t| {
            t.contains("mod craft_ai_sticky;")
                && t.contains("pub use craft_ai_sticky::{")
                && t.contains("PlayerCraftAi")
        })
        .unwrap_or(false);
    sticky.exists() && p_ok && l_ok
}

pub fn patch_lib(lib_path: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(lib_path) else {
        return false;
    };
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("mod craft_ai_sticky;") {
        // After get_or_craft / craft_value / short_craft
        let anchors = [
            "mod get_or_craft;\n",
            "mod short_craft_intent;\n",
            "mod craft_graph;\n",
        ];
        for a in anchors {
            if let Some(idx) = t.find(a) {
                let end = idx + a.len();
                let insert =
                    "// Haxe: AiBase.itemToCraft + failedCraftings sticky on Player (AI-CRAFT-STICKY)\nmod craft_ai_sticky;\n";
                t = format!("{}{}{}", &t[..end], insert, &t[end..]);
                changed = true;
                break;
            }
        }
    }

    let use_block = r#"// Haxe: AiBase.itemToCraft / failedCraftings / itemToCraftId (AI-CRAFT-STICKY / craft_runtime)
pub use craft_ai_sticky::{
    craft_item_with_player_craft_ai, expand_craft_item_player_sticky,
    resolve_seek_or_craft_player_sticky, PlayerCraftAi,
};
"#;

    if !t.contains("pub use craft_ai_sticky::{") {
        // After craft_item / get_or_craft expand re-exports
        let markers = [
            "pub use get_or_craft::{\n",
            "pub use get_or_craft::craft_item::{\n",
            "pub use short_craft_intent::{\n",
        ];
        for m in markers {
            if let Some(idx) = t.find(m) {
                if let Some(end_rel) = t[idx..].find("\n};") {
                    let end = idx + end_rel + "\n};".len();
                    // Prefer after expand_craft_item_live block if present just after
                    let after = &t[end..];
                    let mut insert_at = end;
                    if after.starts_with("\npub use get_or_craft::{")
                        || after.starts_with("\npub use get_or_craft::expand")
                    {
                        // find next }; after this second block
                        if let Some(rel2) = after.find("\n};") {
                            insert_at = end + rel2 + "\n};".len();
                        }
                    }
                    t = format!("{}{}{}{}", &t[..insert_at], "\n", use_block, &t[insert_at..]);
                    changed = true;
                    break;
                }
            }
        }
    }

    // Ensure expand_craft_item_live_sticky is re-exported if missing from get_or_craft use
    if t.contains("expand_craft_item_live_opts")
        && !t.contains("expand_craft_item_live_sticky")
        && t.contains("pub use get_or_craft::{\n    expand_craft_item_live")
    {
        t = t.replace(
            "expand_craft_item_live, expand_craft_item_live_opts, expand_craft_item_live_sticky,",
            "expand_craft_item_live, expand_craft_item_live_opts, expand_craft_item_live_sticky,",
        );
        // if sticky still missing, insert
        if !t.contains("expand_craft_item_live_sticky") {
            t = t.replace(
                "expand_craft_item_live, expand_craft_item_live_opts,",
                "expand_craft_item_live, expand_craft_item_live_opts, expand_craft_item_live_sticky,",
            );
            changed = true;
        }
    }

    if changed {
        let _ = std::fs::write(lib_path, restore_nl(&t, crlf));
    }
    std::fs::read_to_string(lib_path)
        .map(|x| x.contains("mod craft_ai_sticky;") && x.contains("PlayerCraftAi"))
        .unwrap_or(false)
}

pub fn patch_player(player: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(player) else {
        return false;
    };
    if raw.contains("pub craft_ai:") && raw.contains("PlayerCraftAi") {
        // Ensure test present
        if raw.contains("fn craft_ai_sticky_on_player") {
            return true;
        }
    }
    let crlf = raw.contains("\r\n");
    let mut t = normalize_nl(&raw);
    let mut changed = false;

    if !t.contains("pub craft_ai:") {
        let field = r#"    /// Sticky multi-tick craft state (Haxe itemToCraft + failedCraftings + itemToCraftId).
    /// Survives AI ticks; pure craft_item decisions mutate via [`crate::PlayerCraftAi`].
    // Haxe: AiBase.itemToCraft + failedCraftings + itemToCraftId + craftingTasks (AI-CRAFT-STICKY)
    pub craft_ai: crate::PlayerCraftAi,
"#;
        // Insert after farm_task or shepherd / farm_profession block
        if t.contains("pub farm_task: crate::FarmTaskState,\n") {
            t = t.replace(
                "pub farm_task: crate::FarmTaskState,\n",
                &format!("pub farm_task: crate::FarmTaskState,\n{field}"),
            );
            changed = true;
        } else if t.contains("pub farm_profession: crate::FarmProfessionRuntime,\n") {
            t = t.replace(
                "pub farm_profession: crate::FarmProfessionRuntime,\n",
                &format!("pub farm_profession: crate::FarmProfessionRuntime,\n{field}"),
            );
            changed = true;
        }
    }

    if !t.contains("craft_ai: crate::PlayerCraftAi::")
        && !t.contains("craft_ai: crate::PlayerCraftAi::new()")
    {
        if t.contains("farm_task: crate::FarmTaskState::default(),\n") {
            t = t.replace(
                "farm_task: crate::FarmTaskState::default(),\n",
                "farm_task: crate::FarmTaskState::default(),\n            craft_ai: crate::PlayerCraftAi::new(),\n",
            );
            changed = true;
        } else if t.contains("farm_profession: crate::FarmProfessionRuntime::default(),\n") {
            t = t.replace(
                "farm_profession: crate::FarmProfessionRuntime::default(),\n",
                "farm_profession: crate::FarmProfessionRuntime::default(),\n            craft_ai: crate::PlayerCraftAi::new(),\n",
            );
            changed = true;
        }
    }

    // Methods on Player for wipe/begin
    if !t.contains("fn wipe_craft_on_birth") {
        if let Some(idx) = t.find("    pub fn snapshot(&self) -> PlayerSnapshot {") {
            let methods = r#"    /// Haxe `AiBase.newBorn` craft wipe (failedCraftings + itemToCraft + tasks).
    // Haxe: AiBase.newBorn (AI-CRAFT-STICKY)
    pub fn wipe_craft_on_birth(&mut self) {
        self.craft_ai.wipe_on_birth();
    }

    /// Haxe `calledCraftItem = false` each AI doTime entry.
    // Haxe: AiBase.doTimeStuffHelper calledCraftItem = false
    pub fn craft_ai_begin_tick(&mut self) {
        self.craft_ai.begin_tick();
    }

"#;
            t.insert_str(idx, methods);
            changed = true;
        }
    }

    if !t.contains("fn craft_ai_sticky_on_player") {
        if let Some(idx) = t.find("fn soul_sticky_on_player_defaults_and_survives") {
            let test = r#"    #[test]
    fn craft_ai_sticky_on_player_defaults_and_survives() {
        // AI-CRAFT-STICKY: Player.craft_ai sticky across ticks
        let mut p = Player::new(1, 1, "craft@test");
        assert_eq!(p.craft_ai.item_to_craft_id, -1);
        assert!(p.craft_ai.crafting_tasks.is_empty());
        assert_eq!(p.craft_ai.runtime.last_actor_id, -1);
        p.craft_ai.item_to_craft_id = 83;
        p.craft_ai.add_task(71, true);
        p.craft_ai.runtime.failed.record_fail(83, 50.0);
        p.craft_ai.runtime.item = crate::ItemToCraftState::new(83);
        p.craft_ai.runtime.item.count_done = 1;
        // Survive "tick boundary" (same Player)
        assert_eq!(p.craft_ai.item_to_craft_id, 83);
        assert_eq!(p.craft_ai.crafting_tasks, vec![71]);
        assert!(p.craft_ai.runtime.failed.is_cooling_down(83, 55.0));
        assert_eq!(p.craft_ai.runtime.item.count_done, 1);
        // Birth wipe
        p.wipe_craft_on_birth();
        assert_eq!(p.craft_ai.item_to_craft_id, -1);
        assert!(p.craft_ai.crafting_tasks.is_empty());
        assert!(p.craft_ai.runtime.failed.last_fail_sec.is_empty());
        p.craft_ai.runtime.called_craft_item = true;
        p.craft_ai_begin_tick();
        assert!(!p.craft_ai.runtime.called_craft_item);
    }

"#;
            t.insert_str(idx, test);
            changed = true;
        }
    }

    if changed {
        let _ = std::fs::write(player, restore_nl(&t, crlf));
    }
    std::fs::read_to_string(player)
        .map(|x| x.contains("pub craft_ai:") && x.contains("PlayerCraftAi"))
        .unwrap_or(false)
}

pub fn patch_docs(workspace: &Path) -> bool {
    let port = workspace.join("docs").join("port");
    let mut any = false;

    // FILE_MATRIX
    let fm = port.join("FILE_MATRIX.md");
    if let Ok(raw) = std::fs::read_to_string(&fm) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if !t.contains("AI-CRAFT-STICKY") {
            let row = "| **AI-CRAFT-STICKY** / craft_runtime | Player sticky CraftAiRuntime / failedCraftings / itemToCraftId | **DONE** (core) | `Player.craft_ai: PlayerCraftAi` (`craft_ai_sticky.rs`) — itemToCraft + failedCraftings + itemToCraftId + craftingTasks + lastActorId/calledCraftItem; birth wipe; interrupt re-queue; `craft_item_with_player_craft_ai` / expand+resolve sticky; tests craft_ai_sticky::* + player sticky. Residual: npc_ai full GetOrCraft enqueue; countDone after live USE; top-down filters → AI-CRAFT-MULTI |\n";
            if let Some(idx) = t.find("| **AI-CRAFT-MULTI**") {
                t.insert_str(idx, row);
                any = true;
            } else if let Some(idx) = t.find("| AI-CRAFT |") {
                // after AI-CRAFT line
                if let Some(eol) = t[idx..].find('\n') {
                    t.insert_str(idx + eol + 1, row);
                    any = true;
                }
            }
            // Update AI-CRAFT-MULTI residual note: Player field done
            if t.contains("Player field") {
                t = t.replace(
                    "Player field, GetCraftAndDrop",
                    "Player field (**AI-CRAFT-STICKY DONE**), GetCraftAndDrop",
                );
                t = t.replace(
                    "residual top-down filters, hostile/unreachable, Player field,",
                    "residual top-down filters, hostile/unreachable, Player field (**AI-CRAFT-STICKY DONE**),",
                );
                any = true;
            }
        }
        if any {
            let _ = std::fs::write(&fm, restore_nl(&t, crlf));
        }
    }

    // TODO_PORT
    let todo = port.join("TODO_PORT.md");
    if let Ok(raw) = std::fs::read_to_string(&todo) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        let mut changed = false;
        if !t.contains("AI-CRAFT-STICKY") {
            let line = "- [x] **AI-CRAFT-STICKY craft_runtime DONE** — `Player.craft_ai: PlayerCraftAi` sticky itemToCraft + failedCraftings + itemToCraftId + craftingTasks; birth wipe; interrupt re-queue; craft_item_with_player_craft_ai / expand+resolve sticky; tests craft_ai_sticky::* + player. Residual: npc full GetOrCraft multi-step enqueue; countDone after USE\n";
            if let Some(idx) = t.find("AI-CRAFT-MULTI craft_item_live") {
                // insert before multi line
                if let Some(line_start) = t[..idx].rfind("- [") {
                    t.insert_str(line_start, line);
                    changed = true;
                }
            }
        }
        // Update AI-CRAFT-MULTI residual Player.itemToCraft
        if t.contains("Player.itemToCraft") {
            t = t.replace(
                "Player.itemToCraft;",
                "Player.itemToCraft (**AI-CRAFT-STICKY DONE**);",
            );
            t = t.replace(
                "Player.itemToCraft + failedCraftings fields on AI/NPC player structs (pure `CraftAiRuntime` ready)",
                "~~Player.itemToCraft + failedCraftings fields~~ → **AI-CRAFT-STICKY DONE**",
            );
            changed = true;
        }
        // Changelog row
        if !t.contains("AI-CRAFT-STICKY craft_runtime") {
            if let Some(idx) = t.find("| 2026-07-28 | **AI-CRAFT-MULTI") {
                let row = "| 2026-07-28 | **AI-CRAFT-STICKY craft_runtime**: `Player.craft_ai: PlayerCraftAi` sticky itemToCraft+failedCraftings+itemToCraftId+craftingTasks; birth wipe; interrupt re-queue; craft_item_with_player_craft_ai; tests craft_ai_sticky::* + player sticky; residual npc full multi-step enqueue / countDone after USE |\n";
                t.insert_str(idx, row);
                changed = true;
            }
        }
        // Header last updated
        if let Some(idx) = t.find("Last updated:") {
            if let Some(eol) = t[idx..].find('\n') {
                let line = &t[idx..idx + eol];
                if !line.contains("AI-CRAFT-STICKY") {
                    t = t.replacen(line, "Last updated: **2026-07-28** (AI-CRAFT-STICKY craft_runtime)", 1);
                    changed = true;
                }
            }
        }
        if changed {
            let _ = std::fs::write(&todo, restore_nl(&t, crlf));
            any = true;
        }
    }

    // CALL_INDEX
    let ci = port.join("CALL_INDEX.md");
    if let Ok(raw) = std::fs::read_to_string(&ci) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if !t.contains("PlayerCraftAi") {
            let block = r#"
| `PlayerCraftAi` / `Player.craft_ai` | `ol-sim/src/craft_ai_sticky.rs` + `player.rs` | sticky itemToCraft + failedCraftings + itemToCraftId + craftingTasks (AI-CRAFT-STICKY) |
| `wipe_on_birth` / `prepare_for_product` / `add_task` | same | newBorn clear + interrupt re-queue |
| `craft_item_with_player_craft_ai` / `expand_craft_item_player_sticky` / `resolve_seek_or_craft_player_sticky` | same | sticky multi-tick craft expand |
| `Player::wipe_craft_on_birth` / `craft_ai_begin_tick` | `player.rs` | birth wipe + per-tick calledCraftItem guard |
"#;
            if let Some(idx) = t.find("| `FailedCraftings`") {
                t.insert_str(idx, block);
                let _ = std::fs::write(&ci, restore_nl(&t, crlf));
                any = true;
            } else if let Some(idx) = t.find("| `CraftAiRuntime`") {
                t.insert_str(idx, block);
                let _ = std::fs::write(&ci, restore_nl(&t, crlf));
                any = true;
            }
        }
    }

    // QUEUE — mark done
    let q = port.join("QUEUE.md");
    if let Ok(raw) = std::fs::read_to_string(&q) {
        let crlf = raw.contains("\r\n");
        let mut t = normalize_nl(&raw);
        if t.contains("| `AI-CRAFT-STICKY` | craft_runtime |") {
            t = t.replace(
                "| `AI-CRAFT-STICKY` | craft_runtime | Player sticky CraftAiRuntime / failedCraftings |\n",
                "",
            );
            if !t.contains("AI-CRAFT-STICKY") {
                // add to done recently
                if let Some(idx) = t.find("## Done recently") {
                    if let Some(eol) = t[idx..].find('\n') {
                        let insert_at = idx + eol + 1;
                        // find blank after header
                        if let Some(line2) = t[insert_at..].find('\n') {
                            let pos = insert_at + line2 + 1;
                            // prepend to done line if present
                            if t[pos..].starts_with("**") {
                                t.insert_str(pos, "**AI-CRAFT-STICKY** DONE · ");
                            }
                        }
                    }
                }
            }
            let _ = std::fs::write(&q, restore_nl(&t, crlf));
            any = true;
        }
    }

    // Changelog file
    let cl = port.join("changelog").join("2026-07-28-AI-CRAFT-STICKY.md");
    if !cl.exists() {
        let body = r#"# AI-CRAFT-STICKY / craft_runtime (2026-07-28)

## Status: DONE (core)

### Implemented

| Piece | Module | Notes |
|-------|--------|-------|
| `PlayerCraftAi` | `ol-sim/src/craft_ai_sticky.rs` | sticky shell: CraftAiRuntime + itemToCraftId + craftingTasks |
| `Player.craft_ai` | `player.rs` | survives ticks; birth wipe; begin_tick guard |
| `prepare_for_product` | same | Haxe product-change interrupt → addTask when countDone < count |
| `craft_item_with_player_craft_ai` | same | sticky craftItem |
| `expand_craft_item_player_sticky` / `resolve_seek_or_craft_player_sticky` | same | live expand with Player runtime |

### Tests

- `craft_ai_sticky::*` — newborn wipe, addTask, re-queue, cooldown sticky, continue sticky, task shift
- `player::craft_ai_sticky_on_player_defaults_and_survives`

### Residual

1. npc_ai full GetOrCraft multi-step enqueue (still shallow CraftItem walk)
2. countDone increment after live USE success (Haxe ~9086)
3. Top-down DoTransitionSearch filters → AI-CRAFT-MULTI

### Haxe anchors

- `AiBase.itemToCraft` / `failedCraftings` / `itemToCraftId` / `craftingTasks`
- `AiBase.newBorn` ~327–345
- `AiBase.addTask` ~5656
- `AiBase.craftItemHelper` product change re-queue ~6678–6690

### Verify

```powershell
cd C:\OhOl\OpenLife\openlife\RustServer
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo test -p ol-sim --lib -- craft_ai_sticky
cargo test -p ol-sim --lib -- craft_ai_sticky_on_player
```
"#;
        let _ = std::fs::create_dir_all(cl.parent().unwrap());
        let _ = std::fs::write(&cl, body);
        any = true;
    }

    // Update AI-CRAFT-MULTI residual #3
    let multi = port.join("changelog").join("2026-07-28-AI-CRAFT-MULTI.md");
    if let Ok(raw) = std::fs::read_to_string(&multi) {
        if raw.contains("Player.itemToCraft` + `failedCraftings` fields")
            && !raw.contains("AI-CRAFT-STICKY DONE")
        {
            let crlf = raw.contains("\r\n");
            let t = normalize_nl(&raw).replace(
                "3. `Player.itemToCraft` + `failedCraftings` fields on AI/NPC player structs (pure `CraftAiRuntime` ready)",
                "3. ~~`Player.itemToCraft` + `failedCraftings` fields~~ → **AI-CRAFT-STICKY DONE** (`Player.craft_ai`)",
            );
            let _ = std::fs::write(&multi, restore_nl(&t, crlf));
            any = true;
        }
    }

    any
}

/// Apply all AI-CRAFT-STICKY wires.
pub fn patch_all(src: &Path, workspace: &Path) -> bool {
    let lib = src.join("lib.rs");
    let player = src.join("player.rs");
    let a = patch_lib(&lib);
    let b = patch_player(&player);
    let c = patch_docs(workspace);
    let ok = already_wired(src);
    if ok {
        println!("cargo:warning=AI-CRAFT-STICKY: craft_runtime Player.craft_ai wired");
    } else {
        println!(
            "cargo:warning=AI-CRAFT-STICKY: partial wire lib={} player={} docs={}",
            a, b, c
        );
    }
    ok || (a && b)
}

/// Stamp path for build.rs.
pub fn stamp_path(src: &Path) -> PathBuf {
    src.join(".ai_craft_sticky_patched")
}
