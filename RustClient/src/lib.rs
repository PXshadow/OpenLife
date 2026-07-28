//! Headless One Hour One Life client library.
//!
//! Wire format follows Jason Rohrer's `server/protocol.txt` and the official
//! client's `LivingLifePage.cpp` send paths (LOGIN/RLOGIN, MOVE, USE/DROP/REMV/SELF/KA/FORCE).

pub mod account_page;
pub mod actions;
pub mod anim_bank;
pub mod anim_draw;
pub mod binpack;
pub mod category_bank;
pub mod click_tile;
pub mod client_map;
pub mod client_screen;
pub mod content;
pub mod content_binary;
pub mod emotion;
pub mod event_util;
pub mod frame;
pub mod ground_sprites;
pub mod hover_pick;
pub mod hud;
pub mod live_object;
pub mod load_bench;
pub mod load_progress;
pub mod login;
pub mod map_global_offset;
pub mod move_state;
mod multi_move_ext;
pub mod parse;
pub mod pathfind;
pub mod play_snapshot;
pub mod render;
pub mod rmb_action;
pub mod session;
pub mod settings_page;
pub mod music_bank;
pub mod overlay_bank;
pub mod sound_bank;
pub mod sprite_bank;
pub mod tags;
pub mod tga;
pub mod wire_log;

pub use account_page::{
    AccountAction, AccountFocus, AccountKey, AccountPage, ClientAppState, ClientScreen, SecretMode,
};
pub use settings_page::{
    draw_settings_screen, settings_key_command, ClientSettings, GraphicsMode, SettingsAction,
    SettingsFocus, SettingsKey, SettingsPage, CLIENT_SETTINGS_FILE,
};
pub use play_snapshot::{
    draw_snapshot_button, snapshot_button_hit, write_play_snapshot, PlaySnapshot,
    SnapshotViewExtras, DEFAULT_SNAPSHOT_DIR, SNAPSHOT_FORMAT, SNAPSHOT_MAGIC,
};
pub use actions::{
    ObjectAction, encode_baby, encode_drop, encode_emot, encode_force, encode_jump, encode_ka,
    encode_remv, encode_say, encode_self, encode_sremv, encode_swap, encode_ubaby, encode_use,
};
pub use anim_bank::{
    bake_ola1_from_dir, load_ola1, load_ola1_with_version, parse_anim_filename, parse_animation_txt,
    write_ola1, AnimBank, AnimSample, ObjectAnimation, SoundAnimParam, SpriteAnimParam,
    is_extra_anim_type, ANIM_DOING, ANIM_EATING, ANIM_EXTRA, ANIM_EXTRA_B, ANIM_GROUND,
    ANIM_GROUND2, ANIM_HELD, ANIM_MOVING, OLA1_FORMAT_VERSION, OLA1_FORMAT_VERSION_V1, OLA1_MAGIC,
};
pub use anim_draw::{
    action_wiggle_offset_units, baby_wiggle_offset_x_units, clothing_pack_from_person,
    held_by_drop_offset_from_raw, is_anim_fade_needed, is_anim_fade_needed_records,
    sample_slot_pack, sample_sprite_pack, select_clothing_anim_type, select_held_anim_type,
    select_player_anim_type, step_baby_wiggle, step_held_by_drop_offset, step_held_pos_handoff,
    step_pending_action_progress, AnimDrawState, ObjectAnimPack, ACTION_WIGGLE_MAX_UNITS,
    ANIM_END, ANIM_FADE_STEP, BABY_WIGGLE_AMP_UNITS, BABY_WIGGLE_PROGRESS_INC,
    DROP_OFFSET_STEP, PENDING_ACTION_PROGRESS_INC, PENDING_ACTION_START_PROGRESS,
};
pub use binpack::{BinPack, Rect as PackRect};
pub use click_tile::{
    apply_click_gates, can_execute_action_at, click_drop, click_drop_clothing, click_object,
    click_remv, click_remv_hit, click_remove_clothing, click_self, click_sremv_clothing,
    click_swap, click_tile, click_tile_mod, click_tile_mod_ex, click_tile_with, click_use,
    clothing_slot_for_object, hold_walk_or_use_tile, is_grid_adjacent, is_self_tile,
    maybe_close_hold_throw, our_clothing, path_start_tile, plan_click_tile, plan_click_tile_chunks,
    plan_click_tile_chunks_goal, plan_click_tile_chunks_with, plan_stand_for_object,
    plan_stand_for_object_ex, resolve_clothing_equip_slot, resolve_hold_click_dest,
    resolve_use_object_id, select_self_action, select_self_action_ex, select_tile_action,
    slide_blocked_click_dest, stand_allows_access, walk_or_use_tile, walk_or_use_tile_ex,
    walk_or_use_tile_hold, walk_to as walk_to_tile, ClickTileExt, ClickTileResult,
    ObjectClickResult, StandAccess, TileClickPlan, WalkOrUseResult, HOLD_SLIDE_LIMIT,
    MIN_MOUSE_DOWN_FRAMES, NO_MOVE_AGE,
};
pub use client_map::{
    compress_mc_plain, parse_object_raw_contained, parse_object_raw_stack, ClientMap, MapTile,
    ObjectStackNode,
};
pub use client_screen::{
    death_key_command, draw_death_screen, format_death_reason, note_our_death_if_any,
    rebirth_session_config, DeathKey, DeathSummary, ScreenCommand,
};
pub use category_bank::{
    expand_category_transitions, expand_category_transitions_lite,
    expand_category_transitions_pattern, format_category_txt, parse_category_txt, CategoryBank,
    CategoryRecord, ReverseCategoryRecord,
};
pub use content::{
    apply_default_switch_number_of_uses_patches, apply_object_description_tags,
    apply_sprite_use_vis, arm_holding_parameters, compute_held_draw_pos, compute_held_draw_pos_ex,
    description_has_var_numeral, eyes_anchor_from_head, get_object_center_offset,
    get_object_center_offset_simple, insert_normal_or_max_use, insert_transition_record,
    parse_contain_offset_tags, parse_object_sounds_csv, parse_variable_dollar_count,
    rotate_offset_turns, setup_sprite_use_vis, sound_usage_is_blank, target_remains,
    var_object_label, var_object_numeral, variable_target_is_hidden, ClientContent,
    ClientObjectDef, ClientTransition, HoldingPos, ObjectSprite, SpriteCenterInfo,
    MAIN_EYES_OFFSET_AGE,
};
pub use content_binary::{
    assign_multi_use_dummies, assign_variable_dummies, bake_content, cache_dir_for, load_from_cache,
    load_olc1, load_olt1, load_prefer_cache, load_prefer_cache_with_progress,
    materialize_dummy_object_records, materialize_variable_dummy_object_records,
    olt1_lacks_category_expanded, parse_manifest, peek_blob_flags, peek_olc1_format,
    read_data_version, write_olc1, write_olt1, BakeResult, BakeTimings, ContentManifest,
    ManifestBlob, OLC1_FORMAT_VERSION, OLC1_FORMAT_VERSION_V1, OLC1_FORMAT_VERSION_V2,
    OLC1_FORMAT_VERSION_V3, OLC1_FORMAT_VERSION_V4, OLC1_FORMAT_VERSION_V5,
    OLC1_FORMAT_VERSION_V6, OLC1_FORMAT_VERSION_V7, OLC1_MAGIC,
    OLT1_FORMAT_VERSION, OLT1_FORMAT_VERSION_V1, OLT1_F_CATEGORY_EXPANDED, OLT1_MAGIC,
};
pub use emotion::{
    classify_speech_outbound, Emotion, EmotionBank, SpeechOutbound, DEFAULT_EMOT_DURATION_SEC,
};
pub use load_bench::{
    bench_full, bench_graphics_load, bench_headless_load, resolve_content_root, write_report,
    LoadProfile, TimedStep,
};
pub use load_progress::{
    boot_load_prefer_cache, draw_loading_progress, emit_progress, env_log_callback,
    format_progress_line, fractions_monotonic, load_progress_env_enabled, log_progress_line,
    reborrow_cb, report_stage, BootBanks, LoadProgress, LoadStage, LoadingState, ProgressCb,
};
pub use ground_sprites::{
    bake_olg1_from_roots, bake_olg1_to_dir, bake_olga_from_roots, bake_olga_to_dir, biome_color,
    biome_color_varied, ground_map_key_biome, ground_map_key_unknown, ground_overlay_slot,
    ground_variation_index, load_olg1, write_olg1, GroundBank, GroundIndexEntry, GroundTileRect,
    OlgaBakeStats, OlgaLoadStats, Olg1Meta, CELL_D, GROUND_ATLAS, GROUND_OVERLAY_COUNT,
    OLGA_FORMAT_VERSION,
    OLGA_MAGIC, OLG1_FORMAT_VERSION, OLG1_KIND_BIOME, OLG1_KIND_OVERLAY, OLG1_KIND_UNKNOWN,
    OLG1_MAGIC, UNKNOWN_BIOME_CACHE_ID,
};
pub use hover_pick::{
    draw_hover_outline, map_stack_index_to_hit_slot, pick_at_screen, pick_at_screen_with_clothing,
    pick_worn_clothing_slot, resolve_hit_slot, update_scene_hover,
    update_scene_hover_with_clothing, HoverPick, WornClothingPickTarget,
};
pub use hud::{
    ate_screen_pos, curse_token_screen_pos, draw_food_heat_hud, draw_hud_if_visible,
    draw_pencil_string, draw_speech_bubble, draw_speech_bubble_colored, draw_speech_bubble_with,
    glyph5x7, hud_scale, hunger_box_screen_pos, pencil_string_width, temp_arrow_screen_pos,
    yum_screen_pos, HudState, HudStripSprite, HudSprites, HungerSoundEvent, OldArrow, OldHudText,
    PencilFontAtlas, ATE_ORIGIN_X, ATE_ORIGIN_Y_BELOW, CURSE_TOKEN_ORIGIN_X,
    CURSE_TOKEN_ORIGIN_Y_BELOW, GUI_PANEL_Y_BELOW, HUD_DESIGN_H, HUD_DESIGN_W, HUNGER_BOX_ORIGIN_X,
    HUNGER_BOX_ORIGIN_Y_BELOW, HUNGER_BOX_PITCH, HUNGER_SLIP_HIDE_Y, HUNGER_SLIP_SHOW_Y,
    NUM_HOME_ARROWS, NUM_HUNGER_BOX_SPRITES, NUM_HUNGER_DASHES, NUM_HUNGER_SLIPS, NUM_TEMP_ARROWS,
    NUM_YUM_SLIPS, TEMP_ARROW_ORIGIN_X, TEMP_ARROW_ORIGIN_Y_BELOW, TEMP_ARROW_SPAN, YUM_ORIGIN_X,
    YUM_ORIGIN_Y_BELOW, YUM_SLIP_HIDE_Y_BELOW, YUM_SLIP_SHOW_DY,
};
pub use event_util::{bootstrap_label, note_map_changes, note_names, player_says_contains};
pub use frame::{FrameReader, FramedMessage, compress_cm_payload, encode_raw, inflate_cm};
pub use live_object::{
    clothing_char_to_slot, format_curse_tag, home_dir_index, home_location_key_priority,
    says_pointer_ttl_sec, speech_hold_sec, speech_text_rgb, ClothingSet, HomePos, HomePosStack,
    LiveObject, LiveWorld, LocationSpeech, SaysPointerMarker, CLOTHING_SLOT_COUNT,
    CLOTHING_SLOT_NAMES, MAP_SPOT_MARKER_RGBA, MAX_CURSE_TAG_DISPLAY_GAP,
    SAYS_POINTER_DEFAULT_TTL_SEC, SAYS_POINTER_EXPERT_EXTRA_SEC, SPEECH_FADE_STEP,
};
pub use login::{LoginParams, encode_login, hmac_sha1_hex, pure_account_key};
pub use map_global_offset::{
    encode_move_with_offset, MapGlobalOffset,
};
pub use move_state::{
    encode_move, BASE_PATH_SPEED, MAX_PATH_DELTA, MoveError, MoveState, PathDelta,
};
pub use pathfind::{
    cell_blocks_walking, cell_walkable, chunk_deltas_for_move, cumulative_to_steps, find_path,
    find_path_deltas, find_path_deltas_ex, find_path_ex, find_path_via_waypoint,
    find_path_via_waypoint_ex, find_path_with_waypoint_ex, is_bad_biome_at, is_bad_biome_tile,
    parse_bad_biome_ids, parse_bad_biomes, path_cell_count, steps_to_cumulative, PathFindOpts,
    PathFindResult, CLOSE_HOLD_THROW_TILES, DEFAULT_MAX_EXPAND, DEFAULT_MAX_WAYPOINT_PATH_LENGTH,
    PATH_FINDING_D,
};
pub use render::{
    biome_color_for, draw_map_spot_marker, map_window_to_fb, select_packs_for_player,
    stretch_rgba_nearest, tile_screen_rect, Camera, Framebuffer, PlayerAnimSelection,
    SceneRenderer, GRID, ZOOM_DEFAULT, ZOOM_MAX, ZOOM_MIN,
};
pub use rmb_action::{
    click_rmb_tile, click_rmb_tile_ex, click_rmb_tile_hit, our_held_id, tile_allows_remv,
    RmbClickExt, RmbClickResult,
};
pub use music_bank::{
    decode_ogg_vorbis_mono, music_rel_path, next_music_block, parse_music_filename, scan_music_dir,
    MusicBank, MusicIndexEntry, MusicPcm,
};
pub use overlay_bank::{
    bake_olo1_from_root, bake_olo1_to_dir, load_olo1, scan_overlay_index, write_olo1, OverlayBank,
    OverlayRecord, OLO1_FORMAT_VERSION, OLO1_MAGIC,
};
pub use sound_bank::{
    audio_device_active, audio_device_allowed, audio_device_enabled_setting, audio_disable_env_set,
    audio_feature_enabled, bake_olsn_from_dir, bake_olsn_to_dir, music_muted,
    set_audio_device_enabled, set_music_muted, set_sfx_muted, sfx_muted,
    both_same_use_parent, clothing_added_id, clothing_slot_contained_count,
    description_has_off_screen_sound, get_object_parent, get_vector_from_camera, get_volume_and_pan,
    handle_anim_sound, handle_anim_sound_ex, is_less_used_than, is_sprite_subset, load_olsn,
    maybe_register_off_screen_sound, mix_voices_f32, parse_off_screen_sound_flags, parse_sound_usage,
    peek_aiff_header, play_clothing_change_sound, play_clothing_contained_fill_sound,
    play_contained_slot_change_sound, play_container_fill_using_sound, play_creation_sound_at_if,
    play_creation_sound_if, play_drop_settle_sound, play_footstep, play_mx_change_sounds,
    play_object_event_sound, play_object_event_sound_at, play_pcm_samples, play_pcm_samples_stereo,
    play_emot_creation_for_targets, play_emot_decay_for_targets, play_emot_object_sounds,
    play_usage, read_mono16_aiff, reverb_mix_from_volume, same_use_dummy_parent, scan_sounds_dir,
    should_creation_sound_play, single_contained_change_index, sound_param_should_play,
    step_map_ground_anims_with_sounds, stereo_gains_constant_power, this_use_dummy_index,
    volume_pan_reverb, write_olsn, MixVoice, MxSoundContext, OffScreenSoundEvent, PcmSound,
    SoundBank, SoundIndexEntry, SoundPlacement, SoundUsage, SoundUsagePlay, AIFF_SAMPLE_START,
    AUDIO_MAX_VOICES, CURSE_CHIME_REL, CURSE_CHIME_VOLUME, MAX_AUDIBLE_DISTANCE,
    MIN_FADE_START_DISTANCE, OLSN_FORMAT_VERSION, OLSN_F_HEADER_PEEKED, OLSN_F_IS_OGG,
    OLSN_F_MONO16_VERIFIED, OLSN_MAGIC, REVERB_CONSTANT,
};
pub use sprite_bank::{
    bake_ols1_from_dir, bake_ols1_to_dir, bake_olsa_from_dir, bake_olsa_to_dir, compute_alpha_info,
    expand_map, parse_sprite_txt, scan_sprites_dir, OlsaBakeStats, OlsaLoadStats, SpriteAlphaInfo,
    SpriteBank, SpriteMeta, SpriteRect, ATLAS_SIZE, OLSA_FORMAT_VERSION, OLSA_MAGIC,
    OLS1_FORMAT_VERSION, OLS1_FORMAT_VERSION_V1, OLS1_MAGIC,
};
pub use tga::{load_tga_bytes, load_tga_path, RgbaImage};
pub use parse::{
    message_type, parse_cu_message, parse_fx_message, parse_hx_message, parse_inbound,
    parse_login_outcome, parse_ls_message, parse_mc_header, parse_ms_message, parse_mx_line,
    parse_mx_message, parse_nm_message, parse_pe_message, parse_pm_line, parse_pm_message,
    parse_ps_line, parse_ps_message, parse_pu_line, parse_pu_message, parse_sn, Craving,
    CurseScoreChange, CurseTokenChange, CursedPlayer, DyingPlayer, FlightDest, FoodChange,
    GlobalMessage, HeatChange, InboundMessage, Lineage, LocationSays, LoginOutcome, MapChange,
    MapChunkHeader, MonumentCall, PlayerEmot, PlayerMoveStart, PlayerName, PlayerSays, PlayerUpdate,
    SaysMapPointer, SaysTargetLabel, ServerHello, ValleySpacing,
};
pub use session::{SessionConfig, SessionEvent, connect_and_login, connect_and_login_logged};
pub use tags::{ALL_SERVER_TAGS, ServerTag};
pub use wire_log::WireLog;
