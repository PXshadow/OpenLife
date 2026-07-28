# Client port TODO — done vs not

**Living checklist — only place for priority open/done rows.** Update on every chunk.  
**Hub / how it works:** [README.md](README.md) · **Playable bar:** [PORT_COMPLETE.md](PORT_COMPLETE.md) · **Modules:** [ARCHITECTURE_RUST_CLIENT.md](ARCHITECTURE_RUST_CLIENT.md)  
**Also:** [FILE_MATRIX.md](FILE_MATRIX.md) · [CONTENT_BINARY.md](CONTENT_BINARY.md) · server kit separate (`RustServer/docs/port`).

Last updated: **2026-07-27**  
**Priority automation:** **FINISHED** — all Missing-work P1–P5 rows (#1–#40) are **DONE** / **DONE-NA**. No open finish slots; do not launch more finish workflows (scheduler idle).  
**Workflow names:** see [README.md §6](README.md#6-workflows-automation) (do not re-list every run here).

---

## Missing work (highest priority first)

Playable core (move / USE / DROP / clothing / speech / sound *triggers*) is largely **done**.  
Priority Missing-work table (**P1–P5**) is **fully closed** (all rows DONE / DONE-NA). Residuals and deferred items remain below or in non-goals.

### P1 — Playtest quality (do next)

| # | Item | Area | Notes |
|---|------|------|--------|
| 1 | ~~**Real device sound playback**~~ | L-SOUND-TRIG / C-SND | **DONE** — optional `--features audio` (cpal); dedicated `ohol-audio` thread; `mix_voices_f32` mono→all-ch linear resample (32 voices); soft-fail / `OHOL_AUDIO_DISABLE`; boot OLSN index-only (`aiff_opens==0`); `play_id`/`play_usage` unchanged headless. Music OGG beds: **P3#24 DONE**. |
| 2 | ~~**Stereo pan / camera vector**~~ | L-SOUND-TRIG | **DONE** — C++ `getVectorFromCamera` + `getVolumeAndPan` (sigmoid distance, X pan→[0,1]) + constant-power L/R (`cos/sin`); `MixVoice` left/right gains in `mix_voices_f32`; `play_usage_at` / `play_id_at` + listener (SceneRenderer camera / session); `handle_anim_sound` spatial; dry path only (`reverb_mix` computed; wet reverbCache residual). `play_id`/`play_usage` center. Optional: wire MX/PU event sounds to map coords for pan. |
| 3 | ~~**HUD residual (play-visible)**~~ | L-HUD | **DONE** — slip slide/wiggle + `hunger.aiff` oneshot/pulse; C++ `homePosStack` (permanent homeMarker MX + temp PS priority + person track); FX `responsible_id` defer while feeder `moving` then flush; pencilFont/yum/hunger slips/home arrows/temp tip already strong. Residual: true object names on last-ate; optional chrome binary cache; music suppress on starve; dual home slip (ancient) draw |
| 4 | ~~**Worn clothing soft-FB hitMap**~~ | L-ACT | **DONE** — `pick_worn_clothing_slot` / `pick_at_screen_with_clothing`; LMB/RMB via `walk_or_use_tile_ex` / `click_rmb_tile_ex`; magenta outline; keys 1–6 still work |
| 5 | ~~**Contained SREMV `hit_slot` pick**~~ | L-ACT | **DONE** — soft-FB `HoverPick.contained_slot` via contained sprite hitMap (`slot_pos` reverse draw order); map REMV + clothing SREMV; `walk_or_use_tile_ex` / `click_rmb_tile_ex` / hold pass `hit_slot`; GUI LMB/RMB + orange outline |
| 6 | ~~**Container slot UX**~~ | L-ACT | **DONE** — full open/take/put for map containers + clothing bags: empty → `REMV x y i#` (`hit_slot`); held → USE / DROP into (modClick; LMB free-slot put); clothing bag put `DROP c` + LMB soft-FB contained take `SREMV c i`; soft-FB draws worn clothing contained at `slot_pos`; path-to-adjacent; tests `container_slot_ux_*` / clothing-contained draw / worn LMB SREMV; residual: SHIFT/CTRL clothing polish, behindSlots interleave (contained on top of all clothing layers) |
| 7 | ~~**Continuous mouse-hold / blocked-tile slide**~~ | L-ACT | **DONE** — `slide_blocked_click_dest` / `walk_or_use_tile_hold`; LMB hold repath + major-axis slide after `MIN_MOUSE_DOWN_FRAMES`; ground-only while held; gates respected; ohol-client wired |
| 8 | ~~**Fractional `currentPos` for path start**~~ | L-MOVE | **DONE** — `MoveState::{current_pos_x/y, step_current_pos, closest_path_spot}` (`lrint` + path snap); PM `total_sec` speed; `path_start_tile` / hold-slide origin; session `step_anims`/`step_move_pos`; residual: turn-smoothing / camera draw pos polish |

### P2 — Path / world fidelity

| # | Item | Area | Notes |
|---|------|------|--------|
| 9 | ~~**Bad-biome edge routing / rideable `ignoreBad`**~~ | pathfind | **DONE** — `PathFindOpts` + `find_path_ex` / BB parse; floor-less BB biomes block; edge entry; same-biome walk; rideable `ignore_bad`; session `bad_biomes` from BB; click_tile / stand plan; hold `isAutoClick` via `path_find_opts_with` / `click_tile_with` / `plan_click_tile_chunks_with`; session BB integration tests; residual: HUD `mBadBiomeNames` display-only |
| 10 | ~~**useWaypoint two-leg pathFind**~~ | pathfind | **DONE** — C++-faithful two-leg (`shared blocked map`; both legs must reach; `pathLength>maxWaypointPathLength` → rewrite dest to waypoint; fail → direct); `find_path_via_waypoint_ex` / `find_path_with_waypoint_ex` + `MoveState::{use_waypoint,arm_waypoint}`; ground `plan_click_tile_chunks_goal`/`click_tile_with`; stand `plan_stand_for_object_with_opts_wp`/`click_object`; multi-MOVE `eff_goal`; close-hold AABB throw (1..4 tiles, throw len 4); unit + click_tile tests (length threshold + two-leg routes); residual: road auto-walk + waypoint edge polish (minor) |
| 11 | ~~**Multi-MOVE closest-fallback → ultimate goal repath**~~ | multi_move | **DONE** — on `done_moving` after hop short of goal: `continue_multi_move` repaths toward `multi_move_goal` (window clamp / closest-fallback), arms next MOVE before flush; world dest sync; tests far-goal chain + stuck closest |
| 12 | ~~**`mMapGlobalOffset` sendX/sendY**~~ | L-ACT | **DONE-NA** — storage frame == wire frame; identity `MapGlobalOffset::ZERO` + `encode_move_with_offset`; [MAP_GLOBAL_OFFSET.md](MAP_GLOBAL_OFFSET.md) |
| 13 | ~~**Contained MX using-on-fill / offScreenSound**~~ | L-SOUND-TRIG | **DONE** — Contained MX using-on-fill / offScreenSound (L-SOUND-TRIG v6): MX fill using + contained-slot creation/using-on-fill (multi-use less-used); clothing bag fill PU; spatial `play_usage_at`; OSS register anim+held creation (skip self); 39 sound_bank tests + self-check OK. Residual: soft-FB OSS edge-arrow draw. |
| 14 | ~~**Map object ground-anim sounds**~~ | L-SOUND-TRIG | **DONE** — `ClientMap::{anim,floor}_frame_count` + `step_map_ground_anims_with_sounds` (ground/floor SoundAnimParam via `handle_anim_sound_ex`); wired in `session.step_anims`; OLSN still lazy; headless period/oneshot tests. Residual: moving-map-object anim type, draw-path-only culling. |

### P3 — Presentation polish

| # | Item | Area | Notes |
|---|------|------|--------|
| 15 | ~~**Speech chalkBlot / handwritingFont TGA**~~ | L-SAY | **DONE** — `chalkBlot.tga` white-sprite + `font_handwriting_32_32.tga` atlas; tiled multi-hit blots; 5×7 fallback |
| 16 | ~~**Speech curse-tag reinsert / 15s tic**~~ | L-SAY | **DONE** — CU parse/apply, curse-tag reinsert after non-tag bubble clear, 15s nervous tic, babble wrap when gap>15s; **purple soft-FB ink** (`speech_text_rgb` / `draw_speech_bubble_colored`) for successful curse + white cursed speakers / dying; **`mCurseSound`** lazy `otherSounds/curseChime.aiff` via `SoundBank::play_curse_sound_at` on PS isCurse. Residual: `+FAMILY+` `isGeneticFamily` flag (display skip already present) |
| 17 | ~~**PS map-pointer / `*label` UI**~~ | L-SAY | **DONE** — `LiveWorld.says_pointers` stores PS `*map`/`*label` + TTL (`map_age_seconds` or speech hold; expert +120s); soft-FB map-spot pins + label markers; HUD home-arrow + `map_pointer_label`; bubble = stripped spoken only; 410 lib tests + self-check. Residual: meters-away/years-ago speech rewrite; permanent home stack / map-drop `temporaryExpireETA`; temp-home priority trump; overhead person labels ↔ temp-home ETA; `*photo` metadata |
| 18 | ~~**Speech → emote `getEmotionIndex`**~~ | L-EMOT | **DONE** — `EmotionBank::get_emotion_index` (exact trigger) + `classify_speech_outbound` + `encode_emot`; `session.send_say` routes `/happy`→`EMOT 0 0 N#`, plain→SAY, other `/`→local; residual: local fps/die commands, age speech truncate |
| 19 | ~~**PE eyes offset / extraB / emot creation sounds**~~ | L-EMOT | **DONE** — eyes offset + `ANIM_EXTRA_B` PE toggle (dual-fade A/B) + mouth-skip when `mouthEmot` + creation/decay SoundUsage on PE apply/clear (lazy OLSN). 420 lib tests + `ohol-headless --self-check` OK. Residual: OSS edge-arrow soft-FB (L-SOUND); avoid double PE-TTL tick if both `session.step_anims` and `SceneRenderer.draw` per frame. |
| 20 | ~~**Rideable person-under-vehicle draw order**~~ | L-RENDER | **DONE** — C++ LivingLifePage rideable path: vehicle at person pos (not hand HoldingPos); order behind vehicle → rider/clothes/emotes → front vehicle; ridingOffset ≈ −heldOffset; tests `rideable_person_under_vehicle_draw_order` + `rideable_vehicle_at_person_pos_not_hand`. Residual: age body offset polish, hideRider, held-contained on vehicle |
| 21 | ~~**`getObjectCenterOffset` / hideClosestArm ±1**~~ | L-RENDER | **DONE** — `get_object_center_offset` (widest + CCW rot + containOffsetX/Y + only-when-worn skip); non-person held subtract in `compute_held_draw_pos_ex`; SceneRenderer sprite-bank alpha bbox; `arm_holding_parameters` 0/−2/rideable + draw hide ±1/body HoldingPos; residual: OLC1 `only_when_worn` bit (text path OK) |
| 22 | ~~**Action wiggle / baby-held handoff anim**~~ | L-ANIM-DRAW | **DONE** — action-wiggle cosine bounce on person/held draw; BW+JUMP baby arm-wiggle; baby put-down `heldByDropOffset` + held→ground pack handoff; `heldPosOverride` pick-up slide; soft-FB skips held babies / draws at adult HoldingPos. 435 lib tests + self-check OK. Residual: young-baby lie-down rot while drop settles; adult held-track clock copy on drop; draw-path frf for heldPos step |
| 23 | ~~**wallLayer / frontWall sub-order**~~ | L-RENDER | **DONE** — C++ `setupWall` (`floorHugging`/`+wall`/`-wall`/`+frontWall`) on `ClientObjectDef`; front DrawLayer sub-order permanent non-wall → non-permanent → wall → frontWall after players; OLC1 `floorHugging` flag bit11; tests `setup_wall_layer_*` + `wall_layer_front_wall_sub_order` + `front_object_draw_layer_ordering`. Residual: moving-map / flying-held interleave |
| 24 | ~~**Music OGG / `music_NN.ogg` bed**~~ | C-SND | **DONE** — `music_bank.rs`: lazy `music/music_NN.ogg` index (boot `ogg_opens==0`); age-block select (`next_music_block` / C++ musicPlayer2); **lewton** Vorbis decode only on `ensure`/`play_block`; mono mixdown → existing cpal mixer under `--features audio`; synthetic + optional real-tree tests. Residual: continuous `stepMusicPlayer` age-async crossfade / suppression stack / musicHeadroom wire into session. |

### P4 — Content / engine backlog

| # | Item | Area | Notes |
|---|------|------|--------|
| 25 | ~~**`setupSpriteUseVis` / per-use `spriteSkipDrawing` on dummies**~~ | C-OBJ | **DONE** — C++ `setupSpriteUseVis` ported; multi-use dummies progressive `spriteSkipDrawing`; parse `useVanishIndex`/`useAppearIndex`; OLC1 **v5+** use-vis trailer (write **v6**); draw/hit respect `skip_drawing`; residual: golden vis CI vs OneLifeData7 |
| 26 | ~~**`variableDummyIDs` generation**~~ | C-OBJ | **DONE** — `assign_variable_dummies` / `materialize_variable_dummy_object_records` (C++ `$N` + letter/numeral labels + parent `- ?`); OLC1 **v6** lists; multi-use then variable id order; residual: `setupNumericSprites`, `+varSerialNumber` cycle |
| 27 | ~~**Category editor mutators**~~ | C-CAT | **DONE** — in-memory C++ mutators: add/remove/move members, pattern/probSet flags, `set_member_weight`/`make_weight_uniform`/`auto_adjust_weights`, reverse move; `format_category_txt` serialize; play path unchanged (no disk). Residual: disk `saveCategoryToDisk` optional editor I/O |
| 28 | ~~**Bake expanded OLT1 (skip re-expand on load)**~~ | C-TRANS | **DONE** — `OLT1_F_CATEGORY_EXPANDED` header flag; bake after lite+pattern expand; load skips re-expand (bank-only for pick); legacy re-expands; prefer_cache rebakes once if flag absent |
| 29 | ~~**OLT1 `transitions_max_use` / switch tables**~~ | C-TRANS | **DONE** — OLT1 `transitions_max_use` + `switch_number_of_uses` server parity (text load, category expand, bake, load); Haxe `targetRemains` pair split; OLT1 bit6/bit7; ServerSettings dough/masa patches. Residual: reverse-use last-use auto-clone; optional live golden max_use count |
| 30 | ~~**Shared client/server OLC\* crate**~~ | content | **DONE** — `ol-binary` shared crate (`RustServer/crates/ol-binary`): pure OLC1 v1–6 / OLT1 v1–2 DTO parse+encode (zero deps). Client `content_binary` + server `ol-content::binary_cache` both map DTOs. Server OLT1 bit6→`transitions_max_use` / bit7→`switch_number_of_uses`; OLC1 format max raised to v6 (sprites/sounds skipped). Residual: OLA1/OLG1/OLO1/OLSN/OLS1 still client-local; server `finish_cache_boot` always re-expands categories (safe, not skip-on-flag) |
| 31 | ~~**OLS1 written by `bake_content`**~~ | C-SPR | **DONE** — `bake_content` writes `ols1_sprites.bin` + manifest; `SpriteBank::load_prefer_cache` rebakes if missing/stale; pixel pages → **P4#40 OLSA** (optional) |
| 32 | ~~**Full SaveGroundData multi-atlas dump**~~ | C-GRND | **DONE** — optional **OLGA** multi-page full ground atlas (`olga_ground_atlas.bin`): pack-all + write/load rects+RGBA pages; CLI `--bake-ground-atlas`; `GroundBank::load_prefer_atlas_cache`. Default play path remains **OLG1** index + lazy TGA; `bake_content` omits OLGA by design (size). Residual: Haxe unknown-biome white-pixel recolor tints not packed; C++ per-biome `GroundSpriteSet` sheets not mirrored (Rust multi-page BinPack); raw RGBA single-file ≠ Haxe `ground.png`+`SaveGroundData.bin` serializer |
| 33 | ~~**Anim `authorTag` in OLA1**~~ | C-ANIM | **DONE** — parse text `author=`; OLA1 **v2** record trailer `u16+utf8`; write v2; load v1+v2; residual: none for author |
| 34 | ~~**Golden anim vectors vs OneLifeData7**~~ | C-ANIM | **DONE** — synthetic golden `sample`/`frame_time`/`fade` vectors always; live person `19_2`/`19_0` when tree present (sprite counts + layer params + OLA1 roundtrip samples); residual: expand golden set / CI artifact export optional |
| 35 | ~~**Golden dummy id sequence vs server**~~ | C-OBJ | **DONE** — synthetic oracle (sorted multi-use parents from `nextObjectNumber`); live OneLifeData7 multi-use lists vs same C++/server algorithm; OLC1 roundtrip preserves lists. Residual: variable-dummy client-only sequence golden optional |
| 40 | ~~**Sprite atlas pixel pages (disk cache)**~~ | C-SPR | **DONE** — optional **OLSA** (`olsa_sprite_atlas.bin`): pack-all TGA + write/load rects+RGBA pages; CLI `--bake-sprite-atlas`; `SpriteBank::load_prefer_atlas_cache` / `load_olsa_timed`. Default play = OLS1 + lazy TGA. **Timings:** `OlsaBakeStats` pack/write/total; load_bench `sprite_atlas_load` when present; CLI prints ms. HitMaps rebuilt from page alpha. Residual: full-tree bake can be large; SHA1 fingerprint beyond dataVersion optional |

### P5 — Product pages (optional)

| # | Item | Notes |
|---|------|--------|
| 36 | ~~**Loading progress UI**~~ | **DONE** (C-LOAD) — `load_progress` stages+fraction; prefer_cache hooks; `boot_load_prefer_cache`; soft-FB C++ LoadingPage bar; ohol-client Account→Loading→Playing via `connect_with_content`; `OHOL_LOAD_PROGRESS=1` headless/load_bench; lazy OLSN/TGA/OGG preserved. 10 load_progress + 30 load_* tests + self-check OK. Residual: async soft-FB mid-rebake (stage-boundary only); loading.tga chrome. No new deps. |
| 37 | ~~**Account page**~~ | **DONE** — soft-FB `AccountPage` + `ClientScreen::{Account,Loading,Playing,Death,Settings}`; prefill `OHOL_*`/`.env`; Enter=Connect / Esc skip-if-creds / Tab field / F2 key↔password; builds `SessionConfig` → login; headless CLI flags unchanged; tests `account_page::*`. No new crates. |
| 38 | ~~**Death / rebirth pages**~~ | **DONE** — `ClientScreen::Death` + `DeathSummary` (name/age/reason); Playing→Death on our delete PU (`note_our_death_if_any`); soft-FB `draw_death_screen`; R/Enter full reconnect LOGIN rebirth (`rebirth_session_config`); Esc quit; `ohol-client` wired; headless probes unchanged; unit tests `client_screen::*` |
| 39 | ~~**Settings page**~~ | **DONE** — soft-FB `SettingsPage` on `ClientScreen::Settings` (SFX/music volume+mute, show FPS, credential/env display; edit stays on Account); env + `ohol_client_settings.ini` prefill; apply SoundBank/MusicBank + mute atomics; F3 Account/Playing + Esc from Playing; Esc/B back best-effort save; `ohol-client` wired; headless CLI unchanged. Residual: session MusicBank continuous bed step mid-play; minifb fullscreen recreate; optional C++ chrome. No new crates. |

### Deferred / non-goals (do not schedule)

| Item | Why |
|------|-----|
| readyPending mid-move PU hold | Intentional deferral |
| Full editor suite / drawZoomView | Not required to play |
| Photo pipeline | Low priority |
| Pixel-perfect GL / wgpu | Soft FB first |
| C++ folderCache byte format | Replaced by OLC\* binaries |
| Non-zero `mMapGlobalOffset` local maps | **DONE-NA** — client stores wire coords; no float-tile path ([MAP_GLOBAL_OFFSET.md](MAP_GLOBAL_OFFSET.md)) |

---

## Summary

| Area | Done | Partial | Missing |
|------|------|---------|---------|
| Headless binary + self-check | ■■■ | | |
| LOGIN / RLOGIN / HMAC | ■■■ | | |
| MOVE wire + seq + mid-move + FM batch | ■■■ | ■ readyPending mid-move | |
| USE/DROP/REMV/SELF encode | ■■■ | | path-to-adjacent + sideAccess/noBackAccess + food self-tile + modClick + multi-MOVE + **clothing-ux DROP c 0..5** + **worn soft-FB hitMap** + **contained hit_slot** + **container take/put** **DONE** |
| Parse PU/PM/SN | ■■■ | | |
| Full ClientTag surface | ■■■ | | secondary still Known |
| LiveObject apply | ■■ | ■ | |
| Client map MC/MX | ■■ | ■ | |
| Content banks + binary cache | OLC1 v6 / OLT1 v2 / OLA1 / OLS1 / OLG1 / OLO1 / OLSN + **OLGA/OLSA opt** + **ol-binary** + cats/dummies/max-use | numeral sprites / serial cycle | — |
| Pathfind click-to-move | full play path (biomes, waypoint, multi-MOVE, hold-slide, …) | | — |
| Rendering / atlas / anim | soft FB + dual-fade + PE + speech + wall/rideable polish + **OLSA opt** | | wgpu (non-goal) |
| HUD / FX / HX | food/heat + chrome **P1#3** | names / chrome cache | — |
| Audio | OLSN + cpal pan + music OGG | wet reverb; OSS edge; music step wire | — |
| Account / loading / death / settings | loading **P5#36** + account **P5#37** + death/rebirth **P5#38** + settings **P5#39** | master gain→mixer residual | |

---

## P0 — Headless playtest core

- [x] TCP `#` frames  
- [x] SN + LOGIN/RLOGIN + HMAC  
- [x] `.env` credentials (gitignored)  
- [x] MOVE encode (seq from 2, delta clamp)  
- [x] USE/DROP/REMV/SELF/SWAP/SREMV/BABY/UBABY encode  
- [x] Action queue while moving  
- [x] Wire log + probes (`--probe-move`, `--probe-play`, …)  
- [x] `--self-check` fixture peer  
- [x] Parse **all** tags needed to stay in world without desync — **tag enum + structured parse for MC/MX/FX/HX/PS/PE/FM/PO/BW/NM/LN/DY/…** (`tags.rs`, `parse_inbound`)  
- [x] Full PU field set + delete `reason_*` lines + multi-line PU → one event each  
- [x] CM zlib inflate in `FrameReader` (inner message re-parsed)  
- [x] Secondary structured tags: MS/CX/CS/VS/FD/FL/CR/PJ/MN/GH + AP/AD/PONG session events  
- [x] PS `*map` / `*label` pointer conventions  
- [x] FX/HX applied to session `food`/`heat` fields (headless HUD state)  
- [x] Apply PU → lasting LiveObject map (**L-LIVEOBJ** — `live_object.rs` + `session.world`)  
- [x] Apply PM → mark moving + origin on LiveObject (path interpolation later)  
- [x] NM / PE / DY / HE / PO applied onto LiveObject  
- [x] Apply MC/MX → local map store (**L-MAP** — `client_map.rs`, MC zlib collect+decode, MX apply)  
- [x] Content text load (**C-OBJ/C-TRANS** lite — `content.rs`, OHOL_CONTENT_DIR / OneLifeData7)  
- [x] Client pathfind A* + `walk_to` (**pathfind.rs**)
- [x] FX/HX → `HudState` + soft-FB draw (**L-HUD** food boxes + temp arrow)  
- [x] L-HUD fidelity: mult-blend chrome, OldArrow trail, `drawHungerMaxFillLine`, erased bars/dashes, yum/curse pencil glyphs, guiBlood, logout clear peaks  
- [x] Click tile → `screen_to_tile` + `session.walk_to` (**L-ACT** / **L-HUD** wire; cumulative `click_tile` MOVE)  

- [x] PS/LS speech bubbles soft-FB (**L-SAY** — LiveObject speech + TTL/fade, locationSpeech, pencil glyphs A–Z, SceneRenderer draw; `ohol-client` T→SAY; **P3#17** map-pointer / `*label` markers + HUD arrow)  
- [x] PE emote sprites soft-FB (**L-EMOT** — body/face/head layers + **P3#18 speech→EMOT** + **P3#19** eyes offset / extraB / mouth-skip / creation-decay sounds)  
- [x] FORCE handling full parity (ack + cancel pending on FORCE; flush only on done_moving; mid-frame FORCE before rest of batch)  
- [x] Own path truncation cancels pending action (`dest_truncated` + `on_own_path_truncated`)  
- [x] Artificial FORCE when `destTruncated` + PU pos mismatch (C++ ~18031)  
- [x] Baby-held interrupt cancels our `nextAction` (C++ ~19845)  
- [ ] readyPendingReceivedMessages mid-move hold (C++ parity; deferred)  
- [x] FM frame batching (C++ `waitForFrameMessages`) — post-ACCEPTED buffer until FM; MC/PONG/FD/PH pass-through; empty FM ignored; post-FM bytes stay in pending; multi-PU expands after batch release; optional `SessionEvent::Frame` after drain  
- [ ] PHOTO_SIGNATURE typed `SessionEvent` (tag pass-through only today)  
- [x] Full logout/reset path wiring `logout_reset` + `clear_frame_batching`  
- [x] KA timing like official (`maybe_send_ka` / `KA_IDLE_SECS=15` / `last_tx`)

---

## P1 — Content & fast start

(Unchanged body — see git history for full checklist through OLO1 / OLG1 / OLSN / dual-anim / etc.)

- [x] Text load objects/transitions through OLG1/OLO1/OLSN/OLA1/OLC1/OLT1/C-CAT (see prior entries)  
- [x] `setupSpriteUseVis` / per-use `spriteSkipDrawing` on dummies (**P4#25**)  
- [x] C++ `variableDummyIDs` generation (**P4#26** — OLC1 v6 + assign/materialize; residual numeral sprites / serial)  
- [x] Category editor mutators (**P4#27**)  
- [x] Optional: bake expanded OLT1 (**P4#28** — `OLT1_F_CATEGORY_EXPANDED`; load skip re-expand + bank-only)  
- [x] OLT1 `transitions_max_use` / switch tables (**P4#29**)  
- [x] Shared client/server OLC* loader crate (**P4#30** — `ol-binary`)  
- [x] Full SaveGroundData multi-atlas dump (**P4#32** — optional OLGA pack-all/write/load + `--bake-ground-atlas` + `load_prefer_atlas_cache`; OLG1 default; bake_content unchanged)  
- [ ] Live OneLifeData7 golden CI  
- [x] Music OGG bed (P3#24) — lazy index + lewton ensure + optional device play  
- [ ] wet reverbCache residual; music step/suppress session wire  

- [x] Contained MX using-on-fill / offScreenSound (**P2#13**) + map ground-anim sounds (**P2#14**)
- [ ] C++ anim `authorTag` in OLA1  
- [ ] Golden anim vectors vs OneLifeData7  

---

## P2 — Interaction fidelity

- [x] Client pathFind (C++ pathFind.cpp) — A* in `pathfind.rs`  
- [x] Click tile → MOVE path (**L-ACT** — cumulative pathfind + `click_tile` + repath + pending cancel)  
- [x] Path cost-map fidelity: unknown blocked, `blocksWalking` only (not permanent), wide radii expand, blocked goal → closest  
- [x] Mid-move origin: `pathToDest` + `closest_path_spot` (C++ `findClosestPathSpot` + fractional `currentPos`)  
- [x] SameTile vs `xd,yd` (dest), not path start  
- [x] Hover/target object id on USE (map `object_id` fill on `click_use` / `resolve_use_object_id`; soft-FB hitMap hover still open)  
- [x] Object-target path-to-adjacent + queue USE/DROP/REMV (C++ pointerDown object branch — `click_object` / `walk_or_use_tile`)  
- [x] `sideAccess` / `noBackAccess` stand filter + permanent-food self-tile prefer (**L-ACT / side_access_food_stand**)  
- [x] Fractional `currentPos` interp for `findClosestPathSpot` (`step_current_pos` + `lrint` mid-path origin)  
- [x] Bad-biome edge routing / rideable `ignoreBad` in computePathToDest (`PathFindOpts` / `find_path_ex` / BB → `session.bad_biomes` / hold `isAutoClick`)
- [x] useWaypoint two-leg pathFind / `maxWaypointPathLength` (**DONE** P2#10 — two-leg pathFind + maxLen stop-at-wp + direct fallback; click_tile/stand/multi-MOVE + close-hold throw)
- [x] Click gates: playerActionPending, held 0-speed, age<noMoveAge, baby-held JUMP (**L-ACT / click_gates**)  
- [x] Continuous mouse-hold / blocked-tile slide remapping of `clickDest` (**L-ACT / mouse-hold slide** — `slide_blocked_click_dest` + `walk_or_use_tile_hold`; hold = ground-only repath; gates)  
- [x] Multi-MOVE continuation beyond 32-window after `done_moving` (first hop + `arm_multi_move` / `continue_multi_move` + repath when chunks empty; flush only after final hop)  
- [x] Multi-MOVE closest-fallback → ultimate goal repath (**P2#11** — `multi_move_ext` repath on done_moving; keep `multi_move_goal` when hop end short; far-goal chain + stuck-closest tests)  
- [x] MOVE `sendX`/`sendY` + `mMapGlobalOffset` (**DONE-NA** identity offset=0; [MAP_GLOBAL_OFFSET.md](MAP_GLOBAL_OFFSET.md))  
- [x] Container slot UX (**DONE** P1#6 — map REMV/DROP/USE + clothing bag put/take + soft-FB clothing contained draw)  
- [x] Clothing slots (draw from `ClothingSet` + contained; equip/remove/SREMV UX)  
- [ ] Baby hold/jump  
- [ ] Emotes + say  
- [x] KA timing like official (`maybe_send_ka` 15s idle)
- [x] Full modClick DROP/REMV/SWAP/SELF action select (`select_tile_action` / `click_tile_mod` / `click_rmb_tile`)  

---

## P3 — Presentation (optional for headless CI)

- [x] Window + swapchain (optional `gpu` feature / minifb `ohol-client`)  
- [x] Ground / object / player soft-FB + HUD residual + speech + sound triggers (see prior DONE notes)  
- [x] PE emotes drawn on players (**DONE** — layers + speech EMOT **P3#18** + **P3#19** mainEyesOffset / extraB / mouth-skip / creation-decay)  
- [ ] drawZoomView (editor)

---

## P4 — Product pages

- [x] Loading progress (**P5#36 DONE** — stages + soft-FB bar + `OHOL_LOAD_PROGRESS`)  
- [x] Account page (**P5#37 DONE** — soft-FB email/key form + env prefill + Connect; headless CLI kept)  
- [x] Death / rebirth (**P5#38 DONE** — soft-FB summary + R/Enter reconnect)
- [x] Settings (**P5#39 DONE** — soft-FB volumes + show FPS; F3; Esc/Back)

---

## Intentional non-goals (for now)

| Item | Why |
|------|-----|
| Full editor suite | Not required to play |
| Photo pipeline | Server stubs; low priority |
| C++ folderCache byte format | Replaced by OLC1 design |
| minorGems link | Pure Rust |
| Pixel-perfect GL quirks | Prefer clean GPU path |
| Wire mouse pick via hitMap | `get_sprite_hit` ready; soft FB / headless N/A |
| C-SPR async/remap/bake/SHA1/pixel-pages | Deferred; runtime TGA+atlas+OLS1 meta sufficient |
| readyPendingReceivedMessages mid-move hold | Deferred (other-player PU hold while path plays) |
| Non-zero `mMapGlobalOffset` | **DONE-NA** — wire frame == storage |

---

## First playtest focus (current workflows)

Priority for human-in-the-loop playtest (headless + graphical):

1. **Move** — ground click → MOVE (done); keep stable under repath/FORCE  
2. ~~**Interact** — object-target path-to-adjacent + queue USE/DROP/REMV~~ (**DONE** headless API + flush adjacency; probes optional)  
3. ~~**Graphical interact** — LMB on object → USE path~~ (**DONE** `walk_or_use_tile` + soft-FB hitMap hover)  
4. ~~**Drop / REMV / SWAP (modClick)**~~ (**DONE** `select_tile_action` / `click_tile_mod` / `click_rmb_tile`)  
5. ~~**sideAccess / noBackAccess + food self-tile**~~ (**DONE** `plan_stand_for_object_ex` / `stand_allows_access`)  

Workflows: see prior DONE notes; general next lane `rust-client-port-next-headless` / `next-graphics`.

---

## Next chunks (execution)

**Canonical ordered list:** [Missing work (highest priority first)](#missing-work-highest-priority-first) at top of this file.  
Start at **P3 presentation polish** (P2 complete through #14 sound residuals) unless playtest feedback says otherwise.

---

## Changelog

| Date | Note |
|------|------|
| 2026-07-27 | **P5#39 Settings page** (**DONE**): soft-FB `SettingsPage` live on `ClientScreen` graph — SFX/music volume+mute, show FPS, credential/env display (edit stays on Account); env + `ohol_client_settings.ini` prefill; apply SoundBank/MusicBank + mute atomics; F3 Account/Playing and Esc from Playing open Settings; Esc/B back with best-effort save; `ohol-client` wired; headless CLI unchanged; no new crates. Focused settings tests + client_screen death tests + self-check OK; ohol-client builds. Residual: session-held MusicBank continuous bed step not re-applied mid-play (SFX applied on enter/leave Settings); minifb fullscreen live toggle needs window recreate (display hint only); optional C++ SettingsPage chrome (borderless, half-frame, hide UI). Files: `settings_page.rs`, `account_page.rs`, `client_screen.rs`, `sound_bank.rs`, `music_bank.rs`, `bin/ohol_client.rs`, `lib.rs`, `main.rs`, port docs. |
| 2026-07-28 | **P5#38 Death / rebirth pages** (**DONE**): `client_screen.rs` — `DeathSummary` (name/age/reason), `note_our_death_if_any` Playing→`ClientScreen::Death`, soft-FB `draw_death_screen`, R/Enter → full reconnect LOGIN (`rebirth_session_config`), Esc quit; `ohol-client` death branch in live loop; headless probes unchanged. Tests `client_screen::*`. No new crates. Files: `client_screen.rs`, `account_page.rs` (app.death), `ohol_client.rs`, `lib.rs`, port docs. |
| 2026-07-28 | **P5#37 Account page** (**DONE**): `account_page.rs` — `ClientScreen` graph (Account/Loading/Playing/Death/Settings), soft-FB form (email + account key/password, pencilFont), env/`.env` prefill, Enter=Connect / Esc skip-if-creds / Tab / F2 mode, `build_session_config` → `ohol-client` login; headless CLI flags unchanged. Tests screen state + config build. No new crates. Files: `account_page.rs`, `ohol_client.rs`, `lib.rs`, `client_screen.rs` (shared graph), port docs. |
| 2026-07-28 | **P5#36 Loading progress UI** (**DONE**): C++ LoadingPage-style stages + overall 0..1 (`load_progress::{LoadStage,LoadingState,LoadProgress,report_stage,boot_load_prefer_cache}`); prefer_cache hooks on content/anim/ground/sprites/sounds/music (rebake detail); soft-FB `draw_loading_progress` (LOADING + phase + white-border bar); ohol-client Account→Loading→Playing via `run_loading_boot` + `connect_with_content`; headless `OHOL_LOAD_PROGRESS=1`; load_bench optional progress. Lazy OLSN/TGA/OGG preserved. 10 load_progress + 30 load_* tests + self-check OK. Residual: async soft-FB present during multi-second rebake (stage-boundary updates only); official loading.tga chrome not used (pencil soft-FB only). Files: `load_progress.rs`, banks, `content_binary.rs`, `content.rs`, `session.rs`, `load_bench.rs`, `bin/ohol_client.rs`, `lib.rs`, port docs. No new deps. |
| 2026-07-28 | **P1#3 HUD residual (play-visible)** (**DONE**): slip slide/wiggle (`HudState::step_slips` + wiggle amp hungry/starving) + `hunger.aiff` oneshot/pulse (`SoundBank::play_hunger_sound`); C++ `HomePosStack` — permanent `homeMarker` MX (our stake), temp PS map/label priority trump, person-home track on PU; FX `responsible_id` defer while feeder `moving` then flush on settle PU; `sync_home_hud` stack-first; `home_marker` text + `eveHomeMarker` tag. Focused hud/content/live_object tests + `ohol-headless --self-check` OK. Residual: last-ate object names; chrome binary cache; dual ancient home-slip; music suppress on starve. Files: `hud.rs`, `sound_bank.rs`, `live_object.rs`, `session.rs`, `render.rs`, `content.rs`, `content_binary.rs`, `lib.rs`, port docs. No new deps. |
| 2026-07-27 | **P4#34 Golden anim vectors vs OneLifeData7** (**DONE**): always-on synthetic golden sample/frame_time/fade (C++ processFrameTime/getOscOffset/hardness); live load person `19_2` moving + `19_0` ground when OneLifeData7 present — assert sprite count 92, layer 0/1 amps/phases, y-rock golden, OLA1 roundtrip sample equality at probe times. Tests `golden_anim_sample_vectors_*`. Files: `anim_bank.rs`, port docs. No new deps. |
| 2026-07-27 | **P4#33 Anim `authorTag` in OLA1** (**DONE**): parse C++ trailing `author=` → `ObjectAnimation.author_tag`; OLA1 format **2** write (u16+utf8 after slots); load v1+v2; cache validation accepts both. Tests `ola1_roundtrip` / `ola1_v1_legacy_load_without_author` / `parse_author_tag_from_text`. Files: `anim_bank.rs`, `lib.rs`, `CONTENT_BINARY.md` |
| 2026-07-27 | **P4#32 Full SaveGroundData multi-atlas dump** (**DONE**): Optional **OLGA** multi-page full ground atlas (`olga_ground_atlas.bin`) — pack-all + write/load rects+RGBA pages; CLI `ohol-headless --bake-ground-atlas`; `GroundBank::load_prefer_atlas_cache` / pack-all + write/load. Default play path remains OLG1 index + lazy TGA; `bake_content` unchanged (omits OLGA by design — size). Focused `ground_sprites`/`binpack` tests + `ohol-headless --self-check` OK. Residual: Haxe unknown-biome white-pixel recolor tint variants not packed; C++ per-biome `GroundSpriteSet` sheet layout not mirrored (Rust multi-page BinPack); OLGA raw RGBA single-file ≠ Haxe `ground.png`+`SaveGroundData.bin` serializer. Files: `ground_sprites.rs`, `binpack.rs`, `lib.rs`, `main.rs`, port docs. No new deps. |
| 2026-07-27 | **P4#30 Shared client/server OLC\* crate** (**DONE**): Extracted zero-dep `ol-binary` crate (`openlife/RustServer/crates/ol-binary`) with OLC1 v1–6 / OLT1 v1–2 DTO parse+encode (`parse_olc1`/`encode_olc1`/`parse_olt1`/`encode_olt1`, flags, header helpers). Client `content_binary` maps DTOs ↔ `ClientContent` (bake/prefer_cache/dummies unchanged). Server `ol-content::binary_cache` maps DTOs ↔ `ContentDb`; OLT1 **bit6→`transitions_max_use`** / **bit7→`switch_number_of_uses`** (closes P4#29 residual); OLC1 format max **v6** (sprites/sounds consumed, discarded). Focused tests: ol-binary 3, ol-content binary 6, client content_binary 13 + `ohol-headless --self-check` OK. Residual: OLA*/OLG*/OLO*/OLSN/OLS1 still client-local; server `finish_cache_boot` always re-expands categories. Files: `ol-binary/*`, `ol-content/binary_cache.rs`, `ol-content/Cargo.toml`, RustServer workspace Cargo.toml, client `Cargo.toml`, `content_binary.rs`, port docs. New path dep: `ol-binary` (no crates.io). |
| 2026-07-27 | **P4#31 OLS1 written by `bake_content`** (**DONE**): `bake_content` writes `cache/ols1_sprites.bin` + manifest entry (sha1/bytes/count); `bake_ols1_from_dir` / `scan_sprites_dir` (txt + TGA header w/h only; no pixel pages/hitMaps); `SpriteBank::load_prefer_cache` loads OLS1, rebakes on missing/stale `data_version`; `load_from_cache` optional OLS1 sha1 verify. Tests `bake_ols1_from_fixture`, `load_prefer_cache_writes_ols1_when_missing`, bake_and_load asserts. Residual: pre-baked atlas pages; bake-time alpha-bbox (needs full TGA). Files: `sprite_bank.rs`, `content_binary.rs`, `lib.rs`, `main.rs`, `load_bench.rs`, port docs. No new deps. |
| 2026-07-27 | **P4#29 OLT1 `transitions_max_use` / switch tables** (**DONE**): C-TRANS Haxe maxUse pair split (`targetRemains` true → primary; false → `transitions_max_use`) + ServerSettings `switch_number_of_uses` dough/masa keys `(252,3371)/(235,4086)/(1300,3371)/(235,4090)`; `insert_normal_or_max_use` (text + category expand); OLT1 record bit6 max-use / bit7 switch baked and loaded; patches after text and cache load; `find_transition_max_use` / `find_ptrans_max_use`. Focused tests + `ohol-headless --self-check` OK. Residual: Haxe reverseUseActor/Target last-use auto-clone not implemented; server ol_content OLT1 load still ignores bit6/bit7 (shared crate P4#30); optional live golden vs server max_use row count. Files: `content.rs`, `content_binary.rs`, `category_bank.rs`, `lib.rs`, port docs. No new deps. |
| 2026-07-28 | **P4#40 Sprite atlas pixel pages (OLSA)** (**DONE**): optional multi-page dump `cache/olsa_sprite_atlas.bin` (magic OLSA v1: packed rects + raw RGBA pages); `SpriteBank::write_olsa` / `load_olsa` / `load_olsa_timed` / `load_prefer_atlas_cache` / `pack_all_from_meta` (sorted ids); `bake_olsa_from_dir` / `bake_olsa_to_dir`; CLI `--bake-sprite-atlas` prints pack/write/total ms; load_bench step `sprite_atlas_load` (skipped note if absent); restored pages `BinPack::sealed`; invalidate when tree `dataVersionNumber` ≠ blob. Tests: `olsa_roundtrip_from_rgba`, `olsa_bad_magic`, `olsa_version_invalidation`, `olsa_load_prefer_atlas_cache`, `olsa_pack_all_from_meta_sorted`. Default play = OLS1 + lazy TGA. Residual: full-tree size; optional SHA1 beyond dataVersion. Files: `sprite_bank.rs`, `main.rs`, `load_bench.rs`, `content_binary.rs`, `lib.rs`, port docs. No new deps. |
| 2026-07-27 | **P4#28 Bake expanded OLT1 (skip re-expand on load)** (**DONE**): OLT1 header flag `OLT1_F_CATEGORY_EXPANDED` (bit0); `bake_content`/`load_from_dir` mark expanded after lite+pattern; `write_olt1` sets flag; `load_from_cache` skips `expand_category_transitions` when set (still `maybe_load_category_bank_from_root` for `pick_from_prob_set`/`find_ptrans`); legacy/unflagged OLT1 re-expands (correct); `load_prefer_cache` rebuilds once if flag absent. Tests `olt1_bake_expanded_skips_reexpand_on_load`. Residual: none for this slice. Files: `content.rs`, `content_binary.rs`, `lib.rs`, port docs. No new deps. |
| 2026-07-27 | **P4#27 Category editor mutators** (**DONE**): in-memory C++ `categoryBank` mutators on `CategoryBank` — add/delete category, add/remove/move members, pattern/probSet flags, `set_member_weight`/`make_weight_uniform`/`auto_adjust_weights`, reverse list move; `format_category_txt` serialize; tests `category_editor_*`. Play load path unchanged (no disk). Residual: optional `save_to_dir` FS write. Files: `category_bank.rs`, `lib.rs` |
| 2026-07-27 | **P4#25 `setupSpriteUseVis` / per-use `spriteSkipDrawing` on dummies** (**DONE**): C++ `setupSpriteUseVis` ported; multi-use dummies get progressive `spriteSkipDrawing`; OLC1 v5+ use vanish/appear trailer (write path v6); draw/hit skip `skip_drawing`. Concurrent P4#26 variableDummyIDs (OLC1 v6) present. Residual: golden setupSpriteUseVis vectors vs OneLifeData7; `setupNumericSprites` for +varNumeral (P4#26 residual). Files: `content.rs`, `content_binary.rs`, `render.rs`, `hover_pick.rs`, `lib.rs`, port docs. Tests OK. No new deps. |
| 2026-07-27 | **P3#24 Music OGG / `music_NN.ogg` bed** (**DONE**): `music_bank.rs` — lazy scan of `music/music_NN.ogg` (boot `ogg_opens==0`); `next_music_block` age select (C++ musicPlayer2); **lewton** pure-Rust Vorbis decode only on ensure/play; mono mixdown → cpal via existing mixer under `--features audio`; tests synthetic + skip-if-no-file real tree. Residual: stepMusicPlayer async age load, suppression stack, session wire. Files: `music_bank.rs`, `lib.rs`, `Cargo.toml` (+lewton), `load_bench.rs`, port docs. |
| 2026-07-27 | **P3#22 action wiggle / baby-held handoff** (**DONE**): action-wiggle cosine bounce on person/held draw (`pendingActionAnimationProgress` + target dir); local flush + remote actionAttempt start; BW+JUMP baby arm-wiggle while held; soft-FB skips held babies and draws them at adult HoldingPos; `heldByDropOffset` put-down slide + held→ground pack handoff; `heldPosOverride` pick-up slide. Helpers in `anim_draw.rs`; fields/step in `live_object.rs`; wire in `session.rs`/`render.rs`/`lib.rs`. Tests `p3_22_*` + anim_draw. 435 lib tests + `ohol-headless --self-check` OK. No new deps. Residual: young-baby lie-down rot while drop settles (C++ ~5489–5507); adult held-track clock copy on drop (C++ ~19296–19301); heldPosOverride step should use scene frf not hard-coded 1.0. Files: `anim_draw.rs`, `live_object.rs`, `session.rs`, `render.rs`, `lib.rs`, port docs. |
| 2026-07-27 | **P3#23 wallLayer / frontWall sub-order** (**DONE**): C++ `setupWall` → `floor_hugging`/`wall_layer`/`front_wall` on `ClientObjectDef` (`+wall`/`-wall`/`+frontWall` + floorHugging); OLC1 bit11 floorHugging; front DrawLayer after players: permanent non-wall → non-permanent → wall → frontWall. Tests `setup_wall_layer_and_front_wall_tags`, `front_object_draw_layer_ordering`, `wall_layer_front_wall_sub_order`. Residual: flying-held / moving-map interleave. Files: `content.rs`, `content_binary.rs`, `render.rs`, port docs. No new deps. |
| 2026-07-27 | **P3#19 PE eyes offset / extraB / emot creation sounds** (**DONE**): `ANIM_EXTRA_B` PE toggle with dual-fade A/B indices; mouth sprite skip when `mouthEmot`; creation/decay `SoundUsage` on PE apply/clear (lazy OLSN); eyes offset via `setup_eyes_and_mouth` + Face eyeEmot. 420 lib tests + `ohol-headless --self-check` OK. No new deps. Residual: OSS edge-arrow soft-FB (separate L-SOUND); dual PE-TTL tick if both `session.step_anims` + `SceneRenderer.draw`. Files: `anim_bank.rs`, `anim_draw.rs`, `live_object.rs`, `render.rs`, `sound_bank.rs`, `session.rs`, `emotion.rs`, `lib.rs`, port docs. |
| 2026-07-27 | **P3#21 getObjectCenterOffset / hideClosestArm** (**DONE**): C++-faithful center offset (widest non-multiplicative, skip only-when-worn, +2π rot, containOffsetX/Y from tags); held subtract for non-person; SceneRenderer sprite-bank bbox; arm_holding 0/−2/rideable + draw hide ±1 + body HoldingPos. Tests: `object_center_offset_*`, `arm_holding_parameters_hide_closest`, `hide_closest_arm_pm1_and_body_holding_pos`. Residual: OLC1 only_when_worn bit. Files: `content.rs`, `content_binary.rs`, `render.rs`, `lib.rs` |
| 2026-07-27 | **P3#20 Rideable person-under-vehicle draw order** (**DONE**): HoldingPos residual — C++ LivingLifePage ~5443–5916: when held is rideable, vehicle draws at person pos (not hand HoldingPos); behind `spritesDrawnBehind` under rider, front over; ridingOffset ≈ −heldOffset; speech/clothing follow rider. Tests `rideable_person_under_vehicle_draw_order` + `rideable_vehicle_at_person_pos_not_hand`. Residual: age body offset, hideRider, vehicle contained. Files: `render.rs`, `TODO_PORT.md`. No new deps. |
| 2026-07-27 | **P3#19 PE eyes / extraB / mouth-skip / emot sounds** (**DONE**): (1) eyes offset — `setup_eyes_and_mouth` + Face eyeEmot; (2) `ANIM_EXTRA_B` + PE toggle EXTRA↔EXTRA_B (`setExtraIndex`/`setExtraIndexB`, dual-fade sample); (3) skip person mouth when any active PE `mouthEmot>0`; (4) creation on PE apply + decay on temp TTL clear via `play_emot_*` SoundUsage (lazy OLSN). Tests: eyes offset, `extra_b_pack_*`, `pe_permanent_and_ttl_*`, `scene_draw_skips_mouth_*`, `pe_emot_creation_and_decay_*`. Files: `content.rs`, `anim_bank.rs`, `anim_draw.rs`, `live_object.rs`, `render.rs`, `sound_bank.rs`, `session.rs`, `emotion.rs`, `TODO_PORT.md`, `FILE_MATRIX.md`. No new deps. |
| 2026-07-27 | **P3#17 PS map-pointer / `*label` UI** (**DONE**): `LiveWorld.says_pointers` stores PS `*map`/`*label` with TTL (`map_age_seconds` or speech hold; expert +120s); soft-FB map-spot pins + label markers at target/map; HUD home-arrow + `map_pointer_label`; bubble text stripped spoken only; 410 lib tests + self-check OK; no new deps. Residual: C++ meters-away/years-ago speech rewrite on map PS; permanent home stack / map-drop `temporaryExpireETA`; temp-home priority trump (`doesNewTempLocationTrumpPrevious`); overhead person labels tied to temp-home ETA when speech idle; `*photo` metadata display. Files: `live_object.rs`, `parse.rs`, `render.rs`, `hud.rs`, `lib.rs`, `TODO_PORT.md`, `FILE_MATRIX.md`, `CALL_INDEX.md` |
| 2026-07-27 | **P3#18 Speech → emote `getEmotionIndex`** (**DONE**): `EmotionBank::get_emotion_index` C++ exact-match; `classify_speech_outbound` + `encode_emot`; `session.send_say` → EMOT/SAY/local; tests `emotion::*` + `tests/speech_emot_p3_18.rs`. Files: `emotion.rs`, `actions.rs`, `session.rs`, `lib.rs` |
| 2026-07-27 | **P3#16 residual curse polish** (**DONE**): soft-FB purple speech (`speech_text_rgb` C++ 0.875/0.5 purple + white cursed/dying); `draw_speech_bubble_colored`; SceneRenderer wires ink; `SoundBank` lazy path PCM (`ensure_path` / `play_curse_sound_at` → `otherSounds/curseChime.aiff` vol 0.5 spatial) on successful PS isCurse; boot still aiff_opens==0. Files: `live_object.rs`, `hud.rs`, `render.rs`, `sound_bank.rs`, `session.rs`, `lib.rs`, `TODO_PORT.md`. Residual: `+FAMILY+` `isGeneticFamily` flag |
| 2026-07-27 | **P3#16 speech curse-tag reinsert / 15s tic** (**DONE**): L-SAY CU parse/apply, curse-tag reinsert after non-tag bubble clear, 15s nervous tic, babble wrap when gap>15s; files `live_object.rs`/`parse.rs`/`session.rs`/`lib.rs`; unit tests + self-check OK; no new deps. Purple/`mCurseSound` residual closed in curse-polish entry; remaining residual: `+FAMILY+` `isGeneticFamily` flag (display skip already present) |
| 2026-07-27 | **P2#13 Contained MX using-on-fill / offScreenSound** (**DONE**): L-SOUND-TRIG v6 — MX fill using + contained-slot creation/using-on-fill (multi-use less-used); clothing bag fill PU; spatial `play_usage_at`; OSS register (anim + held creation, skip self); 39 sound_bank tests + `ohol-headless --self-check` OK; no new deps. Residual: soft-FB OSS edge-arrow draw (wet reverb / music OGG / contained-swap suppress remain separate). Files: `sound_bank.rs`, `session.rs`, `lib.rs` |
| 2026-07-27 | **P2#13 Contained MX using-on-fill / offScreenSound** + **P2#14 map ground-anim sounds** (**DONE**): `play_mx_change_sounds` (container fill using, contained-slot creation/using-on-fill, spatial MX); clothing bag fill PU; `OffScreenSoundEvent` / `last_off_screen` + anim/held-creation register (skip self); `step_map_ground_anims_with_sounds` + per-tile frame maps; headless `last_played` tests; OLSN still lazy; residual OSS edge draw / moving-map anim type |
| 2026-07-27 | **P2#10 useWaypoint two-leg pathFind** (**DONE**): C++-faithful two-leg pathFind — shared blocked map, both legs must reach; `pathLength>maxWaypointPathLength` rewrites dest to waypoint; fail → direct. Wired: ground `plan_click_tile_chunks_goal`/`click_tile_with`, stand `plan_stand_for_object_with_opts_wp`/`click_object`, multi-MOVE `eff_goal`, close-hold AABB throw (1..4 tiles, throw len 4). Unit + click_tile tests (length threshold + two-leg routes). Residual: road auto-walk + waypoint edge polish (minor). Files: `pathfind.rs`, `move_state.rs`, `click_tile.rs` |
| 2026-07-27 | **P2#11 multi-MOVE ultimate goal repath** (**DONE**): after hop short of click/stand (pathFindingD=32 window clamp or closest-reachable fallback), `done_moving` → `continue_multi_move` repaths toward `multi_move_goal` and arms next MOVE before flush; sync world dest for repath origin; keep goal when `!plan.reached_goal`; tests `multi_move_far_goal_*` + closest-stuck |
| 2026-07-27 | **P2#12 mMapGlobalOffset sendX/sendY** (**DONE-NA**): C++ GPU local-map offset audit; Rust `session.map` / `move_state` / `encode_move` use **wire frame** end-to-end; identity API `MapGlobalOffset::ZERO` + `encode_move_with_offset` in `map_global_offset.rs`; doc [MAP_GLOBAL_OFFSET.md](MAP_GLOBAL_OFFSET.md); not server birth-relative (separate frame) |
| 2026-07-27 | **P2#9 bad-biome edge routing / rideable `ignoreBad`** (**DONE**): C++ `isBadBiome` + `computePathToDest` ~2481–2504 — floor-less BB biomes blocked on long paths; edge-of-bad + bad dest allows entry; standing-in-bad same-biome walk / block other bad; rideable `ignore_bad`; floor cancels bad; `parse_bad_biomes` + session `bad_biomes` from BB; `find_path_ex` / `PathFindOpts`; click_tile + stand plan use `path_find_opts`; **hold/auto_walk `isAutoClick`** via `path_find_opts_with` / `click_tile_with` / `plan_click_tile_chunks_with`; synthetic + session integration tests; residual: bad-biome HUD display names (`mBadBiomeNames`) |
| 2026-07-27 | **L-MOVE / fractional currentPos** (**DONE** P1#8): track `current_pos_x/y` while in motion (`step_current_pos` along `path_to_dest`, √2 diag); `closest_path_spot` = C++ `findClosestPathSpot` (`lrint` then path snap); `path_start_tile` + hold-slide origin; PM `on_own_pm_timing` from `total_sec`; session `step_anims`/`step_move_pos`; FORCE/done_moving snap; tests mid-path repath origin + MOVE wire; residual: turn-smoothing/circling fix, soft-FB camera draw from fractional pos |
| 2026-07-27 | Prior P1#1–#7 / clothing / container / stereo pan / HUD residual — see git history for full notes |
| 2026-07-26 | Prior L-ACT / C-CAT / L-SOUND-TRIG / limb_hide — see git history |
