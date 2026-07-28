# File matrix: C++ (and Haxe) → Rust client

**Legend:** DONE | PARTIAL | STUB | MISSING | NA  
**Last reviewed:** 2026-07-28  
**Priority / open work:** [TODO_PORT.md](TODO_PORT.md) wins if this status column lags.  
**How the client works:** [README.md](README.md) · [ARCHITECTURE_RUST_CLIENT.md](ARCHITECTURE_RUST_CLIENT.md)

C++ root: `.../OneLife/gameSource/`  
Rust root: `C:\OhOl\OpenLife\RustClient\src\`

---

## A. Core C++ game

| ID | C++ | ~LOC | Rust target | Status | Notes |
|----|-----|------|-------------|--------|-------|
| C-GAME | `game.cpp` | 2300 | `main` + `ohol_client` + `ClientScreen` | PARTIAL | page graph Account/Loading/Playing/Death/Settings; headless CLI unchanged |
| C-LOAD | `LoadingPage.*` | 40+ | `load_progress` | **DONE→strong (P5#36)** | stages + prefer_cache hooks; soft-FB bar; `OHOL_LOAD_PROGRESS=1` |
| C-ACC | `ExistingAccountPage.*` | 700 | `account_page.rs` + ohol-client boot | **DONE→v1 (P5#37)** | soft-FB email + key/password; env prefill; Connect → SessionConfig; Esc skip-if-creds; headless CLI flags kept |
| C-LIVE | `LivingLifePage.cpp/.h` | 25k / 900 | `session` + `live_object` + `render` | **DONE→strong** | play path; not line-faithful dump of 25k LOC |
| C-MSG | `message.cpp` | 55 | frame helpers | DONE | `frame.rs` |
| C-PATH | `pathFind.cpp` | 514 | pathfind + click_tile | **DONE→strong** | cumulative MOVE; D=32 fixed arrays; unknown blocked; !blocksWalking (not permanent); wide radii; blocked-goal fail; long-axis nbrs; closestPathSpot; **bad-biome edge + rideable ignoreBad** (P2#9); **useWaypoint two-leg + maxWaypointPathLength** (P2#10) |
| C-GRID | `GridPos.*` | small | math | PARTIAL | i32 coords in map/live |

---

## B. Content banks (C++)

| ID | C++ | ~LOC | Rust | Status |
|----|-----|------|------|--------|
| C-OBJ | `objectBank.cpp` | 6415 | `content.rs` + OLC1 | **PARTIAL→stronger** | text + OLC1 **v6** + dummies; sim fields + wide radii + drawBehind + **heldInHand/rideable** + **body/head/foot indices** + **sideAccess/noBackAccess** + **getObjectCenterOffset/containOffset (P3#21)** + **setupSpriteUseVis (P4#25)** + **variableDummyIDs (P4#26)**; residual: numeral sprites / serial cycle / golden dummy CI |
| C-SPR | `spriteBank.cpp` | 1833 | `sprite_bank` + `tga` + `binpack` OLS1 + **OLSA (P4#40)** | **DONE→strong** | TGA+txt meta, Haxe atlas pack, OLS1 v2 (bbox+author), alpha hitMap+expandMap×3; soft-FB hover uses `get_sprite_hit`; optional **OLSA** full multi-page dump (`olsa_sprite_atlas.bin`, sealed pages, `--bake-sprite-atlas`) |
| C-ANIM | `animationBank.cpp` | 3825 | `anim_bank.rs` + OLA1 | **DONE→stronger** | full SpriteAnimParam text parse; OLA1 R/W (format 1 unchanged); bake; extras `7xN`; C++ `processFrameTimeWithPauses` + fade hardness+0.25; rot_center; fade α blit; `sample_slot`; ground2 alias; load_prefer_cache sha1/data_version; dual-anim/`inAnimFade`/frozen-rot/reseed runtime → **L-ANIM-DRAW DONE→stronger**; SoundAnimParam+footstep→floor via **L-SOUND-TRIG v2**; open residuals: authorTag not in OLA1, golden vs OneLifeData7, JenkinsRandomSource reseed |
| C-TRANS | `transitionBank.cpp` | 2860 | `content.rs` + OLT1 + `category_bank` expand + **`ol-binary`** | **DONE→stronger** | OLT1 v2 full fields + last-use + **max-use** maps; **lite+pattern category expand**; **find_ptrans** / **find_ptrans_max_use**; **P4#28** expanded bake; **P4#29** `transitions_max_use` + `switch_number_of_uses` (OLT1 bit6/bit7 + ServerSettings patches); **P4#30** shared server load; residual: reverse-use last-use auto-clone |
| C-CAT | `categoryBank.cpp` | 785 | `category_bank.rs` | **DONE→stronger** | `CategoryRecord` + reverse map; lite member expand + **pattern second pass** (index-pair); **pick_from_prob_set** / find_ptrans; load `categories/*.txt` into `load_from_dir` + `load_from_cache` (bank-only when OLT1 expanded **P4#28**); tests 722, 1790/1802 C+P, 3221; editor mutators **P4#27** |
| C-SND | `soundBank.cpp` | 1741 | `sound_bank.rs` + OLSN + `music_bank.rs` | **DONE→v4** | OLSN index bake/load; lazy mono-16 AIFF (Haxe @54); SoundUsage parse; SFX OGG index-only; **optional `audio`→cpal** device mixer + **stereo pan**; **P3#24 music bed** (`music/music_NN.ogg` lazy index + **lewton** decode on ensure, mono→device); open: wet reverbCache; music step/suppress session wire |
| C-GRND | `groundSprites.cpp` | 412 | `ground_sprites.rs` + OLG1 + **OLGA (P4#32)** | **DONE→v1+atlas** | Haxe 4×4 var index + TGA pack; OLG1 index bake/load (default); optional OLGA multi-page full dump; `graphics/ground_tN` overlays; lazy TGA; flat color fallback; residual: Haxe unknown recolor / C++ per-biome sheet layout |
| C-OVL | `overlayBank.cpp` | 386 | `overlay_bank.rs` + OLO1 | **DONE→v1** | disk/OLO1 id-tag-path index at boot; `get_overlay` + `ensure_image` lazy TGA; `bake_content` → `olo1_overlays.bin`; no eager Image+sprite dump; **out of scope:** editor `addOverlay`/`deleteOverlayFromBank`, thumbnailSprite mult-blit, OverlayPickable/EditorImportPage |
| C-FC | `folderCache.cpp` | 417 | replaced by CONTENT_BINARY | NA→design |
| C-BFC | `binFolderCache.cpp` | — | replaced by CONTENT_BINARY | NA→design |
| C-REGEN | `regenerateCaches.cpp` | 382 | `content_binary::bake_content` + CLI | **DONE→v1** | OLC1 v6 + OLT1 v2 + OLA1 + OLG1 + OLO1 + OLSN + **OLS1 (P4#31)** + manifest; auto-rebuild on stale/old format |

---

## C. Living systems (inside LivingLifePage — logical chunks)

| ID | Topic | Status |
|----|-------|--------|
| L-NET-PARSE | all inbound tags | **DONE→strong** (`tags.rs` + `parse_inbound`; full PU; CM inflate; multi-PU; MS/CX/CS/VS/FD/FL/CR/PJ/MN/GH; PS `*map`; rest Known) |
| L-LIVEOBJ | LiveObject fields + apply PU | **DONE** (`live_object.rs` + `session.world`; anim pack state) |
| L-MOVE | path, PM, FORCE, KA mid-move | **PARTIAL→stronger** | FM batching **DONE**; FORCE cancel+ack + done_moving flush **DONE**; KA 15s **DONE**; `logout_reset` **DONE**; trunc-cancel + artificial FORCE + baby-held cancel **DONE**; **fractional currentPos** / `findClosestPathSpot` **DONE** (P1#8); remaining: readyPending mid-move hold (deferred) |
| L-MAP | MC + MX + biomes/floors | **DONE** (`client_map.rs`; MC zlib + MX) |
| L-ACT | USE/DROP/REMV queue + click_tile + path-to-adjacent | **DONE→stronger** | queue + ground `click_tile` + object `click_object`/`walk_or_use_tile`; **click_gates**; **modClick** DROP/SWAP/REMV + multi-MOVE repath before flush (`multi_move_ext`); hitMap hover + RMB; **sideAccess/noBackAccess + food self-tile** (`plan_stand_for_object_ex`); **clothing-ux** DROP c 0..5 / self-tile equip / keys 1–6 / `send_sremv` + **worn soft-FB hitMap** (`pick_worn_clothing_slot`) + **contained hit_slot** (`HoverPick.contained_slot` → REMV/SREMV `i`) + **container_slot_ux** (map REMV/DROP/USE; clothing bag DROP put + LMB SREMV take; soft-FB clothing contained draw); **mouse-hold slide** (`slide_blocked_click_dest` / `walk_or_use_tile_hold`); **fractional currentPos** path start (**P1#8**); **bad-biome edge / rideable ignoreBad** (**P2#9**); **useWaypoint two-leg** (**P2#10**); **multi-MOVE ultimate repath** (**P2#11**); open: SHIFT/CTRL clothing polish |
| L-SAY | PS/LS/SAY | **PARTIAL→stronger** | parse + pointers + session apply to LiveObject/`location_speech`; TTL `3+len/5` + fade `0.05*frf`; chalkBlot+handwritingFont TGA (P3#15) w/ 5×7 fallback; `send_say` + ohol-client T; **P3#16 curse-tag reinsert / 15s tic + CU** DONE; **P3#17 map-pointer / `*label` UI** DONE (soft-FB pin + HUD arrow/label); open: +FAMILY+ flags, photo meta, age-speech truncate (speech→emote **P3#18 DONE**) |
| L-EMOT | PE / emotes | **DONE→stronger** | `emotion.rs` table from `contentSettings/emotionWords+Objects.ini`; PE TTL + permanent stack; pack select uses `extraAnimIndex` (not PE row); soft-FB body/face/head layers; session bank; **P3#18** `get_emotion_index` + `send_say`→EMOT; **P3#19** mainEyesOffset/eyesIndex, extraB toggle, mouth-skip, creation/decay SoundUsage (lazy OLSN) |
| L-FX | FX food / HX heat HUD | **PARTIAL→stronger** (parse + `session.food`/`heat`; peaks in `HudState`) |
| L-HUD | hunger boxes, arrows, slips, fonts | **PARTIAL→stronger (P1#3 DONE)** | `hud.rs` mult-blend + slip slide/wiggle + hunger.aiff; `HomePosStack` (MX homeMarker + PS temp priority); FX responsible_id defer; pencilFont/yum/home arrows/temp tip; residual: object names, chrome cache, dual ancient home slip |
| L-ANIM-DRAW | anim stacks on players/objects | **DONE→stronger** | pack select + dual-anim soft-FB; limb-hide/HoldingPos **DONE**; PE `extra`/`extraB` (**P3#19**); SoundAnim wired; **P3#22** action wiggle + baby-held handoff/drop/BW **DONE**; residual: young-baby lie-rot, adult held-clock copy on drop |
| L-RENDER | software scene (ysort, ground, clothing, containers, screen_to_world) | **PARTIAL→stronger** (`render.rs` + dual-pack soft-FB + HUD + tall-object + **HoldingPos/limb-hide** + hover outline + **PE emotion layers** + **clothing contained draw** + **rideable person-under-vehicle P3#20** + **getObjectCenterOffset/hideClosestArm P3#21** + **wallLayer/frontWall sub-order P3#23**; gaps: wgpu) |
| L-SOUND-TRIG | play sounds on events | **DONE→v7** | `handle_anim_sound_ex` + footstep→floor; OLC1 v4 sounds=; **shouldCreationSoundPlay**; MX/PU clothing/drop/baby; **device play** + **stereo pan**; **P2#13** MX contained fill + contained-slot using-on-fill + clothing bag fill + offScreenSound register; **P2#14** map/floor ground-anim step; **P3#24** music OGG bed (`music_bank`); open: wet reverbCache, OSS edge draw, music age-step wire |
| L-PHOTO | photos | NA/low |
| L-CURSE | curse UI | MISSING |
| L-LINEAGE | LN display | PARTIAL (parse) |

---

## D. UI pages (play-adjacent)

| ID | C++ | Status |
|----|-----|--------|
| U-REBIRTH | `RebirthChoicePage` | **DONE→v1 (P5#38)** | `client_screen.rs` DeathSummary + soft-FB; Playing→Death; R/Enter LOGIN reconnect |
| U-FINAL | `FinalMessagePage` | **DONE→v1 (P5#38)** | folded into Death summary page (name/age/reason) |
| U-SET | `SettingsPage` | **DONE→v1 (P5#39)** | `settings_page.rs` soft-FB SettingsPage; SFX/music vol+mute+show FPS; env/ini; F3 Account/Playing; Esc/Back; apply to banks |
| U-TWIN | `TwinPage` | MISSING |
| U-REVIEW | `ReviewPage` | MISSING |
| U-AUTO | `AutoUpdatePage` | NA (own updater later) |
| U-EDITORS | `Editor*Page` | NA (not for play) |

---

## E. Haxe Open Life client

| ID | Haxe | Rust | Status | Steal |
|----|------|------|--------|-------|
| H-CLIENT | `Client.hx` | session/frame | PARTIAL | CM decompress **ported** (`frame::inflate_cm`) |
| H-TAGS | `ClientTag.hx` | `tags.rs` ServerTag | **DONE** (enum + parse table) | full tag list |
| H-RENDER | `Render.hx` | `render.rs` + `ground_sprites` | PARTIAL→stronger | ground var+TGA, ysort floors/objects/players, soft FB, screen_to_world |
| H-BATCH | `SpriteBatch.hx` | render | PARTIAL | CPU blit + mult blend; no GPU batch |
| H-PACK | `BinPack.hx` | `binpack.rs` | **DONE** | free-rect split heuristic |
| H-BAKE | `ObjectBake.hx` | `content_binary` + **`ol-binary`** | **DONE→v1** | multi-use + **variableDummyIDs (P4#26)** + materialize + **setupSpriteUseVis (P4#25)**; OLC1 **v6**/OLT1 v2 R/W via **`ol-binary` (P4#30)**; last-use + max-use maps; auto-rebuild prefer_cache; OLA1+OLG1+OLO1+OLSN+**OLS1 (P4#31)** in `bake_content` |
| H-RES | `Resource.hx` | paths | PARTIAL | sprites + groundTileCache roots; content root + cache/; game graphics for HUD |
| H-OBJ | `Object.hx` | `render` draw_object | PARTIAL | parent chain + rot + ageRange + clothing/containers/heldOffset |
| H-SND | `Sound.hx` | audio | **DONE→v2** | mono-16 AIFF @54 (`read_mono16_aiff`); device via cpal optional; music via `Resource.music` path + `music_bank` (lewton) |

---

## F. Rust modules (current)

| File | Status | Maps to |
|------|--------|---------|
| `frame.rs` | **DONE** | `#` framing + MC skip + **CM zlib inflate** |
| `login.rs` | DONE | LOGIN/RLOGIN HMAC |
| `move_state.rs` | **DONE→strong** | MOVE/FORCE; pathToDest; `closest_path_spot`; repath; dest_truncated + artificial FORCE |
| `actions.rs` | PARTIAL | USE/DROP/REMV/SELF/… + `encode_force` / KA |
| `tags.rs` | **DONE** | Haxe ClientTag / protocol server→client tags |
| `parse.rs` | **DONE→strong** | full PU, PS pointers, secondary tags, `parse_inbound` |
| `event_util.rs` | DONE | probe helpers for structured events |
| `session.rs` | **PARTIAL→stronger** | FM wait; FORCE cancel+ack+clear multi; done_moving **multi-MOVE then adjacency flush**; KA 15s; trunc/baby cancel; `logout_reset`; `walk_to`; `path_and_act`/`path_and_use`/`queue_pending_action` |
| `click_tile.rs` | **DONE→stronger** | L-ACT ground MOVE + object path-to-adjacent + modClick + **click_gates** + **sideAccess/noBackAccess/food self-tile** + **container_slot_ux** (REMV/DROP/USE put + clothing bag take/put); multi-MOVE arm + **P2#11** keep ultimate goal on closest/window short hop; playtest tests multi-before-flush + far-goal repath |
| `multi_move_ext.rs` | **DONE** | multi-MOVE continue + ultimate-goal repath (**P2#11** `continue_multi_move_body`; done_moving repath when hop short of goal) |
| `hover_pick.rs` | **DONE** | L-ACT soft-FB hitMap hover (`get_sprite_hit`) + object/empty outline + **worn clothing slot pick** + **contained_slot** (map/clothing container `slot_pos`) |
| `rmb_action.rs` | **DONE** | L-ACT RMB/Q full modClick via `click_tile_mod` (DROP/SWAP/REMV) + `hit_slot` for contained REMV/SREMV |
| `live_object.rs` | **DONE→stronger** | L-LIVEOBJ + `AnimDrawState` + `step_anims` / fade-skip (L-ANIM-DRAW) |
| `client_map.rs` | **DONE→strong** | L-MAP MC/MX + object_raw container tree |
| `content.rs` | **PARTIAL→stronger** | C-OBJ/C-TRANS + sim fields + draw_behind + **held_in_hand/rideable** + body-part indices + **HoldingPos** + **side_access/no_back_access** + **C-CAT lite+pattern expand** + **find_ptrans** + **transitions_max_use / switch P4#29** |
| `category_bank.rs` | **DONE→stronger** | C-CAT CategoryRecord + reverse map; lite + **pattern index-pair** expand; **pick_from_prob_set**; open: editor mutators |
| `content_binary.rs` | **DONE→v6** | OLC1 v6 / OLT1 v2 via **`ol-binary` (P4#30)** + OLG1/OLS1 bake; **P4#28** expanded flag; **P4#29** max-use bit6 + switch bit7; maps DTOs ↔ ClientContent |
| `ol-binary` (shared crate) | **DONE** | Pure OLC1/OLT1 format parse+encode (zero dep); path: `openlife/RustServer/crates/ol-binary` |
| `sprite_bank.rs` | **DONE→strong** | C-SPR meta + Haxe atlas + OLS1 v2 + hitMap/bbox + **OLSA pixel pages (P4#40)** |
| `binpack.rs` | **DONE** | H-PACK |
| `tga.rs` | **DONE** | TGA 24/32 RLE |
| `anim_bank.rs` | **DONE→stronger** | C-ANIM full param text + **OLA1** R/W (fmt 1) + sample fidelity + `sample_sprite_ex` |
| `anim_draw.rs` | **DONE→stronger** | L-ANIM-DRAW packs + clothing helper + fade-skip; dual-anim sample, frozen-rot, reseed (deterministic stand-in) |
| `ground_sprites.rs` | **DONE→v1+OLGA** | C-GRND var index + TGA + OLG1 + overlays (`ground_tN`); lazy pack default; optional OLGA multi-atlas dump/load (**P4#32**) |
| `overlay_bank.rs` | **DONE→v1** | C-OVL scan/OLO1 index + lazy TGA + bake hook; open only editor UI (add/delete, thumbnailSprite, OverlayPickable) |
| `hud.rs` | **PARTIAL→stronger** | L-HUD mult-blend + slip motion/wiggle/sound + home arrows + temp tip + fade stacks; **P1#3** residual play-visible closed |
| `render.rs` | **PARTIAL→stronger** | L-RENDER + dual-fade + HUD + tall-object + **limb-hide/HoldingPos** person draw + **clothing contained soft-FB** (`slot_pos`) |
| `pathfind.rs` | **DONE→strong** | C-PATH fixed 32²; unknown blocked; permanent≠block; wide expand; blocked goal; long-axis nbrs; adjacent 1-step; **`PathFindOpts` / `find_path_ex` bad-biome + rideable `ignore_bad` + auto_click**; **`find_path_via_waypoint*` / `find_path_with_waypoint_ex` (P2#10)** |
| `wire_log.rs` | DONE | debug |
| `main.rs` | PARTIAL | CLI probes + **`--bake-content`** + **`--bake-sprite-atlas` / `--bake-ground-atlas`** + idle KA tick |
| `lib.rs` | DONE | exports |

---

## G. Recommended chunk queue

1. ~~**L-NET-PARSE** — complete inbound tag parse → enums/events~~  
2. ~~**L-NET-PARSE gaps** — CM inflate, full PU, multi-PU, secondary~~  
3. ~~**L-LIVEOBJ** — apply PU to LiveObject~~  
4. ~~**L-MAP** — MC binary decode + MX client map~~  
5. ~~**C-OBJ + C-TRANS** — content load text → then OLC1/OLT1 bake~~  
6. ~~**C-SPR + H-PACK** — TGA + BinPack atlas + sprite meta + OLS1 meta~~  
7. ~~**FM batching**~~ done  
8. ~~**L-MOVE force_flush_logout_ka**~~ FORCE/flush/KA/logout + trunc/artificial FORCE **DONE** (readyPending deferred)  
9. ~~**H-BAKE + CONTENT_BINARY** baker (OLC1/OLT1)~~ **DONE→v1** (format 2 write, materialize dummies, auto-rebuild)  
10. ~~**C-ANIM / ola1_anim_binary**~~ **DONE→stronger** (OLA1 fmt1 + full param parse + bake + sample/draw fidelity)  
11. ~~**Server `ol-content` load_from_cache**~~ **DONE** (`binary_cache` + prefer_cache; OLC1 v3 sim fields keep prefer-cache on binary)  
12. ~~**C-GRND / L-RENDER polish**~~ PARTIAL→stronger; ground+OLG1 + tall + limb/HoldingPos **DONE**; remaining: PE draw  
13. ~~**L-ACT / click_tile_to_move**~~ **DONE→strong** — cost-map fidelity + closestPathSpot + wide radii  
14. ~~**L-ANIM-DRAW packs**~~ **DONE→stronger** — dual-fade soft-FB + session/draw step; residual limb-arm / baby-held (extraB **P3#19 DONE**)  

15. ~~**L-ACT object path-to-adjacent USE**~~ **DONE→strong** — `click_object`/`click_use`/`click_drop`/`click_remv`; 4-neighbor stand; queue + adjacency flush on done_moving  
15b. ~~**L-ACT / click_gates**~~ **DONE** — playerActionPending, held 0-speed, age&lt;noMoveAge JUMP, baby-held JUMP  
15b. ~~**L-ACT / hitmap_hover_rmb_drop**~~ **DONE** — soft-FB `get_sprite_hit` hover id + RMB/Q DROP/REMV  
15c. ~~**L-ACT / modclick_drop_remv_swap_multimove**~~ **DONE→strong** — select DROP/SWAP/REMV; multi-MOVE before flush; biomes/rideable **P2#9 DONE**; useWaypoint **P2#10 DONE**; multi-MOVE ultimate repath **P2#11 DONE**
15d. ~~**L-ACT / clothing-ux**~~ **DONE** — DROP c 0..5 / SREMV / SELF + keys 1–6 + worn soft-FB hitMap  
15d. ~~**L-ACT / worn clothing soft-FB hitMap**~~ **DONE** — `pick_worn_clothing_slot` hover/click → DROP/SELF/SREMV  

15d. ~~**L-ACT / side_access_food_stand**~~ **DONE** — sideAccess W/E only; noBackAccess exclude N; permanent-food self-tile prefer; OLC1 flag bits  

16. ~~**OLC1 sim fields**~~ **DONE→v3**; ~~**OLG1**~~ **DONE→v1**; ~~**C-OVL / OLO1**~~ **DONE→v1** (editor UI residuals out of scope); ~~**shared loader crate**~~ **DONE (P4#30 `ol-binary`)**  
17. ~~**L-HUD click_walk + food/heat draw**~~ **PARTIAL→stronger** (mult-blend, OldArrow, max-fill line, logout clear; open: true fonts, yum/hunger slips, home arrows, hover tips, fade stacks, FX walk deferral, hideUI, excess_curse CS)  
18. **GPU path** — real wgpu/atlas (minifb shell exists; software FB primary)

Use matrix IDs with workflow `ohol-client-port-chunk` / `rust-client-port-headless`.
