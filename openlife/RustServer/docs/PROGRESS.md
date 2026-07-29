# Implementation progress — Open Life Reborn (Rust)

**Last updated:** 2026-07-26 (MUTE-SAY mute_delivery)  
**Budget:** Weekly SuperGrok pool reset **2026-07-21 ~19:05 UTC** → next **2026-07-28**. Live used about **10%** (GrokBuild); hard planning cap **80%**; implement gate **≤25%** for this plan.  
**Full Haxe parity is not claimed.** Large remaining surface — see below.

Also: [PROGRESS.html](PROGRESS.html) · [BUILD_BACKLOG.md](BUILD_BACKLOG.md) · [USAGE_BUDGET.md](USAGE_BUDGET.md)

**Haxe → Rust systematic port kit:** [docs/port/README.md](port/README.md) (architecture, dependency graphs, file matrix, full TODO, Haxe open-TODOs, call index). Workflow: `.grok/workflows/haxe-port-chunk.rhai`.

---

## Playtest (you)

```powershell
cd C:\OhOl\OpenLifeReborn
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
cargo run -p ol-server --release
# Game TCP: 8005 · Web: http://127.0.0.1:8080/viewer
```

| Item | Value |
|------|--------|
| Game port | **8005** |
| Web / viewer | **http://127.0.0.1:8080/viewer** |
| Health / APIs | `/health`, `/api/selfplay`, `/api/players`, `/api/environment`, `/lineage` |
| Ticket auth | **on** by default (`verify_ohol_ticket`) — set `false` in `server.toml` for offline clients |
| Self-play | **3** agents (forager/farmer/hunter) unless you lower `selfplay_agents` |
| Spawn | Grassland near map center (with AIs), not mountain corner |
| Disconnect | **`!CLOSE`** or **`/close` only** (bare `close` is speech — not a command) |

### Chat rules (important)

| What you type | Treated as |
|---------------|------------|
| Bare text (`i give berries`, `hello`, `close`) | **Speech only** — fans out as PS chat |
| Starts with **`/`** or **`!`** | **Server command** (e.g. `/HELP`, `!CLOSE`, `/CRAFT`) |
| Starts with **`?`** | **Query** (e.g. `?WHERE`, `?HELP`) |

Disconnect: type **`!CLOSE`** (Haxe) or **`/close`**. Saying “close” in chat does nothing special.

---

## Done vs next (short)

### Done (this rebuild)

- **Rust OHOL-style server**: multi-threaded sim, no SQL, embedded web, ticket toggle.
- **Fast restart**: parallel content load, bulk **OLW** world I/O (magic `OLW1`, **version 2** nested + container time) + backup rotation, **OLN1** lineages, **OLA1** accounts, journal; load-from-save boot often **~15–40 ms** `boot_ms`.
- **Core protocol loop**: SN/LOGIN, MOVE, USE, DROP, REMV, eat/multi-use, vitals, MC-on-move, nearby MX/PU.
- **Haxe-style catch-up ticks** (`skip_ticks` = extra advances / compressed dt, not tokio Skip drops) + intent/tick EMA.
- **Timed MovePath + PM** (flag `timed_movement`, **default off**): seq plumbing, walkability trunc, KA mid-path ignore, `PlayerSnapshot.moving` / `done_moving_seq`.
- **USE/DROP/REMV** gates: block while moving; **squared-Euclidean** `isClose` (default d=1).
- **Self-play**: honor moving snapshot; no destination KA while moving; stuck_use only on failed/unchanged object; reason-tagged unstick.
- **OpsSeries** RAM ring + ~5 min journal flush; web **`/ops`** graphs + `/players` `/stats/*` IA.
- **Birth fitness** pure module + fixtures; mother fertile max **42**; `children_birth_mali`; fitness spawn selection hooks.
- **AI NPC scheduler** (single thread, `npc_enabled=false` default, `npc_min=3` when on).
- **Containers / ownership / locks / crime**, nested **wire + OLW v2 persist** (one-level id nests).
- **Social + birth**: FOLLOW/EXILE/ALLY, leadership ORDER/OBEY, mute/deaf (SAY delivery wired), birth/gestation/hold/nurse, relations.
- **Combat / prestige / economy**: KILL/HIT wounds, heal, curse, reputation, pay/trade/gift/loan/treasury, scoreboard + season reset.
- **Environment**: seasons/day/temp/weather/snow/fire, animals + HUNT, pathfind, craft graph RECIPE/NEXTCRAFT.
- **Web viewer** + many `/api/*` surfaces.
- **Tests**: workspace lib green (**ol-sim ~648** unit tests).

### Next (honest remaining Haxe)

1. Enable **`timed_movement=true`** after OneLife client PM playtest (goldens + live Haxe PM capture).  
2. Full **AiBase** craft trees / priorities (NPC scheduler is v1 explore-only).  
3. Full combat weapons/animations; real **PHOTO** / **VOG** backends (**postponed** product).  
4. Motherhood product loop (pregnancy/hold UI multiplayer) beyond fitness hooks.  
5. Accounts/souls + real lineage web product.  
6. Hot/warm/cold **sim skip**, deeper animals, twin multi-server sockets.  
7. 60s self-play **unstick_total==0** gate evidence under timed_movement on.  
8. Deep multi-level nest **meta** (uses/owners per sub-item) / ground_id disk (OLW3).

---

## Evidence snapshot

| Check | Result |
|-------|--------|
| `cargo test --workspace --lib` | **green** (~723+; `ol-sim` **~648**) |
| Nested OLW | `nested_persisted_in_olw2` / `container_api_nested_save_load_roundtrip` / `olw1_v1_load_nested_empty` |
| Self-play | triple agents; moving-snapshot wait; reason-tagged unstick |
| Boot from save | **boot_ms** typically **15–40** (content load separate ~0.2–0.4 s) |
| Craft graph | ~**3052** products / ~**7555** edges |
| Budget | **~10%** used — under **25%** implement gate and **80%** hard cap |
| Design | `docs/design/NEXT_STEPS_MOVEMENT_WEB_AI.md` PR1–PR11 + review fixes (KA/self-play, ops delta journal, integration tests) |
| Tests | ol-sim **~663** lib tests green after review follow-up |

### Self-play motion (2026-07-21, ~12s run — OHOL helpers slice)

| Agent | t≈6s | t≈12s | Moved? | Notes |
|-------|------|-------|--------|-------|
| Forager `conn 9000001` | (246,254) | (247,253) | yes | held=31 at t12 |
| Farmer `conn 9000002` | (274,264) | (274,268) | yes | held=223 at t12 |
| Hunter `conn 9000003` | (273,262) | (272,267) | yes | exploring |

World load 500×500 helpers=4834; **boot_ms=37**; craft graph **products=3052 edges=7555**. Samples: `tmp/selfplay12-ohol/players_t*.json`.

### Prior self-play reference (same day)

| Run | boot_ms | Duration | Agents moved |
|-----|---------|----------|--------------|
| selfplay5 | 19 | ~5s | yes ×3 |
| selfplay8 | 27 | ~8s | yes ×3 |
| selfplay9 | 377 | ~10s | yes ×3 |
| selfplay8s | 16 | ~8s | yes ×3 |
| selfplay10s | 16 | ~10s | yes ×3 |
| selfplay8n | 15 | ~8s | yes ×3 |
| selfplay12-ohol (this) | 37 | ~12s | yes ×3 |

Boot variance is normal (disk cache / concurrent load / debug vs release binary).

### Prior AI goals slice (same day)

| Check | Result |
|-------|--------|
| `cargo test -p ol-sim --lib ai_goals` | **18 passed** (Harvest, smith `products_using` iron, hunter adjacent) |
| `cargo test -p ol-sim --lib craft_graph` | **16 passed** |
| `cargo test -p ol-server` | **6 passed** (self-play harvest/hunt sensors) |
| Notes | **AI goals expand:** Smith via craft-graph; Forager `Goal::Harvest`; Hunter adjacent hunt only |

### Self-play motion (2026-07-21, ~8s run — craft plan slice)

| Agent | Spawn (log) | Final API pos | Moved? | Notes |
|-------|-------------|---------------|--------|-------|
| Forager `conn 9000001` | (248,251) | (246,253) | yes | SeekFood / walk |
| Farmer `conn 9000002` | (268,263) | (273,274) | yes | **SeekObject(242)** USE craft-plan target |
| Hunter `conn 9000003` | (268,263) | (275,270) | yes | Hunt / explore |

World load 500×500 helpers=4834; **boot_ms=15**; craft graph **products=3052 edges=7555** (seeded=5225, cap=50000). Samples: `tmp/selfplay8n_players_t1.json`, `tmp/selfplay8n_selfplay_t1.json`.

### Prior self-play reference (same day)

| Run | boot_ms | Duration | Agents moved |
|-----|---------|----------|--------------|
| selfplay5 | 19 | ~5s | yes ×3 |
| selfplay8 | 27 | ~8s | yes ×3 |
| selfplay9 | 377 | ~10s | yes ×3 |
| selfplay8s | 16 | ~8s | yes ×3 |
| selfplay10s | 16 | ~10s | yes ×3 |
| selfplay8n (this) | 15 | ~8s | yes ×3 |

Boot variance is normal (disk cache / concurrent load / debug vs release binary).

---

## Feature inventory (landed systems)

These subsystems live in the Rust sim (`ol-sim` and related crates) with unit coverage. Each item is a **useful subset** of the legacy Haxe Open Life server, **not** full protocol or gameplay parity.

### Pure helpers

| Module | Role | Status |
|--------|------|--------|
| `ol_sim::afk` | `AfkBook` last-activity stamps, idle/remain secs, AFK list, `format_afk_query` (ok/warn/afk). Default timeout 600s. **No network kick wired yet.** | pure + tests |
| `ol_sim::death_cause` | `DeathCause` enum for `reason_hunger` / `age` / `killed` / `killed_legal` / `suicide`; parse, wire tags, PvP/natural/legal helpers, `format_death_event`, `combat_death`. **Not yet substituted into every death path.** | pure + tests |
| `ol_sim::reputation` | Combat **reputation float** separate from prestige / PrestigeClass. Haxe `lostCombatPrestige` ↔ stored `reputation = -lost`; 7 labels (`super_good`…`super_bad`); `ReputationBook` illegal/legal hit helpers; `format_reputation_query`. **Not wired into every combat path.** | pure + tests |
| `ol_sim::mute` | Per-listener chat **mute list** (`MuteBook`); mute/unmute/should_deliver; `format_mute_query`; parse MUTE/UNMUTE/LIST. **Wired**: SAY/SHOUT/MUMBLE via `send_chat_ps`; WHISPER muted; DEAF separate. | live + tests |
| `ol_sim::version_gate` | Client **data version gate stub** (`VersionGatePolicy` / `check_client_version`); default required **437**; exact / allow-newer / missing policies; reject message formatters. **Not wired into LOGIN/accept path yet.** | pure + tests (stub) |
| `ol_sim::biome_colors` | OHOL map PNG **RGB ↔ biome id/name** table (`BIOME_COLORS`, `Rgb`); `biome_id_from_rgb` / `name_for_biome` / `format_biome_colors_query`. Complements `ol_world::biome_from_rgba`. **Not wired into world gen.** | pure + tests |
| `ol_sim::object_tags` | Parse object **description-line** tags (`+tool`, `$10`, `#` comments, `@` dummy); `ObjectDescription`, `has_tag` / `tag_int_suffix`. **Not wired into content load.** | pure + tests |
| `ol_sim::wire_fields` | Pure string wire helpers: `parse_xy`/`xyz`, `#`-frame extract, `key=value` / csv i32, comment strip. Complements `ol_protocol::parse_message`. | pure + tests |
| `ol_sim::math_wrap` | Toroidal wrap math: `wrap_tile` / `wrap_delta` / Chebyshev·Manhattan·Euclidean on torus; `format_wrap_query`. | pure + tests; SAY `?WRAP` |
| `ol_sim::age_curves` | Age→food_max / drain mult / fertility curve / move mult; `format_age_curve_query`. | pure + tests; SAY `?AGECURVE` |
| `ol_sim::food_fill` | Food-value bands + yum fill + clamp helpers; `format_fill_table_query` / `format_food_fill_status`. | pure + tests (not SAY-wired yet) |
| `ol_sim::heat_ideal` | Ideal heat 0.5, comfort/extreme labels, food extra + move mult; `format_heat_ideal_query`. | pure + tests; SAY `?HEAT` |
| `ol_sim::day_phase_names` | Standalone DAWN/DAY/DUSK/NIGHT from hour; food mult; `format_day_query` / `format_day_phase_query`. | pure + tests; SAY `?DAY` / `?DAYPHASE` |

### Fast restart and persistence

| Feature | Notes |
|---------|-------|
| Parallel content load (rayon) | objects + transitions timed boot |
| OLW world I/O | magic `OLW1`; **write version 2**; load v1+v2 |
| OLW backup rotation | numbered `.bak.N`, default keep 3 |
| World journal | append-only tile changes; size rotation |
| Nested containers | **OLW v2** one-level id nests + top helper time; wire colon cells; v1 load empty nests |
| Lineage OLN1 | `SaveFiles/lineages_v1.bin` load/save round-trip |
| Accounts OLA1 | `SaveFiles/accounts_v1.bin` boot / 60s autosave / shutdown |
| War / posse / crime | **session-local** (no `session_social.bin` yet) |

### Map, containers, ownership

| Feature | Notes |
|---------|-------|
| MC-on-move | resend threshold **10** tiles; chunk 32×30 |
| Containers | DROP put + REMV take; nested colon cells on wire; **persist** via OLW v2 |
| Ownership | `owner_id` on DROP; crime theft classify + `?CRIME` |
| Locks / permissions | LOCK/UNLOCK session; owned gate access |

### Social / lineage / birth

| Feature | Notes |
|---------|-------|
| FOLLOW / EXILE | FW / EX packets + login bootstrap |
| Allies | directed `ALLY` / `?ALLY` |
| Naming + RENAME | random spawn names; NM nearby |
| Birth / fertility / gestation | SAY BIRTH/GESTATE, tick spawn, HOLD/PUTDOWN, NURSE/FEED |
| BW / DY | baby wiggle + dying formatters (starving infants) |
| Relations / leadership | mother/child/gen; `?LEADER` rank |

### Combat / prestige / economy / score

| Feature | Notes |
|---------|-------|
| KILL / HIT | one-shot + multi-wound bleed; weapon range heuristic |
| HEAL / BANDAGE | `?WOUND` |
| PrestigeClass | fixed thresholds + living percentile refresh |
| Economy | wallets, PAY, TRADE/ACCEPT, treasury DONATE/TAX |
| Inheritance | coins → mother else treasury; graves on death |
| Scoreboard | `?SCORE` / `?LEADERBOARD` / `?PRESTIGE` |

### Environment

| Feature | Notes |
|---------|-------|
| Seasons / temp / day phase | biome food mult; extreme-temp drain |
| Weather overlay | storm drain/slow; `?WEATHER` / `/api/weather` |
| Snow cover | winter blanket move/food notes |
| Indoor floor stub | floor id ≠ 0 halves temp extra |
| Fire | ignite/extinguish hazard tiles |
| Animals | sparse spawn/wander + `?ANIMALS` (thin AI) |

### Pathfinding / AI / self-play

| Feature | Notes |
|---------|-------|
| A\* pathfind | 4-connected; gate/door name walkable exception |
| Profession goals | Forager / Farmer / Smith / Explorer / Hunter |
| Reverse craft graph | seed cap `craft_graph_seed_cap` (default 50k); skip ≤0 product/wildcard seeks |
| SAY RECIPE / NEXTCRAFT | held-as-product ingredients; held-as-ingredient products |
| Self-play | 1–3 agents; craft-plan intermediate + keep held ingredients; `/api/selfplay` |
| Chat path probes | SAY PATH / STEPS / WALKABLE |

### Chat / meta / vitals notes

| Feature | Notes |
|---------|-------|
| SAY / SHOUT / WHISPER / MUMBLE | range fan-out; rate limits; **mute/DEAF filter** |
| EMOTE / YAWN / PING | PE + separate emote rate |
| SLEEP/WAKE, SICK/CURE, RIDE/DISMOUNT, SIT/STAND | vitals / speed notes |
| DIE | voluntary `reason_suicide` |
| GLOBAL | GM broadcast; **noble+** prestige gate (wired) |
| PHOTO / VOG | ACK/deny **stubs only** |
| HOME / MARK / GOHOME | markers |
| NOTE / ?NOTES | personal journal on player; max 5 lines (80 chars each) |
| Clothing / CRAFT / backpack | slots + STORE/TAKE/INV |
| YUM / tools / skills / tutorial | variety, LR, XP, tips |
| Curse / apocalypse / war / posse | token + phase + relation subsets |
| Twins | **stub only** `TwinRegistry` + `?TWINS` (no sockets) |
| Metrics | logins/deaths/crafts; `/health` `/api/metrics` |

### Web surface

Viewer `/viewer`; APIs: world summary/overview/view, players, selfplay, environment, weather, accounts, prestige, lineages; lineage HTML page.

---

## Skeptic fixes (required)

| Issue | Fix |
|-------|-----|
| Container `\|\| true` always allowed puts | **Removed.** Only content `containable` items enter slots; non-containable DROP → ground only if tile empty. Unit-tested reject path. |
| Auto-decay not armed on load / DROP | **`arm_decays_for_loaded_world`** after load-from-save; DROP schedules decay; large maps arm thousands of timers. |
| Nested containers dropped on save | **OLW version 2** writes one-level `nested[i]` + container time; load v1→empty nests; tests `nested_persisted_in_olw2` |

---

## Remaining Haxe (still open — honest, large)

**This is not full Haxe parity.** Major areas from the legacy Open Life / OHOL-style server are still missing, stubbed, or only partially covered. Prefer shipping useful slices under the **80%** SuperGrok cap rather than claiming completion.

1. **Full AiBase** — profession craft trees, reverse transition graphs at Haxe depth, feed-other priority stacks, flee/hunt sophistication, optional LLM AI. Current pathfind + profession goals + craft-graph seed is a **thin slice only**.

2. **Deep nested container meta** — multi-level full ObjectHelper fields (uses/owners/custom) on sub-items; `ground_id` disk (OLW3). **One-level id nests already persist in OLW v2.**

3. **Full combat product** — weapon rules tables, wound/animation sequences, kill range edge cases beyond HIT/KILL + name-based range.

4. **Birth and baby lifecycle product** — full pregnancy, holding UI protocol, nursing multiplayer loop, BW/DY in live client sessions. Gestation/birth/hold/nurse slices exist; motherhood game is **not** done.

5. **PHOTO / VOG backends** — real photo signatures and Voice-of-God mutations (ACK/deny stubs only).

6. **Accounts, souls, lineage product** — PlayerAccount / PlayerSoul parity, character pages, family-tree web. OLN1 + OLA1 + simple lineage page are subsets only.

7. **World-scale systems** — hot/warm/cold **simulation** skipping (tier bookkeeping exists), sparse special indices at Haxe depth, animal AI beyond wander, map-slice budgets, snow/season **map mutations**.

8. **Crash-safe WAL** and multi-generation backup fidelity beyond OLW rotate + journal + OLN1/OLA1. War/posse/crime **session dump** still open.

9. **Multi-server / twin code** — **stub only**: pure `TwinPeer` / `TwinRegistry`, config `twin_peers = []`, `SAY ?TWINS`. No ping, handoff, or inter-server sockets.

10. **Vanilla client QA** — long-session play with official OneLife client for every tag edge case.

11. **GLOBAL / GM policy** — noble+ gate exists for GLOBAL; broader admin auth / operator model incomplete.

12. **Ownership / crime / lock product** — owner_id, theft counters, session locks exist; full grief-prevention and transfer rules are incomplete.

13. **Clothing, sleep, sickness, trade, backpack** — SAY-driven subsets; not full Haxe tables / bed objects / disease models / inventory UI parity.

14. **Weather / fire / snow / animals** — stubs and overlays, not full Haxe environment + fauna simulation.

15. **AFK kick / death_cause integration** — pure modules landed; live disconnect on AFK and unified death-path enum substitution **not** fully wired into sim tick.

Continue shipping useful slices while SuperGrok used stays under **80%**. Do **not** treat the checklist above as nearly done.

---

## Run

```powershell
cd C:\OhOl\OpenLifeReborn
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
.\scripts\usage-budget\fetch-usage.ps1
.\scripts\usage-budget\read-usage.ps1
cargo test --workspace
cargo run -p ol-server --release
# http://127.0.0.1:8080/viewer
# http://127.0.0.1:8080/api/environment
# http://127.0.0.1:8080/api/selfplay
# http://127.0.0.1:8080/api/players
```
