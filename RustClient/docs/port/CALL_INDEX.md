# Call index — client AI lookup

**Hub:** [README.md](README.md) · **status:** [TODO_PORT.md](TODO_PORT.md) · **file map:** [FILE_MATRIX.md](FILE_MATRIX.md)

Symbol table only — not a second architecture essay.

---

## C++ entry & pages

| Symbol | File | Role |
|--------|------|------|
| `main` / game loop | `game.cpp` | banks, pages, step/draw |
| `LoadingPage` | `LoadingPage.*` | progressive load → Rust `load_progress` / `boot_load_prefer_cache` (P5#36) |
| `ExistingAccountPage` | `ExistingAccountPage.*` | login UI → Rust `account_page` |
| `SettingsPage` | `SettingsPage.*` | options UI → Rust `settings_page` (P5#39) |
| `LivingLifePage` | `LivingLifePage.*` | in-game |
| `LiveObject` | `LivingLifePage.h` | player entity |

---

## C++ banks

| Symbol | File |
|--------|------|
| `initObjectBank*` / `getObject` | `objectBank.*` |
| `initSpriteBank*` / `getSprite` / `SpriteRecord` | `spriteBank.*` |
| `initAnimationBank*` / `getAnimation` / `SpriteAnimationRecord` | `animationBank.*` |
| `scanAnimationRecordFromString` | `animationBank.cpp` |
| `initTransitionBank*` / `autoGenerateCategoryTransitions` | `transitionBank.*` |
| `initCategoryBank*` / `getCategory` / `getReverseCategory` / `getNumCategoriesForObject` / `getCategoryForObject` | `categoryBank.*` |
| `initFolderCache` / `getFileContents` | `folderCache.*` |
| `initBinFolderCache` | `binFolderCache.*` |

---

## C++ gameplay helpers

| Symbol | File |
|--------|------|
| pathfinding | `pathFind.*` |
| emotions | C++ `emotion.*` · Rust `emotion.rs` (`EmotionBank`, `get_emotion_index`, `classify_speech_outbound`, PE apply, object layers) · `encode_emot` / `session.send_say` |
| ground | `groundSprites.*` / OLG1+OLGA `ground_sprites` (`ensure_tile`, `ensure_overlay`, `load_prefer_cache`, optional `load_prefer_atlas_cache` / `bake_olga_*` **P4#32**) |
| overlays | C++ `overlayBank.*` · Rust `overlay_bank` (`get_overlay`, `ensure_image`, `load_prefer_cache`, OLO1) |
| music | `musicPlayer*.*` |

---

## Protocol

| Doc | Path |
|-----|------|
| Wire bible | `OneLife/server/protocol.txt` |
| Server tags (inbound to server) | also RustServer `ServerTag` / Haxe |
| Client tags (inbound to client) | Haxe `ClientTag.hx` · Rust `tags::ServerTag` |

---

## Haxe

| Symbol | File |
|--------|------|
| `Client` connect/update | `openlife/client/Client.hx` |
| `ClientTag` | `ClientTag.hx` |
| `Render` / `addObject` / `loadSprites` | `Render.hx` |
| `BinPack.pack` | `BinPack.hx` |
| `SpriteBatch` | `SpriteBatch.hx` |
| `ObjectBake` | `resources/ObjectBake.hx` |
| `Resource.spriteImage` / `spriteData` / `animation` | `resources/Resource.hx` |
| `AnimationData` / `AnimationRecord` / `AnimationParameter` | `data/animation/*.hx` |
| `ObjectData.getSpriteData` | `data/object/ObjectData.hx` |
| `SpriteData.inCenterXOffset` | `data/object/SpriteData.hx` |
| `Engine.message` | `engine/Engine.hx` |

---

## Rust (current)

| Symbol | File |
|--------|------|
| `connect_and_login` | `session.rs` |
| `ClientSession.world` / `LiveWorld` / `LiveObject` | `session.rs` + `live_object.rs` |
| `LiveWorld::apply_pu` / `apply_moves_start` / names/emots/dying | `live_object.rs` |
| `encode_login` / `hmac_sha1_hex` | `login.rs` |
| `encode_move` / `MoveState` / FORCE snap+ack gate | `move_state.rs` |
| `MoveState::send_move_repath` / cumulative `PathDelta` | `move_state.rs` |
| `MoveState::on_own_path_truncated` / `dest_truncated` / artificial FORCE | `move_state.rs` |
| `encode_use` / `encode_drop` / `encode_force` / KA | `actions.rs` |
| `FrameReader` / `encode_raw` / `inflate_cm` / `compress_cm_payload` | `frame.rs` |
| `ServerTag` / `ALL_SERVER_TAGS` | `tags.rs` |
| `parse_inbound` / `InboundMessage` | `parse.rs` |
| `parse_mx_line` / `parse_fx_message` / `parse_hx_message` | `parse.rs` |
| `parse_ps_line` / `parse_pe_message` / `parse_mc_header` | `parse.rs` |
| `parse_sn` / `parse_pu_line` / `parse_pu_message` / `parse_pm_*` | `parse.rs` |
| `PlayerUpdate` (full field set + `deleted`/`delete_reason`) | `parse.rs` |
| `PlayerSays` (`map` / `target_label` pointers) | `parse.rs` |
| `SaysTargetLabel::{short_name, marker_rgba}` | `parse.rs` (P3#17) |
| `parse_ms_message` / CU/CX/CS/VS/FD/FL/CR/PJ/MN/GH | `parse.rs` |
| `parse_cu_message` / `CursedPlayer` | `parse.rs` (P3#16) |
| `LiveObject::{show_curse_tag, apply_cursed, tick_speech}` / `format_curse_tag` / `MAX_CURSE_TAG_DISPLAY_GAP` | `live_object.rs` (P3#16 reinsert + 15s tic) |
| `SaysPointerMarker` / `says_pointer_ttl_sec` / `home_dir_index` / `LiveWorld::{apply_says_pointer, says_pointers}` | `live_object.rs` (P3#17 map/label markers) |
| `draw_map_spot_marker` / `SceneRenderer::sync_map_pointer_hud` | `render.rs` (P3#17 soft-FB) |
| `HudState::map_pointer_label` | `hud.rs` (P3#17 home-slip label) |
| `LiveWorld::apply_cursed` | `live_object.rs`
| `SessionEvent::Cursed` | `session.rs` |
| `SessionEvent` (PU nested `pu`, multi-event queue, MS/AP/PONG/…) | `session.rs` |
| `ClientSession::poll_event` (C++ `getNextServerMessage` wait-for-FM) | `session.rs` |
| `ClientSession::poll_frame_events` / `wait_for_frame_messages` / `clear_frame_batching` | `session.rs` |
| `ClientSession::logout_reset` (C++ page reset / death clear) | `session.rs` |
| `ClientSession::flush_pending_action` (done_moving flush; FORCE cancels) | `session.rs` |
| `ClientSession::cancel_pending_action` | `session.rs` |
| `ClientSession::maybe_send_ka` / `needs_ka` / `KA_IDLE_SECS` (15s) | `session.rs` |
| `ClientSession::pending_action` | `session.rs` |
| `ClientSession::walk_to` | `session.rs` (legacy; prefer `click_tile`) |
| `SessionEvent::Frame` (post-batch probe boundary; C++ discards FM) | `session.rs` |
| `ClientSession::{food, heat, curse_tokens, world}` | `session.rs` |
| `find_path` / `find_path_ex` / `PathFindResult` / `PathFindOpts` / `PATH_FINDING_D` / `DEFAULT_MAX_EXPAND` / `find_path_deltas` / `find_path_deltas_ex` | `pathfind.rs` |
| `is_bad_biome_tile` / `is_bad_biome_at` / `parse_bad_biomes` / `parse_bad_biome_ids` | `pathfind.rs` (P2#9) |
| `cell_blocks_walking` / `cell_walkable` / `fill_open_rect` | `pathfind.rs` |
| `build_blocked_window` / `neighbor_order` (long-axis first) / fixed 32² A* scratch | `pathfind.rs` (private helpers) |
| `chunk_deltas_for_move` / `steps_to_cumulative` / `cumulative_to_steps` | `pathfind.rs` |
| `click_tile` / `click_tile_with` / `plan_click_tile` / `plan_click_tile_chunks_with` / `path_start_tile` / `ClickTileExt` / `ClickTileResult` | `click_tile.rs` |
| `ClientSession::{bad_biomes, apply_bad_biomes_message, holding_rideable, path_find_opts, path_find_opts_with}` | `session.rs` (BB + rideable ignoreBad + isAutoClick) |
| `apply_click_gates` / `NO_MOVE_AGE` (0.20) | `click_tile.rs` |
| `is_grid_adjacent` / `plan_stand_for_object` / `plan_stand_for_object_ex` / `stand_allows_access` / `StandAccess` / `click_object` / `click_use` / `click_drop` / `click_remv` / `walk_or_use_tile` / `ObjectClickResult` / `WalkOrUseResult` / `can_execute_action_at` / `resolve_use_object_id` | `click_tile.rs` |
| C++ sideAccess / noBackAccess / food self-tile stand | `LivingLifePage.cpp` ~25810–26038 → `plan_stand_for_object_ex` |
| `ClientObjectDef::{side_access, no_back_access}` / `ClientContent::{side_access, no_back_access}` | `content.rs` (+ OLC1 `OBJ_F_SIDE_ACCESS` / `OBJ_F_NO_BACK_ACCESS`) |
| `HoverPick` / `pick_at_screen` / `update_scene_hover` / `draw_hover_outline` | `hover_pick.rs` |
| `SpriteBank::get_sprite_hit` (soft-FB hover confirm) | `sprite_bank.rs` |
| `RmbClickResult` / `click_rmb_tile` / `tile_allows_remv` / `our_held_id` / `RmbClickExt` | `rmb_action.rs` |
| GUI LMB `walk_or_use_tile` + RMB/Q `click_rmb_tile` + title status | `bin/ohol_client.rs` |
| `encode_jump` / `ClientSession::send_jump` | `actions.rs` / `session.rs` |
| `ClientSession::{player_action_pending, our_age, we_are_held_by_adult, our_held_id}` | `session.rs` |
| `ClientSession::{arm_multi_move, clear_multi_move, continue_multi_move, has_multi_move}` | `session.rs` |
| `continue_multi_move_body` (chunks then repath; before flush) | `multi_move_ext.rs` → `session.continue_multi_move` |
| `select_tile_action` / `select_self_action` / `select_self_action_ex` / `click_tile_mod` / `click_swap` / `click_self` | `click_tile.rs` |
| `clothing_char_to_slot` / `CLOTHING_SLOT_NAMES` / `ClothingSet::resolve_shoe_slot` | `live_object.rs` |
| `clothing_slot_for_object` / `resolve_clothing_equip_slot` / `click_drop_clothing` / `click_sremv_clothing` / `click_remove_clothing` / `is_self_tile` | `click_tile.rs` |
| `ClientSession::{send_sremv, click_drop_clothing, click_self}` | `session.rs` |
| GUI clothing keys 1–6 + E self-auto + Shift SREMV | `bin/ohol_client.rs` |
| FORCE/trunc/baby/death clear multi-MOVE | `session.rs` dispatch |
| `LiveObject::held_by_adult_id` / `is_held_by_adult` (from adult `held_id < 0`) | `live_object.rs` |
| `ClientSession::{queue_pending_action, path_and_act, path_and_use, walk_or_use_tile}` | `session.rs` |
| `flush_pending_action` adjacency gate (C++ isGridAdjacent / same tile) | `session.rs` |
| `MoveError::SameTile` (click vs xd/yd dest, not path start) | `move_state.rs` / `click_tile.rs` |
| `MoveError::NoAdjacentStand` / `ActionPending` / `HoldingImmobile` / `JumpSent` | `move_state.rs` / `click_tile.rs` |
| `MoveState::{path_to_dest, current_pos_*, step_current_pos, closest_path_spot}` (C++ `pathToDest` / `currentPos` / `findClosestPathSpot`) | `move_state.rs` |
| `ClientContent::blocks_walking` (C++ `blocksWalking` only — not permanent) | `content.rs` |
| `ClientObjectDef::{left_blocking_radius, right_blocking_radius}` / `is_wide` | `content.rs` |
| C++ `pointerDown` ground → MOVE | `LivingLifePage.cpp` ~25783, ~26506–26538 |
| C++ `pointerDown` object → path-to-adjacent + `nextActionMessageToSend` | `LivingLifePage.cpp` ~25803–26350 → `click_object` |
| C++ `getSpriteHit` / hitMap expandMap×3 | `spriteBank.cpp` → `get_sprite_hit` / `hover_pick` |
| C++ `isGridAdjacent` (4-neighbor) | `LivingLifePage.cpp` ~2063 → `is_grid_adjacent` |
| C++ flush nextAction when adjacent + !inMotion | `LivingLifePage.cpp` ~23193–23280 → `flush_pending_action` |
| C++ `findClosestPathSpot` | `LivingLifePage.cpp` ~2341 → `closest_path_spot` / `path_start_tile` |
| C++ `computePathToDest` / `pathFind` | `LivingLifePage.cpp` ~2392 / `pathFind.cpp` |
| C++ blockedMap unknown / !blocksWalking / wide radii | `LivingLifePage.cpp` ~2451–2538 → `build_blocked_window` |
| Protocol MOVE cumulative deltas | `server/protocol.txt` MOVE |
| C++ pointerDown gates (playerActionPending / 0-speed / JUMP) | `LivingLifePage.cpp` ~24934–24995 → `apply_click_gates` |
| `pick_worn_clothing_slot` / `WornClothingPickTarget` / `pick_at_screen_with_clothing` | `hover_pick.rs` (worn clothing soft-FB hitMap) |
| `pick_contained_slot_at` / `HoverPick::contained_slot` | `hover_pick.rs` (map + clothing container slot hitMap → REMV/SREMV `i`) |
| `select_tile_action` container take/put / `select_self_action_ex` bag put/take | `click_tile.rs` (P1#6 REMV/DROP/USE + clothing DROP c / LMB SREMV on contained) |
| soft-FB clothing contained draw (`slot_pos` + `parse_object_raw_contained`) | `render.rs` player ClothingSet pass (open UX parity with hit-test) |
| `click_tile_mod_ex` / `walk_or_use_tile_ex` / `click_rmb_tile_ex` | `click_tile.rs` / `rmb_action.rs` (`clothing_slot` + `hit_slot` from hover) |
| `slide_blocked_click_dest` / `resolve_hold_click_dest` / `walk_or_use_tile_hold` | `click_tile.rs` (P1#7 continuous LMB hold repath + blocked-tile slide) |
| C++ `isBadBiome` / bad biome blockedMap / rideable `ignoreBad` | `LivingLifePage.cpp` ~2320–2504 → `pathfind::{is_bad_biome_*, PathFindOpts, find_path_ex}` + session BB |
| `find_path_via_waypoint` / `find_path_with_waypoint_ex` / `path_cell_count` | `pathfind.rs` (P2#10 C++ two-leg pathFind + maxWaypointPathLength) |
| `MoveState::{use_waypoint,arm_waypoint,clear_waypoint}` / close-hold `maybe_close_hold_throw` | `move_state.rs` / `click_tile.rs` |
| `plan_click_tile_chunks_goal` / `plan_stand_for_object_with_opts_wp` | `click_tile.rs` (waypoint-aware ground + stand) |
| open (L-ACT residual) | SHIFT/CTRL clothing polish (multi-MOVE ultimate repath **P2#11 DONE**) |
| C++ `waitForFrameMessages` / `serverFrameMessages` / `serverFrameReady` | `LivingLifePage.cpp` → `session.rs` |
| C++ pass-through while waiting FM: MC / PONG / FD / PH | `LivingLifePage.cpp` → `session.rs` (`framed_is_frame_passthrough`) |
| C++ FORCE ack + cancel `nextActionMessageToSend` | `LivingLifePage.cpp` ~19359 → `session` dispatch |
| C++ artificial force (`destTruncated` + pos mismatch) | `LivingLifePage.cpp` ~18031 → `MoveState::on_player_update` |
| C++ own truncated PM cancels `nextAction` | `LivingLifePage.cpp` ~20503 → PM dispatch + `on_own_path_truncated` |
| C++ baby-held interrupt clears `nextAction` | `LivingLifePage.cpp` ~19845 → PU `held_id < 0` |
| C++ `timeLastMessageSent` / idle `KA 0 0#` | `LivingLifePage.cpp` ~14761 → `maybe_send_ka` |
| `note_names` / `note_map_changes` | `event_util.rs` |
| `WireLog` | `wire_log.rs` |
| `ClientContent` / `parse_object_txt` / `load_prefer_cache` | `content.rs` |
| `ClientObjectDef::{dummy_ids, dummy_parent, use_chance, map_chance, biomes, heat_value, speed_mult, r_value, decay_*}` / `ClientContent::dummy_parent` | `content.rs` |
| `ClientContent::{transitions_last_use, find_transition, find_transition_last_use, find_transition_prefer}` | `content.rs` |
| `ClientContent::{categories, load_categories_and_expand, apply_category_bank, maybe_load_categories_from_root}` | `content.rs` |
| `ClientContent::{materialize_transition, find_ptrans, find_ptrans_last_use}` | `content.rs` ← C++ `getPTrans` probSet subset |
| `ClientTransition` reverse/no_use/move/min_use fields | `content.rs` |
| `CategoryBank` / `CategoryRecord` / `ReverseCategoryRecord` | `category_bank.rs` ← C++ `categoryBank` (**C-CAT DONE→stronger**) |
| `parse_category_txt` / `CategoryBank::load_from_dir` / `insert_record` | `category_bank.rs` |
| C++ `getCategory` / `getReverseCategory` | `CategoryBank::{get_category, get_reverse_category}` |
| C++ `getNumCategoriesForObject` / `getCategoryForObject` | `CategoryBank::{get_num_categories_for_object, get_category_for_object}` |
| `CategoryBank::{is_pattern, is_probability_set, expand_members, pattern_members}` | `category_bank.rs` |
| C++ `pickFromProbSet` / server `transform_target` | `CategoryBank::pick_from_prob_set` |
| C++ `autoGenerateCategoryTransitions` lite + pattern | `expand_category_transitions` / `_lite` / `_pattern` (also max-use insert **P4#29**) |
| `load_from_cache` → bank-only when OLT1 expanded / else re-expand | `content_binary.rs` (`OLT1_F_CATEGORY_EXPANDED` **P4#28**; categories text-only) |
| `maybe_load_category_bank_from_root` / `set_category_bank` | `content.rs` (no expand; pick/find_ptrans only) |
| `ClientContent::transitions_category_expanded` | `content.rs` ↔ OLT1 header flags |
| `ClientContent::transitions_max_use` / `find_transition_max_use` / `find_ptrans_max_use` | `content.rs` (**P4#29** Haxe maxUseTarget) |
| `insert_normal_or_max_use` / `insert_transition_record` / `target_remains` | `content.rs` (Haxe double-transition maxUse) |
| `apply_default_switch_number_of_uses_patches` / `ClientTransition::switch_number_of_uses` | `content.rs` (ServerSettings dough/masa keys) |
| OLT1 record bit6 max-use / bit7 switch | `ol_binary` + `content_binary.rs` / server `binary_cache` `load_olt1` |
| Shared OLC1/OLT1 parse+encode (P4#30) | `ol_binary::{parse_olc1,encode_olc1,parse_olt1,encode_olt1}` |
| `OLT1_F_CATEGORY_EXPANDED` / `olt1_lacks_category_expanded` / `peek_blob_flags` | `content_binary.rs` |
| open (C-CAT residual) | reverse-category play consumers (editor mutators **P4#27 DONE**) |
| `OLC1_MAGIC` / `OLC1_FORMAT_VERSION` (6) / `V1`–`V5` legacy | `content_binary.rs` |
| `OLT1_MAGIC` / `OLT1_FORMAT_VERSION` (2) / `OLT1_FORMAT_VERSION_V1` | `content_binary.rs` |
| `write_olc1` / `load_olc1` / `peek_olc1_format` / `write_olt1` / `load_olt1` (write fmt6; load v1–v6) | `content_binary.rs` |
| `bake_content` / `load_from_cache` / `load_prefer_cache` (auto-rebuild; OLC1 format stale **or** OLT1 unexpanded flag; OLO1) | `content_binary.rs` |
| `OverlayBank` / `OverlayRecord` / `get_overlay` / `ensure_image` / `search_overlays` | `overlay_bank.rs` |
| `write_olo1` / `load_olo1` / `bake_olo1_from_root` / `scan_overlay_index` / `OLO1_MAGIC` | `overlay_bank.rs` |
| C++ `initOverlayBank*` / `getOverlay` / `searchOverlays` | `overlayBank.cpp` → `overlay_bank` (**C-OVL DONE→v1**; add/delete + thumbnailSprite + OverlayPickable out of scope) |
| `assign_multi_use_dummies` / `materialize_dummy_object_records` (H-BAKE / ObjectBake + **setupSpriteUseVis P4#25**) | `content_binary.rs` |
| `assign_variable_dummies` / `materialize_variable_dummy_object_records` (**variableDummyIDs P4#26**) | `content_binary.rs` |
| `parse_variable_dollar_count` / `var_object_label` / `var_object_numeral` | `content.rs` |
| C++ `setupSpriteUseVis` / `spriteSkipDrawing` / `useVanishIndex` / `useAppearIndex` | `content::{setup_sprite_use_vis, apply_sprite_use_vis}` + draw/hit skip |
| Server `ol_content::{load_from_cache, load_prefer_cache, load_olc1, load_olt1, finish_cache_boot}` | `RustServer/crates/ol-content/src/binary_cache.rs` |
| Server OLC1 v3 → ObjectDef sim + `biome_spawn`; prefer-cache text fallback only if all `map_chance==0` | `binary_cache.rs` |
| Haxe `ObjectBake` multi-use dummies | `openlife/resources/ObjectBake.hx` → `assign_multi_use_dummies` |
| `AnimBank` / `ObjectAnimation` / `SpriteAnimParam` / `AnimSample` / `SoundAnimParam` | `anim_bank.rs` |
| `parse_animation_txt` / `parse_anim_filename` | `anim_bank.rs` |
| C++ `scanAnimationRecordFromString` | `animationBank.cpp` → `parse_animation_txt` |
| C++ `processFrameTimeWithPauses` | `animationBank.cpp` → `SpriteAnimParam::frame_time` |
| C++ fade hardness (`fadePhase+0.25`, power-square) | `SpriteAnimParam::sample_fade` |
| `write_ola1` / `load_ola1` / `load_ola1_with_version` / `bake_ola1_from_dir` | `anim_bank.rs` |
| `AnimBank::load_prefer_cache` / `from_ola1` / `load_all_text` / `sample_sprite` / `sample_sprite_ex` / `sample_slot` | `anim_bank.rs` |
| `SpriteAnimParam::sample` / `frame_time` / `sample_fade` | `anim_bank.rs` |
| `ANIM_GROUND` / `ANIM_HELD` / `ANIM_MOVING` / `ANIM_GROUND2` / `ANIM_EATING` / `ANIM_DOING` / `ANIM_EXTRA` | `anim_bank.rs` (GROUND2 runtime→ground) |
| `OLA1_MAGIC` / `OLA1_FORMAT_VERSION` (1) | `anim_bank.rs` |
| soft-FB fade α × `AnimSample.fade` + `rot_center` pivot | `render.rs` (`blit` / object draw) |
| `select_player_anim_type` / `select_held_anim_type` / `select_clothing_anim_type` | `anim_draw.rs` |
| `AnimDrawState` / `ObjectAnimPack` / `sample_sprite_pack` / `sample_slot_pack` | `anim_draw.rs` |
| `clothing_pack_from_person` / `AnimDrawState::maybe_skip_fades` | `anim_draw.rs` |
| `is_anim_fade_needed` / `is_anim_fade_needed_records` | `anim_draw.rs` |
| `select_packs_for_player` / `PlayerAnimSelection` | `render.rs` (type-only probe helper) |
| `SceneRenderer::draw` → sync/step packs + `draw_object_with_pack` | `render.rs` |
| `LiveObject::{anim, sync_anim_packs, step_anim, person_anim_pack, held_anim_pack, desired_anim_type}` | `live_object.rs` |
| `LiveWorld::step_anims` / `ClientSession::step_anims` | `live_object.rs` / `session.rs` |
| action wiggle / baby-held handoff / drop offset / BW | **P3#22 DONE** — `anim_draw::{action_wiggle_offset_units,step_pending_action_progress,baby_wiggle_*,held_by_drop_*,step_held_pos_handoff}` + `LiveObject` fields + `SceneRenderer` held-baby draw |
| open: front/back arm freeze indices, Jenkins reseed | L-ANIM-DRAW residual (extraB **P3#19**, action wiggle/handoff **P3#22 DONE**) |
| SoundAnim / footstep / shouldCreation / clothing-drop / MX contained fill / offScreenSound | L-SOUND-TRIG v6 wired (P2#13–14) |
| PE emotes / emotion layers | C++ `LivingLifePage` PLAYER_EMOT + `setAnimationEmotion` + say-field `getEmotionIndex`→EMOT · Rust `emotion` + `LiveWorld::apply_emots_with_bank` + `SceneRenderer::draw_emotion_layers` + `session.send_say` / `encode_emot` (**P3#18**); **P3#19** mainEyesOffset eyeEmot, EXTRA↔EXTRA_B toggle, mouth skip, creation/decay sounds |

---

## C-SND / L-SOUND-TRIG sound bank

| Symbol | Where |
|--------|-------|
| `SoundBank` / `SoundIndexEntry` / `PcmSound` / `SoundUsage` / `SoundUsagePlay` | `sound_bank.rs` |
| `OLSN_MAGIC` / `OLSN_FORMAT_VERSION` (1) / `AIFF_SAMPLE_START` (54) | `sound_bank.rs` |
| `write_olsn` / `load_olsn` / `bake_olsn_from_dir` / `scan_sounds_dir` | `sound_bank.rs` |
| `SoundBank::load_prefer_cache` / `ensure` / `ensure_pcm` / `get_index` | `sound_bank.rs` |
| `read_mono16_aiff` / `peek_aiff_header` (Haxe mono-16 layout) | `sound_bank.rs` |
| `parse_sound_usage` / `SoundUsage::play_random` / `play_usage` / `play_id` | `sound_bank.rs` |
| `play_pcm_samples` / `play_pcm_samples_stereo` / `mix_voices_f32` / `MixVoice` / `AUDIO_MAX_VOICES` | `sound_bank.rs` (device mixer; L/R gains) |
| `get_vector_from_camera` / `get_volume_and_pan` / `volume_pan_reverb` / `stereo_gains_constant_power` | `sound_bank.rs` (C++ LivingLife + soundBank + minorGems) |
| `SoundBank::{set_listener, play_id_at, play_usage_at, last_pan}` | `sound_bank.rs` (camera vector pan) |
| `SoundPlacement` / `MAX_AUDIBLE_DISTANCE` / `REVERB_CONSTANT` | `sound_bank.rs` |
| `audio_device_active` / `audio_feature_enabled` | `sound_bank.rs` |
| `resolve_footstep_usage` / `play_footstep` / `play_object_event_sound` / `play_anim_sound` | `sound_bank.rs` |
| `sound_param_should_play` / `handle_anim_sound` / `handle_anim_sound_ex` / `floor_using_sound_at` | `sound_bank.rs` (C++ `handleAnimSound` ~4392) |
| `should_creation_sound_play` / `is_sprite_subset` / `same_use_dummy_parent` / `play_creation_sound_if` | `sound_bank.rs` (C++ ~12971) |
| `play_mx_change_sounds` / `MxSoundContext` / `play_container_fill_using_sound` / `play_contained_slot_change_sound` | `sound_bank.rs` (MX ~16812–17210, P2#13) |
| `both_same_use_parent` / `is_less_used_than` / `this_use_dummy_index` / `single_contained_change_index` | `sound_bank.rs` (multi-use fill-up) |
| `OffScreenSoundEvent` / `maybe_register_off_screen_sound` / `parse_off_screen_sound_flags` | `sound_bank.rs` (C++ `addOffScreenSound` ~4245) |
| `play_clothing_contained_fill_sound` / `clothing_slot_contained_count` | `sound_bank.rs` (PU bag fill ~19400) |
| `step_map_ground_anims_with_sounds` | `sound_bank.rs` (P2#14 map ground/floor anim) |
| `play_clothing_change_sound` / `clothing_added_id` / `play_drop_settle_sound` | `sound_bank.rs` (PU clothing ~18372 / drop) |
| `play_emot_object_sounds` / `play_emot_creation_for_targets` / `play_emot_decay_for_targets` | `sound_bank.rs` (PE creation ~21246 / decay ~22475, **P3#19**) |
| `ANIM_EXTRA_B` / `is_extra_anim_type` / `ObjectAnimPack.extra_index_b` | `anim_bank.rs` / `anim_draw.rs` (C++ `extraB` + `setExtraIndexB`) |
| `LiveObject::{emot_extra_anim_type, resolved_emot_extra_pack}` | `live_object.rs` (PE EXTRA↔EXTRA_B toggle) |
| `ClientSession::play_mx_sounds` / `play_pu_sounds` | `session.rs` |
| `ClientObjectDef::{creation_sound_initial_only,creation_sound_force}` | `content.rs` + OLC1 flag bits 9–10 |
| `LiveWorld::step_anims_with_sounds` / `ClientSession::step_anims` | `live_object.rs` / `session.rs` |
| `SceneRenderer::sounds` + draw → step_anims_with_sounds | `render.rs` |
| `ClientObjectDef::{creation,using,eating,decay}_sound` / `parse_object_sounds_csv` | `content.rs` (sounds= CSV) |
| OLC1 format 4 sound trailer | `content_binary.rs` |
| MX creation/decay + flush USE/DROP/SELF + PU eating | `session.rs` |
| C++ `scanSoundUsage` / `playRandom` | `SoundUsage.cpp` → `parse_sound_usage` |
| C++ `playSound(SoundUsage)` / LivingLife footstep ~4466 | wired; device via `--features audio` (cpal) |
| C++ `getVectorFromCamera` / `getVolumeAndPan` / stereo pan | `get_vector_from_camera` + `volume_pan_reverb` + `play_usage_at` / `set_listener`; mixer L/R gains |
| Haxe `Sound.readMono16AIFFData` / `Resource.sound` | `Sound.hx` / `Resource.hx` → `read_mono16_aiff` |
| bake writes `olsn_sounds.bin` + manifest | `content_binary::bake_content` |
| load_bench `sound_index_load` (assert zero AIFF at boot) | `load_bench.rs` |
| Cargo feature `audio = ["dep:cpal"]` | `Cargo.toml`; env `OHOL_AUDIO_DISABLE` skips device |
| Audio thread `ohol-audio` (owns cpal `Stream`) | `sound_bank::device` — mpsc voice queue |

---

## L-HUD food / heat

| Symbol | Where |
|--------|-------|
| `HudState` / `HudSprites` / `HudStripSprite` / `OldArrow` | `hud.rs` |
| `draw_food_heat_hud` / `draw_hud_if_visible` | `hud.rs` (`&mut HudState` for draw-time trail) |
| `hunger_box_screen_pos` / `temp_arrow_screen_pos` / `hud_scale` / `yum_screen_pos` / `ate_screen_pos` / `curse_token_screen_pos` | `hud.rs` |
| `HudState::apply_fx` / `apply_hx` / `apply_curse_tokens` / `apply_excess_curse_points` / `prepare_temp_arrow` / `sync_from_session` / `clear` | `hud.rs` |
| `draw_hunger_max_fill_line` (private) | `hud.rs` ← C++ `drawHungerMaxFillLine` ~5958 |
| mult-blend hunger/arrow chrome | `Framebuffer::put_multiplicative` / HUD blit |
| C++ hunger boxes + fills + erased trailing max | `LivingLifePage.cpp` ~10803–10838 → mult-blend blit |
| C++ temp arrows + OldArrow trail + `(heat-0.5)*120` | `LivingLifePage.cpp` ~10843–10899 → `prepare_temp_arrow` (draw-time) |
| C++ `splitAndExpandSprites` | `LivingLifePage.cpp` ~2893 → `load_strip` |
| C++ `maxFoodStore` / `maxFoodCapacity` | LiveObject → `HudState::apply_fx` |
| `SceneRenderer::{hud, hud_sprites, draw_hud, sync_hud, sync_hud_ex, clear_hud}` | `render.rs` |
| `sync_hud(None, None)` after logout | clears peaks + OldArrow (C++ death reset) |
| `SceneRenderer::{screen_to_tile, set_highlight_from_screen}` | `render.rs` (LMB walk pick) |
| `ClientSession::{walk_to, walk_to_screen}` → `click_tile` | `session.rs` / `click_tile.rs` |
| FX/HX/CX wire | `parse.rs` + `session.food`/`heat`/`curse_tokens` (no `excess_curse_points` on session yet) |
| `ohol-client` LMB → tile → MOVE + `sync_hud_ex` FX/HX/CX/dying; mut `world` for draw | `bin/ohol_client.rs` |
| Graphics TGAs | `hungerBoxes*`, `tempArrows*`, `hungerBars*`/`hungerBarsErased`, `hungerDashes*`/`hungerDashesErased`, `guiPanel`, `guiBlood`, `font_pencil_32_32`/`font_pencil_erased_32_32`, `yumSlip1–4`, `fullSlip`/`hungrySlip`/`starvingSlip`, `homeArrows*` under `OneLifeGameSourceData/graphics/` |
| Residual draw | `PencilFontAtlas`, hunger/yum slips, `set_home_arrow`, temp tip via `set_pointer`, old yum/ate stacks, `hide_gui` |
| `HudState::step_slips` / `HungerSoundEvent` / `hunger_slip_draw_y_below` | `hud.rs` — C++ slip slide + wiggle (~14550) |
| `SoundBank::play_hunger_sound` / `HUNGER_SOUND_REL` | `sound_bank.rs` — `otherSounds/hunger.aiff` center pan |
| `HomePos` / `HomePosStack` / `home_location_key_priority` | `live_object.rs` — C++ `homePosStack` |
| `LiveWorld::apply_home_marker_mx` / PS → `add_temp` | `live_object.rs` + `session.apply_home_marker_mx` |
| `ClientSession::deferred_fx` / `flush_deferred_fx_for` | `session.rs` — FX responsible_id mid-walk defer |
| `SceneRenderer::sync_home_hud` | `render.rs` — stack-first home arrow + label |
| `ClientObjectDef::home_marker` | text `homeMarker=` + `eveHomeMarker` tag |
| open | true object names on last-ate; optional pre-sliced chrome binary cache; dual ancient home-slip draw; music suppress on starve |
