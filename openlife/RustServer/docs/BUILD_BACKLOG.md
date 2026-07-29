# Build backlog — Open Life Reborn (Rust)

**Budget:** Weekly pool **reset ~19:05 UTC 2026-07-21** → **2026-07-28**. Live used about **10%** (GrokBuild); hard planning cap **80%**.  
**Playtest:** `cargo run -p ol-server --release` → game **:8005**, viewer **http://127.0.0.1:8080/viewer**, ops **http://127.0.0.1:8080/ops**.  
**Full Haxe parity is not claimed** — remaining legacy surface is still **large**. See [PROGRESS.md](PROGRESS.md) / [PROGRESS.html](PROGRESS.html) for done vs next.  
**Design:** [docs/design/NEXT_STEPS_MOVEMENT_WEB_AI.md](design/NEXT_STEPS_MOVEMENT_WEB_AI.md).  
**Systematic Haxe→Rust port:** [docs/port/README.md](port/README.md) · living checklist [docs/port/TODO_PORT.md](port/TODO_PORT.md) · workflow `haxe-port-chunk`.

## Playtest handoff (2026-07-22)

| | |
|--|--|
| Status | Workspace lib tests **green** (~648 ol-sim); `timed_movement` **default off** |
| Done (short) | Catch-up skip_ticks; MovePath+PM (flagged); USE/DROP range gates; self-play moving wait; OpsSeries+/ops; birth fitness 42; NPC scheduler stub |
| Next (short) | Client PM playtest → turn timed_movement on; full AiBase; nested OLW1; PHOTO/VOG **postponed**; vanilla client QA |

## Done (implemented subsets)

### Core loop & world
- [x] Shared world MC / USE / DROP / MOVE / vitals / nearby MX/PU
- [x] Multi-use last-use + eat + ticket + OLW1 + viewer
- [x] **Fast restart**: parallel content, bulk world I/O, deferred autosave, atomic write
- [x] **OLW1 backup rotation** (numbered `.bak.N`, default keep 3)
- [x] **World journal** append + simple size rotation
- [x] **MC on move** (threshold 10)
- [x] **Containers** put/REMV + MC wire (nested colon cells in-memory/wire; **not** OLW1-persisted)
- [x] **Auto-decay** scheduling + **arm on world load**
- [x] Ownership `owner_id` on DROP; **crime** theft classify + `?CRIME`
- [x] **Locks** / permissions LOCK/UNLOCK (session)
- [x] Mountain / bad biomes block MOVE; `?BIOMES`
- [x] Chunk interest **tier bookkeeping** `?CHUNKS` (not full skip-sim yet)
- [x] Special-object sparse index `?SPECIAL`
- [x] LOOK relative tile; MAPFORCE stub path where present

### Movement timings & transitions (2026-07-22 design)
- [x] PR1: Haxe catch-up `skip_ticks` + intent/tick EMA + `/api/metrics` fields
- [x] PR2: `MovePath` + PM formatter goldens + `NetIntent::Move.seq` + KA mid-path ignore + snapshot `moving`/`done_moving_seq` (`timed_movement=false` default)
- [x] PR4: USE/DROP/REMV block while moving; squared-Euclidean range d=1
- [x] PR5: self-play honor moving; no dest KA while moving; stuck_use rewrite; reason tags
- [x] PR6–7: OpsSeries ring + journal flush; `/ops` graphs; landing stats IA
- [x] PR8–9: birth fitness pure + fertile max 42 + children_birth_mali + fitness spawn hooks
- [x] PR10: single AI scheduler thread; `npc_enabled=false`; `npc_min=3` when enabled
- [x] PR11: PROGRESS / backlog truth-up; PHOTO/VOG still postponed
- [ ] Playtest: enable `timed_movement=true`; capture live Haxe PM; 60s unstick gate evidence

### Self-play & AI (thin)
- [x] Self-play mobile + respawn (pathfind + profession goals), agents 1–3
- [x] SAY PATH / STEPS / WALKABLE pathfind probes (gate/door name exception documented)
- [x] Reverse craft graph seed + `craft_graph_seed_cap` (skip ≤0 products/wildcard seeks)
- [x] `SAY RECIPE` / `SAY NEXTCRAFT` (held product ingredients / held→products)
- [x] Self-play craft-plan more: intermediate seeks, keep craft held, partner USE
- [x] Hunter hunt / flee goals; SeekFood radius; LASTUSE before USE
- [x] Evidence: self-play **5s** all agents moved, **boot_ms=19**; **selfplay9 ~10s boot_ms=377**; **selfplay8n ~8s boot_ms=15** all agents moved (2026-07-21)
- [x] NPC scheduler v1 (explore MOVE only; adaptive pop min/max)

### Social / lineage / accounts
- [x] **Social**: FOLLOW / EXILE, FW / EX packets, lineage bootstrap
- [x] **Lineage OLN1** binary save/load + `/lineage` + `/api/lineages`
- [x] **Accounts OLA1** soft-account binary save/load — boot / autosave / shutdown; `?ACCOUNT`
- [x] Allies directed links + `?ALLY`
- [x] Relations mother/child query helpers
- [x] Leadership ranking `?LEADER` / `?LEAD`
- [x] Naming on spawn + **RENAME** → NM nearby
- [x] Birth / fertility / gestation tick / HOLD / PUTDOWN / NURSE / FEED (+ poison sick heuristic)
- [x] Baby wiggle / dying formatters; graves on death (content-resolved id)

### Combat / prestige / economy
- [x] **Combat** KILL one-shot + **HIT** multi-wound + bleed drain + weapon range table + `?RANGE`
- [x] HEAL / BANDAGE / `?WOUND`
- [x] **PrestigeClass** fixed thresholds + living percentile refresh
- [x] **Economy** coins, PAY, TRADE/ACCEPT, `?COINS`
- [x] Treasury DONATE / TAX (leader) / `?TREASURY` + death inheritance
- [x] **Score** board, `?SCORE`, `?LEADERBOARD`, `?PRESTIGE`

### Environment
- [x] Seasons / temperature / day phase + biome food multipliers + queries
- [x] Indoor floor stub — floor id ≠ 0 halves `TEMP_FOOD_EXTRA`
- [x] Weather overlay + `/api/weather`
- [x] Snow cover overlay; fire tiles IGNITE/EXTINGUISH
- [x] Animals sparse wander + `?ANIMALS`
- [x] Move-speed notes (ride / snow / storm / fire)
- [x] HX heat emit; PE hunger + sleep snore

### Chat / meta
- [x] PHOTO deny ACK + VOG empty VU ACK (stubs)
- [x] GM **GLOBAL** broadcast (noble gate deferred)
- [x] DIE / EMOT / JUMP; EMOTE alias; YAWN; PING/PONG
- [x] WHISPER / SHOUT; SAY rate limit; separate emote rate limit
- [x] SLEEP/WAKE, SICK/CURE, RIDE/DISMOUNT
- [x] HOME / GOHOME / MARK
- [x] Clothing slots + CLOTHES query
- [x] CRAFT + backpack STORE/TAKE/INV
- [x] YUM + tool slots / LR; skills XP; tutorial tips
- [x] Curse / apocalypse / war / posse
- [x] Metrics logins/deaths/crafts; event log `?LOG`
- [x] Web APIs: metrics, players, selfplay, environment, weather, accounts, prestige, lineages
- [x] Pure **`afk`** book (idle/timeout helpers; kick not wired) + pure **`death_cause`** taxonomy (not substituted into every death path yet)
- [x] Pure **`reputation`** score float (≠ prestige / PrestigeClass; Haxe lostCombat labels; not fully combat-wired)
- [x] Pure **`mute`** list for chat + **SAY fan-out wired** (MUTE-SAY mute_delivery; WHISPER muted; DEAF separate)
- [x] Pure **`version_gate`** client data-version stub (policy + reject strings; LOGIN path not gated yet)
- [x] Pure **`math_wrap`** toroidal coord math + SAY `?WRAP`
- [x] Pure **`age_curves`** food_max/drain/fertility curves + SAY `?AGECURVE`
- [x] Pure **`food_fill`** eat/yum fill table helpers (formatters; not yet on eat path)
- [x] Pure **`heat_ideal`** comfort/extreme heat + SAY `?HEAT` (env temp sample)
- [x] Pure **`day_phase_names`** DAWN/DAY/DUSK/NIGHT + SAY `?DAY` / `?DAYPHASE`
- [x] Evidence: self-play **12s** all agents moved, **boot_ms=37**; workspace lib **664** pass (2026-07-21 ~20:36)

## Still open (honest remaining Haxe — large)

These are **not** nearly done. Subsets above do not equal product parity.

- [ ] Nested container **OLW1 persistence** + full multi-level product parity (wire nests exist; OLW1 drops nests by design today)
- [ ] Full **AiBase** craft trees, deep reverse craft, feed-other/flee/hunt stacks at Haxe depth (current goals + pathfind + craft-graph seed = thin)
- [ ] **LLM AI (Haxe parity)** — `AIProvider` HTTP + `AiHandler` rate-limit/retry/log + AiBase speech async reply (`respondToPlayerAsync`). Secrets via env/`.env` only (`AiApiKey` SecretOmit).
- [ ] Full combat weapons tables, wounds, animations, client FX (beyond HIT/KILL + name range + wounds)
- [ ] Birth / baby holding / motherhood **product** lifecycle (beyond gestation/hold/nurse slices)
- [ ] Real **PHOTO** backend and real **VOG** god-mode mutations (ACK stubs only)
- [ ] Full PlayerSoul / character pages / family-tree web product (OLN1/OLA1 + simple `/lineage` only)
- [ ] War / posse / crime **session dump** (`session_social.bin`) — still session-local
- [ ] Chunked vast-world **sim** (act on hot/warm/cold; tier bookkeeping only), deep animals, map-slice budgets
- [ ] Full WAL + multi-generation backup fidelity beyond OLW1 rotate + journal + OLN1/OLA1
- [x] **Same-server twins** — `TwinWaitQueue` / TWINJOIN / party birth (FERTILITY-TWINS core); residual edges only
- [x] Multi-server twin **stub** kept as-is (`TwinRegistry` + `twin_peers` + `?TWINS` / TWINPONG) — **no further multi-server twin work** (parked)
- [ ] Vanilla client QA (user-driven long sessions)
- [ ] GLOBAL / GM broader admin auth (GLOBAL already noble+ gated; operator model incomplete)
- [ ] Full clothing tables, beds, disease models, inventory UI protocol
- [ ] Full weather/fire/snow/map mutation + fauna AI (overlays/stubs only)
- [ ] Dead-code / API surface cleanup (`try_wear_held` / `try_strip` warn unused if only SAY paths wired)

## Log

| Date | Slice | Used % |
|------|-------|--------|
| 2026-07-26 | MUTE-SAY mute_delivery (MuteBook + send_chat_ps + WHISPER mute; DEAF separate) | — |
| 2026-07-21 | Viewer + world-first | 8% |
| 2026-07-21 | Fast restart + MC/containers/decay | 9% |
| 2026-07-21 | Social OLN1, combat/prestige, env, economy/score, pathfind/AI goals, yum/tools, curse/apoc/war/posse, PHOTO/VOG/GM, naming/BW formatters | **~12%** |
| 2026-07-21 | OLA1 accounts, sleep PE, indoor temp, RENAME, SAY DIE | **~14%** |
| 2026-07-21 | Broad sim expansion + doc truth-up; tests **370** lib; self-play 12s boot_ms=13 | **~16%** (pre-reset) |
| 2026-07-21 | Weekly reset ~19:05 UTC; used_percent **null→1**; cargo test --lib **482**; self-play 5s **boot_ms=19** all agents moved; PROGRESS+BACKLOG honesty pass | **1%** live (under 80%) |
| 2026-07-21 | Pure `afk` + `death_cause`; cargo test --workspace **517** (sim 449); selfplay9 **boot_ms=377**; PROGRESS feature inventory | **1%** (under 80%; goal cap still 80%) |
| 2026-07-21 | Pure `reputation` + `mute` + `version_gate`; `cargo test -p ol-sim --lib` **481** green; usage **2%** | **2%** (under 80%; soft day-1 ≈11%) |
| 2026-07-21 | Pure `math_wrap`+`age_curves`+`food_fill`+`heat_ideal`+`day_phase_names`; SAY HEAT/DAY/AGECURVE/WRAP; workspace **664**; selfplay12 **boot_ms=37** | **5%** live (under 80%; soft day-1 ≈11.43%) |
| 2026-07-21 | Pure `math_wrap`+`age_curves`+`food_fill`+`heat_ideal`+`day_phase_names`; SAY HEAT/DAY/AGECURVE/WRAP; workspace **664**; selfplay12 **boot_ms=37** | **5%** live (under 80%; soft day-1 ≈11.43%) |
